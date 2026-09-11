//! SSH shim lifecycle: per-launch listener setup and teardown.
//!
//! When a workload emits an SSH policy with at least one grant,
//! [`ensure_ssh_shim`] reserves the sandbox's transport CID before VM creation,
//! binds its launch-specific socket, and serves divert decisions on a background
//! thread. Explicit finalization releases the reservation only after listener
//! closure and every retained relay join. Retirement also joins that finalizing
//! loop. Drop alone cancels without authorizing release or proving retirement.
//! Strict-only confinement emits a policy but
//! binds no listener — nothing can divert to it, so it takes no handle.

use crate::microsandbox::broker::audit::AuditLog;
use crate::microsandbox::broker::epoch::EpochToken;
use crate::microsandbox::broker::epoch_provision::{
    AgentConsoleChannel, CONSOLE_PROVISION_TIMEOUT_SECS, EpochProvisionError,
    provision_with_deadline,
};
use crate::microsandbox::broker::registry::{
    CidRegistry, broker_socket_path, broker_vm_socket_path,
};
use crate::microsandbox::broker::shim::{
    BrokerFirstRelay, FixedCidResolver, UnixSocketTransport, run_divert_until,
};
use crate::microsandbox::broker::signing::GrantStore;
use crate::microsandbox::plan::CredentialsPlan;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Deliver the launch epoch to the guest over the console agent channel:
/// bump the CID's persisted wire sequence (every emission goes through the
/// bump, so a value is never sent twice even across a failed send followed
/// by a retry), dial the sandbox's agent relay, gate on the negotiated
/// generation, send the provision, and check the ack. The sandbox identity
/// for the dial and the provision payload both come from the CID's live
/// registry binding (single source of truth — the caller only names the
/// CID). Returns the provisioned wire epoch.
///
/// Unknown or tombstoned CIDs fail with [`EpochProvisionError::NotBound`]
/// before any dial — there is nothing to provision.
/// After the registry bump, one deadline bounds the asynchronous connection,
/// handshake, write and acknowledgement. A timeout consumes that wire epoch;
/// retries still bump the sequence rather than replaying it.
pub async fn provision_epoch_via_console(
    registry: &CidRegistry,
    cid: u32,
) -> Result<u64, EpochProvisionError> {
    let entry = registry
        .lookup(cid)
        .map_err(|e| EpochProvisionError::SendFailed {
            detail: format!("CID {cid} lookup failed: {e}"),
        })?
        .ok_or(EpochProvisionError::NotBound { cid })?;
    let wire_epoch =
        registry
            .bump_wire_epoch(cid)
            .map_err(|e| EpochProvisionError::SendFailed {
                detail: format!("wire epoch bump for CID {cid} failed after live lookup: {e}"),
            })?;
    let deadline = tokio::time::Instant::now()
        + std::time::Duration::from_secs(CONSOLE_PROVISION_TIMEOUT_SECS);
    provision_with_deadline(
        AgentConsoleChannel::connect(&entry.instance),
        &entry.instance,
        cid,
        wire_epoch,
        deadline,
    )
    .await
}

/// Re-issue the epoch for a live CID over the console agent channel: the
/// repeatable provision path for re-attestation and fork recovery. Bumps the
/// persisted wire sequence first, so the re-issue always supersedes — never
/// replays — the previous provision. Fails with
/// [`EpochProvisionError::NotBound`] when the CID has no live binding.
pub async fn reprovision_epoch(state_dir: &Path, cid: u32) -> Result<u64, EpochProvisionError> {
    let registry = CidRegistry::open(state_dir).map_err(|e| EpochProvisionError::SendFailed {
        detail: format!("CID registry open failed: {e}"),
    })?;
    provision_epoch_via_console(&registry, cid).await
}

