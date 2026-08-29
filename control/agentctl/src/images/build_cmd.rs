//! `workestrate workload build` (spec 21 §5.1, phases C+D): selector
//! resolution, the per-workload lock → probe → eval → skew → act flow,
//! `--check`, and the human/JSON renderers.
//!
//! Phase C landed change detection + the skew matrix + the D1 trust record;
//! phase D landed the build/load pipeline behind the Build/Rebuild/
//! RebuildForced decisions ([`crate::images::pipeline`] — nix build →
//! outPath re-load gate → `msb load` → record upsert, all inside the
//! still-held per-tag lock). Recorded phase-C decisions (all still live):
//!
//! - **`skew.rs`'s `StoreTag` stays 2-variant.** The unreachable-store case
//!   is the named §7 error ([`detect::StoreUnreachable`]), not a skew
//!   decision; the nix-absent degrade is the §7 ladder handled AROUND the
//!   matrix (store presence is known, freshness is not), not inside it. No
//!   `Unknown` variant proved necessary.
//! - **D1 trust record shape (A2):** `drv_path` = the current eval;
//!   `out_path` = the EVALUATED out_path (known since the A2 out-path-first
//!   flow evaluates it before deciding); `digest = None`; the
//!   current-pointer for (repo, attr, ctx) moves in the same locked section.
//!   Freshness is the content-addressed presence predicate
//!   ([`detect::record_state_for`]).
//! - **`--check` takes NO lock.** It is a read-only report (probe + load +
//!   eval, no writes); the §3.3 lock guards the eval → build → load → record
//!   critical section, and `--check` has none.
//! - **Batch unreachable-store posture: fail fast.** The first
//!   [`detect::StoreUnreachable`] aborts the whole command with one named
//!   error rather than spamming per-workload failures against a store that
//!   is down for every probe.
//! - **nix-absent degraded path writes NO record** (nothing trustworthy to
//!   record — the freshness of the trusted tag is unverifiable without nix).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::{ConfigFile, ConfigRepoEntry};
use crate::images::detect::{
    record_state_for, DrvEvalError, DrvEvaluator, MsbStoreProbe, NixCliEvaluator, StoreProbe,
};
use crate::images::gc;
use crate::images::lock::ImageTagLock;
use crate::images::pipeline::{
    run_build_pipeline, BuildJob, ImageBuilder, ImageLoader, ImageRemover, LoadAction,
    MsbCliLoader, MsbCliRemover, NixCliBuilder, PipelineOutcome,
};
use crate::images::repo_key::{registered_repo_checkouts, repo_identity_for, repo_key_for};
use crate::images::skew::{decide_skew, RecordState, SkewDecision, StoreTag};
use crate::images::state::{
    compute_image_tag, image_key, image_tag_context, pointer_key, ImageRecord, ImagesState,
    PointerRecord, Provenance, RepoIdentity,
};
use crate::merge::Provenance as MergeProvenance;

/// The exact zero-eligible note (spec §5.1, stderr). Pinned by test.
pub const ZERO_ELIGIBLE_NOTE: &str = "note: no nix-layered workloads in scope; nothing to build";

// ---------------------------------------------------------------------------
// Selector resolution (spec §5.1)
// ---------------------------------------------------------------------------

/// A workload selected for the build verb: everything change detection and
/// the (phase-D) pipeline need, resolved from declaring-layer provenance —
/// NEVER name-guessing (spec §5.1, spec 17).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildTarget {
    /// Workload name from the merged config.
    pub name: String,
    /// Flake attribute to eval/build (`image.name` verbatim — the tool flake
    /// names `buildImagesFromConfig` output attrs by it).
    pub attr: String,
    /// The LEGACY declared `<image.name>:<tag|latest>` (USER DECISION D2).
    /// A2 (ADR 0032 §Image tags): no longer loaded into the store — the
    /// effective store tag is the computed content-addressed tag
    /// ([`crate::images::state::compute_image_tag`]). This field remains as
    /// the pre-migration fallback: the §7 nix-absent ladder probes it when
    /// no current-pointer exists yet.
    pub tag: String,
    /// Config-repo identity (repo_key rule + flake_root) for the record key.
    pub repo: RepoIdentity,
    /// The capsule keep-last-N rung (`image.keep_last`, ADR 0032 §Image
    /// tags — RESOLVED user decision 3): the TOP rung of the prune cascade,
    /// resolved against the repo-entry/settings/default rungs at the
    /// prune-on-load trigger point. `None` = not configured on the capsule.
    pub keep_last: Option<u32>,
}

/// A workload skipped during selection (batch modes report these as notes;
/// the single-name mode escalates to a hard error — spec §7 "No flake.nix in
/// the declaring repo" row).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectSkip {
    /// The declaring repo has no `flake.nix` ancestor.
    NoFlakeRoot {
        workload: String,
        /// repo_key (registered name or canonical path) — the repo named in
        /// the error/note.
        repo_key: String,
        declaring_dir: PathBuf,
    },
}

impl SelectSkip {
    /// The §7 single-target escalation (hard error naming the repo +
    /// remediation). Shared by `cmd_workload_build` (single-name scope) and
    /// the phase-E ensure pre-flight (`images::ensure`) so the wording
    /// stays byte-identical across both callers.
    pub fn hard_error_message(&self) -> String {
        match self {
            SelectSkip::NoFlakeRoot {
                workload,
                repo_key,
                declaring_dir,
            } => format!(
                "workload '{workload}' declares a nix-layered image, but its declaring \
                 repo '{repo_key}' ({}) has no flake.nix ancestor — a nix-layered image \
                 build requires a flake root (spec 21 §7); add a flake.nix to the \
                 config repo, or load the image manually via the config-repo ritual",
                declaring_dir.display()
            ),
        }
    }

    /// The §7 batch note (skip-with-note naming the missing flake; the rest
    /// of the batch proceeds). Shared by `cmd_workload_build` (batch scopes)
    /// and the phase-E batch ensure pass (`cmd_workload_up_all`).
    pub fn note_message(&self) -> String {
        match self {
            SelectSkip::NoFlakeRoot {
                workload,
                repo_key,
                declaring_dir,
            } => format!(
                "note: skipping workload '{workload}': declaring repo '{repo_key}' ({}) \
                 has no flake.nix ancestor (spec 21 §7)",
                declaring_dir.display()
            ),
        }
    }
}

/// The selector shape (clap conflicts guarantee exactly one per invocation).
pub enum BuildScope<'a> {
    /// `build <name>` — one workload in the active context.
    Name(&'a str),
    /// bare `build` — all nix-layered workloads in the active context.
    ActiveContext,
    /// `build --repo <config>` — all nix-layered workloads declared by one
    /// registered repo.
    Repo(&'a str),
    /// `build --all-repos` — all registered repos (`registry.configs`).
    AllRepos,
}

/// Pure selector core: filter `config` to the nix-layered eligibility class
/// (spec §2.3, mirroring `flake_root_requirement`'s predicate) and resolve
/// each survivor's repo identity from declaring-layer provenance. `only`
/// restricts to a single workload name (the `build <name>` form).
///
/// Non-nix-layered workloads are skipped SILENTLY (spec §5.1: "non-nix-layered
/// workloads in a selected set are skipped"; the zero-eligible note covers
/// the empty result). A nix-layered workload whose declaring repo has no
/// `flake.nix` ancestor becomes a [`SelectSkip::NoFlakeRoot`].
pub fn select_eligible(
    config: &ConfigFile,
    provenance: &MergeProvenance,
    layer_dirs: &HashMap<String, PathBuf>,
    registered: &[(String, PathBuf)],
    only: Option<&str>,
) -> Result<(Vec<BuildTarget>, Vec<SelectSkip>)> {
    let mut names: Vec<&String> = config.workloads.keys().collect();
    names.sort();
    let mut targets = Vec::new();
    let mut skips = Vec::new();
    for name in names {
        if let Some(only) = only {
            if name != only {
                continue;
            }
        }
        let wl = &config.workloads[name];
        // §2.3 eligibility: nix-layered only — registry / local_build-only
        // workloads have nothing to build or check.
        if wl.image.recipe != "nix-layered" {
            continue;
        }
        let attr = wl.image.name.clone().ok_or_else(|| {
            anyhow::anyhow!(
                "workload '{name}' declares a nix-layered image without image.name \
                 (the flake attribute to eval); set image.name in the declaring config layer"
            )
        })?;
        let tag = format!("{}:{}", attr, wl.image.tag.as_deref().unwrap_or("latest"));
        // Declaring-layer provenance (spec 17): the layer that declared
        // `workloads.<name>.image` decides the repo identity — never the
        // workload NAME (spec §5.1).
        let layer = provenance
            .get(&format!("workloads.{name}.image"))
            .or_else(|| provenance.get(&format!("workloads.{name}.kind")))
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "cannot resolve the declaring layer for workload '{name}': no merge \
                     provenance for workloads.{name}.image (spec 17); reload the config"
                )
            })?;
        let declaring_dir = layer_dirs.get(layer).cloned().ok_or_else(|| {
            anyhow::anyhow!(
                "cannot resolve the declaring config dir for workload '{name}': layer \
                 '{layer}' has no source path (spec 17 provenance)"
            )
        })?;
        match repo_identity_for(&declaring_dir, registered) {
            Some(repo) => targets.push(BuildTarget {
                name: name.clone(),
                attr,
                tag,
                repo,
                keep_last: wl.image.keep_last,
            }),
            None => skips.push(SelectSkip::NoFlakeRoot {
                workload: name.clone(),
                repo_key: repo_key_for(&declaring_dir, registered),
                declaring_dir,
            }),
        }
    }
    Ok((targets, skips))
}

