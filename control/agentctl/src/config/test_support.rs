//! Shared test-support helpers extracted from `config::tests` and its
//! siblings (`main::tests`, `microsandbox::*::tests`).
//!
//! Everything here is test-only: the module is declared `#[cfg(test)]` at the
//! declaration site in `config.rs`, so none of this lands in the shipped
//! binary. Items are `pub` so every consuming test module can share ONE
//! canonical copy of each helper (previously `TestConfigGuard` existed in
//! three identical copies, and two near-duplicates of `uniq_dir` existed).

use std::path::{Path, PathBuf};

/// Global lock for tests that mutate process env vars.
///
/// Cargo runs unit tests in parallel by default, and tests that set
/// `WORKESTRATE_CONFIG_DIR` or similar env vars would otherwise race.
pub static ENV_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// RAII guard that points `WORKESTRATE_CONFIG_DIR` at the committed test
/// fixture (a copy of the pre-strip-down 5-workload config) and restores the
/// previous state on drop. Holds a global lock so env-var tests do not race
/// when Cargo runs them in parallel.
pub struct TestConfigGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl TestConfigGuard {
    pub fn new() -> Self {
        let lock = ENV_TEST_LOCK.lock().unwrap();
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("config");
        std::env::set_var("WORKESTRATE_CONFIG_DIR", fixture);
        Self { _lock: lock }
    }
}

impl Drop for TestConfigGuard {
    fn drop(&mut self) {
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");
    }
}

/// Snapshot env vars (+ cwd) and restore them on drop, even on panic.
/// Mirrors the manual save/restore in older tests but panic-safe.
pub struct EnvGuard {
    vars: Vec<(&'static str, Option<String>)>,
    cwd: Option<PathBuf>,
}
impl EnvGuard {
    pub fn capture(keys: &'static [&'static str]) -> Self {
        let vars = keys.iter().map(|&k| (k, std::env::var(k).ok())).collect();
        EnvGuard {
            vars,
            cwd: std::env::current_dir().ok(),
        }
    }
}
impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (k, v) in &self.vars {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
        if let Some(cwd) = &self.cwd {
            let _ = std::env::set_current_dir(cwd);
        }
    }
}

pub const HOME_ENV_KEYS: &[&str] = &[
    "WORKESTRATE_HOME",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_STATE_HOME",
    "WORKESTRATE_CONFIG_DIR",
    "WORKESTRATE_NO_PROJECT_CONFIG",
    "HOME",
];

pub fn uniq_dir(label: &str) -> PathBuf {
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

/// Canonical unique temp-dir builder for the `port_registry` test family.
/// Delegates to `uniq_dir`; the temp-dir NAME prefix is not behaviorally
/// asserted anywhere, so a single canonical shape is used.
pub fn unique_state_dir(label: &str) -> PathBuf {
    uniq_dir(label)
}

/// Canonical unique temp-dir builder for the `runtime` test family.
/// Delegates to `uniq_dir`; the temp-dir NAME prefix is not behaviorally
/// asserted anywhere, so a single canonical shape is used.
pub fn unique_state_dir_runtime(label: &str) -> PathBuf {
    uniq_dir(label)
}

pub fn write_overrides(dir: &Path, content: &str) -> PathBuf {
    let path = dir.join("overrides.toml");
    std::fs::write(&path, content).unwrap();
    path
}

/// Helper: write a minimal registry into XDG_CONFIG_HOME that marks the
/// given project as trusted (or NOT trusted, if no paths are passed).
pub fn write_test_registry(home: &Path, trusted_paths: &[&Path]) -> std::io::Result<()> {
    let cfg_dir = home.join(".config").join("workestrate");
    std::fs::create_dir_all(&cfg_dir)?;
    let mut s = String::from("layers = []\n\n");
    for p in trusted_paths {
        s.push_str(&format!(
            "[[trusted_projects]]\npath = \"{}\"\n",
            p.display()
        ));
    }
    std::fs::write(cfg_dir.join("config.toml"), s)?;
    Ok(())
}

/// Build a minimal 1-workload ConfigFile TOML string. The workload name
/// is the discriminator for the trust-gate test.
pub fn one_workload_toml(name: &str) -> String {
    format!(
        "schema_version = 1\n\n\
         [workloads.{name}]\n\
         kind = \"agent\"\n\
         image = {{ recipe = \"registry\", ref = \"node:24\" }}\n\
         command = []\n\n\
         [workloads.{name}.network]\n\
         default_deny = true\n"
    )
}

pub const MINIMAL_VALID_TOML: &str = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.network]\ndefault_deny = true";

/// Minimal valid config with one workload; the caller mutates it per test.
pub fn base_config_for_validation() -> crate::config::ConfigFile {
    let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []
log_stop_errors = false

[workloads.pi.network]
default_deny = true
"#;
    toml::from_str(toml).expect("base config must parse")
}
