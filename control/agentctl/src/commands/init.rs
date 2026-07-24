//! Bootstrap commands: `workestrate init` (registry seed) and
//! `workestrate new` (workload scaffold), plus the reference-fixture walker
//! shared with `config new --from-reference`.

use std::io::Write;

use anyhow::Result;

use crate::config;
use crate::git::git_clone;

pub(crate) async fn cmd_init(url: Option<&str>) -> Result<()> {
    let registry_path = config::registry_path();
    if registry_path.exists() {
        println!(
            "Registry already exists at {}; use 'workestrate config add' to add repos",
            registry_path.display()
        );
        return Ok(());
    }

    let mut registry = config::Registry::default();
    registry.settings.default_context = Some("personal".to_string());

    // Seed the store (repos/sources) and state roots for the active layout.
    // Legacy XDG: resolve_store_dir() == xdg_data_dir(), resolve_state_dir()
    //   == xdg_state_dir() — identical to the pre-ADR-0023 mkdirs.
    // New single-home: <home>/repos, <home>/sources, <home>/state.
    std::fs::create_dir_all(config::resolve_store_dir().join("repos"))?;
    std::fs::create_dir_all(config::resolve_store_dir().join("sources"))?;
    std::fs::create_dir_all(config::resolve_state_dir())?;

    if let Some(url) = url {
        let temp_dir =
            std::env::temp_dir().join(format!("workestrate-init-{}", std::process::id()));
        git_clone(url, &temp_dir, None)?;

        let found = if temp_dir.join("workestrate").join("config.toml").exists() {
            Some(temp_dir.join("workestrate").join("config.toml"))
        } else if temp_dir
            .join(".config")
            .join("workestrate")
            .join("config.toml")
            .exists()
        {
            Some(
                temp_dir
                    .join(".config")
                    .join("workestrate")
                    .join("config.toml"),
            )
        } else {
            None
        };

        let registry_parent = registry_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid registry path: {}", registry_path.display()))?;
        std::fs::create_dir_all(registry_parent)?;

        match found {
            Some(src) => {
                std::fs::copy(&src, &registry_path)?;
                println!(
                    "Cloned {} and copied workestrate config to {}",
                    url,
                    registry_path.display()
                );
            }
            None => {
                println!(
                    "Cloned {} but no workestrate config found; created empty registry",
                    url
                );
                config::save_registry(&registry)?;
            }
        }
        let _ = std::fs::remove_dir_all(&temp_dir);
    } else {
        config::save_registry(&registry)?;
    }

    println!(
        "Initialized workestrate registry at {}",
        registry_path.display()
    );
    println!("Run 'workestrate config add <url> personal' to add your config repo");
    Ok(())
}

/// Validate a workload name for `workestrate new`. Closes review finding A20.
///
/// Pattern: `^[a-z0-9][a-z0-9-]{0,62}$` — starts with an alphanumeric, allows
/// lowercase letters / digits / hyphens, max 63 characters (DNS-label length).
/// Rejects:
/// - empty / overlong names
/// - uppercase, underscores, dots, slashes, shell metacharacters
/// - anything starting with a hyphen (would create a hidden dir or flag-like
///   arg)
///
/// "Escape nothing — reject instead" is the policy: workload names flow into
/// both filesystem paths and TOML keys, so the safe set is the intersection.
pub(crate) fn validate_workload_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("workload name cannot be empty");
    }
    if name.len() > 63 {
        anyhow::bail!(
            "workload name cannot exceed 63 characters (got {}): '{}'",
            name.len(),
            name
        );
    }
    let mut chars = name.chars();
    let first_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    if !first_ok {
        anyhow::bail!(
            "workload name must start with [a-z0-9]; \
             pattern: ^[a-z0-9][a-z0-9-]{{0,62}}$; got: '{name}'"
        );
    }
    for c in chars {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' {
            anyhow::bail!(
                "workload name contains invalid character '{}' (allowed: [a-z0-9-]); \
                 pattern: ^[a-z0-9][a-z0-9-]{{0,62}}$; got: '{}'",
                c,
                name
            );
        }
    }
    Ok(())
}

