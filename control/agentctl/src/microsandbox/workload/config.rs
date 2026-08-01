use super::secrets::{build_env_and_secret_env, build_secret_definitions};
use super::validate::resolve_mount_host_template;
use super::{SandboxCommand, Workload};
use crate::config::WorkloadConfig;
use crate::microsandbox::plan::{EgressRule, EnvVar, HostBoundSecret, NetworkPlan, SandboxPlan};
use anyhow::Result;

/// Workload implementation driven by `workestrate.toml`. This replaces the
/// per-agent `workloads/*.rs` modules with a single generic implementation.
#[derive(Debug)]
pub struct ConfigWorkload {
    pub(super) name: String,
    pub(super) workload: WorkloadConfig,
    pub(super) env: Vec<EnvVar>,
    pub(super) secret_env: Vec<HostBoundSecret>,
    pub(super) provenance: Option<crate::merge::Provenance>,
    /// ADR 0026(d) discovery-lite: depends_on resolutions computed at
    /// construction (declaration triggers resolution on every up/exec/plan
    /// path). `plan()` appends the injected env AFTER the declared env and
    /// the derived egress AFTER the expanded declared egress rules.
    pub(super) depends_resolved: Vec<crate::microsandbox::discovery::ResolvedDependency>,
}

impl ConfigWorkload {
    /// Load the active config and construct a workload by name.
    pub fn new(name: &str) -> Result<Self> {
        Self::new_with_use_overrides(name, &[])
    }

    /// Load the active config and construct a workload by name, applying the
    /// typed `--use <dep>@<instance>` instance-selection overrides
    /// (`(dep, instance-id)` pairs; ADR 0026(d)) to depends_on resolution.
    ///
    /// The overrides are validated INSIDE
    /// [`crate::microsandbox::discovery::resolve_depends_on`] against this
    /// workload's DECLARED depends_on map: an override naming an undeclared
    /// dep (or any override with no depends_on at all) is a hard error here,
    /// so every up/exec/plan path refuses identically.
    pub fn new_with_use_overrides(name: &str, use_overrides: &[(String, String)]) -> Result<Self> {
        let config = crate::config::load_config()?;
        let provenance = crate::merge::take_provenance();
        crate::config::validate_config(&config)?;
        let workload = config
            .workloads
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("workload '{}' not found in config", name))?
            .clone();

        // ADR 0026(d): a declared depends_on map resolves EVERY declared dep
        // at plan time — no flag. A required-but-not-running dep refuses
        // here, on every up/exec/plan path (they all construct via `new`).
        let state_dir = crate::config::resolve_state_dir();
        let depends_resolved = crate::microsandbox::discovery::resolve_depends_on(
            &config,
            name,
            &state_dir,
            use_overrides,
        )?;

        let secrets = build_secret_definitions(&config)?;
        let (env, secret_env) = build_env_and_secret_env(&workload, &secrets)?;

        Ok(Self {
            name: name.to_string(),
            workload,
            env,
            secret_env,
            provenance,
            depends_resolved,
        })
    }

    /// Return the configured kind ("service" or "agent").
    pub fn kind(&self) -> &str {
        &self.workload.kind
    }

    fn resolve_image(&self) -> Option<String> {
        match self.workload.image.recipe.as_str() {
            "registry" => self.workload.image.reference.clone(),
            "nix-layered" => {
                let name = self.workload.image.name.as_deref().unwrap_or("");
                let tag = self.workload.image.tag.as_deref().unwrap_or("latest");
                Some(format!("{}:{}", name, tag))
            }
            _ => None,
        }
    }
}

impl Workload for ConfigWorkload {
    fn name(&self) -> &str {
        &self.name
    }

    fn sandbox_instance_name(&self) -> String {
        match crate::config::active_context_name() {
            Some(ctx) => format!("{}-{}", ctx, self.name),
            None => self.name.clone(),
        }
    }

