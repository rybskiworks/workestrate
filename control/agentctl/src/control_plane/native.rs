//! Internal translation seam to the selected native runtime, not another
//! backend registry. Native implementations retain their own connection/session
//! leases; Workestrate owns permissions and the public operation references.

use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use microsandbox::sandbox::SandboxStatus;
use microsandbox::sandbox::exec::{ExecOptionsBuilder, ExecSink};
use microsandbox::{ExecControl, ExecHandle, MicrosandboxError, Sandbox};
use tokio::sync::mpsc::error::TryRecvError;

use super::types::{
    ControlError, ExecCommand, ExecEvent, ExecInterruptionReason, ExecTermination, InstanceRef,
    LaunchRef, LifecycleOperation, LifecycleState, OpaqueId, RuntimeCapabilities,
};

pub(crate) type NativeFuture<'a, T> =
    Pin<Box<dyn Future<Output = Result<T, ControlError>> + Send + 'a>>;

/// A retained, exact-launch native execution lease. No method reconnects by a
/// reusable name or reuses a protocol request slot after this lease terminates.
pub(crate) trait NativeExec: Send {
    /// Nonblocking drain of already available events. The implementation bounds
    /// its producer queue and retains any partial chunk for the next read.
    fn poll(&mut self, max_bytes: usize) -> Result<Vec<ExecEvent>, ControlError>;

    fn stdin<'a>(&'a mut self, bytes: &'a [u8], eof: bool) -> NativeFuture<'a, ()>;
    fn resize(&mut self, rows: u16, columns: u16) -> NativeFuture<'_, ()>;
    /// Acceptance requests cancellation; only a terminal event proves exit.
    fn cancel(&mut self) -> NativeFuture<'_, ()>;
}

/// Crate-private adapter for the narrow control operations. Implementations
/// reuse the SDK's existing SandboxBackend and retained handles. There is no
/// provider registration, new guest protocol, discovery or create/replace path.
pub(crate) trait NativeControl: Send {
    fn capabilities(&self) -> RuntimeCapabilities;
    fn verify_launch<'a>(&'a self, launch: &'a LaunchRef) -> NativeFuture<'a, ()>;
    fn start<'a>(
        &'a mut self,
        launch: &'a LaunchRef,
        command: ExecCommand,
    ) -> NativeFuture<'a, Box<dyn NativeExec>>;
    fn lifecycle<'a>(
        &'a mut self,
        launch: &'a LaunchRef,
        operation: LifecycleOperation,
    ) -> NativeFuture<'a, LifecycleState>;
}

const MAX_RETAINED_LAUNCHES: usize = 128;
const MAX_POLL_BYTES: usize = 8 * 1024;
const MAX_POLL_EVENTS: usize = 64;
// The SDK's producer queue has a separate 1 MiB byte budget. At most one
// dequeued chunk is retained here, in addition to that queue and a bounded reply.
const MAX_NATIVE_CHUNK: usize = 1024 * 1024;

/// Selected SDK objects, supplied only by the trusted workload owner. This map
/// owns retained launch connections; it never discovers resources or grants
/// policy. Replacing a workload requires a new native identity and explicit
/// retention, not mutation of an existing entry or a name-based reconnect.
pub(crate) struct MicrosandboxControl {
    backend: Arc<dyn microsandbox::Backend>,
    selected: BTreeMap<LaunchRef, SelectedSandbox>,
}

struct SelectedSandbox {
    expected_name: String,
    sandbox: Sandbox,
}

impl MicrosandboxControl {
    pub(crate) fn new(backend: Arc<dyn microsandbox::Backend>) -> Self {
        Self {
            backend,
            selected: BTreeMap::new(),
        }
    }

    pub(crate) const fn supported_capabilities() -> RuntimeCapabilities {
        let supported = cfg!(target_os = "linux");
        RuntimeCapabilities {
            launch_bound_exec: supported,
            stdin: supported,
            pty: supported,
            cancellation: supported,
            launch_bound_lifecycle: supported,
        }
    }

