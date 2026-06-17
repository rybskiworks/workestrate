use anyhow::Result;
use std::path::{Path, PathBuf};

pub fn project_root() -> Result<PathBuf> {
    // 1. AGENTCTL_ROOT env var
    if let Ok(root) = std::env::var("AGENTCTL_ROOT") {
        return Ok(PathBuf::from(root));
    }

    // 2. Walk up from CARGO_MANIFEST_DIR
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut path = PathBuf::from(manifest);
        // CARGO_MANIFEST_DIR is control/agentctl/, go up 2 to repo root
        if path.pop() && path.pop() {
            return Ok(path);
        }
    }

    // 3. Current working directory
    Ok(std::env::current_dir()?)
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
    /// overrides such as `agents/pi` and `agents/odysseus` checkouts.
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
/// `agents/pi` and `agents/odysseus` are documented as optional local
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
            "infra/microsandbox/sdk-notes.md",
            root.join("infra/microsandbox/sdk-notes.md"),
        ),
        required("profiles/litellm.md", root.join("profiles/litellm.md")),
        required("profiles/agents/pi.md", root.join("profiles/agents/pi.md")),
        required(
            "profiles/agents/odysseus.md",
            root.join("profiles/agents/odysseus.md"),
        ),
        required("workspaces/", root.join("workspaces")),
        required("var/", root.join("var")),
        // Optional: Pi and Odysseus are typically supplied via flake
        // inputs. A fresh clone may legitimately omit local
        // `agents/<name>` checkouts.
        optional("agents/pi (or flake input)", root.join("agents/pi")),
        optional(
            "agents/odysseus (or flake input)",
            root.join("agents/odysseus"),
        ),
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
