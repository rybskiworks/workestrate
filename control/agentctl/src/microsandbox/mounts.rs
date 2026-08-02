use super::plan::SandboxPlan;
use anyhow::Result;
use microsandbox::sandbox::SandboxBuilder;
use std::path::{Component, Path, PathBuf};

/// Roots for resolving mount hosts (F1/F2 — spec 17 path resolution).
#[derive(Debug, Clone, Copy)]
pub(crate) struct MountRoots<'a> {
    /// Content root: plain repo-relative hosts resolve here — the directory
    /// of the config layer that DECLARED the workload's mounts (the parent
    /// dir of the layer file; e.g. the capsule dir in a directory-mode
    /// config repo). The caller (`build_sandbox`) falls back to the flake
    /// project root / cwd only when no declaring layer dir is knowable.
    pub content_root: &'a Path,
    /// Flake project root — present only when the workload actually requires
    /// it (F2 lazy gate). Relative BUILD-path hosts that are flake-checkout
    /// artifacts (declared `local_build.fallback` or relative env-override
    /// values, e.g. `agents/<name>/build`) resolve here, keeping the pre-F1
    /// behavior for `${WORKESTRATE_<NAME>_BUILD}`-template mounts.
    pub project_root: Option<&'a Path>,
    /// The workload's `build_path()` value when it is a flake-checkout
    /// artifact (declared fallback or env override), used to detect
    /// build-derived relative hosts AFTER template substitution (a
    /// `${WORKESTRATE_<NAME>_BUILD}` host expands to exactly this string).
    /// `None` when `build_path()` is the UNDECLARED reserved default
    /// (`.workestrate-build/<name>`, spec 21 §6.1): that default is a
    /// config-repo artifact dir resolving declaring-layer-relative (content
    /// root), so it must NOT capture the flake project-root preference.
    pub flake_build_path: Option<&'a str>,
}

fn resolve_mount_host(roots: &MountRoots, host: &str) -> Result<PathBuf> {
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
        let path = Path::new(host);
        if path.is_absolute() {
            // `${CWD}`-template hosts and absolute build paths: substitution
            // already happened in `plan()`; absolute paths pass through
            // unchanged (Path::join would do the same — explicit for clarity).
            return Ok(path.to_path_buf());
        }
        // A relative host equal to the workload's flake-checkout build path
        // came from a `${WORKESTRATE_<NAME>_BUILD}`-template expansion (or a
        // literal declared fallback like `agents/<name>/build`): a build
        // artifact of the flake checkout, so it keeps resolving against the
        // flake project root when one is available. Without a project root
        // it degrades to the content root (the F2 gate normally hard-errors
        // first in that situation). The UNDECLARED reserved default
        // (`.workestrate-build/<name>`, spec 21 §6.1) is excluded —
        // `flake_build_path` is None then — so it falls through to the
        // declaring-layer-relative content root below.
        if let Some(flake_build) = roots.flake_build_path {
            if host == flake_build {
                if let Some(project_root) = roots.project_root {
                    return Ok(project_root.join(host));
                }
            }
        }
        // Plain repo-relative host: resolve against the DECLARING config
        // layer's content dir (F1), not the flake project root.
        Ok(roots.content_root.join(host))
    }
}

