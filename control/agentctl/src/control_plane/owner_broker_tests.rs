//! Owner attachment controls with real local streams and a synthetic wire peer.
//! No guest, policy material, background peer task or alternate owner is created.

use super::super::broker::BrokerTransport;
use super::*;
use microsandbox_protocol::broker as wire;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tokio::io::AsyncReadExt;

struct Transport {
    socket: Mutex<Option<UnixStream>>,
    current: AtomicBool,
    running: AtomicBool,
    checks: AtomicUsize,
    verifies: AtomicUsize,
    connects: AtomicUsize,
    block_verify_at: AtomicUsize,
}

impl BrokerTransport for Transport {
    fn check_current(&self) -> Result<(), ControlError> {
        self.checks.fetch_add(1, Ordering::SeqCst);
        if self.current.load(Ordering::SeqCst) {
            Ok(())
        } else {
            Err(ControlError::StaleLaunch)
        }
    }

    fn verify_running(&self, _deadline: Instant) -> NativeFuture<'_, ()> {
        Box::pin(async move {
            let call = self.verifies.fetch_add(1, Ordering::SeqCst) + 1;
            if self.block_verify_at.load(Ordering::SeqCst) == call {
                std::future::pending::<()>().await;
            }
            if self.running.load(Ordering::SeqCst) {
                Ok(())
            } else {
                Err(ControlError::RuntimeUnavailable)
            }
        })
    }

    fn connect(&self, _deadline: Instant) -> NativeFuture<'_, UnixStream> {
        Box::pin(async move {
            self.connects.fetch_add(1, Ordering::SeqCst);
            self.socket
                .lock()
                .unwrap()
                .take()
                .ok_or(ControlError::RuntimeUnavailable)
        })
    }
}

fn transport() -> (Arc<Transport>, UnixStream) {
    let (socket, peer) = UnixStream::pair().unwrap();
    (
        Arc::new(Transport {
            socket: Mutex::new(Some(socket)),
            current: AtomicBool::new(true),
            running: AtomicBool::new(true),
            checks: AtomicUsize::new(0),
            verifies: AtomicUsize::new(0),
            connects: AtomicUsize::new(0),
            block_verify_at: AtomicUsize::new(0),
        }),
        peer,
    )
}

// Drive the original borrowed owner future and one retained decoder in this
// task. Finite yields allow reactor progress without a detached peer or an
// unbounded wait; an early owner completion is itself a test failure.
async fn request_while<F: Future>(mut owner: Pin<&mut F>, peer: &mut UnixStream) -> wire::Request {
    let mut request = Box::pin(wire::read_request(peer));
    for _ in 0..128 {
        assert!(poll_once(owner.as_mut()).await.is_pending());
        if let Poll::Ready(request) = poll_once(request.as_mut()).await {
            return request.unwrap();
        }
        tokio::task::yield_now().await;
    }
    panic!("owner did not send the expected bounded wire request");
}

async fn welcome(peer: &mut UnixStream, request: wire::Request) {
    let wire::Request::Hello(hello) = request else {
        panic!("expected Hello, not an implicit policy request");
    };
    assert_eq!(hello.version, wire::VERSION);
    wire::write_reply(
        peer,
        &wire::Reply::Hello(wire::Welcome {
            version: wire::VERSION,
            session: wire::BrokerSession {
                controller: hello.controller,
                broker: wire::Id::from_bytes([9; 32]).unwrap(),
                connection: 1,
            },
        }),
    )
    .await
    .unwrap();
}

async fn attached(fixture: &mut Fixture) -> (Arc<Transport>, UnixStream) {
    let (transport, mut peer) = transport();
    let mut attaching = Box::pin(
        fixture
            .owner
            .attach_broker_transport(transport.clone(), Instant::now() + Duration::from_secs(5)),
    );
    let hello = request_while(attaching.as_mut(), &mut peer).await;
    welcome(&mut peer, hello).await;
    attaching.await.unwrap();
    assert_eq!(transport.connects.load(Ordering::SeqCst), 1);
    (transport, peer)
}

async fn answer_probe(peer: &mut UnixStream, request: wire::Request) {
    let wire::Request::Probe(session) = request else {
        panic!("expected Probe, not Apply/Finish or another Hello");
    };
    wire::write_reply(
        peer,
        &wire::Reply::Probe(wire::Probe {
            session,
            current: true,
        }),
    )
    .await
    .unwrap();
}

async fn probe(owner: &mut HostControlOwner<Guest>, peer: &mut UnixStream) {
    let mut probing = Box::pin(owner.probe_broker());
    let request = request_while(probing.as_mut(), peer).await;
    answer_probe(peer, request).await;
    probing.await.unwrap();
}

