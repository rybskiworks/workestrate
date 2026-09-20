//! Integration tests for `workestrate schemas update` — the single-source
//! consumer distribution command: config target (write + idempotency +
//! --check), fleet targets (--fleet scoping, skip rules), and the
//! tool-template target (AGENTCTL_ROOT pinning).
//!
//! Every test pins `AGENTCTL_ROOT` to a scratch workbench root (flake.nix)
//! so the tool-template target can NEVER resolve the real repo: the child
//! inherits `CARGO_MANIFEST_DIR` from the test process, which would otherwise
//! resolve the real checkout (project-root tier 2) and mutate its
//! templates/workestrate-config/schemas/ during the suite. The parent test
//! process never writes outside its own temp dirs.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::{IsolatedHome, TempDir};
use std::path::{Path, PathBuf};

/// A scratch workbench root: flake.nix (so `project_root_optional` accepts
/// it) plus an optional `templates/workestrate-config/` dir. Pinned via
/// `AGENTCTL_ROOT` (tier 1) in every test command.
fn scratch_root(with_template: bool) -> TempDir {
    let root = TempDir::new("cmd-schemas-root");
    std::fs::write(root.path().join("flake.nix"), "").expect("write flake.nix");
    if with_template {
        std::fs::create_dir_all(root.path().join("templates").join("workestrate-config"))
            .expect("create copier template dir");
    }
    root
}

/// Write a registry TOML at `<store>/config.toml` — the single-config layout
/// path that `load_registry` reads when `WORKESTRATE_CONFIG=<store>` is set.
/// (`IsolatedHome::write_registry` targets the legacy-XDG path instead.)
fn write_store_registry(store: &Path, content: &str) {
    std::fs::create_dir_all(store).expect("create store dir");
    std::fs::write(store.join("config.toml"), content).expect("write store registry");
}

/// The canonical committed schema at the workspace root (`schemas/<name>`),
/// resolved from CARGO_MANIFEST_DIR (…/control/agentctl).
fn committed_schema(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("schemas")
        .join(name)
}

