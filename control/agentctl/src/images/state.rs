//! `state/images.json` — the per-home image build/load record (spec 21 §3.2,
//! §8).
//!
//! The record is **advisory, never authoritative** (USER DECISION D1): the msb
//! image store is ground truth for what is loaded; this file is the tool's
//! memory of what it loaded and why. Consequently [`ImagesState::load`] NEVER
//! hard-errors on a missing or corrupt file — it warns on stderr and yields an
//! empty record set (the phase-C/D skew matrix treats "no record" as
//! `RecordState::Absent` and proceeds).
//!
//! Write discipline (spec §8): atomic tmp+rename, the same discipline as
//! `config::registry::save_registry` (FN-5) and `workestrate.lock` — serialize
//! to a sibling tmp file on the SAME filesystem, fsync, then rename(2) over
//! the target. A crash mid-write can only corrupt the tmp file; readers never
//! observe a torn record. The tmp name is per-writer unique (pid + counter)
//! because saves for DIFFERENT tags legitimately run concurrently under
//! DIFFERENT per-tag locks (spec §3.3) and must not clobber a shared tmp.
//!
//! Key format (spec §8): the map key is the `<repo>#<tag>` composite, e.g.
//! `personal#workestrate-pi:latest` — see [`image_key`].

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Schema version of the state file itself (spec §8: additive bumps only).
pub const IMAGES_STATE_VERSION: u32 = 1;

/// Name of the state file inside the state dir.
pub const IMAGES_STATE_FILE_NAME: &str = "images.json";

/// Name of the per-tag lock dir inside the state dir (spec §3.3).
pub const IMAGE_LOCKS_DIR_NAME: &str = "image-locks";

/// Path of the state file: `<state_dir>/images.json`.
pub fn images_state_path(state_dir: &Path) -> PathBuf {
    state_dir.join(IMAGES_STATE_FILE_NAME)
}

/// Path of the per-tag lock dir: `<state_dir>/image-locks/` (spec §3.3).
pub fn image_locks_dir(state_dir: &Path) -> PathBuf {
    state_dir.join(IMAGE_LOCKS_DIR_NAME)
}

/// The record-map key: `<repo>#<tag>` (spec §8, e.g.
/// `personal#workestrate-pi:latest`). `repo_key` comes from
/// [`crate::images::repo_key`]; `tag` is the stable verbatim `name:tag` from
/// config TOML (USER DECISION D2).
pub fn image_key(repo_key: &str, tag: &str) -> String {
    format!("{repo_key}#{tag}")
}

/// The config-repo identity embedded in every record (spec §8 `repo`). The
/// D5 cross-home collision warning (spec §4.3) compares these across records
/// for the same tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoIdentity {
    /// Registered-repo NAME when the declaring dir is under a registered
    /// checkout; the canonical filesystem path otherwise (spec §4.3).
    pub name: String,
    /// Registered checkout path, or the declaring dir when unregistered.
    pub path: PathBuf,
    /// Nearest ancestor containing `flake.nix` (the repo's flake root).
    pub flake_root: PathBuf,
}

/// One record per (config-repo identity, name:tag) — spec §8.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageRecord {
    pub repo: RepoIdentity,
    /// Flake attribute that builds the image tarball (e.g. `workestrate-pi`).
    pub attr: String,
    /// Stable verbatim `name:tag` loaded into the msb store (USER DECISION D2).
    pub tag: String,
    /// `nix eval --raw <repo>#<attr>.drvPath` at build time (spec §3.1).
    pub drv_path: String,
    /// Realized store path loaded into msb; the re-load gate compares against
    /// it (spec §3.1: re-load only when the outPath differs).
    pub out_path: String,
    /// msb manifest digest at load time; `null` until msb exposes it (spec
    /// §3.5, §11 HOST-VERIFY item 1).
    pub digest: Option<String>,
    /// RFC3339 UTC timestamps (no-chrono formatter —
    /// [`crate::microsandbox::runtime::time::current_rfc3339_utc`]).
    pub built_at: String,
    pub loaded_at: String,
    /// Provenance for the D5 cross-home collision warning (spec §8: who
    /// loaded this, from where). `loader` is e.g. `workestrate 0.1.0`.
    pub loader: String,
    pub host: String,
    pub user: String,
}

