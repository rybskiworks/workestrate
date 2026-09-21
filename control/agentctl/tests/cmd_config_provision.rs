//! Integration tests for `workestrate config clone <src> [dest]` — the
//! ADR 0025 provisioning path (commit 1: no workestrate.lock yet). Covers
//! positional dest, usage errors, local absolute/relative sources, local-only
//! fleets (copy + origin wiring + registry url rewrite), remote sources
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

/// Materialize a fake SOURCE config: a git repo with a committed config.toml
/// and .gitignore, plus the standard layout dirs. When `fleet` is
/// `Some(name)`, the config also gets a `fleets/<name>` working copy —
/// its own git repo with two commits holding a valid `workestrate.toml` —
/// and the source registry registers it with a local-path url.
///
/// Returns the source config dir.
fn build_source_config(label: &str, fleet: Option<&str>, trusted_projects: &[&str]) -> TempDir {
    let tmp = TempDir::new(label);
    let src_dir = tmp.path().join("src-config");
    git_init_repo(&src_dir);

    let mut registry = String::from("layers = [\"work\"]\n\n[settings]\nconfig_version = 2\n");
    if let Some(name) = fleet {
        registry.push_str(&format!(
            "\n[fleets.{name}]\nurl = \"{}/fleets/{name}\"\n",
            src_dir.display()
        ));
    }
    for path in trusted_projects {
        registry.push_str(&format!("\n[[trusted_projects]]\npath = \"{path}\"\n"));
    }
    std::fs::write(src_dir.join("config.toml"), registry).expect("write src registry");
    std::fs::write(
        src_dir.join(".gitignore"),
        "/fleets/\n/sources/\n/state/\n/cache/\n",
    )
    .expect("write src .gitignore");
    for dir in ["fleets", "sources", "state", "secrets"] {
        std::fs::create_dir_all(src_dir.join(dir)).expect("create src layout dir");
    }
    git_commit_all(&src_dir, "config: initial");

    if let Some(name) = fleet {
        let repo = src_dir.join("fleets").join(name);
        git_init_repo(&repo);
        std::fs::write(
            repo.join("workestrate.toml"),
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.network.defaults]\negress = \"deny\"\n",
        )
        .expect("write workestrate.toml");
        git_commit_all(&repo, "config: initial");
        std::fs::write(repo.join("notes.md"), "second commit\n").expect("write notes");
        git_commit_all(&repo, "config: notes");
    }
    tmp
}

// ---------------------------------------------------------------------------
// empty scaffold at a custom path (the global --config flag)
// ---------------------------------------------------------------------------

