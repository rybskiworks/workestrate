//! Dependency auto-start orchestration (ADR 0026 addendum 2026-08-01; ADR
//! 0021 addendum 2026-08-01 — compose-mirrored dependency lifecycle).
//!
//! Declared `depends_on` deps START BY DEFAULT on `up`/`exec`, before the
//! dependent's `ConfigWorkload` is constructed (the construction-order rule:
//! `ConfigWorkload::new_with_use_overrides` runs `resolve_depends_on`, which
//! refuses a required-not-running dep — so the dep must already be up).
//!
//! Two layers:
//!
//! - PURE PLANNING ([`plan_dep_starts`], [`plan_bare_up`]): given the merged
//!   config, the port-registry records, and the active context, decide WHAT
//!   would start — no KVM, no spawn, unit-testable. Occupied singleton slot
//!   = SATISFIED (record-as-authoritative, matching `depgraph`; never
//!   restart a running dep). A dep named in `--use` is NEVER auto-started
//!   (pure instance-selection override). An agent-kind dep that is not
//!   running is a hard error BEFORE anything starts.
//! - EXECUTORS ([`auto_start_dependencies`], [`cmd_workload_up_all`]): thin
//!   KVM-dependent shells around the planning seam. Service-kind deps start
//!   DETACHED with a bounded wait-for-port ([`DEFAULT_WAIT`]); a dep with no
//!   ports skips the wait. `--no-deps` and non-start verbs are no-ops.

use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::config::ConfigFile;
use crate::microsandbox::depgraph::{dep_closure, singleton_record, topo_all};
use crate::microsandbox::port_registry::{list_records, SandboxInstanceRecord};
use crate::microsandbox::runtime::{wait_for_port, DEFAULT_WAIT};
use crate::microsandbox::slots::slot_for;

/// How long to poll the registry for a freshly-started dep's singleton
/// record before falling back to its DECLARED host ports (bounded within
/// the overall [`DEFAULT_WAIT`] readiness budget).
const RECORD_POLL_BUDGET: Duration = Duration::from_secs(5);

/// Registry poll interval while waiting for the detached child's record.
const RECORD_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// One planned action for a dependency of a workload, in topological START
/// order (every dep appears after all of its own deps).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepStartAction {
    /// The dep's singleton slot is free: start it detached, then wait for
    /// each published host port.
    StartService {
        dep: String,
        slot: String,
        /// Declared host ports (fallback readiness targets when the
        /// registry record is not yet visible).
        ports: Vec<u16>,
    },
    /// The dep's singleton slot is already occupied (record-as-authoritative)
    /// — skip; never restart a running dep.
    Satisfied { dep: String, slot: String },
}

/// One workload the bare-up batch will start (service-kind, slot free).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BareStart {
    pub name: String,
    pub slot: String,
    /// Declared host ports (fallback readiness targets).
    pub ports: Vec<u16>,
}

/// One workload the bare-up batch will NOT start, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BareSkip {
    /// Agent-kind workload — interactive; never batch-started.
    Agent { name: String },
    /// Service-kind workload whose singleton slot is already occupied
    /// (already running — skip, NOT an error).
    AlreadyRunning { name: String, slot: String },
}

/// Plan the dependency starts for `name` (PURE — no KVM, no spawn).
///
/// Walks [`dep_closure`] in topological START order. For each dep:
///
/// - named in `use_overrides` → skipped ENTIRELY (no StartService, no
///   Satisfied): `--use` is a pure instance-selection override; resolution
///   at construction validates the selected instance;
/// - singleton slot occupied (a record exists) → [`DepStartAction::Satisfied`];
/// - free + service-kind → [`DepStartAction::StartService`] with the dep's
///   declared host ports;
/// - free + agent-kind → hard error naming dep + dependent (agents are
///   interactive — planning fails BEFORE any start executes);
/// - free + unknown kind → hard error naming the kind.
pub fn plan_dep_starts(
    config: &ConfigFile,
    name: &str,
    records: &[SandboxInstanceRecord],
    context: Option<&str>,
    use_overrides: &[(String, String)],
) -> Result<Vec<DepStartAction>> {
    let closure = dep_closure(config, name)?;
    let mut actions = Vec::with_capacity(closure.len());
    for dep in closure {
        if use_overrides.iter().any(|(d, _)| d == &dep) {
            continue;
        }
        let slot = slot_for(&dep, context);
        if singleton_record(records, &slot).is_some() {
            actions.push(DepStartAction::Satisfied { dep, slot });
            continue;
        }
        let kind = config
            .workloads
            .get(&dep)
            .map(|w| w.kind.as_str())
            .unwrap_or("");
        match kind {
            "service" => {
                let ports = declared_host_ports(config, &dep);
                actions.push(DepStartAction::StartService { dep, slot, ports });
            }
            "agent" => anyhow::bail!(
                "dependency '{dep}' of '{name}' is an agent (interactive); start it yourself with `workestrate workload exec {dep}`"
            ),
            other => anyhow::bail!(
                "dependency '{dep}' of '{name}' has unknown kind '{other}' (expected 'service' or 'agent')"
            ),
        }
    }
    Ok(actions)
}

