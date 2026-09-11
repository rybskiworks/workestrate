//! Host-owner controls using real private endpoints and desired stores, but
//! synthetic retained native sessions. No VM or guest process is started.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::pin::Pin;
use std::sync::Mutex;
use std::task::Poll;

use tokio::io::AsyncWriteExt;
use tokio::net::UnixStream;
use tokio::sync::Notify;

use crate::control_plane::custody::SshController;
use crate::control_plane::local::{PROTOCOL_VERSION, RequestEnvelope, request};
use crate::control_plane::native::{NativeExec, NativeFuture};
use crate::control_plane::types::*;
use crate::microsandbox::plan::CredentialsPlan;

#[path = "owner_broker_tests.rs"]
mod broker_tests;

#[derive(Default)]
struct NativeState {
    current: BTreeSet<LaunchRef>,
    blocked: BTreeSet<&'static str>,
    entered: Vec<(&'static str, u64)>,
    finished: Vec<(&'static str, u64)>,
    abandoned: Vec<(&'static str, u64)>,
    sessions: BTreeMap<u64, SessionState>,
    dropped_sessions: Vec<u64>,
    next_session: u64,
    vm_stop_calls: usize,
}

struct SessionState {
    events: VecDeque<ExecEvent>,
    cancel_pending: bool,
}

#[derive(Clone, Default)]
struct NativeFixture {
    state: Arc<Mutex<NativeState>>,
    wake: Arc<Notify>,
}

struct Call {
    fixture: NativeFixture,
    key: (&'static str, u64),
    finished: bool,
}

impl Drop for Call {
    fn drop(&mut self) {
        if !self.finished {
            self.fixture.state.lock().unwrap().abandoned.push(self.key);
        }
    }
}

impl NativeFixture {
    async fn call(&self, kind: &'static str, id: u64) {
        self.state.lock().unwrap().entered.push((kind, id));
        let mut guard = Call {
            fixture: self.clone(),
            key: (kind, id),
            finished: false,
        };
        loop {
            let wake = self.wake.notified();
            let blocked = {
                let state = self.state.lock().unwrap();
                state.blocked.contains(kind)
                    || (kind == "cancel" && state.sessions[&id].cancel_pending)
            };
            if !blocked {
                break;
            }
            wake.await;
        }
        self.state.lock().unwrap().finished.push((kind, id));
        guard.finished = true;
    }

    fn has_entered(&self, kind: &str) -> bool {
        self.state
            .lock()
            .unwrap()
            .entered
            .iter()
            .any(|(name, _)| *name == kind)
    }
}

struct Guest(NativeFixture);
struct Session(NativeFixture, u64);

impl NativeControl for Guest {
    fn capabilities(&self) -> RuntimeCapabilities {
        RuntimeCapabilities {
            launch_bound_exec: true,
            stdin: true,
            pty: true,
            cancellation: true,
            launch_bound_lifecycle: true,
        }
    }

    fn verify_launch<'a>(&'a self, launch: &'a LaunchRef) -> NativeFuture<'a, ()> {
        Box::pin(async move {
            self.0.call("verify", 0).await;
            if self.0.state.lock().unwrap().current.contains(launch) {
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
            self.0.call("start", 0).await;
            let mut state = self.0.state.lock().unwrap();
            if !state.current.contains(launch) {
                return Err(ControlError::StaleLaunch);
            }
            state.next_session += 1;
            let id = state.next_session;
            state.sessions.insert(
                id,
                SessionState {
                    events: VecDeque::from([ExecEvent::Started]),
                    cancel_pending: false,
                },
            );
            Ok(Box::new(Session(self.0.clone(), id)) as Box<dyn NativeExec>)
        })
    }

    fn lifecycle<'a>(
        &'a mut self,
        _launch: &'a LaunchRef,
        operation: LifecycleOperation,
    ) -> NativeFuture<'a, LifecycleState> {
        Box::pin(async move {
            if operation == LifecycleOperation::Stop {
                self.0.state.lock().unwrap().vm_stop_calls += 1;
            }
            self.0.call("lifecycle", 0).await;
            Ok(LifecycleState::Running)
        })
    }
}

impl NativeExec for Session {
    fn poll(&mut self, max_bytes: usize) -> Result<Vec<ExecEvent>, ControlError> {
        assert!((1..=8192).contains(&max_bytes));
        let mut state = self.0.state.lock().unwrap();
        state.entered.push(("poll", self.1));
        let events = &mut state.sessions.get_mut(&self.1).unwrap().events;
        Ok(events.drain(..).collect()) // Only Started/Exited, never output bytes.
    }

    fn stdin<'a>(&'a mut self, _bytes: &'a [u8], _eof: bool) -> NativeFuture<'a, ()> {
        Box::pin(async move {
            self.0.call("stdin", self.1).await;
            Ok(())
        })
    }

    fn resize(&mut self, _rows: u16, _columns: u16) -> NativeFuture<'_, ()> {
        Box::pin(async move {
            self.0.call("resize", self.1).await;
            Ok(())
        })
    }

    fn cancel(&mut self) -> NativeFuture<'_, ()> {
        Box::pin(async move {
            self.0.call("cancel", self.1).await;
            self.0
                .state
                .lock()
                .unwrap()
                .sessions
                .get_mut(&self.1)
                .unwrap()
                .events
                .push_back(ExecEvent::Exited { code: 0 });
            Ok(())
        })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.0.state.lock().unwrap().dropped_sessions.push(self.1);
    }
}

struct Fixture {
    root: tempfile::TempDir,
    native: NativeFixture,
    owner: HostControlOwner<Guest>,
    launch: LaunchRef,
    uid: u32,
}

fn fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let uid = root.path().metadata().unwrap().uid();
    let launch = LaunchRef {
        instance: InstanceRef {
            workload: WorkloadRef {
                context: Some("fixture".into()),
                name: "worker".into(),
            },
            instance: "fixture-worker".into(),
        },
        generation: OpaqueId::from_bytes([7; 32]),
    };
    let native = NativeFixture::default();
    native.state.lock().unwrap().current.insert(launch.clone());
    let mut custody = SshController::initialize(&root.path().join("desired"), uid).unwrap();
    let credentials = CredentialsPlan {
        ssh: Vec::new(),
        signing: Vec::new(),
        strict: true,
        strict_origin: None,
    };
    assert!(
        custody
            .register_launch(launch.clone(), &credentials, None)
            .unwrap()
            .transaction
            .is_none()
    );
    assert!(!custody.confirm_launch(&launch).unwrap().ready);
    let dispatcher = ControlDispatcher::with_custody(Guest(native.clone()), custody);
    let owner = HostControlOwner::bind(&root.path().join("endpoint"), uid, dispatcher).unwrap();
    Fixture {
        root,
        native,
        owner,
        launch,
        uid,
    }
}

