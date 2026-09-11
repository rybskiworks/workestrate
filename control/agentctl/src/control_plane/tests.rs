//! Independent permission and lifecycle oracles; all inputs are synthetic.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::collections::BTreeSet;

use proptest::prelude::*;

use crate::config::SecretViolationPolicy;
use crate::microsandbox::plan::{CredentialBinding, CredentialsPlan, SshGrantPlan};

use super::authorization::{AuthenticatedCaller, CallerIdentity, Permission, ResourceScope};
use super::custody::{SshController, SshPolicyTransaction};
use super::types::*;

fn launch(name: &str, generation: u8) -> LaunchRef {
    LaunchRef {
        instance: InstanceRef {
            workload: WorkloadRef {
                context: Some("personal".into()),
                name: name.into(),
            },
            instance: format!("personal-{name}"),
        },
        generation: OpaqueId::from_bytes([generation; 32]),
    }
}

fn credentials() -> CredentialsPlan {
    CredentialsPlan {
        ssh: vec![SshGrantPlan {
            name: "git".into(),
            material: "GIT_KEY".into(),
            hosts: vec!["git.example.test".into()],
            ports: vec![22],
            users: vec!["git".into()],
            binding: CredentialBinding::Broker,
            on_violation: SecretViolationPolicy::Block,
        }],
        signing: Vec::new(),
        strict: true,
        strict_origin: Some("home".into()),
    }
}

fn applied(transaction: &SshPolicyTransaction) -> BrokerObservation {
    BrokerObservation {
        session: transaction.session.clone(),
        launch: transaction.launch.clone(),
        revision: transaction.revision,
        policy_digest: OpaqueId::from_bytes([42; 32]),
        outcome: AppliedOutcome::Applied,
        failure: None,
    }
}

fn prepare(controller: &mut SshController, transaction: &SshPolicyTransaction) {
    // The wire/material adapter is outside this controller model. A stable
    // synthetic digest represents its resolved policy; these tests never hash
    // credential names as a substitute for the production wire digest.
    let prepared = controller
        .prepare_policy(transaction, OpaqueId::from_bytes([42; 32]))
        .unwrap();
    assert_eq!(prepared.intent, *transaction);
}

#[test]
fn broker_heartbeat_requires_current_session_without_launches() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    let session = BrokerSession {
        controller: OpaqueId::from_bytes([1; 32]),
        broker: OpaqueId::from_bytes([2; 32]),
        connection: 1,
    };
    assert_eq!(
        controller.validate_broker_session(&session),
        Err(ControlError::StaleObservation)
    );
    assert!(
        controller
            .broker_session_authenticated(session.clone())
            .unwrap()
            .is_empty()
    );
    assert_eq!(controller.validate_broker_session(&session), Ok(()));
    let newer = BrokerSession {
        connection: 2,
        ..session.clone()
    };
    controller
        .broker_session_authenticated(newer.clone())
        .unwrap();
    assert_eq!(
        controller.validate_broker_session(&session),
        Err(ControlError::StaleObservation)
    );
    assert_eq!(controller.validate_broker_session(&newer), Ok(()));
    controller.broker_disconnected(&newer).unwrap();
    assert_eq!(
        controller.validate_broker_session(&newer),
        Err(ControlError::StaleObservation)
    );
}

#[test]
fn unprepared_intent_cannot_be_acknowledged_even_after_reconnection() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    let target = launch("worker", 1);
    controller
        .register_launch(target.clone(), &credentials(), None)
        .unwrap();
    controller.confirm_launch(&target).unwrap();
    let intent = controller
        .broker_connected(OpaqueId::from_bytes([2; 32]))
        .unwrap()
        .remove(0);
    assert_eq!(
        controller.observe(applied(&intent)),
        Err(ControlError::StaleObservation)
    );
    prepare(&mut controller, &intent);
    assert!(controller.observe(applied(&intent)).unwrap().ready);
    let next = controller
        .broker_connected(OpaqueId::from_bytes([2; 32]))
        .unwrap()
        .remove(0);
    assert_eq!(
        controller.observe(applied(&next)),
        Err(ControlError::StaleObservation)
    );
    prepare(&mut controller, &next);
    assert_eq!(next.revision, intent.revision);
    assert!(controller.observe(applied(&next)).unwrap().ready);
}

