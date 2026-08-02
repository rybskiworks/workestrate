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

pub(crate) fn resolve_mount_host(roots: &MountRoots, host: &str) -> Result<PathBuf> {
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

/// Owned companion to [`MountRoots`] (which borrows). Resolved once, then
/// [`MountRoots`] borrows from it for the lifetime of a build/plan/preflight.
/// Extracted from `build_sandbox` so the plan-time existence preflight reuses
/// the SAME root-resolution logic as the runtime build path (F1/F2).
pub(crate) struct MountRootsOwned {
    pub content_root: PathBuf,
    pub project_root: Option<PathBuf>,
    pub flake_build_path: Option<String>,
}

impl MountRootsOwned {
    /// Borrow as a [`MountRoots`] for `resolve_mount_host` /
    /// `ensure_mount_sources` / [`preflight_existence`].
    pub fn as_roots(&self) -> MountRoots<'_> {
        MountRoots {
            content_root: &self.content_root,
            project_root: self.project_root.as_deref(),
            flake_build_path: self.flake_build_path.as_deref(),
        }
    }
}

/// Resolve the mount roots for a workload, mirroring `build_sandbox`'s logic
/// exactly (F2 lazy flake-root gate + F1 content-root fallback). Shared by the
/// runtime build path and the plan-time existence preflight so they agree on
/// where a mount host resolves.
pub(crate) fn resolve_mount_roots_owned<W: crate::microsandbox::workload::Workload + ?Sized>(
    workload: &W,
    plan: &SandboxPlan,
) -> Result<MountRootsOwned> {
    // F2 LAZY GATE: `project_root()` hard-errors when the resolved root lacks
    // flake.nix, so it is called ONLY when the workload genuinely needs the
    // flake checkout. Registry-image workloads with no local_build and no
    // relative build-path mounts never touch the gate.
    let project_root: Option<PathBuf> = match workload.flake_root_requirement(plan) {
        Some(feature) => Some(crate::config::project_root().map_err(|e| {
            anyhow::anyhow!(
                "workload '{}' uses {}, which requires a flake project root: {}",
                workload.name(),
                feature,
                e
            )
        })?),
        None => crate::config::project_root_optional(),
    };

    // F1: repo-relative mount hosts resolve against the DECLARING config
    // layer's content root (spec 17). EXPLICIT FALLBACK: when no declaring
    // layer dir is knowable (synthetic layers), fall back to the flake project
    // root when one resolved, else the process cwd.
    let content_root: PathBuf = workload
        .mount_content_root()
        .or_else(|| project_root.clone())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let build_path = workload.build_path();
    // Spec 21 §6.1: the UNDECLARED reserved default (`.workestrate-build/<name>`)
    // resolves declaring-layer-relative and must NOT capture the flake
    // project-root preference. Declared fallbacks and env overrides keep the
    // pre-reservation flake-checkout preference.
    let flake_build_path = if workload.build_path_is_reserved_default() {
        None
    } else {
        Some(build_path)
    };

    Ok(MountRootsOwned {
        content_root,
        project_root,
        flake_build_path,
    })
}

