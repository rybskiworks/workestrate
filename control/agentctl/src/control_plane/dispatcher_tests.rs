//! Public-request controls using a synthetic retained native session. These
//! tests do not run a VM or claim that the pinned SDK implements this contract.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::{Arc, Mutex};

use super::authorization::{AuthenticatedCaller, CallerIdentity, Permission, ResourceScope};
use super::dispatcher::ControlDispatcher;
use super::native::{MicrosandboxControl, NativeControl, NativeExec, NativeFuture};
use super::types::*;

#[derive(Default)]
struct Fixture {
    current: BTreeSet<LaunchRef>,
    calls: Vec<&'static str>,
    events: VecDeque<Vec<ExecEvent>>,
    polled: Vec<LaunchRef>,
    input: Vec<u8>,
    hang_start: bool,
    hang_cancel: bool,
    reject_start: bool,
}

struct Guest {
    state: Arc<Mutex<Fixture>>,
    capabilities: RuntimeCapabilities,
}

struct Session(Arc<Mutex<Fixture>>, LaunchRef);

impl NativeControl for Guest {
    fn capabilities(&self) -> RuntimeCapabilities {
        self.capabilities
    }

    fn verify_launch<'a>(&'a self, launch: &'a LaunchRef) -> NativeFuture<'a, ()> {
        Box::pin(async move {
            let mut state = self.state.lock().unwrap();
            state.calls.push("verify");
            if state.current.contains(launch) {
                Ok(())
            } else {
                Err(ControlError::StaleLaunch)
            }
        })
    }

    fn start<'a>(
        &'a mut self,
        launch: &'a LaunchRef,
        _command: ExecCommand,
    ) -> NativeFuture<'a, Box<dyn NativeExec>> {
        Box::pin(async move {
            let hang = {
                let mut state = self.state.lock().unwrap();
                state.calls.push("start");
                if state.reject_start || !state.current.contains(launch) {
                    return Err(ControlError::StaleLaunch);
                }
                state.hang_start
            };
            if hang {
                std::future::pending::<()>().await;
            }
            Ok(Box::new(Session(self.state.clone(), launch.clone())) as Box<dyn NativeExec>)
        })
    }

    fn lifecycle<'a>(
        &'a mut self,
        launch: &'a LaunchRef,
        operation: LifecycleOperation,
    ) -> NativeFuture<'a, LifecycleState> {
        Box::pin(async move {
            let mut state = self.state.lock().unwrap();
            if !state.current.contains(launch) {
                return Err(ControlError::StaleLaunch);
            }
            state.calls.push("lifecycle");
            Ok(match operation {
                LifecycleOperation::Inspect => LifecycleState::Running,
                LifecycleOperation::Stop => LifecycleState::Stopping,
            })
        })
    }
}

impl NativeExec for Session {
    fn poll(&mut self, _max_bytes: usize) -> Result<Vec<ExecEvent>, ControlError> {
        let mut state = self.0.lock().unwrap();
        state.calls.push("poll");
        state.polled.push(self.1.clone());
        Ok(state.events.pop_front().unwrap_or_default())
    }

    fn stdin<'a>(&'a mut self, bytes: &'a [u8], _eof: bool) -> NativeFuture<'a, ()> {
        Box::pin(async move {
            let mut state = self.0.lock().unwrap();
            state.calls.push("stdin");
            state.input.extend_from_slice(bytes);
            Ok(())
        })
    }

    fn resize(&mut self, _rows: u16, _columns: u16) -> NativeFuture<'_, ()> {
        Box::pin(async move {
            self.0.lock().unwrap().calls.push("resize");
            Ok(())
        })
    }

    fn cancel(&mut self) -> NativeFuture<'_, ()> {
        Box::pin(async move {
            let hang = {
                let mut state = self.0.lock().unwrap();
                state.calls.push("cancel");
                state.hang_cancel
            };
            if hang {
                std::future::pending::<()>().await;
            }
            Ok(())
        })
    }
}

fn launch(generation: u8) -> LaunchRef {
    LaunchRef {
        instance: InstanceRef {
            workload: WorkloadRef {
                context: Some("fixture".into()),
                name: "worker".into(),
            },
            instance: "fixture-worker".into(),
        },
        generation: OpaqueId::from_bytes([generation; 32]),
    }
}

