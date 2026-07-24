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

/// Process-env-only resolution (plan `env` entries, which are not secrets).
pub(crate) fn resolve_templated_value(templated: &str) -> Result<String> {
    // The closure (not `std::env::var` directly) so the higher-ranked
    // `for<'a> Fn(&'a str)` bound unifies: `env::var` is generic over its
    // key type, which does not satisfy the higher-ranked fn-pointer shape.
    resolve_templated_value_by(templated, |name: &str| std::env::var(name))
}
