//! Disposable filesystem and restart tests for the actual durable owner.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};

use super::*;
use crate::config::SecretViolationPolicy;
use crate::control_plane::authorization::AuthenticatedCaller;
use crate::control_plane::custody::SshPolicyTransaction;
use crate::control_plane::types::{
    AppliedOutcome, BrokerObservation, InstanceRef, SshRequest, WorkloadRef,
};
use crate::microsandbox::plan::CredentialsPlan;

fn fixture() -> (tempfile::TempDir, PathBuf, u32) {
    let root = tempfile::tempdir().unwrap();
    let uid = root.path().metadata().unwrap().uid();
    let path = root.path().join("custody");
    (root, path, uid)
}

/// Re-open the store after its previous owner was dropped.
///
/// A `Command` spawned by an unrelated parallel test inherits this
/// process's file descriptors for the fork→exec window; if the fork happens
/// while the previous owner holds `custody.lock`, the child briefly pins the
/// flock past the owner's drop and an immediate re-open fails WouldBlock.
/// The pin lapses as soon as the child execs, so retry briefly instead of
/// asserting on a single attempt.
fn open_after_drop(directory: &Path, uid: u32) -> SshController {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut last = None;
    while std::time::Instant::now() < deadline {
        match SshController::open(directory, uid) {
            Ok(controller) => return controller,
            Err(error) => last = Some(error),
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("custody store did not reopen after owner drop: {last:?}")
}

fn launch(generation: u8) -> LaunchRef {
    LaunchRef {
        instance: InstanceRef {
            workload: WorkloadRef {
                context: Some("fleet".into()),
                name: "worker".into(),
            },
            instance: "worker-1".into(),
        },
        generation: OpaqueId::from_bytes([generation; 32]),
    }
}

fn credentials() -> CredentialsPlan {
    CredentialsPlan {
        ssh: vec![SshGrantPlan {
            name: "git".into(),
            material: "KEY_REFERENCE".into(),
            hosts: vec!["git.example.test".into()],
            users: vec!["git".into()],
            ports: vec![22],
            binding: CredentialBinding::Broker,
            on_violation: SecretViolationPolicy::Block,
        }],
        signing: vec![],
        strict: true,
        strict_origin: None,
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

fn ready(controller: &mut SshController, target: &LaunchRef) -> SshPolicyTransaction {
    controller
        .register_launch(target.clone(), &credentials(), None)
        .unwrap();
    let transaction = controller
        .broker_connected(OpaqueId::from_bytes([44; 32]))
        .unwrap()
        .remove(0);
    controller.confirm_launch(target).unwrap();
    controller
        .prepare_policy(&transaction, OpaqueId::from_bytes([42; 32]))
        .unwrap();
    assert!(controller.observe(applied(&transaction)).unwrap().ready);
    transaction
}

#[test]
fn initialization_is_explicit_and_never_resets_existing_or_missing_state() {
    let (_root, directory, uid) = fixture();
    assert!(SshController::open(&directory, uid).is_err());
    assert!(!directory.exists());
    let first = SshController::initialize(&directory, uid).unwrap();
    let bytes = std::fs::read(directory.join("custody.json")).unwrap();
    assert!(SshController::initialize(&directory, uid).is_err());
    assert_eq!(
        bytes,
        std::fs::read(directory.join("custody.json")).unwrap()
    );
    drop(first);
    std::fs::remove_file(directory.join("custody.json")).unwrap();
    assert!(SshController::open(&directory, uid).is_err());
    assert!(!directory.join("custody.json").exists());
}

#[test]
fn desired_revision_is_durable_before_a_transaction_can_be_returned() {
    let (_root, directory, uid) = fixture();
    let mut controller = SshController::initialize(&directory, uid).unwrap();
    let target = launch(1);
    ready(&mut controller, &target);
    let change = controller
        .request(
            &AuthenticatedCaller::local_operator(uid),
            SshRequest::Revoke {
                launch: target.clone(),
                expected_revision: 1,
            },
        )
        .unwrap();
    assert_eq!(change.transaction.unwrap().revision, 2);
    let saved: SavedState =
        serde_json::from_slice(&std::fs::read(directory.join("custody.json")).unwrap()).unwrap();
    assert_eq!(saved.launches[0].desired.revision, 2);
    assert!(saved.launches[0].desired.credentials.is_empty());
    let old_incarnation = controller.incarnation().clone();
    drop(controller);
    let recovered = open_after_drop(&directory, uid);
    assert_ne!(old_incarnation, *recovered.incarnation());
    let status = recovered.status(&target).unwrap();
    assert_eq!(status.desired.revision, 2);
    assert!(status.observed.is_none());
    assert!(!status.ready);
}

#[test]
fn effective_policy_digest_and_material_rotation_are_committed_before_return() {
    let (_root, directory, uid) = fixture();
    let mut controller = SshController::initialize(&directory, uid).unwrap();
    let target = launch(1);
    let intent = ready(&mut controller, &target);
    let saved: SavedState =
        serde_json::from_slice(&std::fs::read(directory.join("custody.json")).unwrap()).unwrap();
    assert_eq!(
        saved.launches[0].effective_digest,
        Some(OpaqueId::from_bytes([42; 32]))
    );
    let rotated = controller
        .prepare_policy(&intent, OpaqueId::from_bytes([43; 32]))
        .unwrap();
    let saved: SavedState =
        serde_json::from_slice(&std::fs::read(directory.join("custody.json")).unwrap()).unwrap();
    assert_eq!(
        saved.launches[0].effective_digest,
        Some(rotated.policy_digest.clone())
    );
    assert_eq!(saved.launches[0].desired.revision, rotated.intent.revision);
    assert_eq!(rotated.intent.revision, 2);
    drop(controller);
    let mut recovered = open_after_drop(&directory, uid);
    assert!(!recovered.status(&target).unwrap().ready);
    recovered
        .register_launch(target.clone(), &credentials(), None)
        .unwrap();
    recovered.confirm_launch(&target).unwrap();
    let next = recovered
        .broker_connected(OpaqueId::from_bytes([44; 32]))
        .unwrap()
        .remove(0);
    let mut premature = applied(&next);
    premature.policy_digest = rotated.policy_digest.clone();
    assert_eq!(
        recovered.observe(premature),
        Err(ControlError::StaleObservation)
    );
    let reassert = recovered
        .prepare_policy(&next, rotated.policy_digest)
        .unwrap();
    assert_eq!(reassert.intent.revision, 2);
    // A changed material digest after restart is still a new revision; restart
    // must not forget the prior effective policy identity.
    let second = recovered
        .prepare_policy(&reassert.intent, OpaqueId::from_bytes([45; 32]))
        .unwrap();
    assert_eq!(second.intent.revision, 3);
}

#[test]
fn failed_effective_policy_commit_returns_no_sendable_policy() {
    let (_root, directory, uid) = fixture();
    let mut controller = SshController::initialize(&directory, uid).unwrap();
    let target = launch(1);
    let intent = ready(&mut controller, &target);
    let path = directory.join("custody.json");
    let retained = directory.join("previous.json");
    std::fs::rename(&path, &retained).unwrap();
    assert_eq!(
        controller.prepare_policy(&intent, OpaqueId::from_bytes([43; 32])),
        Err(ControlError::StateUnavailable)
    );
    std::fs::rename(&retained, &path).unwrap();
    assert_eq!(
        controller.prepare_policy(&intent, OpaqueId::from_bytes([43; 32])),
        Err(ControlError::StateUnavailable)
    );
    assert_eq!(
        controller.observe(applied(&intent)),
        Err(ControlError::StateUnavailable)
    );
}

#[test]
fn recovery_requires_current_policy_runtime_and_broker_reassertion() {
    let (_root, directory, uid) = fixture();
    let mut controller = SshController::initialize(&directory, uid).unwrap();
    let target = launch(1);
    let old = ready(&mut controller, &target);
    drop(controller);
    let mut recovered = open_after_drop(&directory, uid);
    assert_eq!(
        recovered.observe(applied(&old)),
        Err(ControlError::StaleObservation)
    );
    assert_eq!(
        recovered.confirm_launch(&target),
        Err(ControlError::StateUnavailable)
    );
    assert!(
        recovered
            .broker_connected(OpaqueId::from_bytes([44; 32]))
            .unwrap()
            .is_empty()
    );
    let mut changed = credentials();
    changed.ssh[0].users = vec!["root".into()];
    assert_eq!(
        recovered.register_launch(target.clone(), &changed, None),
        Err(ControlError::InvalidPolicy)
    );
    let transaction = recovered
        .register_launch(target.clone(), &credentials(), None)
        .unwrap()
        .transaction
        .unwrap();
    recovered
        .prepare_policy(&transaction, OpaqueId::from_bytes([42; 32]))
        .unwrap();
    assert!(!recovered.observe(applied(&transaction)).unwrap().ready);
    assert!(recovered.confirm_launch(&target).unwrap().ready);
    assert!(recovered.verify_registered_launch(&target).is_ok());
    let mut relabeled = target;
    relabeled.instance.workload.name = "different-workload".into();
    assert!(recovered.verify_registered_launch(&relabeled).is_err());
}

#[test]
fn observed_disk_loss_permanently_fences_old_acknowledgments() {
    let (_root, directory, uid) = fixture();
    let mut controller = SshController::initialize(&directory, uid).unwrap();
    let target = launch(1);
    let old = ready(&mut controller, &target);
    let file = directory.join("custody.json");
    let retained = directory.join("retained.json");
    std::fs::rename(&file, &retained).unwrap();
    assert_eq!(
        controller.observe(applied(&old)),
        Err(ControlError::StateUnavailable)
    );
    std::fs::rename(&retained, &file).unwrap();
    assert_eq!(
        controller.observe(applied(&old)),
        Err(ControlError::StateUnavailable)
    );
    assert_eq!(
        controller.status(&target),
        Err(ControlError::StateUnavailable)
    );
    assert_eq!(
        controller.broker_connected(OpaqueId::from_bytes([45; 32])),
        Err(ControlError::StateUnavailable)
    );
}

#[test]
fn lock_ownership_is_exclusive_and_recovers_only_after_owner_drop() {
    let (_root, directory, uid) = fixture();
    let controller = SshController::initialize(&directory, uid).unwrap();
    let lock = std::fs::metadata(directory.join("custody.lock")).unwrap();
    assert!(SshController::open(&directory, uid).is_err());
    drop(controller);
    let next = open_after_drop(&directory, uid);
    assert!(same_file(
        &lock,
        &std::fs::metadata(directory.join("custody.lock")).unwrap()
    ));
    drop(next);
}

#[test]
fn replaced_lock_inode_invalidates_the_old_owner_without_unlinking_replacement() {
    let (_root, directory, uid) = fixture();
    let mut controller = SshController::initialize(&directory, uid).unwrap();
    let target = launch(1);
    let transaction = ready(&mut controller, &target);
    let path = directory.join("custody.lock");
    std::fs::rename(&path, directory.join("previous.lock")).unwrap();
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&path)
        .unwrap();
    let replacement = path.metadata().unwrap();
    assert_eq!(
        controller.observe(applied(&transaction)),
        Err(ControlError::StateUnavailable)
    );
    drop(controller);
    assert!(same_file(&replacement, &path.metadata().unwrap()));
}

#[test]
fn snapshots_are_private_nonsecret_and_do_not_persist_readiness() {
    let (_root, directory, uid) = fixture();
    let mut controller = SshController::initialize(&directory, uid).unwrap();
    ready(&mut controller, &launch(1));
    let file = directory.join("custody.json");
    let json: serde_json::Value = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    assert_eq!(file.metadata().unwrap().mode() & 0o777, 0o600);
    assert_eq!(directory.metadata().unwrap().mode() & 0o777, 0o700);
    let row = &json["launches"][0];
    for forbidden in [
        "runtime_live",
        "observed",
        "broker",
        "ready",
        "ceiling_verified",
    ] {
        assert!(row.get(forbidden).is_none());
    }
    assert_eq!(row["credentials"][0]["material"], "KEY_REFERENCE");
}

#[test]
fn retired_generations_survive_restart_and_cannot_be_registered_again() {
    let (_root, directory, uid) = fixture();
    let mut controller = SshController::initialize(&directory, uid).unwrap();
    let target = launch(1);
    ready(&mut controller, &target);
    let retirement = controller
        .retire_launch(&target)
        .unwrap()
        .transaction
        .unwrap();
    controller
        .prepare_policy(&retirement, OpaqueId::from_bytes([42; 32]))
        .unwrap();
    controller.observe(applied(&retirement)).unwrap();
    controller
        .register_launch(launch(2), &credentials(), Some(&target))
        .unwrap();
    drop(controller);
    let mut recovered = open_after_drop(&directory, uid);
    assert_eq!(
        recovered.register_launch(target, &credentials(), None),
        Err(ControlError::StaleLaunch)
    );
    assert!(!recovered.status(&launch(2)).unwrap().ready);
}

#[test]
fn invalid_snapshots_never_become_an_empty_authority() {
    let (_root, directory, uid) = fixture();
    let mut controller = SshController::initialize(&directory, uid).unwrap();
    controller
        .register_launch(launch(1), &credentials(), None)
        .unwrap();
    drop(controller);
    let path = directory.join("custody.json");
    let original = std::fs::read(&path).unwrap();
    let valid: serde_json::Value = serde_json::from_slice(&original).unwrap();
    let mut variants = vec![
        b"{".to_vec(),
        b"{}".to_vec(),
        vec![b' '; MAX_STATE_BYTES + 1],
    ];
    for change in 0..6 {
        let mut json = valid.clone();
        match change {
            0 => json["version"] = 999.into(),
            1 => json["launches"][0]["desired"]["revision"] = 0.into(),
            2 => json["launches"][0]["desired"]["credentials"] = serde_json::json!(["unknown"]),
            3 => json["launches"][0]["desired"]["destroyed"] = true.into(),
            4 => json["launches"]
                .as_array_mut()
                .unwrap()
                .push(valid["launches"][0].clone()),
            _ => json["unexpected"] = true.into(),
        }
        variants.push(serde_json::to_vec(&json).unwrap());
    }
    for bytes in variants {
        std::fs::write(&path, &bytes).unwrap();
        assert!(SshController::open(&directory, uid).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    std::fs::write(&path, original).unwrap();
    let _ = open_after_drop(&directory, uid);
}

#[test]
fn untrusted_files_and_directory_permissions_are_refused_without_repair() {
    let (root, directory, uid) = fixture();
    drop(SshController::initialize(&directory, uid).unwrap());
    let path = directory.join("custody.json");
    let saved = root.path().join("saved.json");
    std::fs::rename(&path, &saved).unwrap();
    symlink(&saved, &path).unwrap();
    assert!(SshController::open(&directory, uid).is_err());
    assert!(path.is_symlink());
    std::fs::remove_file(&path).unwrap();
    std::fs::rename(&saved, &path).unwrap();
    std::fs::hard_link(&path, &saved).unwrap();
    assert!(SshController::open(&directory, uid).is_err());
    std::fs::remove_file(&saved).unwrap();
    std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(SshController::open(&directory, uid).is_err());
    assert_eq!(directory.metadata().unwrap().mode() & 0o777, 0o755);
}

#[test]
fn bounded_encoder_refuses_the_first_byte_beyond_the_limit() {
    let mut writer = BoundedBytes(vec![0; MAX_STATE_BYTES - 1]);
    assert_eq!(writer.write(b"x").unwrap(), 1);
    assert!(writer.write(b"y").is_err());
    assert_eq!(writer.0.len(), MAX_STATE_BYTES);
}

#[test]
fn failed_desired_commit_returns_no_transaction_and_permanently_fences_owner() {
    let (_root, directory, uid) = fixture();
    let mut controller = SshController::initialize(&directory, uid).unwrap();
    let target = launch(1);
    let old = ready(&mut controller, &target);
    let path = directory.join("custody.json");
    let before = std::fs::read(&path).unwrap();
    let mut oversized = credentials();
    let record = oversized.ssh[0].clone();
    oversized.ssh = (0..=MAX_GRANTS)
        .map(|index| {
            let mut grant = record.clone();
            grant.name = format!("credential-{index}");
            grant
        })
        .collect();
    let mut second = launch(2);
    second.instance.instance = "worker-oversized".into();
    assert_eq!(
        controller.register_launch(second, &oversized, None),
        Err(ControlError::StateUnavailable)
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(controller.broker.is_none());
    assert_eq!(
        controller.validate_broker_session(&old.session),
        Err(ControlError::StateUnavailable)
    );
    assert_eq!(
        controller.observe(applied(&old)),
        Err(ControlError::StateUnavailable)
    );
    assert_eq!(
        controller.status(&target),
        Err(ControlError::StateUnavailable)
    );
    drop(controller);
    let recovered = open_after_drop(&directory, uid);
    assert_eq!(recovered.launches.len(), 1);
    assert!(!recovered.status(&target).unwrap().ready);
}
