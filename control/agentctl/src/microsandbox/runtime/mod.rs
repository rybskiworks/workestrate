//! Sandbox runtime: sandbox lifecycle (up/exec/down), `ps` listing, and
//! network-policy conversion for workestrate-managed Microsandbox instances.
//!
//! WP3 split of the former `runtime.rs` god-file into `network`
//! (NetworkPlan → SDK NetworkPolicy), `spawn` (detached spawn + log tail),
//! `run` (build/up/exec foreground paths), `ps` (listing + liveness probe),
//! and `time` (RFC3339 timestamp helpers). Purely mechanical — no behavior
//! changes. The `down` family and the shared `InstanceSpec` /
//! `ForegroundConfig` / `stop_and_remove` live here in `mod.rs` alongside
//! the `check_occupied_or_replace` gate.

mod network;
mod ps;
// `reconcile` is crate-visible so the dep executor (commands/deps.rs) shares
// the ONE fact-gathering + conflict-chain decision with the named up/exec
// occupancy gate (ADR 0030 Phase 0).
pub(crate) mod reconcile;
mod run;
mod spawn;
// `time` is crate-visible so `images::state` (spec 21 §8) reuses the ONE
// no-chrono RFC3339 formatter (`current_rfc3339_utc`).
pub(crate) mod time;
mod wait;

pub use network::network_plan_to_policy;
pub use ps::probe_liveness;
pub use ps::PsKind;
pub use ps::{format_refuse_message, occupancy_from_state, ps, Occupancy, PsEntry};
pub use reconcile::{
    decide_chain, default_chain, gather_facts, sandbox_dir, ChainStep, ReconcileFacts,
};
pub use run::{exec_agent_with_spec, up_service_with_spec};
// Crate-visible so the seed-file env-view builder (env.rs) reuses the ONE
// resolution algorithm/order the runtime applies to the guest env.
pub(crate) use run::resolve_plan_envs;
pub use spawn::logs;
pub use spawn::spawn_detached_service;
pub use wait::{wait_for_port, DEFAULT_WAIT};

use anyhow::Result;
use microsandbox::sandbox::{SandboxHandle, SandboxStatus};
use microsandbox::{MicrosandboxError, Sandbox};
use std::path::Path;

pub async fn stop_and_remove(handle: SandboxHandle) -> Result<()> {
    // Microsandbox 0.6.8 SDK: `SandboxHandle::status()` was removed. `refresh()`
    // returns a fresh handle whose `status_snapshot()` reflects the current DB
    // state; fall back to the creation-time snapshot if the row is already gone.
    let status = handle
        .refresh()
        .await
        .map(|h| h.status_snapshot())
        .unwrap_or_else(|_| handle.status_snapshot());
    match status {
        SandboxStatus::Running | SandboxStatus::Draining | SandboxStatus::Paused => {
            if let Err(e) = handle.stop().await {
                eprintln!("stop failed ({}), attempting kill", e);
                handle.kill().await?;
            }
        }
        _ => {}
    }
    handle.remove().await?;
    Ok(())
}

/// Start `exec_program` with `exec_args` inside `sandbox`, stream its logs to
/// stderr, and block until Ctrl-C — then stop the sandbox.
///
/// This is the shared foreground path used by every service-kind `up`. The
/// service label is used in user-facing messages (e.g. the workload's bare
/// name, "example-litellm").
pub struct ForegroundConfig {
    pub sandbox_name: String,
    pub service_label: String,
    pub command: super::workload::SandboxCommand,
    pub log_stop_errors: bool,
}