/// Plan-time existence preflight (security-model enforcement point; fulfills
/// docs/migration/30-security-model.md:137-141's plan-time promise).
///
/// `hard = true` (the `plan` command): a missing read-only mount source or a
/// missing seed source BAILS with a clear error — this is the exact failure-1
/// signal (a doubled or wrong-root path that points nowhere), surfaced BEFORE
/// any KVM/runtime work. `hard = false` (`validate-config`): everything is
/// collected as a warning, because synthetic/reference configs may legitimately
/// lack the referenced files.
///
/// Read-write mounts are ALWAYS warnings (auto-created at runtime by
/// [`ensure_mount_sources`]); a `local_build.fallback` is ALWAYS a warning (a
/// build output that may not exist yet at plan time).
///
/// Returns the list of warning strings (empty when everything resolves).
pub(crate) fn preflight_existence(
    roots: &MountRoots,
    plan: &SandboxPlan,
    seed_files: &[crate::config::SeedFileConfig],
    seed_content_root: Option<&Path>,
    local_build_fallback: Option<&str>,
    workload_name: &str,
    hard: bool,
) -> Result<Vec<String>> {
    let mut warnings = Vec::new();

    // Mounts: resolve each host the same way `apply_plan_mounts` does, then
    // check existence. RO missing → hard error (plan) / warning (validate);
    // RW missing → always a warning (auto-created at runtime).
    for m in &plan.mounts {
        let path = resolve_mount_host(roots, &m.host)?;
        if !path.exists() {
            if m.read_only {
                let msg = format!(
                    "workload '{}': read-only mount source does not exist: {} (host = {:?})",
                    workload_name,
                    path.display(),
                    m.host
                );
                if hard {
                    anyhow::bail!("{msg}");
                } else {
                    warnings.push(msg);
                }
            } else {
                warnings.push(format!(
                    "workload '{}': read-write mount source does not exist (will be created at runtime): {}",
                    workload_name,
                    path.display()
                ));
            }
        }
    }

    // Seed sources: resolve against the seed content root (the declaring
    // layer's content root), falling back to the mount content root / project
    // root — mirroring `ConfigWorkload::prepare`'s fallback chain. A missing
    // seed source is a hard error at plan time (prepare() would fail to copy
    // it at runtime); warn-only under validate-config.
    let seed_root = seed_content_root
        .or(Some(roots.content_root))
        .or(roots.project_root);
    for seed in seed_files {
        match &seed_root {
            Some(root) => {
                let src = root.join(&seed.source);
                if !src.exists() {
                    let msg = format!(
                        "workload '{}': seed source does not exist: {} (source = {:?})",
                        workload_name,
                        src.display(),
                        seed.source
                    );
                    if hard {
                        anyhow::bail!("{msg}");
                    } else {
                        warnings.push(msg);
                    }
                }
            }
            None => {
                let msg = format!(
                    "workload '{}': seed source {:?} cannot be resolved (no content root)",
                    workload_name, seed.source
                );
                if hard {
                    anyhow::bail!("{msg}");
                } else {
                    warnings.push(msg);
                }
            }
        }
    }

    // local_build fallback: a build output dir. Warn if missing (it may be
    // built later). Resolve a relative fallback against the project root when
    // present (flake-checkout artifact), else the content root; absolute
    // fallbacks are checked verbatim.
    if let Some(fallback) = local_build_fallback {
        let resolved = if std::path::Path::new(fallback).is_absolute() {
            Some(PathBuf::from(fallback))
        } else {
            roots
                .project_root
                .or(Some(roots.content_root))
                .map(|root| root.join(fallback))
        };
        if let Some(p) = resolved {
            if !p.exists() {
                warnings.push(format!(
                    "workload '{}': local_build fallback does not exist (will be built): {}",
                    workload_name,
                    p.display()
                ));
            }
        }
    }

    Ok(warnings)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::{ensure_mount_sources, preflight_existence, resolve_mount_host, MountRoots};
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
            policy: None,
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
            policy: None,
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

    // ---- FIX 1 regression: directory-mode content root is the workestrate/
    // root, NOT the capsule/workloads dir — mount hosts written relative to
    // the workestrate root must NOT double. ----

    /// Unique tempdir for directory-mode fixtures (short name keeps it off the
    /// 108-byte unix-socket budget if a KVM test ever reuses this helper).
    fn dirmode_root(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "wk-dirmode-{}-{}-{}",
            label,
            std::process::id(),
            nanos,
        ))
    }

    /// End-to-end through the real directory-mode loader: a capsule workload
    /// declaring `host = "workloads/litellm"` (a path relative to the
    /// workestrate root) must resolve to `<tmp>/workestrate/workloads/litellm`
    /// — NOT doubled to `<tmp>/workestrate/workloads/litellm/workloads/litellm`
    /// (host-boot failure 1). The content root derived by `layer_dirs_from`
    /// must be the directory-mode root `<tmp>/workestrate/`.
    #[test]
    fn directory_mode_content_root_is_workestrate_root_not_capsule() -> anyhow::Result<()> {
        let tmp = dirmode_root("capsule");
        let wks = tmp.join("workestrate");
        let capsule = wks.join("workloads").join("litellm");
        std::fs::create_dir_all(&capsule)?;
        std::fs::write(wks.join("default.toml"), "schema_version = 1\n")?;
        std::fs::write(
            capsule.join("workload.toml"),
            "kind = \"service\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[[mounts]]\nhost = \"workloads/litellm\"\nguest = \"/app/config\"\nread_only = true\n",
        )?;
        // The real artifact the mount points at — must exist for any future
        // existence preflight, and proves the resolved path is real.
        std::fs::write(capsule.join("config.yaml"), "litellm: {}\n")?;

        let layers = crate::config::loading::load_config_repo_layers("testrepo", &tmp)?;
        let dirs = crate::merge::layer_dirs_from(&layers);
        let (_merged, provenance) = crate::merge::merge_layers(&layers)?;

        let layer_name = provenance
            .get("workloads.litellm.mounts")
            .expect("mounts provenance recorded");
        let content_root = dirs
            .get(layer_name)
            .expect("content root recorded for the mounts-declaring layer");
        // Directory-mode root, NOT the capsule dir.
        assert_eq!(
            content_root, &wks,
            "content root must be the directory-mode root <tmp>/workestrate/, not the capsule dir"
        );

        let roots = roots_for(content_root, None, None);
        let resolved = resolve_mount_host(&roots, "workloads/litellm")?;
        assert_eq!(
            resolved,
            wks.join("workloads").join("litellm"),
            "a workestrate-root-relative mount host must NOT double"
        );
        // And the resolved path actually exists (the real config.yaml lives
        // there — the doubling bug pointed at a non-existent path).
        assert!(
            resolved.join("config.yaml").is_file(),
            "resolved mount source must point at the real capsule dir"
        );

        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    /// Flat-file directory-mode entry: `workloads/<name>.toml`. The content
    /// root must be `<repo>/workestrate/` (NOT `workloads/`), so a
    /// `host = "workloads/svc"` mount resolves without doubling. Uses pure
    /// structural layer construction (no FS) — the loader would otherwise
    /// treat a `workloads/svc/` artifact dir as a capsule entry.
    #[test]
    fn directory_mode_flat_file_content_root_is_workestrate_root() -> anyhow::Result<()> {
        let wks = std::path::Path::new("/tmp/wk-flat-test/workestrate");
        let flat_file = wks.join("workloads").join("svc.toml");
        let layer = crate::merge::Layer::from_string_with_path(
            "testrepo#workestrate/workloads/svc.toml",
            "schema_version = 1\n\n[workloads.svc]\nkind = \"service\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[[workloads.svc.mounts]]\nhost = \"workloads/svc\"\nguest = \"/app\"\nread_only = true\n",
            Some(flat_file),
        )?;
        let dirs = crate::merge::layer_dirs_from(std::slice::from_ref(&layer));
        let (_merged, provenance) = crate::merge::merge_layers(std::slice::from_ref(&layer))?;

        let layer_name = provenance
            .get("workloads.svc.mounts")
            .expect("mounts provenance recorded");
        let content_root = dirs
            .get(layer_name)
            .expect("content root recorded for the flat workload layer");
        assert_eq!(
            content_root,
            wks,
            "flat-file content root must be the directory-mode root <repo>/workestrate/, not workloads/"
        );

        let roots = roots_for(content_root, None, None);
        let resolved = resolve_mount_host(&roots, "workloads/svc")?;
        assert_eq!(
            resolved,
            wks.join("workloads").join("svc"),
            "flat-file workestrate-root-relative mount must NOT double"
        );
        Ok(())
    }

    /// A `local_build.fallback` string (e.g. `agents/svc/build`) resolves
    /// against the directory-mode content root when no flake project root is
    /// available (the UNDECLARED reserved default path; a declared fallback
    /// with a project root would prefer it). Proves the content root — not
    /// the capsule dir — is the base for relative build-path hosts.
    #[test]
    fn directory_mode_local_build_fallback_resolves_against_content_root() -> anyhow::Result<()> {
        let tmp = dirmode_root("build");
        let wks = tmp.join("workestrate");
        let capsule = wks.join("workloads").join("svc");
        std::fs::create_dir_all(&capsule)?;
        std::fs::write(wks.join("default.toml"), "schema_version = 1\n")?;
        std::fs::write(
            capsule.join("workload.toml"),
            "kind = \"service\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[local_build]\nrecipe = \"pip-install\"\nsource = \"flake://svc\"\nfallback = \"agents/svc/build\"\n",
        )?;

        let layers = crate::config::loading::load_config_repo_layers("testrepo", &tmp)?;
        let dirs = crate::merge::layer_dirs_from(&layers);
        let (_merged, provenance) = crate::merge::merge_layers(&layers)?;

        let layer_name = provenance
            .get("workloads.svc.local_build")
            .expect("local_build provenance recorded");
        let content_root = dirs
            .get(layer_name)
            .expect("content root recorded for the local_build-declaring layer");
        assert_eq!(
            content_root, &wks,
            "local_build content root must be the directory-mode root"
        );

        // flake_build_path = None (no project root, reserved-default posture):
        // a relative fallback resolves against the content root, NOT the
        // capsule dir.
        let roots = roots_for(content_root, None, None);
        let resolved = resolve_mount_host(&roots, "agents/svc/build")?;
        assert_eq!(
            resolved,
            wks.join("agents").join("svc").join("build"),
            "relative local_build fallback resolves against the directory-mode root, not the capsule dir"
        );

        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    // ---- FIX 3: plan-time existence preflight ----

    /// A missing read-only mount source BAILS in hard mode (the failure-1
    /// signal: a doubled/wrong-root path that points nowhere).
    #[test]
    fn preflight_missing_readonly_mount_fails_hard() -> anyhow::Result<()> {
        let root = unique_root("pf-ro");
        let plan = minimal_plan(vec![MountPlan {
            host: "missing/config.json".into(),
            guest: "/app/config.json".into(),
            read_only: true,
        }]);
        let roots = roots_for(&root, None, None);
        let err = preflight_existence(&roots, &plan, &[], None, None, "svc", true).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("read-only mount source does not exist"),
            "hard preflight must bail on missing RO mount: {msg}"
        );
        assert!(msg.contains("svc"), "error must name the workload: {msg}");
        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    /// A missing read-write mount source WARNS (not fails) even in hard mode —
    /// it is auto-created at runtime by `ensure_mount_sources`.
    #[test]
    fn preflight_missing_readwrite_mount_warns_not_fails() -> anyhow::Result<()> {
        let root = unique_root("pf-rw");
        let plan = minimal_plan(vec![MountPlan {
            host: "nested/state".into(),
            guest: "/data".into(),
            read_only: false,
        }]);
        let roots = roots_for(&root, None, None);
        let warnings = preflight_existence(&roots, &plan, &[], None, None, "svc", true)?;
        assert_eq!(warnings.len(), 1, "exactly one RW warning: {warnings:?}");
        assert!(
            warnings[0].contains("will be created at runtime"),
            "RW warning must say it will be created: {}",
            warnings[0]
        );
        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    /// A missing seed source BAILS in hard mode (prepare() would fail to copy
    /// it at runtime).
    #[test]
    fn preflight_missing_seed_source_fails_hard() -> anyhow::Result<()> {
        let root = unique_root("pf-seed");
        let plan = minimal_plan(vec![]);
        let seeds = vec![crate::config::SeedFileConfig {
            source: "seed/missing.json".into(),
            target: "workspaces/svc-state/missing.json".into(),
            only_if_missing: None,
        }];
        let roots = roots_for(&root, None, None);
        let err =
            preflight_existence(&roots, &plan, &seeds, Some(&root), None, "svc", true).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("seed source does not exist"),
            "hard preflight must bail on missing seed source: {msg}"
        );
        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    /// A missing `local_build.fallback` WARNS (not fails) — it is a build
    /// output that may not exist yet at plan time.
    #[test]
    fn preflight_missing_local_build_fallback_warns() -> anyhow::Result<()> {
        let root = unique_root("pf-build");
        let plan = minimal_plan(vec![]);
        let roots = roots_for(&root, None, None);
        let warnings = preflight_existence(
            &roots,
            &plan,
            &[],
            None,
            Some("agents/svc/build"),
            "svc",
            true,
        )?;
        assert_eq!(
            warnings.len(),
            1,
            "exactly one fallback warning: {warnings:?}"
        );
        assert!(
            warnings[0].contains("local_build fallback does not exist"),
            "fallback warning text: {}",
            warnings[0]
        );
        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    /// In warn mode (validate-config), a missing RO mount is collected as a
    /// warning, NOT a bail.
    #[test]
    fn preflight_warn_mode_collects_missing_ro_mount() -> anyhow::Result<()> {
        let root = unique_root("pf-warn");
        let plan = minimal_plan(vec![MountPlan {
            host: "missing/cfg.yaml".into(),
            guest: "/app/cfg.yaml".into(),
            read_only: true,
        }]);
        let roots = roots_for(&root, None, None);
        let warnings = preflight_existence(&roots, &plan, &[], None, None, "svc", false)?;
        assert_eq!(warnings.len(), 1, "warn mode collects the missing RO mount");
        assert!(
            warnings[0].contains("read-only mount source does not exist"),
            "warn-mode warning text: {}",
            warnings[0]
        );
        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    /// An existing RO mount + existing seed source → no warnings, no bail.
    #[test]
    fn preflight_existing_sources_yield_no_warnings() -> anyhow::Result<()> {
        let root = unique_root("pf-ok");
        std::fs::create_dir_all(root.join("cfg"))?;
        std::fs::write(root.join("cfg").join("app.yaml"), "ok")?;
        std::fs::create_dir_all(root.join("seed"))?;
        std::fs::write(root.join("seed").join("s.json"), "{}")?;
        let plan = minimal_plan(vec![MountPlan {
            host: "cfg/app.yaml".into(),
            guest: "/app/app.yaml".into(),
            read_only: true,
        }]);
        let seeds = vec![crate::config::SeedFileConfig {
            source: "seed/s.json".into(),
            target: "workspaces/svc-state/s.json".into(),
            only_if_missing: None,
        }];
        let roots = roots_for(&root, None, None);
        let warnings = preflight_existence(&roots, &plan, &seeds, Some(&root), None, "svc", true)?;
        assert!(warnings.is_empty(), "no warnings expected: {warnings:?}");
        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }
}
