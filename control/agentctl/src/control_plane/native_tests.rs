//! Native event/option projections use the real SDK types, without a VM,
//! name lookup or guest process. Transport and launch proofs have separate SDK
//! tests; these controls do not replace an actual retained-launch integration.
//! Listener controls use owned Unix sockets and injected native proof results,
//! never a fabricated SDK launch or a VM.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use microsandbox::ExecEvent as SdkEvent;
use microsandbox::protocol::exec::{ExecFailed, ExecFailureKind, ExecStdinError};
use microsandbox::sandbox::exec::StdinMode;
use std::collections::VecDeque;

fn launch(byte: u8) -> LaunchRef {
    LaunchRef {
        instance: InstanceRef {
            workload: super::super::types::WorkloadRef {
                context: None,
                name: "worker".into(),
            },
            instance: "one".into(),
        },
        generation: OpaqueId::from_bytes([byte; 32]),
    }
}

#[cfg(target_os = "linux")]
mod listener_tests {
    use super::*;
    use std::io;
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
    use std::os::unix::net::UnixListener;
    use std::path::Path;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::time::{Instant, timeout};

    struct Fixture {
        directory: tempfile::TempDir,
        listener: UnixListener,
        route: ProtectedHostListener,
    }

    impl Fixture {
        fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700))
                .unwrap();
            let path = directory.path().join("management.sock");
            let listener = UnixListener::bind(&path).unwrap();
            listener.set_nonblocking(true).unwrap();
            let route = ProtectedHostListener::capture(launch(1), &path).unwrap();
            Self {
                directory,
                listener,
                route,
            }
        }

        fn untouched(&self) {
            assert_eq!(
                self.listener.accept().unwrap_err().kind(),
                io::ErrorKind::WouldBlock
            );
        }

        async fn accepted(&self) -> tokio::net::UnixStream {
            let listener =
                tokio::net::UnixListener::from_std(self.listener.try_clone().unwrap()).unwrap();
            timeout(Duration::from_secs(1), listener.accept())
                .await
                .unwrap()
                .unwrap()
                .0
        }

        async fn closed_without_bytes(&self) {
            let mut accepted = self.accepted().await;
            let mut byte = [0; 1];
            assert_eq!(
                timeout(Duration::from_secs(1), accepted.read(&mut byte))
                    .await
                    .unwrap()
                    .unwrap(),
                0
            );
        }
    }

    fn deadline() -> Instant {
        Instant::now() + Duration::from_secs(2)
    }

    fn peer() -> std::future::Ready<Result<u32, ControlError>> {
        std::future::ready(Ok(std::process::id()))
    }

    #[test]
    fn native_listener_capture_requires_absolute_owned_private_socket() {
        let fixture = Fixture::new();
        for path in [Path::new(""), Path::new("management.sock"), Path::new("/")] {
            assert!(matches!(
                ProtectedHostListener::capture(launch(1), path),
                Err(ControlError::InvalidRequest)
            ));
        }
        let regular = fixture.directory.path().join("regular");
        std::fs::write(&regular, b"not a socket").unwrap();
        assert!(matches!(
            ProtectedHostListener::capture(launch(1), &regular),
            Err(ControlError::PermissionDenied)
        ));
        let metadata = std::fs::symlink_metadata(&fixture.route.path).unwrap();
        assert_eq!(
            listener_socket_identity(&metadata, metadata.uid().wrapping_add(1)),
            Err(ControlError::PermissionDenied)
        );
        let directory = std::fs::symlink_metadata(fixture.directory.path()).unwrap();
        assert_eq!(
            private_listener_directory(&directory, directory.uid().wrapping_add(1)),
            Err(ControlError::PermissionDenied)
        );
        fixture.untouched();
    }

    #[test]
    fn native_listener_capture_rejects_nonprivate_directory_and_symlinks() {
        let fixture = Fixture::new();
        std::fs::set_permissions(
            fixture.directory.path(),
            std::fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        assert!(matches!(
            ProtectedHostListener::capture(launch(1), &fixture.route.path),
            Err(ControlError::PermissionDenied)
        ));
        std::fs::set_permissions(
            fixture.directory.path(),
            std::fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        let alias = fixture.directory.path().join("alias.sock");
        symlink(&fixture.route.path, &alias).unwrap();
        assert!(matches!(
            ProtectedHostListener::capture(launch(1), &alias),
            Err(ControlError::PermissionDenied)
        ));
        let other = tempfile::tempdir().unwrap();
        let parent_alias = other.path().join("parent");
        symlink(fixture.directory.path(), &parent_alias).unwrap();
        assert!(matches!(
            ProtectedHostListener::capture(launch(1), &parent_alias.join("management.sock")),
            Err(ControlError::PermissionDenied)
        ));
        fixture.untouched();
    }

    #[test]
    fn native_listener_token_refuses_wrong_launch_without_invalidating_original() {
        let fixture = Fixture::new();
        assert_eq!(
            fixture.route.check_current(&launch(2)),
            Err(ControlError::StaleLaunch)
        );
        fixture.route.check_current(&launch(1)).unwrap();
        fixture.untouched();
    }

    #[test]
    fn native_listener_observed_replacement_cannot_revive_original_token() {
        let fixture = Fixture::new();
        let moved = fixture.directory.path().join("original.sock");
        std::fs::rename(&fixture.route.path, &moved).unwrap();
        let replacement = UnixListener::bind(&fixture.route.path).unwrap();
        assert_eq!(
            fixture.route.check_current(&launch(1)),
            Err(ControlError::StaleLaunch)
        );
        drop(replacement);
        std::fs::remove_file(&fixture.route.path).unwrap();
        std::fs::rename(&moved, &fixture.route.path).unwrap();
        assert_eq!(
            fixture.route.check_current(&launch(1)),
            Err(ControlError::StaleLaunch)
        );
        fixture.untouched();
    }

    #[test]
    fn native_listener_parent_replacement_is_not_followed() {
        let outer = tempfile::tempdir().unwrap();
        let parent = outer.path().join("private");
        std::fs::create_dir(&parent).unwrap();
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
        let path = parent.join("management.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let route = ProtectedHostListener::capture(launch(1), &path).unwrap();
        let old_parent = outer.path().join("original");
        std::fs::rename(&parent, &old_parent).unwrap();
        std::fs::create_dir(&parent).unwrap();
        std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o700)).unwrap();
        let replacement = UnixListener::bind(&path).unwrap();
        assert_eq!(
            route.check_current(&launch(1)),
            Err(ControlError::StaleLaunch)
        );
        use std::os::fd::AsRawFd;
        assert_eq!(
            std::fs::read_link(format!("/proc/self/fd/{}", route.directory.as_raw_fd())).unwrap(),
            old_parent
        );
        drop((replacement, listener));
    }

    #[test]
    fn native_listener_requires_running_state_and_known_native_peer() {
        for status in [
            SandboxStatus::Created,
            SandboxStatus::Starting,
            SandboxStatus::Paused,
            SandboxStatus::Draining,
            SandboxStatus::Stopped,
            SandboxStatus::Crashed,
        ] {
            assert_eq!(
                running_listener_peer(status, Some(1)),
                Err(ControlError::RuntimeUnavailable)
            );
        }
        assert_eq!(
            running_listener_peer(SandboxStatus::Running, None),
            Err(ControlError::UnsupportedCapability)
        );
        assert_eq!(
            running_listener_peer(SandboxStatus::Running, Some(0)),
            Err(ControlError::UnsupportedCapability)
        );
        assert_eq!(
            running_listener_peer(SandboxStatus::Running, Some(1)),
            Ok(1)
        );
    }

    #[tokio::test]
    async fn native_listener_original_stream_has_no_bytes_before_verified_return() {
        let fixture = Fixture::new();
        let mut proofs = 0;
        let mut stream = connect_listener_checked(&launch(1), &fixture.route, deadline(), || {
            proofs += 1;
            peer()
        })
        .await
        .unwrap();
        assert_eq!(proofs, 2);
        let mut accepted = fixture.accepted().await;
        assert_eq!(
            accepted.try_read(&mut [0; 1]).unwrap_err().kind(),
            io::ErrorKind::WouldBlock
        );
        stream.write_all(b"verified").await.unwrap();
        let mut bytes = [0; 8];
        timeout(Duration::from_secs(1), accepted.read_exact(&mut bytes))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(&bytes, b"verified");
    }

    #[tokio::test]
    async fn native_listener_reconnect_reuses_token_but_rechecks_native_proof() {
        let fixture = Fixture::new();
        for _ in 0..2 {
            let stream = connect_listener_checked(&launch(1), &fixture.route, deadline(), peer)
                .await
                .unwrap();
            drop(stream);
            fixture.closed_without_bytes().await;
        }
        let result = connect_listener_checked(&launch(1), &fixture.route, deadline(), || {
            std::future::ready(Err(ControlError::StaleLaunch))
        })
        .await;
        assert!(matches!(result, Err(ControlError::StaleLaunch)));
        fixture.untouched();
    }

    #[tokio::test]
    async fn native_listener_wrong_peer_is_closed_without_application_bytes() {
        let fixture = Fixture::new();
        let mut proofs = 0;
        let result = connect_listener_checked(&launch(1), &fixture.route, deadline(), || {
            proofs += 1;
            std::future::ready(Ok(std::process::id() + 1))
        })
        .await;
        assert!(matches!(result, Err(ControlError::PermissionDenied)));
        assert_eq!(proofs, 1);
        fixture.closed_without_bytes().await;
    }

    #[tokio::test]
    async fn native_listener_preflight_refusal_has_no_connection_effect() {
        for error in [
            ControlError::UnknownLaunch,
            ControlError::StaleLaunch,
            ControlError::RuntimeUnavailable,
            ControlError::UnsupportedCapability,
        ] {
            let fixture = Fixture::new();
            let result = connect_listener_checked(&launch(1), &fixture.route, deadline(), || {
                std::future::ready(Err(error))
            })
            .await;
            assert!(matches!(result, Err(actual) if actual == error));
            fixture.untouched();
        }
    }

    #[tokio::test]
    async fn native_listener_postconnect_refusal_closes_without_bytes() {
        for error in [
            ControlError::UnknownLaunch,
            ControlError::StaleLaunch,
            ControlError::RuntimeUnavailable,
            ControlError::UnsupportedCapability,
        ] {
            let fixture = Fixture::new();
            let mut proofs = VecDeque::from([Ok(std::process::id()), Err(error)]);
            let result = connect_listener_checked(&launch(1), &fixture.route, deadline(), || {
                std::future::ready(proofs.pop_front().unwrap())
            })
            .await;
            assert!(matches!(result, Err(actual) if actual == error));
            fixture.closed_without_bytes().await;
        }
    }

    #[tokio::test]
    async fn native_listener_changed_postconnect_peer_refuses() {
        let fixture = Fixture::new();
        let mut proofs = VecDeque::from([Ok(std::process::id()), Ok(std::process::id() + 1)]);
        let result = connect_listener_checked(&launch(1), &fixture.route, deadline(), || {
            std::future::ready(proofs.pop_front().unwrap())
        })
        .await;
        assert!(matches!(result, Err(ControlError::StaleLaunch)));
        fixture.closed_without_bytes().await;
    }

    #[tokio::test]
    async fn native_listener_replacement_during_postproof_refuses_before_bytes() {
        let fixture = Fixture::new();
        let mut proofs = 0;
        let mut replacement = None;
        let result = connect_listener_checked(&launch(1), &fixture.route, deadline(), || {
            proofs += 1;
            if proofs == 2 {
                std::fs::rename(
                    &fixture.route.path,
                    fixture.directory.path().join("old.sock"),
                )
                .unwrap();
                replacement = Some(UnixListener::bind(&fixture.route.path).unwrap());
            }
            peer()
        })
        .await;
        assert!(matches!(result, Err(ControlError::StaleLaunch)));
        fixture.closed_without_bytes().await;
        drop(replacement);
    }

    #[tokio::test]
    async fn native_listener_expired_deadline_does_not_call_proof_or_connect() {
        let fixture = Fixture::new();
        let result = connect_listener_checked(&launch(1), &fixture.route, Instant::now(), || {
            panic!("expired connection must not call native proof");
            #[allow(unreachable_code)]
            peer()
        })
        .await;
        assert!(matches!(result, Err(ControlError::RuntimeUnavailable)));
        fixture.untouched();
    }

    #[tokio::test]
    async fn native_listener_deadline_during_postproof_closes_connection() {
        let fixture = Fixture::new();
        let mut proofs = 0;
        let result = connect_listener_checked(&launch(1), &fixture.route, deadline(), || {
            proofs += 1;
            let current = proofs;
            async move {
                if current == 2 {
                    // Advance only after real socket establishment; no
                    // wall-clock scheduling race determines the oracle.
                    tokio::time::pause();
                    tokio::time::advance(Duration::from_secs(3)).await;
                    std::future::pending::<()>().await;
                }
                Ok(std::process::id())
            }
        })
        .await;
        tokio::time::resume();
        assert!(matches!(result, Err(ControlError::RuntimeUnavailable)));
        assert_eq!(proofs, 2);
        fixture.closed_without_bytes().await;
    }

    #[tokio::test]
    async fn native_listener_cancel_during_postproof_drops_original_connection() {
        let fixture = Fixture::new();
        let reached = Arc::new(tokio::sync::Notify::new());
        let mut proofs = 0;
        let selected = launch(1);
        {
            let connection =
                connect_listener_checked(&selected, &fixture.route, deadline(), || {
                    proofs += 1;
                    let current = proofs;
                    let reached = reached.clone();
                    async move {
                        if current == 2 {
                            reached.notify_one();
                            std::future::pending::<()>().await;
                        }
                        Ok(std::process::id())
                    }
                });
            tokio::pin!(connection);
            tokio::select! {
                result = &mut connection => panic!("unexpected completion: {result:?}"),
                _ = reached.notified() => {}
                _ = tokio::time::sleep(Duration::from_secs(1)) => panic!("postproof not reached"),
            }
        }
        assert_eq!(proofs, 2);
        fixture.closed_without_bytes().await;
    }
}