fn start_request(launch: &LaunchRef) -> ControlRequest {
    ControlRequest::GuestExec {
        launch: launch.clone(),
        request: ExecRequest::Start {
            command: ExecCommand {
                program: "/synthetic/never-executed".into(),
                args: Vec::new(),
                cwd: Some("/".into()),
                env: BTreeMap::new(),
                stdin: true,
                tty: false,
                timeout_ms: 1000,
            },
        },
    }
}

struct QueuedReply {
    receive: oneshot::Receiver<Result<ControlResponse, ControlError>>,
    _client: std::os::unix::net::UnixStream,
}

impl QueuedReply {
    fn try_recv(
        &mut self,
    ) -> Result<Result<ControlResponse, ControlError>, oneshot::error::TryRecvError> {
        self.receive.try_recv()
    }
}

fn queue(owner: &HostControlOwner<Guest>, uid: u32, request: ControlRequest) -> QueuedReply {
    let (reply, receive) = oneshot::channel();
    let (client, peer) = std::os::unix::net::UnixStream::pair().unwrap();
    assert!(
        owner
            .sender
            .try_send(QueuedRequest {
                caller: AuthenticatedCaller::local_operator(uid),
                request,
                reply,
                peer: peer.into(),
            })
            .is_ok()
    );
    QueuedReply {
        receive,
        _client: client,
    }
}

