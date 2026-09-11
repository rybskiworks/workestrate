//! Synthetic stream-pair contracts; no VM, provider, or ambient credentials.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::os::unix::net::UnixStream;
use std::sync::atomic::AtomicUsize;

fn id(byte: u8) -> managed_wire::Id {
    managed_wire::Id::from_bytes([byte; 32]).unwrap()
}

fn launch() -> managed_wire::LaunchRef {
    managed_wire::LaunchRef {
        instance: managed_wire::InstanceRef {
            workload: managed_wire::WorkloadRef {
                context: Some("personal".into()),
                name: "agent".into(),
            },
            instance: "personal-agent".into(),
        },
        generation: id(1),
    }
}

fn observation() -> managed_wire::Observation {
    managed_wire::Observation {
        transaction: managed_wire::TransactionRef {
            session: managed_wire::BrokerSession {
                controller: id(2),
                broker: id(3),
                connection: 1,
            },
            launch: launch(),
            revision: 4,
            policy_digest: id(5),
        },
        outcome: managed_wire::Outcome::Applied,
        failure: None,
    }
}

fn destination() -> DivertDestination {
    DivertDestination {
        instance: "personal-agent".into(),
        cid: 65536,
        // Intentionally not a resolved address or a lowercased substitute.
        dest_host: "Git.Example.test".into(),
        dest_port: 2222,
        credentials: vec![SshGrantPlan {
            name: "source".into(),
            material: "selected-key".into(),
            hosts: vec!["Git.Example.test".into()],
            users: vec!["git".into()],
            ports: vec![2222],
            binding: CredentialBinding::Broker,
            on_violation: crate::config::SecretViolationPolicy::Block,
        }],
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .unwrap()
}

fn relay(runtime: &tokio::runtime::Runtime, admission: Arc<ManagedAdmission>) -> BrokerSocketRelay {
    BrokerSocketRelay::managed(
        PathBuf::from("/fixture/private/broker.sock"),
        launch(),
        65536,
        runtime.handle().clone(),
        admission,
    )
    .unwrap()
}

fn pair() -> (UnixStream, UnixStream) {
    let pair = UnixStream::pair().unwrap();
    for stream in [&pair.0, &pair.1] {
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
    }
    pair
}

fn assert_refused_before_dial(relay: &BrokerSocketRelay, dest: &DivertDestination) {
    let (stream, _guest) = pair();
    let mut dialed = false;
    let result = relay.relay_with_dial(stream, dest, || {
        dialed = true;
        Err(std::io::Error::other("unexpected dial"))
    });
    assert_eq!(
        result.unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert!(!dialed);
}

fn captured_relay(relay: &BrokerSocketRelay, dest: &DivertDestination) -> (Vec<u8>, Vec<u8>) {
    let (stream, mut guest) = pair();
    let (broker, mut upstream) = pair();
    guest.write_all(b"SSH-2.0-fixture\r\n\0\xffbinary").unwrap();
    guest.shutdown(Shutdown::Write).unwrap();
    upstream.write_all(b"server\0\xfe").unwrap();
    upstream.shutdown(Shutdown::Write).unwrap();
    relay.relay_with_dial(stream, dest, || Ok(broker)).unwrap();
    let (mut sent, mut received) = (Vec::new(), Vec::new());
    upstream.read_to_end(&mut sent).unwrap();
    guest.read_to_end(&mut received).unwrap();
    (sent, received)
}

#[test]
fn managed_relay_shared_header_preserves_hostname_and_native_bytes() {
    let runtime = runtime();
    let calls = Arc::new(AtomicUsize::new(0));
    let relay = relay(&runtime, {
        let calls = Arc::clone(&calls);
        Arc::new(move |dest| {
            assert_eq!(dest, &destination());
            calls.fetch_add(1, Ordering::Relaxed);
            Ok(observation())
        })
    });
    let (sent, received) = captured_relay(&relay, &destination());
    let mut remaining = sent.as_slice();
    let header = runtime
        .block_on(managed_wire::read_divert(&mut remaining))
        .unwrap();
    assert_eq!(header.version, managed_wire::VERSION);
    assert_eq!(header.transaction, observation().transaction);
    assert_eq!(header.destination_host, "Git.Example.test");
    assert_eq!(header.destination_port, 2222);
    assert_eq!(remaining, b"SSH-2.0-fixture\r\n\0\xffbinary");
    assert_eq!(received, b"server\0\xfe");
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    assert!(decode_ssh_divert_prelude(&sent).is_err());
}

#[test]
fn managed_relay_rejects_wrong_source_before_owner_query_or_dial() {
    let runtime = runtime();
    let relay = relay(&runtime, Arc::new(|_| panic!("wrong source reached owner")));
    for variant in 0..4 {
        let mut dest = destination();
        match variant {
            0 => dest.instance = "other-agent".into(),
            1 => dest.cid += 1,
            2 => dest.credentials.clear(),
            _ => dest.credentials[0].binding = CredentialBinding::Guest,
        }
        assert_refused_before_dial(&relay, &dest);
    }
}

#[test]
fn managed_relay_rejects_pending_lost_rejected_and_malformed_applied() {
    let runtime = runtime();
    for variant in 0..9 {
        let relay = relay(
            &runtime,
            Arc::new(move |_| {
                let mut value = observation();
                match variant {
                    0 => value.outcome = managed_wire::Outcome::Pending,
                    1 => value.outcome = managed_wire::Outcome::StateLost,
                    2 => value.outcome = managed_wire::Outcome::Rejected,
                    3 => value.failure = Some(managed_wire::Failure::Unavailable),
                    4 => value.transaction.launch.generation = id(8),
                    5 => value.transaction.launch.instance.workload.context = Some("other".into()),
                    6 => value.transaction.launch.instance.workload.name = "other".into(),
                    7 => value.transaction.revision = 0,
                    _ => value.transaction.session.connection = 0,
                }
                Ok(value)
            }),
        );
        assert_refused_before_dial(&relay, &destination());
    }
}

#[test]
fn managed_relay_preserves_owner_refusal_without_private_diagnostic() {
    let runtime = runtime();
    let relay = relay(
        &runtime,
        Arc::new(|_| Err(std::io::Error::other("secret sentinel"))),
    );
    let (stream, _guest) = pair();
    let result = relay.relay_with_dial(stream, &destination(), || panic!("unexpected dial"));
    assert_eq!(
        result.unwrap_err().to_string(),
        "managed SSH relay admission is not current"
    );
}

#[test]
fn managed_relay_rejects_invalid_original_destination_before_dial() {
    let runtime = runtime();
    let relay = relay(&runtime, Arc::new(|_| Ok(observation())));
    for variant in 0..3 {
        let mut dest = destination();
        match variant {
            0 => dest.dest_port = 0,
            1 => dest.dest_host.clear(),
            _ => dest.dest_host = "h".repeat(managed_wire::MAX_DIVERT_BYTES),
        }
        assert_refused_before_dial(&relay, &dest);
    }
}

#[test]
fn managed_relay_rotation_or_loss_after_dial_writes_no_header() {
    let runtime = runtime();
    for variant in 0..7 {
        let connected = Arc::new(AtomicBool::new(false));
        let relay = relay(&runtime, {
            let connected = Arc::clone(&connected);
            Arc::new(move |_| {
                let mut value = observation();
                if connected.load(Ordering::Relaxed) {
                    match variant {
                        0 => return Err(std::io::Error::other("owner lost")),
                        1 => value.outcome = managed_wire::Outcome::Pending,
                        2 => value.outcome = managed_wire::Outcome::StateLost,
                        3 => value.transaction.revision += 1,
                        4 => value.transaction.policy_digest = id(9),
                        5 => value.transaction.session.connection += 1,
                        _ => value.transaction.launch.generation = id(9),
                    }
                }
                Ok(value)
            })
        });
        let (stream, _guest) = pair();
        let (broker, mut upstream) = pair();
        let result = relay.relay_with_dial(stream, &destination(), || {
            connected.store(true, Ordering::Relaxed);
            Ok(broker)
        });
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        let mut bytes = Vec::new();
        upstream.read_to_end(&mut bytes).unwrap();
        assert!(bytes.is_empty(), "stale admission emitted bytes");
    }
}

#[test]
fn managed_relay_current_owner_can_refuse_changed_destination() {
    let runtime = runtime();
    let relay = relay(
        &runtime,
        Arc::new(|dest| {
            if dest.dest_host != "Git.Example.test" || dest.dest_port != 2222 {
                return Err(managed_refusal());
            }
            Ok(observation())
        }),
    );
    for variant in 0..2 {
        let mut dest = destination();
        if variant == 0 {
            dest.dest_host = "192.0.2.1".into();
        } else {
            dest.dest_port = 22;
        }
        assert_refused_before_dial(&relay, &dest);
    }
}

#[test]
fn managed_relay_refuses_runtime_worker_instead_of_blocking_it() {
    let runtime = runtime();
    let relay = relay(&runtime, Arc::new(|_| panic!("async worker queried owner")));
    runtime.block_on(async {
        assert_refused_before_dial(&relay, &destination());
    });
}

#[test]
fn managed_relay_failed_dial_does_not_requery_or_forward() {
    let runtime = runtime();
    let calls = Arc::new(AtomicUsize::new(0));
    let relay = relay(&runtime, {
        let calls = Arc::clone(&calls);
        Arc::new(move |_| {
            calls.fetch_add(1, Ordering::Relaxed);
            Ok(observation())
        })
    });
    let (stream, mut guest) = pair();
    let result = relay.relay_with_dial(stream, &destination(), || {
        Err(std::io::Error::from(std::io::ErrorKind::ConnectionRefused))
    });
    assert_eq!(
        result.unwrap_err().kind(),
        std::io::ErrorKind::ConnectionRefused
    );
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    let mut bytes = Vec::new();
    guest.read_to_end(&mut bytes).unwrap();
    assert!(bytes.is_empty());
}

#[test]
fn managed_relay_constructor_rejects_reserved_cid_and_unscoped_paths() {
    let runtime = runtime();
    for (path, cid) in [
        ("/private/broker.sock", 2),
        ("/private/broker.sock", u32::MAX),
        ("relative.sock", 65536),
        ("/private/../broker.sock", 65536),
    ] {
        assert!(
            BrokerSocketRelay::managed(
                path.into(),
                launch(),
                cid,
                runtime.handle().clone(),
                Arc::new(|_| Ok(observation())),
            )
            .is_err()
        );
    }
}

#[test]
fn managed_relay_does_not_change_legacy_constructor_or_first_relay_defaults() {
    let path = PathBuf::from("/fixture/private/broker.sock");
    let runtime = runtime();
    let first = BrokerFirstRelay::managed(
        path.clone(),
        launch(),
        65536,
        runtime.handle().clone(),
        Arc::new(|_| Ok(observation())),
    )
    .unwrap();
    assert!(first.broker.managed.is_some());
    assert!(BrokerFirstRelay::new(path.clone()).broker.managed.is_none());
    for relay in [
        BrokerSocketRelay::new(path.clone()),
        BrokerSocketRelay::with_timeout(path, Duration::from_secs(2)),
    ] {
        assert!(relay.managed.is_none());
        let (sent, received) = captured_relay(&relay, &destination());
        let (prelude, consumed) = decode_ssh_divert_prelude(&sent).unwrap();
        assert_eq!(prelude.dest_host, destination().dest_host);
        assert_eq!(prelude.dest_port, destination().dest_port);
        assert_eq!(prelude.transport_cid, 65536);
        assert!(prelude.epoch > 0);
        assert_eq!(&sent[consumed..], b"SSH-2.0-fixture\r\n\0\xffbinary");
        assert_eq!(received, b"server\0\xfe");
        assert!(
            runtime
                .block_on(managed_wire::read_divert(&mut sent.as_slice()))
                .is_err()
        );
    }
}

fn asynchronous(stream: UnixStream) -> std::io::Result<tokio::net::UnixStream> {
    stream.set_nonblocking(true)?;
    tokio::net::UnixStream::from_std(stream)
}

#[test]
fn owned_managed_copy_preserves_half_close_and_runs_on_a_current_thread_owner() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let relay = relay(&runtime, Arc::new(|_| Ok(observation())));
    let (stream, guest) = pair();
    let (broker, upstream) = pair();
    let worker = std::thread::spawn(move || {
        relay.managed_with_connect(
            stream,
            &destination(),
            &AtomicBool::new(false),
            || async { asynchronous(broker) },
            relay.managed.as_ref().unwrap(),
        )
    });
    runtime.block_on(async {
        tokio::time::timeout(Duration::from_secs(2), async {
            let mut guest = asynchronous(guest).unwrap();
            let mut upstream = asynchronous(upstream).unwrap();
            let header = managed_wire::read_divert(&mut upstream).await.unwrap();
            assert_eq!(header.transaction, observation().transaction);
            tokio::io::AsyncWriteExt::write_all(&mut guest, b"request\0\xff")
                .await
                .unwrap();
            tokio::io::AsyncWriteExt::shutdown(&mut guest)
                .await
                .unwrap();
            let mut request = Vec::new();
            tokio::io::AsyncReadExt::read_to_end(&mut upstream, &mut request)
                .await
                .unwrap();
            assert_eq!(request, b"request\0\xff");
            // Guest write EOF must not close its still-readable response half.
            tokio::io::AsyncWriteExt::write_all(&mut upstream, b"response\0\xfe")
                .await
                .unwrap();
            tokio::io::AsyncWriteExt::shutdown(&mut upstream)
                .await
                .unwrap();
            let mut response = Vec::new();
            tokio::io::AsyncReadExt::read_to_end(&mut guest, &mut response)
                .await
                .unwrap();
            assert_eq!(response, b"response\0\xfe");
            while !worker.is_finished() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
    });
    worker.join().unwrap().unwrap();
}

#[test]
fn owned_managed_stop_before_admission_has_no_query_or_connect() {
    let runtime = runtime();
    let relay = relay(&runtime, Arc::new(|_| panic!("cancelled owner queried")));
    let (stream, _guest) = pair();
    let error = relay
        .managed_with_connect(
            stream,
            &destination(),
            &AtomicBool::new(true),
            || async { panic!("cancelled connect entered") },
            relay.managed.as_ref().unwrap(),
        )
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::Interrupted);
}

#[test]
fn owned_managed_stop_during_connect_drops_the_connect_future() {
    struct Dropped(Arc<AtomicBool>);
    impl Drop for Dropped {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }
    let runtime = runtime();
    let mut relay = relay(&runtime, Arc::new(|_| Ok(observation())));
    relay.connect_timeout = Duration::from_millis(500);
    let (stream, mut guest) = pair();
    let stop = Arc::new(AtomicBool::new(false));
    let cancelled = Arc::clone(&stop);
    let dropped = Arc::new(AtomicBool::new(false));
    let drop_seen = Arc::clone(&dropped);
    let (entered, ready) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        relay.managed_with_connect(
            stream,
            &destination(),
            &cancelled,
            || async {
                let _guard = Dropped(drop_seen);
                entered.send(()).unwrap();
                std::future::pending().await
            },
            relay.managed.as_ref().unwrap(),
        )
    });
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    stop.store(true, Ordering::Release);
    assert_eq!(
        worker.join().unwrap().unwrap_err().kind(),
        std::io::ErrorKind::Interrupted
    );
    assert!(dropped.load(Ordering::Acquire));
    let mut bytes = Vec::new();
    guest.read_to_end(&mut bytes).unwrap();
    assert!(bytes.is_empty());
}

