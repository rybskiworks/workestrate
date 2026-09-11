//! One permission and operation owner, with direct custody/native leaf dispatch.
//!
//! The owner serializes requests; native event polling must be nonblocking and
//! every asynchronous native call has a fixed budget. This is an in-process
//! integration seam, not itself a readiness assertion. Persistent
//! desired-state commit and broker I/O remain the reconciler owner's boundary.

use std::collections::BTreeMap;
use std::time::Duration;

use super::authorization::{AuthenticatedCaller, CallerIdentity};
use super::custody::{SshController, SshPolicyTransaction};
use super::native::{NativeControl, NativeExec, NativeFuture};
use super::types::*;

const NATIVE_CALL_BUDGET: Duration = Duration::from_secs(5);
const MAX_OPERATIONS: usize = 32;
const MAX_COMMAND_BYTES: usize = 32 * 1024;
// JSON byte arrays may expand to four characters per byte. Keep the resulting
// reply, including metadata, below the local codec's 64 KiB frame limit.
const MAX_IO_BYTES: usize = 8 * 1024;
const MAX_EVENTS: usize = 64;

struct OwnedExec {
    caller: CallerIdentity,
    launch: LaunchRef,
    native: Option<Box<dyn NativeExec>>,
    state: ExecState,
    stdin_open: bool,
    tty: bool,
}

/// Internal result: a caller only receives `response`. The trusted reconciler
/// owner consumes `custody_transaction` after its durable commit boundary;
/// accepting this value is not evidence the broker applied it.
pub(crate) struct DispatchOutcome {
    pub response: ControlResponse,
    pub custody_transaction: Option<SshPolicyTransaction>,
}

pub(crate) struct ControlDispatcher<N> {
    incarnation: OpaqueId,
    sequence: u64,
    native: N,
    custody: SshController,
    operations: BTreeMap<OperationRef, OwnedExec>,
    admitting: bool,
}

impl<N: NativeControl> ControlDispatcher<N> {
    pub(crate) fn new(incarnation: OpaqueId, native: N) -> Self {
        Self::with_custody(native, SshController::new(incarnation))
    }

    /// Accept the single configured desired-state owner. Its incarnation also
    /// fences execution handles, so callers cannot supply a conflicting owner.
    pub(crate) fn with_custody(native: N, custody: SshController) -> Self {
        Self {
            incarnation: custody.incarnation().clone(),
            custody,
            sequence: 0,
            native,
            operations: BTreeMap::new(),
            admitting: true,
        }
    }

    /// Trusted runtime/broker integration only; never exposed through the codec.
    pub(crate) fn custody_mut(&mut self) -> &mut SshController {
        &mut self.custody
    }

    /// Retained native resource access for the trusted host owner, not the codec.
    pub(crate) fn native(&self) -> &N {
        &self.native
    }

    pub(crate) async fn dispatch(
        &mut self,
        caller: &AuthenticatedCaller,
        request: ControlRequest,
    ) -> Result<DispatchOutcome, ControlError> {
        // The same matcher serves all adapters. Deny before target discovery.
        caller.authorize_control(&request)?;
        self.check_owner()?;
        if let CallerIdentity::WorkloadLaunch(origin) = caller.identity() {
            self.custody
                .verify_registered_launch(origin)
                .map_err(|_| ControlError::PermissionDenied)?;
            bounded(self.native.verify_launch(origin))
                .await
                .map_err(|_| ControlError::PermissionDenied)?;
            self.check_owner()?;
        }
        let response = match request {
            ControlRequest::Capabilities { .. } => ControlResponse::Capabilities {
                runtime: self.native.capabilities(),
                ssh_custody: true,
            },
            ControlRequest::SshCustody(request) => {
                let change = self.custody.request(caller, request)?;
                return Ok(DispatchOutcome {
                    response: ControlResponse::SshCustody(change.status),
                    custody_transaction: change.transaction,
                });
            }
            ControlRequest::Lifecycle { launch, operation } => {
                if !self.native.capabilities().launch_bound_lifecycle {
                    return Err(ControlError::UnsupportedCapability);
                }
                self.custody.verify_registered_launch(&launch)?;
                bounded(self.native.verify_launch(&launch)).await?;
                self.check_owner()?;
                // The native operation must itself enforce the binding. The
                // precheck is not a substitute for an atomic native fence.
                let state = bounded(self.native.lifecycle(&launch, operation)).await?;
                ControlResponse::Lifecycle { launch, state }
            }
            ControlRequest::GuestExec { launch, request } => {
                ControlResponse::GuestExec(self.exec(caller, launch, request).await?)
            }
        };
        self.check_owner()?;
        Ok(DispatchOutcome {
            response,
            custody_transaction: None,
        })
    }

