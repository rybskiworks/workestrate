//! The down scope ladder (ADR 0032 addendum §Down scope ladder):
//! scope model, classification engine, enumeration, and scope resolution
//! for `workestrate down <scope>`.
//!
//! Ladder (narrowest → widest): `instance < workload < context (= branch) <
//! config-ref < home (--all) < everything (--everything, double-gated)`.
//! The instance/workload rungs stay on the per-workload path
//! (`workload <name> down [--instance|--all-instances]`); THIS module models
//! the four sweep rungs (context / config-ref / home / everything).
//!
//! Classification engine (§ Cleanup family rules STAND): a target is
//! workestrate-managed iff registry record ∨ slot-name pattern
//! (`<context>-<workload>`) ∨ artifact evidence (the detached-child
//! `workestrate.log`, persisted in the state dir since 2026-08-30 — the
//! legacy sandbox-dir location is still probed). Every target at every
//! scope goes through the hardened
//! teardown path ([`super::down_hardened`]); per-target outcomes are
//! reported and ANY failure exits nonzero (§Down scope ladder).

use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

use microsandbox::Sandbox;

/// One rung of the down scope ladder carried by `workestrate down`
/// (ADR 0032 addendum §Down scope ladder). The instance/workload rungs are
/// NOT modeled here — they remain the per-workload verb surface
/// (`workload <name> down [--instance|--all-instances]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DownScope {
    /// Every managed target whose record context (primary) or `<ctx>-`
    /// slot prefix (corroborating) names `ctx`.
    Context(String),
    /// A VALIDATED branch-shaped config ref; resolves as the context the
    /// branch implies. Records carry no config-ref stamp, so v1 config-ref
    /// scope = validated-branch context scope (documented pin).
    ConfigRef(String),
    /// Home scope: every workestrate-managed target (today's `down --all`,
    /// now classification-reported).
    Home,
    /// Everything scope (DOUBLE-GATED): every msb sandbox regardless of
    /// classification; unmanaged candidates are torn down too and reported
    /// with empty evidence.
    Everything,
}

impl DownScope {
    /// The human-readable scope description used in the outcome header
    /// (`down <description>: N target(s)`) and the JSON `scope` field.
    pub fn description(&self) -> String {
        match self {
            DownScope::Context(ctx) => format!("context '{ctx}'"),
            DownScope::ConfigRef(r) => format!("config-ref '{r}'"),
            DownScope::Home => "--all (home)".to_string(),
            DownScope::Everything => "everything".to_string(),
        }
    }
}

/// One piece of workestrate-management evidence for a target
/// (ADR 0032 addendum § Cleanup family classification engine).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Evidence {
    /// A port-registry record exists for the instance.
    RegistryRecord,
    /// The instance's slot (parallel id stripped) carries the
    /// `<context>-<workload>` shape (contains `-`).
    SlotPattern,
    /// The instance carries the detached-child `workestrate.log` artifact
    /// (state dir since 2026-08-30; legacy sandbox dir still probed).
    ArtifactLog,
}

/// A teardown candidate discovered by enumeration, with the evidence that
/// classifies it. `evidence == []` marks an UNMANAGED candidate — it exists
/// only under `--everything`, which tears it down but reports the emptiness
/// honestly (the operator asked for everything; the report stays honest
/// about what was foreign).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagedTarget {
    pub instance: String,
    /// The registry record's context, when a record exists. PRIMARY
    /// evidence for context-scope resolution.
    pub context: Option<String>,
    /// All evidence pieces found for this target (conservative ∨ per the
    /// ADR: any one piece makes the target managed).
    pub evidence: Vec<Evidence>,
}

/// Classify ONE instance name (pure; ADR 0032 addendum § Cleanup family):
///
/// - [`Evidence::RegistryRecord`] when a record exists (`record_context`
///   is `Some(..)`; the inner `Option` carries the record's context);
/// - [`Evidence::SlotPattern`] when the slot (parallel id stripped via
///   `slot_of_instance`) contains `-` — the `<ctx>-<wl>` shape;
/// - [`Evidence::ArtifactLog`] when the instance carries the detached-child
///   `workestrate.log` (state dir since 2026-08-30; legacy sandbox dir
///   still probed).
///
/// Conservative ∨ per the ADR: any one piece of evidence makes the target
/// managed; pieces stack in the returned vec.
pub fn classify(
    instance: &str,
    record_context: Option<Option<&str>>,
    artifact_log_exists: bool,
) -> Vec<Evidence> {
    let mut out = Vec::new();
    if record_context.is_some() {
        out.push(Evidence::RegistryRecord);
    }
    if crate::microsandbox::slots::slot_of_instance(instance).contains('-') {
        out.push(Evidence::SlotPattern);
    }
    if artifact_log_exists {
        out.push(Evidence::ArtifactLog);
    }
    out
}

/// The artifact-evidence file for an instance — PRIMARY location
/// (ADR 0032 addendum 2026-08-30): the detached-child log now lives in
/// the workestrate state dir (`<state_dir>/logs/<instance>/workestrate.log`)
/// so it survives teardown; the RAW identity names the dir.
fn artifact_log_path(instance: &str) -> std::path::PathBuf {
    crate::microsandbox::runtime::detached_log_path(instance)
}

/// The LEGACY artifact-evidence location (pre-relocation homes):
/// `<msb_home>/sandboxes/<name>/workestrate.log` — the ephemeral sandbox
/// dir the log was written to before the 2026-08-30 relocation.
fn legacy_artifact_log_path(instance: &str) -> std::path::PathBuf {
    crate::microsandbox::runtime::reconcile::sandbox_dir(instance).join("workestrate.log")
}

/// Artifact-evidence probe, DUAL-PATH (ADR 0032 addendum 2026-08-30):
/// the NEW state-dir location first, then the LEGACY sandbox-dir fallback
/// (old homes / logs written before the relocation) — mirroring the
/// dual-spelling probe style of [`enumerate_targets`].
fn artifact_log_exists(instance: &str) -> bool {
    artifact_log_path(instance).exists() || legacy_artifact_log_path(instance).exists()
}

