//! Static dependency graph over `[workloads.*.depends_on]` (ADR 0026 addendum
//! 2026-08-01; ADR 0021 addendum 2026-08-01 — compose-mirrored dependency
//! lifecycle, singleton slots only).
//!
//! The W4 compose-mirrored lifecycle needs ORDERING (start a workload's
//! dependencies before the workload itself), which discovery-lite
//! (`discovery.rs`) deliberately does NOT provide: discovery resolves
//! addresses at plan time and REFUSES a required-not-running dep, which is
//! exactly the state W4 fixes by starting the dep. This module therefore
//! builds the graph from the raw public config fields
//! (`ConfigFile.workloads`, `WorkloadConfig.depends_on`) only — never via
//! `discovery::resolve_depends_on`.
//!
//! Provided:
//!
//! - [`dep_closure`]: transitive dependency closure of one workload in
//!   topological START order (every dep appears AFTER all of its own deps).
//! - [`topo_all`]: ALL workloads topo-ordered (deps before dependents),
//!   deterministic via sorted root iteration + sorted dep iteration. The
//!   underlying three-color (white/gray/black) DFS is ALSO the cycle detector
//!   `config::validate_config` calls — one implementation, one error shape:
//!   `dependency cycle detected: a → b → a`.
//! - Running-state helpers against port-registry records
//!   ([`singleton_record`], [`is_slot_running`]): record-as-authoritative,
//!   matching discovery-lite's stance (stale-record nuance is out of scope
//!   for v1). Singleton slots only — a parallel record
//!   (`<ctx>-<wl>@<instance>`) never satisfies a singleton slot.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::config::ConfigFile;
use crate::microsandbox::port_registry::{list_records, SandboxInstanceRecord};
use crate::microsandbox::slots::instance_id_of;

/// Three-color DFS mark: GRAY = on the current path (a back edge to a gray
/// node is a cycle), BLACK = fully explored. Absent from the map = WHITE.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mark {
    Gray,
    Black,
}

/// Shared DFS state. Post-order collection (`out`) yields deps before
/// dependents; deterministic because roots and each workload's deps are
/// visited in sorted name order.
struct Dfs<'a> {
    config: &'a ConfigFile,
    marks: HashMap<&'a str, Mark>,
    stack: Vec<&'a str>,
    out: Vec<&'a str>,
}

impl<'a> Dfs<'a> {
    fn new(config: &'a ConfigFile) -> Self {
        Self {
            config,
            marks: HashMap::new(),
            stack: Vec::new(),
            out: Vec::new(),
        }
    }

    /// Visit `node`: recurse into its deps (sorted by name), then append
    /// `node` to `out` (post-order ⇒ a dep is always emitted before its
    /// dependents). A back edge to a GRAY node is a hard cycle error naming
    /// the cycle path (the gray-stack segment from the first occurrence of
    /// the repeated node through the repeat).
    fn visit(&mut self, node: &'a str) -> Result<()> {
        match self.marks.get(node) {
            Some(Mark::Black) => return Ok(()),
            Some(Mark::Gray) => {
                let start = self.stack.iter().position(|n| *n == node).unwrap_or(0);
                let mut cycle: Vec<&str> = self.stack[start..].to_vec();
                cycle.push(node);
                anyhow::bail!("dependency cycle detected: {}", cycle.join(" → "));
            }
            None => {}
        }
        self.marks.insert(node, Mark::Gray);
        self.stack.push(node);
        if let Some(workload) = self.config.workloads.get(node) {
            let mut deps: Vec<&'a str> = workload.depends_on.keys().map(String::as_str).collect();
            deps.sort_unstable();
            for dep in deps {
                // Undefined deps are a SEPARATE validation error
                // (`validate_config`'s undefined-dep check); treat them as
                // leaves here so the graph stays usable standalone.
                if self.config.workloads.contains_key(dep) {
                    self.visit(dep)?;
                }
            }
        }
        self.stack.pop();
        self.marks.insert(node, Mark::Black);
        self.out.push(node);
        Ok(())
    }
}

