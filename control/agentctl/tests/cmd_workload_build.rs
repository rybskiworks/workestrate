//! Integration tests for the shared `--fleet` selector on `workload build`.
//! The build-local and global flags are one clap arg, so
//! `workestrate --fleet X workload build <name>` and
//! `workestrate workload build <name> --fleet X` both mean "build <name>
//! from fleet X" — coherently, and never via the former clap arg-id clash
//! that panicked with "internal error: entered unreachable code".

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::IsolatedHome;

const PERSONAL_CONFIG_TOML: &str = r#"
schema_version = 1
"#;

const WORK_CONFIG_TOML: &str = r#"
schema_version = 1

[workloads.prime]
kind = "service"
image = { recipe = "nix-layered", name = "wk-prime-image", tag = "latest" }
command = ["true"]

[workloads.prime.network.defaults]
egress = "deny"
"#;

/// Two registered fleets: `personal` (the default, no workloads) and
/// `work` (declares the nix-layered `prime`, deliberately WITHOUT a
/// flake.nix so a single-name build fails with the spec 21 §7 hard error —
/// an error reachable ONLY when prime was resolved out of fleet work).
fn two_fleet_home(label: &str) -> IsolatedHome {
    let home = IsolatedHome::new(label);
    let personal = home.create_fleet_dir("personal");
    std::fs::write(personal.join("workestrate.toml"), PERSONAL_CONFIG_TOML).unwrap();
    let work = home.create_fleet_dir("work");
    std::fs::write(work.join("workestrate.toml"), WORK_CONFIG_TOML).unwrap();
    home.write_registry(&format!(
        r#"
[settings]
default_fleet = "personal"

[fleets.personal]
url = "{}"

[fleets.work]
url = "{}"
"#,
        personal.display(),
        work.display()
    ));
    home
}

fn stderr_of(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn assert_never_panics(stderr: &str) {
    assert!(
        !stderr.contains("internal error") && !stderr.contains("unreachable"),
        "must never hit the clap arg-id clash panic: {stderr}"
    );
}

/// `--fleet work` BEFORE the subcommand: the global selector picks the
/// fleet the named workload is built from.
#[test]
fn global_fleet_selects_the_build_fleet_for_a_named_workload() {
    let home = two_fleet_home("cmd-build-shared-fleet-global");
    let out = home
        .cmd()
        .args(["--fleet", "work", "workload", "build", "prime"])
        .output()
        .expect("run build");
    let stderr = stderr_of(&out);
    assert_never_panics(&stderr);
    assert!(
        !out.status.success(),
        "prime's declaring fleet has no flake.nix — the §7 hard error fails the build: {stderr}"
    );
    assert!(
        stderr.contains("workload 'prime' declares a nix-layered image"),
        "the §7 hard error proves prime resolved: {stderr}"
    );
    assert!(
        !stderr.contains("not found in config"),
        "prime exists only in fleet work — the default fleet must not be the resolution source: {stderr}"
    );
}

/// `--fleet work` AFTER the subcommand: the build-local selector is the
/// same clap arg and must behave identically (main.rs bridges it into the
/// resolver env).
#[test]
fn build_local_fleet_selects_the_build_fleet_for_a_named_workload() {
    let home = two_fleet_home("cmd-build-shared-fleet-local");
    let out = home
        .cmd()
        .args(["workload", "build", "prime", "--fleet", "work"])
        .output()
        .expect("run build");
    let stderr = stderr_of(&out);
    assert_never_panics(&stderr);
    assert!(
        !out.status.success(),
        "prime's declaring fleet has no flake.nix — the §7 hard error fails the build: {stderr}"
    );
    assert!(
        stderr.contains("workload 'prime' declares a nix-layered image"),
        "the §7 hard error proves prime resolved: {stderr}"
    );
    assert!(
        !stderr.contains("not found in config"),
        "prime exists only in fleet work — the default fleet must not be the resolution source: {stderr}"
    );
}

/// `workload build --fleet work` (no name) keeps its batch meaning: build
/// everything declared by fleet work. With no flake.nix the batch skips
/// with a note and exits zero (zero-eligible no-op) — coherent, never a
/// panic.
#[test]
fn build_local_fleet_without_a_name_is_the_fleet_batch() {
    let home = two_fleet_home("cmd-build-shared-fleet-batch");
    let out = home
        .cmd()
        .args(["workload", "build", "--fleet", "work"])
        .output()
        .expect("run build");
    let stderr = stderr_of(&out);
    assert_never_panics(&stderr);
    assert!(
        stderr.contains("skipping workload 'prime'"),
        "batch mode notes the no-flake skip: {stderr}"
    );
    assert!(
        out.status.success(),
        "batch skip-with-note is a zero-eligible no-op: {stderr}"
    );
}

/// A propagated global `--fleet` plus `--all-fleets` bypasses the
/// parse-time conflicts; the dispatch rejects the mixed shape with a usage
/// error instead of the former unreachable!() panic.
#[test]
fn global_fleet_plus_all_fleets_is_a_usage_error_not_a_panic() {
    let home = two_fleet_home("cmd-build-shared-fleet-conflict");
    let out = home
        .cmd()
        .args(["--fleet", "work", "workload", "build", "--all-fleets"])
        .output()
        .expect("run build");
    let stderr = stderr_of(&out);
    assert_never_panics(&stderr);
    assert!(!out.status.success(), "mixed shape must fail: {stderr}");
    assert!(
        stderr.contains("--all-fleets cannot be combined"),
        "mixed shape must name the conflict: {stderr}"
    );
}