#[test]
fn material_rotation_advances_revision_and_fences_unchanged_names() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    let target = launch("worker", 1);
    controller
        .register_launch(target.clone(), &credentials(), None)
        .unwrap();
    controller.confirm_launch(&target).unwrap();
    let intent = controller
        .broker_connected(OpaqueId::from_bytes([2; 32]))
        .unwrap()
        .remove(0);
    prepare(&mut controller, &intent);
    assert!(controller.observe(applied(&intent)).unwrap().ready);
    let prepared = controller
        .prepare_policy(&intent, OpaqueId::from_bytes([43; 32]))
        .unwrap();
    assert_eq!(prepared.intent.revision, intent.revision + 1);
    assert_eq!(prepared.intent.credentials, intent.credentials);
    assert!(!inspect(&mut controller, &target).ready);
    assert_eq!(
        controller.observe(applied(&intent)),
        Err(ControlError::StaleObservation)
    );
    assert_eq!(
        controller.prepare_policy(&intent, OpaqueId::from_bytes([42; 32])),
        Err(ControlError::RevisionConflict)
    );
    let mut report = applied(&prepared.intent);
    assert_eq!(
        controller.observe(report.clone()),
        Err(ControlError::StaleObservation)
    );
    report.policy_digest = prepared.policy_digest.clone();
    assert!(controller.observe(report).unwrap().ready);
    // Resolving the same bytes again does not invent a new revision or erase
    // the current applied receipt.
    let again = controller
        .prepare_policy(&prepared.intent, prepared.policy_digest.clone())
        .unwrap();
    assert_eq!(again, prepared);
    assert!(inspect(&mut controller, &target).ready);
}

#[test]
fn preparation_rejects_stale_or_modified_intent_without_changing_state() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    let target = launch("worker", 1);
    controller
        .register_launch(target.clone(), &credentials(), None)
        .unwrap();
    let intent = controller
        .broker_connected(OpaqueId::from_bytes([2; 32]))
        .unwrap()
        .remove(0);
    let before = inspect(&mut controller, &target);
    for field in 0..5 {
        let mut modified = intent.clone();
        match field {
            0 => modified.revision += 1,
            1 => modified.credentials[0].material = "FOREIGN_KEY".into(),
            2 => modified.destroyed = true,
            3 => modified.session.connection += 1,
            _ => modified.launch.generation = OpaqueId::from_bytes([99; 32]),
        }
        assert!(
            controller
                .prepare_policy(&modified, OpaqueId::from_bytes([42; 32]))
                .is_err()
        );
        assert_eq!(inspect(&mut controller, &target), before);
    }
    assert_eq!(
        controller.prepare_policy(&intent, OpaqueId::from_bytes([0; 32])),
        Err(ControlError::InvalidPolicy)
    );
    assert_eq!(inspect(&mut controller, &target), before);
}

#[test]
fn prepared_handles_are_revalidated_before_transport_writes_or_retries() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    let target = launch("worker", 1);
    controller
        .register_launch(target.clone(), &credentials(), None)
        .unwrap();
    let intent = controller
        .broker_connected(OpaqueId::from_bytes([2; 32]))
        .unwrap()
        .remove(0);
    let first = controller
        .prepare_policy(&intent, OpaqueId::from_bytes([42; 32]))
        .unwrap();
    assert_eq!(controller.validate_prepared(&first), Ok(()));
    let rotated = controller
        .prepare_policy(&intent, OpaqueId::from_bytes([43; 32]))
        .unwrap();
    assert_eq!(
        controller.validate_prepared(&first),
        Err(ControlError::RevisionConflict)
    );
    assert_eq!(controller.validate_prepared(&rotated), Ok(()));
    controller
        .broker_connected(OpaqueId::from_bytes([2; 32]))
        .unwrap();
    assert_eq!(
        controller.validate_prepared(&rotated),
        Err(ControlError::StaleObservation)
    );
    let next = controller
        .request(
            &AuthenticatedCaller::local_operator(1000),
            SshRequest::Reconcile {
                launch: target.clone(),
                expected_revision: rotated.intent.revision,
                credentials: BTreeSet::from(["git".into()]),
            },
        )
        .unwrap()
        .transaction
        .unwrap();
    let selected = controller
        .prepare_policy(&next, OpaqueId::from_bytes([43; 32]))
        .unwrap();
    assert_eq!(controller.validate_prepared(&selected), Ok(()));
    controller.retire_launch(&target).unwrap();
    assert!(controller.validate_prepared(&selected).is_err());
}

