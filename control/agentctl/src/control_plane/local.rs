//! Bounded, kernel-authenticated local transport for Workestrate control.
//!
//! This endpoint is host-only. A host process forwarding a VM connection must
//! use a verified workload adapter instead; forwarding into this socket would
//! incorrectly turn the forwarder's Unix UID into the guest's authority.

use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::future::Future;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};

use super::authorization::{AuthenticatedCaller, CallerIdentity, Permission};
pub use super::types::ControlRequest;
use super::types::{ControlError, ControlResponse};

pub const PROTOCOL_VERSION: u16 = 2;
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;
pub const REQUEST_DEADLINE: Duration = Duration::from_secs(10);

/// Permissions originate in server configuration, not a request or socket name.
#[derive(Debug)]
pub struct LocalAccess {
    operator_uid: u32,
    callers: BTreeMap<u32, Vec<Permission>>,
}

impl LocalAccess {
    pub fn new(operator_uid: u32) -> Self {
        Self {
            operator_uid,
            callers: BTreeMap::new(),
        }
    }

    /// Reachability is separate from permission. The default private endpoint
    /// is not made group/world-accessible by adding an entry here.
    pub fn allow_local_uid(&mut self, uid: u32, permissions: Vec<Permission>) {
        self.callers.insert(uid, permissions);
    }

