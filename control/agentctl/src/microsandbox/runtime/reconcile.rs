//! Shared instance reconciliation (ADR 0030 Phase 0): one authoritative step
//! that gathers facts from all three stores — the port-registry record, the
//! msb sandbox state (DB row + sandbox directory), and host-port service
//! liveness — and computes the conflict-chain disposition (ADR 0030 addendum
//! 2). Used by the named up/exec occupancy gate (runtime), the dep executor
//! (commands/deps.rs), and (for Replace) the create path.

use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;

use crate::config::ConflictStep;
use crate::microsandbox::port_registry::{SandboxInstanceRecord, find_record};
use crate::microsandbox::runtime::time::record_age_secs;
use crate::microsandbox::runtime::wait_for_port;
use microsandbox::MicrosandboxError;
use microsandbox::sandbox::SandboxStatus;

/// Bounded budget for the reuse health probe (host TCP connect to each
/// published port, shared deadline). Short by design: a healthy running
/// instance answers in milliseconds; a dead-port zombie burns at most this
/// budget. (Moved here from commands/deps.rs — shared.)
pub const REUSE_PROBE_TIMEOUT: Duration = Duration::from_millis(500);

/// A slot whose record was created within this window is treated as BOOTING,
/// not a keep-alive zombie: never replace it. (Moved here from
/// commands/deps.rs — shared.)
pub const BOOT_GRACE: Duration = Duration::from_secs(30);

/// Facts gathered per instance (ADR 0030 §4.2): registry record (store 1),
/// msb DB row + status (store 2a), sandbox dir (store 2b), service liveness
/// (store 3), boot grace.
///
/// NOTE: deliberately does NOT derive `PartialEq`/`Eq` — the registry record
/// ([`SandboxInstanceRecord`]) does not implement them.
#[derive(Debug, Clone)]
pub struct ReconcileFacts {
    /// Registry record for the instance, if any.
    pub record: Option<SandboxInstanceRecord>,
    /// msb sandbox status. `None` = no DB row (or msb unavailable — see
    /// [`Self::msb_unavailable`]).
    pub msb_status: Option<SandboxStatus>,
    /// msb itself errored (not SandboxNotFound) — fail-closed per the ADR.
    pub msb_unavailable: bool,
    /// The sandbox directory exists (the msb create gate refuses when it
    /// does — store 2b).
    pub dir_exists: bool,
    /// Host-port liveness probe outcome. `Some(true)` any published port
    /// connects within [`REUSE_PROBE_TIMEOUT`]; `Some(false)` none; `None`
    /// no ports to probe (or msb not Running).
    pub healthy: Option<bool>,
    /// The record's `created_at` is within [`BOOT_GRACE`].
    pub recently_started: bool,
    /// The instance's recorded source directory is gone (ADR 0030 V-addendum
    /// §V3): record present AND `source_dir` recorded AND that path no longer
    /// exists. `source_dir: None` (legacy/unknown posture) is NEVER
    /// source-gone.
    pub source_gone: bool,
}

/// Resolve the msb home dir (`$MSB_HOME` or `~/.microsandbox/current`),
/// mirroring `microsandbox_utils::resolve_home` so the dir check matches
/// the msb create gate exactly: a non-empty `MSB_HOME` is used verbatim, an
/// empty value is treated as unset, else the canonical home — now the
/// `current` generation symlink under `$HOME/.microsandbox` (msb state
/// generations; see [`crate::microsandbox::generation`]) — else
/// `./.microsandbox/current` when HOME is unset (the SDK's `.` fallback).
pub fn msb_home() -> PathBuf {
    if let Some(path) = std::env::var_os("MSB_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(path);
    }
    crate::microsandbox::generation::default_msb_home()
}

/// The msb sandbox directory for `instance` — the directory the msb create
/// gate (`prepare_create_target`) refuses to create over.
pub fn sandbox_dir(instance: &str) -> PathBuf {
    msb_home().join("sandboxes").join(instance)
}

/// The default conflict chain for named verbs (ADR 0030 P0.3 / addendum 2):
/// reuse-if-healthy, start-if-stopped, replace-if-zombie/stale.
pub fn default_chain() -> Vec<ConflictStep> {
    crate::config::DepConflict::default_chain().0
}

