use std::collections::{HashMap, HashSet};

use anyhow::Result;

use crate::microsandbox::plan::SandboxPlan;
use crate::microsandbox::runtime::resolve_plan_envs;

/// Resolve `${VAR}` templates against an explicit lookup map FIRST, falling
/// back to process env for variables the map does not carry (FN-9). Secret
/// values flow through the map returned by `load_secrets`; process env is
/// only consulted for ad-hoc variables (runtime/user exports), never as a
/// read-back of secrets this process wrote.
///
/// The `$${` escape (envsubst convention) renders as a literal `${` and
/// applies to BOTH declared env values and the seed-file renderer (see
/// [`resolve_templated_value_by`]). Existing content is backward compatible:
/// no existing content uses `$${`.
pub(crate) fn resolve_templated_value_with(
    templated: &str,
    vars: &std::collections::HashMap<String, String>,
) -> Result<String> {
    resolve_templated_value_by(templated, move |name: &str| match vars.get(name) {
        Some(v) => Ok(v.clone()),
        None => Err(std::env::VarError::NotPresent),
    })
}

/// Resolve `${VAR}` templates against `vars` FIRST, falling back to process
/// env for variables the map does not carry. Used for plan `env` entries so
/// injected depends_on vars (appended to the plan by
/// `discovery::apply_resolution`, NOT present in the process env) are visible
/// to templated declared values (spec 12 §4: the templated composition is the
/// declared env consuming the injected var).
///
/// KNOWN LIMITATION: map values are RAW (unresolved) — a var referencing
/// another templated var in the map gets its raw `${...}` form; there is no
/// recursive resolution.
///
/// The `$${` escape (envsubst convention) renders as a literal `${` and
/// applies to BOTH declared env values and the seed-file renderer (see
/// [`resolve_templated_value_by`]). Existing content is backward compatible:
/// no existing content uses `$${`.
pub(crate) fn resolve_templated_value_with_env_fallback(
    templated: &str,
    vars: &std::collections::HashMap<String, String>,
) -> Result<String> {
    resolve_templated_value_by(templated, move |name: &str| match vars.get(name) {
        Some(v) => Ok(v.clone()),
        None => std::env::var(name),
    })
}

/// Shared engine: resolve `${VAR}` templates through `lookup`, with the
/// envsubst-standard `$$` escape: `$$` renders as a LITERAL `$` (so `$${`
/// renders as a literal `${`), is never resolved, and the emitted `$` is
/// never re-scanned.
///
/// Scanning is a single left-to-right pass over `templated`:
/// - `$$` is consumed as a unit and emits a literal `$` (envsubst
///   convention). When the next input char is `{` (i.e. `$${`), the `{` is
///   ordinary text, so the result is the literal `${` the caller wants — a
///   var name after an escape is never looked up, `$${FOO}` → `${FOO}` even
///   when `FOO` is in `lookup`, and an unclosed escaped token (`$${X` with
///   no closing `}`) is literal text, not an error.
/// - a plain `${VAR}` resolves through `lookup` exactly as before: missing →
///   error, `${}` (empty name) → error, unclosed `${` → error.
/// - everything else passes through unchanged.
///
/// Collapse is left-to-right and each emitted `$` is never re-scanned, so
/// `$$$${X}` → `$${X}` (`$$` → `$`, then `$${X}` → `${X}`). Applies to
/// declared env values and the seed-file renderer alike.
fn resolve_templated_value_by<F>(templated: &str, lookup: F) -> Result<String>
where
    F: for<'a> Fn(&'a str) -> std::result::Result<String, std::env::VarError>,
{
    let mut result = String::with_capacity(templated.len());
    let mut i = 0;
    while i < templated.len() {
        if templated[i..].starts_with("$$") {
            // envsubst `$$` escape: emit a literal `$` and consume BOTH
            // dollars so the emitted `$` is never re-scanned. `$${` therefore
            // yields the literal `${` (the following `{` is ordinary text).
            result.push('$');
            i += 2;
        } else if templated[i..].starts_with("${") {
            match templated[i + 2..].find('}') {
                Some(end) => {
                    let var_name = &templated[i + 2..i + 2 + end];
                    if var_name.is_empty() {
                        anyhow::bail!("empty variable name in secret template at position {}", i);
                    }
                    let value = lookup(var_name)
                        .map_err(|e| anyhow::anyhow!("missing env var {}: {}", var_name, e))?;
                    result.push_str(&value);
                    i += 2 + end + 1;
                }
                None => anyhow::bail!("unclosed ${{ in secret template at position {}", i),
            }
        } else {
            // Ordinary text: copy up to the next `$` (or the remainder).
            match templated[i..].find('$') {
                Some(offset) if offset > 0 => {
                    result.push_str(&templated[i..i + offset]);
                    i += offset;
                }
                Some(_) => {
                    // A lone `$` that is not part of an escape or template
                    // passes through unchanged.
                    result.push('$');
                    i += 1;
                }
                None => {
                    result.push_str(&templated[i..]);
                    i = templated.len();
                }
            }
        }
    }
    Ok(result)
}