fn command() -> ExecCommand {
    ExecCommand {
        program: "/bin/fixture".into(),
        args: Vec::new(),
        cwd: Some("/work".into()),
        env: BTreeMap::new(),
        stdin: false,
        tty: false,
        timeout_ms: 1000,
    }
}

fn fixture() -> (
    Arc<Mutex<Fixture>>,
    ControlDispatcher<Guest>,
    LaunchRef,
    AuthenticatedCaller,
) {
    let target = launch(1);
    let state = Arc::new(Mutex::new(Fixture {
        current: BTreeSet::from([target.clone()]),
        ..Default::default()
    }));
    let guest = Guest {
        state: state.clone(),
        capabilities: RuntimeCapabilities {
            launch_bound_exec: true,
            stdin: true,
            pty: true,
            cancellation: true,
            launch_bound_lifecycle: true,
        },
    };
    let mut dispatcher = ControlDispatcher::new(OpaqueId::from_bytes([9; 32]), guest);
    register(&mut dispatcher, &target);
    (
        state,
        dispatcher,
        target,
        AuthenticatedCaller::local_operator(1000),
    )
}

fn register(dispatcher: &mut ControlDispatcher<Guest>, target: &LaunchRef) {
    let credentials = crate::microsandbox::plan::CredentialsPlan {
        ssh: Vec::new(),
        signing: Vec::new(),
        strict: true,
        strict_origin: None,
    };
    dispatcher
        .custody_mut()
        .register_launch(target.clone(), &credentials, None)
        .unwrap();
    dispatcher.custody_mut().confirm_launch(target).unwrap();
}

fn request(target: &LaunchRef, request: ExecRequest) -> ControlRequest {
    ControlRequest::GuestExec {
        launch: target.clone(),
        request,
    }
}

async fn exec(
    dispatcher: &mut ControlDispatcher<Guest>,
    caller: &AuthenticatedCaller,
    target: &LaunchRef,
    operation: ExecRequest,
) -> ExecStatus {
    let outcome = dispatcher
        .dispatch(caller, request(target, operation))
        .await
        .unwrap();
    assert!(outcome.custody_transaction.is_none());
    match outcome.response {
        ControlResponse::GuestExec(status) => status,
        _ => panic!("wrong capability response"),
    }
}

async fn start(
    dispatcher: &mut ControlDispatcher<Guest>,
    caller: &AuthenticatedCaller,
    target: &LaunchRef,
) -> OperationRef {
    exec(
        dispatcher,
        caller,
        target,
        ExecRequest::Start { command: command() },
    )
    .await
    .id
}

fn reader(uid: u32, target: &LaunchRef) -> AuthenticatedCaller {
    AuthenticatedCaller::restricted(
        CallerIdentity::LocalUid(uid),
        vec![Permission {
            resource: ResourceScope::Launch(target.clone()),
            operations: BTreeSet::from([ControlOperation::GuestExec(ExecOperation::Read)]),
        }],
    )
}

#[test]
fn consumed_native_sdk_does_not_claim_strict_capabilities() {
    assert_eq!(
        MicrosandboxControl::supported_capabilities(),
        RuntimeCapabilities::default()
    );
}

#[tokio::test]
async fn configured_custody_incarnation_fences_execution_handles() {
    let (state, _, target, caller) = fixture();
    let incarnation = OpaqueId::from_bytes([42; 32]);
    let custody = super::custody::SshController::new(incarnation.clone());
    let mut dispatcher = ControlDispatcher::with_custody(
        Guest {
            state,
            capabilities: RuntimeCapabilities {
                launch_bound_exec: true,
                cancellation: true,
                ..RuntimeCapabilities::default()
            },
        },
        custody,
    );
    register(&mut dispatcher, &target);
    let id = start(&mut dispatcher, &caller, &target).await;
    assert_eq!(id.controller, incarnation);
    assert_eq!(id.sequence, 1);
    assert_eq!(
        ControlError::StateUnavailable.to_string(),
        "control state is not available"
    );
}

