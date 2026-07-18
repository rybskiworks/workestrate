use crate::microsandbox::plan::EgressRule;
use serde::Deserialize;

/// Egress recipe reference as declared in config.
///
/// Config may only reference named recipes; each is expanded by core into the
/// concrete `EgressRule`s it represents.
#[derive(Debug, Clone, Deserialize)]
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
    #[allow(dead_code)]
    pub fn expand(&self) -> Vec<EgressRule> {
        match self {
            Self::Dns => EgressRule::dns(),
            Self::LitellmProxy => vec![EgressRule::litellm_proxy()],
            Self::Github => vec![EgressRule::https(&["github.com", "api.github.com"])],
            Self::AgentBase => EgressRule::agent_base(),
            Self::Https { hosts } => {
                let hosts: Vec<&str> = hosts.iter().map(|s| s.as_str()).collect();
                vec![EgressRule::https(&hosts)]
            }
        }
    }
}
