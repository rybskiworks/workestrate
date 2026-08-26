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
//!   running is a hard error BEFORE anything starts. ADR 0030 P2.1:
//!   `depends_on.<dep>.instance = "scoped"` targets the dep instance named
//!   `<dependent>-<id>` of the dependent's own parallel instance id;
//!   `"fresh"` always plans a start (the executor allocates the slug).
//! - EXECUTORS ([`auto_start_dependencies`], [`cmd_workload_up_all`]): thin
//!   KVM-dependent shells around the planning seam. Service-kind deps start
//!   DETACHED with a bounded wait-for-port ([`DEFAULT_WAIT`]); a dep with no
//!   ports skips the wait. `--no-deps` and non-start verbs are no-ops. The
//!   executor also RECONCILES occupancy with the msb runtime: a slot that msb
//!   reports Running is never blind-started (the record-as-authoritative
//!   planner can miss a running sandbox when the port-registry record is
//!   absent/stale in the active state dir). ADR 0030 P0.2:
//!   [`cmd_workload_up_all`] runs the same reconcile pass parent-side over
//!   BOTH the already-running skips and the planned starts, so a zombie slot
//!   converges (replace) and a record-less-but-running sandbox is reused
//!   with an accurate message.

use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::Result;

use crate::config::{ConfigFile, ConflictStep, DepConflict, DepInstanceMode};
use crate::microsandbox::depgraph::{dep_closure, singleton_record, topo_all};
use crate::microsandbox::port_registry::{auto_allocate_slug, list_records, SandboxInstanceRecord};
use crate::microsandbox::runtime::reconcile::{ChainStep, ReconcileFacts};
use crate::microsandbox::runtime::{
    down_instance, format_refuse_message, wait_for_port, DownStatus, DEFAULT_WAIT,
};
use crate::microsandbox::slots::{instance_id_of, instance_name, slot_for, validate_instance_id};

/// How long to poll the registry for a freshly-started dep's singleton
/// record before falling back to its DECLARED host ports (bounded within
/// the overall [`DEFAULT_WAIT`] readiness budget).
const RECORD_POLL_BUDGET: Duration = Duration::from_secs(5);

/// Registry poll interval while waiting for the detached child's record.
const RECORD_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// One planned action for a dependency of a workload, in topological START
/// order (every dep appears after all of its own deps).
///
/// Both variants carry the TARGET INSTANCE identity (`instance`): the full
/// instance name (`<slot>` or `<slot>@<id>`) the executor reconciles,
/// messages about, and waits on. `None` = the singleton slot (the shared
/// model). A SCOPED dep of a parallel dependent carries
/// `<dep-slot>@<dependent>-<dependent-id>`; a FRESH dep is planned with
/// `instance: None` + `fresh: true` and the EXECUTOR allocates the concrete
/// slug before reconciling (the planner stays pure — it has no state_dir).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DepStartAction {
    /// The dep's target instance is free: start it detached, then wait for
    /// each published host port.
    StartService {
        dep: String,
        slot: String,
        /// Full target instance name; `None` = the singleton slot. A FRESH
        /// action is planned with `None` here — the executor fills it with
        /// the auto-allocated slug before reconciling.
        instance: Option<String>,
        /// True for a FRESH-mode dep (`depends_on.<dep>.instance = "fresh"`
        /// or derived from the dep's `strategy = "parallel"`): always a
        /// start, never Satisfied.
        fresh: bool,
        /// Declared host ports (fallback readiness targets when the
        /// registry record is not yet visible).
        ports: Vec<u16>,
        /// The dep's auto-start conflict policy (default "reuse").
        conflict: DepConflict,
    },
    /// The dep's target instance is already occupied
    /// (record-as-authoritative) — reuse, never restart a running dep; the
    /// executor may still probe health and auto-replace a keep-alive zombie
    /// under "reuse".
    Satisfied {
        dep: String,
        slot: String,
        /// Full target instance name; `None` = the singleton slot.
        instance: Option<String>,
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
    // The namespace is the declaring REPO identity. A layer whose declaring
    // dir is NOT a registered config-repo checkout (synthetic / single-file /
    // test temp dir) has no repo identity -> the legacy "default" namespace.
    let registered = crate::images::repo_key::registered_repo_checkouts();
    crate::images::repo_key::repo_key_for_optional(dir, &registered)
        .unwrap_or_else(crate::microsandbox::port_registry::default_namespace)
}

