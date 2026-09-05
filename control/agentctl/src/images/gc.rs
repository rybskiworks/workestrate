//! Keep-last-N GC cascade for nix-layered image tags (ADR 0032 §Image tags
//! — RESOLVED user decision 3, A2 stage 2).
//!
//! Computed content-addressed tags accumulate per `(name, ctx)` group as
//! builds land; this module owns the retention policy and both enforcement
//! points:
//!
//! - **Prune-on-load** ([`prune_on_load`], called from
//!   `build_cmd::process_target` immediately AFTER a successful
//!   `run_build_pipeline`, INSIDE the still-held per-tag lock, build mode
//!   only): the just-loaded tag counts toward N and is always retained;
//!   older tags in ITS OWN `(name, ctx)` group beyond the resolved N are
//!   removed from the msb store and dropped from the state file.
//! - **Manual sweep** ([`cmd_images_gc`] — `workestrate images gc`): sweeps
//!   ALL groups in the state file. The manual sweep resolves the cascade
//!   WITHOUT the capsule rung (documented scope asymmetry: the sweep
//!   operates on state-dir groups, not invocations — the capsule
//!   `image.keep_last` rung is enforced at load time by prune-on-load).
//!
//! Cascade (first Some wins, scanning capsule → repo → settings → default):
//! built-in [`DEFAULT_IMAGE_KEEP_LAST`] < home settings
//! (`RegistrySettings.image_keep_last`) < config-repo entry
//! (`ConfigRepoEntry.image_keep_last`) < workload capsule
//! (`ImageSpec.keep_last`). ANY explicitly-provided `0` is a hard error
//! ([`resolve_keep_last`]): N >= 1 because the just-loaded/current tag
//! always counts toward N and is always retained.
//!
//! **Running sandboxes are never affected** (RESOLVED decision 3): the
//! protection set is every `image_tag` recorded across ALL port-registry
//! records (`${state_dir}/var/run/*.json`). Conservative by design — stale
//! records over-protect until unregistered at teardown, and a pruned tag is
//! simply rebuilt-from-store on recreate (the §3.4 row-3 skew path).
//! Legacy records without the field parse as `None` → unprotected, but
//! their tags are legacy-shape and never GC candidates anyway.
//!
//! **Candidates are shape-checked** ([`split_computed_tag`]): only tags
//! parsing as `<name>:<sha>` / `<name>:<ctx>.<sha>` with a 12-char
//! lowercase-alphanumeric sha segment are ever touched. Legacy declared
//! tags (e.g. `img-pi:latest`) NEVER parse → never candidates, never
//! removed. Migration shape is TOLERATE: legacy records stay under legacy
//! keys indefinitely; nothing is rewritten on read.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use anyhow::Result;

use crate::images::detect::{MsbStoreProbe, StoreProbe};
use crate::images::lock::ImageTagLock;
use crate::images::pipeline::{ImageRemover, MsbCliRemover};
use crate::images::state::{ImagesState, OUT_PATH_HASH_PREFIX_LEN, PointerRecord};

/// Built-in keep-last-N default (ADR 0032 §Image tags — RESOLVED user
/// decision 3): the lowest rung of the cascade, used when neither the home
/// settings, the config-repo entry, nor the workload capsule configure N.
pub const DEFAULT_IMAGE_KEEP_LAST: u32 = 5;

/// The cascade resolver (ADR 0032 §Image tags — RESOLVED user decision 3;
/// pure): first `Some` wins scanning **capsule → repo → settings →
/// default**. ANY explicitly-provided value of `0` is a HARD ERROR naming
/// the field and its locus — `keep_last >= 1` because the just-loaded /
/// current tag always counts toward N and is always retained.
pub fn resolve_keep_last(
    settings: Option<u32>,
    repo_entry: Option<u32>,
    capsule: Option<u32>,
) -> Result<u32> {
    const CAPSULE_LOCUS: &str = "workloads.<name>.image.keep_last (workload capsule)";
    const REPO_LOCUS: &str = "configs[<repo>].image_keep_last (config-repo entry)";
    const SETTINGS_LOCUS: &str = "settings.image_keep_last (registry [settings])";
    for (value, locus) in [
        (capsule, CAPSULE_LOCUS),
        (repo_entry, REPO_LOCUS),
        (settings, SETTINGS_LOCUS),
    ] {
        if let Some(n) = value {
            if n == 0 {
                anyhow::bail!(
                    "invalid keep_last = 0 in {locus}: keep_last must be >= 1 \
                     (ADR 0032 §Image tags — the just-loaded/current tag always \
                     counts toward N and is always retained)"
                );
            }
            return Ok(n);
        }
    }
    Ok(DEFAULT_IMAGE_KEEP_LAST)
}

/// Split a COMPUTED content-addressed tag into `(name, ctx)` (ADR 0032
/// §Image tags, AMENDED 2026-08-28): accepts ONLY the shapes
/// `compute_image_tag` emits — `<name>:<sha>` (ctx `None`) or
/// `<name>:<ctx>.<sha>` (dot separator — the pre-amendment two-colon
/// `<name>:<ctx>:<sha>` form is an invalid docker/OCI reference and never
/// parses) — where `sha` is exactly [`OUT_PATH_HASH_PREFIX_LEN`]
/// lowercase-alphanumeric chars. The name splits at the FIRST `:` (dots in
/// a name are legal and harmless); the ctx/sha split is at the LAST `.` of
/// the remainder, unambiguous because neither ctx (slugified — dots
/// collapse to dashes) nor sha (base32) ever contains a dot. Anything else
/// (legacy declared tags like `img-pi:latest`, two-colon tags, dotted ctx
/// segments, empty segments, wrong-length or uppercase shas) returns
/// `None`: legacy tags are never GC candidates and never touched.
pub fn split_computed_tag(tag: &str) -> Option<(String, Option<String>)> {
    let (name, rest) = tag.split_once(':')?;
    // Exactly one colon: any second colon is the pre-amendment two-colon
    // shape (or other garbage) — invalid, never a candidate.
    if name.is_empty() || rest.contains(':') {
        return None;
    }
    let (ctx, sha) = match rest.rsplit_once('.') {
        Some((c, s)) => (Some(c), s),
        None => (None, rest),
    };
    // A dotted ctx can never be emitted (slugify strips dots) — reject it
    // rather than guess at a split; empty ctx likewise.
    if ctx.is_some_and(|c| c.is_empty() || c.contains('.')) || !is_sha_segment(sha) {
        return None;
    }
    Some((name.to_string(), ctx.map(str::to_string)))
}

/// The sha-segment predicate: exactly [`OUT_PATH_HASH_PREFIX_LEN`] chars,
/// all ASCII lowercase alphanumeric (nix base32 alphabet — NEVER a dot, so
/// the `<ctx>.<sha>` split at the LAST `.` is unambiguous).
fn is_sha_segment(s: &str) -> bool {
    s.len() == OUT_PATH_HASH_PREFIX_LEN
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
}

// ---------------------------------------------------------------------------
// Selection (pure)
// ---------------------------------------------------------------------------

