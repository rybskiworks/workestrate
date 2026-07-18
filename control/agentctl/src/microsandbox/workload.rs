use crate::config::{ConfigFile, WorkloadConfig};
use crate::microsandbox::plan::{EgressRule, EnvVar, HostBoundSecret, NetworkPlan, SandboxPlan};
use crate::microsandbox::secrets::{RemappedSecret, SecretDefinition};
use anyhow::Result;
use std::collections::HashMap;

/// Program and args to exec inside the sandbox via exec_stream.
#[derive(Debug, Clone)]
pub struct SandboxCommand {
    pub binary: String,
    pub arguments: Vec<String>,
}

impl SandboxCommand {
    #[allow(dead_code)]
    pub fn new(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
            arguments: vec![],
        }
    }
    pub fn with_args(binary: impl Into<String>, args: &[&str]) -> Self {
        Self {
            binary: binary.into(),
            arguments: args.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// How to set the sandbox entrypoint.
#[derive(Debug, Clone)]
pub enum EntrypointSpec {
    /// Blocking keep-alive foreground (`/bin/sh -c "tail -f /dev/null"`) that
    /// keeps the sandbox alive while `exec_stream` runs the real service. A bare
    /// `/bin/sh` would run `/bin/sh <image-cmd>` (Docker ENTRYPOINT+CMD) and exit.
    Shell,
}

/// A sandbox workload. The generic `ConfigWorkload` implementation reads from
/// `workestrate.toml`; the lifecycle in `runtime.rs` operates on `&W where W:
/// Workload`.
pub trait Workload: Send + Sync + std::fmt::Debug {
    /// Sandbox name (used for Sandbox::get, logging, user messages).
    fn name(&self) -> &str;

    /// Build the declarative sandbox plan.
    fn plan(&self) -> SandboxPlan;

    /// Program and args to exec inside the sandbox.
    fn exec(&self) -> SandboxCommand;

    /// Args to pass when re-exec'ing in background mode.
    /// The child invokes `<name> up --foreground` so it blocks instead of
    /// re-detaching forever.
    fn detach_args(&self) -> Vec<String> {
        vec![self.name().into(), "up".into(), "--foreground".into()]
    }

    /// Optional pre-start hook (e.g., writing config files to persistent data dir).
    fn prepare(&self) -> Result<()> {
        Ok(())
    }

    /// Whether to log errors when stopping the sandbox (default: true).
    fn log_stop_errors(&self) -> bool {
        true
    }

    /// Sandbox entrypoint (default: Shell).
    fn entrypoint(&self) -> EntrypointSpec {
        EntrypointSpec::Shell
    }

    /// Where to find the built agent code. Defaults to agents/<name>/build;
    /// override per-agent with WORKESTRATE_<NAME>_BUILD (NAME uppercased,
    /// '-' → '_') — used by the nix wrapper to point at a store path.
    #[allow(dead_code)]
    fn build_path(&self) -> String {
        let key = format!(
            "WORKESTRATE_{}_BUILD",
            self.name().to_ascii_uppercase().replace('-', "_")
        );
        if let Ok(p) = std::env::var(key) {
            p
        } else {
            format!("agents/{}/build", self.name())
        }
    }

    /// Convention: config files live at agents/<name>/config/<filename>
    #[allow(dead_code)]
    fn config_path(&self, filename: &str) -> String {
        format!("agents/{}/config/{}", self.name(), filename)
    }
}

/// Resolved secret definition: direct (has its own env_var) or remapped
/// (exposes another secret under a different name).
#[derive(Debug, Clone)]
enum ResolvedSecret {
    Direct(SecretDefinition),
    Remapped(RemappedSecret),
}

/// Workload implementation driven by `workestrate.toml`. This replaces the
/// per-agent `workloads/*.rs` modules with a single generic implementation.
#[derive(Debug)]
pub struct ConfigWorkload {
    name: String,
    workload: WorkloadConfig,
    env: Vec<EnvVar>,
    secret_env: Vec<HostBoundSecret>,
}

impl ConfigWorkload {
    /// Load the active config and construct a workload by name.
    pub fn new(name: &str) -> Result<Self> {
        let config = crate::config::load_config()?;
        crate::config::validate_config(&config)?;
        let workload = config
            .workloads
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("workload '{}' not found in config", name))?
            .clone();

        let secrets = build_secret_definitions(&config)?;
        let env = build_env(&workload, &secrets)?;
        let secret_env = build_secret_env(&workload, &secrets)?;

        Ok(Self {
            name: name.to_string(),
            workload,
            env,
            secret_env,
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

    fn plan(&self) -> SandboxPlan {
        let egress_rules: Vec<EgressRule> = self
            .workload
            .network
            .egress
            .iter()
            .flat_map(crate::recipes::EgressRecipeRef::expand)
            .collect();

        let mut mounts = self.workload.mounts.clone();
        for m in &mut mounts {
            m.host = resolve_mount_host_template(&m.host, self.name(), &self.build_path());
        }

        SandboxPlan {
            name: self.name.clone(),
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
        let root = crate::config::project_root()?;
        for seed in &self.workload.seed_files {
            let source = root.join(&seed.source);
            let target = root.join(&seed.target);
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

fn resolve_mount_host_template(host: &str, name: &str, build_path: &str) -> String {
    if host == "${CWD}" {
        if let Ok(cwd) = std::env::current_dir() {
            return cwd.to_string_lossy().into_owned();
        }
    }
    if let Some(rest) = host.strip_prefix("${CWD}/") {
        if let Ok(cwd) = std::env::current_dir() {
            return format!("{}/{}", cwd.to_string_lossy(), rest);
        }
    }
    let default_build_var = format!(
        "${{WORKESTRATE_{}_BUILD}}",
        name.to_ascii_uppercase().replace('-', "_")
    );
    if host == default_build_var {
        return build_path.to_string();
    }
    host.to_string()
}

fn build_secret_definitions(config: &ConfigFile) -> Result<HashMap<String, ResolvedSecret>> {
    let mut direct: HashMap<String, SecretDefinition> = HashMap::new();
    for (name, secret) in &config.secrets {
        if let Some(ref env_var) = secret.env_var {
            direct.insert(
                name.clone(),
                SecretDefinition {
                    env_var: env_var.clone(),
                    hosts: secret.hosts.clone().unwrap_or_default(),
                    required: secret.required.unwrap_or(true),
                    placeholder: secret.placeholder.clone(),
                    description: String::new(),
                },
            );
        }
    }

    let mut resolved: HashMap<String, ResolvedSecret> = HashMap::new();
    for (name, secret) in &config.secrets {
        if let Some(ref env_var) = secret.env_var {
            let def = direct
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("secret '{}' env_var '{}' missing", name, env_var))?
                .clone();
            resolved.insert(name.clone(), ResolvedSecret::Direct(def));
        } else if let Some(ref source_name) = secret.source {
            let source = direct
                .get(source_name)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "secret '{}' source '{}' not found or not a direct secret",
                        name,
                        source_name
                    )
                })
                .cloned()?;
            let exposed_as = secret
                .exposed_as
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("remapped secret '{}' has no exposed_as", name))?;
            resolved.insert(
                name.clone(),
                ResolvedSecret::Remapped(RemappedSecret {
                    source,
                    exposed_as: exposed_as.to_string(),
                }),
            );
        } else {
            anyhow::bail!("secret '{}' has neither env_var nor source", name);
        }
    }
    Ok(resolved)
}

fn build_env(
    workload: &WorkloadConfig,
    secrets: &HashMap<String, ResolvedSecret>,
) -> Result<Vec<EnvVar>> {
    let mut env = Vec::new();
    for e in &workload.env {
        match (&e.value, &e.secret) {
            (Some(value), None) => env.push(EnvVar::literal(&e.name, value)),
            (None, Some(secret_name)) => {
                let resolved = secrets.get(secret_name).ok_or_else(|| {
                    anyhow::anyhow!(
                        "env '{}' references undefined secret '{}'",
                        e.name,
                        secret_name
                    )
                })?;
                match resolved {
                    ResolvedSecret::Direct(def) => env.push(EnvVar {
                        name: e.name.clone(),
                        value: format!("${{{}}}", def.env_var),
                        is_secret: true,
                        reject_placeholder: def.placeholder.clone(),
                    }),
                    ResolvedSecret::Remapped(_) => {
                        anyhow::bail!(
                            "env '{}' cannot reference remapped secret '{}'",
                            e.name,
                            secret_name
                        )
                    }
                }
            }
            (None, None) => env.push(EnvVar::literal(&e.name, "")),
            (Some(_), Some(_)) => {
                anyhow::bail!("env '{}' cannot have both value and secret", e.name)
            }
        }
    }
    Ok(env)
}

fn build_secret_env(
    workload: &WorkloadConfig,
    secrets: &HashMap<String, ResolvedSecret>,
) -> Result<Vec<HostBoundSecret>> {
    let mut secret_env = Vec::new();
    for se in &workload.secret_env {
        let resolved = secrets.get(&se.secret).ok_or_else(|| {
            anyhow::anyhow!("secret_env references undefined secret '{}'", se.secret)
        })?;
        match resolved {
            ResolvedSecret::Direct(def) => secret_env.push(HostBoundSecret::from(def)),
            ResolvedSecret::Remapped(remap) => secret_env.push(HostBoundSecret::remapped(remap)),
        }
    }
    Ok(secret_env)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn build_path_reads_per_agent_env_override() -> Result<()> {
        let pi = ConfigWorkload::new("pi")?;
        // Override set → returns the env value.
        std::env::set_var("WORKESTRATE_PI_BUILD", "/tmp/test-pi-build");
        assert_eq!(pi.build_path(), "/tmp/test-pi-build");
        // Override removed → falls back to agents/<name>/build.
        std::env::remove_var("WORKESTRATE_PI_BUILD");
        assert_eq!(pi.build_path(), "agents/pi/build");
        Ok(())
    }
}