/// Transitive dependency closure of `name` (NOT including `name` itself) in
/// topological START order: every dep appears AFTER all of its own deps, so
/// starting them in returned order leaves each dep's own deps already up.
/// Each dep appears exactly once (diamond dedup).
///
/// Errors: unknown `name` → "workload '<name>' not found in config"; cycle →
/// "dependency cycle detected: x → y → x" (defense-in-depth —
/// `validate_config` normally catches cycles first).
pub fn dep_closure(config: &ConfigFile, name: &str) -> Result<Vec<String>> {
    if !config.workloads.contains_key(name) {
        anyhow::bail!("workload '{name}' not found in config");
    }
    let mut dfs = Dfs::new(config);
    dfs.visit(name)?;
    // The root is appended last (post-order, single root); drop it — the
    // closure EXCLUDES `name` itself.
    dfs.out.pop();
    Ok(dfs.out.into_iter().map(str::to_string).collect())
}

/// ALL workloads in the config, topo-ordered (deps before dependents).
/// Deterministic: roots and deps are visited in sorted name order, so the
/// output is stable regardless of `HashMap` iteration order. Same cycle
/// error shape as [`dep_closure`].
pub fn topo_all(config: &ConfigFile) -> Result<Vec<String>> {
    let mut dfs = Dfs::new(config);
    let mut names: Vec<&str> = config.workloads.keys().map(String::as_str).collect();
    names.sort_unstable();
    for name in names {
        dfs.visit(name)?;
    }
    Ok(dfs.out.into_iter().map(str::to_string).collect())
}

/// The registry record for a SINGLETON slot: `instance` equals `slot`
/// exactly (no `@` — a parallel record `<slot>@<instance>` never matches).
pub fn singleton_record<'a>(
    records: &'a [SandboxInstanceRecord],
    slot: &str,
) -> Option<&'a SandboxInstanceRecord> {
    records
        .iter()
        .find(|r| r.instance == slot && instance_id_of(&r.instance).is_none())
}

/// True iff a singleton record for `slot` exists in `list_records(state_dir)`
/// — record-as-authoritative, matching discovery-lite's stance (stale-record
/// nuance is out of scope for v1).
pub fn is_slot_running(state_dir: &Path, slot: &str) -> Result<bool> {
    let records = list_records(state_dir)?;
    Ok(singleton_record(&records, slot).is_some())
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
    use crate::microsandbox::plan::PortMapping;
    use crate::microsandbox::port_registry::check_and_register_sandbox_lifecycle;
    use std::net::{IpAddr, Ipv4Addr};

    /// Chain fixture: a depends_on b, b on c, c on d; d has no deps.
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

[workloads.b.depends_on.c]
env = "C_URL"

[workloads.c]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.c.depends_on.d]
env = "D_URL"

[workloads.d]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []
"#;
        toml::from_str(toml).expect("chain fixture must parse")
    }

    /// Diamond fixture: a → {b, c}, b → d, c → d.
    fn diamond_config() -> ConfigFile {
        let toml = r#"
schema_version = 1

[workloads.a]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.a.depends_on.b]
env = "B_URL"

[workloads.a.depends_on.c]
env = "C_URL"

[workloads.b]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.b.depends_on.d]
env = "D_URL"

[workloads.c]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.c.depends_on.d]
env = "D_URL"

