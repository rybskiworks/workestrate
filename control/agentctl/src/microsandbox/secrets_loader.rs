use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

/// Decrypt `.env.enc` via `sops` and load all secrets into the process
/// environment. Generic — no key filtering, no per-command configuration.
///
/// Called conditionally by code paths that need secrets (build_sandbox,
/// the `run` subcommand). NOT called for plan/check/new/completions.
///
/// Respects the `SECRET_FILE` env var override (defaults to `.env.enc`),
/// and `SOPS_AGE_KEY_FILE` (defaults to ~/.config/sops/age/ai-workbench-secrets.txt).
pub fn load_secrets() -> Result<()> {
    // Resolve the active config directory to find .env.enc.
    let config_dir = resolve_secrets_dir()?;

    let secret_file = std::env::var("SECRET_FILE").unwrap_or_else(|_| ".env.enc".to_string());
    let env_enc = config_dir.join(&secret_file);

    if !env_enc.exists() {
        anyhow::bail!(
            "no secrets store (.env.enc) found for the active config at '{}'\n\
             Run `workestrate init`, `workestrate config add <url> <name>`, or\n\
             `setup-secrets --config <name> init` to create one.",
            config_dir.display()
        );
    }

    // Default SOPS_AGE_KEY_FILE if not set (same default as the shell wrappers).
    if std::env::var("SOPS_AGE_KEY_FILE").is_err() {
        if let Some(home) = std::env::var_os("HOME") {
            let key_path =
                std::path::Path::new(&home).join(".config/sops/age/ai-workbench-secrets.txt");
            std::env::set_var("SOPS_AGE_KEY_FILE", key_path);
        }
    }

    let output = Command::new("sops")
        .args(["decrypt", "--input-type", "dotenv", "--output-type", "json"])
        .arg(&env_enc)
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                anyhow::anyhow!(
                    "sops is required for secret decryption but was not found on PATH\n\
                     Install sops, or enter the dev shell with `nix develop`."
                )
            } else {
                anyhow::anyhow!("failed to run 'sops decrypt': {e}")
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "sops decrypt failed for {} (config dir: {}):\n{stderr}",
            env_enc.display(),
            config_dir.display()
        );
    }

    let values: HashMap<String, String> = serde_json::from_slice(&output.stdout)
        .map_err(|e| anyhow::anyhow!("failed to parse sops JSON output: {e}"))?;

    for (key, value) in values {
        if is_valid_env_name(&key) {
            std::env::set_var(key, value);
        }
    }

    Ok(())
}

/// Resolve the directory where `.env.enc` should be found for the active config.
///
/// Mirrors the resolution order in `config::load_config()`:
/// 1. `WORKESTRATE_CONFIG_DIR` env var
/// 2. Trusted project `./workestrate.toml` (cwd)
/// 3. Registry single layer: `resolve_store_dir()/repos/<name>/`
/// 4. `config.reference/` (shipped with tool — no .env.enc expected)
fn resolve_secrets_dir() -> Result<PathBuf> {
    // 1. WORKESTRATE_CONFIG_DIR
    if let Ok(dir) = std::env::var("WORKESTRATE_CONFIG_DIR") {
        return Ok(PathBuf::from(dir));
    }

    // 2. Trusted project (cwd)
    let skip_project = std::env::var("WORKESTRATE_NO_PROJECT_CONFIG").is_ok();
    if !skip_project {
        let cwd = std::env::current_dir()?;
        let project_path = cwd.join("workestrate.toml");
        if project_path.exists() {
            match crate::config::load_registry()? {
                Some(_) => {
                    if crate::config::is_trusted_project(&cwd) {
                        return Ok(cwd);
                    }
                }
                None => {
                    return Ok(cwd);
                }
            }
        }
    }

    // 3. Registry single layer
    if let Ok(Some(registry)) = crate::config::load_registry() {
        if let Some(name) = registry.layers.first() {
            return Ok(crate::config::resolve_store_dir().join("repos").join(name));
        }
    }

    // 4. config.reference/ (fallback — no .env.enc expected here)
    if let Ok(root) = crate::config::project_root() {
        return Ok(root.join("config.reference"));
    }

    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut path = PathBuf::from(manifest);
        if path.pop() && path.pop() {
            return Ok(path.join("config.reference"));
        }
    }

    anyhow::bail!("could not resolve active config directory for secrets")
}

fn is_valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(
        chars.next(),
        Some(c) if c.is_ascii_alphabetic() || c == '_'
    ) && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_env_enc_produces_actionable_error() {
        // When no .env.enc exists in the resolved config dir, load_secrets
        // should produce an actionable error mentioning setup-secrets.
        // We point WORKESTRATE_CONFIG_DIR at a nonexistent dir to force
        // the "no secrets store" path.
        std::env::set_var(
            "WORKESTRATE_CONFIG_DIR",
            "/tmp/nonexistent-config-dir-12345",
        );
        let result = load_secrets();
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("no secrets store") && err.contains("setup-secrets"),
            "error should mention 'no secrets store' and 'setup-secrets'; got: {err}"
        );
    }
}
