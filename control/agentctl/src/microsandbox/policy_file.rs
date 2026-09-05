//! Host-side policy-file preparation for spec 22 §12.
//!
//! The compiled policy is atomically written beneath the microsandbox
//! loader's approved mount-policy root — `$MSB_HOME/mount-policy/`
//! (default `~/.microsandbox/current/mount-policy/`) — with restrictive
//! permissions, and the path handed to the SDK is the loader-RELATIVE
//! token (`<instance>/<slug>.json`), not an absolute path: the fork's
//! fail-closed loader (`load_mount_policy`) rejects absolute paths and
//! `..` components outright. The root is anchored at MSB_HOME rather than
//! the per-sandbox runtime dir because sandbox create rejects or wipes a
//! pre-existing `sandboxes/<name>` directory, so a policy staged under
//! `sandboxes/<name>/runtime/mount-policy` could never survive to VM
//! build. The microsandbox-side loader owns validation and loading; this
//! module only prepares its file input and removes it during lifecycle
//! cleanup.

use crate::mount_policy::MountPolicyProgram;
use anyhow::Result;
use std::path::{Path, PathBuf};

//--------------------------------------------------------------------------------------------------
// Constants
//--------------------------------------------------------------------------------------------------

/// MSB_HOME-anchored subdirectory holding compiled mount policies.
///
/// MIRRORS the loader-side rule: the approved-dir contract lives only in
/// the msb repo (fork `crates/utils/lib/lib.rs`
/// `MOUNT_POLICY_DIR_NAME`, consumed by the SDK at
/// `sdk/rust/lib/runtime/spawn.rs` `LaunchConfig.mount_policy_dir` and by
/// the loader at `crates/runtime/lib/vm.rs` `policy_root`). This is a
/// documented mirror constant — keep it in sync with the fork.
pub const MOUNT_POLICY_DIR_NAME: &str = "mount-policy";

//--------------------------------------------------------------------------------------------------
// Functions
//--------------------------------------------------------------------------------------------------

/// Resolve the microsandbox home directory.
///
/// Mirror of `microsandbox_utils::resolve_home` (fork
/// `crates/utils/lib/lib.rs`): non-empty `MSB_HOME` verbatim (empty treated
/// as unset), else `$HOME/.microsandbox/current` (the `current` generation
/// symlink — see [`crate::microsandbox::generation`]), else
/// `./.microsandbox/current`. The nix wrapper defaults unset/empty
/// `MSB_HOME` to `$HOME/.microsandbox/current` at runtime
/// (`nix/packages/agentctl.nix` postInstall), so this mirrors the SDK's
/// home in every supported flow.
fn msb_home() -> PathBuf {
    if let Some(path) = std::env::var_os("MSB_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(path);
    }
    crate::microsandbox::generation::default_msb_home()
}

/// The loader-approved mount-policy root (`<msb home>/mount-policy`).
pub fn approved_policy_root() -> PathBuf {
    msb_home().join(MOUNT_POLICY_DIR_NAME)
}

/// The per-mount policy-file path beneath the approved root (spec 22 §12).
pub fn policy_file_path(instance: &str, mount_slug: &str) -> PathBuf {
    approved_policy_root().join(policy_file_rel(instance, mount_slug))
}

/// The loader-RELATIVE policy token (`<instance>/<slug>.json`) handed to
/// the SDK; the fork loader resolves it beneath the approved root.
pub fn policy_file_rel(instance: &str, mount_slug: &str) -> PathBuf {
    Path::new(instance).join(format!("{mount_slug}.json"))
}

/// Convert a guest mount path into its policy-file slug.
///
/// The mapping is INJECTIVE (the §9r collision fix): plain sanitization
/// (`/` → `_`) is lossy — `/da/ta` and `/da_ta` both sanitize to `da_ta`,
/// so two DISTINCT guest paths would map to the same
/// `<instance>/da_ta.json` policy file and silently clobber each other
/// (last writer wins; exact-duplicate guests are already rejected at
/// validation, but slug-colliding distinct paths were not).
///
/// Rule: when the sanitized form contains `_` — either because separators
/// collapsed or because the path literally contains an underscore, either of
/// which could collide — the slug gains a `-<8-hex fnv1a64(raw guest)>`
/// suffix over the RAW guest path, making distinct guests produce distinct
/// files. Simple paths whose sanitized form has no underscore
/// (`/workspace` → `workspace`) keep their UNCHANGED names (host-smoke
/// assets reference plain slugs — do not churn them). The root guest `/`
/// produces the empty slug as before (existing documented edge).
pub fn mount_slug(guest: &str) -> String {
    let base = guest.strip_prefix('/').unwrap_or(guest).replace('/', "_");
    if base.contains('_') {
        format!(
            "{base}-{:08x}",
            crate::microsandbox::provenance::fnv1a64(guest.as_bytes())
        )
    } else {
        base
    }
}

