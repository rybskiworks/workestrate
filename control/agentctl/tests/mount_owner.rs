//! Mount ownership through capsule loading and the real plan CLI; no VM required.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{BIN, IsolatedHome};
use std::process::Command;

#[test]
fn capsule_mount_owner_reaches_json_and_text_plans_without_provisioning() {
    for declaration in [
        "",
        "owner = { uid = 0, gid = 0 }",
        "owner = { uid = 61040, gid = 61040 }",
    ] {
        let home = IsolatedHome::new("mount-owner");
        let fleet = home.dir.join("fleet");
        let tool_home = home.dir.join("tool");
        let mount_source = tool_home.join("state/workspaces/service-state");
        let capsule = fleet.join("workestrate/workloads/service");
        std::fs::create_dir_all(&capsule).unwrap();
        // Read-only sources must already exist; planning must leave it empty.
        std::fs::create_dir_all(&mount_source).unwrap();
        // Directory-mode capsules load through a registered config/context;
        // WORKESTRATE_CONFIG_DIR accepts only a single workestrate.toml layer.
        std::fs::write(
            tool_home.join("config.toml"),
            format!(
                r#"
[settings]
default_context = "selected"
[configs.local]
url = "{}"
[contexts.selected]
layers = ["local"]
"#,
                fleet.display()
            ),
        )
        .unwrap();
        std::fs::write(
            fleet.join("workestrate/default.toml"),
            "schema_version = 1\n",
        )
        .unwrap();
        std::fs::write(
            capsule.join("workload.toml"),
            format!(
                r#"
kind = "service"
image = {{ recipe = "registry", ref = "example.invalid/service:latest" }}
command = ["/bin/true"]
[[mounts]]
host = "workspaces/service-state"
guest = "/data"
mode = "ro"
{declaration}
read.deny = ["private/**"]
"#
            ),
        )
        .unwrap();
        let command = || {
            let mut cmd = Command::new(BIN);
            cmd.env_clear()
                .env("HOME", &home.dir)
                .env("WORKESTRATE_HOME", &tool_home)
                .env("MSB_HOME", home.dir.join("msb"))
                .env("PATH", "/nonexistent")
                .current_dir(&home.dir)
                .stdin(std::process::Stdio::null());
            cmd
        };
        let result = command()
            .args(["--json", "workload", "plan", "service"])
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let plan: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
        let mount = &plan["mounts"][0];
        assert_eq!(mount["host"], "workspaces/service-state");
        assert_eq!(mount["guest"], "/data");
        assert_eq!(mount["mode"], "ro");
        assert!(mount["policy"].is_object());
        let text = command()
            .args(["workload", "plan", "service"])
            .output()
            .unwrap();
        assert!(
            text.status.success(),
            "{}",
            String::from_utf8_lossy(&text.stderr)
        );
        let text = String::from_utf8(text.stdout).unwrap();
        if declaration.is_empty() {
            assert!(mount.get("owner").is_none());
            assert!(!text.contains("  owner:"));
        } else {
            let id = if declaration.contains("61040") {
                61040
            } else {
                0
            };
            assert_eq!(mount["owner"], serde_json::json!({"uid": id, "gid": id}));
            assert!(text.contains(&format!("  owner: uid={id} gid={id}")));
        }
        assert_eq!(std::fs::read_dir(&mount_source).unwrap().count(), 0);
        assert!(!home.dir.join("msb").exists());
    }
}
