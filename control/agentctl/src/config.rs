use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn project_root() -> Result<PathBuf> {
    // 1. AGENTCTL_ROOT env var
    let root = if let Ok(root) = std::env::var("AGENTCTL_ROOT") {
        PathBuf::from(root)
    }
    // 2. Walk up from CARGO_MANIFEST_DIR (compile-time, works in cargo run)
    else if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut path = PathBuf::from(manifest);
        if path.pop() && path.pop() {
            path
        } else {
            std::env::current_dir()?
        }
    }
    // 3. Current working directory
    else {
        std::env::current_dir()?
    };

    // Validate: the root must contain flake.nix
    if !root.join("flake.nix").exists() {
        anyhow::bail!(
            "resolved project root '{}' does not contain flake.nix.\n\
             Set AGENTCTL_ROOT or run from the workbench root directory.",
            root.display()
        );
    }

    Ok(root)
}

/// One row in the `agentctl check` report.
#[derive(Debug, Clone)]
pub struct CheckEntry {
    /// Human-readable label printed in the report.
    pub label: String,
    /// Whether the required artifact is present.
    pub ok: bool,
    /// When `true`, a missing artifact is reported as a warning and does
    /// not cause the command to exit non-zero. Used for optional local
    /// overrides such as `agents/<name>/repo` checkouts.
    pub optional: bool,
}

struct CheckSpec {
    label: &'static str,
    path: PathBuf,
    optional: bool,
}

fn required(label: &'static str, path: PathBuf) -> CheckSpec {
    CheckSpec {
        label,
        path,
        optional: false,
    }
}

fn optional(label: &'static str, path: PathBuf) -> CheckSpec {
    CheckSpec {
        label,
        path,
        optional: true,
    }
}

/// Run the full set of sanity checks for the workbench layout.
///
/// `agents/<name>/repo` directories are documented as optional local
/// overrides (the agents can also be supplied via flake inputs), so missing
/// directories are reported as `[MISSING] (optional)` and do not fail the
/// command. Any other missing artifact is fatal.
pub fn check_required_files(root: &Path) -> Result<Vec<CheckEntry>> {
    let specs: Vec<CheckSpec> = vec![
        required("ai-workbench root", root.to_path_buf()),
        required("flake.nix", root.join("flake.nix")),
        required(
            "infra/litellm/config.yaml",
            root.join("infra/litellm/config.yaml"),
        ),
        required(
            "infra/litellm/models.yaml",
            root.join("infra/litellm/models.yaml"),
        ),
        required(
            "infra/microsandbox/sdk-notes.md",
            root.join("infra/microsandbox/sdk-notes.md"),
        ),
        required("profiles/litellm.md", root.join("profiles/litellm.md")),
        required("profiles/agents/pi.md", root.join("profiles/agents/pi.md")),
        required(
            "profiles/agents/odysseus.md",
            root.join("profiles/agents/odysseus.md"),
        ),
        required(
            "profiles/agents/opencode.md",
            root.join("profiles/agents/opencode.md"),
        ),
        required(
            "profiles/agents/tempest.md",
            root.join("profiles/agents/tempest.md"),
        ),
        required("workspaces/", root.join("workspaces")),
        required("var/", root.join("var")),
        required(
            "agents/odysseus/config/settings.json",
            root.join("agents/odysseus/config/settings.json"),
        ),
        required(
            "agents/opencode/config/opencode.jsonc",
            root.join("agents/opencode/config/opencode.jsonc"),
        ),
        required(".env.enc", root.join(".env.enc")),
        required(".sops.yaml", root.join(".sops.yaml")),
        // Optional: agent repos are typically supplied via flake
        // inputs. A fresh clone may legitimately omit local
        // `agents/<name>/repo` checkouts.
        optional("agents/pi/repo", root.join("agents/pi/repo")),
        optional("agents/odysseus/repo", root.join("agents/odysseus/repo")),
        optional("agents/opencode/repo", root.join("agents/opencode/repo")),
        optional("agents/pi/build", root.join("agents/pi/build")),
        optional("agents/odysseus/build", root.join("agents/odysseus/build")),
        optional("agents/opencode/build", root.join("agents/opencode/build")),
        optional("agents/tempest/repo", root.join("agents/tempest/repo")),
        optional("agents/tempest/build", root.join("agents/tempest/build")),
    ];

    Ok(specs
        .into_iter()
        .map(|spec| CheckEntry {
            label: spec.label.to_string(),
            ok: spec.path.exists(),
            optional: spec.optional,
        })
        .collect())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn project_root_with_agentctl_root_env() {
        std::env::set_var("AGENTCTL_ROOT", "/tmp");
        let result = project_root();
        std::env::remove_var("AGENTCTL_ROOT");
        // /tmp doesn't have flake.nix, so this should error
        assert!(result.is_err(), "expected error when flake.nix missing");
    }

    #[test]
    fn project_root_rejects_missing_flake_nix() {
        std::env::set_var("AGENTCTL_ROOT", "/tmp/nonexistent-ai-workbench-test");
        let result = project_root();
        std::env::remove_var("AGENTCTL_ROOT");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("flake.nix"),
            "error should mention flake.nix: {err}"
        );
    }

    #[test]
    fn check_required_files_finds_missing() {
        let tmp = std::env::temp_dir();
        let entries = check_required_files(&tmp).unwrap();
        // temp dir won't have flake.nix etc
        let missing_required: Vec<_> = entries.iter().filter(|e| !e.ok && !e.optional).collect();
        assert!(
            !missing_required.is_empty(),
            "expected missing required files"
        );
    }

    #[test]
    fn check_required_files_marks_optional_correctly() {
        let tmp = std::env::temp_dir();
        let entries = check_required_files(&tmp).unwrap();
        let optional_entries: Vec<_> = entries.iter().filter(|e| e.optional).collect();
        // Should have 8 optional entries (4 agents × repo + build)
        assert_eq!(
            optional_entries.len(),
            8,
            "expected 8 optional checks, got {}",
            optional_entries.len()
        );
    }
}
