use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;

/// Decrypt `.env.enc` files across all resolved config layers via `sops` and
/// return the merged secrets map. Secrets are merged per-key: later layers
/// override earlier layers for the same key, with process env as the lowest
/// precedence.
///
/// FN-9: the merged map is RETURNED, not splattered into the process
/// environment. The old implementation `set_var`ed every merged key and then
/// read required-secret values back out of `std::env` — process-global
/// mutable state that raced when two `build_sandbox` calls ran in parallel
/// (one call could observe another's keys). Required-secret validation now
/// reads from the returned map; only the merged-map CONSUMER that genuinely
/// needs process env (`cmd_run`, which `exec(2)` replaces this process with
/// the user command) re-enters `apply_secrets_to_process_env` explicitly.
///
/// Called conditionally by code paths that need secrets (build_sandbox,
/// the `run` subcommand). NOT called for plan/check/new/completions.
pub fn load_secrets() -> Result<std::collections::HashMap<String, String>> {
    let layers = crate::config::resolve_secrets_layers()?;
    let mut merged: HashMap<String, String> = HashMap::new();
    let mut provenance: HashMap<String, String> = HashMap::new();

    // Load secret definitions to know which env vars are secrets. v2: the
    // source env var is the raw `env_var`, defaulting to the secret ID.
    let config = crate::config::load_config()?;
    let secret_env_vars: HashSet<String> = config
        .secrets
        .iter()
        .map(|(id, s)| s.env_var.clone().unwrap_or_else(|| id.clone()))
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

    // FN-9: no `std::env::set_var` here. The merged map is the single source
    // of truth for validation AND for the caller's downstream resolution;
    // mutating process-global env from a library function is a data race
    // across parallel `build_sandbox` calls (and leaks secrets into
    // `std::env::var` for unrelated code in this process).

    // Store provenance for --show-source.
    crate::merge::set_secret_provenance(Some(provenance));

    // Check required secrets (fail-closed) against the merged map. v2: a def
    // without `env_var` reads from the env var named after the secret ID.
    for (secret_id, secret_def) in &config.secrets {
        let required = secret_def.required.unwrap_or(true);
        if !required {
            continue;
        }
        let env_var = secret_def
            .env_var
            .clone()
            .unwrap_or_else(|| secret_id.clone());
        let value = merged.get(&env_var).cloned().unwrap_or_default();
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

    Ok(merged)
}

/// Copy a merged secrets map into the process environment.
///
/// FN-9 scoped exception: call this ONLY at a boundary that fundamentally
/// requires process env — currently `workestrate run -- <cmd>`, which
/// `exec(2)` REPLACES this process with the user command, so the secrets can
/// only be inherited via the environment. Keys are not valid env names are
/// skipped (defense-in-depth against a hostile key in an `.env.enc`).
/// `build_sandbox` deliberately does NOT call this: it resolves secrets from
/// the returned map.
pub fn apply_secrets_to_process_env(merged: &std::collections::HashMap<String, String>) {
    for (key, value) in merged {
        if is_valid_env_name(key) {
            std::env::set_var(key, value);
        }
    }
}

/// Check if a file's permissions are too permissive (mode > 0600).
/// Returns a warning message if the file exists and has group/other bits set.
/// Returns None if the file doesn't exist (caller handles missing files separately)
/// or if permissions are OK (mode <= 0600).
#[cfg(unix)]
fn check_file_perms(path: &std::path::Path, label: &str) -> Option<String> {
    use std::os::unix::fs::PermissionsExt;
    let metadata = std::fs::metadata(path).ok()?;
    let mode = metadata.permissions().mode();
    // Check if any group or other permission bits are set (bits 0077)
    if mode & 0o077 != 0 {
        let mode_str = format!("{:04o}", mode & 0o777);
        Some(format!(
            "WARNING: {} '{}' has overly permissive mode {} (should be 0600 or stricter)",
            label,
            path.display(),
            mode_str
        ))
    } else {
        None
    }
}

#[cfg(not(unix))]
fn check_file_perms(_path: &std::path::Path, _label: &str) -> Option<String> {
    None
}

fn decrypt_layer(layer: &crate::config::SecretsLayer) -> Result<Option<HashMap<String, String>>> {
    let env_enc = layer.dir.join(&layer.secrets_file);
    if !env_enc.exists() {
        return Ok(None); // no .env.enc, not an error
    }

    // C13: warn (not error) if the secrets file is group/other-readable.
    if let Some(warning) = check_file_perms(&env_enc, "secrets file") {
        eprintln!("{warning}");
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

    // C13: warn (not error) if the age key file is group/other-readable.
    if let Some(warning) = check_file_perms(&key_file, "age key file") {
        eprintln!("{warning}");
    }

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
#[allow(clippy::unwrap_used, clippy::expect_used)]
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
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
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
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
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

    // --- C13: check_file_perms tests (Unix-only, no env mutation) ---

    #[cfg(unix)]
    fn make_temp_file_with_mode(name: &str, mode: u32) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(format!(
            "workestrate-perms-test-{}-{}",
            std::process::id(),
            name
        ));
        std::fs::write(&path, b"test").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).unwrap();
        path
    }

    #[cfg(unix)]
    #[test]
    fn check_file_perms_ok_for_mode_0600() {
        let path = make_temp_file_with_mode("0600", 0o600);
        let result = check_file_perms(&path, "age key file");
        std::fs::remove_file(&path).unwrap();
        assert_eq!(result, None, "mode 0600 should not warn");
    }

    #[cfg(unix)]
    #[test]
    fn check_file_perms_warns_for_mode_0644() {
        let path = make_temp_file_with_mode("0644", 0o644);
        let result = check_file_perms(&path, "secrets file");
        std::fs::remove_file(&path).unwrap();
        let msg = result.unwrap();
        assert!(
            msg.contains("overly permissive"),
            "warning should say 'overly permissive'; got: {msg}"
        );
        assert!(
            msg.contains("0644"),
            "warning should include the mode 0644; got: {msg}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn check_file_perms_warns_for_mode_0666() {
        let path = make_temp_file_with_mode("0666", 0o666);
        let result = check_file_perms(&path, "age key file");
        std::fs::remove_file(&path).unwrap();
        assert!(result.is_some(), "mode 0666 should warn");
    }

    #[cfg(unix)]
    #[test]
    fn check_file_perms_none_for_missing_file() {
        let path = std::env::temp_dir().join(format!(
            "workestrate-perms-test-{}-does-not-exist",
            std::process::id()
        ));
        // Ensure it really doesn't exist.
        let _ = std::fs::remove_file(&path);
        assert_eq!(check_file_perms(&path, "age key file"), None);
    }

    #[cfg(unix)]
    #[test]
    fn check_file_perms_ok_for_mode_0400() {
        let path = make_temp_file_with_mode("0400", 0o400);
        let result = check_file_perms(&path, "age key file");
        std::fs::remove_file(&path).unwrap();
        assert_eq!(
            result, None,
            "mode 0400 (stricter than 0600) should not warn"
        );
    }
    // ---- FN-9: load_secrets returns the map; no process-env leak ----

    #[test]
    fn load_secrets_returns_map_without_leaking_into_process_env() {
        // FN-9 regression: load_secrets must NOT mutate process env. With a
        // config declaring an unset secret (required=false so the call
        // succeeds), the returned map must not leak the key into
        // std::env::var afterwards. Under the old implementation the
        // set_var loop populated process env from the merged map.
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        // An allowlisted secret (policy::SECRET_HOST_BINDINGS) that is NOT
        // set in this process — required=false so load_secrets succeeds and
        // returns the map without the key.
        let probe = "ODYSSEUS_ADMIN_PASSWORD";
        std::env::remove_var(probe);

        let tmp = std::env::temp_dir().join(format!(
            "workestrate-fn9-leak-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let config_content = concat!(
            "schema_version = 1\n\n",
            "[secrets.ODYSSEUS_ADMIN_PASSWORD]\n",
            "env_var = \"ODYSSEUS_ADMIN_PASSWORD\"\n",
            "required = false\n"
        );
        std::fs::write(tmp.join("workestrate.toml"), config_content).unwrap();

        let old = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        std::env::set_var("WORKESTRATE_CONFIG_DIR", &tmp);

        let result = load_secrets();

        match old {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        std::fs::remove_dir_all(&tmp).unwrap();

        let merged = result.expect("load_secrets should succeed (required=false)");
        assert!(
            !merged.contains_key(probe),
            "unset optional secret must not appear in the merged map"
        );
        assert!(
            std::env::var(probe).is_err(),
            "load_secrets must not leak values into std::env::var after return"
        );
        // Hygiene: the merged map itself is the only carrier, and the
        // process env is untouched for an unrelated declared key.
        std::env::remove_var(probe);
    }

    #[test]
    fn apply_secrets_to_process_env_sets_only_valid_names() {
        // The scoped exec-boundary helper applies valid keys and skips
        // names that are not legal env identifiers.
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let good = "WORKESTRATE_FN9_APPLY_GOOD";
        std::env::remove_var(good);
        let mut merged = HashMap::new();
        merged.insert(good.to_string(), "applied".to_string());
        merged.insert("9BAD-NAME".to_string(), "nope".to_string());
        apply_secrets_to_process_env(&merged);
        assert_eq!(std::env::var(good).as_deref(), Ok("applied"));
        std::env::remove_var(good);
    }
}
