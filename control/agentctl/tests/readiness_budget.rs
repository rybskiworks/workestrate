//! Service-owned startup budgets merge normally without changing omitted values.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

use workestrate::config::{ConfigFile, WorkloadConfig, validate_config};
use workestrate::merge::{Layer, merge_layers};

mod common;

fn config(kind: &str, seconds: Option<u32>) -> ConfigFile {
    let mut config: ConfigFile = toml::from_str(
        "schema_version = 1\n[workloads.example]\nkind = \"service\"\nimage = { recipe = \"registry\", ref = \"alpine:latest\" }",
    ).unwrap();
    let workload = config.workloads.get_mut("example").unwrap();
    workload.kind = kind.to_string();
    workload.readiness_timeout_secs = seconds;
    config
}

#[test]
fn readiness_omission_preserves_legacy_serialization() {
    let config = config("service", None);
    validate_config(&config).unwrap();
    let value = serde_json::to_value(&config.workloads["example"]).unwrap();
    assert!(value.get("readiness_timeout_secs").is_none());
    let decoded: WorkloadConfig = serde_json::from_value(value).unwrap();
    assert_eq!(decoded.readiness_timeout_secs, None);
}

#[test]
fn readiness_budget_validates_finite_startup_range() {
    for seconds in [1, 15, 120, 3600] {
        validate_config(&config("service", Some(seconds))).unwrap();
    }
    for seconds in [0, 3601, u32::MAX] {
        let error = validate_config(&config("service", Some(seconds))).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("readiness_timeout_secs must be in 1..=3600")
        );
    }
}

#[test]
fn readiness_budget_rejects_non_integer_and_unrepresentable_values() {
    for value in ["-1", "4294967296", "1.5", "true", "\"120s\"", "{}", "[]"] {
        assert!(
            toml::from_str::<WorkloadConfig>(&format!("readiness_timeout_secs = {value}")).is_err(),
            "{value}"
        );
        assert!(
            serde_json::from_str::<WorkloadConfig>(&format!(
                "{{\"readiness_timeout_secs\":{value}}}"
            ))
            .is_err(),
            "{value}"
        );
    }
}

#[test]
fn readiness_budget_requires_effective_service_kind() {
    for kind in ["agent", "task", ""] {
        let error = validate_config(&config(kind, Some(120))).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("readiness_timeout_secs requires kind")
        );
    }
    validate_config(&config("agent", None)).unwrap();
}

#[test]
fn readiness_partial_layers_inherit_kind_and_scalar_provenance() {
    let base = || {
        Layer::from_string("base", "schema_version = 1\n[workloads.example]\nkind = \"service\"\nreadiness_timeout_secs = 30").unwrap()
    };
    let other = Layer::from_string("other", "[workloads.example]\nmemory_mib = 2048").unwrap();
    let (inherited, source) = merge_layers(&[base(), other]).unwrap();
    validate_config(&inherited).unwrap();
    assert_eq!(
        inherited.workloads["example"].readiness_timeout_secs,
        Some(30)
    );
    assert_eq!(source["workloads.example.readiness_timeout_secs"], "base");
    let higher = Layer::from_string(
        "higher",
        "[workloads.example]\nreadiness_timeout_secs = 120",
    )
    .unwrap();
    let (replaced, source) = merge_layers(&[base(), higher]).unwrap();
    validate_config(&replaced).unwrap();
    assert_eq!(replaced.workloads["example"].kind, "service");
    assert_eq!(
        replaced.workloads["example"].readiness_timeout_secs,
        Some(120)
    );
    assert_eq!(source["workloads.example.readiness_timeout_secs"], "higher");
}

#[test]
fn readiness_inherited_budget_refuses_later_non_service_kind() {
    let base = Layer::from_string(
        "base",
        "schema_version = 1\n[workloads.example]\nkind = \"service\"\nreadiness_timeout_secs = 30",
    )
    .unwrap();
    let higher = Layer::from_string("higher", "[workloads.example]\nkind = \"agent\"").unwrap();
    let (merged, _) = merge_layers(&[base, higher]).unwrap();
    assert!(
        validate_config(&merged)
            .unwrap_err()
            .to_string()
            .contains("readiness_timeout_secs requires kind")
    );
}

#[test]
fn readiness_editor_schemas_match_range_and_template() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (name, pointer) in [
        (
            "workestrate.schema.json",
            "/definitions/WorkloadConfig/properties/readiness_timeout_secs",
        ),
        (
            "workestrate-workload.schema.json",
            "/properties/readiness_timeout_secs",
        ),
    ] {
        let bytes = std::fs::read(root.join("schemas").join(name)).unwrap();
        let schema: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let field = schema.pointer(pointer).unwrap();
        assert_eq!(field["minimum"], 1.0);
        assert_eq!(field["maximum"], 3600.0);
        assert_eq!(field["format"], "uint32");
        assert_eq!(
            bytes,
            std::fs::read(root.join("templates/workestrate-config/schemas").join(name)).unwrap()
        );
    }
}

#[test]
fn readiness_source_inspection_shows_effective_budget_without_changing_vm_plan() {
    let root = common::TempDir::new("readiness-inspection");
    let config_dir = root.path().join("config");
    std::fs::create_dir(&config_dir).unwrap();
    let backend_config = root.path().join("msb.json");
    std::fs::write(&backend_config, "{}\n").unwrap();
    let inspect = |flag: &str| {
        let output = std::process::Command::new(common::BIN)
            .env_clear()
            .env("HOME", root.path().join("home"))
            .env("WORKESTRATE_HOME", root.path().join("tool"))
            .env("WORKESTRATE_CONFIG_DIR", &config_dir)
            .env("WORKESTRATE_STATE_DIR", root.path().join("state"))
            .env("MSB_HOME", root.path().join("msb"))
            .env("MSB_CONFIG_PATH", &backend_config)
            .current_dir(root.path())
            .stdin(std::process::Stdio::null())
            .args(["workload", "plan", "example", flag])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    };
    let mut plans = Vec::new();
    for (seconds, expected, source) in [(None, "15s", "[core]"), (Some(120), "120s", "[local]")] {
        std::fs::write(
            config_dir.join("workestrate.toml"),
            toml::to_string(&config("service", seconds)).unwrap(),
        )
        .unwrap();
        let text = inspect("--show-source");
        let line = text
            .lines()
            .find(|line| line.starts_with("readiness timeout:"))
            .unwrap();
        assert!(line.contains(&format!(
            "readiness timeout: {expected} (dependency/batch TCP)"
        )));
        assert!(line.ends_with(source));
        let plan: serde_json::Value = serde_json::from_str(&inspect("--json")).unwrap();
        assert!(plan.get("readiness_timeout_secs").is_none());
        plans.push(plan);
    }
    assert_eq!(plans[0], plans[1]);
    assert!(
        !root.path().join("msb").exists(),
        "plan inspection must not create a runtime home"
    );
}