/// Atomically write a compiled program as restrictive JSON beneath the
/// approved root. Returns `(absolute path, loader-relative token)`.
pub fn write_policy_file(
    instance: &str,
    mount_slug: &str,
    program: &MountPolicyProgram,
) -> Result<(PathBuf, PathBuf)> {
    let dir = approved_policy_root().join(instance);
    std::fs::create_dir_all(&dir)?;
    let final_path = policy_file_path(instance, mount_slug);
    let rel = policy_file_rel(instance, mount_slug);
    let tmp = dir.join(format!("{mount_slug}.json.tmp.{}", std::process::id()));
    let bytes = serde_json::to_vec_pretty(program)?;
    std::fs::write(&tmp, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&tmp, &final_path)?;
    Ok((final_path, rel))
}

/// Best-effort cleanup of an instance's policy directory beneath the
/// approved root, plus the LEGACY `<state_dir>/policy/<instance>` dir —
/// the pre-fix spec 22 layout orphaned by the 2026-08-18 relocation.
pub fn remove_policy_dir(instance: &str) -> Result<()> {
    let legacy = crate::config::resolve_state_dir()
        .join("policy")
        .join(instance);
    // Best-effort: a missing legacy dir is not an error.
    let _ = std::fs::remove_dir_all(legacy);
    match std::fs::remove_dir_all(approved_policy_root().join(instance)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;
    use crate::config::test_support::{uniq_dir, EnvGuard, ENV_TEST_LOCK};
    use crate::mount_policy::compile;

    /// Pin MSB_HOME to a fresh temp dir for the duration of a test.
    fn pin_msb_home(label: &str) -> (std::sync::MutexGuard<'static, ()>, EnvGuard, PathBuf) {
        let lock = ENV_TEST_LOCK.lock().unwrap();
        let guard = EnvGuard::capture(&["MSB_HOME"]);
        let home = uniq_dir(label);
        std::env::set_var("MSB_HOME", &home);
        (lock, guard, home)
    }

    #[test]
    fn layout_write_round_trip_permissions_and_cleanup() -> Result<()> {
        let (_lock, _guard, home) = pin_msb_home("policy-file");
        let program = compile(Vec::new())?;
        let (abs, rel) = write_policy_file("slot@id", "workspace", &program)?;
        // Both sides computed from the same helpers: the absolute path is
        // the approved root + the relative token.
        assert_eq!(abs, approved_policy_root().join("slot@id/workspace.json"));
        assert_eq!(abs, home.join("mount-policy/slot@id/workspace.json"));
        assert!(abs.starts_with(approved_policy_root()));
        assert_eq!(rel, PathBuf::from("slot@id/workspace.json"));
        assert_eq!(
            serde_json::from_slice::<MountPolicyProgram>(&std::fs::read(&abs)?)?,
            program
        );
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;
        #[cfg(unix)]
        assert_eq!(std::fs::metadata(&abs)?.permissions().mode() & 0o777, 0o600);
        remove_policy_dir("slot@id")?;
        assert!(!abs.exists());
        // Cleanup is idempotent.
        remove_policy_dir("slot@id")?;
        let _ = std::fs::remove_dir_all(home);
        Ok(())
    }

    #[test]
    fn remove_policy_dir_cleans_legacy_state_dir_layout() -> Result<()> {
        let lock = ENV_TEST_LOCK.lock().unwrap();
        let _guard = EnvGuard::capture(&["MSB_HOME", "WORKESTRATE_HOME"]);
        let msb = uniq_dir("policy-file-legacy-msb");
        std::env::set_var("MSB_HOME", &msb);
        let home = uniq_dir("policy-file-legacy-home");
        std::env::set_var("WORKESTRATE_HOME", &home);
        // Pre-fix spec 22 layout: <state_dir>/policy/<instance>.
        let legacy = crate::config::resolve_state_dir()
            .join("policy")
            .join("slot@id");
        std::fs::create_dir_all(&legacy)?;
        std::fs::write(legacy.join("workspace.json"), b"{}")?;
        remove_policy_dir("slot@id")?;
        assert!(!legacy.exists());
        let _ = std::fs::remove_dir_all(msb);
        let _ = std::fs::remove_dir_all(home);
        drop(lock);
        Ok(())
    }

    #[test]
    fn rel_token_satisfies_loader_path_rules() {
        // Mirror of the fork loader's PathEscape rules (`load_mount_policy`
        // in crates/runtime/lib/vm.rs): not absolute, no ParentDir
        // components.
        let rel = policy_file_rel("slot@id", "workspace");
        assert!(!rel.is_absolute());
        assert!(!rel
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir)));
    }

    // ---- §9r slug-collision fix: mount_slug is injective ----

    /// THE regression: `/da/ta` and `/da_ta` both sanitized to `da_ta`, so
    /// two DIFFERENT guest paths collided on `<instance>/da_ta.json` and the
    /// last writer silently clobbered the first. With the suffix rule they
    /// map to DISTINCT files.
    #[test]
    fn mount_slug_colliding_distinct_paths_map_to_distinct_files() {
        let slashed = mount_slug("/da/ta");
        let literal = mount_slug("/da_ta");
        assert_ne!(
            slashed, literal,
            "distinct guest paths must not share one policy-file slug"
        );
        assert_ne!(
            policy_file_rel("inst", &slashed),
            policy_file_rel("inst", &literal),
            "the colliding guests must resolve to distinct policy files"
        );
        // Both keep the sanitized stem as a recognizable prefix.
        assert!(slashed.starts_with("da_ta-"), "got {slashed}");
        assert!(literal.starts_with("da_ta-"), "got {literal}");
    }

    /// Simple paths whose sanitized form has no underscore keep their
    /// UNCHANGED legacy names (host-smoke assets reference plain slugs).
    #[test]
    fn mount_slug_simple_paths_stay_unsuffixed_and_stable() {
        assert_eq!(mount_slug("/workspace"), "workspace");
        assert_eq!(mount_slug("plain"), "plain");
        // The documented root edge is unchanged.
        assert_eq!(mount_slug("/"), "");
        // Deterministic across calls (the suffix is a pure content hash).
        assert_eq!(mount_slug("/a/b_c"), mount_slug("/a/b_c"));
    }

    /// Suffixed slugs still satisfy the fork loader's rel-token path rules
    /// (no absolute path, no parent-dir components) — the suffix adds only
    /// `[0-9a-f-]`.
    #[test]
    fn mount_slug_suffixed_tokens_satisfy_loader_path_rules() {
        for guest in ["/da/ta", "/da_ta", "/x_y/z_w", "/_"] {
            let rel = policy_file_rel("slot@id", &mount_slug(guest));
            assert!(!rel.is_absolute(), "guest {guest}: {rel:?}");
            assert!(
                !rel.components()
                    .any(|c| matches!(c, std::path::Component::ParentDir)),
                "guest {guest}: {rel:?}"
            );
        }
    }

    // ---- msb_home: SDK resolve_home mirror (non-empty verbatim) ----

    /// A set non-empty MSB_HOME is used verbatim.
    #[test]
    fn msb_home_set_is_verbatim() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _guard = EnvGuard::capture(&["MSB_HOME", "HOME"]);
        let custom = uniq_dir("policy-msb-home-set");
        std::env::set_var("MSB_HOME", &custom);
        assert_eq!(msb_home(), custom);
    }

    /// An empty MSB_HOME is treated as unset (falls back to $HOME).
    #[test]
    fn msb_home_empty_falls_back_to_home() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _guard = EnvGuard::capture(&["MSB_HOME", "HOME"]);
        let fake_home = uniq_dir("policy-msb-home-empty");
        std::env::set_var("HOME", &fake_home);
        std::env::set_var("MSB_HOME", "");
        let via_empty = msb_home();
        std::env::remove_var("MSB_HOME");
        let via_unset = msb_home();
        assert_eq!(via_empty, via_unset);
        assert_eq!(via_empty, fake_home.join(".microsandbox").join("current"));
    }

    /// Unset MSB_HOME falls back to $HOME/.microsandbox/current (or
    /// ./.microsandbox/current when HOME is also unset) — the `current`
    /// generation symlink is the canonical default home.
    #[test]
    fn msb_home_unset_uses_home_fallback() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _guard = EnvGuard::capture(&["MSB_HOME", "HOME"]);
        let fake_home = uniq_dir("policy-msb-home-unset");
        std::env::remove_var("MSB_HOME");
        std::env::set_var("HOME", &fake_home);
        assert_eq!(msb_home(), fake_home.join(".microsandbox").join("current"));
        std::env::remove_var("HOME");
        assert_eq!(
            msb_home(),
            PathBuf::from(".").join(".microsandbox").join("current")
        );
    }
}