/// One group's prune plan (pure selection output — nothing has been
/// removed yet). Richer than the [`prune_selection`] tuple form so the
/// `images gc` report can show what was KEPT and what was skipped for
/// running-sandbox protection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupPlan {
    /// Image name segment (the flake attr the tags were computed from).
    pub name: String,
    /// Tag-context segment (`None` = ctx-less tags).
    pub ctx: Option<String>,
    /// Tags to prune, ordered `loaded_at` DESC then tag ASC.
    pub prune: Vec<String>,
    /// Candidate tags NOT pruned because a running sandbox holds them
    /// (RESOLVED decision 3: running sandboxes are never affected).
    pub skipped_running: Vec<String>,
    /// Retained tags: the newest `keep_last`, PLUS any older tag still
    /// referenced by a current-pointer in `ImagesState.pointers`
    /// (pointers are never dangling by pruning).
    pub kept: Vec<String>,
}

/// The pure selection core (ADR 0032 §Image tags — RESOLVED user decision 3):
/// sweep the record map for computed-shape tags, group by `(name, ctx)`
/// (tags carry no repo; records do), order each group's candidate tags by
/// `ImageRecord.loaded_at` DESCENDING (missing record → treated as oldest;
/// a tag in several records takes its NEWEST `loaded_at`; ties break
/// lexicographically by tag ASC — deterministic), retain the newest
/// `keep_last`, and mark the rest for pruning. NEVER pruned: any tag
/// referenced by ANY pointer, any tag in `protected` (running sandboxes),
/// and the newest tag (implied by `keep_last >= 1`). Unparsable
/// (legacy-shape) tags are ignored ENTIRELY. `only` restricts the output to
/// a single `(name, ctx)` group — the prune-on-load call site passes the
/// just-loaded group; the manual sweep passes `None`.
pub fn plan_prunes(
    records: &BTreeMap<String, crate::images::state::ImageRecord>,
    pointers: &BTreeMap<String, PointerRecord>,
    protected: &BTreeSet<String>,
    keep_last: u32,
    only: Option<(&str, Option<&str>)>,
) -> Vec<GroupPlan> {
    // Pointer-referenced tags are never pruned (never dangle a pointer).
    let pointer_held: BTreeSet<&str> = pointers.values().map(|p| p.tag.as_str()).collect();

    // Newest loaded_at per candidate tag ("" = no usable stamp → oldest);
    // a tag carried by SEVERAL repo records takes its newest loaded_at and
    // appears in its group exactly once.
    let mut loaded_at: BTreeMap<&str, &str> = BTreeMap::new();
    // group (name, ctx) → candidate tags.
    let mut groups: BTreeMap<(String, Option<String>), Vec<String>> = BTreeMap::new();
    for record in records.values() {
        let Some((name, ctx)) = split_computed_tag(&record.tag) else {
            continue; // legacy/unparsable: ignored entirely
        };
        let seen = loaded_at.contains_key(record.tag.as_str());
        let prev = loaded_at.get(record.tag.as_str()).copied().unwrap_or("");
        if record.loaded_at.as_str() > prev {
            loaded_at.insert(record.tag.as_str(), record.loaded_at.as_str());
        }
        if !seen {
            groups
                .entry((name, ctx))
                .or_default()
                .push(record.tag.clone());
        }
    }

    let mut plans = Vec::new();
    for ((name, ctx), mut tags) in groups {
        if let Some((want_name, want_ctx)) = only {
            if name != want_name || ctx.as_deref() != want_ctx {
                continue;
            }
        }
        // loaded_at DESC, tie-break tag ASC (deterministic).
        tags.sort_by(|a, b| {
            let la = loaded_at.get(a.as_str()).copied().unwrap_or("");
            let lb = loaded_at.get(b.as_str()).copied().unwrap_or("");
            lb.cmp(la).then_with(|| a.cmp(b))
        });
        let retain = keep_last as usize;
        let mut kept: Vec<String> = tags[..retain.min(tags.len())].to_vec();
        let mut prune = Vec::new();
        let mut skipped_running = Vec::new();
        for tag in &tags[retain.min(tags.len())..] {
            if pointer_held.contains(tag.as_str()) {
                // Older than N but still a live resolution target: retained,
                // never dangled.
                kept.push(tag.clone());
            } else if protected.contains(tag) {
                skipped_running.push(tag.clone());
            } else {
                prune.push(tag.clone());
            }
        }
        plans.push(GroupPlan {
            name,
            ctx,
            prune,
            skipped_running,
            kept,
        });
    }
    plans
}

/// The pinned selection API (ADR 0032 §Image tags — RESOLVED user decision 3):
/// `(group name, ctx, prune tags)` triples for EVERY swept group (pass
/// `only = Some((name, ctx))` for the single-group prune-on-load form).
/// See [`plan_prunes`] for the full ordering/protection contract.
pub fn prune_selection(
    records: &BTreeMap<String, crate::images::state::ImageRecord>,
    pointers: &BTreeMap<String, PointerRecord>,
    protected: &BTreeSet<String>,
    keep_last: u32,
    only: Option<(&str, Option<&str>)>,
) -> Vec<(String, Option<String>, Vec<String>)> {
    plan_prunes(records, pointers, protected, keep_last, only)
        .into_iter()
        .map(|p| (p.name, p.ctx, p.prune))
        .collect()
}

// ---------------------------------------------------------------------------
// Execution (shared by prune-on-load and the manual sweep)
// ---------------------------------------------------------------------------

/// Outcome of executing one batch of prune plans (per group).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PruneBatchOutcome {
    /// Tags removed from the msb store this batch.
    pub pruned: Vec<String>,
    /// Candidate tags whose probe said Gone — counted as "already gone";
    /// their records/pointers are dropped without a removal call.
    pub already_gone: Vec<String>,
    /// Aggregate failures (unreachable store, refused removal); the batch
    /// continues past each one and the caller surfaces them at the end.
    pub errors: Vec<String>,
}

/// Drop every `<repo>#<tag>` record key and every pointer whose `.tag ==
/// tag` from `state`. Returns true when anything was dropped.
fn drop_tag_from_state(state: &mut ImagesState, tag: &str) -> bool {
    let record_keys: Vec<String> = state
        .images
        .keys()
        .filter(|k| k.split_once('#').is_some_and(|(_, t)| t == tag))
        .cloned()
        .collect();
    let pointer_keys: Vec<String> = state
        .pointers
        .iter()
        .filter(|(_, p)| p.tag == tag)
        .map(|(k, _)| k.clone())
        .collect();
    let dropped = !record_keys.is_empty() || !pointer_keys.is_empty();
    for k in record_keys {
        state.images.remove(&k);
    }
    for k in pointer_keys {
        state.pointers.remove(&k);
    }
    dropped
}

/// The running-sandbox protection set (RESOLVED decision 3): every
/// `image_tag` across ALL port-registry records. Conservative — stale
/// records over-protect until teardown unregisters them.
fn running_protection_set(state_dir: &Path) -> Result<BTreeSet<String>> {
    Ok(crate::microsandbox::port_registry::list_records(state_dir)?
        .into_iter()
        .filter_map(|r| r.image_tag)
        .collect())
}

/// What happened while loading the protection set. `Skipped` = the port
/// registry could not be read, so running protection CANNOT be verified —
/// the conservative direction is to prune NOTHING (a destructive sweep
/// refuses; prune-on-load degrades with a warning).
enum ProtectionSet {
    Protected(BTreeSet<String>),
    Unverifiable(String),
}

