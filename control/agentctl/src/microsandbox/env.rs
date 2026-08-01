use anyhow::Result;

/// Resolve `${VAR}` templates against an explicit lookup map FIRST, falling
/// back to process env for variables the map does not carry (FN-9). Secret
/// values flow through the map returned by `load_secrets`; process env is
/// only consulted for ad-hoc variables (runtime/user exports), never as a
/// read-back of secrets this process wrote.
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
pub(crate) fn resolve_templated_value_with_env_fallback(
    templated: &str,
    vars: &std::collections::HashMap<String, String>,
) -> Result<String> {
    resolve_templated_value_by(templated, move |name: &str| match vars.get(name) {
        Some(v) => Ok(v.clone()),
        None => std::env::var(name),
    })
}

/// Shared engine: resolve `${VAR}` templates through `lookup`.
fn resolve_templated_value_by<F>(templated: &str, lookup: F) -> Result<String>
where
    F: for<'a> Fn(&'a str) -> std::result::Result<String, std::env::VarError>,
{
    let mut result = templated.to_string();
    let mut search_from = 0;
    while let Some(start) = templated[search_from..].find("${") {
        let absolute = search_from + start;
        match templated[absolute + 2..].find('}') {
            Some(end) => {
                let var_name = &templated[absolute + 2..absolute + 2 + end];
                if var_name.is_empty() {
                    anyhow::bail!(
                        "empty variable name in secret template at position {}",
                        absolute
                    );
                }
                let value = lookup(var_name)
                    .map_err(|e| anyhow::anyhow!("missing env var {}: {}", var_name, e))?;
                result = result.replace(&format!("${{{}}}", var_name), &value);
                search_from = absolute + 2 + end + 1;
            }
            None => anyhow::bail!("unclosed ${{ in secret template at position {}", absolute,),
        }
    }
    Ok(result)
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
    use std::collections::HashMap;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
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
}