/// Gather the facts for `instance` (async: msb status + host-port probe).
/// `declared_ports` are the workload's DECLARED host ports, used as probe
/// fallback targets when the registry record does not exist or carries no
/// ports (mirrors the d452575 probe behavior).
pub async fn gather_facts(
    state_dir: &Path,
    instance: &str,
    declared_ports: &[u16],
) -> Result<ReconcileFacts> {
    let record = find_record(state_dir, instance)?;
    // ADR 0030 addendum 2026-08-26: single encoded-name lookup — records
    // drive the chain decision, so a legacy raw-@ sandbox reads as gone.
    // The encoding lives in the ONE SDK-boundary wrapper [`get_sandbox`].
    let (msb_status, msb_unavailable) = match super::get_sandbox(instance).await {
        Ok(handle) => (Some(handle.status_snapshot()), false),
        Err(MicrosandboxError::SandboxNotFound(_)) => (None, false),
        Err(_) => (None, true),
    };
    let dir_exists = sandbox_dir(instance).exists();
    let healthy = if msb_status == Some(SandboxStatus::Running) {
        probe_health(state_dir, instance, record.as_ref(), declared_ports)?
    } else {
        None
    };
    let recently_started = record
        .as_ref()
        .and_then(|r| record_age_secs(&r.created_at))
        .map(|age| age < BOOT_GRACE.as_secs())
        .unwrap_or(false);
    // ADR 0030 V-addendum §V3: the recorded per-dir source directory (the
    // canonical invocation cwd at create) no longer exists. Legacy records
    // (source_dir None) are never source-gone.
    let source_gone = record
        .as_ref()
        .and_then(|r| r.source_dir.as_deref())
        .is_some_and(|p| !Path::new(p).exists());
    Ok(ReconcileFacts {
        record,
        msb_status,
        msb_unavailable,
        dir_exists,
        healthy,
        recently_started,
        source_gone,
    })
}

/// Short host-port health probe for an already-occupied slot: healthy iff
/// ANY published host port accepts a TCP connect within
/// [`REUSE_PROBE_TIMEOUT`] (shared deadline across ports). Targets come from
/// the slot's registry record (port_pairs/ports) when one exists, else the
/// DECLARED host ports on the shared 127.0.0.1 bind. Returns Ok(None) when
/// there are no ports to probe (a no-port instance cannot be verified
/// cheaply — reuse is then decided optimistically; use `on_conflict =
/// "replace"` to force a fresh start).
fn probe_health(
    state_dir: &Path,
    slot: &str,
    record: Option<&SandboxInstanceRecord>,
    declared_ports: &[u16],
) -> Result<Option<bool>> {
    let targets = probe_targets(state_dir, slot, record, declared_ports);
    if targets.is_empty() {
        return Ok(None);
    }
    let deadline = std::time::Instant::now() + REUSE_PROBE_TIMEOUT;
    for (ip, port) in targets {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        if wait_for_port(ip, port, remaining).is_ok() {
            return Ok(Some(true));
        }
    }
    Ok(Some(false))
}

/// Single-shot probe targets for a slot: the registry record's published
/// pairs/ports when a record exists, else the DECLARED host ports on the
/// shared 127.0.0.1 bind (mirrors `readiness_targets`' fallback, without
/// polling — the caller already knows the record view).
fn probe_targets(
    state_dir: &Path,
    slot: &str,
    record: Option<&SandboxInstanceRecord>,
    declared_ports: &[u16],
) -> Vec<(IpAddr, u16)> {
    // The record is resolved by the caller (gather_facts); `state_dir`/`slot`
    // are retained in the signature for parity with the d452575 probe shape.
    let _ = (state_dir, slot);
    if let Some(rec) = record {
        if !rec.port_pairs.is_empty() {
            return rec.port_pairs.iter().map(|p| (p.bind_ip, p.host)).collect();
        }
        if !rec.ports.is_empty() {
            return rec.ports.iter().map(|&p| (rec.bind_ip, p)).collect();
        }
    }
    declared_ports
        .iter()
        .map(|&p| (IpAddr::V4(Ipv4Addr::LOCALHOST), p))
        .collect()
}

/// A1/P3 pure decision: does an adopted record's context disagree with the
/// invocation's active context? Returns `Some((recorded, active))` — the
/// drift pair to surface — ONLY on a genuine mismatch. `recorded None`
/// (legacy unknown-context record) → `None` (silent). Equal → `None`.
/// `active None` against a recorded context counts as drift; it renders as
/// `"(none)"` in the pair.
pub(crate) fn context_drift(
    record_context: Option<&str>,
    active_context: Option<&str>,
) -> Option<(String, String)> {
    let recorded = record_context?;
    if Some(recorded) == active_context {
        return None;
    }
    Some((
        recorded.to_string(),
        active_context.unwrap_or("(none)").to_string(),
    ))
}

/// A1/P3: warn (never fail) when an adopted record's context disagrees with
/// the invocation's active context. Thin stderr wrapper over the pure
/// [`context_drift`] decision; silent when there is no drift.
pub(crate) fn warn_on_context_drift(
    instance: &str,
    record_context: Option<&str>,
    active_context: Option<&str>,
) {
    if let Some((recorded, active)) = context_drift(record_context, active_context) {
        eprintln!(
            "warning: instance '{}' was registered in context '{}' but the active context is '{}'; \
             proceeding (registry record context is informational only)",
            instance, recorded, active
        );
    }
}

/// The chosen conflict-chain step for a slot (ADR 0030 addendum 2 U1/U2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainStep {
    /// Plain fresh start — the slot is free (no row, no dir, no record);
    /// the chain is irrelevant.
    Start,
    /// `reuse` element: adopt the running healthy/booting instance.
    Reuse,
    /// `start` element: start the stopped/crashed sandbox via
    /// `SandboxHandle::start()`.
    StartExisting,
    /// `replace` element: down/remove + fresh create.
    Replace,
    /// `fail` element: refuse with the standard occupied-instance message.
    Fail,
}