fn protection_set(state_dir: &Path) -> ProtectionSet {
    match running_protection_set(state_dir) {
        Ok(set) => ProtectionSet::Protected(set),
        Err(e) => ProtectionSet::Unverifiable(format!("{e:#}")),
    }
}

/// Execute prune plans in the MANUAL-SWEEP discipline: each pruned/
/// already-gone tag gets its own per-tag [`ImageTagLock`]; `ImagesState` is
/// freshly reloaded and saved ONCE per tag inside that lock (spec §3.3
/// critical-section discipline — a concurrent build of the same tag
/// serializes behind the lock).
async fn execute_plans_per_tag_locks<P: StoreProbe, R: ImageRemover>(
    state_dir: &Path,
    plans: &[GroupPlan],
    probe: &mut P,
    remover: &mut R,
) -> Result<BTreeMap<(String, Option<String>), PruneBatchOutcome>> {
    let mut outcomes = BTreeMap::new();
    for plan in plans {
        let mut outcome = PruneBatchOutcome::default();
        for tag in &plan.prune {
            match probe.tag_state(tag).await {
                Err(e) => outcome.errors.push(e.to_string()),
                Ok(crate::images::skew::StoreTag::Gone) => {
                    outcome.already_gone.push(tag.clone());
                    apply_tag_removal_locked(state_dir, tag)?;
                }
                Ok(crate::images::skew::StoreTag::Present) => match remover.remove_tag(tag).await {
                    Err(e) => outcome.errors.push(e.to_string()),
                    Ok(()) => {
                        outcome.pruned.push(tag.clone());
                        apply_tag_removal_locked(state_dir, tag)?;
                    }
                },
            }
        }
        outcomes.insert((plan.name.clone(), plan.ctx.clone()), outcome);
    }
    Ok(outcomes)
}

/// Drop one tag's records/pointers under the per-tag lock keyed on the
/// lexicographically-first `<repo>#<tag>` record key currently referencing
/// it (deterministic; cross-repo same-name groups are pathological per the
/// ADR). Nothing to drop → no lock, no write.
fn apply_tag_removal_locked(state_dir: &Path, tag: &str) -> Result<()> {
    let state = ImagesState::load(state_dir);
    let lock_key = state
        .images
        .keys()
        .find(|k| k.split_once('#').is_some_and(|(_, t)| t == tag))
        .cloned()
        .or_else(|| {
            state
                .pointers
                .iter()
                .find(|(_, p)| p.tag == tag)
                .map(|(k, _)| k.clone())
        });
    let Some(lock_key) = lock_key else {
        return Ok(());
    };
    let _lock = ImageTagLock::acquire(state_dir, &lock_key)?;
    let mut state = ImagesState::load(state_dir); // FRESH reload inside the lock
    if drop_tag_from_state(&mut state, tag) {
        state.save(state_dir)?;
    }
    Ok(())
}

/// Execute prune plans in the PRUNE-ON-LOAD discipline: the caller already
/// holds the per-tag lock for the just-loaded tag, so ALL removals run
/// first and `ImagesState` is freshly reloaded and saved ONCE per batch at
/// the end (spec §3.3 atomicity inside the held lock).
async fn execute_plans_under_held_lock<P: StoreProbe, R: ImageRemover>(
    state_dir: &Path,
    plans: &[GroupPlan],
    probe: &mut P,
    remover: &mut R,
) -> Result<BTreeMap<(String, Option<String>), PruneBatchOutcome>> {
    let mut outcomes = BTreeMap::new();
    let mut processed: Vec<String> = Vec::new();
    for plan in plans {
        let mut outcome = PruneBatchOutcome::default();
        for tag in &plan.prune {
            match probe.tag_state(tag).await {
                Err(e) => outcome.errors.push(e.to_string()),
                Ok(crate::images::skew::StoreTag::Gone) => {
                    outcome.already_gone.push(tag.clone());
                    processed.push(tag.clone());
                }
                Ok(crate::images::skew::StoreTag::Present) => match remover.remove_tag(tag).await {
                    Err(e) => outcome.errors.push(e.to_string()),
                    Ok(()) => {
                        outcome.pruned.push(tag.clone());
                        processed.push(tag.clone());
                    }
                },
            }
        }
        outcomes.insert((plan.name.clone(), plan.ctx.clone()), outcome);
    }
    if !processed.is_empty() {
        let mut state = ImagesState::load(state_dir); // FRESH reload under the held lock
        let mut changed = false;
        for tag in &processed {
            changed |= drop_tag_from_state(&mut state, tag);
        }
        if changed {
            state.save(state_dir)?;
        }
    }
    Ok(outcomes)
}

// ---------------------------------------------------------------------------
// Prune-on-load (the build-flow trigger point)
// ---------------------------------------------------------------------------

/// Prune-on-load (ADR 0032 §Image tags — RESOLVED user decision 3): called
/// from `build_cmd::process_target` immediately AFTER `run_build_pipeline`
/// returns Ok, INSIDE the still-held per-tag lock, build mode only. Sweeps
/// ONLY the just-loaded `(name, ctx)` group with the FULL cascade
/// (capsule → repo → settings → default, resolved by the caller). Both
/// `LoadAction` outcomes prune (the tag is confirmed current either way).
///
/// Posture: cleanup must never fail the load. Per-tag failures aggregate
/// into ONE stderr note; an UNVERIFIABLE running-protection set (unreadable
/// port registry) skips pruning entirely — the conservative direction.
pub async fn prune_on_load<P: StoreProbe, R: ImageRemover>(
    state_dir: &Path,
    name: &str,
    ctx: Option<&str>,
    keep_last: u32,
    probe: &mut P,
    remover: &mut R,
) -> Result<()> {
    let protected = match protection_set(state_dir) {
        ProtectionSet::Unverifiable(detail) => {
            eprintln!(
                "note: skipping image prune-on-load for '{name}': cannot verify the \
                 running-sandbox protection set ({detail}) — nothing pruned \
                 (ADR 0032 §Image tags: running sandboxes are never affected)"
            );
            return Ok(());
        }
        ProtectionSet::Protected(set) => set,
    };
    let state = ImagesState::load(state_dir);
    let plans = plan_prunes(
        &state.images,
        &state.pointers,
        &protected,
        keep_last,
        Some((name, ctx)),
    );
    let outcomes = execute_plans_under_held_lock(state_dir, &plans, probe, remover).await?;
    report_group_notes(&plans, &outcomes, "prune-on-load");
    Ok(())
}

/// One stderr line per group that actually did something (pruned / gone /
/// errored) — silent when the sweep found nothing past N.
fn report_group_notes(
    plans: &[GroupPlan],
    outcomes: &BTreeMap<(String, Option<String>), PruneBatchOutcome>,
    label: &str,
) {
    for plan in plans {
        let Some(outcome) = outcomes.get(&(plan.name.clone(), plan.ctx.clone())) else {
            continue;
        };
        if outcome.pruned.is_empty() && outcome.already_gone.is_empty() && outcome.errors.is_empty()
        {
            continue;
        }
        eprintln!(
            "{label}: {} pruned=[{}] gone=[{}] errors=[{}]",
            render_group_name(&plan.name, &plan.ctx),
            outcome.pruned.join(","),
            outcome.already_gone.join(","),
            outcome.errors.join("; "),
        );
    }
}

