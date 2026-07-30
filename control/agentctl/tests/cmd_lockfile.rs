//! Integration tests for the generated `workestrate.lock` (ADR 0025(e),
//! spec 11 §3): `config add`/`config update`/`config remove` mutate the lock,
//! bare `home init` writes an empty-repos lock, and `home init --from` pins
//! the actual checked-out revs. Legacy tolerance (a home without a lock) is
//! covered by the pre-existing suites, which never create one and must stay
//! green.
//!
//! All git invocations run with a repo-local user.email/user.name and
//! `GIT_CONFIG_NOSYSTEM=1` so the suite is hermetic; clone sources are local
//! paths (git treats them as cloneable URLs), keeping the tests offline.

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

/// Init a git repo at `dir` on branch `main` with a repo-local identity and
/// one committed `workestrate.toml`; return the repo dir.
fn git_source_repo(parent: &Path, name: &str) -> std::path::PathBuf {
    let dir = parent.join(name);
    std::fs::create_dir_all(&dir).expect("create repo dir");
    run_git(&dir, &["init", "-b", "main"]);
    run_git(&dir, &["config", "user.email", "test@example.com"]);
    run_git(&dir, &["config", "user.name", "Test"]);
    std::fs::write(
        dir.join("workestrate.toml"),
        "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.network]\ndefault_deny = true\n",
    )
    .expect("write toml");
    run_git(&dir, &["add", "."]);
    run_git(&dir, &["commit", "-m", "init"]);
    dir
}