/// List every sandbox name currently known to the local msb, paginating
/// through [`Sandbox::list_with`]. On ANY SDK error the caller's contract
/// applies: DEGRADE with a stderr warning to the dir-listing fallback
/// (`<msb_home>/sandboxes/*`) rather than failing the whole sweep — an
/// unreachable store must not brick `down --context`.
async fn msb_sandbox_names() -> Vec<String> {
    const PAGE_LIMIT: u32 = 100;
    // Safety bound on pagination: opaque cursors are trusted to terminate;
    // a hostile/repeating cursor must not hang a teardown sweep forever.
    const MAX_PAGES: usize = 1000;
    let mut names: Vec<String> = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..MAX_PAGES {
        let page = if let Some(c) = cursor.clone() {
            Sandbox::list_with(|b| b.cursor(c).limit(PAGE_LIMIT)).await
        } else {
            Sandbox::list_with(|b| b.limit(PAGE_LIMIT)).await
        };
        match page {
            Ok(p) => {
                for h in p.sandboxes {
                    names.push(h.name().to_string());
                }
                match p.next_cursor {
                    Some(next) if Some(&next) != cursor.as_ref() => cursor = Some(next),
                    _ => return names,
                }
            }
            Err(e) => {
                eprintln!(
                    "warning: could not list msb sandboxes ({e}); \
                     falling back to the sandbox-dir listing under {}",
                    super::reconcile::msb_home().display()
                );
                return dir_listing_names();
            }
        }
    }
    eprintln!(
        "warning: msb sandbox listing did not terminate within {MAX_PAGES} pages; \
         proceeding with the names gathered so far"
    );
    names
}

/// The degraded enumeration fallback: directory names under
/// `<msb_home>/sandboxes/*`. An absent sandboxes dir lists as empty.
fn dir_listing_names() -> Vec<String> {
    let root = super::reconcile::msb_home().join("sandboxes");
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&root) else {
        return out;
    };
    for entry in entries.filter_map(|e| e.ok()) {
        // Only directories are sandbox-dir candidates (the create gate's
        // shape); stray files are ignored.
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                out.push(name.to_string());
            }
        }
    }
    out.sort();
    out
}

/// Enumerate every MANAGED teardown candidate (ADR 0032 addendum §Down
/// scope ladder): the union of port-registry records and the msb sandbox
/// listing (SDK `Sandbox::list`, degrading to the `<msb_home>/sandboxes/*`
/// dir fallback on SDK error), deduped by instance name, each classified by
/// [`classify`] over record presence/context, slot shape, and the
/// `workestrate.log` artifact. Unmanaged candidates (zero evidence) are NOT
/// returned here — they exist only under `--everything`
/// ([`enumerate_all_candidates`]).
pub async fn enumerate_managed(state_dir: &Path) -> Result<Vec<ManagedTarget>> {
    enumerate_targets(state_dir, false).await
}

/// Enumerate EVERY msb sandbox regardless of classification — the
/// `--everything` candidate set. Superset of [`enumerate_managed`]:
/// includes UNMANAGED candidates (present in the msb listing/dir but with
/// zero evidence), reported with `evidence: []`.
pub(crate) async fn enumerate_all_candidates(state_dir: &Path) -> Result<Vec<ManagedTarget>> {
    enumerate_targets(state_dir, true).await
}

async fn enumerate_targets(
    state_dir: &Path,
    include_unmanaged: bool,
) -> Result<Vec<ManagedTarget>> {
    // Store 1: registry records (source-gone instances are ORDINARY records
    // — reconcile.rs gather_facts keeps them registered — so they enter the
    // sweep naturally here). Record-driven targets keep the WORKESTRATE
    // identity in `instance` (down_hardened encodes it back).
    let records = crate::microsandbox::port_registry::list_records(state_dir)?;
    let mut names: Vec<String> = records.iter().map(|r| r.instance.clone()).collect();
    // Store 2: the msb listing (SDK, degrading to the dir fallback).
    // DUAL-SPELLING DEDUP (ADR 0030 addendum 2026-08-26): since the SDK
    // name boundary encodes identities, a record `x@y` and its listed
    // sandbox `x--y-<hash>` are ONE physical target — a listing name is
    // skipped when ANY record matches it directly or under its encoded
    // spelling. Listing-only names (foreign / legacy-unmatched) pass
    // through as today.
    for n in msb_sandbox_names().await {
        let covered_by_record = records.iter().any(|r| {
            n == r.instance || n == crate::microsandbox::slots::msb_name_of_instance(&r.instance)
        });
        if !covered_by_record && !names.contains(&n) {
            names.push(n);
        }
    }
    names.sort();
    let by_name: BTreeMap<&str, &crate::microsandbox::port_registry::SandboxInstanceRecord> =
        records.iter().map(|r| (r.instance.as_str(), r)).collect();
    let mut targets = Vec::with_capacity(names.len());
    for name in &names {
        let record = by_name.get(name.as_str()).copied();
        // Artifact evidence is DUAL-PATH (ADR 0032 addendum 2026-08-30):
        // the state-dir log (RAW identity) first, then the legacy
        // sandbox-dir log whose on-disk name is the ENCODED spelling for
        // anything created after ADR 0030's 2026-08-26 encoding addendum.
        // A record-backed target therefore also probes its identity's
        // encoded dir; listing-derived names keep using the listed (dir)
        // name, as before.
        let artifact = match record {
            Some(r) => {
                artifact_log_exists(name)
                    || artifact_log_exists(&crate::microsandbox::slots::msb_name_of_instance(
                        &r.instance,
                    ))
            }
            None => artifact_log_exists(name),
        };
        let evidence = classify(name, record.map(|r| r.context.as_deref()), artifact);
        if evidence.is_empty() && !include_unmanaged {
            continue;
        }
        targets.push(ManagedTarget {
            instance: name.clone(),
            context: record.and_then(|r| r.context.clone()),
            evidence,
        });
    }
    Ok(targets)
}

