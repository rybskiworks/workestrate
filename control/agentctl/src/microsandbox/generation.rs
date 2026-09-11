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

/// The FIRST-CLASS generation name of the absorbed pre-generation home
/// (`generations/legacy`): host-provision moves a legacy root (`db/` at
/// `$HOME/.microsandbox`) here, so the ONE rule counts it as a generation
/// exactly like a key12 dir (the shell converge treats ANY dir under
/// `generations/` as one — the Rust rule must agree). It is a converge
/// SOURCE, never a target: [`baked_generation_key`] can never be `legacy`
/// (it is always a 12-char store-hash prefix or [`UNMANAGED_KEY`]), so a
/// Current/Healed legacy generation ALWAYS mismatches the baked key and
/// the doctor row / up gate refuse naming host-provision.
pub const LEGACY_KEY: &str = "legacy";

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
    let bin = if bin.is_empty() {
        "msb".to_string()
    } else {
        bin
    };
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

/// The ONE generation-name spelling: true for exactly
/// [`GENERATION_KEY_LEN`] lowercase base32 chars (`[a-z0-9]`) OR exactly
/// [`LEGACY_KEY`] (the absorbed pre-generation home is a first-class
/// generation). Used by [`generation_entries`] — and through it the
/// resolution counting, the Ambiguous key listing, the doctor debris WARN,
/// and the retained-generation down sweep — plus the explicit-home key
/// helpers below. Never re-implement the shape check elsewhere.
pub fn is_generation_name(name: &str) -> bool {
    name == LEGACY_KEY
        || (name.len() == GENERATION_KEY_LEN
            && name.chars().all(|c| matches!(c, 'a'..='z' | '0'..='9')))
}

/// The generation key of an EXPLICIT path whose basename is a generation
/// dir name ([`is_generation_name`]); `None` for any other path
/// (non-generation overrides pass the generation gates unchecked). Pure
/// basename check — no symlink resolution (for the canonicalizing variant
/// used by the doctor row and the up gate see
/// [`generation_key_of_resolved_home`]).
pub fn generation_key_of_path(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    is_generation_name(name).then(|| name.to_string())
}

