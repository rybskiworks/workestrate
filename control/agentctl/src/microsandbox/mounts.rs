use super::plan::SandboxPlan;
use anyhow::Result;
use microsandbox::sandbox::SandboxBuilder;
use std::path::{Path, PathBuf};

/// TODO: make this fallible (return `Result<PathBuf>`) in a future pass so
/// that a missing `HOME` is a clean error rather than a panic.
#[allow(clippy::expect_used)]
fn resolve_mount_host(root: &Path, host: &str) -> PathBuf {
    if let Some(rest) = host.strip_prefix("${MSB_HOME}/") {
        let home = std::env::var("HOME").expect("HOME not set");
        PathBuf::from(home).join(".microsandbox").join(rest)
    } else {
        root.join(host)
    }
}

pub(crate) fn apply_plan_mounts(
    builder: SandboxBuilder,
    root: &Path,
    plan: &SandboxPlan,
) -> SandboxBuilder {
    let mut b = builder;
    for m in &plan.mounts {
        let host = resolve_mount_host(root, &m.host);
        b = b.volume(&m.guest, |v| {
            let v = v.bind(host);
            if m.read_only {
                v.readonly()
            } else {
                v
            }
        });
    }
    b
}

pub(crate) fn ensure_mount_sources(root: &Path, plan: &SandboxPlan) -> Result<()> {
    for m in &plan.mounts {
        let path = resolve_mount_host(root, &m.host);
        if !path.exists() {
            if m.read_only {
                anyhow::bail!("mount source does not exist: {}", path.display());
            } else {
                std::fs::create_dir_all(&path)?;
            }
        }
    }
    Ok(())
}
