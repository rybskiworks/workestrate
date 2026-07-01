use super::plan::SandboxPlan;
use anyhow::Result;
use microsandbox::sandbox::SandboxBuilder;
use std::path::{Path, PathBuf};

fn resolve_mount_host(root: &Path, host: &str) -> Result<PathBuf> {
    if let Some(rest) = host.strip_prefix("${MSB_HOME}/") {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| anyhow::anyhow!("HOME not set"))?;
        Ok(home.join(".microsandbox").join(rest))
    } else {
        Ok(root.join(host))
    }
}

pub(crate) fn apply_plan_mounts(
    builder: SandboxBuilder,
    root: &Path,
    plan: &SandboxPlan,
) -> Result<SandboxBuilder> {
    let mut b = builder;
    for m in &plan.mounts {
        let host = resolve_mount_host(root, &m.host)?;
        b = b.volume(&m.guest, |v| {
            let v = v.bind(host);
            if m.read_only {
                v.readonly()
            } else {
                v
            }
        });
    }
    Ok(b)
}

pub(crate) fn ensure_mount_sources(root: &Path, plan: &SandboxPlan) -> Result<()> {
    for m in &plan.mounts {
        let path = resolve_mount_host(root, &m.host)?;
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

#[cfg(test)]
mod tests {
    use super::ensure_mount_sources;
    use crate::microsandbox::plan::{MountPlan, NetworkPlan, SandboxPlan};

    fn unique_root(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "agentctl-mounts-{}-{}-{}",
            label,
            std::process::id(),
            nanos,
        ))
    }

    fn minimal_plan(mounts: Vec<MountPlan>) -> SandboxPlan {
        SandboxPlan {
            name: "test".into(),
            image: None,
            workdir: None,
            command: vec![],
            cpus: None,
            memory_mib: None,
            env: vec![],
            secret_env: vec![],
            ports: vec![],
            mounts,
            network: NetworkPlan {
                default_deny: true,
                egress_rules: vec![],
                deny_rules: vec![],
                ingress_rules: vec![],
            },
        }
    }

    #[test]
    fn readwrite_mount_sources_are_auto_created() -> anyhow::Result<()> {
        let root = unique_root("rw");
        let plan = minimal_plan(vec![MountPlan::readwrite("nested/state", "/data")]);

        ensure_mount_sources(&root, &plan)?;

        let created = root.join("nested").join("state");
        assert!(
            created.is_dir(),
            "readwrite mount source should be auto-created at {}",
            created.display()
        );

        // Idempotent: re-running over an existing directory must not error.
        ensure_mount_sources(&root, &plan)?;

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn readonly_mount_sources_must_exist() -> anyhow::Result<()> {
        let root = unique_root("ro");
        let plan = minimal_plan(vec![MountPlan::readonly(
            "missing/config.json",
            "/app/config.json",
        )]);

        let result = ensure_mount_sources(&root, &plan);
        assert!(
            result.is_err(),
            "readonly mount source should bail when missing, got {:?}",
            result
        );

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }
}
