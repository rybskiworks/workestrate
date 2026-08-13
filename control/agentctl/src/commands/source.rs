//! Source-override commands (`workestrate source clone|build|list|reset`)
//! and the build-env-var helpers.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::cli_actions::SourceAction;
use crate::config;
use crate::git::{git_checkout_dot, git_checkout_rev, git_clone, git_clone_full};
use crate::merge::Provenance;

pub async fn cmd_source(action: SourceAction) -> Result<()> {
    match action {
        SourceAction::Clone { name, path } => cmd_source_clone(&name, path.as_deref()),
        SourceAction::Build { name } => cmd_source_build(&name),
        SourceAction::List => cmd_source_list(),
        SourceAction::Reset { name } => cmd_source_reset(&name),
    }
}

/// A `flake://` source resolved to a concrete clone URL + pinned revision
/// from the declaring config repo's `flake.lock`.
struct LockedSource {
    clone_url: String,
    rev: String,
}

/// Resolve `flake://<input>` to its locked clone URL + rev from the config
/// repo's `flake.lock`. Supports github- and git-type inputs. Returns `None`
/// when the lock file is missing or unparseable, or the node is absent or is
/// not a github/git node — the caller falls back to guidance.
fn locked_source_from_flake_lock(root: &Path, input: &str) -> Option<LockedSource> {
    let text = std::fs::read_to_string(root.join("flake.lock")).ok()?;
    let json: serde_json::Value = serde_json::from_str(&text).ok()?;
    let locked = json.get("nodes")?.get(input)?.get("locked")?;
    match locked.get("type")?.as_str()? {
        "github" => {
            let owner = locked.get("owner")?.as_str()?;
            let repo = locked.get("repo")?.as_str()?;
            let rev = locked.get("rev")?.as_str()?;
            Some(LockedSource {
                clone_url: format!("https://github.com/{owner}/{repo}"),
                rev: rev.to_string(),
            })
        }
        "git" => {
            let url = locked.get("url")?.as_str()?;
            let rev = locked.get("rev")?.as_str()?;
            Some(LockedSource {
                clone_url: url.to_string(),
                rev: rev.to_string(),
            })
        }
        _ => None,
    }
}

pub fn cmd_source_clone(name: &str, path: Option<&str>) -> Result<()> {
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
        // flake:// sources are materialized by the CONFIG REPO's flake (the
        // repo that declares this workload), not by the tool flake. Resolve
        // the declaring layer's content dir via the phase-0
        // provenance/layer-dir machinery, then walk up to its flake root.
        match config_repo_content_dir(name) {
            Some(content_dir) => match find_flake_root(&content_dir) {
                Some(root) => match locked_source_from_flake_lock(&root, input) {
                    Some(locked) => {
                        if dest.exists() {
                            anyhow::bail!("source path already exists: {}", dest.display());
                        }
                        let parent = dest.parent().ok_or_else(|| {
                            anyhow::anyhow!("invalid source path: {}", dest.display())
                        })?;
                        std::fs::create_dir_all(parent)?;
                        // Full clone (not `--depth 1`) so the pinned rev is
                        // reachable, then check it out detached.
                        git_clone_full(&locked.clone_url, &dest)?;
                        git_checkout_rev(&dest, &locked.rev)?;
                        println!(
                            "Materialized flake://{input} source at rev {} from the config repo's flake.lock to {}",
                            locked.rev,
                            dest.display()
                        );
                    }
                    None => {
                        println!(
                            "flake://{input} is declared by the config repo at {} but its flake.lock has no github/git lock node for input '{}'.",
                            root.display(),
                            input
                        );
                        println!(
                            "To materialize it, run: nix develop {} (materializes flake inputs), or clone manually to: {}",
                            root.display(),
                            dest.display()
                        );
                    }
                },
                None => {
                    println!(
                        "The config repo declaring workload '{}' (content dir: {}) has no flake.nix.",
                        name,
                        content_dir.display()
                    );
                    println!(
                        "To use this flake:// source, add a flake.nix to that config repo declaring an input named '{}', then materialize it with: nix develop <config-repo-root>",
                        input
                    );
                    println!("Or clone manually to: {}", dest.display());
                }
            },
            None => {
                println!(
                    "Could not resolve the config repo that declares workload '{}' (no provenance/layer-dir information available).",
                    name
                );
                println!(
                    "flake:// sources are materialized by the declaring config repo's flake: run 'nix develop <config-repo-root>' in the repo whose flake.nix declares an input named '{}', or clone manually to: {}",
                    input,
                    dest.display()
                );
            }
        }
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

/// Walk `start` and its ancestors for the nearest directory containing a
/// `flake.nix` — the flake root. Returns `None` when no ancestor is one.
pub fn find_flake_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|dir| dir.join("flake.nix").is_file())
        .map(Path::to_path_buf)
}

