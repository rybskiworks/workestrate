//! Bootstrap commands: `workestrate init` (registry seed) and
//! `workestrate new` (workload scaffold), plus the reference-fixture walker
//! shared with `config new --from-reference`.
//!
//! NOTE (WP4-B): verbatim copies of the private items still live in
//! `main.rs`. Commit C cuts `main.rs` over to these and deletes its own.

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
