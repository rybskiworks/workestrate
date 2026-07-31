//! Integration tests for `workestrate home clone <src> [dest]` — the
//! ADR 0025 provisioning path (commit 1: no workestrate.lock yet). Covers
//! positional dest, usage errors, local absolute/relative sources, local-only
//! config repos (copy + origin wiring + registry url rewrite), remote sources
//! with unreproducible entries (loud skip), pre-flight failure with zero dest
//! residue, and trusted_projects carry-over warnings.
//!
//! All git invocations run with a repo-local user.email/user.name and
//! `GIT_CONFIG_NOSYSTEM=1` so the suite is hermetic. "Remote" sources are
//! local paths ending in `.git` (classified as git URLs by
//! `looks_like_git_url`) — the tests stay fully offline.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::{IsolatedHome, TempDir};
use std::path::Path;
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

fn git_init_repo(dir: &Path) {
    std::fs::create_dir_all(dir).expect("create repo dir");
    run_git(dir, &["init", "-b", "main"]);
    run_git(dir, &["config", "user.email", "test@example.com"]);
    run_git(dir, &["config", "user.name", "Test"]);
}

fn git_commit_all(dir: &Path, message: &str) {
    run_git(dir, &["add", "."]);
    run_git(dir, &["commit", "-m", message]);
}

/// Materialize a fake SOURCE home: a git repo with a committed config.toml
/// and .gitignore, plus the standard layout dirs. When `config_repo` is
/// `Some(name)`, the home also gets a `config-repos/<name>` working copy —
/// its own git repo with two commits holding a valid `workestrate.toml` —
/// and the source registry registers it with a local-path url.
///
/// Returns the source home dir.
fn build_source_home(label: &str, config_repo: Option<&str>, trusted_projects: &[&str]) -> TempDir {
    let tmp = TempDir::new(label);
    let home = tmp.path().join("src-home");
    git_init_repo(&home);

    let mut registry = String::from("layers = [\"work\"]\n\n[settings]\nhome_version = 2\n");
    if let Some(name) = config_repo {
        registry.push_str(&format!(
            "\n[configs.{name}]\nurl = \"{}/config-repos/{name}\"\n",
            home.display()
        ));
    }
    for path in trusted_projects {
        registry.push_str(&format!("\n[[trusted_projects]]\npath = \"{path}\"\n"));
    }
    std::fs::write(home.join("config.toml"), registry).expect("write src registry");
    std::fs::write(
        home.join(".gitignore"),
        "/config-repos/\n/sources/\n/state/\n/cache/\n",
    )
    .expect("write src .gitignore");
    for dir in ["config-repos", "sources", "state", "secrets"] {
        std::fs::create_dir_all(home.join(dir)).expect("create src layout dir");
    }
    git_commit_all(&home, "home: initial");

    if let Some(name) = config_repo {
        let repo = home.join("config-repos").join(name);
        git_init_repo(&repo);
        std::fs::write(
            repo.join("workestrate.toml"),
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.network]\ndefault_deny = true\n",
        )
        .expect("write workestrate.toml");
        git_commit_all(&repo, "config: initial");
        std::fs::write(repo.join("notes.md"), "second commit\n").expect("write notes");
        git_commit_all(&repo, "config: notes");
    }
    tmp
}

// ---------------------------------------------------------------------------
// empty scaffold at a custom path (the global --home flag)
// ---------------------------------------------------------------------------

