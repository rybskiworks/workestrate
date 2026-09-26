//! Bind metadata selection through parsing, layer replacement and the plan CLI.

use std::process::{Command, Stdio};
use workestrate::config::ConfigFile;
use workestrate::merge::{Layer, merge_layers};
use workestrate::microsandbox::plan::{MountPlan, MountStatVirtualization};

fn config(declaration: &str) -> String {
    format!(
        r#"
schema_version = 1
[workloads.service]
kind = "service"
image = {{ recipe = "registry", ref = "example.invalid/service:latest" }}
command = ["/bin/true"]
[[workloads.service.mounts]]
host = "workspaces/service-state"
guest = "/data"
mode = "ro"
{declaration}
"#
    )
}

#[test]
fn default_plan_bytes_and_explicit_setting_round_trip() -> anyhow::Result<()> {
    let legacy = r#"{"host":"state","guest":"/data","mode":"ro"}"#;
    let default: MountPlan = serde_json::from_str(legacy)?;
    assert_eq!(default.stat_virtualization, MountStatVirtualization::Strict);
    assert_eq!(serde_json::to_string(&default)?, legacy);
    for (value, expected) in [
        ("strict", MountStatVirtualization::Strict),
        ("off", MountStatVirtualization::Off),
    ] {
        let parsed: ConfigFile =
            toml::from_str(&config(&format!("stat_virtualization = \"{value}\"")))?;
        let mount = &parsed.workloads["service"].mounts[0];
        assert_eq!(mount.stat_virtualization, expected);
        assert!(mount.is_read_only());
        let wire = serde_json::to_value(mount)?;
        assert_eq!(wire.get("stat_virtualization").is_some(), value == "off");
        assert_eq!(serde_json::from_value::<MountPlan>(wire)?, *mount);
    }
    Ok(())
}

