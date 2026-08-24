//! Discovery-lite (ADR 0026(d)): plan-time `depends_on` resolution.
//!
//! Declaring `[workloads.<name>.depends_on.<dep>]` in `workestrate.toml`
//! triggers UNCONDITIONAL plan-time resolution — no flag, no opt-in:
//!
//! 1. The dependency's SINGLETON slot record is looked up in the port
//!    registry (the record whose `instance` name contains no `@`). A
//!    `--use <dep>@<instance>` override (ADR 0026(d)) selects the record for
//!    that parallel instance instead — a PURE instance-selection override:
//!    declaration alone activates discovery; `--use` is never an on/off
//!    switch and never required for the default case.
//! 2. The resolved address is injected into the dependent's plan env as the
//!    spec's `env` var in GUEST-VISIBLE form:
//!    `host.microsandbox.internal:<published-host-port>` — guests reach the
//!    host through the gateway alias; `127.0.0.1` is host-only.
//! 3. An egress allow rule (`tcp:<port> -> host`) is DERIVED for the
//!    dependent, landing in `plan.network.egress_rules` so the same
//!    `network_plan_to_policy` path consumes it identically to declared
//!    egress. Derivation only ADDS — `default_deny` is never touched
//!    (monotonic; FS-16 entitlement untouched).
//!
//! P2 namespaced ports (ADR 0026(d) namespaced-ports rule): resolution emits
//! ONE request per injected var — the `env` request targets the PRIMARY port
//! (the record's UNNAMED published port if it has one, else its first
//! published port — the legacy [`record_host_port`] rule), and each
//! `exports` entry (port name -> env var) targets that NAMED port. Two
//! resolution refusals are hard errors even for optional deps: an
//! auto-allocated DECLARED port (`host = 0`) is never injected (an auto port
//! has no address until the dependency runs), and a named port MISSING on a
//! RUNNING record requires restarting the dependency so its named ports are
//! recorded.
//!
//! Refusal/fallback matrix (ADR 0026(d), spec 12 §3):
//!
//! | registry view                          | required = true | required = false |
//! |----------------------------------------|-----------------|------------------|
//! | singleton record with ports            | record port     | record port      |
//! | singleton record, NO ports             | declared port + warn | declared port + warn |
//! | no singleton record                    | REFUSE with remediation | declared port + warn |
//! | no record and NO declared ports        | config error    | config error     |
//! | auto-allocated declared port (host = 0)| REFUSE          | REFUSE (no address until the dep runs) |
//! | named port missing on a running record | REFUSE with remediation | REFUSE with remediation |
//!
//! Determinism: `depends_on` is iterated SORTED by dependency name and each
//! dep's `exports` SORTED by port name (the `HashMap` order is random; plan
//! output must be deterministic).

use std::path::Path;

use anyhow::Result;

use crate::config::ConfigFile;
use crate::microsandbox::plan::{EgressRule, EgressTarget, Protocol};
use crate::microsandbox::port_registry::{
    list_records_for_workload, list_records_for_workload_any_namespace, SandboxInstanceRecord,
};

/// Guest-visible host alias: guests reach services published on the host via
/// this gateway alias, NOT via `127.0.0.1` (which is host-only). Injected
/// `depends_on` addresses are ALWAYS rendered with this alias (ADR 0026(d)).
pub const GUEST_HOST_ALIAS: &str = "host.microsandbox.internal";

/// How a dependency's address was resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionSource {
    /// A running record (the singleton by default, or the
    /// `--use <dep>@<instance>`-selected parallel record) supplied the
    /// published host port.
    RunningInstance,
    /// No usable running record; the dependency's DECLARED ports on the
    /// shared bind (the convention address) were used instead.
    DeclaredFallback,
}

/// Parse raw `--use <dep>@<instance>` values into typed overrides
/// `(dep, instance-id)` (ADR 0026(d)).
///
/// The value is split on the FIRST `@` (workload names never contain `@`,
/// so this is unambiguous). Malformed values (no `@`, empty dep, empty id)
/// are a clear usage error naming the offending value; the id is validated
/// through the same [`crate::microsandbox::slots::validate_instance_id`]
/// gate `--instance` uses.
///
/// PURE parsing: whether the named dep is DECLARED in the workload's
/// depends_on map (and whether a matching instance record exists) is
/// validated at resolution time inside [`resolve_depends_on`].
pub fn parse_use_overrides(values: &[String]) -> Result<Vec<(String, String)>> {
    let mut overrides = Vec::with_capacity(values.len());
    for v in values {
        let Some((dep, id)) = v.split_once('@') else {
            anyhow::bail!(
                "invalid --use value '{}': expected <dep>@<instance> (missing '@')",
                v
            );
        };
        if dep.is_empty() {
            anyhow::bail!(
                "invalid --use value '{}': the dependency name before '@' cannot be empty",
                v
            );
        }
        if id.is_empty() {
            anyhow::bail!(
                "invalid --use value '{}': the instance id after '@' cannot be empty",
                v
            );
        }
        crate::microsandbox::slots::validate_instance_id(id)
            .map_err(|e| anyhow::anyhow!("invalid --use value '{}': {}", v, e))?;
        overrides.push((dep.to_string(), id.to_string()));
    }
    Ok(overrides)
}

/// The plan-time resolution of one declared dependency request: the spec's
/// `env` var (primary/unnamed port) or one `exports` entry (named port).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDependency {
    /// Dependency workload name (the `depends_on.<dep>` key).
    pub dep: String,
    /// Env var the resolved address is injected as (the spec's `env` or an
    /// `exports` value).
    pub env_var: String,
    /// Guest-visible injected address (`host.microsandbox.internal:<port>`).
    pub address: String,
    /// The resolved host port (drives the derived egress rule).
    pub host_port: u16,
    /// Whether the address came from a running record or the declared-port
    /// fallback.
    pub source: ResolutionSource,
    /// Port name this resolution targeted (`None` = the primary/unnamed port).
    pub port_name: Option<String>,
}

impl ResolvedDependency {
    /// The egress allow rule derived from this resolution, marked with its
    /// origin for plan rendering (` (derived: depends_on '<dep>')`).
    pub fn derived_egress_rule(&self) -> EgressRule {
        EgressRule {
            protocol: Protocol::Tcp,
            port: self.host_port,
            target: EgressTarget::Host,
            derived_from: Some(self.dep.clone()),
        }
    }
}

/// The published host port a running record is reachable on: its first
/// published host port (`port_pairs[0].host` when `port_pairs` is non-empty,
/// else `ports[0]`). `None` when the record carries no ports at all.
fn record_host_port(record: &SandboxInstanceRecord) -> Option<u16> {
    record
        .port_pairs
        .first()
        .map(|p| p.host)
        .or_else(|| record.ports.first().copied())
}

/// The dependency's first DECLARED host port from the merged config
/// (`config.workloads[dep].ports[0].host`).
fn declared_host_port(config: &ConfigFile, dep: &str) -> Option<u16> {
    config
        .workloads
        .get(dep)
        .and_then(|w| w.ports.first())
        .map(|p| p.host)
}

/// The PRIMARY host port a running record is reachable on (ADR 0026(d)
/// namespaced-ports rule): the record's UNNAMED published port if it has one,
/// else the first published port (the legacy [`record_host_port`] rule).
fn record_primary_port(record: &SandboxInstanceRecord) -> Option<u16> {
    record
        .port_pairs
        .iter()
        .find(|p| p.name.is_none())
        .map(|p| p.host)
        .or_else(|| record_host_port(record))
}

/// The host port of a NAMED published port on a running record
/// (`port_pairs` entry whose `name` matches). `None` when the record has no
/// such port — including legacy records that carry no names at all.
fn record_port_by_name(record: &SandboxInstanceRecord, name: &str) -> Option<u16> {
    record
        .port_pairs
        .iter()
        .find(|p| p.name.as_deref() == Some(name))
        .map(|p| p.host)
}

/// The dependency's PRIMARY DECLARED host port from the merged config: the
/// unnamed port if declared, else the first declared port (legacy rule).
fn declared_primary_port(config: &ConfigFile, dep: &str) -> Option<u16> {
    config
        .workloads
        .get(dep)
        .and_then(|w| w.ports.iter().find(|p| p.name.is_none()))
        .map(|p| p.host)
        .or_else(|| declared_host_port(config, dep))
}

