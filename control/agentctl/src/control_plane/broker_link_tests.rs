//! Synthetic duplex transport controls, not broker/VM authentication evidence.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

use super::*;
use crate::control_plane::authorization::AuthenticatedCaller;
use crate::control_plane::types::{InstanceRef, SshRequest, WorkloadRef};
use crate::microsandbox::plan::{CredentialsPlan, SshGrantPlan};

fn target() -> LaunchRef {
    LaunchRef {
        instance: InstanceRef {
            workload: WorkloadRef {
                context: Some("fixture".into()),
                name: "worker".into(),
            },
            instance: "fixture-worker".into(),
        },
        generation: OpaqueId::from_bytes([3; 32]),
    }
}

fn credentials() -> CredentialsPlan {
    CredentialsPlan {
        ssh: vec![SshGrantPlan {
            name: "git".into(),
            material: "GIT_KEY".into(),
            hosts: vec!["git.example.test".into()],
            users: vec!["git".into()],
            ports: vec![22],
            binding: CredentialBinding::Broker,
            on_violation: SecretViolationPolicy::Block,
        }],
        signing: Vec::new(),
        strict: true,
        strict_origin: None,
    }
}

fn policy(seed: u8) -> wire::Policy {
    wire::Policy {
        destroyed: false,
        credentials: vec![wire::ReadyRecord {
            name: "git".into(),
            material: "GIT_KEY".into(),
            binding: wire::Binding::Broker,
            key_version: wire::Id::from_bytes([4; 32]).unwrap(),
            trust_version: wire::Id::from_bytes([5; 32]).unwrap(),
            host: "git.example.test".into(),
            port: 22,
            user: "git".into(),
            on_violation: wire::Violation::Block,
            key_kind: wire::KeyKind::Ed25519Seed,
            key_bytes: wire::SecretBytes::new(vec![seed; 32]),
            // The duplex peer tests typed framing, not cryptographic parsing.
            upstream_public_key: "synthetic-upstream-pin".into(),
        }],
        patterns: vec![],
    }
}

fn owner_ok() -> Result<(), ControlError> {
    Ok(())
}

async fn connected<G: Fn() -> Result<(), ControlError>>(
    guard: G,
) -> (
    SshController,
    BrokerLink<DuplexStream, G>,
    DuplexStream,
    SshPolicyTransaction,
) {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    controller
        .register_launch(target(), &credentials(), None)
        .unwrap();
    controller.confirm_launch(&target()).unwrap();
    let (stream, mut peer) = tokio::io::duplex(1024);
    let handshake = async {
        let wire::Request::Hello(hello) = wire::read_request(&mut peer).await.unwrap() else {
            panic!("expected hello")
        };
        assert_eq!(hello.controller, wire::Id::from_bytes([1; 32]).unwrap());
        wire::write_reply(
            &mut peer,
            &wire::Reply::Hello(wire::Welcome {
                version: wire::VERSION,
                session: wire::BrokerSession {
                    controller: hello.controller,
                    broker: wire::Id::from_bytes([2; 32]).unwrap(),
                    connection: 1,
                },
            }),
        )
        .await
        .unwrap();
    };
    let (result, ()) = tokio::join!(
        BrokerLink::connect_verified(stream, guard, &mut controller),
        handshake
    );
    let (link, mut intents) = result.unwrap();
    assert_eq!(intents.len(), 1);
    (controller, link, peer, intents.remove(0))
}

fn inspect(controller: &mut SshController) -> SshStatus {
    controller
        .request(
            &AuthenticatedCaller::local_operator(1000),
            SshRequest::Inspect { launch: target() },
        )
        .unwrap()
        .status
}

async fn observe(peer: &mut DuplexStream, outcome: wire::Outcome) -> wire::TransactionRef {
    let target = match wire::read_request(peer).await.unwrap() {
        wire::Request::Apply(apply) => {
            assert_eq!(
                wire::policy_digest(&apply.policy).unwrap(),
                apply.transaction.policy_digest
            );
            apply.transaction
        }
        wire::Request::Finish(target) => target,
        _ => panic!("expected policy operation"),
    };
    wire::write_reply(
        peer,
        &wire::Reply::Observation(wire::Observation {
            transaction: target.clone(),
            outcome,
            failure: None,
        }),
    )
    .await
    .unwrap();
    target
}