/// Owns one workload's SSH shim: the bound socket, the allocated
/// transport CID, and the background divert loop. Release via
/// [`SshShimHandle::retire_until`] when the foreground service ends — the
/// socket and CID binding are relinquished only after relay joins. The divert
/// path needs no signing backend, so this type is deliberately
/// non-generic.
pub struct SshShimHandle {
    /// Host-side broker socket path (never guest-visible).
    pub socket_path: PathBuf,
    /// Transport CID attributed to this sandbox's diverted sessions.
    pub transport_cid: u64,
    /// Sandbox identity owning the CID binding (teardown).
    pub instance: String,
    /// Allocated CID (teardown).
    pub cid: u32,
    stop: Arc<AtomicBool>,
    finalize_requested: Arc<AtomicBool>,
    owner_dropped: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<std::io::Result<ShimExit>>>,
    retirement_failure: Option<std::io::Error>,
    retired: bool,
}

enum ShimExit {
    Finalized,
    CancelledWithoutFinalization,
}

impl std::fmt::Debug for SshShimHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshShimHandle")
            .field("socket_path", &self.socket_path)
            .field("instance", &self.instance)
            .field("cid", &self.cid)
            .finish_non_exhaustive()
    }
}

impl SshShimHandle {
    /// Close admission and cancel joined relays without authorizing CID/socket
    /// release. A retained owner may request retirement after it has confirmed
    /// that the old sandbox stopped. Failed stop must leave this reservation.
    pub(crate) fn fence(&self) {
        self.cancel_listener();
    }

    /// Compatibility shutdown for synchronous callers, waiting at most two
    /// seconds. Incomplete shutdown is stderr-loud without claiming retirement;
    /// explicitly requested finalization may still complete afterward.
    /// Async owners must use the non-consuming `retire_until`: blocking here
    /// can prevent a current-thread reactor from driving managed relay cleanup.
    pub fn shutdown(mut self) {
        self.request_shutdown();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            match self.try_retire() {
                Ok(true) => return,
                Ok(false) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                }
                _ => {
                    eprintln!(
                        "WARNING: SSH shim retirement incomplete; no successful completion observed"
                    );
                    return;
                }
            }
        }
    }

    /// Request explicit joined retirement and wake the owned loop. Filesystem
    /// finalization may complete later, but runs only after all relay joins.
    pub fn request_shutdown(&self) {
        self.finalize_requested.store(true, Ordering::Release);
        self.cancel_listener();
    }

    fn cancel_listener(&self) {
        self.stop.store(true, Ordering::Release);
        if let Some(join) = &self.join {
            join.thread().unpark();
        }
    }

    /// Non-consuming shutdown. A deadline, cancellation or failed join leaves
    /// this handle available for an honest retry. A pending finalizer may later
    /// release the reservation after actual joins; only its observed result
    /// establishes successful retirement. Drop alone never authorizes release.
    /// No sleep or join blocks the async executor while a worker is still live.
    pub async fn retire_until(&mut self, deadline: tokio::time::Instant) -> std::io::Result<()> {
        self.request_shutdown();
        loop {
            if self.try_retire()? {
                return Ok(());
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "SSH shim retirement incomplete",
                ));
            }
            tokio::time::sleep_until(
                deadline.min(tokio::time::Instant::now() + std::time::Duration::from_millis(10)),
            )
            .await;
        }
    }

    fn try_retire(&mut self) -> std::io::Result<bool> {
        if let Some(error) = &self.retirement_failure {
            return Err(std::io::Error::new(error.kind(), error.to_string()));
        }
        if self.retired {
            return Ok(true);
        }
        if self.join.as_ref().is_some_and(|join| !join.is_finished()) {
            return Ok(false);
        }
        let result = match self.join.take() {
            Some(join) => match join.join() {
                Ok(Ok(ShimExit::Finalized)) => {
                    self.retired = true;
                    return Ok(true);
                }
                Ok(Ok(ShimExit::CancelledWithoutFinalization)) => {
                    std::io::Error::other("SSH shim cancelled without finalization")
                }
                Ok(Err(error)) => error,
                Err(_) => std::io::Error::other("SSH shim retirement thread panicked"),
            },
            None => std::io::Error::other("SSH shim retirement owner is missing"),
        };
        let returned = std::io::Error::new(result.kind(), result.to_string());
        self.retirement_failure = Some(result);
        Err(returned)
    }
}

