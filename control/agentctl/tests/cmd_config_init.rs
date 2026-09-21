//! Integration tests for `workestrate config init` — home scaffolding
//! (dirs/.gitignore/hook), idempotency, hook behavior, and the `--config`
//! composition with the existing clone+register machinery. Uses an isolated
//! HOME + WORKESTRATE_CONFIG per test so the user's real home is never touched.

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
    let home = IsolatedHome::new("cmd-config-init");
    let store = home.dir.join(".workestrate");

    let out = home
        .cmd()
        .env("WORKESTRATE_CONFIG", &store)
        .args(["config", "init"])
        .output()
        .expect("invoke config init");
    assert!(
        out.status.success(),
        "config init failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Layout dirs + .git.
    for dir in ["fleets", "sources", "state", "secrets"] {
        assert!(
            store.join(dir).is_dir(),
            "expected dir {} under the config",
            dir
        );
    }
    assert!(store.join(".git").is_dir(), ".git must exist after init");

    // .gitignore: all 9 entries present, each on its own line; no *.enc.
    let gitignore = std::fs::read_to_string(store.join(".gitignore")).expect("read .gitignore");
    let lines: Vec<&str> = gitignore.lines().collect();
    for entry in [
        "/fleets/",
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
        hook_content.contains("fleets"),
        "hook must reject store-dir paths:\n{}",
        hook_content
    );
    // tombi gates (spec 15): best-effort block — absent or version-mismatched
    // tombi must skip with an audible stderr echo, never hard-fail (matching
    // the fleet shims; exactness is enforced by `nix flake check`).
    assert!(
        hook_content.contains("tombi format --check"),
        "hook must run the tombi format gate:\n{}",
        hook_content
    );
    assert!(
        hook_content.contains("--error-on-warnings"),
        "hook must run the tombi lint gate with --error-on-warnings:\n{}",
        hook_content
    );
    assert!(
        hook_content.contains("TOMBI_REQUIRED=\"1.2.5\""),
        "hook must pin TOMBI_REQUIRED to 1.2.5:\n{}",
        hook_content
    );
    assert!(
        hook_content.contains("tombi_version=\"$(tombi --version | awk '{print $2}')\""),
        "hook must extract tombi version via awk:\n{}",
        hook_content
    );
    assert!(
        hook_content.contains(
            "config pre-commit: tombi version mismatch (found '${tombi_version:-unknown}', want $TOMBI_REQUIRED); skipping tombi gates"
        ),
        "hook must announce a tombi version mismatch as an audible skip:\n{}",
        hook_content
    );
    assert!(
        hook_content.contains("config pre-commit: tombi not found; skipping tombi gates"),
        "hook must announce a missing tombi as an audible skip:\n{}",
        hook_content
    );
    assert!(
        !hook_content.contains("cargo install tombi"),
        "hook must not hard-fail with a cargo-install suggestion on version mismatch:\n{}",
        hook_content
    );

    // tombi toolchain files: home tombi.toml + vendored schemas (spec 15).
    let tombi_toml = std::fs::read_to_string(store.join("tombi.toml")).expect("read tombi.toml");
    assert!(
        tombi_toml.contains("fleets/*/workestrate.toml"),
        "home tombi.toml must include fleets/*/workestrate.toml:\n{}",
        tombi_toml
    );
    assert!(
        tombi_toml.contains("fleets/*/workestrate/default.toml")
            && tombi_toml.contains("fleets/*/workestrate/secrets.toml"),
        "home tombi.toml must include the full-schema directory-mode entries default.toml and \
         secrets.toml:\n{}",
        tombi_toml
    );
    assert!(
        tombi_toml.contains("fleets/*/workestrate/workloads/**/*.toml"),
        "home tombi.toml must include the workload capsule glob:\n{}",
        tombi_toml
    );
    assert!(
        tombi_toml.contains("schemas/workestrate-workload.schema.json"),
        "home tombi.toml must reference the workload subschema:\n{}",
        tombi_toml
    );
    assert!(
        tombi_toml.contains("schemas/registry.schema.json"),
        "home tombi.toml must reference the registry schema:\n{}",
        tombi_toml
    );
    assert!(
        tombi_toml.matches("[[schemas]]").count() == 3,
        "home tombi.toml must carry three [[schemas]] mappings (full + workload subschema + registry):\n{}",
        tombi_toml
    );
    let schema = std::fs::read_to_string(store.join("schemas").join("workestrate.schema.json"))
        .expect("read vendored schema");
    assert!(
        schema.contains("\"$schema\"") || schema.contains("\"title\""),
        "vendored schema must look like a JSON Schema document"
    );
    let workload_schema = std::fs::read_to_string(
        store
            .join("schemas")
            .join("workestrate-workload.schema.json"),
    )
    .expect("read vendored workload subschema");
    assert!(
        workload_schema.contains("workload capsule entry file"),
        "vendored workload subschema must carry the custom capsule title"
    );
    let registry_schema =
        std::fs::read_to_string(store.join("schemas").join("registry.schema.json"))
            .expect("read vendored registry schema");
    assert!(
        registry_schema.contains("Registry") || registry_schema.contains("\"$schema\""),
        "vendored registry schema must look like a JSON Schema document"
    );
}