#[tokio::test]
async fn broker_link_prepares_exact_wire_digest_and_waits_for_finish() {
    let (mut controller, mut link, mut peer, intent) = connected(owner_ok).await;
    let expected = wire::policy_digest(&policy(7)).unwrap();
    let (result, sent) = tokio::join!(
        link.apply(&mut controller, &intent, policy(7), None),
        observe(&mut peer, wire::Outcome::Pending),
    );
    let (prepared, status) = result.unwrap();
    assert_eq!(sent, transaction(&prepared).unwrap());
    assert_eq!(prepared.policy_digest.as_str(), expected.to_hex());
    assert!(!status.ready);
    let (result, finished) = tokio::join!(
        link.finish(&mut controller, &prepared),
        observe(&mut peer, wire::Outcome::Applied),
    );
    assert_eq!(finished, sent);
    assert!(result.unwrap().ready);
    assert!(inspect(&mut controller).ready);
}

#[tokio::test]
async fn broker_link_material_rotation_uses_new_durable_revision() {
    let (mut controller, mut link, mut peer, intent) = connected(owner_ok).await;
    let (first, old) = tokio::join!(
        link.apply(&mut controller, &intent, policy(7), None),
        observe(&mut peer, wire::Outcome::Applied),
    );
    let (prepared, _) = first.unwrap();
    let (second, new) = tokio::join!(
        link.apply(
            &mut controller,
            &prepared.intent,
            policy(8),
            Some(old.revision)
        ),
        observe(&mut peer, wire::Outcome::Applied),
    );
    assert!(second.unwrap().1.ready);
    assert_eq!(new.revision, old.revision + 1);
    assert_ne!(new.policy_digest, old.policy_digest);
    assert_eq!(new.launch, old.launch);
}

#[tokio::test(start_paused = true)]
async fn broker_link_probe_is_not_applied_and_uses_conservative_deadlines() {
    let (mut controller, mut link, mut peer, _) = connected(owner_ok).await;
    let due = link.next_probe_at();
    tokio::time::advance(Duration::from_secs(wire::MANAGEMENT_PROBE_SECS)).await;
    assert!(Instant::now() >= due);
    let started = Instant::now();
    let (result, ()) = tokio::join!(link.probe(&mut controller), async {
        let wire::Request::Probe(session) = wire::read_request(&mut peer).await.unwrap() else {
            panic!("expected probe")
        };
        tokio::time::sleep(Duration::from_secs(2)).await;
        wire::write_reply(
            &mut peer,
            &wire::Reply::Probe(wire::Probe {
                session,
                current: true,
            }),
        )
        .await
        .unwrap();
    });
    result.unwrap();
    assert_eq!(
        link.next_probe_at(),
        started + Duration::from_secs(wire::MANAGEMENT_PROBE_SECS)
    );
    assert_eq!(
        link.lease_deadline,
        started + Duration::from_secs(wire::MANAGEMENT_LEASE_SECS)
    );
    assert!(!inspect(&mut controller).ready);
}

#[tokio::test(start_paused = true)]
async fn broker_link_missed_absolute_lease_closes_before_probe_bytes() {
    let (mut controller, mut link, mut peer, _) = connected(owner_ok).await;
    let session = link.session().clone();
    tokio::time::advance(Duration::from_secs(wire::MANAGEMENT_LEASE_SECS)).await;
    assert_eq!(
        link.probe(&mut controller).await,
        Err(ControlError::BrokerUnavailable)
    );
    assert_eq!(peer.read(&mut [0; 1]).await.unwrap(), 0);
    assert_eq!(
        controller.validate_broker_session(&session),
        Err(ControlError::StaleObservation)
    );
}

