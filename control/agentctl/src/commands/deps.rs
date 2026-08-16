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
//!   ports skips the wait. `--no-deps` and non-start verbs are no-ops. The
//!   executor also RECONCILES occupancy with the msb runtime: a slot that msb
//!   reports Running is never blind-started (the record-as-authoritative
//!   planner can miss a running sandbox when the port-registry record is
//!   absent/stale in the active state dir).

use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::config::{ConfigFile, DepConflict};
use crate::microsandbox::depgraph::{dep_closure, singleton_record, topo_all};
use crate::microsandbox::port_registry::{list_records, SandboxInstanceRecord};
use crate::microsandbox::runtime::time::record_age_secs;
use crate::microsandbox::runtime::{
    down_instance, format_refuse_message, wait_for_port, DownStatus, DEFAULT_WAIT,
};
use crate::microsandbox::slots::slot_for;
use microsandbox::{MicrosandboxError, Sandbox};

/// How long to poll the registry for a freshly-started dep's singleton
/// record before falling back to its DECLARED host ports (bounded within
/// the overall [`DEFAULT_WAIT`] readiness budget).
const RECORD_POLL_BUDGET: Duration = Duration::from_secs(5);

/// Bounded budget for the reuse health probe (host TCP connect to each
/// published port, shared deadline). Short by design: a healthy running dep
/// answers in milliseconds; a dead-port zombie burns at most this budget.
const REUSE_PROBE_TIMEOUT: Duration = Duration::from_millis(500);

/// A slot whose record was created within this window is treated as BOOTING,
/// not a keep-alive zombie: auto-start must never down a dep that is still
/// coming up (a fresh `up`'s wait-for-port is 15s, so 30s covers a slow boot).
const BOOT_GRACE: Duration = Duration::from_secs(30);

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
        /// The dep's auto-start conflict policy (default "reuse").
        conflict: DepConflict,
    },
    /// The dep's singleton slot is already occupied (record-as-authoritative)
    /// — reuse, never restart a running dep; the executor may still probe
    /// health and auto-replace a keep-alive zombie under "reuse".
    Satisfied {
        dep: String,
        slot: String,
        /// Declared host ports (fallback readiness targets if the executor
        /// decides to replace the slot).
        ports: Vec<u16>,
        /// The dep's auto-start conflict policy (default "reuse").
        conflict: DepConflict,
    },
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

/// The dep's auto-start conflict policy from the DEPENDENT's depends_on spec
/// (ADR 0026 addendum 2026-08-16). `None` (omitted) resolves to "reuse".
fn dep_conflict(config: &ConfigFile, dependent: &str, dep: &str) -> DepConflict {
    config
        .workloads
        .get(dependent)
        .and_then(|w| w.depends_on.get(dep))
        .and_then(|s| s.on_conflict)
        .unwrap_or(DepConflict::Reuse)
}

/// Executor disposition for one planned dependency action after runtime
/// reconciliation (msb occupancy + host port health + record age). PURE —
/// unit-testable without KVM or the SDK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepDisposition {
    /// Reuse the running instance (healthy, or no ports to probe, or still
    /// within the boot grace window).
    Reuse { dep: String, slot: String },
    /// Down the slot (idempotent — clears stale records) then start fresh.
    Replace {
        dep: String,
        slot: String,
        ports: Vec<u16>,
    },
    /// Start fresh (slot free).
    Start {
        dep: String,
        slot: String,
        ports: Vec<u16>,
    },
    /// Refuse with the standard occupied-instance message (on_conflict = "fail").
    Fail {
        dep: String,
        slot: String,
        workload: String,
    },
}

