//! The generated `workestrate.lock` (ADR 0025(e), spec 11 §3).
//!
//! The registry (`config.toml`) stays the HUMAN-EDITED file; the lock is
//! GENERATED — never hand-edited — and pins each config repo's
//! `url`/`ref`/`rev` so a reproduced home is what the source home actually
//! ran, not "whatever main is today". The lock is committed to the home repo
//! (lockfiles exist to be shared, like `Cargo.lock`); it is NOT in the home
//! `.gitignore`.
//!
//! Evolution rules: an absent `version` means the oldest format; a version
//! newer than [`LOCK_VERSION`] is a hard error ("home created by a newer
//! workestrate"); format changes go through explicit migrations (the
//! `migrate-home` precedent, ADR 0023).
//!
//! Writers: `cmd_config_add`, `cmd_config_update`, `cmd_config_remove`,
//! `cmd_home_init` (bare + `--from`). Consumers: `--from` provisioning and
//! the future `up --pin` / spawn-provenance work (the lock is THEIR
//! mechanism; do not build a second one).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config::types::Registry;

/// Newest lockfile version this binary can read/write.
pub const LOCK_VERSION: u32 = 1;

/// Name of the generated lockfile in the home root.
pub const LOCK_FILE_NAME: &str = "workestrate.lock";

/// Serde default for [`HomeLock::version`]: an absent `version` field means
/// the OLDEST supported format (currently 1).
fn default_lock_version() -> u32 {
    1
}

/// The generated home lockfile (`workestrate.lock`). `version` is the
/// lockfile format version; `home_version` mirrors the registry's
/// `settings.home_version` at write time; `tool_version` is the writing
/// binary's `CARGO_PKG_VERSION`; `repos` pins every registered config repo.
/// Unknown fields are rejected so a hand-edited/future key is a loud parse
/// error, not silent data loss (the registry's `deny_unknown_fields`
/// precedent).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HomeLock {
    #[serde(default = "default_lock_version")]
    pub version: u32,
    pub home_version: u32,
    pub tool_version: String,
    #[serde(default)]
    pub repos: BTreeMap<String, LockedRepo>,
}

/// One pinned config repo (`[repos.<name>]`). `rev` is `None` for local-path
/// repos (registered via `config new` — their registry shape has no rev
/// either).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedRepo {
    pub url: String,
    pub r#ref: Option<String>,
    pub rev: Option<String>,
}

/// The lockfile path for an EXPLICIT home dir: `<home>/workestrate.lock`.
pub fn home_lock_path_for(home: &Path) -> PathBuf {
    home.join(LOCK_FILE_NAME)
}

/// The lockfile path for the RESOLVED home: next to the resolved registry
/// (`config.toml`'s directory). Resolving via the registry path (rather than
/// `resolve_home()`) automatically does the right thing for legacy XDG
/// homes, whose registry lives at `$XDG_CONFIG_HOME/workestrate/config.toml`.
pub fn home_lock_path() -> PathBuf {
    crate::config::paths::registry_path().with_file_name(LOCK_FILE_NAME)
}

/// Load the lockfile for an EXPLICIT home dir. Returns `Ok(None)` when the
/// file does not exist (legacy homes tolerated — a home without a lock is
/// not corrupt, it just predates the lock); read/parse failures are hard
/// errors. A `version` newer than [`LOCK_VERSION`] is a hard error naming
/// both versions and containing the exact phrase "home created by a newer
/// workestrate" (the `migrate-home` / ADR 0025 compat-check phrase).
pub fn load_home_lock_from(home: &Path) -> Result<Option<HomeLock>> {
    load_home_lock_at(&home_lock_path_for(home))
}

/// Load the lockfile for the RESOLVED home (see [`home_lock_path`]).
pub fn load_home_lock() -> Result<Option<HomeLock>> {
    load_home_lock_at(&home_lock_path())
}

/// Shared loader: `Ok(None)` when `path` does not exist; hard error on
/// read/parse failure and on a newer-than-supported `version`.
fn load_home_lock_at(path: &Path) -> Result<Option<HomeLock>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read home lock {}: {}", path.display(), e))?;
    let lock: HomeLock = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("failed to parse home lock {}: {}", path.display(), e))?;
    if lock.version > LOCK_VERSION {
        anyhow::bail!(
            "home created by a newer workestrate (lock version {} > {} supported by this binary); \
             upgrade this workestrate before using the home at {}",
            lock.version,
            LOCK_VERSION,
            path.display()
        );
    }
    Ok(Some(lock))
}

/// Persist `lock` into an EXPLICIT home dir, atomically (mirrors
/// `save_registry`: serialize to a sibling `.tmp` on the same filesystem,
/// then rename; create the parent dir first). A crash mid-write can only
/// corrupt the tmp file — the last good lock stays intact under the original
/// name; readers never observe a truncated file.
pub fn save_home_lock_to(home: &Path, lock: &HomeLock) -> Result<()> {
    save_home_lock_at(&home_lock_path_for(home), lock)
}

