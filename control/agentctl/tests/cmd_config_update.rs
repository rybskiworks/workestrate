//! Integration tests for `workestrate config update` — the dirty-clone
//! guard: a registered clone with uncommitted changes (modified tracked
//! files OR untracked-only) is refused BEFORE any pull, and the registry
//! on disk is left untouched. Uses an isolated HOME + XDG per test so the
//! user's real registry is never touched.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::IsolatedHome;

/// A dirty clone (modified tracked file) is refused before any pull.
#[test]
fn update_refuses_dirty_clone() {
    let home = IsolatedHome::new("cmd-config-update");
    home.write_registry_entry("personal", "");
    let repo = home.create_clean_git_repo("personal");
    // Make it dirty: uncommitted modification to a tracked file.
    std::fs::write(repo.join("README.md"), "dirty edit").expect("dirty the repo");

    let out = home
        .cmd()
        .args(["config", "update", "personal"])
        .output()
        .expect("invoke config update");
    assert!(
        !out.status.success(),
        "dirty clone should fail; stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("uncommitted changes"),
        "expected dirty-clone error; got: {}",
        stderr
    );
    // No pull happened.
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("updated to"),
        "no pull should have run; got: {}",
        stdout
    );
    // Registry on disk is unchanged: entry still present, no rev recorded.
    let registry = home.read_registry();
    assert!(
        registry.contains("[configs.personal]"),
        "registry entry must remain after refusal; got:\n{}",
        registry
    );
    assert!(
        !registry.contains("rev ="),
        "registry must not gain a rev line after refusal; got:\n{}",
        registry
    );
}

/// An untracked-only clone also counts as dirty (FN-1 parity with
/// `config remove --delete`).
#[test]
fn update_refuses_untracked_only_dirty_clone() {
    let home = IsolatedHome::new("cmd-config-update");
    home.write_registry_entry("personal", "");
    let repo = home.create_clean_git_repo("personal");
    // Make it dirty via an untracked file only.
    std::fs::write(repo.join("scratch.txt"), "untracked").expect("dirty the repo");

    let out = home
        .cmd()
        .args(["config", "update", "personal"])
        .output()
        .expect("invoke config update");
    assert!(
        !out.status.success(),
        "untracked-only dirty clone should fail; stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("uncommitted changes"),
        "expected dirty-clone error; got: {}",
        stderr
    );
    // No pull happened.
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("updated to"),
        "no pull should have run; got: {}",
        stdout
    );
    // Registry on disk is unchanged: entry still present, no rev recorded.
    let registry = home.read_registry();
    assert!(
        registry.contains("[configs.personal]"),
        "registry entry must remain after refusal; got:\n{}",
        registry
    );
    assert!(
        !registry.contains("rev ="),
        "registry must not gain a rev line after refusal; got:\n{}",
        registry
    );
}
