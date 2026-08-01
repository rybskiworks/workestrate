use crate::config::{ConfigFile, Delivery, EnvBinding, WorkloadConfig};
use crate::microsandbox::plan::{EnvVar, HostBoundSecret};
use crate::microsandbox::secrets::SecretDefinition;
use anyhow::Result;
use std::collections::HashMap;

/// Resolve every merged `[secrets.<NAME>]` def into a [`SecretDefinition`].
///
/// v2 resolution: every def resolves (there is no "neither env_var nor
/// source" bail anymore — remap defs are gone). `env_var` absent defaults
/// `source_env_var` to the secret ID; `hosts` absent defaults to deny-all
/// (an explicit zero allowed hosts); `required` defaults to true;
/// `delivery` defaults to host-bound (secure-by-default).
pub(super) fn build_secret_definitions(
    config: &ConfigFile,
) -> Result<HashMap<String, SecretDefinition>> {
    let mut resolved = HashMap::new();
    for (name, secret) in &config.secrets {
        resolved.insert(
            name.clone(),
            SecretDefinition {
                source_env_var: secret.env_var.clone().unwrap_or_else(|| name.clone()),
                allowed_hosts: secret.hosts.clone().unwrap_or_default(),
                required: secret.required.unwrap_or(true),
                placeholder: secret.placeholder.clone(),
                delivery: secret.delivery.unwrap_or(Delivery::HostBound),
            },
        );
    }
    Ok(resolved)
}

/// Single ordered pass over the workload's env bindings (P1 Wave 1):
/// literals become plan env entries; secret bindings dispatch on the
/// definition's `delivery` — `Env` produces an is-secret plan env entry
/// templated on the definition's `source_env_var` (the P0 resolution path in
/// runtime/run.rs applies unchanged), `HostBound` produces a host-bound plan
/// secret entry keyed by the MAP KEY. An unknown secret reference is a hard
/// error naming the binding key and the secret ID.
pub(super) fn build_env_and_secret_env(
    workload: &WorkloadConfig,
    secrets: &HashMap<String, SecretDefinition>,
) -> Result<(Vec<EnvVar>, Vec<HostBoundSecret>)> {
    let mut env = Vec::new();
    let mut secret_env = Vec::new();
    for (name, binding) in workload.env.iter() {
        match binding {
            EnvBinding::Literal(value) => env.push(EnvVar::literal(name, value)),
            EnvBinding::Secret(id) => {
                let def = secrets.get(id).ok_or_else(|| {
                    anyhow::anyhow!("env '{}' references undefined secret '{}'", name, id)
                })?;
                match def.delivery {
                    Delivery::Env => env.push(EnvVar {
                        name: name.clone(),
                        value: format!("${{{}}}", def.source_env_var),
                        is_secret: true,
                        reject_placeholder: def.placeholder.clone(),
                        injected_by: None,
                    }),
                    Delivery::HostBound => secret_env.push(HostBoundSecret {
                        name: name.clone(),
                        value: format!("${{{}}}", def.source_env_var),
                        allowed_hosts: def.allowed_hosts.clone(),
                        required: def.required,
                        reject_placeholder: def.placeholder.clone(),
                    }),
                }
            }
        }
    }
    Ok((env, secret_env))
}

