//! Trust gating: the `[trusted_projects]` registry list and the base-registry
//! trust-check.

use anyhow::Result;
use std::path::Path;

use crate::config::paths::{base_registry_path, expand_tilde};
use crate::config::{load_registry, Registry, TrustedProject};

/// Base-registry trust check; reads the base registry directly to avoid
/// recursing through [`registry_path`] → [`resolve_home_with_kind`]. The
/// discovery tier that was its original call site was removed (spec 08 step
/// (e)); the function is kept as the base-registry trust check (exercised by
/// the FS-21 test below).
pub fn is_dir_trusted_via_base_registry(dir: &Path) -> bool {
    let path = base_registry_path();
    if !path.exists() {
        return false;
    }
    // FS-21: a base registry that EXISTS but fails to read/parse must not
    // silently disable trust. Trust still fails closed
    // (`false`), but the operator gets a one-time loud stderr warning —
    // mirroring `load_registry_for_dir_resolution`'s corrupt-registry note.
    // One-time: this check may run many times per command, so an
    // unguarded warning would print many times per command.
    let reg = match std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| toml::from_str::<Registry>(&c).ok())
    {
        Some(reg) => reg,
        None => {
            emit_trust_registry_parse_warn(&path);
            return false;
        }
    };
    let canonical_dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    reg.trusted_projects.iter().any(|p| {
        let expanded = expand_tilde(&p.path);
        let canonical_p = std::fs::canonicalize(&expanded)
            .or_else(|_| std::fs::canonicalize(&p.path))
            .unwrap_or_else(|_| expanded.clone());
        canonical_p == canonical_dir || expanded == dir || Path::new(&p.path) == dir
    })
}

/// One-time stderr warning when the trust-check registry exists but fails to
/// parse (FS-21). `is_dir_trusted_via_base_registry` runs per ancestor per
/// home resolution; without this guard the warning would repeat per call.
static TRUST_REGISTRY_PARSE_WARN: std::sync::Once = std::sync::Once::new();

fn emit_trust_registry_parse_warn(path: &Path) {
    TRUST_REGISTRY_PARSE_WARN.call_once(|| {
        eprintln!(
            "WARNING: trust-check registry {} exists but failed to parse; treating all              projects as untrusted (trust fails closed). Fix or remove the registry file.",
            path.display()
        );
    });
}

/// Whether `dir` is in the registry's `[trusted_projects]` list.
///
/// Path comparison canonicalizes both sides first (closes review finding A18),
/// so a project registered via a symlink or a `..`-containing path matches
/// queries through any equivalent path. Falls back to lexical comparison
/// (with `expand_tilde`) when canonicalization fails (e.g. broken symlink,
/// non-existent path), preserving backward compatibility with pre-A18
/// registries that stored non-canonical paths.
pub fn is_trusted_project(dir: &Path) -> bool {
    let canonical_dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    if let Ok(Some(registry)) = load_registry() {
        registry.trusted_projects.iter().any(|p| {
            let expanded = expand_tilde(&p.path);
            let canonical_p = std::fs::canonicalize(&expanded)
                .or_else(|_| std::fs::canonicalize(&p.path))
                .unwrap_or_else(|_| expanded.clone());
            canonical_p == canonical_dir || expanded == dir || Path::new(&p.path) == dir
        })
    } else {
        false
    }
}

/// Register `dir` as trusted. The path is canonicalized before storage (closes
/// review finding A18) so future queries through symlinks or `..`-containing
/// paths match consistently. Dedup considers both canonical and lexical forms
/// of existing entries.
pub fn trust_project(dir: &Path) -> Result<()> {
    // FN-5: the load → mutate → save sequence runs under the advisory
    // registry lock (see config::registry::with_registry_lock); the save
    // itself is an atomic tmp-write + rename.
    crate::config::registry::with_registry_lock(|registry| {
        let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
        let dir_str = canonical.to_string_lossy().to_string();
        let already = registry.trusted_projects.iter().any(|p| {
            if p.path == dir_str {
                return true;
            }
            let expanded = expand_tilde(&p.path);
            std::fs::canonicalize(&expanded)
                .map(|c| c == canonical)
                .unwrap_or(false)
        });
        if !already {
            registry
                .trusted_projects
                .push(TrustedProject { path: dir_str });
        }
        Ok(())
    })
}