#[tokio::test]
async fn unsupported_native_methods_refuse_before_discovery() {
    let (state, _, target, caller) = fixture();
    let mut dispatcher = ControlDispatcher::new(
        OpaqueId::from_bytes([9; 32]),
        Guest {
            state: state.clone(),
            capabilities: MicrosandboxControl::supported_capabilities(),
        },
    );
    for request in [
        request(&target, ExecRequest::Start { command: command() }),
        ControlRequest::Lifecycle {
            launch: target.clone(),
            operation: LifecycleOperation::Stop,
        },
    ] {
        assert_eq!(
            dispatcher.dispatch(&caller, request).await.map(|_| ()),
            Err(ControlError::UnsupportedCapability)
        );
    }
    let response = dispatcher
        .dispatch(&caller, ControlRequest::Capabilities { launch: target })
        .await
        .unwrap()
        .response;
    assert!(
        matches!(response, ControlResponse::Capabilities { runtime, ssh_custody: true } if runtime == RuntimeCapabilities::default())
    );
    assert!(state.lock().unwrap().calls.is_empty());
}

#[tokio::test]
async fn ssh_permission_never_grants_exec_or_lifecycle() {
    let (state, mut dispatcher, target, _) = fixture();
    let caller = AuthenticatedCaller::restricted(
        CallerIdentity::LocalUid(2000),
        vec![Permission {
            resource: ResourceScope::Launch(target.clone()),
            operations: BTreeSet::from([SshOperation::Inspect.into()]),
        }],
    );
    for request in [
        request(&target, ExecRequest::Start { command: command() }),
        ControlRequest::Lifecycle {
            launch: target.clone(),
            operation: LifecycleOperation::Stop,
        },
        ControlRequest::Capabilities { launch: target },
    ] {
        assert_eq!(
            dispatcher.dispatch(&caller, request).await.map(|_| ()),
            Err(ControlError::PermissionDenied)
        );
    }
    assert!(state.lock().unwrap().calls.is_empty());
}

#[test]
fn generic_actions_and_scopes_cannot_be_joined_across_permission_records() {
    let target = launch(1);
    let other = launch(2);
    let caller = AuthenticatedCaller::restricted(
        CallerIdentity::LocalUid(2000),
        vec![
            Permission {
                resource: ResourceScope::Launch(target.clone()),
                operations: BTreeSet::from([ControlOperation::Capabilities]),
            },
            Permission {
                resource: ResourceScope::Launch(other),
                operations: BTreeSet::from([ControlOperation::GuestExec(ExecOperation::Start)]),
            },
        ],
    );
    assert_eq!(
        caller.authorize_control(&request(&target, ExecRequest::Start { command: command() })),
        Err(ControlError::PermissionDenied)
    );
    assert!(
        caller
            .authorize_control(&ControlRequest::Capabilities { launch: target })
            .is_ok()
    );
}

#[tokio::test]
async fn unprivileged_and_retired_callers_cannot_discover_targets() {
    let (state, mut dispatcher, target, _) = fixture();
    let unprivileged = AuthenticatedCaller::restricted(CallerIdentity::LocalUid(2000), Vec::new());
    for target in [target.clone(), launch(2)] {
        assert_eq!(
            dispatcher
                .dispatch(
                    &unprivileged,
                    request(&target, ExecRequest::Start { command: command() })
                )
                .await
                .map(|_| ()),
            Err(ControlError::PermissionDenied)
        );
    }
    assert!(state.lock().unwrap().calls.is_empty());
    let retired = AuthenticatedCaller::restricted(
        CallerIdentity::WorkloadLaunch(launch(2)),
        vec![Permission {
            resource: ResourceScope::Launch(target.clone()),
            operations: BTreeSet::from([ControlOperation::GuestExec(ExecOperation::Start)]),
        }],
    );
    assert_eq!(
        dispatcher
            .dispatch(
                &retired,
                request(&target, ExecRequest::Start { command: command() })
            )
            .await
            .map(|_| ()),
        Err(ControlError::PermissionDenied)
    );
    assert!(state.lock().unwrap().calls.is_empty());
}

#[tokio::test]
async fn workload_scope_cannot_relabel_a_native_instance() {
    let (state, mut dispatcher, target, _) = fixture();
    let mut relabeled = target;
    relabeled.instance.workload.name = "permitted-workload".into();
    // Even a native adapter accepting this ID does not authorize the claimed
    // workload-to-instance association; only trusted registration establishes it.
    state.lock().unwrap().current.insert(relabeled.clone());
    let caller = AuthenticatedCaller::restricted(
        CallerIdentity::LocalUid(2000),
        vec![Permission {
            resource: ResourceScope::Workload(relabeled.instance.workload.clone()),
            operations: BTreeSet::from([ControlOperation::GuestExec(ExecOperation::Start)]),
        }],
    );
    assert_eq!(
        dispatcher
            .dispatch(
                &caller,
                request(&relabeled, ExecRequest::Start { command: command() })
            )
            .await
            .map(|_| ()),
        Err(ControlError::UnknownLaunch)
    );
    assert!(state.lock().unwrap().calls.is_empty());
}

