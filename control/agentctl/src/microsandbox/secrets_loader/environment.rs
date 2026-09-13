//! Explicit development secret input; this does not change delivery authority.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use anyhow::{Result, bail};

use crate::config::{ConfigFile, EnvBinding};

/// Inherited by dependency starts, detached children and nested `run` commands.
/// An explicit CLI selection always overrides this nonsecret setting.
pub const SOURCE_ENV: &str = "WORKESTRATE_SECRETS_SOURCE";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum SecretsSource {
    /// Existing environment plus configured encrypted layers.
    #[default]
    Layered,
    /// Development only: declared environment names, without SOPS or age.
    Env,
}

impl SecretsSource {
    pub fn resolve(explicit: Option<Self>, inherited: Option<&str>) -> Result<Self> {
        if let Some(selected) = explicit {
            return Ok(selected);
        }
        match inherited {
            None | Some("layered") => Ok(Self::Layered),
            Some("env") => Ok(Self::Env),
            Some(_) => bail!("{SOURCE_ENV} must be 'layered' or 'env'"),
        }
    }

    pub fn current() -> Result<Self> {
        match std::env::var(SOURCE_ENV) {
            Ok(value) => Self::resolve(None, Some(&value)),
            Err(std::env::VarError::NotPresent) => Ok(Self::Layered),
            Err(std::env::VarError::NotUnicode(_)) => bail!("{SOURCE_ENV} must be valid text"),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Layered => "layered",
            Self::Env => "env",
        }
    }
}

pub(super) fn process_lookup(name: &str) -> Result<Option<String>> {
    match std::env::var(name) {
        Ok(value) => Ok(Some(value)),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(_)) => {
            bail!("environment secret source '{name}' must be valid text")
        }
    }
}