/// Resolve a ladder scope against a candidate list (PURE; unit-tested).
///
/// - [`DownScope::Context`] / [`DownScope::ConfigRef`]: include a target
///   iff its RECORD context equals the context (PRIMARY) OR the record is
///   absent/None AND the slot-prefix `<ctx>-` matches (CORROBORATING). A
///   record naming ANOTHER context EXCLUDES despite a slot-prefix match
///   (the record is primary evidence — pinned). Matching is keyed on the
///   record/slot-prefix, NEVER bare-name equality: a workload whose NAME
///   equals a context name is not matched by that fact alone. Zero matches
///   → EMPTY outcome (Ok, reported as `0 target(s)` — never an error).
///   ConfigRef behaves exactly as Context of the ref-implied branch AFTER
///   fail-closed validation ([`validate_config_ref`] at the command
///   boundary — records carry no config-ref stamp, so v1 config-ref scope =
///   validated-branch context scope).
/// - [`DownScope::Home`]: every MANAGED target handed in.
/// - [`DownScope::Everything`]: every target handed in INCLUDING unmanaged
///   ones (pass [`enumerate_all_candidates`]'s output); unmanaged targets
///   keep their empty evidence in the report.
pub fn resolve_scope(scope: &DownScope, targets: &[ManagedTarget]) -> Vec<ManagedTarget> {
    let ctx: Option<&str> = match scope {
        DownScope::Context(c) | DownScope::ConfigRef(c) => Some(c),
        DownScope::Home | DownScope::Everything => None,
    };
    targets
        .iter()
        .filter(|t| match (scope, ctx) {
            // Everything: every candidate, managed or not.
            (DownScope::Everything, _) => true,
            // Home: every MANAGED target (defensively excludes zero-evidence
            // candidates should one ever be handed in).
            (DownScope::Home, _) => !t.evidence.is_empty(),
            (_, Some(c)) => {
                if t.evidence.is_empty() {
                    return false;
                }
                match t.context.as_deref() {
                    // PRIMARY: the record names the context.
                    Some(record_ctx) => record_ctx == c,
                    // CORROBORATING: no record context — the `<ctx>-` slot
                    // prefix decides. Bare-name equality is NEVER enough.
                    None => slot_prefix_matches(&t.instance, c),
                }
            }
            (_, None) => false,
        })
        .cloned()
        .collect()
}

/// True iff the instance's slot (parallel id stripped) starts with
/// `<ctx>-`. The trailing dash is what separates a real context prefix from
/// a bare workload name that merely EQUALS the context (the ambiguity pin:
/// context scope is keyed on record/slot-PREFIX, never bare-name equality).
fn slot_prefix_matches(instance: &str, ctx: &str) -> bool {
    crate::microsandbox::slots::slot_of_instance(instance)
        .strip_prefix(ctx)
        .is_some_and(|rest| rest.starts_with('-'))
}

/// True iff `r` is commit-sha-shaped: exactly 40 hex chars. A sha does not
/// imply a context (ADR 0032 addendum §Selection ladder: branch refs imply
/// contexts; SHAS DO NOT).
pub fn is_sha_like_ref(r: &str) -> bool {
    r.len() == 40 && r.chars().all(|c| c.is_ascii_hexdigit())
}

/// Fail-closed validation of a `--config-ref` scope value (ADR 0032
/// addendum §Down scope ladder): the ref must be BRANCH-shaped — a
/// 40-hex sha is refused ("a sha does not imply a context") — and must
/// resolve against the home's KNOWN refs ([`known_config_refs`]); an
/// unknown ref is a hard error LISTING the known refs.
pub fn validate_config_ref(r: &str, known_refs: &[String]) -> Result<()> {
    if is_sha_like_ref(r) {
        anyhow::bail!(
            "'{r}' looks like a commit sha; a sha does not imply a context — \
             name a branch or use --context"
        );
    }
    if known_refs.iter().any(|k| k == r) {
        return Ok(());
    }
    if known_refs.is_empty() {
        anyhow::bail!(
            "unknown config ref '{r}': this home knows no refs yet \
             (register a config repo or run `workestrate config update`)"
        );
    }
    anyhow::bail!(
        "unknown config ref '{r}'; known refs: {}",
        known_refs.join(", ")
    )
}

/// The home's KNOWN config refs for config-ref validation: registered
/// entries' `ref` fields (registry `config.toml`) plus the lockfile's
/// entry refs and per-entry ref keys (`workestrate.lock` v2). Sorted,
/// deduped.
pub fn known_config_refs() -> Result<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    if let Some(registry) = crate::config::load_registry()? {
        for entry in registry.configs.values() {
            if let Some(r) = &entry.r#ref {
                out.push(r.clone());
            }
        }
    }
    if let Some(lock) = crate::config::lockfile::load_home_lock()? {
        for repo in lock.repos.values() {
            if let Some(r) = &repo.r#ref {
                out.push(r.clone());
            }
            for k in repo.refs.keys() {
                out.push(k.clone());
            }
        }
    }
    out.sort();
    out.dedup();
    Ok(out)
}

/// Resolve the CLI flags into exactly ONE ladder scope (ADR 0032 addendum
/// §Down scope ladder). clap `conflicts_with` already enforces one-selector
/// per invocation; this pure fn is the defensive backstop AND the
/// no-selector usage error: bare `down` names the ladder instead of
/// guessing a scope.
pub fn resolve_cli_scope(
    all: bool,
    context: Option<&str>,
    config_ref: Option<&str>,
    everything_count: u8,
) -> Result<DownScope> {
    if everything_count >= 1 {
        return Ok(DownScope::Everything);
    }
    if let Some(r) = config_ref {
        return Ok(DownScope::ConfigRef(r.to_string()));
    }
    if let Some(c) = context {
        return Ok(DownScope::Context(c.to_string()));
    }
    if all {
        return Ok(DownScope::Home);
    }
    anyhow::bail!(
        "`down` needs exactly ONE scope selector — \
         --all (home) | --context <ctx> | --config-ref <ref> | --everything --everything \
         (ADR 0032 addendum §Down scope ladder: instance < workload < context < config-ref \
         < home < everything; per-workload down stays on `workload <name> down`)"
    )
}

