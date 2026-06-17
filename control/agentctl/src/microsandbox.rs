use std::fmt;
use std::path::Path;

use anyhow::Result;
use microsandbox::{
    sandbox::{SandboxBuilder, SandboxHandle, SandboxStatus},
    NetworkPolicy, Sandbox,
};

fn env_var(name: &str) -> Result<String> {
    std::env::var(name).map_err(|e| anyhow::anyhow!("missing env var {}: {}", name, e))
}

/// Validate that all required environment variables are set before trying to
/// start a sandbox.
///
/// A variable is treated as missing if it is unset, empty, or contains only
/// whitespace. The `LITELLM_MASTER_KEY` value is additionally rejected if it
/// matches the committed placeholder from `.env.example`.
///
/// The `context` label is included verbatim in the error message and should be
/// the user-facing command name (e.g. `"agentctl litellm up"`).
fn require_env_vars(context: &str, names: &[&str]) -> Result<()> {
    const LITELLM_MASTER_KEY_PLACEHOLDER: &str = "sk-change-me-local-only";

    let mut missing = Vec::new();
    for name in names {
        let value = std::env::var(name).unwrap_or_default();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            missing.push(*name);
            continue;
        }
        if *name == "LITELLM_MASTER_KEY" && trimmed == LITELLM_MASTER_KEY_PLACEHOLDER {
            return Err(anyhow::anyhow!(
                "LITELLM_MASTER_KEY is set to the placeholder value from .env.example.\n\
                 Replace it with a real secret (e.g. via with-secrets) before running {}",
                context
            ));
        }
    }
    if !missing.is_empty() {
        return Err(anyhow::anyhow!(
            "missing secrets for {}: {}\nSet them in the environment or run via with-secrets",
            context,
            missing.join(", ")
        ));
    }
    Ok(())
}

fn resolve_secret_value(templated: &str) -> Result<String> {
    let mut result = templated.to_string();
    let mut search_from = 0;
    while let Some(start) = templated[search_from..].find("${") {
        let absolute = search_from + start;
        match templated[absolute + 2..].find('}') {
            Some(end) => {
                let var_name = &templated[absolute + 2..absolute + 2 + end];
                if var_name.is_empty() {
                    anyhow::bail!(
                        "empty variable name in secret template at position {}",
                        absolute
                    );
                }
                let value = env_var(var_name)?;
                result = result.replace(&format!("${{{}}}", var_name), &value);
                search_from = absolute + 2 + end + 1;
            }
            None => anyhow::bail!("unclosed ${{ in secret template at position {}", absolute,),
        }
    }
    Ok(result)
}

fn apply_plan_mounts(builder: SandboxBuilder, root: &Path, plan: &SandboxPlan) -> SandboxBuilder {
    let mut b = builder;
    for m in &plan.mounts {
        let host = root.join(&m.host);
        b = b.volume(&m.guest, |v| {
            let v = v.bind(host);
            if m.read_only {
                v.readonly()
            } else {
                v
            }
        });
    }
    b
}

fn apply_plan_secrets(builder: SandboxBuilder, plan: &SandboxPlan) -> Result<SandboxBuilder> {
    let mut b = builder;
    for s in &plan.secret_env {
        let value = resolve_secret_value(&s.value)?;
        b = b.secret_env(&s.var, value, &s.allowed_host);
    }
    Ok(b)
}

fn apply_plan_envs(builder: SandboxBuilder, plan: &SandboxPlan) -> Result<SandboxBuilder> {
    let mut b = builder;
    for e in &plan.env {
        let value = resolve_secret_value(&e.value)?;
        b = b.env(&e.key, value);
    }
    Ok(b)
}

async fn stop_and_remove(handle: SandboxHandle) -> Result<()> {
    match handle.status() {
        SandboxStatus::Running | SandboxStatus::Draining | SandboxStatus::Paused => {
            if let Err(e) = handle.stop().await {
                eprintln!("stop failed ({}), attempting kill", e);
                handle.kill().await?;
            }
        }
        _ => {}
    }
    handle.remove().await?;
    Ok(())
}