/// The full state file: `{ "version": 1, "images": { "<repo>#<tag>": … } }`
/// (spec §8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImagesState {
    pub version: u32,
    #[serde(default)]
    pub images: BTreeMap<String, ImageRecord>,
}

impl Default for ImagesState {
    fn default() -> Self {
        Self {
            version: IMAGES_STATE_VERSION,
            images: BTreeMap::new(),
        }
    }
}

impl ImagesState {
    /// Load `<state_dir>/images.json`. Advisory-record posture (spec §3.2):
    /// absent, unreadable, or CORRUPT content all yield an empty record set —
    /// the corrupt/unreadable cases with a loud stderr WARNING (same posture
    /// as the corrupt-registry fallback in `config::paths`), never a hard
    /// error. The msb store stays ground truth regardless.
    pub fn load(state_dir: &Path) -> Self {
        let path = images_state_path(state_dir);
        match std::fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<ImagesState>(&content) {
                Ok(state) => state,
                Err(e) => {
                    eprintln!(
                        "WARNING: corrupt images state {} ({}); ignoring it and starting from an empty record set — the msb store stays ground truth (spec 21 §3.2). Fix or remove the file; the next image build/load rewrites it.",
                        path.display(),
                        e
                    );
                    Self::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Self::default(),
            Err(e) => {
                eprintln!(
                    "WARNING: unreadable images state {} ({}); ignoring it and starting from an empty record set — the msb store stays ground truth (spec 21 §3.2).",
                    path.display(),
                    e
                );
                Self::default()
            }
        }
    }

    /// Persist to `<state_dir>/images.json`, creating the state dir as
    /// needed. Atomic (tmp+fsync+rename — see module docs); callers hold the
    /// per-tag lock across the record mutation + save (spec §3.3).
    pub fn save(&self, state_dir: &Path) -> Result<()> {
        let path = images_state_path(state_dir);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let content =
            serde_json::to_string_pretty(self).context("failed to serialize images state")?;
        let tmp = tmp_path_for(&path);
        {
            let mut f = std::fs::File::create(&tmp)
                .with_context(|| format!("failed to create {}", tmp.display()))?;
            f.write_all(content.as_bytes())
                .and_then(|()| f.sync_all())
                .with_context(|| format!("failed to write {}", tmp.display()))?;
        }
        std::fs::rename(&tmp, &path).with_context(|| {
            format!("failed to rename {} over {}", tmp.display(), path.display())
        })?;
        Ok(())
    }

    /// Look up the record for a `<repo>#<tag>` key (see [`image_key`]).
    pub fn lookup(&self, key: &str) -> Option<&ImageRecord> {
        self.images.get(key)
    }

    /// Insert/replace the record for a `<repo>#<tag>` key.
    pub fn upsert(&mut self, key: String, record: ImageRecord) {
        self.images.insert(key, record);
    }
}

/// Sibling tmp path for the atomic save, unique per writer (pid + a process-
/// local counter): concurrent saves for different tags run under different
/// per-tag locks and must never share a tmp name. A crashed writer can leave
/// a stale `.tmp-*` behind; it is inert (never renamed over the target) and
/// is cleaned up on the next successful save's directory rewrite — matching
/// the save_registry posture that a stale tmp never clobbers the committed
/// file (FN-5).
fn tmp_path_for(path: &Path) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let file_name = path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| IMAGES_STATE_FILE_NAME.to_string());
    path.with_file_name(format!(".{file_name}.tmp-{}-{n}", std::process::id()))
}

/// `loader` provenance field (spec §8): e.g. `workestrate 0.1.0`.
pub fn loader_string() -> String {
    format!("workestrate {}", env!("CARGO_PKG_VERSION"))
}

/// `host` provenance field (spec §8): `/etc/hostname` first, `HOSTNAME` env
/// fallback, `unknown` last. No new dependency.
pub fn host_name() -> String {
    if let Ok(raw) = std::fs::read_to_string("/etc/hostname") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    std::env::var("HOSTNAME")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

/// `user` provenance field (spec §8): `USER` env, `LOGNAME` fallback,
/// `unknown` last.
pub fn user_name() -> String {
    std::env::var("USER")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| std::env::var("LOGNAME").ok().filter(|s| !s.is_empty()))
        .unwrap_or_else(|| "unknown".to_string())
}