fn failure() -> ExecFailed {
    ExecFailed {
        kind: ExecFailureKind::Other,
        errno: None,
        errno_name: Some("private diagnostic".into()),
        message: "private native diagnostic".into(),
        stage: Some("private native stage".into()),
    }
}

fn poll(drain: &mut EventDrain, events: &mut VecDeque<SdkEvent>, bytes: usize) -> Vec<ExecEvent> {
    drain
        .poll(bytes, || events.pop_front().ok_or(TryRecvError::Empty))
        .unwrap()
}

#[test]
fn native_capabilities_are_linux_only_not_readiness() {
    let value = MicrosandboxControl::supported_capabilities();
    assert_eq!(value.launch_bound_exec, cfg!(target_os = "linux"));
    assert_eq!(value.stdin, value.launch_bound_exec);
    assert_eq!(value.pty, value.launch_bound_exec);
    assert_eq!(value.cancellation, value.launch_bound_exec);
    assert_eq!(value.launch_bound_lifecycle, value.launch_bound_exec);
}

#[test]
fn native_association_requires_original_arc_and_exact_name() {
    let owner = Arc::new(());
    let retained = owner.clone();
    let other = Arc::new(());
    assert!(validate_association(Arc::ptr_eq(&owner, &retained), "a", "a").is_ok());
    assert_eq!(
        validate_association(Arc::ptr_eq(&owner, &other), "a", "a"),
        Err(ControlError::StaleLaunch)
    );
    assert_eq!(
        validate_association(true, "a", "b"),
        Err(ControlError::StaleLaunch)
    );
}