/// Resolved identity + flags for a single `up`/`exec` invocation (ADR 0021).
///
/// Built by the CLI layer from `--replace` / `--instance <id>` / `--new`
/// plus the active context. Consumed by [`build_sandbox`].
pub struct InstanceSpec {
    /// The sandbox name to create: `slot` (singleton) or `slot@<id>` (parallel).
    pub instance: String,
    /// Bare workload name (e.g. `example-litellm`). Used in user-facing messages.
    pub workload: String,
    /// Active context name, if any.
    pub context: Option<String>,
    /// `--replace`. If true, occupancy is torn down before create; otherwise
    /// an occupied slot REFUSES (fail-closed default).
    pub replace: bool,
    /// `--port-auto` (ADR 0026(c)). Publish each port on a lock-probed free
    /// port on the slot's bind; the chosen ports are recorded in the instance
    /// record.
    pub port_auto: bool,
    /// Typed `--use <dep>@<instance>` overrides as `(dep, instance-id)` pairs
    /// (ADR 0026(d)). Pure instance-selection overrides for depends_on
    /// resolution; forwarded to a detached `up` child so it resolves
    /// identically to the parent.
    pub use_overrides: Vec<(String, String)>,
    /// `--no-deps` (ADR 0026 addendum 2026-08-01). When true, the detached
    /// child must NOT re-run dependency auto-start (the parent was told to
    /// skip it); forwarded by `detach_args` as the `--no-deps` flag.
    pub no_deps: bool,
    /// `--images-ready` (spec 21 §2.2, phase E). The ensure-images token:
    /// when true, this spec describes a process whose nix-layered images the
    /// PARENT already ensured (the detached child), so the ensure pre-flight
    /// must be SKIPPED entirely — no nix eval, no store probe (keeps the
    /// FS-8 500ms grace meaningful). `detach_args` appends the hidden
    /// `--images-ready` flag unconditionally; the child reconstitutes this
    /// field from its clap parse. A foreground `up` without the token IS the
    /// parent (false here) and ensures.
    pub images_ready: bool,
}

