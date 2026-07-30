//! Integration tests for the global `--home <DIR>` flag (spec 06):
//! the flag populates the existing `WORKESTRATE_HOME` precedence step, so it
//! wins over an ambient export, works in any position (global), shows up in
//! `--help`, and does NOT perturb the legacy-XDG resolution path when absent.
//! Uses an isolated HOME + XDG per test so the user's real home is never
//! touched.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::{IsolatedHome, TempDir, BIN};
use std::path::Path;
use std::process::Command;

/// A registry naming a single config entry `marker` (unique to home B).
const MARKER_REGISTRY: &str =
    "[configs.marker]\nurl = \"https://example.com/marker.git\"\nref = \"main\"\n";

/// Write `MARKER_REGISTRY` as the single-home registry at `<home>/config.toml`.
fn write_marker_registry(home_dir: &Path) {
    std::fs::create_dir_all(home_dir).expect("create home dir");
    std::fs::write(home_dir.join("config.toml"), MARKER_REGISTRY).expect("write marker registry");
}

/// (a)+(b) The flag sets the tool home AND beats an exported
/// WORKESTRATE_HOME: with WORKESTRATE_HOME=/a in the environment and the
/// `marker` registry only in /b, `--home /b` must resolve /b (stdout lists
/// "marker"), while the same env without the flag resolves /a (no "marker").
#[test]
fn home_flag_beats_exported_env() {
    let shell = IsolatedHome::new("cmd-home-flag");
    let a = TempDir::new("cmd-home-flag-a");
    let b = TempDir::new("cmd-home-flag-b");
    write_marker_registry(b.path());

    let out = shell
        .cmd()
        .env("WORKESTRATE_HOME", a.path())
        .args([
            "--home",
            b.path().to_str().expect("utf8 home b"),
            "config",
            "list",
        ])
        .output()
        .expect("invoke --home config list");
    assert!(
        out.status.success(),
        "config list with --home failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("marker"),
        "--home /b must resolve /b (its registry entry 'marker' must be listed); got:\n{}",
        stdout
    );

    // Control: same exported env, no flag → /a wins (its registry is absent).
    let out = shell
        .cmd()
        .env("WORKESTRATE_HOME", a.path())
        .args(["config", "list"])
        .output()
        .expect("invoke config list (control)");
    assert!(
        out.status.success(),
        "control config list failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("marker"),
        "without --home the exported WORKESTRATE_HOME=/a must win (no 'marker'); got:\n{}",
        stdout
    );
}

/// (d) `--home` is global: it works AFTER the subcommand too.
#[test]
fn home_flag_works_after_subcommand() {
    let shell = IsolatedHome::new("cmd-home-flag");
    let a = TempDir::new("cmd-home-flag-a");
    let b = TempDir::new("cmd-home-flag-b");
    write_marker_registry(b.path());

    let out = shell
        .cmd()
        .env("WORKESTRATE_HOME", a.path())
        .args([
            "config",
            "list",
            "--home",
            b.path().to_str().expect("utf8 home b"),
        ])
        .output()
        .expect("invoke config list --home");
    assert!(
        out.status.success(),
        "config list --home (trailing) failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("marker"),
        "trailing --home /b must also resolve /b; got:\n{}",
        stdout
    );
}

/// (e) `--help` surfaces the flag: `workestrate --help` shows `--home <DIR>`.
#[test]
fn home_flag_shows_in_help() {
    let out = Command::new(BIN)
        .args(["--help"])
        .output()
        .expect("invoke --help");
    assert!(
        out.status.success(),
        "--help failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--home <DIR>"),
        "--help must list '--home <DIR>'; got:\n{}",
        stdout
    );
}

/// With NO --home and NO WORKESTRATE_HOME, the legacy-XDG path is unchanged
/// by the flag's existence: a registry at $XDG_CONFIG_HOME/workestrate/
/// config.toml is still honored. The binary prints a legacy-XDG migration
/// note on stderr in this mode — assert on stdout only.
#[test]
fn legacy_xdg_resolution_unchanged_without_flag() {
    let shell = IsolatedHome::new("cmd-home-flag-xdg");
    shell.write_registry(MARKER_REGISTRY);

    let out = shell
        .cmd()
        .args(["config", "list"])
        .output()
        .expect("invoke config list (legacy xdg)");
    assert!(
        out.status.success(),
        "config list (legacy xdg) failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("marker"),
        "legacy XDG registry must still be honored when --home is absent; got:\n{}",
        stdout
    );
}