#[test]
fn update_writes_both_schemas_into_config() {
    let home = IsolatedHome::new("cmd-schemas");
    let _root = scratch_root(false);
    let store = home.dir.join(".workestrate");
    std::fs::create_dir_all(store.join("schemas")).expect("create store schemas dir");
    std::fs::write(
        store.join("schemas").join("workestrate.schema.json"),
        "{\"stale\": true}\n",
    )
    .expect("write stale schema");

    let out = home
        .cmd()
        .env("WORKESTRATE_CONFIG", &store)
        .env("AGENTCTL_ROOT", _root.path())
        .args(["schemas", "update"])
        .output()
        .expect("invoke schemas update");
    assert!(
        out.status.success(),
        "schemas update failed: stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // All three artifacts exist in <store>/schemas/.
    let full = std::fs::read_to_string(store.join("schemas").join("workestrate.schema.json"))
        .expect("read full schema");
    assert!(
        full.contains("\"$schema\""),
        "full schema must carry the $schema pointer; got:\n{full}"
    );
    let workload = std::fs::read_to_string(
        store
            .join("schemas")
            .join("workestrate-workload.schema.json"),
    )
    .expect("read workload schema");
    assert!(
        workload.contains("workload capsule entry file"),
        "workload schema must carry the capsule title; got:\n{workload}"
    );
    let registry = std::fs::read_to_string(store.join("schemas").join("registry.schema.json"))
        .expect("read registry schema");
    assert!(
        registry.contains("Registry") || registry.contains("\"$schema\""),
        "registry schema must look like a JSON Schema document; got:\n{registry}"
    );
    // The stale file was replaced (no longer the hand-written marker).
    assert!(
        !full.contains("\"stale\": true"),
        "stale marker must have been replaced; got:\n{full}"
    );
}

#[test]
fn update_is_idempotent_second_run_all_skipped() {
    let home = IsolatedHome::new("cmd-schemas");
    let _root = scratch_root(false);
    let store = home.dir.join(".workestrate");
    std::fs::create_dir_all(store.join("schemas")).expect("create store schemas dir");
    std::fs::write(
        store.join("schemas").join("workestrate.schema.json"),
        "{\"stale\": true}\n",
    )
    .expect("write stale schema");

    let run = || {
        home.cmd()
            .env("WORKESTRATE_CONFIG", &store)
            .env("AGENTCTL_ROOT", _root.path())
            .args(["schemas", "update"])
            .output()
            .expect("invoke schemas update")
    };

    let first = run();
    assert!(
        first.status.success(),
        "first run failed: stderr=\n{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let path = store.join("schemas").join("workestrate.schema.json");
    let after_first = std::fs::read(&path).expect("read schema after first run");

    let second = run();
    assert!(
        second.status.success(),
        "second run failed: stderr=\n{}",
        String::from_utf8_lossy(&second.stderr)
    );
    let stdout = String::from_utf8_lossy(&second.stdout);
    assert_eq!(
        stdout.matches("(unchanged)").count(),
        3,
        "second run must skip ALL THREE files; got:\n{stdout}"
    );
    assert!(
        !stdout.contains("wrote"),
        "second run must not write anything; got:\n{stdout}"
    );
    let after_second = std::fs::read(&path).expect("read schema after second run");
    assert_eq!(
        after_first, after_second,
        "file must be byte-identical after the idempotent second run"
    );
}

#[test]
fn update_check_reports_stale_and_exits_1() {
    let home = IsolatedHome::new("cmd-schemas");
    let _root = scratch_root(false);
    let store = home.dir.join(".workestrate");
    std::fs::create_dir_all(store.join("schemas")).expect("create store schemas dir");
    std::fs::write(
        store.join("schemas").join("workestrate.schema.json"),
        "{\"stale\": true}\n",
    )
    .expect("write stale schema");

    let check = || {
        home.cmd()
            .env("WORKESTRATE_CONFIG", &store)
            .env("AGENTCTL_ROOT", _root.path())
            .args(["schemas", "update", "--check"])
            .output()
            .expect("invoke schemas update --check")
    };

    // Stale config copy → --check exits 1 and names the stale file.
    let first = check();
    assert_eq!(
        first.status.code(),
        Some(1),
        "--check with stale copies must exit 1; stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&first.stdout),
        String::from_utf8_lossy(&first.stderr)
    );
    let stdout = String::from_utf8_lossy(&first.stdout);
    assert!(
        stdout.contains("stale"),
        "--check must report the stale file; got:\n{stdout}"
    );

    // Update fixes everything.
    let up = home
        .cmd()
        .env("WORKESTRATE_CONFIG", &store)
        .env("AGENTCTL_ROOT", _root.path())
        .args(["schemas", "update"])
        .output()
        .expect("invoke schemas update");
    assert!(
        up.status.success(),
        "update after --check failed: stderr=\n{}",
        String::from_utf8_lossy(&up.stderr)
    );

    // Now --check is clean: exit 0, "fresh".
    let second = check();
    assert_eq!(
        second.status.code(),
        Some(0),
        "--check with fresh copies must exit 0; stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(
        String::from_utf8_lossy(&second.stdout).contains("fresh"),
        "--check must report all fresh; got:\n{}",
        String::from_utf8_lossy(&second.stdout)
    );
}

#[test]
fn update_fleet_target_writes_only_that_fleet() {
    let home = IsolatedHome::new("cmd-schemas");
    let _root = scratch_root(false);
    let store = home.dir.join(".workestrate");

    // A LOCAL fixture fleet under the store: dir + schemas/ with a
    // stale schema.
    let repo = home.fleet_dir("personal");
    std::fs::create_dir_all(repo.join("schemas")).expect("create repo schemas dir");
    std::fs::write(
        repo.join("schemas").join("workestrate.schema.json"),
        "{\"stale\": true}\n",
    )
    .expect("write stale repo schema");
    // Register it via the single-config registry (WORKESTRATE_CONFIG=<store>).
    write_store_registry(
        &store,
        &format!("[fleets.personal]\nurl = \"{}\"\n", repo.display()),
    );

    let out = home
        .cmd()
        .env("WORKESTRATE_CONFIG", &store)
        .env("AGENTCTL_ROOT", _root.path())
        .args(["schemas", "update", "--fleet", "personal"])
        .output()
        .expect("invoke schemas update --fleet personal");
    assert!(
        out.status.success(),
        "schemas update --fleet failed: stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // The repo's schemas/ now carry the real artifacts.
    let full = std::fs::read_to_string(repo.join("schemas").join("workestrate.schema.json"))
        .expect("read repo full schema");
    assert_eq!(
        full,
        std::fs::read_to_string(committed_schema("workestrate.schema.json"))
            .expect("read committed full schema"),
        "repo full schema must match the real schema byte-for-byte"
    );
    let workload = std::fs::read_to_string(
        repo.join("schemas")
            .join("workestrate-workload.schema.json"),
    )
    .expect("read repo workload schema");
    assert_eq!(
        workload,
        std::fs::read_to_string(committed_schema("workestrate-workload.schema.json"))
            .expect("read committed workload schema"),
        "repo workload schema must match the real schema byte-for-byte"
    );
    let registry = std::fs::read_to_string(repo.join("schemas").join("registry.schema.json"))
        .expect("read repo registry schema");
    assert_eq!(
        registry,
        std::fs::read_to_string(committed_schema("registry.schema.json"))
            .expect("read committed registry schema"),
        "repo registry schema must match the real schema byte-for-byte"
    );

    // --fleet scoping: the HOME target was NOT written (no <store>/schemas/).
    assert!(
        !store.join("schemas").exists(),
        "--fleet must skip the config target; <store>/schemas/ must not exist"
    );
}

#[test]
fn update_skips_fleet_without_schemas_dir() {
    let home = IsolatedHome::new("cmd-schemas");
    let _root = scratch_root(false);
    let store = home.dir.join(".workestrate");

    // A hand-made fleet: the dir exists with workestrate.toml but NO
    // schemas/ dir — the tool must not invent one there.
    let repo = home.fleet_dir("personal");
    std::fs::create_dir_all(&repo).expect("create repo dir");
    std::fs::write(repo.join("workestrate.toml"), "schema_version = 1\n")
        .expect("write repo workestrate.toml");
    write_store_registry(
        &store,
        &format!("[fleets.personal]\nurl = \"{}\"\n", repo.display()),
    );

    let out = home
        .cmd()
        .env("WORKESTRATE_CONFIG", &store)
        .env("AGENTCTL_ROOT", _root.path())
        .args(["schemas", "update"])
        .output()
        .expect("invoke schemas update");
    assert!(
        out.status.success(),
        "schemas update must succeed despite the skipped repo; stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("no schemas/ dir"),
        "expected a skip note naming the missing schemas/ dir; got:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !repo.join("schemas").exists(),
        "the tool must NOT create a schemas/ dir in a hand-made repo"
    );
}

#[test]
fn update_skips_missing_git_clone() {
    let home = IsolatedHome::new("cmd-schemas");
    let _root = scratch_root(false);
    let store = home.dir.join(".workestrate");

    // A git-URL entry whose store clone was never created.
    write_store_registry(
        &store,
        r#"[fleets.work]
url = "https://example.com/repo.git"
ref = "main"
"#,
    );

    let out = home
        .cmd()
        .env("WORKESTRATE_CONFIG", &store)
        .env("AGENTCTL_ROOT", _root.path())
        .args(["schemas", "update"])
        .output()
        .expect("invoke schemas update");
    assert!(
        out.status.success(),
        "schemas update must succeed when a clone is missing; stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("clone not present"),
        "expected a skip note naming the missing clone; got:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn update_tool_template_target_uses_agentctl_root() {
    let home = IsolatedHome::new("cmd-schemas");
    // Scratch workbench root WITH the copier template (no schemas/ subdir
    // initially), plus an uninitialized isolated home (home target skips).
    let root = scratch_root(true);
    let template = root.path().join("templates").join("workestrate-config");
    assert!(
        !template.join("schemas").exists(),
        "fixture must start without a schemas/ subdir"
    );

    let out = home
        .cmd()
        .env("AGENTCTL_ROOT", root.path())
        .args(["schemas", "update"])
        .output()
        .expect("invoke schemas update");
    assert!(
        out.status.success(),
        "schemas update failed: stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // The template's schemas/ subdir was created and populated.
    let full = std::fs::read_to_string(template.join("schemas").join("workestrate.schema.json"))
        .expect("read template full schema");
    assert!(
        full.contains("\"$schema\""),
        "template full schema missing $schema pointer; got:\n{full}"
    );
    let workload = std::fs::read_to_string(
        template
            .join("schemas")
            .join("workestrate-workload.schema.json"),
    )
    .expect("read template workload schema");
    assert!(
        workload.contains("workload capsule entry file"),
        "template workload schema missing the capsule title; got:\n{workload}"
    );
}
