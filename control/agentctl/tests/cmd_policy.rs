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

[policy.mounts.read]
deny = [{ pattern = ".env", final = true }, "docs/secrets/**"]
allow = ["docs/secrets/README.md"]

[policy.mounts.write]
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

[workloads.svc.network.defaults]
egress = "deny"
"#;

/// Operator-scope policy for the main fixture. The old fixture's
/// `protect = [".workestrate/"]` carried protect-bucket ROUTING intent;
/// under the unified surface the protect wire bucket is reached ONLY by an
/// operator scope's final read.deny (spec 22 §5), so the entry moves to the
/// isolated home registry (HomeRegistry scope) and the workload config
/// loads as a config-repo layer through the real registry chain (the
/// WORKESTRATE_CONFIG_DIR bypass collects no operator scope).
const OPERATOR_REGISTRY_TOML: &str = r#"
layers = ["cmd-policy"]

[policy.mounts.read]
deny = [{ pattern = ".workestrate/", final = true }]

[configs.cmd-policy]
url = "file:///unused/cmd-policy"
ref = "main"
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

const TWO_MOUNT_POLICY_TOML: &str = r#"
schema_version = 1

[policy.mounts.read]
deny = ["shared-secret"]

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "alpine:latest" }
command = ["true"]

[[workloads.svc.mounts]]
host = "."
guest = "/workspace"
read_only = false
policy = { read.deny = ["node_modules/"] }

[[workloads.svc.mounts]]
host = "."
guest = "/data"
read_only = false
policy = { read.deny = ["secrets/"] }

[workloads.svc.network.defaults]
egress = "deny"
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

fn two_mount_fixture() -> (TempDir, PathBuf) {
    let tmp = TempDir::new("cmd-policy-two-mount-fixture");
    let dir = tmp.path().to_path_buf();
    std::fs::write(dir.join("workestrate.toml"), TWO_MOUNT_POLICY_TOML).unwrap();
    std::fs::write(dir.join("shared-secret"), "shared").unwrap();
    std::fs::create_dir_all(dir.join("node_modules/deps")).unwrap();
    std::fs::write(dir.join("node_modules/deps/lib.js"), "deps").unwrap();
    std::fs::create_dir_all(dir.join("secrets")).unwrap();
    std::fs::write(dir.join("secrets/api.key"), "api").unwrap();
    (tmp, dir)
}

/// Isolated home whose registry registers the fixture's workestrate.toml as
/// the `cmd-policy` config-repo layer and declares the operator-scope
/// protect policy (OPERATOR_REGISTRY_TOML). The main fixture exercises
/// protect routing, so it must load through the real registry chain.
fn operator_home(fixture_dir: &std::path::Path) -> IsolatedHome {
    let home = IsolatedHome::new("cmd-policy");
    home.write_registry(OPERATOR_REGISTRY_TOML);
    let repo = home.create_repo_dir("cmd-policy");
    std::fs::copy(
        fixture_dir.join("workestrate.toml"),
        repo.join("workestrate.toml"),
    )
    .expect("copy fixture config into the config-repo layer");
    home
}

/// Command against the registry flow: no CONFIG_DIR bypass, no project
/// layer, and no reference base layer (the old bypass-mode fixtures never
/// saw the reference layer either — keep the fixture hermetic).
fn registry_cmd(home: &IsolatedHome) -> std::process::Command {
    let mut cmd = home.cmd();
    cmd.env_remove("WORKESTRATE_REFERENCE_CONFIG");
    cmd.env("WORKESTRATE_NO_PROJECT_CONFIG", "1");
    cmd
}