/// The source-gone guidance error (ADR 0030 V-addendum §V3): the recorded
/// source directory no longer exists, so reusing/starting this instance
/// would resurrect mounts against a deleted input. Names the instance and
/// the gone path; the remediation is a NEW instance (re-invoke from the
/// moved directory) or teardown.
fn source_gone_error(facts: &ReconcileFacts, instance: &str) -> Option<anyhow::Error> {
    if !facts.source_gone {
        return None;
    }
    let path = facts
        .record
        .as_ref()
        .and_then(|r| r.source_dir.as_deref())
        .unwrap_or("(unknown)");
    let slot = crate::microsandbox::slots::slot_of_instance(instance);
    Some(anyhow::anyhow!(
        "instance '{instance}' source directory '{path}' no longer exists; \
         re-invoke from the moved directory (plans a new instance), or \
         'workload down {slot}' / down --all to remove it"
    ))
}

/// Decide the disposition for `instance` by iterating `chain` in order and
/// returning the FIRST element whose precondition holds (ADR 0030 addendum 2
/// U2 behavior matrix). Returns Err (chain exhausted) when no element
/// applies; the error lists the attempts in order.
pub fn decide_chain(
    chain: &[ConflictStep],
    facts: &ReconcileFacts,
    instance: &str,
) -> Result<ChainStep> {
    // msb unavailable → fail-closed per the ADR matrix: record present →
    // Fail; else plain Start. (Checked before source-gone, mirroring the
    // classify_status ordering: msb_unavailable wins.)
    if facts.msb_unavailable {
        return Ok(if facts.record.is_some() {
            ChainStep::Fail
        } else {
            ChainStep::Start
        });
    }
    // ADR 0030 V-addendum §V3: a source-gone instance is never reused,
    // started, or chain-replaced — bail with guidance BEFORE chain walking.
    // (`down` sweeps it by name; no down path consults this.)
    if let Some(err) = source_gone_error(facts, instance) {
        return Err(err);
    }
    // Plain start when the slot is free (no row, no dir, no record) — the
    // chain is irrelevant.
    if facts.msb_status.is_none() && !facts.dir_exists && facts.record.is_none() {
        return Ok(ChainStep::Start);
    }
    let mut attempts: Vec<String> = Vec::new();
    for step in chain {
        match step {
            ConflictStep::Reuse => {
                if facts.msb_status == Some(SandboxStatus::Running)
                    && (facts.healthy != Some(false) || facts.recently_started)
                {
                    return Ok(ChainStep::Reuse);
                }
                attempts.push(if facts.msb_status == Some(SandboxStatus::Running) {
                    "reuse (probe failed)".to_string()
                } else {
                    "reuse (not running)".to_string()
                });
            }
            ConflictStep::Start => {
                if matches!(
                    facts.msb_status,
                    Some(SandboxStatus::Stopped) | Some(SandboxStatus::Crashed)
                ) {
                    return Ok(ChainStep::StartExisting);
                }
                attempts.push("start (not applicable)".to_string());
            }
            ConflictStep::Replace => return Ok(ChainStep::Replace),
            ConflictStep::Fail => return Ok(ChainStep::Fail),
        }
    }
    Err(anyhow::anyhow!(
        "conflict chain exhausted for '{}': {} — no strategy succeeded; use --replace or down first",
        instance,
        attempts.join(", ")
    ))
}

/// The chain slice AFTER the first occurrence of `step` (used by executors to
/// advance past a failed `start` element). `None` when `step` is absent.
pub fn chain_after(chain: &[ConflictStep], step: ConflictStep) -> Option<&[ConflictStep]> {
    let pos = chain.iter().position(|s| *s == step)?;
    Some(&chain[pos + 1..])
}

/// Liveness verdict for one registry record's backing sandbox (stale-record
/// GC). Mirrors the `ps` liveness vocabulary (runtime/ps.rs): only a positive
/// `SandboxNotFound` marks a record stale; every other outcome keeps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecordLiveness {
    /// msb resolved the sandbox (any status) — the record is authoritative.
    Live,
    /// msb definitively reports no such sandbox — the record is STALE.
    Gone,
    /// msb itself errored — fail-closed: the record is kept.
    Unknown,
}

/// Prune stale registry records: a record whose backing sandbox msb
/// definitively reports as gone (`SandboxNotFound`) is unregistered so its
/// ports (and parallel-slot loopback IP) stop blocking future `up`s. Records
/// whose sandbox exists (any status) or whose liveness cannot be determined
/// (msb unreachable) are KEPT — fail-closed, matching the `ps` stale flag
/// and the `down_one` NotFound branch.
///
/// Called from `build_sandbox` (runtime/run.rs) BEFORE any registry-reading
/// selection so a record orphaned by a crash, a killed detached child, or an
/// out-of-band `msb rm` cannot wedge its ports until a manual `down`. The
/// OS-bind probe in `port_is_occupied` remains the backstop for ports held
/// by processes outside the registry.
pub async fn prune_stale_records(state_dir: &Path) -> Result<usize> {
    let records = crate::microsandbox::port_registry::list_records(state_dir)?;
    let mut verdicts = Vec::with_capacity(records.len());
    for r in &records {
        // ADR 0030 addendum 2026-08-26: single encoded-name lookup — a
        // legacy raw-@ sandbox reads as gone (record pruned) only on a
        // positive NotFound, exactly as before for legal names.
        verdicts.push(match super::get_sandbox(&r.instance).await {
            Ok(_) => RecordLiveness::Live,
            Err(MicrosandboxError::SandboxNotFound(_)) => RecordLiveness::Gone,
            Err(_) => RecordLiveness::Unknown,
        });
    }
    prune_by_verdicts(state_dir, &records, &verdicts)
}

