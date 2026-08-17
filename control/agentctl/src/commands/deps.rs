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

use crate::config::{ConfigFile, ConflictStep, DepConflict, DepInstanceMode};
use crate::microsandbox::depgraph::{dep_closure, singleton_record, topo_all};
use crate::microsandbox::port_registry::{list_records, SandboxInstanceRecord};
use crate::microsandbox::runtime::reconcile::{ChainStep, ReconcileFacts};
use crate::microsandbox::runtime::{
    down_instance, format_refuse_message, wait_for_port, DownStatus, DEFAULT_WAIT,
};
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

/// The dep's auto-start conflict chain (ADR 0030 U11 precedence):
/// `depends_on.<dep>.on_conflict` > the DEP's own
/// `workloads.<dep>.instance.on_conflict` > the built-in default
/// ["reuse","start","replace"].
fn dep_conflict(config: &ConfigFile, dependent: &str, dep: &str) -> DepConflict {
    config
        .workloads
        .get(dependent)
        .and_then(|w| w.depends_on.get(dep))
        .and_then(|s| s.on_conflict.clone())
        .or_else(|| {
            config
                .workloads
                .get(dep)
                .and_then(|w| w.instance.on_conflict.clone())
        })
        .unwrap_or(DepConflict::default_chain())
}

/// The declaring config-repo namespace of `workload` (ADR 0030 Phase 2 T1):
/// resolved from the merge provenance (`workloads.<name>.<field>` → layer →
/// layer dir → repo_key), defaulting to "default" when no repo identity is
/// resolvable (legacy/synthetic layers, single-file mode).
/// The declaring config-repo namespace of `workload` (ADR 0030 Phase 2 T1):
/// resolved from the merge provenance (`workloads.<name>.<field>` → layer →
/// layer dir → repo_key), defaulting to "default" when no repo identity is
/// resolvable (legacy/synthetic layers, single-file mode).
///
/// The caller passes the provenance EXPLICITLY (it is a one-shot slot —
/// [`crate::merge::take_provenance`] drains it, so a second read returns
/// None); `ConfigWorkload::new_with_use_overrides` takes it once and threads
/// it here and into `resolve_depends_on`.
pub(crate) fn namespace_for(
    provenance: Option<&crate::merge::Provenance>,
    layer_dirs: &std::collections::HashMap<String, std::path::PathBuf>,
    workload: &str,
) -> String {
    let Some(provenance) = provenance else {
        return crate::microsandbox::port_registry::default_namespace();
    };
    // The workload's declaring layer: any field's provenance names the layer
    // that declared the workload (kind/image/depends_on all resolve to the
    // same declaring repo for a single-repo workload).
    let layer = provenance
        .get(&format!("workloads.{workload}.depends_on"))
        .or_else(|| provenance.get(&format!("workloads.{workload}.kind")))
        .or_else(|| provenance.get(&format!("workloads.{workload}.image")));
    let Some(layer) = layer else {
        return crate::microsandbox::port_registry::default_namespace();
    };
    let Some(dir) = layer_dirs.get(layer) else {
        return crate::microsandbox::port_registry::default_namespace();
    };
    let registered = crate::images::repo_key::registered_repo_checkouts();
    crate::images::repo_key::repo_key_for(dir, &registered)
}

/// The `depends_on.<dep>.instance` mode for a dependency (ADR 0030 §4.1 T2):
/// the declared `instance` when set, else derived from the DEP's own
/// `instance.strategy` (parallel → Fresh, else Shared).
fn dep_instance_mode(config: &ConfigFile, dependent: &str, dep: &str) -> DepInstanceMode {
    config
        .workloads
        .get(dependent)
        .and_then(|w| w.depends_on.get(dep))
        .and_then(|s| s.instance)
        .unwrap_or_else(|| {
            match config
                .workloads
                .get(dep)
                .map(|w| w.instance.strategy)
                .unwrap_or(crate::config::InstanceStrategy::Singleton)
            {
                crate::config::InstanceStrategy::Parallel => DepInstanceMode::Fresh,
                _ => DepInstanceMode::Shared,
            }
        })
}

impl DepStartAction {
    /// The dep's auto-start conflict chain (both variants carry it).
    fn conflict(&self) -> &DepConflict {
        match self {
            DepStartAction::StartService { conflict, .. } => conflict,
            DepStartAction::Satisfied { conflict, .. } => conflict,
        }
    }
}

