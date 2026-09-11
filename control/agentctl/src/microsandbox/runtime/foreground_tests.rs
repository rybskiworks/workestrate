//! Deterministic service-event and ownership controls, with no guest or signals.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use microsandbox::protocol::exec::{ExecFailed, ExecFailureKind, ExecStdinError};
use std::collections::VecDeque;
use std::future::pending;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Events {
    queued: VecDeque<ExecEvent>,
    eof: bool,
    drops: Arc<AtomicUsize>,
}

impl Events {
    fn new(events: impl IntoIterator<Item = ExecEvent>) -> Self {
        Self {
            queued: events.into_iter().collect(),
            eof: true,
            drops: Arc::new(AtomicUsize::new(0)),
        }
    }
}

impl Drop for Events {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

impl ServiceEvents for Events {
    async fn next_event(&mut self) -> Option<ExecEvent> {
        if let Some(event) = self.queued.pop_front() {
            Some(event)
        } else if self.eof {
            None
        } else {
            pending().await
        }
    }
}

fn started() -> ExecEvent {
    ExecEvent::Started { pid: 7 }
}

fn interruption(reason: ExecInterruptionReason, termination: ExecTermination) -> ExecInterruption {
    ExecInterruption {
        reason,
        termination,
    }
}

fn clean() -> Cleanup {
    Cleanup {
        cancellation: Cancellation::NotNeeded,
        stop: Ok(()),
        shim: Ok(()),
    }
}

async fn observe(events: impl IntoIterator<Item = ExecEvent>) -> ServiceEnd {
    supervise(
        &mut Events::new(events),
        pending(),
        Instant::now() + STARTUP_TIMEOUT,
        |_| {},
    )
    .await
}

#[tokio::test]
async fn foreground_zero_exit_ends_before_ctrl_c_and_is_not_health() {
    let end = observe([started(), ExecEvent::Exited { code: 0 }]).await;
    assert!(matches!(
        &end,
        ServiceEnd::Exited {
            code: 0,
            started: true
        }
    ));
    assert!(!end.needs_cancel());
    assert!(
        finish("service", end, clean())
            .unwrap_err()
            .to_string()
            .contains("code 0")
    );
}

#[tokio::test]
async fn foreground_nonzero_exit_preserves_code() {
    let end = observe([started(), ExecEvent::Exited { code: 23 }]).await;
    assert!(
        finish("service", end, clean())
            .unwrap_err()
            .to_string()
            .contains("code 23")
    );
}

#[tokio::test]
async fn foreground_early_exit_is_not_started() {
    let end = observe([ExecEvent::Exited { code: 0 }]).await;
    assert!(matches!(end, ServiceEnd::Exited { started: false, .. }));
}

#[tokio::test]
async fn foreground_spawn_failure_retains_start_phase() {
    for did_start in [false, true] {
        let failed = ExecEvent::Failed(ExecFailed {
            kind: ExecFailureKind::Other,
            errno: None,
            errno_name: None,
            message: "synthetic spawn failure".into(),
            stage: None,
        });
        let events = if did_start {
            vec![started(), failed]
        } else {
            vec![failed]
        };
        let end = observe(events).await;
        assert!(matches!(&end, ServiceEnd::Failed { started, .. } if *started == did_start));
        assert!(!end.needs_cancel());
        assert!(finish("service", end, clean()).is_err());
    }
}

#[tokio::test]
async fn foreground_interruptions_preserve_reason_phase_and_independent_exit() {
    let reasons = [
        ExecInterruptionReason::Timeout(Duration::from_secs(3)),
        ExecInterruptionReason::Cancelled,
        ExecInterruptionReason::OutputLimit,
        ExecInterruptionReason::TransportClosed,
        ExecInterruptionReason::Protocol,
        ExecInterruptionReason::Delivery,
    ];
    for reason in reasons {
        for did_start in [false, true] {
            for confirmed in [false, true] {
                let termination = if confirmed {
                    ExecTermination::Exited(0)
                } else {
                    ExecTermination::Unconfirmed
                };
                let event = ExecEvent::Interrupted(interruption(reason.clone(), termination));
                let events = if did_start {
                    vec![started(), event]
                } else {
                    vec![event]
                };
                let end = observe(events).await;
                let ServiceEnd::Interrupted { value, started } = &end else {
                    panic!("lost interruption");
                };
                assert_eq!(value.reason, reason);
                assert_eq!(*started, did_start);
                assert_eq!(
                    matches!(value.termination, ExecTermination::Exited(0)),
                    confirmed
                );
                assert!(!end.needs_cancel());
                assert!(finish("service", end, clean()).is_err());
            }
        }
    }
}

#[tokio::test]
async fn foreground_eof_never_implies_success() {
    for did_start in [false, true] {
        let events = if did_start { vec![started()] } else { vec![] };
        let end = observe(events).await;
        assert!(matches!(&end, ServiceEnd::Eof { started } if *started == did_start));
        assert!(end.needs_cancel());
        assert!(finish("service", end, clean()).is_err());
    }
}

#[tokio::test]
async fn foreground_stdin_failure_requires_original_session_cleanup() {
    let end = observe([
        started(),
        ExecEvent::StdinError(ExecStdinError {
            errno: Some(32),
            errno_name: None,
            message: "synthetic pipe error".into(),
        }),
    ])
    .await;
    assert!(matches!(&end, ServiceEnd::InputFailure(_)));
    assert!(end.needs_cancel());
    assert!(finish("service", end, clean()).is_err());
}

#[tokio::test]
async fn foreground_unexpected_event_order_fails() {
    for events in [
        vec![ExecEvent::Stdout(vec![1].into())],
        vec![started(), started()],
    ] {
        let end = observe(events).await;
        assert!(matches!(end, ServiceEnd::UnexpectedEvent));
    }
}

#[tokio::test(start_paused = true)]
async fn foreground_startup_deadline_does_not_detach_the_event_owner() {
    let mut events = Events::new([]);
    events.eof = false;
    let drops = events.drops.clone();
    let before = Instant::now();
    let end = supervise(&mut events, pending(), before + STARTUP_TIMEOUT, |_| {}).await;
    assert!(matches!(end, ServiceEnd::StartupTimeout));
    assert_eq!(Instant::now() - before, STARTUP_TIMEOUT);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    // The same receiver is still available after the timed-out borrowed future.
    events.queued.push_back(ExecEvent::Exited { code: 9 });
    assert!(matches!(
        events.next_event().await,
        Some(ExecEvent::Exited { code: 9 })
    ));
    drop(events);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn foreground_shutdown_before_started_leaves_receiver_owned() {
    let mut events = Events::new([started()]);
    let end = supervise(
        &mut events,
        async { Ok(()) },
        Instant::now() + STARTUP_TIMEOUT,
        |_| {},
    )
    .await;
    assert!(matches!(end, ServiceEnd::ShutdownRequested));
    assert!(end.needs_cancel());
    assert_eq!(events.queued.len(), 1);
}

#[tokio::test]
async fn foreground_shutdown_is_not_starved_by_ready_log_output() {
    let mut events = Events::new([started(), ExecEvent::Stdout(vec![1; 1024].into())]);
    let (send, recv) = tokio::sync::oneshot::channel();
    let mut send = Some(send);
    let end = supervise(
        &mut events,
        async { recv.await.map_err(|_| std::io::Error::other("signal test")) },
        Instant::now() + STARTUP_TIMEOUT,
        |_| {
            send.take().unwrap().send(()).unwrap();
        },
    )
    .await;
    assert!(matches!(end, ServiceEnd::ShutdownRequested));
    assert_eq!(events.queued.len(), 1);
}

#[tokio::test(start_paused = true)]
async fn foreground_healthy_service_has_no_startup_lifetime_limit() {
    let mut events = Events::new([started()]);
    events.eof = false;
    let before = Instant::now();
    let end = supervise(
        &mut events,
        async {
            tokio::time::sleep(STARTUP_TIMEOUT * 3).await;
            Ok(())
        },
        before + STARTUP_TIMEOUT,
        |_| {},
    )
    .await;
    assert!(matches!(end, ServiceEnd::ShutdownRequested));
    assert_eq!(Instant::now() - before, STARTUP_TIMEOUT * 3);
}

#[tokio::test]
async fn foreground_logs_stream_before_terminal_without_collection() {
    let mut events = Events::new([
        started(),
        ExecEvent::Stdout(vec![0, 255, 10].into()),
        ExecEvent::Stderr(vec![2, 3].into()),
        ExecEvent::Exited { code: 1 },
    ]);
    let mut bytes = 0;
    let end = supervise(
        &mut events,
        pending(),
        Instant::now() + STARTUP_TIMEOUT,
        |event| {
            if let ExecEvent::Stdout(data) | ExecEvent::Stderr(data) = event {
                bytes += data.len();
            }
        },
    )
    .await;
    assert_eq!(bytes, 5);
    assert!(matches!(end, ServiceEnd::Exited { code: 1, .. }));
}

#[tokio::test]
async fn foreground_signal_failure_is_a_primary_failure() {
    let mut events = Events::new([started()]);
    let end = supervise(
        &mut events,
        async { Err(std::io::Error::other("synthetic signal error")) },
        Instant::now() + STARTUP_TIMEOUT,
        |_| {},
    )
    .await;
    assert!(end.needs_cancel());
    assert!(
        finish("service", end, clean())
            .unwrap_err()
            .to_string()
            .contains("synthetic signal error")
    );
}

#[tokio::test(start_paused = true)]
async fn foreground_cancelling_supervision_does_not_drop_or_detach_receiver() {
    let mut events = Events::new([started()]);
    events.eof = false;
    let drops = events.drops.clone();
    assert!(
        tokio::time::timeout(
            Duration::from_secs(1),
            supervise(
                &mut events,
                pending(),
                Instant::now() + STARTUP_TIMEOUT,
                |_| {},
            )
        )
        .await
        .is_err()
    );
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    drop(events);
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

#[test]
fn foreground_requested_shutdown_requires_observed_cleanup() {
    let mut cleanup = clean();
    cleanup.cancellation = Cancellation::Observed(interruption(
        ExecInterruptionReason::Cancelled,
        ExecTermination::Exited(137),
    ));
    assert!(finish("service", ServiceEnd::ShutdownRequested, cleanup).is_ok());
    for cancellation in [
        Cancellation::TimedOut,
        Cancellation::Observed(interruption(
            ExecInterruptionReason::Cancelled,
            ExecTermination::Unconfirmed,
        )),
        Cancellation::Observed(interruption(
            ExecInterruptionReason::TransportClosed,
            ExecTermination::Exited(0),
        )),
    ] {
        let mut cleanup = clean();
        cleanup.cancellation = cancellation;
        assert!(finish("service", ServiceEnd::ShutdownRequested, cleanup).is_err());
    }
}

#[test]
fn foreground_primary_and_all_cleanup_failures_are_preserved() {
    let error = finish(
        "synthetic service",
        ServiceEnd::Exited {
            code: 19,
            started: true,
        },
        Cleanup {
            cancellation: Cancellation::TimedOut,
            stop: Err("synthetic stop error".into()),
            shim: Err("synthetic shim deadline".into()),
        },
    )
    .unwrap_err()
    .to_string();
    for expected in [
        "code 19",
        "cancellation observation timed out",
        "synthetic stop error",
        "synthetic shim deadline",
    ] {
        assert!(error.contains(expected), "missing {expected}: {error}");
    }
}

#[test]
fn foreground_start_request_error_is_not_lost_after_cleanup() {
    let error = finish(
        "service",
        ServiceEnd::StartRequestFailed("synthetic dispatch error".into()),
        clean(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("synthetic dispatch error"));
}

#[test]
fn foreground_success_never_hides_stop_or_retirement_timeout() {
    for (stop, shim) in [
        (Err("deadline; stopped state unconfirmed".into()), Ok(())),
        (Ok(()), Err("deadline; retirement unconfirmed".into())),
    ] {
        assert!(
            finish(
                "service",
                ServiceEnd::ShutdownRequested,
                Cleanup {
                    cancellation: Cancellation::NotNeeded,
                    stop,
                    shim,
                }
            )
            .is_err()
        );
    }
}