    fn authenticate(&self, stream: &UnixStream) -> io::Result<AuthenticatedCaller> {
        let uid = stream.peer_cred()?.uid();
        if uid == self.operator_uid {
            return Ok(AuthenticatedCaller::local_operator(uid));
        }
        self.callers
            .get(&uid)
            .map(|permissions| {
                AuthenticatedCaller::restricted(CallerIdentity::LocalUid(uid), permissions.clone())
            })
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "local control caller is not authorized",
                )
            })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestEnvelope {
    pub version: u16,
    pub id: u64,
    pub request: ControlRequest,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResponseEnvelope {
    pub version: u16,
    pub id: u64,
    pub result: Result<ControlResponse, ControlError>,
}

/// Process-held singleton ownership and inode-scoped socket cleanup.
/// The stable lock file is never unlinked or taken over using a PID test.
pub struct LocalEndpoint {
    listener: UnixListener,
    path: PathBuf,
    identity: (u64, u64),
    directory: PathBuf,
    directory_file: File,
    owner: File,
    owner_uid: u32,
    lost: AtomicBool,
}

impl LocalEndpoint {
    /// Bind only below an operator-owned private directory. Existing unsafe
    /// directories or files are rejected, never silently chmod'ed or replaced.
    pub fn bind(directory: &Path, access: &LocalAccess) -> io::Result<Self> {
        std::fs::DirBuilder::new()
            .mode(0o700)
            .recursive(true)
            .create(directory)?;
        let metadata = std::fs::symlink_metadata(directory)?;
        private_directory(&metadata, access.operator_uid)?;
        let directory_file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(directory)?;
        if !same_file(&metadata, &directory_file.metadata()?) {
            return Err(ownership_lost());
        }
        let lock_path = directory.join("owner.lock");
        match std::fs::symlink_metadata(&lock_path) {
            Ok(metadata) => private_owner(&metadata, access.operator_uid)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let owner = open_owner(&directory_file)?;
        check_domain(directory, &directory_file, &owner, access.operator_uid)?;
        owner.try_lock().map_err(|error| match error {
            std::fs::TryLockError::WouldBlock => {
                io::Error::new(io::ErrorKind::AddrInUse, "control service is already owned")
            }
            std::fs::TryLockError::Error(error) => error,
        })?;
        check_domain(directory, &directory_file, &owner, access.operator_uid)?;
        let path = directory.join("control.sock");
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if !metadata.file_type().is_socket()
                    || metadata.uid() != access.operator_uid
                    || metadata.nlink() != 1
                {
                    return Err(io::Error::new(
                        io::ErrorKind::PermissionDenied,
                        "control endpoint is not an owned socket",
                    ));
                }
                // Only this process holds the stable ownership lock. An old
                // cooperating service can no longer be listening at this path.
                check_domain(directory, &directory_file, &owner, access.operator_uid)?;
                unlink_socket(&directory_file)?;
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        check_domain(directory, &directory_file, &owner, access.operator_uid)?;
        let listener = UnixListener::bind(&path)?;
        let metadata = std::fs::symlink_metadata(&path)?;
        let endpoint = Self {
            listener,
            path,
            identity: (metadata.dev(), metadata.ino()),
            directory: directory.to_owned(),
            directory_file,
            owner,
            owner_uid: access.operator_uid,
            lost: AtomicBool::new(false),
        };
        endpoint.check_current()?;
        Ok(endpoint)
    }

    pub async fn accept(&self) -> io::Result<UnixStream> {
        self.check_current()?;
        loop {
            tokio::select! {
                accepted = self.listener.accept() => {
                    self.check_current()?;
                    return accepted.map(|(stream, _)| stream);
                }
                _ = tokio::time::sleep(Duration::from_secs(1)) => self.check_current()?,
            }
        }
    }

    /// Refuse permanently after observed namespace/ownership loss, even if its
    /// old path is restored. The service owner must also fence its retained
    /// broker connection on this error. Checks detect observed replacement;
    /// they are not an atomic exclusion of a hostile same-UID filesystem actor.
    pub fn check_current(&self) -> io::Result<()> {
        if self.lost.load(Ordering::Acquire) {
            return Err(ownership_lost());
        }
        let result = (|| {
            check_domain(
                &self.directory,
                &self.directory_file,
                &self.owner,
                self.owner_uid,
            )?;
            let socket = std::fs::symlink_metadata(&self.path)?;
            if !socket.file_type().is_socket()
                || socket.uid() != self.owner_uid
                || socket.nlink() != 1
                || (socket.dev(), socket.ino()) != self.identity
            {
                return Err(ownership_lost());
            }
            Ok(())
        })();
        if result.is_err() {
            self.lost.store(true, Ordering::Release);
        }
        if self.lost.load(Ordering::Acquire) {
            return Err(ownership_lost());
        }
        result
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for LocalEndpoint {
    fn drop(&mut self) {
        if self.check_current().is_ok() {
            // The retained directory anchors cleanup even if its pathname
            // changes after the final check. Never unlink through a new domain.
            let _ = unlink_socket(&self.directory_file);
        }
    }
}

fn ownership_lost() -> io::Error {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        "local control ownership lost",
    )
}

fn same_file(left: &std::fs::Metadata, right: &std::fs::Metadata) -> bool {
    (left.dev(), left.ino()) == (right.dev(), right.ino())
}

fn private_directory(metadata: &std::fs::Metadata, uid: u32) -> io::Result<()> {
    if !metadata.is_dir()
        || metadata.uid() != uid
        || metadata.mode() & 0o077 != 0
        || metadata.nlink() == 0
    {
        return Err(ownership_lost());
    }
    Ok(())
}

fn private_owner(metadata: &std::fs::Metadata, uid: u32) -> io::Result<()> {
    if !metadata.is_file()
        || metadata.uid() != uid
        || metadata.mode() & 0o077 != 0
        || metadata.nlink() != 1
    {
        return Err(ownership_lost());
    }
    Ok(())
}

fn check_domain(directory: &Path, retained: &File, owner: &File, uid: u32) -> io::Result<()> {
    let named_directory = std::fs::symlink_metadata(directory)?;
    let held_directory = retained.metadata()?;
    private_directory(&named_directory, uid)?;
    private_directory(&held_directory, uid)?;
    if !same_file(&named_directory, &held_directory) {
        return Err(ownership_lost());
    }
    let named_owner = std::fs::symlink_metadata(directory.join("owner.lock"))?;
    let held_owner = owner.metadata()?;
    private_owner(&named_owner, uid)?;
    private_owner(&held_owner, uid)?;
    if !same_file(&named_owner, &held_owner) {
        return Err(ownership_lost());
    }
    Ok(())
}

#[allow(unsafe_code)]
fn open_owner(directory: &File) -> io::Result<File> {
    // SAFETY: the directory fd is borrowed live, the literal is NUL terminated,
    // and success returns a new owned fd. No path supplied by a caller is joined.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            c"owner.lock".as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_CLOEXEC | libc::O_NONBLOCK,
            0o600 as libc::mode_t,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: openat succeeded and this is the only owner of the new descriptor.
    Ok(unsafe { File::from_raw_fd(fd) })
}

#[allow(unsafe_code)]
fn unlink_socket(directory: &File) -> io::Result<()> {
    // SAFETY: the fd remains live for this call and the fixed filename is NUL
    // terminated. unlinkat does not follow a final symlink or traverse outside
    // the retained directory; ownership checks precede every production call.
    if unsafe { libc::unlinkat(directory.as_raw_fd(), c"control.sock".as_ptr(), 0) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Authenticate before decoding, authorize before dispatch, and bound the entire
/// request/handler/reply exchange. One connection carries one request.
pub async fn serve_connection<F, Fut>(
    mut stream: UnixStream,
    access: &LocalAccess,
    handler: F,
) -> io::Result<()>
where
    F: FnOnce(AuthenticatedCaller, ControlRequest) -> Fut,
    Fut: Future<Output = Result<ControlResponse, ControlError>>,
{
    let caller = access.authenticate(&stream)?;
    tokio::time::timeout(REQUEST_DEADLINE, async {
        let bytes = read_line(&mut stream).await?;
        let envelope: RequestEnvelope = serde_json::from_slice(&bytes)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid control request"))?;
        let result = if envelope.version != PROTOCOL_VERSION {
            Err(ControlError::UnsupportedVersion)
        } else if envelope.id == 0 {
            Err(ControlError::InvalidRequest)
        } else {
            match caller.authorize_control(&envelope.request) {
                Ok(()) => handler(caller, envelope.request).await,
                Err(error) => Err(error),
            }
        };
        write_json(
            &mut stream,
            &ResponseEnvelope {
                version: PROTOCOL_VERSION,
                id: envelope.id,
                result,
            },
        )
        .await
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "control request deadline exceeded"))?
}

/// Client-side framing validates both version and correlation before accepting
/// state. Callers must still inspect `ready`/`observed`, not just transport success.
pub async fn request(
    mut stream: UnixStream,
    envelope: &RequestEnvelope,
) -> io::Result<ResponseEnvelope> {
    tokio::time::timeout(REQUEST_DEADLINE, async {
        write_json(&mut stream, envelope).await?;
        let bytes = read_line(&mut stream).await?;
        let response: ResponseEnvelope = serde_json::from_slice(&bytes)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid control response"))?;
        if response.version != PROTOCOL_VERSION || response.id != envelope.id {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "control response version or correlation mismatch",
            ));
        }
        Ok(response)
    })
    .await
    .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "control client deadline exceeded"))?
}

async fn read_line(stream: &mut UnixStream) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    let mut reader = BufReader::new(stream.take((MAX_MESSAGE_BYTES + 1) as u64));
    reader.read_until(b'\n', &mut bytes).await?;
    if bytes.len() > MAX_MESSAGE_BYTES || bytes.last() != Some(&b'\n') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "control message is oversized or incomplete",
        ));
    }
    Ok(bytes)
}

