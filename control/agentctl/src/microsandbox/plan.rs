use std::fmt;

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
    pub ingress_rules: Vec<IngressRule>,
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

#[derive(Debug, Clone)]
pub struct IngressRule {
    pub protocol: String,
    pub port: u16,
    pub scope: String,
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
        for rule in &self.network.ingress_rules {
            writeln!(
                f,
                "  ingress: {}:{} {}",
                rule.protocol, rule.port, rule.scope
            )?;
        }
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
            "/app/.venv/bin/litellm".into(),
            "--config".into(),
            "/app/config.yaml".into(),
            "--host".into(),
            "0.0.0.0".into(),
        ],
        cpus: Some(2),
        memory_mib: Some(2048),
        env: vec![
            EnvVar {
                key: "PORT".into(),
                value: "4000".into(),
                is_secret: false,
            },
            // LiteLLM normally fetches its cost map from the LiteLLM GitHub
            // repo on startup, which requires outbound egress to github.com
            // and slows boot. The bundled local map (shipped in the image) is
            // sufficient for our model set, so we skip the fetch.
            EnvVar {
                key: "LITELLM_LOCAL_MODEL_COST_MAP".into(),
                value: "True".into(),
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
        mounts: vec![
            MountPlan {
                host: "${MSB_HOME}/sandboxes/litellm/logs".into(),
                guest: "/var/log/litellm".into(),
                read_only: false,
            },
            MountPlan {
                host: "infra/litellm/config.yaml".into(),
                guest: "/app/config.yaml".into(),
                read_only: true,
            },
        ],
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
            ingress_rules: vec![IngressRule {
                protocol: "tcp".into(),
                port: 4000,
                scope: "local".into(),
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
                    port: Some(4000),
                    allow_hosts: vec!["host".into()],
                },
            ],
            deny_rules: vec![DenyDomainRule {
                domain_suffix: ".pi.dev".into(),
            }],
            ingress_rules: vec![],
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
            EnvVar {
                key: "DATABASE_URL".into(),
                value: "sqlite:///app/data/app.db".into(),
                is_secret: false,
            },
            EnvVar {
                key: "OPENAI_BASE_URL".into(),
                value: "http://host.microsandbox.internal:4000/v1".into(),
                is_secret: false,
            },
            EnvVar {
                key: "OPENAI_MODEL".into(),
                value: "chat".into(),
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
            MountPlan {
                host: "${MSB_HOME}/sandboxes/odysseus/data".into(),
                guest: "/data".into(),
                read_only: false,
            },
            MountPlan {
                host: "${MSB_HOME}/sandboxes/odysseus/data/settings.json".into(),
                guest: "/app/data/settings.json".into(),
                read_only: true,
            },
        ],
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
                    port: Some(4000),
                    allow_hosts: vec!["host".into()],
                },
            ],
            deny_rules: vec![],
            ingress_rules: vec![IngressRule {
                protocol: "tcp".into(),
                port: 7000,
                scope: "local".into(),
            }],
        },
    }
}