/// Validate a mount host string as it appears in the raw config TOML
/// (before `${CWD}` / `${WORKESTRATE_<NAME>_BUILD}` template substitution).
///
/// Rules (closes review finding A2):
/// 1. Reject empty strings.
/// 2. Reject absolute paths (leading `/`). Config authors must use a
///    template prefix (`${CWD}/...`, `${MSB_HOME}/...`) or a relative path
///    that resolves under the declaring config layer's content directory.
/// 3. Reject any `..` path component anywhere in the string. Template
///    prefixes are not traversal escapes — `${CWD}/../../etc` is still
///    rejected because the suffix contains `..`.
///
/// The permitted raw-value forms are:
/// - `${MSB_HOME}/...`, `${CWD}/...`, `${CWD}` (exact), `${WORKESTRATE_<NAME>_BUILD}` (exact)
/// - `workspaces/...`, `var/...` (resolved to XDG state dir)
/// - any other relative path with no `..` component (resolved against the
///   declaring config layer's content directory — see [`MountRoots`])
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
/// e.g. an agent workload mounts its app-state dir under `/home` rw.
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
    roots: &MountRoots,
    plan: &SandboxPlan,
) -> Result<SandboxBuilder> {
    let mut b = builder;
    for m in &plan.mounts {
        let host = resolve_mount_host(roots, &m.host)?;
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

pub(crate) fn ensure_mount_sources(roots: &MountRoots, plan: &SandboxPlan) -> Result<()> {
    for m in &plan.mounts {
        let path = resolve_mount_host(roots, &m.host)?;
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
    use super::{ensure_mount_sources, resolve_mount_host, MountRoots};
    use super::{validate_mount_guest, validate_mount_host};
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

    fn roots_for<'a>(
        content_root: &'a std::path::Path,
        project_root: Option<&'a std::path::Path>,
        flake_build_path: Option<&'a str>,
    ) -> MountRoots<'a> {
        MountRoots {
            content_root,
            project_root,
            flake_build_path,
        }
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

        ensure_mount_sources(&roots_for(&root, None, Some("agents/test/build")), &plan)?;

        let created = root.join("nested").join("state");
        assert!(
            created.is_dir(),
            "readwrite mount source should be auto-created at {}",
            created.display()
        );

        // Idempotent: re-running over an existing directory must not error.
        ensure_mount_sources(&roots_for(&root, None, Some("agents/test/build")), &plan)?;

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

        let result =
            ensure_mount_sources(&roots_for(&root, None, Some("agents/test/build")), &plan);
        assert!(
            result.is_err(),
            "readonly mount source should bail when missing, got {:?}",
            result
        );

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    // ---- F1: repo-relative hosts resolve against the content root ----

    #[test]
    fn relative_host_resolves_against_content_root_not_project_root() -> anyhow::Result<()> {
        let content = unique_root("content");
        let project = unique_root("project");
        let roots = roots_for(&content, Some(&project), Some("agents/test/build"));
        // Capsule-style declaration: `config.yaml` lives next to the
        // declaring layer file (content root), NOT under the flake root.
        let resolved = resolve_mount_host(&roots, "config.yaml")?;
        assert_eq!(resolved, content.join("config.yaml"));
        // Even a nested repo-relative path ignores the project root.
        let resolved = resolve_mount_host(&roots, "nested/dir/file.json")?;
        assert_eq!(resolved, content.join("nested/dir/file.json"));
        Ok(())
    }

    #[test]
    fn relative_build_path_host_prefers_project_root_when_available() -> anyhow::Result<()> {
        let content = unique_root("content");
        let project = unique_root("project");
        // DECLARED fallback: `${WORKESTRATE_TEST_BUILD}` expanded to the
        // declared relative fallback `agents/test/build` — a flake-checkout
        // artifact → project root. Declared fallbacks keep the
        // pre-reservation behavior unchanged (spec 21 §6.1).
        let roots = roots_for(&content, Some(&project), Some("agents/test/build"));
        let resolved = resolve_mount_host(&roots, "agents/test/build")?;
        assert_eq!(resolved, project.join("agents/test/build"));
        Ok(())
    }

    #[test]
    fn relative_build_path_host_degrades_to_content_root_without_project_root() -> anyhow::Result<()>
    {
        let content = unique_root("content");
        let roots = roots_for(&content, None, Some("agents/test/build"));
        let resolved = resolve_mount_host(&roots, "agents/test/build")?;
        assert_eq!(resolved, content.join("agents/test/build"));
        Ok(())
    }

    /// Spec 21 §6.1: the UNDECLARED reserved default `.workestrate-build/<name>`
    /// is NOT a flake-checkout artifact (`flake_build_path` is None), so even
    /// with a flake project root available it resolves declaring-layer-relative
    /// against the content root.
    #[test]
    fn undeclared_reserved_default_resolves_against_content_root_not_project_root(
    ) -> anyhow::Result<()> {
        let content = unique_root("content");
        let project = unique_root("project");
        let roots = roots_for(&content, Some(&project), None);
        let resolved = resolve_mount_host(&roots, ".workestrate-build/test")?;
        assert_eq!(resolved, content.join(".workestrate-build/test"));
        Ok(())
    }

    #[test]
    fn absolute_and_template_hosts_pass_through_unchanged() -> anyhow::Result<()> {
        let content = unique_root("content");
        let project = unique_root("project");
        let roots = roots_for(&content, Some(&project), Some("agents/test/build"));
        // `${CWD}`-substituted hosts and absolute build paths are absolute —
        // they must NOT be re-rooted under either root.
        let resolved = resolve_mount_host(&roots, "/abs/path/from-cwd-template")?;
        assert_eq!(
            resolved,
            std::path::PathBuf::from("/abs/path/from-cwd-template")
        );
        let roots = roots_for(&content, Some(&project), Some("/nix/store/abc-build"));
        let resolved = resolve_mount_host(&roots, "/nix/store/abc-build")?;
        assert_eq!(resolved, std::path::PathBuf::from("/nix/store/abc-build"));
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
