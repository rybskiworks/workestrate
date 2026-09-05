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
//!
//! ## A2 (ADR 0032 §Image tags — DECIDED 2026-08-24)
//!
//! Store tags are **immutable and content-addressed**: `<name>:<ctx>.<sha>`
//! when a tag context is in effect ([`image_tag_context`]), `<name>:<sha>`
//! otherwise, where `sha` is the first [`OUT_PATH_HASH_PREFIX_LEN`] chars of
//! the evaluated out_path's store-hash segment ([`compute_image_tag`]). No
//! mutable registry tags exist for nix-layered images: the mutable
//! per-context **current-pointer** lives HERE, in the state file's `pointers`
//! map ([`PointerRecord`], keyed by [`pointer_key`]), upserted by every
//! successful build+load record update (pipeline stage 4, inside the same
//! locked critical section) and read — read-only — by plan/spawn-time image
//! resolution ([`resolve_image_tag`]). `pointers` is an additive optional
//! field: legacy state files without it parse via serde default (never a
//! hard fail) and resolution falls back to the legacy declared `name:tag`.

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
/// [`crate::images::repo_key`]. Under A2 the `tag` segment is the COMPUTED
/// content-addressed tag ([`compute_image_tag`], e.g.
/// `workestrate-pi:personal.9f3a1c2e7b4d`); pre-migration records keep the
/// verbatim `name:tag` from config TOML (USER DECISION D2).
pub fn image_key(repo_key: &str, tag: &str) -> String {
    format!("{repo_key}#{tag}")
}

// ---------------------------------------------------------------------------
// A2: content-addressed tags + the state-dir current-pointer (ADR 0032
// §Image tags — DECIDED 2026-08-24)
// ---------------------------------------------------------------------------

/// Length of the sha segment in a computed image tag: the FIRST 12 chars of
/// the out_path's 32-char base32 store-hash segment (ADR 0032 §Image tags).
pub const OUT_PATH_HASH_PREFIX_LEN: usize = 12;

/// The tag context segment (ADR 0032 §Image tags): the ARMED inline-override
/// config ref wins (A5 rung 3 — `prime:feat-x` builds
/// `workestrate-prime:feat-x.<sha>` and moves ONLY the `(name, "feat-x")`
/// pointer; the home context's pointer never flaps), else the active context
/// name, else `None` (bare-layers mode → the ctx-less tag form).
///
/// The armed override ref is SLUGIFIED via
/// [`crate::config::registry::slugify_context_candidate`] before becoming
/// the ctx segment — the SAME slug as the context candidate for the same
/// branch (`feat/x` → `feat-x`, `Foo#1.2` → `foo-1-2`), so raw branch names
/// never inject illegal characters (`/`, `#`, `.`, uppercase) into image
/// tags — the ctx segment NEVER contains a dot, keeping the `<ctx>.<sha>`
/// split (at the LAST `.`) unambiguous. A
/// ref that slugifies to `None` (no usable chars, e.g. `###`) falls back to
/// the active context name — the registry ladder's None→falls-through
/// convention. With no armed override the active context name is used
/// verbatim (context names are already legal by derivation/validation).
///
/// The armed override is process-global and arms only for the invocation's
/// own workload (deps are ensured BEFORE arming — see
/// `commands::deps` / main.rs), so no workload-name filter is applied here.
pub fn image_tag_context() -> Option<String> {
    if let Some((_workload, config_ref)) = crate::config::inline_ref::armed_inline_override()
        && let Some(slug) = crate::config::registry::slugify_context_candidate(&config_ref)
    {
        return Some(slug);
    }
    // No usable characters (e.g. `###`) → fall through to the active
    // context name (the registry ladder's None→falls-through convention).
    crate::config::active_context_name()
}

/// Extract the sha segment for a computed tag: the first
/// [`OUT_PATH_HASH_PREFIX_LEN`] chars of the store-hash segment of
/// `out_path` (the base32 run between `/nix/store/` and the first `-` of
/// `-<name>`). Returns `None` when the path is not a `/nix/store/<hash>-…`
/// shape (a malformed nix eval result — the caller hard-errors).
pub fn store_hash_prefix(out_path: &str) -> Option<String> {
    let rest = out_path.strip_prefix("/nix/store/")?;
    let hash = rest.split('-').next().filter(|h| !h.is_empty())?;
    Some(hash.chars().take(OUT_PATH_HASH_PREFIX_LEN).collect())
}

