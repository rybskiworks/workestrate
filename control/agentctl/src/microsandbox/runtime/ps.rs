use super::super::plan::PortMapping;
use super::super::slots;
use super::reconcile::ReconcileFacts;
use anyhow::Result;
use microsandbox::sandbox::SandboxStatus;
use microsandbox::MicrosandboxError;
use std::path::Path;

/// Result of an occupancy probe against the state registry (no msb call).
#[derive(Debug, Clone)]
pub enum Occupancy {
    /// No state record for the instance.
    Free,
    /// A state record exists; the named instance may still be running.
    //
    // Fields carry the occupying identity for diagnostics/refuse messages;
    // the live refuse path currently formats from separately-resolved args
    // (see [`format_refuse_message`]), so these fields are read mainly by
    // tests until the formatter is migrated to consume the enum directly.
    #[allow(dead_code)]
    Occupied {
        occupying_instance: String,
        workload: String,
        context: Option<String>,
    },
}

/// Whether a `ps` row is the singleton instance or a parallel instance.
/// Serialized lowercase to match ADR 0021 §7 (`"singleton"` / `"parallel"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PsKind {
    Singleton,
    Parallel,
}

/// The reconciled status of an instance (ADR 0030 §4.4 + V-addendum §V3
/// `source-gone`): computed
/// from the shared reconcile facts (registry record + msb status + host-port
/// liveness). `stale` (ADR 0021) remains for back-compat in JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStatus {
    /// msb Running + healthy (or no ports to probe, or still booting).
    RunningHealthy,
    /// msb Running + dead port + old record — a keep-alive zombie.
    RunningUnhealthy,
    /// msb Stopped.
    Stopped,
    /// msb Crashed.
    Crashed,
    /// Registry record present + msb gone (no row, no dir) — stale record.
    StaleRecord,
    /// The record's source directory (per-dir create-time cwd) is gone from
    /// the filesystem (ADR 0030 V-addendum §V3).
    SourceGone,
    /// msb unavailable (fail-closed) — status unknown.
    Unknown,
}

impl InstanceStatus {
    /// The human-readable status string for the text renderers (ADR 0030
    /// §4.4). Distinct from the snake_case serde form used in JSON.
    pub fn as_str(&self) -> &'static str {
        match self {
            InstanceStatus::RunningHealthy => "running-healthy",
            InstanceStatus::RunningUnhealthy => "running-unhealthy",
            InstanceStatus::Stopped => "stopped",
            InstanceStatus::Crashed => "crashed",
            InstanceStatus::StaleRecord => "stale-record",
            InstanceStatus::SourceGone => "source-gone",
            InstanceStatus::Unknown => "unknown",
        }
    }
}

/// Classify the status from reconcile facts (PURE — unit-testable).
/// Ordering (ADR 0030 §4.4 + V-addendum §V3): msb-unavailable → Unknown
/// first; then source-gone → SourceGone (a gone source is reported even for
/// a RUNNING instance — its declared input no longer exists); then the
/// existing msb-status logic.
pub fn classify_status(facts: &ReconcileFacts) -> InstanceStatus {
    if facts.msb_unavailable {
        return InstanceStatus::Unknown;
    }
    if facts.source_gone {
        return InstanceStatus::SourceGone;
    }
    match facts.msb_status {
        Some(SandboxStatus::Running) => {
            if facts.healthy == Some(false) && !facts.recently_started {
                InstanceStatus::RunningUnhealthy
            } else {
                InstanceStatus::RunningHealthy
            }
        }
        Some(SandboxStatus::Stopped) => InstanceStatus::Stopped,
        Some(SandboxStatus::Crashed) => InstanceStatus::Crashed,
        // Created/Starting/Draining/Paused → treat as running-ish (healthy
        // unknown); map to RunningHealthy (the reconcile chain treats them
        // as not-replaceable).
        Some(_) => InstanceStatus::RunningHealthy,
        None => {
            if facts.record.is_some() {
                InstanceStatus::StaleRecord
            } else {
                InstanceStatus::Unknown
            }
        }
    }
}

/// Provenance staleness for one `ps` row (ADR 0032 §Provenance stamps):
/// the record's recorded config hash vs the CURRENT build inputs' hash.
/// Both are FULL 16-hex digests — renderers truncate to
/// [`crate::microsandbox::provenance::PROVENANCE_DISPLAY_LEN`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ConfigStaleness {
    /// The config hash stamped on the registry record at create time.
    pub recorded: String,
    /// The config hash over the CURRENT runtime-relevant plan view.
    pub current: String,
}

