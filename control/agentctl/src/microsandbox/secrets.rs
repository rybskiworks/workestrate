use serde::Deserialize;

/// Definition of a secret and its egress bindings.
///
/// In the migrated config-driven model, secrets are declared in the
/// `[secrets.<NAME>]` section of `workestrate.toml`. Core validates them
/// against `policy.rs` and builds these definitions at load time.
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
