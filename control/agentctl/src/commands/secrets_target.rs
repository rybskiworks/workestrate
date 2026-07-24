//! Secrets-target resolution (`workestrate secrets-target`, `secrets-schema`)
//! and the age-recipient / tilde helpers shared with `config new`.

use std::path::PathBuf;

use anyhow::Result;

use crate::config;
use crate::scaffold;

/// Resolve `~` in a path string via `$HOME` (no `dirs` crate dep). Falls
/// back to the literal path if `~/` prefix is absent or `$HOME` is unset.
pub(crate) fn expand_tilde(p: &std::path::Path) -> std::path::PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return std::path::PathBuf::from(home).join(rest);
        }
    }
    p.to_path_buf()
}

/// Derive the age public recipient from a private key file via
/// `age-keygen -y <keyfile>`. Returns `Ok(recipient)` on success.
/// Returns `Err(message)` when either the `age-keygen` binary is missing
/// or the key file does not exist (so the caller can fall back to the
/// placeholder and surface a single, specific reason).
pub(crate) fn derive_age_recipient(key_file: &std::path::Path) -> Result<String> {
    if !key_file.exists() {
        anyhow::bail!("age key file not found at {}", key_file.display());
    }
    let output = std::process::Command::new("age-keygen")
        .arg("-y")
        .arg(key_file)
        .output()
        .map_err(|e| anyhow::anyhow!("age-keygen binary not found: {}", e))?;
    if !output.status.success() {
        anyhow::bail!(
            "age-keygen -y failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Resolve a registered config repo's secrets target paths for setup-secrets.
///
/// The resolution MUST match `config::resolve_secrets_layers()` for
/// context-layer entries: dir = `<store>/repos/<name>`, secrets_file from the
/// entry override or ".env.enc", age_key_file from the entry override
/// (tilde-expanded). When the entry has no age_key_file override, the fallback
/// matches `secrets_loader::decrypt_layer()`: `SOPS_AGE_KEY_FILE` env, else
/// `$HOME` + `scaffold::AGE_KEY_DEFAULT_PATH` with the `~/` prefix stripped.
pub(crate) async fn cmd_secrets_target(name: &str, json: bool) -> Result<()> {
    let registry = config::load_registry()?;
    let entry = registry
        .as_ref()
        .and_then(|r| r.configs.get(name))
        .ok_or_else(|| anyhow::anyhow!("config repo '{}' not registered", name))?;

    let dir = config::resolve_store_dir().join("repos").join(name);
    let secrets_file = entry
        .secrets_file
        .as_deref()
        .unwrap_or(".env.enc")
        .to_string();
    let age_key_file = if let Some(custom) = entry.age_key_file.as_deref() {
        expand_tilde(std::path::Path::new(custom))
    } else if let Ok(env_key) = std::env::var("SOPS_AGE_KEY_FILE") {
        PathBuf::from(env_key)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let rel = scaffold::AGE_KEY_DEFAULT_PATH
            .strip_prefix("~/")
            .unwrap_or(scaffold::AGE_KEY_DEFAULT_PATH);
        PathBuf::from(home).join(rel)
    };
    let exists = dir.join(&secrets_file).exists();

    if json {
        let body = serde_json::json!({
            "dir": dir.display().to_string(),
            "secrets_file": secrets_file,
            "age_key_file": age_key_file.display().to_string(),
            "exists": exists,
        });
        println!("{}", serde_json::to_string_pretty(&body)?);
    } else {
        println!("config repo:    {}", name);
        println!("dir:            {}", dir.display());
        println!("secrets_file:   {}", secrets_file);
        println!("age_key_file:   {}", age_key_file.display());
        println!("exists:         {}", exists);
    }
    Ok(())
}

pub(crate) async fn cmd_secrets_schema() -> Result<()> {
    let config = config::load_config()?;
    let mut names: Vec<&str> = config
        .secrets
        .values()
        .filter_map(|s| s.env_var.as_deref())
        .collect();
    names.sort();
    for name in names {
        println!("{}", name);
    }
    Ok(())
}
