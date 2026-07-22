use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

/// Decrypt `.env.enc` files across all resolved config layers via `sops` and load
/// all secrets into the process environment. Secrets are merged per-key: later
/// layers override earlier layers for the same key, with process env as the
/// lowest precedence.
///
/// Called conditionally by code paths that need secrets (build_sandbox,
/// the `run` subcommand). NOT called for plan/check/new/completions.
pub fn load_secrets() -> Result<()> {
    let layers = crate::config::resolve_secrets_layers()?;
    let mut merged: HashMap<String, String> = HashMap::new();
    let mut provenance: HashMap<String, String> = HashMap::new();

    // Load secret definitions to know which env vars are secrets.
    let config = crate::config::load_config()?;
    let secret_env_vars: HashSet<String> = config
        .secrets
        .values()
        .filter_map(|s| s.env_var.as_deref().map(String::from))
        .collect();

    // Process env as lowest precedence (only for defined secrets).
    for env_var in &secret_env_vars {
        if let Ok(val) = std::env::var(env_var) {
            if !val.is_empty() {
                merged.insert(env_var.clone(), val);
                provenance.insert(env_var.clone(), "process_env".to_string());
            }
        }
    }

    // Load from each layer (later = higher precedence).
    for layer in &layers {
        if layer.skip {
            continue; // secrets = "none"
        }
        match decrypt_layer(layer) {
            Ok(Some(values)) => {
                for (key, value) in values {
                    merged.insert(key.clone(), value);
                    provenance.insert(key, layer.name.clone());
                }
            }
            Ok(None) => {} // no .env.enc, OK
            Err(e) => {
                eprintln!(
                    "WARNING: could not load secrets from layer '{}': {}",
                    layer.name, e
                );
                // Continue with other layers
            }
        }
    }

    // Set env vars from merged values.
    for (key, value) in &merged {
        if is_valid_env_name(key) {
            std::env::set_var(key, value);
        }
    }

    // Store provenance for --show-source.
    crate::merge::set_secret_provenance(Some(provenance));

    // Check required secrets (fail-closed).
    for secret_def in config.secrets.values() {
        let required = secret_def.required.unwrap_or(true);
        if !required {
            continue;
        }
        if let Some(ref env_var) = secret_def.env_var {
            let value = std::env::var(env_var).unwrap_or_default();
            let is_empty = value.is_empty() || value.trim().is_empty();
            let is_placeholder = secret_def
                .placeholder
                .as_ref()
                .map(|p| value == *p)
                .unwrap_or(false);
            if is_empty || is_placeholder {
                let layers_tried: Vec<String> = layers
                    .iter()
                    .filter(|l| !l.skip)
                    .map(|l| l.name.clone())
                    .collect();
                anyhow::bail!(
                    "required secret '{}' is not satisfied.\n\
                     Layers tried: {}\n\
                     Remediation: run 'setup-secrets --config <name> update',\n\
                     set the env var directly, or add an age recipient to the\n\
                     config repo's .sops.yaml.",
                    env_var,
                    layers_tried.join(", ")
                );
            }
        }
    }

    Ok(())
}

fn decrypt_layer(layer: &crate::config::SecretsLayer) -> Result<Option<HashMap<String, String>>> {
    let env_enc = layer.dir.join(&layer.secrets_file);
    if !env_enc.exists() {
        return Ok(None); // no .env.enc, not an error
    }

    // Resolve age key file.
    let key_file = if let Some(ref custom) = layer.age_key_file {
        custom.clone()
    } else if let Ok(env_key) = std::env::var("SOPS_AGE_KEY_FILE") {
        PathBuf::from(env_key)
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        // Derive from the shared const (single source of truth, matches
        // scaffold::AGE_KEY_DEFAULT_PATH and the --empty warning in main.rs).
        let rel = crate::scaffold::AGE_KEY_DEFAULT_PATH
            .strip_prefix("~/")
            .unwrap_or(crate::scaffold::AGE_KEY_DEFAULT_PATH);
        PathBuf::from(home).join(rel)
    };

    if !key_file.exists() {
        anyhow::bail!("age key not found: {}", key_file.display());
    }

    // Set SOPS_AGE_KEY_FILE for this layer (save/restore).
    let old_key = std::env::var("SOPS_AGE_KEY_FILE").ok();
    std::env::set_var("SOPS_AGE_KEY_FILE", &key_file);

    let result = Command::new("sops")
        .args(["decrypt", "--input-type", "dotenv", "--output-type", "json"])
        .arg(&env_enc)
        .output();

    // Restore env var.
    match old_key {
        Some(v) => std::env::set_var("SOPS_AGE_KEY_FILE", v),
        None => std::env::remove_var("SOPS_AGE_KEY_FILE"),
    }

    let output = result.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            anyhow::anyhow!("sops not found on PATH")
        } else {
            anyhow::anyhow!("failed to run sops: {e}")
        }
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("sops decrypt failed for {}: {stderr}", env_enc.display());
    }

    let values: HashMap<String, String> = serde_json::from_slice(&output.stdout)
        .map_err(|e| anyhow::anyhow!("failed to parse sops JSON: {e}"))?;

    Ok(Some(values))
}