/// The sync core of [`prune_stale_records`], split out so the prune decision
/// is unit-testable without an msb instance: unregister exactly the records
/// whose verdict is [`RecordLiveness::Gone`]. Unregister failures are
/// best-effort (the `let _ =` teardown idiom) and not counted.
fn prune_by_verdicts(
    state_dir: &Path,
    records: &[SandboxInstanceRecord],
    verdicts: &[RecordLiveness],
) -> Result<usize> {
    debug_assert_eq!(records.len(), verdicts.len());
    let mut pruned = 0usize;
    for (r, v) in records.iter().zip(verdicts) {
        if *v != RecordLiveness::Gone {
            continue;
        }
        eprintln!(
            "pruning stale registry record for '{}' (msb reports no such sandbox)",
            r.instance
        );
        if crate::microsandbox::port_registry::unregister_sandbox(state_dir, &r.instance).is_ok() {
            pruned += 1;
        }
    }
    Ok(pruned)
}

/// ADR 0030 Phase 2 T1: the registry record in `facts` claims the slot for a
/// FOREIGN namespace (its `namespace` differs from the caller's required
/// `namespace`). Returns the record's namespace when so; `None` when there is
/// no record or the namespaces match. A foreign record must never be
/// reused/adopted by a dependent in another namespace (the dep auto-start
/// executor refuses via [`crate::commands::deps::decide_dep_disposition`]); a
/// record-less running sandbox is unaffected.
pub fn foreign_namespace_record<'a>(facts: &'a ReconcileFacts, namespace: &str) -> Option<&'a str> {
    match &facts.record {
        Some(rec) if rec.namespace != namespace => Some(rec.namespace.as_str()),
        _ => None,
    }
}

