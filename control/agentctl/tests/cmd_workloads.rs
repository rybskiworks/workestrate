//! Integration tests for `workestrate workloads` (ADR 0027) — the discovery
//! verb that lists configured workloads with kind, image summary, and running
//! status. Uses an isolated HOME + XDG per test plus the committed
//! 5-workload fixture (`tests/fixtures/config/workestrate.toml`) via
//! `WORKESTRATE_CONFIG_DIR`; the isolated state dir is empty, so every
//! workload reports "(none running)".

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::IsolatedHome;
use std::path::PathBuf;

fn fixture_config_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("config")
}

/// Text output lists every fixture workload with its kind, and reports
/// "(none running)" against the empty isolated state dir.
#[test]
fn workloads_lists_fixture_workloads_with_kinds() {
    let home = IsolatedHome::new("cmd-workloads");
    let out = home
        .cmd()
        .env("WORKESTRATE_CONFIG_DIR", fixture_config_dir())
        .args(["workloads"])
        .output()
        .expect("invoke workloads");
    assert!(
        out.status.success(),
        "workloads failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("utf8 stdout");

    for (name, kind) in [
        ("example-litellm", "service"),
        ("odysseus", "service"),
        ("opencode", "agent"),
        ("pi", "agent"),
        ("tempest", "agent"),
    ] {
        let line = stdout.lines().find(|l| l.split('\t').next() == Some(name));
        assert!(
            line.is_some(),
            "workloads output must list '{name}'; got:\n{stdout}"
        );
        let line = line.unwrap();
        assert!(
            line.split('\t').nth(1) == Some(kind),
            "workload '{name}' must report kind '{kind}'; got line: {line}"
        );
        assert!(
            line.contains("(none running)"),
            "empty state dir must report '(none running)' for '{name}'; got line: {line}"
        );
    }
}

/// `--json` emits a parseable array of {name, kind, image, instances},
/// sorted by name, with empty instance lists against the empty state dir.
#[test]
fn workloads_json_parses_and_is_sorted() {
    let home = IsolatedHome::new("cmd-workloads");
    let out = home
        .cmd()
        .env("WORKESTRATE_CONFIG_DIR", fixture_config_dir())
        .args(["workloads", "--json"])
        .output()
        .expect("invoke workloads --json");
    assert!(
        out.status.success(),
        "workloads --json failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("utf8 stdout");
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("workloads --json must emit valid JSON");
    let rows = parsed.as_array().expect("top level must be an array");
    assert_eq!(rows.len(), 5, "fixture config defines 5 workloads");

    let names: Vec<&str> = rows
        .iter()
        .map(|r| r["name"].as_str().expect("name must be a string"))
        .collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    assert_eq!(names, sorted, "rows must be sorted by name; got: {names:?}");

    for row in rows {
        for field in ["name", "kind", "image"] {
            assert!(
                row[field].is_string(),
                "row must carry a string '{field}' field; got: {row}"
            );
        }
        assert!(
            row["instances"]
                .as_array()
                .expect("instances array")
                .is_empty(),
            "empty state dir must yield no running instances; got: {row}"
        );
    }
    assert!(
        rows.iter()
            .any(|r| r["name"] == "example-litellm" && r["kind"] == "service"),
        "example-litellm must be listed as a service; got: {rows:?}"
    );
    assert!(
        rows.iter()
            .any(|r| r["name"] == "pi" && r["kind"] == "agent"),
        "pi must be listed as an agent; got: {rows:?}"
    );
}

/// A seeded port-registry record surfaces as a running instance.
#[test]
fn workloads_reports_running_instances_from_registry() {
    let home = IsolatedHome::new("cmd-workloads");
    let run_dir = home.state_dir().join("var").join("run");
    std::fs::create_dir_all(&run_dir).expect("create var/run");
    std::fs::write(
        run_dir.join("pi@xy7.json"),
        r#"{"instance":"pi@xy7","context":null,"workload":"pi","ports":[],"port_pairs":[],"created_at":"","bind_ip":"127.0.0.2"}"#,
    )
    .expect("seed instance record");

    let out = home
        .cmd()
        .env("WORKESTRATE_CONFIG_DIR", fixture_config_dir())
        .args(["workloads"])
        .output()
        .expect("invoke workloads");
    assert!(
        out.status.success(),
        "workloads failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).expect("utf8 stdout");
    let pi_line = stdout
        .lines()
        .find(|l| l.split('\t').next() == Some("pi"))
        .expect("pi row must exist");
    assert!(
        pi_line.contains("running: pi@xy7"),
        "seeded record must surface as running; got line: {pi_line}"
    );
    let example_litellm_line = stdout
        .lines()
        .find(|l| l.split('\t').next() == Some("example-litellm"))
        .expect("example-litellm row must exist");
    assert!(
        example_litellm_line.contains("(none running)"),
        "example-litellm has no record; got line: {example_litellm_line}"
    );
}
