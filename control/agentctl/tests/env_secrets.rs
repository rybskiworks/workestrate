//! Environment-only development input: synthetic credentials, no VM or network.
#![cfg(unix)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

mod common;

use common::{BIN, IsolatedHome};
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::{Command, Output};

const CONFIG: &str = r#"
schema_version = 1
[secrets.MACHINE]
env_var = "MACHINE_GITHUB_TOKEN"
placeholder = "example-only"
allowed_hosts = ["example.invalid"]
[workloads.agent]
kind = "agent"
image = { recipe = "registry", ref = "synthetic:latest" }
command = ["/bin/true"]
[workloads.agent.env]
GITHUB_TOKEN = { secret = "MACHINE", bound = "host" }
GH_TOKEN = { secret = "MACHINE", bound = "host" }
[workloads.service]
kind = "service"
image = { recipe = "registry", ref = "synthetic:latest" }
command = ["/bin/true"]
[workloads.service.env]
GH_TOKEN = { secret = "MACHINE", bound = "host" }
"#;

struct Fixture {
    home: IsolatedHome,
    config: PathBuf,
    spies: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let home = IsolatedHome::new("env-secrets");
        let config = home.dir.join("fleet");
        let spies = home.dir.join("bin");
        std::fs::create_dir_all(&config).unwrap();
        std::fs::create_dir_all(&spies).unwrap();
        std::fs::write(config.join("workestrate.toml"), CONFIG).unwrap();
        std::fs::write(
            config.join(".env.enc"),
            "not encrypted; must not be opened in env mode",
        )
        .unwrap();
        std::fs::write(home.dir.join("fake-age-key"), "not an age key").unwrap();
        for name in ["sops", "nix", "msb"] {
            let path = spies.join(name);
            std::fs::write(
                &path,
                format!("#!/bin/sh\n: > \"$PROBE_ROOT/{name}-called\"\nexit 71\n"),
            )
            .unwrap();
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        Self {
            home,
            config,
            spies,
        }
    }

    fn cmd(&self) -> Command {
        let mut cmd = Command::new(BIN);
        cmd.env_clear()
            .current_dir(&self.home.dir)
            .env("HOME", &self.home.dir)
            .env("XDG_CONFIG_HOME", self.home.dir.join(".config"))
            .env("XDG_DATA_HOME", self.home.dir.join(".local/share"))
            .env("XDG_STATE_HOME", self.home.dir.join(".local/state"))
            .env("WORKESTRATE_HOME", self.home.dir.join("work-home"))
            .env("WORKESTRATE_CONFIG_DIR", &self.config)
            .env("SOPS_AGE_KEY_FILE", self.home.dir.join("fake-age-key"))
            .env("PROBE_ROOT", &self.home.dir)
            .env(
                "PATH",
                format!(
                    "{}:{}",
                    self.spies.display(),
                    std::env::var("PATH").unwrap_or_default()
                ),
            )
            .stdin(std::process::Stdio::null());
        cmd
    }

    fn no_external_commands(&self) {
        for name in ["sops", "nix", "msb"] {
            assert!(
                !self.home.dir.join(format!("{name}-called")).exists(),
                "unexpected {name} invocation"
            );
        }
    }
}