/// Return values only under canonical source names; aliases never rewrite the
/// plan, secret definitions, credential grants or host/guest binding policy.
/// `None` is the host `run`/library scope: all configured secret definitions.
pub(super) fn resolve(
    config: &ConfigFile,
    workload_name: Option<&str>,
    lookup: &dyn Fn(&str) -> Result<Option<String>>,
) -> Result<(HashMap<String, String>, crate::merge::Provenance)> {
    let workloads: Vec<_> = if let Some(name) = workload_name {
        vec![config.workloads.get(name).ok_or_else(|| {
            anyhow::anyhow!("unknown workload '{name}' for environment secret resolution")
        })?]
    } else {
        config.workloads.values().collect()
    };
    let mut selected: BTreeSet<String> = if workload_name.is_none() {
        config.secrets.keys().cloned().collect()
    } else {
        BTreeSet::new()
    };
    let mut aliases_by_secret: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for workload in workloads {
        for (alias, binding) in workload.env.iter() {
            if let EnvBinding::Secret(reference) = binding {
                selected.insert(reference.secret.clone());
                aliases_by_secret
                    .entry(reference.secret.clone())
                    .or_default()
                    .insert(alias.clone());
            }
        }
        for name in &workload.credentials.ssh {
            let credential = config
                .credentials
                .ssh
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("undefined SSH credential '{name}'"))?;
            selected.insert(credential.material.clone());
        }
        for name in &workload.credentials.signing {
            let credential = config
                .credentials
                .signing
                .ssh
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("undefined signing credential '{name}'"))?;
            selected.insert(credential.material.clone());
        }
    }

    let mut sources: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for secret in &selected {
        let definition = config.secrets.get(secret).ok_or_else(|| {
            anyhow::anyhow!("undefined secret '{secret}' for environment resolution")
        })?;
        let canonical = definition.env_var.as_deref().unwrap_or(secret);
        anyhow::ensure!(
            super::is_valid_env_name(canonical),
            "invalid secret source name '{canonical}'"
        );
        let aliases = sources.entry(canonical.to_string()).or_default();
        if let Some(names) = aliases_by_secret.get(secret) {
            for alias in names {
                anyhow::ensure!(
                    super::is_valid_env_name(alias),
                    "invalid secret alias '{alias}'"
                );
                aliases.insert(alias.clone());
            }
        }
    }
    let canonical_names: BTreeSet<_> = sources.keys().cloned().collect();
    let mut alias_owners: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (canonical, aliases) in &sources {
        for alias in aliases {
            alias_owners
                .entry(alias.clone())
                .or_default()
                .insert(canonical.clone());
        }
    }
    let mut values = HashMap::new();
    let mut provenance = crate::merge::Provenance::new();
    for (canonical, aliases) in &sources {
        // Presence is authoritative, even when blank: never hide an explicitly
        // empty canonical source by falling back to another credential.
        let (candidate, origin) = if let Some(value) = lookup(canonical)? {
            (Some(value), format!("environment:{canonical}"))
        } else {
            let mut found: Option<String> = None;
            let mut chosen_alias = None;
            for alias in aliases {
                if alias == canonical {
                    continue;
                }
                let Some(value) = lookup(alias)? else {
                    continue;
                };
                if value.trim().is_empty() {
                    continue;
                }
                anyhow::ensure!(
                    !canonical_names.contains(alias)
                        && alias_owners
                            .get(alias)
                            .is_some_and(|owners| owners.len() == 1),
                    "ambiguous environment alias '{alias}' for secret source '{canonical}'; set the canonical source explicitly"
                );
                if let Some(previous) = &found {
                    anyhow::ensure!(
                        previous == &value,
                        "conflicting environment aliases for secret source '{canonical}'; set the canonical source explicitly"
                    );
                } else {
                    found = Some(value);
                    chosen_alias = Some(alias.as_str());
                }
            }
            (
                found,
                format!("environment-alias:{}", chosen_alias.unwrap_or(canonical)),
            )
        };
        if let Some(value) = candidate.filter(|value| !value.trim().is_empty()) {
            values.insert(canonical.clone(), value);
            provenance.insert(canonical.clone(), origin);
        }
    }

    for secret in selected {
        let definition = &config.secrets[&secret];
        let canonical = definition.env_var.as_deref().unwrap_or(&secret);
        let value = values.get(canonical);
        anyhow::ensure!(
            !definition.required.unwrap_or(true) || value.is_some(),
            "required secret '{secret}' is not satisfied by environment source '{canonical}' or its declared aliases (--secrets-source env); SOPS is disabled"
        );
        if let (Some(value), Some(placeholder)) = (value, &definition.placeholder) {
            anyhow::ensure!(
                value.trim() != placeholder.trim(),
                "secret '{secret}' matches its configured example placeholder (--secrets-source env)"
            );
        }
    }
    Ok((values, provenance))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn config(extra: &str) -> ConfigFile {
        toml::from_str(&format!(
            r#"
schema_version = 1
[secrets.MACHINE]
env_var = "MACHINE_GITHUB_TOKEN"
placeholder = "example-only"
[workloads.agent]
kind = "agent"
image = {{ recipe = "registry", ref = "synthetic:latest" }}
[workloads.agent.env]
GITHUB_TOKEN = {{ secret = "MACHINE", bound = "host" }}
GH_TOKEN = {{ secret = "MACHINE", bound = "host" }}
{extra}
"#
        ))
        .unwrap()
    }

    fn resolved(
        config: &ConfigFile,
        scope: Option<&str>,
        pairs: &[(&str, &str)],
    ) -> Result<HashMap<String, String>> {
        let input: HashMap<_, _> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        resolve(config, scope, &|name| Ok(input.get(name).cloned())).map(|(values, _)| values)
    }

    #[test]
    fn explicit_mode_wins_over_inheritance_and_default_is_layered() {
        assert_eq!(
            SecretsSource::resolve(None, None).unwrap(),
            SecretsSource::Layered
        );
        assert_eq!(
            SecretsSource::resolve(None, Some("env")).unwrap(),
            SecretsSource::Env
        );
        assert_eq!(
            SecretsSource::resolve(Some(SecretsSource::Layered), Some("env")).unwrap(),
            SecretsSource::Layered
        );
        assert_eq!(
            SecretsSource::resolve(Some(SecretsSource::Env), Some("invalid")).unwrap(),
            SecretsSource::Env
        );
        assert!(SecretsSource::resolve(None, Some("invalid")).is_err());
        assert!(SecretsSource::resolve(None, Some("")).is_err());
    }

    #[test]
    fn canonical_wins_and_only_canonical_result_is_returned() {
        let values = resolved(
            &config(""),
            Some("agent"),
            &[
                ("MACHINE_GITHUB_TOKEN", " synthetic-canonical "),
                ("GITHUB_TOKEN", "synthetic-other"),
                ("GH_TOKEN", "synthetic-third"),
                ("UNDECLARED", "not-consumed"),
                ("MACHINE", "not-the-source-env-var"),
            ],
        )
        .unwrap();
        assert_eq!(
            values,
            HashMap::from([(
                "MACHINE_GITHUB_TOKEN".into(),
                " synthetic-canonical ".into()
            )])
        );
    }

    #[test]
    fn alias_fallback_and_identical_multiple_aliases_are_supported() {
        for pairs in [
            vec![("GITHUB_TOKEN", "synthetic")],
            vec![("GH_TOKEN", "synthetic")],
            vec![("GITHUB_TOKEN", "synthetic"), ("GH_TOKEN", "synthetic")],
        ] {
            assert_eq!(
                resolved(&config(""), Some("agent"), &pairs).unwrap()["MACHINE_GITHUB_TOKEN"],
                "synthetic"
            );
        }
    }

    #[test]
    fn conflicting_aliases_refuse_without_exposing_values() {
        let error = resolved(
            &config(""),
            Some("agent"),
            &[("GITHUB_TOKEN", "private-one"), ("GH_TOKEN", "private-two")],
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("conflicting environment aliases"));
        assert!(!error.contains("private-one") && !error.contains("private-two"));
    }

    #[test]
    fn present_blank_canonical_never_uses_alias() {
        for value in ["", " ", "\t\n"] {
            assert!(
                resolved(
                    &config(""),
                    Some("agent"),
                    &[
                        ("MACHINE_GITHUB_TOKEN", value),
                        ("GITHUB_TOKEN", "synthetic")
                    ]
                )
                .is_err()
            );
        }
    }

    #[test]
    fn missing_and_blank_aliases_fail_required_but_optional_absence_is_omitted() {
        assert!(resolved(&config(""), Some("agent"), &[]).is_err());
        assert!(resolved(&config(""), Some("agent"), &[("GITHUB_TOKEN", " ")]).is_err());
        let mut cfg = config("");
        cfg.secrets.get_mut("MACHINE").unwrap().required = Some(false);
        assert!(resolved(&cfg, Some("agent"), &[]).unwrap().is_empty());
        assert!(
            resolved(
                &cfg,
                Some("agent"),
                &[("MACHINE_GITHUB_TOKEN", ""), ("GITHUB_TOKEN", "synthetic")]
            )
            .unwrap()
            .is_empty()
        );
    }

    #[test]
    fn configured_placeholder_rejected_but_delegated_opaque_value_preserved() {
        for name in ["MACHINE_GITHUB_TOKEN", "GITHUB_TOKEN"] {
            assert!(resolved(&config(""), Some("agent"), &[(name, "example-only")]).is_err());
            assert_eq!(
                resolved(
                    &config(""),
                    Some("agent"),
                    &[(name, "$MSB_synthetic_delegated")]
                )
                .unwrap()["MACHINE_GITHUB_TOKEN"],
                "$MSB_synthetic_delegated"
            );
        }
    }

    #[test]
    fn unrelated_required_secrets_and_literal_aliases_are_not_consumed() {
        let cfg = config(
            r#"
UNRELATED_LITERAL = "not-a-secret-binding"
[secrets.UNRELATED_LITERAL]
[secrets.UNRELATED_REQUIRED]
"#,
        );
        assert_eq!(
            resolved(&cfg, Some("agent"), &[("GITHUB_TOKEN", "synthetic")])
                .unwrap()
                .len(),
            1
        );
        assert!(resolved(&cfg, None, &[("GITHUB_TOKEN", "synthetic")]).is_err());
        assert!(resolved(&cfg, Some("missing"), &[]).is_err());
    }

    #[test]
    fn canonical_collision_cannot_be_reinterpreted_as_an_alias() {
        let cfg = config(
            r#"
OTHER = { secret = "GITHUB_TOKEN", bound = "guest" }
[secrets.GITHUB_TOKEN]
"#,
        );
        let error = resolved(&cfg, Some("agent"), &[("GITHUB_TOKEN", "synthetic-other")])
            .unwrap_err()
            .to_string();
        assert!(error.contains("ambiguous environment alias"));
        let values = resolved(
            &cfg,
            Some("agent"),
            &[
                ("MACHINE_GITHUB_TOKEN", "synthetic-machine"),
                ("GITHUB_TOKEN", "synthetic-other"),
            ],
        )
        .unwrap();
        assert_eq!(values.len(), 2);
        assert_eq!(values["GITHUB_TOKEN"], "synthetic-other");
    }

    #[test]
    fn separate_workload_alias_namespaces_do_not_leak_credentials() {
        let cfg = config(
            r#"
[secrets.SECOND]
[workloads.second]
kind = "service"
image = { recipe = "registry", ref = "synthetic:latest" }
[workloads.second.env]
GITHUB_TOKEN = { secret = "SECOND", bound = "host" }
"#,
        );
        assert_eq!(
            resolved(&cfg, Some("agent"), &[("GITHUB_TOKEN", "synthetic")])
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            resolved(&cfg, Some("second"), &[("GITHUB_TOKEN", "synthetic")]).unwrap()["SECOND"],
            "synthetic"
        );
        assert!(
            resolved(&cfg, None, &[("GITHUB_TOKEN", "synthetic")])
                .unwrap_err()
                .to_string()
                .contains("ambiguous")
        );
    }

    #[test]
    fn credentials_without_environment_bindings_still_require_declared_material() {
        let cfg = config(
            r#"
[secrets.SSH_KEY]
[secrets.SIGN_KEY]
[credentials.ssh.login]
material = "SSH_KEY"
hosts = ["example.invalid"]
users = ["git"]
[credentials.signing.ssh.sign]
material = "SIGN_KEY"
namespace = "git"
[workloads.agent.credentials]
ssh = ["login"]
signing = ["sign"]
"#,
        );
        assert!(resolved(&cfg, Some("agent"), &[("GITHUB_TOKEN", "synthetic")]).is_err());
        let values = resolved(
            &cfg,
            Some("agent"),
            &[
                ("GITHUB_TOKEN", "synthetic"),
                ("SSH_KEY", "synthetic-key"),
                ("SIGN_KEY", "synthetic-sign"),
            ],
        )
        .unwrap();
        assert_eq!(values.len(), 3);
        assert_eq!(cfg.credentials.signing.ssh["sign"].namespace, "git");
    }

    #[test]
    fn shared_canonical_source_is_resolved_once_for_multiple_definitions() {
        let cfg = config(
            r#"
SECOND_ALIAS = { secret = "SECOND", bound = "guest" }
[secrets.SECOND]
env_var = "MACHINE_GITHUB_TOKEN"
"#,
        );
        let values = resolved(&cfg, Some("agent"), &[("SECOND_ALIAS", "synthetic")]).unwrap();
        assert_eq!(values.len(), 1);
        assert_eq!(values["MACHINE_GITHUB_TOKEN"], "synthetic");
    }

    #[test]
    fn only_declared_names_are_queried_and_custom_source_does_not_lookup_secret_id() {
        let cfg = config("");
        let names = std::cell::RefCell::new(Vec::new());
        let _ = resolve(&cfg, Some("agent"), &|name| {
            names.borrow_mut().push(name.to_string());
            Ok((name == "GITHUB_TOKEN").then(|| "synthetic".into()))
        })
        .unwrap();
        assert_eq!(
            *names.borrow(),
            vec!["MACHINE_GITHUB_TOKEN", "GH_TOKEN", "GITHUB_TOKEN"]
        );
    }
}