fn ensure_mount_sources(root: &Path, plan: &SandboxPlan) -> Result<()> {
    for m in &plan.mounts {
        let path = root.join(&m.host);
        if !path.exists() {
            if m.read_only {
                anyhow::bail!("mount source does not exist: {}", path.display());
            } else {
                std::fs::create_dir_all(&path)?;
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct SandboxPlan {
    pub name: String,
    pub image: Option<String>,
    pub workdir: Option<String>,
    pub command: Vec<String>,
    pub cpus: Option<u8>,
    pub memory_mib: Option<u32>,
    pub env: Vec<EnvVar>,
    pub secret_env: Vec<SecretEnv>,
    pub ports: Vec<PortMapping>,
    pub mounts: Vec<MountPlan>,
    pub network: NetworkPlan,
}

#[derive(Debug, Clone)]
pub struct PortMapping {
    pub host: u16,
    pub guest: u16,
}

#[derive(Debug, Clone)]
pub struct MountPlan {
    pub host: String,
    pub guest: String,
    pub read_only: bool,
}

#[derive(Debug, Clone)]
pub struct SecretEnv {
    pub var: String,
    pub value: String,
    pub allowed_host: String,
}

#[derive(Debug, Clone)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
    pub is_secret: bool,
}

impl fmt::Display for EnvVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = if self.is_secret {
            "(redacted)"
        } else {
            self.value.as_str()
        };
        write!(f, "{}={}", self.key, value)
    }
}

#[derive(Debug, Clone)]
pub struct NetworkPlan {
    pub default_deny: bool,
    pub egress_rules: Vec<EgressRule>,
    pub deny_rules: Vec<DenyDomainRule>,
}

#[derive(Debug, Clone)]
pub struct EgressRule {
    pub protocol: String,
    pub port: Option<u16>,
    pub allow_hosts: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct DenyDomainRule {
    pub domain_suffix: String,
}

impl fmt::Display for SandboxPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "name: {}", self.name)?;
        if let Some(img) = &self.image {
            writeln!(f, "image: {}", img)?;
        }
        if let Some(wd) = &self.workdir {
            writeln!(f, "workdir: {}", wd)?;
        }
        if !self.command.is_empty() {
            writeln!(f, "command: {}", self.command.join(" "))?;
        }
        if let Some(cpus) = self.cpus {
            writeln!(f, "cpus: {}", cpus)?;
        }
        if let Some(mem) = self.memory_mib {
            writeln!(f, "memory: {} MiB", mem)?;
        }
        for e in &self.env {
            writeln!(f, "env: {}", e)?;
        }
        for se in &self.secret_env {
            writeln!(
                f,
                "secret_env: {} (value redacted, allowed: {})",
                se.var, se.allowed_host
            )?;
        }
        for p in &self.ports {
            writeln!(f, "port: {}:{}", p.host, p.guest)?;
        }
        for m in &self.mounts {
            let ro = if m.read_only { " (ro)" } else { "" };
            writeln!(f, "mount: {}:{}{}", m.host, m.guest, ro)?;
        }
        writeln!(f, "network: default_deny={}", self.network.default_deny)?;
        for rule in &self.network.egress_rules {
            let port = rule.port.map(|p| format!(":{}", p)).unwrap_or_default();
            let hosts = rule.allow_hosts.join(", ");
            writeln!(f, "  egress: {}{} -> {}", rule.protocol, port, hosts)?;
        }
        for rule in &self.network.deny_rules {
            writeln!(f, "  egress: deny domain suffix {}", rule.domain_suffix)?;
        }
        Ok(())
    }
}