async fn start(fixture: &mut Fixture) -> OperationRef {
    let outcome = fixture
        .owner
        .dispatcher
        .dispatch(
            &AuthenticatedCaller::local_operator(fixture.uid),
            start_request(&fixture.launch),
        )
        .await
        .unwrap();
    let ControlResponse::GuestExec(status) = outcome.response else {
        panic!("wrong response")
    };
    assert_eq!(status.state, ExecState::Accepted);
    status.id
}

async fn poll_once<F: Future>(mut future: Pin<&mut F>) -> Poll<F::Output> {
    std::future::poll_fn(|context| Poll::Ready(future.as_mut().poll(context))).await
}

async fn drive_until<F: Future>(mut future: Pin<&mut F>, ready: impl Fn() -> bool) {
    for _ in 0..128 {
        assert!(
            poll_once(future.as_mut()).await.is_pending(),
            "owner completed before fixture phase"
        );
        if ready() {
            return;
        }
        tokio::task::yield_now().await;
    }
    panic!("fixture phase did not become ready");
}

fn replace_desired(root: &Path) {
    let original = root.join("desired/custody.json");
    let saved = root.join("desired/custody.saved");
    std::fs::rename(&original, &saved).unwrap();
    std::fs::copy(saved, original).unwrap(); // Equal content, different inode.
}

fn replace_endpoint_owner(root: &Path) {
    let owner = root.join("endpoint/owner.lock");
    std::fs::rename(&owner, root.join("endpoint/owner.saved")).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(owner)
        .unwrap();
}

#[tokio::test(start_paused = true)]
async fn owner_endpoint_loss_cancels_inflight_start_and_keeps_uncertain_reservation() {
    let mut fixture = fixture();
    fixture.native.state.lock().unwrap().blocked.insert("start");
    let _reply = queue(&fixture.owner, fixture.uid, start_request(&fixture.launch));
    let mut serving = Box::pin(fixture.owner.serve_until(std::future::pending()));
    drive_until(serving.as_mut(), || fixture.native.has_entered("start")).await;
    replace_endpoint_owner(fixture.root.path());
    tokio::time::advance(Duration::from_secs(1)).await;
    let exit = serving.await;
    assert!(exit.primary.is_err());
    assert_eq!(exit.drain.operations.pending.len(), 1);
    assert!(!exit.drain.complete());
    let state = fixture.native.state.lock().unwrap();
    assert!(state.abandoned.contains(&("start", 0)));
    assert_eq!(state.next_session, 0);
    assert_eq!(state.vm_stop_calls, 0);
}

#[tokio::test(start_paused = true)]
async fn owner_store_loss_during_verify_refuses_before_native_start() {
    let mut fixture = fixture();
    fixture
        .native
        .state
        .lock()
        .unwrap()
        .blocked
        .insert("verify");
    let _reply = queue(&fixture.owner, fixture.uid, start_request(&fixture.launch));
    let mut serving = Box::pin(fixture.owner.serve_until(std::future::pending()));
    drive_until(serving.as_mut(), || fixture.native.has_entered("verify")).await;
    replace_desired(fixture.root.path());
    fixture
        .native
        .state
        .lock()
        .unwrap()
        .blocked
        .remove("verify");
    fixture.native.wake.notify_waiters();
    let exit = serving.await;
    assert!(exit.primary.is_err());
    assert!(!fixture.native.has_entered("start"));
    assert!(exit.drain.operations.pending.is_empty());
    assert_eq!(fixture.native.state.lock().unwrap().vm_stop_calls, 0);
}