/// Explicit `AGENTCTL_ROOT` override (ADR 0028): the pinned root when the
/// env var is set AND it contains a `flake.nix`. Returns `None` otherwise —
/// a set-but-flakeless `AGENTCTL_ROOT` is NOT an error here; it just does
/// not participate in declaring-repo-derived resolution (the F2 gate falls
/// through to the declaring repo / legacy `project_root()` tiers).
///
/// This isolates the explicit-override tier from `project_root_optional`'s
/// fold (which would let the CWD tier win before the declaring repo is
/// tried) — the F2 gate's tier order is: AGENTCTL_ROOT → declaring repo →
/// legacy CWD gate (see `microsandbox/mounts.rs` `resolve_mount_roots_owned`).
pub fn flake_root_override() -> Option<PathBuf> {
    let root = std::env::var("AGENTCTL_ROOT").ok().map(PathBuf::from)?;
    if root.join("flake.nix").is_file() {
        Some(root)
    } else {
        None
    }
}

/// Pure core of [`config_repo_content_dir`]: resolve the content dir of the
/// layer that DECLARED `workload`'s local_build, from an explicit
/// provenance + layer-dirs pair.
///
/// Provenance keys are dot-paths (`workloads.<name>.local_build`; the merge
/// engine records local_build wholesale, so the `.source` sub-key is probed
/// first only for forward compatibility). Values are layer names; for
/// directory-mode config repos the layer name is `<repo>#<relpath>` and the
/// layer-dirs map carries its content dir (parent of the layer's source
/// file, per [`crate::merge::layer_dirs_from`]).
fn declaring_layer_content_dir_from(
    provenance: &Provenance,
    layer_dirs: &HashMap<String, PathBuf>,
    workload: &str,
) -> Option<PathBuf> {
    let layer = provenance
        .get(&format!("workloads.{workload}.local_build.source"))
        .or_else(|| provenance.get(&format!("workloads.{workload}.local_build")))?;
    layer_dirs.get(layer).cloned()
}

/// Content dir of the config-repo layer that declared `workload`'s
/// local_build, resolved from the process-global provenance + layer dirs
/// captured by the most recent `load_config` (phase 0). Returns `None` for
/// synthetic layers or when no load has recorded the state.
pub fn config_repo_content_dir(workload: &str) -> Option<PathBuf> {
    let provenance = crate::merge::get_provenance()?;
    let layer_dirs = crate::merge::get_layer_dirs()?;
    declaring_layer_content_dir_from(&provenance, &layer_dirs, workload)
}

/// Env-var name that overrides a workload's build directory for the current
/// session. Hyphens in the workload name become underscores so the variable is
/// settable in any shell (e.g. `my-agent` → `WORKESTRATE_MY_AGENT_BUILD`).
pub fn build_env_var_name(name: &str) -> String {
    format!(
        "WORKESTRATE_{}_BUILD",
        name.to_ascii_uppercase().replace('-', "_")
    )
}