    fn plan(&self) -> SandboxPlan {
        let egress_rules: Vec<EgressRule> = self
            .workload
            .network
            .egress
            .iter()
            .flat_map(crate::recipes::EgressRecipeRef::expand)
            .collect();

        // ADR 0026(d): apply the depends_on resolution — injected env AFTER
        // declared env (declared wins on a name conflict; skipped inside),
        // derived egress AFTER the expanded declared rules (identical rules
        // deduped inside). Derivation only ADDS: default_deny is untouched
        // (monotonic; FS-16 entitlement check untouched), and the derived
        // rules land in `egress_rules` so `network_plan_to_policy` consumes
        // them identically to declared egress.
        let (injected_env, derived_egress) = crate::microsandbox::discovery::apply_resolution(
            &self.depends_resolved,
            &self.env,
            &egress_rules,
        );
        let mut env = self.env.clone();
        env.extend(injected_env);
        let mut egress_rules = egress_rules;
        egress_rules.extend(derived_egress);

        let mut mounts = self.workload.mounts.clone();
        // WP6(c)/A6: a configured local_build.env_override is ALSO honored as
        // a mount-host template token (checked before the default convention).
        let env_override = self
            .workload
            .local_build
            .as_ref()
            .and_then(|b| b.env_override.as_deref());
        for m in &mut mounts {
            m.host =
                resolve_mount_host_template(&m.host, self.name(), &self.build_path(), env_override);
        }

        SandboxPlan {
            name: self.sandbox_instance_name(),
            image: self.resolve_image(),
            workdir: self.workload.workdir.clone(),
            command: self.workload.command.clone(),
            cpus: self.workload.cpus,
            memory_mib: self.workload.memory_mib,
            env,
            secret_env: self.secret_env.clone(),
            ports: self.workload.ports.clone(),
            mounts,
            network: NetworkPlan {
                default_deny: self.workload.network.default_deny.unwrap_or(true),
                egress_rules,
                deny_rules: self.workload.network.deny.clone(),
                ingress_rules: self.workload.network.ingress.clone(),
            },
        }
    }

    fn show_source(&self) -> String {
        self.show_source_render()
    }

    fn exec(&self) -> SandboxCommand {
        match self.workload.command.split_first() {
            Some((binary, args)) => {
                let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                SandboxCommand::with_args(binary, &args)
            }
            None => SandboxCommand::with_args("", &[]),
        }
    }

    fn prepare(&self) -> Result<()> {
        let root = crate::config::project_root()
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
        let state_dir = crate::config::resolve_state_dir();
        for seed in &self.workload.seed_files {
            let source = root.join(&seed.source);
            let target =
                if seed.target.starts_with("workspaces/") || seed.target.starts_with("var/") {
                    state_dir.join(&seed.target)
                } else {
                    root.join(&seed.target)
                };
            if seed.only_if_missing.unwrap_or(true) && target.exists() {
                continue;
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&source, &target).map_err(|e| {
                anyhow::anyhow!(
                    "failed to seed {} to {}: {}",
                    source.display(),
                    target.display(),
                    e
                )
            })?;
        }
        Ok(())
    }

    fn build_path(&self) -> String {
        if let Some(ref build) = self.workload.local_build {
            if let Some(ref env_override) = build.env_override {
                if let Ok(p) = std::env::var(env_override) {
                    return p;
                }
            }
            if let Some(ref fallback) = build.fallback {
                return fallback.clone();
            }
        }
        // Fallback to the workload-trait default env-var convention.
        let key = format!(
            "WORKESTRATE_{}_BUILD",
            self.name.to_ascii_uppercase().replace('-', "_")
        );
        if let Ok(p) = std::env::var(key) {
            return p;
        }
        format!("agents/{}/build", self.name)
    }

    fn log_stop_errors(&self) -> bool {
        self.workload.log_stop_errors.unwrap_or(true)
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
    use crate::config::test_support::TestConfigGuard;
    use crate::microsandbox::plan::EgressTarget;

    #[test]
    fn build_path_reads_per_agent_env_override() -> Result<()> {
        let _guard = TestConfigGuard::new();
        let pi = ConfigWorkload::new("pi")?;
        // Override set → returns the env value.
        std::env::set_var("WORKESTRATE_PI_BUILD", "/tmp/test-pi-build");
        assert_eq!(pi.build_path(), "/tmp/test-pi-build");
        // Override removed → falls back to agents/<name>/build.
        std::env::remove_var("WORKESTRATE_PI_BUILD");
        assert_eq!(pi.build_path(), "agents/pi/build");
        Ok(())
    }

    #[test]
    fn sandbox_instance_name_bare_when_no_context() -> Result<()> {
        let _guard = TestConfigGuard::new();
        // TestConfigGuard sets WORKESTRATE_CONFIG_DIR which bypasses the registry,
        // so active_context_name() is None.
        crate::config::set_active_context(None);
        let pi = ConfigWorkload::new("pi")?;
        assert_eq!(pi.sandbox_instance_name(), "pi");
        assert_eq!(pi.name(), "pi");
        Ok(())
    }

    #[test]
    fn sandbox_instance_namespaced_when_context_active() -> Result<()> {
        let _guard = TestConfigGuard::new();
        let pi = ConfigWorkload::new("pi")?;
        // Simulate an active context after creating the workload; load_config()
        // resets the active context when WORKESTRATE_CONFIG_DIR is set.
        crate::config::set_active_context(Some(crate::config::ActiveContext {
            name: Some("personal".to_string()),
            layers: vec!["personal".to_string()],
        }));
        assert_eq!(pi.sandbox_instance_name(), "personal-pi");
        assert_eq!(pi.name(), "pi"); // bare name unchanged for CLI dispatch
                                     // Clean up
        crate::config::set_active_context(None);
        Ok(())
    }

    // ---- ADR 0026(d): depends_on resolution fires inside ConfigWorkload ----

    /// Minimal two-workload config: `pi` depends on `litellm` (4000:4000).
    /// The caller adjusts `required` per test.
    const DEPENDS_CONFIG_TOML: &str = r#"
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
"#;

    /// RAII guard: point `WORKESTRATE_CONFIG_DIR` at a temp dir holding a
    /// `workestrate.toml` with `content`, and `WORKESTRATE_STATE_DIR` at a
    /// temp dir acting as the (initially empty) port-registry state home.
    /// Holds the global env lock; both vars are removed on drop.
    struct DependsEnvGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        config_dir: std::path::PathBuf,
        state_dir: std::path::PathBuf,
    }

