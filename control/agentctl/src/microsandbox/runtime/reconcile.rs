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
use crate::microsandbox::port_registry::{find_record, SandboxInstanceRecord};
use crate::microsandbox::runtime::time::record_age_secs;
use crate::microsandbox::runtime::wait_for_port;
use microsandbox::sandbox::SandboxStatus;
use microsandbox::{MicrosandboxError, Sandbox};

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
}

/// Resolve the msb home dir (`$MSB_HOME` or `~/.microsandbox`), mirroring
/// `microsandbox_utils::resolve_home` so the dir check matches the msb
/// create gate exactly.
pub fn msb_home() -> PathBuf {
    if let Some(path) = std::env::var_os("MSB_HOME") {
        return PathBuf::from(path);
    }
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".microsandbox")
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
    let (msb_status, msb_unavailable) = match Sandbox::get(instance).await {
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
    Ok(ReconcileFacts {
        record,
        msb_status,
        msb_unavailable,
        dir_exists,
        healthy,
        recently_started,
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
    // Fail; else plain Start.
    if facts.msb_unavailable {
        return Ok(if facts.record.is_some() {
            ChainStep::Fail
        } else {
            ChainStep::Start
        });
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;
    use crate::config::test_support::ENV_TEST_LOCK;
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
        std::env::remove_var("MSB_HOME");
        let prior_home = std::env::var_os("HOME");
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-reconcile-home-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::env::set_var("HOME", &tmp);
        assert_eq!(
            sandbox_dir("personal-b"),
            tmp.join(".microsandbox")
                .join("sandboxes")
                .join("personal-b")
        );
        match prior_msb {
            Some(v) => std::env::set_var("MSB_HOME", v),
            None => std::env::remove_var("MSB_HOME"),
        }
        match prior_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