/// Executor disposition for one planned dependency action after runtime
/// reconciliation (msb occupancy + host port health + record age). PURE —
/// unit-testable without KVM or the SDK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepDisposition {
    /// Reuse the running instance (healthy, or no ports to probe, or still
    /// within the boot grace window).
    Reuse { dep: String, slot: String },
    /// Start a stopped/crashed dep sandbox via msb `handle.start()` — the
    /// detached child executes it (ADR 0030 addendum 2 `start` element).
    StartExisting { dep: String, slot: String },
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
/// facts by iterating the action's conflict CHAIN in order (ADR 0030
/// addendum 2): the first element whose precondition holds wins. Returns
/// Err (chain exhausted) when no element applies.
pub fn decide_dep_disposition(
    chain: &[ConflictStep],
    action: &DepStartAction,
    workload_name: &str,
    facts: &ReconcileFacts,
) -> Result<DepDisposition> {
    let step = crate::microsandbox::runtime::reconcile::decide_chain(
        chain,
        facts,
        slot_of_action(action),
    )?;
    let (dep, slot, ports) = match action {
        DepStartAction::StartService {
            dep, slot, ports, ..
        } => (dep.clone(), slot.clone(), ports.clone()),
        DepStartAction::Satisfied {
            dep, slot, ports, ..
        } => (dep.clone(), slot.clone(), ports.clone()),
    };
    Ok(match step {
        ChainStep::Start => DepDisposition::Start { dep, slot, ports },
        ChainStep::Reuse => DepDisposition::Reuse { dep, slot },
        ChainStep::StartExisting => DepDisposition::StartExisting { dep, slot },
        ChainStep::Replace => DepDisposition::Replace { dep, slot, ports },
        ChainStep::Fail => DepDisposition::Fail {
            dep,
            slot,
            workload: workload_name.to_string(),
        },
    })
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
    namespace: &str,
) -> Result<Vec<DepStartAction>> {
    let closure = dep_closure(config, name)?;
    // ADR 0030 Phase 2 T1: the planner's record view is namespace-scoped — a
    // dependency's singleton slot is only "occupied" by a record in the
    // DEPENDENT's namespace (two repos declaring the same workload name are
    // isolated by their declaring-repo namespace).
    let records: Vec<SandboxInstanceRecord> = records
        .iter()
        .filter(|r| r.namespace == namespace)
        .cloned()
        .collect();
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
        // ADR 0030 T2: depends_on.<dep>.instance SELECTS the target slot.
        // P2 implements shared (the singleton slot, today's model) +
        // the strategy-derived default computation; scoped/fresh AUTO-START
        // (creating a PARALLEL dep instance through the detached child) is a
        // documented P2.1 follow-up — the dependent's instance id is not
        // available at this call depth and the creation machinery builds
        // singleton specs only.
        let mode = dep_instance_mode(config, name, &dep);
        match mode {
            DepInstanceMode::Shared => {}
            DepInstanceMode::Scoped | DepInstanceMode::Fresh
                if config
                    .workloads
                    .get(name)
                    .and_then(|w| w.depends_on.get(&dep))
                    .and_then(|s| s.instance)
                    .is_some() =>
            {
                // EXPLICIT scoped/fresh (opt-in): honest hard error — the
                // parallel dep auto-start is not wired in this phase. Power
                // users can still select a manually-started parallel dep via
                // `--use <dep>@<id>` (which skips this dep in the planner).
                anyhow::bail!(
                    "dependency '{dep}' of '{name}' declares `instance = '{mode}'`: scoped/fresh \
                     dep auto-start is a P2.1 follow-up (parallel dep creation is not wired). \
                     Start the dep instance manually and select it with `--use {dep}@<id>`, or \
                     omit `instance` for shared behavior."
                );
            }
            DepInstanceMode::Scoped | DepInstanceMode::Fresh => {
                // DERIVED fresh (the dep's own `strategy = "parallel"`): warn
                // + fall back to the shared singleton (the pre-P2 behavior);
                // non-breaking — a parallel-strategy dep does not force its
                // dependents onto fresh in this phase.
                eprintln!(
                    "warning: dependency '{dep}' of '{name}': the dep's `instance.strategy = 'parallel'` \
                     defaults this entry to `instance = 'fresh'` (ADR 0030 T2), but scoped/fresh dep \
                     auto-start is a P2.1 follow-up — falling back to the shared singleton slot."
                );
            }
        }
        if singleton_record(&records, &slot).is_some() {
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
    // ADR 0030 Phase 2 T1: the dependent's declaring-repo namespace scopes
    // which dep records are visible to auto-start. auto_start_dependencies
    // runs BEFORE the dependent's ConfigWorkload is constructed, so the
    // provenance slot is still full here — take it (and the layer dirs) and
    // pass them explicitly.
    let provenance = crate::merge::take_provenance();
    let layer_dirs = crate::merge::get_layer_dirs().unwrap_or_default();
    let namespace = namespace_for(provenance.as_ref(), &layer_dirs, workload_name);
    let actions = plan_dep_starts(
        &config,
        workload_name,
        &records,
        context.as_deref(),
        use_overrides,
        &namespace,
    )?;
    for action in actions {
        // ADR 0030 Phase 0: reconcile the planner's record view with the msb
        // runtime + sandbox dir + host-port liveness BEFORE starting, then
        // route the disposition through the dep's conflict CHAIN (default
        // ["reuse", "start", "replace"]). The record-as-authoritative planner
        // can miss a running sandbox when the registry record is
        // absent/stale in the active state dir.
        let facts = crate::microsandbox::runtime::reconcile::gather_facts(
            &state_dir,
            slot_of_action(&action),
            declared_ports_of(&action),
        )
        .await?;
        let mut chain = action.conflict().0.clone();
        let disposition = loop {
            match decide_dep_disposition(&chain, &action, workload_name, &facts)? {
                DepDisposition::StartExisting { dep, slot } => {
                    // The detached child re-reconciles and executes
                    // handle.start(); if that fails the child errors and we
                    // fall through here — advance the chain past `start`.
                    match start_service_detached(&dep, EnsurePreflight::Run { force: false }).await
                    {
                        Ok(()) => break DepDisposition::StartExisting { dep, slot },
                        Err(e) => {
                            match crate::microsandbox::runtime::reconcile::chain_after(
                                &chain,
                                ConflictStep::Start,
                            ) {
                                Some(rest) => {
                                    chain = rest.to_vec();
                                    continue;
                                }
                                None => anyhow::bail!(
                                    "conflict chain exhausted for '{}' after a failed start: {}",
                                    slot,
                                    e
                                ),
                            }
                        }
                    }
                }
                d => break d,
            }
        };
        match disposition {
            DepDisposition::Reuse { dep, slot } => {
                println!("dependency '{dep}' already running (slot '{slot}') — reusing");
            }
            DepDisposition::StartExisting { dep, slot } => {
                println!("started dependency '{dep}' (slot '{slot}') [started stopped sandbox]");
                wait_until_ready(
                    &state_dir,
                    &format!("dependency '{dep}' of '{workload_name}'"),
                    &slot,
                    declared_ports_of(&action),
                )?;
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
    use microsandbox::sandbox::SandboxStatus;

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
            "default",
        )
    }

    // (i) Chain a→b→c, all slots free: both deps start, deepest first, with
    // their declared ports. Omitted on_conflict resolves to the ADR 0030
    // default chain at plan time.
    #[test]
    fn plan_dep_starts_chain_all_absent_starts_in_topo_order() -> Result<()> {
        let config = chain_config();
        let actions = plan_dep_starts(&config, "a", &[], Some("personal"), &[], "default")?;
        assert_eq!(
            actions,
            vec![
                DepStartAction::StartService {
                    dep: "c".to_string(),
                    slot: "personal-c".to_string(),
                    ports: vec![4002],
                    conflict: DepConflict::default_chain(),
                },
                DepStartAction::StartService {
                    dep: "b".to_string(),
                    slot: "personal-b".to_string(),
                    ports: vec![4001],
                    conflict: DepConflict::default_chain(),
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

        let actions = plan_dep_starts(&config, "a", &records, Some("personal"), &[], "default")?;
        assert_eq!(
            actions,
            vec![
                DepStartAction::StartService {
                    dep: "c".to_string(),
                    slot: "personal-c".to_string(),
                    ports: vec![4002],
                    conflict: DepConflict::default_chain(),
                },
                DepStartAction::Satisfied {
                    dep: "b".to_string(),
                    slot: "personal-b".to_string(),
                    ports: vec![4001],
                    conflict: DepConflict::default_chain(),
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
        let err =
            plan_dep_starts(&config, "top", &[], Some("personal"), &[], "default").unwrap_err();
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
        let actions = plan_dep_starts(&config, "a", &[], Some("personal"), &overrides, "default")?;
        assert_eq!(
            actions,
            vec![DepStartAction::StartService {
                dep: "c".to_string(),
                slot: "personal-c".to_string(),
                ports: vec![4002],
                conflict: DepConflict::default_chain(),
            }],
            "only the non-overridden dep may appear: {actions:?}"
        );
        Ok(())
    }

    /// A dep with an explicit `on_conflict` carries it into the planned
    /// action; deps without one default to the ADR 0030 default chain.
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
        let actions = plan_dep_starts(&config, "a", &[], Some("personal"), &[], "default")?;
        assert_eq!(
            actions,
            vec![DepStartAction::StartService {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4001],
                conflict: DepConflict::replace(),
            }],
            "explicit on_conflict must flow into the planned action"
        );
        Ok(())
    }

    /// ADR 0030 U11 precedence: when the DEPENDENT's depends_on spec has no
    /// on_conflict, the DEP's own `workloads.<dep>.instance.on_conflict`
    /// supplies the chain.
    #[test]
    fn dep_conflict_falls_back_to_dep_own_instance_policy() -> Result<()> {
        let toml = r#"
schema_version = 1

[workloads.a]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.a.depends_on.litellm]
env = "LITELLM_URL"

[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[[workloads.litellm.ports]]
host = 4000
guest = 4000

[workloads.litellm.instance]
on_conflict = "fail"
"#;
        let config: ConfigFile = toml::from_str(toml).expect("fixture must parse");
        let actions = plan_dep_starts(&config, "a", &[], Some("personal"), &[], "default")?;
        assert_eq!(
            actions,
            vec![DepStartAction::StartService {
                dep: "litellm".to_string(),
                slot: "personal-litellm".to_string(),
                ports: vec![4000],
                conflict: DepConflict::fail(),
            }],
            "the dep's own instance.on_conflict must supply the chain when depends_on omits it"
        );
        Ok(())
    }

    /// ADR 0030 U11 precedence: `depends_on.<dep>.on_conflict` BEATS the
    /// dep's own `workloads.<dep>.instance.on_conflict`.
    #[test]
    fn dep_conflict_depends_on_beats_dep_policy() -> Result<()> {
        let toml = r#"
schema_version = 1

[workloads.a]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.a.depends_on.litellm]
env = "LITELLM_URL"
on_conflict = "replace"

[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[[workloads.litellm.ports]]
host = 4000
guest = 4000

[workloads.litellm.instance]
on_conflict = "fail"
"#;
        let config: ConfigFile = toml::from_str(toml).expect("fixture must parse");
        let actions = plan_dep_starts(&config, "a", &[], Some("personal"), &[], "default")?;
        assert_eq!(
            actions,
            vec![DepStartAction::StartService {
                dep: "litellm".to_string(),
                slot: "personal-litellm".to_string(),
                ports: vec![4000],
                conflict: DepConflict::replace(),
            }],
            "depends_on.on_conflict must take precedence over the dep's own instance policy"
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

    // ---- ADR 0026 addendum 2026-08-16 / ADR 0030 addendum 2:
    // decide_dep_disposition (chain-iterating over ReconcileFacts) ----

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

    /// A minimal registry record for fact fixtures (only the fields the
    /// decision reads).
    fn record() -> SandboxInstanceRecord {
        SandboxInstanceRecord {
            instance: "personal-b".to_string(),
            context: Some("personal".to_string()),
            workload: "b".to_string(),
            ports: vec![4000],
            port_pairs: vec![PortMapping::new(4000, 4000)],
            created_at: "2026-01-01T00:00:00Z".to_string(),
            bind_ip: IpAddr::V4(Ipv4Addr::LOCALHOST),
            namespace: crate::microsandbox::port_registry::default_namespace(),
        }
    }

    /// Build facts for a pure decision test. `msb_status` None + no record +
    /// no dir → free slot.
    fn facts(
        record: Option<SandboxInstanceRecord>,
        msb_status: Option<SandboxStatus>,
        healthy: Option<bool>,
        recently_started: bool,
    ) -> ReconcileFacts {
        ReconcileFacts {
            record,
            msb_status,
            msb_unavailable: false,
            dir_exists: false,
            healthy,
            recently_started,
        }
    }

    /// msb-running facts (record present, old).
    fn running(healthy: Option<bool>, recently_started: bool) -> ReconcileFacts {
        facts(
            Some(record()),
            Some(SandboxStatus::Running),
            healthy,
            recently_started,
        )
    }

    /// Free-slot facts (no row, no dir, no record).
    fn free() -> ReconcileFacts {
        facts(None, None, None, false)
    }

    #[test]
    fn disposition_start_service_reuse_free_slot_starts() {
        assert_eq!(
            decide_dep_disposition(
                &DepConflict::default_chain().0,
                &start_action("b", DepConflict::reuse()),
                "a",
                &free()
            )
            .unwrap(),
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
                &DepConflict::default_chain().0,
                &start_action("b", DepConflict::reuse()),
                "a",
                &running(Some(true), false)
            )
            .unwrap(),
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
                &DepConflict::default_chain().0,
                &start_action("b", DepConflict::reuse()),
                "a",
                &running(Some(false), false)
            )
            .unwrap(),
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
                &DepConflict::default_chain().0,
                &start_action("b", DepConflict::reuse()),
                "a",
                &running(Some(false), true)
            )
            .unwrap(),
            DepDisposition::Reuse {
                dep: "b".to_string(),
                slot: "personal-b".to_string()
            }
        );
    }

    #[test]
    fn disposition_start_service_reuse_running_no_ports_reuses() {
        // No ports to probe → cannot verify cheaply → reuse optimistically.
        let mut action = start_action("b", DepConflict::reuse());
        if let DepStartAction::StartService { ports, .. } = &mut action {
            ports.clear();
        }
        assert_eq!(
            decide_dep_disposition(
                &DepConflict::default_chain().0,
                &action,
                "a",
                &running(None, false)
            )
            .unwrap(),
            DepDisposition::Reuse {
                dep: "b".to_string(),
                slot: "personal-b".to_string()
            }
        );
    }

    #[test]
    fn disposition_start_service_replace_always_replaces() {
        // Occupied (msb running) → Replace.
        assert_eq!(
            decide_dep_disposition(
                &[ConflictStep::Replace],
                &start_action("b", DepConflict::replace()),
                "a",
                &running(Some(true), false)
            )
            .unwrap(),
            DepDisposition::Replace {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
        // Stale record (msb gone) → Replace too: `down` clears the stale
        // record, then start fresh.
        assert_eq!(
            decide_dep_disposition(
                &[ConflictStep::Replace],
                &start_action("b", DepConflict::replace()),
                "a",
                &facts(Some(record()), None, None, false)
            )
            .unwrap(),
            DepDisposition::Replace {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
        // A GENUINELY free slot (no row, no record, no dir) is a plain Start
        // regardless of chain — there is nothing to replace (the ADR 0030
        // free-slot shortcut).
        assert_eq!(
            decide_dep_disposition(
                &[ConflictStep::Replace],
                &start_action("b", DepConflict::replace()),
                "a",
                &free()
            )
            .unwrap(),
            DepDisposition::Start {
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
                &[ConflictStep::Fail],
                &start_action("b", DepConflict::fail()),
                "a",
                &running(None, false)
            )
            .unwrap(),
            DepDisposition::Fail {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                workload: "a".to_string()
            }
        );
        assert_eq!(
            decide_dep_disposition(
                &[ConflictStep::Fail],
                &start_action("b", DepConflict::fail()),
                "a",
                &free()
            )
            .unwrap(),
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
                &DepConflict::default_chain().0,
                &satisfied_action("b", DepConflict::reuse()),
                "a",
                &running(Some(true), false)
            )
            .unwrap(),
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
                &DepConflict::default_chain().0,
                &satisfied_action("b", DepConflict::reuse()),
                "a",
                &running(Some(false), false)
            )
            .unwrap(),
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
                &DepConflict::default_chain().0,
                &satisfied_action("b", DepConflict::reuse()),
                "a",
                &facts(Some(record()), None, None, false)
            )
            .unwrap(),
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
                &[ConflictStep::Replace],
                &satisfied_action("b", DepConflict::replace()),
                "a",
                &running(Some(true), false)
            )
            .unwrap(),
            DepDisposition::Replace {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
        assert_eq!(
            decide_dep_disposition(
                &[ConflictStep::Fail],
                &satisfied_action("b", DepConflict::fail()),
                "a",
                &running(None, false)
            )
            .unwrap(),
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
            &[ConflictStep::Fail],
            &satisfied_action("litellm", DepConflict::fail()),
            "prime",
            &running(None, false),
        )
        .unwrap();
        let DepDisposition::Fail { workload, slot, .. } = d else {
            panic!("expected Fail disposition");
        };
        let msg = format_refuse_message(&workload, &slot);
        assert!(
            msg.contains("is already running"),
            "Fail disposition must carry the occupied message: {msg}"
        );
    }

    // ---- ADR 0030 addendum 2 U4: conflict-chain decision matrix ----

    #[test]
    fn chain_default_running_healthy_reuses() {
        assert_eq!(
            decide_dep_disposition(
                &DepConflict::default_chain().0,
                &start_action("b", DepConflict::default_chain()),
                "a",
                &running(Some(true), false)
            )
            .unwrap(),
            DepDisposition::Reuse {
                dep: "b".to_string(),
                slot: "personal-b".to_string()
            }
        );
    }

    #[test]
    fn chain_default_stopped_starts_existing() {
        // Stopped → StartExisting (the `start` element).
        let stopped = facts(Some(record()), Some(SandboxStatus::Stopped), None, false);
        assert_eq!(
            decide_dep_disposition(
                &DepConflict::default_chain().0,
                &start_action("b", DepConflict::default_chain()),
                "a",
                &stopped
            )
            .unwrap(),
            DepDisposition::StartExisting {
                dep: "b".to_string(),
                slot: "personal-b".to_string()
            }
        );
        // Crashed → StartExisting too.
        let crashed = facts(Some(record()), Some(SandboxStatus::Crashed), None, false);
        assert_eq!(
            decide_dep_disposition(
                &DepConflict::default_chain().0,
                &start_action("b", DepConflict::default_chain()),
                "a",
                &crashed
            )
            .unwrap(),
            DepDisposition::StartExisting {
                dep: "b".to_string(),
                slot: "personal-b".to_string()
            }
        );
    }

    #[test]
    fn chain_default_stopped_start_fails_falls_to_replace() {
        // The executor retry: the default chain first yields StartExisting;
        // after a failed start the executor advances past `start` and the
        // remainder (["replace"]) yields Replace.
        let stopped = facts(Some(record()), Some(SandboxStatus::Stopped), None, false);
        let chain = DepConflict::default_chain().0;
        assert_eq!(
            decide_dep_disposition(
                &chain,
                &start_action("b", DepConflict::default_chain()),
                "a",
                &stopped
            )
            .unwrap(),
            DepDisposition::StartExisting {
                dep: "b".to_string(),
                slot: "personal-b".to_string()
            }
        );
        let rest =
            crate::microsandbox::runtime::reconcile::chain_after(&chain, ConflictStep::Start)
                .expect("default chain contains start");
        assert_eq!(
            decide_dep_disposition(
                rest,
                &start_action("b", DepConflict::default_chain()),
                "a",
                &stopped
            )
            .unwrap(),
            DepDisposition::Replace {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
    }

    #[test]
    fn chain_default_zombie_replaces() {
        assert_eq!(
            decide_dep_disposition(
                &DepConflict::default_chain().0,
                &start_action("b", DepConflict::default_chain()),
                "a",
                &running(Some(false), false)
            )
            .unwrap(),
            DepDisposition::Replace {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
    }

    #[test]
    fn chain_default_stale_record_replaces() {
        assert_eq!(
            decide_dep_disposition(
                &DepConflict::default_chain().0,
                &satisfied_action("b", DepConflict::default_chain()),
                "a",
                &facts(Some(record()), None, None, false)
            )
            .unwrap(),
            DepDisposition::Replace {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
    }

    #[test]
    fn chain_default_booting_reuses() {
        assert_eq!(
            decide_dep_disposition(
                &DepConflict::default_chain().0,
                &start_action("b", DepConflict::default_chain()),
                "a",
                &running(Some(false), true)
            )
            .unwrap(),
            DepDisposition::Reuse {
                dep: "b".to_string(),
                slot: "personal-b".to_string()
            }
        );
    }

    #[test]
    fn chain_scalar_replace_always_replaces() {
        // Occupied (msb running) → Replace.
        assert_eq!(
            decide_dep_disposition(
                &[ConflictStep::Replace],
                &start_action("b", DepConflict::replace()),
                "a",
                &running(Some(true), false)
            )
            .unwrap(),
            DepDisposition::Replace {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
        // Stale record (msb gone) → Replace too.
        assert_eq!(
            decide_dep_disposition(
                &[ConflictStep::Replace],
                &start_action("b", DepConflict::replace()),
                "a",
                &facts(Some(record()), None, None, false)
            )
            .unwrap(),
            DepDisposition::Replace {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
        // A genuinely free slot is a plain Start regardless of chain (the
        // ADR 0030 free-slot shortcut — nothing to replace).
        assert_eq!(
            decide_dep_disposition(
                &[ConflictStep::Replace],
                &start_action("b", DepConflict::replace()),
                "a",
                &free()
            )
            .unwrap(),
            DepDisposition::Start {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
    }

    #[test]
    fn chain_scalar_fail_fails_when_occupied_starts_when_free() {
        assert_eq!(
            decide_dep_disposition(
                &[ConflictStep::Fail],
                &start_action("b", DepConflict::fail()),
                "a",
                &running(None, false)
            )
            .unwrap(),
            DepDisposition::Fail {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                workload: "a".to_string()
            }
        );
        assert_eq!(
            decide_dep_disposition(
                &[ConflictStep::Fail],
                &start_action("b", DepConflict::fail()),
                "a",
                &free()
            )
            .unwrap(),
            DepDisposition::Start {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
    }

    #[test]
    fn chain_reuse_fail_reuses_when_healthy_fails_when_zombie() {
        // ["reuse","fail"]: healthy → Reuse; zombie → Fail (NOT Replace —
        // there is no replace element in the chain).
        assert_eq!(
            decide_dep_disposition(
                &[ConflictStep::Reuse, ConflictStep::Fail],
                &start_action("b", DepConflict::reuse()),
                "a",
                &running(Some(true), false)
            )
            .unwrap(),
            DepDisposition::Reuse {
                dep: "b".to_string(),
                slot: "personal-b".to_string()
            }
        );
        assert_eq!(
            decide_dep_disposition(
                &[ConflictStep::Reuse, ConflictStep::Fail],
                &start_action("b", DepConflict::reuse()),
                "a",
                &running(Some(false), false)
            )
            .unwrap(),
            DepDisposition::Fail {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                workload: "a".to_string()
            }
        );
    }

    #[test]
    fn chain_start_replace_starts_when_stopped_replaces_when_zombie() {
        // ["start","replace"]: Stopped → StartExisting; zombie → Replace.
        let stopped = facts(Some(record()), Some(SandboxStatus::Stopped), None, false);
        assert_eq!(
            decide_dep_disposition(
                &[ConflictStep::Start, ConflictStep::Replace],
                &start_action("b", DepConflict::reuse()),
                "a",
                &stopped
            )
            .unwrap(),
            DepDisposition::StartExisting {
                dep: "b".to_string(),
                slot: "personal-b".to_string()
            }
        );
        assert_eq!(
            decide_dep_disposition(
                &[ConflictStep::Start, ConflictStep::Replace],
                &start_action("b", DepConflict::reuse()),
                "a",
                &running(Some(false), false)
            )
            .unwrap(),
            DepDisposition::Replace {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            }
        );
    }

    #[test]
    fn chain_reuse_only_zombie_exhausted_error_lists_attempts() {
        // ["reuse"] on a zombie: no element applies → chain-exhausted error
        // listing the attempt.
        let err = decide_dep_disposition(
            &[ConflictStep::Reuse],
            &start_action("b", DepConflict::reuse()),
            "a",
            &running(Some(false), false),
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("conflict chain exhausted"),
            "exhausted chain must error: {msg}"
        );
        assert!(msg.contains("reuse"), "error must list the attempt: {msg}");
        assert!(
            msg.contains("probe failed"),
            "error must name the failure mode: {msg}"
        );
    }

    // ---- ADR 0030 Phase 2: depends_on.<dep>.instance (DepInstanceMode) ----

    /// The strategy-derived default: a dep whose own `instance.strategy` is
    /// "parallel" defaults the entry to Fresh; singleton/replace/reuse default
    /// to Shared.
    #[test]
    fn dep_instance_mode_strategy_derived_default() -> Result<()> {
        let toml = r#"
schema_version = 1

[workloads.a]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.a.depends_on.litellm]
env = "LITELLM_URL"

[workloads.a.depends_on.redis]
env = "REDIS_URL"

[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.litellm.instance]
strategy = "parallel"

[workloads.redis]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []
"#;
        let config: ConfigFile = toml::from_str(toml).expect("fixture must parse");
        assert_eq!(
            dep_instance_mode(&config, "a", "litellm"),
            DepInstanceMode::Fresh,
            "a parallel-strategy dep defaults the entry to fresh"
        );
        assert_eq!(
            dep_instance_mode(&config, "a", "redis"),
            DepInstanceMode::Shared,
            "a singleton-strategy dep defaults the entry to shared"
        );
        Ok(())
    }

    /// A declared `instance` beats the strategy-derived default.
    #[test]
    fn dep_instance_mode_declared_beats_strategy() -> Result<()> {
        let toml = r#"
schema_version = 1

[workloads.a]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.a.depends_on.litellm]
env = "LITELLM_URL"
instance = "scoped"

[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.litellm.instance]
strategy = "parallel"
"#;
        let config: ConfigFile = toml::from_str(toml).expect("fixture must parse");
        assert_eq!(
            dep_instance_mode(&config, "a", "litellm"),
            DepInstanceMode::Scoped,
            "a declared instance mode beats the dep's parallel strategy"
        );
        Ok(())
    }

    /// An EXPLICIT scoped/fresh entry is an honest plan-time error: the
    /// parallel dep auto-start is a P2.1 follow-up (the dependent's instance
    /// id is not available at this call depth and the creation machinery
    /// builds singleton specs only). The error names the follow-up and the
    /// `--use` workaround.
    #[test]
    fn plan_dep_starts_explicit_scoped_fresh_is_a_clear_p21_error() -> Result<()> {
        let toml = r#"
schema_version = 1

[workloads.a]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.a.depends_on.litellm]
env = "LITELLM_URL"
instance = "scoped"

[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []
"#;
        let config: ConfigFile = toml::from_str(toml).expect("fixture must parse");
        let err = plan_dep_starts(&config, "a", &[], Some("personal"), &[], "default").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("P2.1 follow-up"),
            "scoped/fresh auto-start must name the follow-up: {msg}"
        );
        assert!(
            msg.contains("--use litellm@<id>"),
            "the error must offer the --use workaround: {msg}"
        );
        Ok(())
    }

    /// A DERIVED fresh entry (the dep's own parallel strategy, no explicit
    /// `instance`) warns + falls back to the shared singleton — non-breaking.
    #[test]
    fn plan_dep_starts_derived_fresh_warns_and_falls_back_to_shared() -> Result<()> {
        let toml = r#"
schema_version = 1

[workloads.a]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.a.depends_on.litellm]
env = "LITELLM_URL"

[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []
"#;
        let config: ConfigFile = toml::from_str(toml).expect("fixture must parse");
        let actions = plan_dep_starts(&config, "a", &[], Some("personal"), &[], "default")?;
        assert_eq!(
            actions,
            vec![DepStartAction::StartService {
                dep: "litellm".to_string(),
                slot: "personal-litellm".to_string(),
                ports: vec![],
                conflict: DepConflict::default_chain(),
            }],
            "a singleton-strategy dep with no declared instance keeps the shared singleton target"
        );
        Ok(())
    }

    // ---- ADR 0030 Phase 2: namespace_for (provenance → repo_key) ----

    /// `namespace_for` resolves the declaring layer from provenance, maps it
    /// through the layer dirs, and keys it by the registered repo name.
    #[test]
    fn namespace_for_resolves_declaring_repo_from_provenance() -> Result<()> {
        use crate::config::test_support::{uniq_dir, EnvGuard, ENV_TEST_LOCK, HOME_ENV_KEYS};
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let tmp = uniq_dir("ns-for");
        let checkout = tmp.join("checkout");
        std::fs::create_dir_all(checkout.join("workestrate").join("workloads"))?;

        let mut provenance = crate::merge::Provenance::new();
        provenance.insert(
            "workloads.pi.depends_on".to_string(),
            "personal".to_string(),
        );
        let mut dirs = std::collections::HashMap::new();
        dirs.insert(
            "personal".to_string(),
            checkout.join("workestrate").join("workloads"),
        );

        // The registry must resolve the checkout dir to the registered name.
        // registered_repo_checkouts reads the live registry — set one via the
        // config-registry test helper if available, else assert the canonical
        // PATH form (repo_key_for falls back to the canonical path when the
        // dir is not registered). The path fallback is the truthful
        // unregistered posture.
        let ns = namespace_for(Some(&provenance), &dirs, "pi");
        assert!(
            !ns.is_empty() && ns != "default",
            "namespace must resolve to a repo identity (got '{ns}')"
        );
        // No provenance → "default" (legacy/synthetic).
        assert_eq!(
            namespace_for(None, &dirs, "pi"),
            crate::microsandbox::port_registry::default_namespace()
        );
        // Provenance names an unknown layer → "default".
        let mut stray = crate::merge::Provenance::new();
        stray.insert("workloads.pi.kind".to_string(), "ghost".to_string());
        assert_eq!(
            namespace_for(Some(&stray), &dirs, "pi"),
            crate::microsandbox::port_registry::default_namespace()
        );
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }
}