async fn closed_without_more_requests(peer: &mut UnixStream) {
    let mut byte = [0; 1];
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(1), peer.read(&mut byte))
            .await
            .expect("original stream remained open")
            .unwrap(),
        0,
        "unexpected policy, replay or other bytes preceded closure"
    );
}

fn assert_not_used(transport: &Transport) {
    assert_eq!(transport.checks.load(Ordering::SeqCst), 0);
    assert_eq!(transport.verifies.load(Ordering::SeqCst), 0);
    assert_eq!(transport.connects.load(Ordering::SeqCst), 0);
}

fn inspect(fixture: &mut Fixture) -> SshStatus {
    fixture
        .owner
        .dispatcher
        .custody_mut()
        .request(
            &AuthenticatedCaller::local_operator(fixture.uid),
            SshRequest::Inspect {
                launch: fixture.launch.clone(),
            },
        )
        .unwrap()
        .status
}

#[tokio::test(start_paused = true)]
async fn owner_hello_probe_and_reconcile_never_promote_ready_or_send_apply() {
    let mut fixture = fixture();
    let (transport, mut peer) = attached(&mut fixture).await;
    let status = inspect(&mut fixture);
    assert!(!status.ready);
    assert!(status.observed.is_none());
    tokio::time::advance(Duration::from_secs(wire::MANAGEMENT_PROBE_SECS)).await;
    probe(&mut fixture.owner, &mut peer).await;
    let status = inspect(&mut fixture);
    assert!(!status.ready);
    assert!(status.observed.is_none());
    let mut reply = queue(
        &fixture.owner,
        fixture.uid,
        ControlRequest::SshCustody(SshRequest::Reconcile {
            launch: fixture.launch.clone(),
            expected_revision: status.desired.revision,
            credentials: BTreeSet::new(),
        }),
    );
    let mut serving = Box::pin(fixture.owner.serve_until(std::future::pending()));
    let mut response = None;
    for _ in 0..128 {
        assert!(poll_once(serving.as_mut()).await.is_pending());
        if let Ok(value) = reply.try_recv() {
            response = Some(value);
            break;
        }
        tokio::task::yield_now().await;
    }
    let Some(Ok(ControlResponse::SshCustody(updated))) = response else {
        panic!("reconcile was not answered");
    };
    assert_eq!(updated.desired.revision, status.desired.revision + 1);
    assert!(!updated.ready);
    assert!(updated.observed.is_none());
    drop(serving);
    closed_without_more_requests(&mut peer).await;
    assert_eq!(transport.connects.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.native.state.lock().unwrap().vm_stop_calls, 0);
}

#[tokio::test(start_paused = true)]
async fn owner_due_probe_precedes_a_full_request_queue() {
    let mut fixture = fixture();
    let (_transport, mut peer) = attached(&mut fixture).await;
    let mut replies = Vec::new();
    for _ in 0..QUEUE_CAPACITY {
        replies.push(queue(
            &fixture.owner,
            fixture.uid,
            ControlRequest::Capabilities {
                launch: fixture.launch.clone(),
            },
        ));
    }
    assert_eq!(QUEUE_CAPACITY, 32);
    assert_eq!(fixture.owner.sender.capacity(), 0);
    tokio::time::advance(Duration::from_secs(wire::MANAGEMENT_PROBE_SECS)).await;
    let mut serving = Box::pin(fixture.owner.serve_until(std::future::pending()));
    let request = request_while(serving.as_mut(), &mut peer).await;
    assert!(matches!(request, wire::Request::Probe(_)));
    assert!(fixture.native.state.lock().unwrap().entered.is_empty());
    for reply in &mut replies {
        assert!(matches!(
            reply.try_recv(),
            Err(oneshot::error::TryRecvError::Empty)
        ));
    }
    answer_probe(&mut peer, request).await;
    let mut answered = false;
    for _ in 0..128 {
        assert!(poll_once(serving.as_mut()).await.is_pending());
        if let Ok(response) = replies[0].try_recv() {
            assert!(matches!(response, Ok(ControlResponse::Capabilities { .. })));
            answered = true;
            break;
        }
        tokio::task::yield_now().await;
    }
    assert!(answered, "queue did not resume after the due Probe");
    drop(serving);
    closed_without_more_requests(&mut peer).await;
    assert_eq!(fixture.native.state.lock().unwrap().vm_stop_calls, 0);
}

