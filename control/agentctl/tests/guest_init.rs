//! Explicit PID 1 configuration remains separate from workload execution.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

use std::collections::BTreeMap;
use workestrate::config::{ConfigFile, InitConfig, validate_config};
use workestrate::merge::{Layer, merge_layers};

mod common;

fn parse_init(fragment: &str) -> ConfigFile {
    toml::from_str(&format!(
        "schema_version = 1\n[workloads.example]\nkind = \"service\"\nimage = {{ recipe = \"registry\", ref = \"example:latest\" }}\ncommand = [\"/bin/example\"]\n{fragment}"
    ))
    .expect("fixture config")
}

#[test]
fn init_omission_preserves_legacy_serialization() {
    let config = parse_init("");
    validate_config(&config).expect("legacy config");
    assert_eq!(config.workloads["example"].init, None);
    let serialized = serde_json::to_value(&config.workloads["example"]).unwrap();
    assert!(serialized.get("init").is_none());
}

#[test]
fn init_handoff_preserves_literal_arguments_and_sorted_environment() {
    let config = parse_init(
        r#"[workloads.example.init]
mode = "handoff"
cmd = "/init"
args = ["a b", "", "${LITERAL}", "$(not-a-shell)"]
env = { Z = "last", A = "${UNEXPANDED}" }
"#,
    );
    validate_config(&config).expect("valid guest literals");
    let init = config.workloads["example"].init.as_ref().unwrap();
    assert_eq!(
        init,
        &InitConfig::Handoff {
            cmd: "/init".into(),
            args: ["a b", "", "${LITERAL}", "$(not-a-shell)"]
                .map(String::from)
                .to_vec(),
            env: BTreeMap::from([
                ("A".into(), "${UNEXPANDED}".into()),
                ("Z".into(), "last".into()),
            ]),
        }
    );
    assert_eq!(config.workloads["example"].command, ["/bin/example"]);
    let roundtrip: InitConfig = serde_json::from_str(&init.to_string()).unwrap();
    assert_eq!(&roundtrip, init);
}

#[test]
fn init_is_a_closed_tagged_contract() {
    assert!(toml::from_str::<InitConfig>("mode = \"agentd\"\ncmd = \"/init\"").is_err());
    for fragment in [
        "{}",
        r#"{"mode":"auto"}"#,
        r#"{"mode":"agentd","cmd":"/init"}"#,
        r#"{"mode":"handoff"}"#,
        r#"{"mode":"handoff","cmd":"/init","typo":true}"#,
        r#"{"mode":"handoff","cmd":"/init","env":{"SECRET":{"secret":"KEY"}}}"#,
    ] {
        assert!(
            serde_json::from_str::<InitConfig>(fragment).is_err(),
            "{fragment}"
        );
    }
}

#[test]
fn init_validation_is_guest_native_and_rejects_invalid_exec_bytes() {
    for cmd in ["", "auto", "/", "init", "C:\\init", "/a\\b", "/a\0b"] {
        let mut config = parse_init("");
        config.workloads.get_mut("example").unwrap().init = Some(InitConfig::Handoff {
            cmd: cmd.into(),
            args: vec![],
            env: BTreeMap::new(),
        });
        assert!(validate_config(&config).is_err(), "{cmd:?}");
    }
    for (args, env) in [
        (vec!["bad\0arg".into()], BTreeMap::new()),
        (
            vec![],
            BTreeMap::from([("BAD=NAME".into(), "value".into())]),
        ),
        (vec![], BTreeMap::from([("".into(), "value".into())])),
        (
            vec![],
            BTreeMap::from([("GOOD".into(), "bad\0value".into())]),
        ),
    ] {
        let mut config = parse_init("");
        config.workloads.get_mut("example").unwrap().init = Some(InitConfig::Handoff {
            cmd: "/nix/store/not-present-on-host/bin/init".into(),
            args,
            env,
        });
        assert!(validate_config(&config).is_err());
    }
    validate_config(&parse_init(
        "[workloads.example.init]\nmode = \"handoff\"\ncmd = \"/not-present-on-host/init\"",
    ))
    .expect("no host filesystem lookup");
}

#[test]
fn init_layers_replace_whole_block_and_allow_explicit_agentd_reset() {
    let base = || {
        Layer::from_string(
            "base",
            "schema_version = 1\n[workloads.example.init]\nmode = \"handoff\"\ncmd = \"/old-init\"\nargs = [\"old\"]\nenv = { OLD = \"yes\" }",
        )
        .unwrap()
    };
    let inherit = Layer::from_string("inherit", "[workloads.example]\ncpus = 2").unwrap();
    let (inherited, provenance) = merge_layers(&[base(), inherit]).unwrap();
    assert!(matches!(
        inherited.workloads["example"].init,
        Some(InitConfig::Handoff { .. })
    ));
    assert_eq!(provenance["workloads.example.init"], "base");

    let replace = Layer::from_string(
        "replace",
        "[workloads.example.init]\nmode = \"handoff\"\ncmd = \"/new-init\"",
    )
    .unwrap();
    let (replaced, provenance) = merge_layers(&[base(), replace]).unwrap();
    assert_eq!(
        replaced.workloads["example"].init,
        Some(InitConfig::Handoff {
            cmd: "/new-init".into(),
            args: vec![],
            env: BTreeMap::new(),
        })
    );
    assert_eq!(provenance["workloads.example.init"], "replace");

    let reset = Layer::from_string("reset", "[workloads.example.init]\nmode = \"agentd\"").unwrap();
    let (reset, provenance) = merge_layers(&[base(), reset]).unwrap();
    assert_eq!(reset.workloads["example"].init, Some(InitConfig::Agentd {}));
    assert_eq!(provenance["workloads.example.init"], "reset");
}

#[test]
fn init_is_visible_in_public_plans_without_replacing_workload_command() {
    let root = common::TempDir::new("guest-init-plan");
    let config_dir = root.path().join("config");
    std::fs::create_dir(&config_dir).unwrap();
    let config = parse_init(
        "[workloads.example.init]\nmode = \"handoff\"\ncmd = \"/init\"\nargs = [\"a b\", \"\"]\nenv = { A = \"${LITERAL}\" }",
    );
    std::fs::write(
        config_dir.join("workestrate.toml"),
        toml::to_string(&config).unwrap(),
    )
    .unwrap();
    let backend_config = root.path().join("msb.json");
    std::fs::write(&backend_config, "{}\n").unwrap();

    for flag in ["--json", "--show-source"] {
        let output = std::process::Command::new(common::BIN)
            .env_clear()
            .env("HOME", root.path().join("home"))
            .env("WORKESTRATE_CONFIG", root.path().join("tool"))
            .env("WORKESTRATE_FLEET_DIR", &config_dir)
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
            "{flag}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        if flag == "--json" {
            let plan: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(
                plan["init"],
                serde_json::to_value(&config.workloads["example"].init).unwrap()
            );
            assert_eq!(plan["command"], serde_json::json!(["/bin/example"]));
        } else {
            let text = String::from_utf8(output.stdout).unwrap();
            let line = text.lines().find(|line| line.starts_with("init:")).unwrap();
            assert!(line.contains("handoff") && line.contains("/init"));
            assert!(
                line.contains("[local]"),
                "init must retain its source: {line}"
            );
        }
    }
}
