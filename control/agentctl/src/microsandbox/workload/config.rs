use super::secrets::{build_env_and_secret_env, build_secret_definitions};
use super::validate::resolve_mount_host_template;
use super::{SandboxCommand, Workload};
use crate::config::WorkloadConfig;
use crate::microsandbox::plan::{EgressRule, EnvVar, HostBoundSecret, NetworkPlan, SandboxPlan};
use anyhow::Result;
use std::path::PathBuf;

/// Resolve the content root for one workload field (`mounts`,
/// `seed_files`): the content dir of the layer that DECLARED the field, per
/// the merge provenance (`workloads.<name>.<field>` → layer name) and the
/// layer-dirs map (layer name → parent dir of the layer file).
///
/// Returns `None` when provenance or the layer's dir is unavailable
/// (synthetic `from_string` layers without a source path); the caller then
/// applies the documented fallback explicitly.
fn field_content_root(
    provenance: Option<&crate::merge::Provenance>,
    layer_dirs: &std::collections::HashMap<String, PathBuf>,
    workload: &str,
    field: &str,
) -> Option<PathBuf> {
    let layer = provenance?.get(&format!("workloads.{workload}.{field}"))?;
    layer_dirs.get(layer).cloned()
}

/// Workload implementation driven by `workestrate.toml`. This replaces the
/// per-agent `workloads/*.rs` modules with a single generic implementation.
#[derive(Debug)]
pub struct ConfigWorkload {
    pub(super) name: String,
    pub(super) workload: WorkloadConfig,
    pub(super) env: Vec<EnvVar>,
    pub(super) secret_env: Vec<HostBoundSecret>,
    pub(super) provenance: Option<crate::merge::Provenance>,
    /// Content root for repo-relative mount hosts: the directory of the
    /// config layer that declared `workloads.<name>.mounts`. Resolved at
    /// construction from provenance + the load-time layer-dirs map.
    pub(super) mount_content_root: Option<PathBuf>,
    /// Content root for seed-file sources and non-state targets: the
    /// directory of the config layer that declared
    /// `workloads.<name>.seed_files`.
    pub(super) seed_content_root: Option<PathBuf>,
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
        let layer_dirs = crate::merge::get_layer_dirs().unwrap_or_default();
        crate::config::validate_config(&config)?;
        let workload = config
            .workloads
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("workload '{}' not found in config", name))?
            .clone();

        // Content roots (spec 17): repo-relative mount hosts and seed-file
        // paths resolve against the DECLARING layer's directory, not the
        // flake project root. Mounts and seed_files merge wholesale-replace,
        // so each field has exactly one declaring layer.
        let mount_content_root =
            field_content_root(provenance.as_ref(), &layer_dirs, name, "mounts");
        let seed_content_root =
            field_content_root(provenance.as_ref(), &layer_dirs, name, "seed_files");

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
            mount_content_root,
            seed_content_root,
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

    /// Resolve the build path, reporting whether the result is the
    /// UNDECLARED reserved default (`.workestrate-build/<name>`, spec 21
    /// §6.1): no configured `env_override` in effect, no declared `fallback`,
    /// and no conventional `WORKESTRATE_<NAME>_BUILD` env var set. The
    /// reserved default resolves declaring-layer-relative and must not
    /// trigger the flake project-root machinery; declared fallbacks and env
    /// overrides keep the pre-reservation behavior unchanged.
    fn resolve_build_path(&self) -> (String, bool) {
        if let Some(ref build) = self.workload.local_build {
            if let Some(ref env_override) = build.env_override {
                if let Ok(p) = std::env::var(env_override) {
                    return (p, false);
                }
            }
            if let Some(ref fallback) = build.fallback {
                return (fallback.clone(), false);
            }
        }
        // Fallback to the workload-trait default env-var convention.
        let key = format!(
            "WORKESTRATE_{}_BUILD",
            self.name.to_ascii_uppercase().replace('-', "_")
        );
        if let Ok(p) = std::env::var(key) {
            return (p, false);
        }
        (format!(".workestrate-build/{}", self.name), true)
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
        if self.workload.seed_files.is_empty() {
            return Ok(());
        }
        // Content root for seed sources and non-state targets: the directory
        // of the config layer that declared this workload's seed_files
        // (spec 17). EXPLICIT FALLBACK: when no declaring layer dir is
        // knowable (synthetic layers), fall back to the flake project root
        // when one resolves. There is NO silent cwd fallback (F3 — removed
        // the old `unwrap_or_else(current_dir)`): if neither is available,
        // hard-error and name the workload.
        let root = match self
            .seed_content_root
            .clone()
            .or_else(crate::config::project_root_optional)
        {
            Some(root) => root,
            None => anyhow::bail!(
                "workload '{}' declares seed_files but no content root can be resolved: \
                 the declaring config layer has no source directory and no flake project \
                 root was found. Set AGENTCTL_ROOT, run from the workbench root, or \
                 declare seed_files from a file-backed config layer.",
                self.name
            ),
        };
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
        self.resolve_build_path().0
    }

    fn build_path_is_reserved_default(&self) -> bool {
        self.resolve_build_path().1
    }

    fn log_stop_errors(&self) -> bool {
        self.workload.log_stop_errors.unwrap_or(true)
    }

    fn mount_content_root(&self) -> Option<PathBuf> {
        self.mount_content_root.clone()
    }

    /// F2 lazy gate: `build_sandbox` calls `project_root()` ONLY when the
    /// workload genuinely needs the flake checkout. The triggers:
    ///
    /// (a) a `nix-layered` image recipe (image build artifacts live in the
    ///     tool flake);
    /// (b) a `local_build` config (recipes — including `flake://` sources —
    ///     execute against the flake checkout);
    /// (c) a mount whose host, after `${WORKESTRATE_<NAME>_BUILD}` template
    ///     substitution in `plan()`, IS the workload's relative build path
    ///     AND that path is a flake-checkout artifact (a declared fallback
    ///     or a relative env override, e.g. `agents/<name>/build`). The
    ///     UNDECLARED reserved default (`.workestrate-build/<name>`, spec 21
    ///     §6.1) resolves declaring-layer-relative and is excluded — it
    ///     needs no flake root.
    ///
    /// Registry-image workloads with none of these return `None`, so
    /// `build_sandbox` never touches the flake-root gate for them.
    fn flake_root_requirement(&self, plan: &SandboxPlan) -> Option<String> {
        if self.workload.image.recipe == "nix-layered" {
            return Some("nix-layered image".to_string());
        }
        if self.workload.local_build.is_some() {
            return Some("local_build config".to_string());
        }
        let (build_path, reserved_default) = self.resolve_build_path();
        if !reserved_default
            && !std::path::Path::new(&build_path).is_absolute()
            && plan.mounts.iter().any(|m| m.host == build_path)
        {
            return Some(format!("relative build-path mount '{build_path}'"));
        }
        None
    }

    /// ConfigWorkload override: check mounts (via the shared core) PLUS seed
    /// sources (resolved against the declaring layer's seed content root) and
    /// the `local_build.fallback` build output. See the trait method doc for
    /// the hard/warn semantics.
    fn preflight_existence(&self, plan: &SandboxPlan, hard: bool) -> Result<Vec<String>> {
        let owned = crate::microsandbox::mounts::resolve_mount_roots_owned(self, plan)?;
        let roots = owned.as_roots();
        let fallback = self
            .workload
            .local_build
            .as_ref()
            .and_then(|b| b.fallback.as_deref());
        crate::microsandbox::mounts::preflight_existence(
            &roots,
            plan,
            &self.workload.seed_files,
            self.seed_content_root.as_deref(),
            fallback,
            &self.name,
            hard,
        )
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
        // Override set → returns the env value (and is NOT the reserved default).
        std::env::set_var("WORKESTRATE_PI_BUILD", "/tmp/test-pi-build");
        assert_eq!(pi.build_path(), "/tmp/test-pi-build");
        assert!(!pi.build_path_is_reserved_default());
        // Override removed → falls back to the reserved default
        // `.workestrate-build/<name>` (spec 21 §6.1).
        std::env::remove_var("WORKESTRATE_PI_BUILD");
        assert_eq!(pi.build_path(), ".workestrate-build/pi");
        assert!(pi.build_path_is_reserved_default());
        Ok(())
    }

    /// Spec 21 §6.1: the reservation changes the DEFAULT, never a
    /// declaration — a declared `local_build.fallback` resolves exactly as
    /// before, and the env override still beats it.
    #[test]
    fn build_path_declared_fallback_unchanged_and_env_beats_it() -> Result<()> {
        let _guard = TestConfigGuard::new();
        // Fixture odysseus declares `fallback = "agents/odysseus/build"`.
        std::env::remove_var("WORKESTRATE_ODYSSEUS_BUILD");
        let odysseus = ConfigWorkload::new("odysseus")?;
        assert_eq!(odysseus.build_path(), "agents/odysseus/build");
        assert!(!odysseus.build_path_is_reserved_default());
        // Env override precedence unchanged: it beats the declared fallback.
        std::env::set_var("WORKESTRATE_ODYSSEUS_BUILD", "/tmp/test-ody-build");
        assert_eq!(odysseus.build_path(), "/tmp/test-ody-build");
        std::env::remove_var("WORKESTRATE_ODYSSEUS_BUILD");
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

        fn config_dir(&self) -> &std::path::Path {
            &self.config_dir
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

    // ---- F1: content roots resolve against the DECLARING layer's dir ----

    /// Pure resolution: provenance key → layer name → layer dir.
    #[test]
    fn field_content_root_follows_provenance_to_layer_dir() {
        let mut provenance = crate::merge::Provenance::new();
        provenance.insert(
            "workloads.litellm.mounts".to_string(),
            "personal#workestrate/workloads/litellm.toml".to_string(),
        );
        let mut dirs = std::collections::HashMap::new();
        dirs.insert(
            "personal#workestrate/workloads/litellm.toml".to_string(),
            PathBuf::from("/store/config-repos/personal/workestrate/workloads"),
        );
        assert_eq!(
            field_content_root(Some(&provenance), &dirs, "litellm", "mounts"),
            Some(PathBuf::from(
                "/store/config-repos/personal/workestrate/workloads"
            ))
        );
        // No provenance → None (caller applies the documented fallback).
        assert_eq!(field_content_root(None, &dirs, "litellm", "mounts"), None);
        // Provenance names a synthetic layer with no dir → None.
        let mut provenance = crate::merge::Provenance::new();
        provenance.insert(
            "workloads.litellm.mounts".to_string(),
            "synthetic".to_string(),
        );
        assert_eq!(
            field_content_root(Some(&provenance), &dirs, "litellm", "mounts"),
            None
        );
    }

    /// Multi-layer pipeline: a directory-mode-style layer re-declaring
    /// `mounts` moves the mount content root to ITS directory-mode root;
    /// `seed_files` declared only by the base layer keep the BASE layer's
    /// directory-mode root. The content root is the `<repo>/workestrate/`
    /// dir (the parent of `workloads/`), NOT the capsule dir or the
    /// `workloads/` dir — so a `host = "workloads/svc"` mount resolves
    /// without doubling. Mirrors the `load_config` merge + `layer_dirs_from`
    /// wiring end to end.
    #[test]
    fn content_roots_track_the_declaring_layer_across_a_merge() -> Result<()> {
        let base_root = std::path::Path::new("/tmp/f1-base-repo/workestrate");
        let capsule_root = std::path::Path::new("/tmp/f1-personal-repo/workestrate");
        let base_dir = base_root.join("workloads");
        let capsule_dir = capsule_root.join("workloads").join("svc");
        let base = crate::merge::Layer::from_string_with_path(
            "base#workestrate/workloads/svc.toml",
            "schema_version = 1\n\n[workloads.svc]\nkind = \"service\"\nimage = { recipe = \"registry\", ref = \"python:3.12-slim\" }\ncommand = []\n\n[[workloads.svc.mounts]]\nhost = \"base-config.yaml\"\nguest = \"/app/cfg\"\nread_only = true\n\n[[workloads.svc.seed_files]]\nsource = \"seed/s.json\"\ntarget = \"workspaces/svc-state/s.json\"\n\n[workloads.svc.network]\ndefault_deny = true",
            Some(base_dir.join("svc.toml")),
        )?;
        let capsule = crate::merge::Layer::from_string_with_path(
            "personal#workestrate/workloads/svc/workload.toml",
            "schema_version = 1\n\n[workloads.svc]\n\n[[workloads.svc.mounts]]\nhost = \"config.yaml\"\nguest = \"/app/cfg\"\nread_only = true",
            Some(capsule_dir.join("workload.toml")),
        )?;

        let dirs = crate::merge::layer_dirs_from(std::slice::from_ref(&base));
        let mut dirs = dirs;
        dirs.extend(crate::merge::layer_dirs_from(std::slice::from_ref(
            &capsule,
        )));
        let (_merged, provenance) = crate::merge::merge_layers(&[base, capsule])?;

        // Mounts: wholesale-replaced by the capsule layer → the capsule
        // layer's directory-mode root (`<repo>/workestrate/`), NOT the
        // capsule dir.
        assert_eq!(
            field_content_root(Some(&provenance), &dirs, "svc", "mounts"),
            Some(capsule_root.to_path_buf())
        );
        // Seed files: declared only by the base layer → the base layer's
        // directory-mode root, NOT the `workloads/` dir.
        assert_eq!(
            field_content_root(Some(&provenance), &dirs, "svc", "seed_files"),
            Some(base_root.to_path_buf())
        );
        Ok(())
    }

    /// End-to-end through `ConfigWorkload::new` (WORKESTRATE_CONFIG_DIR
    /// single layer named "local"): the content root is the config dir
    /// holding the declaring `workestrate.toml`, and `prepare()` seeds from
    /// it — never from project_root or cwd.
    const SEED_CONFIG_TOML: &str = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = ["true"]

[[workloads.svc.mounts]]
host = "config.yaml"
guest = "/app/config.yaml"
read_only = true

[[workloads.svc.seed_files]]
source = "seed/settings.json"
target = "workspaces/svc-state/settings.json"
only_if_missing = true

[workloads.svc.network]
default_deny = true
"#;

    #[test]
    fn new_resolves_content_roots_to_the_declaring_config_dir() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-content-root", SEED_CONFIG_TOML);
        let svc = ConfigWorkload::new("svc")?;
        assert_eq!(
            svc.mount_content_root.as_deref(),
            Some(guard.config_dir()),
            "mount content root is the declaring layer's dir"
        );
        assert_eq!(
            svc.seed_content_root.as_deref(),
            Some(guard.config_dir()),
            "seed content root is the declaring layer's dir"
        );
        // And via the trait surface used by build_sandbox.
        assert_eq!(
            Workload::mount_content_root(&svc).as_deref(),
            Some(guard.config_dir())
        );
        Ok(())
    }

    /// F3: prepare() copies seed files from the DECLARING layer's dir
    /// (colocated seed payload), never silently from cwd.
    #[test]
    fn prepare_seeds_from_the_declaring_layer_dir() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-prepare-seed", SEED_CONFIG_TOML);
        std::fs::create_dir_all(guard.config_dir().join("seed"))?;
        std::fs::write(
            guard.config_dir().join("seed").join("settings.json"),
            "{\"seeded\":true}",
        )?;

        let svc = ConfigWorkload::new("svc")?;
        svc.prepare()?;

        let target = guard.state_dir().join("workspaces/svc-state/settings.json");
        assert_eq!(
            std::fs::read_to_string(&target)?,
            "{\"seeded\":true}",
            "seed payload must come from the declaring layer's dir"
        );
        Ok(())
    }

    /// F3: seed files present but NO content root resolvable (synthetic
    /// workload, no declaring layer dir, no flake project root reachable)
    /// → explicit hard error naming the workload, NEVER a silent cwd
    /// fallback.
    #[test]
    fn prepare_hard_errors_when_seed_content_root_unresolvable() -> Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(
            crate::config::test_support::HOME_ENV_KEYS,
        );
        let old_root = std::env::var("AGENTCTL_ROOT").ok();
        let old_manifest = std::env::var("CARGO_MANIFEST_DIR").ok();

        // Scrub every project_root tier: no AGENTCTL_ROOT, no
        // CARGO_MANIFEST_DIR, cwd = a flake-less temp dir.
        let cwd = crate::config::test_support::uniq_dir("cw-prepare-noroot");
        std::fs::create_dir_all(&cwd)?;
        std::env::remove_var("AGENTCTL_ROOT");
        std::env::remove_var("CARGO_MANIFEST_DIR");
        std::env::set_current_dir(&cwd)?;

        let result = synthetic_workload(SEED_CONFIG_TOML, "svc").prepare();

        match old_root {
            Some(v) => std::env::set_var("AGENTCTL_ROOT", v),
            None => std::env::remove_var("AGENTCTL_ROOT"),
        }
        match old_manifest {
            Some(v) => std::env::set_var("CARGO_MANIFEST_DIR", v),
            None => std::env::remove_var("CARGO_MANIFEST_DIR"),
        }
        let _ = std::fs::remove_dir_all(&cwd);

        let err = result.expect_err("seed files without a content root must hard-error");
        let msg = err.to_string();
        assert!(
            msg.contains("workload 'svc'") && msg.contains("no content root"),
            "error must name the workload and the missing content root: {msg}"
        );
        Ok(())
    }

    /// F3: no seed files → prepare() is a no-op even with no content root.
    #[test]
    fn prepare_without_seed_files_is_a_noop() -> Result<()> {
        let svc = synthetic_workload(MINIMAL_SEEDLESS_TOML, "svc");
        svc.prepare()
    }

    const MINIMAL_SEEDLESS_TOML: &str = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = ["true"]