#[test]
fn changing_selection_requires_preparation_even_when_effective_digest_repeats() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    let target = launch("worker", 1);
    controller
        .register_launch(target.clone(), &credentials(), None)
        .unwrap();
    controller.confirm_launch(&target).unwrap();
    let intent = controller
        .broker_connected(OpaqueId::from_bytes([2; 32]))
        .unwrap()
        .remove(0);
    prepare(&mut controller, &intent);
    controller.observe(applied(&intent)).unwrap();
    let next = controller
        .request(
            &AuthenticatedCaller::local_operator(1000),
            SshRequest::Reconcile {
                launch: target.clone(),
                expected_revision: 1,
                credentials: BTreeSet::from(["git".into()]),
            },
        )
        .unwrap()
        .transaction
        .unwrap();
    assert_eq!(next.revision, 2);
    assert_eq!(
        controller.observe(applied(&next)),
        Err(ControlError::StaleObservation)
    );
    prepare(&mut controller, &next);
    assert!(controller.observe(applied(&next)).unwrap().ready);
}

fn inspect(controller: &mut SshController, target: &LaunchRef) -> SshStatus {
    let change = controller
        .request(
            &AuthenticatedCaller::local_operator(1000),
            SshRequest::Inspect {
                launch: target.clone(),
            },
        )
        .unwrap();
    assert!(
        change.transaction.is_none(),
        "inspection must not issue broker mutations"
    );
    change.status
}

#[test]
fn readiness_requires_runtime_binding_and_applied_not_accepted_state() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    let target = launch("prime", 1);
    let pending = controller
        .register_launch(target.clone(), &credentials(), None)
        .unwrap();
    assert!(!pending.status.ready);
    assert!(pending.transaction.is_none());
    let transaction = controller
        .broker_connected(OpaqueId::from_bytes([2; 32]))
        .unwrap()
        .remove(0);
    prepare(&mut controller, &transaction);
    let mut report = applied(&transaction);
    report.outcome = AppliedOutcome::Pending;
    assert!(!controller.observe(report).unwrap().ready);
    assert!(!controller.confirm_launch(&target).unwrap().ready);
    assert!(controller.observe(applied(&transaction)).unwrap().ready);
    let before = inspect(&mut controller, &target);
    let mut old = applied(&transaction);
    old.outcome = AppliedOutcome::Pending;
    assert_eq!(controller.observe(old), Err(ControlError::StaleObservation));
    assert_eq!(inspect(&mut controller, &target), before);
}

#[test]
fn replacement_requires_retirement_and_old_cleanup_cannot_revoke_new_launch() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    controller
        .broker_connected(OpaqueId::from_bytes([2; 32]))
        .unwrap();
    let old = launch("prime", 1);
    let replacement = launch("prime", 2);
    let first = controller
        .register_launch(old.clone(), &credentials(), None)
        .unwrap()
        .transaction
        .unwrap();
    controller.confirm_launch(&old).unwrap();
    prepare(&mut controller, &first);
    controller.observe(applied(&first)).unwrap();
    assert_eq!(
        controller.register_launch(replacement.clone(), &credentials(), Some(&old)),
        Err(ControlError::RevisionConflict)
    );
    let retired = controller.retire_launch(&old).unwrap().transaction.unwrap();
    assert!(!inspect(&mut controller, &old).ready);
    assert_eq!(
        controller.register_launch(replacement.clone(), &credentials(), Some(&old)),
        Err(ControlError::RevisionConflict)
    );
    prepare(&mut controller, &retired);
    controller.observe(applied(&retired)).unwrap();
    assert_eq!(
        controller.register_launch(old.clone(), &credentials(), Some(&old)),
        Err(ControlError::StaleLaunch)
    );
    let next = controller
        .register_launch(replacement.clone(), &credentials(), Some(&old))
        .unwrap()
        .transaction
        .unwrap();
    controller.confirm_launch(&replacement).unwrap();
    prepare(&mut controller, &next);
    assert!(controller.observe(applied(&next)).unwrap().ready);
    assert_eq!(
        controller.retire_launch(&old),
        Err(ControlError::StaleLaunch)
    );
    assert_eq!(
        controller.observe(applied(&first)),
        Err(ControlError::StaleObservation)
    );
    assert!(inspect(&mut controller, &replacement).ready);
}