#[test]
fn native_selection_never_replaces_an_issued_generation() {
    let old = launch(1);
    let next = launch(2);
    let mut selected = BTreeMap::from([(old.clone(), "original")]);
    assert_eq!(
        admit_selection(&selected, &old),
        Err(ControlError::RevisionConflict)
    );
    admit_selection(&selected, &next).unwrap();
    selected.insert(next.clone(), "replacement");
    assert_eq!(selected[&old], "original");
    assert_eq!(selected[&next], "replacement");
}

#[test]
fn native_selection_capacity_does_not_evict_old_handles() {
    let selected: BTreeMap<_, _> = (0..MAX_RETAINED_LAUNCHES)
        .map(|i| (launch(i as u8), i))
        .collect();
    assert_eq!(
        admit_selection(&selected, &launch(200)),
        Err(ControlError::ResourceLimit)
    );
    assert_eq!(selected.len(), MAX_RETAINED_LAUNCHES);
    assert_eq!(selected[&launch(0)], 0);
}

#[test]
fn native_exec_options_preserve_literals_deadline_stdin_and_pty() {
    let options = exec_options(
        ExecOptionsBuilder::default(),
        ExecCommand {
            program: "literal-program".into(),
            args: vec!["$(not a shell)".into()],
            cwd: Some("/guest path".into()),
            env: BTreeMap::from([("VAR".into(), "$HOME".into())]),
            stdin: true,
            tty: true,
            timeout_ms: 2345,
        },
    )
    .build()
    .unwrap();
    assert_eq!(options.args, ["$(not a shell)"]);
    assert_eq!(options.cwd.as_deref(), Some("/guest path"));
    assert_eq!(options.env[0].key, "VAR");
    assert_eq!(options.env[0].value, "$HOME");
    assert_eq!(options.timeout, Some(Duration::from_millis(2345)));
    assert!(matches!(options.stdin, StdinMode::Pipe));
    assert!(options.tty);
    assert!(options.user.is_none() && options.rlimits.is_empty());
}

