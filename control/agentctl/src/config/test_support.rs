//! Shared test-support helpers extracted from `config::tests` and its
//! siblings (`main::tests`, `microsandbox::*::tests`).
//!
//! Everything here is test-support only. The module used to be declared
//! `#[cfg(test)]` in `config.rs`; with the lib+bin split it is compiled into
//! the library unconditionally (no `#[cfg(test)]` gate) so the BINARY crate's
//! own test harness (`main.rs::tests`) can also reach it — a bin's tests link
//! the lib WITHOUT `cfg(test)`. The crate is `publish = false`, so this is
//! acceptable. Items are `pub` so every consuming test module can share ONE
//! canonical copy of each helper (previously `TestConfigGuard` existed in
//! three identical copies, and two near-duplicates of `uniq_dir` existed).

use std::path::{Path, PathBuf};

/// Global lock for tests that mutate process env vars.
///
/// Cargo runs unit tests in parallel by default, and tests that set
/// `WORKESTRATE_FLEET_DIR` or similar env vars would otherwise race.
pub static ENV_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Global lock for tests that mutate the process-global provenance stores
/// (`merge::MERGED_PROVENANCE`, `merge::SECRET_PROVENANCE`,
/// `merge::LAYER_DIRS`) directly — set/take around assertions on the slot
/// contents. Serializes the DIRECT mutators across test modules (the stores
/// are process-global, so a concurrent `take_provenance()` from another
/// module's test can steal a value mid-test). Tests that only mutate them
/// INDIRECTLY (via `load_config` / `ConfigWorkload::new`) are not serialized
/// by this lock — see `merge::tests::
/// provenance_survives_tokio_multi_thread_migration` for the residual-risk
/// note.
pub static PROVENANCE_STORAGE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// RAII guard that points `WORKESTRATE_FLEET_DIR` at the committed test
/// fixture (a copy of the pre-strip-down 5-workload config) and restores the
/// previous state on drop. Holds a global lock so env-var tests do not race
/// when Cargo runs them in parallel.
pub struct TestConfigGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl TestConfigGuard {
    #[allow(unsafe_code)]
    pub fn new() -> Self {
        let lock = ENV_TEST_LOCK.lock().unwrap();
        let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("config");
        // SAFETY: holds the ENV_TEST_LOCK mutex guard for its lifetime,
        // serializing env mutation across parallel tests.
        unsafe { std::env::set_var("WORKESTRATE_FLEET_DIR", fixture) };
        Self { _lock: lock }
    }
}

impl Drop for TestConfigGuard {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: the ENV_TEST_LOCK guard is still held during Drop (field
        // drop order runs after this fn body), so env mutation stays
        // serialized.
        unsafe { std::env::remove_var("WORKESTRATE_FLEET_DIR") };
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
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        for (k, v) in &self.vars {
            // SAFETY: restores caller-snapshotted values; test callers hold
            // ENV_TEST_LOCK; the runtime caller (generation sweep in
            // lifecycle.rs) pins/restores MSB_HOME sequentially.
            unsafe {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
        }
        if let Some(cwd) = &self.cwd {
            let _ = std::env::set_current_dir(cwd);
        }
    }
}

pub const CONFIG_ENV_KEYS: &[&str] = &[
    "WORKESTRATE_CONFIG",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_STATE_HOME",
    "WORKESTRATE_FLEET_DIR",
    "WORKESTRATE_NO_PROJECT_CONFIG",
    "WORKESTRATE_INVOKE_CWD",
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
         [workloads.{name}.network.defaults]\n\
         egress = \"deny\"\n"
    )
}

pub const MINIMAL_VALID_TOML: &str = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.network.defaults]\negress = \"deny\"";

/// Minimal valid config with one workload; the caller mutates it per test.
pub fn base_config_for_validation() -> crate::config::ConfigFile {
    let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []
log_stop_errors = false

[workloads.pi.network.defaults]
egress = "deny"
"#;
    toml::from_str(toml).expect("base config must parse")
}
