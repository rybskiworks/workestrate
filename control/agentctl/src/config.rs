//! Paths and locations used across commands.
//!
//! `agentctl` runs from inside the project root (where the user invokes
//! `cargo run -- ...` or, in production, the binary itself). All filesystem
//! locations are resolved relative to a `project_root` that defaults to
//! the current working directory. Each command can override it via the
//! `AGENTCTL_ROOT` env var or a `--root` flag in the future.

use std::path::PathBuf;

/// Resolve the project root.
///
/// Order of precedence:
///   1. `AGENTCTL_ROOT` env var (if set and non-empty)
///   2. `CARGO_MANIFEST_DIR` (when running via `cargo run` from a crate
///      inside `control/agentctl`, this points at `control/agentctl`,
///      so we walk up two levels to reach the project root)
///   3. Current working directory
pub fn project_root() -> PathBuf {
    if let Ok(p) = std::env::var("AGENTCTL_ROOT") {
        if !p.is_empty() {
            return PathBuf::from(p);
        }
    }
    if let Ok(p) = std::env::var("CARGO_MANIFEST_DIR") {
        // CARGO_MANIFEST_DIR points at control/agentctl when run via
        // `cargo run --manifest-path control/agentctl/Cargo.toml`.
        let p = PathBuf::from(p);
        if let Some(root) = p.parent().and_then(|p| p.parent()) {
            return root.to_path_buf();
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// `infra/litellm/config.yaml`
pub fn litellm_config_path() -> PathBuf {
    project_root().join("infra").join("litellm").join("config.yaml")
}

/// `infra/litellm/.env`
pub fn env_file_path() -> PathBuf {
    project_root().join("infra").join("litellm").join(".env")
}

/// `infra/litellm/.env.example`
pub fn env_example_path() -> PathBuf {
    project_root().join("infra").join("litellm").join(".env.example")
}

/// Agent definitions directory (created by `agentctl init`).
pub fn agents_dir() -> PathBuf {
    project_root().join("agents")
}

/// Per-agent workspaces (created by `agentctl init`).
pub fn workspaces_dir() -> PathBuf {
    project_root().join("workspaces")
}

/// Runtime state directory (created by `agentctl init`).
pub fn var_dir() -> PathBuf {
    project_root().join("var")
}

/// Scratch directory (created by `agentctl init`).
pub fn tmp_dir() -> PathBuf {
    project_root().join("tmp")
}


