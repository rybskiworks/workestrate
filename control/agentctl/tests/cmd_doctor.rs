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
        "generation",
        "home",
        "config_repos",
        "schemas",
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

/// A `schemas` row is present in the JSON report with the consumer-location
/// summary (an isolated home with no registry and no home still resolves the
/// tool-template target from the real checkout, so the row reports "consumer
/// location(s) checked"; status is OK or WARN depending on freshness).
#[test]
fn doctor_json_reports_schema_check_row() {
    let home = IsolatedHome::new("cmd-doctor");
    let out = home
        .cmd()
        .args(["doctor", "--json"])
        .output()
        .expect("invoke doctor --json");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let doc: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor --json stdout must be valid JSON");
    let checks = doc["checks"].as_array().expect("checks must be an array");
    let schemas = checks
        .iter()
        .find(|c| c["name"] == "schemas")
        .expect("schemas check must exist");
    let status = schemas["status"].as_str().expect("schemas missing status");
    assert!(
        matches!(status, "OK" | "WARN"),
        "schemas status must be OK or WARN; got: {status}"
    );
    let message = schemas["message"]
        .as_str()
        .expect("schemas missing message");
    assert!(
        message.contains("consumer"),
        "schemas message must mention consumer locations; got: {message}"
    );
    let repos = schemas["repos"]
        .as_array()
        .expect("schemas must carry a repos array");
    assert!(
        repos
            .iter()
            .all(|r| r["target"].is_string() && r["path"].is_string()),
        "every schema entry must carry target and path: {repos:?}"
    );
}

/// A stale consumer copy (tool home carries a hand-written workestrate.schema.json
/// and NO workload file) makes the schemas check WARN with a per-target STALE
/// entry; the human report carries the remediation. The tool-template target
/// (the real checkout, P1-synced) stays FRESH, so exactly the home copy is
/// stale.
#[test]
fn doctor_schemas_reports_stale_home_copy() {
    let home = IsolatedHome::new("cmd-doctor");
    let store = home.dir.join(".workestrate");
    std::fs::create_dir_all(store.join("schemas")).expect("create store schemas dir");
    std::fs::write(
        store.join("schemas").join("workestrate.schema.json"),
        "{\"stale\": true}\n",
    )
    .expect("write stale schema");
    // No workestrate-workload.schema.json — a missing file is stale too.

    let out = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args(["doctor", "--json"])
        .output()
        .expect("invoke doctor --json");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let doc: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor --json stdout must be valid JSON");
    let checks = doc["checks"].as_array().expect("checks must be an array");
    let schemas = checks
        .iter()
        .find(|c| c["name"] == "schemas")
        .expect("schemas check must exist");
    assert_eq!(
        schemas["status"], "WARN",
        "stale copy must make the schemas check WARN; got: {schemas}"
    );
    let message = schemas["message"]
        .as_str()
        .expect("schemas missing message");
    assert!(
        message.contains("1 stale"),
        "exactly the home copy must be stale; got: {message}"
    );
    let repos = schemas["repos"]
        .as_array()
        .expect("schemas must carry a repos array");
    let home_entry = repos
        .iter()
        .find(|r| {
            r["target"]
                .as_str()
                .map(|t| t.contains("tool home"))
                .unwrap_or(false)
        })
        .expect("tool home target must be reported");
    assert_eq!(
        home_entry["status"], "STALE",
        "the stale home copy must be STALE: {home_entry}"
    );
    let template_entry = repos
        .iter()
        .find(|r| {
            r["target"]
                .as_str()
                .map(|t| t.contains("tool template"))
                .unwrap_or(false)
        })
        .expect("tool template target must be reported");
    assert_eq!(
        template_entry["status"], "OK",
        "the P1-synced template copy must be fresh: {template_entry}"
    );

    // Human report carries the remediation for the stale copy.
    let out_h = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args(["doctor"])
        .output()
        .expect("invoke doctor");
    let stdout_h = String::from_utf8_lossy(&out_h.stdout);
    assert!(
        stdout_h.contains("Run 'workestrate schemas update'"),
        "human report must carry the schemas remediation; got:\n{stdout_h}"
    );
}

/// After `schemas update` refreshes the stale home copy, the schemas check
/// reports OK.
#[test]
fn doctor_schemas_is_ok_after_schemas_update() {
    let home = IsolatedHome::new("cmd-doctor");
    let store = home.dir.join(".workestrate");
    std::fs::create_dir_all(store.join("schemas")).expect("create store schemas dir");
    std::fs::write(
        store.join("schemas").join("workestrate.schema.json"),
        "{\"stale\": true}\n",
    )
    .expect("write stale schema");

    let up = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args(["schemas", "update"])
        .output()
        .expect("invoke schemas update");
    assert!(
        up.status.success(),
        "schemas update failed: stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&up.stdout),
        String::from_utf8_lossy(&up.stderr)
    );

    let out = home
        .cmd()
        .env("WORKESTRATE_HOME", &store)
        .args(["doctor", "--json"])
        .output()
        .expect("invoke doctor --json");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let doc: serde_json::Value =
        serde_json::from_str(&stdout).expect("doctor --json stdout must be valid JSON");
    let checks = doc["checks"].as_array().expect("checks must be an array");
    let schemas = checks
        .iter()
        .find(|c| c["name"] == "schemas")
        .expect("schemas check must exist");
    assert_eq!(
        schemas["status"], "OK",
        "fresh copies must make the schemas check OK; got: {schemas}"
    );
}