/// One row of `workestrate ps` output. Pure: no msb calls. The `stale` flag
/// is the exception — it is populated by the async caller via
/// [`probe_liveness`]; [`ps`] itself leaves it `false`. The `status` field is
/// likewise populated by the async caller (via reconcile facts +
/// [`classify_status`]); [`ps`] leaves it `None`.
#[derive(Debug, Clone)]
pub struct PsEntry {
    pub instance: String,
    pub workload: String,
    pub context: Option<String>,
    /// Singleton slot derived from `instance`: the whole string when there is
    /// no `@`, or the part before the first `@` for a parallel instance.
    pub slot: String,
    /// Singleton vs parallel instance (presence of `@` in `instance`).
    pub kind: PsKind,
    /// Effective host:guest port pairs (post-offset host). Legacy records
    /// synthesize `{host: p, guest: p}` from the bare host-port list.
    pub ports: Vec<PortMapping>,
    /// RFC3339 timestamp the instance was registered, or empty for legacy.
    /// Renamed from `created` to match ADR 0021 §7 (`started_at`).
    pub started_at: String,
    /// Best-effort staleness flag; set by [`probe_liveness`] (`false` from
    /// [`ps`]). Surfaced in both `ps --json` and the text footer (ADR 0021 §4).
    pub stale: bool,
    /// Reconciled 5-state status (ADR 0030 §4.4); set by the async caller via
    /// [`classify_status`] (`None` from [`ps`]).
    pub status: Option<InstanceStatus>,
    /// ADR 0032 §Provenance stamps: the record's FULL config hash stamp.
    /// `None` = pre-stamp record (unknown-version posture: NEVER auto-stale,
    /// nothing displayed). Populated by [`ps`]; consumed by the async
    /// caller's staleness computation.
    pub config_hash: Option<String>,
    /// The record's declaring-config-repo namespace (ADR 0030 Phase 2 T1):
    /// the resolution filter deciding whether the ACTIVE config view owns
    /// this row (a foreign-namespace record is never compared).
    pub namespace: String,
    /// Config-hash drift (recorded vs current), set by the async caller
    /// (`cmd_ps`) when both stamps are derivable; `None` from [`ps`] and for
    /// every pre-stamp/unresolvable row (honest unknown — no display).
    pub staleness: Option<ConfigStaleness>,
}

/// The canonical refuse message (text mode). Pinned by ADR 0021 §2.
///
/// Note: the message references the occupying *instance name* (which is the
/// slot for the singleton case, or `slot@id` for a parallel instance). The
/// caller passes the workload's bare name so the suggested `down` command
/// reads naturally (`workestrate workload down <name>` — verb-first,
/// ADR 0027; cleanup phase 4 removed the typed `<name>` subcommands).
pub fn format_refuse_message(workload: &str, occupying_instance: &str) -> String {
    format!(
        "instance '{instance}' is already running. \
         Use --replace to replace it, --instance <id> for a parallel instance, \
         or 'workload down {wl}' to stop it first.",
        instance = occupying_instance,
        wl = workload,
    )
}

/// Pure state-record occupancy probe. Does NOT call msb.
///
/// Returns `Occupied` when a state file exists for `instance` (regardless of
/// whether the backing sandbox is still running). The async caller MUST
/// further verify via `Sandbox::get` to distinguish truly-running from stale.
pub fn occupancy_from_state(state_dir: &Path, instance: &str) -> Result<Occupancy> {
    Ok(
        match super::super::port_registry::find_record(state_dir, instance)? {
            Some(r) => Occupancy::Occupied {
                occupying_instance: r.instance,
                workload: r.workload,
                context: r.context,
            },
            None => Occupancy::Free,
        },
    )
}

/// Pure listing for `workestrate ps`. Reads the registry state files; does
/// NOT call msb (callers may post-process to populate `stale`).
/// CURRENT-GENERATION ONLY by deliberate default (msb state generations):
/// `ps` reflects the RESOLVED home; sweeping every retained generation is
/// a `down` broad-rung behavior, not a `ps` default.
pub fn ps(state_dir: &Path) -> Result<Vec<PsEntry>> {
    let records = super::super::port_registry::list_records(state_dir)?;
    Ok(records
        .into_iter()
        .map(|r| {
            let ports = if r.port_pairs.is_empty() {
                r.ports.iter().map(|&h| PortMapping::new(h, h)).collect()
            } else {
                r.port_pairs
            };
            let kind = if slots::instance_id_of(&r.instance).is_some() {
                PsKind::Parallel
            } else {
                PsKind::Singleton
            };
            let slot = slots::slot_of_instance(&r.instance).to_string();
            PsEntry {
                instance: r.instance,
                workload: r.workload,
                context: r.context,
                slot,
                kind,
                ports,
                started_at: r.created_at,
                stale: false,
                status: None,
                config_hash: r.config_hash,
                namespace: r.namespace,
                staleness: None,
            }
        })
        .collect())
}

