//! Integration tests for the spec 22 mount-policy diagnostics CLI.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::{IsolatedHome, TempDir};
use serde_json::Value;
use std::path::PathBuf;

const WORKESTRATE_TOML: &str = r#"
schema_version = 1

[policy.mounts]
mask = [{ pattern = ".env", overridable = false }, "docs/secrets/**"]
unmask = ["docs/secrets/README.md"]
protect = [".workestrate/"]

[policy.mounts.writes]
allow = ["docs/secrets/generated/**"]
deny = ["docs/secrets/**"]

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "alpine:latest" }
command = ["true"]

[[workloads.svc.mounts]]
host = "."
guest = "/work"
read_only = false

[workloads.svc.network]
default_deny = true
"#;

const NO_POLICY_TOML: &str = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "alpine:latest" }
command = ["true"]

[[workloads.svc.mounts]]
host = "."
guest = "/work"
read_only = false
"#;

fn fixture_with(config: &str) -> (TempDir, PathBuf) {
    let tmp = TempDir::new("cmd-policy-fixture");
    let dir = tmp.path().to_path_buf();
    std::fs::write(dir.join("workestrate.toml"), config).unwrap();
    std::fs::write(dir.join(".env"), "SECRET=1").unwrap();
    std::fs::create_dir_all(dir.join("docs/secrets/generated")).unwrap();
    std::fs::write(dir.join("docs/secrets/README.md"), "safe").unwrap();
    std::fs::hard_link(
        dir.join("docs/secrets/README.md"),
        dir.join("docs/secrets/key.pem"),
    )
    .unwrap();
    std::fs::write(dir.join("docs/secrets/generated/out.txt"), "out").unwrap();
    std::fs::create_dir_all(dir.join(".workestrate")).unwrap();
    std::fs::write(dir.join(".workestrate/secret"), "protected").unwrap();
    std::fs::create_dir_all(dir.join("data/prod/locked")).unwrap();
    std::fs::write(dir.join("data/prod/locked/secret"), "locked").unwrap();
    (tmp, dir)
}

fn fixture() -> (TempDir, PathBuf) {
    fixture_with(WORKESTRATE_TOML)
}

fn explain(config_dir: &PathBuf, path: &str, json: bool) -> std::process::Output {
    let home = IsolatedHome::new("cmd-policy");
    let mut cmd = home.cmd();
    cmd.env("WORKESTRATE_CONFIG_DIR", config_dir).args([
        "policy",
        "mounts",
        "explain",
        "--workload",
        "svc",
        "--mount",
        "/work",
        "--path",
        path,
    ]);
    if json {
        cmd.arg("--json");
    }
    cmd.output().expect("invoke policy mounts explain")
}

fn preview(config_dir: &PathBuf, json: bool) -> std::process::Output {
    let home = IsolatedHome::new("cmd-policy");
    let mut cmd = home.cmd();
    cmd.env("WORKESTRATE_CONFIG_DIR", config_dir)
        .args([
            "policy",
            "mounts",
            "preview",
            "--workload",
            "svc",
            "--mount",
            "/work",
            "--root",
        ])
        .arg(config_dir);
    if json {
        cmd.arg("--json");
    }
    cmd.output().expect("invoke policy mounts preview")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn explain_text_masked_paths_and_write_decisions() {
    let (_tmp, dir) = fixture();
    let out = explain(&dir, ".env", false);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("decision: Masked"));
    assert!(stdout(&out).contains("write: Allow"));

    let out = explain(&dir, "docs/secrets/key.pem", false);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("decision: Masked"));
    assert!(stdout(&out).contains("write: Deny"));
}

#[test]
fn explain_text_visible_traversal_and_protected_paths() {
    let (_tmp, dir) = fixture();
    let out = explain(&dir, "docs/secrets/README.md", false);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("decision: Visible"));
    assert!(stdout(&out).contains("write: Deny"));

    let out = explain(&dir, "docs/secrets", false);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("decision: TraversalOnly"));

    let out = explain(&dir, ".workestrate/secret", false);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("decision: Protected"));
}