#[test]
fn home_flag_scaffolds_at_custom_path_even_when_resolved_home_is_elsewhere() {
    let home = IsolatedHome::new("cmd-home-prov");
    let scratch = TempDir::new("cmd-home-prov");
    let resolved_home = home.dir.join(".workestrate");
    let dest = scratch.path().join("elsewhere");

    let out = home
        .cmd()
        .args(["--home", dest.to_str().expect("utf8 dest"), "home", "init"])
        .output()
        .expect("invoke --home <path> home init");
    assert!(
        out.status.success(),
        "--home <path> home init failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(dest.join(".git").is_dir(), ".git must exist at dest");
    for dir in ["config-repos", "sources", "state", "secrets"] {
        assert!(dest.join(dir).is_dir(), "expected dir {dir} at dest");
    }
    assert!(
        !resolved_home.join(".git").exists(),
        "the resolved home must NOT be initialized when --home points elsewhere"
    );
}

// ---------------------------------------------------------------------------
// usage errors
// ---------------------------------------------------------------------------

#[test]
fn init_with_positional_dest_is_a_usage_error() {
    let home = IsolatedHome::new("cmd-home-prov");
    let out = home
        .cmd()
        .args(["home", "init", "/tmp/some-dest"])
        .output()
        .expect("invoke");
    assert!(
        !out.status.success(),
        "home init <positional> must be rejected by clap"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error:") && stderr.contains("unexpected argument"),
        "clap must report an unexpected argument; stderr=\n{stderr}"
    );
}

#[test]
fn init_with_config_and_positional_dest_is_a_usage_error() {
    let home = IsolatedHome::new("cmd-home-prov");
    let out = home
        .cmd()
        .args([
            "home",
            "init",
            "--config",
            "https://example.invalid/x.git",
            "/tmp/some-dest",
        ])
        .output()
        .expect("invoke");
    assert!(
        !out.status.success(),
        "home init --config + positional dest must be rejected by clap"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error:") && stderr.contains("unexpected argument"),
        "clap must report an unexpected argument; stderr=\n{stderr}"
    );
}

#[test]
fn init_with_name_and_positional_dest_is_a_usage_error() {
    let home = IsolatedHome::new("cmd-home-prov");
    let out = home
        .cmd()
        .args(["home", "init", "--name", "work", "/tmp/some-dest"])
        .output()
        .expect("invoke");
    assert!(
        !out.status.success(),
        "home init --name + positional dest must be rejected by clap"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error:") && stderr.contains("unexpected argument"),
        "clap must report an unexpected argument; stderr=\n{stderr}"
    );
}

#[test]
fn clone_with_config_is_a_usage_error() {
    let home = IsolatedHome::new("cmd-home-prov");
    let out = home
        .cmd()
        .args([
            "home",
            "clone",
            "--config",
            "https://example.invalid/x.git",
            "/tmp/whatever",
        ])
        .output()
        .expect("invoke");
    assert!(
        !out.status.success(),
        "home clone --config must be rejected by clap"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error:") && stderr.contains("unexpected argument"),
        "clap must report an unexpected argument; stderr=\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// clone from local src (absolute + relative)
// ---------------------------------------------------------------------------

#[test]
fn from_absolute_local_path_provisions_dest() {
    let home = IsolatedHome::new("cmd-home-prov");
    let src = build_source_home("cmd-home-prov-src", None, &[]);
    let scratch = TempDir::new("cmd-home-prov");
    let dest = scratch.path().join("dest-home");

    let src_path = src.path().join("src-home").canonicalize().unwrap();
    let out = home
        .cmd()
        .args(["home", "clone"])
        .arg(&src_path)
        .arg(&dest)
        .output()
        .expect("invoke home clone <abs> <dest>");
    assert!(
        out.status.success(),
        "absolute clone src failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dest.join(".git").is_dir(), "dest home must be a git repo");
    assert!(
        dest.join("config.toml").exists(),
        "dest must carry the registry"
    );
}

#[test]
fn from_relative_local_path_resolves_against_cwd() {
    let home = IsolatedHome::new("cmd-home-prov");
    let src = build_source_home("cmd-home-prov-src", None, &[]);
    let scratch = TempDir::new("cmd-home-prov");
    let dest = scratch.path().join("dest-home");

    // Invoke from the source's parent dir and reference it relatively.
    let out = home
        .cmd()
        .current_dir(src.path())
        .args(["home", "clone", "src-home"])
        .arg(&dest)
        .output()
        .expect("invoke home clone <rel> <dest>");
    assert!(
        out.status.success(),
        "relative clone src failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dest.join(".git").is_dir(), "dest home must be a git repo");
    assert!(dest.join("config.toml").exists());
}

// ---------------------------------------------------------------------------
// clone from local src with a local-path config repo
// ---------------------------------------------------------------------------

#[test]
fn from_local_src_copies_local_config_repo_and_rewrites_registry() {
    let home = IsolatedHome::new("cmd-home-prov");
    let src = build_source_home("cmd-home-prov-src", Some("work"), &[]);
    let scratch = TempDir::new("cmd-home-prov");
    let dest = scratch.path().join("dest-home");
    let src_home = src.path().join("src-home");

    let out = home
        .cmd()
        .args(["home", "clone"])
        .arg(&src_home)
        .arg(&dest)
        .output()
        .expect("invoke home clone");
    assert!(
        out.status.success(),
        "provisioning failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Repo present in dest/config-repos/work WITH its .git and both commits.
    let repo = dest.join("config-repos").join("work");
    assert!(repo.join(".git").is_dir(), "copied repo must keep its .git");
    let log = git_stdout(&repo, &["log", "--oneline"]);
    assert_eq!(
        log.lines().count(),
        2,
        "the copy must carry both commits; got:\n{log}"
    );

    // The copy's origin is wired to the SOURCE home's repo path.
    let origin = git_stdout(&repo, &["remote", "get-url", "origin"]);
    let expected_origin = src_home
        .join("config-repos")
        .join("work")
        .to_string_lossy()
        .to_string();
    assert_eq!(
        origin, expected_origin,
        "copied repo origin must point at the src-side repo"
    );

    // Registry url rewritten to the dest-local path; home_version stamped.
    let registry = std::fs::read_to_string(dest.join("config.toml")).expect("read dest registry");
    let dest_local = dest
        .join("config-repos")
        .join("work")
        .to_string_lossy()
        .to_string();
    assert!(
        registry.contains(&format!("url = \"{dest_local}\"")),
        "registry url must be rewritten to the dest-local path:\n{registry}"
    );
    assert!(
        registry.contains("home_version = 2"),
        "dest registry must stamp home_version = 2:\n{registry}"
    );

    // Dest home origin == src home path.
    let home_origin = git_stdout(&dest, &["remote", "get-url", "origin"]);
    assert_eq!(
        home_origin,
        src_home.to_string_lossy().to_string(),
        "dest home origin must be the src home path"
    );

    // state/ and secrets/ are empty.
    assert_eq!(
        std::fs::read_dir(dest.join("state")).unwrap().count(),
        0,
        "state/ must be empty"
    );
    assert_eq!(
        std::fs::read_dir(dest.join("secrets")).unwrap().count(),
        0,
        "secrets/ must be empty"
    );
}

// ---------------------------------------------------------------------------
// clone from a "remote" src (path ending in .git) with a local-path config repo
// ---------------------------------------------------------------------------

#[test]
fn from_remote_src_skips_unreproducible_local_config_repo_loudly() {
    let home = IsolatedHome::new("cmd-home-prov");
    // A "remote" src: a git repo at a path ending in `.git` so the classifier
    // treats it as a git URL; clone it to a .git-suffixed mirror path.
    let plain_src = build_source_home("cmd-home-prov-src", Some("work"), &[]);
    let scratch = TempDir::new("cmd-home-prov");
    let remote_mirror = scratch.path().join("mirror.git");
    run_git(
        scratch.path(),
        &[
            "clone",
            plain_src.path().join("src-home").to_str().expect("utf8"),
            remote_mirror.to_str().expect("utf8"),
        ],
    );

    let dest = scratch.path().join("dest-home");
    let out = home
        .cmd()
        .args(["home", "clone"])
        .arg(&remote_mirror)
        .arg(&dest)
        .output()
        .expect("invoke home clone <remote>");
    assert!(
        out.status.success(),
        "remote clone src failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The local-path entry is SKIPPED with the loud report naming it.
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let combined = format!("{stderr}\n{stdout}");
    assert!(
        combined.contains("unreproducible, clone manually") && combined.contains("work"),
        "output must name the skipped entry with 'unreproducible, clone manually':\n{combined}"
    );

    // The dest home is otherwise fully provisioned (NOT hollow).
    assert!(dest.join(".git").is_dir(), "dest home must be a git repo");
    assert!(
        dest.join("config.toml").exists(),
        "dest must carry the registry"
    );
    assert!(
        !dest.join("config-repos").join("work").exists(),
        "the skipped repo must NOT be materialized"
    );
    // Dest home origin points at the ORIGINAL <src> value, not the temp clone.
    let home_origin = git_stdout(&dest, &["remote", "get-url", "origin"]);
    assert_eq!(
        home_origin,
        remote_mirror.to_string_lossy().to_string(),
        "dest home origin must be the original <src> URL"
    );
}

// ---------------------------------------------------------------------------
// invalid src → failure with zero dest residue
// ---------------------------------------------------------------------------

#[test]
fn from_src_missing_config_toml_fails_with_no_dest_residue() {
    let home = IsolatedHome::new("cmd-home-prov");
    let scratch = TempDir::new("cmd-home-prov");
    let src = scratch.path().join("not-a-home");
    std::fs::create_dir_all(&src).expect("create src");
    let dest = scratch.path().join("dest-home");

    let out = home
        .cmd()
        .args(["home", "clone"])
        .arg(&src)
        .arg(&dest)
        .output()
        .expect("invoke");
    assert!(
        !out.status.success(),
        "missing config.toml must fail; stdout=\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        !dest.exists(),
        "dest must NOT exist after a pre-flight failure (zero residue)"
    );
}

#[test]
fn from_src_unparseable_config_toml_fails_with_no_dest_residue() {
    let home = IsolatedHome::new("cmd-home-prov");
    let scratch = TempDir::new("cmd-home-prov");
    let src = scratch.path().join("bad-home");
    std::fs::create_dir_all(&src).expect("create src");
    std::fs::write(src.join("config.toml"), "this is = not = valid toml [[[")
        .expect("write garbage config.toml");
    let dest = scratch.path().join("dest-home");

    let out = home
        .cmd()
        .args(["home", "clone"])
        .arg(&src)
        .arg(&dest)
        .output()
        .expect("invoke");
    assert!(
        !out.status.success(),
        "unparseable config.toml must fail; stdout=\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    assert!(
        !dest.exists(),
        "dest must NOT exist after a pre-flight failure (zero residue)"
    );
}

// ---------------------------------------------------------------------------
// trusted_projects carried + per-entry warning
// ---------------------------------------------------------------------------

#[test]
fn trusted_projects_are_carried_with_loud_per_entry_warning() {
    let home = IsolatedHome::new("cmd-home-prov");
    let trusted = "/some/other/machine/project";
    let src = build_source_home("cmd-home-prov-src", None, &[trusted]);
    let scratch = TempDir::new("cmd-home-prov");
    let dest = scratch.path().join("dest-home");

    let out = home
        .cmd()
        .args(["home", "clone"])
        .arg(src.path().join("src-home"))
        .arg(&dest)
        .output()
        .expect("invoke home clone");
    assert!(
        out.status.success(),
        "provisioning failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let registry = std::fs::read_to_string(dest.join("config.toml")).expect("read dest registry");
    assert!(
        registry.contains("[[trusted_projects]]") && registry.contains(trusted),
        "dest config.toml must carry the trusted_projects entry:\n{registry}"
    );

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("trusted_projects") && stderr.contains(trusted),
        "a per-entry warning must name the trusted path; stderr=\n{stderr}"
    );
}

// ---------------------------------------------------------------------------
// dest non-empty guard
// ---------------------------------------------------------------------------

#[test]
fn clone_dest_with_non_empty_existing_dir_fails() {
    let home = IsolatedHome::new("cmd-home-prov");
    let src = build_source_home("cmd-home-prov-src", None, &[]);
    let scratch = TempDir::new("cmd-home-prov");
    let dest = scratch.path().join("occupied");
    std::fs::create_dir_all(&dest).expect("create dest");
    std::fs::write(dest.join("sentinel"), "x").expect("write sentinel");

    let out = home
        .cmd()
        .arg("home")
        .arg("clone")
        .arg(src.path().join("src-home"))
        .arg(&dest)
        .output()
        .expect("invoke home clone <src> <dest>");
    assert!(
        !out.status.success(),
        "a non-empty clone dest must fail; stdout=\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("'home clone' requires a missing or empty directory"),
        "error must name 'home clone'; stderr=\n{stderr}"
    );
    // The sentinel survives — the guard bails BEFORE writing anything.
    assert!(dest.join("sentinel").exists());
}

// ---------------------------------------------------------------------------
// newer home_version → hard error
// ---------------------------------------------------------------------------

#[test]
fn from_src_with_newer_home_version_fails() {
    let home = IsolatedHome::new("cmd-home-prov");
    let src = build_source_home("cmd-home-prov-src", None, &[]);
    let src_home = src.path().join("src-home");
    std::fs::write(
        src_home.join("config.toml"),
        "layers = []\n\n[settings]\nhome_version = 99\n",
    )
    .expect("write newer-version registry");
    let scratch = TempDir::new("cmd-home-prov");
    let dest = scratch.path().join("dest-home");

    let out = home
        .cmd()
        .args(["home", "clone"])
        .arg(&src_home)
        .arg(&dest)
        .output()
        .expect("invoke");
    assert!(!out.status.success(), "a newer home_version must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("home created by a newer workestrate"),
        "error must contain the exact phrase; stderr=\n{stderr}"
    );
    assert!(
        !dest.exists(),
        "dest must NOT exist after a pre-flight failure"
    );
}

// ---------------------------------------------------------------------------
// legacy no-lock path: a source WITHOUT workestrate.lock provisions as before
// ---------------------------------------------------------------------------

#[test]
fn legacy_src_without_lock_honors_registry_rev_and_no_pin_warns() {
    let home = IsolatedHome::new("cmd-home-prov");
    let scratch = TempDir::new("cmd-home-prov");

    // Source home with TWO config repos cloned from .git-suffixed mirrors:
    // `pinned` (registry rev = R1, tip = R2) and `unpinned` (no rev, no ref —
    // must warn). NO workestrate.lock in the source (legacy home).
    let src_home = scratch.path().join("src-home");
    git_init_repo(&src_home);

    let mk_mirror = |name: &str, with_pin: bool| -> String {
        let repo = src_home.join("config-repos").join(name);
        git_init_repo(&repo);
        std::fs::write(repo.join("workestrate.toml"), "schema_version = 1\n")
            .expect("write workestrate.toml");
        git_commit_all(&repo, "config: initial");
        let r1 = git_stdout(&repo, &["rev-parse", "HEAD"]);
        std::fs::write(repo.join("notes.md"), "second commit\n").expect("write notes");
        git_commit_all(&repo, "config: notes");
        let mirror = scratch.path().join(format!("{name}.git"));
        run_git(
            scratch.path(),
            &[
                "clone",
                repo.to_str().expect("utf8"),
                mirror.to_str().expect("utf8"),
            ],
        );
        if with_pin {
            format!("url = \"{}\"\nrev = \"{r1}\"\n", mirror.display())
        } else {
            format!("url = \"{}\"\n", mirror.display())
        }
    };
    let pinned_entry = mk_mirror("pinned", true);
    let unpinned_entry = mk_mirror("unpinned", false);
    std::fs::write(
        src_home.join("config.toml"),
        format!(
            "layers = []\n\n[settings]\nhome_version = 2\n\n[configs.pinned]\n{pinned_entry}\n[configs.unpinned]\n{unpinned_entry}"
        ),
    )
    .expect("write src registry");
    std::fs::write(
        src_home.join(".gitignore"),
        "/config-repos/\n/sources/\n/state/\n/cache/\n",
    )
    .expect("write src .gitignore");
    git_commit_all(&src_home, "home: initial");
    // Deliberately NO workestrate.lock — the legacy path.
    assert!(
        !src_home.join("workestrate.lock").exists(),
        "the legacy source must have no lock"
    );

    let dest = scratch.path().join("dest-home");
    let out = home
        .cmd()
        .args(["home", "clone"])
        .arg(&src_home)
        .arg(&dest)
        .output()
        .expect("invoke home clone");
    assert!(
        out.status.success(),
        "legacy no-lock provisioning failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Registry rev honored: dest pinned repo HEAD == R1 (not the mirror tip).
    let r1 = git_stdout(
        &src_home.join("config-repos").join("pinned"),
        &["rev-parse", "HEAD~1"],
    );
    let dest_head = git_stdout(
        &dest.join("config-repos").join("pinned"),
        &["rev-parse", "HEAD"],
    );
    assert_eq!(
        dest_head, r1,
        "the registry rev (R1) must be honored on the legacy path"
    );

    // No-pin case still warns.
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unpinned") && stderr.contains("NOT pinned"),
        "the no-pin repo must warn; stderr=\n{stderr}"
    );

    // The dest lock pins the actual checked-out revs (R1 for pinned).
    let lock = std::fs::read_to_string(dest.join("workestrate.lock")).expect("read dest lock");
    assert!(
        lock.contains(&format!("rev = \"{r1}\"")),
        "dest lock must pin R1:\n{lock}"
    );
}

// ---------------------------------------------------------------------------
// local-only copy + lock: the locked rev is checked out in the copied repo
// ---------------------------------------------------------------------------

#[test]
fn from_local_src_copy_honors_the_locked_rev() {
    let home = IsolatedHome::new("cmd-home-prov");
    let src = build_source_home("cmd-home-prov-src", Some("work"), &[]);
    let src_home = src.path().join("src-home");
    let src_repo = src_home.join("config-repos").join("work");

    // build_source_home committed R1 ("config: initial") then R2 ("config:
    // notes"); the working copy sits at R2. Pin R1 in a source lock.
    let r1 = git_stdout(&src_repo, &["rev-parse", "HEAD~1"]);
    let r2 = git_stdout(&src_repo, &["rev-parse", "HEAD"]);
    assert_ne!(r1, r2, "R1 and R2 must differ");
    let url = format!("{}/config-repos/work", src_home.display());
    std::fs::write(
        src_home.join("workestrate.lock"),
        format!(
            "version = 1\nhome_version = 2\ntool_version = \"0.1.0\"\n\n[repos.work]\nurl = \"{url}\"\nrev = \"{r1}\"\n"
        ),
    )
    .expect("write src lock");

    let scratch = TempDir::new("cmd-home-prov");
    let dest = scratch.path().join("dest-home");
    let out = home
        .cmd()
        .args(["home", "clone"])
        .arg(&src_home)
        .arg(&dest)
        .output()
        .expect("invoke home clone");
    assert!(
        out.status.success(),
        "local-copy + lock provisioning failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The copied repo is checked out at the LOCKED rev R1, not the R2 tip.
    let dest_head = git_stdout(
        &dest.join("config-repos").join("work"),
        &["rev-parse", "HEAD"],
    );
    assert_eq!(
        dest_head, r1,
        "the copied repo must be checked out at the locked rev (R1), not the tip ({r2})"
    );

    // The dest lock pins R1 (the actual checked-out rev).
    let lock = std::fs::read_to_string(dest.join("workestrate.lock")).expect("read dest lock");
    assert!(
        lock.contains(&format!("rev = \"{r1}\"")),
        "dest lock must pin the locked rev (R1):\n{lock}"
    );
}