#[test]
fn owned_managed_stop_at_second_admission_emits_no_header() {
    let runtime = runtime();
    let stop = Arc::new(AtomicBool::new(false));
    let seen = Arc::clone(&stop);
    let calls = AtomicUsize::new(0);
    let relay = relay(
        &runtime,
        Arc::new(move |_| {
            if calls.fetch_add(1, Ordering::Relaxed) == 1 {
                seen.store(true, Ordering::Release);
            }
            Ok(observation())
        }),
    );
    let (stream, mut guest) = pair();
    let (broker, mut upstream) = pair();
    let error = relay
        .managed_with_connect(
            stream,
            &destination(),
            &stop,
            || async { asynchronous(broker) },
            relay.managed.as_ref().unwrap(),
        )
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::Interrupted);
    for peer in [&mut guest, &mut upstream] {
        let mut bytes = Vec::new();
        peer.read_to_end(&mut bytes).unwrap();
        assert!(bytes.is_empty());
    }
}

#[test]
fn owned_managed_two_idle_copies_cancel_and_join_both_socket_owners() {
    let runtime = runtime();
    let mut workers = RelayWorkers::new();
    let mut guests = Vec::new();
    let mut upstreams = Vec::new();
    // Exercise the actual owned-loop entry with real private Unix listeners.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("broker.sock");
    let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    listener.set_nonblocking(true).unwrap();
    let mut relay = relay(&runtime, Arc::new(|_| Ok(observation())));
    relay.broker_socket = path;
    for _ in 0..2 {
        let (stream, guest) = pair();
        workers.spawn(relay.clone(), stream, destination()).unwrap();
        guests.push(guest);
    }
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while upstreams.len() < 2 {
        match listener.accept() {
            Ok((stream, _)) => {
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                upstreams.push(stream);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("fixture accept: {error}"),
        }
    }
    for upstream in &mut upstreams {
        // This fixture reads only the bounded managed header before stopping.
        let mut length = [0; 4];
        upstream.read_exact(&mut length).unwrap();
        let length = u32::from_be_bytes(length) as usize;
        assert!(length < 65536);
        let mut body = vec![0; length];
        upstream.read_exact(&mut body).unwrap();
    }
    workers.cancel();
    while !workers.entries.is_empty() && std::time::Instant::now() < deadline {
        workers.reap();
        std::thread::sleep(Duration::from_millis(5));
    }
    let joined_before_peer_cleanup = workers.entries.is_empty();
    // On a regression close fixture peers before asserting, so cleanup remains
    // bounded by EOF even if the cancellation branch is broken.
    for peer in guests.iter().chain(&upstreams) {
        peer.shutdown(Shutdown::Both).unwrap();
    }
    workers.drain().unwrap();
    assert!(
        joined_before_peer_cleanup,
        "cancellation did not retire idle copies"
    );
    assert!(workers.entries.is_empty());
    assert!(!workers.failed);
}

fn fill_send_buffer(stream: &mut UnixStream) -> usize {
    stream.set_nonblocking(true).unwrap();
    let mut bytes = 0;
    let mut chunk = 4096;
    loop {
        match stream.write(&[0xa5; 4096][..chunk]) {
            Ok(length) => {
                assert!(length > 0);
                bytes += length;
                assert!(bytes <= 1024 * 1024, "fixture socket buffer exceeds bound");
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock && chunk > 1 => {
                chunk = 1;
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => return bytes,
            Err(error) => panic!("fixture buffer fill: {error}"),
        }
    }
}

#[test]
fn owned_managed_header_backpressure_cancels_and_closes_the_socket() {
    let runtime = runtime();
    let stop = Arc::new(AtomicBool::new(false));
    let signal = Arc::clone(&stop);
    let (entered, ready) = std::sync::mpsc::channel();
    let calls = AtomicUsize::new(0);
    let mut relay = relay(
        &runtime,
        Arc::new(move |_| {
            if calls.fetch_add(1, Ordering::Relaxed) == 1 {
                entered.send(()).unwrap();
            }
            Ok(observation())
        }),
    );
    relay.connect_timeout = Duration::from_millis(500);
    let (stream, _guest) = pair();
    let (mut broker, mut upstream) = pair();
    let filler = fill_send_buffer(&mut broker);
    let worker = std::thread::spawn(move || {
        relay.managed_with_connect(
            stream,
            &destination(),
            &signal,
            || async { asynchronous(broker) },
            relay.managed.as_ref().unwrap(),
        )
    });
    ready.recv_timeout(Duration::from_secs(2)).unwrap();
    stop.store(true, Ordering::Release);
    assert_eq!(
        worker.join().unwrap().unwrap_err().kind(),
        std::io::ErrorKind::Interrupted
    );
    let mut bytes = Vec::new();
    upstream.read_to_end(&mut bytes).unwrap();
    assert_eq!(
        bytes,
        vec![0xa5; filler],
        "cancelled header leaked beyond full buffer"
    );
}

#[test]
fn owned_managed_setup_deadline_covers_pending_connect_and_header_write() {
    let runtime = runtime();
    for blocked_header in [false, true] {
        let mut relay = relay(&runtime, Arc::new(|_| Ok(observation())));
        relay.connect_timeout = Duration::from_millis(25);
        let (stream, _guest) = pair();
        let (mut broker, mut upstream) = pair();
        let filler = if blocked_header {
            fill_send_buffer(&mut broker)
        } else {
            0
        };
        let error = relay
            .managed_with_connect(
                stream,
                &destination(),
                &AtomicBool::new(false),
                || async {
                    if !blocked_header {
                        std::future::pending::<()>().await;
                    }
                    asynchronous(broker)
                },
                relay.managed.as_ref().unwrap(),
            )
            .unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::TimedOut);
        let mut bytes = Vec::new();
        upstream.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, vec![0xa5; filler]);
    }
}

#[derive(Clone, Debug)]
struct HeldRelay {
    release: Arc<AtomicBool>,
    calls: Arc<AtomicUsize>,
}

impl SshRelay for HeldRelay {
    fn relay(&self, _stream: UnixStream, _dest: &DivertDestination) -> std::io::Result<()> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while !self.release.load(Ordering::Acquire) && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }
}