pub fn build_litellm_plan() -> SandboxPlan {
    SandboxPlan {
        name: "litellm".into(),
        image: Some("ghcr.io/berriai/litellm:main-stable".into()),
        workdir: Some("/app".into()),
        command: vec![
            "litellm".into(),
            "--config".into(),
            "/app/config.yaml".into(),
        ],
        cpus: Some(2),
        memory_mib: Some(2048),
        env: vec![
            EnvVar {
                key: "PORT".into(),
                value: "4000".into(),
                is_secret: false,
            },
            EnvVar {
                key: "LITELLM_MASTER_KEY".into(),
                value: "${LITELLM_MASTER_KEY}".into(),
                is_secret: true,
            },
        ],
        secret_env: vec![
            SecretEnv {
                var: "OPENROUTER_API_KEY".into(),
                value: "${OPENROUTER_API_KEY}".into(),
                allowed_host: "openrouter.ai".into(),
            },
            SecretEnv {
                var: "KIMI_CODE_API_KEY".into(),
                value: "${KIMI_CODE_API_KEY}".into(),
                allowed_host: "api.kimi.com".into(),
            },
            SecretEnv {
                var: "MINIMAX_CODING_API_KEY".into(),
                value: "${MINIMAX_CODING_API_KEY}".into(),
                allowed_host: "api.minimax.io".into(),
            },
            SecretEnv {
                var: "INCEPTION_API_KEY".into(),
                value: "${INCEPTION_API_KEY}".into(),
                allowed_host: "api.inceptionlabs.ai".into(),
            },
        ],
        ports: vec![PortMapping {
            host: 4000,
            guest: 4000,
        }],
        mounts: vec![MountPlan {
            host: "infra/litellm/config.yaml".into(),
            guest: "/app/config.yaml".into(),
            read_only: true,
        }],
        network: NetworkPlan {
            default_deny: true,
            egress_rules: vec![
                EgressRule {
                    protocol: "tcp".into(),
                    port: Some(53),
                    allow_hosts: vec!["host".into()],
                },
                EgressRule {
                    protocol: "udp".into(),
                    port: Some(53),
                    allow_hosts: vec!["host".into()],
                },
                EgressRule {
                    protocol: "tcp".into(),
                    port: Some(443),
                    allow_hosts: vec![
                        "openrouter.ai".into(),
                        "api.kimi.com".into(),
                        "api.minimax.io".into(),
                        "api.inceptionlabs.ai".into(),
                    ],
                },
            ],
            deny_rules: vec![],
        },
    }
}

pub fn build_pi_plan() -> SandboxPlan {
    SandboxPlan {
        name: "pi".into(),
        image: Some("node:24-bookworm-slim".into()),
        workdir: Some("/app".into()),
        command: vec!["pi".into(), "--mode".into(), "rpc".into()],
        cpus: Some(2),
        memory_mib: Some(2048),
        env: vec![
            EnvVar {
                key: "PI_OFFLINE".into(),
                value: "1".into(),
                is_secret: false,
            },
            EnvVar {
                key: "PI_TELEMETRY".into(),
                value: "0".into(),
                is_secret: false,
            },
        ],
        secret_env: vec![
            // M1 auth: agents use the LiteLLM master key directly (no virtual keys yet).
            SecretEnv {
                var: "OPENAI_API_KEY".into(),
                value: "${LITELLM_MASTER_KEY}".into(),
                allowed_host: "host.microsandbox.internal".into(),
            },
        ],
        ports: vec![],
        mounts: vec![
            MountPlan {
                host: "agents/pi".into(),
                guest: "/app".into(),
                read_only: true,
            },
            MountPlan {
                host: "workspaces/pi".into(),
                guest: "/workspace".into(),
                read_only: true,
            },
        ],
        network: NetworkPlan {
            default_deny: true,
            egress_rules: vec![EgressRule {
                protocol: "tcp".into(),
                port: Some(4000),
                allow_hosts: vec!["host".into()],
            }],
            deny_rules: vec![DenyDomainRule {
                domain_suffix: ".pi.dev".into(),
            }],
        },
    }
}

pub fn build_odysseus_plan() -> SandboxPlan {
    SandboxPlan {
        name: "odysseus".into(),
        image: Some("python:3.12-slim".into()),
        workdir: Some("/app".into()),
        command: vec![
            "uvicorn".into(),
            "app:app".into(),
            "--host".into(),
            "0.0.0.0".into(),
            "--port".into(),
            "7000".into(),
        ],
        cpus: Some(2),
        memory_mib: Some(2048),
        env: vec![
            EnvVar {
                key: "APP_PORT".into(),
                value: "7000".into(),
                is_secret: false,
            },
            EnvVar {
                key: "AUTH_ENABLED".into(),
                value: "true".into(),
                is_secret: false,
            },
            EnvVar {
                key: "LOCALHOST_BYPASS".into(),
                value: "false".into(),
                is_secret: false,
            },
        ],
        secret_env: vec![
            // M1 auth: agents use the LiteLLM master key directly (no virtual keys yet).
            SecretEnv {
                var: "OPENAI_API_KEY".into(),
                value: "${LITELLM_MASTER_KEY}".into(),
                allowed_host: "host.microsandbox.internal".into(),
            },
        ],
        ports: vec![PortMapping {
            host: 7000,
            guest: 7000,
        }],
        mounts: vec![
            MountPlan {
                host: "agents/odysseus".into(),
                guest: "/app".into(),
                read_only: true,
            },
            MountPlan {
                host: "workspaces/odysseus-data".into(),
                guest: "/app/data".into(),
                read_only: false,
            },
        ],
        network: NetworkPlan {
            default_deny: true,
            egress_rules: vec![EgressRule {
                protocol: "tcp".into(),
                port: Some(4000),
                allow_hosts: vec!["host".into()],
            }],
            deny_rules: vec![],
        },
    }
}

