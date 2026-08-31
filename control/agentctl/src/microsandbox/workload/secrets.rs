use crate::config::{
    Bound, ConfigFile, EnvBinding, SecretViolationPolicy, SecretsPolicyFragment, WorkloadConfig,
};
use crate::merge::Provenance;
use crate::microsandbox::plan::{EnvVar, HostBoundSecret};
use crate::microsandbox::secrets::SecretDefinition;
use anyhow::Result;
use std::collections::HashMap;

/// Walk the secret violation-policy ladder for ONE secret and return
/// `(effective policy, deciding-rung origin)`.
///
/// `rungs` are the collected rungs 2-4 in authority-ASCENDING order (home
/// registry first, then config-repo layers in stack order, then the
/// workload capsule's rungs). Each rung's `on_violation` (when present)
/// becomes the effective value; `final = true` is a terminal freeze — the
/// walk stops and every lower rung, including the per-secret entry, is
/// frozen out (mount-policy vocabulary,
/// docs/mount-policy/03-hierarchy-and-precedence.md). A final rung without
/// `on_violation` freezes the value resolved so far (deviation documented
/// on [`SecretsPolicyFragment`]). Rung 5 (the merged per-secret
/// `on_violation`) decides last unless frozen out; absent everywhere, the
/// built-in default (passthrough) stands.
pub(crate) fn resolve_on_violation(
    rungs: &[(&str, &SecretsPolicyFragment)],
    per_secret: Option<SecretViolationPolicy>,
    per_secret_origin: Option<&str>,
) -> (SecretViolationPolicy, String) {
    let mut effective = SecretViolationPolicy::Passthrough;
    let mut origin = "built-in".to_string();
    for (label, fragment) in rungs {
        if let Some(value) = fragment.on_violation {
            effective = value;
            origin = (*label).to_string();
        }
        if fragment.r#final {
            return (effective, origin);
        }
    }
    if let Some(value) = per_secret {
        effective = value;
        origin = per_secret_origin.unwrap_or("secret-entry").to_string();
    }
    (effective, origin)
}

/// Resolve every merged `[secrets.<NAME>]` def into a [`SecretDefinition`].
///
/// Field defaults are applied HERE — post-merge only (spec 16 §5
/// defaults-after-merge): `env_var` absent defaults `source_env_var` to the
/// secret ID; `allowed_hosts` absent defaults to deny-all (an explicit zero
/// allowed hosts); `required` defaults to true; `on_violation` absent
/// defaults to passthrough. The `on_violation` applied here is rung 5 of
/// the secret violation-policy ladder (the per-secret entry, the most
/// specific rung) — [`apply_secret_policy_ladder`] resolves the FULL ladder
/// (collected rungs 2-4 above it) afterwards, overwriting these values.
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
                on_violation: secret.on_violation.unwrap_or_default(),
            },
        );
    }
    Ok(resolved)
}

/// Overwrite each secret's violation policy with the LADDER-RESOLVED value
/// and record the deciding rung in the merge provenance (key
/// `secrets.<NAME>.on_violation.resolved`). Called from
/// `ConfigWorkload::new` right after [`build_secret_definitions`]: the
/// definitions built from the merged config carry rung 5 (the per-secret
/// entry, defaulting to passthrough); this pass walks the collected rungs
/// 2-4 (home registry, config-repo layers, this workload's capsule rungs —
/// from the process-global stored at load time) above them. An empty or
/// absent ladder degrades to the pre-ladder behavior (per-secret entry or
/// built-in passthrough), so synthetic/test paths without a load stay
/// correct.
pub(crate) fn apply_secret_policy_ladder(
    secrets: &mut HashMap<String, SecretDefinition>,
    config: &ConfigFile,
    workload_name: &str,
    provenance: &mut Provenance,
) {
    let ladder = crate::merge::get_secret_policy_ladder().unwrap_or_default();
    let mut rungs: Vec<(String, SecretsPolicyFragment)> = Vec::new();
    if let Some((origin, fragment)) = &ladder.home {
        rungs.push((origin.clone(), fragment.clone()));
    }
    for (origin, fragment) in &ladder.layers {
        rungs.push((origin.clone(), fragment.clone()));
    }
    if let Some(workload_rungs) = ladder.workloads.get(workload_name) {
        for (origin, fragment) in workload_rungs {
            rungs.push((origin.clone(), fragment.clone()));
        }
    }
    for (name, def) in secrets.iter_mut() {
        let per_secret = config.secrets.get(name).and_then(|s| s.on_violation);
        let per_secret_origin = provenance
            .get(&format!("secrets.{name}.on_violation"))
            .cloned();
        let rung_refs: Vec<(&str, &SecretsPolicyFragment)> =
            rungs.iter().map(|(o, f)| (o.as_str(), f)).collect();
        let (policy, origin) =
            resolve_on_violation(&rung_refs, per_secret, per_secret_origin.as_deref());
        def.on_violation = policy;
        provenance.insert(format!("secrets.{name}.on_violation.resolved"), origin);
    }
}