fn success(out: Output) -> String {
    assert!(
        out.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

#[test]
fn env_run_canonical_wins_without_sops_or_age_key() {
    let fixture = Fixture::new();
    std::fs::remove_file(fixture.home.dir.join("fake-age-key")).unwrap();
    let out = fixture.cmd()
        .env("MACHINE_GITHUB_TOKEN", "$MSB_synthetic_delegated")
        .env("GITHUB_TOKEN", "synthetic-different-alias")
        .args(["--secrets-source", "env", "run", "--", "sh", "-c",
            "test \"$MACHINE_GITHUB_TOKEN\" = '$MSB_synthetic_delegated' && test \"$WORKESTRATE_SECRETS_SOURCE\" = env && printf 'resolved\\n'"])
        .output().unwrap();
    assert_eq!(success(out), "resolved\n");
    fixture.no_external_commands();
}

#[test]
fn env_alias_fallback_reaches_nested_run_and_explicit_mode_wins() {
    let fixture = Fixture::new();
    let out = fixture.cmd()
        .env("GITHUB_TOKEN", "synthetic-alias")
        .env("WORKESTRATE_SECRETS_SOURCE", "invalid-inherited")
        .args(["--secrets-source", "env", "run", "--", BIN, "run", "--", "sh", "-c",
            "test \"$MACHINE_GITHUB_TOKEN\" = synthetic-alias && test \"$WORKESTRATE_SECRETS_SOURCE\" = env && printf 'inherited\\n'"])
        .output().unwrap();
    assert_eq!(success(out), "inherited\n");
    fixture.no_external_commands();
}

#[test]
fn default_layered_and_explicit_nested_reset_still_invoke_sops() {
    for nested in [false, true] {
        let fixture = Fixture::new();
        let mut cmd = fixture.cmd();
        cmd.env("MACHINE_GITHUB_TOKEN", "synthetic");
        if nested {
            cmd.args([
                "--secrets-source",
                "env",
                "run",
                "--",
                BIN,
                "--secrets-source",
                "layered",
                "run",
                "--",
                "sh",
                "-c",
                "exit 0",
            ]);
        } else {
            cmd.args(["run", "--", "sh", "-c", "exit 0"]);
        }
        // Layered behavior tolerates a failed decrypt when process values
        // satisfy requirements. The spy, not the exit code, proves the path.
        let _ = cmd.output().unwrap();
        assert!(fixture.home.dir.join("sops-called").exists());
    }
}

#[test]
fn missing_required_fails_named_launch_before_runtime_or_image_commands() {
    for args in [
        vec!["workload", "exec", "agent"],
        vec!["workload", "up", "service"],
    ] {
        let fixture = Fixture::new();
        let out = fixture
            .cmd()
            .args(["--secrets-source", "env"])
            .args(args)
            .output()
            .unwrap();
        assert!(!out.status.success());
        let error = String::from_utf8_lossy(&out.stderr);
        assert!(error.contains("required secret 'MACHINE'"), "{error}");
        fixture.no_external_commands();
        assert!(
            !fixture.home.dir.join("work-home/state").exists(),
            "launch allocated state before secret preflight"
        );
    }
}

#[test]
fn conflicts_and_example_values_fail_without_value_disclosure() {
    for pairs in [
        vec![
            ("GH_TOKEN", "synthetic-private-a"),
            ("GITHUB_TOKEN", "synthetic-private-b"),
        ],
        vec![("MACHINE_GITHUB_TOKEN", "example-only")],
        vec![
            ("MACHINE_GITHUB_TOKEN", ""),
            ("GH_TOKEN", "synthetic-private-a"),
        ],
    ] {
        let fixture = Fixture::new();
        let out = fixture
            .cmd()
            .envs(pairs)
            .args(["--secrets-source", "env", "run", "--", "sh", "-c", "exit 0"])
            .output()
            .unwrap();
        assert!(!out.status.success());
        let output = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!output.contains("synthetic-private") && !output.contains("example-only"));
        fixture.no_external_commands();
    }
}

#[test]
fn plan_does_not_resolve_or_print_secrets_and_preserves_delivery_policy() {
    let fixture = Fixture::new();
    let plain = success(
        fixture
            .cmd()
            .args(["workload", "plan", "agent"])
            .output()
            .unwrap(),
    );
    let env = success(
        fixture
            .cmd()
            .env("MACHINE_GITHUB_TOKEN", "synthetic-never-print")
            .args(["workload", "plan", "agent", "--secrets-source", "env"])
            .output()
            .unwrap(),
    );
    assert_eq!(plain, env);
    for alias in ["GITHUB_TOKEN", "GH_TOKEN"] {
        assert!(env.contains(&format!(
            "secret_env: {alias} (value redacted, allowed: example.invalid, required)"
        )));
    }
    assert!(!env.contains("MACHINE_GITHUB_TOKEN"));
    assert!(!env.contains("synthetic-never-print"));
    fixture.no_external_commands();
}

#[test]
fn explicit_context_and_home_are_preserved_in_env_mode() {
    let fixture = Fixture::new();
    let work_home = fixture.home.dir.join("work-home");
    std::fs::create_dir_all(&work_home).unwrap();
    std::fs::write(
        work_home.join("config.toml"),
        format!(
            r#"
[settings]
default_context = "selected"
[configs.local]
url = "{}"
[contexts.selected]
layers = ["local"]
"#,
            fixture.config.display()
        ),
    )
    .unwrap();
    let out = fixture
        .cmd()
        .env_remove("WORKESTRATE_CONFIG_DIR")
        .env("GH_TOKEN", "synthetic-context")
        .args([
            "--secrets-source",
            "env",
            "--home",
            work_home.to_str().unwrap(),
            "--context",
            "selected",
            "run",
            "--",
            "sh",
            "-c",
            "test \"$MACHINE_GITHUB_TOKEN\" = synthetic-context && printf 'context\\n'",
        ])
        .output()
        .unwrap();
    assert_eq!(success(out), "context\n");
    fixture.no_external_commands();
}

#[test]
fn source_help_and_invalid_selection_are_explicit() {
    let fixture = Fixture::new();
    let help = success(fixture.cmd().arg("--help").output().unwrap());
    assert!(help.contains("--secrets-source") && help.contains("layered") && help.contains("env"));
    let invalid = fixture
        .cmd()
        .args(["--secrets-source", "automatic", "run", "--", "sh"])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(2));
    fixture.no_external_commands();
}
