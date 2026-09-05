//! msb state generations: generation identity keys and the ONE home
//! resolution rule shared by every Rust mirror (and, later, the shell
//! converge script `scripts/host-provision.sh`).
//!
//! A GENERATION is the mutable state of one pinned msb build. Its identity
//! key is the 12-char prefix of the 32-char base32 hash segment of the
//! resolved microsandbox nix store path: `readlink -f` of the baked
//! `MSB_PATH` (`/nix/store/<hash32>-microsandbox-<ver>/bin/msb`) yields the
//! store dir basename `<hash32>-microsandbox-<ver>`; stripping everything
//! from the first `-microsandbox-` leaves `<hash32>`, whose first 12 chars
//! are the key. When the resolved msb is NOT a nix-store microsandbox path
//! (raw PATH install, canonicalize failure, basename pattern mismatch) the
//! key is [`UNMANAGED_KEY`] — the legacy single-generation posture.
//!
//! Layout: `$HOME/.microsandbox/generations/<hash12>/{db,sandboxes,run,...}`
//! plus a `$HOME/.microsandbox/current` symlink flipped atomically (tmp
//! symlink + rename, under a `.flip.lock` flock owned by the shell converge
//! script) and a per-generation `.booted-ok` marker written on first
//! verified up.
//!
//! SOCKET BUDGET: the total MSB_HOME path length is a fork hard limit of
//! 59 chars (the unix-socket paths the fork derives beneath MSB_HOME must
//! fit `sun_path`); 12-char keys keep generation paths short. Keep this in
//! mind before lengthening any name in this module.

use std::path::{Path, PathBuf};

/// The fallback generation key when the resolved msb is NOT a nix-store
/// microsandbox path: a single unmanaged generation (legacy behavior).
pub const UNMANAGED_KEY: &str = "unmanaged";

/// Length of a generation key (the 12-char prefix of the nix store hash).
/// See the module doc for the socket budget that motivates the short key.
pub const GENERATION_KEY_LEN: usize = 12;

/// Length of the full base32 hash segment of a nix store path.
const STORE_HASH_LEN: usize = 32;

/// The separator between the store hash and the package name in the baked
/// store dir basename (`<hash32>-microsandbox-<ver>`).
const STORE_NAME_MARKER: &str = "-microsandbox-";

/// Name of the per-generation container directory under the msb home root.
/// Crate-visible so the retained-generation down sweep
/// (`down_scope::retained_generation_homes`) reuses the ONE spelling.
pub(crate) const GENERATIONS_DIR_NAME: &str = "generations";

/// Name of the `current` symlink at the msb home root.
const CURRENT_LINK_NAME: &str = "current";

/// The outcome of applying the ONE home resolution rule
/// ([`resolve_msb_home_generation`]) when `MSB_HOME` is unset-or-empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HomeResolution {
    /// Rule 1: a non-empty `MSB_HOME` — used verbatim for STATE placement
    /// (never reinterpreted as a home), but the generation IDENTITY check
    /// canonicalizes it (through the `current` symlink) via
    /// [`generation_key_of_resolved_home`].
    Explicit(PathBuf),
    /// Rule 2: the `current` symlink exists — its canonical target
    /// generation dir and that dir's basename key.
    Current { gen_dir: PathBuf, key: String },
    /// Rule 3: `current` missing + exactly ONE generation dir under
    /// `generations/` — the `current` symlink was healed to it atomically
    /// (best-effort; see [`heal_current_symlink`]).
    Healed { gen_dir: PathBuf, key: String },
    /// Rule 4: `current` missing + zero generation dirs + `db/` directly at
    /// the root — a pre-generation home. Provisioning absorbs it as
    /// `generations/legacy`; runtime FAILs naming host-provision.
    LegacyRoot(PathBuf),
    /// Rule 5: `current` missing + zero generation dirs + no `db/` — a
    /// fresh machine. The carried path is the root.
    Fresh(PathBuf),
    /// Rule 6: `current` missing + MORE THAN ONE generation dir — an
    /// operator error; provisioning (host-provision) must pick. Carries the
    /// generation keys.
    Ambiguous(Vec<String>),
}

