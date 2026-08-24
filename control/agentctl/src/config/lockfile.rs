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
//! `cmd_home_init` (scaffold) and `cmd_home_clone` — plus the A5
//! FIRST-RESOLUTION-WITH-NOTICE paths in `config::loading` (consumption
//! verbs that write ONLY when no pin exists yet, and announce the write on
//! stderr; see `layer_content_root`): Session 2 writes the PRIMARY pin via
//! [`upsert_locked_pin`], Session 3a writes `--config-ref` override pins
//! into the refs map via [`upsert_locked_ref`] (the primary pin is never
//! moved by an override resolution). No other verb may write the lock — no
//! verb mutates pins silently as a side effect. Consumers: pinned config
//! consumption (`config::loading`), `home clone` provisioning and
//! the future `up --pin` / spawn-provenance work (the lock is THEIR
//! mechanism; do not build a second one).
//!
//! v2 (A5, ADR 0032 addendum 2026-08-24 §Config source model): adds the
//! ref-aware fields — per-repo `sha`/`fetched_at` and the per-ref
//! `[repos.<name>.refs.<ref>]` map ([`LockedRef`]). Read-side evolution is
//! pure serde defaults: a v1 file (version absent or 1) loads with
//! `sha`/`fetched_at` = None and an empty `refs` map and is NOT
//! version-bumped on read; writers always write `version = 2`. A v2 file
//! read by an OLD binary hard-errors via the version check — that posture
//! is BY DESIGN (the same fail-closed stance as every other versioned
//! file). This is the ONE lock story: no `config-repos.lock`, no second
//! lockfile.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config::types::Registry;

/// Newest lockfile version this binary can read/write. v2 = the A5
/// ref-aware shape ([`LockedRepo::sha`], [`LockedRepo::fetched_at`],
/// [`LockedRepo::refs`]); v1 files load via serde defaults.
pub const LOCK_VERSION: u32 = 2;

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
///
/// A5/v2 fields: `sha` is the commit sha the content archive was produced
/// from — it equals `rev` for git-backed entries today, and exists as a
/// SEPARATE field so the pin (`rev`) is never conflated with the content
/// key (`sha`): future tree-hash keying changes the content key without
/// migrating the pin. `fetched_at` is the RFC3339 (UTC) time the pin was
/// resolved (sourced from
/// `crate::microsandbox::runtime::time::current_rfc3339_utc` when
/// populated). Plain-path entries keep `rev`/`sha`/`fetched_at` = None
/// (content-as-is, branch `"local"`). `refs` pins additional refs of the
/// same repo (the A5 selection ladder consumes them); the key is omitted
/// from serialization when empty so a no-refs v2 lock stays byte-clean of
/// it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedRepo {
    pub url: String,
    pub r#ref: Option<String>,
    pub rev: Option<String>,
    #[serde(default)]
    pub sha: Option<String>,
    #[serde(default)]
    pub fetched_at: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub refs: BTreeMap<String, LockedRef>,
}

/// One pinned ref of a config repo (`[repos.<name>.refs.<ref>]`, lock v2).
/// `rev` is the pin as written (branch, tag, or sha); `sha` is the commit
/// sha the archive for this ref was produced from (the content key — see
/// [`LockedRepo::sha`]); `fetched_at` is the RFC3339 (UTC) resolution time.
/// Not schema-surfaced: the lock is a generated artifact, deliberately
/// outside the schemars projection (verify against
/// `diagnostics::generate_schema_pair` before ever adding schemars here).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedRef {
    pub rev: String,
    pub sha: String,
    pub fetched_at: String,
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
///
/// A5 Session 2: this stays the BOOTSTRAP writer (home init/clone, and the
/// no-prior-lock fallback in `cmd_config_update`) and deliberately leaves
/// `sha`/`fetched_at` empty — the EXPLICIT pin writers (`config add`,
/// `config update`, first-resolution-with-notice) populate those via
/// [`upsert_locked_pin`], which is also why `cmd_config_update` must update
/// a loaded lock IN PLACE rather than rebuilding via this function (a
/// rebuild would clobber the archive-aware fields back to None).
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
                // Bootstrap writer: sha/fetched_at stay empty here (see the
                // fn doc); the explicit pin writers populate them via
                // upsert_locked_pin.
                sha: None,
                fetched_at: None,
                refs: BTreeMap::new(),
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

