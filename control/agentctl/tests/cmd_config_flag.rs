//! Integration tests for the global `--config <DIR>` flag (spec 06):
//! the flag populates the existing `WORKESTRATE_CONFIG` precedence step, so it
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

use common::{BIN, IsolatedHome, TempDir};
use std::path::Path;
use std::process::Command;

/// A registry naming a single fleet entry `marker` (unique to config B).
const MARKER_REGISTRY: &str =
    "[fleets.marker]\nurl = \"https://example.com/marker.git\"\nref = \"main\"\n";

/// Write `MARKER_REGISTRY` as the single-entry registry at `<config>/config.toml`.
fn write_marker_registry(config_dir: &Path) {
    std::fs::create_dir_all(config_dir).expect("create config dir");
    std::fs::write(config_dir.join("config.toml"), MARKER_REGISTRY).expect("write marker registry");
}

/// (a)+(b) The flag sets the config AND beats an exported
/// WORKESTRATE_CONFIG: with WORKESTRATE_CONFIG=/a in the environment and the
/// `marker` registry only in /b, `--config /b` must resolve /b (stdout lists
/// "marker"), while the same env without the flag resolves /a (no "marker").
#[test]
fn config_flag_beats_exported_env() {
    let shell = IsolatedHome::new("cmd-config-flag");
    let a = TempDir::new("cmd-config-flag-a");
    let b = TempDir::new("cmd-config-flag-b");
    write_marker_registry(b.path());

    let out = shell
        .cmd()
        .env("WORKESTRATE_CONFIG", a.path())
        .args([
            "--config",
            b.path().to_str().expect("utf8 config b"),
            "fleet",
            "list",
        ])
        .output()
        .expect("invoke --config fleet list");
    assert!(
        out.status.success(),
        "fleet list with --config failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("marker"),
        "--config /b must resolve /b (its registry entry 'marker' must be listed); got:\n{}",
        stdout
    );

    // Control: same exported env, no flag → /a wins (its registry is absent).
    let out = shell
        .cmd()
        .env("WORKESTRATE_CONFIG", a.path())
        .args(["fleet", "list"])
        .output()
        .expect("invoke fleet list (control)");
    assert!(
        out.status.success(),
        "control fleet list failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("marker"),
        "without --config the exported WORKESTRATE_CONFIG=/a must win (no 'marker'); got:\n{}",
        stdout
    );
}

/// (d) `--config` is global: it works AFTER the subcommand too.
#[test]
fn config_flag_works_after_subcommand() {
    let shell = IsolatedHome::new("cmd-config-flag");
    let a = TempDir::new("cmd-config-flag-a");
    let b = TempDir::new("cmd-config-flag-b");
    write_marker_registry(b.path());

    let out = shell
        .cmd()
        .env("WORKESTRATE_CONFIG", a.path())
        .args([
            "fleet",
            "list",
            "--config",
            b.path().to_str().expect("utf8 config b"),
        ])
        .output()
        .expect("invoke fleet list --config");
    assert!(
        out.status.success(),
        "fleet list --config (trailing) failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("marker"),
        "trailing --config /b must also resolve /b; got:\n{}",
        stdout
    );
}

/// (e) `--help` surfaces the flag: `workestrate --help` shows `--config <DIR>`.
#[test]
fn config_flag_shows_in_help() {
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
        stdout.contains("--config <DIR>"),
        "--help must list '--config <DIR>'; got:\n{}",
        stdout
    );
}

/// With NO --config and NO WORKESTRATE_CONFIG, the legacy-XDG path is unchanged
/// by the flag's existence: a registry at $XDG_CONFIG_HOME/workestrate/
/// config.toml is still honored. The binary prints a legacy-XDG migration
/// note on stderr in this mode — assert on stdout only.
#[test]
fn legacy_xdg_resolution_unchanged_without_flag() {
    let shell = IsolatedHome::new("cmd-config-flag-xdg");
    shell.write_registry(MARKER_REGISTRY);

    let out = shell
        .cmd()
        .args(["fleet", "list"])
        .output()
        .expect("invoke fleet list (legacy xdg)");
    assert!(
        out.status.success(),
        "fleet list (legacy xdg) failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("marker"),
        "legacy XDG registry must still be honored when --config is absent; got:\n{}",
        stdout
    );
}