/// Decide the executor disposition for one planned action from the runtime
/// facts:
///
/// - `msb_running`: `Sandbox::get(slot)` resolved Ok (the sandbox exists).
/// - `healthy`: host port probe outcome (`Some(true)` any port connected,
///   `Some(false)` none within budget, `None` the dep declares no ports).
/// - `recently_started`: the slot's record exists and its `created_at` is
///   within [`BOOT_GRACE`] — still booting, never replace under "reuse".
///
/// Rules (ADR 0026 addendum 2026-08-16):
///
/// - "reuse" (default): free slot → Start; msb-running + healthy (or no
///   ports, or booting) → Reuse; msb-running + dead port + old record →
///   Replace (keep-alive zombie); record present + msb NOT running → Replace
///   (stale record — down clears it, then start fresh).
/// - "replace": always Replace (idempotent down + start fresh).
/// - "fail": free slot → Start; occupied → Fail (the pre-fix failure mode).
pub fn decide_dep_disposition(
    action: &DepStartAction,
    workload_name: &str,
    msb_running: bool,
    healthy: Option<bool>,
    recently_started: bool,
) -> DepDisposition {
    let reuse_ok = healthy != Some(false) || recently_started;
    match action {
        DepStartAction::StartService {
            dep,
            slot,
            ports,
            conflict,
        } => {
            let (dep, slot, ports) = (dep.clone(), slot.clone(), ports.clone());
            match conflict {
                DepConflict::Reuse => {
                    if !msb_running {
                        DepDisposition::Start { dep, slot, ports }
                    } else if reuse_ok {
                        DepDisposition::Reuse { dep, slot }
                    } else {
                        DepDisposition::Replace { dep, slot, ports }
                    }
                }
                DepConflict::Replace => DepDisposition::Replace { dep, slot, ports },
                DepConflict::Fail => {
                    if msb_running {
                        DepDisposition::Fail {
                            dep,
                            slot,
                            workload: workload_name.to_string(),
                        }
                    } else {
                        DepDisposition::Start { dep, slot, ports }
                    }
                }
            }
        }
        DepStartAction::Satisfied {
            dep,
            slot,
            ports,
            conflict,
        } => {
            let (dep, slot, ports) = (dep.clone(), slot.clone(), ports.clone());
            match conflict {
                DepConflict::Reuse => {
                    if !msb_running {
                        // Record exists but the sandbox is gone: stale record.
                        DepDisposition::Replace { dep, slot, ports }
                    } else if reuse_ok {
                        DepDisposition::Reuse { dep, slot }
                    } else {
                        DepDisposition::Replace { dep, slot, ports }
                    }
                }
                DepConflict::Replace => DepDisposition::Replace { dep, slot, ports },
                DepConflict::Fail => DepDisposition::Fail {
                    dep,
                    slot,
                    workload: workload_name.to_string(),
                },
            }
        }
    }
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
///
/// Each action carries the dep's `on_conflict` policy (default "reuse"; ADR
/// 0026 addendum 2026-08-16): the executor reconciles the planned action with
/// msb occupancy and decides reuse/replace/fail at start time.
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
        // Compute the ports + conflict BEFORE the singleton check (both borrow
        // config; `dep` is moved into the action below).
        let ports = declared_host_ports(config, &dep);
        let conflict = dep_conflict(config, name, &dep);
        if singleton_record(records, &slot).is_some() {
            actions.push(DepStartAction::Satisfied {
                dep,
                slot,
                ports,
                conflict,
            });
            continue;
        }
        let kind = config
            .workloads
            .get(&dep)
            .map(|w| w.kind.as_str())
            .unwrap_or("");
        match kind {
            "service" => {
                actions.push(DepStartAction::StartService { dep, slot, ports, conflict });
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
        // ADR 0026 addendum 2026-08-16: reconcile the planner's record view
        // with the msb runtime BEFORE starting — a slot msb reports Running is
        // never blind-started into the occupancy gate. The record-as-
        // authoritative planner can miss a running sandbox when the registry
        // record is absent/stale in the active state dir.
        let msb_running = match &action {
            DepStartAction::StartService { slot, .. } => sandbox_running(slot).await?,
            DepStartAction::Satisfied { slot, .. } => sandbox_running(slot).await?,
        };
        let healthy = if msb_running {
            probe_dep_health(
                &state_dir,
                slot_of_action(&action),
                declared_ports_of(&action),
            )?
        } else {
            None
        };
        let recently_started = recently_started(&state_dir, slot_of_action(&action))?;
        let disposition = decide_dep_disposition(
            &action,
            workload_name,
            msb_running,
            healthy,
            recently_started,
        );
        match disposition {
            DepDisposition::Reuse { dep, slot } => {
                println!("dependency '{dep}' already running (slot '{slot}') — reusing");
            }
            DepDisposition::Replace { dep, slot, ports } => {
                let down = down_instance(&state_dir, &slot).await;
                if matches!(down.status, DownStatus::Error) {
                    anyhow::bail!(
                        "failed to replace dependency '{dep}' (slot '{slot}'): {}",
                        down.message.unwrap_or_default()
                    );
                }
                // Spec 21 §2.1: dep auto-start inherits the ensure-images
                // pre-flight per dependency. force=false — `--reload-images`
                // is named-workload/batch scoped (USER DECISION D3); deps
                // get the plain skew matrix.
                start_service_detached(&dep, EnsurePreflight::Run { force: false }).await?;
                println!("started dependency '{dep}' (slot '{slot}') [replaced]");
                wait_until_ready(
                    &state_dir,
                    &format!("dependency '{dep}' of '{workload_name}'"),
                    &slot,
                    &ports,
                )?;
            }
            DepDisposition::Start { dep, slot, ports } => {
                // Spec 21 §2.1: dep auto-start inherits the ensure-images
                // pre-flight per dependency. force=false — `--reload-images`
                // is named-workload/batch scoped (USER DECISION D3); deps
                // get the plain skew matrix.
                start_service_detached(&dep, EnsurePreflight::Run { force: false }).await?;
                println!("started dependency '{dep}' (slot '{slot}')");
                wait_until_ready(
                    &state_dir,
                    &format!("dependency '{dep}' of '{workload_name}'"),
                    &slot,
                    &ports,
                )?;
            }
            DepDisposition::Fail {
                dep: _,
                slot,
                workload,
            } => {
                anyhow::bail!("{}", format_refuse_message(&workload, &slot));
            }
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
///
/// Spec 21 phase E: `reload_images` is the batch-scoped `--reload-images`
/// force (USER DECISION D3). The ensure-images pre-flight runs for EVERY
/// start BEFORE ANY spawn — a nix-layered workload whose declaring repo
/// lacks a flake.nix is skipped with a note (spec §7 batch row) and a hard
/// ensure failure aborts the batch before anything starts.
pub async fn cmd_workload_up_all(json: bool, reload_images: bool) -> Result<()> {
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

    // Spec 21 §2.1/§5.2: the batch ensure pass, ALL starts BEFORE ANY
    // spawn, with the same force flag for every eligible service workload
    // in the batch (USER DECISION D3).
    let start_names: Vec<String> = starts.iter().map(|s| s.name.clone()).collect();
    crate::images::ensure::ensure_images_for_workloads(&start_names, reload_images).await?;

    let mut started: Vec<String> = Vec::with_capacity(starts.len());
    for s in &starts {
        start_service_detached(&s.name, EnsurePreflight::AlreadyDone).await?;
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

/// Whether [`start_service_detached`] runs the ensure-images pre-flight
/// (spec 21 §2.1, phase E).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnsurePreflight {
    /// Run ensure-images for the workload before spawning it, with this
    /// force flag. Dep auto-start uses `force: false` (the pre-flight
    /// inheritance — `--reload-images` is named-workload/batch scoped per
    /// USER DECISION D3, so deps get the plain skew matrix).
    Run { force: bool },
    /// The caller already ran the batch ensure pass
    /// (`cmd_workload_up_all`); do not re-run per spawn.
    AlreadyDone,
}

/// Start one service-kind workload DETACHED on its singleton slot with no
/// `--use` overrides and no per-slot flags (its own deps are already
/// started/satisfied by topo order). The spawned child is the ensured party
/// (spec 21 §2.2): its spec carries `images_ready = true`, and `detach_args`
/// appends `--images-ready` so the child skips the pre-flight.
async fn start_service_detached(name: &str, preflight: EnsurePreflight) -> Result<()> {
    if let EnsurePreflight::Run { force } = preflight {
        crate::images::ensure::ensure_images_for_workload(name, force).await?;
    }
    let workload =
        crate::microsandbox::workload::ConfigWorkload::new_with_use_overrides(name, &[])?;
    let mut spec = crate::commands::lifecycle::build_instance_spec(
        name,
        false,
        None,
        None,
        false,
        &[],
        false,
    )?;
    spec.images_ready = true;
    crate::microsandbox::runtime::up_service_with_spec(&workload, &spec, false).await
}

/// msb occupancy probe for a slot: true iff `Sandbox::get(instance)` resolves
/// Ok (the sandbox exists). Only a positive Ok counts as running — NotFound
/// → false; any other SDK error → false (the record-based plan stands; the
/// child's occupancy gate fail-closes when it matters).
async fn sandbox_running(instance: &str) -> Result<bool> {
    match Sandbox::get(instance).await {
        Ok(_) => Ok(true),
        Err(MicrosandboxError::SandboxNotFound(_)) => Ok(false),
        Err(_) => Ok(false),
    }
}

/// Short host-port health probe for an already-occupied dep slot: healthy iff
/// ANY published host port accepts a TCP connect within [`REUSE_PROBE_TIMEOUT`]
/// (shared deadline across ports). Targets come from the slot's registry
/// record (port_pairs/ports) when one exists, else the dep's DECLARED host
/// ports on the shared 127.0.0.1 bind. Returns Ok(None) when there are no
/// ports to probe (a no-port dep cannot be verified cheaply — reuse is then
/// decided optimistically; use `on_conflict = "replace"` to force a fresh
/// start).
fn probe_dep_health(state_dir: &Path, slot: &str, declared_ports: &[u16]) -> Result<Option<bool>> {
    let targets = probe_targets(state_dir, slot, declared_ports);
    if targets.is_empty() {
        return Ok(None);
    }
    let deadline = Instant::now() + REUSE_PROBE_TIMEOUT;
    for (ip, port) in targets {
        let remaining = deadline.saturating_duration_since(Instant::now());
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
/// shared 127.0.0.1 bind (mirrors [`readiness_targets`]' fallback, without
/// polling — the caller already knows the record view).
fn probe_targets(state_dir: &Path, slot: &str, declared_ports: &[u16]) -> Vec<(IpAddr, u16)> {
    if let Ok(records) = list_records(state_dir) {
        if let Some(rec) = singleton_record(&records, slot) {
            if !rec.port_pairs.is_empty() {
                return rec.port_pairs.iter().map(|p| (p.bind_ip, p.host)).collect();
            }
            if !rec.ports.is_empty() {
                return rec.ports.iter().map(|&p| (rec.bind_ip, p)).collect();
            }
        }
    }
    declared_ports
        .iter()
        .map(|&p| (IpAddr::V4(Ipv4Addr::LOCALHOST), p))
        .collect()
}

/// Whether the slot's record was created within [`BOOT_GRACE`] — the dep is
/// still booting and must never be replaced under "reuse".
fn recently_started(state_dir: &Path, slot: &str) -> Result<bool> {
    let records = list_records(state_dir)?;
    let Some(rec) = singleton_record(&records, slot) else {
        return Ok(false);
    };
    match record_age_secs(&rec.created_at) {
        Some(age) => Ok(age < BOOT_GRACE.as_secs()),
        None => Ok(false), // legacy/empty created_at — probe decides
    }
}

/// The slot of a planned action (both variants carry it).
fn slot_of_action(action: &DepStartAction) -> &str {
    match action {
        DepStartAction::StartService { slot, .. } => slot,
        DepStartAction::Satisfied { slot, .. } => slot,
    }
}

/// The declared ports of a planned action.
fn declared_ports_of(action: &DepStartAction) -> &[u16] {
    match action {
        DepStartAction::StartService { ports, .. } => ports,
        DepStartAction::Satisfied { ports, .. } => ports,
    }
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
                name: None,
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
                    conflict: DepConflict::Reuse,
                },
                DepStartAction::StartService {
                    dep: "b".to_string(),
                    slot: "personal-b".to_string(),
                    ports: vec![4001],
                    conflict: DepConflict::Reuse,
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
                    conflict: DepConflict::Reuse,
                },
                DepStartAction::Satisfied {
                    dep: "b".to_string(),
                    slot: "personal-b".to_string(),
                    ports: vec![4001],
                    conflict: DepConflict::Reuse,
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
                conflict: DepConflict::Reuse,
            }],
            "only the non-overridden dep may appear: {actions:?}"
        );
        Ok(())
    }

    /// A dep with an explicit `on_conflict` carries it into the planned
    /// action; deps without one default to "reuse".
    #[test]
    fn plan_dep_starts_carries_explicit_on_conflict() -> Result<()> {
        let toml = r#"
schema_version = 1

[workloads.a]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.a.depends_on.b]
env = "B_URL"
on_conflict = "replace"

[workloads.b]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[[workloads.b.ports]]
host = 4001
guest = 4001
"#;
        let config: ConfigFile = toml::from_str(toml).expect("fixture must parse");
        let actions = plan_dep_starts(&config, "a", &[], Some("personal"), &[])?;
        assert_eq!(
            actions,
            vec![DepStartAction::StartService {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4001],
                conflict: DepConflict::Replace,
            }],
            "explicit on_conflict must flow into the planned action"
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

    // ---- ADR 0026 addendum 2026-08-16: decide_dep_disposition ----

    fn start_action(dep: &str, conflict: DepConflict) -> DepStartAction {
        DepStartAction::StartService {
            dep: dep.to_string(),
            slot: format!("personal-{dep}"),
            ports: vec![4000],
            conflict,
        }
    }

    fn satisfied_action(dep: &str, conflict: DepConflict) -> DepStartAction {
        DepStartAction::Satisfied {
            dep: dep.to_string(),
            slot: format!("personal-{dep}"),
            ports: vec![4000],
            conflict,
        }
    }

    #[test]
    fn disposition_start_service_reuse_free_slot_starts() {
        assert_eq!(
            decide_dep_disposition(
                &start_action("b", DepConflict::Reuse),
                "a",
                false,
                None,
                false
            ),
            DepDisposition::Start {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
    }

    #[test]
    fn disposition_start_service_reuse_running_healthy_reuses() {
        assert_eq!(
            decide_dep_disposition(
                &start_action("b", DepConflict::Reuse),
                "a",
                true,
                Some(true),
                false
            ),
            DepDisposition::Reuse {
                dep: "b".to_string(),
                slot: "personal-b".to_string()
            }
        );
    }

    #[test]
    fn disposition_start_service_reuse_running_unhealthy_replaces() {
        // msb-running + dead port + old record → keep-alive zombie → replace.
        assert_eq!(
            decide_dep_disposition(
                &start_action("b", DepConflict::Reuse),
                "a",
                true,
                Some(false),
                false
            ),
            DepDisposition::Replace {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
    }

    #[test]
    fn disposition_start_service_reuse_running_unhealthy_but_booting_reuses() {
        // Dead port but the record is recent → still booting → reuse (never
        // kill a freshly-started dep).
        assert_eq!(
            decide_dep_disposition(
                &start_action("b", DepConflict::Reuse),
                "a",
                true,
                Some(false),
                true
            ),
            DepDisposition::Reuse {
                dep: "b".to_string(),
                slot: "personal-b".to_string()
            }
        );
    }

    #[test]
    fn disposition_start_service_reuse_running_no_ports_reuses() {
        // No ports to probe → cannot verify cheaply → reuse optimistically.
        let mut action = start_action("b", DepConflict::Reuse);
        if let DepStartAction::StartService { ports, .. } = &mut action {
            ports.clear();
        }
        assert_eq!(
            decide_dep_disposition(&action, "a", true, None, false),
            DepDisposition::Reuse {
                dep: "b".to_string(),
                slot: "personal-b".to_string()
            }
        );
    }

    #[test]
    fn disposition_start_service_replace_always_replaces() {
        assert_eq!(
            decide_dep_disposition(
                &start_action("b", DepConflict::Replace),
                "a",
                true,
                Some(true),
                false
            ),
            DepDisposition::Replace {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
        assert_eq!(
            decide_dep_disposition(
                &start_action("b", DepConflict::Replace),
                "a",
                false,
                None,
                false
            ),
            DepDisposition::Replace {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
    }

    #[test]
    fn disposition_start_service_fail_refuses_when_running_starts_when_free() {
        assert_eq!(
            decide_dep_disposition(
                &start_action("b", DepConflict::Fail),
                "a",
                true,
                None,
                false
            ),
            DepDisposition::Fail {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                workload: "a".to_string()
            }
        );
        assert_eq!(
            decide_dep_disposition(
                &start_action("b", DepConflict::Fail),
                "a",
                false,
                None,
                false
            ),
            DepDisposition::Start {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
    }

    #[test]
    fn disposition_satisfied_reuse_healthy_reuses() {
        assert_eq!(
            decide_dep_disposition(
                &satisfied_action("b", DepConflict::Reuse),
                "a",
                true,
                Some(true),
                false
            ),
            DepDisposition::Reuse {
                dep: "b".to_string(),
                slot: "personal-b".to_string()
            }
        );
    }

    #[test]
    fn disposition_satisfied_reuse_zombie_replaces() {
        // Record present + msb-running + dead port + old record → zombie.
        assert_eq!(
            decide_dep_disposition(
                &satisfied_action("b", DepConflict::Reuse),
                "a",
                true,
                Some(false),
                false
            ),
            DepDisposition::Replace {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
    }

    #[test]
    fn disposition_satisfied_reuse_stale_record_replaces() {
        // Record present but msb says the sandbox is gone → stale record:
        // down clears it, then start fresh.
        assert_eq!(
            decide_dep_disposition(
                &satisfied_action("b", DepConflict::Reuse),
                "a",
                false,
                None,
                false
            ),
            DepDisposition::Replace {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
    }

    #[test]
    fn disposition_satisfied_replace_and_fail() {
        assert_eq!(
            decide_dep_disposition(
                &satisfied_action("b", DepConflict::Replace),
                "a",
                true,
                Some(true),
                false
            ),
            DepDisposition::Replace {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
        assert_eq!(
            decide_dep_disposition(
                &satisfied_action("b", DepConflict::Fail),
                "a",
                true,
                None,
                false
            ),
            DepDisposition::Fail {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                workload: "a".to_string()
            }
        );
    }

    /// The Fail disposition carries the canonical refuse message (the same
    /// wording the runtime emits, pinned by ADR 0021 §2).
    #[test]
    fn disposition_fail_message_matches_canonical_refuse() {
        let d = decide_dep_disposition(
            &satisfied_action("litellm", DepConflict::Fail),
            "prime",
            true,
            None,
            false,
        );
        let DepDisposition::Fail { workload, slot, .. } = d else {
            panic!("expected Fail disposition");
        };
        let msg = format_refuse_message(&workload, &slot);
        assert!(
            msg.contains("is already running"),
            "Fail disposition must carry the occupied message: {msg}"
        );
    }
}