#[test]
fn owned_worker_capacity_precedes_relay_effects_and_finished_slots_are_reaped() {
    let mut workers = RelayWorkers::new();
    let relay = HeldRelay {
        release: Arc::new(AtomicBool::new(false)),
        calls: Arc::new(AtomicUsize::new(0)),
    };
    let mut peers = Vec::new();
    for _ in 0..MAX_RELAY_WORKERS {
        let (stream, peer) = pair();
        workers.spawn(relay.clone(), stream, destination()).unwrap();
        peers.push(peer);
    }
    let (stream, _peer) = pair();
    let refused = workers
        .spawn(relay.clone(), stream, destination())
        .unwrap_err();
    assert_eq!(refused.kind(), std::io::ErrorKind::WouldBlock);
    assert_eq!(workers.entries.len(), MAX_RELAY_WORKERS);
    relay.release.store(true, Ordering::Release);
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while workers
        .entries
        .iter()
        .any(|entry| !entry.join.is_finished())
    {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(relay.calls.load(Ordering::Relaxed), MAX_RELAY_WORKERS);
    // spawn itself, not the test, must reap completed entries before admission.
    let (stream, _peer) = pair();
    workers.spawn(relay.clone(), stream, destination()).unwrap();
    assert_eq!(workers.entries.len(), 1);
    workers.drain().unwrap();
}

#[derive(Clone, Debug)]
struct FailedRelay(bool);

impl SshRelay for FailedRelay {
    fn relay(&self, _stream: UnixStream, _dest: &DivertDestination) -> std::io::Result<()> {
        assert!(!self.0, "synthetic relay panic");
        Err(std::io::Error::other("synthetic relay refusal"))
    }
}

#[test]
fn owned_workers_visit_all_joins_and_distinguish_refusal_from_panic() {
    for panic in [false, true] {
        let mut workers = RelayWorkers::new();
        let (stream, _peer) = pair();
        workers
            .spawn(FailedRelay(panic), stream, destination())
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while workers
            .entries
            .iter()
            .any(|entry| !entry.join.is_finished())
        {
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(workers.drain().is_err(), panic);
        assert!(workers.entries.is_empty());
        assert_eq!(workers.failed, panic);
        let (stream, _peer) = pair();
        assert!(
            workers
                .spawn(FailedRelay(false), stream, destination())
                .is_err()
        );
    }
}
