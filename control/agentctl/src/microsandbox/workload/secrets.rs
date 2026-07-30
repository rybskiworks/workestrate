use crate::config::{ConfigFile, WorkloadConfig};
use crate::microsandbox::plan::{EnvVar, HostBoundSecret};
use crate::microsandbox::secrets::{RemappedSecret, SecretDefinition};
use anyhow::Result;
use std::collections::HashMap;

/// Resolved secret definition: direct (has its own env_var) or remapped
/// (exposes another secret under a different name).
#[derive(Debug, Clone)]
pub(super) enum ResolvedSecret {
    Direct(SecretDefinition),
    Remapped(RemappedSecret),
}

pub(super) fn build_secret_definitions(
    config: &ConfigFile,
) -> Result<HashMap<String, ResolvedSecret>> {
    let mut direct: HashMap<String, SecretDefinition> = HashMap::new();
    for (name, secret) in &config.secrets {
        if let Some(ref env_var) = secret.env_var {
            direct.insert(
                name.clone(),
                SecretDefinition {
                    env_var: env_var.clone(),
                    hosts: secret.hosts.clone().unwrap_or_default(),
                    required: secret.required.unwrap_or(true),
                    placeholder: secret.placeholder.clone(),
                },
            );
        }
    }

    let mut resolved: HashMap<String, ResolvedSecret> = HashMap::new();
    for (name, secret) in &config.secrets {
        if let Some(ref env_var) = secret.env_var {
            let def = direct
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("secret '{}' env_var '{}' missing", name, env_var))?
                .clone();
            resolved.insert(name.clone(), ResolvedSecret::Direct(def));
        } else if let Some(ref source_name) = secret.source {
            let source = direct
                .get(source_name)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "secret '{}' source '{}' not found or not a direct secret",
                        name,
                        source_name
                    )
                })
                .cloned()?;
            let exposed_as = secret
                .exposed_as
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("remapped secret '{}' has no exposed_as", name))?;
            resolved.insert(
                name.clone(),
                ResolvedSecret::Remapped(RemappedSecret {
                    source,
                    exposed_as: exposed_as.to_string(),
                }),
            );
        } else {
            anyhow::bail!("secret '{}' has neither env_var nor source", name);
        }
    }
    Ok(resolved)
}

pub(super) fn build_env(
    workload: &WorkloadConfig,
    secrets: &HashMap<String, ResolvedSecret>,
) -> Result<Vec<EnvVar>> {
    let mut env = Vec::new();
    for e in &workload.env {
        match (&e.value, &e.secret) {
            (Some(value), None) => env.push(EnvVar::literal(&e.name, value)),
            (None, Some(secret_name)) => {
                let resolved = secrets.get(secret_name).ok_or_else(|| {
                    anyhow::anyhow!(
                        "env '{}' references undefined secret '{}'",
                        e.name,
                        secret_name
                    )
                })?;
                match resolved {
                    ResolvedSecret::Direct(def) => env.push(EnvVar {
                        name: e.name.clone(),
                        value: format!("${{{}}}", def.env_var),
                        is_secret: true,
                        reject_placeholder: def.placeholder.clone(),
                        injected_by: None,
                    }),
                    ResolvedSecret::Remapped(_) => {
                        anyhow::bail!(
                            "env '{}' cannot reference remapped secret '{}'",
                            e.name,
                            secret_name
                        )
                    }
                }
            }
            (None, None) => env.push(EnvVar::literal(&e.name, "")),
            (Some(_), Some(_)) => {
                anyhow::bail!("env '{}' cannot have both value and secret", e.name)
            }
        }
    }
    Ok(env)
}

pub(super) fn build_secret_env(
    workload: &WorkloadConfig,
    secrets: &HashMap<String, ResolvedSecret>,
) -> Result<Vec<HostBoundSecret>> {
    let mut secret_env = Vec::new();
    for se in &workload.secret_env {
        let resolved = secrets.get(&se.secret).ok_or_else(|| {
            anyhow::anyhow!("secret_env references undefined secret '{}'", se.secret)
        })?;
        match resolved {
            ResolvedSecret::Direct(def) => secret_env.push(HostBoundSecret::from(def)),
            ResolvedSecret::Remapped(remap) => secret_env.push(HostBoundSecret::remapped(remap)),
        }
    }
    Ok(secret_env)
}

/// Map each rendered secret-bearing env name to its `[secrets.NAME]`
/// definition name (WP6(b)/A5).
///
/// For `secret_env` entries the rendered name is the secret's exposed name
/// (its own `env_var` for a direct secret, `exposed_as` for a remapped one)
/// and the def name is `se.secret`. For secret-backed `env` entries
/// (`e.secret = "NAME"`) the rendered name is `e.name` and the def name is
/// `e.secret` (env entries can only reference direct secrets — remapped
/// references bail in `build_env`).
pub(super) fn build_secret_def_name_map(
    workload: &WorkloadConfig,
    secrets: &HashMap<String, ResolvedSecret>,
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for se in &workload.secret_env {
        if let Some(resolved) = secrets.get(&se.secret) {
            let rendered_name = match resolved {
                ResolvedSecret::Direct(def) => def.env_var.clone(),
                ResolvedSecret::Remapped(remap) => remap.exposed_as.clone(),
            };
            map.insert(rendered_name, se.secret.clone());
        }
    }
    for e in &workload.env {
        if let Some(ref secret_name) = e.secret {
            map.insert(e.name.clone(), secret_name.clone());
        }
    }
    map
}