/// The `depends_on.<dep>.instance` mode for a dependency (ADR 0030 §4.1 T2):
/// the declared `instance` when set, else derived from the DEP's own
/// `instance.strategy` (parallel → Fresh, else Shared). `pub(crate)` so
/// discovery ([`crate::microsandbox::discovery::resolve_depends_on_full`])
/// applies the SAME derivation for mode-aware default exports selection —
/// never duplicate this logic.
pub(crate) fn dep_instance_mode(
    config: &ConfigFile,
    dependent: &str,
    dep: &str,
) -> DepInstanceMode {
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
///
/// The `slot` fields carry the TARGET INSTANCE NAME
/// ([`target_of_action`]) — the singleton slot for a shared dep, the
/// composed `<slot>@<dependent>-<id>` name for a scoped dep, the
/// auto-slugged `<slot>@<slug>` for a fresh dep — so messages, teardown,
/// and readiness waits all hit the parallel instance when there is one.
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
///
/// ADR 0030 Phase 2 T1: `namespace` is the DEPENDENT's declaring-repo
/// namespace. A registry record in a FOREIGN namespace never satisfies the
/// dependent — the Reuse/StartExisting dispositions must NOT adopt it (the
/// namespace-blind reuse previously printed "already running — reusing" for
/// a record the dependent's namespace-filtered resolution then refused).
/// The refusal is collision-style (mirrors discovery.rs), naming the dep,
/// the dependent, the required namespace, and the namespace holding the
/// record. A zombie/stale foreign record still converges through the chain's
/// replace element; a record-less running sandbox is unchanged.
pub fn decide_dep_disposition(
    chain: &[ConflictStep],
    action: &DepStartAction,
    workload_name: &str,
    facts: &ReconcileFacts,
    namespace: &str,
) -> Result<DepDisposition> {
    let step = crate::microsandbox::runtime::reconcile::decide_chain(
        chain,
        facts,
        target_of_action(action),
    )?;
    if matches!(step, ChainStep::Reuse | ChainStep::StartExisting) {
        if let Some(found) =
            crate::microsandbox::runtime::reconcile::foreign_namespace_record(facts, namespace)
        {
            let dep = match action {
                DepStartAction::StartService { dep, .. }
                | DepStartAction::Satisfied { dep, .. } => dep,
            };
            anyhow::bail!(
                "dependency '{dep}' of workload '{workload_name}' cannot reuse instance '{}': \
                 its registry record is in namespace '{found}' but the dependent requires \
                 namespace '{namespace}' — the same workload name is declared by multiple \
                 config repos (last layer wins in the merged config). Start it in this \
                 namespace with `workestrate workload up {dep} --replace`, or remove the \
                 foreign record.",
                target_of_action(action),
            );
        }
    }
    let (dep, slot, ports) = match action {
        DepStartAction::StartService {
            dep,
            slot,
            instance,
            ports,
            ..
        }
        | DepStartAction::Satisfied {
            dep,
            slot,
            instance,
            ports,
            ..
        } => (
            dep.clone(),
            instance.clone().unwrap_or_else(|| slot.clone()),
            ports.clone(),
        ),
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
/// - target instance occupied (a record exists) → [`DepStartAction::Satisfied`];
/// - free + service-kind → [`DepStartAction::StartService`] with the dep's
///   declared host ports;
/// - free + agent-kind → hard error naming dep + dependent (agents are
///   interactive — planning fails BEFORE any start executes);
/// - free + unknown kind → hard error naming the kind.
///
/// The TARGET INSTANCE is mode-aware (ADR 0030 §4.1 T2, P2.1):
///
/// - **Shared** (or **Scoped** with a SINGLETON dependent —
///   `dependent_instance_id == None`): the dep's singleton slot. Scoped to
///   a singleton dependent IS the shared singleton — there is no parallel
///   dependent id to scope to.
/// - **Scoped** with `Some(id)`: the dep instance `<dep-slot>@<name>-<id>`
///   (e.g. dependent `prime` on instance `1` → dep `litellm` target
///   `personal-litellm@prime-1`). The composed id is validated through
///   [`validate_instance_id`]; an overlong/invalid composition is a clean
///   plan-time error. A registry record with EXACTLY that instance name (in
///   the dependent's namespace) yields Satisfied for the scoped instance.
/// - **Fresh**: ALWAYS a StartService marked `fresh` (never Satisfied); the
///   concrete slug is allocated by the EXECUTOR
///   ([`auto_start_dependencies`]) — the planner has no state_dir.
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
    dependent_instance_id: Option<&str>,
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
        // Compute the ports + conflict BEFORE the occupancy check (both
        // borrow config; `dep` is moved into the action below).
        let ports = declared_host_ports(config, &dep);
        let conflict = dep_conflict(config, name, &dep);
        // ADR 0030 T2 (P2.1): depends_on.<dep>.instance SELECTS the target
        // instance. Scoped composes the dep instance id from the DEPENDENT's
        // name + parallel id; fresh targets a not-yet-allocated slug (the
        // executor allocates); shared is the singleton slot.
        let mode = dep_instance_mode(config, name, &dep);
        let target: Option<String> = match mode {
            DepInstanceMode::Scoped => match dependent_instance_id {
                Some(id) => {
                    let dep_id = format!("{name}-{id}");
                    validate_instance_id(&dep_id).map_err(|e| {
                        anyhow::anyhow!(
                            "dependency '{dep}' of '{name}' is scoped to dependent instance \
                             '{id}', but the composed dep instance id '{dep_id}' is invalid: {e}"
                        )
                    })?;
                    Some(instance_name(&slot, Some(&dep_id)))
                }
                // Scoped to a SINGLETON dependent IS the shared singleton:
                // the dependent has no parallel id to scope to.
                None => None,
            },
            DepInstanceMode::Shared | DepInstanceMode::Fresh => None,
        };
        let occupied = match (&target, mode) {
            // Scoped: a record with EXACTLY the composed instance name in the
            // dependent's namespace satisfies the dep.
            (Some(t), _) => records.iter().any(|r| &r.instance == t),
            // Fresh is NEVER satisfied: every start allocates a new instance.
            (None, DepInstanceMode::Fresh) => false,
            (None, _) => singleton_record(&records, &slot).is_some(),
        };
        if occupied {
            actions.push(DepStartAction::Satisfied {
                dep,
                slot,
                instance: target,
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
                actions.push(DepStartAction::StartService {
                    dep,
                    slot,
                    instance: target,
                    fresh: mode == DepInstanceMode::Fresh,
                    ports,
                    conflict,
                });
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
/// started/satisfied by topo order), started DETACHED on its target
/// instance, then awaited on its published host ports within
/// [`DEFAULT_WAIT`].
///
/// ADR 0030 P2.1: `dependent_instance_id` is the dependent's OWN resolved
/// parallel instance id (main.rs resolves it BEFORE this call). A scoped
/// dep targets `<dep-slot>@<dependent>-<id>`; a fresh dep gets a concrete
/// auto-allocated slug here (the planner is pure). Every concrete FRESH
/// `(dep, slug)` selection this call started is RETURNED so the caller can
/// inject `<dep>@<slug>` as a `--use` override — the dependent's plan then
/// resolves exports from the fresh record, and the forwarded `--use` makes
/// the DETACHED CHILD skip the fresh dep in its own planner (no second
/// allocation). Scoped needs no injection: the child re-derives
/// `<dependent>-<id>` from its forwarded `--instance`.
pub async fn auto_start_dependencies(
    workload_name: &str,
    verb: &str,
    no_deps: bool,
    use_overrides: &[(String, String)],
    dependent_instance_id: Option<&str>,
) -> Result<Vec<(String, String)>> {
    if no_deps || !matches!(verb, "up" | "exec") {
        return Ok(Vec::new());
    }
    let config = crate::config::load_config()?;
    let Some(workload) = config.workloads.get(workload_name) else {
        // Unknown workload: the ConfigWorkload constructor's own error
        // fires downstream; there is nothing to plan.
        return Ok(Vec::new());
    };
    if !matches!(
        (workload.kind.as_str(), verb),
        ("service", "up") | ("agent", "exec")
    ) {
        return Ok(Vec::new());
    }
    if workload.depends_on.is_empty() {
        return Ok(Vec::new());
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
        dependent_instance_id,
    )?;
    let mut fresh_selections: Vec<(String, String)> = Vec::new();
    for action in actions {
        let mut action = action;
        // ADR 0030 P2.1 FRESH: allocate the concrete slug HERE (the planner
        // is pure — it has no state_dir), fill the target instance name, and
        // remember the selection so the caller injects `<dep>@<slug>` as a
        // --use override (exports resolution + detached-child dedup).
        if let DepStartAction::StartService {
            fresh: true,
            dep,
            slot,
            instance,
            ..
        } = &mut action
        {
            let slug = auto_allocate_slug(&state_dir, slot)?;
            *instance = Some(instance_name(slot, Some(&slug)));
            fresh_selections.push((dep.clone(), slug));
        }
        // ADR 0030 Phase 0: reconcile the planner's record view with the msb
        // runtime + sandbox dir + host-port liveness BEFORE starting, then
        // route the disposition through the dep's conflict CHAIN (default
        // ["reuse", "start", "replace"]). The record-as-authoritative planner
        // can miss a running sandbox when the registry record is
        // absent/stale in the active state dir. The reconcile target is the
        // TARGET INSTANCE NAME (scoped/fresh parallel instance or singleton
        // slot), never the bare slot. ADR 0030 Phase 2 T1: the disposition is
        // namespace-aware — a record in a FOREIGN namespace is never
        // reused/adopted (decide_dep_disposition refuses collision-style).
        let facts = crate::microsandbox::runtime::reconcile::gather_facts(
            &state_dir,
            target_of_action(&action),
            declared_ports_of(&action),
        )
        .await?;
        let mut chain = action.conflict().0.clone();
        let disposition = loop {
            match decide_dep_disposition(&chain, &action, workload_name, &facts, &namespace)? {
                DepDisposition::StartExisting { dep, slot } => {
                    // The detached child re-reconciles and executes
                    // handle.start(); if that fails the child errors and we
                    // fall through here — advance the chain past `start`.
                    match start_service_detached_instance(
                        &dep,
                        instance_id_of(&slot),
                        dep_port_auto(&config, &dep, &slot),
                        EnsurePreflight::Run { force: false },
                    )
                    .await
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
                // A1/P3: adopting a record registered under a different
                // context than the active one is allowed but surfaced
                // (warn-and-proceed).
                crate::microsandbox::runtime::reconcile::warn_on_context_drift(
                    &slot,
                    facts.record.as_ref().and_then(|r| r.context.as_deref()),
                    crate::config::active_context_name().as_deref(),
                );
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
                start_service_detached_instance(
                    &dep,
                    instance_id_of(&slot),
                    dep_port_auto(&config, &dep, &slot),
                    EnsurePreflight::Run { force: false },
                )
                .await?;
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
                start_service_detached_instance(
                    &dep,
                    instance_id_of(&slot),
                    dep_port_auto(&config, &dep, &slot),
                    EnsurePreflight::Run { force: false },
                )
                .await?;
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
    Ok(fresh_selections)
}

/// PORTS precedence for a dep start (ADR 0030 P2.1, LOCKED): a scoped/fresh
/// dep instance (a PARALLEL target — the instance name carries `@`) starts
/// with `port_auto = true` (auto-allocate, host=0 machinery) UNLESS the dep
/// declares its own `[workloads.<dep>.instance] port = ...` policy — then
/// `port_auto = false` and `apply_instance_port_policy` in `build_sandbox`
/// applies the dep's declared policy (the operator's explicit override).
/// Auto-allocation by default keeps the dep's DECLARED fixed ports free for
/// its singleton/shared use. A singleton target (no `@`) never port-autos
/// here — the shared model keeps declared ports as-is.
fn dep_port_auto(config: &ConfigFile, dep: &str, target_instance: &str) -> bool {
    if instance_id_of(target_instance).is_none() {
        return false;
    }
    config
        .workloads
        .get(dep)
        .and_then(|w| w.instance.port.as_ref())
        .is_none()
}

/// Bare `workestrate workload up` (EXECUTOR — KVM-dependent at runtime):
/// topo-ordered start of ALL service-kind workloads in the active context,
/// detached, singleton slots. Agent-kind workloads are printed as SKIPPED;
/// an occupied slot is already running (skip, NOT an error).
///
/// ADR 0030 P0.2 (Q6): after the pure [`plan_bare_up`], the parent runs an
/// explicit RECONCILE pass over both lists via [`decide_bare_up_disposition`]
/// (facts from the registry record + msb status + sandbox dir + host-port
/// liveness, routed through the workload's `instance.on_conflict` chain or
/// the built-in default):
///
/// - `AlreadyRunning` skips are re-probed: Reuse keeps the skip (message
///   says "— reusing"); StartExisting/Replace move the workload into the
///   start set (Replace downs the slot first; a down ERROR aborts the batch
///   naming the workload); Fail keeps the skip with a chain-specific
///   message (NOT a batch-fatal error). A record-present ZOMBIE therefore
///   converges instead of being skipped forever.
/// - Planned starts are re-probed too: Reuse (msb Running + healthy but NO
///   registry record — the registry was lost) short-circuits to a
///   skip-with-reuse message without spawning (mirrors the
///   `up_service_with_spec` parent short-circuit); Start/StartExisting/
///   Replace proceed to spawn (the detached child re-reconciles and
///   executes).
///
/// Output: text by default. With `json`, a single summary object
/// (`{"started": [...], "already_running": [...], "skipped_agents": [...]}`)
/// is printed INSTEAD of the per-workload text lines. Back-compat (P0.2):
/// reused-via-reconcile names land in `already_running`; replaced and
/// started-stopped names land in `started`; chain-fail skips appear in text
/// output only.
///
/// Spec 21 phase E: `reload_images` is the batch-scoped `--reload-images`
/// force (USER DECISION D3). The ensure-images pre-flight runs for EVERY
/// start in the FINAL (post-reconcile) start set BEFORE ANY spawn — a
/// nix-layered workload whose declaring repo lacks a flake.nix is skipped
/// with a note (spec §7 batch row) and a hard ensure failure aborts the
/// batch before anything starts.
pub async fn cmd_workload_up_all(json: bool, reload_images: bool) -> Result<()> {
    let config = crate::config::load_config()?;
    let context = crate::config::active_context_name();
    let state_dir = crate::config::resolve_state_dir();
    let records = list_records(&state_dir)?;
    let (starts, skips) = plan_bare_up(&config, &records, context.as_deref())?;

    // ADR 0030 P0.2: parent-side reconcile pass (see the fn docs). All
    // dispositions are computed BEFORE the ensure pass and ANY spawn.
    let mut final_starts: Vec<PlannedStart> = Vec::new();
    let mut reused: Vec<String> = Vec::new();
    let mut skipped_agents: Vec<String> = Vec::new();
    for skip in skips {
        match skip {
            BareSkip::Agent { name } => {
                if !json {
                    println!(
                        "{name}: skipped (agents are interactive; use `workestrate workload exec {name}`)"
                    );
                }
                skipped_agents.push(name);
            }
            BareSkip::AlreadyRunning { name, slot } => {
                let ports = declared_host_ports(&config, &name);
                let chain = workload_conflict_chain(&config, &name);
                let facts = crate::microsandbox::runtime::reconcile::gather_facts(
                    &state_dir, &slot, &ports,
                )
                .await?;
                match decide_bare_up_disposition(&chain, &facts, &slot) {
                    Ok(BareUpDisposition::Reuse) => {
                        // A1/P3: adopting a record registered under a
                        // different context than the active one is allowed
                        // but surfaced (warn-and-proceed).
                        crate::microsandbox::runtime::reconcile::warn_on_context_drift(
                            &slot,
                            facts.record.as_ref().and_then(|r| r.context.as_deref()),
                            crate::config::active_context_name().as_deref(),
                        );
                        if !json {
                            println!("{name}: already running (slot '{slot}') — reusing");
                        }
                        reused.push(name);
                    }
                    Ok(BareUpDisposition::StartExisting) => {
                        final_starts.push(PlannedStart {
                            name,
                            slot,
                            ports,
                            note: Some("[started stopped sandbox]"),
                        });
                    }
                    Ok(BareUpDisposition::Replace) => {
                        // Zombie/stale record: down the slot first (a down
                        // ERROR is batch-fatal, naming the workload), then
                        // converge via a fresh start.
                        let down = down_instance(&state_dir, &slot).await;
                        if matches!(down.status, DownStatus::Error) {
                            anyhow::bail!(
                                "failed to replace workload '{name}' (slot '{slot}'): {}",
                                down.message.unwrap_or_default()
                            );
                        }
                        final_starts.push(PlannedStart {
                            name,
                            slot,
                            ports,
                            note: Some("[replaced]"),
                        });
                    }
                    Ok(BareUpDisposition::Fail) => {
                        // Not batch-fatal: the workload stays down, the rest
                        // of the batch proceeds.
                        if !json {
                            println!(
                                "{name}: slot '{slot}' occupied and on_conflict chain ends in fail — skipping"
                            );
                        }
                    }
                    Ok(BareUpDisposition::Start) => {
                        // Record-present but the reconcile sees a FREE slot
                        // (unreachable under decide_chain's free-slot
                        // shortcut, which requires no record — kept for
                        // exhaustiveness): plain start.
                        final_starts.push(PlannedStart {
                            name,
                            slot,
                            ports,
                            note: None,
                        });
                    }
                    Err(e) => {
                        // Chain exhausted (e.g. ["reuse"] on a zombie): skip
                        // with the reason, do not abort the batch.
                        if !json {
                            println!("{name}: skipping — {e}");
                        }
                    }
                }
            }
        }
    }
    for s in starts {
        let chain = workload_conflict_chain(&config, &s.name);
        let facts =
            crate::microsandbox::runtime::reconcile::gather_facts(&state_dir, &s.slot, &s.ports)
                .await?;
        match decide_bare_up_disposition(&chain, &facts, &s.slot) {
            Ok(BareUpDisposition::Reuse) => {
                // msb Running + healthy but NO registry record (the registry
                // was lost): short-circuit reuse parent-side — the child
                // would reuse anyway, but spawning it would misreport within
                // the FS-8 grace window.
                // A1/P3: adopting a record registered under a different
                // context than the active one is allowed but surfaced
                // (warn-and-proceed); with no record this is silent.
                crate::microsandbox::runtime::reconcile::warn_on_context_drift(
                    &s.slot,
                    facts.record.as_ref().and_then(|r| r.context.as_deref()),
                    crate::config::active_context_name().as_deref(),
                );
                if !json {
                    println!("{}: already running (slot '{}') — reusing", s.name, s.slot);
                }
                reused.push(s.name);
            }
            Ok(BareUpDisposition::Fail) => {
                if !json {
                    println!(
                        "{}: slot '{}' occupied and on_conflict chain ends in fail — skipping",
                        s.name, s.slot
                    );
                }
            }
            Ok(
                BareUpDisposition::Start
                | BareUpDisposition::StartExisting
                | BareUpDisposition::Replace,
            ) => {
                // Spawn: the detached child re-reconciles and executes
                // (start / handle.start() / teardown + create).
                final_starts.push(PlannedStart {
                    name: s.name,
                    slot: s.slot,
                    ports: s.ports,
                    note: None,
                });
            }
            Err(e) => {
                if !json {
                    println!("{}: skipping — {e}", s.name);
                }
            }
        }
    }

    // Spec 21 §2.1/§5.2: the batch ensure pass, ALL starts BEFORE ANY
    // spawn, with the same force flag for every eligible service workload
    // in the batch (USER DECISION D3). P0.2: over the FINAL start set
    // (post-reconcile moves).
    let start_names: Vec<String> = final_starts.iter().map(|s| s.name.clone()).collect();
    crate::images::ensure::ensure_images_for_workloads(&start_names, reload_images).await?;

    let mut started: Vec<String> = Vec::with_capacity(final_starts.len());
    for s in &final_starts {
        start_service_detached(&s.name, EnsurePreflight::AlreadyDone).await?;
        if !json {
            match s.note {
                Some(note) => println!("started '{}' (slot '{}') {}", s.name, s.slot, note),
                None => println!("started '{}' (slot '{}')", s.name, s.slot),
            }
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
        let body = serde_json::json!({
            "started": started,
            "already_running": reused,
            "skipped_agents": skipped_agents,
        });
        println!("{}", serde_json::to_string_pretty(&body)?);
    }
    Ok(())
}

/// One workload in the FINAL bare-up start set (post-reconcile), with an
/// optional message note marking a reconcile-driven start (`[replaced]`,
/// `[started stopped sandbox]`).
#[derive(Debug)]
struct PlannedStart {
    name: String,
    slot: String,
    ports: Vec<u16>,
    note: Option<&'static str>,
}

/// The parent-side bare-up reconcile disposition (ADR 0030 P0.2): the
/// conflict-chain step mapped to the batch action. PURE — unit-testable
/// across the U2 behavior matrix without KVM.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BareUpDisposition {
    /// Adopt the running healthy/booting instance: keep/convert to an
    /// already-running skip with a "— reusing" message.
    Reuse,
    /// Plain fresh start (slot free) — spawn the detached child.
    Start,
    /// Start the stopped/crashed sandbox — move into the start set with a
    /// `[started stopped sandbox]` note.
    StartExisting,
    /// Down the slot, then start fresh — move into the start set with a
    /// `[replaced]` note.
    Replace,
    /// The chain ends in fail — keep the skip with a chain-specific
    /// message (NOT batch-fatal).
    Fail,
}

/// Map (chain, facts) to the bare-up disposition — the pure decision seam
/// of the P0.2 reconcile pass (a thin [`ChainStep`] → [`BareUpDisposition`]
/// mapping over
/// [`crate::microsandbox::runtime::reconcile::decide_chain`]; chain
/// exhaustion propagates as Err).
pub fn decide_bare_up_disposition(
    chain: &[ConflictStep],
    facts: &ReconcileFacts,
    instance: &str,
) -> Result<BareUpDisposition> {
    Ok(
        match crate::microsandbox::runtime::reconcile::decide_chain(chain, facts, instance)? {
            ChainStep::Start => BareUpDisposition::Start,
            ChainStep::Reuse => BareUpDisposition::Reuse,
            ChainStep::StartExisting => BareUpDisposition::StartExisting,
            ChainStep::Replace => BareUpDisposition::Replace,
            ChainStep::Fail => BareUpDisposition::Fail,
        },
    )
}

/// The workload's conflict chain for the bare-up reconcile pass (ADR 0030
/// U11 precedence, ConfigFile-level): `workloads.<name>.instance.on_conflict`
/// when declared, else the built-in default ["reuse","start","replace"].
/// Mirrors `Workload::instance_conflict_chain` for the batch path, which
/// plans from the merged config directly (no `ConfigWorkload` is
/// constructed per batch member).
fn workload_conflict_chain(config: &ConfigFile, name: &str) -> Vec<ConflictStep> {
    config
        .workloads
        .get(name)
        .and_then(|w| w.instance.on_conflict.clone())
        .map(|c| c.0)
        .unwrap_or_else(crate::microsandbox::runtime::reconcile::default_chain)
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
/// started/satisfied by topo order). Thin wrapper over
/// [`start_service_detached_instance`] for the singleton model.
async fn start_service_detached(name: &str, preflight: EnsurePreflight) -> Result<()> {
    start_service_detached_instance(name, None, false, preflight).await
}

/// Start one service-kind workload DETACHED on the given parallel instance
/// (`instance_id` = the bare `@`-suffix id; `None` = the singleton slot).
/// The spawned child is the ensured party (spec 21 §2.2): its spec carries
/// `images_ready = true`, and `detach_args` appends `--images-ready` so the
/// child skips the pre-flight. `detach_args` likewise forwards `--instance
/// <id>` for a parallel target, so the child re-derives the SAME instance
/// name from its own context.
///
/// PORTS (ADR 0030 P2.1, LOCKED precedence): callers pass `port_auto =
/// true` for scoped/fresh dep instances (auto-allocate via the host=0
/// machinery, keeping the dep's DECLARED fixed ports free for its
/// singleton/shared use) UNLESS the dep declares its own
/// `[workloads.<dep>.instance] port = ...` policy — then `port_auto =
/// false` and `apply_instance_port_policy` in `build_sandbox` applies the
/// declared policy (the operator's explicit override).
async fn start_service_detached_instance(
    name: &str,
    instance_id: Option<&str>,
    port_auto: bool,
    preflight: EnsurePreflight,
) -> Result<()> {
    if let EnsurePreflight::Run { force } = preflight {
        crate::images::ensure::ensure_images_for_workload(name, force).await?;
    }
    let workload =
        crate::microsandbox::workload::ConfigWorkload::new_with_use_overrides(name, &[])?;
    let mut spec = crate::commands::lifecycle::build_instance_spec(
        name,
        false,
        instance_id,
        None,
        port_auto,
        &[],
        false,
        // Dep auto-start children never reseed: --reseed is a named-up/exec
        // affordance scoped to the workload the operator invoked.
        false,
    )?;
    spec.images_ready = true;
    crate::microsandbox::runtime::up_service_with_spec(&workload, &spec, false).await
}

/// The TARGET INSTANCE NAME of a planned action (both variants carry it):
/// the full instance name when set (scoped/fresh parallel dep), else the
/// singleton slot. Reconcile facts, messages, teardown, and readiness waits
/// all address the target — a parallel record `<slot>@<id>` is found by its
/// EXACT instance name, never by the bare slot.
fn target_of_action(action: &DepStartAction) -> &str {
    match action {
        DepStartAction::StartService { slot, instance, .. }
        | DepStartAction::Satisfied { slot, instance, .. } => instance.as_deref().unwrap_or(slot),
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
/// and the detached child's log file. `instance` is the TARGET INSTANCE
/// NAME (singleton slot or `<slot>@<id>`).
fn wait_until_ready(
    state_dir: &Path,
    subject: &str,
    instance: &str,
    declared_ports: &[u16],
) -> Result<()> {
    let deadline = Instant::now() + DEFAULT_WAIT;
    let targets = readiness_targets(state_dir, instance, declared_ports);
    for (ip, port) in targets {
        let remaining = deadline.saturating_duration_since(Instant::now());
        wait_for_port(ip, port, remaining).map_err(|_| {
            anyhow::anyhow!(
                "{subject} did not become ready on {ip}:{port} within {}s; see log: ~/.microsandbox/sandboxes/{instance}/workestrate.log",
                DEFAULT_WAIT.as_secs()
            )
        })?;
    }
    Ok(())
}

/// Learn the published host addresses for a freshly-started workload: poll
/// the registry for the record whose instance name matches `instance`
/// EXACTLY (ADR 0030 P2.1: a parallel record `<slot>@<id>` must be found —
/// a singleton-only matcher would never see a scoped/fresh dep instance and
/// would fall through to the declared-ports fallback, which auto-allocated
/// ports make WRONG), falling back to the DECLARED host ports on the shared
/// 127.0.0.1 bind when the record is not yet visible within
/// [`RECORD_POLL_BUDGET`]. An empty result means no ports to wait on (the
/// caller skips the wait).
fn readiness_targets(
    state_dir: &Path,
    instance: &str,
    declared_ports: &[u16],
) -> Vec<(IpAddr, u16)> {
    let poll_deadline = Instant::now() + RECORD_POLL_BUDGET;
    loop {
        if let Ok(records) = list_records(state_dir) {
            if let Some(rec) = records.iter().find(|r| r.instance == instance) {
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
            None,
            // A2 (ADR 0032 §Image tags): no running tag known at this site.
            None,
            None,
            None,
        )
    }

    // (i) Chain a→b→c, all slots free: both deps start, deepest first, with
    // their declared ports. Omitted on_conflict resolves to the ADR 0030
    // default chain at plan time.
    #[test]
    fn plan_dep_starts_chain_all_absent_starts_in_topo_order() -> Result<()> {
        let config = chain_config();
        let actions = plan_dep_starts(&config, "a", &[], Some("personal"), &[], "default", None)?;
        assert_eq!(
            actions,
            vec![
                DepStartAction::StartService {
                    dep: "c".to_string(),
                    slot: "personal-c".to_string(),
                    instance: None,
                    fresh: false,
                    ports: vec![4002],
                    conflict: DepConflict::default_chain(),
                },
                DepStartAction::StartService {
                    dep: "b".to_string(),
                    slot: "personal-b".to_string(),
                    instance: None,
                    fresh: false,
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

        let actions = plan_dep_starts(
            &config,
            "a",
            &records,
            Some("personal"),
            &[],
            "default",
            None,
        )?;
        assert_eq!(
            actions,
            vec![
                DepStartAction::StartService {
                    dep: "c".to_string(),
                    slot: "personal-c".to_string(),
                    instance: None,
                    fresh: false,
                    ports: vec![4002],
                    conflict: DepConflict::default_chain(),
                },
                DepStartAction::Satisfied {
                    dep: "b".to_string(),
                    slot: "personal-b".to_string(),
                    instance: None,
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
        let err = plan_dep_starts(&config, "top", &[], Some("personal"), &[], "default", None)
            .unwrap_err();
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
        let actions = plan_dep_starts(
            &config,
            "a",
            &[],
            Some("personal"),
            &overrides,
            "default",
            None,
        )?;
        assert_eq!(
            actions,
            vec![DepStartAction::StartService {
                dep: "c".to_string(),
                slot: "personal-c".to_string(),
                instance: None,
                fresh: false,
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
        let actions = plan_dep_starts(&config, "a", &[], Some("personal"), &[], "default", None)?;
        assert_eq!(
            actions,
            vec![DepStartAction::StartService {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                instance: None,
                fresh: false,
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
        let actions = plan_dep_starts(&config, "a", &[], Some("personal"), &[], "default", None)?;
        assert_eq!(
            actions,
            vec![DepStartAction::StartService {
                dep: "litellm".to_string(),
                slot: "personal-litellm".to_string(),
                instance: None,
                fresh: false,
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
        let actions = plan_dep_starts(&config, "a", &[], Some("personal"), &[], "default", None)?;
        assert_eq!(
            actions,
            vec![DepStartAction::StartService {
                dep: "litellm".to_string(),
                slot: "personal-litellm".to_string(),
                instance: None,
                fresh: false,
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
        auto_start_dependencies("pi", "exec", true, &[], None).await?;
        auto_start_dependencies("litellm", "up", true, &[], None).await?;
        for verb in ["plan", "down", "logs"] {
            auto_start_dependencies("pi", verb, false, &[], None).await?;
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
        auto_start_dependencies("ghost", "up", false, &[], None).await?;
        // Kind/verb mismatch per the CONFIG kind (pi is an agent): the
        // workload_route kind error fires downstream.
        auto_start_dependencies("pi", "up", false, &[], None).await?;
        // The fixture config declares no depends_on: nothing to plan.
        auto_start_dependencies("pi", "exec", false, &[], None).await?;
        auto_start_dependencies("litellm", "up", false, &[], None).await?;
        Ok(())
    }

    // ---- ADR 0026 addendum 2026-08-16 / ADR 0030 addendum 2:
    // decide_dep_disposition (chain-iterating over ReconcileFacts) ----

    fn start_action(dep: &str, conflict: DepConflict) -> DepStartAction {
        DepStartAction::StartService {
            dep: dep.to_string(),
            slot: format!("personal-{dep}"),
            instance: None,
            fresh: false,
            ports: vec![4000],
            conflict,
        }
    }

    fn satisfied_action(dep: &str, conflict: DepConflict) -> DepStartAction {
        DepStartAction::Satisfied {
            dep: dep.to_string(),
            slot: format!("personal-{dep}"),
            instance: None,
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
            source_gone: false,
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
                &free(),
                "default",
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
                &running(Some(true), false),
                "default"
            )
            .unwrap(),
            DepDisposition::Reuse {
                dep: "b".to_string(),
                slot: "personal-b".to_string()
            }
        );
    }

    /// ADR 0030 Phase 2 T1: a record in a FOREIGN namespace never satisfies
    /// the dependent — the Reuse/StartExisting dispositions must NOT adopt it
    /// (the namespace-blind reuse previously printed "already running —
    /// reusing" for a record the dependent's namespace-filtered resolution
    /// then refused). The refusal names the dep, the dependent, the required
    /// namespace, and the namespace holding the record. A same-namespace
    /// record still reuses.
    #[test]
    fn dep_reuse_refuses_foreign_namespace_record() {
        // The record() fixture registers "personal-b" in the "default"
        // namespace; the dependent requires "personal".
        let err = decide_dep_disposition(
            &DepConflict::default_chain().0,
            &start_action("b", DepConflict::reuse()),
            "a",
            &running(Some(true), false),
            "personal",
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("'b'"), "refusal must name the dep: {msg}");
        assert!(
            msg.contains("'a'"),
            "refusal must name the dependent: {msg}"
        );
        assert!(
            msg.contains("namespace 'personal'"),
            "refusal must name the required namespace: {msg}"
        );
        assert!(
            msg.contains("namespace 'default'"),
            "refusal must name the namespace holding the record: {msg}"
        );

        // StartExisting is an adoption too: a stopped sandbox whose record is
        // foreign must not be started under the dependent.
        let stopped = facts(Some(record()), Some(SandboxStatus::Stopped), None, false);
        assert!(
            decide_dep_disposition(
                &DepConflict::default_chain().0,
                &start_action("b", DepConflict::reuse()),
                "a",
                &stopped,
                "personal",
            )
            .is_err(),
            "StartExisting on a foreign-namespace record must refuse"
        );

        // Same-namespace record: Reuse is unchanged.
        assert_eq!(
            decide_dep_disposition(
                &DepConflict::default_chain().0,
                &start_action("b", DepConflict::reuse()),
                "a",
                &running(Some(true), false),
                "default",
            )
            .unwrap(),
            DepDisposition::Reuse {
                dep: "b".to_string(),
                slot: "personal-b".to_string()
            },
            "a record in the dependent's namespace still reuses"
        );

        // A zombie (running + dead port + old record) with a foreign record
        // still converges through the chain's replace element — only
        // adoption (Reuse/StartExisting) is namespace-gated.
        assert_eq!(
            decide_dep_disposition(
                &DepConflict::default_chain().0,
                &start_action("b", DepConflict::reuse()),
                "a",
                &running(Some(false), false),
                "personal",
            )
            .unwrap(),
            DepDisposition::Replace {
                dep: "b".to_string(),
                slot: "personal-b".to_string(),
                ports: vec![4000]
            },
            "a zombie foreign record still converges via replace"
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
                &running(Some(false), false),
                "default",
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
                &running(Some(false), true),
                "default",
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
                &running(None, false),
                "default",
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
                &running(Some(true), false),
                "default",
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
                &facts(Some(record()), None, None, false),
                "default",
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
                &free(),
                "default",
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
                &running(None, false),
                "default",
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
                &free(),
                "default",
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
                &running(Some(true), false),
                "default",
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
                &running(Some(false), false),
                "default",
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
                &facts(Some(record()), None, None, false),
                "default",
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
                &running(Some(true), false),
                "default",
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
                &running(None, false),
                "default",
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
            "default",
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
                &running(Some(true), false),
                "default",
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
                &stopped,
                "default",
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
                &crashed,
                "default",
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
                &stopped,
                "default",
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
                &stopped,
                "default",
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
                &running(Some(false), false),
                "default",
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
                &facts(Some(record()), None, None, false),
                "default",
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
                &running(Some(false), true),
                "default",
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
                &running(Some(true), false),
                "default",
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
                &facts(Some(record()), None, None, false),
                "default",
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
                &free(),
                "default",
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
                &running(None, false),
                "default",
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
                &free(),
                "default",
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
                &running(Some(true), false),
                "default",
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
                &running(Some(false), false),
                "default",
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
                &stopped,
                "default",
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
                &running(Some(false), false),
                "default",
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
            "default",
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

    // ---- ADR 0030 P2.1: scoped/fresh dep auto-start planning ----

    /// Scoped-mode fixture: dependent `prime` (agent) depends on `litellm`
    /// (service, 4000) with `instance = "scoped"`.
    fn scoped_config() -> ConfigFile {
        let toml = r#"
schema_version = 1

[workloads.prime]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.prime.depends_on.litellm]
env = "LITELLM_URL"
instance = "scoped"

[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[[workloads.litellm.ports]]
host = 4000
guest = 4000
"#;
        toml::from_str(toml).expect("scoped fixture must parse")
    }

    /// Register a PARALLEL lifecycle record `<context>-<workload>@<id>` in
    /// the "default" namespace.
    fn register_parallel(
        state_dir: &Path,
        workload: &str,
        id: &str,
        bind: IpAddr,
        host: u16,
        guest: u16,
    ) -> Result<()> {
        check_and_register_sandbox_lifecycle(
            state_dir,
            &format!("personal-{workload}@{id}"),
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
            None,
            // A2 (ADR 0032 §Image tags): no running tag known at this site.
            None,
            None,
            None,
        )
    }

    // P2.1: scoped + parallel dependent (`prime` on instance `1`) → the dep
    // targets the COMPOSED instance `personal-litellm@prime-1`.
    #[test]
    fn plan_dep_starts_scoped_with_dependent_id_targets_composed_instance() -> Result<()> {
        let config = scoped_config();
        let actions = plan_dep_starts(
            &config,
            "prime",
            &[],
            Some("personal"),
            &[],
            "default",
            Some("1"),
        )?;
        assert_eq!(
            actions,
            vec![DepStartAction::StartService {
                dep: "litellm".to_string(),
                slot: "personal-litellm".to_string(),
                instance: Some("personal-litellm@prime-1".to_string()),
                fresh: false,
                ports: vec![4000],
                conflict: DepConflict::default_chain(),
            }],
            "a scoped dep of prime@1 must target litellm@prime-1"
        );
        Ok(())
    }

    // P2.1: scoped + SINGLETON dependent (no parallel id) → the shared
    // singleton (scoped to a singleton dependent IS the shared singleton).
    #[test]
    fn plan_dep_starts_scoped_singleton_dependent_keeps_shared_singleton() -> Result<()> {
        let config = scoped_config();
        let actions = plan_dep_starts(
            &config,
            "prime",
            &[],
            Some("personal"),
            &[],
            "default",
            None,
        )?;
        assert_eq!(
            actions,
            vec![DepStartAction::StartService {
                dep: "litellm".to_string(),
                slot: "personal-litellm".to_string(),
                instance: None,
                fresh: false,
                ports: vec![4000],
                conflict: DepConflict::default_chain(),
            }],
            "a scoped dep of a singleton dependent keeps the shared singleton target"
        );
        Ok(())
    }

    // P2.1: a scoped record with EXACTLY the composed instance name (in the
    // dependent's namespace) → Satisfied for the scoped instance.
    #[test]
    fn plan_dep_starts_scoped_occupied_scoped_instance_is_satisfied() -> Result<()> {
        let state_dir = unique_state_dir("deps-scoped-occupied");
        register_parallel(
            &state_dir,
            "litellm",
            "prime-1",
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)),
            14000,
            4000,
        )?;
        let records = list_records(&state_dir)?;
        let config = scoped_config();

        let actions = plan_dep_starts(
            &config,
            "prime",
            &records,
            Some("personal"),
            &[],
            "default",
            Some("1"),
        )?;
        assert_eq!(
            actions,
            vec![DepStartAction::Satisfied {
                dep: "litellm".to_string(),
                slot: "personal-litellm".to_string(),
                instance: Some("personal-litellm@prime-1".to_string()),
                ports: vec![4000],
                conflict: DepConflict::default_chain(),
            }],
            "an existing litellm@prime-1 record must satisfy the scoped dep"
        );
        // A record for a DIFFERENT dependent (prime-2) does NOT satisfy it.
        let state_dir2 = unique_state_dir("deps-scoped-other");
        register_parallel(
            &state_dir2,
            "litellm",
            "prime-2",
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 3)),
            14001,
            4000,
        )?;
        let records2 = list_records(&state_dir2)?;
        let actions2 = plan_dep_starts(
            &config,
            "prime",
            &records2,
            Some("personal"),
            &[],
            "default",
            Some("1"),
        )?;
        assert!(
            matches!(actions2.as_slice(), [DepStartAction::StartService { .. }]),
            "litellm@prime-2 must not satisfy the prime@1-scoped dep: {actions2:?}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        let _ = std::fs::remove_dir_all(&state_dir2);
        Ok(())
    }

    // P2.1: EXPLICIT fresh plans a fresh start (marked `fresh`, no concrete
    // instance — the executor allocates) instead of the old P2.1-stopgap
    // error.
    #[test]
    fn plan_dep_starts_explicit_fresh_plans_fresh_start() -> Result<()> {
        let toml = r#"
schema_version = 1

[workloads.prime]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.prime.depends_on.litellm]
env = "LITELLM_URL"
instance = "fresh"

[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[[workloads.litellm.ports]]
host = 4000
guest = 4000
"#;
        let config: ConfigFile = toml::from_str(toml).expect("fixture must parse");
        let actions = plan_dep_starts(
            &config,
            "prime",
            &[],
            Some("personal"),
            &[],
            "default",
            Some("1"),
        )?;
        assert_eq!(
            actions,
            vec![DepStartAction::StartService {
                dep: "litellm".to_string(),
                slot: "personal-litellm".to_string(),
                instance: None,
                fresh: true,
                ports: vec![4000],
                conflict: DepConflict::default_chain(),
            }],
            "explicit fresh must plan a fresh-marked start, not error"
        );
        Ok(())
    }

    // P2.1: DERIVED fresh (the dep's own `strategy = "parallel"`, no
    // explicit `instance`) plans the same fresh-marked start — the old
    // warn-and-fall-back-to-shared stopgap is gone.
    #[test]
    fn plan_dep_starts_derived_fresh_plans_fresh_start() -> Result<()> {
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

[workloads.litellm.instance]
strategy = "parallel"
"#;
        let config: ConfigFile = toml::from_str(toml).expect("fixture must parse");
        let actions = plan_dep_starts(&config, "a", &[], Some("personal"), &[], "default", None)?;
        assert_eq!(
            actions,
            vec![DepStartAction::StartService {
                dep: "litellm".to_string(),
                slot: "personal-litellm".to_string(),
                instance: None,
                fresh: true,
                ports: vec![],
                conflict: DepConflict::default_chain(),
            }],
            "a parallel-strategy dep derives a fresh-marked start (no warn/fallback)"
        );
        Ok(())
    }

    // P2.1: a scoped composition whose dep instance id is overlong is a
    // clean plan-time error naming the composed id.
    #[test]
    fn plan_dep_starts_scoped_overlong_composed_id_is_a_clean_error() {
        let config = scoped_config();
        let long_id = "a".repeat(28); // "prime-" + 28 = 34 > 32
        let err = plan_dep_starts(
            &config,
            "prime",
            &[],
            Some("personal"),
            &[],
            "default",
            Some(&long_id),
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("composed dep instance id"),
            "the error must name the composed id: {msg}"
        );
        assert!(
            msg.contains(&format!("prime-{long_id}")),
            "the error must carry the offending id: {msg}"
        );
    }

    // P2.1: decide_dep_disposition on a scoped action carries the TARGET
    // INSTANCE NAME (not the bare slot) into the disposition.
    #[test]
    fn disposition_scoped_action_carries_target_instance() {
        let action = DepStartAction::StartService {
            dep: "litellm".to_string(),
            slot: "personal-litellm".to_string(),
            instance: Some("personal-litellm@prime-1".to_string()),
            fresh: false,
            ports: vec![4000],
            conflict: DepConflict::default_chain(),
        };
        assert_eq!(
            decide_dep_disposition(
                &DepConflict::default_chain().0,
                &action,
                "prime",
                &free(),
                "default",
            )
            .unwrap(),
            DepDisposition::Start {
                dep: "litellm".to_string(),
                slot: "personal-litellm@prime-1".to_string(),
                ports: vec![4000]
            },
            "the disposition must address the scoped instance, not the singleton slot"
        );
    }

    // P2.1: readiness_targets matches the EXACT instance name — a parallel
    // record `<slot>@<id>` is found (the old singleton-only matcher missed
    // it), and the singleton still matches.
    #[test]
    fn readiness_targets_matches_parallel_instance_exactly() -> Result<()> {
        let state_dir = unique_state_dir("deps-readiness");
        register_singleton(&state_dir, "litellm", 4000, 4000)?;
        register_parallel(
            &state_dir,
            "litellm",
            "prime-1",
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)),
            14000,
            4000,
        )?;
        // The parallel record's published pair wins for the scoped instance.
        let targets = readiness_targets(&state_dir, "personal-litellm@prime-1", &[4000]);
        assert_eq!(
            targets,
            vec![(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)), 14000)],
            "the scoped instance must resolve to its own record's ports"
        );
        // The singleton still resolves to its own record.
        let targets = readiness_targets(&state_dir, "personal-litellm", &[4000]);
        assert_eq!(
            targets,
            vec![(IpAddr::V4(Ipv4Addr::LOCALHOST), 4000)],
            "the singleton record still matches exactly"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- ADR 0030 P0.2: bare-up parent-side reconcile (decision seam) ----

    /// The U2 matrix on the bare-up disposition seam: running+healthy →
    /// Reuse; zombie → Replace; stopped → StartExisting; stale record →
    /// Replace; free → Start; a fail-chain on an occupied slot → Fail.
    #[test]
    fn bare_up_disposition_u2_matrix() {
        let default = DepConflict::default_chain().0;
        // running + healthy → reuse.
        assert_eq!(
            decide_bare_up_disposition(&default, &running(Some(true), false), "personal-b")
                .unwrap(),
            BareUpDisposition::Reuse
        );
        // zombie (running + dead port + old record) → replace.
        assert_eq!(
            decide_bare_up_disposition(&default, &running(Some(false), false), "personal-b")
                .unwrap(),
            BareUpDisposition::Replace
        );
        // stopped → start-existing.
        let stopped = facts(Some(record()), Some(SandboxStatus::Stopped), None, false);
        assert_eq!(
            decide_bare_up_disposition(&default, &stopped, "personal-b").unwrap(),
            BareUpDisposition::StartExisting
        );
        // stale record (msb gone) → replace.
        assert_eq!(
            decide_bare_up_disposition(
                &default,
                &facts(Some(record()), None, None, false),
                "personal-b"
            )
            .unwrap(),
            BareUpDisposition::Replace
        );
        // genuinely free → start.
        assert_eq!(
            decide_bare_up_disposition(&default, &free(), "personal-b").unwrap(),
            BareUpDisposition::Start
        );
        // fail chain on an occupied slot → fail.
        assert_eq!(
            decide_bare_up_disposition(&[ConflictStep::Fail], &running(None, false), "personal-b")
                .unwrap(),
            BareUpDisposition::Fail
        );
    }

    /// The batch chain helper mirrors `Workload::instance_conflict_chain`:
    /// the declared `instance.on_conflict` wins, else the default chain.
    #[test]
    fn workload_conflict_chain_declared_beats_default() {
        let toml = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.svc.instance]
on_conflict = "fail"

[workloads.plain]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []
"#;
        let config: ConfigFile = toml::from_str(toml).expect("fixture must parse");
        assert_eq!(
            workload_conflict_chain(&config, "svc"),
            vec![ConflictStep::Fail],
            "the declared chain must win"
        );
        assert_eq!(
            workload_conflict_chain(&config, "plain"),
            crate::microsandbox::runtime::reconcile::default_chain(),
            "an undeclared workload gets the built-in default chain"
        );
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
        // registered_repo_checkouts reads the LIVE registry (which is
        // read-only in this container), so we cannot register a repo here.
        // Instead, assert the corrected P3 semantics directly via the pure
        // repo_key_for_optional: a REGISTERED checkout resolves to its repo
        // name; an UNREGISTERED dir has no repo identity -> "default".
        let registered = vec![("personal".to_string(), checkout.clone())];
        assert_eq!(
            crate::images::repo_key::repo_key_for_optional(
                &checkout.join("workestrate").join("workloads"),
                &registered,
            ),
            Some("personal".to_string()),
            "a registered checkout must resolve to its repo name"
        );
        assert_eq!(
            crate::images::repo_key::repo_key_for_optional(&tmp.join("not-a-repo"), &registered,),
            None,
            "an unregistered dir has no repo identity"
        );
        // namespace_for reads the LIVE registry (read-only in this container),
        // so the registered-name end-to-end path is covered by the pure
        // repo_key_for_optional assertions above; here we assert the
        // "default" fallbacks that do not depend on the live registry.
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
        // An UNREGISTERED declaring dir (not a config-repo checkout) has no
        // repo identity → "default" (the corrected P3 semantics: a non-repo /
        // synthetic / test layer is not a namespace).
        let mut unreg = crate::merge::Provenance::new();
        unreg.insert("workloads.pi.kind".to_string(), "synthetic".to_string());
        let mut unreg_dirs = std::collections::HashMap::new();
        unreg_dirs.insert("synthetic".to_string(), tmp.join("not-a-repo"));
        assert_eq!(
            namespace_for(Some(&unreg), &unreg_dirs, "pi"),
            crate::microsandbox::port_registry::default_namespace(),
            "an unregistered declaring dir must fall back to the default namespace"
        );
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    /// ADR 0030 Phase 2 T1 regression pin: a STANDALONE construction under a
    /// registered directory-mode config repo resolves the record namespace to
    /// the declaring repo key — including right after the provenance slot is
    /// drained. `take_provenance` is one-shot, but every construction re-loads
    /// the config first (workload/config.rs `load_config` → `set_provenance`),
    /// re-filling the slot before the take.
    #[test]
    fn standalone_construction_namespace_is_declaring_repo_key() -> Result<()> {
        use crate::config::test_support::{uniq_dir, EnvGuard, ENV_TEST_LOCK, HOME_ENV_KEYS};
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = uniq_dir("ns-standalone");
        std::fs::create_dir_all(&home)?;
        std::env::set_var("WORKESTRATE_HOME", &home);
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");
        // Directory-mode repo at <store>/config-repos/personal (store = the
        // home in the single-home layout).
        let repo = home.join("config-repos").join("personal");
        let workloads = repo.join("workestrate").join("workloads");
        std::fs::create_dir_all(&workloads)?;
        std::fs::write(
            repo.join("workestrate").join("default.toml"),
            "schema_version = 1\n",
        )?;
        std::fs::write(
            workloads.join("litellm.toml"),
            "[workloads.litellm]\nkind = \"service\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\n\n[workloads.litellm.network]\ndefault_deny = true\n",
        )?;
        let canonical = repo.canonicalize()?;
        crate::config::register_config("personal", &canonical.to_string_lossy(), None, None)?;

        let wl = crate::microsandbox::workload::ConfigWorkload::new("litellm")?;
        assert_eq!(
            crate::microsandbox::workload::Workload::namespace(&wl),
            "personal",
            "a standalone construction's record namespace is the declaring repo key"
        );

        // The drain pin: take_provenance empties the slot (one-shot); the
        // NEXT construction re-loads the config first and STILL resolves the
        // repo key (the slot is re-filled before every take).
        let _cfg = crate::config::load_config()?;
        assert!(
            crate::merge::take_provenance().is_some(),
            "load_config must re-fill the provenance slot"
        );
        let wl = crate::microsandbox::workload::ConfigWorkload::new("litellm")?;
        assert_eq!(
            crate::microsandbox::workload::Workload::namespace(&wl),
            "personal",
            "construction after a drained slot still resolves the declaring repo key"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }
}
