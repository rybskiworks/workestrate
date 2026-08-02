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
        "WORKESTRATE_CONFIG_DIR",
        "WORKESTRATE_NO_PROJECT_CONFIG",
        "WORKESTRATE_CONTEXT",
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

/// Resolve mount-host template tokens (WP6(c)/A6).
///
/// Matches, in precedence order:
/// 1. `${<env_override>}` — the workload's configured
///    `local_build.env_override` var name (checked FIRST so a custom override
///    that happens to equal the default convention still wins).
/// 2. `${WORKESTRATE_<NAME>_BUILD}` — the default convention (NAME
///    uppercased, '-' → '_'), kept for backward compatibility.
/// 3. `${CWD}` / `${CWD}/...` — current working directory (unchanged).
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
    if host == "${CWD}" {
        if let Ok(cwd) = std::env::current_dir() {
            return cwd.to_string_lossy().into_owned();
        }
    }
    if let Some(rest) = host.strip_prefix("${CWD}/") {
        if let Ok(cwd) = std::env::current_dir() {
            return format!("{}/{}", cwd.to_string_lossy(), rest);
        }
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
            "WORKESTRATE_CONFIG_DIR",
            "WORKESTRATE_NO_PROJECT_CONFIG",
            "WORKESTRATE_CONTEXT",
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
        let got = resolve_mount_host_template("${CWD}", "pi", "/tmp/build-out", None);
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(got, cwd.to_string_lossy());
        let got =
            resolve_mount_host_template("${CWD}/sub", "pi", "/tmp/build-out", Some("X_BUILD"));
        assert_eq!(got, format!("{}/sub", cwd.to_string_lossy()));
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