#[tokio::test(start_paused = true)]
async fn broker_link_partial_reply_does_not_extend_exchange_budget() {
    let (mut controller, mut link, mut peer, _) = connected(owner_ok).await;
    let (result, ()) = tokio::join!(link.probe(&mut controller), async {
        assert!(matches!(
            wire::read_request(&mut peer).await.unwrap(),
            wire::Request::Probe(_)
        ));
        peer.write_all(&[0, 0]).await.unwrap();
        tokio::time::sleep(EXCHANGE_BUDGET / 2).await;
        peer.write_all(&[0]).await.unwrap();
    });
    assert_eq!(result, Err(ControlError::BrokerUnavailable));
    assert!(link.stream.is_none());
    assert_eq!(peer.read(&mut [0; 1]).await.unwrap(), 0);
}

#[tokio::test]
async fn broker_link_cancelled_apply_closes_and_invalidates_session() {
    let (mut controller, mut link, mut peer, intent) = connected(owner_ok).await;
    let session = link.session().clone();
    {
        let operation = link.apply(&mut controller, &intent, policy(7), None);
        tokio::pin!(operation);
        tokio::select! {
            result = &mut operation => panic!("unexpected completion: {}", result.is_ok()),
            request = wire::read_request(&mut peer) => assert!(matches!(request.unwrap(), wire::Request::Apply(_))),
        }
        // Dropping the polled future must retire transport ownership even
        // though the broker may already have received the whole policy.
    }
    assert!(link.stream.is_none());
    assert_eq!(
        controller.validate_broker_session(&session),
        Err(ControlError::StaleObservation)
    );
    assert_eq!(peer.read(&mut [0; 1]).await.unwrap(), 0);
    assert!(!inspect(&mut controller).ready);
    assert_eq!(
        link.probe(&mut controller).await,
        Err(ControlError::StaleObservation)
    );
}

#[tokio::test]
async fn broker_link_mismatched_reply_cannot_establish_readiness() {
    let (mut controller, mut link, mut peer, intent) = connected(owner_ok).await;
    let (result, ()) = tokio::join!(
        link.apply(&mut controller, &intent, policy(7), None),
        async {
            let wire::Request::Apply(mut apply) = wire::read_request(&mut peer).await.unwrap()
            else {
                panic!("expected apply")
            };
            apply.transaction.launch.generation = wire::Id::from_bytes([99; 32]).unwrap();
            wire::write_reply(
                &mut peer,
                &wire::Reply::Observation(wire::Observation {
                    transaction: apply.transaction,
                    outcome: wire::Outcome::Applied,
                    failure: None,
                }),
            )
            .await
            .unwrap();
        }
    );
    assert!(matches!(result, Err(ControlError::StaleObservation)));
    assert!(link.stream.is_none());
    assert!(!inspect(&mut controller).ready);
}

#[tokio::test]
async fn broker_link_state_lost_requires_new_welcome_and_reapply() {
    let (mut controller, mut link, mut peer, intent) = connected(owner_ok).await;
    let session = link.session().clone();
    let (result, _) = tokio::join!(
        link.apply(&mut controller, &intent, policy(7), None),
        observe(&mut peer, wire::Outcome::StateLost),
    );
    let (prepared, status) = result.unwrap();
    assert_eq!(status.observed.unwrap().outcome, AppliedOutcome::StateLost);
    assert!(!status.ready);
    assert!(link.stream.is_none());
    assert_eq!(
        controller.validate_broker_session(&session),
        Err(ControlError::StaleObservation)
    );
    assert!(matches!(
        link.finish(&mut controller, &prepared).await,
        Err(ControlError::StaleObservation)
    ));
    assert_eq!(peer.read(&mut [0; 1]).await.unwrap(), 0);
}

