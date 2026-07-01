/// Definition of a secret and its egress bindings.
///
/// Defined once here as a `const`. Workloads reference these via
/// `HostBoundSecret::from()` — no positional args or magic strings.
pub struct SecretDefinition {
    /// Environment variable name in the host process.
    pub env_var: &'static str,
    /// Egress hosts this secret is bound to.
    pub hosts: &'static [&'static str],
    /// Whether the secret must be set (true) or can be missing (false).
    pub required: bool,
    /// Known-bad placeholder value to reject.
    pub placeholder: Option<&'static str>,
    /// Human-readable description.
    #[allow(dead_code)]
    pub description: &'static str,
}

/// A secret that is exposed under a different name inside the sandbox.
///
/// For example, `LITELLM_MASTER_KEY` is exposed as `OPENAI_API_KEY` to
/// the sandbox process. The source definition provides the hosts and
/// required/optional status; this struct adds the destination name.
pub struct RemappedSecret {
    /// The source secret definition.
    pub source: &'static SecretDefinition,
    /// The env var name the sandbox process sees.
    pub exposed_as: &'static str,
}

// --- Secret definitions ---

pub const LITELLM_MASTER_KEY: SecretDefinition = SecretDefinition {
    env_var: "LITELLM_MASTER_KEY",
    hosts: &["host.microsandbox.internal"],
    required: true,
    placeholder: None,
    description: "LiteLLM proxy authentication",
};

pub const LITELLM_AUTH: RemappedSecret = RemappedSecret {
    source: &LITELLM_MASTER_KEY,
    exposed_as: "OPENAI_API_KEY",
};

pub const OPENROUTER: SecretDefinition = SecretDefinition {
    env_var: "OPENROUTER_API_KEY",
    hosts: &["openrouter.ai"],
    required: true,
    placeholder: None,
    description: "OpenRouter LLM provider",
};

pub const KIMI: SecretDefinition = SecretDefinition {
    env_var: "KIMI_CODE_API_KEY",
    hosts: &["api.kimi.com"],
    required: true,
    placeholder: None,
    description: "Kimi for Coding LLM provider",
};

pub const NEURALWATT: SecretDefinition = SecretDefinition {
    env_var: "NEURALWATT_API_KEY",
    hosts: &["api.neuralwatt.com"],
    required: true,
    placeholder: None,
    description: "Neuralwatt LLM provider",
};

pub const MINIMAX: SecretDefinition = SecretDefinition {
    env_var: "MINIMAX_CODING_API_KEY",
    hosts: &["api.minimax.io"],
    required: true,
    placeholder: None,
    description: "MiniMax Coding LLM provider",
};

pub const GITHUB_TOKEN: SecretDefinition = SecretDefinition {
    env_var: "GITHUB_TOKEN",
    hosts: &["github.com", "api.github.com"],
    required: false,
    placeholder: None,
    description: "GitHub PAT for agent git/API access",
};

pub const ODYSSEUS_ADMIN_PASSWORD: SecretDefinition = SecretDefinition {
    env_var: "ODYSSEUS_ADMIN_PASSWORD",
    hosts: &[],
    required: true,
    placeholder: Some("change_me_before_first_boot"),
    description: "Odysseus admin login password (internal, not egress-bound)",
};