/// The generation identity of an EXPLICIT (verbatim) `MSB_HOME`: canonicalize
/// (`readlink -f` equivalent — this resolves the nix wrapper's default
/// `$HOME/.microsandbox/current` symlink to its target generation dir) and
/// accept the canonical path ONLY when its parent dir basename is
/// `generations` AND its own basename is a generation name
/// ([`is_generation_name`] — a 12-char base32 key or `legacy`). Returns the
/// canonical generation dir + its key. `None` when canonicalization fails
/// or the target is not a `generations/<name>` dir — a genuinely verbatim
/// unmanaged override.
///
/// `MSB_HOME` stays VERBATIM for STATE placement (msb resolves the path it
/// is handed); this helper exists so the generation IDENTITY check (doctor
/// row + fail-closed up gate) canonicalizes through the `current` symlink
/// instead of trusting the literal value — otherwise the wrapper default
/// would make the mismatch check dead code.
pub fn generation_key_of_resolved_home(path: &Path) -> Option<(PathBuf, String)> {
    let canonical = path.canonicalize().ok()?;
    // Bind the OWNED key string first — a `&str` borrow of `canonical`
    // must not be live when `canonical` moves into the return tuple.
    let name = canonical.file_name()?.to_str()?.to_string();
    if !is_generation_name(&name) {
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
    Some((canonical, name))
}

/// Enumerate `<root>/generations/`: `(valid keys, debris names)`. A VALID
/// entry is a directory (symlink-to-dir counts — metadata follows
/// symlinks) whose name satisfies [`is_generation_name`] (a 12-char base32
/// key OR `legacy` — the absorbed pre-generation home is first-class, so
/// `legacy` alone heals and `legacy` + a key12 is Ambiguous, matching the
/// shell converge). EVERYTHING else (regular files, non-conforming names —
/// including 12-char non-base32 ones — unreadable entries) is debris; this
/// includes any `*.converge-tmp*` leftovers from an interrupted converge.
/// A missing/unreadable `generations/` yields two empty vecs. Both vecs
/// are sorted for stable reporting.
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
        if is_dir && is_generation_name(&name) {
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
/// one generation dir → [`HomeResolution::Ambiguous`]. "Generation dir"
/// means [`is_generation_name`]: `generations/legacy` (the absorbed
/// pre-generation home) COUNTS — legacy alone heals to `legacy`, legacy +
/// a key12 is Ambiguous, matching the shell converge. A Current/Healed
/// `legacy` generation always mismatches the baked key
/// ([`baked_generation_key`] can never be `legacy` — it is a converge
/// SOURCE, never a target), so the doctor row / up gate refuse naming
/// host-provision with no special-casing here.
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
    if std::fs::symlink_metadata(&current).is_ok()
        && let Ok(gen_dir) = std::fs::canonicalize(&current)
    {
        let key = gen_dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        return HomeResolution::Current { gen_dir, key };
    }
    // Dangling symlink: fall through to the enumeration rules.
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
    // Lock-file existence only (the shell converge script flocks it — its
    // content is irrelevant), so truncate is explicitly OFF: an existing
    // lock file must survive the touch (clippy::suspicious_open_options).
    let _ = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
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

//--------------------------------------------------------------------------------------------------
// Tests
//--------------------------------------------------------------------------------------------------

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
#[allow(unsafe_code)]
mod tests {
    use super::*;
    use crate::config::test_support::{ENV_TEST_LOCK, EnvGuard, uniq_dir};

    // ---- key derivation (generation_key_from_msb_path) ----

    /// 32-char lowercase base32 store-hash segment shared by the derivation
    /// tests; the expected key is its 12-char prefix.
    const HASH32: &str = "0123456789abcdef0123456789abcdef";
    const KEY12: &str = "0123456789ab";

    /// Create `<root>/nix/store/<hash>-microsandbox-0.6.16/bin/msb` as a real
    /// file (so canonicalize succeeds) and return the msb path.
    fn fake_store_msb(root: &Path, hash: &str) -> PathBuf {
        let bin_dir = root
            .join("nix")
            .join("store")
            .join(format!("{hash}-microsandbox-0.6.16"))
            .join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let msb = bin_dir.join("msb");
        std::fs::write(&msb, b"#!/bin/sh\n").unwrap();
        msb
    }

    #[test]
    fn store_path_msb_key_is_hash_prefix() {
        let root = uniq_dir("gen-key-store");
        let msb = fake_store_msb(&root, HASH32);
        assert_eq!(generation_key_from_msb_path(&msb), KEY12);
        let _ = std::fs::remove_dir_all(root);
    }

    /// A SYMLINKED msb path canonicalizes to the store target; the key
    /// derives from the resolved target, never from the link's own path.
    #[cfg(unix)]
    #[test]
    fn symlinked_msb_key_derives_from_resolved_target() {
        use std::os::unix::fs::symlink;
        let root = uniq_dir("gen-key-symlink");
        let real = fake_store_msb(&root, HASH32);
        let link_dir = root.join("wrappers").join("bin");
        std::fs::create_dir_all(&link_dir).unwrap();
        let link = link_dir.join("msb");
        symlink(&real, &link).unwrap();
        assert_eq!(generation_key_from_msb_path(&link), KEY12);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn uppercase_hash_segment_is_unmanaged() {
        let root = uniq_dir("gen-key-upper");
        let msb = fake_store_msb(&root, "0123456789ABCDEF0123456789abcdef");
        assert_eq!(generation_key_from_msb_path(&msb), UNMANAGED_KEY);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn thirty_one_char_hash_segment_is_unmanaged() {
        let root = uniq_dir("gen-key-31");
        let msb = fake_store_msb(&root, "0123456789abcdef0123456789abcde");
        assert_eq!(generation_key_from_msb_path(&msb), UNMANAGED_KEY);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn thirty_three_char_hash_segment_is_unmanaged() {
        let root = uniq_dir("gen-key-33");
        let msb = fake_store_msb(&root, "0123456789abcdef0123456789abcdef0");
        assert_eq!(generation_key_from_msb_path(&msb), UNMANAGED_KEY);
        let _ = std::fs::remove_dir_all(root);
    }

    /// A real msb-shaped path WITHOUT the `-microsandbox-` store marker
    /// (raw PATH install shape) is unmanaged.
    #[test]
    fn non_store_path_is_unmanaged() {
        let root = uniq_dir("gen-key-nonstore");
        let bin_dir = root.join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let msb = bin_dir.join("msb");
        std::fs::write(&msb, b"#!/bin/sh\n").unwrap();
        assert_eq!(generation_key_from_msb_path(&msb), UNMANAGED_KEY);
        let _ = std::fs::remove_dir_all(root);
    }

    /// Canonicalize failure (missing file) is unmanaged.
    #[test]
    fn missing_msb_file_is_unmanaged() {
        let root = uniq_dir("gen-key-missing");
        let ghost = root
            .join("nix")
            .join("store")
            .join(format!("{HASH32}-microsandbox-0.6.16"))
            .join("bin")
            .join("msb");
        assert_eq!(generation_key_from_msb_path(&ghost), UNMANAGED_KEY);
        let _ = std::fs::remove_dir_all(root);
    }

    /// Tail-shape rule: a store path NOT ending in `<store-dir>/bin/msb`
    /// (e.g. the sibling `libexec/agentd`) is unmanaged, even though its
    /// store-dir basename carries a valid hash — the baked identity comes
    /// ONLY from the msb binary itself.
    #[test]
    fn store_path_without_bin_msb_tail_is_unmanaged() {
        let root = uniq_dir("gen-key-tail");
        let libexec = root
            .join("nix")
            .join("store")
            .join(format!("{HASH32}-microsandbox-0.6.16"))
            .join("libexec");
        std::fs::create_dir_all(&libexec).unwrap();
        let agentd = libexec.join("agentd");
        std::fs::write(&agentd, b"#!/bin/sh\n").unwrap();
        assert_eq!(generation_key_from_msb_path(&agentd), UNMANAGED_KEY);
        let _ = std::fs::remove_dir_all(root);
    }

    // ---- resolution rule matrix (resolve_msb_home_generation) ----
    //
    // These tests mutate process env (MSB_HOME/HOME), so they serialize on
    // ENV_TEST_LOCK and restore via EnvGuard (the reconcile/policy_file
    // pattern).

    /// Pin HOME to a fresh temp root with MSB_HOME removed; the returned
    /// guards MUST stay alive for the test body.
    fn pin_home(label: &str) -> (std::sync::MutexGuard<'static, ()>, EnvGuard, PathBuf) {
        let lock = ENV_TEST_LOCK.lock().unwrap();
        let guard = EnvGuard::capture(&["MSB_HOME", "HOME"]);
        let home = uniq_dir(label);
        std::fs::create_dir_all(&home).unwrap();
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("MSB_HOME") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", &home) };
        (lock, guard, home)
    }

    /// Rule 1: a non-empty MSB_HOME is Explicit, verbatim (never
    /// canonicalized, never required to exist).
    #[test]
    fn explicit_msb_home_is_verbatim() {
        let (_lock, _guard, home) = pin_home("gen-resolve-explicit");
        let explicit = home.join("somewhere").join("else");
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("MSB_HOME", &explicit) };
        match resolve_msb_home_generation() {
            HomeResolution::Explicit(p) => assert_eq!(p, explicit),
            other => panic!("expected Explicit, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(home);
    }

    /// Rule 1 edge: an EMPTY MSB_HOME is treated as unset and falls through
    /// to the enumeration rules (a fresh root here).
    #[test]
    fn empty_msb_home_is_treated_as_unset() {
        let (_lock, _guard, home) = pin_home("gen-resolve-empty");
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("MSB_HOME", "") };
        match resolve_msb_home_generation() {
            HomeResolution::Fresh(root) => assert_eq!(root, home.join(".microsandbox")),
            other => panic!("expected Fresh, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(home);
    }

    /// The unset-MSB_HOME default home is the `current` symlink under the
    /// msb home root (the wrapper default change).
    #[test]
    fn default_home_is_current_symlink() {
        let (_lock, _guard, home) = pin_home("gen-default-home");
        assert_eq!(
            default_msb_home(),
            home.join(".microsandbox").join("current")
        );
        let _ = std::fs::remove_dir_all(home);
    }

    /// Rule 2: an existing `current` symlink resolves to its target
    /// generation dir and that dir's basename key.
    #[cfg(unix)]
    #[test]
    fn current_symlink_resolves_to_generation() {
        use std::os::unix::fs::symlink;
        let (_lock, _guard, home) = pin_home("gen-resolve-current");
        let root = home.join(".microsandbox");
        let gen_dir = root.join("generations").join(KEY12);
        std::fs::create_dir_all(&gen_dir).unwrap();
        symlink(&gen_dir, root.join("current")).unwrap();
        match resolve_msb_home_generation() {
            HomeResolution::Current { gen_dir: got, key } => {
                assert_eq!(got, gen_dir.canonicalize().unwrap());
                assert_eq!(key, KEY12);
            }
            other => panic!("expected Current, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(home);
    }

    /// Rule 3: `current` missing + exactly ONE generation dir heals the
    /// symlink and reports Healed.
    #[cfg(unix)]
    #[test]
    fn single_generation_heals_missing_current() {
        let (_lock, _guard, home) = pin_home("gen-resolve-heal");
        let root = home.join(".microsandbox");
        let gen_dir = root.join("generations").join(KEY12);
        std::fs::create_dir_all(&gen_dir).unwrap();
        match resolve_msb_home_generation() {
            HomeResolution::Healed { gen_dir: got, key } => {
                assert_eq!(got, gen_dir);
                assert_eq!(key, KEY12);
            }
            other => panic!("expected Healed, got {other:?}"),
        }
        // The heal is real: `current` now resolves to the generation dir.
        assert_eq!(root.join("current").canonicalize().unwrap(), gen_dir);
        let _ = std::fs::remove_dir_all(home);
    }

    /// Rule 4: `current` missing + zero generation dirs + `db/` at the root
    /// is a pre-generation home.
    #[test]
    fn legacy_root_db_is_pre_generation_home() {
        let (_lock, _guard, home) = pin_home("gen-resolve-legacy");
        let root = home.join(".microsandbox");
        std::fs::create_dir_all(root.join("db")).unwrap();
        match resolve_msb_home_generation() {
            HomeResolution::LegacyRoot(got) => assert_eq!(got, root),
            other => panic!("expected LegacyRoot, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(home);
    }

    /// Rule 5: `current` missing + zero generation dirs + no `db/` is fresh.
    #[test]
    fn no_db_no_generations_is_fresh() {
        let (_lock, _guard, home) = pin_home("gen-resolve-fresh");
        match resolve_msb_home_generation() {
            HomeResolution::Fresh(root) => assert_eq!(root, home.join(".microsandbox")),
            other => panic!("expected Fresh, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(home);
    }

    /// Rule 6: `current` missing + MORE THAN ONE generation dir is
    /// Ambiguous, naming both keys (sorted).
    #[test]
    fn two_generations_without_current_is_ambiguous() {
        let (_lock, _guard, home) = pin_home("gen-resolve-ambiguous");
        let gens = home.join(".microsandbox").join("generations");
        std::fs::create_dir_all(gens.join("bbbbbbbbbbbb")).unwrap();
        std::fs::create_dir_all(gens.join("aaaaaaaaaaaa")).unwrap();
        match resolve_msb_home_generation() {
            HomeResolution::Ambiguous(keys) => {
                assert_eq!(keys, vec!["aaaaaaaaaaaa", "bbbbbbbbbbbb"])
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(home);
    }

    /// `legacy` is a FIRST-CLASS generation: `generations/legacy` alone +
    /// no `current` heals the symlink to it (matching the shell converge),
    /// reported as Healed with key "legacy". (The doctor row / up gate then
    /// FAIL/refuse on the inevitable baked-key mismatch — `legacy` is a
    /// converge SOURCE, never a target.)
    #[cfg(unix)]
    #[test]
    fn legacy_only_generation_heals_missing_current() {
        let (_lock, _guard, home) = pin_home("gen-resolve-legacy-heal");
        let root = home.join(".microsandbox");
        let gen_dir = root.join("generations").join(LEGACY_KEY);
        std::fs::create_dir_all(&gen_dir).unwrap();
        match resolve_msb_home_generation() {
            HomeResolution::Healed { gen_dir: got, key } => {
                assert_eq!(got, gen_dir);
                assert_eq!(key, LEGACY_KEY);
            }
            other => panic!("expected Healed, got {other:?}"),
        }
        assert_eq!(root.join("current").canonicalize().unwrap(), gen_dir);
        let _ = std::fs::remove_dir_all(home);
    }

    /// `legacy` + one key12 + no `current` is Ambiguous (matching the shell
    /// converge), NOT a heal — both names listed, sorted.
    #[test]
    fn legacy_and_key12_without_current_is_ambiguous() {
        let (_lock, _guard, home) = pin_home("gen-resolve-legacy-ambiguous");
        let gens = home.join(".microsandbox").join("generations");
        std::fs::create_dir_all(gens.join(KEY12)).unwrap();
        std::fs::create_dir_all(gens.join(LEGACY_KEY)).unwrap();
        match resolve_msb_home_generation() {
            HomeResolution::Ambiguous(keys) => {
                assert_eq!(keys, vec![KEY12, LEGACY_KEY])
            }
            other => panic!("expected Ambiguous, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(home);
    }

    /// A DANGLING `current` symlink (exists but does not canonicalize) is
    /// treated as missing: the enumeration rules apply, and the rule-3 heal
    /// atomically REPLACES the dangling link.
    #[cfg(unix)]
    #[test]
    fn dangling_current_is_treated_as_missing() {
        use std::os::unix::fs::symlink;
        let (_lock, _guard, home) = pin_home("gen-resolve-dangling");
        let root = home.join(".microsandbox");
        let gen_dir = root.join("generations").join(KEY12);
        std::fs::create_dir_all(&gen_dir).unwrap();
        symlink(
            root.join("generations").join("999999999999"),
            root.join("current"),
        )
        .unwrap();
        match resolve_msb_home_generation() {
            HomeResolution::Healed { gen_dir: got, key } => {
                assert_eq!(got, gen_dir);
                assert_eq!(key, KEY12);
            }
            other => panic!("expected Healed, got {other:?}"),
        }
        // The dangling link was replaced by the heal.
        assert_eq!(root.join("current").canonicalize().unwrap(), gen_dir);
        let _ = std::fs::remove_dir_all(home);
    }

    /// The ONE name spelling: 12-char lowercase base32 or exactly `legacy`
    /// are generation names; everything else is not.
    #[test]
    fn is_generation_name_covers_key12_and_legacy() {
        assert!(is_generation_name(KEY12));
        assert!(is_generation_name(LEGACY_KEY));
        for non in ["", "abcde", "ABCDEFGHIJKL", "unmanaged", "legacyy", "legac"] {
            assert!(
                !is_generation_name(non),
                "{non} must not be a generation name"
            );
        }
    }

    /// generation_entries: a valid 12-char DIRECTORY counts as a key, and
    /// so does the first-class `legacy` dir (the absorbed pre-generation
    /// home); wrong length names, 12-char NON-base32 names,
    /// `.converge-tmp-*` staging leftovers, and regular files (even 12-char
    /// ones) are debris.
    #[test]
    fn generation_entries_separates_valid_keys_from_debris() {
        let root = uniq_dir("gen-entries");
        let gens = root.join("generations");
        std::fs::create_dir_all(gens.join(KEY12)).unwrap();
        std::fs::create_dir_all(gens.join(LEGACY_KEY)).unwrap();
        std::fs::create_dir_all(gens.join("abcde")).unwrap();
        std::fs::create_dir_all(gens.join("ABCDEFGHIJKL")).unwrap();
        std::fs::create_dir_all(gens.join(".converge-tmp-xyz")).unwrap();
        std::fs::write(gens.join("ffffffffffff"), b"regular file").unwrap();
        std::fs::write(gens.join("notes.txt"), b"debris").unwrap();
        let (keys, debris) = generation_entries(&root);
        assert_eq!(keys, vec![KEY12, LEGACY_KEY]);
        assert_eq!(
            debris,
            vec![
                ".converge-tmp-xyz",
                "ABCDEFGHIJKL",
                "abcde",
                "ffffffffffff",
                "notes.txt"
            ]
        );
        let _ = std::fs::remove_dir_all(root);
    }

    /// A missing `generations/` dir yields two empty vecs (no error).
    #[test]
    fn generation_entries_of_missing_dir_is_empty() {
        let root = uniq_dir("gen-entries-missing");
        let (keys, debris) = generation_entries(&root);
        assert!(keys.is_empty());
        assert!(debris.is_empty());
        let _ = std::fs::remove_dir_all(root);
    }
}