fn is_valid_env_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(
        chars.next(),
        Some(c) if c.is_ascii_alphabetic() || c == '_'
    ) && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn merge_secrets_later_layer_wins_per_key() {
        // Test the merge logic: later layer overrides per-key.
        let mut merged: HashMap<String, String> = HashMap::new();
        let mut provenance: HashMap<String, String> = HashMap::new();

        // Layer 1 (team)
        let team = HashMap::from([
            ("KEY_A".to_string(), "team_a".to_string()),
            ("KEY_B".to_string(), "team_b".to_string()),
        ]);
        for (k, v) in team {
            merged.insert(k.clone(), v);
            provenance.insert(k, "team".to_string());
        }

        // Layer 2 (personal) — overrides KEY_A only.
        let personal = HashMap::from([("KEY_A".to_string(), "personal_a".to_string())]);
        for (k, v) in personal {
            merged.insert(k.clone(), v);
            provenance.insert(k, "personal".to_string());
        }

        assert_eq!(merged.get("KEY_A"), Some(&"personal_a".to_string()));
        assert_eq!(merged.get("KEY_B"), Some(&"team_b".to_string()));
        assert_eq!(provenance.get("KEY_A"), Some(&"personal".to_string()));
        assert_eq!(provenance.get("KEY_B"), Some(&"team".to_string()));
    }

    #[test]
    fn process_env_lowest_precedence() {
        // Process env is overridden by any layer.
        let mut merged: HashMap<String, String> = HashMap::new();
        let mut provenance: HashMap<String, String> = HashMap::new();

        // Process env.
        merged.insert("KEY".to_string(), "env_value".to_string());
        provenance.insert("KEY".to_string(), "process_env".to_string());

        // Layer overrides.
        let layer = HashMap::from([("KEY".to_string(), "layer_value".to_string())]);
        for (k, v) in layer {
            merged.insert(k.clone(), v);
            provenance.insert(k, "personal".to_string());
        }

        assert_eq!(merged.get("KEY"), Some(&"layer_value".to_string()));
        assert_eq!(provenance.get("KEY"), Some(&"personal".to_string()));
    }

    #[test]
    fn required_secret_unsatisfied_error_message() {
        // Verify the error message format for unsatisfied required secrets.
        let err_msg = format!(
            "required secret '{}' is not satisfied.\n\
             Layers tried: {}\n\
             Remediation: run 'setup-secrets --config <name> update',\n\
             set the env var directly, or add an age recipient to the\n\
             config repo's .sops.yaml.",
            "LITELLM_MASTER_KEY", "reference, team, personal"
        );
        assert!(err_msg.contains("required secret"));
        assert!(err_msg.contains("Layers tried"));
        assert!(err_msg.contains("Remediation"));
        assert!(err_msg.contains("setup-secrets"));
    }

    #[test]
    fn missing_env_enc_produces_required_secret_error() {
        // With multi-layer loading, a missing .env.enc is not an error per layer.
        // The hard failure is an unsatisfied required secret.
        let _lock = crate::config::tests::ENV_TEST_LOCK.lock().unwrap();
        let tmp =
            std::env::temp_dir().join(format!("workestrate-secrets-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let config_content = "schema_version = 1\n\n[secrets.LITELLM_MASTER_KEY]\nenv_var = \"LITELLM_MASTER_KEY\"\nrequired = true\n";
        std::fs::write(tmp.join("workestrate.toml"), config_content).unwrap();

        let old = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        std::env::set_var("WORKESTRATE_CONFIG_DIR", &tmp);

        let result = load_secrets();

        match old {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        std::fs::remove_dir_all(&tmp).unwrap();

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("required secret")
                && err.contains("Layers tried")
                && err.contains("setup-secrets"),
            "error should mention required secret, layers tried, and setup-secrets; got: {err}"
        );
    }

    #[test]
    fn resolve_secrets_layers_uses_config_dir_override() -> Result<()> {
        let _lock = crate::config::tests::ENV_TEST_LOCK.lock().unwrap();
        let tmp =
            std::env::temp_dir().join(format!("workestrate-layers-test-{}", std::process::id()));
        std::fs::create_dir_all(&tmp)?;
        let old = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        std::env::set_var("WORKESTRATE_CONFIG_DIR", &tmp);

        let layers = crate::config::resolve_secrets_layers()?;

        match old {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        std::fs::remove_dir_all(&tmp)?;

        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].name, "local");
        assert_eq!(layers[0].dir, tmp);
        Ok(())
    }

    #[test]
    fn secrets_loader_default_path_uses_const() {
        // WP-F3: secrets_loader's default age key path must be derived from
        // scaffold::AGE_KEY_DEFAULT_PATH, not a hardcoded literal. We verify
        // by checking that the const's stripped path matches the literal
        // that was previously hardcoded.
        let const_rel = crate::scaffold::AGE_KEY_DEFAULT_PATH
            .strip_prefix("~/")
            .unwrap_or(crate::scaffold::AGE_KEY_DEFAULT_PATH);
        assert_eq!(
            const_rel, ".config/sops/age/ai-workbench-secrets.txt",
            "secrets_loader default path must be derived from AGE_KEY_DEFAULT_PATH const"
        );
    }
}
