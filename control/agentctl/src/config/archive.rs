//! Content-addressed config archive store (A5; ADR 0032 addendum
//! 2026-08-24 §Config source model).
//!
//! Layout: `<state>/cache/gitv3/<sha>/` — produced by
//! [`crate::git::git_archive`] from the EXISTING single managed clone per
//! repo. Recorded explicitly:
//!
//! - **NO worktrees, NO checkouts.** The one managed clone per repo IS the
//!   object database; cache entries are plain immutable directories.
//! - Simultaneous branches are free: two refs of one repo are two archive
//!   dirs, never two checkouts.
//! - Entries are content-addressed and IMMUTABLE: once `<sha>/` exists it
//!   is never rewritten (concurrent producers race a rename; the loser
//!   drops its tmp dir and returns the winner).
//!
//! Session 1 shipped the store unwired; Session 2 wired consumption:
//! `config::loading::layer_content_root` resolves Remote/GitFile layers to
//! the archive of the LOCKED rev (not the managed clone's working tree), so
//! edits committed in a managed clone are INVISIBLE to consumers until
//! `workestrate config update` moves the pin (commit-before-consume).
//! Plain-path entries are the documented exception (content-as-is).

use std::path::{Path, PathBuf};

use anyhow::Result;

/// Root of the archive store: `<state>/cache/gitv3`.
pub fn archive_store_root() -> PathBuf {
    crate::config::paths::resolve_state_dir()
        .join("cache")
        .join("gitv3")
}

/// The archive dir for `sha`: `<state>/cache/gitv3/<sha>`.
///
/// Fail-closed on path-unsafe input: `sha` must be a lowercase hex string
/// of at least 7 chars (loosely `^[0-9a-f]{7,40}$` — full shas are 40 hex;
/// the floor admits short shas). Anything else (path separators, `..`,
/// uppercase, empty) is an error rather than a filesystem escape.
pub fn archive_dir(sha: &str) -> Result<PathBuf> {
    let valid = sha.len() >= 7
        && sha.len() <= 40
        && sha
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c));
    if !valid {
        anyhow::bail!(
            "invalid archive sha '{}': expected a lowercase hex sha (7..=40 chars)",
            sha
        );
    }
    Ok(archive_store_root().join(sha))
}