/// Plan a bare `workestrate workload up` (PURE — no KVM, no spawn): every
/// workload in the config via [`topo_all`], split into the start list
/// (service-kind, singleton slot free, topo order) and the skip list
/// (agent-kind → interactive message; service-kind with an occupied slot →
/// already running, NOT an error).
pub fn plan_bare_up(
    config: &ConfigFile,
    records: &[SandboxInstanceRecord],
    context: Option<&str>,
) -> Result<(Vec<BareStart>, Vec<BareSkip>)> {
    let mut starts = Vec::new();
    let mut skips = Vec::new();
    for name in topo_all(config)? {
        let kind = config
            .workloads
            .get(&name)
            .map(|w| w.kind.as_str())
            .unwrap_or("");
        match kind {
            "service" => {
                let slot = slot_for(&name, context);
                if singleton_record(records, &slot).is_some() {
                    skips.push(BareSkip::AlreadyRunning { name, slot });
                } else {
                    let ports = declared_host_ports(config, &name);
                    starts.push(BareStart { name, slot, ports });
                }
            }
            "agent" => skips.push(BareSkip::Agent { name }),
            other => anyhow::bail!("unknown workload kind '{other}' for '{name}'"),
        }
    }
    Ok((starts, skips))
}

/// The dep's DECLARED host ports (`config.workloads[dep].ports[*].host`).
fn declared_host_ports(config: &ConfigFile, dep: &str) -> Vec<u16> {
    config
        .workloads
        .get(dep)
        .map(|w| w.ports.iter().map(|p| p.host).collect())
        .unwrap_or_default()
}

/// Auto-start `workload_name`'s declared dependencies (EXECUTOR —
/// KVM-dependent at runtime).
///
/// No-ops when: `no_deps` is set (the `--no-deps` opt-out: keep today's
/// behavior — required deps refuse-with-remediation at construction,
/// optional deps fall back per convention + warn), `verb` is not a start
/// verb (`up`/`exec`), the workload is unknown, the verb mismatches the
/// CONFIG kind (the `workload_route` kind error fires downstream; the
/// hardcoded subcommand arms keep today's unchecked behavior), or the
/// workload declares no depends_on.
///
/// Otherwise plans via [`plan_dep_starts`] and executes each
/// [`DepStartAction::StartService`]: the dep's `ConfigWorkload` is
/// constructed with NO `--use` overrides (its own deps are already
/// started/satisfied by topo order), started DETACHED on its singleton
/// slot, then awaited on its published host ports within [`DEFAULT_WAIT`].
pub async fn auto_start_dependencies(
    workload_name: &str,
    verb: &str,
    no_deps: bool,
    use_overrides: &[(String, String)],
) -> Result<()> {
    if no_deps || !matches!(verb, "up" | "exec") {
        return Ok(());
    }
    let config = crate::config::load_config()?;
    let Some(workload) = config.workloads.get(workload_name) else {
        // Unknown workload: the ConfigWorkload constructor's own error
        // fires downstream; there is nothing to plan.
        return Ok(());
    };
    if !matches!(
        (workload.kind.as_str(), verb),
        ("service", "up") | ("agent", "exec")
    ) {
        return Ok(());
    }
    if workload.depends_on.is_empty() {
        return Ok(());
    }
    let context = crate::config::active_context_name();
    let state_dir = crate::config::resolve_state_dir();
    let records = list_records(&state_dir)?;
    let actions = plan_dep_starts(
        &config,
        workload_name,
        &records,
        context.as_deref(),
        use_overrides,
    )?;
    for action in actions {
        if let DepStartAction::StartService { dep, slot, ports } = action {
            start_service_detached(&dep).await?;
            println!("started dependency '{dep}' (slot '{slot}')");
            wait_until_ready(
                &state_dir,
                &format!("dependency '{dep}' of '{workload_name}'"),
                &slot,
                &ports,
            )?;
        }
    }
    Ok(())
}

