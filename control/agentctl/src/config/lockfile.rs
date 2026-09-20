//! The generated `workestrate.lock` (ADR 0025(e), spec 11 §3).
//!
//! The registry (`config.toml`) stays the HUMAN-EDITED file; the lock is
//! GENERATED — never hand-edited — and pins each fleet's
//! `url`/`ref`/`rev` so a reproduced config is what the source config actually
//! ran, not "whatever main is today". The lock is committed to the fleet
//! (lockfiles exist to be shared, like `Cargo.lock`); it is NOT in the config
//! `.gitignore`.
//!
//! Evolution rules: an absent `version` means the oldest format; a version
//! newer than [`LOCK_VERSION`] is a hard error ("config created by a newer
//! workestrate"); format changes go through explicit migrations (the
//! `migrate-config` precedent, ADR 0023).
//!
//! Writers: `cmd_fleet_add`, `cmd_fleet_update`, `cmd_fleet_remove`,
//! `cmd_config_init` (scaffold) and `cmd_config_clone` — plus the A5
//! FIRST-RESOLUTION-WITH-NOTICE paths in `config::loading` (consumption
//! verbs that write ONLY when no pin exists yet, and announce the write on
//! stderr; see `layer_content_root`): Session 2 writes the PRIMARY pin via
//! [`upsert_locked_pin`], Session 3a writes `--config-ref` override pins
//! into the refs map via [`upsert_locked_ref`] (the primary pin is never
//! moved by an override resolution). No other verb may write the lock — no
//! verb mutates pins silently as a side effect. Consumers: pinned config
//! consumption (`config::loading`), `config clone` provisioning and
//! the future `up --pin` / spawn-provenance work (the lock is THEIR
//! mechanism; do not build a second one).
//!
//! v2 (A5, ADR 0032 addendum 2026-08-24 §Config source model): adds the
//! ref-aware fields — per-fleet `sha`/`fetched_at` and the per-ref
//! `[fleets.<name>.refs.<ref>]` map ([`LockedRef`]). Read-side evolution is
//! pure serde defaults: a v1 file (version absent or 1) loads with
//! `sha`/`fetched_at` = None and an empty `refs` map and is NOT
//! version-bumped on read; writers always write `version = 2`. A v2 file
//! read by an OLD binary hard-errors via the version check — that posture
//! is BY DESIGN (the same fail-closed stance as every other versioned
//! file). This is the ONE lock story: no `fleets.lock`, no second
//! lockfile.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config::types::Registry;

/// Newest lockfile version this binary can read/write. v2 = the A5
/// ref-aware shape ([`LockedFleet::sha`], [`LockedFleet::fetched_at`],
/// [`LockedFleet::refs`]); v1 files load via serde defaults.
pub const LOCK_VERSION: u32 = 2;

/// Name of the generated lockfile in the config root.
pub const LOCK_FILE_NAME: &str = "workestrate.lock";

/// Serde default for [`ConfigLock::version`]: an absent `version` field means
/// the OLDEST supported format (currently 1).
fn default_lock_version() -> u32 {
    1
}

/// The generated config lockfile (`workestrate.lock`). `version` is the
/// lockfile format version; `config_version` mirrors the registry's
/// `settings.config_version` at write time; `tool_version` is the writing
/// binary's `CARGO_PKG_VERSION`; `fleets` pins every registered fleet.
/// Unknown fields are rejected so a hand-edited/future key is a loud parse
/// error, not silent data loss (the registry's `deny_unknown_fields`
/// precedent).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigLock {
    #[serde(default = "default_lock_version")]
    pub version: u32,
    pub config_version: u32,
    pub tool_version: String,
    #[serde(default)]
    pub fleets: BTreeMap<String, LockedFleet>,
}

/// One pinned fleet (`[fleets.<name>]`). `rev` is `None` for local-path
/// fleets (registered via `fleet new` — their registry shape has no rev
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
/// same fleet (the A5 selection ladder consumes them); the key is omitted
/// from serialization when empty so a no-refs v2 lock stays byte-clean of
/// it.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LockedFleet {
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

/// One pinned ref of a fleet (`[fleets.<name>.refs.<ref>]`, lock v2).
/// `rev` is the pin as written (branch, tag, or sha); `sha` is the commit
/// sha the archive for this ref was produced from (the content key — see
/// [`LockedFleet::sha`]); `fetched_at` is the RFC3339 (UTC) resolution time.
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