/// Derive the generation key from an msb binary path: canonicalize
/// (`readlink -f` equivalent), require the `<store-dir>/bin/msb` tail
/// shape, take the store-dir basename, split at the FIRST
/// `-microsandbox-`, and require the left segment to be exactly
/// [`STORE_HASH_LEN`] lowercase base32 chars (`[a-z0-9]`); the key is its
/// first [`GENERATION_KEY_LEN`] chars. ANY mismatch (canonicalize failure,
/// non-`bin/msb` shape, basename pattern mismatch) yields
/// [`UNMANAGED_KEY`].
pub fn generation_key_from_msb_path(msb_path: &Path) -> String {
    let canonical = match msb_path.canonicalize() {
        Ok(p) => p,
        Err(_) => return UNMANAGED_KEY.to_string(),
    };
    // The baked path shape is `<store-dir>/bin/msb`; anything else (raw
    // PATH install, a wrapper dir) is unmanaged.
    if canonical.file_name().and_then(|n| n.to_str()) != Some("msb") {
        return UNMANAGED_KEY.to_string();
    }
    let Some(store_dir) = canonical
        .parent()
        .filter(|bin| bin.file_name().and_then(|n| n.to_str()) == Some("bin"))
        .and_then(Path::parent)
    else {
        return UNMANAGED_KEY.to_string();
    };
    let Some(base) = store_dir.file_name().and_then(|n| n.to_str()) else {
        return UNMANAGED_KEY.to_string();
    };
    // `<hash32>-microsandbox-<ver>`: the separator must be present and the
    // left segment exactly STORE_HASH_LEN lowercase base32 chars.
    if !base.contains(STORE_NAME_MARKER) {
        return UNMANAGED_KEY.to_string();
    }
    let hash = base.split(STORE_NAME_MARKER).next().unwrap_or(base);
    if hash.len() != STORE_HASH_LEN || !hash.chars().all(|c| matches!(c, 'a'..='z' | '0'..='9')) {
        return UNMANAGED_KEY.to_string();
    }
    hash.chars().take(GENERATION_KEY_LEN).collect()
}

/// The generation key of the BAKED msb binary: `MSB_PATH` when non-empty,
/// else `msb` resolved on PATH (the `msb_binary()` convention in
/// `commands/doctor.rs`), fed through [`generation_key_from_msb_path`].
/// [`UNMANAGED_KEY`] when no binary resolves or the resolved binary is not
/// a nix-store microsandbox path.
pub fn baked_generation_key() -> String {
    let msb = std::env::var_os("MSB_PATH")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(resolve_msb_on_path);
    match msb {
        Some(path) => generation_key_from_msb_path(&path),
        None => UNMANAGED_KEY.to_string(),
    }
}

/// `msb` on PATH (the `MSB_PATH`-unset arm of the `msb_binary()`
/// convention): the first `msb` file found scanning `PATH`. `None` when
/// PATH is unset or holds no `msb`.
fn resolve_msb_on_path() -> Option<PathBuf> {
    let bin = crate::commands::doctor::msb_binary();
    let bin = if bin.is_empty() { "msb".to_string() } else { bin };
    if bin.contains('/') {
        let p = PathBuf::from(&bin);
        return p.is_file().then_some(p);
    }
    let path_env = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_env) {
        let candidate = dir.join(&bin);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// The msb state ROOT (`$HOME/.microsandbox`, with the SDK
/// `resolve_home`-style `.` fallback when HOME is unset) — the parent of
/// `generations/`, the `current` symlink, and `.flip.lock`. NOT the home
/// handed to msb (that is [`default_msb_home`]).
pub fn msb_home_root() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".microsandbox")
}

/// The default msb home when `MSB_HOME` is unset-or-empty:
/// `$HOME/.microsandbox/current` — the `current` generation symlink (with
/// the SDK-style `.` fallback via [`msb_home_root`]). msb resolves the
/// symlink itself, so the home it uses is the target generation dir; the
/// 12-char key keeps that path inside the 59-char socket budget (see the
/// module doc).
pub fn default_msb_home() -> PathBuf {
    msb_home_root().join(CURRENT_LINK_NAME)
}

/// The generation key of an EXPLICIT path whose basename is a 12-char
/// generation dir name; `None` for any other path (non-generation
/// overrides pass the generation gates unchecked). Pure basename check —
/// no symlink resolution (for the canonicalizing variant used by the
/// doctor row and the up gate see [`generation_key_of_resolved_home`]).
pub fn generation_key_of_path(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    (name.chars().count() == GENERATION_KEY_LEN).then(|| name.to_string())
}

/// The generation identity of an EXPLICIT (verbatim) `MSB_HOME`: canonicalize
/// (`readlink -f` equivalent — this resolves the nix wrapper's default
/// `$HOME/.microsandbox/current` symlink to its target generation dir) and
/// accept the canonical path ONLY when its parent dir basename is
/// `generations` AND its own basename is exactly [`GENERATION_KEY_LEN`]
/// lowercase base32 chars (`[a-z0-9]`). Returns the canonical generation
/// dir + its key. `None` when canonicalization fails or the target is not
/// a `generations/<key12>` dir — a genuinely verbatim unmanaged override.
///
/// `MSB_HOME` stays VERBATIM for STATE placement (msb resolves the path it
/// is handed); this helper exists so the generation IDENTITY check (doctor
/// row + fail-closed up gate) canonicalizes through the `current` symlink
/// instead of trusting the literal value — otherwise the wrapper default
/// would make the mismatch check dead code.
pub fn generation_key_of_resolved_home(path: &Path) -> Option<(PathBuf, String)> {
    let canonical = path.canonicalize().ok()?;
    let name = canonical.file_name()?.to_str()?;
    let is_key = name.len() == GENERATION_KEY_LEN
        && name.chars().all(|c| matches!(c, 'a'..='z' | '0'..='9'));
    if !is_key {
        return None;
    }
    let under_generations = canonical
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        == Some(GENERATIONS_DIR_NAME);
    if !under_generations {
        return None;
    }
    Some((canonical, name.to_string()))
}