#[test]
fn invalid_metadata_modes_and_owner_combinations_are_rejected() -> anyhow::Result<()> {
    for value in [
        r#""relaxed""#,
        r#""OFF""#,
        r#""unknown""#,
        "true",
        "1",
        "[]",
    ] {
        let text = config(&format!("stat_virtualization = {value}"));
        assert!(toml::from_str::<ConfigFile>(&text).is_err());
        let json = format!(r#"{{"host":"state","guest":"/data","stat_virtualization":{value}}}"#);
        assert!(serde_json::from_str::<MountPlan>(&json).is_err());
    }
    for owner in ["{ uid = 0, gid = 0 }", "{ uid = 61040, gid = 61040 }"] {
        let text = config(&format!("stat_virtualization = \"off\"\nowner = {owner}"));
        let result = toml::from_str::<ConfigFile>(&text);
        let Err(error) = result else {
            anyhow::bail!("literal metadata unexpectedly accepted owner {owner}");
        };
        assert!(error.to_string().contains("cannot be combined with owner"));
        let compatible = config(&format!(
            "stat_virtualization = \"strict\"\nowner = {owner}"
        ));
        assert!(toml::from_str::<ConfigFile>(&compatible).is_ok());
    }
    let json = r#"{"host":"state","guest":"/data","mode":"ro","stat_virtualization":"off","owner":{"uid":0,"gid":0}}"#;
    assert!(serde_json::from_str::<MountPlan>(json).is_err());
    Ok(())
}

#[test]
fn literal_metadata_requires_resolved_read_only_mode() -> anyhow::Result<()> {
    for access in ["", "mode = \"rw\"", "read_only = false"] {
        let text = format!(
            "host = \"state\"\nguest = \"/data\"\nstat_virtualization = \"off\"\n{access}\n"
        );
        let Err(error) = toml::from_str::<MountPlan>(&text) else {
            anyhow::bail!("literal metadata unexpectedly accepted access {access:?}");
        };
        assert!(error.to_string().contains("requires mode = \"ro\""));
    }
    for access in [
        "mode = \"ro\"",
        "read_only = true",
        "mode = \"ro\"\nread_only = true",
    ] {
        let text = format!(
            "host = \"state\"\nguest = \"/data\"\nstat_virtualization = \"off\"\n{access}\n"
        );
        let mount: MountPlan = toml::from_str(&text)?;
        assert!(mount.is_read_only());
        assert_eq!(mount.stat_virtualization, MountStatVirtualization::Off);
    }
    for access in ["", r#", "mode":"rw""#, r#", "read_only":false"#] {
        let text =
            format!(r#"{{"host":"state","guest":"/data","stat_virtualization":"off"{access}}}"#);
        assert!(serde_json::from_str::<MountPlan>(&text).is_err());
    }
    Ok(())
}

#[test]
fn replacing_mount_array_does_not_inherit_metadata_setting() -> anyhow::Result<()> {
    for declaration in [
        "",
        "stat_virtualization = \"strict\"",
        "stat_virtualization = \"off\"",
    ] {
        let base = Layer::from_string("base", &config("stat_virtualization = \"off\""))?;
        let overlay = Layer::from_string("overlay", &config(declaration))?;
        let (merged, provenance) = merge_layers(&[base, overlay])?;
        let expected: ConfigFile = toml::from_str(&config(declaration))?;
        assert_eq!(
            merged.workloads["service"].mounts,
            expected.workloads["service"].mounts
        );
        assert_eq!(
            provenance
                .get("workloads.service.mounts")
                .map(String::as_str),
            Some("overlay")
        );
    }
    Ok(())
}

#[test]
fn capsule_metadata_setting_reaches_both_plans_without_provisioning() -> anyhow::Result<()> {
    for declaration in [
        "",
        "stat_virtualization = \"strict\"",
        "stat_virtualization = \"off\"",
    ] {
        let home = tempfile::tempdir()?;
        let fleet = home.path().join("fleet");
        let tool_home = home.path().join("tool");
        let source = tool_home.join("state/workspaces/service-state");
        let capsule = fleet.join("workestrate/workloads/service");
        std::fs::create_dir_all(&capsule)?;
        std::fs::create_dir_all(&source)?;
        std::fs::write(
            tool_home.join("config.toml"),
            format!(
                "[settings]\ndefault_fleet = \"local\"\n[fleets.local]\nurl = \"{}\"\n",
                fleet.display()
            ),
        )?;
        std::fs::write(
            fleet.join("workestrate/default.toml"),
            "schema_version = 1\n",
        )?;
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
        )?;
        let command = || {
            let mut cmd = Command::new(env!("CARGO_BIN_EXE_workestrate"));
            cmd.env_clear()
                .env("HOME", home.path())
                .env("WORKESTRATE_CONFIG", &tool_home)
                .env("MSB_HOME", home.path().join("msb"))
                .env("PATH", "/nonexistent")
                .current_dir(home.path())
                .stdin(Stdio::null());
            cmd
        };
        let result = command()
            .args(["--json", "workload", "plan", "service"])
            .output()?;
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let plan: serde_json::Value = serde_json::from_slice(&result.stdout)?;
        let mount = &plan["mounts"][0];
        assert_eq!(mount["mode"], "ro");
        assert!(mount["policy"].is_object());
        let text = command().args(["workload", "plan", "service"]).output()?;
        assert!(
            text.status.success(),
            "{}",
            String::from_utf8_lossy(&text.stderr)
        );
        let text = String::from_utf8(text.stdout)?;
        if declaration.ends_with("\"off\"") {
            assert_eq!(mount["stat_virtualization"], "off");
            assert!(text.contains("  stat_virtualization: off"));
        } else {
            assert!(mount.get("stat_virtualization").is_none());
            assert!(!text.contains("  stat_virtualization:"));
        }
        assert_eq!(std::fs::read_dir(&source)?.count(), 0);
        assert!(!home.path().join("msb").exists());
    }
    Ok(())
}
