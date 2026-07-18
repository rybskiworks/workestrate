use serde::Deserialize;
use std::sync::LazyLock;

/// Definition of a secret and its egress bindings.
///
/// Defined once here as a lazy static. Workloads reference these via
/// `&secrets::NAME` — no positional args or magic strings.
#[derive(Debug, Clone, Deserialize)]
pub struct SecretDefinition {
    /// Environment variable name in the host process.
    pub env_var: String,
    /// Egress hosts this secret is bound to.
    pub hosts: Vec<String>,
    /// Whether the secret must be set (true) or can be missing (false).
    pub required: bool,
    /// Known-bad placeholder value to reject.
    pub placeholder: Option<String>,
    /// Human-readable description.
    #[allow(dead_code)]
    pub description: String,
}

/// A secret that is exposed under a different name inside the sandbox.
///
/// For example, `LITELLM_MASTER_KEY` is exposed as `OPENAI_API_KEY` to
/// the sandbox process. The source definition provides the hosts and
/// required/optional status; this struct adds the destination name.
#[derive(Debug, Clone, Deserialize)]
pub struct RemappedSecret {
    /// The source secret definition.
    pub source: SecretDefinition,
    /// The env var name the sandbox process sees.
    pub exposed_as: String,
}

// --- Secret definitions ---

pub static LITELLM_MASTER_KEY: LazyLock<SecretDefinition> = LazyLock::new(|| SecretDefinition {
    env_var: "LITELLM_MASTER_KEY".to_string(),
    hosts: vec!["host.microsandbox.internal".to_string()],
    required: true,
    placeholder: None,
    description: "LiteLLM proxy authentication".to_string(),
});

pub static LITELLM_AUTH: LazyLock<RemappedSecret> = LazyLock::new(|| RemappedSecret {
    source: LITELLM_MASTER_KEY.clone(),
    exposed_as: "OPENAI_API_KEY".to_string(),
});

pub static OPENROUTER: LazyLock<SecretDefinition> = LazyLock::new(|| SecretDefinition {
    env_var: "OPENROUTER_API_KEY".to_string(),
    hosts: vec!["openrouter.ai".to_string()],
    required: true,
    placeholder: None,
    description: "OpenRouter LLM provider".to_string(),
});

pub static KIMI: LazyLock<SecretDefinition> = LazyLock::new(|| SecretDefinition {
    env_var: "KIMI_CODE_API_KEY".to_string(),
    hosts: vec!["api.kimi.com".to_string()],
    required: true,
    placeholder: None,
    description: "Kimi for Coding LLM provider".to_string(),
});

pub static NEURALWATT: LazyLock<SecretDefinition> = LazyLock::new(|| SecretDefinition {
    env_var: "NEURALWATT_API_KEY".to_string(),
    hosts: vec!["api.neuralwatt.com".to_string()],
    required: true,
    placeholder: None,
    description: "Neuralwatt LLM provider".to_string(),
});

pub static MINIMAX: LazyLock<SecretDefinition> = LazyLock::new(|| SecretDefinition {
    env_var: "MINIMAX_CODING_API_KEY".to_string(),
    hosts: vec!["api.minimax.io".to_string()],
    required: true,
    placeholder: None,
    description: "MiniMax Coding LLM provider".to_string(),
});

pub static GITHUB_TOKEN: LazyLock<SecretDefinition> = LazyLock::new(|| SecretDefinition {
    env_var: "GITHUB_TOKEN".to_string(),
    hosts: vec!["github.com".to_string(), "api.github.com".to_string()],
    required: false,
    placeholder: None,
    description: "GitHub PAT for agent git/API access".to_string(),
});

pub static ODYSSEUS_ADMIN_PASSWORD: LazyLock<SecretDefinition> =
    LazyLock::new(|| SecretDefinition {
        env_var: "ODYSSEUS_ADMIN_PASSWORD".to_string(),
        hosts: vec![],
        required: true,
        placeholder: Some("change_me_before_first_boot".to_string()),
        description: "Odysseus admin login password (internal, not egress-bound)".to_string(),
    });