    async fn exec(
        &mut self,
        caller: &AuthenticatedCaller,
        launch: LaunchRef,
        request: ExecRequest,
    ) -> Result<ExecStatus, ControlError> {
        let capabilities = self.native.capabilities();
        if !capabilities.launch_bound_exec {
            return Err(ControlError::UnsupportedCapability);
        }
        if let ExecRequest::Start { command } = request {
            validate_command(&command)?;
            if (command.stdin && !capabilities.stdin)
                || (command.tty && !capabilities.pty)
                || !capabilities.cancellation
            {
                return Err(ControlError::UnsupportedCapability);
            }
            if self.operations.len() >= MAX_OPERATIONS {
                return Err(ControlError::ResourceLimit);
            }
            // The trusted owner binds workload/context to the instance. A
            // request cannot relabel another instance using a known native ID.
            self.custody.verify_registered_launch(&launch)?;
            bounded(self.native.verify_launch(&launch)).await?;
            self.check_owner()?;
            self.sequence = self
                .sequence
                .checked_add(1)
                .ok_or(ControlError::ResourceLimit)?;
            let id = OperationRef {
                controller: self.incarnation.clone(),
                sequence: self.sequence,
            };
            // Preserve an indeterminate reservation if this future is dropped
            // after native dispatch. A timeout cannot free an uncertain slot or
            // permit an automatic retry/replay of a possibly executed command.
            self.operations.insert(
                id.clone(),
                OwnedExec {
                    caller: caller.identity().clone(),
                    launch: launch.clone(),
                    native: None,
                    state: ExecState::Indeterminate,
                    stdin_open: command.stdin,
                    tty: command.tty,
                },
            );
            let started = bounded(self.native.start(&launch, command)).await;
            let entry = self
                .operations
                .get_mut(&id)
                .ok_or(ControlError::UnknownOperation)?;
            if let Ok(native) = started {
                entry.native = Some(native);
                entry.state = ExecState::Accepted;
            }
            // An interrupted or failed dispatch remains visible as uncertain,
            // never fabricated into a guest exit or a successful cancellation.
            return Ok(status(&id, entry, Vec::new()));
        }
        let id = request.id().ok_or(ControlError::InvalidRequest)?.clone();
        let entry = self
            .operations
            .get(&id)
            .ok_or(ControlError::UnknownOperation)?;
        if entry.caller != *caller.identity() || entry.launch != launch {
            return Err(ControlError::UnknownOperation);
        }
        let entry = self
            .operations
            .get_mut(&id)
            .ok_or(ControlError::UnknownOperation)?;
        if let ExecRequest::Release { .. } = request {
            if !entry.state.terminal() {
                return Err(ControlError::OperationIndeterminate);
            }
            let result = status(&id, entry, Vec::new());
            self.operations.remove(&id);
            return Ok(result);
        }
        if entry.state.terminal() || matches!(entry.state, ExecState::Interrupted { .. }) {
            return match request {
                ExecRequest::Read { .. } | ExecRequest::Cancel { .. } => {
                    Ok(status(&id, entry, Vec::new()))
                }
                _ => Err(ControlError::OperationClosed),
            };
        }
        // Terminal status/release above only access the caller's completed local
        // record. Any access to a live native lease revalidates the exact launch.
        self.custody.verify_registered_launch(&launch)?;
        bounded(self.native.verify_launch(&launch)).await?;
        self.custody.ensure_state()?;
        let Some(native) = entry.native.as_mut() else {
            return match request {
                ExecRequest::Read { .. } => Ok(status(&id, entry, Vec::new())),
                _ => Err(ControlError::OperationIndeterminate),
            };
        };
        let events = match request {
            ExecRequest::Read { max_bytes, .. } => {
                let budget = max_bytes as usize;
                if budget == 0 || budget > MAX_IO_BYTES {
                    return Err(ControlError::InvalidRequest);
                }
                match native.poll(budget) {
                    Ok(events) => {
                        if let Err(error) = accept_events(entry, &events, budget) {
                            entry.state = ExecState::Indeterminate;
                            return Err(error);
                        }
                        events
                    }
                    Err(_) => {
                        entry.state = ExecState::Indeterminate;
                        return Err(ControlError::OperationIndeterminate);
                    }
                }
            }
            ExecRequest::WriteStdin { bytes, eof, .. } => {
                if !capabilities.stdin {
                    return Err(ControlError::UnsupportedCapability);
                }
                if !entry.stdin_open || bytes.len() > MAX_IO_BYTES || (bytes.is_empty() && !eof) {
                    return Err(ControlError::InvalidRequest);
                }
                let previous = entry.state;
                entry.state = ExecState::Indeterminate;
                if bounded(native.stdin(&bytes, eof)).await.is_err() {
                    return Err(ControlError::OperationIndeterminate);
                }
                entry.state = previous;
                entry.stdin_open &= !eof;
                Vec::new()
            }
            ExecRequest::Resize { rows, columns, .. } => {
                if !capabilities.pty {
                    return Err(ControlError::UnsupportedCapability);
                }
                if !entry.tty || rows == 0 || columns == 0 {
                    return Err(ControlError::InvalidRequest);
                }
                let previous = entry.state;
                entry.state = ExecState::Indeterminate;
                if bounded(native.resize(rows, columns)).await.is_err() {
                    return Err(ControlError::OperationIndeterminate);
                }
                entry.state = previous;
                Vec::new()
            }
            ExecRequest::Cancel { .. } => {
                if !capabilities.cancellation {
                    return Err(ControlError::UnsupportedCapability);
                }
                entry.state = ExecState::Indeterminate;
                if bounded(native.cancel()).await.is_err() {
                    return Err(ControlError::OperationIndeterminate);
                }
                entry.state = ExecState::CancellationRequested;
                Vec::new()
            }
            ExecRequest::Start { .. } | ExecRequest::Release { .. } => {
                return Err(ControlError::InvalidRequest);
            }
        };
        Ok(status(&id, entry, events))
    }
}