#[test]
fn reconnect_requires_reassertion_even_when_broker_process_did_not_change() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    let target = launch("codex", 1);
    controller
        .register_launch(target.clone(), &credentials(), None)
        .unwrap();
    controller.confirm_launch(&target).unwrap();
    let old = controller
        .broker_connected(OpaqueId::from_bytes([2; 32]))
        .unwrap()
        .remove(0);
    prepare(&mut controller, &old);
    controller.observe(applied(&old)).unwrap();
    controller.broker_disconnected(&old.session).unwrap();
    assert!(!inspect(&mut controller, &target).ready);
    let new = controller
        .broker_connected(OpaqueId::from_bytes([2; 32]))
        .unwrap()
        .remove(0);
    assert_ne!(old.session, new.session);
    assert_eq!(
        controller.observe(applied(&old)),
        Err(ControlError::StaleObservation)
    );
    assert_eq!(
        controller.broker_disconnected(&old.session),
        Err(ControlError::StaleObservation)
    );
    assert!(!inspect(&mut controller, &target).ready);
    prepare(&mut controller, &new);
    assert!(controller.observe(applied(&new)).unwrap().ready);
}

#[test]
fn authenticated_broker_sequence_is_adopted_and_replay_is_rejected() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    let target = launch("prime", 1);
    controller
        .register_launch(target.clone(), &credentials(), None)
        .unwrap();
    controller.confirm_launch(&target).unwrap();
    let session = BrokerSession {
        controller: OpaqueId::from_bytes([1; 32]),
        broker: OpaqueId::from_bytes([2; 32]),
        connection: 913,
    };
    let transaction = controller
        .broker_session_authenticated(session.clone())
        .unwrap()
        .remove(0);
    assert_eq!(transaction.session, session);
    prepare(&mut controller, &transaction);
    assert!(controller.observe(applied(&transaction)).unwrap().ready);
    let before = inspect(&mut controller, &target);
    for candidate in [
        session.clone(),
        BrokerSession {
            connection: 912,
            ..session.clone()
        },
        BrokerSession {
            connection: 0,
            ..session.clone()
        },
        BrokerSession {
            controller: OpaqueId::from_bytes([9; 32]),
            connection: 914,
            ..session.clone()
        },
        BrokerSession {
            broker: OpaqueId::from_bytes([0; 32]),
            ..session.clone()
        },
    ] {
        assert_eq!(
            controller.broker_session_authenticated(candidate),
            Err(ControlError::StaleObservation)
        );
        assert_eq!(inspect(&mut controller, &target), before);
    }
    controller.broker_disconnected(&session).unwrap();
    assert_eq!(
        controller.broker_session_authenticated(session.clone()),
        Err(ControlError::StaleObservation)
    );
    let next = controller
        .broker_session_authenticated(BrokerSession {
            connection: 914,
            ..session.clone()
        })
        .unwrap()
        .remove(0);
    assert!(!inspect(&mut controller, &target).ready);
    prepare(&mut controller, &next);
    assert!(controller.observe(applied(&next)).unwrap().ready);
    let restarted = controller
        .broker_session_authenticated(BrokerSession {
            broker: OpaqueId::from_bytes([3; 32]),
            connection: 1,
            ..session.clone()
        })
        .unwrap()
        .remove(0);
    assert!(!inspect(&mut controller, &target).ready);
    assert_eq!(
        controller.observe(applied(&next)),
        Err(ControlError::StaleObservation)
    );
    prepare(&mut controller, &restarted);
    assert!(controller.observe(applied(&restarted)).unwrap().ready);
    assert_eq!(
        controller.broker_session_authenticated(session),
        Err(ControlError::StaleObservation)
    );
}