#[test]
fn native_noninteractive_command_keeps_null_stdin() {
    let options = exec_options(
        ExecOptionsBuilder::default(),
        ExecCommand {
            program: "literal".into(),
            args: vec![],
            cwd: None,
            env: BTreeMap::new(),
            stdin: false,
            tty: false,
            timeout_ms: 1,
        },
    )
    .build()
    .unwrap();
    assert!(matches!(options.stdin, StdinMode::Null));
    assert!(!options.tty);
    assert_eq!(options.timeout, Some(Duration::from_millis(1)));
}

#[test]
fn native_state_projection_does_not_call_paused_or_starting_running() {
    assert_eq!(
        lifecycle_state(SandboxStatus::Running),
        Ok(LifecycleState::Running)
    );
    assert_eq!(
        lifecycle_state(SandboxStatus::Draining),
        Ok(LifecycleState::Stopping)
    );
    for state in [SandboxStatus::Stopped, SandboxStatus::Crashed] {
        assert_eq!(lifecycle_state(state), Ok(LifecycleState::Stopped));
    }
    for state in [
        SandboxStatus::Created,
        SandboxStatus::Starting,
        SandboxStatus::Paused,
    ] {
        assert_eq!(
            lifecycle_state(state),
            Err(ControlError::RuntimeUnavailable)
        );
    }
}