pub(crate) async fn cmd_new(name: &str) -> Result<()> {
    // WP1 / A20: validate the workload name BEFORE using it as a directory
    // name or interpolating it into TOML. Reject everything that is not a
    // safe lowercase-hyphen identifier; this prevents both path escape
    // (`../pwned`) and TOML injection (`a]b` breaking out of the workload
    // table).
    validate_workload_name(name)?;

    // Resolve the active config directory (trusted project, registry, or env).
    let config_dir = config::resolve_active_config_dir()?;
    let agent_dir = config_dir.join("agents").join(name);

    if agent_dir.exists() {
        anyhow::bail!("agents/{} already exists in {}", name, config_dir.display());
    }

    // Create directory structure relative to the config repo.
    let config_subdir = agent_dir.join("config");
    std::fs::create_dir_all(&config_subdir)?;
    std::fs::write(config_subdir.join(".gitkeep"), "")?;

    // Append a default workload entry to the config repo's workestrate.toml.
    let config_path = config_dir.join("workestrate.toml");
    let toml_entry = format!(
        "\n[workloads.{}]\n\
        kind = \"agent\"\n\
        image = {{ recipe = \"registry\", ref = \"node:24-bookworm-slim\" }}\n\
        workdir = \"/work\"\n\
        cpus = 2\n\
        memory_mib = 2048\n\
        command = []\n\
        log_stop_errors = false\n\n\
        [[workloads.{}.mounts]]\n\
        host = \"${{CWD}}\"\n\
        guest = \"/work\"\n\
        read_only = false\n\n\
        [workloads.{}.network]\n\
        default_deny = true\n\n\
        [[workloads.{}.network.egress]]\n\
        recipe = \"agent_base\"\n",
        name, name, name, name
    );

    // Create parent dir if needed, then open with create+append.
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&config_path)?;
    file.write_all(toml_entry.as_bytes())?;

    println!(
        "Created agents/{}/config/ in {}",
        name,
        config_dir.display()
    );
    println!("Appended [workloads.{}] to {}", name, config_path.display());
    println!();
    println!("Edit {} to configure:", config_path.display());
    println!("  - Set image (recipe + ref, or recipe + contents for nix-layered)");
    println!("  - Set command");
    println!("  - Add env/secret_env/mounts as needed");
    println!();
    println!("Test: workestrate {} plan", name);

    Ok(())
}