#[tokio::test]
async fn broker_link_owner_loss_before_or_after_exchange_refuses() {
    for after_write in [false, true] {
        let live = Arc::new(AtomicBool::new(true));
        let checked = live.clone();
        let (mut controller, mut link, mut peer, intent) = connected(move || {
            if checked.load(Ordering::SeqCst) {
                Ok(())
            } else {
                Err(ControlError::StateUnavailable)
            }
        })
        .await;
        if after_write {
            let (result, ()) = tokio::join!(
                link.apply(&mut controller, &intent, policy(7), None),
                async {
                    let wire::Request::Apply(apply) = wire::read_request(&mut peer).await.unwrap()
                    else {
                        panic!("expected apply")
                    };
                    live.store(false, Ordering::SeqCst);
                    wire::write_reply(
                        &mut peer,
                        &wire::Reply::Observation(wire::Observation {
                            transaction: apply.transaction,
                            outcome: wire::Outcome::Applied,
                            failure: None,
                        }),
                    )
                    .await
                    .unwrap();
                }
            );
            assert!(matches!(result, Err(ControlError::StateUnavailable)));
        } else {
            live.store(false, Ordering::SeqCst);
            assert!(matches!(
                link.apply(&mut controller, &intent, policy(7), None).await,
                Err(ControlError::StateUnavailable)
            ));
        }
        assert!(link.stream.is_none());
        assert_eq!(peer.read(&mut [0; 1]).await.unwrap(), 0);
        assert!(!inspect(&mut controller).ready);
    }
}

#[tokio::test]
async fn broker_link_old_link_cleanup_preserves_newer_session() {
    let (mut controller, mut link, mut peer, _) = connected(owner_ok).await;
    let mut newer = link.session().clone();
    newer.connection += 1;
    controller
        .broker_session_authenticated(newer.clone())
        .unwrap();
    assert_eq!(
        link.probe(&mut controller).await,
        Err(ControlError::StaleObservation)
    );
    assert!(controller.validate_broker_session(&newer).is_ok());
    assert_eq!(peer.read(&mut [0; 1]).await.unwrap(), 0);
}

#[tokio::test]
async fn broker_link_projection_preserves_whole_tuple_and_all_combinations() {
    let (_, _, _, intent) = connected(owner_ok).await;
    assert!(validate_projection(&intent, &policy(7)).is_ok());
    for field in 0..8 {
        let mut changed = policy(7);
        let record = &mut changed.credentials[0];
        match field {
            0 => record.name = "other".into(),
            1 => record.material = "OTHER_KEY".into(),
            2 => record.host = "127.0.0.1".into(),
            3 => record.user = "root".into(),
            4 => record.port = 2222,
            5 => record.binding = wire::Binding::Guest,
            6 => record.on_violation = wire::Violation::Passthrough,
            _ => changed.destroyed = true,
        }
        assert_eq!(
            validate_projection(&intent, &changed),
            Err(ControlError::InvalidPolicy)
        );
    }
    let mut expanded = intent.clone();
    expanded.credentials[0]
        .hosts
        .push("other.example.test".into());
    assert_eq!(
        validate_projection(&expanded, &policy(7)),
        Err(ControlError::InvalidPolicy)
    );
    let mut complete = policy(7);
    let mut extra = policy(7).credentials.remove(0);
    extra.host = "other.example.test".into();
    complete.credentials.push(extra);
    assert!(validate_projection(&expanded, &complete).is_ok());
    expanded.credentials[0].hosts[1] = "*.example.test".into();
    assert_eq!(
        validate_projection(&expanded, &complete),
        Err(ControlError::InvalidPolicy)
    );
}

#[tokio::test]
async fn broker_link_invalid_resolved_policy_sends_no_apply() {
    let (mut controller, mut link, mut peer, intent) = connected(owner_ok).await;
    let mut invalid = policy(7);
    invalid.credentials[0].user = "different-user".into();
    assert!(matches!(
        link.apply(&mut controller, &intent, invalid, None).await,
        Err(ControlError::InvalidPolicy)
    ));
    assert_eq!(peer.read(&mut [0; 1]).await.unwrap(), 0);
    assert!(!inspect(&mut controller).ready);
}

