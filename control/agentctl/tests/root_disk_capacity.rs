//! Create-time managed capacity is distinct from an existing disk's state.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

use workestrate::config::{ConfigFile, WorkloadConfig, validate_config};
use workestrate::merge::{Layer, merge_layers};

mod common;

fn config(size: Option<u32>) -> ConfigFile {
    let mut config: ConfigFile = toml::from_str(
        "schema_version = 1\n[workloads.example]\nkind = \"service\"\nimage = { recipe = \"registry\", ref = \"alpine:latest\" }\ncommand = [\"/bin/example\"]",
    ).unwrap();
    config.workloads.get_mut("example").unwrap().root_disk_mib = size;
    config
}

#[test]
fn root_disk_omission_preserves_legacy_workload_serialization() {
    let config = config(None);
    validate_config(&config).unwrap();
    let value = serde_json::to_value(&config.workloads["example"]).unwrap();
    assert!(value.get("root_disk_mib").is_none());
    let decoded: WorkloadConfig = serde_json::from_value(value).unwrap();
    assert_eq!(decoded.root_disk_mib, None);
}

#[test]
fn root_disk_capacity_validates_nonzero_sdk_representation() {
    assert!(
        validate_config(&config(Some(0)))
            .unwrap_err()
            .to_string()
            .contains("root_disk_mib")
    );
    for value in [1, 4096, 16384, u32::MAX] {
        validate_config(&config(Some(value))).unwrap();
    }
    // Representation acceptance is not a free-space or filesystem-format guarantee.
    for value in [
        "-1",
        "4294967296",
        "9223372036854775807",
        "1.5",
        "true",
        "\"8GiB\"",
        "{}",
        "[]",
    ] {
        let text = format!("root_disk_mib = {value}");
        assert!(toml::from_str::<WorkloadConfig>(&text).is_err(), "{value}");
        let json = format!("{{\"root_disk_mib\":{value}}}");
        assert!(
            serde_json::from_str::<WorkloadConfig>(&json).is_err(),
            "{value}"
        );
    }
}

#[test]
fn root_disk_layers_inherit_or_replace_one_scalar_with_provenance() {
    let base = || Layer::from_string("base", "[workloads.example]\nroot_disk_mib = 8192").unwrap();
    let other = Layer::from_string("other", "[workloads.example]\nmemory_mib = 2048").unwrap();
    let (inherited, source) = merge_layers(&[base(), other]).unwrap();
    assert_eq!(inherited.workloads["example"].root_disk_mib, Some(8192));
    assert_eq!(source["workloads.example.root_disk_mib"], "base");
    let higher =
        Layer::from_string("higher", "[workloads.example]\nroot_disk_mib = 16384").unwrap();
    let (replaced, source) = merge_layers(&[base(), higher]).unwrap();
    assert_eq!(replaced.workloads["example"].root_disk_mib, Some(16384));
    assert_eq!(source["workloads.example.root_disk_mib"], "higher");
}

#[test]
fn root_disk_capacity_is_visible_in_json_text_and_source_plans() {
    let root = common::TempDir::new("root-disk-plan");
    let config_dir = root.path().join("config");
    std::fs::create_dir(&config_dir).unwrap();
    std::fs::write(
        config_dir.join("workestrate.toml"),
        toml::to_string(&config(Some(16384))).unwrap(),
    )
    .unwrap();
    let backend_config = root.path().join("msb.json");
    std::fs::write(&backend_config, "{}\n").unwrap();
    for flag in [Some("--json"), Some("--show-source"), None] {
        let mut command = std::process::Command::new(common::BIN);
        command
            .env_clear()
            .env("HOME", root.path().join("home"))
            .env("WORKESTRATE_HOME", root.path().join("tool"))
            .env("WORKESTRATE_CONFIG_DIR", &config_dir)
            .env("WORKESTRATE_STATE_DIR", root.path().join("state"))
            .env("MSB_HOME", root.path().join("msb"))
            .env("MSB_CONFIG_PATH", &backend_config)
            .current_dir(root.path())
            .stdin(std::process::Stdio::null())
            .args(["workload", "plan", "example"]);
        if let Some(flag) = flag {
            command.arg(flag);
        }
        let output = command.output().unwrap();
        assert!(
            output.status.success(),
            "{flag:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if flag == Some("--json") {
            let plan: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(plan["root_disk_mib"], 16384);
            assert_eq!(plan["command"], serde_json::json!(["/bin/example"]));
        } else {
            let text = String::from_utf8(output.stdout).unwrap();
            let line = text
                .lines()
                .find(|line| line.starts_with("root disk:"))
                .unwrap();
            assert!(line.contains("16384 MiB (new managed OCI disk)"));
            if flag.is_some() {
                assert!(line.contains("[local]"));
            }
        }
    }
}

#[test]
fn root_disk_editor_schemas_match_the_representable_range() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (name, pointer) in [
        (
            "workestrate.schema.json",
            "/definitions/WorkloadConfig/properties/root_disk_mib",
        ),
        (
            "workestrate-workload.schema.json",
            "/properties/root_disk_mib",
        ),
    ] {
        let source = std::fs::read(root.join("schemas").join(name)).unwrap();
        let schema: serde_json::Value = serde_json::from_slice(&source).unwrap();
        let field = schema.pointer(pointer).unwrap();
        assert_eq!(field["minimum"], 1.0);
        assert_eq!(field["maximum"], f64::from(u32::MAX));
        assert_eq!(field["format"], "uint32");
        assert_eq!(
            source,
            std::fs::read(root.join("templates/workestrate-config/schemas").join(name)).unwrap()
        );
    }
}
