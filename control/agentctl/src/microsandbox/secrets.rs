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
            env_var: "MY_KEY".to_string(),
            hosts: vec!["a.com".to_string(), "b.com".to_string()],
            required: false,
            placeholder: None,
        };
        assert_eq!(def.env_var, "MY_KEY");
        assert_eq!(def.hosts, vec!["a.com".to_string(), "b.com".to_string()]);
        assert!(!def.required);
        assert!(def.placeholder.is_none());
    }

    #[test]
    fn secret_definition_clone_and_debug() {
        let def = SecretDefinition {
            env_var: "K".to_string(),
            hosts: vec![],
            required: true,
            placeholder: Some("PLACEHOLDER".to_string()),
        };
        let cloned = def.clone();
        assert_eq!(cloned.env_var, def.env_var);
        assert_eq!(cloned.placeholder, def.placeholder);
        let dbg = format!("{def:?}");
        assert!(
            dbg.contains("SecretDefinition"),
            "debug names the type: {dbg}"
        );
        assert!(dbg.contains("\"K\""), "debug shows env_var: {dbg}");
    }

    #[test]
    fn remapped_secret_wraps_source_and_exposed_name() {
        let remapped = RemappedSecret {
            source: SecretDefinition {
                env_var: "LITELLM_MASTER_KEY".to_string(),
                hosts: vec!["host.microsandbox.internal".to_string()],
                required: true,
                placeholder: None,
            },
            exposed_as: "OPENAI_API_KEY".to_string(),
        };
        assert_eq!(remapped.exposed_as, "OPENAI_API_KEY");
        assert_eq!(remapped.source.env_var, "LITELLM_MASTER_KEY");
        let cloned = remapped.clone();
        assert_eq!(cloned.exposed_as, remapped.exposed_as);
        let dbg = format!("{remapped:?}");
        assert!(
            dbg.contains("RemappedSecret"),
            "debug names the type: {dbg}"
        );
    }

    #[test]
    fn secret_definition_deserializes_from_toml() {
        let def: SecretDefinition = toml::from_str(
            r#"
env_var = "GITHUB_TOKEN"
hosts = ["github.com", "api.github.com"]
required = false
placeholder = "ghp_CHANGEME"
"#,
        )
        .expect("SecretDefinition must deserialize from its TOML shape");
        assert_eq!(def.env_var, "GITHUB_TOKEN");
        assert_eq!(def.hosts, vec!["github.com", "api.github.com"]);
        assert!(!def.required);
        assert_eq!(def.placeholder.as_deref(), Some("ghp_CHANGEME"));
    }

    #[test]
    fn remapped_secret_deserializes_from_json() {
        let remapped: RemappedSecret = serde_json::from_str(
            r#"{
                "source": {
                    "env_var": "LITELLM_MASTER_KEY",
                    "hosts": ["host.microsandbox.internal"],
                    "required": true,
                    "placeholder": null
                },
                "exposed_as": "OPENAI_API_KEY"
            }"#,
        )
        .expect("RemappedSecret must deserialize from its JSON shape");
        assert_eq!(remapped.exposed_as, "OPENAI_API_KEY");
        assert!(remapped.source.required);
    }
}