    /// The caller resolves the Workestrate instance and expected SDK name from
    /// trusted workload state, never from an unverified guest request. No new
    /// backend lookup is performed. Application readiness remains separate.
    pub(crate) async fn retain(
        &mut self,
        instance: InstanceRef,
        expected_name: &str,
        sandbox: Sandbox,
    ) -> Result<LaunchRef, ControlError> {
        if !self.capabilities().launch_bound_exec {
            return Err(ControlError::UnsupportedCapability);
        }
        validate_association(
            Arc::ptr_eq(&self.backend, sandbox.backend()),
            expected_name,
            sandbox.name(),
        )?;
        let launch = LaunchRef {
            instance,
            generation: OpaqueId::from_bytes(
                *sandbox.launch_identity().map_err(native_error)?.as_bytes(),
            ),
        };
        admit_selection(&self.selected, &launch)?;
        // This is a generation-fenced refresh, not get-by-name reconciliation.
        // Terminal state is valid for draining a retained session. Missing
        // ephemeral rows cannot be validated by this SDK API and fail closed.
        sandbox.status().await.map_err(native_error)?;
        self.selected.insert(
            launch.clone(),
            SelectedSandbox {
                expected_name: expected_name.to_owned(),
                sandbox,
            },
        );
        Ok(launch)
    }

    /// Retain the selected SDK object and the already captured trusted route.
    /// This owns a connection capability, not authority to create or stop a VM.
    /// The route is never recaptured or resolved from a public control request.
    #[cfg(unix)]
    pub(crate) fn retain_broker_route(
        &self,
        launch: &LaunchRef,
        route: ProtectedHostListener,
    ) -> Result<Arc<RetainedBrokerRoute>, ControlError> {
        route.check_current(launch)?;
        let sandbox = self.selected(launch)?;
        let peer_pid = sandbox
            .local()
            .and_then(|local| local.client.peer_pid())
            .filter(|pid| *pid != 0)
            .ok_or(ControlError::UnsupportedCapability)?;
        Ok(Arc::new(RetainedBrokerRoute {
            launch: launch.clone(),
            route,
            backend: Arc::clone(&self.backend),
            expected_name: sandbox.name().to_owned(),
            sandbox: sandbox.clone(),
            peer_pid,
        }))
    }

    /// Capture the trusted owner's intended route only after the runtime has
    /// bound it. The owner retains this token for every subsequent connection;
    /// it must not recapture a replacement endpoint for an issued launch.
    #[cfg(unix)]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "broker bootstrap does not yet supply a newly bound management listener to the host owner"
        )
    )]
    pub(crate) async fn capture_host_listener(
        &self,
        launch: &LaunchRef,
        intended_path: &std::path::Path,
        deadline: tokio::time::Instant,
    ) -> Result<ProtectedHostListener, ControlError> {
        if !cfg!(target_os = "linux") {
            return Err(ControlError::UnsupportedCapability);
        }
        listener_deadline(deadline)?;
        tokio::time::timeout_at(deadline, async {
            let peer = self.running_peer(launch).await?;
            let route = ProtectedHostListener::capture(launch.clone(), intended_path)?;
            if self.running_peer(launch).await? != peer {
                return Err(ControlError::StaleLaunch);
            }
            route.check_current(launch)?;
            listener_deadline(deadline)?;
            Ok(route)
        })
        .await
        .map_err(|_| ControlError::RuntimeUnavailable)?
    }

    /// Return the original connected stream only after exact-launch and kernel
    /// peer checks. No application bytes, guest exec, name lookup or replay are
    /// used. Dropping this future drops any not-yet-verified connection.
    #[cfg(unix)]
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "direct protected-listener bootstrap checks are not yet invoked by the production owner"
        )
    )]
    pub(crate) async fn connect_host_listener(
        &self,
        launch: &LaunchRef,
        route: &ProtectedHostListener,
        deadline: tokio::time::Instant,
    ) -> Result<tokio::net::UnixStream, ControlError> {
        if !cfg!(target_os = "linux") {
            return Err(ControlError::UnsupportedCapability);
        }
        connect_listener_checked(launch, route, deadline, || self.running_peer(launch)).await
    }

    #[cfg(unix)]
    async fn running_peer(&self, launch: &LaunchRef) -> Result<u32, ControlError> {
        let sandbox = self.selected(launch)?;
        let status = sandbox.status().await.map_err(native_error)?;
        running_listener_peer(
            status,
            sandbox.local().and_then(|local| local.client.peer_pid()),
        )
    }

    fn selected(&self, launch: &LaunchRef) -> Result<&Sandbox, ControlError> {
        if !self.capabilities().launch_bound_exec {
            return Err(ControlError::UnsupportedCapability);
        }
        let selected = self
            .selected
            .get(launch)
            .ok_or(ControlError::UnknownLaunch)?;
        let sandbox = &selected.sandbox;
        validate_association(
            Arc::ptr_eq(&self.backend, sandbox.backend()),
            &selected.expected_name,
            sandbox.name(),
        )?;
        if launch.generation
            != OpaqueId::from_bytes(*sandbox.launch_identity().map_err(native_error)?.as_bytes())
        {
            return Err(ControlError::StaleLaunch);
        }
        Ok(sandbox)
    }
}