/// One instance's liveness, abstracted over the `Sandbox::get` result so the
/// stale/unreachable mapping logic is unit-testable without the SDK's
/// process-global DB pool (ADR 0021 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProbeOutcome {
    /// `Sandbox::get` Ok — genuinely running.
    Alive,
    /// `Sandbox::get` SandboxNotFound — registry record exists, sandbox gone.
    NotFound,
    /// `Sandbox::get` Io / Http / Database — the msb DB (or the SDK's view of
    /// it) could not be reached (e.g. ENOTDIR on the db dir). Stale is left
    /// unchanged; the caller emits ONE honest "could not verify" note.
    Unreachable,
    /// `Sandbox::get` any OTHER error variant — not a reachability failure.
    /// FS-7: previously lumped into Unreachable, which mislabeled arbitrary
    /// SDK errors as "db unreachable". Stale is left unchanged; the caller
    /// emits a DISTINCT stderr note naming the instance and error.
    Unknown,
}

/// Apply a parallel slice of liveness outcomes to `ps` entries in place
/// (ADR 0021 §4): NotFound → `stale = true`; every other outcome leaves
/// `stale` unchanged (honest — only a positive NotFound marks stale).
/// Returns `(unreachable, unknown)` counts so the caller emits ONE honest
/// stderr note per failure class (FS-7: reachability failures and
/// unexpected errors are DISTINCT notes). Pure: no msb, no I/O — the async
/// SDK calls happen in [`probe_liveness`], which delegates here.
fn apply_liveness_outcomes(entries: &mut [PsEntry], outcomes: &[ProbeOutcome]) -> (usize, usize) {
    let mut unreachable = 0usize;
    let mut unknown = 0usize;
    for (e, o) in entries.iter_mut().zip(outcomes) {
        // FS-7: only a positive NotFound writes `stale = true`; every other
        // outcome leaves the flag UNCHANGED. (Previously non-NotFound
        // outcomes forcibly cleared the flag, contradicting the documented
        // "leave stale unchanged" contract for Unreachable.)
        if matches!(o, ProbeOutcome::NotFound) {
            e.stale = true;
        }
        match o {
            ProbeOutcome::Unreachable => unreachable += 1,
            ProbeOutcome::Unknown => unknown += 1,
            _ => {}
        }
    }
    (unreachable, unknown)
}