#[test]
fn explain_json_contains_matches_parents_and_frozen_origin() {
    let (_tmp, dir) = fixture();
    let out = explain(&dir, ".env", true);
    assert!(out.status.success(), "{}", stderr(&out));
    let doc: Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(doc["decision"], "masked");
    assert_eq!(doc["write"], "allow");
    assert!(doc["matches"].as_array().unwrap().iter().all(|m| {
        m["pattern"].is_string()
            && m["effect"].is_string()
            && m["terminal"].is_boolean()
            && m["frozen_out"].is_boolean()
            && m["origin"]["layer"].is_string()
            && m["origin"]["file"].is_string()
            && m["origin"]["scope_kind"].is_string()
    }));
    assert!(!doc["matches"].as_array().unwrap().is_empty());
    assert!(doc["parents"].is_array());
    assert!(doc["frozen_by"]["layer"].is_string());
    assert!(doc["frozen_by"]["file"].is_string());
    assert!(doc["frozen_by"]["scope_kind"].is_string());
}

#[test]
fn preview_text_shows_decisions_pruning_and_hardlink_alias() {
    let (_tmp, dir) = fixture();
    let out = preview(&dir, false);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains(".env [masked]"));
    assert!(text.contains("docs/secrets/README.md [visible]"));
    assert!(text.contains("docs/secrets [traversal_only]"));
    assert!(text.contains(".workestrate [protected]"));
    assert!(text.contains("(pruned)"));
    assert!(text.contains("hardlink alias:"));
}

#[test]
fn preview_json_contains_tree_and_hardlink_warning() {
    let (_tmp, dir) = fixture();
    let out = preview(&dir, true);
    assert!(out.status.success(), "{}", stderr(&out));
    let doc: Value = serde_json::from_str(&stdout(&out)).unwrap();
    let tree = doc["tree"].as_array().unwrap();
    assert!(tree.iter().any(|x| x["decision"] == "masked"));
    assert!(tree.iter().any(|x| x["decision"] == "visible"));
    assert!(tree.iter().any(|x| x["decision"] == "protected"));
    assert!(tree.iter().all(|x| {
        x["path"].is_string()
            && x["decision"].is_string()
            && x["write"].is_string()
            && x["protected"].is_boolean()
            && x["kind"].is_string()
    }));
    let warnings = doc["hardlink_warnings"].as_array().unwrap();
    assert!(!warnings.is_empty());
    assert!(warnings.iter().any(|w| {
        w["masked"] == "docs/secrets/key.pem"
            && w["visible"] == "docs/secrets/README.md"
            && w["dev"].is_number()
            && w["ino"].is_number()
    }));
}

#[test]
fn policy_mounts_errors_return_one() {
    let (_tmp, dir) = fixture();
    for (args, message) in [
        (
            vec!["nonexistent", "/work", ".env"],
            "workload 'nonexistent' not found",
        ),
        (
            vec!["svc", "/nonexistent", ".env"],
            "no mount with guest path",
        ),
        (vec!["svc", "/work", "/absolute"], "absolute"),
    ] {
        let home = IsolatedHome::new("cmd-policy-error");
        let out = home
            .cmd()
            .env("WORKESTRATE_CONFIG_DIR", &dir)
            .args([
                "policy",
                "mounts",
                "explain",
                "--workload",
                args[0],
                "--mount",
                args[1],
                "--path",
                args[2],
            ])
            .output()
            .expect("invoke policy error case");
        assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
        assert!(stderr(&out).contains(message), "{}", stderr(&out));
    }

    let (_tmp, no_policy) = fixture_with(NO_POLICY_TOML);
    let home = IsolatedHome::new("cmd-policy-no-policy");
    let out = home
        .cmd()
        .env("WORKESTRATE_CONFIG_DIR", &no_policy)
        .args([
            "policy",
            "mounts",
            "explain",
            "--workload",
            "svc",
            "--mount",
            "/work",
            "--path",
            ".env",
        ])
        .output()
        .expect("invoke no-policy error case");
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("has no compiled mount policy"));
}