/// One original native launch and protected listener, retained beyond setup.
/// Async health checks require Running; transport closure never implies VM exit.
#[cfg(unix)]
pub(crate) struct RetainedBrokerRoute {
    launch: LaunchRef,
    route: ProtectedHostListener,
    backend: Arc<dyn microsandbox::Backend>,
    expected_name: String,
    sandbox: Sandbox,
    peer_pid: u32,
}

#[cfg(unix)]
impl RetainedBrokerRoute {
    pub(crate) fn check_current(&self) -> Result<(), ControlError> {
        self.route.check_current(&self.launch)?;
        validate_association(
            Arc::ptr_eq(&self.backend, self.sandbox.backend()),
            &self.expected_name,
            self.sandbox.name(),
        )?;
        if self.launch.generation
            != OpaqueId::from_bytes(
                *self
                    .sandbox
                    .launch_identity()
                    .map_err(native_error)?
                    .as_bytes(),
            )
            || self
                .sandbox
                .local()
                .and_then(|local| local.client.peer_pid())
                != Some(self.peer_pid)
        {
            return Err(ControlError::StaleLaunch);
        }
        Ok(())
    }

    async fn running_peer(&self) -> Result<u32, ControlError> {
        self.check_current()?;
        let peer = running_listener_peer(
            self.sandbox.status().await.map_err(native_error)?,
            self.sandbox
                .local()
                .and_then(|local| local.client.peer_pid()),
        )?;
        self.check_current()?;
        if peer != self.peer_pid {
            return Err(ControlError::StaleLaunch);
        }
        Ok(peer)
    }

    pub(crate) async fn verify_running(
        &self,
        deadline: tokio::time::Instant,
    ) -> Result<(), ControlError> {
        listener_deadline(deadline)?;
        tokio::time::timeout_at(deadline, self.running_peer())
            .await
            .map_err(|_| ControlError::RuntimeUnavailable)??;
        listener_deadline(deadline)
    }

    pub(crate) async fn connect(
        &self,
        deadline: tokio::time::Instant,
    ) -> Result<tokio::net::UnixStream, ControlError> {
        connect_listener_checked(&self.launch, &self.route, deadline, || self.running_peer()).await
    }
}

/// Endpoint ownership is not authentication: the connector also requires the
/// retained native launch and its kernel-reported peer. Capture is crate-private
/// and accepts only a trusted builder's route, never a control request path.
/// Checks detect observed namespace changes; they do not exclude a hostile
/// same-UID writer atomically swapping and restoring a socket between checks.
#[cfg(unix)]
pub(crate) struct ProtectedHostListener {
    launch: LaunchRef,
    path: std::path::PathBuf,
    parent: std::path::PathBuf,
    directory: std::fs::File,
    directory_identity: (u64, u64, u32, u32),
    socket_identity: (u64, u64, u32),
    lost: std::sync::atomic::AtomicBool,
}