pub async fn up_litellm() -> Result<()> {
    require_env_vars(
        "agentctl litellm up",
        &[
            "LITELLM_MASTER_KEY",
            "OPENROUTER_API_KEY",
            "KIMI_CODE_API_KEY",
            "MINIMAX_CODING_API_KEY",
            "INCEPTION_API_KEY",
        ],
    )?;
    let root = crate::config::project_root()?;
    let plan = build_litellm_plan();
    ensure_mount_sources(&root, &plan)?;

    // NOTE: This policy must stay in sync with `build_litellm_plan().network`.
    //       If you change one, change the other.
    let policy = NetworkPolicy::builder()
        .default_deny()
        .ingress(|i| i.tcp().port(4000).allow_host())
        .egress(|e| e.udp().port(53).allow_host())
        .egress(|e| e.tcp().port(53).allow_host())
        .egress(|e| {
            e.tcp().port(443).allow_domains([
                "openrouter.ai",
                "api.kimi.com",
                "api.minimax.io",
                "api.inceptionlabs.ai",
            ])
        })
        .build()?;

    let mut builder = Sandbox::builder(&plan.name)
        .image(plan.image.as_deref().unwrap_or("alpine:latest"))
        .cpus(plan.cpus.unwrap_or(2))
        .memory(plan.memory_mib.unwrap_or(2048))
        .workdir(plan.workdir.as_deref().unwrap_or("/app"))
        .entrypoint(plan.command.iter().map(String::as_str))
        .port(4000, 4000)
        .network(|n| n.policy(policy))
        .detached(true);

    builder = apply_plan_envs(builder, &plan)?;

    builder = apply_plan_mounts(builder, &root, &plan);
    builder = apply_plan_secrets(builder, &plan)?;

    let sandbox = builder.replace().create().await?;
    println!("Sandbox '{}' started", sandbox.name());
    Ok(())
}

