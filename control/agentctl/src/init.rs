//! `agentctl init` — create the on-disk layout the rest of the CLI expects.
//!
//! The POC already has `var/` from the bash scripts. We don't want to
//! clobber existing state. We only create what doesn't already exist.

use std::fs;
use std::path::Path;

use crate::config;

pub fn run() -> Result<u8, String> {
    let root = config::project_root();
    eprintln!("agentctl init: project root = {}", root.display());

    let created = create_if_missing(&config::agents_dir())?;
    let workspaces = create_if_missing(&config::workspaces_dir())?;
    let var = create_if_missing(&config::var_dir())?;
    let tmp = create_if_missing(&config::tmp_dir())?;
    eprintln!(
        "  agents/      = {} {}",
        config::agents_dir().display(),
        tag(created)
    );
    eprintln!(
        "  workspaces/  = {} {}",
        config::workspaces_dir().display(),
        tag(workspaces)
    );
    eprintln!(
        "  var/         = {} {}",
        config::var_dir().display(),
        tag(var)
    );
    eprintln!(
        "  tmp/         = {} {}",
        config::tmp_dir().display(),
        tag(tmp)
    );

    // Also create a .gitkeep in agents/ and workspaces/ so the directories
    // are visible in `git status` and the layout is reproducible.
    for d in [config::agents_dir(), config::workspaces_dir()] {
        let keep = d.join(".gitkeep");
        if !keep.exists() {
            let _ = fs::write(&keep, b"# created by agentctl init\n");
        }
    }

    // Copy .env.example -> .env if .env doesn't exist. The POC already
    // ships with a real .env (gitignored, mode 0600), so this is a no-op
    // in this environment.
    let env = config::env_file_path();
    let env_example = config::env_example_path();
    if !env.exists() && env_example.exists() {
        fs::copy(&env_example, &env).map_err(|e| {
            format!(
                "failed to copy {} -> {}: {}",
                env_example.display(),
                env.display(),
                e
            )
        })?;
        eprintln!(
            "  .env         = {} (copied from .env.example — fill in real values)",
            env.display()
        );
    } else if env.exists() {
        eprintln!("  .env         = {} (exists, not modified)", env.display());
    } else {
        eprintln!("  .env         = (no .env.example found; skipped)");
    }

    println!("agentctl init: layout ready");
    Ok(0)
}

fn create_if_missing(path: &Path) -> Result<bool, String> {
    if path.exists() {
        return Ok(false);
    }
    fs::create_dir_all(path)
        .map_err(|e| format!("failed to create {}: {}", path.display(), e))?;
    Ok(true)
}

fn tag(created: bool) -> &'static str {
    if created { "(created)" } else { "(exists)" }
}
