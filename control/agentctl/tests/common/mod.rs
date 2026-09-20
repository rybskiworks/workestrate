//! Shared integration-test harness: one canonical `IsolatedHome` (isolated
//! HOME + XDG per test, with RAII temp-dir cleanup) plus a bare `TempDir`
//! guard for suites that only need a scratch directory (e.g. scaffold
//! renders). Replaces the per-file `IsolatedHome` copies that each leaked
//! their `/tmp/workestrate-*` dir.
//!
//! Every method is part of the shared surface: not every consuming test file
//! uses every helper, so `dead_code` is allowed at the item level rather
//! than crate-wide.

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

pub const BIN: &str = env!("CARGO_BIN_EXE_workestrate");

/// RAII temp-dir guard: removes the directory tree on drop (best-effort).
pub struct TempDir {
    dir: PathBuf,
}

impl TempDir {
    pub fn new(label: &str) -> Self {
        let dir = unique_temp_dir(label);
        std::fs::create_dir_all(&dir).expect("create temp dir");
        Self { dir }
    }

    pub fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn unique_temp_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "workestrate-{}-{}-{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}

/// Isolated sandbox: a fresh HOME under `std::env::temp_dir()` with the
/// legacy-XDG subdirs pre-created. `Drop` removes the whole tree (fixes the
/// historical `/tmp/workestrate-*` leak in every per-file copy).
pub struct IsolatedHome {
    pub dir: PathBuf,
}

impl IsolatedHome {
    /// `label` namespaces the temp dir per suite (e.g. "cmd-clean").
    pub fn new(label: &str) -> Self {
        let dir = unique_temp_dir(label);
        std::fs::create_dir_all(&dir).expect("create isolated HOME");
        std::fs::create_dir_all(dir.join(".config")).expect("create .config");
        std::fs::create_dir_all(dir.join(".local").join("share")).expect("create .local/share");
        std::fs::create_dir_all(dir.join(".local").join("state")).expect("create .local/state");
        Self { dir }
    }

    /// Build a Command with HOME / XDG pointed at this isolated root and all
    /// workestrate/sops env overrides removed. Stdin is /dev/null
    /// (non-interactive) unless the caller overrides it.
    ///
    /// `WORKESTRATE_REFERENCE_CONFIG=1` is set explicitly: these suites were
    /// written against the reference base layer being auto-included (the
    /// spawned child inherits `CARGO_MANIFEST_DIR`, so the repo's
    /// `config.reference/workestrate.toml` resolves via the manifest tier).
    /// Since cleanup phase 2 the base layer is opt-in; suites that exercise
    /// the DEFAULT (opted-out) behavior build their own Command instead
    /// (see `cwd_reference_warning.rs`).
    pub fn cmd(&self) -> Command {
        let mut c = Command::new(BIN);
        c.env("HOME", &self.dir);
        c.env("XDG_CONFIG_HOME", self.dir.join(".config"));
        c.env("XDG_DATA_HOME", self.dir.join(".local").join("share"));
        c.env("XDG_STATE_HOME", self.dir.join(".local").join("state"));
        c.env("WORKESTRATE_REFERENCE_CONFIG", "1");
        c.env_remove("WORKESTRATE_FLEET_DIR");
        c.env_remove("WORKESTRATE_NO_PROJECT_CONFIG");
        c.env_remove("WORKESTRATE_CONFIG");
        c.env_remove("WORKESTRATE_CONTEXT");
        c.env_remove("SOPS_AGE_KEY_FILE");
        c.stdin(std::process::Stdio::null());
        c
    }

    /// Legacy-XDG state dir: `$XDG_STATE_HOME/workestrate`.
    pub fn state_dir(&self) -> PathBuf {
        self.dir.join(".local").join("state").join("workestrate")
    }

    /// Store dir: `$XDG_DATA_HOME/workestrate`.
    pub fn store_dir(&self) -> PathBuf {
        self.dir.join(".local").join("share").join("workestrate")
    }

    /// Registry file: `$XDG_CONFIG_HOME/workestrate/config.toml`.
    pub fn registry_path(&self) -> PathBuf {
        self.dir
            .join(".config")
            .join("workestrate")
            .join("config.toml")
    }

    /// Store-clone path for a registered fleet name.
    pub fn fleet_dir(&self, name: &str) -> PathBuf {
        self.store_dir().join("fleets").join(name)
    }

    /// Write a registry TOML with verbatim content.
    pub fn write_registry(&self, content: &str) {
        let workestrate_dir = self.dir.join(".config").join("workestrate");
        std::fs::create_dir_all(&workestrate_dir).expect("create workestrate config dir");
        std::fs::write(self.registry_path(), content).expect("write registry");
    }

    /// Write a registry TOML with a single `[fleets.<name>]` entry plus
    /// optional extra per-entry lines.
    pub fn write_registry_entry(&self, name: &str, extra_lines: &str) {
        let content = format!(
            "[fleets.{name}]\nurl = \"https://example.com/repo.git\"\nref = \"main\"\n{extra_lines}"
        );
        self.write_registry(&content);
    }

    pub fn read_registry(&self) -> String {
        std::fs::read_to_string(self.registry_path()).expect("read registry")
    }

    /// Create the fleet checkout dir in the store so `dir` exists.
    pub fn create_fleet_dir(&self, name: &str) -> PathBuf {
        let dir = self.fleet_dir(name);
        std::fs::create_dir_all(&dir).expect("create fleet dir");
        dir
    }

    /// Create a clean git repo at the store-clone path for `name`.
    pub fn create_clean_git_fleet(&self, name: &str) -> PathBuf {
        let dir = self.fleet_dir(name);
        std::fs::create_dir_all(&dir).expect("create fleet dir");
        run_git(&dir, &["init"]);
        run_git(&dir, &["config", "user.email", "test@example.com"]);
        run_git(&dir, &["config", "user.name", "Test"]);
        std::fs::write(dir.join("README.md"), "readme").expect("write readme");
        run_git(&dir, &["add", "."]);
        run_git(&dir, &["commit", "-m", "init"]);
        dir
    }
}

impl Drop for IsolatedHome {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn run_git(dir: &Path, args: &[&str]) {
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

/// Short unique MSB_HOME for KVM tests that BOOT a sandbox. microsandbox
/// derives its agent-relay socket as `$MSB_HOME/run/agent/<32hex>.sock` =
/// len+48 bytes, capped at the 108-byte Linux `sockaddr_un` limit, so
/// `len(MSB_HOME)` must be `<= 59`. The usual
/// `workestrate-<label>-<pid>-<nanos>` temp dir under a deep `$TMPDIR` blows
/// that budget. These tests are Linux-only KVM-gated, so a fixed short
/// `/tmp` base is honest and decouples the length from `$TMPDIR`. `pid+nanos`
/// matches the codebase uniq idiom and is collision-free across parallel
/// tests in one binary. The caller owns cleanup (RAII `Drop` ->
/// `remove_dir_all`).
pub fn short_msb_home() -> PathBuf {
    let p = PathBuf::from(format!(
        "/tmp/wk-msb-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    debug_assert!(
        p.to_string_lossy().len() + 48 < 108,
        "MSB_HOME too long for the 108-byte unix-socket budget: {} bytes",
        p.to_string_lossy().len()
    );
    p
}