#[test]
fn native_errors_are_bounded_categories_without_diagnostic_text() {
    assert_eq!(
        native_error(MicrosandboxError::LaunchBindingUnsupported),
        ControlError::UnsupportedCapability
    );
    assert_eq!(
        native_error(MicrosandboxError::SandboxNotFound("private name".into())),
        ControlError::StaleLaunch
    );
    assert_eq!(
        native_error(MicrosandboxError::Runtime("private endpoint".into())),
        ControlError::RuntimeUnavailable
    );
}

#[test]
fn native_poll_splits_output_without_loss_or_terminal_overtaking() {
    let bytes: Vec<u8> = (0..17_000).map(|i| (i % 251) as u8).collect();
    let mut input = VecDeque::from([
        SdkEvent::Started { pid: 999 },
        SdkEvent::Stdout(bytes.clone().into()),
        SdkEvent::Stderr(vec![8, 9].into()),
        SdkEvent::Exited { code: 3 },
    ]);
    let mut drain = EventDrain::default();
    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut started = 0;
    let mut terminal = 0;
    while !drain.ended {
        let output = poll(&mut drain, &mut input, 997);
        let mut used = 0;
        for event in output {
            match event {
                ExecEvent::Started => started += 1,
                ExecEvent::Stdout { bytes } => {
                    used += bytes.len();
                    stdout.extend(bytes);
                }
                ExecEvent::Stderr { bytes } => {
                    used += bytes.len();
                    stderr.extend(bytes);
                }
                ExecEvent::Exited { code } => {
                    assert_eq!(code, 3);
                    assert_eq!(stdout, bytes);
                    assert_eq!(stderr, [8, 9]);
                    terminal += 1;
                }
                _ => panic!("unexpected event"),
            }
        }
        assert!(used <= 997);
    }
    assert_eq!((started, terminal), (1, 1));
    assert!(input.is_empty() && drain.pending.is_none());
}

