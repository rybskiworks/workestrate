use super::secrets::{
    build_env, build_secret_def_name_map, build_secret_definitions, build_secret_env,
};
use super::validate::resolve_mount_host_template;
use super::{SandboxCommand, Workload};
use crate::config::WorkloadConfig;
use crate::microsandbox::plan::{EgressRule, EnvVar, HostBoundSecret, NetworkPlan, SandboxPlan};
use anyhow::Result;
use std::collections::HashMap;

/// Workload implementation driven by `workestrate.toml`. This replaces the
/// per-agent `workloads/*.rs` modules with a single generic implementation.
#[derive(Debug)]
pub struct ConfigWorkload {
    pub(super) name: String,
    pub(super) workload: WorkloadConfig,
    pub(super) env: Vec<EnvVar>,
    pub(super) secret_env: Vec<HostBoundSecret>,
    /// Maps the rendered env/secret_env name (for a remapped secret, the
    /// `exposed_as` name) back to the originating `[secrets.NAME]` definition
    /// name. Merge provenance is recorded under the SECRET DEF NAME
    /// (`workloads.{wl}.secret_env.{SECRET_DEF_NAME}`), while the rendered
    /// `HostBoundSecret.name` is the EXPOSED name — without this map a
    /// remapped secret like LITELLM_AUTH (exposed as OPENAI_API_KEY) looked up
    /// the wrong provenance key and fell back to "core" (WP6(b)/A5).
    pub(super) secret_def_names: HashMap<String, String>,
    pub(super) provenance: Option<crate::merge::Provenance>,
}

impl ConfigWorkload {
    /// Load the active config and construct a workload by name.
    pub fn new(name: &str) -> Result<Self> {
        let config = crate::config::load_config()?;
        let provenance = crate::merge::take_provenance();
        crate::config::validate_config(&config)?;
        let workload = config
            .workloads
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("workload '{}' not found in config", name))?
            .clone();

        let secrets = build_secret_definitions(&config)?;
        let env = build_env(&workload, &secrets)?;
        let secret_env = build_secret_env(&workload, &secrets)?;
        let secret_def_names = build_secret_def_name_map(&workload, &secrets);

        Ok(Self {
            name: name.to_string(),
            workload,
            env,
            secret_env,
            secret_def_names,
            provenance,
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
            env: self.env.clone(),
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
}
