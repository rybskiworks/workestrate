//! Registry persistence and active-context resolution.

use anyhow::Result;

use crate::config::paths::registry_path;
use crate::config::{ActiveContext, ConfigRepoEntry, Registry};

pub fn load_registry() -> Result<Option<Registry>> {
    let path = registry_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("failed to read registry {}: {}", path.display(), e))?;
    let registry: Registry = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("failed to parse registry {}: {}", path.display(), e))?;
    Ok(Some(registry))
}

pub fn save_registry(registry: &Registry) -> Result<()> {
    let path = registry_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(registry)
        .map_err(|e| anyhow::anyhow!("failed to serialize registry: {}", e))?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Insert/replace a config repo entry in the registry. If `layers` is empty,
/// push `name` as the default layer (mirrors cmd_config_add's behavior).
/// Shared by `cmd_config_add` (clone + register) and `cmd_config_new`
/// (local path + register). For local-path scaffolds, pass `git_ref = None`
/// and `rev = None` — `cmd_config_update` recognizes this as a local-path
/// repo and skips the pull step.
pub fn register_config(
    name: &str,
    url: &str,
    git_ref: Option<&str>,
    rev: Option<&str>,
) -> Result<()> {
    let mut registry = load_registry()?.unwrap_or_default();
    registry.configs.insert(
        name.to_string(),
        ConfigRepoEntry {
            url: url.to_string(),
            r#ref: git_ref.map(|s| s.to_string()),
            rev: rev.map(|s| s.to_string()),
            secrets: None,
            secrets_file: None,
            age_key_file: None,
        },
    );
    if registry.layers.is_empty() {
        registry.layers.push(name.to_string());
    }
    save_registry(&registry)
}

/// Resolve the active context.
///
/// Precedence:
/// 1. WORKESTRATE_CONTEXT env (set by --context flag or by user)
/// 2. [settings] default_context
/// 3. If NO contexts defined: bare `layers` (backward compat)
///
/// Returns ActiveContext { name: None, layers: registry.layers } when no
/// contexts are defined (backward-compat). Returns an error if contexts are
/// defined but neither env nor default_context resolves to a valid context.
pub fn resolve_active_context() -> Result<ActiveContext> {
    let registry = match load_registry()? {
        Some(r) => r,
        None => {
            return Ok(ActiveContext {
                name: None,
                layers: vec![],
            })
        }
    };

    // Backward-compat: no contexts defined → use bare layers.
    if registry.contexts.is_empty() {
        return Ok(ActiveContext {
            name: None,
            layers: registry.layers,
        });
    }

    // 1. WORKESTRATE_CONTEXT env (set by --context flag or by user)
    if let Ok(name) = std::env::var("WORKESTRATE_CONTEXT") {
        if let Some(ctx) = registry.contexts.get(&name) {
            return Ok(ActiveContext {
                name: Some(name),
                layers: ctx.layers.clone(),
            });
        }
        anyhow::bail!(
            "context '{}' not found in registry; available contexts: {}",
            name,
            registry
                .contexts
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // 2. [settings] default_context
    if let Some(ref default) = registry.settings.default_context {
        if let Some(ctx) = registry.contexts.get(default) {
            return Ok(ActiveContext {
                name: Some(default.clone()),
                layers: ctx.layers.clone(),
            });
        }
        anyhow::bail!(
            "default_context '{}' not found in registry contexts; available: {}",
            default,
            registry
                .contexts
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // 3. Contexts exist but no env/default → error
    anyhow::bail!(
        "contexts are defined but no default_context is set; use --context <name> or set WORKESTRATE_CONTEXT env. Available contexts: {}",
        registry.contexts.keys().cloned().collect::<Vec<_>>().join(", ")
    );
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_in_result
    )]
    use super::*;
    use crate::config::test_support::*;

    #[test]
    fn resolve_active_context_no_registry_uses_empty_layers() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-no-reg-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp_home)?;
        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let ctx = resolve_active_context()?;

        // Restore
        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        assert_eq!(ctx.name, None);
        assert!(ctx.layers.is_empty());
        Ok(())
    }

    #[test]
    fn resolve_active_context_no_contexts_uses_bare_layers() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-bare-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        // Registry with bare layers, no contexts
        std::fs::write(
            config_dir.join("config.toml"),
            "layers = [\"work\", \"personal\"]\n\n[settings]\ndefault_context = \"personal\"\n",
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let ctx = resolve_active_context()?;

        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        // No contexts defined → bare layers, name is None (backward compat)
        assert_eq!(ctx.name, None);
        assert_eq!(ctx.layers, vec!["work", "personal"]);
        Ok(())
    }

    #[test]
    fn resolve_active_context_env_overrides_default() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-env-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        std::fs::write(
            config_dir.join("config.toml"),
            "[settings]\ndefault_context = \"personal\"\n\n[contexts.personal]\nlayers = [\"personal\"]\n\n[contexts.work]\nlayers = [\"team\", \"personal\"]\n",
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::set_var("WORKESTRATE_CONTEXT", "work");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let ctx = resolve_active_context()?;

        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        assert_eq!(ctx.name.as_deref(), Some("work"));
        assert_eq!(ctx.layers, vec!["team", "personal"]);
        Ok(())
    }

    #[test]
    fn resolve_active_context_default_when_no_env() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-default-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        std::fs::write(
            config_dir.join("config.toml"),
            "[settings]\ndefault_context = \"personal\"\n\n[contexts.personal]\nlayers = [\"personal\"]\n\n[contexts.work]\nlayers = [\"team\", \"personal\"]\n",
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let ctx = resolve_active_context()?;

        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        assert_eq!(ctx.name.as_deref(), Some("personal"));
        assert_eq!(ctx.layers, vec!["personal"]);
        Ok(())
    }

    #[test]
    fn resolve_active_context_unknown_env_errors() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-unknown-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        std::fs::write(
            config_dir.join("config.toml"),
            "[settings]\ndefault_context = \"personal\"\n\n[contexts.personal]\nlayers = [\"personal\"]\n",
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::set_var("WORKESTRATE_CONTEXT", "nonexistent");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let result = resolve_active_context();

        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found"),
            "error should mention 'not found': {err}"
        );
        Ok(())
    }

    #[test]
    fn resolve_active_context_contexts_but_no_default_no_env_errors() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-nodflt-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        // Contexts defined but no default_context and no env
        std::fs::write(
            config_dir.join("config.toml"),
            "[contexts.personal]\nlayers = [\"personal\"]\n\n[contexts.work]\nlayers = [\"team\"]\n",
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let result = resolve_active_context();

        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("default_context") || err.contains("--context"),
            "error should mention default_context or --context: {err}"
        );
        Ok(())
    }
}