/// The lockfile path for an EXPLICIT config dir: `<config>/workestrate.lock`.
pub fn config_lock_path_for(config_dir: &Path) -> PathBuf {
    config_dir.join(LOCK_FILE_NAME)
}

/// The lockfile path for the RESOLVED config: next to the resolved registry
/// (`config.toml`'s directory). Resolving via the registry path (rather than
/// `resolve_config_dir()`) automatically does the right thing for legacy XDG
/// homes, whose registry lives at `$XDG_CONFIG_HOME/workestrate/config.toml`.
pub fn config_lock_path() -> PathBuf {
    crate::config::paths::registry_path().with_file_name(LOCK_FILE_NAME)
}

/// Load the lockfile for an EXPLICIT config dir. Returns `Ok(None)` when the
/// file does not exist (legacy configs tolerated — a config without a lock is
/// not corrupt, it just predates the lock); read/parse failures are hard
/// errors. A `version` newer than [`LOCK_VERSION`] is a hard error naming
/// both versions and containing the exact phrase "config created by a newer
/// workestrate" (the `migrate-config` / ADR 0025 compat-check phrase).
pub fn load_config_lock_from(config_dir: &Path) -> Result<Option<ConfigLock>> {
    load_config_lock_at(&config_lock_path_for(config_dir))
}

/// Load the lockfile for the RESOLVED config (see [`config_lock_path`]).
pub fn load_config_lock() -> Result<Option<ConfigLock>> {
    load_config_lock_at(&config_lock_path())
}

/// Shared loader: `Ok(None)` when `path` does not exist; hard error on
/// read/parse failure and on a newer-than-supported `version`.
fn load_config_lock_at(path: &Path) -> Result<Option<ConfigLock>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read config lock {}: {}", path.display(), e))?;
    let lock: ConfigLock = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("failed to parse config lock {}: {}", path.display(), e))?;
    if lock.version > LOCK_VERSION {
        anyhow::bail!(
            "config created by a newer workestrate (lock version {} > {} supported by this binary); \
             upgrade this workestrate before using the config at {}",
            lock.version,
            LOCK_VERSION,
            path.display()
        );
    }
    Ok(Some(lock))
}

/// Persist `lock` into an EXPLICIT config dir, atomically (mirrors
/// `save_registry`: serialize to a sibling `.tmp` on the same filesystem,
/// then rename; create the parent dir first). A crash mid-write can only
/// corrupt the tmp file — the last good lock stays intact under the original
/// name; readers never observe a truncated file.
pub fn save_config_lock_to(config_dir: &Path, lock: &ConfigLock) -> Result<()> {
    save_config_lock_at(&config_lock_path_for(config_dir), lock)
}

/// Persist `lock` into the RESOLVED config (see [`config_lock_path`]), atomically.
pub fn save_config_lock(lock: &ConfigLock) -> Result<()> {
    save_config_lock_at(&config_lock_path(), lock)
}