/// The DEPENDENCY's DECLARED host port for a NAMED port from the merged config.
fn declared_port_by_name(config: &ConfigFile, dep: &str, name: &str) -> Option<u16> {
    config
        .workloads
        .get(dep)
        .and_then(|w| w.ports.iter().find(|p| p.name.as_deref() == Some(name)))
        .map(|p| p.host)
}

/// Resolve every dependency declared by `workload_name` against the port
/// registry at `state_dir` (ADR 0026(d) discovery-lite).
///
/// `use_overrides` carries the typed `--use <dep>@<instance>` selections
/// (parsed by [`parse_use_overrides`]): for the named dep the record whose
/// instance id matches is selected instead of the singleton. An override is
/// a PURE instance-selection override — it is validated against the
/// workload's DECLARED depends_on map here:
///
/// - an override naming a dep NOT in the map → hard error (`--use` only
///   overrides selection for DECLARED depends_on entries);
/// - any override when the workload declares no depends_on at all → hard
///   error;
/// - no running record with the selected instance id → hard error naming
///   dep + instance (unknown instance; no declared-port fallback — the user
///   explicitly chose an instance).
///
/// Iterates `depends_on` SORTED by dependency name (determinism: the plan
/// output derived from this list must not depend on `HashMap` order) and
/// emits ONE [`ResolvedDependency`] per request: the spec's `env` targets
/// the PRIMARY port (unnamed-if-present-else-first) and each `exports` entry
/// targets the NAMED port it references, with `exports` iterated SORTED by
/// port name.
///
/// Returns one [`ResolvedDependency`] per env/export request, or a hard error
/// when (a) a `required = true` dependency has no running singleton record
/// (the refusal names the start command), (b) no address can be derived at
/// all (no running record AND no declared ports), (c) an exports port name is
/// missing on a RUNNING record (restart it so its named ports are recorded),
/// or (d) the only declared port is auto-allocated (`host = 0`) — an auto
/// port has no address until the dependency runs, so even an optional dep
/// refuses rather than inject `host.microsandbox.internal:0`.
///
/// Warnings (fallbacks, per-IP binds, port-less records) are emitted on
/// stderr via `eprintln!`, matching the house "WARNING:"/"warning:"
/// conventions.
pub fn resolve_depends_on(
    config: &ConfigFile,
    workload_name: &str,
    state_dir: &Path,
    use_overrides: &[(String, String)],
    namespace: &str,
) -> Result<Vec<ResolvedDependency>> {
    resolve_depends_on_full(
        config,
        workload_name,
        state_dir,
        use_overrides,
        namespace,
        None,
    )
}