async fn bounded<T>(call: NativeFuture<'_, T>) -> Result<T, ControlError> {
    tokio::time::timeout(NATIVE_CALL_BUDGET, call)
        .await
        .map_err(|_| ControlError::OperationIndeterminate)?
}

fn status(id: &OperationRef, entry: &OwnedExec, events: Vec<ExecEvent>) -> ExecStatus {
    ExecStatus {
        launch: entry.launch.clone(),
        id: id.clone(),
        state: entry.state,
        events,
    }
}

fn accept_events(
    entry: &mut OwnedExec,
    events: &[ExecEvent],
    budget: usize,
) -> Result<(), ControlError> {
    if events.len() > MAX_EVENTS {
        return Err(ControlError::InvalidRuntimeResponse);
    }
    let mut bytes = 0usize;
    let mut state = entry.state;
    for event in events {
        if state.terminal() || matches!(state, ExecState::Interrupted { .. }) {
            return Err(ControlError::InvalidRuntimeResponse);
        }
        match event {
            ExecEvent::Started => {
                if state == ExecState::Accepted {
                    state = ExecState::Running;
                } else if state != ExecState::CancellationRequested
                    && state != ExecState::Indeterminate
                {
                    return Err(ControlError::InvalidRuntimeResponse);
                }
            }
            ExecEvent::Stdout { bytes: chunk } | ExecEvent::Stderr { bytes: chunk } => {
                if entry.tty && matches!(event, ExecEvent::Stderr { .. }) {
                    return Err(ControlError::InvalidRuntimeResponse);
                }
                bytes = bytes
                    .checked_add(chunk.len())
                    .ok_or(ControlError::InvalidRuntimeResponse)?;
                if bytes > budget {
                    return Err(ControlError::InvalidRuntimeResponse);
                }
            }
            ExecEvent::Exited { code } => state = ExecState::Exited { code: *code },
            ExecEvent::SpawnFailed => state = ExecState::SpawnFailed,
            ExecEvent::TransportLost => state = ExecState::Indeterminate,
            ExecEvent::Interrupted {
                reason,
                termination,
            } => {
                state = ExecState::Interrupted {
                    reason: *reason,
                    termination: *termination,
                };
            }
        }
    }
    entry.state = state;
    Ok(())
}

fn validate_command(command: &ExecCommand) -> Result<(), ControlError> {
    let valid = |value: &str| !value.contains('\0');
    if command.program.is_empty()
        || !valid(&command.program)
        || command.timeout_ms == 0
        || command.timeout_ms > 3_600_000
        || command.args.len() > 256
        || command.env.len() > 256
        || command.args.iter().any(|arg| !valid(arg))
        || command
            .cwd
            .as_ref()
            .is_some_and(|cwd| !cwd.starts_with('/') || !valid(cwd))
        || command
            .env
            .iter()
            .any(|(key, value)| key.is_empty() || key.contains('=') || !valid(key) || !valid(value))
    {
        return Err(ControlError::InvalidRequest);
    }
    let size = command
        .args
        .iter()
        .chain(command.cwd.iter())
        .map(String::len)
        .chain(
            command
                .env
                .iter()
                .map(|(key, value)| key.len().saturating_add(value.len())),
        )
        .try_fold(command.program.len(), usize::checked_add)
        .ok_or(ControlError::InvalidRequest)?;
    if size > MAX_COMMAND_BYTES {
        return Err(ControlError::InvalidRequest);
    }
    Ok(())
}