/// Upsert the PRIMARY pin for `name` in `lock` (A5 Session 2; ADR 0032
/// addendum §Config source model). This is THE shared write path of the
/// explicit lock writers (`config add`, `config update`, and the
/// first-resolution-with-notice path in `config::loading`):
///
/// - `rev` = `sha` = the resolved commit — the pin IS the commit; `sha`
///   additionally keys the content archive (`<state>/cache/gitv3/<sha>/`).
/// - `fetched_at` is stamped with
///   `crate::microsandbox::runtime::time::current_rfc3339_utc`.
/// - An existing `refs` map (pins for NON-primary refs, written by the
///   `--config-ref` override path via [`upsert_locked_ref`]) is PRESERVED.
///
/// Callers own `lock.version`/`tool_version` stamping and the atomic save.
pub fn upsert_locked_pin(
    lock: &mut HomeLock,
    name: &str,
    url: &str,
    git_ref: Option<&str>,
    sha: &str,
) {
    let refs = lock
        .repos
        .get(name)
        .map(|existing| existing.refs.clone())
        .unwrap_or_default();
    lock.repos.insert(
        name.to_string(),
        LockedRepo {
            url: url.to_string(),
            r#ref: git_ref.map(|s| s.to_string()),
            rev: Some(sha.to_string()),
            sha: Some(sha.to_string()),
            fetched_at: Some(crate::microsandbox::runtime::time::current_rfc3339_utc()),
            refs,
        },
    );
}