/// Outcome of stopping one instance. Used by `down --instance`, `down
/// --all-instances`, and `down --all`.
#[derive(Debug, Clone)]
pub struct DownResult {
    pub instance: String,
    pub status: DownStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownStatus {
    /// Sandbox was running and was stopped+removed cleanly.
    Stopped,
    /// No sandbox found (state record cleared if present).
    NotFound,
    /// msb returned a hard error during stop/remove.
    Error,
}

/// Occupancy teardown for the `--replace` flag path ONLY. When `spec.replace`
/// is true, the existing sandbox at `spec.instance` is torn down
/// (best-effort) and stale state is cleared. When false, the function REFUSES
/// (returns Err) if any of:
///   - msb reports the sandbox running, OR
///   - a state record exists AND msb is unavailable (fail-closed).
///
/// NOTE: the conflict-chain decision (reuse/start/replace/fail) lives in
/// [`reconcile`]; this function now handles ONLY the explicit `--replace`
/// teardown (ADR 0030 Phase 0).
///
/// Returns Ok(()) when the slot is free (or has been cleared by --replace).
pub async fn check_occupied_or_replace(spec: &InstanceSpec, state_dir: &Path) -> Result<()> {
    if spec.replace {
        match Sandbox::get(&spec.instance).await {
            Ok(handle) => {
                stop_and_remove(handle).await?;
            }
            Err(MicrosandboxError::SandboxNotFound(_)) => {
                let _ = super::port_registry::unregister_sandbox(state_dir, &spec.instance);
            }
            Err(e) => {
                eprintln!(
                    "warning: could not verify sandbox '{}' via msb ({}); \
                     proceeding with --replace and clearing any stale state",
                    spec.instance, e
                );
                let _ = super::port_registry::unregister_sandbox(state_dir, &spec.instance);
            }
        }
        return Ok(());
    }

    match Sandbox::get(&spec.instance).await {
        Ok(_handle) => {
            // Truly running — refuse. The handle drops without stopping;
            // the existing sandbox keeps running (this is the desired
            // fail-closed behavior).
            anyhow::bail!("{}", format_refuse_message(&spec.workload, &spec.instance));
        }
        Err(MicrosandboxError::SandboxNotFound(_)) => {
            // Not running. A state record, if any, is stale.
            if matches!(
                occupancy_from_state(state_dir, &spec.instance)?,
                Occupancy::Occupied { .. }
            ) {
                eprintln!(
                    "warning: state record for '{}' exists but msb reports the sandbox \
                     is not running (stale record); refusing by default. \
                     Use --replace to clear.",
                    spec.instance
                );
                anyhow::bail!("{}", format_refuse_message(&spec.workload, &spec.instance));
            }
            Ok(())
        }
        Err(e) => {
            // msb unavailable. Fall back to state-record verdict (fail-closed).
            if matches!(
                occupancy_from_state(state_dir, &spec.instance)?,
                Occupancy::Occupied { .. }
            ) {
                eprintln!(
                    "warning: could not verify sandbox '{}' via msb ({}); \
                     treating state record as authoritative and refusing.",
                    spec.instance, e
                );
                anyhow::bail!("{}", format_refuse_message(&spec.workload, &spec.instance));
            }
            Ok(())
        }
    }
}

/// Idempotent three-store teardown for the conflict chain's `replace`
/// disposition (ADR 0030 Phase 0): stop+remove the msb sandbox when present,
/// then clear the port-registry record and the policy dir — all best-effort.
/// Also removes the lingering sandbox DIRECTORY when the msb DB row is gone
/// (ADR 0030 §2.2 / disposition table row "msb gone + dir exists → replace"):
/// without that, the subsequent fresh create would hit the msb create gate
/// and surface the opaque `SandboxAlreadyExists` error the ADR forbids.
/// Unlike the `--replace` branch of [`check_occupied_or_replace`], this never
/// refuses and never hard-errors on a missing sandbox: the caller proceeds to
/// a fresh create either way.
pub(crate) async fn teardown_for_replace(state_dir: &Path, instance: &str) -> Result<()> {
    match Sandbox::get(instance).await {
        Ok(handle) => {
            stop_and_remove(handle).await?;
        }
        Err(MicrosandboxError::SandboxNotFound(_)) => {}
        Err(e) => {
            eprintln!(
                "warning: could not verify sandbox '{}' via msb ({}); \
                 proceeding with replace and clearing any stale state",
                instance, e
            );
        }
    }
    let _ = super::port_registry::unregister_sandbox(state_dir, instance);
    let _ = super::policy_file::remove_policy_dir(state_dir, instance);
    // Best-effort removal of the lingering sandbox dir (create-gate cleanup).
    // `remove_dir_all` on a non-existent dir returns Err(NotFound), swallowed
    // by the `let _ =`.
    let _ = std::fs::remove_dir_all(self::reconcile::sandbox_dir(instance));
    Ok(())
}

/// Generic lifecycle: stop and remove any sandbox by name.
//
// Legacy single-name teardown; the CLI's `down` subcommand routes through the
// state-dir-explicit [`down_instance`] / [`down_all`] / [`down_all_instances`]
// variants so it can clean parallel-instance state. Retained on the migration
// branch as the resolved-state-dir convenience wrapper documented by
// [`down_instance`].
#[allow(dead_code)]
pub async fn down(name: &str) -> Result<()> {
    match Sandbox::get(name).await {
        Ok(handle) => {
            stop_and_remove(handle).await?;
            let state_dir = crate::config::resolve_state_dir();
            super::port_registry::unregister_sandbox(&state_dir, name)?;
            println!("Sandbox '{}' stopped and removed", name);
            Ok(())
        }
        Err(MicrosandboxError::SandboxNotFound(_)) => {
            // Sandbox not running, but there may be a stale state file.
            let state_dir = crate::config::resolve_state_dir();
            super::port_registry::unregister_sandbox(&state_dir, name)?;
            println!("Sandbox '{}' not found", name);
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// Stop a single instance by name (state-dir-explicit). The legacy
/// [`down`] wraps this with the resolved state dir for back-compat.
pub async fn down_instance(state_dir: &Path, instance: &str) -> DownResult {
    match down_one(state_dir, instance).await {
        Ok(DownOutcome::Stopped) => DownResult {
            instance: instance.to_string(),
            status: DownStatus::Stopped,
            message: None,
        },
        Ok(DownOutcome::NotFound) => DownResult {
            instance: instance.to_string(),
            status: DownStatus::NotFound,
            message: None,
        },
        Err(e) => DownResult {
            instance: instance.to_string(),
            status: DownStatus::Error,
            message: Some(e.to_string()),
        },
    }
}

#[derive(Debug, Clone, Copy)]
enum DownOutcome {
    Stopped,
    NotFound,
}

async fn down_one(state_dir: &Path, instance: &str) -> Result<DownOutcome> {
    match Sandbox::get(instance).await {
        Ok(handle) => {
            stop_and_remove(handle).await?;
            let _ = super::port_registry::unregister_sandbox(state_dir, instance);
            let _ = super::policy_file::remove_policy_dir(state_dir, instance);
            Ok(DownOutcome::Stopped)
        }
        Err(MicrosandboxError::SandboxNotFound(_)) => {
            let _ = super::port_registry::unregister_sandbox(state_dir, instance);
            let _ = super::policy_file::remove_policy_dir(state_dir, instance);
            Ok(DownOutcome::NotFound)
        }
        Err(e) => Err(e.into()),
    }
}

/// Stop every instance whose workload matches `workload`. Used by
/// `workestrate <wl> down --all-instances`.
///
/// Teardown is namespace-agnostic: `down --all-instances` stops every
/// instance of the workload regardless of its declaring-repo namespace (the
/// namespace is a RESOLUTION filter, not a teardown scope).
pub async fn down_all_instances(state_dir: &Path, workload: &str) -> Result<Vec<DownResult>> {
    let records =
        super::port_registry::list_records_for_workload_any_namespace(state_dir, workload)?;
    let mut results = Vec::with_capacity(records.len());
    for r in records {
        results.push(down_instance(state_dir, &r.instance).await);
    }
    Ok(results)
}

/// Stop every workestrate-tracked instance. Used by `workestrate down --all`.
pub async fn down_all(state_dir: &Path) -> Result<Vec<DownResult>> {
    let records = super::port_registry::list_records(state_dir)?;
    let mut results = Vec::with_capacity(records.len());
    for r in records {
        results.push(down_instance(state_dir, &r.instance).await);
    }
    Ok(results)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::{down_all_instances, sandbox_dir, DownStatus};
    use crate::config::test_support::unique_state_dir_runtime;

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

    /// down_all_instances with an UNREACHABLE msb db must deterministically
    /// surface DownStatus::Error for each attempted record (never panic, never
    /// NotFound). The msb DB is made unreachable by pointing MSB_HOME at a path
    /// under a regular file, so the SDK's `create_dir_all(<MSB_HOME>/db)` fails
    /// with ENOTDIR and `Sandbox::get` errors out.
    ///
    /// The prior incarnation (`down_all_instances_without_msb_returns_error_results`)
    /// was non-deterministic: it asserted Error but actually returned NotFound
    /// whenever a real msb happened to be reachable in the test environment
    /// (the devshell has one). Pinning MSB_HOME at an unwritable path fixes the
    /// outcome to Error regardless of environment.
    #[tokio::test]
    // ENV_TEST_LOCK (std::sync::Mutex) is held across the `.await` below. This
    // is safe because #[tokio::test] uses a single-threaded current-thread
    // runtime with no spawned tasks, so the await can never yield to a task
    // that contends on the lock (no deadlock). The lock MUST span the await so
    // no concurrent env-mutating test (config::tests) races this set_var/read.
    #[allow(clippy::await_holding_lock)]
    async fn down_all_instances_returns_error_when_msb_db_unreachable() -> anyhow::Result<()> {
        // Serialize with ALL env-mutating tests (see the ENV_TEST_LOCK note
        // above) so this set_var(MSB_HOME) cannot race a config test's env read.
        // Declared before `_msb` so the lock is released AFTER MsbHomeGuard
        // restores MSB_HOME on drop (incl. panic).
        let _env_lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        // `<tmp>/blocker` is a regular file, so `<MSB_HOME>/db` (=
        // `<tmp>/blocker/db`) cannot be created → init_global fails →
        // Sandbox::get returns a hard error.
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-msb-unreachable-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::create_dir_all(&tmp)?;
        std::fs::write(tmp.join("blocker"), b"x")?;
        let _msb = MsbHomeGuard::set(&tmp.join("blocker"));

        let dir = unique_state_dir_runtime("down-all-inst-msb-unreachable");
        crate::microsandbox::port_registry::register_sandbox(
            &dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        let results = down_all_instances(&dir, "litellm").await?;
        assert_eq!(results.len(), 1, "one record was attempted");
        assert!(
            matches!(results[0].status, DownStatus::Error),
            "expected Error status when the msb db is unreachable; got {:?} ({:?})",
            results[0].status,
            results[0].message,
        );
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    /// Positive control: with a writable, empty MSB_HOME the SDK opens its DB
    /// fine and `down_all_instances` resolves to DownStatus::NotFound (msb
    /// reachable, but no such sandbox is registered there).
    ///
    /// `#[ignore]`'d because the SDK pins its DB pool in a process-global
    /// `OnceCell` on the first *successful* `init_global`. If this test and the
    /// Error test both ran in one `cargo test` invocation, whichever
    /// initialized first would fix the home for the whole process and the pair
    /// would be non-deterministic (a writable pin makes the Error test see
    /// NotFound). Keeping this `#[ignore]`'d means:
    ///   - `cargo test`           → Error test runs alone (deterministic Error);
    ///   - `cargo test --ignored` → this test runs alone (deterministic
    ///                               NotFound), the Error test is skipped.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see Error test
    #[ignore = "shares the SDK process-global DB pool with the Error test; run alone with --ignored"]
    async fn down_all_instances_returns_notfound_when_msb_db_empty_but_openable(
    ) -> anyhow::Result<()> {
        // Serialize with ALL env-mutating tests (see the ENV_TEST_LOCK note above).
        let _env_lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-msb-empty-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::create_dir_all(&tmp)?;
        let _msb = MsbHomeGuard::set(&tmp);

        let dir = unique_state_dir_runtime("down-all-inst-msb-empty");
        crate::microsandbox::port_registry::register_sandbox(
            &dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        let results = down_all_instances(&dir, "litellm").await?;
        assert_eq!(results.len(), 1, "one record was attempted");
        assert!(
            matches!(results[0].status, DownStatus::NotFound),
            "expected NotFound when msb db is openable but empty; got {:?} ({:?})",
            results[0].status,
            results[0].message,
        );
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    /// `teardown_for_replace` must clear the port-registry record AND remove
    /// the lingering sandbox DIRECTORY when the msb DB is unreachable (ADR
    /// 0030 §2.2 / disposition table row "msb gone + dir exists → replace"):
    /// the disposition owns the dir cleanup, so the subsequent fresh create
    /// never hits the msb create gate's opaque `SandboxAlreadyExists` error.
    /// Returns Ok even though the msb side cannot be verified.
    ///
    /// The msb DB is made unreachable by pointing MSB_HOME at a tmp dir whose
    /// `db` path is a regular FILE, so the SDK's `create_dir_all(<MSB_HOME>/db)`
    /// fails (ENOTDIR) and `Sandbox::get` returns a hard error — never
    /// SandboxNotFound, and never a successful init (so the process-global DB
    /// pool is not pinned for the shared `cargo test` run). Unlike the
    /// `#[ignore]`'d empty-db test, this one is deterministic in any ordering.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see Error test
    async fn teardown_for_replace_clears_record_and_dir_when_msb_db_unreachable(
    ) -> anyhow::Result<()> {
        // Serialize with ALL env-mutating tests (see the Error test note).
        let _env_lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-replace-db-blocked-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::create_dir_all(&tmp)?;
        // `<MSB_HOME>/db` is a regular FILE, so `<MSB_HOME>/db` cannot be
        // created as a directory → `Sandbox::get` errors (never NotFound) and
        // the DB pool is never successfully initialized (nothing to pin).
        std::fs::write(tmp.join("db"), b"x")?;
        let _msb = MsbHomeGuard::set(&tmp);

        let dir = unique_state_dir_runtime("teardown-for-replace");
        crate::microsandbox::port_registry::register_sandbox(
            &dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        // The lingering sandbox directory the msb create gate would refuse on
        // (store 2b): `<MSB_HOME>/sandboxes/<instance>`.
        let lingering = sandbox_dir("personal-litellm");
        std::fs::create_dir_all(&lingering)?;

        // Err branch of `Sandbox::get` (warn + continue) → still unregister +
        // remove dir, and return Ok.
        super::teardown_for_replace(&dir, "personal-litellm").await?;

        assert!(
            !lingering.exists(),
            "teardown_for_replace must remove the lingering sandbox dir so the \
             fresh create does not hit the msb create gate"
        );
        assert!(
            !dir.join("var")
                .join("run")
                .join("personal-litellm.json")
                .exists(),
            "teardown_for_replace must unregister the port-registry record"
        );
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }
}