#[derive(Debug)]
pub(crate) struct DrainSummary {
    pub completed: Vec<OperationRef>,
    pub pending: Vec<(OperationRef, ExecState)>,
    pub errors: Vec<(OperationRef, ControlError)>,
}

impl<N: NativeControl> ControlDispatcher<N> {
    pub(crate) fn check_owner(&self) -> Result<(), ControlError> {
        if !self.admitting {
            return Err(ControlError::StateUnavailable);
        }
        self.custody.ensure_state()
    }

    pub(crate) fn fence(&mut self) {
        self.admitting = false;
    }

    /// Stop admission and drain the original leases, not new authorized calls.
    /// Dropping this future keeps all operation entries in this owner. No VM is
    /// stopped, no name is resolved, and cancellation acceptance is not exit.
    pub(crate) async fn drain_until(&mut self, deadline: tokio::time::Instant) -> DrainSummary {
        use std::future::Future;
        use std::task::Poll;

        self.fence();
        let mut errors = Vec::new();
        {
            // At most MAX_OPERATIONS futures, borrowed from the existing map.
            // Poll every lease each round; a slow first cancellation cannot
            // consume the entire budget before later leases are even signalled.
            let mut drains: Vec<_> = self
                .operations
                .iter_mut()
                .map(|(id, entry)| (id.clone(), Some(Box::pin(drain_owned(entry, deadline)))))
                .collect();
            std::future::poll_fn(|context| {
                let mut pending = false;
                for (id, future) in &mut drains {
                    let Some(active) = future else { continue };
                    match active.as_mut().poll(context) {
                        Poll::Ready(error) => {
                            if let Some(error) = error {
                                errors.push((id.clone(), error));
                            }
                            *future = None;
                        }
                        Poll::Pending => pending = true,
                    }
                }
                if pending {
                    Poll::Pending
                } else {
                    Poll::Ready(())
                }
            })
            .await;
        }
        let completed: Vec<_> = self
            .operations
            .iter()
            .filter(|(_, entry)| entry.state.terminal())
            .map(|(id, _)| id.clone())
            .collect();
        for id in &completed {
            self.operations.remove(id);
        }
        DrainSummary {
            completed,
            pending: self
                .operations
                .iter()
                .map(|(id, entry)| (id.clone(), entry.state))
                .collect(),
            errors,
        }
    }
}

async fn drain_owned(
    entry: &mut OwnedExec,
    deadline: tokio::time::Instant,
) -> Option<ControlError> {
    if entry.state.terminal() {
        return None;
    }
    if entry.native.is_none() || matches!(entry.state, ExecState::Interrupted { .. }) {
        return Some(ControlError::OperationIndeterminate);
    }
    // Check already-buffered terminal evidence before requesting cancellation.
    if let Err(error) = poll_owned(entry) {
        return Some(error);
    }
    if entry.state.terminal() {
        return None;
    }
    if matches!(entry.state, ExecState::Interrupted { .. }) {
        return Some(ControlError::OperationIndeterminate);
    }
    if tokio::time::Instant::now() >= deadline {
        return Some(ControlError::OperationIndeterminate);
    }
    entry.state = ExecState::Indeterminate;
    let Some(native) = entry.native.as_mut() else {
        return Some(ControlError::OperationIndeterminate);
    };
    let cancelled = tokio::time::timeout_at(deadline, native.cancel()).await;
    let mut error = None;
    match cancelled {
        Ok(Ok(())) => entry.state = ExecState::CancellationRequested,
        Ok(Err(cause)) => error = Some(cause),
        Err(_) => error = Some(ControlError::OperationIndeterminate),
    }
    loop {
        if let Err(cause) = poll_owned(entry) {
            return Some(cause);
        }
        if entry.state.terminal() {
            return error;
        }
        if matches!(entry.state, ExecState::Interrupted { .. })
            || tokio::time::Instant::now() >= deadline
        {
            return Some(error.unwrap_or(ControlError::OperationIndeterminate));
        }
        // Both byte/event work and the delay are bounded. Every lease has its
        // own timer while sharing the one absolute shutdown deadline.
        tokio::time::sleep_until(std::cmp::min(
            deadline,
            tokio::time::Instant::now() + Duration::from_millis(10),
        ))
        .await;
    }
}

fn poll_owned(entry: &mut OwnedExec) -> Result<(), ControlError> {
    let native = entry
        .native
        .as_mut()
        .ok_or(ControlError::OperationIndeterminate)?;
    let events = match native.poll(MAX_IO_BYTES) {
        Ok(events) => events,
        Err(_) => {
            entry.state = ExecState::Indeterminate;
            return Err(ControlError::OperationIndeterminate);
        }
    };
    if let Err(error) = accept_events(entry, &events, MAX_IO_BYTES) {
        entry.state = ExecState::Indeterminate;
        return Err(error);
    }
    Ok(())
}
