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
//! Refusal/fallback matrix (ADR 0026(d), spec 12 §3):
//!
//! | registry view                          | required = true | required = false |
//! |----------------------------------------|-----------------|------------------|
//! | singleton record with ports            | record port     | record port      |
//! | singleton record, NO ports             | declared port + warn | declared port + warn |
//! | no singleton record                    | REFUSE with remediation | declared port + warn |
//! | no record and NO declared ports        | config error    | config error     |
//!
//! Determinism: `depends_on` is iterated SORTED by dependency name (the
//! `HashMap` order is random; plan output must be deterministic).

use std::path::Path;

use anyhow::Result;

use crate::config::ConfigFile;
use crate::microsandbox::plan::{EgressRule, EgressTarget, Protocol};
use crate::microsandbox::port_registry::{list_records_for_workload, SandboxInstanceRecord};

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

/// The plan-time resolution of one declared dependency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedDependency {
    /// Dependency workload name (the `depends_on.<dep>` key).
    pub dep: String,
    /// Env var the resolved address is injected as (the spec's `env`).
    pub env_var: String,
    /// Guest-visible injected address (`host.microsandbox.internal:<port>`).
    pub address: String,
    /// The resolved host port (drives the derived egress rule).
    pub host_port: u16,
    /// Whether the address came from a running record or the declared-port
    /// fallback.
    pub source: ResolutionSource,
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
/// output derived from this list must not depend on `HashMap` order).
///
/// Returns one [`ResolvedDependency`] per declared dep, or a hard error when
/// (a) a `required = true` dependency has no running singleton record (the
/// refusal names the start command), or (b) no address can be derived at all
/// (no running record AND no declared ports).
///
/// Warnings (fallbacks, per-IP binds, port-less records) are emitted on
/// stderr via `eprintln!`, matching the house "WARNING:"/"warning:"
/// conventions.
pub fn resolve_depends_on(
    config: &ConfigFile,
    workload_name: &str,
    state_dir: &Path,
    use_overrides: &[(String, String)],
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
        let records = list_records_for_workload(state_dir, dep)?;
        // Selection (ADR 0026(d)): default = the dependency's SINGLETON slot
        // record (the one whose `instance` name contains no `@`); a `--use
        // <dep>@<instance>` override selects the record whose parallel id
        // matches instead — a PURE selection override with no declared-port
        // fallback when the chosen instance is not running.
        let (selected, selector): (Option<&SandboxInstanceRecord>, String) =
            match use_overrides.iter().find(|(d, _)| d == dep) {
                Some((_, id)) => {
                    let Some(record) = records.iter().find(|r| {
                        crate::microsandbox::slots::instance_id_of(&r.instance) == Some(id.as_str())
                    }) else {
                        anyhow::bail!(
                        "--use {}@{}: no running instance '{}' of dependency '{}' is registered \
                             (start it with `workestrate {} up --instance {}`)",
                        dep,
                        id,
                        id,
                        dep,
                        dep,
                        id
                    );
                    };
                    (Some(record), format!("--use {}@{}", dep, id))
                }
                None => (
                    records.iter().find(|r| {
                        crate::microsandbox::slots::instance_id_of(&r.instance).is_none()
                    }),
                    "singleton".to_string(),
                ),
            };

        match (selected, selector) {
            (Some(record), selector) => match record_host_port(record) {
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
                    resolved.push(ResolvedDependency {
                        dep: dep.clone(),
                        env_var: spec.env.clone(),
                        address: format!("{}:{}", GUEST_HOST_ALIAS, port),
                        host_port: port,
                        source: ResolutionSource::RunningInstance,
                    });
                }
                None => {
                    // A running record with NO ports at all carries no
                    // address: fall back to the declared-port behavior + warn.
                    resolved.push(declared_fallback(
                        config,
                        dep,
                        &spec.env,
                        &format!(
                            "its {} record '{}' publishes no ports",
                            selector, record.instance
                        ),
                    )?);
                }
            },
            (None, _) => {
                if spec.required {
                    anyhow::bail!(
                        "dependency '{}' of workload '{}' is required but not running; start it with `workestrate {} up`",
                        dep,
                        workload_name,
                        dep
                    );
                }
                resolved.push(declared_fallback(
                    config,
                    dep,
                    &spec.env,
                    "no singleton instance is running",
                )?);
            }
        }
    }
    Ok(resolved)
}