/// Resolve the provenance layer for a rendered secret line (WP6(b)/A5).
///
/// Lookup order:
/// 1. Secret-load provenance (`get_secret_provenance()`), keyed by the
///    rendered ENV VAR name (e.g. LITELLM_MASTER_KEY).
/// 2. Merge provenance `workloads.{wl}.secret_env.{SECRET_DEF_NAME}` — keyed
///    by the SECRET DEF NAME so remapped secrets (LITELLM_AUTH exposed as
///    OPENAI_API_KEY) resolve to the layer that actually bound them, instead
///    of falling through to "core" when keyed by the exposed name.
/// 3. `default` ("core").
pub(super) fn secret_line_source<'a>(
    rendered_name: &str,
    secret_def_names: &HashMap<String, String>,
    workload_name: &str,
    provenance: Option<&'a crate::merge::Provenance>,
    secret_prov: Option<&'a crate::merge::Provenance>,
    default: &'a str,
) -> &'a str {
    if let Some(layer) = secret_prov.and_then(|p| p.get(rendered_name)) {
        return layer.as_str();
    }
    if let Some(def_name) = secret_def_names.get(rendered_name) {
        let key = format!("workloads.{workload_name}.secret_env.{def_name}");
        if let Some(layer) = provenance.and_then(|p| p.get(&key)) {
            return layer.as_str();
        }
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

    // ---- WP6(b)/A5: secret provenance resolves by SECRET DEF NAME ----

    /// A5 before/after demonstration:
    /// - BEFORE (keyed by exposed name): provenance lookup
    ///   `workloads.odysseus.secret_env.OPENAI_API_KEY` misses → "core".
    /// - AFTER (keyed by secret-def name):
    ///   `workloads.odysseus.secret_env.LITELLM_AUTH` hits → the true layer.
    #[test]
    fn secret_provenance_resolves_remapped_secret_by_def_name() -> Result<()> {
        // Base layer defines the direct secret + binds LITELLM_AUTH on odysseus;
        // the team layer redefines LITELLM_AUTH (changes exposed_as).
        let base = crate::merge::Layer::from_string(
            "base",
            "schema_version = 1\n\n[secrets.LITELLM_MASTER_KEY]\nenv_var = \"LITELLM_MASTER_KEY\"\n\n[secrets.LITELLM_AUTH]\nsource = \"LITELLM_MASTER_KEY\"\nexposed_as = \"OPENAI_API_KEY\"\n\n[workloads.odysseus]\nkind = \"service\"\nimage = { recipe = \"registry\", ref = \"python:3.12-slim\" }\ncommand = []\n\n[[workloads.odysseus.secret_env]]\nsecret = \"LITELLM_AUTH\"\n\n[workloads.odysseus.network]\ndefault_deny = true",
        )?;
        let team = crate::merge::Layer::from_string(
            "team",
            "schema_version = 1\n\n[secrets.LITELLM_AUTH]\nsource = \"LITELLM_MASTER_KEY\"\nexposed_as = \"OPENAI_API_KEY\"",
        )?;

        let (config, provenance) = crate::merge::merge_layers(&[base, team])?;
        let secrets = build_secret_definitions(&config)?;
        let workload = config.workloads.get("odysseus").unwrap().clone();
        let secret_env = build_secret_env(&workload, &secrets)?;
        let def_names = build_secret_def_name_map(&workload, &secrets);

        // The rendered HostBoundSecret carries the EXPOSED name.
        let rendered = secret_env
            .iter()
            .find(|se| se.name == "OPENAI_API_KEY")
            .expect("remapped secret should render under its exposed name");

        // BEFORE: the buggy lookup keyed by the exposed name misses and falls
        // back to "core".
        let before = provenance
            .get("workloads.odysseus.secret_env.OPENAI_API_KEY")
            .map(|s| s.as_str())
            .unwrap_or("core");
        assert_eq!(
            before, "core",
            "BEFORE: keying by exposed name misses merge provenance → 'core'"
        );

        // AFTER: the helper resolves via the secret-def name → the layer that
        // bound the secret_env entry.
        let after = secret_line_source(
            &rendered.name,
            &def_names,
            "odysseus",
            Some(&provenance),
            None,
            "core",
        );
        assert_eq!(
            after, "base",
            "AFTER: keying by secret-def name (LITELLM_AUTH) yields the true layer"
        );
        Ok(())
    }

    #[test]
    fn secret_provenance_prefers_secret_load_provenance_by_env_name() {
        // secret-load provenance (keyed by env var name) wins over the merge
        // fallback when both are present.
        let mut secret_prov: crate::merge::Provenance = HashMap::new();
        secret_prov.insert("OPENAI_API_KEY".to_string(), "user-global".to_string());
        let mut prov: crate::merge::Provenance = HashMap::new();
        prov.insert(
            "workloads.odysseus.secret_env.LITELLM_AUTH".to_string(),
            "team".to_string(),
        );
        let mut def_names: HashMap<String, String> = HashMap::new();
        def_names.insert("OPENAI_API_KEY".to_string(), "LITELLM_AUTH".to_string());

        let src = secret_line_source(
            "OPENAI_API_KEY",
            &def_names,
            "odysseus",
            Some(&prov),
            Some(&secret_prov),
            "core",
        );
        assert_eq!(src, "user-global");
    }

    #[test]
    fn secret_provenance_falls_back_to_core_when_unrecorded() {
        let def_names: HashMap<String, String> = HashMap::new();
        let src = secret_line_source("SOME_KEY", &def_names, "pi", None, None, "core");
        assert_eq!(src, "core");
    }
}