impl Drop for SshShimHandle {
    fn drop(&mut self) {
        // Never block the async owner here or infer retirement from a dropped
        // JoinHandle. The loop still owns and cancels its relay workers; without
        // an explicit finalization request, the CID and exact socket remain
        // reserved. Drop does not undo finalization already explicitly requested.
        self.owner_dropped.store(true, Ordering::Release);
        self.cancel_listener();
    }
}

// Runs only on the original owned loop thread, after every relay join and
// listener close. Fencing does not abandon this finalization owner. No new
// thread or blocking registry work is moved onto the async caller.
fn await_finalization_intent(finalize: &AtomicBool, owner_dropped: &AtomicBool) -> bool {
    loop {
        if finalize.load(Ordering::Acquire) {
            return true;
        }
        if owner_dropped.load(Ordering::Acquire) {
            return false;
        }
        std::thread::park();
    }
}

// The state directory is host-owned. Preserve a replacement at this pathname
// during normal lifecycle races; this is not a defense against a hostile host UID.
fn remove_owned_socket(path: &Path, identity: (u64, u64)) -> std::io::Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if (metadata.dev(), metadata.ino()) == identity {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Runs on the existing owned loop thread, only after successful relay joins
/// and listener closure. Preserve both reservations while the registry lock is
/// contended. Filesystem operations are not transactional: an unlink failure
/// after successful CID expiry remains an explicit finalization error.
fn finalize_shim(
    registry: &CidRegistry,
    cid: u32,
    instance: &str,
    epoch: &EpochToken,
    socket_path: &Path,
    socket_identity: (u64, u64),
) -> std::io::Result<ShimExit> {
    registry
        .expire_binding(cid, instance, epoch)
        .map_err(std::io::Error::other)?;
    if let Err(error) = remove_owned_socket(socket_path, socket_identity)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        return Err(error);
    }
    Ok(ShimExit::Finalized)
}

