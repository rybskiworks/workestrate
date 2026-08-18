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
//! - **D1 trust record shape:** `drv_path` = the current eval; `out_path =
//!   ""` (unknown until phase D realizes the drv — documented at the write
//!   site); `digest = None`. Freshness is the drvPath-only predicate
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
use crate::images::lock::ImageTagLock;
use crate::images::pipeline::{
    run_build_pipeline, BuildJob, ImageBuilder, ImageLoader, LoadAction, MsbCliLoader,
    NixCliBuilder, PipelineOutcome,
};
use crate::images::repo_key::{registered_repo_checkouts, repo_identity_for, repo_key_for};
use crate::images::skew::{decide_skew, RecordState, SkewDecision, StoreTag};
use crate::images::state::{image_key, ImageRecord, ImagesState, Provenance, RepoIdentity};
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
    /// Stable verbatim `<image.name>:<tag|latest>` (USER DECISION D2).
    pub tag: String,
    /// Config-repo identity (repo_key rule + flake_root) for the record key.
    pub repo: RepoIdentity,
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

/// The four seam implementations the flow drives, bundled so
/// [`process_target`] stays readable (and within the argument-count lint):
/// the two phase-C change-detection seams (`detect.rs`) plus the two
/// phase-D process seams (`pipeline.rs`).
pub struct TargetSeams<'a, P: StoreProbe, E: DrvEvaluator, B: ImageBuilder, L: ImageLoader> {
    pub probe: &'a mut P,
    pub eval: &'a mut E,
    pub builder: &'a mut B,
    pub loader: &'a mut L,
}