/// The immutable per-build store tag (ADR 0032 §Image tags, AMENDED
/// 2026-08-28): `<name>:<ctx>.<sha>` when a tag context is in effect,
/// `<name>:<sha>` when `ctx` is `None`. `name` is the image name
/// (`image.name`, verbatim flake attr); `sha` derives from the EVALUATED
/// out_path (eval-only — no build), so unchanged inputs yield the identical
/// tag (the content-addressed skip) and changed inputs a fresh tag.
///
/// The separator between ctx and sha is a DOT, never a second colon: the
/// pre-amendment `<name>:<ctx>:<sha>` form is an INVALID docker/OCI image
/// reference (exactly one colon separates name from tag) and the first real
/// host `msb load` of such a tag failed with `manifest parse error: invalid
/// image reference` (host Bug B; the container tests' fake backends never
/// exercised the parse). The dotted form parses unambiguously: the name is
/// split at the FIRST `:` (dots in a flake-attr name are legal and
/// harmless), the sha segment ([`OUT_PATH_HASH_PREFIX_LEN`] lowercase
/// base32 chars) NEVER contains a dot, and the ctx segment never does
/// either ([`crate::config::registry::slugify_context_candidate`] collapses
/// every non-alphanumeric run — including dots — to dashes), so the tag
/// segment splits at the LAST `.`. The whole string satisfies the docker
/// tag grammar (`[a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}`) — pinned by
/// `compute_image_tag_output_is_a_valid_docker_reference` below.
pub fn compute_image_tag(name: &str, ctx: Option<&str>, out_path: &str) -> Result<String> {
    let sha = store_hash_prefix(out_path).ok_or_else(|| {
        anyhow::anyhow!(
            "nix eval printed an outPath in an unexpected shape: '{out_path}' \
             (expected /nix/store/<hash>-<name>)"
        )
    })?;
    Ok(match ctx {
        Some(ctx) => format!("{name}:{ctx}.{sha}"),
        None => format!("{name}:{sha}"),
    })
}

/// The pointer-map key (ADR 0032 §Image tags): `<repo>#<name>` when no tag
/// context is in effect, `<repo>#<name>#<ctx>` otherwise. `#` never appears
/// in repo keys (registered name or canonical path), image names, or
/// context/ref names by construction (git refnames and workload/image names
/// exclude it), so the segments split unambiguously.
pub fn pointer_key(repo: &str, name: &str, ctx: Option<&str>) -> String {
    match ctx {
        Some(ctx) => format!("{repo}#{name}#{ctx}"),
        None => format!("{repo}#{name}"),
    }
}

/// True when `key` (a [`pointer_key`] composite) addresses image `name`
/// under tag context `ctx`, for ANY repo (the cross-repo scan half of
/// [`resolve_image_tag`]).
fn pointer_key_matches(key: &str, name: &str, ctx: Option<&str>) -> bool {
    let parts: Vec<&str> = key.split('#').collect();
    match (parts.as_slice(), ctx) {
        ([_, n], None) => *n == name,
        ([_, n, c], Some(want)) => *n == name && *c == want,
        _ => false,
    }
}

/// The mutable current-pointer record (ADR 0032 §Image tags): the state-dir
/// replacement for the superseded mutable alias TAG. `tag` is the immutable
/// content-addressed tag this (repo, name, ctx) currently resolves to;
/// `updated_at` is the RFC3339 UTC stamp of the build+load (or D1 trust)
/// that moved the pointer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PointerRecord {
    pub tag: String,
    pub updated_at: String,
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
/// (spec §8), plus the additive A2 `"pointers"` map (serde-defaulted —
/// legacy files without it parse unchanged, and `version` stays 1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImagesState {
    pub version: u32,
    #[serde(default)]
    pub images: BTreeMap<String, ImageRecord>,
    /// A2 current-pointers: `<repo>#<name>[#<ctx>]` → the immutable
    /// content-addressed tag that (repo, name, ctx) currently resolves to
    /// (ADR 0032 §Image tags). Additive/optional — never hard-fail old state.
    #[serde(default)]
    pub pointers: BTreeMap<String, PointerRecord>,
}