#[test]
fn second_run_is_idempotent_noop() {
    let home = IsolatedHome::new("cmd-config-init");
    let store = home.dir.join(".workestrate");

    let first = home
        .cmd()
        .env("WORKESTRATE_CONFIG", &store)
        .args(["config", "init"])
        .output()
        .expect("first config init");
    assert!(
        first.status.success(),
        "first config init failed: stderr=\n{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let gitignore_before =
        std::fs::read_to_string(store.join(".gitignore")).expect("read .gitignore (first)");
    let tombi_before =
        std::fs::read_to_string(store.join("tombi.toml")).expect("read tombi.toml (first)");

    let second = home
        .cmd()
        .env("WORKESTRATE_CONFIG", &store)
        .args(["config", "init"])
        .output()
        .expect("second config init");
    assert!(
        second.status.success(),
        "second config init must succeed (idempotent no-op): stderr=\n{}",
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
    let tombi_after =
        std::fs::read_to_string(store.join("tombi.toml")).expect("read tombi.toml (second)");
    assert_eq!(
        tombi_before, tombi_after,
        "tombi.toml must be identical between runs (no rewrite on the no-op path)"
    );
}

#[test]
fn hook_rejects_gitlink() {
    let home = IsolatedHome::new("cmd-config-init");
    let store = home.dir.join(".workestrate");

    let out = home
        .cmd()
        .env("WORKESTRATE_CONFIG", &store)
        .args(["config", "init"])
        .output()
        .expect("invoke config init");
    assert!(
        out.status.success(),
        "config init failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Create an embedded standalone repo inside the config, then stage it as a
    // gitlink (mode 160000) in the config index.
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

/// Behavioral (spec 15 degradation): the hook must exit 0 with an audible
/// stderr echo when tombi is ABSENT or version-MISMATCHED — the tombi gates
/// skip, they never brick a commit. (tombi present + matching is host-nix
/// territory and not asserted here.)
#[test]
fn hook_skips_tombi_gates_audibly_when_absent_or_mismatched() {
    let home = IsolatedHome::new("cmd-config-init");
    let store = home.dir.join(".workestrate");

    let out = home
        .cmd()
        .env("WORKESTRATE_CONFIG", &store)
        .args(["config", "init"])
        .output()
        .expect("invoke config init");
    assert!(
        out.status.success(),
        "config init failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let hook = store.join(".git").join("hooks").join("pre-commit");

    // Case absent: PATH points at an empty temp dir so `command -v tombi`
    // fails inside the hook (git/grep/awk are missing too — the guards then
    // see an empty index and pass vacuously, same trick as
    // config_new_hook_behavioral). Must warn and exit 0.
    let empty_path = TempDir::new("cmd-home-init-empty-path");
    let out = Command::new("/bin/sh")
        .arg(&hook)
        .current_dir(&store)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("PATH", empty_path.path())
        .output()
        .expect("run pre-commit hook (tombi absent)");
    assert!(
        out.status.success(),
        "hook must exit 0 when tombi is absent; stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("tombi not found; skipping tombi gates"),
        "hook stderr must note the skipped tombi gates; got:\n{}",
        stderr
    );

    // Case mismatch: a fake `tombi` printing a wrong version shadows the
    // real one (dir prepended to the host PATH so git/awk still resolve);
    // the hook must skip with an audible echo and still exit 0.
    let fake_bin = TempDir::new("cmd-home-init-fake-tombi");
    let fake_tombi = fake_bin.path().join("tombi");
    std::fs::write(&fake_tombi, "#!/bin/sh\necho 'tombi 0.0.0-fake'\n").expect("write fake tombi");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake_tombi, std::fs::Permissions::from_mode(0o755))
            .expect("chmod fake tombi");
    }
    let path_with_fake = format!(
        "{}:{}",
        fake_bin.path().display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let out = Command::new("/bin/sh")
        .arg(&hook)
        .current_dir(&store)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("PATH", &path_with_fake)
        .output()
        .expect("run pre-commit hook (tombi mismatched)");
    assert!(
        out.status.success(),
        "hook must exit 0 on a tombi version mismatch; stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("tombi version mismatch") && stderr.contains("skipping tombi gates"),
        "hook stderr must note the mismatch skip; got:\n{}",
        stderr
    );
}
