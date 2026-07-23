//! Integration tests for `workestrate config remove` — unregistering,
//! optional store-clone deletion, dirty-clone refusal, and force override.
//! Uses an isolated HOME + XDG per test so the user's real registry is never
//! touched.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_workestrate");

struct IsolatedHome {
    dir: PathBuf,
}

impl IsolatedHome {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "workestrate-cmd-config-remove-{}-{}",
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

    fn cmd(&self) -> Command {
        let mut c = Command::new(BIN);
        c.env("HOME", &self.dir);
        c.env("XDG_CONFIG_HOME", self.dir.join(".config"));
        c.env("XDG_DATA_HOME", self.dir.join(".local").join("share"));
        c.env_remove("WORKESTRATE_CONFIG_DIR");
        c.env_remove("WORKESTRATE_NO_PROJECT_CONFIG");
        c.env_remove("WORKESTRATE_HOME");
        c.stdin(std::process::Stdio::null());
        c
    }

    fn registry_path(&self) -> PathBuf {
        self.dir
            .join(".config")
            .join("workestrate")
            .join("config.toml")
    }

    fn store_dir(&self) -> PathBuf {
        self.dir.join(".local").join("share").join("workestrate")
    }

    fn repo_dir(&self, name: &str) -> PathBuf {
        self.store_dir().join("repos").join(name)
    }

    /// Write a registry TOML with a single `[configs.<name>]` entry.
    fn write_registry(&self, name: &str) {
        let workestrate_dir = self.dir.join(".config").join("workestrate");
        std::fs::create_dir_all(&workestrate_dir).expect("create workestrate config dir");
        let content =
            format!("[configs.{name}]\nurl = \"https://example.com/repo.git\"\nref = \"main\"\n");
        std::fs::write(self.registry_path(), content).expect("write registry");
    }

    fn read_registry(&self) -> String {
        std::fs::read_to_string(self.registry_path()).expect("read registry")
    }

    /// Create a clean git repo at the store-clone path for `name`.
    fn create_clean_git_repo(&self, name: &str) -> PathBuf {
        let dir = self.repo_dir(name);
        std::fs::create_dir_all(&dir).expect("create repo dir");
        run_git(&dir, &["init"]);
        run_git(&dir, &["config", "user.email", "test@example.com"]);
        run_git(&dir, &["config", "user.name", "Test"]);
        std::fs::write(dir.join("README.md"), "readme").expect("write readme");
        run_git(&dir, &["add", "."]);
        run_git(&dir, &["commit", "-m", "init"]);
        dir
    }
}

fn run_git(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("spawn git");
    assert!(
        status.success(),
        "git {:?} failed in {}",
        args,
        dir.display()
    );
}

/// `config remove` unregisters the repo from the registry without touching
/// the store clone.
#[test]
fn remove_unregisters_from_registry() {
    let home = IsolatedHome::new();
    home.write_registry("personal");
    home.create_clean_git_repo("personal");

    let out = home
        .cmd()
        .args(["config", "remove", "personal"])
        .output()
        .expect("invoke config remove");
    assert!(
        out.status.success(),
        "config remove failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let registry = home.read_registry();
    assert!(
        !registry.contains("[configs.personal]"),
        "registry should no longer contain the entry; got:\n{}",
        registry
    );
    // Without --delete the clone stays on disk.
    assert!(
        home.repo_dir("personal").exists(),
        "store clone should remain without --delete"
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Unregistered config repo: personal"),
        "expected unregister confirmation; got: {}",
        stdout
    );
}

/// `config remove --delete` removes both the registry entry and the store
/// clone directory.
#[test]
fn remove_with_delete_removes_store_clone() {
    let home = IsolatedHome::new();
    home.write_registry("personal");
    home.create_clean_git_repo("personal");

    let out = home
        .cmd()
        .args(["config", "remove", "personal", "--delete"])
        .output()
        .expect("invoke config remove --delete");
    assert!(
        out.status.success(),
        "config remove --delete failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        !home.repo_dir("personal").exists(),
        "store clone should be deleted"
    );
    let registry = home.read_registry();
    assert!(
        !registry.contains("[configs.personal]"),
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
    let home = IsolatedHome::new();
    home.write_registry("personal");
    let repo = home.create_clean_git_repo("personal");
    // Make it dirty: uncommitted modification to a tracked file.
    std::fs::write(repo.join("README.md"), "dirty edit").expect("dirty the repo");

    let out = home
        .cmd()
        .args(["config", "remove", "personal", "--delete"])
        .output()
        .expect("invoke config remove --delete");
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
        home.read_registry().contains("[configs.personal]"),
        "registry entry must remain after refusal"
    );
}

/// --force overrides the dirty-clone refusal.
#[test]
fn remove_delete_force_deletes_dirty_clone() {
    let home = IsolatedHome::new();
    home.write_registry("personal");
    let repo = home.create_clean_git_repo("personal");
    std::fs::write(repo.join("README.md"), "dirty edit").expect("dirty the repo");

    let out = home
        .cmd()
        .args(["config", "remove", "personal", "--delete", "--force"])
        .output()
        .expect("invoke config remove --delete --force");
    assert!(
        out.status.success(),
        "config remove --delete --force failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!repo.exists(), "dirty clone should be force-deleted");
    assert!(
        !home.read_registry().contains("[configs.personal]"),
        "registry entry should be gone"
    );
}

/// Removing a name that is not registered errors.
#[test]
fn remove_unregistered_name_errors() {
    let home = IsolatedHome::new();
    home.write_registry("personal");

    let out = home
        .cmd()
        .args(["config", "remove", "ghost"])
        .output()
        .expect("invoke config remove");
    assert!(!out.status.success(), "unregistered name should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("config repo 'ghost' is not registered"),
        "expected not-registered error; got: {}",
        stderr
    );
}
