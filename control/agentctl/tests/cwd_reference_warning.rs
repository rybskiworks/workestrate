//! Spec-05 + cleanup-phase-2 regression: the reference config gates emit the
//! correct one-time stderr signals.
//!
//! The unit tests in `src/config/paths.rs` pin the return-value behavior of
//! `reference_config_path()`, but the warning/note emission goes through
//! `std::sync::Once` statics (fire once per process), so in-process stderr
//! capture is unreliable. This integration test spawns the built binary in a
//! fresh process per scenario and asserts on its stderr.
//!
//! Two stacked gates are covered:
//!   1. cleanup phase 2: the reference base layer is opt-in
//!      (`WORKESTRATE_REFERENCE_CONFIG=1`); without it the reference is
//!      ignored with a one-time note.
//!   2. spec 05: with the phase-2 opt-in set, a CWD-DERIVED reference still
//!      requires `WORKESTRATE_ALLOW_CWD_REFERENCE=1`.
//!
//! Command choice: `workestrate check` (cmd_check,
//! `src/commands/diagnostics.rs:292`) — it calls `config::load_config()` in
//! its "Source overrides" section, which resolves `reference_config_path()`
//! as its base layer, and it needs no msb/runtime, no registry, and tolerates
//! a failing exit status (we ignore it).

// Test harness: panic!/expect are the idiomatic way to fail a test, so the
// crate-wide clippy denies are relaxed here (mirrors tests/schema_drift.rs).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

use std::path::{Path, PathBuf};
use std::process::Command;

/// Unique scratch dir under the system temp dir.
fn uniq_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "workestrate-{}-{}-{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}

/// Build a workbench-shaped cwd fixture: `<tmp>/flake.nix` +
/// `<tmp>/config.reference/workestrate.toml`.
fn make_cwd_fixture(label: &str) -> PathBuf {
    let tmp = uniq_dir(label);
    std::fs::create_dir_all(tmp.join("config.reference")).expect("create config.reference dir");
    std::fs::write(tmp.join("flake.nix"), "").expect("write flake.nix");
    std::fs::write(
        tmp.join("config.reference").join("workestrate.toml"),
        "schema_version = 1\n",
    )
    .expect("write reference config");
    tmp
}

/// Run `workestrate check` from `cwd` with a hermetic HOME/XDG scratch home
/// and both reference-gate env keys cleared. `phase2_opt_in` sets
/// `WORKESTRATE_REFERENCE_CONFIG=1` (reference base layer opt-in);
/// `cwd_opt_in` additionally sets `WORKESTRATE_ALLOW_CWD_REFERENCE=1`
/// (spec-05 cwd-derived opt-in).
fn run_check(cwd: &Path, scratch_home: &Path, phase2_opt_in: bool, cwd_opt_in: bool) -> String {
    let bin = env!("CARGO_BIN_EXE_workestrate");
    let mut cmd = Command::new(bin);
    cmd.arg("check")
        .current_dir(cwd)
        // Clear every env that could steer root/config resolution.
        .env_remove("AGENTCTL_ROOT")
        .env_remove("CARGO_MANIFEST_DIR")
        .env_remove("WORKESTRATE_ALLOW_CWD_REFERENCE")
        .env_remove("WORKESTRATE_REFERENCE_CONFIG")
        .env_remove("WORKESTRATE_CONFIG_DIR")
        .env_remove("WORKESTRATE_HOME")
        .env_remove("WORKESTRATE_STATE_DIR")
        // Hermetic home/XDG so no real operator registry leaks in.
        .env("HOME", scratch_home)
        .env("XDG_CONFIG_HOME", scratch_home.join(".config"))
        .env("XDG_DATA_HOME", scratch_home.join(".local/share"))
        .env("XDG_STATE_HOME", scratch_home.join(".local/state"));
    if phase2_opt_in {
        cmd.env("WORKESTRATE_REFERENCE_CONFIG", "1");
    }
    if cwd_opt_in {
        cmd.env("WORKESTRATE_ALLOW_CWD_REFERENCE", "1");
    }
    let output = cmd.output().expect("failed to spawn `workestrate check`");
    // Exit status is intentionally ignored: cmd_check reports [MISSING] rows
    // for the hermetic scratch home and exits non-zero either way.
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn cwd_reference_gate_emits_expected_stderr_signals() {
    let cwd_fixture = make_cwd_fixture("spec05-cwd");
    let scratch_home = uniq_dir("spec05-home");
    std::fs::create_dir_all(&scratch_home).expect("create scratch home");

    // Scenario 0 (cleanup phase 2): NO opt-ins at all → the reference base
    // layer is IGNORED and a one-time note names the full path and the
    // phase-2 opt-in escape hatch. The spec-05 cwd gate never even fires.
    let stderr = run_check(&cwd_fixture, &scratch_home, false, false);
    assert!(
        stderr.contains("set WORKESTRATE_REFERENCE_CONFIG=1 to include it as the base layer"),
        "expected the phase-2 reference-ignored note on stderr; got:\n{stderr}"
    );
    assert!(
        !stderr.contains("WORKESTRATE_ALLOW_CWD_REFERENCE"),
        "the spec-05 cwd gate must stay unreachable without the phase-2 opt-in; got:\n{stderr}"
    );

    // Scenario 1: phase-2 opt-in only → the cwd-derived reference is IGNORED
    // and a one-time note names the full path and the spec-05 escape hatch.
    let stderr = run_check(&cwd_fixture, &scratch_home, true, false);
    assert!(
        stderr.contains("found in cwd (set WORKESTRATE_ALLOW_CWD_REFERENCE=1 to use it)"),
        "expected the ignored-cwd-reference note on stderr; got:\n{stderr}"
    );

    // Scenario 2: both opt-ins → the cwd-derived reference is LOADED and a
    // one-time warning names the cwd source plus the AGENTCTL_ROOT
    // remediation.
    let stderr = run_check(&cwd_fixture, &scratch_home, true, true);
    assert!(
        stderr.contains("reference config loaded from cwd"),
        "expected the loaded-from-cwd warning on stderr; got:\n{stderr}"
    );

    let _ = std::fs::remove_dir_all(&cwd_fixture);
    let _ = std::fs::remove_dir_all(&scratch_home);
}
