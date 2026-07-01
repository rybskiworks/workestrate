use anyhow::Result;

pub(crate) fn env_var(name: &str) -> Result<String> {
    std::env::var(name).map_err(|e| anyhow::anyhow!("missing env var {}: {}", name, e))
}

pub(crate) fn resolve_templated_value(templated: &str) -> Result<String> {
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
                let value = env_var(var_name)?;
                result = result.replace(&format!("${{{}}}", var_name), &value);
                search_from = absolute + 2 + end + 1;
            }
            None => anyhow::bail!("unclosed ${{ in secret template at position {}", absolute,),
        }
    }
    Ok(result)
}