/// The up/exec chain step with the operator's explicit `--replace` flag
/// applied (ADR 0030 U11 precedence, locked): the flag PREEMPTS the chain —
/// an explicit `--replace` always forces teardown + fresh create
/// ([`ChainStep::Replace`]) on every path (the detached parent's
/// short-circuit and the child's build gate alike), even when the chain would
/// reuse or fail. Without the flag the chain decides unchanged.
pub fn decide_step(
    chain: &[ConflictStep],
    facts: &ReconcileFacts,
    instance: &str,
    replace: bool,
) -> Result<ChainStep> {
    // ADR 0030 V-addendum §V3: source-gone bails BEFORE the --replace
    // preemption too — replacing in place would recreate an instance whose
    // declared input no longer exists; the guidance is re-invoke-from-moved
    // (a NEW instance) or down. (msb-unavailable fail-closed handling stays
    // inside decide_chain; with replace=true the operator's explicit
    // teardown intent still wins over that, as before.)
    if let Some(err) = source_gone_error(facts, instance) {
        return Err(err);
    }
    if replace {
        return Ok(ChainStep::Replace);
    }
    decide_chain(chain, facts, instance)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
#[allow(unsafe_code)]
mod tests {
    use super::*;
    use crate::config::test_support::{ENV_TEST_LOCK, EnvGuard};
    use crate::microsandbox::plan::PortMapping;

    /// Build a minimal record for fact fixtures (only the fields the
    /// decision reads).
    fn record(created_at: &str) -> SandboxInstanceRecord {
        SandboxInstanceRecord {
            instance: "personal-b".to_string(),
            context: Some("personal".to_string()),
            workload: "b".to_string(),
            ports: vec![4000],
            port_pairs: vec![PortMapping::new(4000, 4000)],
            created_at: created_at.to_string(),
            bind_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
            namespace: crate::microsandbox::port_registry::default_namespace(),
            source_dir: None,
            image_tag: None,
            image_out_hash: None,
            config_hash: None,
        }
    }

    /// Build facts for a pure decision test. `msb_status` None + no record +
    /// no dir → free slot.
    fn facts(
        record: Option<SandboxInstanceRecord>,
        msb_status: Option<SandboxStatus>,
        msb_unavailable: bool,
        dir_exists: bool,
        healthy: Option<bool>,
        recently_started: bool,
    ) -> ReconcileFacts {
        ReconcileFacts {
            record,
            msb_status,
            msb_unavailable,
            dir_exists,
            healthy,
            recently_started,
            source_gone: false,
        }
    }

    fn running(healthy: Option<bool>, recently_started: bool) -> ReconcileFacts {
        facts(
            Some(record("2026-01-01T00:00:00Z")),
            Some(SandboxStatus::Running),
            false,
            false,
            healthy,
            recently_started,
        )
    }

    fn free() -> ReconcileFacts {
        facts(None, None, false, false, None, false)
    }

    // ---- A1/P3: context_drift (pure warn decision) ----

    #[test]
    fn context_drift_equal_is_silent() {
        assert_eq!(context_drift(Some("personal"), Some("personal")), None);
        assert_eq!(context_drift(None, None), None);
    }

    #[test]
    fn context_drift_recorded_none_is_silent() {
        // Legacy unknown-context records never warn, whatever is active.
        assert_eq!(context_drift(None, Some("personal")), None);
        assert_eq!(context_drift(None, None), None);
    }

    #[test]
    fn context_drift_mismatch_yields_pair() {
        assert_eq!(
            context_drift(Some("personal"), Some("work")),
            Some(("personal".to_string(), "work".to_string()))
        );
    }

    #[test]
    fn context_drift_recorded_some_active_none_yields_pair() {
        // A namespaced record adopted under bare-layers (no active context)
        // is drift; the active side renders as "(none)".
        assert_eq!(
            context_drift(Some("personal"), None),
            Some(("personal".to_string(), "(none)".to_string()))
        );
    }

    #[test]
    fn warn_on_context_drift_never_panics_and_is_pure_pass_through() {
        // The wrapper only eprintln!s on drift; it must be a no-op (no
        // panic, no control-flow change) for every input combination.
        for recorded in [None, Some("personal"), Some("work")] {
            for active in [None, Some("personal"), Some("work")] {
                warn_on_context_drift("personal-litellm", recorded, active);
            }
        }
    }

    // ---- ADR 0030 addendum 2 U2 behavior matrix (pure decide_chain) ----

    #[test]
    fn matrix_running_healthy_reuses() {
        assert_eq!(
            decide_chain(&default_chain(), &running(Some(true), false), "b").unwrap(),
            ChainStep::Reuse
        );
    }

    #[test]
    fn matrix_zombie_replaces() {
        // Running + dead port + old record → keep-alive zombie → Replace.
        assert_eq!(
            decide_chain(&default_chain(), &running(Some(false), false), "b").unwrap(),
            ChainStep::Replace
        );
    }

    #[test]
    fn matrix_booting_reuses() {
        // Running + dead port but recent record → still booting → Reuse.
        assert_eq!(
            decide_chain(&default_chain(), &running(Some(false), true), "b").unwrap(),
            ChainStep::Reuse
        );
    }

    #[test]
    fn matrix_stopped_starts_existing() {
        let f = facts(
            Some(record("2026-01-01T00:00:00Z")),
            Some(SandboxStatus::Stopped),
            false,
            false,
            None,
            false,
        );
        assert_eq!(
            decide_chain(&default_chain(), &f, "b").unwrap(),
            ChainStep::StartExisting
        );
    }

    #[test]
    fn matrix_crashed_starts_existing() {
        let f = facts(
            Some(record("2026-01-01T00:00:00Z")),
            Some(SandboxStatus::Crashed),
            false,
            false,
            None,
            false,
        );
        assert_eq!(
            decide_chain(&default_chain(), &f, "b").unwrap(),
            ChainStep::StartExisting
        );
    }

    #[test]
    fn matrix_stale_record_replaces() {
        // Record present + msb says the sandbox is gone → stale → Replace.
        let f = facts(
            Some(record("2026-01-01T00:00:00Z")),
            None,
            false,
            false,
            None,
            false,
        );
        assert_eq!(
            decide_chain(&default_chain(), &f, "b").unwrap(),
            ChainStep::Replace
        );
    }

    #[test]
    fn matrix_msb_gone_dir_exists_replaces() {
        // No DB row but the sandbox DIRECTORY exists (store 2b) → the create
        // gate would refuse → Replace.
        let f = facts(None, None, false, true, None, false);
        assert_eq!(
            decide_chain(&default_chain(), &f, "b").unwrap(),
            ChainStep::Replace
        );
    }

    #[test]
    fn matrix_nothing_exists_starts() {
        assert_eq!(
            decide_chain(&default_chain(), &free(), "b").unwrap(),
            ChainStep::Start
        );
    }

    #[test]
    fn matrix_msb_unavailable_with_record_fails() {
        let f = facts(
            Some(record("2026-01-01T00:00:00Z")),
            None,
            true,
            false,
            None,
            false,
        );
        assert_eq!(
            decide_chain(&default_chain(), &f, "b").unwrap(),
            ChainStep::Fail
        );
    }

    #[test]
    fn matrix_msb_unavailable_without_record_starts() {
        let f = facts(None, None, true, false, None, false);
        assert_eq!(
            decide_chain(&default_chain(), &f, "b").unwrap(),
            ChainStep::Start
        );
    }

    #[test]
    fn matrix_custom_fail_chain_occupied_fails() {
        let f = running(Some(true), false);
        assert_eq!(
            decide_chain(&[ConflictStep::Fail], &f, "b").unwrap(),
            ChainStep::Fail
        );
    }

    #[test]
    fn matrix_reuse_only_chain_exhausted_errors() {
        let f = running(Some(false), false);
        let err = decide_chain(&[ConflictStep::Reuse], &f, "b").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("conflict chain exhausted"),
            "exhausted chain must error: {msg}"
        );
        assert!(msg.contains("reuse"), "error must list the attempt: {msg}");
        assert!(
            msg.contains("probe failed"),
            "error must name the failure: {msg}"
        );
    }

    // ---- chain_after ----

    /// The explicit `--replace` flag preempts the conflict chain on every
    /// path (ADR 0030 U11 precedence, locked): an occupied HEALTHY slot that
    /// the chain would reuse is torn down + recreated; a free slot still goes
    /// through the Replace step (the teardown is idempotent). Without the
    /// flag the chain decides unchanged.
    #[test]
    fn replace_flag_preempts_chain_reuse() {
        let occupied_healthy = running(Some(true), false);
        assert_eq!(
            decide_step(&default_chain(), &occupied_healthy, "b", true).unwrap(),
            ChainStep::Replace,
            "--replace must force Replace on an occupied healthy slot, never Reuse"
        );
        assert_eq!(
            decide_step(&default_chain(), &occupied_healthy, "b", false).unwrap(),
            ChainStep::Reuse,
            "without --replace the chain still reuses an occupied healthy slot"
        );
        assert_eq!(
            decide_step(&default_chain(), &free(), "b", true).unwrap(),
            ChainStep::Replace,
            "--replace on a free slot still selects Replace (teardown is idempotent)"
        );
        assert_eq!(
            decide_step(&default_chain(), &free(), "b", false).unwrap(),
            ChainStep::Start,
            "without --replace a free slot is a plain start"
        );
    }

    // ---- ADR 0030 V-addendum §V3: source-gone ----

    /// Facts with a record whose `source_dir` points at a path that does not
    /// exist → source-gone.
    fn source_gone_facts(
        msb_status: Option<SandboxStatus>,
        healthy: Option<bool>,
    ) -> ReconcileFacts {
        let mut rec = record("2026-01-01T00:00:00Z");
        rec.source_dir = Some("/definitely/not/a/real/path/workestrate-test".to_string());
        let mut f = facts(Some(rec), msb_status, false, false, healthy, false);
        f.source_gone = true;
        f
    }

    /// A source-gone record + an explicit target: decide_chain bails with
    /// guidance naming the instance and the gone path, BEFORE chain walking
    /// (even a running-healthy instance is not reused).
    #[test]
    fn source_gone_decide_chain_bails_with_guidance() {
        let f = source_gone_facts(Some(SandboxStatus::Running), Some(true));
        let err = decide_chain(&default_chain(), &f, "pd@work-1234abcd").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("instance 'pd@work-1234abcd' source directory '/definitely/not/a/real/path/workestrate-test' no longer exists"),
            "error must name the instance and the gone path: {msg}"
        );
        assert!(
            msg.contains("re-invoke from the moved directory"),
            "error must point at the new-instance remediation: {msg}"
        );
        assert!(
            msg.contains("workload down pd") && msg.contains("down --all"),
            "error must point at teardown: {msg}"
        );
    }

    /// decide_step bails on source-gone too — INCLUDING under an explicit
    /// --replace (the replace preemption never fires for a gone source).
    #[test]
    fn source_gone_decide_step_bails_even_with_replace() {
        let f = source_gone_facts(Some(SandboxStatus::Running), Some(true));
        assert!(decide_step(&default_chain(), &f, "pd@work-1234abcd", false).is_err());
        let err = decide_step(&default_chain(), &f, "pd@work-1234abcd", true).unwrap_err();
        assert!(
            err.to_string().contains("no longer exists"),
            "replace must not bypass the source-gone bail: {err}"
        );
    }

    /// A LEGACY record (source_dir None) is never source-gone: the chain
    /// disposes normally (running healthy → Reuse).
    #[test]
    fn legacy_none_source_dir_is_never_source_gone() {
        let f = running(Some(true), false);
        assert!(!f.source_gone, "fixture record has no source_dir");
        assert_eq!(
            decide_chain(&default_chain(), &f, "b").unwrap(),
            ChainStep::Reuse,
            "legacy posture must reuse normally"
        );
    }

    /// msb-unavailable beats source-gone in decide_chain (fail-closed Fail,
    /// mirroring the classify_status ordering).
    #[test]
    fn msb_unavailable_beats_source_gone_in_decide_chain() {
        let mut f = source_gone_facts(None, None);
        f.msb_unavailable = true;
        assert_eq!(
            decide_chain(&default_chain(), &f, "b").unwrap(),
            ChainStep::Fail,
            "msb-unavailable fail-closed wins over the source-gone bail"
        );
    }

    /// gather_facts computes source_gone from the record + filesystem: a
    /// recorded source_dir that exists is not source-gone; one that does not
    /// is; a missing source_dir is never source-gone. The msb DB is made
    /// unreachable (MSB_HOME under a regular file — the blocker idiom) so
    /// the SDK's process-global pool is never pinned by this test; the
    /// source_gone fact is record-derived and unaffected by msb state.
    #[tokio::test]
    // ENV_TEST_LOCK held across `.await`: single-threaded test runtime, no
    // spawned tasks — see the prune_stale_records msb-unreachable test.
    #[allow(clippy::await_holding_lock)]
    async fn gather_facts_computes_source_gone() -> Result<()> {
        let _env_lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-gather-source-gone-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::create_dir_all(&tmp)?;
        std::fs::write(tmp.join("blocker"), b"x")?;
        let _msb = MsbHomeGuard::set(&tmp.join("blocker"));

        let dir = crate::config::test_support::unique_state_dir_runtime("gather-source-gone");
        let gone = "/definitely/not/a/real/path/workestrate-gather-test";
        // Record WITH a gone source_dir.
        crate::microsandbox::port_registry::check_and_register_sandbox_lifecycle(
            &dir,
            "gone-src",
            None,
            "gone-src",
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            &[4500],
            &[PortMapping::new(4500, 4500)],
            "2026-01-01T00:00:00Z",
            "default",
            Some(gone),
            None,
            None,
            None,
        )?;
        // Record WITH a live source_dir (the state dir exists).
        crate::microsandbox::port_registry::check_and_register_sandbox_lifecycle(
            &dir,
            "live-src",
            None,
            "live-src",
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            &[4501],
            &[PortMapping::new(4501, 4501)],
            "2026-01-01T00:00:00Z",
            "default",
            Some(dir.to_str().unwrap()),
            None,
            None,
            None,
        )?;
        // Record with NO source_dir (legacy posture — the legacy minimal
        // register writes None).
        crate::microsandbox::port_registry::register_sandbox(
            &dir,
            "legacy-src",
            None,
            "legacy-src",
            &[4502],
        )?;

        let f = gather_facts(&dir, "gone-src", &[]).await?;
        assert!(f.source_gone, "gone recorded source_dir → source_gone");
        let f = gather_facts(&dir, "live-src", &[]).await?;
        assert!(!f.source_gone, "live recorded source_dir → not source_gone");
        let f = gather_facts(&dir, "legacy-src", &[]).await?;
        assert!(!f.source_gone, "legacy None source_dir → never source_gone");
        let f = gather_facts(&dir, "no-record", &[]).await?;
        assert!(!f.source_gone, "no record → not source_gone");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn chain_after_absent_returns_none() {
        assert_eq!(
            chain_after(&[ConflictStep::Reuse], ConflictStep::Start),
            None
        );
    }

    #[test]
    fn chain_after_present_returns_rest() {
        let chain = [
            ConflictStep::Reuse,
            ConflictStep::Start,
            ConflictStep::Replace,
        ];
        assert_eq!(chain_after(&chain, ConflictStep::Start), Some(&chain[2..]));
    }

    // ---- sandbox_dir resolution ----

    /// RAII guard: point `MSB_HOME` at `path` for the duration of a test and
    /// restore the prior value (or unset it) on drop. Mirrors the
    /// `MsbHomeGuard` pattern in runtime/mod.rs tests.
    struct MsbHomeGuard {
        prior: Option<std::ffi::OsString>,
    }

    impl MsbHomeGuard {
        fn set(path: &std::path::Path) -> Self {
            let prior = std::env::var_os("MSB_HOME");
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            unsafe { std::env::set_var("MSB_HOME", path) };
            Self { prior }
        }
    }

    impl Drop for MsbHomeGuard {
        fn drop(&mut self) {
            match &self.prior {
                // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
                Some(v) => unsafe { std::env::set_var("MSB_HOME", v) },
                // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
                None => unsafe { std::env::remove_var("MSB_HOME") },
            }
        }
    }

    #[test]
    fn sandbox_dir_resolves_under_msb_home() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-reconcile-sandbox-dir-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        let _msb = MsbHomeGuard::set(&tmp);
        assert_eq!(
            sandbox_dir("personal-b"),
            tmp.join("sandboxes").join("personal-b")
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sandbox_dir_falls_back_to_home_microsandbox() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let prior_msb = std::env::var_os("MSB_HOME");
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("MSB_HOME") };
        let prior_home = std::env::var_os("HOME");
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-reconcile-home-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", &tmp) };
        assert_eq!(
            sandbox_dir("personal-b"),
            tmp.join(".microsandbox")
                .join("current")
                .join("sandboxes")
                .join("personal-b")
        );
        match prior_msb {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("MSB_HOME", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("MSB_HOME") },
        }
        match prior_home {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("HOME") },
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ---- msb_home: SDK resolve_home mirror (non-empty verbatim) ----

    /// A set non-empty MSB_HOME is used verbatim.
    #[test]
    fn msb_home_set_is_verbatim() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _guard = EnvGuard::capture(&["MSB_HOME", "HOME"]);
        let custom = std::env::temp_dir().join(format!(
            "workestrate-reconcile-msb-home-set-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("MSB_HOME", &custom) };
        assert_eq!(msb_home(), custom);
    }

    /// An empty MSB_HOME is treated as unset (falls back to $HOME).
    #[test]
    fn msb_home_empty_falls_back_to_home() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _guard = EnvGuard::capture(&["MSB_HOME", "HOME"]);
        let fake_home = std::env::temp_dir().join(format!(
            "workestrate-reconcile-msb-home-empty-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", &fake_home) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("MSB_HOME", "") };
        let via_empty = msb_home();
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("MSB_HOME") };
        let via_unset = msb_home();
        assert_eq!(via_empty, via_unset);
        assert_eq!(via_empty, fake_home.join(".microsandbox").join("current"));
    }

    /// Unset MSB_HOME falls back to $HOME/.microsandbox/current (or
    /// ./.microsandbox/current when HOME is also unset) — the `current`
    /// generation symlink is the canonical default home.
    #[test]
    fn msb_home_unset_uses_home_fallback() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _guard = EnvGuard::capture(&["MSB_HOME", "HOME"]);
        let fake_home = std::env::temp_dir().join(format!(
            "workestrate-reconcile-msb-home-unset-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("MSB_HOME") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", &fake_home) };
        assert_eq!(msb_home(), fake_home.join(".microsandbox").join("current"));
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("HOME") };
        assert_eq!(
            msb_home(),
            std::path::PathBuf::from(".")
                .join(".microsandbox")
                .join("current")
        );
    }

    // ---- stale-record GC (prune_stale_records / prune_by_verdicts) ----

    fn register(dir: &std::path::Path, instance: &str, ports: &[u16]) -> Result<()> {
        // A1: keep (instance, workload, context) consistent — a
        // `<ctx>-<wl>` instance registers under Some(<ctx>) with workload
        // `<wl>`; a bare instance registers under None with workload =
        // the instance name.
        if let Some(wl) = instance.strip_prefix("personal-") {
            crate::microsandbox::port_registry::register_sandbox(
                dir,
                instance,
                Some("personal"),
                wl,
                ports,
            )
        } else {
            crate::microsandbox::port_registry::register_sandbox(
                dir, instance, None, instance, ports,
            )
        }
    }

    /// The prune decision core: exactly the records whose verdict is `Gone`
    /// are unregistered; `Live` stays authoritative and `Unknown` (msb
    /// unreachable) is kept — fail-closed, matching the `ps` stale flag.
    #[test]
    fn prune_by_verdicts_removes_only_gone_records() -> Result<()> {
        let dir = crate::config::test_support::unique_state_dir_runtime("prune-verdicts");
        register(&dir, "gone-1", &[4000])?;
        register(&dir, "live-1", &[4001])?;
        register(&dir, "unknown-1", &[4002])?;
        let records = crate::microsandbox::port_registry::list_records(&dir)?;
        assert_eq!(records.len(), 3, "fixture must register three records");
        let verdicts: Vec<RecordLiveness> = records
            .iter()
            .map(|r| match r.instance.as_str() {
                "gone-1" => RecordLiveness::Gone,
                "live-1" => RecordLiveness::Live,
                _ => RecordLiveness::Unknown,
            })
            .collect();
        let pruned = prune_by_verdicts(&dir, &records, &verdicts)?;
        assert_eq!(pruned, 1, "exactly the Gone record is pruned");
        assert!(
            crate::microsandbox::port_registry::find_record(&dir, "gone-1")?.is_none(),
            "the Gone record must be unregistered"
        );
        assert!(
            crate::microsandbox::port_registry::find_record(&dir, "live-1")?.is_some(),
            "a Live record stays authoritative"
        );
        assert!(
            crate::microsandbox::port_registry::find_record(&dir, "unknown-1")?.is_some(),
            "an Unknown (msb-unreachable) record is kept — fail-closed"
        );
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    /// Fail-closed end-to-end: with an UNREACHABLE msb db every verdict is
    /// `Unknown`, so `prune_stale_records` removes nothing. The msb DB is
    /// made unreachable by pointing MSB_HOME at a path under a regular file,
    /// so the SDK's `create_dir_all(<MSB_HOME>/db)` fails with ENOTDIR and
    /// `Sandbox::get` errors out (never NotFound). Mirrors the
    /// `down_all_instances_returns_error_when_msb_db_unreachable` pattern.
    #[tokio::test]
    // ENV_TEST_LOCK held across `.await`: single-threaded test runtime, no
    // spawned tasks — see the runtime/mod.rs Error test for the full note.
    #[allow(clippy::await_holding_lock)]
    async fn prune_stale_records_keeps_records_when_msb_db_unreachable() -> Result<()> {
        let _env_lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-prune-msb-unreachable-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::create_dir_all(&tmp)?;
        std::fs::write(tmp.join("blocker"), b"x")?;
        let _msb = MsbHomeGuard::set(&tmp.join("blocker"));

        let dir = crate::config::test_support::unique_state_dir_runtime("prune-msb-unreachable");
        register(&dir, "personal-litellm", &[4000])?;
        let pruned = prune_stale_records(&dir).await?;
        assert_eq!(pruned, 0, "fail-closed: unreachable msb must prune nothing");
        assert!(
            crate::microsandbox::port_registry::find_record(&dir, "personal-litellm")?.is_some(),
            "the record must survive when liveness cannot be determined"
        );
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }
}
