use crate::microsandbox::plan::EgressRule;
use serde::{Deserialize, Serialize};

/// Egress recipe reference as declared in config.
///
/// Config may only reference named recipes; each is expanded by core into the
/// concrete `EgressRule`s it represents.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, schemars::JsonSchema)]
#[serde(tag = "recipe", rename_all = "snake_case")]
pub enum EgressRecipeRef {
    Dns,
    LitellmProxy,
    Github,
    AgentBase,
    Https { hosts: Vec<String> },
}

impl EgressRecipeRef {
    /// Expand this recipe into the concrete egress rules it represents.
    pub fn expand(&self) -> Vec<EgressRule> {
        match self {
            Self::Dns => EgressRule::dns(),
            Self::LitellmProxy => vec![EgressRule::litellm_proxy()],
            Self::Github => vec![EgressRule::https(crate::policy::GITHUB_HOSTS)],
            Self::AgentBase => EgressRule::agent_base(),
            Self::Https { hosts } => {
                let hosts: Vec<&str> = hosts.iter().map(|s| s.as_str()).collect();
                vec![EgressRule::https(&hosts)]
            }
        }
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
    use crate::microsandbox::plan::{EgressTarget, Protocol};

    fn assert_rule(rule: &EgressRule, protocol: Protocol, port: u16, target: EgressTarget) {
        assert_eq!(rule.protocol, protocol, "protocol mismatch");
        assert_eq!(rule.port, port, "port mismatch");
        assert_eq!(rule.target, target, "target mismatch");
    }

    #[test]
    fn dns_expands_to_tcp_and_udp_53_to_host() {
        let rules = EgressRecipeRef::Dns.expand();
        assert_eq!(rules.len(), 2, "dns recipe must expand to tcp+udp");
        assert_rule(&rules[0], Protocol::Tcp, 53, EgressTarget::Host);
        assert_rule(&rules[1], Protocol::Udp, 53, EgressTarget::Host);
    }

    #[test]
    fn litellm_proxy_expands_to_tcp_4000_to_host() {
        let rules = EgressRecipeRef::LitellmProxy.expand();
        assert_eq!(rules.len(), 1);
        assert_rule(&rules[0], Protocol::Tcp, 4000, EgressTarget::Host);
    }

    #[test]
    fn github_expands_to_https_443_to_github_hosts() {
        let rules = EgressRecipeRef::Github.expand();
        assert_eq!(rules.len(), 1);
        assert_rule(
            &rules[0],
            Protocol::Tcp,
            443,
            EgressTarget::Domains(vec!["github.com".into(), "api.github.com".into()]),
        );
    }

    #[test]
    fn agent_base_is_dns_plus_litellm_plus_github() {
        let rules = EgressRecipeRef::AgentBase.expand();
        assert_eq!(rules.len(), 4, "agent_base = 2 dns + litellm + github");
        assert_rule(&rules[0], Protocol::Tcp, 53, EgressTarget::Host);
        assert_rule(&rules[1], Protocol::Udp, 53, EgressTarget::Host);
        assert_rule(&rules[2], Protocol::Tcp, 4000, EgressTarget::Host);
        assert_rule(
            &rules[3],
            Protocol::Tcp,
            443,
            EgressTarget::Domains(vec!["github.com".into(), "api.github.com".into()]),
        );
    }

    #[test]
    fn https_expands_single_host_to_tcp_443_domains() {
        let rules = EgressRecipeRef::Https {
            hosts: vec!["openrouter.ai".to_string()],
        }
        .expand();
        assert_eq!(rules.len(), 1);
        assert_rule(
            &rules[0],
            Protocol::Tcp,
            443,
            EgressTarget::Domains(vec!["openrouter.ai".into()]),
        );
    }

    #[test]
    fn https_expands_multiple_hosts_in_order() {
        let hosts = vec![
            "huggingface.co".to_string(),
            "cdn-lfs.huggingface.co".to_string(),
            "cdn-lfs-us-1.huggingface.co".to_string(),
        ];
        let rules = EgressRecipeRef::Https {
            hosts: hosts.clone(),
        }
        .expand();
        assert_eq!(
            rules.len(),
            1,
            "https recipe is a single rule per recipe ref"
        );
        assert_rule(&rules[0], Protocol::Tcp, 443, EgressTarget::Domains(hosts));
    }
}