/// The rendered group label: `name` or `name:ctx`.
fn render_group_name(name: &str, ctx: &Option<String>) -> String {
    match ctx {
        Some(ctx) => format!("{name}:{ctx}"),
        None => name.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Manual sweep (`workestrate images gc`)
// ---------------------------------------------------------------------------

/// One group row of the `workestrate images gc` report (text and `--json`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GcGroupReport {
    /// Rendered group label: `name[:ctx]`.
    pub group: String,
    /// How many tags the group retains after the sweep.
    pub kept: usize,
    /// Tags removed from the msb store.
    pub pruned: Vec<String>,
    /// Candidate tags skipped because a running sandbox holds them.
    pub skipped_running: Vec<String>,
    /// Candidate tags already gone from the store (records cleaned up).
    pub already_gone: Vec<String>,
    /// Aggregate removal/probe failures for this group.
    pub errors: Vec<String>,
}

/// The seam-injected manual-sweep core (unit-tested with fakes + a temp
/// state dir, following `ensure_resolved`'s pattern): sweeps ALL
/// computed-shape groups in the state file. The cascade resolves WITHOUT
/// the capsule rung (documented asymmetry — the sweep operates on state-dir
/// groups, not invocations): repo entry > settings > default. Repo lookup
/// per group uses the repo identity of the group's LEXICOGRAPHICALLY-FIRST
/// record; canonical-path repo keys match no registered entry → fall
/// through. An unverifiable running-protection set REFUSES the sweep
/// (fail-closed — a destructive command must know what is running).
pub async fn gc_resolved<P: StoreProbe, R: ImageRemover>(
    state_dir: &Path,
    settings_keep_last: Option<u32>,
    repo_keep_last: &BTreeMap<String, u32>,
    probe: &mut P,
    remover: &mut R,
) -> Result<Vec<GcGroupReport>> {
    let protected = match protection_set(state_dir) {
        ProtectionSet::Unverifiable(detail) => anyhow::bail!(
            "images gc: cannot verify the running-sandbox protection set ({detail}) — \
             refusing to sweep (ADR 0032 §Image tags: running sandboxes are never \
             affected); fix the port registry and retry"
        ),
        ProtectionSet::Protected(set) => set,
    };
    let state = ImagesState::load(state_dir);

    // Distinct groups in deterministic order, each with its group's
    // lexicographically-first record's repo identity.
    let mut group_repo: BTreeMap<(String, Option<String>), String> = BTreeMap::new();
    for key in state.images.keys() {
        let Some((repo, tag)) = key.split_once('#') else {
            continue;
        };
        let Some((name, ctx)) = split_computed_tag(tag) else {
            continue;
        };
        group_repo
            .entry((name, ctx))
            .or_insert_with(|| repo.to_string());
    }

    let mut plans = Vec::new();
    for (group, repo) in &group_repo {
        let repo_rung = repo_keep_last.get(repo).copied();
        let keep_last = resolve_keep_last(settings_keep_last, repo_rung, None)?;
        plans.extend(plan_prunes(
            &state.images,
            &state.pointers,
            &protected,
            keep_last,
            Some((&group.0, group.1.as_deref())),
        ));
    }

    let outcomes = execute_plans_per_tag_locks(state_dir, &plans, probe, remover).await?;

    let mut reports = Vec::new();
    for plan in &plans {
        let outcome = outcomes
            .get(&(plan.name.clone(), plan.ctx.clone()))
            .cloned()
            .unwrap_or_default();
        reports.push(GcGroupReport {
            group: render_group_name(&plan.name, &plan.ctx),
            kept: plan.kept.len(),
            pruned: outcome.pruned,
            skipped_running: plan.skipped_running.clone(),
            already_gone: outcome.already_gone,
            errors: outcome.errors,
        });
    }
    Ok(reports)
}

/// Human text for the sweep: the summary line, then one line per group in
/// deterministic (sorted-group) order.
pub fn render_gc_text(reports: &[GcGroupReport]) -> String {
    let mut out = String::new();
    let swept = reports.len();
    let pruned: usize = reports.iter().map(|r| r.pruned.len()).sum();
    let skipped: usize = reports.iter().map(|r| r.skipped_running.len()).sum();
    let errors: usize = reports.iter().map(|r| r.errors.len()).sum();
    out.push_str(&format!(
        "images gc: swept {swept} group(s), pruned {pruned} tag(s), \
         skipped {skipped} running, errors {errors}\n"
    ));
    for r in reports {
        out.push_str(&format!(
            "{} kept={} pruned=[{}] skipped-running=[{}] gone=[{}]\n",
            r.group,
            r.kept,
            r.pruned.join(","),
            r.skipped_running.join(","),
            r.already_gone.join(","),
        ));
    }
    out
}

/// `workestrate images gc` — the manual keep-last-N sweep (ADR 0032 §Image
/// tags — RESOLVED user decision 3). Exit is Ok unless any removal errored,
/// then nonzero AFTER sweeping everything (cleanup-family aggregate rule).
pub async fn cmd_images_gc(json: bool) -> Result<()> {
    let state_dir = crate::config::resolve_state_dir();
    // Unreadable registry → rungs degrade to defaults (the advisory posture
    // every other consumer of load_registry applies); a sweep must not die
    // on registry corruption when defaults are safe.
    let registry = crate::config::load_registry().ok().flatten();
    let settings_keep_last = registry.as_ref().and_then(|r| r.settings.image_keep_last);
    let repo_keep_last: BTreeMap<String, u32> = registry
        .map(|r| {
            r.configs
                .into_iter()
                .filter_map(|(name, entry)| entry.image_keep_last.map(|v| (name, v)))
                .collect()
        })
        .unwrap_or_default();

    let mut probe = MsbStoreProbe;
    let mut remover = MsbCliRemover::new();
    let reports = gc_resolved(
        &state_dir,
        settings_keep_last,
        &repo_keep_last,
        &mut probe,
        &mut remover,
    )
    .await?;

    if json {
        // Bare-array envelope (house style — see json_out.rs).
        println!("{}", serde_json::to_string_pretty(&reports)?);
    } else {
        print!("{}", render_gc_text(&reports));
    }

    let total_errors: usize = reports.iter().map(|r| r.errors.len()).sum();
    if total_errors > 0 {
        anyhow::bail!(
            "images gc: {total_errors} removal error(s) — the sweep completed; \
             inspect the per-group errors above and retry"
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
    use crate::config::test_support::unique_state_dir;
    use crate::images::detect::test_fakes::FakeStoreProbe;
    use crate::images::pipeline::test_fakes::FakeRemover;
    use crate::images::skew::StoreTag;
    use crate::images::state::{
        ImageRecord, ImagesState, PointerRecord, RepoIdentity, image_key, pointer_key,
    };
    use std::path::PathBuf;

    const SHA_A: &str = "aaaaaaaaaaaa";
    const SHA_B: &str = "bbbbbbbbbbbb";
    const SHA_C: &str = "cccccccccccc";
    const SHA_D: &str = "dddddddddddd";

    fn repo_identity(name: &str) -> RepoIdentity {
        RepoIdentity {
            name: name.to_string(),
            path: PathBuf::from(format!("/tmp/{name}")),
            flake_root: PathBuf::from(format!("/tmp/{name}")),
        }
    }

    fn record(repo: &str, tag: &str, loaded_at: &str) -> (String, ImageRecord) {
        (
            image_key(repo, tag),
            ImageRecord {
                repo: repo_identity(repo),
                attr: tag.split(':').next().unwrap_or("img").to_string(),
                tag: tag.to_string(),
                drv_path: "/nix/store/x.drv".to_string(),
                out_path: "/nix/store/x".to_string(),
                digest: None,
                built_at: loaded_at.to_string(),
                loaded_at: loaded_at.to_string(),
                loader: "workestrate 0.1.0".to_string(),
                host: "devbox".to_string(),
                user: "node".to_string(),
            },
        )
    }

    fn pointer(repo: &str, name: &str, ctx: Option<&str>, tag: &str) -> (String, PointerRecord) {
        (
            pointer_key(repo, name, ctx),
            PointerRecord {
                tag: tag.to_string(),
                updated_at: "2026-08-24T10:00:00Z".to_string(),
            },
        )
    }

    // ---- split_computed_tag ----

    /// Accepts exactly the two computed shapes; rejects the legacy shapes
    /// and malformed segments (ADR 0032 §Image tags: legacy declared tags
    /// are NEVER GC candidates).
    #[test]
    fn split_computed_tag_accepts_only_computed_shapes() {
        assert_eq!(
            split_computed_tag(&format!("img-pi:{SHA_A}")),
            Some(("img-pi".to_string(), None))
        );
        assert_eq!(
            split_computed_tag(&format!("img-pi:feat-x.{SHA_A}")),
            Some(("img-pi".to_string(), Some("feat-x".to_string())))
        );
        // digits + letters mixed, all lowercase alnum.
        assert_eq!(
            split_computed_tag("w:0123456789ab"),
            Some(("w".to_string(), None))
        );
        // Dots in the NAME are legal and harmless: the name splits at the
        // FIRST colon, the ctx/sha split at the LAST dot of the remainder.
        assert_eq!(
            split_computed_tag(&format!("img.with.dots:feat-x.{SHA_A}")),
            Some(("img.with.dots".to_string(), Some("feat-x".to_string())))
        );
        assert_eq!(
            split_computed_tag(&format!("img.with.dots:{SHA_A}")),
            Some(("img.with.dots".to_string(), None))
        );
    }

    #[test]
    fn split_computed_tag_rejects_legacy_and_malformed_shapes() {
        assert_eq!(split_computed_tag("img-pi:latest"), None, "legacy tag");
        assert_eq!(split_computed_tag("img:latest:v2"), None, "legacy-ish");
        assert_eq!(split_computed_tag(""), None, "empty");
        assert_eq!(split_computed_tag("img:aaaaaaaaaaaaa"), None, "13-char sha");
        assert_eq!(split_computed_tag("img:aaaaaaaaaaa"), None, "11-char sha");
        assert_eq!(
            split_computed_tag("img:AAAAAAAAAAAA"),
            None,
            "uppercase sha"
        );
        assert_eq!(split_computed_tag("img:a-a2-a4-a6-a8"), None, "non-alnum");
        assert_eq!(split_computed_tag(":aaaaaaaaaaaa"), None, "empty name");
        assert_eq!(split_computed_tag("img::aaaaaaaaaaaa"), None, "empty ctx");
        assert_eq!(
            split_computed_tag("img:a:b:aaaaaaaaaaaa"),
            None,
            "two colons (the pre-amendment name:ctx:sha shape — an invalid \
             docker/OCI reference, host Bug B) is never a candidate"
        );
        assert_eq!(
            split_computed_tag("img:ctx.aaaaaaaaaaaa:extra"),
            None,
            "a second colon anywhere in the tag portion is invalid"
        );
        assert_eq!(
            split_computed_tag("img:.aaaaaaaaaaaa"),
            None,
            "empty ctx before the dot"
        );
        assert_eq!(
            split_computed_tag("img:ctx."),
            None,
            "empty sha after the dot"
        );
        assert_eq!(
            split_computed_tag("img:a.b.aaaaaaaaaaaa"),
            None,
            "a dotted ctx can never be emitted (slugify strips dots) — reject \
             rather than guess the split"
        );
    }

    // ---- resolve_keep_last (the cascade) ----

    #[test]
    fn resolve_keep_last_cascade_first_some_wins_capsule_to_default() {
        assert_eq!(resolve_keep_last(None, None, None).unwrap(), 5);
        assert_eq!(resolve_keep_last(Some(7), None, None).unwrap(), 7);
        assert_eq!(resolve_keep_last(Some(7), Some(3), None).unwrap(), 3);
        assert_eq!(resolve_keep_last(Some(7), Some(3), Some(2)).unwrap(), 2);
        assert_eq!(resolve_keep_last(None, Some(9), None).unwrap(), 9);
        assert_eq!(resolve_keep_last(None, None, Some(1)).unwrap(), 1);
    }

    #[test]
    fn resolve_keep_last_zero_is_a_hard_error_naming_field_and_locus() {
        for (capsule, repo, settings, locus) in [
            (Some(0), None, None, "workloads.<name>.image.keep_last"),
            (None, Some(0), None, "configs[<repo>].image_keep_last"),
            (None, None, Some(0), "settings.image_keep_last"),
        ] {
            let err =
                resolve_keep_last(settings, repo, capsule).expect_err("explicit 0 must hard-error");
            let msg = err.to_string();
            assert!(msg.contains("keep_last = 0"), "{msg}");
            assert!(msg.contains(locus), "names the locus '{locus}': {msg}");
            assert!(msg.contains(">= 1"), "states the floor: {msg}");
        }
        // Zero BEHIND a higher rung never fires (first Some wins).
        assert_eq!(resolve_keep_last(Some(0), Some(3), None).unwrap(), 3);
    }

    // ---- plan_prunes / prune_selection ----

    #[test]
    fn prune_selection_orders_by_loaded_at_desc_ties_by_tag_asc() {
        let mut records = BTreeMap::new();
        // Loaded order: c (oldest) < a < b (newest); d's record carries an
        // EMPTY loaded_at stamp → treated as the oldest candidate.
        records.extend([
            record("personal", &format!("img:{SHA_B}"), "2026-08-24T03:00:00Z"),
            record("personal", &format!("img:{SHA_A}"), "2026-08-24T02:00:00Z"),
            record("personal", &format!("img:{SHA_C}"), "2026-08-24T01:00:00Z"),
            record("personal", &format!("img:{SHA_D}"), ""),
        ]);
        let pointers = BTreeMap::new();
        let protected = BTreeSet::new();

        let plans = plan_prunes(&records, &pointers, &protected, 2, None);
        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].name, "img");
        assert_eq!(plans[0].ctx, None);
        // Newest two kept: b (03:00) then a (02:00). Candidates in
        // loaded_at-DESC order: c (01:00) then d (empty stamp → oldest).
        assert_eq!(
            plans[0].kept,
            vec![format!("img:{SHA_B}"), format!("img:{SHA_A}")]
        );
        assert_eq!(
            plans[0].prune,
            vec![format!("img:{SHA_C}"), format!("img:{SHA_D}")]
        );

        // The pinned tuple API agrees.
        let tuples = prune_selection(&records, &pointers, &protected, 2, None);
        assert_eq!(
            tuples,
            vec![(
                "img".to_string(),
                None,
                vec![format!("img:{SHA_C}"), format!("img:{SHA_D}")]
            )]
        );
    }

    #[test]
    fn prune_selection_tie_breaks_equal_loaded_at_lexicographically() {
        let mut records = BTreeMap::new();
        let stamp = "2026-08-24T03:00:00Z";
        records.extend([
            record("personal", &format!("img:{SHA_B}"), stamp),
            record("personal", &format!("img:{SHA_A}"), stamp),
            record("personal", &format!("img:{SHA_C}"), stamp),
        ]);
        let plans = plan_prunes(&records, &BTreeMap::new(), &BTreeSet::new(), 1, None);
        // All three share loaded_at → tag ASC: a, b, c. Keep a; prune b, c.
        assert_eq!(plans[0].kept, vec![format!("img:{SHA_A}")]);
        assert_eq!(
            plans[0].prune,
            vec![format!("img:{SHA_B}"), format!("img:{SHA_C}")]
        );
    }

    #[test]
    fn prune_selection_never_prunes_pointer_referenced_or_running_tags() {
        let mut records = BTreeMap::new();
        for (i, sha) in [SHA_A, SHA_B, SHA_C, SHA_D].iter().enumerate() {
            let (_, r) = record(
                "personal",
                &format!("img:{sha}"),
                &format!("2026-08-24T0{:0>2}:00:00Z", i + 1),
            );
            records.insert(image_key("personal", &format!("img:{sha}")), r);
        }
        // A pointer still resolves the OLDEST tag (cross-repo case).
        let pointers = BTreeMap::from([pointer("other", "img", None, &format!("img:{SHA_A}"))]);
        // A running sandbox holds the second-oldest tag.
        let protected = BTreeSet::from([format!("img:{SHA_B}")]);

        let plans = plan_prunes(&records, &pointers, &protected, 2, None);
        // Newest two (c, d) kept by N; a retained via pointer; b skipped-running.
        assert_eq!(
            plans[0].kept,
            vec![
                format!("img:{SHA_D}"),
                format!("img:{SHA_C}"),
                format!("img:{SHA_A}")
            ]
        );
        assert_eq!(plans[0].skipped_running, vec![format!("img:{SHA_B}")]);
        assert!(plans[0].prune.is_empty(), "nothing left to prune");
    }

    #[test]
    fn prune_selection_ignores_unparsable_legacy_tags_entirely() {
        let mut records = BTreeMap::new();
        records.extend([
            record("personal", "img-pi:latest", "2026-08-24T01:00:00Z"),
            record("personal", &format!("img:{SHA_A}"), "2026-08-24T02:00:00Z"),
            record("personal", &format!("img:{SHA_B}"), "2026-08-24T03:00:00Z"),
            record("personal", &format!("img:{SHA_C}"), "2026-08-24T04:00:00Z"),
            record("personal", &format!("img:{SHA_D}"), "2026-08-24T05:00:00Z"),
        ]);
        // keep_last 2: the four computed tags prune down to c+d; the LEGACY
        // record is invisible to the sweep (never a candidate, never touched).
        let plans = plan_prunes(&records, &BTreeMap::new(), &BTreeSet::new(), 2, None);
        assert_eq!(plans.len(), 1, "the legacy tag forms NO group");
        assert_eq!(
            plans[0].prune,
            vec![format!("img:{SHA_B}"), format!("img:{SHA_A}")]
        );
        assert!(!plans[0].prune.iter().any(|t| t == "img-pi:latest"));
    }

    #[test]
    fn prune_selection_groups_by_name_and_ctx_and_filters_with_only() {
        let mut records = BTreeMap::new();
        records.extend([
            record("personal", &format!("img:{SHA_A}"), "2026-08-24T01:00:00Z"),
            record("personal", &format!("img:{SHA_B}"), "2026-08-24T02:00:00Z"),
            record(
                "personal",
                &format!("img:feat-x.{SHA_A}"),
                "2026-08-24T03:00:00Z",
            ),
            record(
                "personal",
                &format!("prime:{SHA_A}"),
                "2026-08-24T04:00:00Z",
            ),
        ]);
        let all = prune_selection(&records, &BTreeMap::new(), &BTreeSet::new(), 1, None);
        let mut names: Vec<_> = all.iter().map(|(n, c, _)| (n.clone(), c.clone())).collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                ("img".to_string(), None),
                ("img".to_string(), Some("feat-x".to_string())),
                ("prime".to_string(), None),
            ],
            "groups keyed by (name, ctx); tags carry no repo"
        );
        // only=(img, None) → exactly that group's plan.
        let only_img = prune_selection(
            &records,
            &BTreeMap::new(),
            &BTreeSet::new(),
            1,
            Some(("img", None)),
        );
        assert_eq!(only_img.len(), 1);
        assert_eq!(only_img[0].0, "img");
        assert_eq!(only_img[0].1, None);
        assert_eq!(only_img[0].2, vec![format!("img:{SHA_A}")]);
        // only=(img, feat-x) → the ctx group.
        let only_ctx = prune_selection(
            &records,
            &BTreeMap::new(),
            &BTreeSet::new(),
            1,
            Some(("img", Some("feat-x"))),
        );
        assert_eq!(only_ctx.len(), 1);
        assert_eq!(only_ctx[0].1.as_deref(), Some("feat-x"));
        assert!(only_ctx[0].2.is_empty(), "single-tag group: nothing past N");
    }

    #[test]
    fn prune_selection_cross_repo_records_share_one_group_newest_loaded_at_wins() {
        let mut records = BTreeMap::new();
        // Same tag loaded by TWO repos at different times; plus an older tag.
        records.extend([
            record("personal", &format!("img:{SHA_A}"), "2026-08-24T05:00:00Z"),
            record("work", &format!("img:{SHA_A}"), "2026-08-24T01:00:00Z"),
            record("personal", &format!("img:{SHA_B}"), "2026-08-24T03:00:00Z"),
        ]);
        let plans = plan_prunes(&records, &BTreeMap::new(), &BTreeSet::new(), 1, None);
        assert_eq!(plans.len(), 1, "one (name, ctx) group despite two repos");
        // a's NEWEST loaded_at (05:00) beats b (03:00) → keep a, prune b.
        assert_eq!(plans[0].kept, vec![format!("img:{SHA_A}")]);
        assert_eq!(plans[0].prune, vec![format!("img:{SHA_B}")]);
    }

    // ---- gc_resolved end-to-end (fake seams + temp state dir) ----

    /// Seed the state dir with records/pointers and return nothing (the
    /// caller re-loads).
    fn seed_state(
        state_dir: &Path,
        entries: Vec<(String, ImageRecord)>,
        pointers: Vec<(String, PointerRecord)>,
    ) {
        let mut state = ImagesState::default();
        for (k, r) in entries {
            state.upsert(k, r);
        }
        for (k, p) in pointers {
            state.upsert_pointer(k, p);
        }
        state.save(state_dir).unwrap();
    }

    /// Register a port-registry record carrying `image_tag` (the running-
    /// sandbox protection source).
    fn register_running(state_dir: &Path, instance: &str, workload: &str, image_tag: Option<&str>) {
        crate::microsandbox::port_registry::check_and_register_sandbox_lifecycle(
            state_dir,
            instance,
            None,
            workload,
            std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            &[4000],
            &[crate::microsandbox::plan::PortMapping::new(4000, 4000)],
            "2026-08-24T00:00:00Z",
            "default",
            None,
            image_tag,
            None,
            None,
        )
        .unwrap();
    }

    #[tokio::test]
    async fn gc_resolved_sweeps_all_groups_and_updates_state() -> Result<()> {
        let state_dir = unique_state_dir("gc-sweep");
        seed_state(
            &state_dir,
            vec![
                record("personal", &format!("img:{SHA_A}"), "2026-08-24T01:00:00Z"),
                record("personal", &format!("img:{SHA_B}"), "2026-08-24T02:00:00Z"),
                record("personal", &format!("img:{SHA_C}"), "2026-08-24T03:00:00Z"),
                record(
                    "personal",
                    &format!("prime:feat-x.{SHA_A}"),
                    "2026-08-24T04:00:00Z",
                ),
                record("personal", "legacy-img:latest", "2026-08-24T05:00:00Z"),
            ],
            vec![pointer(
                "personal",
                "prime",
                Some("feat-x"),
                &format!("prime:feat-x.{SHA_A}"),
            )],
        );

        let mut probe = FakeStoreProbe::new();
        // img group: keep_last=1 → candidates [b, a] (loaded_at DESC) → both
        // Present (two removals).
        probe.push(StoreTag::Present);
        probe.push(StoreTag::Present);

        let mut remover = FakeRemover::new();
        let reports = gc_resolved(
            &state_dir,
            Some(1),
            &BTreeMap::new(),
            &mut probe,
            &mut remover,
        )
        .await?;

        assert_eq!(reports.len(), 2, "img + prime:feat-x groups swept");
        assert_eq!(reports[0].group, "img");
        assert_eq!(reports[0].kept, 1);
        assert_eq!(
            reports[0].pruned,
            vec![format!("img:{SHA_B}"), format!("img:{SHA_A}")]
        );
        assert_eq!(reports[1].group, "prime:feat-x");
        assert_eq!(reports[1].kept, 1);
        assert!(reports[1].pruned.is_empty());

        assert_eq!(
            remover.calls,
            vec![format!("img:{SHA_B}"), format!("img:{SHA_A}")],
            "removals in loaded_at-desc order"
        );

        // State: pruned tags' records dropped; legacy record + kept tags +
        // pointer intact.
        let state = ImagesState::load(&state_dir);
        assert!(
            state
                .lookup(&image_key("personal", &format!("img:{SHA_A}")))
                .is_none()
        );
        assert!(
            state
                .lookup(&image_key("personal", &format!("img:{SHA_B}")))
                .is_none()
        );
        assert!(
            state
                .lookup(&image_key("personal", &format!("img:{SHA_C}")))
                .is_some()
        );
        assert!(
            state
                .lookup(&image_key("personal", "legacy-img:latest"))
                .is_some(),
            "legacy record untouched (TOLERATE migration shape)"
        );
        assert_eq!(
            state
                .lookup_pointer(&pointer_key("personal", "prime", Some("feat-x")))
                .map(|p| p.tag.as_str()),
            Some(&format!("prime:feat-x.{SHA_A}")[..]),
        );

        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[tokio::test]
    async fn gc_resolved_protects_running_sandboxes_and_counts_already_gone() -> Result<()> {
        let state_dir = unique_state_dir("gc-protect");
        seed_state(
            &state_dir,
            vec![
                record("personal", &format!("img:{SHA_A}"), "2026-08-24T01:00:00Z"),
                record("personal", &format!("img:{SHA_B}"), "2026-08-24T02:00:00Z"),
                record("personal", &format!("img:{SHA_C}"), "2026-08-24T03:00:00Z"),
            ],
            vec![],
        );
        // A running sandbox holds the OLDEST tag (A1-consistent fixture:
        // bare instance name == workload, context None).
        register_running(&state_dir, "img", "img", Some(&format!("img:{SHA_A}")));

        let mut probe = FakeStoreProbe::new();
        // keep_last=1 → prune candidates are [b, a] (loaded_at DESC); b is
        // probed Present and removed; a is skipped-running (never probed).
        probe.push(StoreTag::Present);

        let mut remover = FakeRemover::new();
        let reports = gc_resolved(
            &state_dir,
            Some(1),
            &BTreeMap::new(),
            &mut probe,
            &mut remover,
        )
        .await?;

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].kept, 1);
        assert_eq!(reports[0].skipped_running, vec![format!("img:{SHA_A}")]);
        assert_eq!(reports[0].pruned, vec![format!("img:{SHA_B}")]);
        assert_eq!(
            probe.calls,
            vec![format!("img:{SHA_B}")],
            "running tag never probed"
        );

        let state = ImagesState::load(&state_dir);
        assert!(
            state
                .lookup(&image_key("personal", &format!("img:{SHA_A}")))
                .is_some(),
            "running sandbox's tag record survives"
        );

        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[tokio::test]
    async fn gc_resolved_counts_already_gone_and_cleans_records_without_removal() -> Result<()> {
        let state_dir = unique_state_dir("gc-gone");
        seed_state(
            &state_dir,
            vec![
                record("personal", &format!("img:{SHA_A}"), "2026-08-24T01:00:00Z"),
                record("personal", &format!("img:{SHA_B}"), "2026-08-24T02:00:00Z"),
            ],
            vec![],
        );
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Gone); // a: already gone
        let mut remover = FakeRemover::new();
        let reports = gc_resolved(
            &state_dir,
            Some(1),
            &BTreeMap::new(),
            &mut probe,
            &mut remover,
        )
        .await?;
        assert_eq!(reports[0].already_gone, vec![format!("img:{SHA_A}")]);
        assert!(reports[0].pruned.is_empty());
        assert!(
            remover.calls.is_empty(),
            "gone tags never reach the remover"
        );
        let state = ImagesState::load(&state_dir);
        assert!(
            state
                .lookup(&image_key("personal", &format!("img:{SHA_A}")))
                .is_none(),
            "already-gone tag's record dropped"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[tokio::test]
    async fn gc_resolved_aggregates_removal_errors_and_keeps_sweeping() -> Result<()> {
        let state_dir = unique_state_dir("gc-errors");
        seed_state(
            &state_dir,
            vec![
                record("personal", &format!("img:{SHA_A}"), "2026-08-24T01:00:00Z"),
                record("personal", &format!("img:{SHA_B}"), "2026-08-24T02:00:00Z"),
                record("personal", &format!("img:{SHA_C}"), "2026-08-24T03:00:00Z"),
            ],
            vec![],
        );
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present); // a
        probe.push(StoreTag::Present); // b
        let mut remover = FakeRemover::new();
        // Candidates [b, a] (loaded_at DESC): b's removal succeeds; a's is
        // refused — the sweep continues past the error.
        remover.push_ok();
        remover.push_err(crate::images::pipeline::RemoveError::Refused {
            tag: format!("img:{SHA_A}"),
            detail: "backend refusal".to_string(),
        });
        let reports = gc_resolved(
            &state_dir,
            Some(1),
            &BTreeMap::new(),
            &mut probe,
            &mut remover,
        )
        .await?;
        assert_eq!(reports[0].errors.len(), 1);
        assert!(reports[0].errors[0].contains("refused to remove"));
        assert_eq!(reports[0].pruned, vec![format!("img:{SHA_B}")]);
        // The failed tag's record STAYS (nothing dropped for it).
        let state = ImagesState::load(&state_dir);
        assert!(
            state
                .lookup(&image_key("personal", &format!("img:{SHA_A}")))
                .is_some()
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[tokio::test]
    async fn gc_resolved_refuses_when_protection_set_is_unverifiable() -> Result<()> {
        let state_dir = unique_state_dir("gc-refuses");
        seed_state(
            &state_dir,
            vec![record(
                "personal",
                &format!("img:{SHA_A}"),
                "2026-08-24T01:00:00Z",
            )],
            vec![],
        );
        // Make the run dir a FILE → list_records fails → protection set
        // unverifiable → the sweep refuses (fail-closed).
        let run_parent = state_dir.join("var");
        std::fs::create_dir_all(&run_parent).unwrap();
        std::fs::write(run_parent.join("run"), b"not a dir").unwrap();

        let mut probe = FakeStoreProbe::new();
        let mut remover = FakeRemover::new();
        let err = gc_resolved(&state_dir, None, &BTreeMap::new(), &mut probe, &mut remover)
            .await
            .expect_err("unverifiable protection must refuse the sweep");
        assert!(
            err.to_string().contains("running-sandbox protection set"),
            "{err}"
        );
        assert!(probe.calls.is_empty() && remover.calls.is_empty());
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[tokio::test]
    async fn gc_resolved_cascade_uses_repo_entry_over_settings_per_group() -> Result<()> {
        let state_dir = unique_state_dir("gc-cascade");
        // personal group: 3 tags; work group: 3 tags.
        let mut entries = Vec::new();
        for (repo, shas) in [
            ("personal", [SHA_A, SHA_B, SHA_C]),
            ("work", [SHA_A, SHA_B, SHA_C]),
        ] {
            for (i, sha) in shas.iter().enumerate() {
                entries.push(record(
                    repo,
                    &format!("img-{repo}:{sha}"),
                    &format!("2026-08-24T0{}:00:00Z", i + 1),
                ));
            }
        }
        seed_state(&state_dir, entries, vec![]);

        // settings=2; personal repo entry=1 → personal keeps 1, work keeps 2.
        let repo_rungs = BTreeMap::from([("personal".to_string(), 1u32)]);
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present); // personal:a (pruned under N=1)
        probe.push(StoreTag::Present); // personal:b (pruned under N=1)
        probe.push(StoreTag::Present); // work:a (pruned under N=2)
        let mut remover = FakeRemover::new();
        let reports =
            gc_resolved(&state_dir, Some(2), &repo_rungs, &mut probe, &mut remover).await?;

        let personal = reports.iter().find(|r| r.group == "img-personal").unwrap();
        let work = reports.iter().find(|r| r.group == "img-work").unwrap();
        assert_eq!(personal.kept, 1, "repo rung 1 wins over settings 2");
        assert_eq!(personal.pruned.len(), 2);
        assert_eq!(work.kept, 2, "settings rung 2 applies without a repo entry");
        assert_eq!(work.pruned.len(), 1);

        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- render_gc_text ----

    #[test]
    fn render_gc_text_matches_the_pinned_summary_shape() {
        let reports = vec![
            GcGroupReport {
                group: "img".to_string(),
                kept: 2,
                pruned: vec!["img:aaaaaaaaaaaa".to_string()],
                skipped_running: vec!["img:bbbbbbbbbbbb".to_string()],
                already_gone: vec![],
                errors: vec![],
            },
            GcGroupReport {
                group: "prime:feat-x".to_string(),
                kept: 1,
                pruned: vec![],
                skipped_running: vec![],
                already_gone: vec!["prime:feat-x.cccccccccccc".to_string()],
                errors: vec![],
            },
        ];
        let text = render_gc_text(&reports);
        let mut lines = text.lines();
        assert_eq!(
            lines.next(),
            Some("images gc: swept 2 group(s), pruned 1 tag(s), skipped 1 running, errors 0")
        );
        assert_eq!(
            lines.next(),
            Some("img kept=2 pruned=[img:aaaaaaaaaaaa] skipped-running=[img:bbbbbbbbbbbb] gone=[]")
        );
        assert_eq!(
            lines.next(),
            Some(
                "prime:feat-x kept=1 pruned=[] skipped-running=[] gone=[prime:feat-x.cccccccccccc]"
            )
        );
        assert_eq!(lines.next(), None);
    }

    // ---- prune_on_load (the build-flow trigger, fake seams) ----

    #[tokio::test]
    async fn prune_on_load_prunes_only_the_loaded_group_under_the_held_lock() -> Result<()> {
        let state_dir = unique_state_dir("gc-prune-on-load");
        seed_state(
            &state_dir,
            vec![
                record("personal", &format!("img:{SHA_A}"), "2026-08-24T01:00:00Z"),
                record("personal", &format!("img:{SHA_B}"), "2026-08-24T02:00:00Z"),
                record("personal", &format!("img:{SHA_C}"), "2026-08-24T03:00:00Z"),
                // Another group that must NOT be touched.
                record(
                    "personal",
                    &format!("prime:{SHA_A}"),
                    "2026-08-24T04:00:00Z",
                ),
            ],
            vec![],
        );
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present); // a
        probe.push(StoreTag::Present); // b
        let mut remover = FakeRemover::new();

        // keep_last=1: the just-loaded tag is c (retained); a, b prune.
        prune_on_load(&state_dir, "img", None, 1, &mut probe, &mut remover).await?;

        assert_eq!(
            remover.calls,
            vec![format!("img:{SHA_B}"), format!("img:{SHA_A}")],
            "candidates prune in loaded_at-DESC order"
        );
        let state = ImagesState::load(&state_dir);
        assert!(
            state
                .lookup(&image_key("personal", &format!("img:{SHA_A}")))
                .is_none()
        );
        assert!(
            state
                .lookup(&image_key("personal", &format!("img:{SHA_B}")))
                .is_none()
        );
        assert!(
            state
                .lookup(&image_key("personal", &format!("img:{SHA_C}")))
                .is_some()
        );
        assert!(
            state
                .lookup(&image_key("personal", &format!("prime:{SHA_A}")))
                .is_some(),
            "other groups untouched by prune-on-load (single-group scope)"
        );
        // No lock file survives the batch.
        assert!(
            !crate::images::state::image_locks_dir(&state_dir).exists()
                || std::fs::read_dir(crate::images::state::image_locks_dir(&state_dir))
                    .unwrap()
                    .count()
                    == 0
        );

        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[tokio::test]
    async fn prune_on_load_skips_when_protection_is_unverifiable() -> Result<()> {
        let state_dir = unique_state_dir("gc-prune-unverifiable");
        seed_state(
            &state_dir,
            vec![record(
                "personal",
                &format!("img:{SHA_A}"),
                "2026-08-24T01:00:00Z",
            )],
            vec![],
        );
        let run_parent = state_dir.join("var");
        std::fs::create_dir_all(&run_parent).unwrap();
        std::fs::write(run_parent.join("run"), b"not a dir").unwrap();

        let mut probe = FakeStoreProbe::new();
        let mut remover = FakeRemover::new();
        prune_on_load(&state_dir, "img", None, 1, &mut probe, &mut remover).await?;
        assert!(
            probe.calls.is_empty() && remover.calls.is_empty(),
            "conservative: nothing pruned when protection cannot be verified"
        );
        let state = ImagesState::load(&state_dir);
        assert!(
            state
                .lookup(&image_key("personal", &format!("img:{SHA_A}")))
                .is_some()
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }
}