/// The full resolution seam (ADR 0030 P2.1): [`resolve_depends_on`] plus
/// the DEPENDENT's own parallel instance id for mode-aware DEFAULT exports
/// selection.
///
/// When a dep's mode (the shared
/// [`crate::commands::deps::dep_instance_mode`] derivation) is **Scoped**
/// and `dependent_instance_id` is `Some(id)`, the no-override selection
/// picks the record whose parallel id is `<workload_name>-<id>` (e.g.
/// dependent `prime` on instance `1` → `litellm@prime-1`) within the
/// namespace. When that scoped record is ABSENT: a required dep refuses
/// with the start command naming the scoped instance; an optional dep falls
/// back to the declared port (existing machinery). **Fresh** without an
/// explicit `--use` keeps the singleton-selection behavior — the fresh
/// record is only knowable via the injected `--use` the dep auto-start
/// executor returns (this is the plan-time view for a not-yet-started
/// dependent). **Shared** is unchanged (singleton).
pub fn resolve_depends_on_full(
    config: &ConfigFile,
    workload_name: &str,
    state_dir: &Path,
    use_overrides: &[(String, String)],
    namespace: &str,
    dependent_instance_id: Option<&str>,
) -> Result<Vec<ResolvedDependency>> {
    let Some(workload) = config.workloads.get(workload_name) else {
        anyhow::bail!("workload '{}' not found in config", workload_name);
    };
    if workload.depends_on.is_empty() {
        if let Some((dep, _)) = use_overrides.first() {
            anyhow::bail!(
                "--use {}@…: workload '{}' declares no depends_on entries; \
                 --use is a pure instance-selection override for DECLARED dependencies (ADR 0026(d))",
                dep,
                workload_name
            );
        }
        return Ok(Vec::new());
    }
    for (dep, _) in use_overrides {
        if !workload.depends_on.contains_key(dep) {
            anyhow::bail!(
                "--use {}@…: '{}' is not a declared depends_on entry of workload '{}' \
                 (declared: {}); --use only overrides instance selection for DECLARED dependencies (ADR 0026(d))",
                dep,
                dep,
                workload_name,
                workload
                    .depends_on
                    .keys()
                    .map(|k| k.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    let mut deps: Vec<(&String, &crate::config::DependsOnSpec)> =
        workload.depends_on.iter().collect();
    deps.sort_by(|a, b| a.0.cmp(b.0));

    let mut resolved = Vec::with_capacity(deps.len());
    for (dep, spec) in deps {
        // P2: one resolution request per `env` (primary/unnamed port) and per
        // `exports` entry (named port). exports are sorted by port name so
        // plan output is deterministic (HashMap order is random).
        let mut requests: Vec<(String, Option<String>)> = Vec::new();
        if let Some(env_var) = &spec.env {
            requests.push((env_var.clone(), None));
        }
        let mut exports: Vec<(&String, &String)> = spec.exports.iter().collect();
        exports.sort_by(|a, b| a.0.cmp(b.0));
        for (name, env_var) in exports {
            requests.push((env_var.clone(), Some(name.clone())));
        }

        let records = list_records_for_workload(state_dir, dep, namespace)?;
        // Selection (ADR 0026(d) + ADR 0030 Phase 2 T1 + P2.1): default =
        // the dependency's SINGLETON slot record in the DEPENDENT's
        // namespace (the one whose `instance` name contains no `@`); a
        // `--use <dep>@<instance>` override selects the record whose
        // parallel id matches instead — a PURE selection override with no
        // declared-port fallback when the chosen instance is not running.
        //
        // P2.1 mode-aware default: a SCOPED dep of a parallel dependent
        // (`dependent_instance_id = Some(id)`) selects the record whose
        // parallel id is `<workload_name>-<id>` (e.g. `litellm@prime-1`).
        // FRESH without an explicit `--use` keeps singleton selection (the
        // fresh record is only knowable via the injected --use the
        // auto-start executor returns). SHARED is unchanged.
        //
        // Collision-visibility (ADR 0030 Phase 2): when the dependent's
        // namespace has NO record for this dep but ANOTHER namespace does, the
        // two repos declaring the same workload name collide in the merged
        // config (last layer wins). Make the collision VISIBLE: warn (or refuse
        // for required deps) naming the namespace that holds the record.
        let mut absent_reason: Option<String> = None;
        let (selected, selector): (Option<&SandboxInstanceRecord>, String) = match use_overrides
            .iter()
            .find(|(d, _)| d == dep)
        {
            Some((_, id)) => {
                let Some(record) = records.iter().find(|r| {
                    crate::microsandbox::slots::instance_id_of(&r.instance) == Some(id.as_str())
                }) else {
                    anyhow::bail!(
                        "--use {}@{}: no running instance '{}' of dependency '{}' is registered \
                             in namespace '{}' (start it with `workestrate workload up {} --instance {}`)",
                        dep,
                        id,
                        id,
                        dep,
                        namespace,
                        dep,
                        id
                    );
                };
                (Some(record), format!("--use {}@{}", dep, id))
            }
            None => {
                let scoped_id =
                    match crate::commands::deps::dep_instance_mode(config, workload_name, dep) {
                        crate::config::DepInstanceMode::Scoped => {
                            dependent_instance_id.map(|id| format!("{workload_name}-{id}"))
                        }
                        crate::config::DepInstanceMode::Shared
                        | crate::config::DepInstanceMode::Fresh => None,
                    };
                if let Some(scoped_id) = scoped_id {
                    let scoped = records.iter().find(|r| {
                        crate::microsandbox::slots::instance_id_of(&r.instance)
                            == Some(scoped_id.as_str())
                    });
                    match scoped {
                        Some(record) => (
                            Some(record),
                            format!("scoped instance '{}@{}'", dep, scoped_id),
                        ),
                        None => {
                            if spec.required {
                                anyhow::bail!(
                                    "dependency '{dep}' of workload '{workload_name}' is required \
                                     but its scoped instance '{dep}@{scoped_id}' is not running in \
                                     namespace '{namespace}'; start it with `workestrate workload \
                                     up {dep} --instance {scoped_id}`"
                                );
                            }
                            // Optional dep: declared-port fallback (existing
                            // machinery), with a scoped-aware reason.
                            absent_reason =
                                Some(format!("no scoped instance '{dep}@{scoped_id}' is running"));
                            (None, format!("scoped instance '{}@{}'", dep, scoped_id))
                        }
                    }
                } else {
                    let singleton = records.iter().find(|r| {
                        crate::microsandbox::slots::instance_id_of(&r.instance).is_none()
                    });
                    if singleton.is_none() {
                        // Collision-visibility: no record in THIS namespace, but
                        // another namespace holds one for the same workload.
                        let other = list_records_for_workload_any_namespace(state_dir, dep)?
                            .into_iter()
                            .filter(|r| r.namespace != namespace)
                            .map(|r| r.namespace)
                            .collect::<std::collections::BTreeSet<_>>();
                        if !other.is_empty() {
                            let namespaces = other.into_iter().collect::<Vec<_>>().join(", ");
                            if spec.required {
                                anyhow::bail!(
                                    "dependency '{}' of workload '{}' is required but not running in \
                                     namespace '{}'; a record for it exists in namespace(s) [{}] — \
                                     the same workload name is declared by multiple config repos \
                                     (last layer wins in the merged config). Start it in this \
                                     namespace, or use `--use {}@<instance>` to select a record from \
                                     another namespace.",
                                    dep,
                                    workload_name,
                                    namespace,
                                    namespaces,
                                    dep
                                );
                            }
                            eprintln!(
                                "warning: depends_on '{}': no record in namespace '{}', but a record \
                                 exists in namespace(s) [{}] — the same workload name is declared by \
                                 multiple config repos (last layer wins in the merged config). Falling \
                                 back to the declared port.",
                                dep, namespace, namespaces
                            );
                        }
                    }
                    (singleton, "singleton".to_string())
                }
            }
        };

        for (env_var, port_name) in requests {
            let resolved_one = match (selected, &port_name) {
                // PRIMARY rule: the unnamed port if present, else the first port.
                (Some(record), None) => match record_primary_port(record) {
                    Some(port) => {
                        // ADR 0026(f) DEFERRED-PENDING-E1: a record published on
                        // a per-IP parallel bind (bind_ip != 127.0.0.1) is NOT
                        // guest-reachability-verified. Inject the SAME guest form
                        // (host.microsandbox.internal:<port>) but warn loudly.
                        if record.bind_ip != crate::microsandbox::plan::default_bind_ip() {
                            eprintln!(
                                "warning: depends_on '{}': selected record '{}' ({}) is published on the per-IP bind {}; \
                                 guest-reachability of non-127.0.0.1 loopbacks via {} is DEFERRED-PENDING-E1 \
                                 (ADR 0026(f) conservative default) — the injected address may not be reachable from the guest",
                                dep,
                                record.instance,
                                selector,
                                record.bind_ip,
                                GUEST_HOST_ALIAS
                            );
                        }
                        ResolvedDependency {
                            dep: dep.clone(),
                            env_var,
                            address: format!("{}:{}", GUEST_HOST_ALIAS, port),
                            host_port: port,
                            source: ResolutionSource::RunningInstance,
                            port_name: None,
                        }
                    }
                    None => declared_fallback(
                        config,
                        dep,
                        &env_var,
                        None,
                        &format!(
                            "its {} record '{}' publishes no ports",
                            selector, record.instance
                        ),
                    )?,
                },
                (Some(record), Some(name)) => match record_port_by_name(record, name) {
                    Some(port) => {
                        // ADR 0026(f) DEFERRED-PENDING-E1 (named-port arm): as
                        // above, warn loudly on per-IP binds and inject the
                        // same guest form.
                        if record.bind_ip != crate::microsandbox::plan::default_bind_ip() {
                            eprintln!("warning: depends_on '{}': selected record '{}' ({}) is published on the per-IP bind {}; guest-reachability of non-127.0.0.1 loopbacks via {} is DEFERRED-PENDING-E1 (ADR 0026(f) conservative default) — the injected address for port '{}' may not be reachable from the guest", dep, record.instance, selector, record.bind_ip, GUEST_HOST_ALIAS, name);
                        }
                        ResolvedDependency {
                            dep: dep.clone(),
                            env_var,
                            address: format!("{}:{}", GUEST_HOST_ALIAS, port),
                            host_port: port,
                            source: ResolutionSource::RunningInstance,
                            port_name: Some(name.clone()),
                        }
                    }
                    None => anyhow::bail!(
                        "dependency '{}' is running ({} '{}') but has no published port named '{}'{}; \
                         restart it with `workestrate workload up {}` so its named ports are recorded",
                        dep,
                        selector,
                        record.instance,
                        name,
                        if record.port_pairs.is_empty() {
                            " (its registry record is legacy — it stores no port names)"
                        } else {
                            ""
                        },
                        dep
                    ),
                },
                (None, _) => {
                    if spec.required {
                        anyhow::bail!(
                            "dependency '{}' of workload '{}' is required but not running; start it with `workestrate workload up {}`",
                            dep,
                            workload_name,
                            dep
                        );
                    }
                    let reason = match &port_name {
                        None => absent_reason
                            .clone()
                            .unwrap_or_else(|| "no singleton instance is running".to_string()),
                        Some(name) => format!(
                            "{} (port '{name}')",
                            absent_reason
                                .as_deref()
                                .unwrap_or("no singleton instance is running")
                        ),
                    };
                    declared_fallback(
                        config,
                        dep,
                        &env_var,
                        port_name.as_deref(),
                        &reason,
                    )?
                }
            };
            resolved.push(resolved_one);
        }
    }
    Ok(resolved)
}

/// The declared-port fallback arm: resolve to the dependency's DECLARED host
/// port on the shared bind (the convention address), warning on stderr.
///
/// `port_name` selects the NAMED declared port (`None` = the primary/unnamed
/// port). Refusals (hard errors, even for optional deps):
///
/// - an auto-allocated declared port (`host = 0`) is NEVER injected — an auto
///   port has no address until the dependency runs, so the fallback refuses
///   instead of producing `host.microsandbox.internal:0`;
/// - no declared port at all → a config error (no address can be derived).
fn declared_fallback(
    config: &ConfigFile,
    dep: &str,
    env_var: &str,
    port_name: Option<&str>,
    reason: &str,
) -> Result<ResolvedDependency> {
    let port = match port_name {
        None => declared_primary_port(config, dep),
        Some(name) => declared_port_by_name(config, dep, name),
    };
    match port {
        Some(0) => {
            let port_label = match port_name {
                Some(name) => format!(" '{}'", name),
                None => String::new(),
            };
            anyhow::bail!(
                "dependency '{}' declares an auto-allocated port{} but is not running; start it first \
                 (`workestrate workload up {}`) so its port is allocated and recorded (an auto port has \
                 no address until the dependency runs)",
                dep,
                port_label,
                dep
            );
        }
        Some(port) => {
            eprintln!(
                "warning: depends_on '{}': {} — falling back to the declared port {} on {} (the convention address); \
                 the address may be stale if the instance publishes elsewhere",
                dep, reason, port, GUEST_HOST_ALIAS
            );
            Ok(ResolvedDependency {
                dep: dep.to_string(),
                env_var: env_var.to_string(),
                address: format!("{}:{}", GUEST_HOST_ALIAS, port),
                host_port: port,
                source: ResolutionSource::DeclaredFallback,
                port_name: port_name.map(str::to_string),
            })
        }
        None => match port_name {
            None => anyhow::bail!(
                "dependency '{}' cannot be resolved: it is not running and declares no ports \
                 (no address can be derived). Declare at least one host port on workload '{}' \
                 or start it with `workestrate workload up {}`.",
                dep,
                dep,
                dep
            ),
            Some(name) => anyhow::bail!(
                "dependency '{}' cannot be resolved for port '{}': it is not running and declares no \
                 port named '{}'. Declare it on workload '{}' or start it with `workestrate workload up {}`.",
                dep,
                name,
                name,
                dep,
                dep
            ),
        },
    }
}

/// Apply resolution results to a plan in construction: append injected env
/// vars AFTER the declared env and derived egress rules AFTER the expanded
/// declared egress rules.
///
/// Dedup/conflict rules (normative):
///
/// - DECLARED ENV WINS: when `declared_env` already contains the injection
///   var name, the explicit declared value wins — injection is skipped with
///   a stderr warning.
/// - EGRESS DEDUP: when an identical egress rule (same protocol + port +
///   target) already exists in `expanded_egress`, no duplicate derived rule
///   is added.
///
/// Pure and warning-only; resolution failures happened earlier in
/// [`resolve_depends_on`].
pub fn apply_resolution(
    resolved: &[ResolvedDependency],
    declared_env: &[crate::microsandbox::plan::EnvVar],
    expanded_egress: &[EgressRule],
) -> (Vec<crate::microsandbox::plan::EnvVar>, Vec<EgressRule>) {
    let mut injected_env = Vec::new();
    let mut derived_egress = Vec::new();
    for r in resolved {
        if declared_env.iter().any(|e| e.name == r.env_var) {
            eprintln!(
                "warning: depends_on '{}': declared env already contains '{}' — the explicit declared value wins; \
                 skipping injection of '{}'",
                r.dep, r.env_var, r.address
            );
        } else {
            // The injected value is a URL, NOT a secret: render the value
            // (never redact), is_secret = false.
            injected_env.push(crate::microsandbox::plan::EnvVar {
                name: r.env_var.clone(),
                value: r.address.clone(),
                is_secret: false,
                reject_placeholder: None,
                injected_by: Some(r.dep.clone()),
                injected_port: r.port_name.clone(),
            });
        }
        let rule = r.derived_egress_rule();
        let duplicate = expanded_egress
            .iter()
            .chain(derived_egress.iter())
            .any(|existing| {
                existing.protocol == rule.protocol
                    && existing.port == rule.port
                    && existing.target == rule.target
            });
        if !duplicate {
            derived_egress.push(rule);
        }
    }
    (injected_env, derived_egress)
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
    use crate::microsandbox::plan::{EnvVar, PortMapping};
    use crate::microsandbox::port_registry::{
        check_and_register_sandbox_lifecycle, register_sandbox,
    };
    use std::net::{IpAddr, Ipv4Addr};

    /// Merged-config fixture: dependent `pi` with a `depends_on` map, plus
    /// the dependency workloads `litellm` (declares 4000:4000) and `noports`
    /// (declares none). The caller mutates the depends_on map per test.
    fn depends_config() -> ConfigFile {
        let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"

[workloads.pi.network]
default_deny = true

[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[[workloads.litellm.ports]]
host = 4000
guest = 4000

[workloads.litellm.network]
default_deny = true

[workloads.noports]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.noports.network]
default_deny = true
"#;
        toml::from_str(toml).expect("depends_on fixture must parse")
    }

    fn loopback(n: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, n))
    }

    /// Register a singleton lifecycle record for `workload` with one
    /// host:guest pair on `bind`.
    fn register_singleton(
        state_dir: &Path,
        workload: &str,
        bind: IpAddr,
        host: u16,
        guest: u16,
    ) -> Result<()> {
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
            "2026-07-30T00:00:00Z",
            "default",
            None,
        )
    }

    /// Register a singleton lifecycle record for `workload` with one NAMED
    /// host:guest pair on `bind` (namespaced-ports records; P2).
    fn register_singleton_named(
        state_dir: &Path,
        workload: &str,
        bind: IpAddr,
        host: u16,
        guest: u16,
        name: &str,
    ) -> Result<()> {
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
                name: Some(name.to_string()),
            }],
            "2026-07-30T00:00:00Z",
            "default",
            None,
        )
    }

    /// `depends_config()` variant: `litellm` ALSO declares a named port
    /// `api:14000:14000` next to its legacy unnamed 4000 — the canonical
    /// namespaced-exports fixture. Callers mutate `pi.depends_on.litellm`
    /// (`exports`, `required`, `env`) per test.
    fn named_config() -> ConfigFile {
        let mut config = depends_config();
        config
            .workloads
            .get_mut("litellm")
            .unwrap()
            .ports
            .push(PortMapping {
                host: 14000,
                guest: 14000,
                bind_ip: loopback(1),
                name: Some("api".to_string()),
            });
        config
    }

    // (1) Singleton resolution: a running singleton record supplies the
    // published host port; the injected address is the guest form.
    #[test]
    fn singleton_record_resolves_to_guest_form_address() -> Result<()> {
        let state_dir = unique_state_dir("disc-singleton");
        register_singleton(&state_dir, "litellm", loopback(1), 4000, 4000)?;
        let config = depends_config();

        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;
        assert_eq!(resolved.len(), 1);
        let r = &resolved[0];
        assert_eq!(r.dep, "litellm");
        assert_eq!(r.env_var, "LITELLM_URL");
        assert_eq!(r.address, "host.microsandbox.internal:4000");
        assert_eq!(r.host_port, 4000);
        assert_eq!(r.source, ResolutionSource::RunningInstance);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // (2) Required + not running → refuse at plan time with remediation.
    #[test]
    fn required_dep_not_running_refuses_with_remediation() -> Result<()> {
        let state_dir = unique_state_dir("disc-required");
        let mut config = depends_config();
        config
            .workloads
            .get_mut("pi")
            .unwrap()
            .depends_on
            .get_mut("litellm")
            .unwrap()
            .required = true;

        let err = resolve_depends_on(&config, "pi", &state_dir, &[], "default").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("is required but not running; start it with"),
            "refusal must carry the remediation lead: {msg}"
        );
        assert!(
            msg.contains("workestrate workload up litellm"),
            "refusal must name the start command: {msg}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // (3) Optional + not running → declared-port fallback; resolution
    // succeeds on the DECLARED port with the guest form.
    #[test]
    fn optional_dep_not_running_falls_back_to_declared_port() -> Result<()> {
        let state_dir = unique_state_dir("disc-optional");
        let config = depends_config();

        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;
        assert_eq!(resolved.len(), 1);
        let r = &resolved[0];
        assert_eq!(r.address, "host.microsandbox.internal:4000");
        assert_eq!(r.host_port, 4000);
        assert_eq!(r.source, ResolutionSource::DeclaredFallback);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // (4) Guest-form correctness: injected addresses NEVER contain
    // 127.0.0.1 and ALWAYS start with the gateway alias — across every
    // resolution arm (running singleton, declared fallback, per-IP record).
    #[test]
    fn injected_address_never_uses_loopback_always_guest_form() -> Result<()> {
        let state_dir = unique_state_dir("disc-guest-form");
        // Arm A: singleton record on the shared bind.
        register_singleton(&state_dir, "litellm", loopback(1), 4000, 4000)?;
        // Arm B: singleton record on a PER-IP bind (legacy-shaped name with
        // no `@`, bind != 127.0.0.1).
        register_singleton(&state_dir, "noports", loopback(2), 9000, 9000)?;
        let mut config = depends_config();
        config.workloads.get_mut("pi").unwrap().depends_on.insert(
            "noports".to_string(),
            crate::config::DependsOnSpec {
                env: Some("NOPORTS_URL".to_string()),
                required: false,
                exports: Default::default(),
                on_conflict: None,
                instance: None,
            },
        );

        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;
        assert_eq!(resolved.len(), 2);
        for r in &resolved {
            assert!(
                r.address.starts_with("host.microsandbox.internal:"),
                "address must start with the guest gateway alias: {}",
                r.address
            );
            assert!(
                !r.address.contains("127.0.0.1"),
                "address must never contain 127.0.0.1 (host-only): {}",
                r.address
            );
        }
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // (5) Declared-env conflict: the dependent's explicit declared env WINS;
    // injection is skipped.
    #[test]
    fn declared_env_wins_over_injection() -> Result<()> {
        let state_dir = unique_state_dir("disc-env-conflict");
        register_singleton(&state_dir, "litellm", loopback(1), 4000, 4000)?;
        let config = depends_config();
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;

        let declared = vec![EnvVar::literal("LITELLM_URL", "http://custom:1")];
        let (injected, _) = apply_resolution(&resolved, &declared, &[]);
        assert!(
            injected.is_empty(),
            "declared env var must win; injection skipped: {injected:?}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // (6) Derived-egress dedup: the dependent already expands an identical
    // rule (litellm_proxy = tcp:4000 -> host) → no duplicate derived rule.
    #[test]
    fn derived_egress_dedups_against_expanded_declared_rules() -> Result<()> {
        let state_dir = unique_state_dir("disc-egress-dedup");
        register_singleton(&state_dir, "litellm", loopback(1), 4000, 4000)?;
        let config = depends_config();
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;

        let declared_expanded = vec![EgressRule::litellm_proxy()];
        let (_, derived) = apply_resolution(&resolved, &[], &declared_expanded);
        assert!(
            derived.is_empty(),
            "identical expanded rule must suppress the derived duplicate: {derived:?}"
        );

        // Sanity: with NO declared overlap the rule IS derived, marked.
        let (_, derived) = apply_resolution(&resolved, &[], &[]);
        assert_eq!(derived.len(), 1);
        assert_eq!(derived[0].port, 4000);
        assert_eq!(derived[0].protocol, Protocol::Tcp);
        assert_eq!(derived[0].target, EgressTarget::Host);
        assert_eq!(derived[0].derived_from.as_deref(), Some("litellm"));
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // (7) Per-IP record: a singleton record on 127.0.0.2 still injects the
    // SAME guest form (the warning path is exercised; reachability is
    // DEFERRED-PENDING-E1 per ADR 0026(f)).
    #[test]
    fn per_ip_record_still_injects_guest_form() -> Result<()> {
        let state_dir = unique_state_dir("disc-per-ip");
        register_singleton(&state_dir, "litellm", loopback(2), 4000, 4000)?;
        let config = depends_config();

        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].address, "host.microsandbox.internal:4000");
        assert_eq!(resolved[0].source, ResolutionSource::RunningInstance);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // (8) Determinism: two deps → resolution order is sorted by dep name,
    // stable across runs (HashMap iteration order is random).
    #[test]
    fn resolution_order_is_sorted_by_dep_name() -> Result<()> {
        let state_dir = unique_state_dir("disc-order");
        let mut config = depends_config();
        // Add a second dep that sorts BEFORE litellm.
        config.workloads.get_mut("pi").unwrap().depends_on.insert(
            "aaa".to_string(),
            crate::config::DependsOnSpec {
                env: Some("AAA_URL".to_string()),
                required: false,
                exports: Default::default(),
                on_conflict: None,
                instance: None,
            },
        );
        // Give the `litellm` workload an alias `aaa` with its own declared
        // port by renaming: simplest is to add an `aaa` workload.
        let litellm = config.workloads.get("litellm").unwrap().clone();
        config.workloads.insert("aaa".to_string(), litellm);

        for _ in 0..8 {
            let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;
            let order: Vec<&str> = resolved.iter().map(|r| r.dep.as_str()).collect();
            assert_eq!(
                order,
                vec!["aaa", "litellm"],
                "resolution must be sorted by dep name across runs"
            );
            let env_order: Vec<&str> = resolved.iter().map(|r| r.env_var.as_str()).collect();
            assert_eq!(env_order, vec!["AAA_URL", "LITELLM_URL"]);
        }
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // Spec §3: no record AND no declared ports → hard config error (no
    // address can be derived), even when required = false.
    #[test]
    fn no_record_and_no_declared_ports_is_a_config_error() -> Result<()> {
        let state_dir = unique_state_dir("disc-no-address");
        let mut config = depends_config();
        config.workloads.get_mut("pi").unwrap().depends_on.insert(
            "noports".to_string(),
            crate::config::DependsOnSpec {
                env: Some("NOPORTS_URL".to_string()),
                required: false,
                exports: Default::default(),
                on_conflict: None,
                instance: None,
            },
        );

        let err = resolve_depends_on(&config, "pi", &state_dir, &[], "default").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("declares no ports"),
            "config error must explain no address can be derived: {msg}"
        );
        assert!(msg.contains("noports"), "error must name the dep: {msg}");
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // Spec §7: a RUNNING record with NO ports at all falls back to the
    // declared-port behavior + warn.
    #[test]
    fn running_record_without_ports_falls_back_to_declared() -> Result<()> {
        let state_dir = unique_state_dir("disc-record-noports");
        // A legacy-shaped record (register_sandbox writes no port_pairs) with
        // an empty ports list for the `noports` workload.
        register_sandbox(
            &state_dir,
            "personal-noports",
            Some("personal"),
            "noports",
            &[],
        )?;
        let mut config = depends_config();
        config.workloads.get_mut("pi").unwrap().depends_on.insert(
            "noports".to_string(),
            crate::config::DependsOnSpec {
                env: Some("NOPORTS_URL".to_string()),
                required: false,
                exports: Default::default(),
                on_conflict: None,
                instance: None,
            },
        );
        // `noports` declares no ports either → the fallback itself errors.
        let err = resolve_depends_on(&config, "pi", &state_dir, &[], "default").unwrap_err();
        assert!(err.to_string().contains("declares no ports"));

        // Now give `noports` a declared port: the port-less RECORD falls
        // back to the declared port and resolution succeeds.
        config
            .workloads
            .get_mut("noports")
            .unwrap()
            .ports
            .push(PortMapping::new(9100, 9100));
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;
        let r = resolved
            .iter()
            .find(|r| r.dep == "noports")
            .expect("noports resolution present");
        assert_eq!(r.address, "host.microsandbox.internal:9100");
        assert_eq!(r.source, ResolutionSource::DeclaredFallback);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // Parallel `<slot>@<id>` records are NOT selected by declaration-
    // triggered resolution (only the singleton is; `--use` comes later).
    #[test]
    fn parallel_records_are_ignored_without_a_singleton() -> Result<()> {
        let state_dir = unique_state_dir("disc-parallel-ignored");
        // Only a PARALLEL record exists for litellm (name carries `@`).
        check_and_register_sandbox_lifecycle(
            &state_dir,
            "personal-litellm@canary",
            Some("personal"),
            "litellm",
            loopback(2),
            &[14000],
            &[PortMapping {
                host: 14000,
                guest: 4000,
                bind_ip: loopback(2),
                name: None,
            }],
            "2026-07-30T00:00:00Z",
            "default",
            None,
        )?;
        let config = depends_config();

        // No singleton → optional dep falls back to the DECLARED port (not
        // the parallel record's 14000).
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].host_port, 4000);
        assert_eq!(resolved[0].source, ResolutionSource::DeclaredFallback);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // Legacy record (register_sandbox: no port_pairs, ports list only) →
    // port resolution uses ports[0].
    #[test]
    fn legacy_record_without_port_pairs_uses_ports_list() -> Result<()> {
        let state_dir = unique_state_dir("disc-legacy");
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        let config = depends_config();
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;
        assert_eq!(resolved[0].address, "host.microsandbox.internal:4000");
        assert_eq!(resolved[0].source, ResolutionSource::RunningInstance);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- ADR 0026(d)/C3-W2: --use <dep>@<instance> selection override ----

    /// Register a PARALLEL lifecycle record `<slot>@<id>` for `workload`.
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
            "2026-07-30T00:00:00Z",
            "default",
            None,
        )
    }

    // parse_use_overrides: well-formed values split on the FIRST `@` into
    // (dep, id) pairs in order.
    #[test]
    fn parse_use_overrides_splits_on_first_at() -> Result<()> {
        let values = vec!["litellm@canary".to_string(), "redis@blue-2".to_string()];
        let parsed = parse_use_overrides(&values)?;
        assert_eq!(
            parsed,
            vec![
                ("litellm".to_string(), "canary".to_string()),
                ("redis".to_string(), "blue-2".to_string()),
            ]
        );
        assert!(parse_use_overrides(&[])?.is_empty());
        Ok(())
    }

    // parse_use_overrides: malformed values (no `@`, empty dep, empty id)
    // are clear usage errors naming the offending value.
    #[test]
    fn parse_use_overrides_rejects_malformed_values() {
        for bad in ["nodep", "@x", "dep@"] {
            let err = parse_use_overrides(&[bad.to_string()]).unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains("invalid --use value"),
                "error must flag the value as an invalid --use: {msg}"
            );
            assert!(
                msg.contains(bad),
                "error must name the offending value '{bad}': {msg}"
            );
        }
    }

    // parse_use_overrides: the id passes through validate_instance_id —
    // an invalid slug is rejected with the validator's reason.
    #[test]
    fn parse_use_overrides_rejects_invalid_instance_id() {
        for bad in ["litellm@ALL", "litellm@1234", "litellm@-lead"] {
            assert!(
                parse_use_overrides(&[bad.to_string()]).is_err(),
                "invalid id in '{bad}' must be rejected"
            );
        }
        // `all` is a valid shape but reserved — must also be rejected here.
        let err = parse_use_overrides(&["litellm@all".to_string()]).unwrap_err();
        assert!(err.to_string().contains("reserved"));
    }

    // Override resolution: BOTH a singleton record AND a parallel record
    // exist — the default selects the singleton; `--use litellm@canary`
    // selects the parallel record (per-IP bind → the DEFERRED-PENDING-E1
    // warning path is exercised; the injected guest form carries the
    // PARALLEL record's port).
    #[test]
    fn use_override_selects_parallel_record_over_singleton() -> Result<()> {
        let state_dir = unique_state_dir("disc-use-override");
        register_singleton(&state_dir, "litellm", loopback(1), 4000, 4000)?;
        register_parallel(&state_dir, "litellm", "canary", loopback(2), 14000, 4000)?;
        let config = depends_config();

        // Default: the singleton record wins (port 4000).
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;
        assert_eq!(resolved[0].host_port, 4000);
        assert_eq!(resolved[0].address, "host.microsandbox.internal:4000");
        assert_eq!(resolved[0].source, ResolutionSource::RunningInstance);

        // `--use litellm@canary`: the parallel record's port (14000) is
        // injected — the bind is per-IP, so this ALSO exercises the
        // DEFERRED-PENDING-E1 warning arm (stderr; not asserted here).
        let overrides = vec![("litellm".to_string(), "canary".to_string())];
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &overrides, "default")?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].dep, "litellm");
        assert_eq!(resolved[0].host_port, 14000);
        assert_eq!(resolved[0].address, "host.microsandbox.internal:14000");
        assert_eq!(resolved[0].source, ResolutionSource::RunningInstance);
        // The derived egress rule follows the SELECTED record's port.
        assert_eq!(resolved[0].derived_egress_rule().port, 14000);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // Unknown instance: `--use litellm@ghost` names dep + instance and does
    // NOT fall back to the declared port (the user explicitly selected).
    #[test]
    fn use_override_unknown_instance_is_a_hard_error() -> Result<()> {
        let state_dir = unique_state_dir("disc-use-ghost");
        register_singleton(&state_dir, "litellm", loopback(1), 4000, 4000)?;
        let config = depends_config();

        let overrides = vec![("litellm".to_string(), "ghost".to_string())];
        let err = resolve_depends_on(&config, "pi", &state_dir, &overrides, "default").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("litellm"), "error must name the dep: {msg}");
        assert!(msg.contains("ghost"), "error must name the instance: {msg}");
        assert!(
            msg.contains("no running instance"),
            "error must explain the selection found nothing: {msg}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // Unknown dep: `--use redis@canary` when depends_on only declares
    // litellm → hard error naming redis (override only applies to DECLARED
    // deps).
    #[test]
    fn use_override_for_undeclared_dep_is_a_hard_error() -> Result<()> {
        let state_dir = unique_state_dir("disc-use-undeclared");
        let config = depends_config();
        let overrides = vec![("redis".to_string(), "canary".to_string())];
        let err = resolve_depends_on(&config, "pi", &state_dir, &overrides, "default").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("redis"), "error must name the dep: {msg}");
        assert!(
            msg.contains("not a declared depends_on entry"),
            "error must explain --use only overrides DECLARED deps: {msg}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // `--use` on a workload with NO depends_on at all → hard error.
    #[test]
    fn use_override_without_any_depends_on_is_a_hard_error() -> Result<()> {
        let state_dir = unique_state_dir("disc-use-nodeps");
        let mut config = depends_config();
        config.workloads.get_mut("pi").unwrap().depends_on.clear();
        let overrides = vec![("litellm".to_string(), "canary".to_string())];
        let err = resolve_depends_on(&config, "pi", &state_dir, &overrides, "default").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("declares no depends_on entries"),
            "error must explain the workload has no depends_on: {msg}"
        );
        assert!(msg.contains("litellm"), "error must name the dep: {msg}");
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- P2 namespaced ports: exports (named-port) resolution ----

    // (1) exports→named resolution: a running singleton carrying a NAMED
    // port `api` resolves the `exports` entry to that port's host, marks the
    // resolution with the port name, and derives the matching egress rule.
    #[test]
    fn exports_resolve_to_named_port_of_running_dep() -> Result<()> {
        let state_dir = unique_state_dir("disc-exports-named");
        register_singleton_named(&state_dir, "litellm", loopback(1), 14000, 14000, "api")?;
        let mut config = named_config();
        let spec = config
            .workloads
            .get_mut("pi")
            .unwrap()
            .depends_on
            .get_mut("litellm")
            .unwrap();
        spec.env = None;
        spec.exports
            .insert("api".to_string(), "LITELLM_API_URL".to_string());

        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;
        assert_eq!(resolved.len(), 1);
        let r = &resolved[0];
        assert_eq!(r.dep, "litellm");
        assert_eq!(r.env_var, "LITELLM_API_URL");
        assert_eq!(r.address, "host.microsandbox.internal:14000");
        assert_eq!(r.host_port, 14000);
        assert_eq!(r.source, ResolutionSource::RunningInstance);
        assert_eq!(r.port_name.as_deref(), Some("api"));
        // The derived egress rule matches the NAMED port's host.
        assert_eq!(r.derived_egress_rule().port, 14000);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // (2) PRIMARY rule — unnamed-wins: a running record with BOTH an unnamed
    // port (4000) and a named `api` (14000) resolves `env` to the UNNAMED
    // port, never the named one.
    #[test]
    fn primary_rule_prefers_unnamed_port_when_present() -> Result<()> {
        let state_dir = unique_state_dir("disc-primary-unnamed");
        check_and_register_sandbox_lifecycle(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            loopback(1),
            &[4000, 14000],
            &[
                PortMapping {
                    host: 4000,
                    guest: 4000,
                    bind_ip: loopback(1),
                    name: None,
                },
                PortMapping {
                    host: 14000,
                    guest: 14000,
                    bind_ip: loopback(1),
                    name: Some("api".to_string()),
                },
            ],
            "2026-07-30T00:00:00Z",
            "default",
            None,
        )?;
        let config = depends_config();

        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].address, "host.microsandbox.internal:4000");
        assert_eq!(resolved[0].host_port, 4000);
        assert_eq!(resolved[0].port_name, None);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // (3) PRIMARY rule — else-first: a running record with ONLY named ports
    // (`api` first) resolves `env` to the FIRST published port (the legacy
    // first-port rule, with no `port_name` marker).
    #[test]
    fn primary_rule_falls_back_to_first_port_when_only_named() -> Result<()> {
        let state_dir = unique_state_dir("disc-primary-first");
        check_and_register_sandbox_lifecycle(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            loopback(1),
            &[14000, 15000],
            &[
                PortMapping {
                    host: 14000,
                    guest: 4000,
                    bind_ip: loopback(1),
                    name: Some("api".to_string()),
                },
                PortMapping {
                    host: 15000,
                    guest: 5000,
                    bind_ip: loopback(1),
                    name: Some("admin".to_string()),
                },
            ],
            "2026-07-30T00:00:00Z",
            "default",
            None,
        )?;
        let config = depends_config();

        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].address, "host.microsandbox.internal:14000");
        assert_eq!(resolved[0].host_port, 14000);
        assert_eq!(resolved[0].port_name, None);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // (4) exports determinism: two exports are resolved SORTED by port name
    // (`admin` before `api`), regardless of the HashMap's iteration order.
    #[test]
    fn exports_resolve_sorted_by_port_name() -> Result<()> {
        let state_dir = unique_state_dir("disc-exports-order");
        check_and_register_sandbox_lifecycle(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            loopback(1),
            &[14000, 15000],
            &[
                PortMapping {
                    host: 14000,
                    guest: 4000,
                    bind_ip: loopback(1),
                    name: Some("api".to_string()),
                },
                PortMapping {
                    host: 15000,
                    guest: 5000,
                    bind_ip: loopback(1),
                    name: Some("admin".to_string()),
                },
            ],
            "2026-07-30T00:00:00Z",
            "default",
            None,
        )?;
        let mut config = named_config();
        let spec = config
            .workloads
            .get_mut("pi")
            .unwrap()
            .depends_on
            .get_mut("litellm")
            .unwrap();
        spec.env = None;
        spec.exports
            .insert("admin".to_string(), "LITELLM_ADMIN_URL".to_string());
        spec.exports
            .insert("api".to_string(), "LITELLM_API_URL".to_string());

        for _ in 0..8 {
            let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;
            assert_eq!(resolved.len(), 2);
            let env_order: Vec<&str> = resolved.iter().map(|r| r.env_var.as_str()).collect();
            assert_eq!(env_order, vec!["LITELLM_ADMIN_URL", "LITELLM_API_URL"]);
            let port_order: Vec<Option<&str>> =
                resolved.iter().map(|r| r.port_name.as_deref()).collect();
            assert_eq!(port_order, vec![Some("admin"), Some("api")]);
        }
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // (5a) AUTO-allocated declared port (host = 0) + NOT running: refused
    // even when required = false — an auto port has no address until the dep
    // runs. Exercised through an EXPORTS entry (env-less spec).
    #[test]
    fn auto_port_on_not_running_dep_refuses_via_exports() -> Result<()> {
        let state_dir = unique_state_dir("disc-auto-refuse-exports");
        let mut config = named_config();
        // litellm's ONLY declared port is an auto-allocated NAMED port.
        config.workloads.get_mut("litellm").unwrap().ports.clear();
        config
            .workloads
            .get_mut("litellm")
            .unwrap()
            .ports
            .push(PortMapping {
                host: 0,
                guest: 14000,
                bind_ip: loopback(1),
                name: Some("api".to_string()),
            });
        let spec = config
            .workloads
            .get_mut("pi")
            .unwrap()
            .depends_on
            .get_mut("litellm")
            .unwrap();
        spec.env = None;
        spec.exports
            .insert("api".to_string(), "LITELLM_API_URL".to_string());

        let err = resolve_depends_on(&config, "pi", &state_dir, &[], "default").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("auto-allocated"),
            "refusal must mention the auto port: {msg}"
        );
        assert!(
            msg.contains("start it first"),
            "refusal must direct the user to start the dep: {msg}"
        );
        assert!(msg.contains("'api'"), "refusal must name the port: {msg}");
        assert!(
            !msg.contains("host.microsandbox.internal:0"),
            "no :0 address may be produced: {msg}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // (5b) AUTO-allocated declared port (host = 0) + NOT running: refused via
    // the `env` (primary) request.
    #[test]
    fn auto_port_on_not_running_dep_refuses_via_env() -> Result<()> {
        let state_dir = unique_state_dir("disc-auto-refuse-env");
        let mut config = depends_config();
        // litellm's ONLY declared port is an auto-allocated UNNAMED port.
        config.workloads.get_mut("litellm").unwrap().ports.clear();
        config
            .workloads
            .get_mut("litellm")
            .unwrap()
            .ports
            .push(PortMapping::new(0, 4000));

        let err = resolve_depends_on(&config, "pi", &state_dir, &[], "default").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("auto-allocated"),
            "refusal must mention the auto port: {msg}"
        );
        assert!(
            msg.contains("start it first"),
            "refusal must direct the user to start the dep: {msg}"
        );
        assert!(
            !msg.contains("host.microsandbox.internal:0"),
            "no :0 address may be produced: {msg}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // (6) MISSING named port on a LEGACY running record (register_sandbox:
    // no port_pairs): the exports entry for that name is a hard error naming
    // the port and directing a restart so named ports get recorded.
    #[test]
    fn exports_to_missing_named_port_on_legacy_record_is_a_hard_error() -> Result<()> {
        let state_dir = unique_state_dir("disc-legacy-missing-name");
        // Legacy record: register_sandbox writes NO port_pairs (no names).
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        let mut config = named_config();
        let spec = config
            .workloads
            .get_mut("pi")
            .unwrap()
            .depends_on
            .get_mut("litellm")
            .unwrap();
        spec.env = None;
        spec.exports
            .insert("api".to_string(), "LITELLM_API_URL".to_string());

        let err = resolve_depends_on(&config, "pi", &state_dir, &[], "default").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("'api'"), "error must name the port: {msg}");
        assert!(
            msg.contains("restart it"),
            "error must direct a restart: {msg}"
        );
        assert!(
            msg.contains("legacy"),
            "error must explain the record stores no port names: {msg}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // (7) Named port NOT declared on a RUNNING record: exports referencing
    // `admin` when the dep only declares/runs `api` → hard error naming
    // `admin` (runtime enforcement; discovery does not rely on validation).
    #[test]
    fn exports_to_undeclared_named_port_of_running_dep_is_a_hard_error() -> Result<()> {
        let state_dir = unique_state_dir("disc-undeclared-name");
        register_singleton_named(&state_dir, "litellm", loopback(1), 14000, 14000, "api")?;
        let mut config = named_config();
        let spec = config
            .workloads
            .get_mut("pi")
            .unwrap()
            .depends_on
            .get_mut("litellm")
            .unwrap();
        spec.env = None;
        spec.exports
            .insert("admin".to_string(), "LITELLM_ADMIN_URL".to_string());

        let err = resolve_depends_on(&config, "pi", &state_dir, &[], "default").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("'admin'"), "error must name the port: {msg}");
        assert!(
            msg.contains("restart it"),
            "error must direct a restart: {msg}"
        );
        assert!(
            !msg.contains("'api'"),
            "error must name the MISSING port, not the present one: {msg}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // (8) `--use <dep>@<id>` + exports: the export resolves from the
    // SELECTED parallel record's named port, not the singleton's.
    #[test]
    fn use_override_selects_parallel_record_for_exports() -> Result<()> {
        let state_dir = unique_state_dir("disc-use-exports");
        // Singleton WITHOUT the named port; the parallel `canary` has it.
        register_singleton(&state_dir, "litellm", loopback(1), 4000, 4000)?;
        check_and_register_sandbox_lifecycle(
            &state_dir,
            "personal-litellm@canary",
            Some("personal"),
            "litellm",
            loopback(2),
            &[14000],
            &[PortMapping {
                host: 14000,
                guest: 4000,
                bind_ip: loopback(2),
                name: Some("api".to_string()),
            }],
            "2026-07-30T00:00:00Z",
            "default",
            None,
        )?;
        let mut config = named_config();
        let spec = config
            .workloads
            .get_mut("pi")
            .unwrap()
            .depends_on
            .get_mut("litellm")
            .unwrap();
        spec.env = None;
        spec.exports
            .insert("api".to_string(), "LITELLM_API_URL".to_string());

        let overrides = vec![("litellm".to_string(), "canary".to_string())];
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &overrides, "default")?;
        assert_eq!(resolved.len(), 1);
        let r = &resolved[0];
        assert_eq!(r.env_var, "LITELLM_API_URL");
        assert_eq!(r.address, "host.microsandbox.internal:14000");
        assert_eq!(r.host_port, 14000);
        assert_eq!(r.port_name.as_deref(), Some("api"));
        assert_eq!(r.source, ResolutionSource::RunningInstance);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- ADR 0030 Phase 2: namespace-scoped resolution ----

    /// Two namespaces, same dep name `litellm` with DIFFERENT ports: the
    /// dependent's namespace selects ITS OWN record (isolation), not the
    /// other namespace's.
    #[test]
    fn resolve_depends_on_filters_by_namespace() -> Result<()> {
        let state_dir = unique_state_dir("disc-ns-isolation");
        // A singleton `litellm` registered by repo-a (port 4000). The
        // registry is keyed by INSTANCE NAME, so two namespaces cannot hold
        // the same singleton slot — the namespace distinguishes which repo
        // registered a record, it does NOT partition the key space (ADR 0030
        // T1 documented limitation; the namespace makes the collision
        // VISIBLE).
        check_and_register_sandbox_lifecycle(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            loopback(1),
            &[4000],
            &[PortMapping {
                host: 4000,
                guest: 4000,
                bind_ip: loopback(1),
                name: None,
            }],
            "2026-07-30T00:00:00Z",
            "repo-a",
            None,
        )?;
        let config = depends_config();

        // Dependent in namespace repo-a resolves ITS record (port 4000).
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "repo-a")?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].host_port, 4000);
        assert_eq!(resolved[0].source, ResolutionSource::RunningInstance);

        // Dependent in namespace repo-b sees NO record in its own namespace —
        // the collision-visibility path: an optional dep warns (stderr) and
        // falls back to the declared port (4000, the convention address).
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "repo-b")?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].host_port, 4000);
        assert_eq!(resolved[0].source, ResolutionSource::DeclaredFallback);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// A legacy record with NO namespace field resolves under "default" (the
    /// `#[serde(default = "default_namespace")]` back-compat).
    #[test]
    fn resolve_depends_on_legacy_record_default_namespace() -> Result<()> {
        let state_dir = unique_state_dir("disc-ns-legacy");
        // register_sandbox (the legacy minimal form) writes namespace "default".
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        let config = depends_config();

        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].address, "host.microsandbox.internal:4000");
        assert_eq!(resolved[0].source, ResolutionSource::RunningInstance);

        // A DIFFERENT namespace sees no record → optional dep falls back to
        // the declared port (with the collision-visibility warning).
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "other-repo")?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].host_port, 4000);
        assert_eq!(resolved[0].source, ResolutionSource::DeclaredFallback);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- ADR 0030 Phase 3: a preferred/increment-chosen effective port
    // flows through exports ----

    /// A singleton litellm record whose EFFECTIVE port is 4001 (simulating a
    /// preferred+increment choice: preferred 4000 was occupied, the chain
    /// chose 4001). The registry record carries the effective port, so
    /// `resolve_depends_on` must inject `host.microsandbox.internal:4001`
    /// into the dependent's env/export — proving the Phase 3 selection flows
    /// through named-port exports automatically.
    #[test]
    fn preferred_chosen_port_flows_through_exports() -> Result<()> {
        let state_dir = unique_state_dir("disc-preferred-port");
        register_singleton(&state_dir, "litellm", loopback(1), 4001, 4000)?;
        let config = depends_config();
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[], "default")?;
        assert_eq!(resolved.len(), 1);
        let r = &resolved[0];
        assert_eq!(r.env_var, "LITELLM_URL");
        assert_eq!(r.address, "host.microsandbox.internal:4001");
        assert_eq!(r.host_port, 4001);
        assert_eq!(r.source, ResolutionSource::RunningInstance);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- ADR 0030 P2.1: resolve_depends_on_full mode-aware DEFAULT
    // selection (the dependent's own parallel instance id) ----

    /// `depends_config()` variant: `pi.depends_on.litellm` carries an
    /// EXPLICIT `instance = scoped|fresh` mode (and `required` per arg).
    fn mode_config(mode: crate::config::DepInstanceMode, required: bool) -> ConfigFile {
        let mut config = depends_config();
        let spec = config
            .workloads
            .get_mut("pi")
            .unwrap()
            .depends_on
            .get_mut("litellm")
            .unwrap();
        spec.instance = Some(mode);
        spec.required = required;
        config
    }

    /// (a) Scoped + Some(id), scoped record present: the record whose
    /// parallel id is `<workload>-<id>` (`litellm@pi-1` for dependent `pi`
    /// on instance `1`) is selected over the singleton.
    #[test]
    fn scoped_record_present_is_selected_over_singleton() -> Result<()> {
        let state_dir = unique_state_dir("disc-scoped-present");
        register_singleton(&state_dir, "litellm", loopback(1), 4000, 4000)?;
        register_parallel(&state_dir, "litellm", "pi-1", loopback(2), 14000, 4000)?;
        let config = mode_config(crate::config::DepInstanceMode::Scoped, false);

        let resolved =
            resolve_depends_on_full(&config, "pi", &state_dir, &[], "default", Some("1"))?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].dep, "litellm");
        assert_eq!(resolved[0].host_port, 14000);
        assert_eq!(resolved[0].address, "host.microsandbox.internal:14000");
        assert_eq!(resolved[0].source, ResolutionSource::RunningInstance);
        assert_eq!(resolved[0].derived_egress_rule().port, 14000);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// (b) Scoped + Some(id), scoped record ABSENT, required: hard error
    /// naming the scoped instance AND its start command (no singleton or
    /// declared-port fallback for a required scoped dep).
    #[test]
    fn scoped_absent_required_refuses_naming_scoped_start_command() -> Result<()> {
        let state_dir = unique_state_dir("disc-scoped-required");
        // A singleton record EXISTS but must not satisfy the scoped request.
        register_singleton(&state_dir, "litellm", loopback(1), 4000, 4000)?;
        let config = mode_config(crate::config::DepInstanceMode::Scoped, true);

        let err = resolve_depends_on_full(&config, "pi", &state_dir, &[], "default", Some("1"))
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("litellm@pi-1"),
            "error must name the scoped instance: {msg}"
        );
        assert!(
            msg.contains("workestrate workload up litellm --instance pi-1"),
            "error must name the scoped start command: {msg}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// (c) Scoped + Some(id), scoped record ABSENT, optional: declared-port
    /// fallback (ResolutionSource::DeclaredFallback on the declared 4000) —
    /// the scoped-aware reason rides the stderr warning (not asserted).
    #[test]
    fn scoped_absent_optional_falls_back_to_declared_port() -> Result<()> {
        let state_dir = unique_state_dir("disc-scoped-optional");
        let config = mode_config(crate::config::DepInstanceMode::Scoped, false);

        let resolved =
            resolve_depends_on_full(&config, "pi", &state_dir, &[], "default", Some("1"))?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].host_port, 4000);
        assert_eq!(resolved[0].address, "host.microsandbox.internal:4000");
        assert_eq!(resolved[0].source, ResolutionSource::DeclaredFallback);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// (d) Scoped + None (singleton dependent): the scoped arm does NOT
    /// fire — singleton selection is unchanged even when a `<dep>@<name>-*`
    /// record exists.
    #[test]
    fn scoped_mode_without_dependent_id_keeps_singleton_selection() -> Result<()> {
        let state_dir = unique_state_dir("disc-scoped-none");
        register_singleton(&state_dir, "litellm", loopback(1), 4000, 4000)?;
        register_parallel(&state_dir, "litellm", "pi-1", loopback(2), 14000, 4000)?;
        let config = mode_config(crate::config::DepInstanceMode::Scoped, false);

        let resolved = resolve_depends_on_full(&config, "pi", &state_dir, &[], "default", None)?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].host_port, 4000);
        assert_eq!(resolved[0].source, ResolutionSource::RunningInstance);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// (e) Fresh WITHOUT --use: singleton selection unchanged even with a
    /// dependent id (the fresh record is only knowable via the injected
    /// --use the auto-start executor returns).
    #[test]
    fn fresh_mode_without_use_keeps_singleton_selection() -> Result<()> {
        let state_dir = unique_state_dir("disc-fresh-no-use");
        register_singleton(&state_dir, "litellm", loopback(1), 4000, 4000)?;
        register_parallel(&state_dir, "litellm", "ab2z", loopback(2), 14000, 4000)?;
        let config = mode_config(crate::config::DepInstanceMode::Fresh, false);

        let resolved =
            resolve_depends_on_full(&config, "pi", &state_dir, &[], "default", Some("1"))?;
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].host_port, 4000);
        assert_eq!(resolved[0].source, ResolutionSource::RunningInstance);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }
}