[workloads.svc.network]
default_deny = true
"#;

    /// Build a ConfigWorkload directly (no config load): provenance and
    /// content roots are None, simulating a synthetic layer stack.
    fn synthetic_workload(toml: &str, name: &str) -> ConfigWorkload {
        let cf: crate::config::ConfigFile =
            toml::from_str(toml).expect("synthetic config must parse");
        let workload = cf.workloads.get(name).expect("workload present").clone();
        ConfigWorkload {
            name: name.to_string(),
            workload,
            env: Vec::new(),
            secret_env: Vec::new(),
            provenance: None,
            mount_content_root: None,
            seed_content_root: None,
            depends_resolved: Vec::new(),
        }
    }

    // ---- F2: the lazy flake-root gate predicate ----

    /// Registry image, no local_build, no build-path mounts → NO
    /// requirement: build_sandbox must never call project_root for
    /// example-litellm.
    #[test]
    fn flake_root_requirement_none_for_plain_registry_workload() -> Result<()> {
        let _guard = TestConfigGuard::new();
        let example_litellm = ConfigWorkload::new("example-litellm")?;
        assert_eq!(
            example_litellm.flake_root_requirement(&example_litellm.plan()),
            None
        );
        Ok(())
    }

    /// nix-layered image recipe → requirement naming the feature.
    #[test]
    fn flake_root_requirement_names_nix_layered_image() -> Result<()> {
        let _guard = TestConfigGuard::new();
        let pi = ConfigWorkload::new("pi")?;
        let req = pi
            .flake_root_requirement(&pi.plan())
            .expect("nix-layered pi must require the flake root");
        assert!(req.contains("nix-layered"), "got: {req}");
        Ok(())
    }

    /// Registry image WITH local_build → requirement naming local_build.
    #[test]
    fn flake_root_requirement_names_local_build() -> Result<()> {
        let _guard = TestConfigGuard::new();
        let odysseus = ConfigWorkload::new("odysseus")?;
        let req = odysseus
            .flake_root_requirement(&odysseus.plan())
            .expect("local_build odysseus must require the flake root");
        assert!(req.contains("local_build"), "got: {req}");
        Ok(())
    }

    /// Case (c): a `${WORKESTRATE_<NAME>_BUILD}` mount requires the flake
    /// root only when the build path is a RELATIVE flake-checkout artifact
    /// (an env override to a relative path). The UNDECLARED reserved default
    /// (`.workestrate-build/<name>`, spec 21 §6.1) resolves
    /// declaring-layer-relative and needs NO flake root.
    #[test]
    fn flake_root_requirement_names_relative_build_path_mount() -> Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let toml = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = ["true"]

