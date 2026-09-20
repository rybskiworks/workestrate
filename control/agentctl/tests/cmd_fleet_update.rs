//! Integration tests for `workestrate fleet update` — the dirty-clone
//! guard: a registered clone with uncommitted changes (modified tracked
//! files OR untracked-only) is refused BEFORE any pull, and the registry
//! on disk is left untouched. Uses an isolated HOME + XDG per test so the
//! user's real registry is never touched.
//!
//! A5 review LOW-3: the pull ref must come from the SAME effective-ref
//! ladder consumption uses (explicit `ref` > origin/HEAD of the managed
//! clone), not a hardcoded "main" — covered by the `update_ref_less_*`
//! tests below.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::IsolatedHome;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Run git in `dir` with the system config disabled; assert success.
fn run_git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("spawn git");
    assert!(
        status.success(),
        "git {:?} failed in {}",
        args,
        dir.display()
    );
}

/// Run git in `dir`; return trimmed stdout. Asserts success.
fn git_stdout(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("spawn git");
    assert!(
        out.status.success(),
        "git {:?} failed in {}: {}",
        args,
        dir.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Create a source repo at `dir` on branch `branch` with one committed
/// workestrate.toml; return the tip sha. `dir` should end in `.git` so the
/// registry url classifies as a git remote (not a local path).
fn source_repo(dir: &Path, branch: &str) -> String {
    std::fs::create_dir_all(dir).expect("create source repo");
    run_git(dir, &["init", "--quiet", "-b", branch]);
    run_git(dir, &["config", "user.email", "test@example.com"]);
    run_git(dir, &["config", "user.name", "Test"]);
    std::fs::write(dir.join("workestrate.toml"), "schema_version = 1\n").expect("write config");
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "--quiet", "-m", "init"]);
    git_stdout(dir, &["rev-parse", "HEAD"])
}

/// Commit one more rev on top of `repo`; return the new tip sha.
fn advance_repo(repo: &Path) -> String {
    std::fs::write(repo.join("notes.txt"), "advance").expect("write advance file");
    run_git(repo, &["add", "."]);
    run_git(repo, &["commit", "--quiet", "-m", "advance"]);
    git_stdout(repo, &["rev-parse", "HEAD"])
}

/// Pin WORKESTRATE_CONFIG at `<dir>/.workestrate` and return it (registry =
/// `<dir>/config.toml`, clone store = `<dir>/fleets/<name>`, lock =
/// `<dir>/workestrate.lock`).
fn pinned_config(home: &IsolatedHome) -> PathBuf {
    let dir = home.dir.join(".workestrate");
    std::fs::create_dir_all(&dir).expect("create pinned config");
    dir
}

/// A5 review LOW-3: a ref-less registry entry pulls the managed clone's
/// origin/HEAD branch — the SAME effective ref consumption resolves — not a
/// hardcoded "main". Origin's default branch here is `trunk`; pre-fix this
/// pulled "main" and failed outright.
#[test]
fn update_ref_less_entry_pulls_the_origin_default_branch() {
    let home = IsolatedHome::new("cmd-fleet-update");
    let store = pinned_config(&home);
    let src = home.dir.join("src-team.git");
    source_repo(&src, "trunk");
    // The managed clone: a real clone so origin/HEAD resolves to trunk.
    let repos_dir = store.join("fleets");
    std::fs::create_dir_all(&repos_dir).expect("create fleets");
    run_git(
        &repos_dir,
        &["clone", "--quiet", src.to_str().expect("utf8 src"), "team"],
    );
    // Registry entry with NO ref field: consumption resolves origin/HEAD.
    std::fs::write(
        store.join("config.toml"),
        format!(
            "layers = [\"team\"]\n\n[fleets.team]\nurl = \"{}\"\n",
            src.display()
        ),
    )
    .expect("write registry");
    // Advance the origin AFTER the clone: update must pull trunk forward.
    let new_sha = advance_repo(&src);

    let out = home
        .cmd()
        .env("WORKESTRATE_CONFIG", &store)
        .args(["fleet", "update", "team"])
        .output()
        .expect("invoke fleet update");
    assert!(
        out.status.success(),
        "ref-less update must pull origin/HEAD (trunk); stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("team: updated to"),
        "expected an update line; got: {stdout}"
    );
    let registry = std::fs::read_to_string(store.join("config.toml")).expect("read registry");
    assert!(
        registry.contains(&format!("rev = \"{new_sha}\"")),
        "registry must record the pulled trunk tip ({new_sha}):\n{registry}"
    );
    let lock = std::fs::read_to_string(store.join("workestrate.lock")).expect("read lock");
    assert!(
        lock.contains(&new_sha),
        "the lock must pin the pulled rev ({new_sha}):\n{lock}"
    );
}

