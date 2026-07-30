//! Integration tests for `workestrate home init` — home scaffolding
//! (dirs/.gitignore/hook), idempotency, hook behavior, and the `--config`
//! composition with the existing clone+register machinery. Uses an isolated
//! HOME + WORKESTRATE_HOME per test so the user's real home is never touched.

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

/// Init a git repo at `dir` on branch `main` with a repo-local identity.
fn git_init_repo(dir: &Path) {
    std::fs::create_dir_all(dir).expect("create repo dir");
    run_git(dir, &["init", "-b", "main"]);
    run_git(dir, &["config", "user.email", "test@example.com"]);
    run_git(dir, &["config", "user.name", "Test"]);
}

#[test]
fn init_creates_structure_gitignore_and_hook() {
    let home = IsolatedHome::new("cmd-home-init");
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

    // Layout dirs + .git.
    for dir in ["config-repos", "sources", "state", "secrets"] {
        assert!(
            store.join(dir).is_dir(),
            "expected dir {} under the home",
            dir
        );
    }
    assert!(store.join(".git").is_dir(), ".git must exist after init");

    // .gitignore: all 9 entries present, each on its own line; no *.enc.
    let gitignore = std::fs::read_to_string(store.join(".gitignore")).expect("read .gitignore");
    let lines: Vec<&str> = gitignore.lines().collect();
    for entry in [
        "/config-repos/",
        "/sources/",
        "/state/",
        "/cache/",
        "*.agekey",
        "age.txt",
        "*.pem",
        "id_rsa*",
        ".env",
    ] {
        assert!(
            lines.contains(&entry),
            ".gitignore missing line '{}'; got:\n{}",
            entry,
            gitignore
        );
    }
    assert!(
        !gitignore.contains("*.enc"),
        ".gitignore must NOT ignore *.enc (age ciphertext stays committable):\n{}",
        gitignore
    );

    // Pre-commit hook: exists, executable, contains the gitlink + store-dir
    // guards.
    let hook = store.join(".git").join("hooks").join("pre-commit");
    assert!(hook.exists(), "pre-commit hook must exist");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&hook).unwrap().permissions().mode();
        assert!(
            mode & 0o111 != 0,
            "pre-commit hook must be executable (mode {:o})",
            mode
        );
    }
    let hook_content = std::fs::read_to_string(&hook).expect("read pre-commit hook");
    assert!(
        hook_content.contains("160000"),
        "hook must reject gitlinks (mode 160000):\n{}",
        hook_content
    );
    assert!(
        hook_content.contains("config-repos"),
        "hook must reject store-dir paths:\n{}",
        hook_content
    );
}

#[test]
fn second_run_is_idempotent_noop() {
    let home = IsolatedHome::new("cmd-home-init");
    let store = home.dir.join(".workestrate");

    let first = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args(["home", "init"])
        .output()
        .expect("first home init");
    assert!(
        first.status.success(),
        "first home init failed: stderr=\n{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let gitignore_before =
        std::fs::read_to_string(store.join(".gitignore")).expect("read .gitignore (first)");

    let second = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args(["home", "init"])
        .output()
        .expect("second home init");
    assert!(
        second.status.success(),
        "second home init must succeed (idempotent no-op): stderr=\n{}",
        String::from_utf8_lossy(&second.stderr)
    );
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert!(
        stdout.contains("already initialized"),
        "second run must report the no-op; got:\n{}",
        stdout
    );

    let gitignore_after =
        std::fs::read_to_string(store.join(".gitignore")).expect("read .gitignore (second)");
    assert_eq!(
        gitignore_before, gitignore_after,
        ".gitignore must be identical between runs (no rewrite on the no-op path)"
    );
}

#[test]
fn hook_rejects_gitlink() {
    let home = IsolatedHome::new("cmd-home-init");
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

    // Create an embedded standalone repo inside the home, then stage it as a
    // gitlink (mode 160000) in the home index.
    let embedded = store.join("embedded");
    git_init_repo(&embedded);
    std::fs::write(embedded.join("file.txt"), "embedded").expect("write embedded file");
    run_git(&embedded, &["add", "."]);
    run_git(&embedded, &["commit", "-m", "init"]);

    // `git add embedded` stages a gitlink (git prints a warning about adding
    // an embedded repository on stdout/stderr; the exit status is success).
    run_git(&store, &["add", "embedded"]);

    // Execute the hook directly — the deterministic way to test it.
    let hook_out = Command::new("sh")
        .arg(store.join(".git").join("hooks").join("pre-commit"))
        .current_dir(&store)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("run pre-commit hook");
    assert!(
        !hook_out.status.success(),
        "hook must reject a staged gitlink; stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&hook_out.stdout),
        String::from_utf8_lossy(&hook_out.stderr)
    );
    let stderr = String::from_utf8_lossy(&hook_out.stderr);
    assert!(
        stderr.contains("embedded") || stderr.contains("gitlink") || stderr.contains("160000"),
        "hook stderr should mention the gitlink/embedded path; got:\n{}",
        stderr
    );
}

#[test]
fn config_flag_composes_registration() {
    let home = IsolatedHome::new("cmd-home-init");
    let store = home.dir.join(".workestrate");
    let src_parent = TempDir::new("cmd-home-init-src");
    let src = src_parent.path().join("src-repo");

    // A local source git repo on branch `main` holding a minimal valid
    // workestrate.toml. `cmd_config_add` clones with `--branch main`.
    git_init_repo(&src);
    std::fs::write(src.join("workestrate.toml"), "schema_version = 1\n").expect("write toml");
    run_git(&src, &["add", "."]);
    run_git(&src, &["commit", "-m", "init"]);

    let out = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args([
            "home",
            "init",
            "--config",
            src.to_str().expect("utf8 src path"),
            "--name",
            "personal",
        ])
        .output()
        .expect("invoke home init --config");
    assert!(
        out.status.success(),
        "home init --config failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // The clone landed in the home's config-repos/ as a working repo.
    assert!(
        store
            .join("config-repos")
            .join("personal")
            .join(".git")
            .exists(),
        "config-repos/personal must be a cloned git repo"
    );

    // The registry was created by the registration path and records the
    // repo + a layers entry containing "personal".
    let registry = std::fs::read_to_string(store.join("config.toml")).expect("read registry");
    assert!(
        registry.contains("[configs.personal]"),
        "registry must contain [configs.personal]:\n{}",
        registry
    );
    let layers_line = registry
        .lines()
        .find(|l| l.starts_with("layers"))
        .expect("registry must have a layers line");
    assert!(
        layers_line.contains("\"personal\""),
        "layers line must contain \"personal\": {}",
        layers_line
    );

    // The with-config next-steps form is printed.
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("config list"),
        "with-config next steps must mention 'config list'; got:\n{}",
        stdout
    );
}