/// The declared-port fallback arm: resolve to the dependency's first
/// DECLARED host port on the shared bind (the convention address), warning
/// on stderr. A hard config error when the dependency declares no ports —
/// no address can be derived.
fn declared_fallback(
    config: &ConfigFile,
    dep: &str,
    env_var: &str,
    reason: &str,
) -> Result<ResolvedDependency> {
    match declared_host_port(config, dep) {
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
            })
        }
        None => anyhow::bail!(
            "dependency '{}' cannot be resolved: it is not running and declares no ports \
             (no address can be derived). Declare at least one host port on workload '{}' \
             or start it with `workestrate {} up`.",
            dep,
            dep,
            dep
        ),
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
            }],
            "2026-07-30T00:00:00Z",
        )
    }

    // (1) Singleton resolution: a running singleton record supplies the
    // published host port; the injected address is the guest form.
    #[test]
    fn singleton_record_resolves_to_guest_form_address() -> Result<()> {
        let state_dir = unique_state_dir("disc-singleton");
        register_singleton(&state_dir, "litellm", loopback(1), 4000, 4000)?;
        let config = depends_config();

        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[])?;
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

        let err = resolve_depends_on(&config, "pi", &state_dir, &[]).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("is required but not running; start it with"),
            "refusal must carry the remediation lead: {msg}"
        );
        assert!(
            msg.contains("workestrate litellm up"),
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

        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[])?;
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
                env: "NOPORTS_URL".to_string(),
                required: false,
            },
        );

        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[])?;
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
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[])?;

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
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[])?;

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

        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[])?;
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
                env: "AAA_URL".to_string(),
                required: false,
            },
        );
        // Give the `litellm` workload an alias `aaa` with its own declared
        // port by renaming: simplest is to add an `aaa` workload.
        let litellm = config.workloads.get("litellm").unwrap().clone();
        config.workloads.insert("aaa".to_string(), litellm);

        for _ in 0..8 {
            let resolved = resolve_depends_on(&config, "pi", &state_dir, &[])?;
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
                env: "NOPORTS_URL".to_string(),
                required: false,
            },
        );

        let err = resolve_depends_on(&config, "pi", &state_dir, &[]).unwrap_err();
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
                env: "NOPORTS_URL".to_string(),
                required: false,
            },
        );
        // `noports` declares no ports either → the fallback itself errors.
        let err = resolve_depends_on(&config, "pi", &state_dir, &[]).unwrap_err();
        assert!(err.to_string().contains("declares no ports"));

        // Now give `noports` a declared port: the port-less RECORD falls
        // back to the declared port and resolution succeeds.
        config
            .workloads
            .get_mut("noports")
            .unwrap()
            .ports
            .push(PortMapping::new(9100, 9100));
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[])?;
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
            }],
            "2026-07-30T00:00:00Z",
        )?;
        let config = depends_config();

        // No singleton → optional dep falls back to the DECLARED port (not
        // the parallel record's 14000).
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[])?;
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
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[])?;
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
            }],
            "2026-07-30T00:00:00Z",
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
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &[])?;
        assert_eq!(resolved[0].host_port, 4000);
        assert_eq!(resolved[0].address, "host.microsandbox.internal:4000");
        assert_eq!(resolved[0].source, ResolutionSource::RunningInstance);

        // `--use litellm@canary`: the parallel record's port (14000) is
        // injected — the bind is per-IP, so this ALSO exercises the
        // DEFERRED-PENDING-E1 warning arm (stderr; not asserted here).
        let overrides = vec![("litellm".to_string(), "canary".to_string())];
        let resolved = resolve_depends_on(&config, "pi", &state_dir, &overrides)?;
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
        let err = resolve_depends_on(&config, "pi", &state_dir, &overrides).unwrap_err();
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
        let err = resolve_depends_on(&config, "pi", &state_dir, &overrides).unwrap_err();
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
        let err = resolve_depends_on(&config, "pi", &state_dir, &overrides).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("declares no depends_on entries"),
            "error must explain the workload has no depends_on: {msg}"
        );
        assert!(msg.contains("litellm"), "error must name the dep: {msg}");
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }
}
