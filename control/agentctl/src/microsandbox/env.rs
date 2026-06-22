use anyhow::Result;

pub(crate) fn env_var(name: &str) -> Result<String> {
    std::env::var(name).map_err(|e| anyhow::anyhow!("missing env var {}: {}", name, e))
}

/// Validate that all required environment variables are set before trying to
/// start a sandbox.
///
/// A variable is treated as missing if it is unset, empty, or contains only
/// whitespace. The `LITELLM_MASTER_KEY` value is additionally rejected if it
/// matches the committed placeholder from `.env.example`.
///
/// The `context` label is included verbatim in the error message and should be
/// the user-facing command name (e.g. `"agentctl litellm up"`).
pub(crate) fn require_env_vars(context: &str, names: &[&str]) -> Result<()> {
    const LITELLM_MASTER_KEY_PLACEHOLDER: &str = "sk-change-me-local-only";

    let mut missing = Vec::new();
    for name in names {
        let value = std::env::var(name).unwrap_or_default();
        let trimmed = value.trim();
        if trimmed.is_empty() {
            missing.push(*name);
            continue;
        }
        if *name == "LITELLM_MASTER_KEY" && trimmed == LITELLM_MASTER_KEY_PLACEHOLDER {
            return Err(anyhow::anyhow!(
                "LITELLM_MASTER_KEY is set to the placeholder value from .env.example.\n\
                 Replace it with a real secret (e.g. via with-secrets) before running {}",
                context
            ));
        }
    }
    if !missing.is_empty() {
        return Err(anyhow::anyhow!(
            "missing secrets for {}: {}\nSet them in the environment or run via with-secrets",
            context,
            missing.join(", ")
        ));
    }
    Ok(())
}

pub(crate) fn resolve_secret_value(templated: &str) -> Result<String> {
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

#[cfg(test)]
// Using unwrap_err() is standard practice in unit tests for asserting
// that a Result is an Err.
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn require_env_vars_succeeds_when_all_set() {
        let vars = ["AGENTCTL_TEST_A", "AGENTCTL_TEST_B"];
        for name in &vars {
            std::env::set_var(name, "value");
        }
        let result = require_env_vars("test", &vars);
        for name in &vars {
            std::env::remove_var(name);
        }
        assert!(result.is_ok());
    }

    #[test]
    fn require_env_vars_reports_missing_var() {
        std::env::remove_var("AGENTCTL_TEST_MISSING");
        let err = require_env_vars("agentctl test up", &["AGENTCTL_TEST_MISSING"])
            .unwrap_err()
            .to_string();
        assert!(err.contains("agentctl test up"), "context missing: {err}");
        assert!(
            err.contains("AGENTCTL_TEST_MISSING"),
            "var name missing: {err}"
        );
    }

    #[test]
    fn require_env_vars_treats_empty_and_whitespace_as_missing() {
        std::env::set_var("AGENTCTL_TEST_EMPTY", "");
        std::env::set_var("AGENTCTL_TEST_SPACE", "   ");
        let err = require_env_vars("test", &["AGENTCTL_TEST_EMPTY", "AGENTCTL_TEST_SPACE"])
            .unwrap_err()
            .to_string();
        assert!(err.contains("AGENTCTL_TEST_EMPTY"), "{err}");
        assert!(err.contains("AGENTCTL_TEST_SPACE"), "{err}");
    }

    #[test]
    fn require_env_vars_rejects_litellm_placeholder() {
        std::env::set_var("LITELLM_MASTER_KEY", "sk-change-me-local-only");
        let err = require_env_vars("agentctl litellm up", &["LITELLM_MASTER_KEY"])
            .unwrap_err()
            .to_string();
        assert!(err.contains("placeholder"), "{err}");
        std::env::remove_var("LITELLM_MASTER_KEY");
    }
}