/// Bare `workestrate workload up` (EXECUTOR — KVM-dependent at runtime):
/// topo-ordered start of ALL service-kind workloads in the active context,
/// detached, singleton slots. Agent-kind workloads are printed as SKIPPED;
/// an occupied slot is already running (skip, NOT an error).
///
/// Output: text by default. With `json`, a single summary object
/// (`{"started": [...], "already_running": [...], "skipped_agents": [...]}`)
/// is printed INSTEAD of the per-workload text lines.
pub async fn cmd_workload_up_all(json: bool) -> Result<()> {
    let config = crate::config::load_config()?;
    let context = crate::config::active_context_name();
    let state_dir = crate::config::resolve_state_dir();
    let records = list_records(&state_dir)?;
    let (starts, skips) = plan_bare_up(&config, &records, context.as_deref())?;

    if !json {
        for skip in &skips {
            match skip {
                BareSkip::Agent { name } => println!(
                    "{name}: skipped (agents are interactive; use `workestrate workload exec {name}`)"
                ),
                BareSkip::AlreadyRunning { name, slot } => {
                    println!("{name}: already running (slot '{slot}')")
                }
            }
        }
    }

    let mut started: Vec<String> = Vec::with_capacity(starts.len());
    for s in &starts {
        start_service_detached(&s.name).await?;
        if !json {
            println!("started '{}' (slot '{}')", s.name, s.slot);
        }
        wait_until_ready(
            &state_dir,
            &format!("workload '{}'", s.name),
            &s.slot,
            &s.ports,
        )?;
        started.push(s.name.clone());
    }

    if json {
        let already_running: Vec<&str> = skips
            .iter()
            .filter_map(|s| match s {
                BareSkip::AlreadyRunning { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        let skipped_agents: Vec<&str> = skips
            .iter()
            .filter_map(|s| match s {
                BareSkip::Agent { name } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        let body = serde_json::json!({
            "started": started,
            "already_running": already_running,
            "skipped_agents": skipped_agents,
        });
        println!("{}", serde_json::to_string_pretty(&body)?);
    }
    Ok(())
}

/// Start one service-kind workload DETACHED on its singleton slot with no
/// `--use` overrides and no per-slot flags (its own deps are already
/// started/satisfied by topo order).
async fn start_service_detached(name: &str) -> Result<()> {
    let workload =
        crate::microsandbox::workload::ConfigWorkload::new_with_use_overrides(name, &[])?;
    let spec = crate::commands::lifecycle::build_instance_spec(
        name,
        false,
        None,
        None,
        false,
        &[],
        false,
    )?;
    crate::microsandbox::runtime::up_service_with_spec(&workload, &spec, false).await
}

/// Wait for a freshly-started workload's published host ports within
/// [`DEFAULT_WAIT`]. A workload with no ports skips the wait. On timeout
/// the error names the subject (dep-of-dependent or workload), the address,
/// and the detached child's log file.
fn wait_until_ready(
    state_dir: &Path,
    subject: &str,
    slot: &str,
    declared_ports: &[u16],
) -> Result<()> {
    let deadline = Instant::now() + DEFAULT_WAIT;
    let targets = readiness_targets(state_dir, slot, declared_ports);
    for (ip, port) in targets {
        let remaining = deadline.saturating_duration_since(Instant::now());
        wait_for_port(ip, port, remaining).map_err(|_| {
            anyhow::anyhow!(
                "{subject} did not become ready on {ip}:{port} within {}s; see log: ~/.microsandbox/sandboxes/{slot}/workestrate.log",
                DEFAULT_WAIT.as_secs()
            )
        })?;
    }
    Ok(())
}

/// Learn the published host addresses for a freshly-started workload: poll
/// the registry for its singleton record (the detached child's registration
/// lands shortly after its sandbox is created), falling back to the
/// DECLARED host ports on the shared 127.0.0.1 bind when the record is not
/// yet visible within [`RECORD_POLL_BUDGET`]. An empty result means no
/// ports to wait on (the caller skips the wait).
fn readiness_targets(state_dir: &Path, slot: &str, declared_ports: &[u16]) -> Vec<(IpAddr, u16)> {
    let poll_deadline = Instant::now() + RECORD_POLL_BUDGET;
    loop {
        if let Ok(records) = list_records(state_dir) {
            if let Some(rec) = singleton_record(&records, slot) {
                if !rec.port_pairs.is_empty() {
                    return rec.port_pairs.iter().map(|p| (p.bind_ip, p.host)).collect();
                }
                return rec.ports.iter().map(|&p| (rec.bind_ip, p)).collect();
            }
        }
        if Instant::now() >= poll_deadline {
            return declared_ports
                .iter()
                .map(|&p| (IpAddr::V4(Ipv4Addr::LOCALHOST), p))
                .collect();
        }
        std::thread::sleep(RECORD_POLL_INTERVAL);
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
    use crate::config::test_support::{unique_state_dir, TestConfigGuard};
    use crate::microsandbox::plan::PortMapping;
    use crate::microsandbox::port_registry::check_and_register_sandbox_lifecycle;

    /// Chain fixture: a (agent) → b (service, 4001) → c (service, 4002).
    fn chain_config() -> ConfigFile {
        let toml = r#"
schema_version = 1

[workloads.a]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.a.depends_on.b]
env = "B_URL"

[workloads.b]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[[workloads.b.ports]]
host = 4001
guest = 4001

[workloads.b.depends_on.c]
env = "C_URL"

[workloads.c]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[[workloads.c.ports]]
host = 4002
guest = 4002
"#;
        toml::from_str(toml).expect("chain fixture must parse")
    }

    /// Bare-up fixture: svc-top (service) → svc-dep (service, 4003), plus
    /// ag (agent, no deps).
    fn bare_up_config() -> ConfigFile {
        let toml = r#"
schema_version = 1

[workloads.ag]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.svc-top]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.svc-top.depends_on.svc-dep]
env = "DEP_URL"

[workloads.svc-dep]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[[workloads.svc-dep.ports]]
host = 4003
guest = 4003
"#;
        toml::from_str(toml).expect("bare-up fixture must parse")
    }

    /// Register a singleton lifecycle record for slot `personal-<workload>`.
    fn register_singleton(state_dir: &Path, workload: &str, host: u16, guest: u16) -> Result<()> {
        let bind = IpAddr::V4(Ipv4Addr::LOCALHOST);
        check_and_register_sandbox_lifecycle(
            state_dir,
            &format!("personal-{workload}"),
            Some("personal"),
            workload,
            bind,
            &[host],
            &[PortMapping {
                host,
                guest,
                bind_ip: bind,
            }],
            "2026-08-01T00:00:00Z",
        )
    }

    // (i) Chain a→b→c, all slots free: both deps start, deepest first, with
    // their declared ports.
    #[test]
    fn plan_dep_starts_chain_all_absent_starts_in_topo_order() -> Result<()> {
        let config = chain_config();
        let actions = plan_dep_starts(&config, "a", &[], Some("personal"), &[])?;
        assert_eq!(
            actions,
            vec![
                DepStartAction::StartService {
                    dep: "c".to_string(),
                    slot: "personal-c".to_string(),
                    ports: vec![4002],
                },
                DepStartAction::StartService {
                    dep: "b".to_string(),
                    slot: "personal-b".to_string(),
                    ports: vec![4001],
                },
            ]
        );
        Ok(())
    }

    // (ii) The middle dep (b) already running → Satisfied; only the
    // not-running dep (c) starts.
    #[test]
    fn plan_dep_starts_running_dep_is_satisfied() -> Result<()> {
        let state_dir = unique_state_dir("deps-satisfied");
        register_singleton(&state_dir, "b", 4001, 4001)?;
        let records = list_records(&state_dir)?;
        let config = chain_config();

        let actions = plan_dep_starts(&config, "a", &records, Some("personal"), &[])?;
        assert_eq!(
            actions,
            vec![
                DepStartAction::StartService {
                    dep: "c".to_string(),
                    slot: "personal-c".to_string(),
                    ports: vec![4002],
                },
                DepStartAction::Satisfied {
                    dep: "b".to_string(),
                    slot: "personal-b".to_string(),
                },
            ]
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // (iii) A not-running AGENT-kind dep is a hard error naming dep +
    // dependent (agents are interactive; never auto-started).
    #[test]
    fn plan_dep_starts_agent_dep_is_a_hard_error() {
        let toml = r#"
schema_version = 1

[workloads.top]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.top.depends_on.helper]
env = "HELPER_URL"

[workloads.helper]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []
"#;
        let config: ConfigFile = toml::from_str(toml).expect("agent-dep fixture must parse");
        let err = plan_dep_starts(&config, "top", &[], Some("personal"), &[]).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains(
                "is an agent (interactive); start it yourself with `workestrate workload exec helper`"
            ),
            "error must carry the interactive-agent remediation: {msg}"
        );
        assert!(msg.contains("helper"), "error must name the dep: {msg}");
        assert!(msg.contains("top"), "error must name the dependent: {msg}");
    }

    // (iv) A dep named in --use is NEVER auto-started: it appears in
    // neither StartService nor Satisfied.
    #[test]
    fn plan_dep_starts_use_override_dep_is_absent_from_actions() -> Result<()> {
        let config = chain_config();
        let overrides = vec![("b".to_string(), "canary".to_string())];
        let actions = plan_dep_starts(&config, "a", &[], Some("personal"), &overrides)?;
        assert_eq!(
            actions,
            vec![DepStartAction::StartService {
                dep: "c".to_string(),
                slot: "personal-c".to_string(),
                ports: vec![4002],
            }],
            "only the non-overridden dep may appear: {actions:?}"
        );
        Ok(())
    }

    // Bare up: starts are service-kind only, in topo order (dep before
    // dependent); the agent lands in the skip list.
    #[test]
    fn plan_bare_up_starts_services_in_topo_order_and_skips_agents() -> Result<()> {
        let config = bare_up_config();
        let (starts, skips) = plan_bare_up(&config, &[], Some("personal"))?;
        let start_names: Vec<&str> = starts.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(start_names, vec!["svc-dep", "svc-top"]);
        assert_eq!(starts[0].ports, vec![4003]);
        assert!(
            skips.contains(&BareSkip::Agent {
                name: "ag".to_string()
            }),
            "the agent must be skipped as interactive: {skips:?}"
        );
        Ok(())
    }

    // Bare up with the dep already running: only the dependent starts; the
    // dep is an already-running skip (NOT an error).
    #[test]
    fn plan_bare_up_running_service_is_an_already_running_skip() -> Result<()> {
        let state_dir = unique_state_dir("deps-bare-running");
        register_singleton(&state_dir, "svc-dep", 4003, 4003)?;
        let records = list_records(&state_dir)?;
        let config = bare_up_config();

        let (starts, skips) = plan_bare_up(&config, &records, Some("personal"))?;
        let start_names: Vec<&str> = starts.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(start_names, vec!["svc-top"]);
        assert!(
            skips.contains(&BareSkip::AlreadyRunning {
                name: "svc-dep".to_string(),
                slot: "personal-svc-dep".to_string(),
            }),
            "the running dep must be an already-running skip: {skips:?}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // --no-deps preserves current semantics: the executor is a no-op
    // (returns Ok before loading config or touching records), so a required
    // dep absent would still refuse at ConfigWorkload construction
    // (discovery.rs, unchanged). Non-start verbs are likewise no-ops.
    #[tokio::test]
    async fn auto_start_dependencies_no_deps_and_non_start_verbs_are_no_ops() -> Result<()> {
        // Returns before load_config: no config env needed at all.
        auto_start_dependencies("pi", "exec", true, &[]).await?;
        auto_start_dependencies("litellm", "up", true, &[]).await?;
        for verb in ["plan", "down", "logs"] {
            auto_start_dependencies("pi", verb, false, &[]).await?;
        }
        Ok(())
    }

    // Unknown workloads, kind/verb mismatches per the CONFIG kind, and
    // workloads with no depends_on are no-ops (the existing downstream
    // errors/routes are unchanged).
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see runtime::tests
    async fn auto_start_dependencies_unknown_or_mismatch_or_depfree_is_a_no_op() -> Result<()> {
        let _guard = TestConfigGuard::new();
        // Unknown workload: the constructor's error fires downstream.
        auto_start_dependencies("ghost", "up", false, &[]).await?;
        // Kind/verb mismatch per the CONFIG kind (pi is an agent): the
        // workload_route kind error fires downstream.
        auto_start_dependencies("pi", "up", false, &[]).await?;
        // The fixture config declares no depends_on: nothing to plan.
        auto_start_dependencies("pi", "exec", false, &[]).await?;
        auto_start_dependencies("litellm", "up", false, &[]).await?;
        Ok(())
    }
}