async fn write_json(stream: &mut UnixStream, value: &impl Serialize) -> io::Result<()> {
    let mut bytes = serde_json::to_vec(value).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "control response serialization failed",
        )
    })?;
    if bytes.len() >= MAX_MESSAGE_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "control message exceeds size limit",
        ));
    }
    bytes.push(b'\n');
    stream.write_all(&bytes).await?;
    stream.shutdown().await
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use std::os::unix::fs::{PermissionsExt, symlink};
    use std::task::Poll;

    fn domain() -> (tempfile::TempDir, PathBuf, LocalAccess) {
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("control");
        let access = LocalAccess::new(root.path().metadata().unwrap().uid());
        (root, directory, access)
    }

    fn private_file(path: &Path) {
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(path)
            .unwrap();
    }

    #[tokio::test]
    async fn endpoint_owner_is_exclusive_and_normal_cleanup_preserves_lock() {
        let (_root, directory, access) = domain();
        let endpoint = LocalEndpoint::bind(&directory, &access).unwrap();
        endpoint.check_current().unwrap();
        assert!(matches!(LocalEndpoint::bind(&directory, &access),
            Err(error) if error.kind() == io::ErrorKind::AddrInUse));
        let (accepted, client) =
            tokio::join!(endpoint.accept(), UnixStream::connect(endpoint.path()));
        drop(accepted.unwrap());
        drop(client.unwrap());
        drop(endpoint);
        assert!(!directory.join("control.sock").exists());
        assert!(directory.join("owner.lock").is_file());
        drop(LocalEndpoint::bind(&directory, &access).unwrap());
    }

    #[tokio::test]
    async fn replaced_owner_remains_fenced_after_original_path_restored() {
        let (_root, directory, access) = domain();
        let endpoint = LocalEndpoint::bind(&directory, &access).unwrap();
        let owner = directory.join("owner.lock");
        let saved = directory.join("saved-owner");
        std::fs::rename(&owner, &saved).unwrap();
        private_file(&owner);
        assert!(endpoint.check_current().is_err());
        std::fs::remove_file(&owner).unwrap();
        std::fs::rename(&saved, &owner).unwrap();
        assert!(endpoint.check_current().is_err());
        assert!(
            tokio::time::timeout(Duration::from_millis(100), endpoint.accept())
                .await
                .unwrap()
                .is_err()
        );
        drop(endpoint);
        assert!(directory.join("control.sock").exists());
    }

    #[tokio::test]
    async fn replaced_directory_does_not_clean_either_restored_or_foreign_socket() {
        let (root, directory, access) = domain();
        let endpoint = LocalEndpoint::bind(&directory, &access).unwrap();
        let saved = root.path().join("saved");
        let replacement = root.path().join("replacement");
        std::fs::rename(&directory, &saved).unwrap();
        let foreign = LocalEndpoint::bind(&directory, &access).unwrap();
        assert!(endpoint.check_current().is_err());
        std::fs::rename(&directory, &replacement).unwrap();
        std::fs::rename(&saved, &directory).unwrap();
        assert!(endpoint.check_current().is_err());
        drop(endpoint);
        assert!(directory.join("control.sock").exists());
        drop(foreign);
        assert!(replacement.join("control.sock").exists());
    }

    #[tokio::test]
    async fn replaced_socket_is_not_removed_by_old_owner() {
        let (_root, directory, access) = domain();
        let endpoint = LocalEndpoint::bind(&directory, &access).unwrap();
        std::fs::remove_file(endpoint.path()).unwrap();
        let foreign = UnixListener::bind(endpoint.path()).unwrap();
        assert!(endpoint.check_current().is_err());
        drop(endpoint);
        assert!(directory.join("control.sock").exists());
        drop(foreign);
    }

    #[tokio::test]
    async fn permission_loss_and_restoration_cannot_revive_endpoint() {
        for name in [None, Some("owner.lock")] {
            let (_root, directory, access) = domain();
            let endpoint = LocalEndpoint::bind(&directory, &access).unwrap();
            let path = name.map_or(directory.clone(), |name| directory.join(name));
            let old = path.metadata().unwrap().permissions();
            let changed = if name.is_some() { 0o640 } else { 0o750 };
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(changed)).unwrap();
            assert!(endpoint.check_current().is_err());
            std::fs::set_permissions(&path, old).unwrap();
            assert!(endpoint.check_current().is_err());
            drop(endpoint);
            assert!(directory.join("control.sock").exists());
        }
    }

    #[tokio::test]
    async fn hardlinked_owner_is_rejected_at_bind_and_after_acquisition() {
        let (root, directory, access) = domain();
        let endpoint = LocalEndpoint::bind(&directory, &access).unwrap();
        let alias = root.path().join("alias");
        std::fs::hard_link(directory.join("owner.lock"), &alias).unwrap();
        assert!(endpoint.check_current().is_err());
        drop(endpoint);
        assert!(LocalEndpoint::bind(&directory, &access).is_err());
        std::fs::remove_file(alias).unwrap();
        drop(LocalEndpoint::bind(&directory, &access).unwrap());
    }

    #[tokio::test]
    async fn bind_refuses_symlinks_and_nonprivate_or_wrong_owner_domains() {
        let (root, directory, access) = domain();
        std::fs::create_dir(&directory).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let alias = root.path().join("alias");
        symlink(&directory, &alias).unwrap();
        assert!(LocalEndpoint::bind(&alias, &access).is_err());
        let foreign = root.path().join("foreign");
        private_file(&foreign);
        symlink(&foreign, directory.join("owner.lock")).unwrap();
        assert!(LocalEndpoint::bind(&directory, &access).is_err());
        assert!(open_owner(&File::open(&directory).unwrap()).is_err());
        std::fs::remove_file(directory.join("owner.lock")).unwrap();
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o750)).unwrap();
        assert!(LocalEndpoint::bind(&directory, &access).is_err());
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).unwrap();
        let wrong_uid = LocalAccess::new(access.operator_uid.wrapping_add(1));
        assert!(LocalEndpoint::bind(&directory, &wrong_uid).is_err());
    }

    #[tokio::test]
    async fn pending_accept_rechecks_owner_before_returning_connected_stream() {
        let (_root, directory, access) = domain();
        let endpoint = LocalEndpoint::bind(&directory, &access).unwrap();
        let mut accepted = Box::pin(endpoint.accept());
        std::future::poll_fn(|context| {
            assert!(accepted.as_mut().poll(context).is_pending());
            Poll::Ready(())
        })
        .await;
        std::fs::rename(directory.join("owner.lock"), directory.join("saved-owner")).unwrap();
        private_file(&directory.join("owner.lock"));
        let _client = UnixStream::connect(endpoint.path()).await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_secs(2), accepted)
                .await
                .unwrap()
                .is_err()
        );
        assert!(endpoint.check_current().is_err());
    }

    #[tokio::test(start_paused = true)]
    async fn idle_accept_observes_owner_loss_without_an_incoming_connection() {
        let (_root, directory, access) = domain();
        let endpoint = LocalEndpoint::bind(&directory, &access).unwrap();
        let mut accepted = Box::pin(endpoint.accept());
        std::future::poll_fn(|context| {
            assert!(accepted.as_mut().poll(context).is_pending());
            Poll::Ready(())
        })
        .await;
        std::fs::rename(directory.join("owner.lock"), directory.join("saved-owner")).unwrap();
        tokio::time::advance(Duration::from_secs(1)).await;
        assert!(accepted.await.is_err());
    }

    #[tokio::test]
    async fn cleanup_is_anchored_to_directory_when_its_path_is_replaced() {
        let (root, directory, access) = domain();
        let endpoint = LocalEndpoint::bind(&directory, &access).unwrap();
        let saved = root.path().join("saved");
        std::fs::rename(&directory, &saved).unwrap();
        std::fs::create_dir(&directory).unwrap();
        let foreign = directory.join("control.sock");
        std::fs::write(&foreign, b"foreign directory must be untouched").unwrap();
        // Simulate replacement after the final pathname check but before the
        // deletion syscall: only the retained directory may be modified.
        unlink_socket(&endpoint.directory_file).unwrap();
        assert!(!saved.join("control.sock").exists());
        assert_eq!(
            std::fs::read(foreign).unwrap(),
            b"foreign directory must be untouched"
        );
    }

    #[tokio::test]
    #[allow(unsafe_code)]
    async fn retained_descriptors_are_close_on_exec() {
        let (_root, directory, access) = domain();
        let endpoint = LocalEndpoint::bind(&directory, &access).unwrap();
        for file in [&endpoint.directory_file, &endpoint.owner] {
            // SAFETY: F_GETFD reads flags from the live borrowed file only.
            let flags = unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) };
            assert!(flags >= 0 && flags & libc::FD_CLOEXEC != 0);
        }
    }
}
