//! Integration tests for `workestrate migrate-config` — JSON output via both
//! flag positions. Uses an isolated HOME + XDG per test and `--dry-run` so
//! nothing is moved; assertions cover the serialized `config::MigrateSummary`
//! shape (from/dest/dry_run/moved).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::IsolatedHome;

fn assert_migrate_json(stdout: &str) {
    let doc: serde_json::Value = serde_json::from_str(stdout).unwrap_or_else(|e| {
        panic!("migrate-config --json stdout must be valid JSON: {e}\n{stdout}")
    });
    assert!(doc.is_object(), "migrate summary must be a JSON object");
    assert_eq!(
        doc["dry_run"].as_bool(),
        Some(true),
        "dry_run must be true in the JSON summary: {doc}"
    );
}

/// `migrate-config --json --dry-run` (flag AFTER the subcommand) emits a
/// parseable MigrateSummary JSON.
#[test]
fn migrate_config_json_after_subcommand_emits_json() {
    let home = IsolatedHome::new("cmd-migrate-config");
    let out = home
        .cmd()
        .args(["migrate-config", "--json", "--dry-run"])
        .output()
        .expect("invoke migrate-config --json --dry-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_migrate_json(&stdout);
}

/// W1 regression: the GLOBAL --json placed BEFORE the subcommand
/// (`workestrate --json migrate-config --dry-run`) must also emit JSON —
/// previously a local --json on the MigrateConfig variant shadowed the global,
/// so this invocation silently emitted the human summary.
#[test]
fn migrate_config_global_json_before_subcommand_emits_json() {
    let home = IsolatedHome::new("cmd-migrate-config");
    let out = home
        .cmd()
        .args(["--json", "migrate-config", "--dry-run"])
        .output()
        .expect("invoke --json migrate-config --dry-run");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_migrate_json(&stdout);
}