#[cfg(unix)]
impl ProtectedHostListener {
    fn capture(launch: LaunchRef, path: &std::path::Path) -> Result<Self, ControlError> {
        use std::os::unix::fs::OpenOptionsExt;
        if !cfg!(target_os = "linux") {
            return Err(ControlError::UnsupportedCapability);
        }
        if !path.is_absolute()
            || path.file_name().is_none()
            || path.components().any(|component| {
                !matches!(
                    component,
                    std::path::Component::RootDir | std::path::Component::Normal(_)
                )
            })
        {
            return Err(ControlError::InvalidRequest);
        }
        let parent = path
            .parent()
            .ok_or(ControlError::InvalidRequest)?
            .to_owned();
        if std::fs::canonicalize(&parent).map_err(|_| ControlError::RuntimeUnavailable)? != parent {
            return Err(ControlError::PermissionDenied);
        }
        // SAFETY: geteuid has no pointer arguments or memory-safety preconditions.
        #[allow(unsafe_code)]
        let owner = unsafe { libc::geteuid() };
        let directory_identity = private_listener_directory(
            &std::fs::symlink_metadata(&parent).map_err(|_| ControlError::RuntimeUnavailable)?,
            owner,
        )?;
        let directory = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(&parent)
            .map_err(|_| ControlError::PermissionDenied)?;
        if private_listener_directory(
            &directory
                .metadata()
                .map_err(|_| ControlError::RuntimeUnavailable)?,
            owner,
        )? != directory_identity
        {
            return Err(ControlError::StaleLaunch);
        }
        let socket_identity = listener_socket_identity(
            &std::fs::symlink_metadata(path).map_err(|_| ControlError::RuntimeUnavailable)?,
            owner,
        )?;
        let route = Self {
            launch,
            path: path.to_owned(),
            parent,
            directory,
            directory_identity,
            socket_identity,
            lost: std::sync::atomic::AtomicBool::new(false),
        };
        route.check_current(&route.launch)?;
        Ok(route)
    }

