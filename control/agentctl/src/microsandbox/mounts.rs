use super::plan::SandboxPlan;
use anyhow::Result;
use microsandbox::sandbox::SandboxBuilder;
use std::path::{Component, Path, PathBuf};

fn resolve_mount_host(root: &Path, host: &str) -> Result<PathBuf> {
    if let Some(rest) = host.strip_prefix("${MSB_HOME}/") {
        let home = std::env::var("HOME")
            .map(PathBuf::from)
            .map_err(|_| anyhow::anyhow!("HOME not set"))?;
        Ok(home.join(".microsandbox").join(rest))
    } else if host.starts_with("workspaces/") || host.starts_with("var/") {
        // Resolve to XDG state dir at runtime
        let state_dir = crate::config::resolve_state_dir();
        Ok(state_dir.join(host))
    } else {
        Ok(root.join(host))
    }
}

/// Validate a mount host string as it appears in the raw config TOML
/// (before `${CWD}` / `${WORKESTRATE_<NAME>_BUILD}` template substitution).
///
/// Rules (closes review finding A2):
/// 1. Reject empty strings.
/// 2. Reject absolute paths (leading `/`). Config authors must use a
///    template prefix (`${CWD}/...`, `${MSB_HOME}/...`) or a relative path
///    that resolves under the project root.
/// 3. Reject any `..` path component anywhere in the string. Template
///    prefixes are not traversal escapes — `${CWD}/../../etc` is still
///    rejected because the suffix contains `..`.
///
/// The permitted raw-value forms are:
/// - `${MSB_HOME}/...`, `${CWD}/...`, `${CWD}` (exact), `${WORKESTRATE_<NAME>_BUILD}` (exact)
/// - `workspaces/...`, `var/...` (resolved to XDG state dir)
/// - any other relative path with no `..` component (resolved to `root.join(host)`)
pub fn validate_mount_host(host: &str) -> Result<()> {
    if host.is_empty() {
        anyhow::bail!("mount host cannot be empty");
    }
    if host.starts_with('/') {
        anyhow::bail!(
            "mount host cannot be an absolute path (got '{host}'); use a relative path, \
             ${{CWD}}/..., ${{MSB_HOME}}/..., workspaces/..., or var/..."
        );
    }
    // Walk components and reject any `..`. Template prefixes like ${CWD} are
    // treated as normal path components — their `${...}` payload is opaque to
    // the path parser, so a literal `..` inside or after them is still caught.
    for component in Path::new(host).components() {
        if let Component::ParentDir = component {
            anyhow::bail!(
                "mount host contains '..' component (got '{host}'); \
                 path traversal is not allowed"
            );
        }
    }
    Ok(())
}

