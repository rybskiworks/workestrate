use anyhow::Result;

/// Validate a `local_build.env_override` value as it appears in config.
/// Closes review finding C2: previously, a config layer could set
/// `env_override = "HOME"`, causing `build_path()` to read `$HOME` and (if a
/// mount host used the conventional `${WORKESTRATE_<NAME>_BUILD}` template)
/// bind-mount the operator's home directory into the sandbox.
///
/// Rules:
/// 1. Must match `^[A-Z_][A-Z0-9_]*$` (POSIX-ish env-var name).
/// 2. Must not be a process-global name that would hijack a meaningful
///    variable: HOME, USER, PATH, SHELL, PWD, SOPS_AGE_KEY_FILE,
///    WORKESTRATE_*, AGENTCTL_ROOT, XDG_*.
pub fn validate_env_override(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("local_build.env_override cannot be empty");
    }
    let mut chars = name.chars();
    let first_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_uppercase() || c == '_');
    if !first_ok {
        anyhow::bail!(
            "local_build.env_override='{name}' must match ^[A-Z_][A-Z0-9_]*$              (must start with uppercase letter or underscore)"
        );
    }
    for c in chars {
        if !c.is_ascii_uppercase() && !c.is_ascii_digit() && c != '_' {
            anyhow::bail!(
                "local_build.env_override='{name}' must match ^[A-Z_][A-Z0-9_]*$                  (invalid character '{c}')"
            );
        }
    }
    const DENYLIST: &[&str] = &[
        "HOME",
        "USER",
        "PATH",
        "SHELL",
        "PWD",
        "SOPS_AGE_KEY_FILE",
        "WORKESTRATE_FLEET_DIR",
        "WORKESTRATE_NO_PROJECT_CONFIG",
        "WORKESTRATE_FLEET",
        "WORKESTRATE_INVOKE_CWD",
        "AGENTCTL_ROOT",
    ];
    if DENYLIST.contains(&name) {
        anyhow::bail!(
            "local_build.env_override='{name}' is denied (process-global name);              choose a workload-specific name such as WORKESTRATE_MYWORKLOAD_BUILD"
        );
    }
    if name.starts_with("XDG_") {
        anyhow::bail!(
            "local_build.env_override='{name}' is denied (XDG_* process-global name);              choose a workload-specific name such as WORKESTRATE_MYWORKLOAD_BUILD"
        );
    }
    Ok(())
}

/// Validate a `seed_files.source` value. Closes review finding C3:
/// `root.join(&seed.source)` had no traversal check, so `source = "../../etc/passwd"`
/// would copy host files into sandbox state.
///
/// Rules:
/// 1. Reject empty.
/// 2. Reject absolute paths (leading `/`). Seed sources resolve relative to
///    the declaring config layer's content directory (spec 17).
/// 3. Reject any `..` component.
///
/// Note: seed sources do not participate in mount-template substitution, so
/// template prefixes like `${CWD}` are not expanded here — they would be
/// treated as literal directory names. Reject anything that is not a plain
/// relative path.
pub fn validate_seed_source(src: &str) -> Result<()> {
    use std::path::{Component, Path};
    if src.is_empty() {
        anyhow::bail!("seed_files.source cannot be empty");
    }
    if src.starts_with('/') {
        anyhow::bail!(
            "seed_files.source cannot be an absolute path (got '{src}');              seed sources must be relative to the declaring config layer's directory"
        );
    }
    for component in Path::new(src).components() {
        if let Component::ParentDir = component {
            anyhow::bail!(
                "seed_files.source contains '..' component (got '{src}');                  path traversal is not allowed"
            );
        }
    }
    // Reject template-looking prefixes — they are not expanded for seed sources
    // and would silently create a literal directory named ${CWD}.
    if src.starts_with("${") {
        anyhow::bail!(
            "seed_files.source='{src}' looks like a template token, but seed sources              are not template-substituted; use a path relative to the declaring config layer"
        );
    }
    Ok(())
}