    fn check_current(&self, launch: &LaunchRef) -> Result<(), ControlError> {
        use std::sync::atomic::Ordering;
        if launch != &self.launch {
            return Err(ControlError::StaleLaunch);
        }
        if self.lost.load(Ordering::Acquire) {
            return Err(ControlError::StaleLaunch);
        }
        let check = || {
            let owner = self.directory_identity.2;
            if std::fs::canonicalize(&self.parent).map_err(|_| ControlError::StaleLaunch)?
                != self.parent
                || private_listener_directory(
                    &std::fs::symlink_metadata(&self.parent)
                        .map_err(|_| ControlError::StaleLaunch)?,
                    owner,
                )? != self.directory_identity
                || private_listener_directory(
                    &self
                        .directory
                        .metadata()
                        .map_err(|_| ControlError::StaleLaunch)?,
                    owner,
                )? != self.directory_identity
                || listener_socket_identity(
                    &std::fs::symlink_metadata(&self.path)
                        .map_err(|_| ControlError::StaleLaunch)?,
                    owner,
                )? != self.socket_identity
            {
                return Err(ControlError::StaleLaunch);
            }
            Ok(())
        };
        if check().is_err() {
            self.lost.store(true, Ordering::Release);
            return Err(ControlError::StaleLaunch);
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn connect_path(&self) -> std::path::PathBuf {
        use std::os::fd::AsRawFd;
        // Dial beneath the retained parent, not a replacement of its pathname.
        // The socket identity and native peer are still checked independently.
        std::path::PathBuf::from(format!("/proc/self/fd/{}", self.directory.as_raw_fd()))
            .join(self.path.file_name().unwrap_or_default())
    }
}

#[cfg(unix)]
fn private_listener_directory(
    metadata: &std::fs::Metadata,
    owner: u32,
) -> Result<(u64, u64, u32, u32), ControlError> {
    use std::os::unix::fs::MetadataExt;
    if !metadata.file_type().is_dir()
        || metadata.uid() != owner
        || metadata.mode() & 0o7777 != 0o700
    {
        return Err(ControlError::PermissionDenied);
    }
    Ok((
        metadata.dev(),
        metadata.ino(),
        metadata.uid(),
        metadata.mode(),
    ))
}

#[cfg(unix)]
fn listener_socket_identity(
    metadata: &std::fs::Metadata,
    owner: u32,
) -> Result<(u64, u64, u32), ControlError> {
    use std::os::unix::fs::{FileTypeExt, MetadataExt};
    if !metadata.file_type().is_socket() || metadata.uid() != owner || metadata.nlink() != 1 {
        return Err(ControlError::PermissionDenied);
    }
    Ok((metadata.dev(), metadata.ino(), metadata.uid()))
}

#[cfg(unix)]
fn running_listener_peer(status: SandboxStatus, peer: Option<u32>) -> Result<u32, ControlError> {
    if status != SandboxStatus::Running {
        return Err(ControlError::RuntimeUnavailable);
    }
    peer.filter(|pid| *pid != 0)
        .ok_or(ControlError::UnsupportedCapability)
}

#[cfg(unix)]
fn listener_deadline(deadline: tokio::time::Instant) -> Result<(), ControlError> {
    if tokio::time::Instant::now() >= deadline {
        Err(ControlError::RuntimeUnavailable)
    } else {
        Ok(())
    }
}

#[cfg(unix)]
async fn connect_listener_checked<F, Fut>(
    launch: &LaunchRef,
    route: &ProtectedHostListener,
    deadline: tokio::time::Instant,
    mut proof: F,
) -> Result<tokio::net::UnixStream, ControlError>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<u32, ControlError>>,
{
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (launch, route, deadline, &mut proof);
        Err(ControlError::UnsupportedCapability)
    }
    #[cfg(target_os = "linux")]
    {
        listener_deadline(deadline)?;
        tokio::time::timeout_at(deadline, async {
            route.check_current(launch)?;
            let expected = proof().await?;
            if expected == 0 {
                return Err(ControlError::UnsupportedCapability);
            }
            route.check_current(launch)?;
            listener_deadline(deadline)?;
            let stream = tokio::net::UnixStream::connect(route.connect_path())
                .await
                .map_err(|_| ControlError::RuntimeUnavailable)?;
            let peer = stream
                .peer_cred()
                .map_err(|_| ControlError::RuntimeUnavailable)?;
            if peer.pid().and_then(|pid| u32::try_from(pid).ok()) != Some(expected)
                || peer.uid() != route.directory_identity.2
            {
                return Err(ControlError::PermissionDenied);
            }
            if proof().await? != expected {
                return Err(ControlError::StaleLaunch);
            }
            route.check_current(launch)?;
            listener_deadline(deadline)?;
            Ok(stream)
        })
        .await
        .map_err(|_| ControlError::RuntimeUnavailable)?
    }
}

impl NativeControl for MicrosandboxControl {
    fn capabilities(&self) -> RuntimeCapabilities {
        if self.backend.kind() == microsandbox::BackendKind::Local {
            Self::supported_capabilities()
        } else {
            RuntimeCapabilities::default()
        }
    }

    fn verify_launch<'a>(&'a self, launch: &'a LaunchRef) -> NativeFuture<'a, ()> {
        Box::pin(async move {
            self.selected(launch)?
                .status()
                .await
                .map_err(native_error)?;
            Ok(())
        })
    }