#[tokio::test]
async fn caller_and_exact_launch_own_every_exec_followup() {
    let (state, mut dispatcher, target, caller) = fixture();
    let id = start(&mut dispatcher, &caller, &target).await;
    state.lock().unwrap().calls.clear();
    for (caller, target) in [
        (reader(2000, &target), target.clone()),
        (AuthenticatedCaller::local_operator(1000), launch(2)),
    ] {
        assert_eq!(
            dispatcher
                .dispatch(
                    &caller,
                    request(
                        &target,
                        ExecRequest::Read {
                            id: id.clone(),
                            max_bytes: 100
                        }
                    )
                )
                .await
                .map(|_| ()),
            Err(ControlError::UnknownOperation)
        );
    }
    assert!(state.lock().unwrap().calls.is_empty());
    let revoked = AuthenticatedCaller::restricted(CallerIdentity::LocalUid(1000), Vec::new());
    assert_eq!(
        dispatcher
            .dispatch(
                &revoked,
                request(&target, ExecRequest::Read { id, max_bytes: 100 })
            )
            .await
            .map(|_| ()),
        Err(ControlError::PermissionDenied)
    );
    assert!(state.lock().unwrap().calls.is_empty());
}

#[tokio::test]
async fn replaced_runtime_cannot_receive_stale_controls() {
    let (state, mut dispatcher, target, caller) = fixture();
    let id = start(&mut dispatcher, &caller, &target).await;
    {
        let mut state = state.lock().unwrap();
        state.current = BTreeSet::from([launch(2)]);
        state.calls.clear();
    }
    assert_eq!(
        dispatcher
            .dispatch(&caller, request(&target, ExecRequest::Cancel { id }))
            .await
            .map(|_| ()),
        Err(ControlError::StaleLaunch)
    );
    assert_eq!(state.lock().unwrap().calls, ["verify"]);
}

#[tokio::test]
async fn simultaneous_launches_keep_independent_retained_operation_handles() {
    let (state, mut dispatcher, first, caller) = fixture();
    let mut second = launch(2);
    second.instance.instance = "fixture-other".into();
    second.instance.workload.name = "other".into();
    state.lock().unwrap().current.insert(second.clone());
    register(&mut dispatcher, &second);
    let first_id = start(&mut dispatcher, &caller, &first).await;
    let second_id = start(&mut dispatcher, &caller, &second).await;
    assert_ne!(first_id, second_id);
    exec(
        &mut dispatcher,
        &caller,
        &second,
        ExecRequest::Read {
            id: second_id,
            max_bytes: 100,
        },
    )
    .await;
    exec(
        &mut dispatcher,
        &caller,
        &first,
        ExecRequest::Read {
            id: first_id,
            max_bytes: 100,
        },
    )
    .await;
    assert_eq!(state.lock().unwrap().polled, [second, first]);
}

#[tokio::test]
async fn cancellation_acknowledgment_is_not_terminal_completion() {
    let (state, mut dispatcher, target, caller) = fixture();
    let id = start(&mut dispatcher, &caller, &target).await;
    assert_eq!(
        exec(
            &mut dispatcher,
            &caller,
            &target,
            ExecRequest::Cancel { id: id.clone() }
        )
        .await
        .state,
        ExecState::CancellationRequested
    );
    assert_eq!(
        dispatcher
            .dispatch(
                &caller,
                request(&target, ExecRequest::Release { id: id.clone() })
            )
            .await
            .map(|_| ()),
        Err(ControlError::OperationIndeterminate)
    );
    state
        .lock()
        .unwrap()
        .events
        .push_back(vec![ExecEvent::Exited { code: 137 }]);
    assert_eq!(
        exec(
            &mut dispatcher,
            &caller,
            &target,
            ExecRequest::Read {
                id: id.clone(),
                max_bytes: 100
            }
        )
        .await
        .state,
        ExecState::Exited { code: 137 }
    );
    // Releasing a completed local record is safe after the original VM exits.
    state.lock().unwrap().current.clear();
    dispatcher.custody_mut().retire_launch(&target).unwrap();
    exec(
        &mut dispatcher,
        &caller,
        &target,
        ExecRequest::Release { id: id.clone() },
    )
    .await;
    assert_eq!(
        dispatcher
            .dispatch(
                &caller,
                request(&target, ExecRequest::Read { id, max_bytes: 100 })
            )
            .await
            .map(|_| ()),
        Err(ControlError::UnknownOperation)
    );
}