#[tokio::test(start_paused = true)]
async fn broker_link_policy_traffic_does_not_postpone_probe_or_expiry() {
    let (mut controller, mut link, mut peer, intent) = connected(owner_ok).await;
    let due = link.next_probe_at();
    let deadline = link.lease_deadline;
    let (result, _) = tokio::join!(
        link.apply(&mut controller, &intent, policy(7), None),
        observe(&mut peer, wire::Outcome::Pending),
    );
    let (prepared, _) = result.unwrap();
    tokio::time::advance(Duration::from_secs(11)).await;
    let (result, _) = tokio::join!(
        link.apply(
            &mut controller,
            &prepared.intent,
            policy(7),
            Some(prepared.intent.revision)
        ),
        observe(&mut peer, wire::Outcome::Pending),
    );
    result.unwrap();
    assert!(Instant::now() >= due);
    assert_eq!(link.next_probe_at(), due);
    assert_eq!(link.lease_deadline, deadline);
    tokio::time::advance(Duration::from_secs(11)).await;
    let (result, _) = tokio::join!(
        link.finish(&mut controller, &prepared),
        observe(&mut peer, wire::Outcome::Pending),
    );
    result.unwrap();
    assert_eq!(link.next_probe_at(), due);
    assert_eq!(link.lease_deadline, deadline);
    tokio::time::advance(Duration::from_secs(9)).await;
    assert!(matches!(
        link.finish(&mut controller, &prepared).await,
        Err(ControlError::BrokerUnavailable)
    ));
    assert_eq!(peer.read(&mut [0; 1]).await.unwrap(), 0);
    assert!(!inspect(&mut controller).ready);
}

#[tokio::test]
async fn broker_link_cancelled_replacement_hello_invalidates_old_readiness() {
    let (mut controller, mut old, mut old_peer, intent) = connected(owner_ok).await;
    let (result, _) = tokio::join!(
        old.apply(&mut controller, &intent, policy(7), None),
        observe(&mut old_peer, wire::Outcome::Applied),
    );
    let (_, old_status) = result.unwrap();
    assert!(old_status.ready);
    let old_receipt = old_status.observed.unwrap();
    let (stream, mut peer) = tokio::io::duplex(1024);
    {
        let replacement = BrokerLink::connect_verified(stream, owner_ok, &mut controller);
        tokio::pin!(replacement);
        tokio::select! {
            result = &mut replacement => panic!("unexpected replacement completion: {}", result.is_ok()),
            request = wire::read_request(&mut peer) => assert!(matches!(request.unwrap(), wire::Request::Hello(_))),
        }
    }
    assert_eq!(peer.read(&mut [0; 1]).await.unwrap(), 0);
    assert!(!inspect(&mut controller).ready);
    assert!(matches!(
        controller.observe(old_receipt),
        Err(ControlError::StaleObservation)
    ));
    old.close(&mut controller);
}

#[tokio::test]
async fn broker_link_invalid_or_absent_replacement_welcome_invalidates_old_readiness() {
    for malformed in [false, true] {
        let (mut controller, mut old, mut old_peer, intent) = connected(owner_ok).await;
        let (result, _) = tokio::join!(
            old.apply(&mut controller, &intent, policy(7), None),
            observe(&mut old_peer, wire::Outcome::Applied),
        );
        let (_, old_status) = result.unwrap();
        assert!(old_status.ready);
        let old_receipt = old_status.observed.unwrap();
        let (stream, mut peer) = tokio::io::duplex(1024);
        let (result, ()) = tokio::join!(
            BrokerLink::connect_verified(stream, owner_ok, &mut controller),
            async {
                assert!(matches!(
                    wire::read_request(&mut peer).await.unwrap(),
                    wire::Request::Hello(_)
                ));
                if malformed {
                    peer.write_all(&[0, 0, 0, 1, 0xff]).await.unwrap();
                } else {
                    peer.shutdown().await.unwrap();
                }
            },
        );
        assert!(matches!(result, Err(ControlError::BrokerUnavailable)));
        assert_eq!(peer.read(&mut [0; 1]).await.unwrap(), 0);
        assert!(!inspect(&mut controller).ready);
        assert!(matches!(
            controller.observe(old_receipt),
            Err(ControlError::StaleObservation)
        ));
        old.close(&mut controller);
    }
}