    fn start<'a>(
        &'a mut self,
        launch: &'a LaunchRef,
        command: ExecCommand,
    ) -> NativeFuture<'a, Box<dyn NativeExec>> {
        Box::pin(async move {
            let sandbox = self.selected(launch)?;
            let program = command.program.clone();
            // The receiver checks the original launch and retained peer before
            // dispatch. There is no replay after delivery failure or timeout.
            let mut handle = sandbox
                .exec_stream_with(program, |options| exec_options(options, command))
                .await
                .map_err(native_error)?;
            let stdin = handle.take_stdin();
            let control = handle.control();
            Ok(Box::new(MicrosandboxExec {
                handle: Some(handle),
                control,
                stdin,
                events: EventDrain::default(),
            }) as Box<dyn NativeExec>)
        })
    }

    fn lifecycle<'a>(
        &'a mut self,
        launch: &'a LaunchRef,
        operation: LifecycleOperation,
    ) -> NativeFuture<'a, LifecycleState> {
        Box::pin(async move {
            let sandbox = self.selected(launch)?;
            match operation {
                LifecycleOperation::Inspect => {
                    lifecycle_state(sandbox.status().await.map_err(native_error)?)
                }
                LifecycleOperation::Stop => {
                    sandbox.request_stop().await.map_err(native_error)?;
                    // Delivery is not exit proof, even if the request was an
                    // idempotent no-op for an already terminal selected launch.
                    Ok(LifecycleState::Stopping)
                }
            }
        })
    }
}

fn validate_association(
    same_backend: bool,
    expected: &str,
    actual: &str,
) -> Result<(), ControlError> {
    if !same_backend || expected != actual {
        Err(ControlError::StaleLaunch)
    } else {
        Ok(())
    }
}

fn admit_selection<T>(
    selected: &BTreeMap<LaunchRef, T>,
    launch: &LaunchRef,
) -> Result<(), ControlError> {
    if selected.contains_key(launch) {
        return Err(ControlError::RevisionConflict);
    }
    if selected.len() >= MAX_RETAINED_LAUNCHES {
        return Err(ControlError::ResourceLimit);
    }
    Ok(())
}

fn exec_options(mut options: ExecOptionsBuilder, command: ExecCommand) -> ExecOptionsBuilder {
    options = options
        .args(command.args)
        .envs(command.env)
        .timeout(Duration::from_millis(u64::from(command.timeout_ms)))
        .tty(command.tty);
    if let Some(cwd) = command.cwd {
        options = options.cwd(cwd);
    }
    if command.stdin {
        options.stdin_pipe()
    } else {
        options.stdin_null()
    }
}

fn lifecycle_state(status: SandboxStatus) -> Result<LifecycleState, ControlError> {
    match status {
        SandboxStatus::Running => Ok(LifecycleState::Running),
        SandboxStatus::Draining => Ok(LifecycleState::Stopping),
        SandboxStatus::Stopped | SandboxStatus::Crashed => Ok(LifecycleState::Stopped),
        // The neutral model has no Starting/Paused/Created category. Do not
        // mislabel those states as running, stopped or application-ready.
        SandboxStatus::Created | SandboxStatus::Starting | SandboxStatus::Paused => {
            Err(ControlError::RuntimeUnavailable)
        }
    }
}

fn native_error(error: MicrosandboxError) -> ControlError {
    match error {
        MicrosandboxError::LaunchBindingUnsupported | MicrosandboxError::Unsupported { .. } => {
            ControlError::UnsupportedCapability
        }
        MicrosandboxError::SandboxLaunchChanged { .. }
        | MicrosandboxError::SandboxReplaced { .. }
        | MicrosandboxError::SandboxNotFound(_) => ControlError::StaleLaunch,
        MicrosandboxError::ExecInterrupted(_) => ControlError::OperationIndeterminate,
        _ => ControlError::RuntimeUnavailable,
    }
}

struct MicrosandboxExec {
    handle: Option<ExecHandle>,
    control: ExecControl,
    stdin: Option<ExecSink>,
    events: EventDrain,
}

impl NativeExec for MicrosandboxExec {
    fn poll(&mut self, max_bytes: usize) -> Result<Vec<ExecEvent>, ControlError> {
        let events = self.events.poll(max_bytes, || {
            self.handle
                .as_mut()
                .map_or(Err(TryRecvError::Disconnected), ExecHandle::try_recv)
        });
        if self.events.delivery_failed {
            // SDK receiver closure requests its bounded original-session
            // cleanup. Keep the control lease so the ID cannot be reused, and
            // retain an unconfirmed operation reservation in the dispatcher.
            self.stdin.take();
            self.handle.take();
        }
        events
    }

