use crate::config::{Bound, ConfigFile, EnvBinding, WorkloadConfig};
use crate::microsandbox::plan::{EnvVar, HostBoundSecret};
use crate::microsandbox::secrets::SecretDefinition;
use anyhow::Result;
use std::collections::HashMap;

/// Resolve every merged `[secrets.<NAME>]` def into a [`SecretDefinition`].
///
/// Field defaults are applied HERE — post-merge only (spec 16 §5
/// defaults-after-merge): `env_var` absent defaults `source_env_var` to the
/// secret ID; `allowed_hosts` absent defaults to deny-all (an explicit zero
/// allowed hosts); `required` defaults to true.
pub(crate) fn build_secret_definitions(
    config: &ConfigFile,
) -> Result<HashMap<String, SecretDefinition>> {
    let mut resolved = HashMap::new();
    for (name, secret) in &config.secrets {
        resolved.insert(
            name.clone(),
            SecretDefinition {
                source_env_var: secret.env_var.clone().unwrap_or_else(|| name.clone()),
                allowed_hosts: secret.allowed_hosts.clone().unwrap_or_default(),
                required: secret.required.unwrap_or(true),
                placeholder: secret.placeholder.clone(),
            },
        );
    }
    Ok(resolved)
}

/// Single ordered pass over the workload's env bindings (spec 16 §4):
/// literals become plan env entries; secret bindings dispatch on the
/// BINDING's `bound` — `Some(Bound::Guest)` produces a real-value is-secret
/// plan env entry templated on the definition's `source_env_var` (the P0
/// resolution path in runtime/run.rs applies unchanged), anything else
/// (`None` or `Some(Bound::Host)` — the default applies here, post-merge)
/// produces a host-bound plan secret entry keyed by the MAP KEY carrying the
/// DEF's `allowed_hosts` / `required` / `placeholder`. An unknown secret
/// reference is a hard error naming the binding key and the secret ID.
pub(super) fn build_env_and_secret_env(
    workload: &WorkloadConfig,
    secrets: &HashMap<String, SecretDefinition>,
) -> Result<(Vec<EnvVar>, Vec<HostBoundSecret>)> {
    let mut env = Vec::new();
    let mut secret_env = Vec::new();
    for (name, binding) in workload.env.iter() {
        match binding {
            EnvBinding::Literal(value) => env.push(EnvVar::literal(name, value)),
            EnvBinding::Secret(ref_) => {
                let def = secrets.get(&ref_.secret).ok_or_else(|| {
                    anyhow::anyhow!(
                        "env '{}' references undefined secret '{}'",
                        name,
                        ref_.secret
                    )
                })?;
                match ref_.bound {
                    Some(Bound::Guest) => env.push(EnvVar {
                        name: name.clone(),
                        value: format!("${{{}}}", def.source_env_var),
                        is_secret: true,
                        reject_placeholder: def.placeholder.clone(),
                        injected_by: None,
                        injected_port: None,
                    }),
                    _ => secret_env.push(HostBoundSecret {
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

/// Resolve the provenance layer for a rendered secret line.
///
/// Lookup order:
/// 1. Secret-load provenance (`get_secret_provenance()`), keyed by the
///    rendered/exposed name (e.g. GITHUB_TOKEN).
/// 2. Merge provenance keyed by the BINDING SITE
///    `workloads.{wl}.env.{NAME}` (merge.rs records env provenance per key).
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
            "schema_version = 1\n\n{secret_defs}\n[workloads.pi]\nkind = \"agent\"\nimage = {{ recipe = \"registry\", ref = \"node:24\" }}\ncommand = []\n{workload}\n\n[workloads.pi.network.defaults]\negress = \"deny\""
        )
    }

    // ---- build_secret_definitions ----

    #[test]
    fn build_secret_definitions_resolves_defaults() -> Result<()> {
        // env_var absent → source_env_var = secret ID; allowed_hosts absent
        // → deny-all; required → true.
        let layer =
            crate::merge::Layer::from_string("base", &defs_toml("[secrets.GITHUB_TOKEN]\n", ""))?;
        let (config, _) = crate::merge::merge_layers(&[layer])?;
        let secrets = build_secret_definitions(&config)?;
        let def = &secrets["GITHUB_TOKEN"];
        assert_eq!(def.source_env_var, "GITHUB_TOKEN");
        assert!(
            def.allowed_hosts.is_empty(),
            "omitted allowed_hosts resolve to deny-all"
        );
        assert!(def.required);
        Ok(())
    }

    // ---- build_env_and_secret_env: per-binding bound dispatch ----

    /// A literal binding renders a plain plan env entry; a guest-bound
    /// secret binding renders a real-value is-secret env entry templated on
    /// the definition's source env var.
    #[test]
    fn guest_bound_produces_is_secret_plan_env_entry() -> Result<()> {
        let layer = crate::merge::Layer::from_string(
            "base",
            &defs_toml(
                "[secrets.LITELLM_MASTER_KEY]\n",
                "env = { PORT = \"4000\", MASTER = { secret = \"LITELLM_MASTER_KEY\", bound = \"guest\" } }",
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
        assert!(env[1].reject_placeholder.is_none());
        assert!(env[1].injected_by.is_none());
        Ok(())
    }

    /// A same-name guest sugar (`KEY = { bound = "guest" }` — the key-name
    /// default filled `secret` at parse) renders the same real-value env
    /// entry under the map key.
    #[test]
    fn guest_bound_sugar_renders_real_value_under_map_key() -> Result<()> {
        let layer = crate::merge::Layer::from_string(
            "base",
            &defs_toml(
                "[secrets.LITELLM_MASTER_KEY]\nplaceholder = \"CHANGEME\"\n",
                "env = { LITELLM_MASTER_KEY = { bound = \"guest\" } }",
            ),
        )?;
        let (config, _) = crate::merge::merge_layers(&[layer])?;
        let secrets = build_secret_definitions(&config)?;
        let workload = config.workloads.get("pi").unwrap();
        let (env, secret_env) = build_env_and_secret_env(workload, &secrets)?;

        assert!(secret_env.is_empty());
        assert_eq!(env.len(), 1);
        assert_eq!(env[0].name, "LITELLM_MASTER_KEY");
        assert_eq!(env[0].value, "${LITELLM_MASTER_KEY}");
        assert!(env[0].is_secret);
        assert_eq!(env[0].reject_placeholder.as_deref(), Some("CHANGEME"));
        Ok(())
    }

    /// A bound-less (`KEY = true` / `{ secret = "ID" }`) or explicit
    /// host-bound secret binding renders a plan secret_env entry keyed by
    /// the MAP KEY, carrying the DEF's allowed_hosts / required /
    /// placeholder (never the binding's — `allowed_hosts` is credential
    /// material).
    #[test]
    fn host_bound_produces_plan_secret_env_entry() -> Result<()> {
        for workload in [
            "env = { GITHUB_TOKEN = true }",
            "env = { GITHUB_TOKEN = { secret = \"GITHUB_TOKEN\" } }",
            "env = { GITHUB_TOKEN = { secret = \"GITHUB_TOKEN\", bound = \"host\" } }",
        ] {
            let layer = crate::merge::Layer::from_string(
                "base",
                &defs_toml(
                    "[secrets.GITHUB_TOKEN]\nallowed_hosts = [\"github.com\"]\nrequired = false\nplaceholder = \"ghp_CHANGEME\"\n",
                    workload,
                ),
            )?;
            let (config, _) = crate::merge::merge_layers(&[layer])?;
            let secrets = build_secret_definitions(&config)?;
            let wl = config.workloads.get("pi").unwrap();
            let (env, secret_env) = build_env_and_secret_env(wl, &secrets)?;

            assert!(env.is_empty(), "no env entries for {workload}");
            assert_eq!(secret_env.len(), 1);
            let se = &secret_env[0];
            assert_eq!(se.name, "GITHUB_TOKEN");
            assert_eq!(se.value, "${GITHUB_TOKEN}");
            assert_eq!(se.allowed_hosts, vec!["github.com".to_string()]);
            assert!(!se.required);
            assert_eq!(se.reject_placeholder.as_deref(), Some("ghp_CHANGEME"));
        }
        Ok(())
    }

    /// A renamed host-bound binding keys the plan secret entry by the MAP
    /// KEY (the exposed name), while the value templates on the DEF's
    /// source env var.
    #[test]
    fn host_bound_rename_keys_plan_entry_by_map_key() -> Result<()> {
        let layer = crate::merge::Layer::from_string(
            "base",
            &defs_toml(
                "[secrets.LITELLM_MASTER_KEY]\n",
                "env = { OPENAI_API_KEY = { secret = \"LITELLM_MASTER_KEY\" } }",
            ),
        )?;
        let (config, _) = crate::merge::merge_layers(&[layer])?;
        let secrets = build_secret_definitions(&config)?;
        let workload = config.workloads.get("pi").unwrap();
        let (env, secret_env) = build_env_and_secret_env(workload, &secrets)?;

        assert!(env.is_empty());
        assert_eq!(secret_env.len(), 1);
        assert_eq!(secret_env[0].name, "OPENAI_API_KEY");
        assert_eq!(secret_env[0].value, "${LITELLM_MASTER_KEY}");
        assert!(
            secret_env[0].allowed_hosts.is_empty(),
            "def omitted allowed_hosts → deny-all"
        );
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

    // ---- secret provenance keyed by the binding site ----

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

    /// A direct binding attributes to the layer that declared it via the
    /// binding-site merge provenance (`workloads.{wl}.env.{NAME}`).
    #[test]
    fn secret_provenance_resolves_binding_site_from_merge_provenance() -> Result<()> {
        let base = crate::merge::Layer::from_string(
            "base",
            &defs_toml(
                "[secrets.GITHUB_TOKEN]\nallowed_hosts = [\"github.com\"]\n",
                "env = { GITHUB_TOKEN = true }",
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
