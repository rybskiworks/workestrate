//! Internal translation seam to the selected native runtime, not another
//! backend registry. Native implementations retain their own connection/session
//! leases; Workestrate owns permissions and the public operation references.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::types::{
    ControlError, ExecCommand, ExecEvent, LaunchRef, LifecycleOperation, LifecycleState,
    RuntimeCapabilities,
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

/// Keeps the already-selected SDK backend. The consumed SDK does not yet offer
/// authoritative launch-bound exec/session and lifecycle operations, so this
/// adapter refuses before invoking older name- or sandbox-ID-addressed methods.
/// Adopting a newer SDK must replace this explicit refusal with its native
/// bound handles, not a precheck followed by the old methods.
pub(crate) struct MicrosandboxControl {
    _backend: Arc<dyn microsandbox::Backend>,
}

impl MicrosandboxControl {
    pub(crate) fn new(backend: Arc<dyn microsandbox::Backend>) -> Self {
        Self { _backend: backend }
    }

    pub(crate) const fn supported_capabilities() -> RuntimeCapabilities {
        RuntimeCapabilities {
            launch_bound_exec: false,
            stdin: false,
            pty: false,
            cancellation: false,
            launch_bound_lifecycle: false,
        }
    }
}

impl NativeControl for MicrosandboxControl {
    fn capabilities(&self) -> RuntimeCapabilities {
        Self::supported_capabilities()
    }

    fn verify_launch<'a>(&'a self, _launch: &'a LaunchRef) -> NativeFuture<'a, ()> {
        Box::pin(async { Err(ControlError::UnsupportedCapability) })
    }

    fn start<'a>(
        &'a mut self,
        _launch: &'a LaunchRef,
        _command: ExecCommand,
    ) -> NativeFuture<'a, Box<dyn NativeExec>> {
        Box::pin(async { Err(ControlError::UnsupportedCapability) })
    }

    fn lifecycle<'a>(
        &'a mut self,
        _launch: &'a LaunchRef,
        _operation: LifecycleOperation,
    ) -> NativeFuture<'a, LifecycleState> {
        Box::pin(async { Err(ControlError::UnsupportedCapability) })
    }
}