/// Guest-visible env view for `template = true` seed-file rendering.
///
/// Built by [`build_seed_env_view`]; consumed by [`render_seed_text`].
pub(crate) struct SeedEnvView {
    pub(crate) vars: HashMap<String, String>,
    pub(crate) defined_secrets: HashSet<String>,
}

/// Build the guest-visible env view a `template = true` seed file renders
/// against.
///
/// The view is the EXACT environment the workload's guest process carries, so
/// rendered seed content matches the runtime byte-for-byte:
///
/// 1. declared non-secret `env` values are pre-resolved with the SAME
///    single-pass algorithm/order as `resolve_plan_envs` — env-fallback is
///    allowed for declared non-secret values, matching the runtime exactly;
/// 2. injected `depends_on` vars already live in `plan.env`
///    (`is_secret = false`, `injected_by = Some`) and flow through
///    `resolve_plan_envs` unchanged;
/// 3. host-bound secrets (`plan.secret_env` entries) become the placeholder
///    string EXACTLY as the guest carries it: `$MSB_<name>` where `name` is
///    the plan secret entry's BINDING MAP KEY (`s.name` — the same key run.rs
///    feeds into the fork's 3-arg `secret_env`; the fork's default placeholder
///    is `format!("$MSB_{env_var}")`, microsandbox-fork
///    `crates/network/lib/builder.rs`). NEVER the decrypted value, NEVER the
///    definition's `source_env_var`;
/// 4. guest-bound secrets (`plan.env` entries with `is_secret = true`) carry
///    the real decrypted value (same exposure as the guest env);
/// 5. a defined-but-unbound secret referenced in a template is a HARD ERROR
///    (surfaced by [`render_seed_text`] naming the secret and the seed file);
/// 6. a missing var is a HARD ERROR (surfaced by [`render_seed_text`] naming
///    the var and the source→target).
///
/// Resolution is map-only: NO process-env fallback for seed content beyond
/// the env-fallback `resolve_plan_envs` applies to declared non-secret values.
pub(crate) fn build_seed_env_view(
    plan: &SandboxPlan,
    secrets: &HashMap<String, String>,
    defined_secret_ids: &HashSet<String>,
) -> Result<SeedEnvView> {
    let resolved = resolve_plan_envs(plan, secrets)?;
    let mut vars = HashMap::with_capacity(resolved.len() + plan.secret_env.len());
    for (name, value) in resolved {
        vars.insert(name, value);
    }
    for s in &plan.secret_env {
        // Placeholder EXACTLY as the guest carries it (fork's SecretBuilder
        // default: `format!("$MSB_{env_var}")` with env_var = the binding key).
        vars.insert(s.name.clone(), format!("$MSB_{}", s.name));
    }
    let defined_secrets = HashSet::from_iter(defined_secret_ids.iter().cloned());
    Ok(SeedEnvView {
        vars,
        defined_secrets,
    })
}

/// Render a seed file's text against the guest-visible env view.
///
/// Map-only: `${VAR}` resolves against [`SeedEnvView::vars`] ONLY — there is
/// NO process-env fallback for seed content. Missing vars and
/// defined-but-unbound secrets are HARD errors (see [`build_seed_env_view`]
/// for the exposure contract). `file_label` is the caller's source→target
/// description, included verbatim in every error so the failing seed file is
/// unambiguous.
pub(crate) fn render_seed_text(text: &str, view: &SeedEnvView, file_label: &str) -> Result<String> {
    let result = resolve_templated_value_with(text, &view.vars);
    match result {
        Ok(rendered) => Ok(rendered),
        Err(e) => {
            let msg = e.to_string();
            if let Some(name) = missing_var_name(&msg) {
                if view.defined_secrets.contains(name) {
                    anyhow::bail!(
                        "seed file {file_label} references secret '{name}', which is defined but not bound to this workload's env; bind it (e.g. env = {{ \"{name}\" = true }}) or remove the reference"
                    );
                } else {
                    anyhow::bail!("seed file {file_label}: missing env var {name}");
                }
            } else {
                anyhow::bail!("seed file {file_label}: {msg}");
            }
        }
    }
}