/// Validate a `seed_files.glob` pattern (P0). The pattern is resolved
/// relative to the declaring config layer's content directory, so it must be
/// a plain relative pattern that can never escape that root and is never
/// template-substituted.
///
/// Rules:
/// 1. Reject empty.
/// 2. Reject absolute patterns (leading `/`). Glob matches resolve relative
///    to the declaring config layer's content directory (spec 17).
/// 3. Reject any `..` component (component-based only — brace forms like
///    `{a,b}` are fine unless a literal `..` component appears).
/// 4. Reject `${` anywhere — glob patterns are NOT template-substituted (a
///    `${VAR}`-looking pattern would otherwise silently match a literal
///    directory named `${VAR}`).
/// 5. Must compile as a `glob::Pattern`.
pub fn validate_seed_glob(pattern: &str) -> Result<()> {
    use std::path::{Component, Path};
    if pattern.is_empty() {
        anyhow::bail!("seed_files.glob cannot be empty");
    }
    if pattern.starts_with('/') {
        anyhow::bail!(
            "seed_files.glob cannot be an absolute path (got '{pattern}');              glob patterns must be relative to the declaring config layer's directory"
        );
    }
    for component in Path::new(pattern).components() {
        if let Component::ParentDir = component {
            anyhow::bail!(
                "seed_files.glob contains '..' component (got '{pattern}');                  path traversal is not allowed"
            );
        }
    }
    if pattern.contains("${") {
        anyhow::bail!(
            "seed_files.glob='{pattern}' looks like a template token, but glob patterns              are not template-substituted; use a pattern relative to the declaring config layer"
        );
    }
    glob::Pattern::new(pattern).map_err(|e| {
        anyhow::anyhow!("seed_files.glob='{pattern}' is not a valid glob pattern: {e}")
    })?;
    Ok(())
}

/// Validate a `seed_files.target` value. The target is the HOST-side render
/// destination for a seeded file (guest-visible only through a declared
/// mount); it must be a plain relative path so seeding can never write
/// outside the declaring layer's content root or the state dir.
///
/// Rules:
/// 1. Reject empty.
/// 2. Reject absolute paths (leading `/`).
/// 3. Reject any `..` component.
///
/// Note: path-shape safety is necessary but NOT sufficient — the target must
/// also be covered by a declared mount host or the seed silently renders on
/// the host where the guest never sees it. See
/// [`validate_seed_target_coverage`].
pub fn validate_seed_target(target: &str) -> Result<()> {
    use std::path::{Component, Path};
    if target.is_empty() {
        anyhow::bail!("seed_files.target cannot be empty");
    }
    if target.starts_with('/') {
        anyhow::bail!(
            "seed_files.target cannot be an absolute path (got '{target}');              seed targets must be relative to the sandbox root"
        );
    }
    for component in Path::new(target).components() {
        if let Component::ParentDir = component {
            anyhow::bail!(
                "seed_files.target contains '..' component (got '{target}');                  path traversal is not allowed"
            );
        }
    }
    Ok(())
}