/// Extract the value lines of `[repos.<name>]` from a lock TOML string.
fn lock_repo_section(lock: &str, name: &str) -> String {
    let header = format!("[repos.{name}]");
    let mut in_section = false;
    let mut out = String::new();
    for line in lock.lines() {
        if line.starts_with('[') {
            if in_section {
                break;
            }
            in_section = line == header;
            continue;
        }
        if in_section {
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

// ---------------------------------------------------------------------------
// config add writes the lock
// ---------------------------------------------------------------------------

#[test]
fn config_add_writes_lock_with_url_ref_and_actual_head_rev() {
    let home = IsolatedHome::new("cmd-lockfile");
    let store = home.dir.join(".workestrate");
    let scratch = TempDir::new("cmd-lockfile-src");
    let src = git_source_repo(scratch.path(), "src-repo");
    let src_head = git_stdout(&src, &["rev-parse", "HEAD"]);

    let out = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args([
            "config",
            "add",
            src.to_str().expect("utf8 src path"),
            "personal",
            "--ref",
            "main",
        ])
        .output()
        .expect("invoke config add");
    assert!(
        out.status.success(),
        "config add failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let lock_path = store.join("workestrate.lock");
    assert!(lock_path.exists(), "config add must write workestrate.lock");
    let lock = std::fs::read_to_string(&lock_path).expect("read lock");
    assert!(
        lock.contains("version = 1"),
        "lock must carry version = 1:\n{lock}"
    );
    assert!(
        lock.contains("home_version = 2"),
        "lock must default home_version to 2:\n{lock}"
    );
    let section = lock_repo_section(&lock, "personal");
    assert!(
        !section.is_empty(),
        "lock must contain [repos.personal]:\n{lock}"
    );
    assert!(
        section.contains(&format!("url = \"{}\"", src.display())),
        "lock entry must record the source url:\n{section}"
    );
    assert!(
        section.contains("ref = \"main\""),
        "lock entry must record ref main:\n{section}"
    );
    assert!(
        section.contains(&format!("rev = \"{src_head}\"")),
        "lock entry must pin the clone's actual HEAD ({src_head}):\n{section}"
    );
    // The clone's HEAD equals the source HEAD (fresh clone of a local repo).
    let clone_head = git_stdout(
        &store.join("config-repos").join("personal"),
        &["rev-parse", "HEAD"],
    );
    assert!(
        section.contains(&format!("rev = \"{clone_head}\"")),
        "lock rev must equal the checkout HEAD ({clone_head}):\n{section}"
    );
}

// ---------------------------------------------------------------------------
// config update follows the new HEAD
// ---------------------------------------------------------------------------

#[test]
fn config_update_advances_the_lock_rev_to_the_new_head() {
    let home = IsolatedHome::new("cmd-lockfile");
    let store = home.dir.join(".workestrate");
    let scratch = TempDir::new("cmd-lockfile-src");
    let src = git_source_repo(scratch.path(), "src-repo");

    let add = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args([
            "config",
            "add",
            src.to_str().expect("utf8 src path"),
            "personal",
            "--ref",
            "main",
        ])
        .output()
        .expect("invoke config add");
    assert!(
        add.status.success(),
        "config add failed: stderr=\n{}",
        String::from_utf8_lossy(&add.stderr)
    );
    let first_head = git_stdout(&src, &["rev-parse", "HEAD"]);

    // Advance the source repo.
    std::fs::write(src.join("notes.md"), "second commit\n").expect("write notes");
    run_git(&src, &["add", "."]);
    run_git(&src, &["commit", "-m", "notes"]);
    let second_head = git_stdout(&src, &["rev-parse", "HEAD"]);
    assert_ne!(first_head, second_head, "the source must have advanced");

    let out = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args(["config", "update", "personal"])
        .output()
        .expect("invoke config update");
    assert!(
        out.status.success(),
        "config update failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let lock = std::fs::read_to_string(store.join("workestrate.lock")).expect("read lock");
    let section = lock_repo_section(&lock, "personal");
    assert!(
        section.contains(&format!("rev = \"{second_head}\"")),
        "lock rev must follow the new HEAD ({second_head}):\n{section}"
    );
    assert!(
        !section.contains(&format!("rev = \"{first_head}\"")),
        "the stale rev must be gone:\n{section}"
    );
}

// ---------------------------------------------------------------------------
// config remove drops the lock entry
// ---------------------------------------------------------------------------

#[test]
fn config_remove_drops_the_lock_entry() {
    let home = IsolatedHome::new("cmd-lockfile");
    let store = home.dir.join(".workestrate");
    let scratch = TempDir::new("cmd-lockfile-src");
    let src = git_source_repo(scratch.path(), "src-repo");

    let add = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args([
            "config",
            "add",
            src.to_str().expect("utf8 src path"),
            "personal",
            "--ref",
            "main",
        ])
        .output()
        .expect("invoke config add");
    assert!(
        add.status.success(),
        "config add failed: stderr=\n{}",
        String::from_utf8_lossy(&add.stderr)
    );
    assert!(store.join("workestrate.lock").exists());

    let out = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args(["config", "remove", "personal"])
        .output()
        .expect("invoke config remove");
    assert!(
        out.status.success(),
        "config remove failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let lock = std::fs::read_to_string(store.join("workestrate.lock")).expect("read lock");
    assert!(
        !lock.contains("[repos.personal]"),
        "the lock entry must be dropped after remove:\n{lock}"
    );
    // The lock itself still exists (a home that had a lock keeps one).
    assert!(
        lock.contains("version = 1"),
        "the lock file itself must survive a remove:\n{lock}"
    );
}

// ---------------------------------------------------------------------------
// bare home init writes an empty-repos lock
// ---------------------------------------------------------------------------

#[test]
fn bare_home_init_writes_a_lock_with_empty_repos() {
    let home = IsolatedHome::new("cmd-lockfile");
    let store = home.dir.join(".workestrate");

    let out = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args(["home", "init"])
        .output()
        .expect("invoke home init");
    assert!(
        out.status.success(),
        "home init failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let lock_path = store.join("workestrate.lock");
    assert!(lock_path.exists(), "bare home init must write the lock");
    let lock = std::fs::read_to_string(&lock_path).expect("read lock");
    assert!(
        lock.contains("version = 1"),
        "lock must carry version = 1:\n{lock}"
    );
    assert!(
        lock.contains("home_version = 2"),
        "lock must carry home_version = 2 (registry default):\n{lock}"
    );
    let tool_line = lock
        .lines()
        .find(|l| l.starts_with("tool_version"))
        .expect("lock must have a tool_version line");
    assert!(
        tool_line.contains('"') && !tool_line.contains("\"\""),
        "tool_version must be non-empty: {tool_line}"
    );
    assert!(
        !lock.contains("[repos."),
        "a bare init with no config repos writes an empty repos map:\n{lock}"
    );
}

// ---------------------------------------------------------------------------
// home init --from pins the actual checked-out rev
// ---------------------------------------------------------------------------

#[test]
fn home_init_from_writes_dest_lock_pinning_the_checked_out_rev() {
    let home = IsolatedHome::new("cmd-lockfile");
    let scratch = TempDir::new("cmd-lockfile-src");

    // Build a SOURCE home: a git repo with a committed config.toml + a
    // local-path config repo with TWO commits; the source registry pins the
    // FIRST commit's rev so provisioning checks out that rev (not the tip).
    let src_home = scratch.path().join("src-home");
    std::fs::create_dir_all(&src_home).expect("create src home");
    run_git(&src_home, &["init", "-b", "main"]);
    run_git(&src_home, &["config", "user.email", "test@example.com"]);
    run_git(&src_home, &["config", "user.name", "Test"]);

    let src_repo = git_source_repo(&src_home.join("config-repos"), "work");
    let first_rev = git_stdout(&src_repo, &["rev-parse", "HEAD"]);
    std::fs::write(src_repo.join("notes.md"), "second commit\n").expect("write notes");
    run_git(&src_repo, &["add", "."]);
    run_git(&src_repo, &["commit", "-m", "notes"]);

    // Register the repo with a url that classifies as a git remote (path
    // ending in .git) so provisioning RE-CLONES it, pinned at first_rev.
    let remoteish = format!("{}.git", src_repo.display());
    std::fs::write(
        src_home.join("config.toml"),
        format!(
            "layers = [\"work\"]\n\n[settings]\nhome_version = 2\n\n[configs.work]\nurl = \"{remoteish}\"\nref = \"main\"\nrev = \"{first_rev}\"\n"
        ),
    )
    .expect("write src registry");
    std::fs::write(src_home.join(".gitignore"), "/config-repos/\n/state/\n").expect("gitignore");
    run_git(&src_home, &["add", "."]);
    run_git(&src_home, &["commit", "-m", "home: initial"]);

    // `git clone <src_repo>` needs the source reachable at the .git-suffixed
    // path; create it as a real clone so the classifier + clone both work.
    run_git(
        scratch.path(),
        &["clone", src_repo.to_str().expect("utf8"), &remoteish],
    );
    // Advance the MIRROR source past first_rev so the pin is observable: the
    // dest clone must land on first_rev, not the mirror's tip.
    run_git(
        &remoteish_as_path(&remoteish),
        &["config", "user.email", "test@example.com"],
    );
    run_git(
        &remoteish_as_path(&remoteish),
        &["config", "user.name", "Test"],
    );
    std::fs::write(
        remoteish_as_path(&remoteish).join("third.md"),
        "third commit\n",
    )
    .expect("write third");
    run_git(&remoteish_as_path(&remoteish), &["add", "."]);
    run_git(&remoteish_as_path(&remoteish), &["commit", "-m", "third"]);
    let mirror_tip = git_stdout(&remoteish_as_path(&remoteish), &["rev-parse", "HEAD"]);
    assert_ne!(first_rev, mirror_tip);

    let dest = scratch.path().join("dest-home");
    let out = home
        .cmd()
        .args(["home", "init", "--from"])
        .arg(&src_home)
        .arg(&dest)
        .output()
        .expect("invoke home init --from");
    assert!(
        out.status.success(),
        "home init --from failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The dest checkout is pinned at first_rev (not the mirror tip).
    let dest_head = git_stdout(
        &dest.join("config-repos").join("work"),
        &["rev-parse", "HEAD"],
    );
    assert_eq!(
        dest_head, first_rev,
        "the dest checkout must be pinned at the locked rev, not the tip"
    );

    // The dest lock pins the ACTUAL checked-out rev.
    let lock_path = dest.join("workestrate.lock");
    assert!(
        lock_path.exists(),
        "provisioning must write dest/workestrate.lock"
    );
    let lock = std::fs::read_to_string(&lock_path).expect("read dest lock");
    assert!(lock.contains("version = 1"), "lock version:\n{lock}");
    assert!(
        lock.contains("home_version = 2"),
        "lock home_version (src registry recorded 2):\n{lock}"
    );
    let section = lock_repo_section(&lock, "work");
    assert!(
        section.contains(&format!("rev = \"{first_rev}\"")),
        "dest lock must pin the actual checked-out rev ({first_rev}):\n{section}"
    );
    assert!(
        section.contains(&format!("url = \"{remoteish}\"")),
        "dest lock records the registered url:\n{section}"
    );
    assert!(
        section.contains("ref = \"main\""),
        "dest lock records the ref:\n{section}"
    );
}

fn remoteish_as_path(remoteish: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(remoteish)
}

// ---------------------------------------------------------------------------
// home init --from consumes the SOURCE lock: locked rev wins over the registry
// ---------------------------------------------------------------------------

/// Build a SOURCE home whose config repo has commits R1 (older) and R2 (tip);
/// the src registry records rev R2 while the src workestrate.lock pins rev R1
/// (the source advanced past the lock). Returns (scratch, src_home, mirror
/// path, R1, R2).
fn build_lock_driven_source(
    scratch: &Path,
) -> (std::path::PathBuf, std::path::PathBuf, String, String) {
    let src_home = scratch.join("src-home");
    std::fs::create_dir_all(&src_home).expect("create src home");
    run_git(&src_home, &["init", "-b", "main"]);
    run_git(&src_home, &["config", "user.email", "test@example.com"]);
    run_git(&src_home, &["config", "user.name", "Test"]);

    // The config repo: R1 then R2.
    let src_repo = git_source_repo(&src_home.join("config-repos"), "work");
    let r1 = git_stdout(&src_repo, &["rev-parse", "HEAD"]);
    std::fs::write(src_repo.join("notes.md"), "second commit\n").expect("write notes");
    run_git(&src_repo, &["add", "."]);
    run_git(&src_repo, &["commit", "-m", "notes"]);
    let r2 = git_stdout(&src_repo, &["rev-parse", "HEAD"]);
    assert_ne!(r1, r2, "R1 and R2 must differ");

    // A cloneable mirror at a .git-suffixed path (classifier: git URL).
    let remoteish = format!("{}.git", src_repo.display());
    run_git(
        scratch,
        &["clone", src_repo.to_str().expect("utf8"), &remoteish],
    );

    // The registry records the TIP (R2); the lock pins the OLDER R1.
    std::fs::write(
        src_home.join("config.toml"),
        format!(
            "layers = [\"work\"]\n\n[settings]\nhome_version = 2\n\n[configs.work]\nurl = \"{remoteish}\"\nref = \"main\"\nrev = \"{r2}\"\n"
        ),
    )
    .expect("write src registry");
    std::fs::write(
        src_home.join("workestrate.lock"),
        format!(
            "version = 1\nhome_version = 2\ntool_version = \"0.1.0\"\n\n[repos.work]\nurl = \"{remoteish}\"\nref = \"main\"\nrev = \"{r1}\"\n"
        ),
    )
    .expect("write src lock");
    std::fs::write(src_home.join(".gitignore"), "/config-repos/\n/state/\n").expect("gitignore");
    run_git(&src_home, &["add", "."]);
    run_git(&src_home, &["commit", "-m", "home: initial"]);

    (src_home, remoteish_as_path(&remoteish), r1, r2)
}

#[test]
fn home_init_from_lock_rev_wins_over_registry_rev() {
    let home = IsolatedHome::new("cmd-lockfile");
    let scratch = TempDir::new("cmd-lockfile-src");
    let (src_home, _mirror, r1, r2) = build_lock_driven_source(scratch.path());

    let dest = scratch.path().join("dest-home");
    let out = home
        .cmd()
        .args(["home", "init", "--from"])
        .arg(&src_home)
        .arg(&dest)
        .output()
        .expect("invoke home init --from");
    assert!(
        out.status.success(),
        "home init --from failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The dest checkout lands on R1 (the LOCK rev), NOT R2 (registry rev/tip).
    let dest_head = git_stdout(
        &dest.join("config-repos").join("work"),
        &["rev-parse", "HEAD"],
    );
    assert_eq!(
        dest_head, r1,
        "the lock rev must win over the registry rev ({r2})"
    );

    // The dest lock pins R1 (the actual checked-out rev).
    let lock = std::fs::read_to_string(dest.join("workestrate.lock")).expect("read dest lock");
    let section = lock_repo_section(&lock, "work");
    assert!(
        section.contains(&format!("rev = \"{r1}\"")),
        "dest lock must pin the locked rev ({r1}):\n{section}"
    );
    assert!(
        !section.contains(&format!("rev = \"{r2}\"")),
        "the registry tip must NOT leak into the dest lock:\n{section}"
    );

    // The summary notes the locked rev per repo line.
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("locked rev") && stdout.contains(&r1[..7]),
        "summary must note the locked rev: stdout=\n{stdout}"
    );
}

// ---------------------------------------------------------------------------
// too-new source lock → hard error, zero dest residue
// ---------------------------------------------------------------------------

#[test]
fn home_init_from_src_with_newer_lock_version_fails_with_no_dest_residue() {
    let home = IsolatedHome::new("cmd-lockfile");
    let scratch = TempDir::new("cmd-lockfile-src");

    // Minimal source home: a git repo with a committed config.toml.
    let src_home = scratch.path().join("src-home");
    std::fs::create_dir_all(&src_home).expect("create src home");
    run_git(&src_home, &["init", "-b", "main"]);
    run_git(&src_home, &["config", "user.email", "test@example.com"]);
    run_git(&src_home, &["config", "user.name", "Test"]);
    std::fs::write(
        src_home.join("config.toml"),
        "layers = []\n\n[settings]\nhome_version = 2\n",
    )
    .expect("write src registry");
    // A lock from the future.
    std::fs::write(
        src_home.join("workestrate.lock"),
        "version = 99\nhome_version = 2\ntool_version = \"0.1.0\"\n",
    )
    .expect("write too-new lock");
    run_git(&src_home, &["add", "."]);
    run_git(&src_home, &["commit", "-m", "home: initial"]);

    let dest = scratch.path().join("dest-home");
    let out = home
        .cmd()
        .args(["home", "init", "--from"])
        .arg(&src_home)
        .arg(&dest)
        .output()
        .expect("invoke home init --from");
    assert!(
        !out.status.success(),
        "a too-new source lock must fail; stdout=\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let combined = format!("{stderr}\n{stdout}");
    assert!(
        combined.contains("home created by a newer workestrate"),
        "error must contain the exact phrase:\n{combined}"
    );
    assert!(
        !dest.exists(),
        "dest must NOT exist after a pre-flight failure (zero residue)"
    );
}
