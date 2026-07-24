//! Integration tests for `workestrate config new` — flags, refusal cases,
//! and side effects. Uses an isolated HOME + XDG_CONFIG_HOME per test so
//! the user's real workestrate registry is never touched.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::IsolatedHome;
use std::path::{Path, PathBuf};
fn unique_dest(parent: &Path, label: &str) -> PathBuf {
    let p = parent.join(format!(
        "{}-{}",
        label,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&p).expect("create dest parent");
    p
}

/// Invalid names are rejected before any filesystem writes happen.
#[test]
fn rejects_invalid_name() {
    let home = IsolatedHome::new("cmd-config-new");
    let dest = home.dir.join("bad-name-dest");
    let out = home
        .cmd()
        .args(["config", "new", "Bad Name", "--path"])
        .arg(&dest)
        .args(["--no-register", "--no-git-init"])
        .output()
        .expect("invoke config new");
    assert!(
        !out.status.success(),
        "invalid name should be rejected; got success"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("config name") && stderr.contains("[a-z0-9]"),
        "expected name-validation error, got: {}",
        stderr
    );
    assert!(
        !dest.exists(),
        "destination should NOT be created when the name is invalid"
    );
}

/// A non-empty destination is refused.
#[test]
fn rejects_non_empty_dest() {
    let home = IsolatedHome::new("cmd-config-new");
    let dest = unique_dest(&home.dir, "non-empty-dest");
    // Populate with one file to make it non-empty.
    std::fs::write(dest.join("blocker"), "x").expect("seed blocker");

    let out = home
        .cmd()
        .args(["config", "new", "goodname", "--path"])
        .arg(&dest)
        .args(["--no-register", "--no-git-init"])
        .output()
        .expect("invoke config new");
    assert!(!out.status.success(), "non-empty dest should be rejected");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("exists and is non-empty"),
        "expected non-empty-dest error, got: {}",
        stderr
    );
}

