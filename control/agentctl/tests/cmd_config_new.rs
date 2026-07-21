//! Integration tests for `workestrate config new` — flags, refusal cases,
//! and side effects. Uses an isolated HOME + XDG_CONFIG_HOME per test so
//! the user's real workestrate registry is never touched.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_workestrate");

/// Isolated sandbox: creates a fresh HOME under std::env::temp_dir() and
/// returns it along with the env vars to set. Drop is the caller's job
/// (these tests don't need cleanup — the temp_dir is process-id-namespaced
/// and the OS reaps it eventually).
struct IsolatedHome {
    dir: PathBuf,
}

impl IsolatedHome {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "workestrate-cmd-config-new-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("create isolated HOME");
        std::fs::create_dir_all(dir.join(".config")).expect("create .config");
        std::fs::create_dir_all(dir.join(".local").join("share")).expect("create .local/share");
        Self { dir }
    }

    /// Build a Command with HOME / XDG / WORKESTRATE_CONFIG_DIR pointed at
    /// this isolated root.
    fn cmd(&self) -> Command {
        let mut c = Command::new(BIN);
        c.env("HOME", &self.dir);
        c.env("XDG_CONFIG_HOME", self.dir.join(".config"));
        c.env("XDG_DATA_HOME", self.dir.join(".local").join("share"));
        c.env_remove("WORKESTRATE_CONFIG_DIR");
        c.env_remove("WORKESTRATE_NO_PROJECT_CONFIG");
        c
    }
}

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
    let home = IsolatedHome::new();
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
    let home = IsolatedHome::new();
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
    let home = IsolatedHome::new();
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
    let home = IsolatedHome::new();
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
    let home = IsolatedHome::new();
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
    let home = IsolatedHome::new();
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
    let home = IsolatedHome::new();
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