    fn stdin<'a>(&'a mut self, bytes: &'a [u8], eof: bool) -> NativeFuture<'a, ()> {
        Box::pin(async move {
            validate_stdin(&self.events, self.stdin.is_some(), bytes.len(), eof)?;
            let attempt = InputAttempt {
                handle: &mut self.handle,
                stdin: &mut self.stdin,
                events: &mut self.events,
                committed: false,
            };
            let sink = attempt
                .stdin
                .as_ref()
                .ok_or(ControlError::OperationClosed)?;
            write_then_eof(!bytes.is_empty(), eof, |close| async move {
                if close {
                    sink.close().await
                } else {
                    sink.write(bytes).await
                }
                .map_err(native_error)
            })
            .await?;
            attempt.commit(eof);
            Ok(())
        })
    }

    fn resize(&mut self, rows: u16, columns: u16) -> NativeFuture<'_, ()> {
        Box::pin(async move {
            if self.events.ended || self.events.delivery_failed {
                return Err(ControlError::OperationClosed);
            }
            if rows == 0 || columns == 0 {
                return Err(ControlError::InvalidRequest);
            }
            self.control
                .resize(rows, columns)
                .await
                .map_err(native_error)
        })
    }

    fn cancel(&mut self) -> NativeFuture<'_, ()> {
        Box::pin(async move {
            if !self.events.ended && !self.events.delivery_failed {
                // The independent ordered event queue carries the outcome.
                // Returning here never upgrades a request to confirmed exit.
                let _ = self.control.cancel().await;
            }
            Ok(())
        })
    }
}

fn validate_stdin(
    events: &EventDrain,
    has_sink: bool,
    bytes: usize,
    eof: bool,
) -> Result<(), ControlError> {
    if events.ended || events.delivery_failed || !has_sink {
        return Err(ControlError::OperationClosed);
    }
    if bytes > MAX_POLL_BYTES || (bytes == 0 && !eof) {
        return Err(ControlError::InvalidRequest);
    }
    Ok(())
}

async fn write_then_eof<F, Fut>(has_bytes: bool, eof: bool, mut send: F) -> Result<(), ControlError>
where
    F: FnMut(bool) -> Fut,
    Fut: Future<Output = Result<(), ControlError>>,
{
    if has_bytes {
        send(false).await?;
    }
    if eof {
        send(true).await?;
    }
    Ok(())
}

/// Cancellation drops this guard too. Only complete delivery can keep stdin
/// writable. The separate ExecControl is intentionally outside this guard:
/// receiver-drop cleanup must not release its original protocol ID lease.
struct InputAttempt<'a, H, S> {
    handle: &'a mut Option<H>,
    stdin: &'a mut Option<S>,
    events: &'a mut EventDrain,
    committed: bool,
}

impl<H, S> InputAttempt<'_, H, S> {
    fn commit(mut self, eof: bool) {
        if eof {
            self.stdin.take();
        }
        self.committed = true;
    }
}

impl<H, S> Drop for InputAttempt<'_, H, S> {
    fn drop(&mut self) {
        if !self.committed {
            self.events.delivery_failed = true;
            self.events.pending_delivery = true;
            self.stdin.take();
            // The SDK actor observes receiver closure and owns bounded cleanup.
            // Already-dequeued partial output remains ahead of the interruption.
            self.handle.take();
        }
    }
}

#[derive(Default)]
struct EventDrain {
    pending: Option<microsandbox::ExecEvent>,
    offset: usize,
    ended: bool,
    delivery_failed: bool,
    pending_delivery: bool,
}