/// Single ordered pass over the workload's env bindings (spec 16 §4):
/// literals become plan env entries; secret bindings dispatch on the
/// BINDING's `bound` — `Some(Bound::Guest)` produces a real-value is-secret
/// plan env entry templated on the definition's `source_env_var` (the P0
/// resolution path in runtime/run.rs applies unchanged), anything else
/// (`None` or `Some(Bound::Host)` — the default applies here, post-merge)
/// produces a host-bound plan secret entry keyed by the MAP KEY carrying the
/// DEF's `allowed_hosts` / `required` / `placeholder` / `on_violation`. An
/// unknown secret reference is a hard error naming the binding key and the
/// secret ID.
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
                        on_violation: def.on_violation,
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
        // → deny-all; required → true; on_violation absent → passthrough.
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
        assert_eq!(
            def.on_violation,
            SecretViolationPolicy::Passthrough,
            "omitted on_violation resolves to passthrough"
        );
        Ok(())
    }

    /// Every kebab-case `on_violation` string parses to the matching
    /// [`SecretViolationPolicy`] variant (the serde naming mirrors the SDK's
    /// `ViolationAction`).
    #[test]
    fn on_violation_kebab_case_strings_parse_to_variants() -> Result<()> {
        for (toml_value, expected) in [
            ("passthrough", SecretViolationPolicy::Passthrough),
            ("block", SecretViolationPolicy::Block),
            ("block-and-log", SecretViolationPolicy::BlockAndLog),
            ("block-and-terminate", SecretViolationPolicy::BlockAndTerminate),
        ] {
            let layer = crate::merge::Layer::from_string(
                "base",
                &defs_toml(&format!("[secrets.GITHUB_TOKEN]\non_violation = \"{toml_value}\"\n"), ""),
            )?;
            let (config, _) = crate::merge::merge_layers(&[layer])?;
            let secrets = build_secret_definitions(&config)?;
            assert_eq!(
                secrets["GITHUB_TOKEN"].on_violation, expected,
                "`{toml_value}` parses to the matching variant"
            );
        }
        Ok(())
    }

    /// An unknown `on_violation` value is a parse error (the layer refuses
    /// to load — fail-closed, no silent fallback to the default).
    #[test]
    fn on_violation_invalid_value_is_parse_error() {
        let err = crate::merge::Layer::from_string(
            "base",
            &defs_toml("[secrets.GITHUB_TOKEN]\non_violation = \"nuke\"\n", ""),
        );
        assert!(err.is_err(), "invalid on_violation must fail the layer parse");
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
    /// placeholder / on_violation (never the binding's — `allowed_hosts` is
    /// credential material).
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
                    "[secrets.GITHUB_TOKEN]\nallowed_hosts = [\"github.com\"]\nrequired = false\nplaceholder = \"ghp_CHANGEME\"\non_violation = \"block-and-terminate\"\n",
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
            assert_eq!(
                se.on_violation,
                SecretViolationPolicy::BlockAndTerminate,
                "the plan entry carries the DEF's violation policy"
            );
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

    // ---- secret violation-policy ladder: resolve_on_violation ----

    /// Each authority-ascending rung overrides the previous one when no
    /// rung is final, and the per-secret entry (rung 5) — the most specific
    /// — decides last. Dropping later rungs exposes the intermediate
    /// resolutions (home only → "home-registry"; home+layer → the layer's
    /// origin).
    #[test]
    fn ladder_each_rung_overrides_the_previous() {
        let home = ("home-registry", SecretsPolicyFragment {
            on_violation: Some(SecretViolationPolicy::Block),
            r#final: false,
        });
        let layer = ("team", SecretsPolicyFragment {
            on_violation: Some(SecretViolationPolicy::BlockAndLog),
            r#final: false,
        });
        let workload = ("personal", SecretsPolicyFragment {
            on_violation: Some(SecretViolationPolicy::BlockAndTerminate),
            r#final: false,
        });

        // All four rungs present, nothing final: per-secret wins.
        let rungs = vec![
            (home.0, &home.1),
            (layer.0, &layer.1),
            (workload.0, &workload.1),
        ];
        let (policy, origin) = resolve_on_violation(
            &rungs,
            Some(SecretViolationPolicy::Passthrough),
            Some("personal#secrets.toml"),
        );
        assert_eq!(policy, SecretViolationPolicy::Passthrough);
        assert_eq!(origin, "personal#secrets.toml");

        // Home rung only (no per-secret entry): the home rung decides.
        let home_only = vec![(home.0, &home.1)];
        let (policy, origin) = resolve_on_violation(&home_only, None, None);
        assert_eq!(policy, SecretViolationPolicy::Block);
        assert_eq!(origin, "home-registry");

        // Home + layer (no per-secret entry): the layer's value and origin win.
        let home_layer = vec![(home.0, &home.1), (layer.0, &layer.1)];
        let (policy, origin) = resolve_on_violation(&home_layer, None, None);
        assert_eq!(policy, SecretViolationPolicy::BlockAndLog);
        assert_eq!(origin, "team");
    }

    /// Nothing declared anywhere — no rungs, no per-secret entry — resolves
    /// to the built-in default (passthrough, origin "built-in").
    #[test]
    fn ladder_absent_everywhere_resolves_passthrough() {
        let (policy, origin) = resolve_on_violation(&[], None, None);
        assert_eq!(policy, SecretViolationPolicy::Passthrough);
        assert_eq!(origin, "built-in");
    }

    /// A final home rung freezes the walk: config-repo layers, the workload
    /// capsule, and the per-secret entry are all frozen out.
    #[test]
    fn home_final_beats_config_workload_and_per_secret() {
        let home = ("home-registry", SecretsPolicyFragment {
            on_violation: Some(SecretViolationPolicy::Block),
            r#final: true,
        });
        let layer = ("team", SecretsPolicyFragment {
            on_violation: Some(SecretViolationPolicy::BlockAndLog),
            r#final: false,
        });
        let workload = ("personal", SecretsPolicyFragment {
            on_violation: Some(SecretViolationPolicy::BlockAndTerminate),
            r#final: false,
        });
        let rungs = vec![
            (home.0, &home.1),
            (layer.0, &layer.1),
            (workload.0, &workload.1),
        ];
        let (policy, origin) = resolve_on_violation(
            &rungs,
            Some(SecretViolationPolicy::BlockAndTerminate),
            Some("personal#secrets.toml"),
        );
        assert_eq!(policy, SecretViolationPolicy::Block);
        assert_eq!(origin, "home-registry");
    }

    /// A final config-repo-layer rung beats the workload capsule and the
    /// per-secret entry, but only after the non-final home rung applied.
    #[test]
    fn config_final_beats_workload_and_per_secret() {
        let home = ("home-registry", SecretsPolicyFragment {
            on_violation: Some(SecretViolationPolicy::Block),
            r#final: false,
        });
        let layer = ("team", SecretsPolicyFragment {
            on_violation: Some(SecretViolationPolicy::BlockAndLog),
            r#final: true,
        });
        let workload = ("personal", SecretsPolicyFragment {
            on_violation: Some(SecretViolationPolicy::BlockAndTerminate),
            r#final: false,
        });
        let rungs = vec![
            (home.0, &home.1),
            (layer.0, &layer.1),
            (workload.0, &workload.1),
        ];
        let (policy, origin) = resolve_on_violation(
            &rungs,
            Some(SecretViolationPolicy::Passthrough),
            Some("personal#secrets.toml"),
        );
        assert_eq!(policy, SecretViolationPolicy::BlockAndLog);
        assert_eq!(origin, "team");
    }

    /// A final workload-capsule rung beats the per-secret entry (and the
    /// non-final home/layer rungs applied before it).
    #[test]
    fn workload_final_beats_per_secret() {
        let home = ("home-registry", SecretsPolicyFragment {
            on_violation: Some(SecretViolationPolicy::Block),
            r#final: false,
        });
        let layer = ("team", SecretsPolicyFragment {
            on_violation: Some(SecretViolationPolicy::BlockAndLog),
            r#final: false,
        });
        let workload = ("personal", SecretsPolicyFragment {
            on_violation: Some(SecretViolationPolicy::BlockAndTerminate),
            r#final: true,
        });
        let rungs = vec![
            (home.0, &home.1),
            (layer.0, &layer.1),
            (workload.0, &workload.1),
        ];
        let (policy, origin) = resolve_on_violation(
            &rungs,
            Some(SecretViolationPolicy::Passthrough),
            Some("personal#secrets.toml"),
        );
        assert_eq!(policy, SecretViolationPolicy::BlockAndTerminate);
        assert_eq!(origin, "personal");
    }

    /// A final rung WITHOUT `on_violation` (the documented deviation from
    /// mount-policy, where every rule carries an action) freezes the value
    /// resolved SO FAR — value AND origin stay with the rung that set them.
    #[test]
    fn final_without_value_freezes_the_resolved_value() {
        let home = ("home-registry", SecretsPolicyFragment {
            on_violation: Some(SecretViolationPolicy::Block),
            r#final: false,
        });
        let layer = ("team", SecretsPolicyFragment {
            on_violation: None,
            r#final: true,
        });
        let rungs = vec![(home.0, &home.1), (layer.0, &layer.1)];
        let (policy, origin) = resolve_on_violation(
            &rungs,
            Some(SecretViolationPolicy::BlockAndTerminate),
            Some("personal#secrets.toml"),
        );
        assert_eq!(policy, SecretViolationPolicy::Block);
        assert_eq!(origin, "home-registry", "the freeze keeps the home origin");
    }

    /// With nothing above final, the per-secret entry wins; its origin is
    /// the merge-provenance layer name, falling back to "secret-entry"
    /// when no provenance was recorded.
    #[test]
    fn per_secret_wins_when_nothing_above_is_final() {
        let home = ("home-registry", SecretsPolicyFragment {
            on_violation: Some(SecretViolationPolicy::BlockAndTerminate),
            r#final: false,
        });
        let layer = ("team", SecretsPolicyFragment {
            on_violation: Some(SecretViolationPolicy::BlockAndLog),
            r#final: false,
        });
        let workload = ("personal", SecretsPolicyFragment {
            on_violation: Some(SecretViolationPolicy::BlockAndTerminate),
            r#final: false,
        });
        let rungs = vec![
            (home.0, &home.1),
            (layer.0, &layer.1),
            (workload.0, &workload.1),
        ];
        let (policy, origin) = resolve_on_violation(
            &rungs,
            Some(SecretViolationPolicy::Block),
            Some("personal#secrets.toml"),
        );
        assert_eq!(policy, SecretViolationPolicy::Block);
        assert_eq!(origin, "personal#secrets.toml");

        let (policy, origin) =
            resolve_on_violation(&rungs, Some(SecretViolationPolicy::Block), None);
        assert_eq!(policy, SecretViolationPolicy::Block);
        assert_eq!(origin, "secret-entry");
    }

    // ---- secret violation-policy ladder: schema surface ----

    /// `final` is NOT available on per-secret entries (nothing sits below
    /// them): `deny_unknown_fields` rejects the key at layer parse.
    #[test]
    fn per_secret_final_key_is_rejected() {
        let err = crate::merge::Layer::from_string(
            "base",
            &defs_toml("[secrets.GITHUB_TOKEN]\non_violation = \"block\"\nfinal = true\n", ""),
        );
        assert!(err.is_err(), "per-secret `final` must fail the layer parse");
    }

    /// `[policy.secrets]` parses with the `final` rename, and an unknown
    /// field inside the fragment is a parse error (deny_unknown_fields).
    #[test]
    fn policy_secrets_fragment_parses_with_final() {
        let layer = crate::merge::Layer::from_string(
            "base",
            r#"
schema_version = 1

[policy.secrets]
on_violation = "block-and-log"
final = true
"#,
        )
        .expect("the fragment parses");
        let fragment = layer.config.policy.secrets.expect("secrets rung collected");
        assert_eq!(
            fragment.on_violation,
            Some(SecretViolationPolicy::BlockAndLog)
        );
        assert!(fragment.r#final);

        let err = crate::merge::Layer::from_string(
            "bad-policy",
            "schema_version = 1\n[policy.secrets]\ntypo = true\n",
        )
        .err()
        .expect("unknown field must fail the layer parse")
        .to_string();
        assert!(
            err.contains("unknown field"),
            "error must name the unknown field: {err}"
        );
    }

    // ---- secret violation-policy ladder: apply_secret_policy_ladder ----

    /// End-to-end-ish: the home rung's `final` freezes out the per-secret
    /// `block-and-terminate`, the definition carries the home value, and
    /// the deciding rung is recorded in the merge provenance under
    /// `secrets.<NAME>.on_violation.resolved`.
    ///
    /// Isolation: the ladder store is process-global, so this test holds
    /// [`crate::config::test_support::PROVENANCE_STORAGE_TEST_LOCK`] (the
    /// same serialization the direct provenance-store mutators use) AND
    /// sets exactly the ladder it needs at the start — no dependence on
    /// test execution order.
    #[test]
    fn apply_secret_policy_ladder_resolves_and_records_provenance() -> Result<()> {
        let _guard = crate::config::test_support::PROVENANCE_STORAGE_TEST_LOCK
            .lock()
            .unwrap();
        crate::merge::set_secret_policy_ladder(Some(crate::merge::SecretPolicyLadder {
            home: Some((
                "home-registry".to_string(),
                SecretsPolicyFragment {
                    on_violation: Some(SecretViolationPolicy::Block),
                    r#final: true,
                },
            )),
            ..Default::default()
        }));

        let layer = crate::merge::Layer::from_string(
            "base",
            &defs_toml("[secrets.GITHUB_TOKEN]\non_violation = \"block-and-terminate\"\n", ""),
        )?;
        let (config, _) = crate::merge::merge_layers(&[layer])?;
        let mut secrets = build_secret_definitions(&config)?;
        let mut provenance = crate::merge::Provenance::new();
        provenance.insert(
            "secrets.GITHUB_TOKEN.on_violation".to_string(),
            "personal#secrets.toml".to_string(),
        );

        apply_secret_policy_ladder(&mut secrets, &config, "pi", &mut provenance);

        assert_eq!(
            secrets["GITHUB_TOKEN"].on_violation,
            SecretViolationPolicy::Block,
            "the home final froze out the per-secret block-and-terminate"
        );
        assert_eq!(
            provenance
                .get("secrets.GITHUB_TOKEN.on_violation.resolved")
                .map(|s| s.as_str()),
            Some("home-registry"),
            "the deciding rung is recorded in the provenance"
        );

        crate::merge::set_secret_policy_ladder(None);
        Ok(())
    }

    /// No stored ladder → the pre-ladder behavior: the per-secret entry
    /// (with its merge-provenance layer origin) stands, and a secret with
    /// no per-secret entry resolves to the built-in passthrough.
    ///
    /// Isolation: same process-global discipline as
    /// `apply_secret_policy_ladder_resolves_and_records_provenance` — the
    /// storage lock plus an explicit `None` ladder at the start.
    #[test]
    fn apply_secret_policy_ladder_without_ladder_keeps_merge_result() -> Result<()> {
        let _guard = crate::config::test_support::PROVENANCE_STORAGE_TEST_LOCK
            .lock()
            .unwrap();
        crate::merge::set_secret_policy_ladder(None);

        let layer = crate::merge::Layer::from_string(
            "base",
            &defs_toml(
                "[secrets.GITHUB_TOKEN]\non_violation = \"block-and-log\"\n\n[secrets.OTHER]\n",
                "",
            ),
        )?;
        let (config, mut provenance) = crate::merge::merge_layers(&[layer])?;
        let mut secrets = build_secret_definitions(&config)?;

        apply_secret_policy_ladder(&mut secrets, &config, "pi", &mut provenance);

        assert_eq!(
            secrets["GITHUB_TOKEN"].on_violation,
            SecretViolationPolicy::BlockAndLog,
            "the per-secret entry stands when no rungs are collected"
        );
        assert_eq!(
            provenance
                .get("secrets.GITHUB_TOKEN.on_violation.resolved")
                .map(|s| s.as_str()),
            Some("base"),
            "the per-secret layer origin comes from the merge provenance key"
        );
        assert_eq!(
            secrets["OTHER"].on_violation,
            SecretViolationPolicy::Passthrough,
            "no per-secret entry resolves to the built-in default"
        );
        assert_eq!(
            provenance
                .get("secrets.OTHER.on_violation.resolved")
                .map(|s| s.as_str()),
            Some("built-in"),
        );

        Ok(())
    }
}
