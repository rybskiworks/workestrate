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

pub fn check_required_files(root: &Path) -> Result<Vec<(String, bool)>> {
    let mut checks = Vec::new();

    let paths = vec![
        ("ai-workbench root", root.to_path_buf()),
        (
            "infra/litellm/config.yaml",
            root.join("infra/litellm/config.yaml"),
        ),
        (
            "infra/microsandbox/sdk-notes.md",
            root.join("infra/microsandbox/sdk-notes.md"),
        ),
        ("profiles/litellm.md", root.join("profiles/litellm.md")),
        ("profiles/agents/pi.md", root.join("profiles/agents/pi.md")),
        (
            "profiles/agents/odysseus.md",
            root.join("profiles/agents/odysseus.md"),
        ),
        ("workspaces/", root.join("workspaces")),
        ("var/", root.join("var")),
        ("agents/pi (or flake input)", root.join("agents/pi")),
        (
            "agents/odysseus (or flake input)",
            root.join("agents/odysseus"),
        ),
    ];

    for (label, path) in paths {
        let exists = if label.starts_with("agents/") {
            // Agent check: local override OR flake input is OK.
            // For now, just check local path. Flake input check is future work.
            path.exists()
        } else {
            path.exists()
        };
        checks.push((label.to_string(), exists));
    }

    Ok(checks)
}