/// Impure selector entry: resolve the invocation's scope against the live
/// config + registry.
pub fn resolve_targets(scope: BuildScope) -> Result<(Vec<BuildTarget>, Vec<SelectSkip>)> {
    let registered = registered_repo_checkouts();
    match scope {
        BuildScope::Name(name) => {
            let config = crate::config::load_config()?;
            if !config.workloads.contains_key(name) {
                anyhow::bail!("workload '{name}' not found in config");
            }
            let provenance = crate::merge::get_provenance().unwrap_or_default();
            let layer_dirs = crate::merge::get_layer_dirs().unwrap_or_default();
            select_eligible(&config, &provenance, &layer_dirs, &registered, Some(name))
        }
        BuildScope::ActiveContext => {
            let config = crate::config::load_config()?;
            let provenance = crate::merge::get_provenance().unwrap_or_default();
            let layer_dirs = crate::merge::get_layer_dirs().unwrap_or_default();
            select_eligible(&config, &provenance, &layer_dirs, &registered, None)
        }
        BuildScope::Repo(name) => {
            let registry = crate::config::load_registry()?.ok_or_else(|| {
                anyhow::anyhow!("no config repos registered; run 'workestrate config add' first")
            })?;
            let entry = registry.configs.get(name).ok_or_else(|| {
                anyhow::anyhow!(
                    "config repo '{name}' is not registered; run 'workestrate config list' \
                     to see registered repos"
                )
            })?;
            targets_for_repo(name, entry, &registered)
        }
        BuildScope::AllRepos => {
            let Some(registry) = crate::config::load_registry()? else {
                // No registry at all → zero repos → zero-eligible no-op.
                return Ok((Vec::new(), Vec::new()));
            };
            let mut names: Vec<&String> = registry.configs.keys().collect();
            names.sort();
            let mut targets = Vec::new();
            let mut skips = Vec::new();
            for name in names {
                // Batch posture (§7 spirit): a repo that cannot be loaded or
                // merged (e.g. it fails the current policy gates standalone)
                // is skipped with a note naming the repo; the REST of the
                // batch proceeds. Explicit `--repo <name>` hard-errors on the
                // same failure (the operator asked for that repo).
                match targets_for_repo(name, &registry.configs[name], &registered) {
                    Ok((t, s)) => {
                        targets.extend(t);
                        skips.extend(s);
                    }
                    Err(e) => eprintln!("note: skipping config repo '{name}': {e:#}"),
                }
            }
            Ok((targets, skips))
        }
    }
}

/// One registered repo's contribution (the `--repo`/`--all-repos` unit):
/// load the repo's OWN layers (its declarations, not the active-context
/// merge), then the pure selector core.
fn targets_for_repo(
    name: &str,
    entry: &ConfigRepoEntry,
    registered: &[(String, PathBuf)],
) -> Result<(Vec<BuildTarget>, Vec<SelectSkip>)> {
    let checkout = if let Some(dir) = crate::config::local_entry_checkout_dir(entry) {
        dir
    } else {
        crate::config::config_repo_dir(name)
    };
    if !checkout.is_dir() {
        eprintln!(
            "note: config repo '{name}' checkout {} is missing; skipping it",
            checkout.display()
        );
        return Ok((Vec::new(), Vec::new()));
    }
    let layers = crate::config::loading::load_config_repo_layers(name, &checkout)
        .with_context(|| format!("failed to load config repo '{name}'"))?;
    if layers.is_empty() {
        // A repo with no config content declares nothing — zero-eligible.
        return Ok((Vec::new(), Vec::new()));
    }
    let layer_dirs = crate::merge::layer_dirs_from(&layers);
    let (config, provenance) = crate::merge::merge_layers(&layers)
        .with_context(|| format!("failed to merge config repo '{name}'"))?;
    select_eligible(&config, &provenance, &layer_dirs, registered, None)
}

// ---------------------------------------------------------------------------
// Per-workload report
// ---------------------------------------------------------------------------

/// One row of the build verb's report (human table and `--json` envelope).
/// String labels (not the enums) so the JSON shape is stable and decoupled
/// from internal variant naming.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetReport {
    pub name: String,
    /// repo_key (registered name or canonical path).
    pub repo: String,
    pub attr: String,
    pub tag: String,
    /// "absent" | "fresh" | "stale" | "unknown" (nix-absent degrade).
    pub record_state: String,
    /// "present" | "gone".
    pub store_state: String,
    /// The current drvPath eval; `None` when nix is absent (unverifiable).
    pub drv_path: Option<String>,
    /// "skip" | "trust+record" | "would trust+record" | "build" |
    /// "would build" | "rebuild" | "would rebuild" | "rebuild (forced)" |
    /// "would rebuild (forced)" | "trust (unverified)".
    pub decision: String,
    pub action_taken: String,
}

fn record_state_label(state: RecordState) -> &'static str {
    match state {
        RecordState::Absent => "absent",
        RecordState::Fresh => "fresh",
        RecordState::Stale => "stale",
    }
}

fn store_state_label(state: StoreTag) -> &'static str {
    match state {
        StoreTag::Present => "present",
        StoreTag::Gone => "gone",
    }
}

/// The five seam implementations the flow drives, bundled so
/// [`process_target`] stays readable (and within the argument-count lint):
/// the two phase-C change-detection seams (`detect.rs`) plus the two
/// phase-D process seams (`pipeline.rs`) and the A2-stage-2 removal seam
/// ([`pipeline::ImageRemover`] — the keep-last-N GC cascade's msb tag
/// removal, ADR 0032 §Image tags).
pub struct TargetSeams<
    'a,
    P: StoreProbe,
    E: DrvEvaluator,
    B: ImageBuilder,
    L: ImageLoader,
    R: ImageRemover,
> {
    pub probe: &'a mut P,
    pub eval: &'a mut E,
    pub builder: &'a mut B,
    pub loader: &'a mut L,
    pub remover: &'a mut R,
}

/// The §7 "nix absent from PATH" ladder report (shared by the out_path and
/// drvPath eval arms of [`process_target`]): `store` presence decides
/// degrade-with-note (Present — no record written, nothing trustworthy to
/// record) vs hard error with the install-nix / config-repo-ritual
/// remediation (Gone). `tag` is the best tag nameable without nix — the
/// computed content tag when the out_path eval succeeded, else the
/// pointer/legacy fallback.
fn nix_absent_ladder(target: &BuildTarget, tag: &str, store: StoreTag) -> Result<TargetReport> {
    match store {
        StoreTag::Present => {
            // Degrade, don't block: the tag may be fresh; nothing is
            // verifiable without nix. NO record is written — nothing
            // trustworthy to record (documented phase-C decision).
            eprintln!(
                "note: nix not found on PATH; cannot verify freshness of '{}' — \
                 proceeding with the store tag as-is; no record written (spec 21 §7)",
                tag
            );
            Ok(TargetReport {
                name: target.name.clone(),
                repo: target.repo.name.clone(),
                attr: target.attr.clone(),
                tag: tag.to_string(),
                record_state: "unknown".to_string(),
                store_state: "present".to_string(),
                drv_path: None,
                decision: "trust (unverified)".to_string(),
                action_taken: "none (nix absent; store tag trusted)".to_string(),
            })
        }
        StoreTag::Gone => Err(anyhow::anyhow!(
            "nix is required to build '{}': no store tag is present and nix was not \
             found on PATH — install nix, or load the image manually via the \
             config-repo ritual (the declaring repo's 'load-images' recipe), then \
             retry (spec 21 §7)",
            tag
        )),
    }
}

