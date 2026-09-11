#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;
use tokio::time::{Instant, timeout};

struct TimedChannel {
    reply_delay: Duration,
    dropped: Arc<AtomicBool>,
    requested: Arc<AtomicBool>,
}

impl Drop for TimedChannel {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

fn echo_body(body: &[u8]) -> Vec<u8> {
    let envelope: DecodedEnvelope = ciborium::from_reader(body).unwrap();
    assert_eq!(envelope.t, SSH_EPOCH_PROVISION_WIRE);
    let provision: SshEpochProvision = ciborium::from_reader(&envelope.p[..]).unwrap();
    encode_envelope(
        EPOCH_INTRO_GENERATION,
        SSH_EPOCH_ACK_WIRE,
        encode_payload(&SshEpochAck {
            cid: provision.cid,
            epoch: provision.epoch,
            ok: true,
        })
        .unwrap(),
    )
    .unwrap()
}

impl ConsoleChannel for TimedChannel {
    fn negotiated_generation(&self) -> u8 {
        EPOCH_INTRO_GENERATION
    }

    async fn request(&self, flags: u8, body: Vec<u8>) -> Result<ConsoleResponse, String> {
        self.requested.store(true, Ordering::SeqCst);
        tokio::time::sleep(self.reply_delay).await;
        Ok(ConsoleResponse {
            id: 1,
            flags,
            body: echo_body(&body),
        })
    }
}

#[tokio::test(start_paused = true)]
async fn connect_and_reply_share_one_deadline_and_drop_the_channel() {
    for (connect_secs, reply_secs, succeeds) in [
        (0, 9, true),
        (6, 3, true),
        (6, 6, false),
        (11, 0, false),
        (0, 11, false),
        (0, 10, false),
    ] {
        let dropped = Arc::new(AtomicBool::new(false));
        let channel = TimedChannel {
            reply_delay: Duration::from_secs(reply_secs),
            dropped: dropped.clone(),
            requested: Arc::new(AtomicBool::new(false)),
        };
        let started = Instant::now();
        let result = timeout(
            Duration::from_secs(30),
            provision_with_deadline(
                async move {
                    // Own the fixture across the connection wait as the SDK
                    // owns its socket while awaiting the relay handshake.
                    tokio::time::sleep(Duration::from_secs(connect_secs)).await;
                    Ok(channel)
                },
                "instance",
                7,
                12,
                started + Duration::from_secs(10),
            ),
        )
        .await
        .expect("independent watchdog bounds broken deadline implementations");
        assert_eq!(
            result,
            if succeeds {
                Ok(12)
            } else {
                Err(EpochProvisionError::DeadlineExceeded)
            },
            "connect={connect_secs}s, reply={reply_secs}s"
        );
        assert_eq!(
            Instant::now() - started,
            Duration::from_secs(if succeeds {
                connect_secs + reply_secs
            } else {
                10
            })
        );
        assert!(dropped.load(Ordering::SeqCst));
    }
}

#[tokio::test(start_paused = true)]
async fn expired_attempt_does_not_poll_the_connector() {
    let polled = AtomicBool::new(false);
    let result = provision_with_deadline(
        async {
            polled.store(true, Ordering::SeqCst);
            Ok(TimedChannel {
                reply_delay: Duration::ZERO,
                dropped: Arc::new(AtomicBool::new(false)),
                requested: Arc::new(AtomicBool::new(false)),
            })
        },
        "instance",
        7,
        12,
        Instant::now(),
    )
    .await;
    assert_eq!(result, Err(EpochProvisionError::DeadlineExceeded));
    assert!(!polled.load(Ordering::SeqCst));
}

#[tokio::test(start_paused = true)]
async fn connector_ready_at_deadline_cannot_emit_a_request() {
    use std::task::Poll;

    let requested = Arc::new(AtomicBool::new(false));
    let dropped = Arc::new(AtomicBool::new(false));
    let (tx, rx) = tokio::sync::oneshot::channel();
    let operation = provision_with_deadline(
        async { rx.await.unwrap() },
        "instance",
        7,
        12,
        Instant::now() + Duration::from_secs(10),
    );
    tokio::pin!(operation);
    std::future::poll_fn(|cx| {
        assert!(operation.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    tokio::time::advance(Duration::from_secs(10)).await;
    assert!(
        tx.send(Ok(TimedChannel {
            reply_delay: Duration::ZERO,
            dropped: dropped.clone(),
            requested: requested.clone(),
        }))
        .is_ok()
    );
    assert_eq!(operation.await, Err(EpochProvisionError::DeadlineExceeded));
    assert!(!requested.load(Ordering::SeqCst));
    assert!(dropped.load(Ordering::SeqCst));
}

#[tokio::test(start_paused = true)]
async fn connection_errors_remain_typed() {
    let expected = EpochProvisionError::SendFailed {
        detail: "relay closed".to_string(),
    };
    let result = provision_with_deadline(
        std::future::ready(Err::<TimedChannel, _>(expected.clone())),
        "instance",
        7,
        12,
        Instant::now() + Duration::from_secs(10),
    )
    .await;
    assert_eq!(result, Err(expected));
}

#[cfg(unix)]
mod sockets {
    use super::*;
    use microsandbox_protocol::{
        codec::{self, RawFrame},
        core::Ready,
        message::{Message, MessageType},
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{UnixListener, UnixStream};
    use tokio::task::JoinHandle;

    struct Attempt {
        _dir: tempfile::TempDir,
        peer: UnixStream,
        task: JoinHandle<Result<u64, EpochProvisionError>>,
    }

    impl Drop for Attempt {
        fn drop(&mut self) {
            // Also bound cleanup when a fixture assertion fails.
            self.task.abort();
        }
    }

    async fn bounded<F: Future>(future: F) -> F::Output {
        timeout(Duration::from_secs(5), future)
            .await
            .expect("socket test watchdog")
    }

    async fn attempt() -> Attempt {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("relay.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let task = tokio::spawn(async move {
            provision_with_deadline(
                async move {
                    // Longer than the whole-operation budget: the tests must
                    // exercise Workestrate's deadline, not the SDK's own one.
                    let client = microsandbox::agent::AgentClient::connect_with_timeout(
                        path,
                        Duration::from_secs(60),
                    )
                    .await
                    .map_err(|e| EpochProvisionError::SendFailed {
                        detail: e.to_string(),
                    })?;
                    Ok(AgentConsoleChannel { client })
                },
                "fixture",
                7,
                12,
                Instant::now() + Duration::from_secs(30),
            )
            .await
        });
        let (peer, _) = bounded(listener.accept()).await.unwrap();
        Attempt {
            _dir: dir,
            peer,
            task,
        }
    }

    async fn handshake(peer: &mut UnixStream, generation: u8) {
        let mut ready = Message::with_payload(MessageType::Ready, 0, &Ready::default()).unwrap();
        ready.v = generation;
        bounded(async {
            peer.write_all(&1u32.to_be_bytes()).await.unwrap();
            peer.write_all(&microsandbox_protocol::AGENT_RELAY_ID_RANGE_STEP.to_be_bytes())
                .await
                .unwrap();
            codec::write_message(peer, &ready).await.unwrap();
        })
        .await;
    }

    async fn assert_closed(peer: &mut UnixStream) {
        let mut byte = [0];
        assert_eq!(
            bounded(peer.read(&mut byte)).await.unwrap(),
            0,
            "transport must close"
        );
    }

    #[tokio::test]
    async fn deadline_closes_real_sdk_socket_during_handshake_and_reply() {
        for complete_handshake in [false, true] {
            let mut attempt = attempt().await;
            if complete_handshake {
                handshake(&mut attempt.peer, EPOCH_INTRO_GENERATION).await;
                let request = bounded(codec::read_raw_frame(&mut attempt.peer))
                    .await
                    .unwrap();
                assert_ne!(request.id, 0);
                assert_eq!(request.flags, 0);
                // Decode the actual SDK request before deliberately withholding
                // its ack. This is not merely an accepted-socket test.
                let _ = echo_body(&request.body);
            }
            // Pause only after real I/O reaches the chosen stall. Starting
            // paused could let virtual time outrun readiness on a real socket.
            tokio::time::pause();
            tokio::time::advance(Duration::from_secs(31)).await;
            let result = bounded(&mut attempt.task).await.unwrap();
            tokio::time::resume();
            assert_eq!(result, Err(EpochProvisionError::DeadlineExceeded));
            assert_closed(&mut attempt.peer).await;
        }
    }

    #[tokio::test]
    async fn caller_cancellation_closes_real_sdk_socket_during_handshake_and_reply() {
        for complete_handshake in [false, true] {
            let mut attempt = attempt().await;
            if complete_handshake {
                handshake(&mut attempt.peer, EPOCH_INTRO_GENERATION).await;
                bounded(codec::read_raw_frame(&mut attempt.peer))
                    .await
                    .unwrap();
            }
            attempt.task.abort();
            assert!(bounded(&mut attempt.task).await.unwrap_err().is_cancelled());
            assert_closed(&mut attempt.peer).await;
        }
    }

    #[tokio::test]
    async fn successful_real_ack_drops_the_one_shot_connection() {
        let mut attempt = attempt().await;
        handshake(&mut attempt.peer, EPOCH_INTRO_GENERATION).await;
        let request = bounded(codec::read_raw_frame(&mut attempt.peer))
            .await
            .unwrap();
        bounded(codec::write_raw_frame(
            &mut attempt.peer,
            &RawFrame {
                id: request.id,
                flags: 0,
                body: echo_body(&request.body),
            },
        ))
        .await
        .unwrap();
        assert_eq!(bounded(&mut attempt.task).await.unwrap(), Ok(12));
        assert_closed(&mut attempt.peer).await;
    }

    #[tokio::test]
    async fn generation_gate_closes_real_connection_without_a_request() {
        let mut attempt = attempt().await;
        handshake(&mut attempt.peer, EPOCH_INTRO_GENERATION - 1).await;
        assert_eq!(
            bounded(&mut attempt.task).await.unwrap(),
            Err(EpochProvisionError::GenerationGate {
                peer_generation: EPOCH_INTRO_GENERATION - 1,
                required: EPOCH_INTRO_GENERATION,
            })
        );
        assert_closed(&mut attempt.peer).await;
    }
}