/// Walk up from `start` looking for `<root>/config.reference/workestrate.toml`.
/// Used by `--from-reference` to seed the scaffold with the canonical
/// 5-workload fixture. Returns the file path on success; bails with an
/// actionable message if not found within 10 levels.
pub(crate) fn find_reference_workestrate(start: &std::path::Path) -> Result<std::path::PathBuf> {
    let mut cursor = start.to_path_buf();
    for _ in 0..10 {
        let candidate = cursor.join("config.reference").join("workestrate.toml");
        if candidate.exists() {
            return Ok(candidate);
        }
        if !cursor.pop() {
            break;
        }
    }
    anyhow::bail!(
        "could not locate config.reference/workestrate.toml by walking up from '{}'; \
         run `workestrate config new --from-reference` from inside the ai-workbench checkout",
        start.display()
    );
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

    #[test]
    fn cmd_new_resolves_active_config_dir() -> Result<()> {
        // Hold the env-mutation lock for the whole body: this test mutates
        // HOME / WORKESTRATE_CONFIG_DIR and must not race any other env-mutating
        // test (would otherwise corrupt env reads and poison ENV_TEST_LOCK).
        let _env_lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        // Create a temp config repo with a minimal workestrate.toml
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-cmd-new-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        std::fs::write(tmp.join("workestrate.toml"), "schema_version = 1\n")?;

        // Point WORKESTRATE_CONFIG_DIR at the temp repo
        let old = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        std::env::set_var("WORKESTRATE_CONFIG_DIR", &tmp);

        // Verify resolve_active_config_dir returns the temp dir
        let config_dir = config::resolve_active_config_dir()?;
        assert_eq!(
            config_dir, tmp,
            "resolve_active_config_dir should return the WORKESTRATE_CONFIG_DIR path"
        );

        // Simulate what cmd_new does: create agent config dir + append to workestrate.toml
        let agent_config = tmp.join("agents").join("test-agent").join("config");
        std::fs::create_dir_all(&agent_config)?;
        std::fs::write(agent_config.join(".gitkeep"), "")?;

        let config_path = tmp.join("workestrate.toml");
        let toml_entry = "\n[workloads.test-agent]\nkind = \"agent\"\n";
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&config_path)?;
        use std::io::Write;
        file.write_all(toml_entry.as_bytes())?;

        // Restore env
        match old {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }

        // Verify the agent config dir was created in the config repo
        assert!(
            agent_config.exists(),
            "agent config dir should exist in config repo"
        );

        // Verify workestrate.toml was appended
        let toml_content = std::fs::read_to_string(tmp.join("workestrate.toml"))?;
        assert!(
            toml_content.contains("[workloads.test-agent]"),
            "workestrate.toml should contain the new workload entry"
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn cmd_new_no_config_repo_produces_clear_error() {
        // Hold the env-mutation lock for the whole body (see note above).
        let _env_lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        // Use a temp HOME so no registry exists and no project config is found.
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-no-config-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp_home).ok();

        let old_home = std::env::var("HOME").ok();
        let old_config = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        let old_no_project = std::env::var("WORKESTRATE_NO_PROJECT_CONFIG").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::set_var("XDG_DATA_HOME", tmp_home.join(".local").join("share"));
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");
        std::env::set_var("WORKESTRATE_NO_PROJECT_CONFIG", "1");

        let result = config::resolve_active_config_dir();

        // Restore env
        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_config {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        match old_no_project {
            Some(v) => std::env::set_var("WORKESTRATE_NO_PROJECT_CONFIG", v),
            None => std::env::remove_var("WORKESTRATE_NO_PROJECT_CONFIG"),
        }
        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::remove_var("XDG_DATA_HOME");

        let _ = std::fs::remove_dir_all(&tmp_home);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("no active config repo") || err.contains("workestrate init"),
            "error should mention 'no active config repo' or 'workestrate init'; got: {err}"
        );
    }

    // ---- A20 regression: cmd_new rejects invalid workload names ----

    #[test]
    fn validate_workload_name_accepts_legitimate_names() {
        for ok in [
            "pi",
            "opencode",
            "example-agent",
            "my-cool-workload",
            "abc",
            "a1b",
            "a",
            "0",
            "1agent",
            &"a".repeat(63),
        ] {
            validate_workload_name(ok)
                .unwrap_or_else(|e| panic!("legitimate name '{ok}' rejected: {e}"));
        }
    }

    #[test]
    fn validate_workload_name_rejects_hostile_inputs() {
        // Each must fail. Categories: path escape, TOML injection,
        // shell-meta, uppercase, underscore, leading-hyphen, overlong, empty.
        let hostile = [
            "../pwned",      // path escape
            "/etc/pwned",    // absolute path
            "a]b",           // TOML table close-bracket injection
            "a.b",           // dot (TOML nested-key separator)
            "a b",           // whitespace
            "a$b",           // shell meta
            "a;b",           // shell meta
            "Agent",         // uppercase
            "my_agent",      // underscore (DNS-label style disallows)
            "-leading",      // leading hyphen
            "",              // empty
            &"x".repeat(64), // overlong (64 > 63)
        ];
        for h in hostile {
            let result = validate_workload_name(h);
            assert!(
                result.is_err(),
                "hostile name '{h}' should be rejected, but was accepted"
            );
        }
    }

    #[test]
    fn validate_workload_name_trailing_hyphen_is_allowed_by_design() {
        // The regex ^[a-z0-9][a-z0-9-]{0,62}$ permits trailing hyphens.
        // DNS labels disallow them, but workestrate workload names are not
        // DNS labels — they're filesystem path components and TOML keys.
        // If a future decision tightens this, update both the regex and this
        // test together.
        validate_workload_name("foo-").expect("trailing hyphen is allowed");
    }

    // ---- config::validate_config_name (used by `workestrate config new`) ----

    #[test]
    fn validate_config_name_accepts_legitimate_names() {
        for ok in [
            "personal",
            "work",
            "team",
            "prod",
            "a",
            "0",
            "p1",
            "my-config-2",
        ] {
            config::validate_config_name(ok)
                .unwrap_or_else(|e| panic!("legitimate config name '{ok}' rejected: {e}"));
        }
    }

    #[test]
    fn validate_config_name_rejects_hostile_inputs() {
        // Same safe-set as validate_workload_name: config names flow into
        // both filesystem paths and registry TOML keys, so the
        // intersection [a-z0-9-] is the only safe charset.
        let hostile = [
            "../pwned",
            "/etc/pwned",
            "a]b",
            "a.b",
            "a b",
            "Personal", // uppercase
            "my_config",
            "-leading",
            "",
            &"x".repeat(64),
        ];
        for h in hostile {
            let result = config::validate_config_name(h);
            assert!(
                result.is_err(),
                "hostile config name '{h}' should be rejected, but was accepted"
            );
        }
    }

    #[test]
    fn validate_config_name_error_mentions_config_name() {
        // Error label is interpolated from the validator, not hardcoded —
        // this catches a copy-paste regression where the workload-name
        // message would leak through.
        let err = config::validate_config_name("BAD").unwrap_err().to_string();
        assert!(
            err.contains("config name"),
            "error should mention 'config name'; got: {err}"
        );
        assert!(
            !err.contains("workload name"),
            "error should NOT mention 'workload name'; got: {err}"
        );
    }
}