#[tokio::test(start_paused = true)]
async fn owner_expired_idle_lease_closes_without_a_late_probe_or_redial() {
    let mut fixture = fixture();
    let (transport, mut peer) = attached(&mut fixture).await;
    tokio::time::advance(Duration::from_secs(wire::MANAGEMENT_LEASE_SECS)).await;
    let exit = fixture.owner.serve_until(std::future::pending()).await;
    assert!(exit.primary.is_err());
    assert!(exit.drain.complete());
    assert!(fixture.owner.path().is_err());
    closed_without_more_requests(&mut peer).await;
    assert_eq!(transport.connects.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.native.state.lock().unwrap().vm_stop_calls, 0);
}

#[tokio::test(start_paused = true)]
async fn owner_idle_shutdown_and_drop_close_link_without_stopping_adopted_vm() {
    for shutdown in [true, false] {
        let mut fixture = fixture();
        let (transport, mut peer) = attached(&mut fixture).await;
        if shutdown {
            let exit = fixture.owner.serve_until(std::future::ready(Ok(()))).await;
            assert!(exit.primary.is_ok());
            assert!(exit.drain.complete());
            assert!(fixture.owner.path().is_err());
        } else {
            drop(fixture.owner);
        }
        closed_without_more_requests(&mut peer).await;
        assert_eq!(transport.connects.load(Ordering::SeqCst), 1);
        let state = fixture.native.state.lock().unwrap();
        assert_eq!(state.vm_stop_calls, 0);
        assert!(state.current.contains(&fixture.launch));
    }
}

#[tokio::test(start_paused = true)]
async fn owner_partial_hello_cancellation_or_eof_closes_and_never_redials() {
    for cancel in [true, false] {
        let mut fixture = fixture();
        let (original, mut peer) = transport();
        let mut attaching = Box::pin(
            fixture
                .owner
                .attach_broker_transport(original.clone(), Instant::now() + Duration::from_secs(5)),
        );
        assert!(matches!(
            request_while(attaching.as_mut(), &mut peer).await,
            wire::Request::Hello(_)
        ));
        if cancel {
            // Cancel while waiting for Welcome, with no unread reply bytes
            // that could turn peer EOF into a Unix connection-reset result.
            drop(attaching);
        } else {
            // A truncated Welcome must be consumed and refused, not mistaken
            // for a completed attachment eligible for reuse.
            peer.write_all(&[0, 0]).await.unwrap();
            peer.shutdown().await.unwrap();
            assert!(attaching.await.is_err());
        }
        assert!(fixture.owner.path().is_err());
        closed_without_more_requests(&mut peer).await;
        let (replacement, _unused_peer) = transport();
        assert!(
            fixture
                .owner
                .attach_broker_transport(
                    replacement.clone(),
                    Instant::now() + Duration::from_secs(5),
                )
                .await
                .is_err()
        );
        assert_not_used(&replacement);
        assert_eq!(original.connects.load(Ordering::SeqCst), 1);
        assert_eq!(fixture.native.state.lock().unwrap().vm_stop_calls, 0);
    }
}

#[tokio::test(start_paused = true)]
async fn owner_endpoint_store_route_and_runtime_loss_close_original_idle_link() {
    for loss in ["endpoint", "store", "route", "runtime"] {
        let mut fixture = fixture();
        let (transport, mut peer) = attached(&mut fixture).await;
        let mut serving = Box::pin(fixture.owner.serve_until(std::future::pending()));
        assert!(poll_once(serving.as_mut()).await.is_pending());
        match loss {
            "endpoint" => replace_endpoint_owner(fixture.root.path()),
            "store" => replace_desired(fixture.root.path()),
            "route" => transport.current.store(false, Ordering::SeqCst),
            "runtime" => transport.running.store(false, Ordering::SeqCst),
            _ => unreachable!(),
        }
        tokio::time::advance(Duration::from_secs(1)).await;
        let exit = tokio::time::timeout(Duration::from_secs(6), serving)
            .await
            .expect("idle loss exceeded the existing bounded observation");
        assert!(exit.primary.is_err(), "{loss}");
        assert!(exit.drain.complete(), "{loss}");
        assert!(fixture.owner.path().is_err(), "{loss}");
        closed_without_more_requests(&mut peer).await;
        assert_eq!(transport.connects.load(Ordering::SeqCst), 1, "{loss}");
        let state = fixture.native.state.lock().unwrap();
        assert_eq!(state.vm_stop_calls, 0, "{loss}");
        assert!(state.current.contains(&fixture.launch), "{loss}");
    }
}