[workloads.d]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []
"#;
        toml::from_str(toml).expect("diamond fixture must parse")
    }

    /// 2-cycle fixture: a → b, b → a.
    fn cycle_config() -> ConfigFile {
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

[workloads.b.depends_on.a]
env = "A_URL"
"#;
        toml::from_str(toml).expect("cycle fixture must parse")
    }

    fn loopback(n: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, n))
    }

    /// Register a singleton lifecycle record for slot `personal-<workload>`
    /// (same shape as the discovery.rs `register_singleton` helper).
    fn register_singleton(state_dir: &Path, workload: &str, host: u16, guest: u16) -> Result<()> {
        let bind = loopback(1);
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

    /// Register a PARALLEL record for `personal-<workload>@<instance>`.
    fn register_parallel(
        state_dir: &Path,
        workload: &str,
        instance: &str,
        host: u16,
        guest: u16,
    ) -> Result<()> {
        let bind = loopback(2);
        check_and_register_sandbox_lifecycle(
            state_dir,
            &format!("personal-{workload}@{instance}"),
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

    #[test]
    fn dep_closure_chain_order_is_start_order() -> Result<()> {
        let config = chain_config();
        let closure = dep_closure(&config, "a")?;
        assert_eq!(closure, vec!["d", "c", "b"]);
        Ok(())
    }

    #[test]
    fn dep_closure_diamond_dedups_and_orders() -> Result<()> {
        let config = diamond_config();
        let closure = dep_closure(&config, "a")?;
        // Each of b, c, d exactly once (diamond dedup).
        let mut sorted = closure.clone();
        sorted.sort();
        assert_eq!(sorted, vec!["b", "c", "d"]);
        // d precedes both b and c (start order).
        let pos = |n: &str| closure.iter().position(|x| x == n).unwrap();
        assert!(pos("d") < pos("b"), "d must precede b: {closure:?}");
        assert!(pos("d") < pos("c"), "d must precede c: {closure:?}");
        Ok(())
    }

    #[test]
    fn dep_closure_unknown_workload_is_a_hard_error() {
        let config = chain_config();
        let err = dep_closure(&config, "ghost").unwrap_err().to_string();
        assert_eq!(err, "workload 'ghost' not found in config");
    }

    #[test]
    fn dep_closure_cycle_errors_with_cycle_message() {
        let config = cycle_config();
        let err = dep_closure(&config, "a").unwrap_err().to_string();
        assert!(
            err.starts_with("dependency cycle detected: "),
            "same shape as validation: {err}"
        );
        assert_eq!(err, "dependency cycle detected: a → b → a");
    }

    #[test]
    fn topo_all_is_deterministic_and_deps_precede_dependents() -> Result<()> {
        // Mixed graph: the diamond fixture (a → {b,c} → d) plus the chain's
        // independent nodes would overlap; the diamond alone exercises both
        // branching and dedup.
        let config = diamond_config();
        let first = topo_all(&config)?;
        for _ in 0..8 {
            let again = topo_all(&config)?;
            assert_eq!(
                again, first,
                "topo_all must be deterministic across runs (HashMap order is random)"
            );
        }
        let pos = |n: &str| first.iter().position(|x| x == n).unwrap();
        assert!(pos("d") < pos("b"));
        assert!(pos("d") < pos("c"));
        assert!(pos("b") < pos("a"));
        assert!(pos("c") < pos("a"));
        assert_eq!(first.len(), 4, "every workload exactly once: {first:?}");
        Ok(())
    }

    #[test]
    fn topo_all_cycle_errors_with_cycle_message() {
        let config = cycle_config();
        let err = topo_all(&config).unwrap_err().to_string();
        assert_eq!(err, "dependency cycle detected: a → b → a");
    }

    #[test]
    fn singleton_record_matches_exact_slot_only() -> Result<()> {
        let state_dir = unique_state_dir("depgraph-singleton");
        register_singleton(&state_dir, "litellm", 4000, 4000)?;
        let records = list_records(&state_dir)?;

        let rec = singleton_record(&records, "personal-litellm")
            .expect("singleton record for the slot must be found");
        assert_eq!(rec.instance, "personal-litellm");
        assert_eq!(rec.workload, "litellm");

        assert!(
            singleton_record(&records, "personal-absent").is_none(),
            "an absent slot must not match"
        );
        assert!(
            singleton_record(&records, "litellm").is_none(),
            "a bare workload name must not match the context-namespaced slot"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn is_slot_running_true_for_singleton_false_for_absent() -> Result<()> {
        let state_dir = unique_state_dir("depgraph-running");
        register_singleton(&state_dir, "litellm", 4000, 4000)?;

        assert!(is_slot_running(&state_dir, "personal-litellm")?);
        assert!(!is_slot_running(&state_dir, "personal-absent")?);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn parallel_record_does_not_satisfy_singleton_slot() -> Result<()> {
        let state_dir = unique_state_dir("depgraph-parallel");
        register_parallel(&state_dir, "litellm", "canary", 4001, 4000)?;
        let records = list_records(&state_dir)?;

        assert!(
            singleton_record(&records, "personal-litellm").is_none(),
            "a parallel record (personal-litellm@canary) must NOT satisfy the singleton slot"
        );
        assert!(
            !is_slot_running(&state_dir, "personal-litellm")?,
            "is_slot_running must be false when only a parallel record exists"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }
}