/// Upsert a NON-PRIMARY ref pin into `repos.<name>.refs.<ref>` (A5 Session
/// 3a; ADR 0032 addendum §Selection ladder). This is THE write path of the
/// `--config-ref` consumption override (`config::loading`): the resolved ref
/// rides the refs map KEYED BY THE REF ITSELF (a sha-shaped ref is legal and
/// keyed by itself) as `{rev = <ref>, sha = <resolved commit>, fetched_at}`.
///
/// The PRIMARY pin (`rev`/`sha`/`fetched_at` at `[repos.<name>]`) is NEVER
/// moved by this writer — only [`upsert_locked_pin`] moves it; an existing
/// entry's primary fields pass through untouched. When the repo has no lock
/// entry yet, one is created carrying the registry's `url`/`ref` with EMPTY
/// primary pin fields (`rev`/`sha`/`fetched_at` = None) — a home whose ONLY
/// resolutions were `--config-ref` overrides still has no primary pin.
///
/// Callers own `lock.version`/`tool_version` stamping and the atomic save.
pub fn upsert_locked_ref(
    lock: &mut HomeLock,
    name: &str,
    url: &str,
    git_ref: Option<&str>,
    ref_name: &str,
    sha: &str,
) {
    let entry = lock
        .repos
        .entry(name.to_string())
        .or_insert_with(|| LockedRepo {
            url: url.to_string(),
            r#ref: git_ref.map(|s| s.to_string()),
            rev: None,
            sha: None,
            fetched_at: None,
            refs: BTreeMap::new(),
        });
    entry.refs.insert(
        ref_name.to_string(),
        LockedRef {
            rev: ref_name.to_string(),
            sha: sha.to_string(),
            fetched_at: crate::microsandbox::runtime::time::current_rfc3339_utc(),
        },
    );
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
                sha: None,
                fetched_at: None,
                refs: BTreeMap::new(),
            },
        );
        repos.insert(
            "work".to_string(),
            LockedRepo {
                url: "https://example.invalid/work.git".to_string(),
                r#ref: Some("main".to_string()),
                rev: Some("def456".to_string()),
                sha: None,
                fetched_at: None,
                refs: BTreeMap::new(),
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
        // A5 Session 1: the v2 fields are threaded empty until Session 2
        // wires the archive-aware writer.
        assert_eq!(lock.repos["cloned"].sha, None);
        assert_eq!(lock.repos["cloned"].fetched_at, None);
        assert!(lock.repos["cloned"].refs.is_empty());

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    // ---- A5 / lock v2: ref-aware fields ----

    /// A v1 lock (old shape: version absent or 1, no sha/fetched_at/refs
    /// keys) parses with the v2 fields DEFAULTED — and loading does NOT
    /// bump the in-memory version (writers, not readers, own the bump).
    #[test]
    fn v1_lock_parses_with_defaulted_v2_fields_and_is_not_bumped() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("lock-v1-read");

        // version absent (= oldest) and an old-shape [repos.x] entry.
        std::fs::write(
            home_lock_path(),
            "home_version = 2\ntool_version = \"0.1.0\"\n\n[repos.work]\nurl = \"https://example.invalid/work.git\"\nref = \"main\"\nrev = \"abc123\"\n",
        )?;

        let lock = load_home_lock()?.expect("v1 lock must parse");
        assert_eq!(lock.version, 1, "reading must NOT bump the version");
        let repo = &lock.repos["work"];
        assert_eq!(repo.rev.as_deref(), Some("abc123"));
        assert_eq!(repo.sha, None, "v1 entry has no sha");
        assert_eq!(repo.fetched_at, None, "v1 entry has no fetched_at");
        assert!(repo.refs.is_empty(), "v1 entry has an empty refs map");

        // Explicit version = 1 parses the same way.
        std::fs::write(
            home_lock_path(),
            "version = 1\nhome_version = 2\ntool_version = \"0.1.0\"\n\n[repos.work]\nurl = \"u\"\n",
        )?;
        let lock = load_home_lock()?.expect("explicit v1 lock must parse");
        assert_eq!(lock.version, 1);
        assert_eq!(lock.repos["work"].sha, None);

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// v2 round-trip: sha/fetched_at and the refs map survive save+load.
    #[test]
    fn v2_round_trip_includes_sha_fetched_at_and_refs() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("lock-v2-roundtrip");

        let mut lock = sample_lock();
        lock.repos.get_mut("personal").unwrap().sha = Some("abc123".to_string());
        lock.repos.get_mut("personal").unwrap().fetched_at =
            Some("2026-08-24T10:00:00Z".to_string());
        let mut refs = BTreeMap::new();
        refs.insert(
            "feat-x".to_string(),
            LockedRef {
                rev: "feat-x".to_string(),
                sha: "0123456789abcdef0123456789abcdef01234567".to_string(),
                fetched_at: "2026-08-24T10:01:00Z".to_string(),
            },
        );
        lock.repos.get_mut("work").unwrap().refs = refs;

        save_home_lock(&lock)?;
        let content = std::fs::read_to_string(home_lock_path())?;
        assert!(
            content.contains("version = 2"),
            "writers must stamp version = 2:\n{content}"
        );
        assert!(
            content.contains("[repos.work.refs.feat-x]"),
            "the refs map must serialize as [repos.<name>.refs.<ref>]:\n{content}"
        );
        let loaded = load_home_lock()?.expect("v2 lock parses");
        assert_eq!(loaded, lock);

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Version 3 (one past the newest supported) hard-errors with the exact
    /// compat phrase — the BY-DESIGN posture: a v2 file read by an old
    /// (v1-only) binary fails the same way.
    #[test]
    fn version_3_is_a_hard_error_naming_the_phrase() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("lock-v3");

        std::fs::write(
            home_lock_path(),
            "version = 3\nhome_version = 2\ntool_version = \"0.1.0\"\n",
        )?;
        let err = load_home_lock().expect_err("version 3 must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("home created by a newer workestrate"),
            "error must contain the exact phrase: {msg}"
        );
        assert!(msg.contains('3'), "error must name the lock version: {msg}");

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// `skip_serializing_if` keeps a no-refs lock byte-clean of the `refs`
    /// key (a v2 lock with no extra pins is a v1-looking file plus the
    /// version stamp).
    #[test]
    fn no_refs_lock_serializes_without_the_refs_key() {
        let content = toml::to_string_pretty(&sample_lock()).expect("serialize sample lock");
        assert!(
            !content.contains("refs"),
            "an empty refs map must not appear in the serialized lock:\n{content}"
        );
    }

    /// An unknown key inside `[repos.x.refs.<ref>]` is a loud parse error
    /// (deny_unknown_fields on LockedRef, same posture as the rest of the
    /// lock).
    #[test]
    fn unknown_key_inside_refs_is_rejected() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("lock-refs-unknown");

        std::fs::write(
            home_lock_path(),
            "version = 2\nhome_version = 2\ntool_version = \"0.1.0\"\n\n[repos.x]\nurl = \"u\"\n\n[repos.x.refs.main]\nrev = \"main\"\nsha = \"0123456789abcdef0123456789abcdef01234567\"\nfetched_at = \"2026-08-24T10:00:00Z\"\nbogus = 1\n",
        )?;
        assert!(
            load_home_lock().is_err(),
            "an unknown key inside [repos.x.refs.main] must fail to parse"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    // ---- A5 Session 2: upsert_locked_pin (the explicit pin write path) ----

    #[test]
    fn upsert_locked_pin_stamps_sha_and_fetched_at_and_preserves_refs() {
        let mut lock = sample_lock();
        // Pre-existing NON-primary ref pin must survive an upsert of the
        // primary pin.
        lock.repos.get_mut("work").unwrap().refs.insert(
            "feat-x".to_string(),
            LockedRef {
                rev: "feat-x".to_string(),
                sha: "0123456789abcdef0123456789abcdef01234567".to_string(),
                fetched_at: "2026-08-24T10:01:00Z".to_string(),
            },
        );

        upsert_locked_pin(
            &mut lock,
            "work",
            "https://example.invalid/work.git",
            Some("main"),
            "aaaaaaa1111111bbbbbbb2222222ccccccc3333333",
        );

        let repo = &lock.repos["work"];
        assert_eq!(
            repo.rev.as_deref(),
            Some("aaaaaaa1111111bbbbbbb2222222ccccccc3333333"),
            "rev = the resolved commit"
        );
        assert_eq!(
            repo.sha.as_deref(),
            Some("aaaaaaa1111111bbbbbbb2222222ccccccc3333333"),
            "sha = rev (the pin IS the commit)"
        );
        let fetched_at = repo.fetched_at.as_deref().expect("fetched_at stamped");
        assert!(
            fetched_at.ends_with('Z') && fetched_at.contains('T'),
            "fetched_at must be RFC3339 UTC: {fetched_at}"
        );
        assert_eq!(repo.r#ref.as_deref(), Some("main"));
        assert!(
            lock.repos["work"].refs.contains_key("feat-x"),
            "an existing refs map must be preserved across a primary-pin upsert"
        );

        // A fresh entry (name not present) gets an empty refs map.
        upsert_locked_pin(
            &mut lock,
            "new",
            "https://example.invalid/n.git",
            None,
            "0123abc",
        );
        let new = &lock.repos["new"];
        assert_eq!(new.sha.as_deref(), Some("0123abc"));
        assert_eq!(new.r#ref, None);
        assert!(new.refs.is_empty());
    }

    // ---- A5 Session 3a: upsert_locked_ref (the --config-ref write path) ----

    #[test]
    fn upsert_locked_ref_pins_the_ref_without_moving_the_primary_pin() {
        let mut lock = sample_lock();
        // "work" starts with the sample primary pin (rev = def456).
        upsert_locked_ref(
            &mut lock,
            "work",
            "https://example.invalid/work.git",
            Some("main"),
            "feat-x",
            "0123456789abcdef0123456789abcdef01234567",
        );
        let repo = &lock.repos["work"];
        // The PRIMARY pin is untouched.
        assert_eq!(repo.rev.as_deref(), Some("def456"));
        assert_eq!(repo.sha, None, "the sample primary sha stays as it was");
        assert_eq!(repo.fetched_at, None);
        // The refs map carries the new pin keyed by the ref itself.
        let pin = &repo.refs["feat-x"];
        assert_eq!(pin.rev, "feat-x");
        assert_eq!(
            pin.sha, "0123456789abcdef0123456789abcdef01234567",
            "sha = the resolved commit (the archive content key)"
        );
        assert!(
            pin.fetched_at.ends_with('Z') && pin.fetched_at.contains('T'),
            "fetched_at must be RFC3339 UTC: {}",
            pin.fetched_at
        );

        // A sha-shaped ref is legal and keyed by itself.
        let sha_ref = "aaaaaaa1111111bbbbbbb2222222ccccccc3333333";
        upsert_locked_ref(
            &mut lock,
            "work",
            "https://example.invalid/work.git",
            Some("main"),
            sha_ref,
            sha_ref,
        );
        assert_eq!(lock.repos["work"].refs[sha_ref].rev, sha_ref);

        // A repo with NO lock entry yet gets one with EMPTY primary fields —
        // a --config-ref-only history never manufactures a primary pin.
        upsert_locked_ref(
            &mut lock,
            "fresh",
            "https://example.invalid/f.git",
            Some("main"),
            "feat-y",
            "bbbbbbb",
        );
        let fresh = &lock.repos["fresh"];
        assert_eq!(fresh.url, "https://example.invalid/f.git");
        assert_eq!(fresh.r#ref.as_deref(), Some("main"));
        assert_eq!(fresh.rev, None, "no primary pin is fabricated");
        assert_eq!(fresh.sha, None);
        assert_eq!(fresh.fetched_at, None);
        assert_eq!(fresh.refs["feat-y"].sha, "bbbbbbb");
    }
}