/// The per-workload flow (spec §3.3/§3.4/§7 + A2), generic over the four
/// seams so tests drive fakes with a temp state dir:
///
/// 1. **A2 out_path eval FIRST** (eval-only, no build — ADR 0032 §Image
///    tags): the content-addressed tag `<name>:<ctx>.<sha>` (or
///    `<name>:<sha>`) derives from the evaluated out_path, so the eval must
///    precede the store probe and the lock acquisition (the lock keys on
///    the computed tag). The eval is deterministic per flake content, so
///    racing processes compute the SAME tag and serialize on the SAME lock.
/// 2. acquire the per-tag lock (build mode only — `--check` takes NO lock);
/// 3. INSIDE the lock: probe the store for the COMPUTED tag, load
///    `images.json`, run the drvPath eval (recorded for provenance — spec
///    §8 `drv_path`; no longer the freshness signal), derive the record
///    state (presence under the computed key — content-addressed), and
///    `decide_skew` (re-checked inside the lock per the §7 concurrent row);
/// 4. act: Skip → report; TrustAndRecord → write the D1 baseline record
///    AND move the current-pointer (upsert + save INSIDE the lock);
///    Build/Rebuild/RebuildForced → the phase-D pipeline (nix build →
///    outPath gate → `msb load` → record + pointer), still inside the lock;
///    `--check` → structured "would …" report, never the pipeline, never a
///    write.
pub async fn process_target<
    P: StoreProbe,
    E: DrvEvaluator,
    B: ImageBuilder,
    L: ImageLoader,
    R: ImageRemover,
>(
    target: &BuildTarget,
    state_dir: &Path,
    check: bool,
    force: bool,
    seams: &mut TargetSeams<'_, P, E, B, L, R>,
) -> Result<TargetReport> {
    let TargetSeams {
        probe,
        eval,
        builder,
        loader,
        remover,
    } = seams;
    // Reborrow the &mut fields so the generic seam bounds (P: StoreProbe
    // etc.) are satisfied by &mut P directly, not &mut &mut P.
    let probe = &mut **probe;
    let eval = &mut **eval;
    let builder = &mut **builder;
    let loader = &mut **loader;
    let remover = &mut **remover;

    // Step 1: the A2 out_path eval decides the effective tag BEFORE any
    // probe/lock. §7 nix-absent ladder: without nix the computed tag is
    // unknowable — probe the best nameable tag instead: the state-dir
    // current-pointer for (repo, attr, ctx) when one exists (post-migration
    // homes keep degrading correctly), else the legacy declared tag
    // (pre-migration behavior, byte-identical).
    let out_path = match eval.eval_out_path(&target.repo.flake_root, &target.attr) {
        Ok(out) => out,
        Err(DrvEvalError::NixAbsent) => {
            let ctx = image_tag_context();
            let fallback = ImagesState::load(state_dir)
                .lookup_pointer(&pointer_key(
                    &target.repo.name,
                    &target.attr,
                    ctx.as_deref(),
                ))
                .map(|p| p.tag.clone())
                .unwrap_or_else(|| target.tag.clone());
            let store = probe
                .tag_state(&fallback)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            return nix_absent_ladder(target, &fallback, store);
        }
        Err(DrvEvalError::AttrMissing { attr, detail }) => {
            return Err(anyhow::anyhow!(
                "flake {} does not provide attribute '{attr}' (needed by workload '{}'): \
                 {detail}",
                target.repo.flake_root.display(),
                target.name
            ));
        }
        Err(DrvEvalError::EvalFailed { detail }) => {
            return Err(anyhow::anyhow!(
                "nix eval failed for '{}#{}.outPath': {detail}",
                target.repo.flake_root.display(),
                target.attr
            ));
        }
    };
    let tag_ctx = image_tag_context();
    let tag = compute_image_tag(&target.attr, tag_ctx.as_deref(), &out_path)?;
    let key = image_key(&target.repo.name, &tag);

    // Step 2: §3.3 — the per-tag lock spans the probe → eval → build → load
    // → record critical section. `--check` has no critical section
    // (read-only) → no lock.
    let _lock = if check {
        None
    } else {
        Some(ImageTagLock::acquire(state_dir, &key)?)
    };

    // Step 3: store presence INSIDE the lock (§7 concurrent row: everything
    // is re-checked inside). An unreachable store fails fast — the named §7
    // error aborts the command (documented batch posture).
    let store = probe
        .tag_state(&tag)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let state = ImagesState::load(state_dir);

    // The drvPath eval: recorded for provenance/diagnostics (spec §8), no
    // longer the freshness signal (the content-addressed tag is).
    let drv = match eval.eval_drv_path(&target.repo.flake_root, &target.attr) {
        Ok(drv) => drv,
        // Nix vanished between the two evals (or a PATH flip): the §7
        // ladder applies verbatim, with the computed tag already known.
        Err(DrvEvalError::NixAbsent) => return nix_absent_ladder(target, &tag, store),
        Err(DrvEvalError::AttrMissing { attr, detail }) => {
            return Err(anyhow::anyhow!(
                "flake {} does not provide attribute '{attr}' (needed by workload '{}'): \
                 {detail}",
                target.repo.flake_root.display(),
                target.name
            ));
        }
        Err(DrvEvalError::EvalFailed { detail }) => {
            return Err(anyhow::anyhow!(
                "nix eval failed for '{}#{}.drvPath': {detail}",
                target.repo.flake_root.display(),
                target.attr
            ));
        }
    };

    // Content-addressed freshness: a record under the COMPUTED key exists
    // only when exactly this content was built+loaded (or D1-trusted).
    let record_state = record_state_for(state.lookup(&key));
    let decision = decide_skew(record_state, store, force);

    let mut report = TargetReport {
        name: target.name.clone(),
        repo: target.repo.name.clone(),
        attr: target.attr.clone(),
        tag: tag.clone(),
        record_state: record_state_label(record_state).to_string(),
        store_state: store_state_label(store).to_string(),
        drv_path: Some(drv.clone()),
        decision: String::new(),
        action_taken: String::new(),
    };

    match decision {
        SkewDecision::Skip => {
            report.decision = "skip".to_string();
            report.action_taken = "none (up to date)".to_string();
        }
        SkewDecision::TrustAndRecord => {
            if check {
                report.decision = "would trust+record".to_string();
                report.action_taken = "none (--check)".to_string();
            } else {
                report.decision = "trust+record".to_string();
                // USER DECISION D1: the store tag is trusted as-is (records
                // are advisory); write the baseline so the NEXT run skips.
                // A2: the out_path is KNOWN (the step-1 eval) — recording it
                // lets the phase-D re-load gate skip a redundant `msb load`
                // after a later forced rebuild. `digest = None` until the
                // msb digest surface lands (§3.5/§11). The current-pointer
                // moves in the SAME locked critical section (ADR 0032): the
                // trusted tag is what plan/spawn-time resolution should
                // resolve.
                let prov = Provenance::capture();
                let mut state = state;
                state.upsert(
                    key.clone(),
                    ImageRecord {
                        repo: target.repo.clone(),
                        attr: target.attr.clone(),
                        tag: tag.clone(),
                        drv_path: drv.clone(),
                        out_path: out_path.clone(),
                        digest: None,
                        built_at: prov.now.clone(),
                        loaded_at: prov.now.clone(),
                        loader: prov.loader,
                        host: prov.host,
                        user: prov.user,
                    },
                );
                state.upsert_pointer(
                    pointer_key(&target.repo.name, &target.attr, tag_ctx.as_deref()),
                    PointerRecord {
                        tag: tag.clone(),
                        updated_at: prov.now,
                    },
                );
                // INSIDE the lock (§3.3 atomicity).
                state.save(state_dir)?;
                report.action_taken = "recorded baseline (D1 trust)".to_string();
            }
        }
        SkewDecision::Build | SkewDecision::Rebuild | SkewDecision::RebuildForced => {
            let verb = match decision {
                SkewDecision::Build => "build",
                SkewDecision::Rebuild => "rebuild",
                SkewDecision::RebuildForced => "rebuild (forced)",
                other => unreachable!("matched only the three build decisions; got {other:?}"),
            };
            if check {
                // --check never touches the pipeline and never writes.
                report.decision = format!("would {verb}");
                report.action_taken = "none (--check)".to_string();
            } else {
                report.decision = verb.to_string();
                // The phase-D pipeline: nix build → outPath re-load gate →
                // `msb load` → record + pointer upsert. The lock is still
                // held here — the whole critical section runs inside it
                // (spec §3.3).
                let PipelineOutcome { action, .. } = run_build_pipeline(
                    &BuildJob {
                        workload: target.name.clone(),
                        repo: target.repo.clone(),
                        attr: target.attr.clone(),
                        tag: tag.clone(),
                        tag_ctx: tag_ctx.clone(),
                        drv_path: drv,
                        force,
                    },
                    state_dir,
                    builder,
                    loader,
                    probe,
                )
                .await?;
                report.action_taken = match action {
                    LoadAction::Loaded => "built+loaded+recorded".to_string(),
                    // The §3.1 re-load gate skip: exact operator note.
                    LoadAction::AlreadyCurrent => {
                        "image unchanged in store; tag already current".to_string()
                    }
                };
                // A2 stage 2 PRUNE-ON-LOAD (ADR 0032 §Image tags — RESOLVED
                // user decision 3): immediately after a successful pipeline
                // (BOTH LoadAction outcomes — the just-loaded tag is
                // confirmed current either way), INSIDE the still-held
                // per-tag lock, build mode only. The capsule rung rides the
                // target; repo/settings rungs resolve from the live registry
                // (unreadable → defaults, the advisory posture). Cleanup
                // never fails the load: per-tag failures aggregate into one
                // stderr note inside prune_on_load.
                let registry = crate::config::load_registry().ok().flatten();
                let settings_rung = registry.as_ref().and_then(|r| r.settings.image_keep_last);
                let repo_rung = registry
                    .as_ref()
                    .and_then(|r| r.configs.get(&target.repo.name))
                    .and_then(|e| e.image_keep_last);
                let keep_last = gc::resolve_keep_last(settings_rung, repo_rung, target.keep_last)?;
                gc::prune_on_load(
                    state_dir,
                    &target.attr,
                    tag_ctx.as_deref(),
                    keep_last,
                    probe,
                    remover,
                )
                .await?;
            }
        }
    }
    Ok(report)
}