#[test]
fn native_poll_has_independent_event_and_byte_limits() {
    let mut input = VecDeque::from(vec![SdkEvent::Stdout(Vec::new().into()); 200]);
    let mut drain = EventDrain::default();
    assert_eq!(poll(&mut drain, &mut input, 1).len(), MAX_POLL_EVENTS);
    assert_eq!(input.len(), 200 - MAX_POLL_EVENTS);
    let mut input = VecDeque::from([
        SdkEvent::Stdout(vec![1; 16].into()),
        SdkEvent::Exited { code: 0 },
    ]);
    let mut drain = EventDrain::default();
    assert!(
        matches!(&poll(&mut drain, &mut input, 8)[..], [ExecEvent::Stdout { bytes }] if bytes.len() == 8)
    );
    assert!(
        matches!(&poll(&mut drain, &mut input, 8)[..], [ExecEvent::Stdout { bytes }] if bytes.len() == 8)
    );
    assert!(!drain.ended);
    assert!(matches!(
        &poll(&mut drain, &mut input, 8)[..],
        [ExecEvent::Exited { code: 0 }]
    ));
}

#[test]
fn native_poll_empty_is_not_disconnect_or_process_exit() {
    let mut drain = EventDrain::default();
    assert!(
        drain
            .poll(8, || Err(TryRecvError::Empty))
            .unwrap()
            .is_empty()
    );
    assert!(!drain.ended);
    let events = drain.poll(8, || Err(TryRecvError::Disconnected)).unwrap();
    assert!(matches!(&events[..], [ExecEvent::TransportLost]));
    assert!(drain.ended);
    assert!(
        drain
            .poll(8, || panic!("closed receiver must not be read again"))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn native_poll_invalid_budget_does_not_consume_input() {
    for budget in [0, MAX_POLL_BYTES + 1] {
        assert!(matches!(
            EventDrain::default().poll(budget, || panic!("no read")),
            Err(ControlError::InvalidRequest)
        ));
    }
}

#[test]
fn native_poll_oversized_native_chunk_refuses_without_retained_allocation() {
    let mut input = VecDeque::from([SdkEvent::Stdout(vec![1; MAX_NATIVE_CHUNK + 1].into())]);
    let mut drain = EventDrain::default();
    let events = poll(&mut drain, &mut input, 8);
    assert!(matches!(
        &events[..],
        [ExecEvent::Interrupted {
            reason: ExecInterruptionReason::OutputLimit,
            termination: ExecTermination::Unconfirmed
        }]
    ));
    assert!(drain.ended && drain.delivery_failed && drain.pending.is_none());
}

#[test]
fn native_poll_slow_consumer_has_no_shared_mapper_queue() {
    let mut slow = EventDrain::default();
    let mut fast = EventDrain::default();
    let mut slow_input = VecDeque::from([SdkEvent::Stdout(vec![1; MAX_NATIVE_CHUNK].into())]);
    let mut fast_input = VecDeque::from([SdkEvent::Exited { code: 5 }]);
    poll(&mut slow, &mut slow_input, 1);
    assert!(slow.pending.is_some());
    assert!(matches!(
        &poll(&mut fast, &mut fast_input, 1)[..],
        [ExecEvent::Exited { code: 5 }]
    ));
    assert!(!slow.ended && fast.ended);
}

#[test]
fn native_stdin_failure_closes_writes_without_claiming_exit() {
    let mut input = VecDeque::from([
        SdkEvent::StdinError(ExecStdinError {
            errno: Some(32),
            errno_name: None,
            message: "private input error".into(),
        }),
        SdkEvent::Exited { code: 0 },
    ]);
    let mut drain = EventDrain::default();
    let events = poll(&mut drain, &mut input, 8);
    assert!(matches!(
        &events[..],
        [ExecEvent::Interrupted {
            reason: ExecInterruptionReason::Delivery,
            termination: ExecTermination::Unconfirmed
        }]
    ));
    assert!(drain.ended && drain.delivery_failed);
    assert_eq!(input.len(), 1);
    assert!(poll(&mut drain, &mut input, 8).is_empty());
}

#[test]
fn native_interruption_preserves_all_reasons_and_independent_termination() {
    use microsandbox::{ExecInterruption as I, ExecInterruptionReason as R, ExecTermination as T};
    for (source, reason) in [
        (
            R::Timeout(Duration::from_secs(1)),
            ExecInterruptionReason::Timeout,
        ),
        (R::Cancelled, ExecInterruptionReason::Cancelled),
        (R::OutputLimit, ExecInterruptionReason::OutputLimit),
        (R::TransportClosed, ExecInterruptionReason::TransportClosed),
        (R::Protocol, ExecInterruptionReason::Protocol),
        (R::Delivery, ExecInterruptionReason::Delivery),
    ] {
        for (native, termination) in [
            (T::Exited(0), ExecTermination::Exited { code: 0 }),
            (T::Exited(7), ExecTermination::Exited { code: 7 }),
            (T::SpawnFailed(failure()), ExecTermination::SpawnFailed),
            (T::Unconfirmed, ExecTermination::Unconfirmed),
        ] {
            let event = project_interruption(I {
                reason: source.clone(),
                termination: native,
            });
            assert!(
                matches!(event, ExecEvent::Interrupted { reason: got, termination: state } if got == reason && state == termination)
            );
        }
    }
}

#[test]
fn native_interrupted_terminal_never_becomes_zero_exit() {
    let mut input = VecDeque::from([SdkEvent::Interrupted(microsandbox::ExecInterruption {
        reason: microsandbox::ExecInterruptionReason::Timeout(Duration::from_millis(1)),
        termination: microsandbox::ExecTermination::Exited(0),
    })]);
    let mut drain = EventDrain::default();
    assert!(matches!(
        &poll(&mut drain, &mut input, 8)[..],
        [ExecEvent::Interrupted {
            reason: ExecInterruptionReason::Timeout,
            termination: ExecTermination::Exited { code: 0 }
        }]
    ));
    assert!(drain.ended);
}

#[test]
fn native_spawn_failure_omits_native_diagnostics() {
    let mut input = VecDeque::from([SdkEvent::Failed(failure())]);
    let mut drain = EventDrain::default();
    assert!(matches!(
        &poll(&mut drain, &mut input, 8)[..],
        [ExecEvent::SpawnFailed]
    ));
    assert!(drain.ended);
}

#[derive(Clone)]
struct DropProbe(Arc<std::sync::atomic::AtomicUsize>);

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

fn dropped(probe: &Arc<std::sync::atomic::AtomicUsize>) -> usize {
    probe.load(std::sync::atomic::Ordering::SeqCst)
}

#[tokio::test]
async fn native_failed_write_does_not_send_eof_or_allow_resend() {
    let calls = std::cell::RefCell::new(Vec::new());
    let mut handle = Some(());
    let mut stdin = Some(());
    let mut drain = EventDrain::default();
    {
        let _attempt = InputAttempt {
            handle: &mut handle,
            stdin: &mut stdin,
            events: &mut drain,
            committed: false,
        };
        assert_eq!(
            write_then_eof(true, true, |close| {
                calls.borrow_mut().push(close);
                std::future::ready(Err(ControlError::OperationIndeterminate))
            })
            .await,
            Err(ControlError::OperationIndeterminate)
        );
    }
    assert_eq!(*calls.borrow(), [false]);
    assert!(handle.is_none() && stdin.is_none());
    assert_eq!(
        validate_stdin(&drain, true, 1, false),
        Err(ControlError::OperationClosed)
    );
    assert!(matches!(
        &drain
            .poll(8, || panic!("failed stdin cannot redial"))
            .unwrap()[..],
        [ExecEvent::Interrupted {
            reason: ExecInterruptionReason::Delivery,
            termination: ExecTermination::Unconfirmed
        }]
    ));
}

#[tokio::test]
async fn native_failed_close_keeps_input_order_and_cannot_be_replayed() {
    let calls = std::cell::RefCell::new(Vec::new());
    let mut handle = Some(());
    let mut stdin = Some(());
    let mut drain = EventDrain::default();
    {
        let _attempt = InputAttempt {
            handle: &mut handle,
            stdin: &mut stdin,
            events: &mut drain,
            committed: false,
        };
        assert_eq!(
            write_then_eof(true, true, |close| {
                calls.borrow_mut().push(close);
                std::future::ready(if close {
                    Err(ControlError::RuntimeUnavailable)
                } else {
                    Ok(())
                })
            })
            .await,
            Err(ControlError::RuntimeUnavailable)
        );
    }
    assert_eq!(*calls.borrow(), [false, true]);
    assert_eq!(
        validate_stdin(&drain, stdin.is_some(), 0, true),
        Err(ControlError::OperationClosed)
    );
    assert!(drain.pending_delivery && drain.delivery_failed);
}

#[tokio::test]
async fn native_cancelled_input_drops_receiver_but_retains_control_lease() {
    let receiver_drops = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let stdin_drops = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let control_drops = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mut handle = Some(DropProbe(receiver_drops.clone()));
    let mut stdin = Some(DropProbe(stdin_drops.clone()));
    let control = DropProbe(control_drops.clone());
    let mut drain = EventDrain::default();
    let mut operation = Box::pin(async {
        let attempt = InputAttempt {
            handle: &mut handle,
            stdin: &mut stdin,
            events: &mut drain,
            committed: false,
        };
        write_then_eof(true, false, |_| {
            std::future::pending::<Result<(), ControlError>>()
        })
        .await?;
        attempt.commit(false);
        Ok::<_, ControlError>(())
    });
    std::future::poll_fn(|context| {
        assert!(operation.as_mut().poll(context).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    drop(operation);
    assert_eq!(dropped(&receiver_drops), 1);
    assert_eq!(dropped(&stdin_drops), 1);
    assert_eq!(dropped(&control_drops), 0);
    assert!(drain.pending_delivery && drain.delivery_failed && !drain.ended);
    drop(control);
    assert_eq!(dropped(&control_drops), 1);
}

#[test]
fn native_input_failure_drains_retained_partial_output_before_interruption() {
    let mut drain = EventDrain::default();
    let mut input = VecDeque::from([SdkEvent::Stdout(vec![1, 2, 3, 4, 5].into())]);
    assert!(
        matches!(&poll(&mut drain, &mut input, 2)[..], [ExecEvent::Stdout { bytes }] if bytes == &[1, 2])
    );
    let mut handle = Some(());
    let mut stdin = Some(());
    drop(InputAttempt {
        handle: &mut handle,
        stdin: &mut stdin,
        events: &mut drain,
        committed: false,
    });
    let output = drain
        .poll(8, || {
            panic!("only retained partial chunk and failure may be read")
        })
        .unwrap();
    assert!(
        matches!(&output[..], [ExecEvent::Stdout { bytes }, ExecEvent::Interrupted {
        reason: ExecInterruptionReason::Delivery, termination: ExecTermination::Unconfirmed
    }] if bytes == &[3, 4, 5])
    );
    assert!(drain.ended && drain.pending.is_none());
}

#[tokio::test]
async fn native_successful_input_keeps_lease_and_closes_only_completed_eof() {
    for eof in [false, true] {
        let calls = std::cell::RefCell::new(Vec::new());
        let mut handle = Some(());
        let mut stdin = Some(());
        let mut drain = EventDrain::default();
        let attempt = InputAttempt {
            handle: &mut handle,
            stdin: &mut stdin,
            events: &mut drain,
            committed: false,
        };
        write_then_eof(true, eof, |close| {
            calls.borrow_mut().push(close);
            std::future::ready(Ok(()))
        })
        .await
        .unwrap();
        attempt.commit(eof);
        assert!(handle.is_some());
        assert_eq!(stdin.is_none(), eof);
        assert!(!drain.delivery_failed && !drain.pending_delivery && !drain.ended);
        assert_eq!(
            *calls.borrow(),
            if eof { vec![false, true] } else { vec![false] }
        );
    }
}

#[tokio::test]
async fn native_eof_without_payload_never_invents_an_empty_write() {
    let calls = std::cell::RefCell::new(Vec::new());
    write_then_eof(false, true, |close| {
        calls.borrow_mut().push(close);
        std::future::ready(Ok(()))
    })
    .await
    .unwrap();
    assert_eq!(*calls.borrow(), [true]);
}