fn explain(config_dir: &std::path::Path, path: &str, json: bool) -> std::process::Output {
    let home = operator_home(config_dir);
    let mut cmd = registry_cmd(&home);
    cmd.args([
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

fn explain_mount(
    config_dir: &PathBuf,
    mount: &str,
    path: &str,
    json: bool,
) -> std::process::Output {
    let home = IsolatedHome::new("cmd-policy");
    let mut cmd = home.cmd();
    cmd.env("WORKESTRATE_CONFIG_DIR", config_dir).args([
        "policy",
        "mounts",
        "explain",
        "--workload",
        "svc",
        "--mount",
        mount,
        "--path",
        path,
    ]);
    if json {
        cmd.arg("--json");
    }
    cmd.output().expect("invoke policy mounts explain")
}

fn preview(config_dir: &std::path::Path, json: bool) -> std::process::Output {
    let home = operator_home(config_dir);
    let mut cmd = registry_cmd(&home);
    cmd.args([
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

fn preview_mount(config_dir: &PathBuf, mount: &str, json: bool) -> std::process::Output {
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
            mount,
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

#[test]
fn explain_per_mount_picks_the_right_program() {
    let (_tmp, dir) = two_mount_fixture();
    for (mount, path, decision) in [
        ("/workspace", "node_modules/deps/lib.js", "Masked"),
        ("/workspace", "secrets/api.key", "Visible"),
        ("/data", "node_modules/deps/lib.js", "Visible"),
        ("/data", "secrets/api.key", "Masked"),
        ("/workspace", "shared-secret", "Masked"),
        ("/data", "shared-secret", "Masked"),
    ] {
        let out = explain_mount(&dir, mount, path, false);
        assert!(out.status.success(), "{}", stderr(&out));
        assert!(
            stdout(&out).contains(&format!("decision: {decision}")),
            "{}",
            stdout(&out)
        );
    }
}

#[test]
fn explain_per_mount_no_cross_mount_rules_in_matches() {
    let (_tmp, dir) = two_mount_fixture();
    let out = explain_mount(&dir, "/workspace", "secrets/api.key", true);
    assert!(out.status.success(), "{}", stderr(&out));
    let doc: Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(doc["decision"], "visible");
    let matches = doc["matches"].as_array().unwrap();
    assert!(!matches.iter().any(|m| m["pattern"] == "secrets/"));

    let out = explain_mount(&dir, "/workspace", "shared-secret", true);
    assert!(out.status.success(), "{}", stderr(&out));
    let doc: Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(doc["decision"], "masked");
    assert!(doc["matches"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["pattern"] == "shared-secret"));

    let out = explain_mount(&dir, "/data", "node_modules/deps/lib.js", true);
    assert!(out.status.success(), "{}", stderr(&out));
    let doc: Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(doc["decision"], "visible");
    assert!(!doc["matches"]
        .as_array()
        .unwrap()
        .iter()
        .any(|m| m["pattern"] == "node_modules/"));
}

#[test]
fn preview_per_mount_evaluates_the_right_program() {
    let (_tmp, dir) = two_mount_fixture();
    let out = preview_mount(&dir, "/workspace", false);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("node_modules [masked]"));
    assert!(!text.contains("secrets [masked]"));
    assert!(text.contains("shared-secret [masked]"));

    let out = preview_mount(&dir, "/data", false);
    assert!(out.status.success(), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("secrets [masked]"));
    assert!(!text.contains("node_modules [masked]"));
    assert!(text.contains("shared-secret [masked]"));
}

#[test]
fn reference_config_ships_sensitive_mount_defaults() {
    let config_reference_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("config.reference");
    let home = IsolatedHome::new("cmd-policy-reference-defaults");
    let out = home
        .cmd()
        .env("WORKESTRATE_CONFIG_DIR", &config_reference_dir)
        .args([
            "policy",
            "mounts",
            "explain",
            "--workload",
            "example-service",
            "--mount",
            "/data",
            "--path",
            ".env",
            "--json",
        ])
        .output()
        .expect("invoke reference policy mounts explain");
    assert!(out.status.success(), "{}", stderr(&out));
    let doc: Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(doc["decision"], "masked");
    let matches = doc["matches"].as_array().unwrap();
    assert!(!matches.is_empty());
    assert!(matches.iter().any(|m| m["pattern"] == "**/.env"));
}

// ---- Unified mount-policy surface (S4) ----

/// Mount-row sugar fixture (spec 22 §8.3): `read.deny` / `write.deny`
/// dotted keys directly on the `[[mounts]]` row, parse-time normalized into
/// the row's policy fragment.
const SUGAR_TOML: &str = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "alpine:latest" }
command = ["true"]

[[workloads.svc.mounts]]
host = "."
guest = "/work"
mode = "rw"
read.deny = ["scratch/**"]
write.deny = [".env", "*.key"]

[workloads.svc.network.defaults]
egress = "deny"
"#;

/// Workload-scope policy with a FINAL ALLOW: rejected by the compile-time
/// trust gate (spec 22 §5) on whichever axis declares it.
fn final_allow_toml(axis: &str) -> String {
    format!(
        r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = {{ recipe = "registry", ref = "alpine:latest" }}
command = ["true"]

[[workloads.svc.mounts]]
host = "."
guest = "/work"
mode = "rw"

[workloads.svc.policy.mounts.{axis}]
allow = [{{ pattern = "generated/**", final = true }}]

[workloads.svc.network.defaults]
egress = "deny"
"#
    )
}

/// Workload-scope FINAL DENY: the fail-closed direction, accepted from any
/// scope (spec 22 §5).
const FINAL_READ_DENY_TOML: &str = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "alpine:latest" }
command = ["true"]

[[workloads.svc.mounts]]
host = "."
guest = "/work"
mode = "rw"

[workloads.svc.policy.mounts.read]
deny = [{ pattern = ".env", final = true }]

[workloads.svc.network.defaults]
egress = "deny"
"#;

/// Old-surface vocabulary (`mask`) must be a hard unknown-field error.
const OLD_VOCAB_TOML: &str = r#"
schema_version = 1

[policy.mounts]
mask = ["secrets/**"]

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "alpine:latest" }
command = ["true"]

[workloads.svc.network.defaults]
egress = "deny"
"#;

#[test]
fn mount_row_sugar_compiles_and_surfaces_in_explain_and_preview() {
    let tmp = TempDir::new("cmd-policy-sugar-fixture");
    let dir = tmp.path().to_path_buf();
    std::fs::write(dir.join("workestrate.toml"), SUGAR_TOML).unwrap();
    std::fs::create_dir_all(dir.join("scratch")).unwrap();
    std::fs::write(dir.join("scratch/tmp.txt"), "tmp").unwrap();
    std::fs::write(dir.join(".env"), "SECRET=1").unwrap();
    std::fs::write(dir.join("visible.txt"), "v").unwrap();

    // read.deny sugar: scratch/** is masked.
    let out = explain_mount(&dir, "/work", "scratch/tmp.txt", false);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("decision: Masked"));

    // write.deny sugar: .env stays visible but is write-denied; unmatched
    // paths keep the default write-allow.
    let out = explain_mount(&dir, "/work", ".env", false);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("decision: Visible"));
    assert!(stdout(&out).contains("write: Deny"));

    let out = explain_mount(&dir, "/work", "visible.txt", false);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("decision: Visible"));
    assert!(stdout(&out).contains("write: Allow"));

    // preview reflects the sugar-derived program too.
    let out = preview_mount(&dir, "/work", false);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(stdout(&out).contains("scratch [masked]"));
}