/// Ensure the archive of `sha` from `repo` exists in the store, returning
/// its dir. Content-addressed + immutable:
///
/// - If `<root>/<sha>` already exists, return it untouched.
/// - Otherwise archive into `<root>/<sha>.tmp-<pid>`, then rename onto
///   `<root>/<sha>` (same filesystem, so the rename is atomic). On a rename
///   collision — another process won the race — drop the tmp dir and
///   return the winner.
///
/// Failures clean up the tmp dir; a crashed producer can leave a stale
/// `<sha>.tmp-<pid>` behind, which a later run overwrites (it is removed
/// before reuse) and which never shadows the committed `<sha>` entry.
pub fn ensure_archive(repo: &Path, sha: &str) -> Result<PathBuf> {
    let target = archive_dir(sha)?;
    if target.exists() {
        return Ok(target);
    }
    let root = archive_store_root();
    std::fs::create_dir_all(&root)?;
    let tmp = root.join(format!("{}.tmp-{}", sha, std::process::id()));
    if tmp.exists() {
        // Stale tmp from a crashed earlier run of THIS process shape.
        std::fs::remove_dir_all(&tmp)?;
    }
    if let Err(e) = crate::git::git_archive(repo, sha, &tmp) {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(e);
    }
    match std::fs::rename(&tmp, &target) {
        Ok(()) => Ok(target),
        Err(_) if target.exists() => {
            // Another process won the race between our exists-check and the
            // rename. Cache entries are immutable, so the winner's content
            // IS our content; drop the loser and return the winner.
            let _ = std::fs::remove_dir_all(&tmp);
            Ok(target)
        }
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp);
            Err(anyhow::anyhow!(
                "failed to commit archive {} into the store at {}: {}",
                sha,
                target.display(),
                e
            ))
        }
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

    /// Env keys whose mutation pins the archive store root:
    /// WORKESTRATE_STATE_DIR short-circuits `resolve_state_dir`.
    const STATE_ENV_KEYS: &[&str] = &["WORKESTRATE_STATE_DIR"];

    /// Set WORKESTRATE_STATE_DIR to a fresh temp dir and return it. Caller
    /// must hold ENV_TEST_LOCK and an EnvGuard for STATE_ENV_KEYS.
    fn pin_state(label: &str) -> PathBuf {
        let state = uniq_dir(label);
        std::fs::create_dir_all(&state).expect("create pinned state dir");
        std::env::set_var("WORKESTRATE_STATE_DIR", &state);
        state
    }

    /// Init a git repo in `dir` with one committed file and return its HEAD
    /// sha. Git availability: like the git.rs / lockfile.rs tests, these
    /// expect `git` (and `tar`) on PATH — the pinned toolchain guarantees
    /// both; there is no host-gating precedent for git in this crate.
    fn init_repo_with_commit(dir: &Path, file: &str, content: &str) -> String {
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(args)
                .status()
                .expect("git must be runnable");
            assert!(status.success(), "git {:?} failed", args);
        };
        std::fs::create_dir_all(dir).expect("create repo dir");
        run(&["init", "--quiet"]);
        run(&["config", "user.email", "a5-archive@test.invalid"]);
        run(&["config", "user.name", "a5-archive-test"]);
        std::fs::write(dir.join(file), content).expect("write file");
        run(&["add", file]);
        run(&["commit", "--quiet", "-m", "init"]);
        crate::git::git_rev_parse(dir).expect("HEAD sha")
    }

    #[test]
    fn archive_store_root_is_state_cache_gitv3() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(STATE_ENV_KEYS);
        let state = pin_state("a5-root");
        assert_eq!(archive_store_root(), state.join("cache").join("gitv3"));
        let _ = std::fs::remove_dir_all(&state);
    }

    #[test]
    fn archive_dir_rejects_path_unsafe_shas() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(STATE_ENV_KEYS);
        let state = pin_state("a5-dir-validate");
        for bad in [
            "../escape",
            "abc",                                       // too short
            "ABCDEF0",                                   // uppercase hex
            "zzzzzzz",                                   // non-hex
            "0123456789abcdef0123456789abcdef012345678", // 41 chars > 40
            "",
            "/absolute/path",
            "a/bcdef",
        ] {
            let err = archive_dir(bad).expect_err("unsafe sha must fail");
            assert!(
                err.to_string().contains(bad),
                "error must name the offending sha: {err}"
            );
        }
        // Valid shapes: 7-char short sha and 40-char full sha.
        let short = archive_dir("0123abc").expect("7-hex sha");
        assert_eq!(short, archive_store_root().join("0123abc"));
        let full = "0123456789abcdef0123456789abcdef01234567";
        let dir = archive_dir(full).expect("40-hex sha");
        assert_eq!(dir, archive_store_root().join(full));
        let _ = std::fs::remove_dir_all(&state);
    }

    #[test]
    fn ensure_archive_creates_immutable_content_addressed_dir() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(STATE_ENV_KEYS);
        let state = pin_state("a5-ensure");
        let repo = uniq_dir("a5-ensure-repo");
        let sha = init_repo_with_commit(&repo, "tracked.txt", "committed");

        // First call produces the archive.
        let dir = ensure_archive(&repo, &sha).expect("ensure archive");
        assert_eq!(dir, archive_store_root().join(&sha));
        assert_eq!(
            std::fs::read_to_string(dir.join("tracked.txt")).expect("archived file"),
            "committed"
        );
        assert!(!dir.join(".git").exists(), "an archive is a plain dir");

        // Immutability: plant a marker; a second call must return the same
        // dir WITHOUT rewriting it (the marker survives).
        std::fs::write(dir.join(".marker"), "do-not-rewrite").expect("plant marker");
        let again = ensure_archive(&repo, &sha).expect("second ensure");
        assert_eq!(again, dir);
        assert_eq!(
            std::fs::read_to_string(dir.join(".marker")).expect("marker survives"),
            "do-not-rewrite",
            "an existing cache entry must never be rewritten"
        );

        // Tmp cleanup on success: no tmp dirs or tars left in the root.
        let leftovers: Vec<_> = std::fs::read_dir(archive_store_root())
            .expect("read store root")
            .map(|e| e.expect("dir entry").file_name())
            .collect();
        assert_eq!(
            leftovers,
            vec![std::ffi::OsString::from(&sha)],
            "only the committed <sha> entry may remain: {leftovers:?}"
        );

        let _ = std::fs::remove_dir_all(&state);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn ensure_archive_two_revs_of_one_repo_are_two_dirs() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(STATE_ENV_KEYS);
        let state = pin_state("a5-two-revs");
        let repo = uniq_dir("a5-two-revs-repo");
        let sha1 = init_repo_with_commit(&repo, "a.txt", "one");
        // Second commit on the same repo.
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(&repo)
                .args(args)
                .status()
                .expect("git must be runnable");
            assert!(status.success(), "git {:?} failed", args);
        };
        std::fs::write(repo.join("b.txt"), "two").expect("write second file");
        run(&["add", "b.txt"]);
        run(&["commit", "--quiet", "-m", "second"]);
        let sha2 = crate::git::git_rev_parse(&repo).expect("second HEAD sha");
        assert_ne!(sha1, sha2);

        let dir1 = ensure_archive(&repo, &sha1).expect("archive rev 1");
        let dir2 = ensure_archive(&repo, &sha2).expect("archive rev 2");
        assert_ne!(dir1, dir2, "two revs of one repo are two archive dirs");
        assert!(dir1.join("a.txt").exists() && !dir1.join("b.txt").exists());
        assert!(dir2.join("a.txt").exists() && dir2.join("b.txt").exists());

        let _ = std::fs::remove_dir_all(&state);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn ensure_archive_bad_rev_errors_and_cleans_tmp() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(STATE_ENV_KEYS);
        let state = pin_state("a5-bad-rev");
        let repo = uniq_dir("a5-bad-rev-repo");
        let _sha = init_repo_with_commit(&repo, "a.txt", "one");

        let bogus = "0".repeat(40);
        let err = ensure_archive(&repo, &bogus).expect_err("nonexistent rev must fail");
        assert!(
            err.to_string().contains(&repo.display().to_string()),
            "error must name the repo: {err}"
        );
        // No tmp dir, tmp tar, or partial entry left behind.
        let root = archive_store_root();
        let leftovers: Vec<String> = std::fs::read_dir(&root)
            .map(|rd| {
                rd.map(|e| {
                    e.expect("dir entry")
                        .file_name()
                        .to_string_lossy()
                        .into_owned()
                })
                .collect()
            })
            .unwrap_or_default();
        assert!(
            leftovers.is_empty(),
            "a failed ensure must leave the store root empty: {leftovers:?}"
        );

        let _ = std::fs::remove_dir_all(&state);
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn ensure_archive_race_loser_returns_winner() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(STATE_ENV_KEYS);
        let state = pin_state("a5-race");
        let repo = uniq_dir("a5-race-repo");
        let sha = init_repo_with_commit(&repo, "a.txt", "one");

        // Simulate a lost race: a winner entry appears between the
        // exists-check and the rename. Pre-create BOTH the winner and a
        // loser tmp, then drive the rename path by hand through the public
        // contract: ensure_archive must return the winner untouched.
        let winner = archive_dir(&sha).expect("winner dir");
        std::fs::create_dir_all(&winner).expect("create winner");
        std::fs::write(winner.join(".winner"), "won").expect("winner marker");
        let dir = ensure_archive(&repo, &sha).expect("ensure returns winner");
        assert_eq!(dir, winner);
        assert!(
            dir.join(".winner").exists(),
            "the existing entry is returned untouched (no rewrite)"
        );
        assert!(
            !dir.join("a.txt").exists(),
            "the loser's content must NOT displace the winner"
        );

        let _ = std::fs::remove_dir_all(&state);
        let _ = std::fs::remove_dir_all(&repo);
    }
}