/// Enumerate `<root>/generations/`: `(valid keys, debris names)`. A VALID
/// entry is a directory (symlink-to-dir counts — metadata follows
/// symlinks) whose name is exactly [`GENERATION_KEY_LEN`] chars. EVERYTHING
/// else (regular files, non-12-char names, unreadable entries) is debris —
/// this includes any `*.converge-tmp*` leftovers from an interrupted
/// converge. A missing/unreadable `generations/` yields two empty vecs.
/// Both vecs are sorted for stable reporting.
pub fn generation_entries(root: &Path) -> (Vec<String>, Vec<String>) {
    let mut keys = Vec::new();
    let mut debris = Vec::new();
    let entries = match std::fs::read_dir(root.join(GENERATIONS_DIR_NAME)) {
        Ok(e) => e,
        Err(_) => return (keys, debris),
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        // std::fs::metadata follows symlinks (DirEntry::metadata does not):
        // a symlink-to-dir generation counts as a directory.
        let is_dir = std::fs::metadata(entry.path())
            .map(|m| m.is_dir())
            .unwrap_or(false);
        if is_dir && name.chars().count() == GENERATION_KEY_LEN {
            keys.push(name);
        } else {
            debris.push(name);
        }
    }
    keys.sort();
    debris.sort();
    (keys, debris)
}

/// Apply the ONE home resolution rule (see the module doc): non-empty
/// `MSB_HOME` verbatim ([`HomeResolution::Explicit`]); else, rooted at
/// [`msb_home_root`]: existing `current` symlink → [`HomeResolution::Current`]
/// (its canonical target generation dir); missing `current` + exactly one
/// generation dir → heal + [`HomeResolution::Healed`]; missing `current` +
/// zero generation dirs → [`HomeResolution::LegacyRoot`] when `db/` sits at
/// the root, else [`HomeResolution::Fresh`]; missing `current` + more than
/// one generation dir → [`HomeResolution::Ambiguous`].
///
/// INTERPRETATION: a DANGLING `current` symlink (exists but does not
/// canonicalize) is treated as missing — the enumeration rules apply, and
/// a rule-3 heal atomically replaces the dangling link via rename.
pub fn resolve_msb_home_generation() -> HomeResolution {
    if let Some(path) = std::env::var_os("MSB_HOME").filter(|v| !v.is_empty()) {
        return HomeResolution::Explicit(PathBuf::from(path));
    }
    let root = msb_home_root();
    let current = root.join(CURRENT_LINK_NAME);
    // symlink_metadata: the symlink itself existing counts (canonicalize
    // below follows it to the target generation dir).
    if std::fs::symlink_metadata(&current).is_ok() {
        if let Ok(gen_dir) = std::fs::canonicalize(&current) {
            let key = gen_dir
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            return HomeResolution::Current { gen_dir, key };
        }
        // Dangling symlink: fall through to the enumeration rules.
    }
    let (keys, _debris) = generation_entries(&root);
    if keys.is_empty() {
        return if root.join("db").is_dir() {
            HomeResolution::LegacyRoot(root)
        } else {
            HomeResolution::Fresh(root)
        };
    }
    if keys.len() > 1 {
        return HomeResolution::Ambiguous(keys);
    }
    // Exactly one generation dir: heal the missing `current` symlink.
    let key = keys.into_iter().next().unwrap_or_default();
    let gen_dir = root.join(GENERATIONS_DIR_NAME).join(&key);
    heal_current_symlink(&root, &gen_dir);
    HomeResolution::Healed { gen_dir, key }
}

/// Rule-3 heal: point `current` at `gen_dir` atomically — a tmp symlink
/// `.current.tmp-<pid>` renamed over `current`. The `.flip.lock` file is
/// created (the shell converge script flocks it), but NO flock is taken
/// here: Cargo.toml carries no flock-capable dependency (fs2/libc/nix) and
/// `unsafe_code` is forbidden, so the Rust heal is best-effort
/// atomic-via-rename only — `scripts/host-provision.sh` owns the flock
/// coordination. All failures are ignored (best-effort).
#[cfg(unix)]
fn heal_current_symlink(root: &Path, gen_dir: &Path) {
    use std::os::unix::fs::symlink;
    let _ = std::fs::create_dir_all(root);
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open(root.join(".flip.lock"));
    let tmp = root.join(format!(".current.tmp-{}", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    if symlink(gen_dir, &tmp).is_ok() {
        let _ = std::fs::rename(&tmp, root.join(CURRENT_LINK_NAME));
    } else {
        let _ = std::fs::remove_file(&tmp);
    }
}

/// Non-unix stub: the symlink-flip heal is unix-only. The resolution
/// verdict ([`HomeResolution::Healed`]) is still returned — the shell
/// converge script owns real provisioning.
#[cfg(not(unix))]
fn heal_current_symlink(_root: &Path, _gen_dir: &Path) {}