/// Resolve the provenance layer for a rendered secret line (P1 Wave 1).
///
/// Lookup order:
/// 1. Secret-load provenance (`get_secret_provenance()`), keyed by the
///    rendered/exposed name (e.g. GITHUB_TOKEN).
/// 2. Merge provenance keyed by the BINDING SITE
///    `workloads.{wl}.env.{NAME}` (merge.rs records env provenance per key;
///    the v1-compat fold re-keys legacy `secret_env` provenance here).
/// 3. `default` ("core").
pub(super) fn secret_line_source<'a>(
    rendered_name: &str,
    workload_name: &str,
    provenance: Option<&'a crate::merge::Provenance>,
    secret_prov: Option<&'a crate::merge::Provenance>,
    default: &'a str,
) -> &'a str {
    if let Some(layer) = secret_prov.and_then(|p| p.get(rendered_name)) {
        return layer.as_str();
    }
    let key = format!("workloads.{workload_name}.env.{rendered_name}");
    if let Some(layer) = provenance.and_then(|p| p.get(&key)) {
        return layer.as_str();
    }
    default
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

    fn defs_toml(secret_defs: &str, workload: &str) -> String {
        format!(
            "schema_version = 2\n\n{secret_defs}\n[workloads.pi]\nkind = \"agent\"\nimage = {{ recipe = \"registry\", ref = \"node:24\" }}\ncommand = []\n{workload}\n\n[workloads.pi.network]\ndefault_deny = true"
        )
    }

    // ---- build_secret_definitions ----

    #[test]
    fn build_secret_definitions_resolves_defaults() -> Result<()> {
        // env_var absent → source_env_var = secret ID; hosts absent →
        // deny-all; required → true; delivery → HostBound.
        let layer =
            crate::merge::Layer::from_string("base", &defs_toml("[secrets.GITHUB_TOKEN]\n", ""))?;
        let (config, _) = crate::merge::merge_layers(&[layer])?;
        let secrets = build_secret_definitions(&config)?;
        let def = &secrets["GITHUB_TOKEN"];
        assert_eq!(def.source_env_var, "GITHUB_TOKEN");
        assert!(
            def.allowed_hosts.is_empty(),
            "omitted hosts resolve to deny-all"
        );
        assert!(def.required);
        assert_eq!(def.delivery, Delivery::HostBound);
        Ok(())
    }

    // ---- build_env_and_secret_env: delivery resolution ----

    /// A literal binding renders a plain plan env entry; a secret binding
    /// with `delivery = "env"` renders an is-secret env entry templated on
    /// the definition's source env var.
    #[test]
    fn delivery_env_produces_is_secret_plan_env_entry() -> Result<()> {
        let layer = crate::merge::Layer::from_string(
            "base",
            &defs_toml(
                "[secrets.LITELLM_MASTER_KEY]\ndelivery = \"env\"\n",
                "env = { PORT = \"4000\", MASTER = { secret = \"LITELLM_MASTER_KEY\" } }",
            ),
        )?;
        let (config, _) = crate::merge::merge_layers(&[layer])?;
        let secrets = build_secret_definitions(&config)?;
        let workload = config.workloads.get("pi").unwrap();
        let (env, secret_env) = build_env_and_secret_env(workload, &secrets)?;

        assert!(secret_env.is_empty(), "no host-bound entries");
        assert_eq!(env.len(), 2);
        assert_eq!(env[0].name, "PORT");
        assert_eq!(env[0].value, "4000");
        assert!(!env[0].is_secret);
        assert_eq!(env[1].name, "MASTER");
        assert_eq!(env[1].value, "${LITELLM_MASTER_KEY}");
        assert!(env[1].is_secret);
        Ok(())
    }

    /// A secret binding with the default host-bound delivery renders a plan
    /// secret_env entry keyed by the MAP KEY, carrying the def's hosts /
    /// required / placeholder.
    #[test]
    fn delivery_host_bound_produces_plan_secret_env_entry() -> Result<()> {
        let layer = crate::merge::Layer::from_string(
            "base",
            &defs_toml(
                "[secrets.GITHUB_TOKEN]\nhosts = [\"github.com\"]\nrequired = false\nplaceholder = \"ghp_CHANGEME\"\n",
                "env = { GITHUB_TOKEN = { secret = \"GITHUB_TOKEN\" } }",
            ),
        )?;
        let (config, _) = crate::merge::merge_layers(&[layer])?;
        let secrets = build_secret_definitions(&config)?;
        let workload = config.workloads.get("pi").unwrap();
        let (env, secret_env) = build_env_and_secret_env(workload, &secrets)?;

        assert!(env.is_empty(), "no env entries");
        assert_eq!(secret_env.len(), 1);
        let se = &secret_env[0];
        assert_eq!(se.name, "GITHUB_TOKEN");
        assert_eq!(se.value, "${GITHUB_TOKEN}");
        assert_eq!(se.allowed_hosts, vec!["github.com".to_string()]);
        assert!(!se.required);
        assert_eq!(se.reject_placeholder.as_deref(), Some("ghp_CHANGEME"));
        Ok(())
    }

    /// An unknown secret reference hard-errors, naming the binding key and
    /// the secret ID.
    #[test]
    fn unknown_secret_reference_hard_errors() {
        let layer = crate::merge::Layer::from_string(
            "base",
            &defs_toml("", "env = { API_KEY = { secret = \"NOPE\" } }"),
        )
        .unwrap();
        let (config, _) = crate::merge::merge_layers(&[layer]).unwrap();
        let secrets = build_secret_definitions(&config).unwrap();
        let workload = config.workloads.get("pi").unwrap();
        let err = build_env_and_secret_env(workload, &secrets).unwrap_err();
        assert_eq!(
            err.to_string(),
            "env 'API_KEY' references undefined secret 'NOPE'"
        );
    }

    // ---- P1 Wave 1: secret provenance keyed by the binding site ----

    /// Secret-load provenance (keyed by the rendered env var name) wins over
    /// the merge-provenance fallback when both are present.
    #[test]
    fn secret_provenance_prefers_secret_load_provenance_by_env_name() {
        let mut secret_prov: crate::merge::Provenance = HashMap::new();
        secret_prov.insert("GITHUB_TOKEN".to_string(), "user-global".to_string());
        let mut prov: crate::merge::Provenance = HashMap::new();
        prov.insert(
            "workloads.pi.env.GITHUB_TOKEN".to_string(),
            "team".to_string(),
        );

        let src = secret_line_source(
            "GITHUB_TOKEN",
            "pi",
            Some(&prov),
            Some(&secret_prov),
            "core",
        );
        assert_eq!(src, "user-global");
    }

    /// A direct v2 binding attributes to the layer that declared it via the
    /// binding-site merge provenance (`workloads.{wl}.env.{NAME}`).
    #[test]
    fn secret_provenance_resolves_binding_site_from_merge_provenance() -> Result<()> {
        let base = crate::merge::Layer::from_string(
            "base",
            &defs_toml(
                "[secrets.GITHUB_TOKEN]\nhosts = [\"github.com\"]\n",
                "env = { GITHUB_TOKEN = { secret = \"GITHUB_TOKEN\" } }",
            ),
        )?;
        let (config, provenance) = crate::merge::merge_layers(&[base])?;
        let secrets = build_secret_definitions(&config)?;
        let workload = config.workloads.get("pi").unwrap();
        let (_, secret_env) = build_env_and_secret_env(workload, &secrets)?;
        let rendered = secret_env
            .iter()
            .find(|se| se.name == "GITHUB_TOKEN")
            .expect("the host-bound binding renders under the map key");

        let src = secret_line_source(&rendered.name, "pi", Some(&provenance), None, "core");
        assert_eq!(
            src, "base",
            "binding-site merge provenance attributes the line to its layer"
        );
        Ok(())
    }

    /// No provenance recorded anywhere → the "core" default.
    #[test]
    fn secret_provenance_falls_back_to_core_when_unrecorded() {
        let src = secret_line_source("SOME_KEY", "pi", None, None, "core");
        assert_eq!(src, "core");
    }
}
