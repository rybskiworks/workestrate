//! Integration tests for `workestrate fleet remove` — unregistering,
//! optional store-clone deletion, dirty-clone refusal, and force override.
//! Uses an isolated HOME + XDG per test so the user's real registry is never
//! touched.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::IsolatedHome;

/// `fleet remove` unregisters the repo from the registry without touching
/// the store clone.
#[test]
fn remove_unregisters_from_registry() {
    let home = IsolatedHome::new("cmd-config-remove");
    home.write_registry_entry("personal", "");
    home.create_clean_git_fleet("personal");

    let out = home
        .cmd()
        .args(["fleet", "remove", "personal"])
        .output()
        .expect("invoke fleet remove");
    assert!(
        out.status.success(),
        "fleet remove failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let registry = home.read_registry();
    assert!(
        !registry.contains("[fleets.personal]"),
        "registry should no longer contain the entry; got:\n{}",
        registry
    );
    // Without --delete the clone stays on disk.
    assert!(
        home.fleet_dir("personal").exists(),
        "store clone should remain without --delete"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Unregistered fleet: personal"),
        "expected unregister confirmation; got: {}",
        stdout
    );
}

/// `fleet remove --delete` removes both the registry entry and the store
/// clone directory.
#[test]
fn remove_with_delete_removes_store_clone() {
    let home = IsolatedHome::new("cmd-config-remove");
    home.write_registry_entry("personal", "");
    home.create_clean_git_fleet("personal");

    let out = home
        .cmd()
        .args(["fleet", "remove", "personal", "--delete"])
        .output()
        .expect("invoke fleet remove --delete");
    assert!(
        out.status.success(),
        "fleet remove --delete failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        !home.fleet_dir("personal").exists(),
        "store clone should be deleted"
    );
    let registry = home.read_registry();
    assert!(
        !registry.contains("[fleets.personal]"),
        "registry entry should be gone; got:\n{}",
        registry
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Deleted store clone:"),
        "expected deletion notice; got: {}",
        stdout
    );
}

/// A dirty clone is refused without --force.
#[test]
fn remove_delete_refuses_dirty_clone() {
    let home = IsolatedHome::new("cmd-config-remove");
    home.write_registry_entry("personal", "");
    let repo = home.create_clean_git_fleet("personal");
    // Make it dirty: uncommitted modification to a tracked file.
    std::fs::write(repo.join("README.md"), "dirty edit").expect("dirty the repo");

    let out = home
        .cmd()
        .args(["fleet", "remove", "personal", "--delete"])
        .output()
        .expect("invoke fleet remove --delete");
    assert!(
        !out.status.success(),
        "dirty clone without --force should fail"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("uncommitted changes") && stderr.contains("--force"),
        "expected dirty-clone error; got: {}",
        stderr
    );
    // Clone and registry entry must both still be present.
    assert!(repo.exists(), "clone must remain after refusal");
    assert!(
        home.read_registry().contains("[fleets.personal]"),
        "registry entry must remain after refusal"
    );
}

/// --force overrides the dirty-clone refusal.
#[test]
fn remove_delete_force_deletes_dirty_clone() {
    let home = IsolatedHome::new("cmd-config-remove");
    home.write_registry_entry("personal", "");
    let repo = home.create_clean_git_fleet("personal");
    std::fs::write(repo.join("README.md"), "dirty edit").expect("dirty the repo");

    let out = home
        .cmd()
        .args(["fleet", "remove", "personal", "--delete", "--force"])
        .output()
        .expect("invoke fleet remove --delete --force");
    assert!(
        out.status.success(),
        "fleet remove --delete --force failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!repo.exists(), "dirty clone should be force-deleted");
    assert!(
        !home.read_registry().contains("[fleets.personal]"),
        "registry entry should be gone"
    );
}

/// Removing a name that is not registered errors.
#[test]
fn remove_unregistered_name_errors() {
    let home = IsolatedHome::new("cmd-config-remove");
    home.write_registry_entry("personal", "");

    let out = home
        .cmd()
        .args(["fleet", "remove", "ghost"])
        .output()
        .expect("invoke fleet remove");
    assert!(!out.status.success(), "unregistered name should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("fleet 'ghost' is not registered"),
        "expected not-registered error; got: {}",
        stderr
    );
}