/// The per-workload flow (spec §3.3/§3.4/§7), generic over the four seams so
/// tests drive fakes with a temp state dir:
///
/// 1. acquire the per-tag lock (build mode only — `--check` takes NO lock);
/// 2. INSIDE the lock: probe the store tag, load `images.json`, run the
///    drvPath eval, derive the record state (drvPath-only, §3.1), and
///    `decide_skew` (re-checked inside the lock per the §7 concurrent row);
/// 3. act: Skip → report; TrustAndRecord → write the D1 baseline record
///    (upsert + save INSIDE the lock); Build/Rebuild/RebuildForced → the
///    phase-D pipeline (nix build → outPath gate → `msb load` → record),
///    still inside the lock; `--check` → structured "would …" report, never
///    the pipeline, never a write.
pub async fn process_target<P: StoreProbe, E: DrvEvaluator, B: ImageBuilder, L: ImageLoader>(
    target: &BuildTarget,
    state_dir: &Path,
    check: bool,
    force: bool,
    seams: &mut TargetSeams<'_, P, E, B, L>,
) -> Result<TargetReport> {
    let TargetSeams {
        probe,
        eval,
        builder,
        loader,
    } = seams;
    // Reborrow the &mut fields so the generic seam bounds (P: StoreProbe
    // etc.) are satisfied by &mut P directly, not &mut &mut P.
    let probe = &mut **probe;
    let eval = &mut **eval;
    let builder = &mut **builder;
    let loader = &mut **loader;
    let key = image_key(&target.repo.name, &target.tag);
    // §3.3: the per-tag lock spans the eval → build → load → record critical
    // section. `--check` has no critical section (read-only) → no lock.
    let _lock = if check {
        None
    } else {
        Some(ImageTagLock::acquire(state_dir, &key)?)
    };

    // Store presence INSIDE the lock (§7 concurrent row: everything is
    // re-checked inside). An unreachable store fails fast — the named §7
    // error aborts the command (documented batch posture).
    let store = probe
        .tag_state(&target.tag)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    let state = ImagesState::load(state_dir);

    let drv = match eval.eval_drv_path(&target.repo.flake_root, &target.attr) {
        Ok(drv) => drv,
        // §7 "nix absent from PATH" ladder, literally, in BOTH modes.
        Err(DrvEvalError::NixAbsent) => {
            return match store {
                StoreTag::Present => {
                    // Degrade, don't block: the tag may be fresh; nothing is
                    // verifiable without nix. NO record is written — nothing
                    // trustworthy to record (documented phase-C decision).
                    eprintln!(
                        "note: nix not found on PATH; cannot verify freshness of '{}' — \
                         proceeding with the store tag as-is; no record written (spec 21 §7)",
                        target.tag
                    );
                    Ok(TargetReport {
                        name: target.name.clone(),
                        repo: target.repo.name.clone(),
                        attr: target.attr.clone(),
                        tag: target.tag.clone(),
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
                    target.tag
                )),
            };
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
                "nix eval failed for '{}#{}.drvPath': {detail}",
                target.repo.flake_root.display(),
                target.attr
            ));
        }
    };

    // drvPath is THE §3.1 signal in phase C; the outPath re-load gate is
    // phase D.
    let record_state = record_state_for(state.lookup(&key), &drv);
    let decision = decide_skew(record_state, store, force);

    let mut report = TargetReport {
        name: target.name.clone(),
        repo: target.repo.name.clone(),
        attr: target.attr.clone(),
        tag: target.tag.clone(),
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
                // are advisory); write the baseline so the NEXT run has a
                // drvPath to compare. `out_path = ""` — unknown until the
                // phase-D pipeline realizes the drv (the drvPath-only
                // freshness predicate never consults it). `digest = None`
                // until the msb digest surface lands (§3.5/§11).
                let prov = Provenance::capture();
                let mut state = state;
                state.upsert(
                    key.clone(),
                    ImageRecord {
                        repo: target.repo.clone(),
                        attr: target.attr.clone(),
                        tag: target.tag.clone(),
                        drv_path: drv.clone(),
                        out_path: String::new(),
                        digest: None,
                        built_at: prov.now.clone(),
                        loaded_at: prov.now.clone(),
                        loader: prov.loader,
                        host: prov.host,
                        user: prov.user,
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
                // `msb load` → record upsert. The lock is still held here —
                // the whole critical section runs inside it (spec §3.3).
                let PipelineOutcome { action, .. } = run_build_pipeline(
                    &BuildJob {
                        workload: target.name.clone(),
                        repo: target.repo.clone(),
                        attr: target.attr.clone(),
                        tag: target.tag.clone(),
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
                    // The §3.1 re-load gate skip: exact operator-facing note.
                    LoadAction::AlreadyCurrent => {
                        "image unchanged in store; tag already current".to_string()
                    }
                };
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
    use super::super::pipeline::test_fakes::{FakeBuilder, FakeLoader};
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
    /// declaring `wl_name` (nix-layered). `default_deny_false` writes a
    /// policy-gate-violating network (no entitlement) so the repo's standalone
    /// merge FAILS (the --all-repos skip leg).
    fn write_local_repo(
        home: &Path,
        repo: &str,
        wl_name: &str,
        default_deny_false: bool,
    ) -> PathBuf {
        let dir = home.join("repos").join(repo);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("flake.nix"), "{}\n").unwrap();
        let network = if default_deny_false {
            "[workloads.wl.network]\ndefault_deny = false\n"
        } else {
            "[workloads.wl.network]\ndefault_deny = true\n"
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
            format!("{err:#}").contains("default_deny_false"),
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

    fn fake_eval_with(drv: &str) -> FakeEvaluator {
        let mut e = FakeEvaluator::new();
        e.push_ok(drv);
        e
    }

    /// Builder/loader fakes for tests whose flow never reaches the pipeline
    /// (skip / trust / --check / ladder rows): empty queues assert the
    /// pipeline seams are never touched.
    fn no_pipeline() -> (FakeBuilder, FakeLoader) {
        (FakeBuilder::new(), FakeLoader::new())
    }

    /// Bundle the four fakes into the [`TargetSeams`] shape
    /// [`process_target`] takes.
    fn seams<'a>(
        probe: &'a mut FakeStoreProbe,
        eval: &'a mut FakeEvaluator,
        builder: &'a mut FakeBuilder,
        loader: &'a mut FakeLoader,
    ) -> TargetSeams<'a, FakeStoreProbe, FakeEvaluator, FakeBuilder, FakeLoader> {
        TargetSeams {
            probe,
            eval,
            builder,
            loader,
        }
    }

    /// D1 trust record: record absent + tag present → TrustAndRecord writes
    /// the baseline INSIDE the lock, with the phase-C record shape (drv_path
    /// = current eval, out_path = "", digest = None, provenance fields).
    #[tokio::test]
    async fn d1_trust_writes_baseline_record_inside_lock() -> Result<()> {
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
                &mut fake_eval_with("drv-A"),
                &mut builder,
                &mut loader,
            ),
        )
        .await?;

        assert_eq!(report.record_state, "absent");
        assert_eq!(report.store_state, "present");
        assert_eq!(report.drv_path.as_deref(), Some("drv-A"));
        assert_eq!(report.decision, "trust+record");
        assert_eq!(report.action_taken, "recorded baseline (D1 trust)");

        // Record content assertions (spec §8 + the phase-C shape decisions).
        let state = ImagesState::load(&state_dir);
        assert_eq!(state.version, IMAGES_STATE_VERSION);
        let key = image_key("personal", "img-pi:latest");
        let record = state.lookup(&key).expect("baseline record written");
        assert_eq!(record.repo.name, "personal");
        assert_eq!(record.attr, "img-pi");
        assert_eq!(record.tag, "img-pi:latest");
        assert_eq!(record.drv_path, "drv-A", "drv_path = the current eval");
        assert_eq!(
            record.out_path, "",
            "out_path stays empty until phase D realizes the drv"
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

    /// Fresh record + tag present → Skip; a drvPath change flips the SAME
    /// setup to Rebuild, which runs the real pipeline over fakes: nix build
    /// (fake) → gate (record out_path="" → load) → msb load (fake) → record
    /// upsert with the realized outPath.
    #[tokio::test]
    async fn skip_then_drv_drift_rebuilds_through_the_pipeline() -> Result<()> {
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
                &mut fake_eval_with("drv-A"),
                &mut builder,
                &mut loader,
            ),
        )
        .await?;
        assert_eq!(first.decision, "trust+record");

        // Same drv → fresh → skip.
        let mut probe2 = FakeStoreProbe::new();
        probe2.push(StoreTag::Present);
        let skip = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(
                &mut probe2,
                &mut fake_eval_with("drv-A"),
                &mut builder,
                &mut loader,
            ),
        )
        .await?;
        assert_eq!(skip.record_state, "fresh");
        assert_eq!(skip.decision, "skip");
        assert_eq!(skip.action_taken, "none (up to date)");

        // drvPath drift → stale + present → Rebuild → the pipeline runs:
        // skew probe (present), pre-gate probe (present — record out_path=""
        // so the gate loads anyway), post-load verification (present).
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present);
        probe.push(StoreTag::Present);
        probe.push(StoreTag::Present);
        let mut builder = FakeBuilder::new();
        builder.push_ok("/nix/store/out-B-img.tar.gz");
        let mut loader = FakeLoader::new();
        loader.push_ok();
        let report = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(
                &mut probe,
                &mut fake_eval_with("drv-B"),
                &mut builder,
                &mut loader,
            ),
        )
        .await?;
        assert_eq!(report.decision, "rebuild");
        assert_eq!(report.action_taken, "built+loaded+recorded");
        assert_eq!(
            loader.calls,
            vec![(
                PathBuf::from("/nix/store/out-B-img.tar.gz"),
                "img-pi:latest".to_string()
            )]
        );

        // The record now carries the phase-D shape: realized out_path.
        let key = image_key("personal", "img-pi:latest");
        let record = ImagesState::load(&state_dir)
            .lookup(&key)
            .expect("pipeline upserted the record")
            .clone();
        assert_eq!(record.drv_path, "drv-B");
        assert_eq!(record.out_path, "/nix/store/out-B-img.tar.gz");
        assert_eq!(record.digest, None);

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// Record present + tag gone → Rebuild (§3.4 row 3): the pipeline's
    /// pre-gate probe sees the tag gone and loads anyway even when the
    /// realized outPath MATCHES the recorded one (out-of-band deletion).
    /// Record absent + tag gone → Build (row 5) → same load path.
    #[tokio::test]
    async fn tag_gone_rows_load_anyway_and_build_row_records() -> Result<()> {
        let (tmp, target) = target_fixture("flow-rows", "pi");
        let state_dir = unique_state_dir("flow-rows-state");

        // Seed a record so row 3 applies, with a phase-D out_path.
        let key = image_key("personal", "img-pi:latest");
        let mut state = ImagesState::default();
        let prov = Provenance::capture();
        state.upsert(
            key.clone(),
            ImageRecord {
                repo: target.repo.clone(),
                attr: target.attr.clone(),
                tag: target.tag.clone(),
                drv_path: "drv-A".to_string(),
                out_path: "/nix/store/out-A-img.tar.gz".to_string(),
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
        builder.push_ok("/nix/store/out-A-img.tar.gz");
        let mut loader = FakeLoader::new();
        loader.push_ok();
        let report = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(
                &mut probe,
                &mut fake_eval_with("drv-A"),
                &mut builder,
                &mut loader,
            ),
        )
        .await?;
        assert_eq!(report.decision, "rebuild");
        assert_eq!(report.action_taken, "built+loaded+recorded");
        assert_eq!(loader.calls.len(), 1, "tag gone → load anyway (§3.1 gate)");

        // Row 5: absent + absent → Build → pipeline loads and records.
        let state_dir2 = unique_state_dir("flow-rows-state2");
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Gone);
        probe.push(StoreTag::Gone);
        probe.push(StoreTag::Present);
        let mut builder = FakeBuilder::new();
        builder.push_ok("/nix/store/out-A-img.tar.gz");
        let mut loader = FakeLoader::new();
        loader.push_ok();
        let report = process_target(
            &target,
            &state_dir2,
            false,
            false,
            &mut seams(
                &mut probe,
                &mut fake_eval_with("drv-A"),
                &mut builder,
                &mut loader,
            ),
        )
        .await?;
        assert_eq!(report.decision, "build");
        assert_eq!(report.action_taken, "built+loaded+recorded");
        let record = ImagesState::load(&state_dir2)
            .lookup(&key)
            .expect("Build row writes the record")
            .clone();
        assert_eq!(record.out_path, "/nix/store/out-A-img.tar.gz");

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&state_dir);
        let _ = std::fs::remove_dir_all(&state_dir2);
        Ok(())
    }

    /// The §3.1 re-load gate end-to-end through process_target: drvPath
    /// drift forces a Rebuild, but the realized outPath MATCHES the recorded
    /// one and the tag is present → `msb load` is SKIPPED and the action is
    /// the exact operator note "image unchanged in store; tag already
    /// current"; the record's drv_path still refreshes.
    #[tokio::test]
    async fn outpath_gate_skip_reports_tag_already_current() -> Result<()> {
        let (tmp, target) = target_fixture("flow-gate", "pi");
        let state_dir = unique_state_dir("flow-gate-state");

        // Seed a phase-D record (drv-A, realized out_path).
        let key = image_key("personal", "img-pi:latest");
        let mut state = ImagesState::default();
        let prov = Provenance::capture();
        state.upsert(
            key.clone(),
            ImageRecord {
                repo: target.repo.clone(),
                attr: target.attr.clone(),
                tag: target.tag.clone(),
                drv_path: "drv-A".to_string(),
                out_path: "/nix/store/out-A-img.tar.gz".to_string(),
                digest: None,
                built_at: prov.now.clone(),
                loaded_at: prov.now.clone(),
                loader: prov.loader,
                host: prov.host,
                user: prov.user,
            },
        );
        state.save(&state_dir)?;

        // drv drift (drv-B) → stale + present → Rebuild; the builder
        // realizes the SAME outPath (eval churn, nix-store dedup); the
        // pre-gate probe sees the tag present → SkipLoad, no loader call.
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present);
        probe.push(StoreTag::Present);
        let mut builder = FakeBuilder::new();
        builder.push_ok("/nix/store/out-A-img.tar.gz");
        let mut loader = FakeLoader::new(); // empty queue: must NOT be called
        let report = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(
                &mut probe,
                &mut fake_eval_with("drv-B"),
                &mut builder,
                &mut loader,
            ),
        )
        .await?;
        assert_eq!(report.decision, "rebuild");
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
        assert_eq!(record.out_path, "/nix/store/out-A-img.tar.gz");

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// §7 "nix absent + store tag present": degrade with a note, NO record
    /// written. "nix absent + store tag missing": HARD ERROR + remediation.
    #[tokio::test]
    async fn nix_absent_ladder_degrades_or_hard_errors_per_store_tag() -> Result<()> {
        let (tmp, target) = target_fixture("flow-nix-absent", "pi");
        let state_dir = unique_state_dir("flow-nix-absent-state");

        // Tag present → degrade; no record.
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present);
        let mut eval = FakeEvaluator::new();
        eval.push_err(DrvEvalError::NixAbsent);
        let (mut builder, mut loader) = no_pipeline();
        let report = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(&mut probe, &mut eval, &mut builder, &mut loader),
        )
        .await?;
        assert_eq!(report.record_state, "unknown");
        assert_eq!(report.drv_path, None);
        assert_eq!(report.decision, "trust (unverified)");
        assert!(
            !images_state_path(&state_dir).exists(),
            "the degraded path writes NO record (nothing trustworthy to record)"
        );

        // Tag missing → hard error + remediation (install nix / manual load).
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Gone);
        let mut eval = FakeEvaluator::new();
        eval.push_err(DrvEvalError::NixAbsent);
        let err = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(&mut probe, &mut eval, &mut builder, &mut loader),
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

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// §7 "msb store unreachable": the named error aborts the flow (the
    /// batch command fails fast on it — documented posture).
    #[tokio::test]
    async fn unreachable_store_is_the_named_section7_error() -> Result<()> {
        let (tmp, target) = target_fixture("flow-unreachable", "pi");
        let state_dir = unique_state_dir("flow-unreachable-state");
        let mut probe = FakeStoreProbe::new();
        probe.push_unreachable("img-pi:latest", "io error: not a directory");
        let mut eval = FakeEvaluator::new();
        eval.push_ok("drv-A");
        let (mut builder, mut loader) = no_pipeline();

        let err = process_target(
            &target,
            &state_dir,
            false,
            false,
            &mut seams(&mut probe, &mut eval, &mut builder, &mut loader),
        )
        .await
        .expect_err("unreachable store fails the flow");
        let msg = err.to_string();
        assert!(msg.contains("msb image store unreachable"), "{msg}");
        assert!(msg.contains("db unreachable"), "ps.rs vocabulary: {msg}");
        // The eval never ran — the store is probed first.
        assert!(eval.calls.is_empty());

        let _ = std::fs::remove_dir_all(&tmp);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// --check: NO lock file, NO state file, structured "would …" decisions,
    /// and the D-seam is never reached even when a rebuild is due.
    #[tokio::test]
    async fn check_mode_is_read_only_and_reports_would_verdicts() -> Result<()> {
        let (tmp, target) = target_fixture("flow-check", "pi");
        let state_dir = unique_state_dir("flow-check-state");

        // Stale + present → "would rebuild" (no seam error in --check).
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present);
        let mut eval = FakeEvaluator::new();
        eval.push_ok("drv-B");
        // Seed a record with a DIFFERENT drv via a prior build-mode run.
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
                &mut fake_eval_with("drv-A"),
                &mut builder,
                &mut loader,
            ),
        )
        .await?;
        let state_before = std::fs::read_to_string(images_state_path(&state_dir)).unwrap();

        let report = process_target(
            &target,
            &state_dir,
            true,
            false,
            &mut seams(&mut probe, &mut eval, &mut builder, &mut loader),
        )
        .await?;
        assert_eq!(report.record_state, "stale");
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
                &mut fake_eval_with("drv-A"),
                &mut builder,
                &mut loader,
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
                &mut fake_eval_with("drv-A"),
                &mut builder,
                &mut loader,
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
