//! ensure-images pre-flight (spec 21 §2, phase E): the parent-side lifecycle
//! wiring that makes `workload up` / `workload exec` / bare `workload up`
//! build+load nix-layered images BEFORE any spawn.
//!
//! This module is a thin lifecycle entry point over the phase-C/D machinery
//! — it deliberately REUSES [`build_cmd::resolve_targets`] (declaring-layer
//! provenance selection, §2.3 eligibility: `image.recipe == "nix-layered"`
//! only, mirroring `flake_root_requirement`'s predicate) and
//! [`build_cmd::process_target`] (the lock → probe → eval → skew → pipeline
//! flow) rather than duplicating orchestration.
//!
//! The parent/child split (§2.1/§2.2):
//!
//! - The PARENT (a foreground `up`/`exec` without the token, or the detached
//!   up's spawning process) ensures: [`ensure_images_for_workload`] for the
//!   named verb, [`ensure_images_for_workloads`] for the bare-up batch.
//! - The DETACHED CHILD carries the `images_ready` token (the hidden
//!   `--images-ready` flag `detach_args` appends unconditionally) and skips
//!   ensure ENTIRELY — no nix eval, no store probe — via
//!   [`ensure_should_run`]. A child-side build is a guaranteed-invisible
//!   timeout (redirected output + the FS-8 500ms grace), so the skip is
//!   load-bearing, not an optimization.
//! - Dependency auto-start inherits the pre-flight per dependency (§2.1):
//!   `commands::deps::start_service_detached` runs
//!   [`ensure_images_for_workload`] with `force = false` before spawning
//!   each dep (the dep path never carries the token). `--reload-images` is
//!   named-workload/batch scoped (USER DECISION D3), so deps get the plain
//!   skew matrix, not the force.
//!
//! Ordering (§2.4 + A2/A5 seam): `main.rs` ensures the NAMED workload BEFORE
//! `auto_start_dependencies` runs — fail fast on the workload the operator
//! actually asked for before spending minutes starting its dep closure.
//! EXCEPTION (A2, ADR 0032 §Image tags): when a per-workload inline override
//! is pending, the named ensure runs AFTER dep auto-start + arming so it
//! sees the substituted config and tags under the override's tag context —
//! see [`ensure_after_arming`]. The bare-up batch ensures ALL starts before
//! ANY spawn (`cmd_workload_up_all`).
//!
//! §7 degradation is inherited verbatim from `process_target` (nix absent +
//! tag present → proceed with a stderr note; nix absent + tag missing →
//! hard error + remediation; unreachable store → the named error). The "No
//! flake.nix in the declaring repo" row is mode-split exactly like the build
//! verb: single-target hard error naming the repo, batch skip-with-note —
//! the shared [`build_cmd::SelectSkip`] message helpers keep the wording
//! byte-identical.

use std::path::Path;

use anyhow::Result;

use crate::images::build_cmd::{
    process_target, resolve_targets, BuildScope, BuildTarget, TargetSeams,
};
use crate::images::detect::{DrvEvaluator, MsbStoreProbe, NixCliEvaluator, StoreProbe};
use crate::images::pipeline::{ImageBuilder, ImageLoader, MsbCliLoader, NixCliBuilder};

/// The token gate (spec §2.2, pure): ensure-images runs only on the start
/// verbs (`up`/`exec`) AND only when the process does NOT carry the
/// `images_ready` detach token. `plan`/`down`/`logs` never ensure; the
/// detached child (token present) never ensures.
pub fn ensure_should_run(verb: &str, images_ready: bool) -> bool {
    matches!(verb, "up" | "exec") && !images_ready
}

/// A2/A5 seam (ADR 0032 §Image tags — DECIDED 2026-08-24; pure): when a
/// per-workload inline override is PENDING, the ensure pre-flight must run
/// AFTER `auto_start_dependencies` + `arm_inline_override` (not in its
/// historic pre-auto-start fail-fast slot) so the ensure sees the
/// SUBSTITUTED config and tags/pointers under the OVERRIDE's tag context
/// ([`crate::images::state::image_tag_context`]). With no pending override
/// the historic order stands (fail-fast preserved). The detached child
/// (`images_ready` token) and non-start verbs never reorder — they never
/// ensure at all.
pub fn ensure_after_arming(verb: &str, images_ready: bool, pending_override: bool) -> bool {
    pending_override
        && crate::config::verb_arms_after_dep_autostart(verb)
        && ensure_should_run(verb, images_ready)
}