#[test]
fn broker_incarnation_history_is_bounded_without_forgetting_old_fences() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    for number in 1u64..=256 {
        let mut bytes = [0; 32];
        bytes[..8].copy_from_slice(&number.to_be_bytes());
        controller
            .broker_session_authenticated(BrokerSession {
                controller: OpaqueId::from_bytes([1; 32]),
                broker: OpaqueId::from_bytes(bytes),
                connection: 1,
            })
            .unwrap();
    }
    assert_eq!(
        controller.broker_session_authenticated(BrokerSession {
            controller: OpaqueId::from_bytes([1; 32]),
            broker: OpaqueId::from_bytes([9; 32]),
            connection: 1,
        }),
        Err(ControlError::ResourceLimit)
    );
    let mut first = [0; 32];
    first[..8].copy_from_slice(&1u64.to_be_bytes());
    assert_eq!(
        controller.broker_session_authenticated(BrokerSession {
            controller: OpaqueId::from_bytes([1; 32]),
            broker: OpaqueId::from_bytes(first),
            connection: 1,
        }),
        Err(ControlError::StaleObservation)
    );
}

#[test]
fn brokerless_launches_replace_without_an_ssh_service_but_keep_generation_fences() {
    for guest_key in [false, true] {
        let mut plan = credentials();
        if guest_key {
            plan.ssh[0].binding = CredentialBinding::Guest;
        } else {
            plan.ssh.clear();
        }
        let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
        let old = launch("worker", 1);
        let next = launch("worker", 2);
        controller
            .register_launch(old.clone(), &plan, None)
            .unwrap();
        controller.confirm_launch(&old).unwrap();
        assert_eq!(
            controller.register_launch(next.clone(), &plan, Some(&old)),
            Err(ControlError::RevisionConflict)
        );
        controller.retire_launch(&old).unwrap();
        assert_eq!(
            controller.register_launch(next.clone(), &plan, None),
            Err(ControlError::RevisionConflict)
        );
        assert!(
            controller
                .register_launch(next.clone(), &plan, Some(&old))
                .unwrap()
                .transaction
                .is_none()
        );
        controller.confirm_launch(&next).unwrap();
        assert!(controller.verify_registered_launch(&next).is_ok());
        assert_eq!(
            controller.verify_registered_launch(&old),
            Err(ControlError::StaleLaunch)
        );
        assert_eq!(
            controller.retire_launch(&old),
            Err(ControlError::StaleLaunch)
        );
        assert_eq!(
            controller.register_launch(old, &plan, Some(&next)),
            Err(ControlError::StaleLaunch)
        );
    }
}

#[test]
fn an_empty_selection_does_not_bypass_retirement_of_a_broker_capable_launch() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    let old = launch("worker", 1);
    controller
        .register_launch(old.clone(), &credentials(), None)
        .unwrap();
    controller.confirm_launch(&old).unwrap();
    controller
        .request(
            &AuthenticatedCaller::local_operator(1000),
            SshRequest::Revoke {
                launch: old.clone(),
                expected_revision: 1,
            },
        )
        .unwrap();
    controller.retire_launch(&old).unwrap();
    assert_eq!(
        controller.register_launch(launch("worker", 2), &credentials(), Some(&old)),
        Err(ControlError::RevisionConflict)
    );
}