/// Set up the SSH shim for one workload launch, or `None` when the
/// workload carries no SSH grants (strict-only confinement emits a policy
/// but binds no listener — nothing can divert to it).
///
/// Call before VM creation and pass the returned CID and endpoint together to
/// the SDK. On cancellation Drop fences the listener; only explicit joined
/// retirement releases the reservation. Failed launch owners must retain the
/// handle to await cleanup rather than equating cancellation with completion.
/// This prepares host dispatch only: it does not install policy in brokerd or
/// establish broker readiness. A workload's console is not the broker console.
pub async fn ensure_ssh_shim(
    state_dir: &Path,
    instance: &str,
    credentials: &CredentialsPlan,
) -> anyhow::Result<Option<SshShimHandle>> {
    if credentials.ssh.is_empty() {
        return Ok(None);
    }
    // The issue-time CID is documentation only (the token is opaque
    // entropy); the real (instance, CID) binding persists at allocate.
    let epoch = EpochToken::issue(instance, 0)
        .map_err(|e| anyhow::anyhow!("ssh shim: epoch issue failed: {e}"))?;
    let registry = Arc::new(CidRegistry::open(state_dir)?);
    let cid = registry.allocate(instance, &epoch)?;
    // A dedicated endpoint binds this host network context to its reservation.
    // A second launch cannot overwrite the first launch's listener or identity.
    let socket_path = broker_socket_path(state_dir).with_file_name(format!("divert-{cid}.sock"));
    let transport = match UnixSocketTransport::bind_fresh(&socket_path, FixedCidResolver(cid)) {
        Ok(transport) => transport,
        Err(error) => {
            registry.expire_binding(cid, instance, &epoch)?;
            return Err(anyhow::anyhow!(
                "ssh shim: socket bind failed for {}: {error}",
                socket_path.display()
            ));
        }
    };
    let setup = (|| {
        let metadata = std::fs::symlink_metadata(&socket_path)?;
        let identity = (metadata.dev(), metadata.ino());
        if let Err(error) = transport.set_nonblocking() {
            let _ = remove_owned_socket(&socket_path, identity);
            return Err(error);
        }
        Ok::<_, std::io::Error>(identity)
    })();
    let socket_identity = match setup {
        Ok(identity) => identity,
        Err(error) => {
            registry.expire_binding(cid, instance, &epoch)?;
            return Err(anyhow::anyhow!("ssh shim: listener setup failed: {error}"));
        }
    };
    let grants = GrantStore::compile(&[(instance, credentials)]);
    let audit = Arc::new(AuditLog::with_state_dir(state_dir));
    let stop = Arc::new(AtomicBool::new(false));
    let finalize_requested = Arc::new(AtomicBool::new(false));
    let owner_dropped = Arc::new(AtomicBool::new(false));
    // Custody-first relay: broker-bound sessions ride the broker VM socket
    // (fail-closed while it is absent), guest-bound sessions relay direct.
    let broker_socket = broker_vm_socket_path(state_dir);
    let join = std::thread::Builder::new()
        .name(format!("ssh-shim-{instance}"))
        .spawn({
            let loop_registry = Arc::clone(&registry);
            let loop_stop = Arc::clone(&stop);
            let loop_audit = Arc::clone(&audit);
            let loop_finalize = Arc::clone(&finalize_requested);
            let loop_owner_dropped = Arc::clone(&owner_dropped);
            let loop_instance = instance.to_owned();
            let loop_epoch = epoch.clone();
            let loop_path = socket_path.clone();
            move || {
                let relay = BrokerFirstRelay::new(broker_socket);
                run_divert_until(
                    &transport,
                    &loop_registry,
                    &grants,
                    &loop_audit,
                    relay,
                    &loop_stop,
                )?;
                // No accepted listener survives reservation release. Failed
                // worker joins return above and skip filesystem finalization.
                drop(transport);
                if await_finalization_intent(&loop_finalize, &loop_owner_dropped) {
                    finalize_shim(
                        &loop_registry,
                        cid,
                        &loop_instance,
                        &loop_epoch,
                        &loop_path,
                        socket_identity,
                    )
                } else {
                    Ok(ShimExit::CancelledWithoutFinalization)
                }
            }
        });
    let join = match join {
        Ok(join) => join,
        Err(error) => {
            let cleanup = remove_owned_socket(&socket_path, socket_identity);
            registry.expire_binding(cid, instance, &epoch)?;
            cleanup?;
            return Err(anyhow::anyhow!(
                "ssh shim: divert thread spawn failed: {error}"
            ));
        }
    };
    Ok(Some(SshShimHandle {
        socket_path,
        transport_cid: u64::from(cid),
        instance: instance.to_string(),
        cid,
        stop,
        finalize_requested,
        owner_dropped,
        join: Some(join),
        retirement_failure: None,
        retired: false,
    }))
}