/// A5 review LOW-3 (fail-closed edge): a ref-less entry whose managed clone
/// has NO origin/HEAD cannot resolve a default ref — the update refuses
/// BEFORE any pull, naming the repo and the remediation.
#[test]
fn update_ref_less_entry_without_origin_head_fails_with_remediation() {
    let home = IsolatedHome::new("cmd-fleet-update");
    let store = pinned_config(&home);
    // A bare `git init` clone: no origin remote at all, so no origin/HEAD.
    let dest = store.join("fleets").join("team");
    std::fs::create_dir_all(&dest).expect("create clone dir");
    run_git(&dest, &["init", "--quiet", "-b", "trunk"]);
    run_git(&dest, &["config", "user.email", "test@example.com"]);
    run_git(&dest, &["config", "user.name", "Test"]);
    std::fs::write(dest.join("workestrate.toml"), "schema_version = 1\n").expect("write config");
    run_git(&dest, &["add", "."]);
    run_git(&dest, &["commit", "--quiet", "-m", "init"]);
    // A git-URL-classified entry (https) with no ref → NOT local-path, so
    // the pull path runs and effective-ref resolution must fail closed.
    std::fs::write(
        store.join("config.toml"),
        "layers = [\"team\"]\n\n[fleets.team]\nurl = \"https://example.com/repo.git\"\n",
    )
    .expect("write registry");

    let out = home
        .cmd()
        .env("WORKESTRATE_CONFIG", &store)
        .args(["fleet", "update", "team"])
        .output()
        .expect("invoke fleet update");
    assert!(
        !out.status.success(),
        "an unresolvable default ref must fail closed; stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cannot resolve a default ref") && stderr.contains("team"),
        "error must name the repo and the resolution failure; got: {stderr}"
    );
    assert!(
        stderr.contains("set `ref` in the registry entry"),
        "error must carry the remediation; got: {stderr}"
    );
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("updated to"),
        "no pull may run before the ref resolves"
    );
}

/// A dirty clone (modified tracked file) is refused before any pull.
#[test]
fn update_refuses_dirty_clone() {
    let home = IsolatedHome::new("cmd-fleet-update");
    home.write_registry_entry("personal", "");
    let repo = home.create_clean_git_fleet("personal");
    // Make it dirty: uncommitted modification to a tracked file.
    std::fs::write(repo.join("README.md"), "dirty edit").expect("dirty the repo");

    let out = home
        .cmd()
        .args(["fleet", "update", "personal"])
        .output()
        .expect("invoke fleet update");
    assert!(
        !out.status.success(),
        "dirty clone should fail; stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("uncommitted changes"),
        "expected dirty-clone error; got: {}",
        stderr
    );
    // No pull happened.
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("updated to"),
        "no pull should have run; got: {}",
        stdout
    );
    // Registry on disk is unchanged: entry still present, no rev recorded.
    let registry = home.read_registry();
    assert!(
        registry.contains("[fleets.personal]"),
        "registry entry must remain after refusal; got:\n{}",
        registry
    );
    assert!(
        !registry.contains("rev ="),
        "registry must not gain a rev line after refusal; got:\n{}",
        registry
    );
}

/// An untracked-only clone also counts as dirty (FN-1 parity with
/// `fleet remove --delete`).
#[test]
fn update_refuses_untracked_only_dirty_clone() {
    let home = IsolatedHome::new("cmd-fleet-update");
    home.write_registry_entry("personal", "");
    let repo = home.create_clean_git_fleet("personal");
    // Make it dirty via an untracked file only.
    std::fs::write(repo.join("scratch.txt"), "untracked").expect("dirty the repo");

    let out = home
        .cmd()
        .args(["fleet", "update", "personal"])
        .output()
        .expect("invoke fleet update");
    assert!(
        !out.status.success(),
        "untracked-only dirty clone should fail; stdout: {}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("uncommitted changes"),
        "expected dirty-clone error; got: {}",
        stderr
    );
    // No pull happened.
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("updated to"),
        "no pull should have run; got: {}",
        stdout
    );
    // Registry on disk is unchanged: entry still present, no rev recorded.
    let registry = home.read_registry();
    assert!(
        registry.contains("[fleets.personal]"),
        "registry entry must remain after refusal; got:\n{}",
        registry
    );
    assert!(
        !registry.contains("rev ="),
        "registry must not gain a rev line after refusal; got:\n{}",
        registry
    );
}