#[test]
fn workload_scope_final_allow_fails_the_trust_gate_on_both_axes() {
    for (axis, surface) in [("read", "final read.allow"), ("write", "final write.allow")] {
        let (_tmp, dir) = fixture_with(&final_allow_toml(axis));
        let out = explain_mount(&dir, "/work", "generated/out.txt", false);
        assert_eq!(
            out.status.code(),
            Some(1),
            "workload-scope {surface} must fail: {}",
            stdout(&out)
        );
        let err = stderr(&out);
        assert!(
            err.contains(surface),
            "error must name the axis surface key ({surface}): {err}"
        );
        assert!(
            err.contains("generated/**"),
            "error must name the offending pattern: {err}"
        );
        assert!(
            err.contains("workload scope"),
            "error must name the declaring origin: {err}"
        );
    }
}

#[test]
fn workload_scope_final_read_deny_is_accepted() {
    let (_tmp, dir) = fixture_with(FINAL_READ_DENY_TOML);
    let out = explain_mount(&dir, "/work", ".env", true);
    assert!(
        out.status.success(),
        "workload-scope final read.deny must compile: {}",
        stderr(&out)
    );
    let doc: Value = serde_json::from_str(&stdout(&out)).unwrap();
    assert_eq!(doc["decision"], "masked");
    // The final deny freezes the decision at the workload scope.
    assert_eq!(doc["frozen_by"]["scope_kind"], "workload");
}

#[test]
fn old_vocabulary_is_a_hard_unknown_field_error_end_to_end() {
    let (_tmp, dir) = fixture_with(OLD_VOCAB_TOML);
    let home = IsolatedHome::new("cmd-policy-old-vocab");
    let out = home
        .cmd()
        .env("WORKESTRATE_CONFIG_DIR", &dir)
        .args(["validate-config"])
        .output()
        .expect("invoke validate-config");
    assert!(
        !out.status.success(),
        "old-surface `mask` must be rejected: {}",
        stdout(&out)
    );
    let err = stderr(&out);
    assert!(
        err.contains("unknown field"),
        "error must be the unknown-field rejection: {err}"
    );
    assert!(
        err.contains("mask"),
        "error must name the removed key: {err}"
    );
}