impl EventDrain {
    fn poll(
        &mut self,
        max_bytes: usize,
        mut receive: impl FnMut() -> Result<microsandbox::ExecEvent, TryRecvError>,
    ) -> Result<Vec<ExecEvent>, ControlError> {
        if max_bytes == 0 || max_bytes > MAX_POLL_BYTES {
            return Err(ControlError::InvalidRequest);
        }
        let mut output = Vec::new();
        let mut remaining = max_bytes;
        while !self.ended && output.len() < MAX_POLL_EVENTS && remaining > 0 {
            let event = match self.pending.take() {
                Some(event) => event,
                None if self.pending_delivery => {
                    self.pending_delivery = false;
                    microsandbox::ExecEvent::Interrupted(microsandbox::ExecInterruption {
                        reason: microsandbox::ExecInterruptionReason::Delivery,
                        termination: microsandbox::ExecTermination::Unconfirmed,
                    })
                }
                None => match receive() {
                    Ok(event) => event,
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        self.ended = true;
                        output.push(ExecEvent::TransportLost);
                        break;
                    }
                },
            };
            match event {
                microsandbox::ExecEvent::Stdout(ref bytes)
                | microsandbox::ExecEvent::Stderr(ref bytes) => {
                    if bytes.len() > MAX_NATIVE_CHUNK {
                        // Refuse an incompatible native producer before
                        // retaining its oversized allocation across polls.
                        self.ended = true;
                        self.delivery_failed = true;
                        output.push(ExecEvent::Interrupted {
                            reason: ExecInterruptionReason::OutputLimit,
                            termination: ExecTermination::Unconfirmed,
                        });
                        break;
                    }
                    let count = remaining.min(bytes.len() - self.offset);
                    let data = bytes[self.offset..self.offset + count].to_vec();
                    output.push(if matches!(event, microsandbox::ExecEvent::Stdout(_)) {
                        ExecEvent::Stdout { bytes: data }
                    } else {
                        ExecEvent::Stderr { bytes: data }
                    });
                    remaining -= count;
                    self.offset += count;
                    if self.offset < bytes.len() {
                        self.pending = Some(event);
                    } else {
                        self.offset = 0;
                    }
                }
                microsandbox::ExecEvent::Started { .. } => output.push(ExecEvent::Started),
                microsandbox::ExecEvent::Exited { code } => {
                    self.ended = true;
                    output.push(ExecEvent::Exited { code });
                }
                microsandbox::ExecEvent::Failed(_) => {
                    self.ended = true;
                    output.push(ExecEvent::SpawnFailed);
                }
                microsandbox::ExecEvent::StdinError(_) => {
                    self.ended = true;
                    self.delivery_failed = true;
                    output.push(ExecEvent::Interrupted {
                        reason: ExecInterruptionReason::Delivery,
                        termination: ExecTermination::Unconfirmed,
                    });
                }
                microsandbox::ExecEvent::Interrupted(value) => {
                    self.ended = true;
                    output.push(project_interruption(value));
                }
            }
        }
        Ok(output)
    }
}

fn project_interruption(value: microsandbox::ExecInterruption) -> ExecEvent {
    let reason = match value.reason {
        microsandbox::ExecInterruptionReason::Timeout(_) => ExecInterruptionReason::Timeout,
        microsandbox::ExecInterruptionReason::Cancelled => ExecInterruptionReason::Cancelled,
        microsandbox::ExecInterruptionReason::OutputLimit => ExecInterruptionReason::OutputLimit,
        microsandbox::ExecInterruptionReason::TransportClosed => {
            ExecInterruptionReason::TransportClosed
        }
        microsandbox::ExecInterruptionReason::Protocol => ExecInterruptionReason::Protocol,
        microsandbox::ExecInterruptionReason::Delivery => ExecInterruptionReason::Delivery,
    };
    let termination = match value.termination {
        microsandbox::ExecTermination::Exited(code) => ExecTermination::Exited { code },
        microsandbox::ExecTermination::SpawnFailed(_) => ExecTermination::SpawnFailed,
        microsandbox::ExecTermination::Unconfirmed => ExecTermination::Unconfirmed,
    };
    ExecEvent::Interrupted {
        reason,
        termination,
    }
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;

#[cfg(all(test, target_os = "linux"))]
#[path = "native_vm_tests.rs"]
mod vm_tests;
