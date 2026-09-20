//! Integration test: a `depends_on` cycle in the config is rejected by
//! `workestrate validate-config` (ADR 0026 addendum 2026-08-01, W4) with a
//! non-zero exit and the `dependency cycle detected: a → b → a` message on
//! stderr. The config fixture is written per-test into a temp dir and
//! pointed at via `WORKESTRATE_FLEET_DIR` (the same single-layer bypass
//! `cmd_workloads.rs` uses against the committed fixture).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::IsolatedHome;

/// A 2-cycle fixture: a depends_on b, b depends_on a.
const CYCLE_CONFIG: &str = r#"
schema_version = 1

[workloads.a]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.a.depends_on.b]
env = "B_URL"

[workloads.b]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.b.depends_on.a]
env = "A_URL"
"#;

#[test]
fn validate_config_rejects_dependency_cycle() {
    let home = IsolatedHome::new("cmd-validate-cycle");
    let cfg_dir = home.dir.join("cfg");
    std::fs::create_dir_all(&cfg_dir).expect("create config dir");
    std::fs::write(cfg_dir.join("workestrate.toml"), CYCLE_CONFIG).expect("write cycle config");

    let out = home
        .cmd()
        .env("WORKESTRATE_FLEET_DIR", &cfg_dir)
        .args(["validate-config"])
        .output()
        .expect("invoke validate-config");
    assert!(
        !out.status.success(),
        "validate-config must fail on a dependency cycle; stdout=\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let stderr = String::from_utf8(out.stderr).expect("utf8 stderr");
    assert!(
        stderr.contains("dependency cycle detected:"),
        "stderr must carry the cycle message; got:\n{stderr}"
    );
}
