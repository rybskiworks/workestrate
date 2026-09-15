//! In-task supervision of one retained foreground exec session.

use std::future::Future;
use std::time::Duration;

use microsandbox::sandbox::exec::{
    ExecEvent, ExecHandle, ExecInterruption, ExecInterruptionReason, ExecTermination,
};
use tokio::time::Instant;

pub(super) const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
pub(super) const CANCEL_TIMEOUT: Duration = Duration::from_secs(10);
pub(super) const STOP_TIMEOUT: Duration = Duration::from_secs(45);
pub(super) const SHIM_TIMEOUT: Duration = Duration::from_secs(2);

// A private test seam, not a second runtime abstraction or forwarding queue.
pub(super) trait ServiceEvents {
    async fn next_event(&mut self) -> Option<ExecEvent>;
}

impl ServiceEvents for ExecHandle {
    async fn next_event(&mut self) -> Option<ExecEvent> {
        self.recv().await
    }
}

#[derive(Debug)]
pub(super) enum ServiceEnd {
    ShutdownRequested,
    StartRequestFailed(String),
    StartupTimeout,
    SignalFailure(String),
    Exited {
        code: i32,
        started: bool,
    },
    Failed {
        detail: String,
        started: bool,
    },
    Interrupted {
        value: ExecInterruption,
        started: bool,
    },
    InputFailure(String),
    UnexpectedEvent,
    Eof {
        started: bool,
    },
}

impl ServiceEnd {
    pub(super) fn from_signal(result: std::io::Result<()>) -> Self {
        match result {
            Ok(()) => Self::ShutdownRequested,
            Err(error) => Self::SignalFailure(error.to_string()),
        }
    }

    pub(super) fn needs_cancel(&self) -> bool {
        !matches!(
            self,
            Self::Exited { .. } | Self::Failed { .. } | Self::Interrupted { .. }
        )
    }

    fn failure(&self) -> Option<String> {
        Some(match self {
            Self::ShutdownRequested => return None,
            Self::StartRequestFailed(detail) => format!("exec request failed: {detail}"),
            Self::StartupTimeout => "timed out waiting for service start".into(),
            Self::SignalFailure(detail) => format!("signal handler failed: {detail}"),
            Self::Exited { code, started } => format!(
                "service exited unexpectedly with code {code} ({})",
                phase(*started)
            ),
            Self::Failed { detail, started } => {
                format!("service spawn failed ({}): {detail}", phase(*started))
            }
            Self::Interrupted { value, started } => {
                format!("service interrupted ({}): {value}", phase(*started))
            }
            Self::InputFailure(detail) => format!("service stdin failed: {detail}"),
            Self::UnexpectedEvent => "unexpected service exec event ordering".into(),
            Self::Eof { started } => {
                format!(
                    "service exec EOF without terminal event ({})",
                    phase(*started)
                )
            }
        })
    }
}

fn phase(started: bool) -> &'static str {
    if started {
        "after Started"
    } else {
        "before Started"
    }
}

/// The receiver remains borrowed by this future; cancellation never detaches a
/// log task. Started reports process creation, not application readiness.
pub(super) async fn supervise<S, F, L>(
    events: &mut S,
    shutdown: F,
    startup_deadline: Instant,
    mut log: L,
) -> ServiceEnd
where
    S: ServiceEvents,
    F: Future<Output = std::io::Result<()>>,
    L: FnMut(&ExecEvent),
{
    tokio::pin!(shutdown);
    let mut started = false;
    loop {
        // Give cancellation priority over a continuously readable log stream.
        // The startup clock is disabled after Started, not reset by output.
        let event = tokio::select! {
            biased;
            result = &mut shutdown => return ServiceEnd::from_signal(result),
            _ = tokio::time::sleep_until(startup_deadline), if !started => {
                return ServiceEnd::StartupTimeout;
            }
            event = events.next_event() => event,
        };
        match event {
            Some(event @ ExecEvent::Started { .. }) if !started => {
                started = true;
                log(&event);
            }
            Some(event @ (ExecEvent::Stdout(_) | ExecEvent::Stderr(_))) if started => {
                log(&event);
            }
            Some(ExecEvent::Exited { code }) => return ServiceEnd::Exited { code, started },
            Some(ExecEvent::Failed(error)) => {
                return ServiceEnd::Failed {
                    detail: format!("{error:?}"),
                    started,
                };
            }
            Some(ExecEvent::Interrupted(value)) => {
                return ServiceEnd::Interrupted { value, started };
            }
            Some(ExecEvent::StdinError(error)) => {
                return ServiceEnd::InputFailure(format!("{error:?}"));
            }
            Some(_) => return ServiceEnd::UnexpectedEvent,
            None => return ServiceEnd::Eof { started },
        }
    }
}

#[derive(Debug)]
pub(super) enum Cancellation {
    NotNeeded,
    Observed(ExecInterruption),
    TimedOut,
}

/// A terminal backend row and a successful wait of the original owned runtime
/// are different evidence. Only the latter proves all its guest execs ended.
#[derive(Clone, Copy, Debug)]
pub(super) enum StopObservation {
    StoppedState,
    OwnedRuntimeExited,
}

pub(super) fn owned_runtime_exit(
    status: std::process::ExitStatus,
) -> Result<StopObservation, String> {
    if status.success() {
        Ok(StopObservation::OwnedRuntimeExited)
    } else {
        Err(format!(
            "original sandbox runtime exited unsuccessfully: {status}"
        ))
    }
}

/// These are independent observations, not one total-cleanup deadline. Legacy
/// broker-forwarder shutdown is outside them and still has a blocking join.
pub(super) struct Cleanup {
    pub cancellation: Cancellation,
    pub stop: Result<StopObservation, String>,
    pub shim: Result<(), String>,
}

pub(super) fn finish(label: &str, end: ServiceEnd, cleanup: Cleanup) -> anyhow::Result<()> {
    let owned_shutdown = matches!(&end, ServiceEnd::ShutdownRequested)
        && matches!(&cleanup.stop, Ok(StopObservation::OwnedRuntimeExited));
    let mut failures = Vec::new();
    if let Some(primary) = end.failure() {
        failures.push(primary);
    }
    match cleanup.cancellation {
        Cancellation::NotNeeded => {}
        Cancellation::TimedOut => {
            failures.push("exec cancellation observation timed out; termination unconfirmed".into())
        }
        Cancellation::Observed(value) => {
            if !matches!(value.reason, ExecInterruptionReason::Cancelled)
                || (matches!(value.termination, ExecTermination::Unconfirmed) && !owned_shutdown)
            {
                failures.push(format!("exec cleanup interruption: {value}"));
            }
        }
    }
    if let Err(error) = cleanup.stop {
        failures.push(format!("sandbox stop incomplete: {error}"));
    }
    if let Err(error) = cleanup.shim {
        failures.push(format!("SSH shim retirement incomplete: {error}"));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        anyhow::bail!("{label}: {}", failures.join("; "))
    }
}

#[cfg(test)]
#[path = "foreground_tests.rs"]
mod tests;
