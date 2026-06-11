use std::fmt;

#[derive(Debug, Clone)]
pub struct SandboxPlan {
    pub name: String,
    pub image: Option<String>,
    pub workdir: Option<String>,
    pub command: Vec<String>,
    pub cpus: Option<u8>,
    pub memory_mib: Option<u32>,
    pub env: Vec<(String, String)>,
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
    pub value_source: String,
    pub allowed_host: String,
}

#[derive(Debug, Clone)]
pub struct NetworkPlan {
    pub default_deny: bool,
    pub egress_rules: Vec<EgressRule>,
}

#[derive(Debug, Clone)]
pub struct EgressRule {
    pub protocol: String,
    pub port: Option<u16>,
    pub allow_hosts: Vec<String>,
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
        for (k, v) in &self.env {
            writeln!(f, "env: {}={}", k, v)?;
        }
        for se in &self.secret_env {
            writeln!(
                f,
                "secret_env: {} (from {}, allowed: {})",
                se.var, se.value_source, se.allowed_host
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
            writeln!(
                f,
                "  egress: {}{}{} -> {}",
                rule.protocol,
                port,
                if hosts.is_empty() { "" } else { " to " },
                hosts
            )?;
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
        memory_mib: Some(1024),
        env: vec![("LITELLM_PORT".into(), "4000".into())],
        secret_env: vec![
            SecretEnv {
                var: "LITELLM_MASTER_KEY".into(),
                value_source: "env/LITELLM_MASTER_KEY".into(),
                allowed_host: "localhost".into(),
            },
            SecretEnv {
                var: "OPENAI_API_KEY".into(),
                value_source: "env/OPENAI_API_KEY".into(),
                allowed_host: "api.openai.com".into(),
            },
            SecretEnv {
                var: "ANTHROPIC_API_KEY".into(),
                value_source: "env/ANTHROPIC_API_KEY".into(),
                allowed_host: "api.anthropic.com".into(),
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
            egress_rules: vec![EgressRule {
                protocol: "tcp".into(),
                port: Some(443),
                allow_hosts: vec!["public".into()],
            }],
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
            ("PI_OFFLINE".into(), "1".into()),
            ("PI_TELEMETRY".into(), "0".into()),
        ],
        secret_env: vec![SecretEnv {
            var: "OPENAI_API_KEY".into(),
            value_source: "env/OPENAI_DUMMY_KEY".into(),
            allowed_host: "host".into(),
        }],
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
            ("APP_PORT".into(), "7000".into()),
            ("AUTH_ENABLED".into(), "true".into()),
            ("LOCALHOST_BYPASS".into(), "false".into()),
        ],
        secret_env: vec![SecretEnv {
            var: "OPENAI_API_KEY".into(),
            value_source: "env/OPENAI_DUMMY_KEY".into(),
            allowed_host: "host".into(),
        }],
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
        },
    }
}