#[tokio::test(start_paused = true)]
async fn owner_store_loss_during_issued_native_calls_is_observed_by_call_budget() {
    for kind in ["start", "stdin", "lifecycle"] {
        let mut fixture = fixture();
        let operation = if kind == "stdin" {
            Some(start(&mut fixture).await)
        } else {
            None
        };
        let request = match operation {
            Some(id) => ControlRequest::GuestExec {
                launch: fixture.launch.clone(),
                request: ExecRequest::WriteStdin {
                    id,
                    bytes: vec![b'x'],
                    eof: false,
                },
            },
            None if kind == "start" => start_request(&fixture.launch),
            None => ControlRequest::Lifecycle {
                launch: fixture.launch.clone(),
                operation: LifecycleOperation::Inspect,
            },
        };
        fixture.native.state.lock().unwrap().blocked.insert(kind);
        let _reply = queue(&fixture.owner, fixture.uid, request);
        let began = Instant::now();
        let mut serving = Box::pin(fixture.owner.serve_until(std::future::pending()));
        drive_until(serving.as_mut(), || fixture.native.has_entered(kind)).await;
        replace_desired(fixture.root.path());
        // Storage checks are before/after bounded calls, not a separate watcher.
        tokio::time::advance(Duration::from_secs(1)).await;
        assert!(poll_once(serving.as_mut()).await.is_pending());
        let exit = serving.await;
        assert!(exit.primary.is_err(), "{kind}");
        assert!(Instant::now() - began <= Duration::from_secs(6), "{kind}");
        let state = fixture.native.state.lock().unwrap();
        assert!(
            state.abandoned.iter().any(|(name, _)| *name == kind),
            "{kind}"
        );
        assert_eq!(state.vm_stop_calls, 0);
        if kind == "start" {
            assert_eq!(exit.drain.operations.pending.len(), 1);
        }
    }
}