/// Validate that a `seed_files.target` is COVERED by at least one declared
/// mount host (fail-closed; closes host "Bug C").
///
/// Seeds are mount-backed BY DESIGN: `prepare()` renders the seed on the
/// HOST — targets starting with `workspaces/` or `var/` resolve under the
/// XDG state dir, anything else resolves under the declaring config layer's
/// content root — and the file reaches the GUEST only when that host path is
/// covered by a declared `[[mounts]]` entry. Without this check a seed
/// targeting a non-mounted path silently renders into the host working tree
/// and the guest never sees it (the litellm `app/config/config.yaml` bug).
///
/// The target is COVERED iff at least one mount host:
/// 1. is in the same resolution class as the target: state class = starts
///    with `workspaces/` or `var/`; content class = anything else. Classes
///    must match (a content-class mount can never cover a state-class
///    target and vice versa);
/// 2. contains no `${` — template hosts (`${CWD}`, `${MSB_HOME}`,
///    `${WORKESTRATE_*_BUILD}`, ...) are runtime-resolved and can NEVER
///    prove static coverage, so they are skipped;
/// 3. is non-empty, not absolute, and has no `..` component (defensive;
///    separately validated);
/// 4. is a COMPONENT-WISE prefix of the target (equal counts as covered).
///    Component-wise, NOT string-prefix: `workspaces/svc-state-evil` must
///    NOT cover `workspaces/svc-state/x.json`.
pub fn validate_seed_target_coverage(target: &str, mount_hosts: &[&str]) -> Result<()> {
    use std::path::{Component, Path};

    let target_state_class = target.starts_with("workspaces/") || target.starts_with("var/");
    let target_components: Vec<_> = Path::new(target).components().collect();

    let covered = mount_hosts.iter().any(|host| {
        // Template hosts are runtime-resolved: static coverage unprovable.
        if host.contains("${") {
            return false;
        }
        let host_path = Path::new(host);
        if host.is_empty() || host_path.is_absolute() {
            return false;
        }
        let host_components: Vec<_> = host_path.components().collect();
        if host_components
            .iter()
            .any(|c| matches!(c, Component::ParentDir))
        {
            return false;
        }
        // The mount host and the seed target must resolve under the SAME
        // base (state dir vs declaring-layer content root).
        let host_state_class = host.starts_with("workspaces/") || host.starts_with("var/");
        if host_state_class != target_state_class {
            return false;
        }
        host_components.len() <= target_components.len()
            && host_components
                .iter()
                .zip(target_components.iter())
                .all(|(h, t)| h == t)
    });

    if covered {
        return Ok(());
    }
    anyhow::bail!(
        "seed_files.target '{target}' is not covered by any declared mount host: \
         seed targets are rendered on the HOST (state-dir-relative for \
         workspaces//var/ targets, otherwise declaring-layer-content-root-relative) \
         and reach the guest ONLY through a declared mount whose host path is a \
         component-wise prefix of the target; fix: declare a mount with \
         host = \"workspaces/<name>-state\" (or a content-root-relative dir that \
         is a prefix of the target) and keep the target under it"
    )
}
/// Resolve mount-host template tokens (WP6(c)/A6).
///
/// Matches, in precedence order:
/// 1. `${<env_override>}` — the workload's configured
///    `local_build.env_override` var name (checked FIRST so a custom override
///    that happens to equal the default convention still wins).
/// 2. `${WORKESTRATE_<NAME>_BUILD}` — the default convention (NAME
///    uppercased, '-' → '_'), kept for backward compatibility.
/// 3. `${CWD}` / `${CWD}/...` — the operator's INVOCATION working directory
///    (captured once at CLI entry into `WORKESTRATE_INVOKE_CWD`; falls back
///    to the process cwd when uncaptured). Never re-read lazily: a child
///    process whose cwd differs from the operator's must still mount the
///    ORIGINAL invocation dir (the wrong-CWD mount bug).
///
/// Anything else is returned unchanged (literal).
pub(super) fn resolve_mount_host_template(
    host: &str,
    name: &str,
    build_path: &str,
    env_override: Option<&str>,
) -> String {
    if let Some(custom) = env_override {
        let custom_template = format!("${{{custom}}}");
        if host == custom_template {
            return build_path.to_string();
        }
    }
    let default_build_var = format!(
        "${{WORKESTRATE_{}_BUILD}}",
        name.to_ascii_uppercase().replace('-', "_")
    );
    if host == default_build_var {
        return build_path.to_string();
    }
    if host == "${CWD}"
        && let Some(cwd) = crate::config::invoke_cwd()
    {
        return cwd.to_string_lossy().into_owned();
    }
    if let Some(rest) = host.strip_prefix("${CWD}/")
        && let Some(cwd) = crate::config::invoke_cwd()
    {
        return format!("{}/{}", cwd.to_string_lossy(), rest);
    }
    host.to_string()
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    // ---- C2 regression: validate_env_override rejects dangerous names ----

    #[test]
    fn env_override_rejects_process_global_names() {
        for hostile in [
            "HOME",
            "USER",
            "PATH",
            "SHELL",
            "PWD",
            "SOPS_AGE_KEY_FILE",
            "WORKESTRATE_FLEET_DIR",
            "WORKESTRATE_NO_PROJECT_CONFIG",
            "WORKESTRATE_FLEET",
            "WORKESTRATE_INVOKE_CWD",
            "AGENTCTL_ROOT",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
        ] {
            let err = validate_env_override(hostile).unwrap_err();
            assert!(
                err.to_string().contains("denied"),
                "env_override='{hostile}' should be denied; got: {err}"
            );
        }
    }

    #[test]
    fn env_override_rejects_lowercase_and_invalid_chars() {
        // All entries must FAIL the regex `^[A-Z_][A-Z0-9_]*$` OR be empty.
        // (Syntactically valid names that are not in the denylist, e.g.
        // `BUILD_PATH`, are tested in the accept-list test.)
        for hostile in ["home", "Home", "FOO-BAR", "1FOO", "FOO BAR", ""] {
            assert!(
                validate_env_override(hostile).is_err(),
                "env_override='{hostile}' should be rejected"
            );
        }
    }

    #[test]
    fn env_override_accepts_legitimate_workload_specific_names() {
        for ok in [
            "WORKESTRATE_PI_BUILD",
            "PI_BUILD_DIR",
            "MY_WORKLOAD_PATH",
            "_FOO",
            "A",
            "ABC_123",
        ] {
            validate_env_override(ok)
                .unwrap_or_else(|e| panic!("legitimate env_override='{ok}' rejected: {e}"));
        }
    }

    // ---- C3 regression: validate_seed_source rejects traversal ----

    #[test]
    fn seed_source_rejects_traversal_and_absolute() {
        for hostile in [
            "../../etc/passwd",
            "foo/../bar",
            "..",
            "/etc/passwd",
            "${CWD}/secret",
            "${MSB_HOME}/secret",
        ] {
            let err = validate_seed_source(hostile).unwrap_err();
            assert!(
                err.to_string().contains("..")
                    || err.to_string().contains("absolute")
                    || err.to_string().contains("template"),
                "seed_source='{hostile}' should be rejected; got: {err}"
            );
        }
    }

    #[test]
    fn seed_source_accepts_relative_paths() {
        for ok in [
            "agents/pi/config.json",
            "config.reference/workestrate.toml",
            "seed/db.sql",
        ] {
            validate_seed_source(ok)
                .unwrap_or_else(|e| panic!("legitimate seed_source='{ok}' rejected: {e}"));
        }
    }

    // ---- P0: validate_seed_glob / validate_seed_target ----

    /// Each hostile pattern must be rejected with the message naming the
    /// specific rule that fired (empty / absolute / '..' / template /
    /// pattern).
    #[test]
    fn validate_seed_glob_rejects_bad_patterns() {
        for (hostile, keyword) in [
            ("", "empty"),
            ("/abs/*.json", "absolute"),
            ("../x/**", "'..'"),
            ("a/../b/*.json", "'..'"),
            ("${CWD}/**", "template"),
            ("seed/[unclosed", "pattern"),
        ] {
            let err = validate_seed_glob(hostile).unwrap_err();
            assert!(
                err.to_string().contains(keyword),
                "seed_glob='{hostile}' should be rejected with '{keyword}'; got: {err}"
            );
        }
    }

    #[test]
    fn validate_seed_glob_accepts_relative_patterns() {
        for ok in [
            "agents/**/*.json",
            "seed/*.env",
            "conf/{a,b}.toml",
            "a/b?/c*.txt",
        ] {
            validate_seed_glob(ok)
                .unwrap_or_else(|e| panic!("legitimate seed_glob='{ok}' rejected: {e}"));
        }
    }

    #[test]
    fn validate_seed_target_rejects_absolute_and_traversal() {
        for hostile in ["/etc/x", "a/../b", ".."] {
            assert!(
                validate_seed_target(hostile).is_err(),
                "seed_target='{hostile}' should be rejected"
            );
        }
    }

    #[test]
    fn validate_seed_target_accepts_relative() {
        for ok in [
            "workspaces/pi-state/agent/x.json",
            "var/log/f.log",
            "agents/pi/config.json",
        ] {
            validate_seed_target(ok)
                .unwrap_or_else(|e| panic!("legitimate seed_target='{ok}' rejected: {e}"));
        }
    }

    // ---- WP6(c)/A6: build_path template honors custom env_override ----

    #[test]
    fn mount_template_matches_custom_env_override() {
        // ${CUSTOM_BUILD} with env_override = CUSTOM_BUILD → build_path.
        let got = resolve_mount_host_template(
            "${CUSTOM_BUILD}",
            "pi",
            "/tmp/build-out",
            Some("CUSTOM_BUILD"),
        );
        assert_eq!(got, "/tmp/build-out");
    }

    #[test]
    fn mount_template_still_matches_default_convention_with_override_set() {
        // Backward compat: ${WORKESTRATE_PI_BUILD} still resolves even when a
        // custom env_override is configured.
        let got = resolve_mount_host_template(
            "${WORKESTRATE_PI_BUILD}",
            "pi",
            "/tmp/build-out",
            Some("CUSTOM_BUILD"),
        );
        assert_eq!(got, "/tmp/build-out");
    }

    #[test]
    fn mount_template_custom_token_literal_without_override() {
        // Without a configured override, ${CUSTOM_BUILD} is NOT substituted.
        let got = resolve_mount_host_template("${CUSTOM_BUILD}", "pi", "/tmp/build-out", None);
        assert_eq!(got, "${CUSTOM_BUILD}");
    }

    #[test]
    fn mount_template_cwd_unchanged() {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(&[crate::config::INVOKE_CWD_ENV]);
        // Pin the fallback path: with no captured invocation cwd, the
        // process cwd wins (pre-capture behavior).
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var(crate::config::INVOKE_CWD_ENV) };

        let got = resolve_mount_host_template("${CWD}", "pi", "/tmp/build-out", None);
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(got, cwd.to_string_lossy());
        let got =
            resolve_mount_host_template("${CWD}/sub", "pi", "/tmp/build-out", Some("X_BUILD"));
        assert_eq!(got, format!("{}/sub", cwd.to_string_lossy()));
    }

    // ---- Wrong-CWD `${CWD}` mount fix: the captured invocation cwd wins ----

    /// With WORKESTRATE_INVOKE_CWD captured at CLI entry (dir A) and the
    /// process cwd elsewhere (dir B — the re-exec'd / detached child case),
    /// `${CWD}` resolves to A, never to the child's own cwd.
    #[test]
    fn mount_template_cwd_prefers_invoke_cwd_capture() {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(&[crate::config::INVOKE_CWD_ENV]);

        let invoke = crate::config::test_support::uniq_dir("invoke-cwd-a");
        let elsewhere = crate::config::test_support::uniq_dir("invoke-cwd-b");
        std::fs::create_dir_all(&invoke).unwrap();
        std::fs::create_dir_all(&elsewhere).unwrap();
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var(crate::config::INVOKE_CWD_ENV, &invoke) };
        std::env::set_current_dir(&elsewhere).unwrap();

        let got = resolve_mount_host_template("${CWD}", "pi", "/tmp/build-out", None);
        assert_eq!(
            got,
            invoke.to_string_lossy(),
            "${{CWD}} must resolve to the captured invocation cwd, not the process cwd"
        );
        let got = resolve_mount_host_template("${CWD}/sub", "pi", "/tmp/build-out", None);
        assert_eq!(got, format!("{}/sub", invoke.to_string_lossy()));

        let _ = std::fs::remove_dir_all(&invoke);
        let _ = std::fs::remove_dir_all(&elsewhere);
    }

    #[test]
    fn mount_template_default_convention_without_override() {
        let got = resolve_mount_host_template("${WORKESTRATE_PI_BUILD}", "pi", "/tmp/b", None);
        assert_eq!(got, "/tmp/b");
        // Hyphenated workload names map '-' → '_'.
        let got = resolve_mount_host_template(
            "${WORKESTRATE_EXAMPLE_AGENT_BUILD}",
            "example-agent",
            "/tmp/b",
            None,
        );
        assert_eq!(got, "/tmp/b");
    }
}