#[test]
fn broker_state_loss_cannot_be_repaired_by_replaying_an_old_acknowledgment() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    let target = launch("prime", 1);
    controller
        .register_launch(target.clone(), &credentials(), None)
        .unwrap();
    controller.confirm_launch(&target).unwrap();
    let old = controller
        .broker_connected(OpaqueId::from_bytes([2; 32]))
        .unwrap()
        .remove(0);
    prepare(&mut controller, &old);
    controller.observe(applied(&old)).unwrap();
    let desired = inspect(&mut controller, &target).desired;
    let mut lost = applied(&old);
    lost.outcome = AppliedOutcome::StateLost;
    assert!(!controller.observe(lost).unwrap().ready);
    assert_eq!(
        controller.validate_broker_session(&old.session),
        Err(ControlError::StaleObservation)
    );
    assert_eq!(
        controller.observe(applied(&old)),
        Err(ControlError::StaleObservation)
    );
    assert_eq!(inspect(&mut controller, &target).desired, desired);
    let fresh = controller
        .broker_connected(OpaqueId::from_bytes([2; 32]))
        .unwrap()
        .remove(0);
    prepare(&mut controller, &fresh);
    assert!(controller.observe(applied(&fresh)).unwrap().ready);
}

#[test]
fn state_loss_fences_all_launch_receipts_until_authenticated_reassertion() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    let targets = [launch("prime", 1), launch("codex", 2)];
    for target in &targets {
        controller
            .register_launch(target.clone(), &credentials(), None)
            .unwrap();
        controller.confirm_launch(target).unwrap();
    }
    let old = controller
        .broker_connected(OpaqueId::from_bytes([2; 32]))
        .unwrap();
    for transaction in &old {
        prepare(&mut controller, transaction);
        assert!(controller.observe(applied(transaction)).unwrap().ready);
    }
    let mut lost = applied(&old[0]);
    lost.outcome = AppliedOutcome::StateLost;
    assert!(!controller.observe(lost).unwrap().ready);
    for transaction in &old {
        assert!(!inspect(&mut controller, &transaction.launch).ready);
        assert_eq!(
            controller.observe(applied(transaction)),
            Err(ControlError::StaleObservation)
        );
    }
    let pending = controller
        .request(
            &AuthenticatedCaller::local_operator(1000),
            SshRequest::Reconcile {
                launch: targets[0].clone(),
                expected_revision: 1,
                credentials: BTreeSet::from(["git".into()]),
            },
        )
        .unwrap();
    assert!(pending.transaction.is_none());
    assert!(!pending.status.ready);
    let new = controller
        .broker_connected(OpaqueId::from_bytes([2; 32]))
        .unwrap();
    for transaction in &new {
        assert_ne!(transaction.session, old[0].session);
        assert!(!inspect(&mut controller, &transaction.launch).ready);
        prepare(&mut controller, transaction);
        assert!(controller.observe(applied(transaction)).unwrap().ready);
    }
    // A delayed loss from the old connection cannot invalidate the new one.
    let mut stale_loss = applied(&old[0]);
    stale_loss.outcome = AppliedOutcome::StateLost;
    assert_eq!(
        controller.observe(stale_loss),
        Err(ControlError::StaleObservation)
    );
    for target in &targets {
        assert!(inspect(&mut controller, target).ready);
    }
}

#[test]
fn invalid_state_loss_does_not_invalidate_current_applied_state() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    let target = launch("prime", 1);
    controller
        .register_launch(target.clone(), &credentials(), None)
        .unwrap();
    controller.confirm_launch(&target).unwrap();
    let transaction = controller
        .broker_connected(OpaqueId::from_bytes([2; 32]))
        .unwrap()
        .remove(0);
    prepare(&mut controller, &transaction);
    controller.observe(applied(&transaction)).unwrap();
    let before = inspect(&mut controller, &target);
    for field in 0..4 {
        let mut lost = applied(&transaction);
        lost.outcome = AppliedOutcome::StateLost;
        match field {
            0 => lost.launch = launch("absent", 3),
            1 => lost.revision += 1,
            2 => lost.policy_digest = OpaqueId::from_bytes([9; 32]),
            _ => lost.failure = Some(BrokerFailure::Unavailable),
        }
        assert_eq!(
            controller.observe(lost),
            Err(ControlError::StaleObservation)
        );
        assert_eq!(inspect(&mut controller, &target), before);
    }
}

#[test]
fn unauthorized_requests_do_not_disclose_registration_or_change_desired_state() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    let existing = launch("prime", 1);
    controller
        .register_launch(existing.clone(), &credentials(), None)
        .unwrap();
    let before = inspect(&mut controller, &existing);
    let nobody = AuthenticatedCaller::restricted(CallerIdentity::LocalUid(2000), Vec::new());
    for target in [existing.clone(), launch("absent", 99)] {
        assert_eq!(
            controller.request(
                &nobody,
                SshRequest::Revoke {
                    launch: target,
                    expected_revision: 1
                }
            ),
            Err(ControlError::PermissionDenied)
        );
    }
    assert_eq!(inspect(&mut controller, &existing), before);
}