/// Persist `lock` into the RESOLVED home (see [`home_lock_path`]), atomically.
pub fn save_home_lock(lock: &HomeLock) -> Result<()> {
    save_home_lock_at(&home_lock_path(), lock)
}

/// Shared atomic writer behind [`save_home_lock_to`] / [`save_home_lock`].
fn save_home_lock_at(path: &Path, lock: &HomeLock) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(lock)
        .map_err(|e| anyhow::anyhow!("failed to serialize home lock: {}", e))?;
    // Atomic write: serialize to `<home>/workestrate.lock.tmp` on the SAME
    // filesystem, then rename(2) over the target (the save_registry FN-5
    // pattern). A stale tmp from a crashed writer is consumed (overwritten +
    // renamed) by the next save.
    let tmp = path.with_file_name(format!("{}.tmp", LOCK_FILE_NAME));
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Build a FRESH lock from a registry plus the ACTUAL checked-out revs under
/// `home`: for each `[configs.<name>]`, rev = `git rev-parse HEAD` of
/// `<home>/config-repos/<name>` when that checkout exists; otherwise fall
/// back to the registry-recorded rev (a registered-but-not-cloned repo still
/// gets its last-known pin, and local-path repos keep rev=None — that is
/// their registry shape too). `home_version` defaults to 2 (the ADR 0023
/// single-home layout) when the registry does not record one; `tool_version`
/// is this binary's version; `version` is [`LOCK_VERSION`].
pub fn lock_from_registry(registry: &Registry, home: &Path) -> HomeLock {
    let mut repos = BTreeMap::new();
    for (name, entry) in &registry.configs {
        let checkout = home.join("config-repos").join(name);
        let rev = if checkout.join(".git").exists() {
            crate::git::git_rev_parse(&checkout)
                .ok()
                .or(entry.rev.clone())
        } else {
            entry.rev.clone()
        };
        repos.insert(
            name.clone(),
            LockedRepo {
                url: entry.url.clone(),
                r#ref: entry.r#ref.clone(),
                rev,
            },
        );
    }
    HomeLock {
        version: LOCK_VERSION,
        home_version: registry.settings.home_version.unwrap_or(2),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        repos,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_in_result
    )]
    use super::*;
    use crate::config::test_support::*;

    /// Set WORKESTRATE_HOME to a fresh temp dir (HomeKind::Env → lock at
    /// `<tmp>/workestrate.lock`) and return the dir. Caller must hold
    /// ENV_TEST_LOCK and an EnvGuard for HOME_ENV_KEYS.
    fn pin_home(label: &str) -> PathBuf {
        let home = uniq_dir(label);
        std::fs::create_dir_all(&home).expect("create pinned home");
        std::env::set_var("WORKESTRATE_HOME", &home);
        home
    }

    fn sample_lock() -> HomeLock {
        let mut repos = BTreeMap::new();
        repos.insert(
            "personal".to_string(),
            LockedRepo {
                url: "https://example.invalid/personal.git".to_string(),
                r#ref: Some("main".to_string()),
                rev: Some("abc123".to_string()),
            },
        );
        repos.insert(
            "work".to_string(),
            LockedRepo {
                url: "https://example.invalid/work.git".to_string(),
                r#ref: Some("main".to_string()),
                rev: Some("def456".to_string()),
            },
        );
        HomeLock {
            version: LOCK_VERSION,
            home_version: 2,
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            repos,
        }
    }

    #[test]
    fn lock_round_trip_save_then_load_equals() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("lock-roundtrip");

        let lock = sample_lock();
        save_home_lock(&lock)?;
        let loaded = load_home_lock()?.expect("lock should exist after save");
        assert_eq!(loaded, lock);
        // And via the explicit-home variants.
        let loaded_from = load_home_lock_from(&home)?.expect("lock via explicit home");
        assert_eq!(loaded_from, lock);
        assert_eq!(home_lock_path(), home_lock_path_for(&home));

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn save_is_atomic_no_tmp_survives_and_stale_tmp_is_consumed() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("lock-atomic");

        save_home_lock(&sample_lock())?;
        let tmp = home_lock_path().with_file_name(format!("{}.tmp", LOCK_FILE_NAME));
        assert!(!tmp.exists(), "tmp file must not survive the rename");

        // A stale garbage tmp from a crashed writer neither clobbers the
        // committed lock nor survives the next save.
        std::fs::write(&tmp, "garbage-partial-write")?;
        let before = std::fs::read_to_string(home_lock_path())?;
        let mut next = sample_lock();
        next.home_version = 1;
        save_home_lock(&next)?;
        assert!(!tmp.exists(), "save must consume (rename) the tmp file");
        let loaded = load_home_lock()?.expect("lock parses after save");
        assert_eq!(loaded.home_version, 1);
        assert_ne!(
            std::fs::read_to_string(home_lock_path())?,
            before,
            "the new save must replace the old lock"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn absent_lock_is_tolerated_as_none() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("lock-absent");

        assert_eq!(load_home_lock()?, None);
        assert_eq!(load_home_lock_from(&home)?, None);

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn absent_version_field_loads_as_oldest() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("lock-oldest");
        std::fs::write(
            home_lock_path(),
            "home_version = 2\ntool_version = \"0.1.0\"\n",
        )?;

        let lock = load_home_lock()?.expect("lock without a version parses");
        assert_eq!(lock.version, 1, "absent version = oldest supported");
        assert!(lock.repos.is_empty(), "absent repos = empty map");

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn newer_version_is_a_hard_error_naming_both_versions() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("lock-newer");
        std::fs::write(
            home_lock_path(),
            "version = 99\nhome_version = 2\ntool_version = \"0.1.0\"\n",
        )?;

        let err = load_home_lock().expect_err("version 99 must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("home created by a newer workestrate"),
            "error must contain the exact phrase: {msg}"
        );
        assert!(
            msg.contains("99"),
            "error must name the lock version: {msg}"
        );
        assert!(
            msg.contains(&LOCK_VERSION.to_string()),
            "error must name the supported version: {msg}"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn unknown_top_level_and_nested_keys_are_rejected() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("lock-unknown");

        // Unknown TOP-LEVEL key.
        std::fs::write(
            home_lock_path(),
            "home_version = 2\ntool_version = \"0.1.0\"\nbogus = 1\n",
        )?;
        assert!(
            load_home_lock().is_err(),
            "an unknown top-level key must fail to parse"
        );

        // Unknown key INSIDE [repos.x].
        std::fs::write(
            home_lock_path(),
            "home_version = 2\ntool_version = \"0.1.0\"\n\n[repos.x]\nurl = \"u\"\nbogus = 1\n",
        )?;
        assert!(
            load_home_lock().is_err(),
            "an unknown key inside [repos.x] must fail to parse"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn lock_from_registry_uses_checkout_rev_then_registry_rev() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("lock-from-registry");

        // One entry with a real checkout (rev comes from git), one without
        // (rev falls back to the registry-recorded pin), one local-path
        // (rev stays None).
        let checkout = home.join("config-repos").join("cloned");
        std::fs::create_dir_all(&checkout)?;
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&checkout)
                .args(args)
                .env("GIT_CONFIG_NOSYSTEM", "1")
                .status()
                .expect("spawn git");
            assert!(status.success(), "git {:?} failed", args);
        };
        run(&["init", "--quiet", "-b", "main"]);
        run(&["config", "user.email", "lock@test.invalid"]);
        run(&["config", "user.name", "lock-test"]);
        std::fs::write(checkout.join("f.txt"), "x")?;
        run(&["add", "f.txt"]);
        run(&["commit", "--quiet", "-m", "init"]);
        let head = crate::git::git_rev_parse(&checkout)?;

        let mut registry = Registry::default();
        let entry =
            |url: &str, git_ref: Option<&str>, rev: Option<&str>| crate::config::ConfigRepoEntry {
                url: url.to_string(),
                r#ref: git_ref.map(|s| s.to_string()),
                rev: rev.map(|s| s.to_string()),
                secrets: None,
                secrets_file: None,
                age_key_file: None,
            };
        registry.configs.insert(
            "cloned".to_string(),
            entry(
                "https://example.invalid/cloned.git",
                Some("main"),
                Some("stale"),
            ),
        );
        registry.configs.insert(
            "recorded-only".to_string(),
            entry(
                "https://example.invalid/rec.git",
                Some("main"),
                Some("rec123"),
            ),
        );
        registry
            .configs
            .insert("local".to_string(), entry("/some/local/path", None, None));

        let lock = lock_from_registry(&registry, &home);
        assert_eq!(lock.version, LOCK_VERSION);
        assert_eq!(lock.home_version, 2, "absent home_version defaults to 2");
        assert!(!lock.tool_version.is_empty());
        assert_eq!(
            lock.repos["cloned"].rev.as_deref(),
            Some(head.as_str()),
            "an existing checkout's ACTUAL HEAD wins over the stale registry rev"
        );
        assert_eq!(
            lock.repos["recorded-only"].rev.as_deref(),
            Some("rec123"),
            "no checkout → fall back to the registry-recorded rev"
        );
        assert_eq!(
            lock.repos["local"].rev, None,
            "local-path repos keep rev=None"
        );
        assert_eq!(
            lock.repos["cloned"].url,
            "https://example.invalid/cloned.git"
        );
        assert_eq!(lock.repos["cloned"].r#ref.as_deref(), Some("main"));

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }
}