/// Remove `dir` from the trusted list. Matches by canonical OR lexical form
/// (closes review finding A18) so untrusting via a different-but-equivalent
/// path still works.
pub fn untrust_project(dir: &Path) -> Result<()> {
    crate::config::registry::with_registry_lock(|registry| {
        let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
        let canonical_str = canonical.to_string_lossy().to_string();
        registry.trusted_projects.retain(|p| {
            if p.path == canonical_str {
                return false;
            }
            let expanded = expand_tilde(&p.path);
            let p_canonical = std::fs::canonicalize(&expanded).unwrap_or_else(|_| expanded.clone());
            p_canonical != canonical
        });
        Ok(())
    })
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_in_result
    )]
    use super::*;
    use crate::config::test_support::*;

    // ---- A1 regression: workestrate.local.toml requires trust gate ----

    /// A1 regression: with a registry present but cwd NOT trusted, a
    /// workestrate.local.toml in cwd must NOT be loaded. After trusting
    /// the project dir, the local layer MUST be loaded.
    ///
    /// The test opts into the reference base layer
    /// (`WORKESTRATE_REFERENCE_CONFIG=1`, cleanup phase 2): case 1 gates out
    /// EVERY non-reference layer, and without the base layer `load_config`
    /// would bail with "no config found" before the assertion could observe
    /// the gated marker's absence.
    #[test]
    #[ignore = "S4 migrates config.reference/workestrate.toml to the read/write mount-policy vocabulary"]
    fn local_toml_requires_trust_gate() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();

        let root = std::env::temp_dir().join(format!(
            "workestrate-a1-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(root.join("cwd"))?;
        std::fs::create_dir_all(root.join("home").join(".config").join("workestrate"))?;

        std::fs::write(
            root.join("cwd").join("workestrate.local.toml"),
            one_workload_toml("a1_hostile_local_marker"),
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        let old_no_project = std::env::var("WORKESTRATE_NO_PROJECT_CONFIG").ok();
        let old_ref = std::env::var("WORKESTRATE_REFERENCE_CONFIG").ok();
        let old_cwd = std::env::current_dir().ok();

        std::env::set_var("HOME", root.join("home"));
        std::env::set_var(
            "XDG_CONFIG_HOME",
            root.join("home").join(".config").to_string_lossy().as_ref(),
        );
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");
        std::env::remove_var("WORKESTRATE_NO_PROJECT_CONFIG");
        // Cleanup phase 2: opt into the reference base layer so case 1 (all
        // cwd layers gated out) still resolves a config instead of bailing
        // with "no config found".
        std::env::set_var("WORKESTRATE_REFERENCE_CONFIG", "1");
        std::env::set_current_dir(root.join("cwd"))?;

        // Case 1: registry exists, cwd NOT trusted -> local layer gated out.
        write_test_registry(&root.join("home"), &[])?;
        let cfg = crate::config::load_config()?;
        assert!(
            !cfg.workloads.contains_key("a1_hostile_local_marker"),
            "A1 regression: local.toml loaded without trust! workloads: {:?}",
            cfg.workloads.keys().collect::<Vec<_>>()
        );

        // Case 2: trust cwd -> local layer loads.
        let cwd_canonical = std::fs::canonicalize(root.join("cwd"))?;
        trust_project(&cwd_canonical)?;
        let cfg2 = crate::config::load_config()?;
        assert!(
            cfg2.workloads.contains_key("a1_hostile_local_marker"),
            "A1 regression: local.toml NOT loaded after trust! workloads: {:?}",
            cfg2.workloads.keys().collect::<Vec<_>>()
        );

        // Restore env
        if let Some(c) = old_cwd {
            std::env::set_current_dir(c)?;
        }
        for (k, v) in [
            ("HOME", old_home),
            ("XDG_CONFIG_HOME", old_xdg),
            ("WORKESTRATE_CONTEXT", old_ctx),
            ("WORKESTRATE_CONFIG_DIR", old_config_dir),
            ("WORKESTRATE_NO_PROJECT_CONFIG", old_no_project),
            ("WORKESTRATE_REFERENCE_CONFIG", old_ref),
        ] {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    /// A1 corollary: in bootstrap mode (no registry yet), local.toml IS
    /// loaded — this matches the project-layer bootstrap behavior.
    #[test]
    fn local_toml_loaded_in_bootstrap_mode() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();

        let root = std::env::temp_dir().join(format!(
            "workestrate-a1boot-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(root.join("cwd"))?;
        std::fs::create_dir_all(root.join("home"))?;

        std::fs::write(
            root.join("cwd").join("workestrate.local.toml"),
            one_workload_toml("a1_bootstrap_marker"),
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        let old_no_project = std::env::var("WORKESTRATE_NO_PROJECT_CONFIG").ok();
        let old_cwd = std::env::current_dir().ok();

        std::env::set_var("HOME", root.join("home"));
        std::env::set_var(
            "XDG_CONFIG_HOME",
            root.join("home").join(".config").to_string_lossy().as_ref(),
        );
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");
        std::env::remove_var("WORKESTRATE_NO_PROJECT_CONFIG");
        std::env::set_current_dir(root.join("cwd"))?;

        // NO registry file present -> load_registry returns None -> bootstrap.
        let cfg = crate::config::load_config()?;
        assert!(
            cfg.workloads.contains_key("a1_bootstrap_marker"),
            "A1 bootstrap: local.toml should load when no registry exists; got {:?}",
            cfg.workloads.keys().collect::<Vec<_>>()
        );

        if let Some(c) = old_cwd {
            std::env::set_current_dir(c)?;
        }
        for (k, v) in [
            ("HOME", old_home),
            ("XDG_CONFIG_HOME", old_xdg),
            ("WORKESTRATE_CONTEXT", old_ctx),
            ("WORKESTRATE_CONFIG_DIR", old_config_dir),
            ("WORKESTRATE_NO_PROJECT_CONFIG", old_no_project),
        ] {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    // ---- FS-21: corrupt trust-check registry warns and fails closed ----

    /// FS-21: when the base registry EXISTS but is corrupt, the trust check
    /// must fail closed (dir reported untrusted — the pre-existing behavior)
    /// AND surface a one-time stderr warning (the new part; the warning
    /// emission itself is exercised by this call but not captured — stderr
    /// capture is not available without interprocess plumbing).
    #[test]
    fn corrupt_base_registry_fails_closed_and_warns() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let root = std::env::temp_dir().join(format!(
            "workestrate-fs21-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let home = root.join("home");
        let project = root.join("project");
        std::fs::create_dir_all(home.join(".workestrate"))?;
        std::fs::create_dir_all(&project)?;

        // Corrupt base registry at the Default home location.
        std::fs::write(
            home.join(".workestrate").join("config.toml"),
            "this is = not = valid toml [[[",
        )?;

        std::env::set_var("HOME", &home);
        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::remove_var("XDG_DATA_HOME");
        std::env::remove_var("XDG_STATE_HOME");
        std::env::remove_var("WORKESTRATE_HOME");

        // Fails closed: untrusted. (The one-time stderr warning fires here.)
        assert!(
            !is_dir_trusted_via_base_registry(&project),
            "corrupt trust registry must fail closed (untrusted)"
        );

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    // ---- A18 regression: trust paths are canonicalized ----

    /// A18 regression: trust a project via a path containing a `.` or via a
    /// symlink, then query trust via a different-but-equivalent path — both
    /// must report trusted.
    #[test]
    fn trust_paths_are_canonicalized() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();

        let root = std::env::temp_dir().join(format!(
            "workestrate-a18-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let real = root.join("real");
        let linked = root.join("link");
        std::fs::create_dir_all(&real)?;
        std::os::unix::fs::symlink(&real, &linked).or_else(|_| std::fs::create_dir_all(&linked))?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("HOME", &root);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            root.join(".config").to_string_lossy().as_ref(),
        );
        std::fs::create_dir_all(root.join(".config").join("workestrate"))?;

        std::fs::write(
            root.join(".config").join("workestrate").join("config.toml"),
            "layers = []\n",
        )?;

        // Trust via the symlink path; query via the canonical path.
        trust_project(&linked)?;
        let canonical_real = std::fs::canonicalize(&real)?;
        assert!(
            is_trusted_project(&canonical_real),
            "A18: trust via symlink '{}' did not match canonical '{}'",
            linked.display(),
            canonical_real.display()
        );

        // Query via a `..`-containing equivalent path through a parent reference.
        let sibling = root.join("sibling");
        std::fs::create_dir_all(&sibling)?;
        let dotted = sibling.join("..").join("real");
        assert!(
            is_trusted_project(&dotted),
            "A18: trust query via dotted path '{}' failed",
            dotted.display()
        );

        // untrust via canonical, then verify gone via symlink.
        untrust_project(&canonical_real)?;
        assert!(
            !is_trusted_project(&linked),
            "A18: untrust via canonical did not remove symlink-form trust"
        );

        for (k, v) in [("HOME", old_home), ("XDG_CONFIG_HOME", old_xdg)] {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }
}