#[test]
fn authenticated_retired_workload_cannot_manage_another_resource() {
    let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
    let origin = launch("prime", 1);
    let target = launch("codex", 2);
    for item in [&origin, &target] {
        controller
            .register_launch(item.clone(), &credentials(), None)
            .unwrap();
        controller.confirm_launch(item).unwrap();
    }
    let caller = AuthenticatedCaller::restricted(
        CallerIdentity::WorkloadLaunch(origin.clone()),
        vec![Permission {
            resource: ResourceScope::Workload(target.instance.workload.clone()),
            operations: BTreeSet::from([SshOperation::Inspect.into()]),
        }],
    );
    assert!(
        controller
            .request(
                &caller,
                SshRequest::Inspect {
                    launch: target.clone()
                }
            )
            .is_ok()
    );
    controller.retire_launch(&origin).unwrap();
    assert_eq!(
        controller.request(&caller, SshRequest::Inspect { launch: target }),
        Err(ControlError::PermissionDenied)
    );
}

proptest! {
    #[test]
    fn permissions_match_action_and_scope_in_the_same_record(
        rows in proptest::collection::vec((0u8..3, 0u8..3, any::<bool>()), 0..17),
        workload in 0u8..3, operation in 0u8..3, generation in 1u8..4,
    ) {
        let operations = [SshOperation::Inspect, SshOperation::Reconcile, SshOperation::Revoke];
        let names = ["prime", "codex", "other"];
        let target = launch(names[workload as usize], generation);
        let permissions = rows.iter().map(|&(w, op, exact)| {
            let resource = launch(names[w as usize], 1);
            Permission {
                resource: if exact { ResourceScope::Launch(resource) } else { ResourceScope::Workload(resource.instance.workload) },
                operations: BTreeSet::from([operations[op as usize].into()]),
            }
        }).collect();
        let caller = AuthenticatedCaller::restricted(CallerIdentity::LocalUid(2000), permissions);
        let request = match operation {
            0 => SshRequest::Inspect { launch: target },
            1 => SshRequest::Reconcile { launch: target, expected_revision: 1, credentials: BTreeSet::from(["git".into()]) },
            _ => SshRequest::Revoke { launch: target, expected_revision: 1 },
        };
        let expected = rows.iter().any(|&(w, op, exact)| w == workload && op == operation && (!exact || generation == 1));
        prop_assert_eq!(caller.authorize(&request).is_ok(), expected);
    }

    #[test]
    fn narrowing_preserves_full_compiled_records_and_rejects_foreign_names(
        keys in proptest::collection::btree_set(0u8..8, 1..9),
        chosen in proptest::collection::btree_set(0u8..10, 0..11),
    ) {
        let mut plan = credentials();
        plan.ssh = keys.iter().map(|key| {
            let mut row = plan.ssh[0].clone();
            row.name = format!("key-{key}"); row.material = format!("MATERIAL_{key}"); row.users = vec![format!("user-{key}")]; row
        }).collect();
        let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
        controller.broker_connected(OpaqueId::from_bytes([2; 32])).unwrap();
        let target = launch("prime", 1);
        controller.register_launch(target.clone(), &plan, None).unwrap();
        let before = inspect(&mut controller, &target);
        let result = controller.request(&AuthenticatedCaller::local_operator(1000), SshRequest::Reconcile {
            launch: target.clone(), expected_revision: 1,
            credentials: chosen.iter().map(|key| format!("key-{key}")).collect(),
        });
        if chosen.is_subset(&keys) {
            let transaction = result.unwrap().transaction.unwrap();
            let expected: Vec<_> = plan.ssh.iter().filter(|record| chosen.iter().any(|key| record.name == format!("key-{key}"))).cloned().collect();
            prop_assert_eq!(transaction.credentials, expected);
            prop_assert_eq!(transaction.revision, 2);
        } else {
            prop_assert_eq!(result, Err(ControlError::InvalidPolicy));
            prop_assert_eq!(inspect(&mut controller, &target), before);
        }
    }

    #[test]
    fn independent_workload_lifecycles_match_readiness_model(
        actions in proptest::collection::vec((0u8..7, 0usize..2, any::<bool>()), 1..49),
    ) {
        let mut controller = SshController::new(OpaqueId::from_bytes([1; 32]));
        controller.broker_connected(OpaqueId::from_bytes([2; 32])).unwrap();
        let targets = [launch("prime", 1), launch("codex", 2)];
        let mut transactions = targets.iter().map(|target| controller.register_launch(target.clone(), &credentials(), None).unwrap().transaction.unwrap()).collect::<Vec<_>>();
        for transaction in &transactions { prepare(&mut controller, transaction); }
        let mut live = [false; 2];
        let mut installed = [false; 2];
        let mut selected = [true; 2];
        let mut revision = [1u64; 2];
        let mut connected = true;
        for (action, index, valid) in actions {
            match action {
                0 => { controller.confirm_launch(&targets[index]).unwrap(); live[index] = true; },
                1 => {
                    let mut report = applied(&transactions[index]);
                    if !valid { report.revision += 1; }
                    let result = controller.observe(report);
                    prop_assert_eq!(result.is_ok(), valid && connected);
                    if valid && connected { installed[index] = true; }
                },
                2 | 3 => {
                    let expected_revision = revision[index] + u64::from(!valid);
                    let request = if action == 2 {
                        SshRequest::Revoke { launch: targets[index].clone(), expected_revision }
                    } else {
                        SshRequest::Reconcile { launch: targets[index].clone(), expected_revision, credentials: BTreeSet::from(["git".into()]) }
                    };
                    let result = controller.request(&AuthenticatedCaller::local_operator(1000), request);
                    prop_assert_eq!(result.is_ok(), valid);
                    if valid {
                        revision[index] += 1; selected[index] = action == 3; installed[index] = false;
                        let transaction = result.unwrap().transaction;
                        prop_assert_eq!(transaction.is_some(), connected);
                        if let Some(transaction) = transaction { prepare(&mut controller, &transaction); transactions[index] = transaction; }
                    }
                },
                4 => {
                    let old = transactions.clone();
                    let reassert = controller.broker_connected(OpaqueId::from_bytes([2 + u8::from(valid); 32])).unwrap();
                    for item in reassert { prepare(&mut controller, &item); let i = usize::from(item.launch == targets[1]); transactions[i] = item; }
                    installed = [false; 2];
                    connected = true;
                    for item in old { prop_assert_eq!(controller.observe(applied(&item)), Err(ControlError::StaleObservation)); }
                },
                5 => {
                    let mut report = applied(&transactions[index]); report.outcome = AppliedOutcome::StateLost;
                    let result = controller.observe(report);
                    prop_assert_eq!(result.is_ok(), connected);
                    if connected { installed = [false; 2]; connected = false; }
                },
                _ => {
                    let mut report = applied(&transactions[index]); report.policy_digest = OpaqueId::from_bytes([255; 32]);
                    prop_assert_eq!(controller.observe(report), Err(ControlError::StaleObservation));
                },
            }
            for i in 0..2 {
                let actual = inspect(&mut controller, &targets[i]);
                prop_assert_eq!(actual.ready, live[i] && installed[i] && selected[i]);
                prop_assert_eq!(actual.desired.revision, revision[i]);
            }
        }
    }
}

#[test]
fn wire_request_cannot_assert_a_caller_or_insert_credential_material() {
    let request = SshRequest::Inspect {
        launch: launch("prime", 1),
    };
    let mut value = serde_json::to_value(request).unwrap();
    value["caller"] = serde_json::json!({"uid": 1000});
    assert!(serde_json::from_value::<SshRequest>(value).is_err());
    assert!(serde_json::from_str::<OpaqueId>("\"abc\"").is_err());
    assert!(serde_json::from_value::<OpaqueId>(serde_json::json!("A".repeat(64))).is_err());
}
