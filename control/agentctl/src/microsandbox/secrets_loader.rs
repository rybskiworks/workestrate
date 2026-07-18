use anyhow::Result;
use std::collections::HashMap;
use std::process::Command;

/// Decrypt `.env.enc` via `sops` and load all secrets into the process
/// environment. Generic — no key filtering, no per-command configuration.
///
/// Called conditionally by code paths that need secrets (build_sandbox,
/// the `run` subcommand). NOT called for plan/check/new/completions.
///
/// If `.env.enc` does not exist, returns Ok(()) (no-op — e.g., a fresh
/// clone where setup-secrets hasn't been run yet).
///
/// Respects the `SECRET_FILE` env var override (defaults to `.env.enc`),
/// and `SOPS_AGE_KEY_FILE` (defaults to ~/.config/sops/age/ai-workbench-secrets.txt).
pub fn load_secrets() -> Result<()> {
    let root = crate::config::project_root()?;

    let secret_file = std::env::var("SECRET_FILE").unwrap_or_else(|_| ".env.enc".to_string());
    let env_enc = root.join(&secret_file);

    if !env_enc.exists() {
        return Ok(());
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
        .map_err(|e| anyhow::anyhow!("failed to run 'sops decrypt' (is sops on PATH?): {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("sops decrypt failed for {}: {stderr}", env_enc.display());
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

fn is_valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(
        chars.next(),
        Some(c) if c.is_ascii_alphabetic() || c == '_'
    ) && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