#[test]
fn config_flag_scaffolds_at_custom_path_even_when_resolved_config_is_elsewhere() {
    let home = IsolatedHome::new("cmd-config-prov");
    let scratch = TempDir::new("cmd-config-prov");
    let resolved_config = home.dir.join(".workestrate");
    let dest = scratch.path().join("elsewhere");

    let out = home
        .cmd()
        .args([
            "--config",
            dest.to_str().expect("utf8 dest"),
            "config",
            "init",
        ])
        .output()
        .expect("invoke --config <path> config init");
    assert!(
        out.status.success(),
        "--config <path> config init failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(dest.join(".git").is_dir(), ".git must exist at dest");
    for dir in ["fleets", "sources", "state", "secrets"] {
        assert!(dest.join(dir).is_dir(), "expected dir {dir} at dest");
    }
    assert!(
        !resolved_config.join(".git").exists(),
        "the resolved config must NOT be initialized when --config points elsewhere"
    );
}

// ---------------------------------------------------------------------------
// usage errors
// ---------------------------------------------------------------------------

#[test]
fn init_with_positional_dest_is_a_usage_error() {
    let home = IsolatedHome::new("cmd-config-prov");
    let out = home
        .cmd()
        .args(["config", "init", "/tmp/some-dest"])
        .output()
        .expect("invoke");
    assert!(
        !out.status.success(),
        "config init <positional> must be rejected by clap"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error:") && stderr.contains("unexpected argument"),
        "clap must report an unexpected argument; stderr=\n{stderr}"
    );
}

#[test]
fn init_rejects_positional_dest_even_with_global_fleet() {
    let home = IsolatedHome::new("cmd-config-prov");
    // `--fleet` is the global fleet-selection flag; it parses here, but
    // `config init` still takes no positional dest.
    let out = home
        .cmd()
        .args(["config", "init", "--fleet", "work", "/tmp/some-dest"])
        .output()
        .expect("invoke");
    assert!(
        !out.status.success(),
        "config init --fleet work <positional> must be rejected by clap"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("error:") && stderr.contains("unexpected argument"),
        "clap must report an unexpected argument; stderr=\n{stderr}"
    );
}

#[test]
fn init_with_name_flag_is_a_usage_error() {
    let home = IsolatedHome::new("cmd-config-prov");
    // The former `config init --name` scaffolding flag is retired; clap must
    // reject it as an unknown argument.
    let out = home
        .cmd()
        .args(["config", "init", "--name", "work"])
        .output()
        .expect("invoke");
    assert!(
        !out.status.success(),
        "config init --name must be rejected by clap"
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
    let home = IsolatedHome::new("cmd-config-prov");
    let src = build_source_config("cmd-config-prov-src", None, &[]);
    let scratch = TempDir::new("cmd-config-prov");
    let dest = scratch.path().join("dest-config");

    let src_path = src.path().join("src-config").canonicalize().unwrap();
    let out = home
        .cmd()
        .args(["config", "clone"])
        .arg(&src_path)
        .arg(&dest)
        .output()
        .expect("invoke config clone <abs> <dest>");
    assert!(
        out.status.success(),
        "absolute clone src failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dest.join(".git").is_dir(), "dest config must be a git repo");
    assert!(
        dest.join("config.toml").exists(),
        "dest must carry the registry"
    );
}

#[test]
fn from_relative_local_path_resolves_against_cwd() {
    let home = IsolatedHome::new("cmd-config-prov");
    let src = build_source_config("cmd-config-prov-src", None, &[]);
    let scratch = TempDir::new("cmd-config-prov");
    let dest = scratch.path().join("dest-config");

    // Invoke from the source's parent dir and reference it relatively.
    let out = home
        .cmd()
        .current_dir(src.path())
        .args(["config", "clone", "src-config"])
        .arg(&dest)
        .output()
        .expect("invoke config clone <rel> <dest>");
    assert!(
        out.status.success(),
        "relative clone src failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(dest.join(".git").is_dir(), "dest config must be a git repo");
    assert!(dest.join("config.toml").exists());
}

// ---------------------------------------------------------------------------
// clone from local src with a local-path fleet
// ---------------------------------------------------------------------------

#[test]
fn from_local_src_copies_local_fleet_and_rewrites_registry() {
    let home = IsolatedHome::new("cmd-config-prov");
    let src = build_source_config("cmd-config-prov-src", Some("work"), &[]);
    let scratch = TempDir::new("cmd-config-prov");
    let dest = scratch.path().join("dest-config");
    let src_config = src.path().join("src-config");

    let out = home
        .cmd()
        .args(["config", "clone"])
        .arg(&src_config)
        .arg(&dest)
        .output()
        .expect("invoke config clone");
    assert!(
        out.status.success(),
        "provisioning failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Repo present in dest/fleets/work WITH its .git and both commits.
    let repo = dest.join("fleets").join("work");
    assert!(repo.join(".git").is_dir(), "copied repo must keep its .git");
    let log = git_stdout(&repo, &["log", "--oneline"]);
    assert_eq!(
        log.lines().count(),
        2,
        "the copy must carry both commits; got:\n{log}"
    );

    // The copy's origin is wired to the SOURCE config's repo path.
    let origin = git_stdout(&repo, &["remote", "get-url", "origin"]);
    let expected_origin = src_config
        .join("fleets")
        .join("work")
        .to_string_lossy()
        .to_string();
    assert_eq!(
        origin, expected_origin,
        "copied repo origin must point at the src-side repo"
    );

    // Registry url rewritten to the dest-local path; config_version stamped.
    let registry = std::fs::read_to_string(dest.join("config.toml")).expect("read dest registry");
    let dest_local = dest
        .join("fleets")
        .join("work")
        .to_string_lossy()
        .to_string();
    assert!(
        registry.contains(&format!("url = \"{dest_local}\"")),
        "registry url must be rewritten to the dest-local path:\n{registry}"
    );
    assert!(
        registry.contains("config_version = 2"),
        "dest registry must stamp config_version = 2:\n{registry}"
    );

    // Dest config origin == src config path.
    let home_origin = git_stdout(&dest, &["remote", "get-url", "origin"]);
    assert_eq!(
        home_origin,
        src_config.to_string_lossy().to_string(),
        "dest config origin must be the src config path"
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
// clone from a "remote" src (path ending in .git) with a local-path fleet
// ---------------------------------------------------------------------------

#[test]
fn from_remote_src_skips_unreproducible_local_fleet_loudly() {
    let home = IsolatedHome::new("cmd-config-prov");
    // A "remote" src: a git repo at a path ending in `.git` so the classifier
    // treats it as a git URL; clone it to a .git-suffixed mirror path.
    let plain_src = build_source_config("cmd-config-prov-src", Some("work"), &[]);
    let scratch = TempDir::new("cmd-config-prov");
    let remote_mirror = scratch.path().join("mirror.git");
    run_git(
        scratch.path(),
        &[
            "clone",
            plain_src.path().join("src-config").to_str().expect("utf8"),
            remote_mirror.to_str().expect("utf8"),
        ],
    );

    let dest = scratch.path().join("dest-config");
    let out = home
        .cmd()
        .args(["config", "clone"])
        .arg(&remote_mirror)
        .arg(&dest)
        .output()
        .expect("invoke config clone <remote>");
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

    // The dest config is otherwise fully provisioned (NOT hollow).
    assert!(dest.join(".git").is_dir(), "dest config must be a git repo");
    assert!(
        dest.join("config.toml").exists(),
        "dest must carry the registry"
    );
    assert!(
        !dest.join("fleets").join("work").exists(),
        "the skipped repo must NOT be materialized"
    );
    // Dest config origin points at the ORIGINAL <src> value, not the temp clone.
    let home_origin = git_stdout(&dest, &["remote", "get-url", "origin"]);
    assert_eq!(
        home_origin,
        remote_mirror.to_string_lossy().to_string(),
        "dest config origin must be the original <src> URL"
    );
}

// ---------------------------------------------------------------------------
// invalid src → failure with zero dest residue
// ---------------------------------------------------------------------------

#[test]
fn from_src_missing_config_toml_fails_with_no_dest_residue() {
    let home = IsolatedHome::new("cmd-config-prov");
    let scratch = TempDir::new("cmd-config-prov");
    let src = scratch.path().join("not-a-config");
    std::fs::create_dir_all(&src).expect("create src");
    let dest = scratch.path().join("dest-config");

    let out = home
        .cmd()
        .args(["config", "clone"])
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
fn from_src_unparsable_config_toml_fails_with_no_dest_residue() {
    let home = IsolatedHome::new("cmd-config-prov");
    let scratch = TempDir::new("cmd-config-prov");
    let src = scratch.path().join("bad-config");
    std::fs::create_dir_all(&src).expect("create src");
    std::fs::write(src.join("config.toml"), "this is = not = valid toml [[[")
        .expect("write garbage config.toml");
    let dest = scratch.path().join("dest-config");

    let out = home
        .cmd()
        .args(["config", "clone"])
        .arg(&src)
        .arg(&dest)
        .output()
        .expect("invoke");
    assert!(
        !out.status.success(),
        "unparsable config.toml must fail; stdout=\n{}",
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
    let home = IsolatedHome::new("cmd-config-prov");
    let trusted = "/some/other/machine/project";
    let src = build_source_config("cmd-config-prov-src", None, &[trusted]);
    let scratch = TempDir::new("cmd-config-prov");
    let dest = scratch.path().join("dest-config");

    let out = home
        .cmd()
        .args(["config", "clone"])
        .arg(src.path().join("src-config"))
        .arg(&dest)
        .output()
        .expect("invoke config clone");
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
    let home = IsolatedHome::new("cmd-config-prov");
    let src = build_source_config("cmd-config-prov-src", None, &[]);
    let scratch = TempDir::new("cmd-config-prov");
    let dest = scratch.path().join("occupied");
    std::fs::create_dir_all(&dest).expect("create dest");
    std::fs::write(dest.join("sentinel"), "x").expect("write sentinel");

    let out = home
        .cmd()
        .arg("config")
        .arg("clone")
        .arg(src.path().join("src-config"))
        .arg(&dest)
        .output()
        .expect("invoke config clone <src> <dest>");
    assert!(
        !out.status.success(),
        "a non-empty clone dest must fail; stdout=\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("'config clone' requires a missing or empty directory"),
        "error must name 'config clone'; stderr=\n{stderr}"
    );
    // The sentinel survives — the guard bails BEFORE writing anything.
    assert!(dest.join("sentinel").exists());
}

// ---------------------------------------------------------------------------
// newer config_version → hard error
// ---------------------------------------------------------------------------

#[test]
fn from_src_with_newer_config_version_fails() {
    let home = IsolatedHome::new("cmd-config-prov");
    let src = build_source_config("cmd-config-prov-src", None, &[]);
    let src_config = src.path().join("src-config");
    std::fs::write(
        src_config.join("config.toml"),
        "layers = []\n\n[settings]\nconfig_version = 99\n",
    )
    .expect("write newer-version registry");
    let scratch = TempDir::new("cmd-config-prov");
    let dest = scratch.path().join("dest-config");

    let out = home
        .cmd()
        .args(["config", "clone"])
        .arg(&src_config)
        .arg(&dest)
        .output()
        .expect("invoke");
    assert!(!out.status.success(), "a newer config_version must fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("config created by a newer workestrate"),
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
    let home = IsolatedHome::new("cmd-config-prov");
    let scratch = TempDir::new("cmd-config-prov");

    // Source config with TWO fleets cloned from .git-suffixed mirrors:
    // `pinned` (registry rev = R1, tip = R2) and `unpinned` (no rev, no ref —
    // must warn). NO workestrate.lock in the source (legacy config).
    let src_config = scratch.path().join("src-config");
    git_init_repo(&src_config);

    let mk_mirror = |name: &str, with_pin: bool| -> String {
        let repo = src_config.join("fleets").join(name);
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
        src_config.join("config.toml"),
        format!(
            "layers = []\n\n[settings]\nconfig_version = 2\n\n[fleets.pinned]\n{pinned_entry}\n[fleets.unpinned]\n{unpinned_entry}"
        ),
    )
    .expect("write src registry");
    std::fs::write(
        src_config.join(".gitignore"),
        "/fleets/\n/sources/\n/state/\n/cache/\n",
    )
    .expect("write src .gitignore");
    git_commit_all(&src_config, "config: initial");
    // Deliberately NO workestrate.lock — the legacy path.
    assert!(
        !src_config.join("workestrate.lock").exists(),
        "the legacy source must have no lock"
    );

    let dest = scratch.path().join("dest-config");
    let out = home
        .cmd()
        .args(["config", "clone"])
        .arg(&src_config)
        .arg(&dest)
        .output()
        .expect("invoke config clone");
    assert!(
        out.status.success(),
        "legacy no-lock provisioning failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Registry rev honored: dest pinned repo HEAD == R1 (not the mirror tip).
    let r1 = git_stdout(
        &src_config.join("fleets").join("pinned"),
        &["rev-parse", "HEAD~1"],
    );
    let dest_head = git_stdout(&dest.join("fleets").join("pinned"), &["rev-parse", "HEAD"]);
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
// commitless git src (git init'd, no commits) → file-copy fallback
// ---------------------------------------------------------------------------

#[test]
fn from_commitless_git_src_falls_back_to_file_copy() {
    let home = IsolatedHome::new("cmd-config-prov");
    let tmp = TempDir::new("cmd-config-prov-src");
    let src_config = tmp.path().join("src-config");
    // Same layout as build_source_config but WITHOUT the commit step: the src
    // is a git repo on disk with no resolvable HEAD, so config.toml is
    // untracked and a git clone of it would yield an empty tree.
    git_init_repo(&src_config);
    std::fs::write(
        src_config.join("config.toml"),
        "layers = [\"work\"]\n\n[settings]\nconfig_version = 2\n",
    )
    .expect("write src registry");
    std::fs::write(
        src_config.join(".gitignore"),
        "/fleets/\n/sources/\n/state/\n/cache/\n",
    )
    .expect("write src .gitignore");
    for dir in ["fleets", "sources", "state", "secrets"] {
        std::fs::create_dir_all(src_config.join(dir)).expect("create src layout dir");
    }

    let scratch = TempDir::new("cmd-config-prov");
    let dest = scratch.path().join("dest-config");
    let out = home
        .cmd()
        .args(["config", "clone"])
        .arg(&src_config)
        .arg(&dest)
        .output()
        .expect("invoke config clone");
    assert!(
        out.status.success(),
        "commitless-src provisioning failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The file-copy path materialized dest: the registry carries the src's
    // (uncommitted) content, stamped at the current config_version.
    let registry = std::fs::read_to_string(dest.join("config.toml")).expect("read dest registry");
    assert!(
        registry.contains("layers = [\"work\"]"),
        "dest registry must carry the src layers:\n{registry}"
    );
    assert!(
        registry.contains("config_version = 2"),
        "dest registry must stamp config_version = 2:\n{registry}"
    );
    assert!(dest.join(".git").is_dir(), "dest config must be a git repo");
    assert!(
        dest.join(".git").join("hooks").join("pre-commit").exists(),
        "dest must carry the pre-commit hook"
    );
    assert!(
        dest.join(".gitignore").exists(),
        "dest must carry .gitignore"
    );
    for dir in ["fleets", "sources", "state", "secrets"] {
        assert!(dest.join(dir).is_dir(), "expected dir {dir} at dest");
    }
}

// ---------------------------------------------------------------------------
// local-only copy + lock: the locked rev is checked out in the copied repo
// ---------------------------------------------------------------------------

#[test]
fn from_local_src_copy_honors_the_locked_rev() {
    let home = IsolatedHome::new("cmd-config-prov");
    let src = build_source_config("cmd-config-prov-src", Some("work"), &[]);
    let src_config = src.path().join("src-config");
    let src_repo = src_config.join("fleets").join("work");

    // build_source_config committed R1 ("config: initial") then R2 ("config:
    // notes"); the working copy sits at R2. Pin R1 in a source lock.
    let r1 = git_stdout(&src_repo, &["rev-parse", "HEAD~1"]);
    let r2 = git_stdout(&src_repo, &["rev-parse", "HEAD"]);
    assert_ne!(r1, r2, "R1 and R2 must differ");
    let url = format!("{}/fleets/work", src_config.display());
    std::fs::write(
        src_config.join("workestrate.lock"),
        format!(
            "version = 1\nconfig_version = 2\ntool_version = \"0.1.0\"\n\n[fleets.work]\nurl = \"{url}\"\nrev = \"{r1}\"\n"
        ),
    )
    .expect("write src lock");

    let scratch = TempDir::new("cmd-config-prov");
    let dest = scratch.path().join("dest-config");
    let out = home
        .cmd()
        .args(["config", "clone"])
        .arg(&src_config)
        .arg(&dest)
        .output()
        .expect("invoke config clone");
    assert!(
        out.status.success(),
        "local-copy + lock provisioning failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The copied repo is checked out at the LOCKED rev R1, not the R2 tip.
    let dest_head = git_stdout(&dest.join("fleets").join("work"), &["rev-parse", "HEAD"]);
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