impl Default for ImagesState {
    fn default() -> Self {
        Self {
            version: IMAGES_STATE_VERSION,
            images: BTreeMap::new(),
            pointers: BTreeMap::new(),
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

    /// Look up the current-pointer for a [`pointer_key`] composite.
    pub fn lookup_pointer(&self, key: &str) -> Option<&PointerRecord> {
        self.pointers.get(key)
    }

    /// Move the current-pointer for a [`pointer_key`] composite. Called
    /// alongside every successful build+load record upsert (pipeline stage
    /// 4) and by the D1 trust baseline, inside the same locked critical
    /// section (ADR 0032 §Image tags).
    pub fn upsert_pointer(&mut self, key: String, record: PointerRecord) {
        self.pointers.insert(key, record);
    }
}

/// Plan/spawn-time image resolution (ADR 0032 §Image tags — DECIDED
/// 2026-08-24): the tag a nix-layered workload's sandbox image resolves to.
///
/// READ-ONLY (a cheap `images.json` load — `resolve_image` runs at plan
/// time; this path NEVER writes): look up the current-pointer for
/// `(name, image_tag_context())`, preferring the `repo`-keyed entry when the
/// declaring-repo identity is known; on a MISS fall back to the legacy
/// declared `name:<declared-tag>` form — byte-identical to pre-migration
/// behavior (golden plans, and not-yet-rebuilt homes such as pi/tempest,
/// keep working until their first post-upgrade ensure writes a pointer).
///
/// Repo matching: when `repo` is `None` (declaring repo not determinable at
/// plan time — synthetic/single-file layers), pointers are matched by
/// name+ctx ACROSS repos; BTreeMap order makes a multi-repo collision
/// resolve deterministically (lexicographically first key).
pub fn resolve_image_tag(
    state_dir: &Path,
    repo: Option<&str>,
    name: &str,
    declared_tag: &str,
) -> String {
    let legacy = format!("{name}:{declared_tag}");
    let ctx = image_tag_context();
    let state = ImagesState::load(state_dir);
    if let Some(repo) = repo
        && let Some(p) = state.lookup_pointer(&pointer_key(repo, name, ctx.as_deref()))
    {
        return p.tag.clone();
    }
    for (key, p) in &state.pointers {
        if pointer_key_matches(key, name, ctx.as_deref()) {
            return p.tag.clone();
        }
    }
    legacy
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

    // ---- A2: content-addressed tags + current-pointers (ADR 0032) ----

    /// A realistic 32-char base32 store-hash segment fixture.
    const HASH32: &str = "abcdefghijklmnopqrstuvwxyz012345";

    /// Tag computation (pure): `<name>:<ctx>.<sha>` with a tag context,
    /// `<name>:<sha>` without; the sha is the FIRST 12 chars of the
    /// out_path's store-hash segment.
    #[test]
    fn compute_image_tag_composes_name_ctx_sha() {
        let out = format!("/nix/store/{HASH32}-workestrate-prime.tar.gz");
        assert_eq!(
            compute_image_tag("workestrate-prime", Some("personal"), &out).unwrap(),
            "workestrate-prime:personal.abcdefghijkl",
            "ctx-present form under a tag context; 12-char sha slice, dot separator"
        );
        assert_eq!(
            compute_image_tag("workestrate-prime", None, &out).unwrap(),
            "workestrate-prime:abcdefghijkl",
            "ctx-less form when no tag context is in effect"
        );
        // The name half of the store path may itself carry '-' — the hash
        // segment ends at the FIRST '-'.
        let dashed = format!("/nix/store/{HASH32}-workestrate-pi.tar.gz");
        assert_eq!(store_hash_prefix(&dashed).as_deref(), Some("abcdefghijkl"));
        // A name carrying the override ref as ctx (A5): prime:feat-x builds
        // workestrate-prime:feat-x.<sha>.
        assert_eq!(
            compute_image_tag("workestrate-prime", Some("feat-x"), &out).unwrap(),
            "workestrate-prime:feat-x.abcdefghijkl"
        );
        // Malformed eval output is a hard error, never a garbage tag.
        assert!(compute_image_tag("img", None, "not-a-store-path").is_err());
        assert!(compute_image_tag("img", None, "/nix/store/").is_err());
    }

    /// HOST BUG B REGRESSION (ADR 0032 §Image tags, AMENDED 2026-08-28):
    /// every shape `compute_image_tag` can emit must be a VALID docker/OCI
    /// image reference — validated with the REAL parser the msb backend
    /// uses (`microsandbox_image::Reference`, re-exported from the vendored
    /// fork's `oci-client`), not a local approximation. The pre-amendment
    /// `<name>:<ctx>:<sha>` shape failed the first real host `msb load`
    /// (`manifest parse error: invalid image reference`); the container
    /// tests' fake backends never exercised the parse, which is why this
    /// test asserts against the parser directly.
    #[test]
    fn compute_image_tag_output_is_a_valid_docker_reference() {
        let out = format!("/nix/store/{HASH32}-workestrate-prime.tar.gz");
        let long_ctx = "a".repeat(100);
        let cases: Vec<(&str, Option<&str>)> = vec![
            // ctx-less (bare-layers mode).
            ("workestrate-prime", None),
            // Plain ctx (home context name).
            ("workestrate-prime", Some("personal")),
            // Slugified branch ctx with dashes.
            ("workestrate-prime", Some("migration-tool-model")),
            ("workestrate-prime", Some("foo-1-2")),
            // Long ctx (100 chars) — the whole tag must stay within the
            // 128-char docker tag bound.
            ("workestrate-prime", Some(long_ctx.as_str())),
            // Underscores/digits (legal in both ctx slugs and the docker
            // tag charset) and a dotted flake-attr NAME (dots in the name
            // are legal: the name splits at the FIRST colon).
            ("img.with.dots", Some("ctx_2")),
            // Minimal shapes.
            ("w", Some("x")),
            ("w", None),
        ];
        for (name, ctx) in &cases {
            let tag = compute_image_tag(name, *ctx, &out).unwrap();
            tag.parse::<microsandbox_image::Reference>()
                .unwrap_or_else(|e| {
                    panic!("computed tag '{tag}' is not a valid docker/OCI reference: {e}")
                });
            // Exactly one colon (the name:tag separator) — the two-colon
            // shape is the invalid form this regression pins against.
            assert_eq!(
                tag.matches(':').count(),
                1,
                "exactly one colon in '{tag}' (the pre-amendment name:ctx:sha \
                 shape is an invalid reference)"
            );
            // And the emitted tag round-trips through the GC parser.
            let (parsed_name, parsed_ctx) = crate::images::gc::split_computed_tag(&tag)
                .unwrap_or_else(|| panic!("computed tag '{tag}' must parse as computed"));
            assert_eq!((parsed_name.as_str(), parsed_ctx.as_deref()), (*name, *ctx));
        }

        // The pre-amendment shape itself must NOT parse — the exact host
        // failure mode, pinned so the grammar can never regress to it.
        assert!(
            "workestrate-prime:main:3n87p1a3ncbr"
                .parse::<microsandbox_image::Reference>()
                .is_err(),
            "the two-colon shape is an invalid docker/OCI reference (host Bug B)"
        );
    }

    /// Pointer key shape: `<repo>#<name>` / `<repo>#<name>#<ctx>`.
    #[test]
    fn pointer_key_shape_and_matching() {
        assert_eq!(pointer_key("personal", "img", None), "personal#img");
        assert_eq!(
            pointer_key("personal", "img", Some("feat-x")),
            "personal#img#feat-x"
        );
        assert!(pointer_key_matches("personal#img", "img", None));
        assert!(!pointer_key_matches(
            "personal#img",
            "img",
            Some("personal")
        ));
        assert!(pointer_key_matches(
            "personal#img#feat-x",
            "img",
            Some("feat-x")
        ));
        assert!(!pointer_key_matches("personal#img#feat-x", "img", None));
        assert!(!pointer_key_matches("personal#other", "img", None));
    }

    /// Legacy state files (no `pointers` key) parse via serde default —
    /// NEVER a hard fail — and resolution falls back to the legacy declared
    /// tag (byte-identical pre-migration behavior).
    #[test]
    fn legacy_state_without_pointers_parses_and_resolution_falls_back() {
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
        let state_dir = unique_state_dir("images-legacy");
        std::fs::create_dir_all(&state_dir).unwrap();
        std::fs::write(images_state_path(&state_dir), json).unwrap();

        let state = ImagesState::load(&state_dir);
        assert!(state.pointers.is_empty(), "legacy file → empty pointers");
        assert!(
            state.lookup("personal#workestrate-pi:latest").is_some(),
            "the legacy image record still parses"
        );
        assert_eq!(
            resolve_image_tag(&state_dir, Some("personal"), "workestrate-pi", "latest"),
            "workestrate-pi:latest",
            "pointer MISS → legacy declared tag (byte-identical pre-migration)"
        );
        // A hint-less lookup (repo not determinable) falls back identically.
        assert_eq!(
            resolve_image_tag(&state_dir, None, "workestrate-pi", "latest"),
            "workestrate-pi:latest"
        );

        let _ = std::fs::remove_dir_all(&state_dir);
    }

    /// Pointer upsert/lookup round-trip through save+load, and
    /// resolve_image_tag honoring the repo hint then the cross-repo scan.
    #[test]
    fn pointer_round_trip_and_resolution_preference() {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        crate::config::set_active_context(None);
        crate::config::clear_inline_override();
        let state_dir = unique_state_dir("images-pointers");
        let mut state = ImagesState::default();
        state.upsert_pointer(
            pointer_key("personal", "img", None),
            PointerRecord {
                tag: "img:aaaaaaaaaaaa".to_string(),
                updated_at: "2026-08-24T10:00:00Z".to_string(),
            },
        );
        state.upsert_pointer(
            pointer_key("other", "img", None),
            PointerRecord {
                tag: "img:bbbbbbbbbbbb".to_string(),
                updated_at: "2026-08-24T10:01:00Z".to_string(),
            },
        );
        state.save(&state_dir).expect("save");

        // Repo hint wins over the cross-repo scan.
        assert_eq!(
            resolve_image_tag(&state_dir, Some("personal"), "img", "latest"),
            "img:aaaaaaaaaaaa"
        );
        // Unknown repo hint → the cross-repo scan by name+ctx decides
        // (deterministically: lexicographically first key).
        assert_eq!(
            resolve_image_tag(&state_dir, Some("nosuch"), "img", "latest"),
            "img:bbbbbbbbbbbb",
            "scan order is BTreeMap (lexicographic): 'other' < 'personal'"
        );
        assert_eq!(
            resolve_image_tag(&state_dir, None, "img", "latest"),
            "img:bbbbbbbbbbbb"
        );
        // Unknown name → legacy fallback.
        assert_eq!(
            resolve_image_tag(&state_dir, None, "ghost", "v3"),
            "ghost:v3"
        );

        let _ = std::fs::remove_dir_all(&state_dir);
    }

    /// image_tag_context precedence: armed inline-override ref > active
    /// context name > None.
    #[test]
    fn image_tag_context_prefers_armed_override_then_active_context() {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        crate::config::clear_inline_override();
        crate::config::set_active_context(None);
        assert_eq!(image_tag_context(), None, "no context, no override");

        crate::config::set_active_context(Some(crate::config::ActiveContext {
            name: Some("personal".to_string()),
            layers: vec!["personal".to_string()],
        }));
        assert_eq!(image_tag_context().as_deref(), Some("personal"));

        crate::config::set_pending_inline_override("prime", "feat-x");
        assert_eq!(
            image_tag_context().as_deref(),
            Some("personal"),
            "a PENDING (unarmed) override is invisible"
        );
        crate::config::arm_inline_override();
        assert_eq!(
            image_tag_context().as_deref(),
            Some("feat-x"),
            "the armed override ref IS the tag context"
        );

        crate::config::clear_inline_override();
        crate::config::set_active_context(None);
    }

    /// The armed override ref is SLUGIFIED into the tag ctx segment: branch
    /// names with illegal image-tag characters (`/`, `#`, uppercase) yield
    /// the same slug as the registry's context candidate; an already-legal
    /// ref passes through unchanged.
    #[test]
    fn image_tag_context_slugifies_the_armed_override_ref() {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        crate::config::clear_inline_override();
        crate::config::set_active_context(None);

        for (raw, want) in [
            ("feat/x", "feat-x"),
            ("Foo#1.2", "foo-1-2"),
            ("migration/tool-model", "migration-tool-model"),
            ("feat-x", "feat-x"), // already legal → passthrough
        ] {
            crate::config::set_pending_inline_override("prime", raw);
            crate::config::arm_inline_override();
            assert_eq!(
                image_tag_context().as_deref(),
                Some(want),
                "armed override {raw:?} slugifies to {want:?}"
            );
            crate::config::clear_inline_override();
        }

        crate::config::clear_inline_override();
        crate::config::set_active_context(None);
    }

    /// An armed override ref with NO usable characters (e.g. `###`) slugifies
    /// to None and falls back to the active context name — the registry
    /// ladder's None→falls-through convention; with no active context the
    /// tag ctx is None.
    #[test]
    fn image_tag_context_unslugifiable_override_falls_back_to_active_context() {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        crate::config::clear_inline_override();
        crate::config::set_active_context(None);

        crate::config::set_pending_inline_override("prime", "###");
        crate::config::arm_inline_override();
        assert_eq!(
            image_tag_context(),
            None,
            "### slugifies to None; no active context → None"
        );

        crate::config::set_active_context(Some(crate::config::ActiveContext {
            name: Some("personal".to_string()),
            layers: vec!["personal".to_string()],
        }));
        assert_eq!(
            image_tag_context().as_deref(),
            Some("personal"),
            "### slugifies to None → falls back to the active context name"
        );

        crate::config::clear_inline_override();
        crate::config::set_active_context(None);
    }

    /// Consistency: the tag ctx segment under an armed override is the SAME
    /// slug the registry derives as the context candidate for the same
    /// branch — tag ctx and context name never diverge for a given ref.
    #[test]
    fn image_tag_context_matches_registry_slug_for_the_same_branch() {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        crate::config::clear_inline_override();
        crate::config::set_active_context(None);

        for branch in ["feat/x", "Foo#1.2", "migration/tool-model", "feat-x"] {
            crate::config::set_pending_inline_override("prime", branch);
            crate::config::arm_inline_override();
            assert_eq!(
                image_tag_context(),
                crate::config::registry::slugify_context_candidate(branch),
                "tag ctx for {branch:?} equals the registry context-candidate slug"
            );
            crate::config::clear_inline_override();
        }

        crate::config::clear_inline_override();
        crate::config::set_active_context(None);
    }

    /// Under an armed override, resolution moves to the OVERRIDE-ctx pointer
    /// and never touches the home-ctx pointer (the home pointer never flaps).
    #[test]
    fn resolve_image_tag_under_armed_override_uses_the_override_ctx_pointer() {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        crate::config::clear_inline_override();
        crate::config::set_active_context(Some(crate::config::ActiveContext {
            name: Some("personal".to_string()),
            layers: vec!["personal".to_string()],
        }));
        let state_dir = unique_state_dir("images-override-ctx");
        let mut state = ImagesState::default();
        state.upsert_pointer(
            pointer_key("personal", "workestrate-prime", Some("personal")),
            PointerRecord {
                tag: "workestrate-prime:personal.111111111111".to_string(),
                updated_at: "2026-08-24T10:00:00Z".to_string(),
            },
        );
        state.upsert_pointer(
            pointer_key("personal", "workestrate-prime", Some("feat-x")),
            PointerRecord {
                tag: "workestrate-prime:feat-x.222222222222".to_string(),
                updated_at: "2026-08-24T10:05:00Z".to_string(),
            },
        );
        state.save(&state_dir).expect("save");

        // Home context: the home-ctx pointer resolves.
        assert_eq!(
            resolve_image_tag(&state_dir, Some("personal"), "workestrate-prime", "latest"),
            "workestrate-prime:personal.111111111111"
        );
        // Armed override: the override-ctx pointer resolves instead.
        crate::config::set_pending_inline_override("prime", "feat-x");
        crate::config::arm_inline_override();
        assert_eq!(
            resolve_image_tag(&state_dir, Some("personal"), "workestrate-prime", "latest"),
            "workestrate-prime:feat-x.222222222222",
            "the armed override resolves the override-ctx pointer, not the home one"
        );
        // And the home pointer record itself is untouched (never flaps).
        let state = ImagesState::load(&state_dir);
        assert_eq!(
            state
                .lookup_pointer(&pointer_key(
                    "personal",
                    "workestrate-prime",
                    Some("personal")
                ))
                .map(|p| p.tag.as_str()),
            Some("workestrate-prime:personal.111111111111")
        );

        crate::config::clear_inline_override();
        crate::config::set_active_context(None);
        let _ = std::fs::remove_dir_all(&state_dir);
    }
}