/// Extract the variable name from a `resolve_templated_value_with` missing-var
/// error message (`format!("missing env var {}: {}", var_name, e)`).
fn missing_var_name(msg: &str) -> Option<&str> {
    msg.strip_prefix("missing env var ")
        .and_then(|rest| rest.split(':').next())
        .filter(|n| !n.is_empty())
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
    use crate::config::SecretViolationPolicy;
    use crate::microsandbox::plan::{EnvVar, HostBoundSecret, NetworkPlan, SandboxPlan};
    use std::collections::{HashMap, HashSet};

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    fn plan_with(env: Vec<EnvVar>, secret_env: Vec<HostBoundSecret>) -> SandboxPlan {
        SandboxPlan {
            name: "test".to_string(),
            image: None,
            workdir: None,
            command: Vec::new(),
            cpus: None,
            memory_mib: None,
            env,
            secret_env,
            ports: Vec::new(),
            mounts: Vec::new(),
            network: NetworkPlan {
                egress_default_deny: false,
                ingress_default_deny: false,
                egress_rules: Vec::new(),
                deny_rules: Vec::new(),
                ingress_rules: Vec::new(),
            },
            instance_policy: None,
            virtualization: None,
        }
    }

    fn secrets_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn substitutes_single_var() {
        let m = vars(&[("FOO", "bar")]);
        assert_eq!(resolve_templated_value_with("${FOO}", &m).unwrap(), "bar");
    }

    #[test]
    fn substitutes_var_embedded_in_text() {
        let m = vars(&[("NAME", "world")]);
        assert_eq!(
            resolve_templated_value_with("hello ${NAME}!", &m).unwrap(),
            "hello world!"
        );
    }

    #[test]
    fn substitutes_multiple_vars() {
        let m = vars(&[("A", "1"), ("B", "2")]);
        assert_eq!(
            resolve_templated_value_with("${A}-${B}-${A}", &m).unwrap(),
            "1-2-1"
        );
    }

    #[test]
    fn missing_var_is_an_error_naming_the_var() {
        let m = vars(&[]);
        let err = resolve_templated_value_with("${NOPE}", &m).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("NOPE"),
            "error should name the missing var: {msg}"
        );
    }

    #[test]
    fn empty_template_placeholder_is_an_error() {
        let m = vars(&[]);
        let err = resolve_templated_value_with("x ${} y", &m).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("empty variable name"),
            "expected empty-name error: {msg}"
        );
    }

    #[test]
    fn unclosed_placeholder_is_an_error() {
        let m = vars(&[("A", "1")]);
        let err = resolve_templated_value_with("value ${A", &m).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("unclosed"),
            "expected unclosed-template error: {msg}"
        );
    }

    #[test]
    fn plain_text_passes_through_unchanged() {
        let m = vars(&[]);
        assert_eq!(
            resolve_templated_value_with("no templates here", &m).unwrap(),
            "no templates here"
        );
        assert_eq!(resolve_templated_value_with("", &m).unwrap(), "");
    }

    #[test]
    fn dollar_without_brace_is_not_a_template() {
        let m = vars(&[("A", "1")]);
        assert_eq!(
            resolve_templated_value_with("costs $5 or $A", &m).unwrap(),
            "costs $5 or $A"
        );
    }

    #[test]
    fn env_fallback_prefers_map_over_process_env() {
        let m = vars(&[("LITELLM_ADDR", "host.microsandbox.internal:4000")]);
        assert_eq!(
            resolve_templated_value_with_env_fallback("http://${LITELLM_ADDR}/v1", &m).unwrap(),
            "http://host.microsandbox.internal:4000/v1"
        );
    }

    #[test]
    fn env_fallback_uses_process_env_when_map_lacks_the_var() {
        // A name the map does not carry falls back to the process env.
        let unique = "WORKESTRATE_TEST_ENV_FALLBACK_VAR";
        std::env::set_var(unique, "from-process-env");
        let m = vars(&[]);
        let templated = format!("${{{}}}", unique);
        assert_eq!(
            resolve_templated_value_with_env_fallback(&templated, &m).unwrap(),
            "from-process-env"
        );
        std::env::remove_var(unique);
    }

    #[test]
    fn env_fallback_errors_when_neither_map_nor_process_env_has_the_var() {
        let m = vars(&[]);
        let err = resolve_templated_value_with_env_fallback("${NOPE_NEVER_SET}", &m).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("NOPE_NEVER_SET"),
            "error should name the missing var: {msg}"
        );
    }

    // ---- `$${` escape (envsubst convention) ----

    #[test]
    fn escape_renders_literal_dollar_brace_even_when_var_in_map() {
        let m = vars(&[("FOO", "bar")]);
        assert_eq!(
            resolve_templated_value_with("$${FOO}", &m).unwrap(),
            "${FOO}"
        );
    }

    #[test]
    fn escape_litellm_master_key_passthrough() {
        let m = vars(&[("LITELLM_MASTER_KEY", "s3cr3t")]);
        assert_eq!(
            resolve_templated_value_with("$${LITELLM_MASTER_KEY}", &m).unwrap(),
            "${LITELLM_MASTER_KEY}"
        );
    }

    #[test]
    fn mixed_escape_and_resolution() {
        let m = vars(&[("X", "must-not-appear"), ("Y", "<Y-value>")]);
        assert_eq!(
            resolve_templated_value_with("a $${X} b ${Y} c", &m).unwrap(),
            "a ${X} b <Y-value> c"
        );
    }

    #[test]
    fn plain_template_unchanged() {
        let m = vars(&[("X", "plain-value")]);
        assert_eq!(
            resolve_templated_value_with("${X}", &m).unwrap(),
            "plain-value"
        );
    }

    #[test]
    fn double_escape_collapses_once() {
        let m = vars(&[("X", "must-not-appear")]);
        assert_eq!(
            resolve_templated_value_with("$$$${X}", &m).unwrap(),
            "$${X}"
        );
    }

    #[test]
    fn escape_wins_over_unclosed() {
        let m = vars(&[("X", "must-not-appear")]);
        assert_eq!(resolve_templated_value_with("$${X", &m).unwrap(), "${X");
    }

    // ---- P1.1: seed-file env view + template renderer ----

    #[test]
    fn seed_view_declared_env_presolved() -> Result<()> {
        // Declared literal plus a templated declared value consuming an
        // injected depends_on var (mirrors run.rs
        // resolve_plan_envs_sees_injected_depends_on_var).
        let plan = plan_with(
            vec![
                EnvVar::literal("APP_PORT", "8080"),
                EnvVar::literal("OPENAI_BASE_URL", "http://${LITELLM_ADDR}/v1"),
                EnvVar {
                    name: "LITELLM_ADDR".to_string(),
                    value: "host.microsandbox.internal:4000".to_string(),
                    is_secret: false,
                    reject_placeholder: None,
                    injected_by: Some("litellm".to_string()),
                    injected_port: None,
                },
            ],
            vec![],
        );
        let view = build_seed_env_view(&plan, &secrets_map(&[]), &HashSet::new())?;
        assert_eq!(
            view.vars.get("APP_PORT").map(String::as_str),
            Some("8080"),
            "declared literal must be pre-resolved into the view"
        );
        assert_eq!(
            view.vars.get("OPENAI_BASE_URL").map(String::as_str),
            Some("http://host.microsandbox.internal:4000/v1"),
            "templated declared value must consume the injected var like the runtime"
        );
        assert_eq!(
            view.vars.get("LITELLM_ADDR").map(String::as_str),
            Some("host.microsandbox.internal:4000"),
            "injected var itself must flow through unchanged"
        );
        Ok(())
    }

    #[test]
    fn seed_view_guest_bound_secret_resolves_real_value() -> Result<()> {
        // plan.env entry with is_secret=true → same exposure as the guest env:
        // the real decrypted value.
        let plan = plan_with(
            vec![EnvVar {
                name: "LITELLM_MASTER_KEY".to_string(),
                value: "${LITELLM_MASTER_KEY}".to_string(),
                is_secret: true,
                reject_placeholder: None,
                injected_by: None,
                injected_port: None,
            }],
            vec![],
        );
        let view = build_seed_env_view(
            &plan,
            &secrets_map(&[("LITELLM_MASTER_KEY", "s3cr3t")]),
            &HashSet::new(),
        )?;
        assert_eq!(
            view.vars["LITELLM_MASTER_KEY"], "s3cr3t",
            "guest-bound secret carries the real value, same as the guest env"
        );
        Ok(())
    }

    /// THE namespace rule: a host-bound secret's BINDING MAP KEY (not the
    /// decrypted value, not the definition's source_env_var) is what the guest
    /// (and therefore the env view) carries.
    #[test]
    fn seed_view_host_bound_renamed_binding_uses_map_key_and_real_value_never_appears() -> Result<()>
    {
        let plan = plan_with(
            vec![],
            vec![HostBoundSecret {
                name: "CUSTOM".to_string(),
                value: "${GITHUB_TOKEN}".to_string(),
                allowed_hosts: vec!["github.com".to_string()],
                required: true,
                reject_placeholder: None,
                on_violation: SecretViolationPolicy::Passthrough,
            }],
        );
        let view = build_seed_env_view(
            &plan,
            &secrets_map(&[("GITHUB_TOKEN", "tok-abc")]),
            &HashSet::from(["GITHUB_TOKEN".to_string()]),
        )?;
        assert_eq!(
            view.vars["CUSTOM"], "$MSB_CUSTOM",
            "host-bound secret exposes the $MSB_<binding-key> placeholder"
        );
        assert!(
            !view.vars.values().any(|v| v == "tok-abc"),
            "the real secret value must NEVER appear in the env view"
        );
        assert!(
            !view.vars.contains_key("GITHUB_TOKEN"),
            "the def's source_env_var must never appear as a key"
        );
        Ok(())
    }

    #[test]
    fn seed_view_host_bound_same_name_binding() -> Result<()> {
        // Binding key == source_env_var: still a placeholder, never the value.
        let plan = plan_with(
            vec![],
            vec![HostBoundSecret {
                name: "GITHUB_TOKEN".to_string(),
                value: "${GITHUB_TOKEN}".to_string(),
                allowed_hosts: vec!["github.com".to_string()],
                required: true,
                reject_placeholder: None,
                on_violation: SecretViolationPolicy::Passthrough,
            }],
        );
        let view = build_seed_env_view(
            &plan,
            &secrets_map(&[("GITHUB_TOKEN", "tok-abc")]),
            &HashSet::new(),
        )?;
        assert_eq!(view.vars["GITHUB_TOKEN"], "$MSB_GITHUB_TOKEN");
        assert!(
            !view.vars.values().any(|v| v == "tok-abc"),
            "the real secret value must NEVER appear in the env view"
        );
        Ok(())
    }

    #[test]
    fn seed_view_injected_depends_on_present() -> Result<()> {
        let plan = plan_with(
            vec![EnvVar {
                name: "LITELLM_ADDR".to_string(),
                value: "host.microsandbox.internal:4000".to_string(),
                is_secret: false,
                reject_placeholder: None,
                injected_by: Some("litellm".to_string()),
                injected_port: None,
            }],
            vec![],
        );
        let view = build_seed_env_view(&plan, &secrets_map(&[]), &HashSet::new())?;
        assert_eq!(view.vars["LITELLM_ADDR"], "host.microsandbox.internal:4000");
        Ok(())
    }

    #[test]
    fn render_seed_text_substitutes_against_view() -> Result<()> {
        let plan = plan_with(
            vec![EnvVar::literal(
                "LITELLM_ADDR",
                "host.microsandbox.internal:4000",
            )],
            vec![],
        );
        let view = build_seed_env_view(&plan, &secrets_map(&[]), &HashSet::new())?;
        let rendered = render_seed_text("host: ${LITELLM_ADDR}", &view, "s -> t")?;
        assert_eq!(rendered, "host: host.microsandbox.internal:4000");
        Ok(())
    }

    #[test]
    fn render_seed_text_passthrough_when_no_tokens() -> Result<()> {
        // A template=false file is never rendered at this layer (that's
        // prepare's fs::copy, tested next commit); a plain text byte-identical
        // passthrough is the renderer's contract here.
        let plan = plan_with(vec![], vec![]);
        let view = build_seed_env_view(&plan, &secrets_map(&[]), &HashSet::new())?;
        let text = "no tokens";
        assert_eq!(render_seed_text(text, &view, "s -> t")?, text);
        Ok(())
    }

    #[test]
    fn render_seed_text_missing_var_hard_errors() -> Result<()> {
        let plan = plan_with(vec![], vec![]);
        let view = build_seed_env_view(&plan, &secrets_map(&[]), &HashSet::new())?;
        let err = render_seed_text("${NOPE}", &view, "src -> dst").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("NOPE"), "must name the missing var: {msg}");
        assert!(
            msg.contains("src -> dst"),
            "must name the source→target: {msg}"
        );
        Ok(())
    }

    #[test]
    fn render_seed_text_unbound_secret_hard_errors() -> Result<()> {
        // UNBOUND is DEFINED (secrets catalog) but not bound to this workload:
        // hard error naming the secret + the seed file; the real value (if
        // any) must never leak into the error.
        let plan = plan_with(vec![], vec![]);
        let view = build_seed_env_view(
            &plan,
            &secrets_map(&[("UNBOUND", "super-secret")]),
            &HashSet::from(["UNBOUND".to_string()]),
        )?;
        let err = render_seed_text("${UNBOUND}", &view, "src -> dst").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("UNBOUND"), "must name the secret: {msg}");
        assert!(msg.contains("not bound"), "must say 'not bound': {msg}");
        assert!(
            msg.contains("src -> dst"),
            "must name the source→target: {msg}"
        );
        assert!(
            !msg.contains("super-secret"),
            "real secret value must never appear in the error: {msg}"
        );
        Ok(())
    }

    #[test]
    fn render_seed_text_unclosed_template_hard_errors() -> Result<()> {
        let plan = plan_with(vec![], vec![]);
        let view = build_seed_env_view(&plan, &secrets_map(&[]), &HashSet::new())?;
        let err = render_seed_text("value ${A", &view, "src -> dst").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("unclosed"), "unclosed template: {msg}");
        assert!(
            msg.contains("src -> dst"),
            "must name the source→target: {msg}"
        );
        Ok(())
    }

    #[test]
    fn render_seed_text_empty_name_hard_errors() -> Result<()> {
        let plan = plan_with(vec![], vec![]);
        let view = build_seed_env_view(&plan, &secrets_map(&[]), &HashSet::new())?;
        let err = render_seed_text("x ${} y", &view, "src -> dst").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("empty variable name"), "empty-name: {msg}");
        assert!(
            msg.contains("src -> dst"),
            "must name the source→target: {msg}"
        );
        Ok(())
    }

    /// END-STATE PROOF: pi's seeded models.json renders against the guest env
    /// view with `baseUrl` consuming the injected LITELLM_ADDR and `apiKey`
    /// carrying the ESCAPED literal `${LITELLM_MASTER_KEY}` — the token pi
    /// itself re-expands at runtime. The host-bound secret's guest placeholder
    /// (`$MSB_LITELLM_MASTER_KEY`) must NEVER leak into the rendered file.
    #[test]
    fn render_seed_models_json_escaped_api_key_and_resolved_base_url() -> Result<()> {
        let plan = plan_with(
            vec![EnvVar {
                name: "LITELLM_ADDR".to_string(),
                value: "host.microsandbox.internal:4000".to_string(),
                is_secret: false,
                reject_placeholder: None,
                injected_by: Some("litellm".to_string()),
                injected_port: None,
            }],
            vec![HostBoundSecret {
                name: "LITELLM_MASTER_KEY".to_string(),
                value: "${LITELLM_MASTER_KEY}".to_string(),
                allowed_hosts: vec![],
                required: true,
                reject_placeholder: None,
                on_violation: SecretViolationPolicy::Passthrough,
            }],
        );
        let view = build_seed_env_view(
            &plan,
            &secrets_map(&[("LITELLM_MASTER_KEY", "real-secret")]),
            &HashSet::new(),
        )?;
        let models_json = r#"{
  "mcpServers": {},
  "providers": {
    "litellm": {
      "baseUrl": "http://${LITELLM_ADDR}/v1",
      "apiKey": "$${LITELLM_MASTER_KEY}"
    }
  }
}"#;
        let rendered = render_seed_text(models_json, &view, "pi models.json -> guest")?;
        assert!(
            rendered.contains("\"baseUrl\": \"http://host.microsandbox.internal:4000/v1\""),
            "baseUrl must consume the injected LITELLM_ADDR: {rendered}"
        );
        assert!(
            rendered.contains("\"apiKey\": \"${LITELLM_MASTER_KEY}\""),
            "apiKey must be the escaped literal, not $MSB_...: {rendered}"
        );
        assert!(
            !rendered.contains("$MSB_LITELLM_MASTER_KEY"),
            "the guest placeholder must never leak into the seed file: {rendered}"
        );
        Ok(())
    }
}