#[tokio::test]
async fn binary_streams_keep_stdout_and_stderr_separate() {
    let (state, mut dispatcher, target, caller) = fixture();
    let id = start(&mut dispatcher, &caller, &target).await;
    state.lock().unwrap().events.push_back(vec![
        ExecEvent::Started,
        ExecEvent::Stdout {
            bytes: vec![0, 255, 10],
        },
        ExecEvent::Stderr {
            bytes: vec![254, 0],
        },
        ExecEvent::Exited { code: 3 },
    ]);
    let result = exec(
        &mut dispatcher,
        &caller,
        &target,
        ExecRequest::Read { id, max_bytes: 5 },
    )
    .await;
    assert_eq!(result.state, ExecState::Exited { code: 3 });
    assert!(matches!(&result.events[1], ExecEvent::Stdout { bytes } if bytes == &[0,255,10]));
    assert!(matches!(&result.events[2], ExecEvent::Stderr { bytes } if bytes == &[254,0]));
}

#[tokio::test]
async fn native_output_overflow_or_events_after_exit_are_not_success() {
    for events in [
        vec![ExecEvent::Stdout { bytes: vec![0; 9] }],
        vec![
            ExecEvent::Exited { code: 0 },
            ExecEvent::Stdout { bytes: vec![1] },
        ],
        vec![ExecEvent::Started; 65],
    ] {
        let (state, mut dispatcher, target, caller) = fixture();
        let id = start(&mut dispatcher, &caller, &target).await;
        state.lock().unwrap().events.push_back(events);
        assert_eq!(
            dispatcher
                .dispatch(
                    &caller,
                    request(
                        &target,
                        ExecRequest::Read {
                            id: id.clone(),
                            max_bytes: 8
                        }
                    )
                )
                .await
                .map(|_| ()),
            Err(ControlError::InvalidRuntimeResponse)
        );
        assert_eq!(
            exec(
                &mut dispatcher,
                &caller,
                &target,
                ExecRequest::Read { id, max_bytes: 8 }
            )
            .await
            .state,
            ExecState::Indeterminate
        );
    }
}

#[tokio::test]
async fn eof_is_once_only_and_binary_stdin_is_preserved() {
    let (state, mut dispatcher, target, caller) = fixture();
    let mut command = command();
    command.stdin = true;
    let id = exec(
        &mut dispatcher,
        &caller,
        &target,
        ExecRequest::Start { command },
    )
    .await
    .id;
    exec(
        &mut dispatcher,
        &caller,
        &target,
        ExecRequest::WriteStdin {
            id: id.clone(),
            bytes: vec![0, 255],
            eof: true,
        },
    )
    .await;
    assert_eq!(state.lock().unwrap().input, [0, 255]);
    assert_eq!(
        dispatcher
            .dispatch(
                &caller,
                request(
                    &target,
                    ExecRequest::WriteStdin {
                        id,
                        bytes: vec![1],
                        eof: false
                    }
                )
            )
            .await
            .map(|_| ()),
        Err(ControlError::InvalidRequest)
    );
    assert_eq!(state.lock().unwrap().input, [0, 255]);
}

