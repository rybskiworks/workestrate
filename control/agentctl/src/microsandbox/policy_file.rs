//! Host-side policy-file preparation for spec 22 §12.
//!
//! The compiled policy is atomically written beneath the microsandbox
//! loader's approved mount-policy root — `$MSB_HOME/mount-policy/`
//! (default `~/.microsandbox/mount-policy/`) — with restrictive
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
/// `crates/utils/lib/lib.rs`): `MSB_HOME` verbatim, else
/// `$HOME/.microsandbox`, else `./.microsandbox`. The nix wrapper
/// force-sets `MSB_HOME=$HOME/.microsandbox` at runtime
/// (`nix/packages/agentctl.nix` postInstall), so this mirrors the SDK's
/// home in every supported flow.
fn msb_home() -> PathBuf {
    if let Some(path) = std::env::var_os("MSB_HOME") {
        return PathBuf::from(path);
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".microsandbox")
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
/// The root guest path `/` produces an empty slug; validation normally makes
/// this unusual, but callers should use a non-empty fallback if needed.
pub fn mount_slug(guest: &str) -> String {
    guest.strip_prefix('/').unwrap_or(guest).replace('/', "_")
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
}