/// Ensure the named workload's nix-layered images are built+loaded+recorded
/// (single-target semantics): a NoFlakeRoot declaring repo is a HARD ERROR
/// naming the repo (spec §7); a non-nix-layered workload is a silent no-op
/// (§2.3). `force` is the `--reload-images` flag (spec §5.2).
pub async fn ensure_images_for_workload(name: &str, force: bool) -> Result<()> {
    ensure_named(std::slice::from_ref(&name.to_string()), force, true).await
}

/// Batch ensure (bare `workload up`, spec §2.1 + §5.2 USER DECISION D3):
/// every named workload gets the pre-flight with the SAME force flag, all
/// BEFORE any spawn. A NoFlakeRoot declaring repo is a skip-with-note naming
/// the missing flake; the rest of the batch proceeds (spec §7 batch row).
pub async fn ensure_images_for_workloads(names: &[String], force: bool) -> Result<()> {
    ensure_named(names, force, false).await
}

/// Shared resolution + ensure: resolve every name FIRST (fail fast on
/// single-target NoFlakeRoot before any build work), then run the ensure
/// pass over the collected targets with the real seams.
async fn ensure_named(names: &[String], force: bool, single: bool) -> Result<()> {
    let mut targets: Vec<BuildTarget> = Vec::new();
    for name in names {
        let (resolved, skips) = resolve_targets(BuildScope::Name(name))?;
        for skip in &skips {
            if single {
                anyhow::bail!("{}", skip.hard_error_message());
            }
            eprintln!("{}", skip.note_message());
        }
        targets.extend(resolved);
    }
    if targets.is_empty() {
        return Ok(());
    }
    let state_dir = crate::config::resolve_state_dir();
    let mut probe = MsbStoreProbe;
    let mut eval = NixCliEvaluator::new();
    let mut builder = NixCliBuilder::new();
    let mut loader = MsbCliLoader::new();
    let mut seams = TargetSeams {
        probe: &mut probe,
        eval: &mut eval,
        builder: &mut builder,
        loader: &mut loader,
    };
    ensure_resolved(&targets, &state_dir, force, &mut seams).await
}

