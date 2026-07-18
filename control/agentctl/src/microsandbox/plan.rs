use serde::Deserialize;
use std::fmt;

use crate::microsandbox::secrets::{RemappedSecret, SecretDefinition};

/// Network protocol for ingress/egress rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct PortMapping {
    pub host: u16,
    pub guest: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MountPlan {
    pub host: String,
    pub guest: String,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HostBoundSecret {
    pub name: String,
    pub value: String,
    pub allowed_hosts: Vec<String>,
    pub required: bool,
    pub reject_placeholder: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NetworkPlan {
    pub default_deny: bool,
    pub egress_rules: Vec<EgressRule>,
    pub deny_rules: Vec<DenyDomainRule>,
    pub ingress_rules: Vec<IngressRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct EgressRule {
    pub protocol: Protocol,
    pub port: u16,
    pub target: EgressTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct DenyDomainRule {
    pub domain_suffix: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
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
    pub fn secret(definition: &SecretDefinition) -> Self {
        Self {
            name: definition.env_var.clone(),
            value: format!("${{{}}}", definition.env_var),
            is_secret: true,
            reject_placeholder: definition.placeholder.clone(),
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

impl MountPlan {
    pub fn readonly(host: impl Into<String>, guest: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            guest: guest.into(),
            read_only: true,
        }
    }
    pub fn readwrite(host: impl Into<String>, guest: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            guest: guest.into(),
            read_only: false,
        }
    }
}

impl PortMapping {
    pub fn same(port: u16) -> Self {
        Self {
            host: port,
            guest: port,
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
        rules.push(Self::https(&["github.com", "api.github.com"]));
        rules
    }
}

impl IngressRule {
    pub fn local_tcp(port: u16) -> Self {
        Self {
            protocol: Protocol::Tcp,
            port,
            scope: Scope::Local,
        }
    }
}