pub fn build_command_string(name: &str, local_build: &config::LocalBuildConfig) -> String {
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

pub fn nix_available() -> bool {
    std::process::Command::new("nix")
        .arg("--version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn cmd_source_build(name: &str) -> Result<()> {
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

pub fn cmd_source_list() -> Result<()> {
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

pub fn cmd_source_reset(name: &str) -> Result<()> {
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;

    // --- FN-27: build-env var name sanitization -----------------------------

    #[test]
    fn build_env_var_name_sanitizes_hyphens() {
        assert_eq!(build_env_var_name("my-agent"), "WORKESTRATE_MY_AGENT_BUILD");
    }

    #[test]
    fn build_env_var_name_preserves_simple_names() {
        assert_eq!(build_env_var_name("litellm"), "WORKESTRATE_LITELLM_BUILD");
    }

    #[test]
    fn build_env_var_name_handles_multiple_hyphens() {
        assert_eq!(build_env_var_name("a-b-c"), "WORKESTRATE_A_B_C_BUILD");
    }

    /// Guards the `cmd_doctor` "Source overrides" site (see line ~3064), which
    /// previously built the env-var name with an unsanitized
    /// `format!("WORKESTRATE_{}_BUILD", name.to_uppercase())` that leaked a
    /// hyphen (e.g. `WORKESTRATE_MY-AGENT_BUILD`) for hyphenated workloads.
    #[test]
    fn build_env_var_name_doctor_site_no_hyphen_leak() {
        let env_var = build_env_var_name("my-agent");
        assert!(!env_var.contains('-'));
        assert_eq!(env_var, "WORKESTRATE_MY_AGENT_BUILD");
    }

    // --- flake:// config-repo flake-root resolution -------------------------

    /// Unique temp dir per test invocation (same pattern as the integration
    /// tests; no tempfile dep).
    fn uniq_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "workestrate-source-{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ))
    }

    #[test]
    fn find_flake_root_walks_up_to_nearest_flake_nix() {
        let root = uniq_dir("flake-root");
        let nested = root.join("workestrate").join("workloads").join("pi");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(root.join("flake.nix"), "{}\n").unwrap();

        assert_eq!(find_flake_root(&nested).as_deref(), Some(root.as_path()));
        assert_eq!(
            find_flake_root(&root).as_deref(),
            Some(root.as_path()),
            "the flake root itself resolves"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn find_flake_root_prefers_nearest_ancestor() {
        let outer = uniq_dir("flake-outer");
        let inner = outer.join("config-repo");
        let nested = inner.join("workestrate");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(outer.join("flake.nix"), "{}\n").unwrap();
        std::fs::write(inner.join("flake.nix"), "{}\n").unwrap();

        assert_eq!(
            find_flake_root(&nested).as_deref(),
            Some(inner.as_path()),
            "the NEAREST ancestor with flake.nix wins"
        );

        let _ = std::fs::remove_dir_all(&outer);
    }

    #[test]
    fn find_flake_root_returns_none_when_absent() {
        let root = uniq_dir("flake-less");
        let nested = root.join("a").join("b");
        std::fs::create_dir_all(&nested).unwrap();
        // Walk starts at `nested`; nothing below / has a flake.nix in this
        // tree, but ancestors ABOVE `root` (e.g. /tmp) must not leak in — so
        // assert against the tree itself: no ancestor within `root` matches.
        assert!(
            find_flake_root(&nested).is_none_or(|found| !found.starts_with(&root)),
            "no flake.nix inside the temp tree must not resolve into it"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// The pure resolution core: provenance dot-path + layer-dirs map → the
    /// declaring layer's content dir. Directory-mode layer names are
    /// `<repo>#<relpath>`; the map lookup is by exact layer name.
    #[test]
    fn declaring_layer_content_dir_resolves_via_provenance_and_layer_dirs() {
        let mut provenance = Provenance::new();
        provenance.insert(
            "workloads.pi.local_build".to_string(),
            "personal#workestrate/workloads/pi.toml".to_string(),
        );
        let mut layer_dirs = HashMap::new();
        layer_dirs.insert(
            "personal#workestrate/workloads/pi.toml".to_string(),
            PathBuf::from("/repo/workestrate/workloads"),
        );

        assert_eq!(
            declaring_layer_content_dir_from(&provenance, &layer_dirs, "pi"),
            Some(PathBuf::from("/repo/workestrate/workloads"))
        );
        assert_eq!(
            declaring_layer_content_dir_from(&provenance, &layer_dirs, "odysseus"),
            None,
            "a workload with no local_build provenance resolves to None"
        );
    }

    /// A provenance entry whose layer has no recorded content dir (synthetic
    /// layer) resolves to None — the caller falls back to generic guidance.
    #[test]
    fn declaring_layer_content_dir_none_for_synthetic_layer() {
        let mut provenance = Provenance::new();
        provenance.insert(
            "workloads.pi.local_build".to_string(),
            "synthetic".to_string(),
        );
        let layer_dirs = HashMap::new();

        assert_eq!(
            declaring_layer_content_dir_from(&provenance, &layer_dirs, "pi"),
            None
        );
    }

    // --- flake:// materialization: flake.lock parsing ----------------------

    #[test]
    fn locked_source_github_type_resolves_clone_url_and_rev() {
        let root = uniq_dir("flake-lock-github");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("flake.lock"),
            r#"{
  "nodes": {
    "odysseus": {
      "locked": {
        "type": "github",
        "owner": "georgrybski",
        "repo": "odysseus",
        "rev": "fc8e6366ddb627935af092ba81ea5faa5d05e1b3"
      }
    }
  }
}"#,
        )
        .unwrap();

        let locked = locked_source_from_flake_lock(&root, "odysseus").expect("resolves");
        assert_eq!(locked.clone_url, "https://github.com/georgrybski/odysseus");
        assert_eq!(locked.rev, "fc8e6366ddb627935af092ba81ea5faa5d05e1b3");

        let _ = std::fs::remove_dir_all(&root);
    }

    /// Exact-case node key match: `flake://tempest` maps to the `T3MP3ST`
    /// input (repo name is uppercase in the config flake).
    #[test]
    fn locked_source_exact_case_key_match() {
        let root = uniq_dir("flake-lock-case");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("flake.lock"),
            r#"{
  "nodes": {
    "T3MP3ST": {
      "locked": {
        "type": "github",
        "owner": "georgrybski",
        "repo": "T3MP3ST",
        "rev": "ae32cf505174a422c55d7ca970f5f23816218f38"
      }
    }
  }
}"#,
        )
        .unwrap();

        let locked =
            locked_source_from_flake_lock(&root, "T3MP3ST").expect("resolves exact-case key");
        assert_eq!(locked.clone_url, "https://github.com/georgrybski/T3MP3ST");
        assert_eq!(locked.rev, "ae32cf505174a422c55d7ca970f5f23816218f38");
        assert!(locked_source_from_flake_lock(&root, "tempest").is_none());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn locked_source_git_type_resolves_url_and_rev() {
        let root = uniq_dir("flake-lock-git");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("flake.lock"),
            r#"{
  "nodes": {
    "workestrate": {
      "locked": {
        "type": "git",
        "url": "file:///home/rybski/Development/agent-workbench/workestrate",
        "rev": "c45494bd1ec827489422f631294b003b34ec59f5"
      }
    }
  }
}"#,
        )
        .unwrap();

        let locked = locked_source_from_flake_lock(&root, "workestrate").expect("resolves");
        assert_eq!(
            locked.clone_url,
            "file:///home/rybski/Development/agent-workbench/workestrate"
        );
        assert_eq!(locked.rev, "c45494bd1ec827489422f631294b003b34ec59f5");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn locked_source_missing_node_returns_none() {
        let root = uniq_dir("flake-lock-missing");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("flake.lock"),
            r#"{
  "nodes": {
    "pi": {
      "locked": {
        "type": "github",
        "owner": "georgrybski",
        "repo": "pi",
        "rev": "371adcf37130629ffb9bbeed9f5548ce08ffa93b"
      }
    }
  }
}"#,
        )
        .unwrap();

        assert!(locked_source_from_flake_lock(&root, "odysseus").is_none());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn locked_source_unparseable_lock_returns_none() {
        let root = uniq_dir("flake-lock-bad");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("flake.lock"), "not json at all {").unwrap();

        assert!(locked_source_from_flake_lock(&root, "pi").is_none());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn locked_source_missing_lock_file_returns_none() {
        let root = uniq_dir("flake-lock-absent");
        std::fs::create_dir_all(&root).unwrap();

        assert!(locked_source_from_flake_lock(&root, "pi").is_none());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn locked_source_non_github_git_node_returns_none() {
        let root = uniq_dir("flake-lock-path");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("flake.lock"),
            r#"{
  "nodes": {
    "nixpkgs": {
      "locked": {
        "type": "path",
        "path": "/nix/store/00000000000000000000000000000000-source"
      }
    }
  }
}"#,
        )
        .unwrap();

        assert!(locked_source_from_flake_lock(&root, "nixpkgs").is_none());

        let _ = std::fs::remove_dir_all(&root);
    }
}
