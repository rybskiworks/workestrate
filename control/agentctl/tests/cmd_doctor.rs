//! Integration tests for `workestrate doctor` — JSON shape, check-name
//! coverage, human report sections, overall-verdict values, and config-repo
//! health reporting. Uses an isolated HOME + XDG per test so the user's real
//! registry and state are never touched. Individual check statuses are
//! environment-dependent (KVM, nix, msb may or may not exist), so tests
//! assert shape and names rather than specific OK/FAIL outcomes.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::IsolatedHome;
/// `doctor --json` emits parseable JSON with a checks array and an overall
/// verdict; every check carries name/status/message.
#[test]
fn doctor_json_is_parseable() {
    let home = IsolatedHome::new("cmd-doctor");
    let out = home
        .cmd()
        .args(["doctor", "--json"])
        .output()
        .expect("invoke doctor --json");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let doc: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("doctor --json stdout must be valid JSON: {e}\n{stdout}"));

    let checks = doc["checks"].as_array().expect("checks must be an array");
    assert!(!checks.is_empty(), "checks array must not be empty");
    for check in checks {
        assert!(check["name"].is_string(), "check missing name: {check}");
        let status = check["status"].as_str().expect("check missing status");
        assert!(
            matches!(status, "OK" | "WARN" | "FAIL"),
            "unexpected check status: {status}"
        );
        assert!(
            check["message"].is_string(),
            "check missing message: {check}"
        );
    }
    let overall = doc["overall"].as_str().expect("missing overall field");
    assert!(
        matches!(overall, "OK" | "WARN" | "FAIL"),
        "unexpected overall value: {overall}"
    );
}

/// W1 regression: the GLOBAL --json placed BEFORE the subcommand
/// (`workestrate --json doctor`) must also emit JSON — previously a local
/// --json on the Doctor variant shadowed the global, so this invocation
/// silently emitted the human report.
#[test]
fn doctor_global_json_before_subcommand_emits_json() {
    let home = IsolatedHome::new("cmd-doctor");
    let out = home
        .cmd()
        .args(["--json", "doctor"])
        .output()
        .expect("invoke --json doctor");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let doc: serde_json::Value = serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("--json doctor stdout must be valid JSON: {e}\n{stdout}"));

    let checks = doc["checks"].as_array().expect("checks must be an array");
    assert!(!checks.is_empty(), "checks array must not be empty");
    let overall = doc["overall"].as_str().expect("missing overall field");
    assert!(
        matches!(overall, "OK" | "WARN" | "FAIL"),
        "unexpected overall value: {overall}"
    );
}

/// The JSON report covers every documented check name.
#[test]
fn doctor_json_has_expected_check_names() {
    let home = IsolatedHome::new("cmd-doctor");
    let out = home
        .cmd()
        .args(["doctor", "--json"])
        .output()
        .expect("invoke doctor --json");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let doc: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor --json stdout must be valid JSON");
    let names: Vec<&str> = doc["checks"]
        .as_array()
        .expect("checks must be an array")
        .iter()
        .filter_map(|c| c["name"].as_str())
        .collect();
    for expected in [
        "dev_kvm",
        "nix",
        "sops",
        "age_keygen",
        "age_key_file",
        "msb",
        "home",
        "config_repos",
    ] {
        assert!(
            names.contains(&expected),
            "missing check '{expected}'; got: {names:?}"
        );
    }
}

/// Human output has the section header and mentions the key checks.
#[test]
fn doctor_human_output_has_section_header() {
    let home = IsolatedHome::new("cmd-doctor");
    let out = home.cmd().args(["doctor"]).output().expect("invoke doctor");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("=== workestrate doctor ==="),
        "missing section header; got: {stdout}"
    );
    assert!(
        stdout.contains("dev_kvm"),
        "missing dev_kvm check; got: {stdout}"
    );
    assert!(
        stdout.contains("config_repos"),
        "missing config_repos check; got: {stdout}"
    );
}

/// The command always runs and reports a valid overall verdict; the exit
/// code itself is environment-dependent (FAIL exits 1, OK/WARN exit 0) and
/// must agree with the reported verdict.
#[test]
fn doctor_overall_verdict_matches_exit_code() {
    let home = IsolatedHome::new("cmd-doctor");
    let out = home
        .cmd()
        .args(["doctor", "--json"])
        .output()
        .expect("invoke doctor --json");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let doc: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor --json stdout must be valid JSON");
    let overall = doc["overall"].as_str().expect("missing overall field");
    assert!(
        matches!(overall, "OK" | "WARN" | "FAIL"),
        "unexpected overall value: {overall}"
    );
    if overall == "FAIL" {
        assert!(
            !out.status.success(),
            "overall FAIL must exit non-zero; got success"
        );
    } else {
        assert!(
            out.status.success(),
            "overall {overall} must exit zero; stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// A registered config repo whose store clone is missing (or is not a real
/// git clone) shows up in the config_repos check with a non-OK status.
#[test]
fn doctor_config_repos_reports_registered_repo() {
    let home = IsolatedHome::new("cmd-doctor");
    home.write_registry(
        r#"
[configs.personal]
url = "https://example.invalid/personal.git"
ref = "main"
"#,
    );
    // Create the repo dir (empty, not a git clone) where the store expects it:
    // legacy-XDG layout → $XDG_DATA_HOME/workestrate/config-repos/<name>.
    let repo_dir = home
        .dir
        .join(".local")
        .join("share")
        .join("workestrate")
        .join("config-repos")
        .join("personal");
    std::fs::create_dir_all(&repo_dir).expect("create fake repo dir");

    let out = home
        .cmd()
        .args(["doctor", "--json"])
        .output()
        .expect("invoke doctor --json");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let doc: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor --json stdout must be valid JSON");
    let checks = doc["checks"].as_array().expect("checks must be an array");
    let config_repos = checks
        .iter()
        .find(|c| c["name"] == "config_repos")
        .expect("config_repos check must exist");
    let repos = config_repos["repos"]
        .as_array()
        .expect("config_repos must carry a repos array");
    let personal = repos
        .iter()
        .find(|r| r["name"] == "personal")
        .expect("registered repo 'personal' must be reported");
    let status = personal["status"].as_str().expect("repo missing status");
    assert!(
        matches!(status, "WARN" | "FAIL"),
        "fake repo dir must not report OK; got status: {status}"
    );
}