#[cfg(test)]
#[path = "ssh_lifecycle_owned_tests.rs"]
mod owned_tests;

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::microsandbox::plan::CredentialBinding;
    use std::io::{Read, Write};

    fn ssh_credentials() -> CredentialsPlan {
        CredentialsPlan {
            ssh: vec![crate::microsandbox::plan::SshGrantPlan {
                name: "deploy".to_string(),
                material: "DEPLOY_KEY".to_string(),
                hosts: vec!["github.com".to_string()],
                users: vec!["git".to_string()],
                ports: vec![22],
                binding: CredentialBinding::Broker,
                on_violation: crate::config::SecretViolationPolicy::Passthrough,
            }],
            signing: Vec::new(),
            strict: false,
            strict_origin: None,
        }
    }

    #[tokio::test]
    async fn reprovision_unknown_cid_fails_closed_without_dialing() {
        use crate::microsandbox::broker::epoch_provision::EpochProvisionError;
        let dir = crate::config::test_support::unique_state_dir("ssh-reprov-unknown");
        // No binding exists: typed rejection before any console dial (there
        // is no sandbox here at all — this must not hang or dial).
        let err = reprovision_epoch(&dir, 4242).await.unwrap_err();
        assert_eq!(err, EpochProvisionError::NotBound { cid: 4242 });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn ensure_returns_none_without_ssh_grants() {
        let dir = crate::config::test_support::unique_state_dir("ssh-shim-none");
        let strict_only = CredentialsPlan {
            ssh: Vec::new(),
            signing: Vec::new(),
            strict: true,
            strict_origin: None,
        };
        assert!(
            ensure_ssh_shim(&dir, "personal-pi", &strict_only)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            !broker_socket_path(&dir).exists(),
            "strict-only workloads bind no listener"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn concurrent_launches_have_distinct_owned_endpoints_and_cids() {
        let dir = crate::config::test_support::unique_state_dir("ssh-broker-socket");
        let credentials = ssh_credentials();
        let mut first = ensure_ssh_shim(&dir, "first", &credentials)
            .await
            .unwrap()
            .unwrap();
        let second = ensure_ssh_shim(&dir, "second", &credentials)
            .await
            .unwrap()
            .unwrap();
        assert_ne!(first.cid, second.cid);
        assert_ne!(first.socket_path, second.socket_path);
        let first_path = first.socket_path.clone();
        let first_cid = first.cid;
        first
            .retire_until(tokio::time::Instant::now() + std::time::Duration::from_secs(2))
            .await
            .unwrap();
        assert!(!first_path.exists());
        assert!(second.socket_path.exists());
        let registry = CidRegistry::open(&dir).unwrap();
        assert!(registry.lookup(first_cid).unwrap().is_none());
        assert_eq!(
            registry.lookup(second.cid).unwrap().unwrap().instance,
            "second"
        );
        second.shutdown();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn ensure_binds_socket_and_registry_then_shutdown_releases() {
        let dir = crate::config::test_support::unique_state_dir("ssh-shim-life");
        let credentials = ssh_credentials();
        let handle = ensure_ssh_shim(&dir, "personal-pi", &credentials)
            .await
            .unwrap()
            .expect("grants bind a shim");
        let socket_path = handle.socket_path.clone();
        assert_ne!(socket_path, broker_socket_path(&dir));
        assert!(handle.socket_path.exists(), "listener socket file exists");
        assert_eq!(handle.transport_cid, u64::from(handle.cid));
        // The registry lookup resolves the allocated CID to the instance.
        let registry = CidRegistry::open(&dir).unwrap();
        let entry = registry
            .lookup(handle.cid)
            .unwrap()
            .expect("CID must be bound");
        assert_eq!(entry.instance, "personal-pi");
        // The overlay helper carries the same workload's SSH policy.
        let builder = crate::microsandbox::broker::apply_ssh_policy(
            microsandbox::Sandbox::builder("ssh-shim-life"),
            Some(&credentials),
        );
        let ssh = builder
            .spec()
            .network
            .ssh
            .clone()
            .expect("overlay must carry the ssh policy");
        assert_eq!(ssh.grants.len(), 1);
        assert!(!ssh.strict);
        // Shutdown removes the socket and expires the binding.
        let cid = handle.cid;
        handle.shutdown();
        assert!(!socket_path.exists(), "shutdown removes the socket");
        let registry = CidRegistry::open(&dir).unwrap();
        assert!(
            registry.lookup(cid).unwrap().is_none(),
            "shutdown expires the binding"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn preboot_setup_does_not_provision_the_workload_console() {
        // There is no workload VM yet. Host setup reserves identity without
        // pretending its console is the shared broker's control channel.
        let dir = crate::config::test_support::unique_state_dir("ssh-shim-nocons");
        let credentials = ssh_credentials();
        let handle = ensure_ssh_shim(&dir, "personal-pi", &credentials)
            .await
            .unwrap()
            .expect("host setup does not require an already-running workload");
        assert!(handle.socket_path.exists(), "listener still binds");
        let registry = CidRegistry::open(&dir).unwrap();
        assert_eq!(registry.lookup(handle.cid).unwrap().unwrap().wire_epoch, 0);
        assert!(
            registry.lookup(handle.cid).unwrap().is_some(),
            "CID still binds"
        );
        let cid = handle.cid;
        handle.shutdown();
        assert!(
            CidRegistry::open(&dir)
                .unwrap()
                .lookup(cid)
                .unwrap()
                .is_none()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn ensure_refuses_an_existing_launch_endpoint_without_overwriting_it() {
        let dir = crate::config::test_support::unique_state_dir("ssh-shim-stale");
        let cid = crate::microsandbox::broker::registry::CID_ALLOC_BASE;
        let socket_path = broker_socket_path(&dir).with_file_name(format!("divert-{cid}.sock"));
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&socket_path, b"stale").unwrap();
        let credentials = ssh_credentials();
        assert!(
            ensure_ssh_shim(&dir, "personal-pi", &credentials)
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(&socket_path).unwrap(), b"stale");
        assert!(
            CidRegistry::open(&dir)
                .unwrap()
                .lookup(cid)
                .unwrap()
                .is_none()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn shutdown_without_socket_file_is_quiet() {
        // Shutdown when the socket file is already gone (crashed listener,
        // double teardown): no panic, and the CID binding still expires.
        let dir = crate::config::test_support::unique_state_dir("ssh-shim-nosock");
        let credentials = ssh_credentials();
        let handle = ensure_ssh_shim(&dir, "personal-pi", &credentials)
            .await
            .unwrap()
            .expect("grants bind a shim");
        let cid = handle.cid;
        std::fs::remove_file(&handle.socket_path).unwrap();
        shutdown_with_deadline(handle);
        let registry = CidRegistry::open(&dir).unwrap();
        assert!(
            registry.lookup(cid).unwrap().is_none(),
            "shutdown expires the binding even with the socket already gone"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn shutdown_with_deadline(handle: SshShimHandle) {
        let (done, wait) = std::sync::mpsc::channel();
        let worker = std::thread::spawn(move || {
            handle.shutdown();
            let _ = done.send(());
        });
        wait.recv_timeout(std::time::Duration::from_secs(2))
            .expect("listener shutdown must not depend on its socket pathname");
        worker.join().unwrap();
    }

    #[tokio::test]
    async fn shutdown_preserves_a_replacement_at_the_old_socket_path() {
        let dir = crate::config::test_support::unique_state_dir("ssh-shim-replaced-socket");
        let handle = ensure_ssh_shim(&dir, "personal-pi", &ssh_credentials())
            .await
            .unwrap()
            .unwrap();
        let path = handle.socket_path.clone();
        let cid = handle.cid;
        // Keep the old socket inode alive so allocation cannot reuse it.
        let retired = path.with_extension("retired");
        std::fs::rename(&path, &retired).unwrap();
        let replacement = std::os::unix::net::UnixListener::bind(&path).unwrap();
        replacement.set_nonblocking(true).unwrap();
        shutdown_with_deadline(handle);
        assert!(path.exists(), "old owner must not unlink the replacement");
        assert_eq!(
            replacement.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock,
            "old owner must not dial the replacement to wake itself"
        );
        assert!(
            CidRegistry::open(&dir)
                .unwrap()
                .lookup(cid)
                .unwrap()
                .is_none()
        );
        drop(replacement);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn cancelling_a_pending_launch_fences_but_does_not_prove_retirement() {
        let dir = crate::config::test_support::unique_state_dir("ssh-shim-cancel-launch");
        let launch_dir = dir.clone();
        let (ready, wait) = tokio::sync::oneshot::channel();
        let launch = tokio::spawn(async move {
            let handle = ensure_ssh_shim(&launch_dir, "personal-pi", &ssh_credentials())
                .await
                .unwrap()
                .unwrap();
            ready
                .send((handle.socket_path.clone(), handle.cid))
                .unwrap();
            std::future::pending::<()>().await;
            drop(handle);
        });
        let (path, cid) = wait.await.unwrap();
        assert!(path.exists());
        launch.abort();
        assert!(launch.await.unwrap_err().is_cancelled());
        assert!(
            path.exists(),
            "drop must retain the exact socket reservation"
        );
        assert!(
            CidRegistry::open(&dir)
                .unwrap()
                .lookup(cid)
                .unwrap()
                .is_some()
        );
        // Observe listener closure, not a historical joined-worker proof. The
        // reservation deliberately remains unavailable without the owner.
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            while std::os::unix::net::UnixStream::connect(&path).is_ok() {
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// SSH grants covering one loopback TCP port (the upstream stub below).
    /// Guest-bound: the key lives in the guest, so the divert thread relays
    /// these sessions direct to the granted upstream.
    fn loopback_credentials(port: u16) -> CredentialsPlan {
        CredentialsPlan {
            ssh: vec![crate::microsandbox::plan::SshGrantPlan {
                name: "stub".to_string(),
                material: "TEST_KEY".to_string(),
                hosts: vec!["127.0.0.1".to_string()],
                users: vec!["git".to_string()],
                ports: vec![port],
                binding: CredentialBinding::Guest,
                on_violation: crate::config::SecretViolationPolicy::Passthrough,
            }],
            signing: Vec::new(),
            strict: false,
            strict_origin: None,
        }
    }

    /// Broker-bound grants covering one loopback TCP port. The key lives in
    /// broker custody, so these sessions must ride the broker VM socket.
    fn broker_bound_credentials(port: u16) -> CredentialsPlan {
        CredentialsPlan {
            ssh: vec![crate::microsandbox::plan::SshGrantPlan {
                name: "stub".to_string(),
                material: "TEST_KEY".to_string(),
                hosts: vec!["127.0.0.1".to_string()],
                users: vec!["git".to_string()],
                ports: vec![port],
                binding: CredentialBinding::Broker,
                on_violation: crate::config::SecretViolationPolicy::Passthrough,
            }],
            signing: Vec::new(),
            strict: false,
            strict_origin: None,
        }
    }

    /// The divert thread the launch binds hands allowed guest-bound sessions
    /// to the direct TCP relay: a full guest→shim→stub round trip through
    /// [`ensure_ssh_shim`], ending with exactly one persisted allow record.
    /// Broker-bound sessions never take this path (they ride the broker VM
    /// socket instead — see below).
    #[tokio::test]
    async fn ensure_divert_thread_relays_allowed_sessions_to_the_granted_upstream() {
        use microsandbox_network::ssh::gateway::{SshDivertPrelude, encode_ssh_divert_prelude};

        // The granted upstream: a local TCP stub speaking fixed banners.
        // Nonblocking accept with a deadline so a never-dialing relay
        // fails the test instead of hanging it.
        let stub = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let stub_port = stub.local_addr().unwrap().port();
        stub.set_nonblocking(true).unwrap();
        let stub_worker = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            let (mut conn, _) = loop {
                match stub.accept() {
                    Ok(v) => break v,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if std::time::Instant::now() > deadline {
                            panic!("upstream stub accepted nothing: the relay never dialed");
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => panic!("upstream stub accept failed: {e}"),
                }
            };
            conn.set_read_timeout(Some(std::time::Duration::from_secs(15)))
                .unwrap();
            let mut banner = [0u8; 9];
            conn.read_exact(&mut banner).unwrap();
            assert_eq!(&banner, b"SSH-GUEST");
            conn.write_all(b"SSH-STUB!").unwrap();
            let mut rest = Vec::new();
            conn.read_to_end(&mut rest).unwrap();
            assert!(rest.is_empty(), "guest must send nothing after its banner");
        });

        let dir = crate::config::test_support::unique_state_dir("ssh-relay-wire");
        let credentials = loopback_credentials(stub_port);
        let handle = ensure_ssh_shim(&dir, "personal-pi", &credentials)
            .await
            .unwrap()
            .expect("grants bind a shim");
        let socket_path = handle.socket_path.clone();
        let cid = handle.cid;

        // Guest side: connect to the shim socket and divert to the stub.
        let mut guest = std::os::unix::net::UnixStream::connect(&socket_path).unwrap();
        guest
            .set_read_timeout(Some(std::time::Duration::from_secs(15)))
            .unwrap();
        let prelude = SshDivertPrelude {
            dest_host: "127.0.0.1".to_string(),
            dest_port: stub_port,
            transport_cid: u64::from(cid),
            epoch: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        guest
            .write_all(&encode_ssh_divert_prelude(&prelude))
            .unwrap();
        // Post-prelude bytes reach the upstream through the relay, and the
        // upstream answer flows back: the second banner of a diverted session.
        guest.write_all(b"SSH-GUEST").unwrap();
        let mut answer = [0u8; 9];
        guest.read_exact(&mut answer).unwrap();
        assert_eq!(&answer, b"SSH-STUB!");
        // The guest half-close propagates to the stub; the stub close
        // surfaces here as EOF.
        guest.shutdown(std::net::Shutdown::Write).unwrap();
        let mut tail = Vec::new();
        guest.read_to_end(&mut tail).unwrap();
        assert!(tail.is_empty(), "upstream close must surface as guest EOF");
        stub_worker.join().unwrap();

        handle.shutdown();

        // Relay establishment audits exactly one allow record: the handoff
        // from decision to relay is observable in the persisted log.
        let log =
            std::fs::read_to_string(crate::microsandbox::broker::audit::audit_file_path(&dir))
                .unwrap();
        let records: Vec<serde_json::Value> = log
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            records.len(),
            1,
            "one divert decision per session: {records:?}"
        );
        assert_eq!(records[0]["instance"], "personal-pi");
        assert_eq!(
            records[0]["key_id"],
            format!("127.0.0.1:{stub_port}").as_str()
        );
        assert_eq!(records[0]["scheme"], "ssh-divert");
        assert_eq!(records[0]["namespace"], "divert");
        assert_eq!(records[0]["result"], "allow");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Broker-bound sessions fail closed while no broker VM serves its
    /// socket: the decision still allows (exactly one persisted allow
    /// record), but the custody relay cannot dial and the guest observes
    /// EOF — never a direct-TCP dial to the granted upstream.
    #[tokio::test]
    async fn ensure_divert_thread_fails_broker_bound_sessions_closed_without_broker() {
        use microsandbox_network::ssh::gateway::{SshDivertPrelude, encode_ssh_divert_prelude};

        // No TCP stub listens here on purpose: any direct dial would refuse
        // loudly, but the custody path must not dial at all — the guest
        // must see EOF from the dropped relay stream.
        let closed = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = closed.local_addr().unwrap().port();
        drop(closed);

        let dir = crate::config::test_support::unique_state_dir("ssh-relay-nobroker");
        // No broker VM socket exists under this state dir.
        assert!(
            !crate::microsandbox::broker::registry::broker_vm_socket_path(&dir).exists(),
            "the test needs no broker VM running"
        );
        let credentials = broker_bound_credentials(port);
        let handle = ensure_ssh_shim(&dir, "personal-pi", &credentials)
            .await
            .unwrap()
            .expect("grants bind a shim");
        let socket_path = handle.socket_path.clone();
        let cid = handle.cid;

        let mut guest = std::os::unix::net::UnixStream::connect(&socket_path).unwrap();
        guest
            .set_read_timeout(Some(std::time::Duration::from_secs(20)))
            .unwrap();
        let prelude = SshDivertPrelude {
            dest_host: "127.0.0.1".to_string(),
            dest_port: port,
            transport_cid: u64::from(cid),
            epoch: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        guest
            .write_all(&encode_ssh_divert_prelude(&prelude))
            .unwrap();
        // The custody relay drops the session: EOF, not upstream bytes.
        let mut tail = Vec::new();
        guest.read_to_end(&mut tail).unwrap();
        assert!(
            tail.is_empty(),
            "broker-bound session without a broker must close"
        );

        // Give the worker a moment to finish its audit write, then shut down.
        std::thread::sleep(std::time::Duration::from_millis(200));
        handle.shutdown();

        let log =
            std::fs::read_to_string(crate::microsandbox::broker::audit::audit_file_path(&dir))
                .unwrap();
        let records: Vec<serde_json::Value> = log
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            records.len(),
            1,
            "one divert decision per session: {records:?}"
        );
        assert_eq!(records[0]["result"], "allow");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
