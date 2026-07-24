use serde::{Deserialize, Serialize};
use std::fmt;

use crate::microsandbox::secrets::{RemappedSecret, SecretDefinition};

/// Network protocol for ingress/egress rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Tcp,
    Udp,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Tcp => write!(f, "tcp"),
            Protocol::Udp => write!(f, "udp"),
        }
    }
}

/// Network scope for ingress rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Local,
    /// Public ingress (not yet used by any workload).
    #[allow(dead_code)]
    Public,
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Scope::Local => write!(f, "local"),
            Scope::Public => write!(f, "public"),
        }
    }
}

/// Egress destination: host bridge or specific DNS domains.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum EgressTarget {
    /// Host bridge networking (microsandbox host, including host.microsandbox.internal).
    Host,
    /// Specific DNS domains.
    Domains(Vec<String>),
}

impl fmt::Display for EgressTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EgressTarget::Host => write!(f, "host"),
            EgressTarget::Domains(hosts) => write!(f, "{}", hosts.join(", ")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SandboxPlan {
    pub name: String,
    pub image: Option<String>,
    pub workdir: Option<String>,
    pub command: Vec<String>,
    pub cpus: Option<u8>,
    pub memory_mib: Option<u32>,
    pub env: Vec<EnvVar>,
    pub secret_env: Vec<HostBoundSecret>,
    pub ports: Vec<PortMapping>,
    pub mounts: Vec<MountPlan>,
    pub network: NetworkPlan,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PortMapping {
    pub host: u16,
    pub guest: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct MountPlan {
    pub host: String,
    pub guest: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HostBoundSecret {
    pub name: String,
    pub value: String,
    pub allowed_hosts: Vec<String>,
    pub required: bool,
    pub reject_placeholder: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EnvVar {
    pub name: String,
    pub value: String,
    pub is_secret: bool,
    pub reject_placeholder: Option<String>,
}

impl fmt::Display for EnvVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = if self.is_secret {
            "(redacted)"
        } else {
            self.value.as_str()
        };
        write!(f, "{}={}", self.name, value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NetworkPlan {
    pub default_deny: bool,
    pub egress_rules: Vec<EgressRule>,
    pub deny_rules: Vec<DenyDomainRule>,
    pub ingress_rules: Vec<IngressRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EgressRule {
    pub protocol: Protocol,
    pub port: u16,
    pub target: EgressTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DenyDomainRule {
    pub domain_suffix: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct IngressRule {
    pub protocol: Protocol,
    pub port: u16,
    pub scope: Scope,
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
            let req = if se.required { "required" } else { "optional" };
            writeln!(
                f,
                "secret_env: {} (value redacted, allowed: {}, {})",
                se.name,
                se.allowed_hosts.join(", "),
                req
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
            writeln!(
                f,
                "  egress: {}:{} -> {}",
                rule.protocol, rule.port, rule.target
            )?;
        }
        for rule in &self.network.deny_rules {
            writeln!(f, "  egress: deny domain suffix {}", rule.domain_suffix)?;
        }
        Ok(())
    }
}

impl EnvVar {
    pub fn literal(name: &str, value: &str) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            is_secret: false,
            reject_placeholder: None,
        }
    }
}

impl HostBoundSecret {
    /// Direct binding — the secret is exposed under its own name.
    /// All metadata comes from the definition.
    pub fn from(definition: &SecretDefinition) -> Self {
        Self {
            name: definition.env_var.clone(),
            value: format!("${{{}}}", definition.env_var),
            allowed_hosts: definition.hosts.clone(),
            required: definition.required,
            reject_placeholder: definition.placeholder.clone(),
        }
    }

    /// Remapped binding — the source secret is exposed under a different name.
    pub fn remapped(mapping: &RemappedSecret) -> Self {
        Self {
            name: mapping.exposed_as.clone(),
            value: format!("${{{}}}", mapping.source.env_var),
            allowed_hosts: mapping.source.hosts.clone(),
            required: mapping.source.required,
            reject_placeholder: mapping.source.placeholder.clone(),
        }
    }
}

impl EgressRule {
    pub fn dns() -> Vec<Self> {
        vec![
            Self {
                protocol: Protocol::Tcp,
                port: 53,
                target: EgressTarget::Host,
            },
            Self {
                protocol: Protocol::Udp,
                port: 53,
                target: EgressTarget::Host,
            },
        ]
    }
    pub fn litellm_proxy() -> Self {
        Self {
            protocol: Protocol::Tcp,
            port: 4000,
            target: EgressTarget::Host,
        }
    }
    pub fn https(domains: &[&str]) -> Self {
        Self {
            protocol: Protocol::Tcp,
            port: 443,
            target: EgressTarget::Domains(domains.iter().map(|d| d.to_string()).collect()),
        }
    }
    pub fn agent_base() -> Vec<Self> {
        let mut rules = Self::dns();
        rules.push(Self::litellm_proxy());
        rules.push(Self::https(crate::policy::GITHUB_HOSTS));
        rules
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

    fn definition(env_var: &str) -> SecretDefinition {
        SecretDefinition {
            env_var: env_var.to_string(),
            hosts: vec!["example.com".to_string()],
            required: true,
            placeholder: Some("CHANGEME".to_string()),
        }
    }

    // ---- Display stability (golden-inline; guards Display drift) ----

    #[test]
    fn sandbox_plan_display_is_stable() {
        let plan = SandboxPlan {
            name: "demo".to_string(),
            image: Some("img:1".to_string()),
            workdir: Some("/app".to_string()),
            command: vec!["run".to_string(), "--fast".to_string()],
            cpus: Some(2),
            memory_mib: Some(512),
            env: vec![
                EnvVar::literal("PLAIN", "value"),
                EnvVar {
                    name: "TOKEN".to_string(),
                    value: "supersecret".to_string(),
                    is_secret: true,
                    reject_placeholder: None,
                },
            ],
            secret_env: vec![HostBoundSecret {
                name: "API_KEY".to_string(),
                value: "${API_KEY}".to_string(),
                allowed_hosts: vec!["example.com".to_string()],
                required: false,
                reject_placeholder: None,
            }],
            ports: vec![PortMapping {
                host: 8080,
                guest: 80,
            }],
            mounts: vec![
                MountPlan {
                    host: "/data".to_string(),
                    guest: "/mnt".to_string(),
                    read_only: false,
                },
                MountPlan {
                    host: "/cfg".to_string(),
                    guest: "/etc/cfg".to_string(),
                    read_only: true,
                },
            ],
            network: NetworkPlan {
                default_deny: true,
                egress_rules: vec![
                    EgressRule::litellm_proxy(),
                    EgressRule::https(&["example.com"]),
                ],
                deny_rules: vec![DenyDomainRule {
                    domain_suffix: ".evil".to_string(),
                }],
                ingress_rules: vec![IngressRule {
                    protocol: Protocol::Tcp,
                    port: 80,
                    scope: Scope::Local,
                }],
            },
        };
        let expected = "\
name: demo
image: img:1
workdir: /app
command: run --fast
cpus: 2
memory: 512 MiB
env: PLAIN=value
env: TOKEN=(redacted)
secret_env: API_KEY (value redacted, allowed: example.com, optional)
port: 8080:80
mount: /data:/mnt
mount: /cfg:/etc/cfg (ro)
network: default_deny=true
  ingress: tcp:80 local
  egress: tcp:4000 -> host
  egress: tcp:443 -> example.com
  egress: deny domain suffix .evil
";
        assert_eq!(format!("{plan}"), expected);
    }

    #[test]
    fn sandbox_plan_display_omits_absent_optional_fields() {
        let plan = SandboxPlan {
            name: "bare".to_string(),
            image: None,
            workdir: None,
            command: vec![],
            cpus: None,
            memory_mib: None,
            env: vec![],
            secret_env: vec![],
            ports: vec![],
            mounts: vec![],
            network: NetworkPlan {
                default_deny: false,
                egress_rules: vec![],
                deny_rules: vec![],
                ingress_rules: vec![],
            },
        };
        assert_eq!(
            format!("{plan}"),
            "name: bare\nnetwork: default_deny=false\n"
        );
    }

    #[test]
    fn env_var_display_redacts_secrets_only() {
        let plain = EnvVar::literal("A", "1");
        assert_eq!(format!("{plain}"), "A=1");
        let secret = EnvVar {
            name: "S".to_string(),
            value: "hunter2".to_string(),
            is_secret: true,
            reject_placeholder: None,
        };
        let shown = format!("{secret}");
        assert_eq!(shown, "S=(redacted)");
        assert!(!shown.contains("hunter2"), "secret value must not leak");
    }

    // ---- constructors ----

    #[test]
    fn env_var_literal_marks_non_secret_and_no_placeholder() {
        let v = EnvVar::literal("K", "v");
        assert_eq!(v.name, "K");
        assert_eq!(v.value, "v");
        assert!(!v.is_secret);
        assert_eq!(v.reject_placeholder, None);
    }

    #[test]
    fn egress_rule_dns_is_tcp_and_udp_53_to_host() {
        let rules = EgressRule::dns();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].protocol, Protocol::Tcp);
        assert_eq!(rules[1].protocol, Protocol::Udp);
        for r in &rules {
            assert_eq!(r.port, 53);
            assert_eq!(r.target, EgressTarget::Host);
        }
    }

    #[test]
    fn egress_rule_litellm_proxy_is_tcp_4000_to_host() {
        let r = EgressRule::litellm_proxy();
        assert_eq!(r.protocol, Protocol::Tcp);
        assert_eq!(r.port, 4000);
        assert_eq!(r.target, EgressTarget::Host);
    }

    #[test]
    fn egress_rule_https_is_tcp_443_to_domains() {
        let r = EgressRule::https(&["a.com", "b.com"]);
        assert_eq!(r.protocol, Protocol::Tcp);
        assert_eq!(r.port, 443);
        assert_eq!(
            r.target,
            EgressTarget::Domains(vec!["a.com".to_string(), "b.com".to_string()])
        );
    }

    #[test]
    fn egress_rule_agent_base_composes_dns_litellm_github() {
        let rules = EgressRule::agent_base();
        assert_eq!(rules.len(), 4);
        assert_eq!(rules[2], EgressRule::litellm_proxy());
        assert_eq!(rules[3], EgressRule::https(crate::policy::GITHUB_HOSTS));
    }

    // ---- HostBoundSecret ----

    #[test]
    fn host_bound_secret_from_definition_templates_value_and_propagates_meta() {
        let def = definition("MY_SECRET");
        let bound = HostBoundSecret::from(&def);
        assert_eq!(bound.name, "MY_SECRET");
        assert_eq!(bound.value, "${MY_SECRET}");
        assert_eq!(bound.allowed_hosts, vec!["example.com".to_string()]);
        assert!(bound.required);
        assert_eq!(bound.reject_placeholder, Some("CHANGEME".to_string()));
    }

    #[test]
    fn host_bound_secret_remapped_uses_exposed_name_and_source_template() {
        let mapping = RemappedSecret {
            source: definition("SOURCE_KEY"),
            exposed_as: "OPENAI_API_KEY".to_string(),
        };
        let bound = HostBoundSecret::remapped(&mapping);
        assert_eq!(bound.name, "OPENAI_API_KEY");
        assert_eq!(
            bound.value, "${SOURCE_KEY}",
            "remapped value must template the SOURCE env var"
        );
        assert_eq!(bound.allowed_hosts, vec!["example.com".to_string()]);
        assert!(bound.required);
        assert_eq!(bound.reject_placeholder, Some("CHANGEME".to_string()));
    }
}