#[tokio::test]
async fn owner_disconnected_queued_request_has_no_native_effects() {
    let mut fixture = fixture();
    drop(queue(
        &fixture.owner,
        fixture.uid,
        start_request(&fixture.launch),
    ));
    let mut response = queue(
        &fixture.owner,
        fixture.uid,
        ControlRequest::Capabilities {
            launch: fixture.launch.clone(),
        },
    );
    let mut serving = Box::pin(fixture.owner.serve_until(std::future::pending()));
    let mut answered = false;
    for _ in 0..128 {
        assert!(poll_once(serving.as_mut()).await.is_pending());
        if let Ok(result) = response.try_recv() {
            assert!(matches!(result, Ok(ControlResponse::Capabilities { .. })));
            answered = true;
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(answered, "live queued control request did not complete");
    assert!(!fixture.native.has_entered("verify"));
    assert!(!fixture.native.has_entered("start"));
    drop(serving);
    assert!(
        fixture
            .owner
            .drain_until(Instant::now() + Duration::from_secs(1))
            .await
            .complete()
    );
}

#[tokio::test]
async fn owner_queue_has_exactly_thirty_two_bounded_slots_and_fence_rejects_waiters() {
    let mut fixture = fixture();
    assert_eq!(QUEUE_CAPACITY, 32);
    let mut replies = Vec::new();
    for _ in 0..32 {
        replies.push(queue(
            &fixture.owner,
            fixture.uid,
            start_request(&fixture.launch),
        ));
    }
    let (reply, _) = oneshot::channel();
    let (_client, peer) = std::os::unix::net::UnixStream::pair().unwrap();
    assert!(matches!(
        fixture.owner.sender.try_send(QueuedRequest {
            caller: AuthenticatedCaller::local_operator(fixture.uid),
            request: start_request(&fixture.launch),
            reply,
            peer: peer.into(),
        }),
        Err(mpsc::error::TrySendError::Full(_))
    ));
    fixture.owner.fence();
    for mut reply in replies {
        assert!(matches!(
            reply.try_recv(),
            Ok(Err(ControlError::StateUnavailable))
        ));
    }
    assert!(!fixture.native.has_entered("start"));
    assert_eq!(fixture.owner.sender.capacity(), 32);
    assert!(fixture.owner.sender.is_closed());
}

#[tokio::test(start_paused = true)]
async fn owner_sixteen_partial_clients_bound_admission_and_reap_releases_one_slot() {
    let mut fixture = fixture();
    assert_eq!(MAX_CLIENTS, 16);
    let path = fixture.owner.path().unwrap().to_owned();
    let mut clients = Vec::new();
    for _ in 0..16 {
        let mut client = UnixStream::connect(&path).await.unwrap();
        client.write_all(b"{").await.unwrap(); // Decoder remains owned, no complete request.
        clients.push(client);
    }
    let last = UnixStream::connect(&path).await.unwrap();
    let envelope = RequestEnvelope {
        version: PROTOCOL_VERSION,
        id: 1,
        request: ControlRequest::Capabilities {
            launch: fixture.launch.clone(),
        },
    };
    let mut reply = Box::pin(request(last, &envelope));
    let mut serving = Box::pin(fixture.owner.serve_until(std::future::pending()));
    for _ in 0..128 {
        assert!(poll_once(serving.as_mut()).await.is_pending());
        assert!(
            poll_once(reply.as_mut()).await.is_pending(),
            "seventeenth client admitted"
        );
        tokio::task::yield_now().await;
    }
    drop(clients.remove(0)); // Exactly one accepted client closes, not all sixteen.
    let mut answered = false;
    for _ in 0..128 {
        assert!(poll_once(serving.as_mut()).await.is_pending());
        if let Poll::Ready(response) = poll_once(reply.as_mut()).await {
            assert!(matches!(
                response.unwrap().result,
                Ok(ControlResponse::Capabilities { .. })
            ));
            answered = true;
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(answered, "reaped client did not release one admission slot");
    drop(reply);
    drop(serving);
    drop(clients);
    let drained = fixture
        .owner
        .drain_until(Instant::now() + Duration::from_secs(1))
        .await;
    assert!(drained.complete());
    assert_eq!(drained.pending_clients, 0);
}

#[tokio::test(start_paused = true)]
async fn owner_cancelled_serve_and_drain_retain_the_same_native_lease_for_retry() {
    let mut fixture = fixture();
    let id = start(&mut fixture).await;
    fixture
        .native
        .state
        .lock()
        .unwrap()
        .sessions
        .get_mut(&1)
        .unwrap()
        .cancel_pending = true;
    let mut serving = Box::pin(fixture.owner.serve_until(std::future::pending()));
    assert!(poll_once(serving.as_mut()).await.is_pending());
    drop(serving);
    assert!(fixture.owner.path().is_err());
    let mut draining = Box::pin(
        fixture
            .owner
            .drain_until(Instant::now() + Duration::from_secs(1)),
    );
    drive_until(draining.as_mut(), || fixture.native.has_entered("cancel")).await;
    drop(draining);
    assert!(
        fixture
            .native
            .state
            .lock()
            .unwrap()
            .dropped_sessions
            .is_empty()
    );
    fixture
        .native
        .state
        .lock()
        .unwrap()
        .sessions
        .get_mut(&1)
        .unwrap()
        .cancel_pending = false;
    let retry = fixture
        .owner
        .drain_until(Instant::now() + Duration::from_secs(1))
        .await;
    assert!(retry.complete());
    assert_eq!(retry.operations.completed, vec![id]);
    let state = fixture.native.state.lock().unwrap();
    assert_eq!(state.next_session, 1);
    assert_eq!(
        state
            .entered
            .iter()
            .filter(|call| **call == ("cancel", 1))
            .count(),
        2
    );
    assert_eq!(state.dropped_sessions, vec![1]);
    assert_eq!(state.vm_stop_calls, 0);
}

#[tokio::test(start_paused = true)]
async fn owner_drain_polls_later_leases_despite_a_blocked_first_cancellation() {
    let mut fixture = fixture();
    let first = start(&mut fixture).await;
    let second = start(&mut fixture).await;
    fixture
        .native
        .state
        .lock()
        .unwrap()
        .sessions
        .get_mut(&1)
        .unwrap()
        .cancel_pending = true;
    let result = fixture
        .owner
        .drain_until(Instant::now() + Duration::from_millis(100))
        .await;
    assert!(!result.complete());
    assert_eq!(result.operations.completed, vec![second]);
    assert_eq!(result.operations.pending.len(), 1);
    assert_eq!(result.operations.pending[0].0, first);
    {
        let state = fixture.native.state.lock().unwrap();
        assert!(state.entered.contains(&("cancel", 1)));
        assert!(state.finished.contains(&("cancel", 2)));
        assert_eq!(state.dropped_sessions, vec![2]);
        assert_eq!(state.vm_stop_calls, 0);
    }
    fixture
        .native
        .state
        .lock()
        .unwrap()
        .sessions
        .get_mut(&1)
        .unwrap()
        .cancel_pending = false;
    assert!(
        fixture
            .owner
            .drain_until(Instant::now() + Duration::from_secs(1))
            .await
            .complete()
    );
}

#[tokio::test]
async fn owner_normal_shutdown_cancels_its_exec_but_never_stops_an_adopted_vm() {
    let mut fixture = fixture();
    let operation = start(&mut fixture).await;
    let exit = fixture.owner.serve_until(std::future::ready(Ok(()))).await;
    assert!(exit.primary.is_ok());
    assert!(exit.drain.complete());
    assert_eq!(exit.drain.operations.completed, vec![operation]);
    let state = fixture.native.state.lock().unwrap();
    assert_eq!(state.vm_stop_calls, 0);
    assert!(state.current.contains(&fixture.launch));
    assert_eq!(state.dropped_sessions, vec![1]);
}

#[tokio::test]
async fn owner_without_broker_keeps_reconciled_desired_state_unobserved_and_unready() {
    let mut fixture = fixture();
    let mut reply = queue(
        &fixture.owner,
        fixture.uid,
        ControlRequest::SshCustody(SshRequest::Reconcile {
            launch: fixture.launch.clone(),
            expected_revision: 1,
            credentials: BTreeSet::new(),
        }),
    );
    let mut serving = Box::pin(fixture.owner.serve_until(std::future::pending()));
    let mut accepted = false;
    for _ in 0..128 {
        assert!(poll_once(serving.as_mut()).await.is_pending());
        if let Ok(response) = reply.try_recv() {
            let Ok(ControlResponse::SshCustody(status)) = response else {
                panic!("wrong custody reply")
            };
            assert_eq!(status.desired.revision, 2);
            assert!(status.observed.is_none());
            assert!(!status.ready);
            accepted = true;
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(accepted);
    drop(serving);
    assert!(
        fixture
            .owner
            .drain_until(Instant::now() + Duration::from_secs(1))
            .await
            .complete()
    );
    let saved: serde_json::Value = serde_json::from_slice(
        &std::fs::read(fixture.root.path().join("desired/custody.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(saved["launches"][0]["desired"]["revision"], 2);
    assert!(fixture.native.state.lock().unwrap().entered.is_empty());
}

#[tokio::test(start_paused = true)]
async fn owner_retry_signal_during_drain_keeps_the_same_absolute_deadline_and_leases() {
    use crate::commands::control::retain_until_retired;
    let mut fixture = fixture();
    let operation = start(&mut fixture).await;
    fixture
        .native
        .state
        .lock()
        .unwrap()
        .sessions
        .get_mut(&1)
        .unwrap()
        .cancel_pending = true;
    let first = fixture.owner.drain_until(Instant::now()).await;
    assert!(first.pending());
    let (signals, mut receive) = mpsc::channel::<()>(2);
    signals.try_send(()).unwrap();
    let delivered = Arc::new(Mutex::new(Vec::new()));
    let observed = Arc::clone(&delivered);
    let began = Instant::now();
    let mut retirement = Box::pin(retain_until_retired(
        &mut fixture.owner,
        first,
        async || {
            receive
                .recv()
                .await
                .ok_or_else(|| io::Error::other("closed synthetic signal source"))?;
            observed.lock().unwrap().push(Instant::now());
            Ok(())
        },
    ));
    drive_until(retirement.as_mut(), || fixture.native.has_entered("cancel")).await;
    assert_eq!(*delivered.lock().unwrap(), vec![began]);
    tokio::time::advance(Duration::from_secs(3)).await;
    signals.try_send(()).unwrap();
    assert!(poll_once(retirement.as_mut()).await.is_pending());
    assert_eq!(delivered.lock().unwrap().len(), 1);
    tokio::time::advance(Duration::from_secs(7)).await;
    drive_until(retirement.as_mut(), || delivered.lock().unwrap().len() == 2).await;
    assert_eq!(
        *delivered.lock().unwrap(),
        vec![began, began + Duration::from_secs(10)]
    );
    assert!(
        fixture
            .native
            .state
            .lock()
            .unwrap()
            .dropped_sessions
            .is_empty()
    );
    fixture
        .native
        .state
        .lock()
        .unwrap()
        .sessions
        .get_mut(&1)
        .unwrap()
        .cancel_pending = false;
    fixture.native.wake.notify_waiters();
    let (drain, earlier_error) = retirement.await;
    assert!(drain.complete());
    assert!(earlier_error);
    assert_eq!(drain.operations.completed, vec![operation]);
    let state = fixture.native.state.lock().unwrap();
    assert_eq!(state.next_session, 1);
    assert_eq!(state.dropped_sessions, vec![1]);
    assert_eq!(state.vm_stop_calls, 0);
}

#[tokio::test]
async fn owner_closed_retry_signal_source_does_not_release_uncertain_leases() {
    use crate::commands::control::retain_until_retired;
    let mut fixture = fixture();
    let operation = start(&mut fixture).await;
    let first = fixture.owner.drain_until(Instant::now()).await;
    let mut retained = Box::pin(retain_until_retired(
        &mut fixture.owner,
        first,
        async || Err(io::Error::other("closed synthetic signal source")),
    ));
    assert!(poll_once(retained.as_mut()).await.is_pending());
    assert!(
        fixture
            .native
            .state
            .lock()
            .unwrap()
            .dropped_sessions
            .is_empty()
    );
    drop(retained); // Borrowed owner is still present for an explicit cleanup.
    fixture
        .native
        .state
        .lock()
        .unwrap()
        .sessions
        .get_mut(&1)
        .unwrap()
        .events
        .push_back(ExecEvent::Exited { code: 0 });
    let completed = fixture.owner.drain_until(Instant::now()).await;
    assert!(completed.complete());
    assert_eq!(completed.operations.completed, vec![operation]);
    assert_eq!(fixture.native.state.lock().unwrap().vm_stop_calls, 0);
}

async fn accepted_client(fixture: &mut Fixture) -> UnixStream {
    let client = UnixStream::connect(fixture.owner.path().unwrap())
        .await
        .unwrap();
    let server = fixture
        .owner
        .endpoint
        .as_ref()
        .unwrap()
        .accept()
        .await
        .unwrap();
    // Use the same production acceptance/decoder/peer-fd/queue path. Keeping
    // admission explicit lets the test hold dispatch behind a known first call.
    fixture.owner.accept_client(server).unwrap();
    client
}

#[tokio::test]
async fn owner_actual_half_closed_request_still_dispatches_and_receives_its_reply() {
    let mut fixture = fixture();
    let client = UnixStream::connect(fixture.owner.path().unwrap())
        .await
        .unwrap();
    let envelope = RequestEnvelope {
        version: PROTOCOL_VERSION,
        id: 1,
        request: start_request(&fixture.launch),
    };
    let mut response = Box::pin(request(client, &envelope));
    let mut serving = Box::pin(fixture.owner.serve_until(std::future::pending()));
    let mut answered = false;
    for _ in 0..256 {
        assert!(poll_once(serving.as_mut()).await.is_pending());
        if let Poll::Ready(result) = poll_once(response.as_mut()).await {
            assert!(
                matches!(result.unwrap().result, Ok(ControlResponse::GuestExec(status))
                if status.state == ExecState::Accepted)
            );
            answered = true;
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(answered);
    drop(response);
    drop(serving);
    assert_eq!(fixture.native.state.lock().unwrap().next_session, 1);
    assert!(
        fixture
            .owner
            .drain_until(Instant::now() + Duration::from_secs(1))
            .await
            .complete()
    );
}

#[tokio::test]
async fn owner_actual_fully_closed_queued_client_is_refused_before_native_dispatch() {
    let mut fixture = fixture();
    let mut abandoned = accepted_client(&mut fixture).await;
    let barrier = accepted_client(&mut fixture).await;
    fixture.native.state.lock().unwrap().blocked.insert("start");
    let _first_reply = queue(&fixture.owner, fixture.uid, start_request(&fixture.launch));
    let sender = fixture.owner.sender.clone();
    let rejected = RequestEnvelope {
        version: PROTOCOL_VERSION,
        id: 2,
        request: start_request(&fixture.launch),
    };
    let final_request = RequestEnvelope {
        version: PROTOCOL_VERSION,
        id: 3,
        request: ControlRequest::Capabilities {
            launch: fixture.launch.clone(),
        },
    };
    let mut serving = Box::pin(fixture.owner.serve_until(std::future::pending()));
    drive_until(serving.as_mut(), || fixture.native.has_entered("start")).await;
    let mut bytes = serde_json::to_vec(&rejected).unwrap();
    bytes.push(b'\n');
    abandoned.write_all(&bytes).await.unwrap();
    abandoned.shutdown().await.unwrap();
    drive_until(serving.as_mut(), || sender.capacity() == QUEUE_CAPACITY - 1).await;
    drop(abandoned); // Full close occurs while its actual decoded request is queued.
    let mut response = Box::pin(request(barrier, &final_request));
    for _ in 0..256 {
        assert!(poll_once(serving.as_mut()).await.is_pending());
        assert!(poll_once(response.as_mut()).await.is_pending());
        if sender.capacity() == QUEUE_CAPACITY - 2 {
            break;
        }
        tokio::task::yield_now().await;
    }
    assert_eq!(sender.capacity(), QUEUE_CAPACITY - 2);
    fixture.native.state.lock().unwrap().blocked.remove("start");
    fixture.native.wake.notify_waiters();
    let mut answered = false;
    for _ in 0..256 {
        assert!(poll_once(serving.as_mut()).await.is_pending());
        if let Poll::Ready(result) = poll_once(response.as_mut()).await {
            assert!(matches!(
                result.unwrap().result,
                Ok(ControlResponse::Capabilities { .. })
            ));
            answered = true;
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(
        answered,
        "FIFO barrier after the abandoned client did not complete"
    );
    drop(response);
    drop(serving);
    {
        let state = fixture.native.state.lock().unwrap();
        assert_eq!(
            state.next_session, 1,
            "abandoned second request started a native command"
        );
        assert_eq!(
            state
                .entered
                .iter()
                .filter(|(name, _)| *name == "start")
                .count(),
            1
        );
        assert_eq!(state.vm_stop_calls, 0);
    }
    assert!(
        fixture
            .owner
            .drain_until(Instant::now() + Duration::from_secs(1))
            .await
            .complete()
    );
}