/// Shared atomic writer behind [`save_config_lock_to`] / [`save_config_lock`].
fn save_config_lock_at(path: &Path, lock: &ConfigLock) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(lock)
        .map_err(|e| anyhow::anyhow!("failed to serialize config lock: {}", e))?;
    // Atomic write: serialize to `<config>/workestrate.lock.tmp` on the SAME
    // filesystem, then rename(2) over the target (the save_registry FN-5
    // pattern). A stale tmp from a crashed writer is consumed (overwritten +
    // renamed) by the next save.
    let tmp = path.with_file_name(format!("{}.tmp", LOCK_FILE_NAME));
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Build a FRESH lock from a registry plus the ACTUAL checked-out revs under
/// `config_dir`: for each `[fleets.<name>]`, rev = `git rev-parse HEAD` of
/// `<config>/fleets/<name>` when that checkout exists; otherwise fall
/// back to the registry-recorded rev (a registered-but-not-cloned fleet still
/// gets its last-known pin, and local-path fleets keep rev=None — that is
/// their registry shape too). `config_version` defaults to 2 (the ADR 0023
/// single-config layout) when the registry does not record one; `tool_version`
/// is this binary's version; `version` is [`LOCK_VERSION`].
///
/// A5 Session 2: this stays the BOOTSTRAP writer (config init/clone, and the
/// no-prior-lock fallback in `cmd_fleet_update`) and deliberately leaves
/// `sha`/`fetched_at` empty — the EXPLICIT pin writers (`fleet add`,
/// `fleet update`, first-resolution-with-notice) populate those via
/// [`upsert_locked_pin`], which is also why `cmd_fleet_update` must update
/// a loaded lock IN PLACE rather than rebuilding via this function (a
/// rebuild would clobber the archive-aware fields back to None).
pub fn lock_from_registry(registry: &Registry, config_dir: &Path) -> ConfigLock {
    let mut fleets = BTreeMap::new();
    for (name, entry) in &registry.fleets {
        let checkout = config_dir.join("fleets").join(name);
        let rev = if checkout.join(".git").exists() {
            crate::git::git_rev_parse(&checkout)
                .ok()
                .or(entry.rev.clone())
        } else {
            entry.rev.clone()
        };
        fleets.insert(
            name.clone(),
            LockedFleet {
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
    ConfigLock {
        version: LOCK_VERSION,
        config_version: registry.settings.config_version.unwrap_or(2),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        fleets,
    }
}

/// Upsert the PRIMARY pin for `name` in `lock` (A5 Session 2; ADR 0032
/// addendum §Config source model). This is THE shared write path of the
/// explicit lock writers (`fleet add`, `fleet update`, and the
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
    lock: &mut ConfigLock,
    name: &str,
    url: &str,
    git_ref: Option<&str>,
    sha: &str,
) {
    let refs = lock
        .fleets
        .get(name)
        .map(|existing| existing.refs.clone())
        .unwrap_or_default();
    lock.fleets.insert(
        name.to_string(),
        LockedFleet {
            url: url.to_string(),
            r#ref: git_ref.map(|s| s.to_string()),
            rev: Some(sha.to_string()),
            sha: Some(sha.to_string()),
            fetched_at: Some(crate::microsandbox::runtime::time::current_rfc3339_utc()),
            refs,
        },
    );
}

/// Upsert a NON-PRIMARY ref pin into `fleets.<name>.refs.<ref>` (A5 Session
/// 3a; ADR 0032 addendum §Selection ladder). This is THE write path of the
/// `--config-ref` consumption override (`config::loading`): the resolved ref
/// rides the refs map KEYED BY THE REF ITSELF (a sha-shaped ref is legal and
/// keyed by itself) as `{rev = <ref>, sha = <resolved commit>, fetched_at}`.
///
/// The PRIMARY pin (`rev`/`sha`/`fetched_at` at `[fleets.<name>]`) is NEVER
/// moved by this writer — only [`upsert_locked_pin`] moves it; an existing
/// entry's primary fields pass through untouched. When the fleet has no lock
/// entry yet, one is created carrying the registry's `url`/`ref` with EMPTY
/// primary pin fields (`rev`/`sha`/`fetched_at` = None) — a config whose ONLY
/// resolutions were `--config-ref` overrides still has no primary pin.
///
/// Callers own `lock.version`/`tool_version` stamping and the atomic save.
pub fn upsert_locked_ref(
    lock: &mut ConfigLock,
    name: &str,
    url: &str,
    git_ref: Option<&str>,
    ref_name: &str,
    sha: &str,
) {
    let entry = lock
        .fleets
        .entry(name.to_string())
        .or_insert_with(|| LockedFleet {
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
        unsafe_code,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_in_result
    )]
    use super::*;
    use crate::config::test_support::*;

    /// Set WORKESTRATE_CONFIG to a fresh temp dir (ConfigDirKind::Env → lock at
    /// `<tmp>/workestrate.lock`) and return the dir. Caller must hold
    /// ENV_TEST_LOCK and an EnvGuard for CONFIG_ENV_KEYS.
    fn pin_config(label: &str) -> PathBuf {
        let config_dir = uniq_dir(label);
        std::fs::create_dir_all(&config_dir).expect("create pinned config dir");
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG", &config_dir) };
        config_dir
    }

    fn sample_lock() -> ConfigLock {
        let mut fleets = BTreeMap::new();
        fleets.insert(
            "personal".to_string(),
            LockedFleet {
                url: "https://example.invalid/personal.git".to_string(),
                r#ref: Some("main".to_string()),
                rev: Some("abc123".to_string()),
                sha: None,
                fetched_at: None,
                refs: BTreeMap::new(),
            },
        );
        fleets.insert(
            "work".to_string(),
            LockedFleet {
                url: "https://example.invalid/work.git".to_string(),
                r#ref: Some("main".to_string()),
                rev: Some("def456".to_string()),
                sha: None,
                fetched_at: None,
                refs: BTreeMap::new(),
            },
        );
        ConfigLock {
            version: LOCK_VERSION,
            config_version: 2,
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            fleets,
        }
    }

    #[test]
    fn lock_round_trip_save_then_load_equals() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("lock-roundtrip");

        let lock = sample_lock();
        save_config_lock(&lock)?;
        let loaded = load_config_lock()?.expect("lock should exist after save");
        assert_eq!(loaded, lock);
        // And via the explicit-config variants.
        let loaded_from =
            load_config_lock_from(&config_dir)?.expect("lock via explicit config dir");
        assert_eq!(loaded_from, lock);
        assert_eq!(config_lock_path(), config_lock_path_for(&config_dir));

        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    #[test]
    fn save_is_atomic_no_tmp_survives_and_stale_tmp_is_consumed() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("lock-atomic");

        save_config_lock(&sample_lock())?;
        let tmp = config_lock_path().with_file_name(format!("{}.tmp", LOCK_FILE_NAME));
        assert!(!tmp.exists(), "tmp file must not survive the rename");

        // A stale garbage tmp from a crashed writer neither clobbers the
        // committed lock nor survives the next save.
        std::fs::write(&tmp, "garbage-partial-write")?;
        let before = std::fs::read_to_string(config_lock_path())?;
        let mut next = sample_lock();
        next.config_version = 1;
        save_config_lock(&next)?;
        assert!(!tmp.exists(), "save must consume (rename) the tmp file");
        let loaded = load_config_lock()?.expect("lock parses after save");
        assert_eq!(loaded.config_version, 1);
        assert_ne!(
            std::fs::read_to_string(config_lock_path())?,
            before,
            "the new save must replace the old lock"
        );

        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    #[test]
    fn absent_lock_is_tolerated_as_none() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("lock-absent");

        assert_eq!(load_config_lock()?, None);
        assert_eq!(load_config_lock_from(&config_dir)?, None);

        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    #[test]
    fn absent_version_field_loads_as_oldest() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("lock-oldest");
        std::fs::write(
            config_lock_path(),
            "config_version = 2\ntool_version = \"0.1.0\"\n",
        )?;

        let lock = load_config_lock()?.expect("lock without a version parses");
        assert_eq!(lock.version, 1, "absent version = oldest supported");
        assert!(lock.fleets.is_empty(), "absent fleets = empty map");

        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    #[test]
    fn newer_version_is_a_hard_error_naming_both_versions() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("lock-newer");
        std::fs::write(
            config_lock_path(),
            "version = 99\nconfig_version = 2\ntool_version = \"0.1.0\"\n",
        )?;

        let err = load_config_lock().expect_err("version 99 must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("config created by a newer workestrate"),
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

        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    #[test]
    fn unknown_top_level_and_nested_keys_are_rejected() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("lock-unknown");

        // Unknown TOP-LEVEL key.
        std::fs::write(
            config_lock_path(),
            "config_version = 2\ntool_version = \"0.1.0\"\nbogus = 1\n",
        )?;
        assert!(
            load_config_lock().is_err(),
            "an unknown top-level key must fail to parse"
        );

        // Unknown key INSIDE [fleets.x].
        std::fs::write(
            config_lock_path(),
            "config_version = 2\ntool_version = \"0.1.0\"\n\n[fleets.x]\nurl = \"u\"\nbogus = 1\n",
        )?;
        assert!(
            load_config_lock().is_err(),
            "an unknown key inside [fleets.x] must fail to parse"
        );

        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    #[test]
    fn lock_from_registry_uses_checkout_rev_then_registry_rev() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("lock-from-registry");

        // One entry with a real checkout (rev comes from git), one without
        // (rev falls back to the registry-recorded pin), one local-path
        // (rev stays None).
        let checkout = config_dir.join("fleets").join("cloned");
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
            |url: &str, git_ref: Option<&str>, rev: Option<&str>| crate::config::FleetEntry {
                url: url.to_string(),
                r#ref: git_ref.map(|s| s.to_string()),
                rev: rev.map(|s| s.to_string()),
                secrets: None,
                secrets_file: None,
                age_key_file: None,
                image_keep_last: None,
            };
        registry.fleets.insert(
            "cloned".to_string(),
            entry(
                "https://example.invalid/cloned.git",
                Some("main"),
                Some("stale"),
            ),
        );
        registry.fleets.insert(
            "recorded-only".to_string(),
            entry(
                "https://example.invalid/rec.git",
                Some("main"),
                Some("rec123"),
            ),
        );
        registry
            .fleets
            .insert("local".to_string(), entry("/some/local/path", None, None));

        let lock = lock_from_registry(&registry, &config_dir);
        assert_eq!(lock.version, LOCK_VERSION);
        assert_eq!(
            lock.config_version, 2,
            "absent config_version defaults to 2"
        );
        assert!(!lock.tool_version.is_empty());
        assert_eq!(
            lock.fleets["cloned"].rev.as_deref(),
            Some(head.as_str()),
            "an existing checkout's ACTUAL HEAD wins over the stale registry rev"
        );
        assert_eq!(
            lock.fleets["recorded-only"].rev.as_deref(),
            Some("rec123"),
            "no checkout → fall back to the registry-recorded rev"
        );
        assert_eq!(
            lock.fleets["local"].rev, None,
            "local-path fleets keep rev=None"
        );
        assert_eq!(
            lock.fleets["cloned"].url,
            "https://example.invalid/cloned.git"
        );
        assert_eq!(lock.fleets["cloned"].r#ref.as_deref(), Some("main"));
        // A5 Session 1: the v2 fields are threaded empty until Session 2
        // wires the archive-aware writer.
        assert_eq!(lock.fleets["cloned"].sha, None);
        assert_eq!(lock.fleets["cloned"].fetched_at, None);
        assert!(lock.fleets["cloned"].refs.is_empty());

        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    // ---- A5 / lock v2: ref-aware fields ----

    /// A v1 lock (old shape: version absent or 1, no sha/fetched_at/refs
    /// keys) parses with the v2 fields DEFAULTED — and loading does NOT
    /// bump the in-memory version (writers, not readers, own the bump).
    #[test]
    fn v1_lock_parses_with_defaulted_v2_fields_and_is_not_bumped() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("lock-v1-read");

        // version absent (= oldest) and an old-shape [fleets.x] entry.
        std::fs::write(
            config_lock_path(),
            "config_version = 2\ntool_version = \"0.1.0\"\n\n[fleets.work]\nurl = \"https://example.invalid/work.git\"\nref = \"main\"\nrev = \"abc123\"\n",
        )?;

        let lock = load_config_lock()?.expect("v1 lock must parse");
        assert_eq!(lock.version, 1, "reading must NOT bump the version");
        let fleet = &lock.fleets["work"];
        assert_eq!(fleet.rev.as_deref(), Some("abc123"));
        assert_eq!(fleet.sha, None, "v1 entry has no sha");
        assert_eq!(fleet.fetched_at, None, "v1 entry has no fetched_at");
        assert!(fleet.refs.is_empty(), "v1 entry has an empty refs map");

        // Explicit version = 1 parses the same way.
        std::fs::write(
            config_lock_path(),
            "version = 1\nconfig_version = 2\ntool_version = \"0.1.0\"\n\n[fleets.work]\nurl = \"u\"\n",
        )?;
        let lock = load_config_lock()?.expect("explicit v1 lock must parse");
        assert_eq!(lock.version, 1);
        assert_eq!(lock.fleets["work"].sha, None);

        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    /// v2 round-trip: sha/fetched_at and the refs map survive save+load.
    #[test]
    fn v2_round_trip_includes_sha_fetched_at_and_refs() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("lock-v2-roundtrip");

        let mut lock = sample_lock();
        lock.fleets.get_mut("personal").unwrap().sha = Some("abc123".to_string());
        lock.fleets.get_mut("personal").unwrap().fetched_at =
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
        lock.fleets.get_mut("work").unwrap().refs = refs;

        save_config_lock(&lock)?;
        let content = std::fs::read_to_string(config_lock_path())?;
        assert!(
            content.contains("version = 2"),
            "writers must stamp version = 2:\n{content}"
        );
        assert!(
            content.contains("[fleets.work.refs.feat-x]"),
            "the refs map must serialize as [fleets.<name>.refs.<ref>]:\n{content}"
        );
        let loaded = load_config_lock()?.expect("v2 lock parses");
        assert_eq!(loaded, lock);

        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    /// Version 3 (one past the newest supported) hard-errors with the exact
    /// compat phrase — the BY-DESIGN posture: a v2 file read by an old
    /// (v1-only) binary fails the same way.
    #[test]
    fn version_3_is_a_hard_error_naming_the_phrase() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("lock-v3");

        std::fs::write(
            config_lock_path(),
            "version = 3\nconfig_version = 2\ntool_version = \"0.1.0\"\n",
        )?;
        let err = load_config_lock().expect_err("version 3 must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("config created by a newer workestrate"),
            "error must contain the exact phrase: {msg}"
        );
        assert!(msg.contains('3'), "error must name the lock version: {msg}");

        let _ = std::fs::remove_dir_all(&config_dir);
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

    /// An unknown key inside `[fleets.x.refs.<ref>]` is a loud parse error
    /// (deny_unknown_fields on LockedRef, same posture as the rest of the
    /// lock).
    #[test]
    fn unknown_key_inside_refs_is_rejected() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("lock-refs-unknown");

        std::fs::write(
            config_lock_path(),
            "version = 2\nconfig_version = 2\ntool_version = \"0.1.0\"\n\n[fleets.x]\nurl = \"u\"\n\n[fleets.x.refs.main]\nrev = \"main\"\nsha = \"0123456789abcdef0123456789abcdef01234567\"\nfetched_at = \"2026-08-24T10:00:00Z\"\nbogus = 1\n",
        )?;
        assert!(
            load_config_lock().is_err(),
            "an unknown key inside [fleets.x.refs.main] must fail to parse"
        );

        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    // ---- A5 Session 2: upsert_locked_pin (the explicit pin write path) ----

    #[test]
    fn upsert_locked_pin_stamps_sha_and_fetched_at_and_preserves_refs() {
        let mut lock = sample_lock();
        // Pre-existing NON-primary ref pin must survive an upsert of the
        // primary pin.
        lock.fleets.get_mut("work").unwrap().refs.insert(
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

        let fleet = &lock.fleets["work"];
        assert_eq!(
            fleet.rev.as_deref(),
            Some("aaaaaaa1111111bbbbbbb2222222ccccccc3333333"),
            "rev = the resolved commit"
        );
        assert_eq!(
            fleet.sha.as_deref(),
            Some("aaaaaaa1111111bbbbbbb2222222ccccccc3333333"),
            "sha = rev (the pin IS the commit)"
        );
        let fetched_at = fleet.fetched_at.as_deref().expect("fetched_at stamped");
        assert!(
            fetched_at.ends_with('Z') && fetched_at.contains('T'),
            "fetched_at must be RFC3339 UTC: {fetched_at}"
        );
        assert_eq!(fleet.r#ref.as_deref(), Some("main"));
        assert!(
            lock.fleets["work"].refs.contains_key("feat-x"),
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
        let new = &lock.fleets["new"];
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
        let fleet = &lock.fleets["work"];
        // The PRIMARY pin is untouched.
        assert_eq!(fleet.rev.as_deref(), Some("def456"));
        assert_eq!(fleet.sha, None, "the sample primary sha stays as it was");
        assert_eq!(fleet.fetched_at, None);
        // The refs map carries the new pin keyed by the ref itself.
        let pin = &fleet.refs["feat-x"];
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
        assert_eq!(lock.fleets["work"].refs[sha_ref].rev, sha_ref);

        // A fleet with NO lock entry yet gets one with EMPTY primary fields —
        // a --config-ref-only history never manufactures a primary pin.
        upsert_locked_ref(
            &mut lock,
            "fresh",
            "https://example.invalid/f.git",
            Some("main"),
            "feat-y",
            "bbbbbbb",
        );
        let fresh = &lock.fleets["fresh"];
        assert_eq!(fresh.url, "https://example.invalid/f.git");
        assert_eq!(fresh.r#ref.as_deref(), Some("main"));
        assert_eq!(fresh.rev, None, "no primary pin is fabricated");
        assert_eq!(fresh.sha, None);
        assert_eq!(fresh.fetched_at, None);
        assert_eq!(fresh.refs["feat-y"].sha, "bbbbbbb");
    }
}