/// Validate a mount guest path (inside the sandbox). Closes review finding C4.
///
/// Rules:
/// 1. Reject empty strings.
/// 2. Require an absolute path (guests live inside the sandbox's rootfs).
/// 3. Reject any `..` component.
/// 4. Reject read-write mounts of guest paths that microsandbox pivots to
///    real host kernel resources: `/proc`, `/sys`, `/dev` and anything under
///    them. Read-only mounts are allowed (operator's explicit choice).
///
/// **Deviation from `80-remediation-plan.md` WP1:** the plan also named
/// `/etc`, `/root`, `/home` as sensitive. Those are *sandbox-internal* rootfs
/// directories (not host pivots) and are legitimate mount targets —
/// e.g. opencode mounts `/home/node/.local/share/opencode` rw for app state.
/// Only `/proc`, `/sys`, `/dev` actually pivot to host kernel resources in
/// microsandbox. This matches the plan's actual intent ("paths that
/// microsandbox may pivot to real host resources") and is documented here as
/// a justified narrowing.
pub fn validate_mount_guest(guest: &str, read_only: bool) -> Result<()> {
    if guest.is_empty() {
        anyhow::bail!("mount guest cannot be empty");
    }
    if !guest.starts_with('/') {
        anyhow::bail!("mount guest must be an absolute path inside the sandbox (got '{guest}')");
    }
    for component in Path::new(guest).components() {
        if let Component::ParentDir = component {
            anyhow::bail!(
                "mount guest contains '..' component (got '{guest}'); \
                 path traversal is not allowed"
            );
        }
    }
    const SENSITIVE_RW_PREFIXES: &[&str] = &["/proc", "/sys", "/dev"];
    if !read_only {
        for prefix in SENSITIVE_RW_PREFIXES {
            if guest == *prefix || guest.starts_with(&format!("{prefix}/")) {
                anyhow::bail!(
                    "mount guest '{guest}' cannot be mounted read-write (sensitive path); \
                     set read_only = true if you truly need it"
                );
            }
        }
    }
    Ok(())
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
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::{ensure_mount_sources, validate_mount_guest, validate_mount_host};
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
        let plan = minimal_plan(vec![MountPlan {
            host: "nested/state".into(),
            guest: "/data".into(),
            read_only: false,
        }]);

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
        let plan = minimal_plan(vec![MountPlan {
            host: "missing/config.json".into(),
            guest: "/app/config.json".into(),
            read_only: true,
        }]);

        let result = ensure_mount_sources(&root, &plan);
        assert!(
            result.is_err(),
            "readonly mount source should bail when missing, got {:?}",
            result
        );

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    // ---- A2 regression: validate_mount_host rejects hostile inputs ----

    #[test]
    fn validate_mount_host_rejects_absolute_paths() {
        let err = validate_mount_host("/etc/passwd").unwrap_err();
        assert!(err.to_string().contains("absolute path"), "got: {err}");
    }

    #[test]
    fn validate_mount_host_rejects_traversal() {
        for hostile in [
            "../../etc",
            "foo/../bar",
            "foo/..",
            "..",
            "${CWD}/../../etc",
            "${MSB_HOME}/../etc",
            "workspaces/../etc",
            "var/../../etc",
            "agents/pi/../../../etc",
        ] {
            let err = validate_mount_host(hostile).unwrap_err();
            assert!(
                err.to_string().contains(".."),
                "hostile='{hostile}' should be rejected for traversal; got: {err}"
            );
        }
    }

    #[test]
    fn validate_mount_host_rejects_empty() {
        assert!(validate_mount_host("").is_err());
    }

    #[test]
    fn validate_mount_host_accepts_legitimate_prefixes() {
        // Each of these must pass — they are the documented config patterns.
        for ok in [
            "workspaces/pi/state",
            "var/run/pi.json",
            "${MSB_HOME}/pi/config",
            "${CWD}",
            "${CWD}/config",
            "${WORKESTRATE_PI_BUILD}",
            "agents/pi/build",
            "config.reference/workestrate.toml",
            "nested/state",
        ] {
            validate_mount_host(ok)
                .unwrap_or_else(|e| panic!("legitimate host '{ok}' rejected: {e}"));
        }
    }

    // ---- C4 regression: validate_mount_guest rejects sensitive rw mounts ----

    #[test]
    fn validate_mount_guest_rejects_sensitive_rw() {
        // Only /proc, /sys, /dev are pivots to host kernel resources.
        // /etc, /root, /home are sandbox-internal rootfs dirs and are
        // legitimate mount targets (e.g. opencode mounts
        // /home/node/.local/share/opencode for app state).
        for sensitive in ["/proc", "/sys", "/dev"] {
            let err = validate_mount_guest(sensitive, false).unwrap_err();
            assert!(
                err.to_string().contains("read-write"),
                "guest='{sensitive}' rw should be rejected; got: {err}"
            );
            // Subpaths are also rejected.
            let sub = format!("{sensitive}/foo");
            let err = validate_mount_guest(&sub, false).unwrap_err();
            assert!(
                err.to_string().contains("read-write"),
                "guest='{sub}' rw should be rejected; got: {err}"
            );
        }
    }

    #[test]
    fn validate_mount_guest_allows_sensitive_readonly() {
        // Read-only mounts of sensitive paths are an explicit operator choice.
        for sensitive in ["/proc", "/sys", "/etc"] {
            validate_mount_guest(sensitive, true)
                .unwrap_or_else(|e| panic!("read-only '{sensitive}' should be allowed: {e}"));
        }
    }

    #[test]
    fn validate_mount_guest_rejects_relative_paths() {
        let err = validate_mount_guest("relative/path", false).unwrap_err();
        assert!(err.to_string().contains("absolute"), "got: {err}");
    }

    #[test]
    fn validate_mount_guest_rejects_traversal() {
        let err = validate_mount_guest("/data/../etc", false).unwrap_err();
        assert!(err.to_string().contains(".."), "got: {err}");
    }

    #[test]
    fn validate_mount_guest_accepts_normal_data_dir_rw() {
        validate_mount_guest("/data", false).unwrap();
        validate_mount_guest("/work", false).unwrap();
        validate_mount_guest("/app/config.json", false).unwrap();
    }
}