pub async fn down_litellm() -> Result<()> {
    match Sandbox::get("litellm").await {
        Ok(handle) => {
            stop_and_remove(handle).await?;
            println!("Sandbox 'litellm' stopped and removed");
            Ok(())
        }
        Err(microsandbox::MicrosandboxError::SandboxNotFound(_)) => {
            println!("Sandbox 'litellm' not found");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn up_pi() -> Result<()> {
    require_env_vars("agentctl agent up pi", &["LITELLM_MASTER_KEY"])?;
    let root = crate::config::project_root()?;
    let plan = build_pi_plan();
    ensure_mount_sources(&root, &plan)?;

    let policy = NetworkPolicy::builder()
        .default_deny()
        .egress(|e| e.udp().port(53).allow_host())
        .egress(|e| e.tcp().port(53).allow_host())
        .egress(|e| e.tcp().port(4000).allow_host())
        .egress(|e| e.deny_domain_suffixes([".pi.dev"]))
        .build()?;

    let mut builder = Sandbox::builder(&plan.name)
        .image(plan.image.as_deref().unwrap_or("alpine:latest"))
        .cpus(plan.cpus.unwrap_or(2))
        .memory(plan.memory_mib.unwrap_or(2048))
        .workdir(plan.workdir.as_deref().unwrap_or("/app"))
        .entrypoint(plan.command.iter().map(String::as_str))
        .network(|n| n.policy(policy))
        .detached(true);

    builder = apply_plan_envs(builder, &plan)?;

    builder = apply_plan_mounts(builder, &root, &plan);
    builder = apply_plan_secrets(builder, &plan)?;

    let sandbox = builder.replace().create().await?;
    println!("Sandbox '{}' started", sandbox.name());
    Ok(())
}

pub async fn down_pi() -> Result<()> {
    match Sandbox::get("pi").await {
        Ok(handle) => {
            stop_and_remove(handle).await?;
            println!("Sandbox 'pi' stopped and removed");
            Ok(())
        }
        Err(microsandbox::MicrosandboxError::SandboxNotFound(_)) => {
            println!("Sandbox 'pi' not found");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn up_odysseus() -> Result<()> {
    require_env_vars("agentctl agent up odysseus", &["LITELLM_MASTER_KEY"])?;
    let root = crate::config::project_root()?;
    let plan = build_odysseus_plan();
    ensure_mount_sources(&root, &plan)?;

    let policy = NetworkPolicy::builder()
        .default_deny()
        .ingress(|i| i.tcp().port(7000).allow_host())
        .egress(|e| e.udp().port(53).allow_host())
        .egress(|e| e.tcp().port(53).allow_host())
        .egress(|e| e.tcp().port(4000).allow_host())
        .build()?;

    let mut builder = Sandbox::builder(&plan.name)
        .image(plan.image.as_deref().unwrap_or("alpine:latest"))
        .cpus(plan.cpus.unwrap_or(2))
        .memory(plan.memory_mib.unwrap_or(2048))
        .workdir(plan.workdir.as_deref().unwrap_or("/app"))
        .entrypoint(plan.command.iter().map(String::as_str))
        .port(7000, 7000)
        .network(|n| n.policy(policy))
        .detached(true);

    builder = apply_plan_envs(builder, &plan)?;

    builder = apply_plan_mounts(builder, &root, &plan);
    builder = apply_plan_secrets(builder, &plan)?;

    let sandbox = builder.replace().create().await?;
    println!("Sandbox '{}' started", sandbox.name());
    Ok(())
}

pub async fn down_odysseus() -> Result<()> {
    match Sandbox::get("odysseus").await {
        Ok(handle) => {
            stop_and_remove(handle).await?;
            println!("Sandbox 'odysseus' stopped and removed");
            Ok(())
        }
        Err(microsandbox::MicrosandboxError::SandboxNotFound(_)) => {
            println!("Sandbox 'odysseus' not found");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn require_env_vars_succeeds_when_all_set() {
        let vars = ["AGENTCTL_TEST_A", "AGENTCTL_TEST_B"];
        for name in &vars {
            std::env::set_var(name, "value");
        }
        let result = require_env_vars("test", &vars);
        for name in &vars {
            std::env::remove_var(name);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn require_env_vars_reports_missing_var() {
        std::env::remove_var("AGENTCTL_TEST_MISSING");
        let err = require_env_vars("agentctl test up", &["AGENTCTL_TEST_MISSING"])
            .unwrap_err()
            .to_string();
        assert!(err.contains("agentctl test up"), "context missing: {err}");
        assert!(
            err.contains("AGENTCTL_TEST_MISSING"),
            "var name missing: {err}"
        );
    }

    #[test]
    fn require_env_vars_treats_empty_and_whitespace_as_missing() {
        std::env::set_var("AGENTCTL_TEST_EMPTY", "");
        std::env::set_var("AGENTCTL_TEST_SPACE", "   ");
        let err = require_env_vars("test", &["AGENTCTL_TEST_EMPTY", "AGENTCTL_TEST_SPACE"])
            .unwrap_err()
            .to_string();
        assert!(err.contains("AGENTCTL_TEST_EMPTY"), "{err}");
        assert!(err.contains("AGENTCTL_TEST_SPACE"), "{err}");
    }

    #[test]
    fn require_env_vars_rejects_litellm_placeholder() {
        std::env::set_var("LITELLM_MASTER_KEY", "sk-change-me-local-only");
        let err = require_env_vars("agentctl litellm up", &["LITELLM_MASTER_KEY"])
            .unwrap_err()
            .to_string();
        assert!(err.contains("placeholder"), "{err}");
        std::env::remove_var("LITELLM_MASTER_KEY");
    }
}