/// The seam-injected ensure core (unit-tested with the phase-C/D fakes):
/// run each target through the full lock → probe → eval → skew → pipeline
/// flow (build mode — never `--check`) with the same force flag, emitting
/// one operator-facing stderr line per ensured target. Hard errors (the §7
/// unreachable-store / nix-absent-tag-missing / eval / pipeline rows) abort
/// the pass — the documented fail-fast posture.
pub async fn ensure_resolved<P: StoreProbe, E: DrvEvaluator, B: ImageBuilder, L: ImageLoader>(
    targets: &[BuildTarget],
    state_dir: &Path,
    force: bool,
    seams: &mut TargetSeams<'_, P, E, B, L>,
) -> Result<()> {
    for target in targets {
        let report = process_target(target, state_dir, false, force, &mut *seams).await?;
        eprintln!(
            "ensure-images: {}: {} — {}",
            report.tag, report.decision, report.action_taken
        );
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::config::test_support::{unique_state_dir, EnvGuard, ENV_TEST_LOCK, HOME_ENV_KEYS};
    use crate::config::ConfigFile;
    use crate::images::build_cmd::SelectSkip;
    use crate::images::detect::test_fakes::{FakeEvaluator, FakeStoreProbe};
    use crate::images::detect::DrvEvalError;
    use crate::images::pipeline::test_fakes::{FakeBuilder, FakeLoader};
    use crate::images::repo_key::repo_identity_for;
    use crate::images::skew::StoreTag;
    use crate::images::state::{image_key, images_state_path, ImagesState};
    use crate::merge::Provenance as MergeProvenance;

    // ---- the token gate (spec §2.2, BOTH directions) ----

    /// The make-or-break token mechanism: foreground + token → NO ensure
    /// (the detached child never re-ensures inside the FS-8 window);
    /// foreground WITHOUT the token → ensure runs (the parent). Non-start
    /// verbs never ensure, token or not.
    #[test]
    fn ensure_should_run_gates_on_verb_and_token_both_directions() {
        // The child (token) must skip: no double-build inside the FS-8 grace.
        assert!(!ensure_should_run("up", true));
        assert!(!ensure_should_run("exec", true));
        // The parent (no token) must ensure: never skip when needed.
        assert!(ensure_should_run("up", false));
        assert!(ensure_should_run("exec", false));
        // Non-start verbs never ensure.
        for verb in ["plan", "down", "logs", "build", "new"] {
            assert!(!ensure_should_run(verb, false), "{verb} must not ensure");
            assert!(!ensure_should_run(verb, true), "{verb} must not ensure");
        }
    }

    // ---- §7 missing-flake wording (shared helpers, byte-pinned) ----

    fn no_flake_skip() -> SelectSkip {
        SelectSkip::NoFlakeRoot {
            workload: "pi".to_string(),
            repo_key: "personal".to_string(),
            declaring_dir: PathBuf::from("/tmp/declaring"),
        }
    }

    /// Single-target: hard error naming the repo + the remediation.
    #[test]
    fn missing_flake_single_target_is_a_hard_error_naming_the_repo() {
        let msg = no_flake_skip().hard_error_message();
        assert!(msg.contains("workload 'pi'"), "names the workload: {msg}");
        assert!(msg.contains("repo 'personal'"), "names the repo: {msg}");
        assert!(msg.contains("has no flake.nix ancestor"), "{msg}");
        assert!(msg.contains("add a flake.nix"), "remediation: {msg}");
        assert!(msg.contains("config-repo ritual"), "remediation: {msg}");
    }

    /// Batch: skip-with-note naming the missing flake (the batch proceeds).
    #[test]
    fn missing_flake_batch_is_a_skip_with_note_naming_the_flake() {
        let msg = no_flake_skip().note_message();
        assert!(msg.starts_with("note: skipping workload 'pi'"), "{msg}");
        assert!(msg.contains("repo 'personal'"), "names the repo: {msg}");
        assert!(msg.contains("has no flake.nix ancestor"), "{msg}");
    }

    // ---- eligibility (§2.3): nix-layered only ----

    /// nix-layered vs registry vs local_build-only: the ensure selection
    /// inherits `select_eligible`'s predicate verbatim — only
    /// `image.recipe == "nix-layered"` is selected; a registry workload
    /// WITH a local_build is still a no-op (the predicate is recipe-based,
    /// mirroring `flake_root_requirement`'s nix-layered branch).
    #[test]
    fn ensure_selection_is_nix_layered_only() {
        let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "nix-layered", name = "img-pi" }
command = []

[workloads.web]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.built]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.built.local_build]
recipe = "npm-build"
source = "flake://built"
gating_file = "package-lock.json"
"#;
        let config: ConfigFile = toml::from_str(toml).expect("eligibility fixture parses");

        let tmp = unique_state_dir("ensure-eligibility");
        let checkout = tmp.join("checkout");
        std::fs::create_dir_all(&checkout).unwrap();
        std::fs::write(checkout.join("flake.nix"), "{}\n").unwrap();
        let declaring = checkout.join("workestrate");
        std::fs::create_dir_all(&declaring).unwrap();
        let registered = vec![("personal".to_string(), checkout.clone())];
        let provenance: MergeProvenance = ["pi", "web", "built"]
            .iter()
            .map(|n| (format!("workloads.{n}.image"), "personal".to_string()))
            .collect();
        let layer_dirs = HashMap::from([("personal".to_string(), declaring)]);

        let (targets, skips) = crate::images::build_cmd::select_eligible(
            &config,
            &provenance,
            &layer_dirs,
            &registered,
            None,
        )
        .unwrap();
        let names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["pi"],
            "nix-layered only; local_build is a no-op"
        );
        assert!(skips.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ---- the seam-driven ensure core ----

    /// Pin the A2 tag context to None for the duration of a flow test (the
    /// computed tags asserted below are the two-segment `<name>:<sha>`
    /// form only when no context/override leaks from a parallel test).
    fn pin_no_tag_context() -> std::sync::MutexGuard<'static, ()> {
        let lock = ENV_TEST_LOCK.lock().unwrap();
        crate::config::set_active_context(None);
        crate::config::clear_inline_override();
        lock
    }

    fn target_fixture(label: &str, name: &str) -> (PathBuf, BuildTarget) {
        let tmp = unique_state_dir(label);
        let checkout = tmp.join("checkout");
        std::fs::create_dir_all(&checkout).unwrap();
        std::fs::write(checkout.join("flake.nix"), "{}\n").unwrap();
        let registered = vec![("personal".to_string(), checkout.clone())];
        let declaring = checkout.join("workestrate").join("workloads").join(name);
        std::fs::create_dir_all(&declaring).unwrap();
        let repo = repo_identity_for(&declaring, &registered).expect("flake root resolves");
        let attr = format!("img-{name}");
        let target = BuildTarget {
            name: name.to_string(),
            tag: format!("{attr}:latest"),
            attr,
            repo,
        };
        (tmp, target)
    }

    /// A2 fixtures: evaluated out_paths per test target and their computed
    /// tags (ctx=None).
    const OUT_A: &str = "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-img-alpha.tar.gz";
    const OUT_B: &str = "/nix/store/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb-img-beta.tar.gz";
    const TAG_A: &str = "img-alpha:aaaaaaaaaaaa";
    const TAG_B: &str = "img-beta:bbbbbbbbbbbb";

    /// Fake evaluator queued for one target pass: the A2 out_path eval
    /// (popped FIRST, decides the tag) then the drvPath eval (provenance).
    fn fake_eval(out: &str, drv: &str) -> FakeEvaluator {
        let mut e = FakeEvaluator::new();
        e.push_out_ok(out);
        e.push_ok(drv);
        e
    }

    /// D3 batch force scope at the ensure seam: force=true flips EVERY
    /// eligible target in the batch to a forced rebuild (not just the
    /// first, not just a named one) — both pipelines run, both records
    /// land, both pointers move.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime::tests
    async fn batch_force_applies_to_every_eligible_target() -> Result<()> {
        let _guard = pin_no_tag_context();
        let (tmp_a, target_a) = target_fixture("ensure-force-a", "alpha");
        let (tmp_b, target_b) = target_fixture("ensure-force-b", "beta");
        let state_dir = unique_state_dir("ensure-force-state");

        // Per target: skew probe (present) → rebuild forced → pipeline
        // pre-gate probe (present; no record under the computed key → load)
        // → post-load probe.
        let mut probe = FakeStoreProbe::new();
        for _ in 0..2 {
            probe.push(StoreTag::Present);
            probe.push(StoreTag::Present);
            probe.push(StoreTag::Present);
        }
        let mut eval = FakeEvaluator::new();
        eval.push_out_ok(OUT_A);
        eval.push_ok("drv-A");
        eval.push_out_ok(OUT_B);
        eval.push_ok("drv-B");
        let mut builder = FakeBuilder::new();
        builder.push_ok(OUT_A);
        builder.push_ok(OUT_B);
        let mut loader = FakeLoader::new();
        loader.push_ok();
        loader.push_ok();
        let mut seams = TargetSeams {
            probe: &mut probe,
            eval: &mut eval,
            builder: &mut builder,
            loader: &mut loader,
        };

        ensure_resolved(
            &[target_a.clone(), target_b.clone()],
            &state_dir,
            true,
            &mut seams,
        )
        .await?;

        assert_eq!(
            loader.calls.len(),
            2,
            "BOTH targets force-rebuild+load (D3 batch scope): {:?}",
            loader.calls
        );
        for (target, tag, drv) in [(&target_a, TAG_A, "drv-A"), (&target_b, TAG_B, "drv-B")] {
            let key = image_key("personal", tag);
            let state = ImagesState::load(&state_dir);
            let record = state
                .lookup(&key)
                .unwrap_or_else(|| panic!("record written for {tag}"));
            assert_eq!(record.drv_path, drv);
            assert_eq!(
                state
                    .lookup_pointer(&crate::images::state::pointer_key(
                        "personal",
                        &target.attr,
                        None
                    ))
                    .map(|p| p.tag.as_str()),
                Some(tag),
                "the pointer moved for {}",
                target.name
            );
        }

        let _ = std::fs::remove_dir_all(&tmp_a);
        let _ = std::fs::remove_dir_all(&tmp_b);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// Plain (unforced) batch ensure: D1 trust records the baseline AND moves
    /// the pointer; a second pass hits the content-addressed skip — the batch
    /// does NOT rebuild without the force.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime::tests
    async fn unforced_batch_trusts_then_skips() -> Result<()> {
        let _guard = pin_no_tag_context();
        let (tmp_a, target_a) = target_fixture("ensure-plain-a", "alpha");
        let state_dir = unique_state_dir("ensure-plain-state");

        // Pass 1: absent record (under the computed tag) + tag present →
        // D1 trust+record (no build).
        let mut probe1 = FakeStoreProbe::new();
        probe1.push(StoreTag::Present);
        let mut eval1 = fake_eval(OUT_A, "drv-A");
        let (mut builder1, mut loader1) = (FakeBuilder::new(), FakeLoader::new());
        let mut seams1 = TargetSeams {
            probe: &mut probe1,
            eval: &mut eval1,
            builder: &mut builder1,
            loader: &mut loader1,
        };
        ensure_resolved(
            std::slice::from_ref(&target_a),
            &state_dir,
            false,
            &mut seams1,
        )
        .await?;
        assert!(loader1.calls.is_empty(), "D1 trust never builds");
        assert!(images_state_path(&state_dir).exists());

        // Pass 2: same evaluated out_path → same computed tag → fresh record
        // → content-addressed skip; the pipeline seams stay untouched.
        let mut probe2 = FakeStoreProbe::new();
        probe2.push(StoreTag::Present);
        let mut eval2 = fake_eval(OUT_A, "drv-A");
        let (mut builder2, mut loader2) = (FakeBuilder::new(), FakeLoader::new());
        let mut seams2 = TargetSeams {
            probe: &mut probe2,
            eval: &mut eval2,
            builder: &mut builder2,
            loader: &mut loader2,
        };
        ensure_resolved(
            std::slice::from_ref(&target_a),
            &state_dir,
            false,
            &mut seams2,
        )
        .await?;
        assert!(builder2.calls.is_empty() && loader2.calls.is_empty());

        let _ = std::fs::remove_dir_all(&tmp_a);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// §7 degradation ladder through the ensure seam: nix absent + tag
    /// present → Ok (degrade with a note, no record); nix absent + tag
    /// missing → HARD ERROR with the install-nix / manual-load remediation.
    /// A2: the out_path eval fails first, so the ladder probes the legacy
    /// declared tag (no pointer exists in these fixtures).
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime::tests
    async fn nix_absent_ladder_degrades_or_hard_errors() -> Result<()> {
        let _guard = pin_no_tag_context();
        let (tmp, target) = target_fixture("ensure-nix-absent", "pi");
        let state_dir = unique_state_dir("ensure-nix-absent-state");

        // Tag present → degrade (Ok), no record written.
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present);
        let mut eval = FakeEvaluator::new();
        eval.push_out_err(DrvEvalError::NixAbsent);
        let (mut builder, mut loader) = (FakeBuilder::new(), FakeLoader::new());
        let mut seams = TargetSeams {
            probe: &mut probe,
            eval: &mut eval,
            builder: &mut builder,
            loader: &mut loader,
        };
        ensure_resolved(std::slice::from_ref(&target), &state_dir, false, &mut seams).await?;
        assert!(
            !images_state_path(&state_dir).exists(),
            "the degraded path writes no record"
        );

        // Tag missing → hard error + remediation.
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Gone);
        let mut eval = FakeEvaluator::new();
        eval.push_out_err(DrvEvalError::NixAbsent);
        let (mut builder, mut loader) = (FakeBuilder::new(), FakeLoader::new());
        let mut seams = TargetSeams {
            probe: &mut probe,
            eval: &mut eval,
            builder: &mut builder,
            loader: &mut loader,
        };
        let err = ensure_resolved(std::slice::from_ref(&target), &state_dir, false, &mut seams)
            .await
            .expect_err("nix absent + tag missing is a hard error (§7)");
        let msg = err.to_string();
        assert!(msg.contains("install nix"), "remediation: {msg}");
        assert!(msg.contains("load-images"), "ritual pointer: {msg}");

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- A2: the ensure-ordering decision (main.rs A5 seam) ----

    /// `ensure_after_arming` (pure): the ensure pre-flight moves to AFTER
    /// `auto_start_dependencies` + `arm_inline_override` ONLY when a pending
    /// inline override exists on a start verb in the parent (no detach
    /// token). Every other shape keeps the historic pre-auto-start fail-fast
    /// order.
    #[test]
    fn ensure_after_arming_only_for_pending_override_on_parent_start_verbs() {
        // Pending override + up/exec + parent → reorder.
        assert!(ensure_after_arming("up", false, true));
        assert!(ensure_after_arming("exec", false, true));
        // The detached child (token) never ensures — never reorders.
        assert!(!ensure_after_arming("up", true, true));
        assert!(!ensure_after_arming("exec", true, true));
        // No pending override → today's order stands (fail-fast preserved).
        assert!(!ensure_after_arming("up", false, false));
        assert!(!ensure_after_arming("exec", false, false));
        // Non-start verbs never ensure.
        for verb in ["plan", "down", "logs", "build"] {
            assert!(!ensure_after_arming(verb, false, true), "{verb}");
            assert!(!ensure_after_arming(verb, false, false), "{verb}");
        }
    }

    // ---- A2: ensure → pointer → plan-time resolution (fake seams) ----

    /// Legacy state (no `pointers` key) resolves to the declared tag; after
    /// an ensure pass with the fake seams builds+loads, the pointer exists
    /// and plan-time resolution returns the content-addressed sha tag.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime::tests
    async fn legacy_state_falls_back_then_ensure_pointer_resolves_sha_tag() -> Result<()> {
        let _guard = pin_no_tag_context();
        let (tmp, target) = target_fixture("ensure-pointer", "alpha");
        let state_dir = unique_state_dir("ensure-pointer-state");

        // Seed a LEGACY images.json: a record under the old declared-tag key,
        // NO pointers key at all (serde-default migration).
        let legacy_json = format!(
            r#"{{"version": 1, "images": {{"personal#img-alpha:latest": {{
              "repo": {{"name": "personal", "path": "{}", "flake_root": "{}"}},
              "attr": "img-alpha", "tag": "img-alpha:latest",
              "drv_path": "/nix/store/old.drv", "out_path": "", "digest": null,
              "built_at": "2026-08-02T10:15:00Z", "loaded_at": "2026-08-02T10:16:12Z",
              "loader": "workestrate 0.1.0", "host": "devbox", "user": "node"
            }}}}}}"#,
            tmp.join("checkout").display(),
            tmp.join("checkout").display()
        );
        std::fs::create_dir_all(&state_dir)?;
        std::fs::write(images_state_path(&state_dir), legacy_json)?;

        // Pre-migration resolution: pointer MISS → legacy declared tag
        // (byte-identical pre-migration behavior — golden plans stay green).
        assert_eq!(
            crate::images::state::resolve_image_tag(
                &state_dir,
                Some("personal"),
                "img-alpha",
                "latest"
            ),
            "img-alpha:latest"
        );
        // The legacy record does NOT satisfy the computed-tag lookup: the
        // ensure pass builds (absent record under TAG_A + absent tag).
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Gone); // skew probe for the computed tag
        probe.push(StoreTag::Gone); // pipeline pre-gate probe
        probe.push(StoreTag::Present); // post-load verification
        let mut eval = fake_eval(OUT_A, "drv-A");
        let mut builder = FakeBuilder::new();
        builder.push_ok(OUT_A);
        let mut loader = FakeLoader::new();
        loader.push_ok();
        let mut seams = TargetSeams {
            probe: &mut probe,
            eval: &mut eval,
            builder: &mut builder,
            loader: &mut loader,
        };
        ensure_resolved(std::slice::from_ref(&target), &state_dir, false, &mut seams).await?;

        // Post-migration: the pointer exists and resolution returns the sha
        // tag. The legacy record is untouched (immutable records per tag).
        let state = ImagesState::load(&state_dir);
        assert!(state.lookup("personal#img-alpha:latest").is_some());
        assert_eq!(
            state
                .lookup_pointer(&crate::images::state::pointer_key(
                    "personal",
                    "img-alpha",
                    None
                ))
                .map(|p| p.tag.as_str()),
            Some(TAG_A)
        );
        assert_eq!(
            crate::images::state::resolve_image_tag(
                &state_dir,
                Some("personal"),
                "img-alpha",
                "latest"
            ),
            TAG_A,
            "resolution returns the content-addressed tag once the pointer exists"
        );

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- the env-backed entry points: missing-flake single vs batch ----

    /// A flake-less config dir (WORKESTRATE_CONFIG_DIR single-layer mode)
    /// declaring `wl_name` nix-layered — the §7 missing-flake row.
    fn write_flakeless_config_dir(label: &str, wl_name: &str) -> PathBuf {
        let dir = unique_state_dir(label);
        std::fs::create_dir_all(&dir).unwrap();
        let toml = format!(
            "schema_version = 1\n\n\
             [workloads.{wl_name}]\nkind = \"agent\"\n\
             image = {{ recipe = \"nix-layered\", name = \"img-{wl_name}\" }}\n\
             command = []\n\n\
             [workloads.{wl_name}.network]\ndefault_deny = true\n"
        );
        std::fs::write(dir.join("workestrate.toml"), toml).unwrap();
        dir
    }

    /// Single-target ensure hard-errors naming the flake-less declaring
    /// repo; the batch entry skips-with-note and proceeds (Ok). Env-backed
    /// via WORKESTRATE_CONFIG_DIR (the single-layer dev mode sets the same
    /// provenance/layer-dirs globals the registry path sets).
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime::tests
    async fn missing_flake_single_hard_errors_batch_proceeds() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = unique_state_dir("ensure-flakeless-home");
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("WORKESTRATE_HOME", &home);
        // Not covered by HOME_ENV_KEYS; pin it off so the state dir derives
        // from the temp home hermetically.
        std::env::remove_var("WORKESTRATE_STATE_DIR");
        let config_dir = write_flakeless_config_dir("ensure-flakeless-config", "pi");
        std::env::set_var("WORKESTRATE_CONFIG_DIR", &config_dir);

        // Single: hard error naming the repo (before any nix/msb touch).
        let err = ensure_images_for_workload("pi", false)
            .await
            .expect_err("single-target missing flake is a hard error");
        assert!(
            err.to_string().contains("has no flake.nix ancestor"),
            "§7 single-target wording: {err}"
        );
        assert!(
            err.to_string().contains("workload 'pi'"),
            "names the workload: {err}"
        );

        // Batch: skip-with-note; the batch proceeds (Ok, nothing ensured).
        ensure_images_for_workloads(&["pi".to_string()], false).await?;

        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    /// A registry-recipe workload is a silent no-op for BOTH entry points
    /// (§2.3: nothing to build, nothing to check) — no seams are touched,
    /// so this passes with no nix and no msb store.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime::tests
    async fn non_nix_layered_workloads_are_silent_no_ops() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = unique_state_dir("ensure-noop-home");
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("WORKESTRATE_HOME", &home);
        std::env::remove_var("WORKESTRATE_STATE_DIR");
        let dir = unique_state_dir("ensure-noop-config");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("workestrate.toml"),
            "schema_version = 1\n\n\
             [workloads.web]\nkind = \"service\"\n\
             image = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\n\
             command = []\n\n\
             [workloads.web.network]\ndefault_deny = true\n",
        )?;
        std::env::set_var("WORKESTRATE_CONFIG_DIR", &dir);

        ensure_images_for_workload("web", false).await?;
        ensure_images_for_workload("web", true).await?;
        ensure_images_for_workloads(&["web".to_string()], true).await?;
        assert!(
            !images_state_path(&home.join("state")).exists(),
            "a no-op ensure never touches the state store"
        );

        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }
}