    impl DependsEnvGuard {
        fn new(label: &str, content: &str) -> Self {
            let lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
            let config_dir = crate::config::test_support::unique_state_dir(label);
            let state_dir = crate::config::test_support::unique_state_dir(label);
            std::fs::create_dir_all(&config_dir).expect("create temp config dir");
            std::fs::create_dir_all(&state_dir).expect("create temp state dir");
            std::fs::write(config_dir.join("workestrate.toml"), content)
                .expect("write temp workestrate.toml");
            std::env::set_var("WORKESTRATE_CONFIG_DIR", &config_dir);
            std::env::set_var("WORKESTRATE_STATE_DIR", &state_dir);
            Self {
                _lock: lock,
                config_dir,
                state_dir,
            }
        }

        fn state_dir(&self) -> &std::path::Path {
            &self.state_dir
        }
    }

    impl Drop for DependsEnvGuard {
        fn drop(&mut self) {
            std::env::remove_var("WORKESTRATE_CONFIG_DIR");
            std::env::remove_var("WORKESTRATE_STATE_DIR");
            let _ = std::fs::remove_dir_all(&self.config_dir);
            let _ = std::fs::remove_dir_all(&self.state_dir);
        }
    }

    fn register_litellm_singleton(state_dir: &std::path::Path) -> Result<()> {
        crate::microsandbox::port_registry::check_and_register_sandbox_lifecycle(
            state_dir,
            "litellm",
            None,
            "litellm",
            crate::microsandbox::plan::default_bind_ip(),
            &[4000],
            &[crate::microsandbox::plan::PortMapping::new(4000, 4000)],
            "2026-07-30T00:00:00Z",
        )
    }

    /// ConfigWorkload::new resolves declared deps and plan() injects the
    /// guest-form address env + derives the egress rule, marked.
    #[test]
    fn new_resolves_depends_on_and_plan_injects_env_and_egress() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-inject", DEPENDS_CONFIG_TOML);
        register_litellm_singleton(guard.state_dir())?;

        let pi = ConfigWorkload::new("pi")?;
        let plan = pi.plan();

        let injected = plan
            .env
            .iter()
            .find(|e| e.name == "LITELLM_URL")
            .expect("plan env must carry the injected LITELLM_URL");
        assert_eq!(injected.value, "host.microsandbox.internal:4000");
        assert!(!injected.is_secret, "the injected URL is not a secret");
        assert_eq!(injected.injected_by.as_deref(), Some("litellm"));
        // The plan Display marks the injection.
        assert!(
            format!("{plan}").contains(
                "env: LITELLM_URL=host.microsandbox.internal:4000 (injected: depends_on 'litellm')"
            ),
            "plan render must mark the injected env; got:\n{plan}"
        );