// ---------------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------------

/// Render the report rows to `out` (ps-style stable columns). Pure I/O;
/// `cmd_workload_build` passes stdout, tests pass a `Vec<u8>`.
pub fn print_build_text_to<W: std::io::Write>(
    reports: &[TargetReport],
    out: &mut W,
) -> std::io::Result<()> {
    if reports.is_empty() {
        writeln!(out, "(no nix-layered workloads in scope)")?;
        return Ok(());
    }
    // Stable column layout: NAME | REPO | TAG | RECORD | STORE | DECISION | ACTION
    writeln!(
        out,
        "{:<16} {:<12} {:<28} {:<9} {:<8} {:<24} ACTION",
        "NAME", "REPO", "TAG", "RECORD", "STORE", "DECISION"
    )?;
    for r in reports {
        writeln!(
            out,
            "{:<16} {:<12} {:<28} {:<9} {:<8} {:<24} {}",
            r.name, r.repo, r.tag, r.record_state, r.store_state, r.decision, r.action_taken
        )?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Command entry
// ---------------------------------------------------------------------------

/// `workestrate workload build [name] [--repo <config> | --all-repos]
/// [--check] [--force] [--json]` (spec 21 §5.1).
pub async fn cmd_workload_build(
    name: Option<&str>,
    repo: Option<&str>,
    all_repos: bool,
    check: bool,
    force: bool,
    json: bool,
) -> Result<()> {
    let scope = match (name, repo, all_repos) {
        (Some(n), None, false) => BuildScope::Name(n),
        (None, None, false) => BuildScope::ActiveContext,
        (None, Some(r), false) => BuildScope::Repo(r),
        (None, None, true) => BuildScope::AllRepos,
        // clap's conflicts_with declarations make every other shape
        // unparseable.
        other => unreachable!("clap conflicts guarantee one selector shape; got {other:?}"),
    };
    let single_name = name.is_some();

    let (targets, skips) = resolve_targets(scope)?;
    for skip in &skips {
        // §7 "No flake.nix in the declaring repo": single-name mode HARD
        // ERRORS naming the repo; batch modes skip-with-note and the rest
        // of the batch proceeds.
        if single_name {
            anyhow::bail!("{}", skip.hard_error_message());
        }
        eprintln!("{}", skip.note_message());
    }

    if targets.is_empty() {
        // Zero-eligible no-op (spec §5.1): the EXACT note, exit 0.
        eprintln!("{ZERO_ELIGIBLE_NOTE}");
        if json {
            println!("[]");
        }
        return Ok(());
    }

    let state_dir = crate::config::resolve_state_dir();
    let mut probe = MsbStoreProbe;
    let mut eval = NixCliEvaluator::new();
    let mut builder = NixCliBuilder::new();
    let mut loader = MsbCliLoader::new();
    let mut remover = MsbCliRemover::new();
    let mut reports = Vec::with_capacity(targets.len());
    for target in &targets {
        // Fail-fast: the first hard error (unreachable store, nix-absent with
        // a missing tag, eval failure, or a pipeline stage failure) aborts
        // the command — one named error, not per-workload spam.
        let mut seams = TargetSeams {
            probe: &mut probe,
            eval: &mut eval,
            builder: &mut builder,
            loader: &mut loader,
            remover: &mut remover,
        };
        reports.push(process_target(target, &state_dir, check, force, &mut seams).await?);
    }

    if json {
        // Envelope: a bare ARRAY of per-workload result objects — the same
        // shape for single-name and batch scopes (matches the ps/workloads
        // bare-array convention in json_out.rs; documented there).
        println!(
            "{}",
            serde_json::to_string_pretty(&crate::json_out::build_results_json(&reports))?
        );
    } else {
        print_build_text_to(&reports, &mut std::io::stdout())?;
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
    use super::super::detect::test_fakes::{FakeEvaluator, FakeStoreProbe};
    use super::super::pipeline::test_fakes::{FakeBuilder, FakeLoader, FakeRemover};
    use super::*;
    use crate::config::test_support::unique_state_dir;
    use crate::images::detect::StoreUnreachable;
    use crate::images::state::{images_state_path, IMAGES_STATE_VERSION};

    // ---- fixtures ----

    /// A repo fixture: `<tmp>/checkout` with a flake.nix, registered as
    /// `personal`. Returns (tmp, checkout, registered pairs).
    fn repo_fixture(label: &str) -> (PathBuf, PathBuf, Vec<(String, PathBuf)>) {
        let tmp = unique_state_dir(label);
        let checkout = tmp.join("checkout");
        std::fs::create_dir_all(&checkout).unwrap();
        std::fs::write(checkout.join("flake.nix"), "{}\n").unwrap();
        let registered = vec![("personal".to_string(), checkout.clone())];
        (tmp, checkout, registered)
    }

    fn config_with(workloads: &[(&str, &str)]) -> ConfigFile {
        // (name, recipe) pairs → a minimal ConfigFile.
        let mut config = ConfigFile::default();
        for (name, recipe) in workloads {
            let wl = config.workloads.entry(name.to_string()).or_default();
            wl.kind = "agent".to_string();
            wl.image.recipe = recipe.to_string();
            if *recipe == "nix-layered" {
                wl.image.name = Some(format!("img-{name}"));
            }
        }
        config
    }

    fn provenance_for(names: &[&str], layer: &str) -> MergeProvenance {
        names
            .iter()
            .map(|n| (format!("workloads.{n}.image"), layer.to_string()))
            .collect()
    }

    fn target_fixture(label: &str, name: &str) -> (PathBuf, BuildTarget) {
        let (tmp, checkout, registered) = repo_fixture(label);
        let declaring = checkout.join("workestrate").join("workloads").join(name);
        std::fs::create_dir_all(&declaring).unwrap();
        let repo = repo_identity_for(&declaring, &registered).expect("flake root resolves");
        let attr = format!("img-{name}");
        let target = BuildTarget {
            name: name.to_string(),
            tag: format!("{attr}:latest"),
            attr,
            repo,
            keep_last: None,
        };
        (tmp, target)
    }

    // ---- selector core ----

    /// Bare scope: only nix-layered workloads are selected; registry /
    /// local_build-only workloads are skipped silently (spec §2.3/§5.1).
    #[test]
    fn select_eligible_filters_to_nix_layered_only() {
        let (tmp, checkout, registered) = repo_fixture("select-eligibility");
        let declaring = checkout.join("workestrate").join("workloads");
        std::fs::create_dir_all(&declaring).unwrap();
        let config = config_with(&[
            ("pi", "nix-layered"),
            ("web", "registry"),
            ("tempest", "nix-layered"),
        ]);
        let provenance = provenance_for(&["pi", "web", "tempest"], "personal#workestrate");
        let layer_dirs = HashMap::from([("personal#workestrate".to_string(), declaring)]);

        let (targets, skips) =
            select_eligible(&config, &provenance, &layer_dirs, &registered, None).unwrap();

        let names: Vec<&str> = targets.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(names, vec!["pi", "tempest"], "sorted, nix-layered only");
        assert!(skips.is_empty(), "no flake-root skips in this fixture");
        let pi = &targets[0];
        assert_eq!(pi.attr, "img-pi");
        assert_eq!(pi.tag, "img-pi:latest", "default tag is `latest`");
        assert_eq!(pi.repo.name, "personal", "registered checkout → repo NAME");
        assert_eq!(pi.repo.flake_root, checkout.canonicalize().unwrap());

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// Single-name scope: `only` restricts the selection to the named
    /// workload; a non-nix-layered name selects NOTHING (zero-eligible, not
    /// an error — spec §5.1 skip semantics).
    #[test]
    fn select_eligible_only_name_and_non_nix_layered_name_selects_nothing() {
        let (tmp, checkout, registered) = repo_fixture("select-only");
        let declaring = checkout.join("workestrate").join("workloads");
        std::fs::create_dir_all(&declaring).unwrap();
        let config = config_with(&[("pi", "nix-layered"), ("web", "registry")]);
        let provenance = provenance_for(&["pi", "web"], "personal");
        let layer_dirs = HashMap::from([("personal".to_string(), declaring)]);

        let (targets, _) =
            select_eligible(&config, &provenance, &layer_dirs, &registered, Some("pi")).unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name, "pi");

        let (targets, _) =
            select_eligible(&config, &provenance, &layer_dirs, &registered, Some("web")).unwrap();
        assert!(
            targets.is_empty(),
            "a registry-recipe name is skipped → zero-eligible note path"
        );

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// §7 "No flake.nix in the declaring repo": the workload becomes a
    /// NoFlakeRoot skip that names the repo (batch note / single-name hard
    /// error is the caller's escalation).
    #[test]
    fn select_eligible_no_flake_root_is_a_named_skip() {
        let tmp = unique_state_dir("select-flakeless");
        let declaring = tmp.join("flakeless-repo").join("workestrate");
        std::fs::create_dir_all(&declaring).unwrap();
        let config = config_with(&[("pi", "nix-layered")]);
        let provenance = provenance_for(&["pi"], "adhoc");
        let layer_dirs = HashMap::from([("adhoc".to_string(), declaring.clone())]);

        let (targets, skips) =
            select_eligible(&config, &provenance, &layer_dirs, &[], None).unwrap();
        assert!(targets.is_empty());
        match &skips[..] {
            [SelectSkip::NoFlakeRoot {
                workload,
                repo_key,
                declaring_dir: dir,
            }] => {
                assert_eq!(workload, "pi");
                assert_eq!(
                    repo_key,
                    &declaring
                        .canonicalize()
                        .unwrap()
                        .to_string_lossy()
                        .to_string(),
                    "unregistered → canonical path repo_key (spec §4.3)"
                );
                assert_eq!(dir, &declaring);
            }
            other => panic!("expected one NoFlakeRoot skip, got {other:?}"),
        }

        let _ = std::fs::remove_dir_all(&tmp);
    }

    /// The zero-eligible note is pinned EXACTLY (spec §5.1 stderr contract).
    #[test]
    fn zero_eligible_note_is_byte_identical_to_spec() {
        assert_eq!(
            ZERO_ELIGIBLE_NOTE,
            "note: no nix-layered workloads in scope; nothing to build"
        );
    }

    // ---- --repo / --all-repos selector resolution (env-backed) ----

    /// Write a local-path config repo: flake.nix + a file-mode workestrate.toml
    /// declaring `wl_name` (nix-layered). `default_egress_allow` writes a
    /// policy-gate-violating network (no entitlement) so the repo's standalone
    /// merge FAILS (the --all-repos skip leg).
    fn write_local_repo(
        home: &Path,
        repo: &str,
        wl_name: &str,
        default_egress_allow: bool,
    ) -> PathBuf {
        let dir = home.join("repos").join(repo);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("flake.nix"), "{}\n").unwrap();
        let network = if default_egress_allow {
            "[workloads.wl.network.defaults]\negress = \"allow\"\n"
        } else {
            "[workloads.wl.network.defaults]\negress = \"deny\"\n"
        };
        let toml = format!(
            "schema_version = 1\n\n\
             [workloads.{wl_name}]\nkind = \"agent\"\n\
             image = {{ recipe = \"nix-layered\", name = \"img-{wl_name}\" }}\n\
             command = []\n\n{}",
            network.replace("workloads.wl", &format!("workloads.{wl_name}"))
        );
        std::fs::write(dir.join("workestrate.toml"), toml).unwrap();
        dir
    }

    /// Registry fixture: `layers = []` + local-path `[configs.<repo>]` entries.
    fn write_registry(home: &Path, repos: &[(&str, &Path)]) {
        let mut content = "layers = []\n".to_string();
        for (name, dir) in repos {
            content.push_str(&format!(
                "\n[configs.{name}]\nurl = \"{}\"\n",
                dir.display()
            ));
        }
        std::fs::write(home.join("config.toml"), content).unwrap();
    }

    /// `--repo <name>` resolves that repo's nix-layered workloads against the
    /// repo's OWN layers; an unregistered name and a policy-failing repo are
    /// HARD ERRORS (explicit scope).
    #[test]
    fn repo_scope_resolves_registered_repo_and_hard_errors() {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(
            crate::config::test_support::HOME_ENV_KEYS,
        );
        let home = unique_state_dir("build-repo-scope-home");
        std::fs::create_dir_all(&home).unwrap();
        let good = write_local_repo(&home, "good", "pi", false);
        let bad = write_local_repo(&home, "bad", "evil", true);
        write_registry(&home, &[("good", &good), ("bad", &bad)]);
        std::env::set_var("WORKESTRATE_HOME", &home);

        let (targets, skips) = resolve_targets(BuildScope::Repo("good")).unwrap();
        assert!(skips.is_empty());
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name, "pi");
        assert_eq!(
            targets[0].repo.name, "good",
            "repo_key = the registered name"
        );
        assert_eq!(targets[0].repo.flake_root, good.canonicalize().unwrap());

        let err = resolve_targets(BuildScope::Repo("bad"))
            .expect_err("a repo failing the policy gates hard-errors under explicit --repo");
        assert!(
            format!("{err:#}").contains("default_egress_allow"),
            "the underlying policy gate surfaces in the chain: {err:#}"
        );

        let err = resolve_targets(BuildScope::Repo("nosuch"))
            .expect_err("unregistered repo is a hard error");
        assert!(
            err.to_string().contains("'nosuch' is not registered"),
            "names the repo + remediation: {err}"
        );

        let _ = std::fs::remove_dir_all(&home);
    }

    /// `--all-repos` iterates every registered repo; a repo whose standalone
    /// load/merge fails is SKIPPED WITH A NOTE and the rest of the batch
    /// proceeds (documented batch posture).
    #[test]
    fn all_repos_skips_a_failing_repo_and_proceeds() {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(
            crate::config::test_support::HOME_ENV_KEYS,
        );
        let home = unique_state_dir("build-all-repos-home");
        std::fs::create_dir_all(&home).unwrap();
        let good = write_local_repo(&home, "good", "pi", false);
        let bad = write_local_repo(&home, "bad", "evil", true);
        write_registry(&home, &[("good", &good), ("bad", &bad)]);
        std::env::set_var("WORKESTRATE_HOME", &home);

        let (targets, skips) =
            resolve_targets(BuildScope::AllRepos).expect("a failing repo must NOT fail the batch");
        assert_eq!(targets.len(), 1, "only the good repo contributes");
        assert_eq!(targets[0].name, "pi");
        assert_eq!(targets[0].repo.name, "good");
        assert!(
            skips.is_empty(),
            "repo-level failure is a note, not a skip row"
        );

        let _ = std::fs::remove_dir_all(&home);
    }

    // ---- the per-workload flow (skew-in-lock integration, fake seams) ----

    /// A2 fixtures: well-formed evaluated out_paths and their computed
    /// content-addressed tags (ctx is None in these tests — each flow test
    /// pins `set_active_context(None)` + `clear_inline_override()` under
    /// ENV_TEST_LOCK so no leaked process-global context can flip the tag
    /// to the ctx-carrying form).
    const OUT_A: &str = "/nix/store/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-img-pi.tar.gz";
    const OUT_B: &str = "/nix/store/bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb-img-pi.tar.gz";
    const TAG_A: &str = "img-pi:aaaaaaaaaaaa";
    const TAG_B: &str = "img-pi:bbbbbbbbbbbb";

    /// Pin the A2 tag context to None for the duration of a flow test.
    fn pin_no_tag_context() -> std::sync::MutexGuard<'static, ()> {
        let lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        crate::config::set_active_context(None);
        crate::config::clear_inline_override();
        lock
    }

    /// Fake evaluator queued for one target pass: the A2 out_path eval
    /// (popped FIRST, decides the tag) then the drvPath eval (provenance).
    fn fake_eval(out: &str, drv: &str) -> FakeEvaluator {
        let mut e = FakeEvaluator::new();
        e.push_out_ok(out);
        e.push_ok(drv);
        e
    }

    /// Builder/loader fakes for tests whose flow never reaches the pipeline
    /// (skip / trust / --check / ladder rows): empty queues assert the
    /// pipeline seams are never touched.
    fn no_pipeline() -> (FakeBuilder, FakeLoader) {
        (FakeBuilder::new(), FakeLoader::new())
    }

    /// Bundle the five fakes into the [`TargetSeams`] shape
    /// [`process_target`] takes.
    fn seams<'a>(
        probe: &'a mut FakeStoreProbe,
        eval: &'a mut FakeEvaluator,
        builder: &'a mut FakeBuilder,
        loader: &'a mut FakeLoader,
        remover: &'a mut FakeRemover,
    ) -> TargetSeams<'a, FakeStoreProbe, FakeEvaluator, FakeBuilder, FakeLoader, FakeRemover> {
        TargetSeams {
            probe,
            eval,
            builder,
            loader,
            remover,
        }
    }

    /// D1 trust record: record absent (no record under the COMPUTED tag) +
    /// tag present → TrustAndRecord writes the baseline INSIDE the lock with
    /// the A2 shape (drv_path = current eval, out_path = the EVALUATED
    /// out_path — known since the out-path-first flow, digest = None) and
    /// moves the current-pointer in the same locked section.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime::tests
    async fn d1_trust_writes_baseline_record_inside_lock() -> Result<()> {
        let _guard = pin_no_tag_context();
        let (tmp, target) = target_fixture("flow-d1", "pi");
        let state_dir = unique_state_dir("flow-d1-state");
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present);
        let (mut builder, mut loader) = no_pipeline();

        let report = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(
                &mut probe,
                &mut fake_eval(OUT_A, "drv-A"),
                &mut builder,
                &mut loader,
                &mut FakeRemover::new(),
            ),
        )
        .await?;

        assert_eq!(report.record_state, "absent");
        assert_eq!(report.store_state, "present");
        assert_eq!(report.drv_path.as_deref(), Some("drv-A"));
        assert_eq!(report.decision, "trust+record");
        assert_eq!(report.action_taken, "recorded baseline (D1 trust)");
        assert_eq!(report.tag, TAG_A, "the report names the COMPUTED tag");

        // Record content assertions (spec §8 + the A2 record shape).
        let state = ImagesState::load(&state_dir);
        assert_eq!(state.version, IMAGES_STATE_VERSION);
        let key = image_key("personal", TAG_A);
        let record = state.lookup(&key).expect("baseline record written");
        assert_eq!(record.repo.name, "personal");
        assert_eq!(record.attr, "img-pi");
        assert_eq!(record.tag, TAG_A);
        assert_eq!(record.drv_path, "drv-A", "drv_path = the current eval");
        assert_eq!(
            record.out_path, OUT_A,
            "A2: the trust record carries the EVALUATED out_path (known before the decision)"
        );
        assert_eq!(record.digest, None);
        assert!(
            record.loader.starts_with("workestrate "),
            "loader provenance: {}",
            record.loader
        );
        assert!(!record.host.is_empty() && !record.user.is_empty());
        assert!(
            record.built_at.len() == 20 && record.built_at.ends_with('Z'),
            "rfc3339 provenance timestamps: {}",
            record.built_at
        );
        assert_eq!(record.loaded_at, record.built_at);
        // A2: the current-pointer moved to the trusted tag, same stamp.
        let pointer = state
            .lookup_pointer(&crate::images::state::pointer_key(
                "personal", "img-pi", None,
            ))
            .expect("D1 trust moves the current-pointer");
        assert_eq!(pointer.tag, TAG_A);
        assert_eq!(pointer.updated_at, record.built_at);
        // The lock file is released when the flow returns.
        assert!(
            !crate::images::state::image_locks_dir(&state_dir)
                .join(crate::images::lock::lock_file_name(&key))
                .exists(),
            "lock released after the flow"
        );

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// THE A2 content-addressed skip: the SAME evaluated out_path yields the
    /// SAME computed tag → a record under that key is Fresh → tag Present →
    /// SkewDecision::Skip with NO build invoked — even when the drvPath text
    /// churned (eval churn costs nothing). A CHANGED out_path (new content)
    /// computes a NEW tag → absent record + absent store tag → Build, and
    /// the pipeline loads + records + moves the pointer.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime::tests
    async fn content_addressed_skip_then_content_change_builds() -> Result<()> {
        let _guard = pin_no_tag_context();
        let (tmp, target) = target_fixture("flow-skip", "pi");
        let state_dir = unique_state_dir("flow-skip-state");
        let (mut builder, mut loader) = no_pipeline();
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present);

        let first = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(
                &mut probe,
                &mut fake_eval(OUT_A, "drv-A"),
                &mut builder,
                &mut loader,
                &mut FakeRemover::new(),
            ),
        )
        .await?;
        assert_eq!(first.decision, "trust+record");

        // Same out_path → same tag → fresh record → skip. The drv text
        // churned (drv-B) — irrelevant under content-addressed tags.
        let mut probe2 = FakeStoreProbe::new();
        probe2.push(StoreTag::Present);
        let skip = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(
                &mut probe2,
                &mut fake_eval(OUT_A, "drv-B"),
                &mut builder,
                &mut loader,
                &mut FakeRemover::new(),
            ),
        )
        .await?;
        assert_eq!(skip.record_state, "fresh");
        assert_eq!(skip.decision, "skip");
        assert_eq!(skip.action_taken, "none (up to date)");
        assert!(
            builder.calls.is_empty() && loader.calls.is_empty(),
            "content-addressed skip: no build invoked"
        );

        // Content change → new out_path → NEW tag → absent record + absent
        // store tag → Build → the pipeline runs: build (fake) → pre-gate
        // probe (gone) → msb load (fake) → post-load verification.
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Gone); // skew probe for the NEW tag
        probe.push(StoreTag::Gone); // pipeline pre-gate probe
        probe.push(StoreTag::Present); // post-load verification
        let mut builder = FakeBuilder::new();
        builder.push_ok(OUT_B);
        let mut loader = FakeLoader::new();
        loader.push_ok();
        let report = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(
                &mut probe,
                &mut fake_eval(OUT_B, "drv-C"),
                &mut builder,
                &mut loader,
                &mut FakeRemover::new(),
            ),
        )
        .await?;
        assert_eq!(report.decision, "build");
        assert_eq!(report.action_taken, "built+loaded+recorded");
        assert_eq!(
            loader.calls,
            vec![(PathBuf::from(OUT_B), TAG_B.to_string())]
        );

        // The record carries the phase-D shape under the NEW tag key; the
        // pointer moved to the new tag.
        let state = ImagesState::load(&state_dir);
        let record = state
            .lookup(&image_key("personal", TAG_B))
            .expect("pipeline upserted the record")
            .clone();
        assert_eq!(record.drv_path, "drv-C");
        assert_eq!(record.out_path, OUT_B);
        assert_eq!(record.digest, None);
        assert_eq!(
            state
                .lookup_pointer(&crate::images::state::pointer_key(
                    "personal", "img-pi", None
                ))
                .map(|p| p.tag.as_str()),
            Some(TAG_B),
            "the pointer moved to the freshly-built tag"
        );

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// Record present (under the computed tag) + tag gone → Rebuild (§3.4
    /// row 3): the pipeline's pre-gate probe sees the tag gone and loads
    /// anyway even when the realized outPath MATCHES the recorded one
    /// (out-of-band deletion). Record absent + tag gone → Build (row 5) →
    /// same load path.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime::tests
    async fn tag_gone_rows_load_anyway_and_build_row_records() -> Result<()> {
        let _guard = pin_no_tag_context();
        let (tmp, target) = target_fixture("flow-rows", "pi");
        let state_dir = unique_state_dir("flow-rows-state");

        // Seed a phase-D record under the computed tag (drv-A, realized
        // out_path).
        let key = image_key("personal", TAG_A);
        let mut state = ImagesState::default();
        let prov = Provenance::capture();
        state.upsert(
            key.clone(),
            ImageRecord {
                repo: target.repo.clone(),
                attr: target.attr.clone(),
                tag: TAG_A.to_string(),
                drv_path: "drv-A".to_string(),
                out_path: OUT_A.to_string(),
                digest: None,
                built_at: prov.now.clone(),
                loaded_at: prov.now.clone(),
                loader: prov.loader,
                host: prov.host,
                user: prov.user,
            },
        );
        state.save(&state_dir)?;

        // Row 3: skew probe (gone) → Rebuild; builder realizes the SAME
        // outPath; pre-gate probe (gone) → the gate loads anyway; post-load
        // probe (present).
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Gone);
        probe.push(StoreTag::Gone);
        probe.push(StoreTag::Present);
        let mut builder = FakeBuilder::new();
        builder.push_ok(OUT_A);
        let mut loader = FakeLoader::new();
        loader.push_ok();
        let report = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(
                &mut probe,
                &mut fake_eval(OUT_A, "drv-A"),
                &mut builder,
                &mut loader,
                &mut FakeRemover::new(),
            ),
        )
        .await?;
        assert_eq!(report.decision, "rebuild");
        assert_eq!(report.action_taken, "built+loaded+recorded");
        assert_eq!(loader.calls.len(), 1, "tag gone → load anyway (§3.1 gate)");
        assert_eq!(
            loader.calls[0].1, TAG_A,
            "the load targets the computed tag"
        );

        // Row 5: absent + absent → Build → pipeline loads and records.
        let state_dir2 = unique_state_dir("flow-rows-state2");
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Gone);
        probe.push(StoreTag::Gone);
        probe.push(StoreTag::Present);
        let mut builder = FakeBuilder::new();
        builder.push_ok(OUT_A);
        let mut loader = FakeLoader::new();
        loader.push_ok();
        let report = process_target(
            &target,
            &state_dir2,
            false,
            false,
            &mut seams(
                &mut probe,
                &mut fake_eval(OUT_A, "drv-A"),
                &mut builder,
                &mut loader,
                &mut FakeRemover::new(),
            ),
        )
        .await?;
        assert_eq!(report.decision, "build");
        assert_eq!(report.action_taken, "built+loaded+recorded");
        let record = ImagesState::load(&state_dir2)
            .lookup(&key)
            .expect("Build row writes the record")
            .clone();
        assert_eq!(record.out_path, OUT_A);

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&state_dir);
        let _ = std::fs::remove_dir_all(&state_dir2);
        Ok(())
    }

    /// The §3.1 re-load gate end-to-end through process_target: under A2 a
    /// same-content run is a plain Skip (never reaches the pipeline), so the
    /// gate is exercised via `--reload-images`: force flips the fresh+present
    /// Skip to RebuildForced, the builder realizes the SAME outPath (nix-store
    /// dedup) and the tag is present → `msb load` is SKIPPED and the action is
    /// the exact operator note "image unchanged in store; tag already
    /// current"; the record's drv_path still refreshes.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime::tests
    async fn outpath_gate_skip_reports_tag_already_current() -> Result<()> {
        let _guard = pin_no_tag_context();
        let (tmp, target) = target_fixture("flow-gate", "pi");
        let state_dir = unique_state_dir("flow-gate-state");

        // Seed a phase-D record (drv-A, realized out_path) under the
        // computed tag.
        let key = image_key("personal", TAG_A);
        let mut state = ImagesState::default();
        let prov = Provenance::capture();
        state.upsert(
            key.clone(),
            ImageRecord {
                repo: target.repo.clone(),
                attr: target.attr.clone(),
                tag: TAG_A.to_string(),
                drv_path: "drv-A".to_string(),
                out_path: OUT_A.to_string(),
                digest: None,
                built_at: prov.now.clone(),
                loaded_at: prov.now.clone(),
                loader: prov.loader,
                host: prov.host,
                user: prov.user,
            },
        );
        state.save(&state_dir)?;

        // force → RebuildForced; the builder realizes the SAME outPath; the
        // pre-gate probe sees the tag present → SkipLoad, no loader call.
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present);
        probe.push(StoreTag::Present);
        let mut builder = FakeBuilder::new();
        builder.push_ok(OUT_A);
        let mut loader = FakeLoader::new(); // empty queue: must NOT be called
        let report = process_target(
            &target,
            &state_dir,
            false,
            true,
            &mut seams(
                &mut probe,
                &mut fake_eval(OUT_A, "drv-B"),
                &mut builder,
                &mut loader,
                &mut FakeRemover::new(),
            ),
        )
        .await?;
        assert_eq!(report.decision, "rebuild (forced)");
        assert_eq!(
            report.action_taken, "image unchanged in store; tag already current",
            "the exact §3.1 gate-skip note"
        );
        assert!(loader.calls.is_empty(), "the gate skipped msb load");
        let record = ImagesState::load(&state_dir)
            .lookup(&key)
            .expect("record upserted on the gate-skip path")
            .clone();
        assert_eq!(record.drv_path, "drv-B", "drv_path refreshes");
        assert_eq!(record.out_path, OUT_A);

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// §7 "nix absent + store tag present": degrade with a note, NO record
    /// written. "nix absent + store tag missing": HARD ERROR + remediation.
    /// A2: the out_path eval fails first (no tag computable), so the ladder
    /// probes the best nameable tag — the state-dir current-pointer when one
    /// exists (post-migration homes), else the legacy declared tag.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime::tests
    async fn nix_absent_ladder_degrades_or_hard_errors_per_store_tag() -> Result<()> {
        let _guard = pin_no_tag_context();
        let (tmp, target) = target_fixture("flow-nix-absent", "pi");
        let state_dir = unique_state_dir("flow-nix-absent-state");

        // Legacy fallback (no pointer): tag present → degrade; no record.
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present);
        let mut eval = FakeEvaluator::new();
        eval.push_out_err(DrvEvalError::NixAbsent);
        let (mut builder, mut loader) = no_pipeline();
        let report = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(
                &mut probe,
                &mut eval,
                &mut builder,
                &mut loader,
                &mut FakeRemover::new(),
            ),
        )
        .await?;
        assert_eq!(report.record_state, "unknown");
        assert_eq!(report.drv_path, None);
        assert_eq!(report.decision, "trust (unverified)");
        assert_eq!(report.tag, "img-pi:latest", "legacy declared fallback");
        assert_eq!(
            probe.calls,
            vec!["img-pi:latest"],
            "the ladder probed the legacy declared tag"
        );
        assert!(
            !images_state_path(&state_dir).exists(),
            "the degraded path writes NO record (nothing trustworthy to record)"
        );

        // Tag missing → hard error + remediation (install nix / manual load).
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Gone);
        let mut eval = FakeEvaluator::new();
        eval.push_out_err(DrvEvalError::NixAbsent);
        let err = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(
                &mut probe,
                &mut eval,
                &mut builder,
                &mut loader,
                &mut FakeRemover::new(),
            ),
        )
        .await
        .expect_err("nix absent + tag missing is a hard error (§7)");
        let msg = err.to_string();
        assert!(msg.contains("install nix"), "remediation: {msg}");
        assert!(
            msg.contains("load-images"),
            "config-repo ritual pointer: {msg}"
        );
        assert!(msg.contains("img-pi:latest"), "names the tag: {msg}");

        // Post-migration fallback: a current-pointer exists → the ladder
        // probes/names the POINTER's computed tag, not the legacy one.
        let mut state = ImagesState::default();
        state.upsert_pointer(
            crate::images::state::pointer_key("personal", "img-pi", None),
            PointerRecord {
                tag: TAG_A.to_string(),
                updated_at: "2026-08-24T10:00:00Z".to_string(),
            },
        );
        state.save(&state_dir)?;
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present);
        let mut eval = FakeEvaluator::new();
        eval.push_out_err(DrvEvalError::NixAbsent);
        let report = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(
                &mut probe,
                &mut eval,
                &mut builder,
                &mut loader,
                &mut FakeRemover::new(),
            ),
        )
        .await?;
        assert_eq!(report.tag, TAG_A, "the pointer's tag is the fallback");
        assert_eq!(probe.calls, vec![TAG_A]);
        assert_eq!(report.decision, "trust (unverified)");

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// §7 "msb store unreachable": the named error aborts the flow (the
    /// batch command fails fast on it — documented posture).
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime::tests
    async fn unreachable_store_is_the_named_section7_error() -> Result<()> {
        let _guard = pin_no_tag_context();
        let (tmp, target) = target_fixture("flow-unreachable", "pi");
        let state_dir = unique_state_dir("flow-unreachable-state");
        let mut probe = FakeStoreProbe::new();
        probe.push_unreachable(TAG_A, "io error: not a directory");
        let mut eval = FakeEvaluator::new();
        eval.push_out_ok(OUT_A);
        eval.push_ok("drv-A");
        let (mut builder, mut loader) = no_pipeline();

        let err = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(
                &mut probe,
                &mut eval,
                &mut builder,
                &mut loader,
                &mut FakeRemover::new(),
            ),
        )
        .await
        .expect_err("unreachable store fails the flow");
        let msg = err.to_string();
        assert!(msg.contains("msb image store unreachable"), "{msg}");
        assert!(msg.contains("db unreachable"), "ps.rs vocabulary: {msg}");
        // A2 ordering: the out_path eval ran FIRST (it decides the tag), but
        // the drvPath eval never ran — the store is probed before it.
        assert_eq!(eval.out_calls.len(), 1);
        assert!(eval.calls.is_empty(), "drv eval runs only after the probe");

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// A2 stage 2 GC framing — RECREATE-AFTER-PRUNE (ADR 0032 §Image tags,
    /// RESOLVED decision 3: "a pruned tag = rebuild-from-store on recreate"):
    /// a record + pointer exist for TAG_A but the store tag was pruned
    /// (probe Gone). The skew matrix says Rebuild (row 3), the pipeline
    /// reloads the SAME computed tag, and the current-pointer survives the
    /// recreate INTACT (it already points at TAG_A; stage 4 re-points it to
    /// the same tag). No special case, no error.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime::tests
    async fn recreate_after_prune_rebuilds_and_pointer_survives() -> Result<()> {
        let _guard = pin_no_tag_context();
        let (tmp, target) = target_fixture("flow-recreate", "pi");
        let state_dir = unique_state_dir("flow-recreate-state");

        // Seed the record AND the current-pointer for TAG_A (as if TAG_A had
        // been loaded before and later pruned from the store).
        let key = image_key("personal", TAG_A);
        let mut state = ImagesState::default();
        let prov = Provenance::capture();
        state.upsert(
            key.clone(),
            ImageRecord {
                repo: target.repo.clone(),
                attr: target.attr.clone(),
                tag: TAG_A.to_string(),
                drv_path: "drv-A".to_string(),
                out_path: OUT_A.to_string(),
                digest: None,
                built_at: prov.now.clone(),
                loaded_at: prov.now.clone(),
                loader: prov.loader,
                host: prov.host,
                user: prov.user,
            },
        );
        state.upsert_pointer(
            crate::images::state::pointer_key("personal", "img-pi", None),
            PointerRecord {
                tag: TAG_A.to_string(),
                updated_at: "2026-08-24T09:00:00Z".to_string(),
            },
        );
        state.save(&state_dir)?;

        // Pruned-tag probe: Gone at skew time → Rebuild (§3.4 row 3); gone
        // again at the pipeline pre-gate → load anyway; present post-load.
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Gone);
        probe.push(StoreTag::Gone);
        probe.push(StoreTag::Present);
        let mut builder = FakeBuilder::new();
        builder.push_ok(OUT_A);
        let mut loader = FakeLoader::new();
        loader.push_ok();
        let report = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(
                &mut probe,
                &mut fake_eval(OUT_A, "drv-A"),
                &mut builder,
                &mut loader,
                &mut FakeRemover::new(),
            ),
        )
        .await?;

        assert_eq!(report.decision, "rebuild", "pruned tag → row 3 rebuild");
        assert_eq!(report.action_taken, "built+loaded+recorded");
        assert_eq!(loader.calls.len(), 1, "the pruned tag is re-loaded");
        assert_eq!(loader.calls[0].1, TAG_A, "recreate targets the SAME tag");

        // The pointer survived the recreate — still TAG_A, never dangled.
        let state = ImagesState::load(&state_dir);
        assert_eq!(
            state
                .lookup_pointer(&crate::images::state::pointer_key(
                    "personal", "img-pi", None
                ))
                .map(|p| p.tag.as_str()),
            Some(TAG_A),
            "the current-pointer is intact after the prune-recreate"
        );
        assert!(state.lookup(&key).is_some(), "the record is re-established");

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// --check: NO lock file, NO state file, structured "would …" decisions,
    /// and the D-seam is never reached even when a rebuild is due.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime::tests
    async fn check_mode_is_read_only_and_reports_would_verdicts() -> Result<()> {
        let _guard = pin_no_tag_context();
        let (tmp, target) = target_fixture("flow-check", "pi");
        let state_dir = unique_state_dir("flow-check-state");

        // Seed a record via a prior build-mode run (trust baseline).
        let mut probe0 = FakeStoreProbe::new();
        probe0.push(StoreTag::Present);
        let (mut builder, mut loader) = no_pipeline();
        process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(
                &mut probe0,
                &mut fake_eval(OUT_A, "drv-A"),
                &mut builder,
                &mut loader,
                &mut FakeRemover::new(),
            ),
        )
        .await?;
        let state_before = std::fs::read_to_string(images_state_path(&state_dir)).unwrap();

        // Record present (under the computed tag) + tag GONE →
        // "would rebuild" (no seam error in --check).
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Gone);
        let report = process_target(
            &target,
            &state_dir,
            true,
            false,
            &mut seams(
                &mut probe,
                &mut fake_eval(OUT_A, "drv-A"),
                &mut builder,
                &mut loader,
                &mut FakeRemover::new(),
            ),
        )
        .await?;
        assert_eq!(report.record_state, "fresh");
        assert_eq!(report.decision, "would rebuild");
        assert_eq!(report.action_taken, "none (--check)");
        assert_eq!(
            std::fs::read_to_string(images_state_path(&state_dir)).unwrap(),
            state_before,
            "--check writes nothing"
        );

        // Absent + present → "would trust+record"; --force on fresh+present
        // → "would rebuild (forced)".
        let state_dir2 = unique_state_dir("flow-check-state2");
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present);
        let report = process_target(
            &target,
            &state_dir2,
            true,
            false,
            &mut seams(
                &mut probe,
                &mut fake_eval(OUT_A, "drv-A"),
                &mut builder,
                &mut loader,
                &mut FakeRemover::new(),
            ),
        )
        .await?;
        assert_eq!(report.decision, "would trust+record");
        assert!(
            !state_dir2.join("image-locks").exists() && !images_state_path(&state_dir2).exists(),
            "--check creates neither the lock dir nor the state file"
        );

        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present);
        let report = process_target(
            &target,
            &state_dir,
            true,
            true,
            &mut seams(
                &mut probe,
                &mut fake_eval(OUT_A, "drv-A"),
                &mut builder,
                &mut loader,
                &mut FakeRemover::new(),
            ),
        )
        .await?;
        assert_eq!(
            report.decision, "would rebuild (forced)",
            "--check --force shows the flip"
        );

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&state_dir);
        let _ = std::fs::remove_dir_all(&state_dir2);
        Ok(())
    }

    /// The unreachable error type stays constructible for the JSON/human
    /// surfaces (fields are pub by design).
    #[test]
    fn store_unreachable_fields_are_public() {
        let e = StoreUnreachable {
            tag: "t".to_string(),
            source: "s".to_string(),
        };
        assert_eq!((e.tag.as_str(), e.source.as_str()), ("t", "s"));
    }

    // ---- output renderers ----

    fn sample_report() -> TargetReport {
        TargetReport {
            name: "pi".to_string(),
            repo: "personal".to_string(),
            attr: "workestrate-pi".to_string(),
            tag: "workestrate-pi:latest".to_string(),
            record_state: "absent".to_string(),
            store_state: "present".to_string(),
            drv_path: Some("/nix/store/abc-workestrate-pi.tar.gz.drv".to_string()),
            decision: "would trust+record".to_string(),
            action_taken: "none (--check)".to_string(),
        }
    }

    /// Human table: ps-style stable columns, one row per workload.
    #[test]
    fn human_renderer_prints_stable_columns() {
        let mut out: Vec<u8> = Vec::new();
        print_build_text_to(&[sample_report()], &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(
            text.contains("NAME")
                && text.contains("REPO")
                && text.contains("RECORD")
                && text.contains("STORE")
                && text.contains("DECISION")
                && text.contains("ACTION"),
            "header row: {text}"
        );
        assert!(text.contains("pi"));
        assert!(text.contains("personal"));
        assert!(text.contains("workestrate-pi:latest"));
        assert!(text.contains("would trust+record"));
    }

    /// JSON envelope: a bare array of per-workload objects with exactly the
    /// spec'd fields (single-name scope is the same array of one).
    #[test]
    fn json_envelope_is_a_bare_array_with_the_spec_fields() {
        let json = crate::json_out::build_results_json(&[sample_report()]);
        let value = serde_json::to_value(&json).unwrap();
        let array = value.as_array().expect("bare array envelope");
        assert_eq!(array.len(), 1);
        let obj = array[0].as_object().unwrap();
        let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "action_taken",
                "attr",
                "decision",
                "drv_path",
                "name",
                "record_state",
                "repo",
                "store_state",
                "tag"
            ],
            "exactly the spec §5.1 phase-C fields"
        );
        assert_eq!(obj["name"], "pi");
        assert_eq!(obj["repo"], "personal");
        assert_eq!(obj["attr"], "workestrate-pi");
        assert_eq!(obj["tag"], "workestrate-pi:latest");
        assert_eq!(obj["record_state"], "absent");
        assert_eq!(obj["store_state"], "present");
        assert_eq!(obj["decision"], "would trust+record");
        assert_eq!(obj["action_taken"], "none (--check)");
        // drv_path is null (not absent) when unknown.
        let mut unknown = sample_report();
        unknown.drv_path = None;
        let value = serde_json::to_value(crate::json_out::build_results_json(&[unknown])).unwrap();
        assert!(value[0]["drv_path"].is_null());
    }
}
