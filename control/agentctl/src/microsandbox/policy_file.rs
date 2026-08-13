//! Host-side policy-file preparation for spec 22 §12.
//!
//! The compiled policy is atomically written under the resolved host state
//! directory with restrictive permissions. The microsandbox-side loader owns
//! validation and loading; this module only prepares its file input and
//! removes it during lifecycle cleanup.

use crate::mount_policy::MountPolicyProgram;
use anyhow::Result;
use std::path::{Path, PathBuf};

/// The approved policy root inside the host state dir (spec 22 §12).
pub fn policy_dir(state_dir: &Path) -> PathBuf {
    state_dir.join("policy")
}

/// The per-mount policy-file path (spec 22 §12).
pub fn policy_file_path(state_dir: &Path, instance: &str, mount_slug: &str) -> PathBuf {
    policy_dir(state_dir)
        .join(instance)
        .join(format!("{mount_slug}.json"))
}

/// Convert a guest mount path into its policy-file slug.
///
/// The root guest path `/` produces an empty slug; validation normally makes
/// this unusual, but callers should use a non-empty fallback if needed.
pub fn mount_slug(guest: &str) -> String {
    guest.strip_prefix('/').unwrap_or(guest).replace('/', "_")
}

/// Atomically write a compiled program as restrictive JSON.
pub fn write_policy_file(
    state_dir: &Path,
    instance: &str,
    mount_slug: &str,
    program: &MountPolicyProgram,
) -> Result<PathBuf> {
    let dir = policy_dir(state_dir).join(instance);
    std::fs::create_dir_all(&dir)?;
    let final_path = policy_file_path(state_dir, instance, mount_slug);
    let tmp = dir.join(format!("{mount_slug}.json.tmp.{}", std::process::id()));
    let bytes = serde_json::to_vec_pretty(program)?;
    std::fs::write(&tmp, bytes)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
    }
    std::fs::rename(&tmp, &final_path)?;
    Ok(final_path)
}

/// Best-effort cleanup of an instance's policy directory.
pub fn remove_policy_dir(state_dir: &Path, instance: &str) -> Result<()> {
    match std::fs::remove_dir_all(policy_dir(state_dir).join(instance)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
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
    use crate::config::test_support::unique_state_dir;
    use crate::mount_policy::compile;

    #[test]
    fn layout_write_round_trip_permissions_and_cleanup() -> Result<()> {
        let state = unique_state_dir("policy-file");
        let program = compile(Vec::new())?;
        let path = write_policy_file(&state, "slot@id", "workspace", &program)?;
        assert_eq!(path, state.join("policy/slot@id/workspace.json"));
        assert_eq!(
            serde_json::from_slice::<MountPolicyProgram>(&std::fs::read(&path)?)?,
            program
        );
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;
        #[cfg(unix)]
        assert_eq!(
            std::fs::metadata(&path)?.permissions().mode() & 0o777,
            0o600
        );
        remove_policy_dir(&state, "slot@id")?;
        assert!(!path.exists());
        remove_policy_dir(&state, "slot@id")?;
        let _ = std::fs::remove_dir_all(state);
        Ok(())
    }
}