        let derived: Vec<_> = plan
            .network
            .egress_rules
            .iter()
            .filter(|r| r.derived_from.as_deref() == Some("litellm"))
            .collect();
        assert_eq!(derived.len(), 1, "exactly one derived egress rule");
        assert_eq!(derived[0].port, 4000);
        assert_eq!(derived[0].target, EgressTarget::Host);
        assert!(
            format!("{plan}")
                .contains("  egress: tcp:4000 -> host (derived: depends_on 'litellm')"),
            "plan render must mark the derived egress; got:\n{plan}"
        );
        // Monotonic: derivation only ADDED; default_deny untouched.
        assert!(plan.network.default_deny);
        Ok(())
    }

    /// Optional dep with NO running record: resolution falls back to the
    /// declared port; the injection still lands in the plan.
    #[test]
    fn new_falls_back_to_declared_port_when_dep_not_running() -> Result<()> {
        let _guard = DependsEnvGuard::new("cw-fallback", DEPENDS_CONFIG_TOML);
        let pi = ConfigWorkload::new("pi")?;
        let plan = pi.plan();
        let injected = plan
            .env
            .iter()
            .find(|e| e.name == "LITELLM_URL")
            .expect("fallback injection still lands in the plan");
        assert_eq!(injected.value, "host.microsandbox.internal:4000");
        Ok(())
    }

    /// Required dep with NO running record: ConfigWorkload::new REFUSES —
    /// every up/exec/plan path constructs through `new`, so the refusal
    /// propagates on all of them.
    #[test]
    fn new_refuses_when_required_dep_not_running() -> Result<()> {
        let required_toml = DEPENDS_CONFIG_TOML.replace(
            "env = \"LITELLM_URL\"",
            "env = \"LITELLM_URL\"\nrequired = true",
        );
        let _guard = DependsEnvGuard::new("cw-refuse", &required_toml);
        let err = ConfigWorkload::new("pi").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("is required but not running; start it with"),
            "refusal must carry the remediation lead: {msg}"
        );
        assert!(
            msg.contains("workestrate litellm up"),
            "refusal must name the start command: {msg}"
        );
        Ok(())
    }

    /// The reference/test fixtures declare NO depends_on → construction is
    /// unaffected and the plan carries no injected/derived markers.
    #[test]
    fn plan_without_depends_on_is_unaffected() -> Result<()> {
        let _guard = TestConfigGuard::new();
        let pi = ConfigWorkload::new("pi")?;
        let plan = pi.plan();
        assert!(
            plan.env.iter().all(|e| e.injected_by.is_none()),
            "no env may be marked injected without depends_on"
        );
        assert!(
            plan.network
                .egress_rules
                .iter()
                .all(|r| r.derived_from.is_none()),
            "no egress may be marked derived without depends_on"
        );
        Ok(())
    }

    // ---- ADR 0026(d)/C3-W2: new_with_use_overrides ----

    /// A singleton AND a parallel record exist; `new` selects the singleton
    /// while `new_with_use_overrides([("litellm","canary")])` injects the
    /// PARALLEL record's port (guest form) into the plan env.
    #[test]
    fn new_with_use_overrides_selects_the_parallel_record() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-use", DEPENDS_CONFIG_TOML);
        register_litellm_singleton(guard.state_dir())?;
        crate::microsandbox::port_registry::check_and_register_sandbox_lifecycle(
            guard.state_dir(),
            "litellm@canary",
            None,
            "litellm",
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 2)),
            &[14000],
            &[crate::microsandbox::plan::PortMapping::new(14000, 4000)],
            "2026-07-30T00:00:00Z",
        )?;

        let pi = ConfigWorkload::new("pi")?;
        let injected = pi
            .plan()
            .env
            .iter()
            .find(|e| e.name == "LITELLM_URL")
            .expect("default injection present")
            .value
            .clone();
        assert_eq!(injected, "host.microsandbox.internal:4000");

        let pi = ConfigWorkload::new_with_use_overrides(
            "pi",
            &[("litellm".to_string(), "canary".to_string())],
        )?;
        let plan = pi.plan();
        let injected = plan
            .env
            .iter()
            .find(|e| e.name == "LITELLM_URL")
            .expect("override injection present");
        assert_eq!(injected.value, "host.microsandbox.internal:14000");
        let derived: Vec<_> = plan
            .network
            .egress_rules
            .iter()
            .filter(|r| r.derived_from.as_deref() == Some("litellm"))
            .collect();
        assert_eq!(derived.len(), 1);
        assert_eq!(derived[0].port, 14000, "egress follows the selected record");
        Ok(())
    }

    /// An override naming a dep the workload does NOT declare refuses at
    /// construction — every up/exec/plan path surfaces the same hard error.
    #[test]
    fn new_with_use_overrides_refuses_undeclared_dep() -> Result<()> {
        let _guard = DependsEnvGuard::new("cw-use-undeclared", DEPENDS_CONFIG_TOML);
        let err = ConfigWorkload::new_with_use_overrides(
            "pi",
            &[("redis".to_string(), "canary".to_string())],
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("redis"), "error must name the dep: {msg}");
        assert!(
            msg.contains("not a declared depends_on entry"),
            "error must explain --use only overrides DECLARED deps: {msg}"
        );
        Ok(())
    }

    /// An unknown selected instance refuses at construction, naming dep +
    /// instance.
    #[test]
    fn new_with_use_overrides_refuses_unknown_instance() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-use-ghost", DEPENDS_CONFIG_TOML);
        register_litellm_singleton(guard.state_dir())?;
        let err = ConfigWorkload::new_with_use_overrides(
            "pi",
            &[("litellm".to_string(), "ghost".to_string())],
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("litellm") && msg.contains("ghost"));
        Ok(())
    }
}