[[workloads.svc.mounts]]
host = "${WORKESTRATE_SVC_BUILD}"
guest = "/app"
read_only = true

[workloads.svc.network]
default_deny = true
"#;
        // UNDECLARED reserved default: declaring-layer-relative → no gate.
        std::env::remove_var("WORKESTRATE_SVC_BUILD");
        let svc = synthetic_workload(toml, "svc");
        assert_eq!(svc.build_path(), ".workestrate-build/svc");
        assert_eq!(
            svc.flake_root_requirement(&svc.plan()),
            None,
            "the reserved default resolves declaring-layer-relative; no flake root needed"
        );

        // Env override to a RELATIVE path (flake-checkout artifact, e.g.
        // `agents/svc/build`) → the gate still fires.
        std::env::set_var("WORKESTRATE_SVC_BUILD", "agents/svc/build");
        let svc = synthetic_workload(toml, "svc");
        let req = svc
            .flake_root_requirement(&svc.plan())
            .expect("a relative flake-checkout build-path mount must require the flake root");
        assert!(req.contains("agents/svc/build"), "got: {req}");

        // An ABSOLUTE build path (env override to a store path) needs no root.
        std::env::set_var("WORKESTRATE_SVC_BUILD", "/nix/store/abc-build");
        let svc = synthetic_workload(toml, "svc");
        assert_eq!(svc.flake_root_requirement(&svc.plan()), None);
        std::env::remove_var("WORKESTRATE_SVC_BUILD");
        Ok(())
    }
}