#[tokio::test(start_paused = true)]
async fn owner_route_loss_during_native_call_closes_link_and_retains_uncertain_lease() {
    let mut fixture = fixture();
    let (transport, mut peer) = attached(&mut fixture).await;
    fixture.native.state.lock().unwrap().blocked.insert("start");
    let _reply = queue(&fixture.owner, fixture.uid, start_request(&fixture.launch));
    let mut serving = Box::pin(fixture.owner.serve_until(std::future::pending()));
    drive_until(serving.as_mut(), || fixture.native.has_entered("start")).await;
    transport.current.store(false, Ordering::SeqCst);
    tokio::time::advance(Duration::from_secs(1)).await;
    let exit = tokio::time::timeout(Duration::from_secs(6), serving)
        .await
        .expect("route loss did not fence the in-flight native call");
    assert!(exit.primary.is_err());
    assert_eq!(exit.drain.operations.pending.len(), 1);
    assert!(!exit.drain.complete());
    closed_without_more_requests(&mut peer).await;
    assert_eq!(transport.connects.load(Ordering::SeqCst), 1);
    let state = fixture.native.state.lock().unwrap();
    assert!(state.abandoned.contains(&("start", 0)));
    assert_eq!(state.next_session, 0);
    assert_eq!(state.vm_stop_calls, 0);
}

#[tokio::test(start_paused = true)]
async fn owner_healthy_duplicate_preserves_original_link_without_inspecting_replacement() {
    let mut fixture = fixture();
    let (original, mut peer) = attached(&mut fixture).await;
    let (replacement, _unused_peer) = transport();
    assert!(
        fixture
            .owner
            .attach_broker_transport(replacement.clone(), Instant::now() + Duration::from_secs(5),)
            .await
            .is_err()
    );
    assert_not_used(&replacement);
    assert!(fixture.owner.path().is_ok());
    tokio::time::advance(Duration::from_secs(wire::MANAGEMENT_PROBE_SECS)).await;
    probe(&mut fixture.owner, &mut peer).await;
    assert_eq!(original.connects.load(Ordering::SeqCst), 1);
    assert!(
        fixture
            .owner
            .drain_until(Instant::now() + Duration::from_secs(1))
            .await
            .complete()
    );
    closed_without_more_requests(&mut peer).await;
    assert_eq!(fixture.native.state.lock().unwrap().vm_stop_calls, 0);
}

#[tokio::test(start_paused = true)]
async fn owner_repeat_attach_after_observed_loss_closes_original_without_redial() {
    for loss in ["endpoint", "store", "route"] {
        let mut fixture = fixture();
        let (original, mut peer) = attached(&mut fixture).await;
        match loss {
            "endpoint" => replace_endpoint_owner(fixture.root.path()),
            "store" => replace_desired(fixture.root.path()),
            "route" => original.current.store(false, Ordering::SeqCst),
            _ => unreachable!(),
        }
        let (replacement, _unused_peer) = transport();
        assert!(
            fixture
                .owner
                .attach_broker_transport(
                    replacement.clone(),
                    Instant::now() + Duration::from_secs(5),
                )
                .await
                .is_err(),
            "{loss}"
        );
        assert_not_used(&replacement);
        assert!(fixture.owner.path().is_err(), "{loss}");
        closed_without_more_requests(&mut peer).await;
        assert_eq!(original.connects.load(Ordering::SeqCst), 1, "{loss}");
        assert_eq!(fixture.native.state.lock().unwrap().vm_stop_calls, 0);
    }
}

#[tokio::test(start_paused = true)]
async fn owner_cancelled_post_hello_health_check_closes_installed_original_link() {
    let mut fixture = fixture();
    let (transport, mut peer) = transport();
    transport.block_verify_at.store(2, Ordering::SeqCst);
    let mut attaching = Box::pin(
        fixture
            .owner
            .attach_broker_transport(transport.clone(), Instant::now() + Duration::from_secs(5)),
    );
    let hello = request_while(attaching.as_mut(), &mut peer).await;
    welcome(&mut peer, hello).await;
    drive_until(attaching.as_mut(), || {
        transport.verifies.load(Ordering::SeqCst) == 2
    })
    .await;
    drop(attaching);
    assert!(fixture.owner.path().is_err());
    closed_without_more_requests(&mut peer).await;
    assert_eq!(transport.connects.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.native.state.lock().unwrap().vm_stop_calls, 0);
}

#[tokio::test]
async fn owner_serve_future_is_send_with_a_retained_non_sync_native_exec() {
    fn assert_send<T: Send>(value: T) -> T {
        value
    }
    let mut fixture = fixture();
    let operation = start(&mut fixture).await;
    // NativeExec promises Send, not Sync. This compile-time assertion catches
    // an immutable borrow of the entire dispatcher surviving a health await.
    let serving = assert_send(fixture.owner.serve_until(std::future::ready(Ok(()))));
    let exit = serving.await;
    assert!(exit.primary.is_ok());
    assert!(exit.drain.complete());
    assert_eq!(exit.drain.operations.completed, vec![operation]);
    assert_eq!(fixture.native.state.lock().unwrap().vm_stop_calls, 0);
}