/// Best-effort liveness probe for a batch of `ps` entries (ADR 0021 §4).
///
/// Mutates each entry's `stale` flag in place and returns
/// `(unreachable, unknown)` counts so the caller emits ONE honest stderr note
/// per failure class. Per-instance outcomes (FS-7):
///
/// - `Sandbox::get` Ok              → genuinely running          → `stale` unchanged.
/// - `Sandbox::get` SandboxNotFound → state record, sandbox gone → `stale = true`.
/// - `Sandbox::get` Io / Http / Database → msb DB unreachable   → `stale` left
///   unchanged; counted in `.0` (reachability failure class).
/// - `Sandbox::get` any OTHER error → unexpected SDK error      → `stale` left
///   unchanged; counted in `.1` AND noted individually on stderr (distinct
///   from the aggregate unreachable note, since these are not reachability
///   failures and were previously mislabeled as such).
///
/// [`ps`] stays pure (no msb); this is the only place `ps` rows touch msb,
/// keeping the pure function unit-testable. Sequential — N is small (rarely
/// more than a handful of instances per host). The stale/unreachable mapping
/// itself lives in the pure [`apply_liveness_outcomes`].
pub async fn probe_liveness(entries: &mut [PsEntry]) -> (usize, usize) {
    // Probe each instance sequentially via the SDK, collecting outcomes before
    // delegating the stale/unreachable mapping to the pure helper. Collecting
    // first means the immutable borrow of `entries` (for e.instance) is
    // released before apply_liveness_outcomes takes it mutably.
    let mut outcomes: Vec<ProbeOutcome> = Vec::with_capacity(entries.len());
    for e in entries.iter() {
        // ADR 0030 addendum 2026-08-26: probe the ENCODED msb name (records
        // keep the workestrate identity; a legacy raw-@ sandbox simply reads
        // as gone → stale). The encoding lives in the ONE SDK-boundary
        // wrapper [`super::get_sandbox`].
        let outcome = match super::get_sandbox(&e.instance).await {
            Ok(_) => ProbeOutcome::Alive,
            Err(MicrosandboxError::SandboxNotFound(_)) => ProbeOutcome::NotFound,
            // FS-7: match the error variants the SDK actually exposes for
            // reachability — Io (ENOTDIR/EACCES on the db dir), Http (SDK
            // transport), Database (sea-orm open/query failure). Everything
            // else is NOT a reachability failure.
            Err(MicrosandboxError::Io(_))
            | Err(MicrosandboxError::Http(_))
            | Err(MicrosandboxError::Database(_)) => ProbeOutcome::Unreachable,
            Err(other) => {
                eprintln!(
                    "note: liveness probe for '{}' returned an unexpected error ({other});                      leaving stale unchanged",
                    e.instance
                );
                ProbeOutcome::Unknown
            }
        };
        outcomes.push(outcome);
    }
    apply_liveness_outcomes(entries, &outcomes)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::{
        apply_liveness_outcomes, classify_status, format_refuse_message, occupancy_from_state,
        probe_liveness, ps, InstanceStatus, Occupancy, ProbeOutcome, PsEntry, PsKind,
    };
    use crate::config::test_support::unique_state_dir_runtime;
    use crate::microsandbox::plan::PortMapping;
    // The msb / MSB_HOME-mutating tests below mutate the process-global
    // MSB_HOME env var. They MUST run single-file with every other env-mutating
    // test (notably the `config::tests` family, which serialize via
    // `crate::config::test_support::ENV_TEST_LOCK`): an unsynchronized `set_var`
    // racing with a config test's env read panics that test while it holds
    // ENV_TEST_LOCK, poisoning the mutex and cascading ~30 PoisonError failures.
    // Acquiring ENV_TEST_LOCK (the SAME lock the config tests use) makes the
    // whole env-mutating group mutually exclusive. `serial_test`'s separate
    // group cannot help here because the config tests do not use it.

    #[test]
    fn refuse_message_is_byte_identical_to_pinned_text() {
        // The text below is pinned by ADR 0021 §2 and asserted by external
        // tooling; do not rephrase without coordinating with the docs track.
        // (Cleanup phase 4: the `down` hint moved to the canonical verb-first
        // form — the typed `<name> down` subcommands were removed.)
        let msg = format_refuse_message("litellm", "personal-litellm");
        let expected = "instance 'personal-litellm' is already running. \
         Use --replace to replace it, --instance <id> for a parallel instance, \
         or 'workload down litellm' to stop it first.";
        assert_eq!(msg, expected, "refuse message drifted from pinned text");
    }

    #[test]
    fn refuse_message_for_parallel_instance_names_slot_at_id() {
        let msg = format_refuse_message("litellm", "personal-litellm@canary");
        assert!(
            msg.contains("instance 'personal-litellm@canary' is already running"),
            "refuse message must name the occupying instance verbatim: {msg}"
        );
    }

    #[test]
    fn occupancy_from_state_free_when_no_record() -> anyhow::Result<()> {
        let dir = unique_state_dir_runtime("occ-free");
        match occupancy_from_state(&dir, "absent")? {
            Occupancy::Free => {}
            other => panic!("expected Free, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn occupancy_from_state_occupied_when_record_present() -> anyhow::Result<()> {
        let dir = unique_state_dir_runtime("occ-set");
        crate::microsandbox::port_registry::register_sandbox(
            &dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        match occupancy_from_state(&dir, "personal-litellm")? {
            Occupancy::Occupied {
                occupying_instance,
                workload,
                context,
            } => {
                assert_eq!(occupying_instance, "personal-litellm");
                assert_eq!(workload, "litellm");
                assert_eq!(context.as_deref(), Some("personal"));
            }
            other => panic!("expected Occupied, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn occupancy_from_state_free_for_corrupt_record() -> anyhow::Result<()> {
        let dir = unique_state_dir_runtime("occ-corrupt");
        let run_dir = dir.join("var").join("run");
        std::fs::create_dir_all(&run_dir)?;
        std::fs::write(run_dir.join("corrupt.json"), "not json")?;
        match occupancy_from_state(&dir, "corrupt")? {
            Occupancy::Free => {}
            other => panic!("corrupt record should be treated as Free, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn ps_returns_empty_when_no_records() -> anyhow::Result<()> {
        let dir = unique_state_dir_runtime("ps-empty");
        let entries = ps(&dir)?;
        assert!(entries.is_empty(), "expected empty ps list");
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn ps_lists_lifecycle_records_with_port_pairs() -> anyhow::Result<()> {
        let dir = unique_state_dir_runtime("ps-lifecycle");
        let pairs = vec![PortMapping::new(14000, 4000), PortMapping::new(14001, 4001)];
        crate::microsandbox::port_registry::register_sandbox_lifecycle(
            &dir,
            "personal-litellm@canary",
            Some("personal"),
            "litellm",
            crate::microsandbox::plan::default_bind_ip(),
            &[14000, 14001],
            &pairs,
            "2026-07-20T14:05:42Z",
            "default",
            None,
            // A2 (ADR 0032 §Image tags): no running tag known at this site.
            None,
            None,
            None,
        )?;
        let entries = ps(&dir)?;
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.instance, "personal-litellm@canary");
        assert_eq!(e.workload, "litellm");
        assert_eq!(e.context.as_deref(), Some("personal"));
        assert_eq!(e.ports.len(), 2);
        assert_eq!(e.ports[0].host, 14000);
        assert_eq!(e.ports[0].guest, 4000);
        assert_eq!(e.started_at, "2026-07-20T14:05:42Z");
        // ADR 0021 §7 derived fields.
        assert_eq!(e.slot, "personal-litellm");
        assert_eq!(e.kind, PsKind::Parallel);
        assert!(!e.stale);
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn ps_preserves_port_names_through_registry_round_trip() -> anyhow::Result<()> {
        let dir = unique_state_dir_runtime("ps-named");
        // P3: a lifecycle record whose port_pairs carry declared names.
        let pairs = vec![
            PortMapping {
                host: 14000,
                guest: 4000,
                bind_ip: crate::microsandbox::plan::default_bind_ip(),
                name: Some("api".to_string()),
            },
            PortMapping::new(14001, 4001),
        ];
        crate::microsandbox::port_registry::register_sandbox_lifecycle(
            &dir,
            "personal-litellm@canary",
            Some("personal"),
            "litellm",
            crate::microsandbox::plan::default_bind_ip(),
            &[14000, 14001],
            &pairs,
            "2026-07-20T14:05:42Z",
            "default",
            None,
            // A2 (ADR 0032 §Image tags): no running tag known at this site.
            None,
            None,
            None,
        )?;
        let entries = ps(&dir)?;
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.ports.len(), 2);
        // P3: names flow through the registry file into ps() intact.
        assert_eq!(
            e.ports[0].name.as_deref(),
            Some("api"),
            "named pair must survive the registry round-trip"
        );
        assert_eq!(e.ports[0].host, 14000);
        assert_eq!(e.ports[0].guest, 4000);
        // Unnamed pair stays None.
        assert_eq!(e.ports[1].name, None);
        assert_eq!(e.ports[1].host, 14001);
        assert_eq!(e.ports[1].guest, 4001);
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn ps_synthesizes_port_pairs_for_legacy_records() -> anyhow::Result<()> {
        let dir = unique_state_dir_runtime("ps-legacy");
        // Legacy record: only the bare host-port list; no port_pairs. Written
        // directly — a pre-A1 on-disk record could carry a namespaced-looking
        // instance with context null (the A1 write-side refuse only gates NEW
        // registrations; legacy records still load).
        let run_dir = dir.join("var").join("run");
        std::fs::create_dir_all(&run_dir)?;
        std::fs::write(
            run_dir.join("legacy-litellm.json"),
            r#"{
  "instance": "legacy-litellm",
  "context": null,
  "workload": "litellm",
  "ports": [4000, 4001]
}"#,
        )?;
        let entries = ps(&dir)?;
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.ports.len(), 2);
        // Legacy: host==guest in the synthesized pairs.
        assert_eq!(e.ports[0].host, 4000);
        assert_eq!(e.ports[0].guest, 4000);
        assert_eq!(e.ports[1].host, 4001);
        assert_eq!(e.ports[1].guest, 4001);
        // P3: synthesized PortMapping::new(h, h) carries name: None.
        assert_eq!(e.ports[0].name, None);
        assert_eq!(e.ports[1].name, None);
        // Legacy record: singleton (no `@`), slot == instance.
        assert_eq!(e.slot, "legacy-litellm");
        assert_eq!(e.kind, PsKind::Singleton);
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    /// RAII guard: point `MSB_HOME` at `path` for the duration of a test and
    /// restore the prior value (or unset it) on drop. The microsandbox SDK
    /// resolves its DB home from `MSB_HOME` inside `db::init_global`, which is
    /// a process-global `OnceCell` — so the value MUST be set before the first
    /// `Sandbox::get` in the process, and restored afterwards so other tests /
    /// the devshell are not disturbed.
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

    // ---- ADR 0021 §4 liveness probing (probe_liveness) ----

    /// `probe_liveness` with an UNREACHABLE msb db must leave every entry's
    /// `stale` flag `false` and report a non-zero unreachable count, so the
    /// caller can emit one honest stderr note. The msb DB is made unreachable
    /// by pointing MSB_HOME at a path under a regular file (ENOTDIR on
    /// `<MSB_HOME>/db`), exactly as the WP-E Error test does.
    ///
    /// Deterministic in normal `cargo test`: a failing `init_global` does NOT
    /// pin the SDK's process-global DB pool, so this test cannot contaminate
    /// the WP-E Error test (or vice-versa) — both see a fresh failed init.
    #[tokio::test]
    // ENV_TEST_LOCK held across `.await`: safe on the single-threaded
    // current-thread test runtime (no spawned tasks, no yield onto a contended
    // lock). Spans the await so the set_var(MSB_HOME) cannot race a config
    // test's env read. Mirrors the WP-E Error test's proven idiom.
    #[allow(clippy::await_holding_lock)]
    async fn probe_liveness_leaves_stale_false_when_msb_db_unreachable() -> anyhow::Result<()> {
        let _env_lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-ps-stale-unreachable-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::create_dir_all(&tmp)?;
        std::fs::write(tmp.join("blocker"), b"x")?;
        let _msb = MsbHomeGuard::set(&tmp.join("blocker"));

        let dir = unique_state_dir_runtime("probe-liveness-unreachable");
        crate::microsandbox::port_registry::register_sandbox(
            &dir,
            "personal-litellm@canary",
            Some("personal"),
            "litellm",
            &[14000],
        )?;
        let mut entries = ps(&dir)?;
        assert_eq!(entries.len(), 1, "one record listed");
        assert!(!entries[0].stale, "ps() must default stale=false");

        let (unreachable, unknown) = probe_liveness(&mut entries).await;
        assert_eq!(
            unreachable, 1,
            "the single entry's liveness could not be verified (db unreachable)"
        );
        assert_eq!(unknown, 0, "a db-reachability failure is not Unknown");
        assert!(
            !entries[0].stale,
            "unreachable db must NOT mark stale; got stale=true"
        );

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    /// Pure mapping logic for `ps` liveness (ADR 0021 §4). Tests the three
    /// outcome → (stale, unreachable) branches directly against
    /// `apply_liveness_outcomes`, WITHOUT the SDK's process-global DB pool.
    ///
    /// Why not an msb-DB integration test for the NotFound→stale branch: the
    /// SDK pins its SQLite pool in a process-global OnceCell on the first
    /// *successful* `init_global`. A writable-MSB_HOME test that triggers
    /// NotFound PINS the pool at its tmp dir and then deletes it on cleanup,
    /// invalidating the pool for any later test in the same process (proven
    /// empirically: it flips a sibling `#[ignore]`'d NotFound test to a hard
    /// error). Two such tests therefore cannot coexist in one `cargo test`
    /// invocation. The mapping logic — which is OURS to get right — is fully
    /// covered here; the SDK's Ok/NotFound/Err behavior is already proven by
    /// the WP-E Error + NotFound tests. The end-to-end Unreachable path is
    /// covered by `probe_liveness_leaves_stale_false_when_msb_db_unreachable`
    /// above (blocker MSB_HOME, no pool pin, deterministic).
    #[test]
    fn apply_liveness_outcomes_marks_not_found_stale_and_counts_unreachable() {
        use crate::microsandbox::plan::PortMapping;
        // Three entries: NotFound (stale), Alive (not stale), Unreachable
        // (not stale, counted). The pure helper must set stale per outcome and
        // return the Unreachable count, regardless of any SDK state.
        let mut entries = vec![
            PsEntry {
                instance: "a@x".into(),
                workload: "w".into(),
                context: None,
                slot: "a".into(),
                kind: PsKind::Parallel,
                ports: vec![PortMapping::new(1, 1)],
                started_at: String::new(),
                stale: false,
                status: None,
                config_hash: None,
                namespace: crate::microsandbox::port_registry::default_namespace(),
                staleness: None,
            },
            PsEntry {
                instance: "b".into(),
                workload: "w".into(),
                context: None,
                slot: "b".into(),
                kind: PsKind::Singleton,
                ports: vec![],
                started_at: String::new(),
                stale: false,
                status: None,
                config_hash: None,
                namespace: crate::microsandbox::port_registry::default_namespace(),
                staleness: None,
            },
            PsEntry {
                instance: "c@y".into(),
                workload: "w".into(),
                context: None,
                slot: "c".into(),
                kind: PsKind::Parallel,
                ports: vec![],
                started_at: String::new(),
                stale: false,
                status: None,
                config_hash: None,
                namespace: crate::microsandbox::port_registry::default_namespace(),
                staleness: None,
            },
        ];
        let outcomes = [
            ProbeOutcome::NotFound,    // a@x → stale
            ProbeOutcome::Alive,       // b   → not stale
            ProbeOutcome::Unreachable, // c@y → not stale, counted
        ];
        let (unreachable, unknown) = apply_liveness_outcomes(&mut entries, &outcomes);
        assert_eq!(unreachable, 1, "exactly one Unreachable outcome");
        assert_eq!(unknown, 0, "no Unknown outcome in this slice");
        assert!(entries[0].stale, "NotFound → stale=true");
        assert!(!entries[1].stale, "Alive → stale=false");
        assert!(
            !entries[2].stale,
            "Unreachable → stale=false (honest, not stale)"
        );
    }

    /// FS-7: an Unknown (non-reachability) outcome leaves stale unchanged
    /// (does NOT clear a pre-existing stale flag — only NotFound writes
    /// stale) and is counted in the DISTINCT unknown bucket, not lumped into
    /// the unreachable count.
    #[test]
    fn apply_liveness_outcomes_unknown_is_distinct_and_preserves_stale() {
        use crate::microsandbox::plan::PortMapping;
        let mut entries = vec![
            PsEntry {
                instance: "a".into(),
                workload: "w".into(),
                context: None,
                slot: "a".into(),
                kind: PsKind::Singleton,
                ports: vec![PortMapping::new(1, 1)],
                started_at: String::new(),
                stale: true, // pre-existing; Unknown must NOT overwrite it
                status: None,
                config_hash: None,
                namespace: crate::microsandbox::port_registry::default_namespace(),
                staleness: None,
            },
            PsEntry {
                instance: "b".into(),
                workload: "w".into(),
                context: None,
                slot: "b".into(),
                kind: PsKind::Singleton,
                ports: vec![],
                started_at: String::new(),
                stale: false,
                status: None,
                config_hash: None,
                namespace: crate::microsandbox::port_registry::default_namespace(),
                staleness: None,
            },
        ];
        let outcomes = [ProbeOutcome::Unknown, ProbeOutcome::Unknown];
        let (unreachable, unknown) = apply_liveness_outcomes(&mut entries, &outcomes);
        assert_eq!(unreachable, 0, "Unknown is not an Unreachable outcome");
        assert_eq!(unknown, 2, "both outcomes counted as Unknown");
        assert!(
            entries[0].stale,
            "Unknown leaves a pre-existing stale flag unchanged"
        );
        assert!(!entries[1].stale, "Unknown leaves stale=false unchanged");
    }

    /// `apply_liveness_outcomes` with all-Alive reports zero failures and
    /// leaves stale flags unchanged (Alive does not mark stale; a
    /// pre-existing stale flag persists — only NotFound writes stale).
    #[test]
    fn apply_liveness_outcomes_all_alive_is_clean() {
        let mut entries = vec![
            PsEntry {
                instance: "a".into(),
                workload: "w".into(),
                context: None,
                slot: "a".into(),
                kind: PsKind::Singleton,
                ports: vec![],
                started_at: String::new(),
                stale: true, // start stale; Alive must clear it
                status: None,
                config_hash: None,
                namespace: crate::microsandbox::port_registry::default_namespace(),
                staleness: None,
            },
            PsEntry {
                instance: "b".into(),
                workload: "w".into(),
                context: None,
                slot: "b".into(),
                kind: PsKind::Singleton,
                ports: vec![],
                started_at: String::new(),
                stale: true,
                status: None,
                config_hash: None,
                namespace: crate::microsandbox::port_registry::default_namespace(),
                staleness: None,
            },
        ];
        let outcomes = [ProbeOutcome::Alive, ProbeOutcome::Alive];
        let (unreachable, unknown) = apply_liveness_outcomes(&mut entries, &outcomes);
        assert_eq!(unreachable, 0);
        assert_eq!(unknown, 0);
        assert!(
            entries[0].stale,
            "Alive leaves a pre-existing stale flag unchanged (only NotFound writes stale)"
        );
        assert!(
            entries[1].stale,
            "Alive leaves a pre-existing stale flag unchanged (only NotFound writes stale)"
        );
    }

    // ---- ADR 0030 §4.4 5-state status classification (classify_status) ----

    use crate::microsandbox::port_registry::SandboxInstanceRecord;
    use crate::microsandbox::runtime::reconcile::ReconcileFacts;
    use microsandbox::sandbox::SandboxStatus;

    /// Minimal registry record for fact fixtures (only the fields the
    /// classifier reads).
    fn record() -> SandboxInstanceRecord {
        SandboxInstanceRecord {
            instance: "personal-b".to_string(),
            context: Some("personal".to_string()),
            workload: "b".to_string(),
            ports: vec![4000],
            port_pairs: vec![PortMapping::new(4000, 4000)],
            created_at: "2026-01-01T00:00:00Z".to_string(),
            bind_ip: std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            namespace: crate::microsandbox::port_registry::default_namespace(),
            source_dir: None,
            image_tag: None,
            image_out_hash: None,
            config_hash: None,
        }
    }

    fn facts(
        record: Option<SandboxInstanceRecord>,
        msb_status: Option<SandboxStatus>,
        msb_unavailable: bool,
        healthy: Option<bool>,
        recently_started: bool,
    ) -> ReconcileFacts {
        ReconcileFacts {
            record,
            msb_status,
            msb_unavailable,
            dir_exists: false,
            healthy,
            recently_started,
            source_gone: false,
        }
    }

    #[test]
    fn classify_status_running_healthy() {
        let f = facts(
            Some(record()),
            Some(SandboxStatus::Running),
            false,
            Some(true),
            false,
        );
        assert_eq!(classify_status(&f), InstanceStatus::RunningHealthy);
    }

    #[test]
    fn classify_status_running_no_ports_healthy() {
        let f = facts(
            Some(record()),
            Some(SandboxStatus::Running),
            false,
            None,
            false,
        );
        assert_eq!(classify_status(&f), InstanceStatus::RunningHealthy);
    }

    #[test]
    fn classify_status_running_booting_healthy() {
        let f = facts(
            Some(record()),
            Some(SandboxStatus::Running),
            false,
            Some(false),
            true,
        );
        assert_eq!(classify_status(&f), InstanceStatus::RunningHealthy);
    }

    #[test]
    fn classify_status_running_zombie() {
        let f = facts(
            Some(record()),
            Some(SandboxStatus::Running),
            false,
            Some(false),
            false,
        );
        assert_eq!(classify_status(&f), InstanceStatus::RunningUnhealthy);
    }

    #[test]
    fn classify_status_stopped() {
        let f = facts(
            Some(record()),
            Some(SandboxStatus::Stopped),
            false,
            None,
            false,
        );
        assert_eq!(classify_status(&f), InstanceStatus::Stopped);
    }

    #[test]
    fn classify_status_crashed() {
        let f = facts(
            Some(record()),
            Some(SandboxStatus::Crashed),
            false,
            None,
            false,
        );
        assert_eq!(classify_status(&f), InstanceStatus::Crashed);
    }

    #[test]
    fn classify_status_stale_record() {
        let f = facts(Some(record()), None, false, None, false);
        assert_eq!(classify_status(&f), InstanceStatus::StaleRecord);
    }

    #[test]
    fn classify_status_msb_unavailable() {
        let f = facts(Some(record()), None, true, None, false);
        assert_eq!(classify_status(&f), InstanceStatus::Unknown);
    }

    #[test]
    fn classify_status_no_record_no_msb() {
        let f = facts(None, None, false, None, false);
        assert_eq!(classify_status(&f), InstanceStatus::Unknown);
    }

    // ---- ADR 0030 V-addendum §V3: source-gone classification ----

    fn source_gone_facts(
        msb_status: Option<SandboxStatus>,
        msb_unavailable: bool,
        healthy: Option<bool>,
    ) -> ReconcileFacts {
        let mut rec = record();
        rec.source_dir = Some("/definitely/not/a/real/path/workestrate-ps-test".to_string());
        let mut f = facts(Some(rec), msb_status, msb_unavailable, healthy, false);
        f.source_gone = true;
        f
    }

    /// source-gone beats RUNNING: a running instance whose recorded source
    /// directory is gone reports source-gone, not running-healthy.
    #[test]
    fn classify_status_source_gone_beats_running() {
        let f = source_gone_facts(Some(SandboxStatus::Running), false, Some(true));
        assert_eq!(classify_status(&f), InstanceStatus::SourceGone);
    }

    /// msb-unavailable beats source-gone (fail-closed Unknown first).
    #[test]
    fn classify_status_msb_unavailable_beats_source_gone() {
        let f = source_gone_facts(None, true, None);
        assert_eq!(classify_status(&f), InstanceStatus::Unknown);
    }

    /// source-gone also beats the stopped/crashed/stale-record arms.
    #[test]
    fn classify_status_source_gone_beats_stopped_and_stale() {
        let f = source_gone_facts(Some(SandboxStatus::Stopped), false, None);
        assert_eq!(classify_status(&f), InstanceStatus::SourceGone);
        let f = source_gone_facts(None, false, None);
        assert_eq!(classify_status(&f), InstanceStatus::SourceGone);
    }

    /// The text form is "source-gone" and the serde form is snake_case
    /// `source_gone`.
    #[test]
    fn source_gone_status_strings() {
        assert_eq!(InstanceStatus::SourceGone.as_str(), "source-gone");
        assert_eq!(
            serde_json::to_string(&InstanceStatus::SourceGone).unwrap(),
            "\"source_gone\""
        );
    }
}