/// `--no-register` skips the registry write.
#[test]
fn no_register_skips_registry() {
    let home = IsolatedHome::new("cmd-config-new");
    let dest = home.dir.join("no-register-dest");

    let out = home
        .cmd()
        .args(["config", "new", "noreg", "--path"])
        .arg(&dest)
        .args([
            "--no-register",
            "--no-git-init",
            "--age-recipient",
            "age1TEST",
        ])
        .output()
        .expect("invoke config new");
    assert!(
        out.status.success(),
        "config new --no-register failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The workestrate registry under HOME/.config/workestrate/config.toml
    // must NOT exist or must NOT contain the name.
    let reg_path = home
        .dir
        .join(".config")
        .join("workestrate")
        .join("config.toml");
    if reg_path.exists() {
        let content = std::fs::read_to_string(&reg_path).unwrap();
        assert!(
            !content.contains("noreg"),
            "registry should not contain 'noreg' under --no-register; got:\n{}",
            content
        );
    }
}

/// `--no-git-init` skips `git init` (no .git directory in dest).
#[test]
fn no_git_init_skips_git() {
    let home = IsolatedHome::new("cmd-config-new");
    let dest = home.dir.join("no-git-dest");

    let out = home
        .cmd()
        .args(["config", "new", "nogit", "--path"])
        .arg(&dest)
        .args([
            "--no-register",
            "--no-git-init",
            "--age-recipient",
            "age1TEST",
        ])
        .output()
        .expect("invoke config new");
    assert!(
        out.status.success(),
        "config new --no-git-init failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !dest.join(".git").exists(),
        ".git should NOT exist under --no-git-init"
    );
    // Files should still be written.
    assert!(dest.join("workestrate.toml").exists());
}

/// When age-keygen is unavailable (no key file), the scaffold falls back
/// to age1PLACEHOLDER + a stderr warning.
#[test]
fn placeholder_recipient_when_derivation_fails() {
    let home = IsolatedHome::new("cmd-config-new");
    let dest = home.dir.join("placeholder-dest");

    let out = home
        .cmd()
        .args(["config", "new", "pholder", "--path"])
        .arg(&dest)
        .args(["--no-register", "--no-git-init"])
        .output()
        .expect("invoke config new");
    assert!(
        out.status.success(),
        "config new with missing key file failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let sops = std::fs::read_to_string(dest.join(".sops.yaml")).expect("read .sops.yaml");
    assert!(
        sops.contains("age1PLACEHOLDER"),
        ".sops.yaml should contain age1PLACEHOLDER when derivation fails; got:\n{}",
        sops
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("warning") && stderr.contains("age1PLACEHOLDER"),
        "stderr should warn about the placeholder; got:\n{}",
        stderr
    );
}

/// Already-registered name bails BEFORE writing any files (fail-fast).
#[test]
fn already_registered_bails_before_writes() {
    let home = IsolatedHome::new("cmd-config-new");
    let dest1 = home.dir.join("dest1");
    let dest2 = home.dir.join("dest2");

    // First invocation registers "dupe".
    let out1 = home
        .cmd()
        .args(["config", "new", "dupe", "--path"])
        .arg(&dest1)
        .args(["--no-git-init", "--age-recipient", "age1TEST"])
        .output()
        .expect("first config new");
    assert!(
        out1.status.success(),
        "first config new failed: stderr=\n{}",
        String::from_utf8_lossy(&out1.stderr)
    );

    // Second invocation with the same name must fail AND leave dest2 empty.
    let out2 = home
        .cmd()
        .args(["config", "new", "dupe", "--path"])
        .arg(&dest2)
        .args(["--no-git-init", "--age-recipient", "age1TEST"])
        .output()
        .expect("second config new");
    assert!(!out2.status.success(), "duplicate name should be rejected");
    let stderr = String::from_utf8_lossy(&out2.stderr);
    assert!(
        stderr.contains("already registered"),
        "expected already-registered error, got: {}",
        stderr
    );
    assert!(
        !dest2.exists(),
        "dest2 should NOT be created when the name is already registered"
    );
}

/// `--json` emits a valid JSON envelope with the expected fields.
#[test]
fn json_envelope_is_valid() {
    let home = IsolatedHome::new("cmd-config-new");
    let dest = home.dir.join("json-dest");

    let out = home
        .cmd()
        .args(["config", "new", "jsontest", "--path"])
        .arg(&dest)
        .args([
            "--no-register",
            "--no-git-init",
            "--age-recipient",
            "age1JSONTEST",
            "--json",
        ])
        .output()
        .expect("invoke config new --json");
    assert!(
        out.status.success(),
        "config new --json failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let v: serde_json::Value =
        serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("invalid JSON: {}\n{}", e, stdout));

    assert_eq!(v["name"], "jsontest");
    assert_eq!(v["age_recipient_source"], "flag");
    assert_eq!(v["age_recipient"], "age1JSONTEST");
    assert_eq!(v["git_initialized"], false);
    assert_eq!(v["registered"], false);
    let files: Vec<String> =
        serde_json::from_value(v["files_written"].clone()).expect("files array");
    assert!(files.contains(&"workestrate.toml".to_string()));
    assert!(files.contains(&".sops.yaml".to_string()));
    assert!(files.contains(&".copier-answers.yml".to_string()));
}

/// WP-C: default dest is the managed store, not ./<name>. The repo lands
/// in <store>/repos/<name> and a subsequent validate-config (which calls
/// load_config) sees it as a layer.
#[test]
fn config_new_default_path_is_store() {
    let home = IsolatedHome::new("cmd-config-new");

    // Use WORKESTRATE_HOME so the store path is predictable.
    let store = home.dir.join(".workestrate");
    let expected = store.join("repos").join("personal");

    let out = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args([
            "config",
            "new",
            "personal",
            "--no-git-init",
            "--age-recipient",
            "age1TEST",
        ])
        .output()
        .expect("invoke config new");
    assert!(
        out.status.success(),
        "config new failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The repo must land in the store, not ./personal.
    assert!(
        expected.join("workestrate.toml").exists(),
        "workestrate.toml should exist at {} (store default), not ./personal",
        expected.display()
    );
    assert!(
        !home.dir.join("personal").join("workestrate.toml").exists(),
        "repo should NOT be at ./personal (old default)"
    );

    // A subsequent validate-config must succeed — load_config resolves the
    // active context, finds "personal" in the bare layers list (auto-added
    // by register_config), and loads the workestrate.toml from the store.
    let out = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args(["validate-config"])
        .output()
        .expect("invoke validate-config");
    assert!(
        out.status.success(),
        "validate-config should succeed (repo is a layer in the store); stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// WP-C: explicit --path outside the store prints a warning when
/// registration is enabled.
#[test]
fn config_new_explicit_path_outside_store_warns() {
    let home = IsolatedHome::new("cmd-config-new");
    let store = home.dir.join(".workestrate");
    let dest = home.dir.join("outside-store-dest");

    let out = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args(["config", "new", "personal", "--path"])
        .arg(&dest)
        .args(["--no-git-init", "--age-recipient", "age1TEST"])
        .output()
        .expect("invoke config new");

    assert!(
        out.status.success(),
        "config new with explicit --path should succeed; stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("warning") && stderr.contains("outside the config store"),
        "stderr should warn about path outside store; got:\n{}",
        stderr
    );
    assert!(
        stderr.contains("won't be active for layer resolution"),
        "stderr should explain the consequence; got:\n{}",
        stderr
    );

    // The repo should be at the explicit path, not in the store.
    assert!(
        dest.join("workestrate.toml").exists(),
        "repo should be at explicit --path"
    );
    assert!(
        !store
            .join("repos")
            .join("personal")
            .join("workestrate.toml")
            .exists(),
        "repo should NOT be in the store when --path is explicit"
    );
}

/// FS-25: when `--path` contains a symlink component, canonicalize() resolves
/// it to a DIFFERENT registered url — the CLI must emit the "canonicalized
/// path" note on stderr, and the registry records the canonical form.
#[cfg(unix)]
#[test]
fn config_new_symlinked_path_registers_canonical_url_with_note() {
    let home = IsolatedHome::new("cmd-config-new");
    let store = home.dir.join(".workestrate");
    let real_parent = unique_dest(&home.dir, "fs25-real");
    let link = home.dir.join("fs25-link");
    std::os::unix::fs::symlink(&real_parent, &link).expect("create symlink");
    let dest_via_link = link.join("dest");

    let out = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args(["config", "new", "personal", "--path"])
        .arg(&dest_via_link)
        .args(["--no-git-init", "--age-recipient", "age1TEST"])
        .output()
        .expect("invoke config new");

    assert!(
        out.status.success(),
        "config new via symlinked --path should succeed; stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("canonicalized path"),
        "stderr should note the canonicalize() rewrite; got:\n{}",
        stderr
    );

    // The registry records the CANONICAL (symlink-resolved) url.
    let registry_raw = std::fs::read_to_string(store.join("config.toml")).expect("read registry");
    let canonical = std::fs::canonicalize(real_parent.join("dest")).unwrap();
    assert!(
        registry_raw.contains(&canonical.to_string_lossy().to_string()),
        "registry should record the canonical url {}:\n{}",
        canonical.display(),
        registry_raw
    );
}
