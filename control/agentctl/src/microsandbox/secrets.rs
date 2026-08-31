use crate::config::SecretViolationPolicy;

/// Resolved definition of a secret (final unified model, spec 16), built
/// from the merged `[secrets.<NAME>]` config at load time.
///
/// In the migrated config-driven model, secrets are declared in the
/// `[secrets.<NAME>]` section of `workestrate.toml` as a pure catalog of
/// intrinsic credential properties. Core validates them against `policy.rs`
/// and builds these definitions at load time. Exposure mode is NOT part of
/// the definition — it is declared per workload at the binding site
/// (`bound` on the env binding).
#[derive(Debug, Clone)]
pub struct SecretDefinition {
    /// The host environment variable the resolved value is read from (the
    /// raw `env_var`, defaulting to the secret ID when absent).
    pub source_env_var: String,
    /// Egress hosts whose rewrites may substitute the real value (an
    /// omitted `allowed_hosts` resolves to deny-all — an explicit zero
    /// allowed hosts).
    pub allowed_hosts: Vec<String>,
    /// Whether the secret must be set (true) or can be missing (false).
    pub required: bool,
    /// Known-bad placeholder value to reject.
    pub placeholder: Option<String>,
    /// Violation policy for egress traffic carrying the placeholder to a
    /// non-allowed host; defaults to passthrough.
    pub on_violation: SecretViolationPolicy,
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

    #[test]
    fn secret_definition_constructs_and_exposes_fields() {
        let def = SecretDefinition {
            source_env_var: "MY_KEY".to_string(),
            allowed_hosts: vec!["a.com".to_string(), "b.com".to_string()],
            required: false,
            placeholder: None,
            on_violation: SecretViolationPolicy::Passthrough,
        };
        assert_eq!(def.source_env_var, "MY_KEY");
        assert_eq!(
            def.allowed_hosts,
            vec!["a.com".to_string(), "b.com".to_string()]
        );
        assert!(!def.required);
        assert!(def.placeholder.is_none());
    }

    #[test]
    fn secret_definition_clone_and_debug() {
        let def = SecretDefinition {
            source_env_var: "K".to_string(),
            allowed_hosts: vec![],
            required: true,
            placeholder: Some("PLACEHOLDER".to_string()),
            on_violation: SecretViolationPolicy::BlockAndLog,
        };
        let cloned = def.clone();
        assert_eq!(cloned.source_env_var, def.source_env_var);
        assert_eq!(cloned.placeholder, def.placeholder);
        let dbg = format!("{def:?}");
        assert!(
            dbg.contains("SecretDefinition"),
            "debug names the type: {dbg}"
        );
        assert!(dbg.contains("\"K\""), "debug shows source_env_var: {dbg}");
    }
}
