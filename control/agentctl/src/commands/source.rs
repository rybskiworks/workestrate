//! Source-override commands (`workestrate source clone|build|list|reset`)
//! and the build-env-var helpers.
//!
//! NOTE (WP4-B): verbatim copies of the private items still live in
//! `main.rs`. Commit C cuts `main.rs` over to these and deletes its own.

use std::path::PathBuf;

use anyhow::Result;

use crate::config;
use crate::git::{git_checkout_dot, git_clone};
use crate::SourceAction;

pub(crate) async fn cmd_source(action: SourceAction) -> Result<()> {
    match action {
        SourceAction::Clone { name, path } => cmd_source_clone(&name, path.as_deref()).await,
        SourceAction::Build { name } => cmd_source_build(&name).await,
        SourceAction::List => cmd_source_list().await,
        SourceAction::Reset { name } => cmd_source_reset(&name).await,
    }
}

pub(crate) async fn cmd_source_clone(name: &str, path: Option<&str>) -> Result<()> {
    let cfg = config::load_config()?;
    let workload = cfg
        .workloads
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("workload '{}' not found", name))?;
    let local_build = workload
        .local_build
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("workload '{}' has no local_build recipe", name))?;

    let dest = match path {
        Some(p) => PathBuf::from(p),
        None => config::source_store_dir(name).join("repo"),
    };

    if local_build.source.starts_with("flake://") {
        let input = local_build
            .source
            .strip_prefix("flake://")
            .unwrap_or(&local_build.source);
        println!(
            "To clone the canonical source, run: nix develop (materializes flake inputs). Or clone manually to: {}",
            dest.display()
        );
        println!("Flake input name: {}", input);
    } else {
        if dest.exists() {
            anyhow::bail!("source path already exists: {}", dest.display());
        }
        let parent = dest
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid source path: {}", dest.display()))?;
        std::fs::create_dir_all(parent)?;
        git_clone(&local_build.source, &dest, None)?;
        println!("Cloned {} source to {}", name, dest.display());
    }

    let env_var = build_env_var_name(name);
    println!("Set {}={} for this session", env_var, dest.display());
    Ok(())
}

/// Env-var name that overrides a workload's build directory for the current
/// session. Hyphens in the workload name become underscores so the variable is
/// settable in any shell (e.g. `my-agent` → `WORKESTRATE_MY_AGENT_BUILD`).
pub(crate) fn build_env_var_name(name: &str) -> String {
    format!(
        "WORKESTRATE_{}_BUILD",
        name.to_ascii_uppercase().replace('-', "_")
    )
}

pub(crate) fn build_command_string(name: &str, local_build: &config::LocalBuildConfig) -> String {
    match local_build.recipe.as_str() {
        "pip-install" | "pip" => {
            let target = local_build.target.as_deref().unwrap_or(".deps");
            let req = local_build
                .requirements_file
                .as_deref()
                .unwrap_or("requirements.txt");
            format!(
                "REQ=$([ -f requirements.lock ] && echo requirements.lock || echo {}) && python3.12 -m pip install --only-binary=:all: --break-system-packages --target ./{} -r \"$REQ\"",
                req, target
            )
        }
        "npm-build" | "npm" => "npm install && npm run build".to_string(),
        "bun-install" | "bun" => "HUSKY=0 bun install && bun run build".to_string(),
        other => format!("{} build recipe for {}", other, name),
    }
}

pub(crate) fn nix_available() -> bool {
    std::process::Command::new("nix")
        .arg("--version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub(crate) async fn cmd_source_build(name: &str) -> Result<()> {
    let cfg = config::load_config()?;
    let workload = cfg
        .workloads
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("workload '{}' not found", name))?;
    let local_build = workload
        .local_build
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("workload '{}' has no local_build recipe", name))?;

    let command = build_command_string(name, local_build);
    println!("Build command for {}: {}", name, command);

    let env_var = build_env_var_name(name);
    let build_dir = match std::env::var(&env_var) {
        Ok(path) => PathBuf::from(path),
        Err(_) => config::source_store_dir(name).join("build"),
    };

    let repo_dir = config::source_store_dir(name).join("repo");
    if repo_dir.exists() {
        if build_dir.exists() {
            std::fs::remove_dir_all(&build_dir)?;
        }
        if let Some(parent) = build_dir.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let status = std::process::Command::new("cp")
            .args([
                "-r",
                repo_dir.to_str().ok_or_else(|| {
                    anyhow::anyhow!(
                        "source repo path is not valid UTF-8: {}",
                        repo_dir.display()
                    )
                })?,
                build_dir.to_str().ok_or_else(|| {
                    anyhow::anyhow!(
                        "build directory path is not valid UTF-8: {}",
                        build_dir.display()
                    )
                })?,
            ])
            .status()?;
        if !status.success() {
            anyhow::bail!(
                "failed to copy source to build directory {}",
                build_dir.display()
            );
        }
    }

    if !build_dir.exists() {
        println!("Build directory {} does not exist.", build_dir.display());
        println!("Run: workestrate source clone {}", name);
        return Ok(());
    }

    if nix_available() {
        println!("Running build in {} via nix shell...", build_dir.display());
        let status = std::process::Command::new("nix")
            .args(["shell", ".", "--command", "bash", "-c", &command])
            .current_dir(&build_dir)
            .status()?;
        if !status.success() {
            anyhow::bail!("nix shell build failed for {}", name);
        }
        println!("Build succeeded: {}", build_dir.display());
    } else {
        println!("Nix not available. To build manually, run:");
        println!(
            "  cd {} && nix develop --command bash -c '{}'",
            build_dir.display(),
            command
        );
    }
    Ok(())
}

pub(crate) async fn cmd_source_list() -> Result<()> {
    let cfg = config::load_config()?;
    println!("Source overrides:");
    let mut found = false;
    for (name, workload) in &cfg.workloads {
        if workload.local_build.is_none() {
            continue;
        }
        found = true;
        let env_var = build_env_var_name(name);
        let repo = config::source_store_dir(name).join("repo");
        let build = config::source_store_dir(name).join("build");
        let repo_status = if repo.exists() {
            "checked out"
        } else {
            "not checked out"
        };
        let build_status = if build.exists() { "built" } else { "not built" };
        match std::env::var(&env_var) {
            Ok(path) => println!(
                "  {}: override {} (repo: {}, build: {})",
                name, path, repo_status, build_status
            ),
            Err(_) => println!(
                "  {}: no override ({} not set) (repo: {}, build: {})",
                name, env_var, repo_status, build_status
            ),
        }
    }
    if !found {
        println!("  (none)");
    }
    Ok(())
}

pub(crate) async fn cmd_source_reset(name: &str) -> Result<()> {
    let repo = config::source_store_dir(name).join("repo");
    if !repo.exists() {
        anyhow::bail!(
            "source for '{}' not checked out at {}",
            name,
            repo.display()
        );
    }
    git_checkout_dot(&repo)?;
    println!("Reset {} source to canonical", name);
    Ok(())
}