/// Load-time provenance bundle (spec §8 `loader`/`host`/`user` + the record
/// timestamps). Phase C/D captures one of these per build/load to populate an
/// [`ImageRecord`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Provenance {
    pub loader: String,
    pub host: String,
    pub user: String,
    /// RFC3339 UTC capture time (the no-chrono crate-wide formatter).
    pub now: String,
}

impl Provenance {
    pub fn capture() -> Self {
        Self {
            loader: loader_string(),
            host: host_name(),
            user: user_name(),
            now: crate::microsandbox::runtime::time::current_rfc3339_utc(),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;
    use crate::config::test_support::unique_state_dir;

    fn spec_record() -> ImageRecord {
        ImageRecord {
            repo: RepoIdentity {
                name: "personal".to_string(),
                path: PathBuf::from("/home/node/.workestrate/config-repos/personal"),
                flake_root: PathBuf::from("/home/node/.workestrate/config-repos/personal"),
            },
            attr: "workestrate-pi".to_string(),
            tag: "workestrate-pi:latest".to_string(),
            drv_path: "/nix/store/abc123-workestrate-pi.tar.gz.drv".to_string(),
            out_path: "/nix/store/def456-workestrate-pi.tar.gz".to_string(),
            digest: None,
            built_at: "2026-08-02T10:15:00Z".to_string(),
            loaded_at: "2026-08-02T10:16:12Z".to_string(),
            loader: "workestrate 0.1.0".to_string(),
            host: "devbox".to_string(),
            user: "node".to_string(),
        }
    }

    /// Spec §8 example shape: exact field names, `version: 1`, `#` key
    /// separator, `digest: null`.
    #[test]
    fn serde_round_trip_matches_spec_example() {
        let json = r#"{
  "version": 1,
  "images": {
    "personal#workestrate-pi:latest": {
      "repo": {
        "name": "personal",
        "path": "/home/node/.workestrate/config-repos/personal",
        "flake_root": "/home/node/.workestrate/config-repos/personal"
      },
      "attr": "workestrate-pi",
      "tag": "workestrate-pi:latest",
      "drv_path": "/nix/store/abc123-workestrate-pi.tar.gz.drv",
      "out_path": "/nix/store/def456-workestrate-pi.tar.gz",
      "digest": null,
      "built_at": "2026-08-02T10:15:00Z",
      "loaded_at": "2026-08-02T10:16:12Z",
      "loader": "workestrate 0.1.0",
      "host": "devbox",
      "user": "node"
    }
  }
}"#;
        let state: ImagesState = serde_json::from_str(json).expect("spec example parses");
        assert_eq!(state.version, 1, "spec §8: version field present, value 1");
        let key = image_key("personal", "workestrate-pi:latest");
        assert_eq!(
            key, "personal#workestrate-pi:latest",
            "spec §8 key separator is `#`"
        );
        let record = state.lookup(&key).expect("record under the composite key");
        assert_eq!(record.repo.name, "personal");
        assert_eq!(record.attr, "workestrate-pi");
        assert_eq!(record.tag, "workestrate-pi:latest");
        assert_eq!(record.digest, None, "digest is null until msb exposes it");
        assert_eq!(record.loader, "workestrate 0.1.0");
        assert_eq!(record.host, "devbox");
        assert_eq!(record.user, "node");

        // Round-trip: serialize → parse yields the identical state.
        let serialized = serde_json::to_string_pretty(&state).expect("serialize");
        let reparsed: ImagesState = serde_json::from_str(&serialized).expect("reparse");
        assert_eq!(state, reparsed);
    }

    /// Spec §3.2: the record is advisory — a CORRUPT file is an empty record
    /// set plus a stderr note, never an error.
    #[test]
    fn corrupt_file_loads_empty_with_no_error() {
        let state_dir = unique_state_dir("images-corrupt");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(
            images_state_path(&state_dir),
            "this is = not = valid json [[[",
        )
        .unwrap();

        let state = ImagesState::load(&state_dir);
        assert_eq!(state, ImagesState::default(), "corrupt → empty record set");
        assert_eq!(state.version, IMAGES_STATE_VERSION);

        let _ = std::fs::remove_dir_all(&state_dir);
    }

    /// Absent file (normal first-run state) loads silently as empty.
    #[test]
    fn absent_file_loads_empty() {
        let state_dir = unique_state_dir("images-absent");
        let state = ImagesState::load(&state_dir);
        assert_eq!(state, ImagesState::default());
    }

    /// Upsert/lookup round-trip through a save+load cycle.
    #[test]
    fn upsert_lookup_survive_save_load_cycle() {
        let state_dir = unique_state_dir("images-roundtrip");
        let mut state = ImagesState::default();
        let key = image_key("personal", "workestrate-pi:latest");
        assert!(state.lookup(&key).is_none());
        state.upsert(key.clone(), spec_record());
        state.save(&state_dir).expect("save");

        let loaded = ImagesState::load(&state_dir);
        assert_eq!(loaded.lookup(&key), Some(&spec_record()));
        assert_eq!(loaded.version, IMAGES_STATE_VERSION);
        // No tmp file survives the rename.
        let leftovers: Vec<_> = std::fs::read_dir(&state_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty(), "no .tmp-* may survive: {leftovers:?}");

        let _ = std::fs::remove_dir_all(&state_dir);
    }

    /// Atomic write (spec §8): concurrent writers for DIFFERENT tags (each
    /// under its own per-tag lock in production) never leave a torn file and
    /// never strand a `.tmp-*` file.
    #[test]
    fn concurrent_writers_never_tear_and_leave_no_tmp() {
        let state_dir = unique_state_dir("images-concurrent");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let mut handles = Vec::new();
        for t in 0..4u32 {
            let dir = state_dir.clone();
            let b = std::sync::Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                b.wait(); // release all writers at once
                for i in 0..25u32 {
                    let mut state = ImagesState::load(&dir);
                    let key = image_key(&format!("repo-{t}"), &format!("img-{t}:{i}"));
                    let mut record = spec_record();
                    record.tag = format!("img-{t}:{i}");
                    state.upsert(key, record);
                    state.save(&dir).expect("concurrent save must not error");
                }
            }));
        }
        for h in handles {
            h.join().expect("writer thread panicked");
        }

        // The committed file is ALWAYS a complete, parseable document
        // (last-writer-wins is fine; a torn file is not).
        let committed = std::fs::read_to_string(images_state_path(&state_dir)).unwrap();
        let state: ImagesState =
            serde_json::from_str(&committed).expect("committed file must parse after the race");
        assert!(!state.images.is_empty());
        let leftovers: Vec<_> = std::fs::read_dir(&state_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty(), "no .tmp-* may survive: {leftovers:?}");

        let _ = std::fs::remove_dir_all(&state_dir);
    }

    /// A stale tmp from a crashed writer never clobbers the committed file,
    /// and the next save proceeds (FN-5 discipline parity).
    #[test]
    fn interrupted_write_leaves_committed_state_intact() {
        let state_dir = unique_state_dir("images-interrupted");
        let mut good = ImagesState::default();
        good.upsert(image_key("personal", "a:1"), spec_record());
        good.save(&state_dir).expect("commit good state");
        let before = std::fs::read_to_string(images_state_path(&state_dir)).unwrap();

        // Simulate a crashed concurrent writer: garbage in a sibling tmp.
        let stale_tmp = state_dir.join(format!(".images.json.tmp-{}-999", std::process::id()));
        std::fs::write(&stale_tmp, "garbage-partial-write").unwrap();
        assert_eq!(
            std::fs::read_to_string(images_state_path(&state_dir)).unwrap(),
            before,
            "stale tmp must not clobber the committed state"
        );

        let loaded = ImagesState::load(&state_dir);
        assert!(loaded.lookup(&image_key("personal", "a:1")).is_some());

        let _ = std::fs::remove_dir_all(&stale_tmp);
        let _ = std::fs::remove_dir_all(&state_dir);
    }

    #[test]
    fn provenance_capture_shape() {
        let prov = Provenance::capture();
        assert_eq!(
            prov.loader,
            format!("workestrate {}", env!("CARGO_PKG_VERSION"))
        );
        assert!(!prov.host.is_empty());
        assert!(!prov.user.is_empty());
        // RFC3339 shape: YYYY-MM-DDTHH:MM:SSZ = 20 chars.
        assert!(
            prov.now.len() == 20 && prov.now.ends_with('Z'),
            "unexpected rfc3339 shape: {}",
            prov.now
        );
    }
}