/// The DOUBLE GATE for `--everything` (ADR 0032 addendum §Down scope
/// ladder): TWO INDEPENDENT confirmations — (a) the flag must appear TWICE
/// (`flag_count >= 2`; once is a usage error), and (b) the standard
/// yes-gate: `--yes` skips it, an interactive confirmation passes, a
/// declined prompt aborts, and non-interactive stdin WITHOUT `--yes` is a
/// hard refusal (cmd_clean posture). Pure — the full matrix is unit-tested.
pub fn everything_gate(flag_count: u8, yes: bool, confirmed: Option<bool>) -> Result<()> {
    if flag_count < 2 {
        anyhow::bail!(
            "--everything was given once; repeat it (--everything --everything) \
             to confirm everything-scope teardown"
        );
    }
    if yes {
        return Ok(());
    }
    match confirmed {
        Some(true) => Ok(()),
        Some(false) => anyhow::bail!("aborted"),
        None => anyhow::bail!(
            "refusing to tear down EVERY msb sandbox in non-interactive mode \
             without --yes"
        ),
    }
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

    // ---- classification engine ----

    #[test]
    fn classify_each_evidence_path_alone() {
        // Record only (record present, no dash in slot, no artifact).
        assert_eq!(
            classify("litellm", Some(Some("personal")), false),
            vec![Evidence::RegistryRecord]
        );
        // Record with a None context still counts as a record.
        assert_eq!(
            classify("litellm", Some(None), false),
            vec![Evidence::RegistryRecord]
        );
        // Slot pattern only (dashed slot, no record, no artifact).
        assert_eq!(
            classify("personal-litellm", None, false),
            vec![Evidence::SlotPattern]
        );
        // Artifact only (flat slot, no record, log present).
        assert_eq!(
            classify("foreignthing", None, true),
            vec![Evidence::ArtifactLog]
        );
    }

    #[test]
    fn classify_stacks_multiple_evidence_pieces() {
        assert_eq!(
            classify("personal-litellm@canary", Some(Some("personal")), true),
            vec![
                Evidence::RegistryRecord,
                Evidence::SlotPattern,
                Evidence::ArtifactLog
            ]
        );
        // Parallel id stripped before the dash check: personal-pi@x has a
        // dashed SLOT even though the full name's dash sits left of @.
        assert_eq!(
            classify("personal-pi@canary", None, false),
            vec![Evidence::SlotPattern]
        );
    }

    #[test]
    fn classify_unmanaged_candidate_has_no_evidence() {
        // Present in the msb listing but zero evidence: flat name, no
        // record, no artifact log.
        assert_eq!(classify("plainbox", None, false), vec![]);
        assert_eq!(classify("foreignthing", None, false), vec![]);
    }

    #[test]
    fn classify_slot_pattern_requires_the_dash_shape() {
        // A bare workload name (no context) has NO dash → no slot-pattern
        // evidence even though it may be perfectly legitimate.
        assert_eq!(classify("litellm", None, false), vec![]);
    }

    // ---- scope resolution across the ladder ----

    fn target(instance: &str, context: Option<&str>, evidence: Vec<Evidence>) -> ManagedTarget {
        ManagedTarget {
            instance: instance.to_string(),
            context: context.map(|c| c.to_string()),
            evidence,
        }
    }

    #[test]
    fn resolve_context_includes_record_primary_match() {
        let targets = vec![target(
            "personal-litellm",
            Some("personal"),
            vec![Evidence::RegistryRecord],
        )];
        let picked = resolve_scope(&DownScope::Context("personal".into()), &targets);
        assert_eq!(picked.len(), 1);
        assert_eq!(picked[0].instance, "personal-litellm");
    }

    #[test]
    fn resolve_context_slot_prefix_corroborates_without_record_context() {
        // Record ABSENT entirely: slot prefix corroborates.
        let targets = vec![target("personal-pi", None, vec![Evidence::SlotPattern])];
        assert_eq!(
            resolve_scope(&DownScope::Context("personal".into()), &targets).len(),
            1
        );
        // Record PRESENT but with a None (legacy) context: same verdict.
        let targets = vec![target(
            "personal-pi",
            None,
            vec![Evidence::RegistryRecord, Evidence::SlotPattern],
        )];
        assert_eq!(
            resolve_scope(&DownScope::Context("personal".into()), &targets).len(),
            1
        );
    }

    #[test]
    fn resolve_context_contradicting_record_excludes_despite_slot_match() {
        // The slot prefix says personal-, but the RECORD (primary evidence)
        // names another context → excluded. Pinned.
        let targets = vec![target(
            "personal-litellm",
            Some("work"),
            vec![Evidence::RegistryRecord, Evidence::SlotPattern],
        )];
        let picked = resolve_scope(&DownScope::Context("personal".into()), &targets);
        assert!(picked.is_empty(), "record-primary exclusion failed");
    }

    #[test]
    fn resolve_context_zero_matches_is_an_empty_ok_outcome() {
        let targets = vec![target(
            "work-pi",
            Some("work"),
            vec![Evidence::RegistryRecord],
        )];
        let picked = resolve_scope(&DownScope::Context("nomans".into()), &targets);
        assert!(picked.is_empty(), "zero matches must resolve empty");
    }

    /// AMBIGUITY PIN: a workload name that is ALSO a context name. Context
    /// scope is keyed on RECORD context / slot PREFIX — never bare-name
    /// equality. The bare instance literally named `staging` (no record
    /// context, no dash) is NOT matched by `--context staging`; the
    /// `staging-staging` instance (workload named like its context) IS.
    #[test]
    fn resolve_context_disambiguates_bare_name_equal_to_context() {
        let targets = vec![
            // Bare singleton whose whole name equals the context name.
            target("staging", None, vec![Evidence::RegistryRecord]),
            // Workload named exactly like the context (slot staging-staging).
            target(
                "staging-staging",
                Some("staging"),
                vec![Evidence::RegistryRecord, Evidence::SlotPattern],
            ),
            // Different workload under the same context (prefix match only).
            target("staging-pi", None, vec![Evidence::SlotPattern]),
        ];
        let picked = resolve_scope(&DownScope::Context("staging".into()), &targets);
        let names: Vec<&str> = picked.iter().map(|t| t.instance.as_str()).collect();
        // Input order preserved; the bare "staging" singleton is excluded.
        assert_eq!(names, vec!["staging-staging", "staging-pi"]);
    }

    #[test]
    fn resolve_context_ignores_unmanaged_candidates() {
        let targets = vec![target(
            "personal-foreign",
            None,
            vec![], // unmanaged
        )];
        assert!(resolve_scope(&DownScope::Context("personal".into()), &targets).is_empty());
    }

    #[test]
    fn resolve_home_includes_every_managed_target() {
        let targets = vec![
            target(
                "personal-litellm",
                Some("personal"),
                vec![Evidence::RegistryRecord],
            ),
            target("work-pi", Some("work"), vec![Evidence::SlotPattern]),
            target("legacybox", None, vec![Evidence::ArtifactLog]),
            target("foreignbox", None, vec![]),
        ];
        let picked = resolve_scope(&DownScope::Home, &targets);
        assert_eq!(picked.len(), 3, "home = every MANAGED target");
    }

    #[test]
    fn resolve_everything_includes_unmanaged_with_empty_evidence() {
        let targets = vec![
            target(
                "personal-litellm",
                Some("personal"),
                vec![Evidence::RegistryRecord],
            ),
            target("foreignbox", None, vec![]),
        ];
        let picked = resolve_scope(&DownScope::Everything, &targets);
        assert_eq!(picked.len(), 2, "everything spares nothing");
        let foreign = picked.iter().find(|t| t.instance == "foreignbox").unwrap();
        assert!(
            foreign.evidence.is_empty(),
            "unmanaged candidates report empty evidence honestly"
        );
    }

    #[test]
    fn resolve_config_ref_behaves_as_context_of_the_branch() {
        // PURE half of the v1 pin: after validation, config-ref scope
        // resolves exactly as the context the branch name implies.
        let targets = vec![
            target("feat-x-api", Some("feat-x"), vec![Evidence::RegistryRecord]),
            target("feat-x-pi", None, vec![Evidence::SlotPattern]),
            target("main-api", Some("main"), vec![Evidence::RegistryRecord]),
        ];
        let picked = resolve_scope(&DownScope::ConfigRef("feat-x".into()), &targets);
        let names: Vec<&str> = picked.iter().map(|t| t.instance.as_str()).collect();
        assert_eq!(names, vec!["feat-x-api", "feat-x-pi"]);
    }

    // ---- source-gone instances are ordinary sweep members (pin) ----

    /// A record shaped like a per-dir source-gone instance (source_dir set
    /// to a path that no longer exists — irrelevant to down) must be
    /// INCLUDED in context/home/everything resolutions. It is an ordinary
    /// record; this pins that a future filter cannot silently drop it.
    #[test]
    fn source_gone_record_is_included_in_every_resolution() {
        let gone = target(
            "personal-mover",
            Some("personal"),
            vec![Evidence::RegistryRecord],
        );
        // Shape marker only (the enumeration test builds the REAL record
        // with source_dir); resolution must not care either way.
        let targets = vec![gone];
        assert_eq!(
            resolve_scope(&DownScope::Context("personal".into()), &targets).len(),
            1,
            "context scope includes the source-gone record"
        );
        assert_eq!(resolve_scope(&DownScope::Home, &targets).len(), 1);
        assert_eq!(resolve_scope(&DownScope::Everything, &targets).len(), 1);
    }

    // ---- config-ref validation (fail-closed) ----

    #[test]
    fn is_sha_like_ref_matrix() {
        assert!(is_sha_like_ref(&"a".repeat(40)));
        assert!(is_sha_like_ref("0123456789abcdef0123456789abcdef01234567"));
        assert!(is_sha_like_ref("0123456789ABCDEF0123456789abcdef01234567"));
        assert!(!is_sha_like_ref("main"));
        assert!(!is_sha_like_ref("feat-x"));
        assert!(!is_sha_like_ref(&"a".repeat(39)), "39 hex is not a sha");
        assert!(!is_sha_like_ref(&"a".repeat(41)), "41 chars is not a sha");
        assert!(!is_sha_like_ref(&"g".repeat(40)), "non-hex is not a sha");
    }

    #[test]
    fn validate_config_ref_refuses_shas_naming_the_reason() {
        let err = validate_config_ref(
            "0123456789abcdef0123456789abcdef01234567",
            &["main".to_string()],
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("does not imply a context"),
            "sha refusal must explain why: {err}"
        );
        assert!(
            err.contains("--context"),
            "sha refusal must point at --context: {err}"
        );
    }

    #[test]
    fn validate_config_ref_unknown_lists_known_refs() {
        let known = vec!["main".to_string(), "feat-x".to_string()];
        let err = validate_config_ref("nope", &known).unwrap_err().to_string();
        assert!(err.contains("unknown config ref 'nope'"), "{err}");
        assert!(err.contains("main") && err.contains("feat-x"), "{err}");
    }

    #[test]
    fn validate_config_ref_unknown_on_empty_home_names_the_bootstrap_path() {
        let err = validate_config_ref("nope", &[]).unwrap_err().to_string();
        assert!(err.contains("knows no refs"), "{err}");
    }

    #[test]
    fn validate_config_ref_accepts_known_branches() {
        validate_config_ref("main", &["main".to_string(), "dev".to_string()])
            .expect("known branch must validate");
    }

    // ---- CLI flag → scope resolution ----

    #[test]
    fn resolve_cli_scope_maps_each_selector() {
        assert_eq!(
            resolve_cli_scope(false, Some("personal"), None, 0).unwrap(),
            DownScope::Context("personal".into())
        );
        assert_eq!(
            resolve_cli_scope(false, None, Some("feat-x"), 0).unwrap(),
            DownScope::ConfigRef("feat-x".into())
        );
        assert_eq!(
            resolve_cli_scope(true, None, None, 0).unwrap(),
            DownScope::Home
        );
        assert_eq!(
            resolve_cli_scope(false, None, None, 1).unwrap(),
            DownScope::Everything,
            "count==1 resolves Everything; the DOUBLE GATE refuses it later"
        );
        assert_eq!(
            resolve_cli_scope(false, None, None, 2).unwrap(),
            DownScope::Everything
        );
    }

    #[test]
    fn resolve_cli_scope_bare_down_is_a_usage_error_naming_the_ladder() {
        let err = resolve_cli_scope(false, None, None, 0)
            .unwrap_err()
            .to_string();
        for scope in ["--all", "--context", "--config-ref", "--everything"] {
            assert!(err.contains(scope), "error must list {scope}: {err}");
        }
    }

    // ---- the --everything double gate ----

    #[test]
    fn everything_gate_single_flag_refuses() {
        let err = everything_gate(1, true, None).unwrap_err().to_string();
        assert!(
            err.contains("given once") && err.contains("repeat"),
            "count==1 must demand the repeat: {err}"
        );
        assert!(everything_gate(0, true, None).is_err());
    }

    #[test]
    fn everything_gate_doubled_with_yes_passes() {
        everything_gate(2, true, None).expect("--yes skips the interactive gate");
        everything_gate(3, true, Some(false)).expect("--yes wins over any answer");
    }

    #[test]
    fn everything_gate_doubled_interactive_confirm_passes_and_decline_aborts() {
        everything_gate(2, false, Some(true)).expect("interactive y confirms");
        let err = everything_gate(2, false, Some(false))
            .unwrap_err()
            .to_string();
        assert_eq!(err, "aborted");
    }

    #[test]
    fn everything_gate_doubled_noninteractive_without_yes_hard_refuses() {
        let err = everything_gate(2, false, None).unwrap_err().to_string();
        assert!(
            err.contains("non-interactive") && err.contains("--yes"),
            "cmd_clean posture refusal: {err}"
        );
    }

    // ---- scope descriptions (header line + JSON scope field) ----

    #[test]
    fn scope_description_shapes() {
        assert_eq!(
            DownScope::Context("personal".into()).description(),
            "context 'personal'"
        );
        assert_eq!(
            DownScope::ConfigRef("feat-x".into()).description(),
            "config-ref 'feat-x'"
        );
        assert_eq!(DownScope::Home.description(), "--all (home)");
        assert_eq!(DownScope::Everything.description(), "everything");
    }

    // ---- enumeration (env-guarded; msb made UNREACHABLE so the SDK never
    // initializes its process-global DB pool and the dir fallback runs) ----

    /// RAII guard: point `MSB_HOME` at `path` for the duration of a test
    /// (mirror of the runtime tests' guard; restored on drop).
    struct MsbHomeGuard {
        prior: Option<std::ffi::OsString>,
    }

    impl MsbHomeGuard {
        fn set(path: &std::path::Path) -> Self {
            let prior = std::env::var_os("MSB_HOME");
            std::env::set_var("MSB_HOME", path);
            Self { prior }
        }
    }

    impl Drop for MsbHomeGuard {
        fn drop(&mut self) {
            match &self.prior {
                Some(v) => std::env::set_var("MSB_HOME", v),
                None => std::env::remove_var("MSB_HOME"),
            }
        }
    }

    /// RAII guard: point `WORKESTRATE_STATE_DIR` at `path` for the duration
    /// of a test (mirror of [`MsbHomeGuard`]; restored on drop). Needed
    /// because the artifact-evidence probe resolves the detached log via
    /// `resolve_state_dir()`, whose env override is NOT covered by
    /// `HOME_ENV_KEYS`.
    struct StateDirGuard {
        prior: Option<std::ffi::OsString>,
    }

    impl StateDirGuard {
        fn set(path: &std::path::Path) -> Self {
            let prior = std::env::var_os("WORKESTRATE_STATE_DIR");
            std::env::set_var("WORKESTRATE_STATE_DIR", path);
            Self { prior }
        }
    }

    impl Drop for StateDirGuard {
        fn drop(&mut self) {
            match &self.prior {
                Some(v) => std::env::set_var("WORKESTRATE_STATE_DIR", v),
                None => std::env::remove_var("WORKESTRATE_STATE_DIR"),
            }
        }
    }

    /// Point MSB_HOME at a tmp dir whose `db` path is a regular FILE so the
    /// SDK's init fails (ENOTDIR): `Sandbox::list` errors → the DEGRADED
    /// dir-listing fallback runs, deterministically, without ever pinning
    /// the process-global DB pool.
    fn unreachable_msb_home(label: &str) -> std::path::PathBuf {
        let tmp = crate::config::test_support::uniq_dir(label);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("db"), b"x").unwrap();
        tmp
    }

    fn write_record(
        state_dir: &Path,
        instance: &str,
        context: Option<&str>,
        source_dir: Option<&str>,
    ) {
        let record = crate::microsandbox::port_registry::SandboxInstanceRecord {
            instance: instance.to_string(),
            context: context.map(|c| c.to_string()),
            workload: "wl".to_string(),
            ports: vec![],
            port_pairs: vec![],
            created_at: String::new(),
            bind_ip: crate::microsandbox::plan::default_bind_ip(),
            namespace: crate::microsandbox::port_registry::default_namespace(),
            source_dir: source_dir.map(|s| s.to_string()),
            image_tag: None,
            image_out_hash: None,
            config_hash: None,
        };
        let run_dir = state_dir.join("var").join("run");
        std::fs::create_dir_all(&run_dir).unwrap();
        std::fs::write(
            run_dir.join(format!("{instance}.json")),
            serde_json::to_string(&record).unwrap(),
        )
        .unwrap();
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime tests
    async fn enumerate_managed_unions_registry_and_degraded_dir_listing() -> anyhow::Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let msb = unreachable_msb_home("down-scope-enum");
        let _msb = MsbHomeGuard::set(&msb);

        let state_dir = crate::config::test_support::unique_state_dir_runtime("down-scope-enum");
        let _state = StateDirGuard::set(&state_dir);
        // Registry-only record (no sandbox dir): RegistryRecord evidence.
        write_record(&state_dir, "work-pi", Some("work"), None);
        // Record AND legacy sandbox dir AND log: stacked evidence, deduped.
        // The log stays in the LEGACY sandbox dir (pre-relocation fallback).
        write_record(&state_dir, "personal-litellm", Some("personal"), None);
        let sb_dir = crate::microsandbox::runtime::reconcile::sandbox_dir("personal-litellm");
        std::fs::create_dir_all(&sb_dir)?;
        std::fs::write(sb_dir.join("workestrate.log"), b"managed\n")?;
        // Dir-only WITH log (no record): SlotPattern + ArtifactLog. The log
        // lives at the NEW state-dir location (post-relocation), while the
        // sandbox DIR is still created so the degraded dir listing
        // discovers the candidate.
        let dir_only = crate::microsandbox::runtime::reconcile::sandbox_dir("personal-agent");
        std::fs::create_dir_all(&dir_only)?;
        let agent_log = state_dir
            .join("logs")
            .join("personal-agent")
            .join("workestrate.log");
        std::fs::create_dir_all(agent_log.parent().expect("agent log parent"))?;
        std::fs::write(&agent_log, b"managed\n")?;
        // Dir-only WITHOUT log, dashed name (no record): SlotPattern only.
        let dashed = crate::microsandbox::runtime::reconcile::sandbox_dir("team-worker");
        std::fs::create_dir_all(&dashed)?;
        // Unmanaged dir candidate: flat name, no log, no record.
        let foreign = crate::microsandbox::runtime::reconcile::sandbox_dir("foreignbox");
        std::fs::create_dir_all(&foreign)?;

        let managed = enumerate_managed(&state_dir).await?;
        let instances: Vec<&str> = managed.iter().map(|t| t.instance.as_str()).collect();
        assert_eq!(
            instances,
            vec![
                "personal-agent",
                "personal-litellm",
                "team-worker",
                "work-pi"
            ],
            "union of records + dir fallback, deduped, sorted; unmanaged excluded"
        );

        let litellm = managed
            .iter()
            .find(|t| t.instance == "personal-litellm")
            .unwrap();
        assert_eq!(litellm.context.as_deref(), Some("personal"));
        assert_eq!(
            litellm.evidence,
            vec![
                Evidence::RegistryRecord,
                Evidence::SlotPattern,
                Evidence::ArtifactLog
            ],
            "evidence stacks across stores for the deduped target; the artifact \
             comes via the LEGACY sandbox-dir fallback"
        );
        let agent = managed
            .iter()
            .find(|t| t.instance == "personal-agent")
            .unwrap();
        assert_eq!(agent.context, None);
        assert_eq!(
            agent.evidence,
            vec![Evidence::SlotPattern, Evidence::ArtifactLog],
            "artifact evidence comes via the NEW state-dir log location"
        );
        let work = managed.iter().find(|t| t.instance == "work-pi").unwrap();
        assert_eq!(
            work.evidence,
            vec![Evidence::RegistryRecord, Evidence::SlotPattern],
            "record + dashed slot shape, no artifact (no sandbox dir)"
        );

        // --everything adds the unmanaged candidate with honest empty evidence.
        let everything = enumerate_all_candidates(&state_dir).await?;
        let foreign = everything
            .iter()
            .find(|t| t.instance == "foreignbox")
            .expect("unmanaged candidate must appear under everything-enumeration");
        assert!(foreign.evidence.is_empty());

        let _ = std::fs::remove_dir_all(&state_dir);
        let _ = std::fs::remove_dir_all(&msb);
        Ok(())
    }

    /// SOURCE-GONE SWEEP PIN (packet item 7): a record shaped like a per-dir
    /// source-gone instance (source_dir recorded, the dir itself absent —
    /// irrelevant to down) enters the enumeration as an ORDINARY record and
    /// survives into context/home/everything resolutions.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime tests
    async fn enumerate_managed_includes_source_gone_record_in_all_resolutions() -> anyhow::Result<()>
    {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let msb = unreachable_msb_home("down-scope-source-gone");
        let _msb = MsbHomeGuard::set(&msb);

        let state_dir =
            crate::config::test_support::unique_state_dir_runtime("down-scope-source-gone");
        let _state = StateDirGuard::set(&state_dir);
        write_record(
            &state_dir,
            "personal-mover",
            Some("personal"),
            Some("/nonexistent/per-dir/source"),
        );

        let managed = enumerate_managed(&state_dir).await?;
        assert_eq!(managed.len(), 1, "the source-gone record is enumerated");
        assert_eq!(managed[0].instance, "personal-mover");

        for scope in [
            DownScope::Context("personal".into()),
            DownScope::Home,
            DownScope::Everything,
        ] {
            let picked = resolve_scope(&scope, &managed);
            assert_eq!(
                picked.len(),
                1,
                "source-gone record must survive {:?} resolution",
                scope
            );
        }

        let _ = std::fs::remove_dir_all(&state_dir);
        let _ = std::fs::remove_dir_all(&msb);
        Ok(())
    }

    /// ENUMERATION DEDUP under the msb-name encoding (ADR 0030 addendum
    /// 2026-08-26): a registry record `personal-pi@canary` and its LISTED
    /// sandbox dir named `msb_name_of_instance("personal-pi@canary")` are
    /// ONE physical target — enumerate_managed yields EXACTLY ONE target,
    /// carrying the WORKESTRATE identity (not the encoded name), with
    /// RegistryRecord + ArtifactLog evidence stacked, and context scope
    /// selects it. The log lives at the NEW state-dir location under the
    /// RAW `@` identity (ADR 0032 addendum 2026-08-30) — pinning that the
    /// state-dir probe needs no msb-name encoding — while the encoded
    /// sandbox DIR is still created so the degraded listing dedups.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime tests
    async fn enumerate_managed_dedupes_record_against_encoded_listing_name() -> anyhow::Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let msb = unreachable_msb_home("down-scope-enum-encoded");
        let _msb = MsbHomeGuard::set(&msb);

        let state_dir =
            crate::config::test_support::unique_state_dir_runtime("down-scope-enum-encoded");
        let _state = StateDirGuard::set(&state_dir);
        let identity = "personal-pi@canary";
        write_record(&state_dir, identity, Some("personal"), None);
        // The physical sandbox dir carries the ENCODED msb name (kept for
        // the degraded dir listing's dedup side).
        let encoded = crate::microsandbox::slots::msb_name_of_instance(identity);
        assert_ne!(
            encoded, identity,
            "precondition: the identity needs encoding"
        );
        assert!(
            !encoded.contains('@'),
            "precondition: the encoded name is SDK-legal"
        );
        let sb_dir = crate::microsandbox::runtime::reconcile::sandbox_dir(&encoded);
        std::fs::create_dir_all(&sb_dir)?;
        // The log itself lives at the NEW state-dir location, named by the
        // RAW identity (`@` is legal in dir names; no encoding on this side
        // of the SDK boundary).
        let log_path = state_dir
            .join("logs")
            .join(identity)
            .join("workestrate.log");
        std::fs::create_dir_all(log_path.parent().expect("log parent"))?;
        std::fs::write(&log_path, b"managed\n")?;

        let managed = enumerate_managed(&state_dir).await?;
        assert_eq!(
            managed.len(),
            1,
            "record + its encoded listing dir must dedupe to ONE target"
        );
        assert_eq!(
            managed[0].instance, identity,
            "the target keeps the WORKESTRATE identity, not the encoded name"
        );
        assert!(managed[0].evidence.contains(&Evidence::RegistryRecord));
        assert!(
            managed[0].evidence.contains(&Evidence::ArtifactLog),
            "artifact evidence is found via the RAW-identity state-dir log"
        );

        let picked = resolve_scope(&DownScope::Context("personal".into()), &managed);
        assert_eq!(picked.len(), 1, "context scope selects the deduped target");
        assert_eq!(picked[0].instance, identity);

        let _ = std::fs::remove_dir_all(&state_dir);
        let _ = std::fs::remove_dir_all(&msb);
        Ok(())
    }

    /// DUAL-MATCHING MUST NOT SWALLOW FOREIGN DIRS: a listing-only dir that
    /// merely LOOKS encoded (matches no record's identity or encoding)
    /// still appears — managed via its artifact evidence under home scope
    /// and listed under everything-enumeration. The log stays in the
    /// LEGACY encoded sandbox dir, pinning the legacy fallback for
    /// listing-only names.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime tests
    async fn enumerate_keeps_listing_only_encoded_looking_dirs() -> anyhow::Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let msb = unreachable_msb_home("down-scope-enum-foreign");
        let _msb = MsbHomeGuard::set(&msb);

        let state_dir =
            crate::config::test_support::unique_state_dir_runtime("down-scope-enum-foreign");
        let _state = StateDirGuard::set(&state_dir);
        // An unrelated record to prove matching is per-record, not global.
        write_record(&state_dir, "other-wl", Some("team"), None);
        // Foreign encoded-looking dir: no record matches its identity or
        // its encoding; it carries artifact evidence only.
        let foreign = crate::microsandbox::slots::msb_name_of_instance("team-worker@foreign");
        let sb_dir = crate::microsandbox::runtime::reconcile::sandbox_dir(&foreign);
        std::fs::create_dir_all(&sb_dir)?;
        std::fs::write(sb_dir.join("workestrate.log"), b"foreign\n")?;

        let managed = enumerate_managed(&state_dir).await?;
        assert!(
            managed.iter().any(|t| t.instance == foreign),
            "a listing-only encoded-looking dir must survive dual-spelling dedup"
        );
        let everything = enumerate_all_candidates(&state_dir).await?;
        assert!(
            everything.iter().any(|t| t.instance == foreign),
            "the foreign dir also appears under everything-enumeration"
        );

        let _ = std::fs::remove_dir_all(&state_dir);
        let _ = std::fs::remove_dir_all(&msb);
        Ok(())
    }

    /// known_config_refs unions the registry entries' refs and the lockfile
    /// entry refs + ref KEYS (A5 helpers), sorted and deduped.
    #[test]
    fn known_config_refs_unions_registry_and_lockfile() -> anyhow::Result<()> {
        use crate::config::test_support::{uniq_dir, EnvGuard, ENV_TEST_LOCK};
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _env = EnvGuard::capture(&["WORKESTRATE_HOME"]);
        let home = uniq_dir("down-scope-refs-home");
        std::fs::create_dir_all(&home)?;
        std::env::set_var("WORKESTRATE_HOME", &home);

        // Registry: one entry carrying ref "reg-branch".
        std::fs::write(
            home.join("config.toml"),
            "layers = []\n\n[configs.managed]\nurl = \"https://example.invalid/m.git\"\nref = \"reg-branch\"\n",
        )?;
        // Lockfile: entry ref "lock-main" + extra ref key "feat-x".
        std::fs::write(
            home.join("workestrate.lock"),
            r#"version = 2
home_version = 1
tool_version = "test"

[repos.alpha]
url = "https://example.invalid/a.git"
ref = "lock-main"
rev = "abc"

[repos.alpha.refs.feat-x]
rev = "feat-x"
sha = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
fetched_at = "2026-08-26T00:00:00Z"
"#,
        )?;

        let refs = known_config_refs()?;
        assert_eq!(
            refs,
            vec!["feat-x", "lock-main", "reg-branch"],
            "registry refs + lockfile entry refs + lockfile ref keys, sorted+deduped"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }
}
