use crate::config::Delivery;

/// Resolved definition of a secret (v2), built from the merged
/// `[secrets.<NAME>]` config at load time.
///
/// In the migrated config-driven model, secrets are declared in the
/// `[secrets.<NAME>]` section of `workestrate.toml`. Core validates them
/// against `policy.rs` and builds these definitions at load time.
#[derive(Debug, Clone)]
pub struct SecretDefinition {
    /// The host environment variable the resolved value is read from (the
    /// raw `env_var`, defaulting to the secret ID when absent).
    pub source_env_var: String,
    /// Egress hosts this secret is bound to (host-bound delivery only; an
    /// omitted `hosts` resolves to deny-all — an explicit zero allowed
    /// hosts).
    pub allowed_hosts: Vec<String>,
    /// Whether the secret must be set (true) or can be missing (false).
    pub required: bool,
    /// Known-bad placeholder value to reject.
    pub placeholder: Option<String>,
    /// Delivery mode: env var vs host-bound injection (default host-bound;
    /// secure-by-default).
    pub delivery: Delivery,
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
            delivery: Delivery::HostBound,
        };
        assert_eq!(def.source_env_var, "MY_KEY");
        assert_eq!(
            def.allowed_hosts,
            vec!["a.com".to_string(), "b.com".to_string()]
        );
        assert!(!def.required);
        assert!(def.placeholder.is_none());
        assert_eq!(def.delivery, Delivery::HostBound);
    }

    #[test]
    fn secret_definition_clone_and_debug() {
        let def = SecretDefinition {
            source_env_var: "K".to_string(),
            allowed_hosts: vec![],
            required: true,
            placeholder: Some("PLACEHOLDER".to_string()),
            delivery: Delivery::Env,
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
        assert!(dbg.contains("Env"), "debug shows the delivery mode: {dbg}");
    }
}