#[tokio::test]
async fn pty_and_pipe_requests_keep_distinct_contracts() {
    let (state, mut dispatcher, target, caller) = fixture();
    let pipe = start(&mut dispatcher, &caller, &target).await;
    assert_eq!(
        dispatcher
            .dispatch(
                &caller,
                request(
                    &target,
                    ExecRequest::Resize {
                        id: pipe,
                        rows: 24,
                        columns: 80
                    }
                )
            )
            .await
            .map(|_| ()),
        Err(ControlError::InvalidRequest)
    );
    let mut command = command();
    command.tty = true;
    let id = exec(
        &mut dispatcher,
        &caller,
        &target,
        ExecRequest::Start { command },
    )
    .await
    .id;
    exec(
        &mut dispatcher,
        &caller,
        &target,
        ExecRequest::Resize {
            id: id.clone(),
            rows: 24,
            columns: 80,
        },
    )
    .await;
    state
        .lock()
        .unwrap()
        .events
        .push_back(vec![ExecEvent::Stderr { bytes: vec![1] }]);
    assert_eq!(
        dispatcher
            .dispatch(
                &caller,
                request(&target, ExecRequest::Read { id, max_bytes: 100 })
            )
            .await
            .map(|_| ()),
        Err(ControlError::InvalidRuntimeResponse)
    );
}

#[tokio::test]
async fn malformed_or_oversized_commands_refuse_before_native_access() {
    for change in 0..7 {
        let (state, mut dispatcher, target, caller) = fixture();
        let mut command = command();
        match change {
            0 => command.program.clear(),
            1 => command.program.push('\0'),
            2 => command.timeout_ms = 0,
            3 => command.cwd = Some("relative".into()),
            4 => {
                command.env.insert("BAD=KEY".into(), "value".into());
            }
            5 => command.args = vec!["x".repeat(32 * 1024)],
            _ => command.timeout_ms = 3_600_001,
        }
        assert_eq!(
            dispatcher
                .dispatch(&caller, request(&target, ExecRequest::Start { command }))
                .await
                .map(|_| ()),
            Err(ControlError::InvalidRequest)
        );
        assert!(state.lock().unwrap().calls.is_empty());
    }
}

#[tokio::test]
async fn operation_capacity_and_released_ids_never_allow_reuse() {
    let (state, mut dispatcher, target, caller) = fixture();
    let first = start(&mut dispatcher, &caller, &target).await;
    for _ in 1..32 {
        start(&mut dispatcher, &caller, &target).await;
    }
    assert_eq!(
        dispatcher
            .dispatch(
                &caller,
                request(&target, ExecRequest::Start { command: command() })
            )
            .await
            .map(|_| ()),
        Err(ControlError::ResourceLimit)
    );
    assert_eq!(
        state
            .lock()
            .unwrap()
            .calls
            .iter()
            .filter(|&&call| call == "start")
            .count(),
        32
    );
    state
        .lock()
        .unwrap()
        .events
        .push_back(vec![ExecEvent::Exited { code: 0 }]);
    exec(
        &mut dispatcher,
        &caller,
        &target,
        ExecRequest::Read {
            id: first.clone(),
            max_bytes: 100,
        },
    )
    .await;
    exec(
        &mut dispatcher,
        &caller,
        &target,
        ExecRequest::Release { id: first.clone() },
    )
    .await;
    let next = start(&mut dispatcher, &caller, &target).await;
    assert!(next.sequence > first.sequence);
    assert_eq!(
        dispatcher
            .dispatch(&caller, request(&target, ExecRequest::Cancel { id: first }))
            .await
            .map(|_| ()),
        Err(ControlError::UnknownOperation)
    );
}

#[tokio::test(start_paused = true)]
async fn start_timeout_remains_indeterminate_and_is_not_replayed() {
    let (state, mut dispatcher, target, caller) = fixture();
    state.lock().unwrap().hang_start = true;
    let result = exec(
        &mut dispatcher,
        &caller,
        &target,
        ExecRequest::Start { command: command() },
    )
    .await;
    assert_eq!(result.state, ExecState::Indeterminate);
    assert_eq!(
        exec(
            &mut dispatcher,
            &caller,
            &target,
            ExecRequest::Read {
                id: result.id.clone(),
                max_bytes: 100
            }
        )
        .await
        .state,
        ExecState::Indeterminate
    );
    assert_eq!(
        dispatcher
            .dispatch(
                &caller,
                request(&target, ExecRequest::Release { id: result.id })
            )
            .await
            .map(|_| ()),
        Err(ControlError::OperationIndeterminate)
    );
    assert_eq!(
        state
            .lock()
            .unwrap()
            .calls
            .iter()
            .filter(|&&call| call == "start")
            .count(),
        1
    );
}

#[tokio::test(start_paused = true)]
async fn canceled_control_future_cannot_keep_a_false_running_observation() {
    let (state, mut dispatcher, target, caller) = fixture();
    let id = start(&mut dispatcher, &caller, &target).await;
    state.lock().unwrap().hang_cancel = true;
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(1),
            dispatcher.dispatch(
                &caller,
                request(&target, ExecRequest::Cancel { id: id.clone() })
            )
        )
        .await
        .is_err()
    );
    assert_eq!(
        exec(
            &mut dispatcher,
            &caller,
            &target,
            ExecRequest::Read { id, max_bytes: 100 }
        )
        .await
        .state,
        ExecState::Indeterminate
    );
}

#[tokio::test]
async fn lifecycle_uses_native_bound_entry_and_stop_is_not_stopped() {
    let (state, mut dispatcher, target, caller) = fixture();
    let result = dispatcher
        .dispatch(
            &caller,
            ControlRequest::Lifecycle {
                launch: target,
                operation: LifecycleOperation::Stop,
            },
        )
        .await
        .unwrap();
    assert!(matches!(
        result.response,
        ControlResponse::Lifecycle {
            state: LifecycleState::Stopping,
            ..
        }
    ));
    assert_eq!(state.lock().unwrap().calls, ["verify", "lifecycle"]);
    assert!(result.custody_transaction.is_none());
}

#[tokio::test]
async fn direct_custody_dispatch_does_not_execute_guest_commands() {
    let (state, mut dispatcher, target, caller) = fixture();
    let credentials = crate::microsandbox::plan::CredentialsPlan {
        ssh: Vec::new(),
        signing: Vec::new(),
        strict: true,
        strict_origin: None,
    };
    dispatcher
        .custody_mut()
        .register_launch(target.clone(), &credentials, None)
        .unwrap();
    let result = dispatcher
        .dispatch(
            &caller,
            ControlRequest::SshCustody(SshRequest::Inspect { launch: target }),
        )
        .await
        .unwrap();
    assert!(matches!(result.response, ControlResponse::SshCustody(status) if !status.ready));
    assert!(result.custody_transaction.is_none());
    assert!(state.lock().unwrap().calls.is_empty());
}

#[test]
fn common_wire_rejects_identity_injection_and_does_not_debug_command_data() {
    let target = launch(1);
    let mut command = command();
    command.args.push("ARG_SENTINEL".into());
    command.env.insert("FIXTURE".into(), "ENV_SENTINEL".into());
    assert!(!format!("{command:?}").contains("SENTINEL"));
    let mut wire = serde_json::to_value(request(&target, ExecRequest::Start { command })).unwrap();
    wire["caller"] = serde_json::json!({"uid": 1000});
    assert!(serde_json::from_value::<ControlRequest>(wire).is_err());
}

#[cfg(unix)]
#[tokio::test]
async fn local_codec_refuses_old_version_and_correlates_typed_response() {
    use super::local::{LocalAccess, PROTOCOL_VERSION, RequestEnvelope, request, serve_connection};
    use tokio::net::UnixStream;
    for version in [1, PROTOCOL_VERSION] {
        let (client, server) = UnixStream::pair().unwrap();
        let access = LocalAccess::new(server.peer_cred().unwrap().uid());
        let called = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = called.clone();
        let task = tokio::spawn(async move {
            serve_connection(server, &access, move |_, request| async move {
                flag.store(true, std::sync::atomic::Ordering::SeqCst);
                assert!(matches!(request, ControlRequest::Capabilities { .. }));
                Ok(ControlResponse::Capabilities {
                    runtime: RuntimeCapabilities::default(),
                    ssh_custody: true,
                })
            })
            .await
            .unwrap();
        });
        let response = request(
            client,
            &RequestEnvelope {
                version,
                id: 7,
                request: ControlRequest::Capabilities { launch: launch(1) },
            },
        )
        .await
        .unwrap();
        task.await.unwrap();
        assert_eq!(response.id, 7);
        assert_eq!(
            called.load(std::sync::atomic::Ordering::SeqCst),
            version == PROTOCOL_VERSION
        );
        if version == 1 {
            assert!(matches!(
                response.result,
                Err(ControlError::UnsupportedVersion)
            ));
        } else {
            assert!(matches!(
                response.result,
                Ok(ControlResponse::Capabilities { .. })
            ));
        }
    }
}
