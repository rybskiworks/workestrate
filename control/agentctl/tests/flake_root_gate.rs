//! F2 lazy project-root gate — integration tests.
//!
//! Root cause being guarded against: `build_sandbox` used to call
//! `crate::config::project_root()` UNCONDITIONALLY, which hard-errors when
//! the resolved root lacks `flake.nix`. Detached service children re-exec
//! and inherit the operator's cwd, so `workestrate workload up litellm`
//! from a flake-less directory failed even though litellm is a registry
//! image with zero flake artifacts.
//!
//! The gate is now LAZY: `project_root()` is called only when the workload
//! uses (a) a nix-layered image recipe, (b) a `local_build` config, or (c) a
//! relative build-path mount — and the error names the triggering feature.
//!
//! The non-ignored tests here are KVM-free: they drive the real binary from
//! a flake-less TempDir cwd with every project_root tier scrubbed
//! (`AGENTCTL_ROOT` + `CARGO_MANIFEST_DIR` removed) and assert on the error
//! TEXT. The `#[ignore]`d test is the end-to-end regression on a KVM host.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_workestrate");

/// Minimal three-workload fixture: a plain registry-image service (`web`),
/// a nix-layered service (`builder`), and a registry-image service with a
/// `local_build` (`built`). No secrets, no mounts — the tests isolate the
/// project-root gate, nothing else.
const GATE_CONFIG_TOML: &str = r#"
schema_version = 1

[workloads.web]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = ["python", "-m", "http.server", "8080"]
log_stop_errors = true

[workloads.web.network.defaults]
egress = "deny"

[workloads.builder]
kind = "service"
image = { recipe = "nix-layered", name = "builder", tag = "latest", contents = ["cacert", "busybox", "fakeNss"], binary = { recipe = "npm-build", src = "flake://builder", npm_deps_hash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=" }, features = ["create_tmp"] }
command = ["true"]
log_stop_errors = true

[workloads.builder.network.defaults]
egress = "deny"

[workloads.built]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = ["true"]
log_stop_errors = true

[workloads.built.local_build]
recipe = "npm-build"
source = "flake://built"
gating_file = "package-lock.json"

[workloads.built.network.defaults]
egress = "deny"
"#;

fn uniq_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "workestrate-gate-{}-{}-{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ))
}

/// A fully isolated, flake-less environment: fresh HOME (+ XDG dirs), a
/// fresh writable MSB_HOME, the given config dir as the single layer, and
/// EVERY project_root tier scrubbed (`AGENTCTL_ROOT` and `CARGO_MANIFEST_DIR`
/// removed) — the process cwd is the flake-less `cwd`, so tier 3 cannot
/// rescue the resolution either.
fn gate_cmd(home: &Path, config_dir: &Path, cwd: &Path) -> Command {
    let mut c = Command::new(BIN);
    c.current_dir(cwd);
    c.env("HOME", home);
    c.env("XDG_CONFIG_HOME", home.join(".config"));
    c.env("XDG_DATA_HOME", home.join(".local").join("share"));
    c.env("XDG_STATE_HOME", home.join(".local").join("state"));
    c.env("MSB_HOME", home.join("msb-home"));
    c.env("WORKESTRATE_CONFIG_DIR", config_dir);
    c.env_remove("WORKESTRATE_STATE_DIR");
    c.env_remove("WORKESTRATE_NO_PROJECT_CONFIG");
    c.env_remove("WORKESTRATE_CONTEXT");
    c.env_remove("WORKESTRATE_HOME");
    c.env_remove("AGENTCTL_ROOT");
    c.env_remove("CARGO_MANIFEST_DIR");
    c
}

/// Write the gate fixture + a flake-less cwd + an isolated home.
struct GateFixture {
    home: PathBuf,
    config_dir: PathBuf,
    cwd: PathBuf,
}

impl GateFixture {
    fn new(label: &str) -> Self {
        let home = uniq_dir(&format!("{label}-home"));
        let config_dir = uniq_dir(&format!("{label}-config"));
        let cwd = uniq_dir(&format!("{label}-cwd"));
        std::fs::create_dir_all(&home).expect("create home");
        std::fs::create_dir_all(&config_dir).expect("create config dir");
        std::fs::create_dir_all(&cwd).expect("create cwd");
        std::fs::write(config_dir.join("workestrate.toml"), GATE_CONFIG_TOML)
            .expect("write gate fixture");
        assert!(
            !cwd.join("flake.nix").exists(),
            "test cwd must be flake-less"
        );
        Self {
            home,
            config_dir,
            cwd,
        }
    }

    fn cmd(&self) -> Command {
        gate_cmd(&self.home, &self.config_dir, &self.cwd)
    }
}

impl Drop for GateFixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.home);
        let _ = std::fs::remove_dir_all(&self.config_dir);
        let _ = std::fs::remove_dir_all(&self.cwd);
    }
}

/// A registry-image workload from a flake-less cwd must get PAST the
/// project-root gate: it fails later (sandbox create needs the msb
/// daemon/KVM, absent here), but NEVER with the flake-root error.
#[test]
fn registry_up_from_flake_less_cwd_never_hits_flake_root_gate() {
    let fx = GateFixture::new("registry");
    let out = fx
        .cmd()
        .args(["workload", "up", "web", "--foreground"])
        .output()
        .expect("spawn workload up web --foreground");
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    assert!(
        !out.status.success(),
        "web up cannot succeed without msb/KVM; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stdout),
        stderr
    );
    assert!(
        !stderr.contains("flake.nix") && !stderr.contains("project root"),
        "registry workload must NOT fail at the flake-root gate; stderr:\n{stderr}"
    );
}

/// A nix-layered workload declared by a FLAKE-LESS repo now fails EARLIER
/// than the project-root gate: the spec-21 phase-E ensure-images pre-flight
/// runs before dispatch (§2.4 fail-fast ordering), and the §7 "No flake.nix
/// in the declaring repo" row is a HARD ERROR naming the repo. The F2 gate
/// itself is unchanged — it still fires for nix-layered/local_build
/// workloads whose ensure passed (see the local_build test below and the
/// KVM-gated e2e in `ensure_images_e2e.rs`, which drives ensure past the
/// build and into the spawn path).
#[test]
fn nix_layered_up_without_declaring_flake_fails_at_ensure() {
    let fx = GateFixture::new("nix-layered");
    let out = fx
        .cmd()
        .args(["workload", "up", "builder", "--foreground"])
        .output()
        .expect("spawn workload up builder --foreground");
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    assert!(!out.status.success());
    assert!(
        stderr.contains("workload 'builder' declares a nix-layered image"),
        "the ensure pre-flight names the workload; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("has no flake.nix ancestor"),
        "the §7 missing-flake hard error; stderr:\n{stderr}"
    );
    assert!(
        stderr.contains("add a flake.nix to the config repo"),
        "remediation wording; stderr:\n{stderr}"
    );
}

/// A `local_build` workload (flake:// source) in the same flake-less
/// environment MUST fail at the gate, naming local_build.
#[test]
fn local_build_up_fails_with_feature_naming_gate_error() {
    let fx = GateFixture::new("local-build");
    let out = fx
        .cmd()
        .args(["workload", "up", "built", "--foreground"])
        .output()
        .expect("spawn workload up built --foreground");
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();

    assert!(!out.status.success());
    assert!(
        stderr.contains(
            "workload 'built' uses local_build config, which requires a flake project root"
        ),
        "gate error must name the workload and local_build; stderr:\n{stderr}"
    );
}

/// End-to-end regression on a KVM host: detached `workload up example-litellm`
/// from a flake-less cwd (the operator scenario that motivated the fix — the
/// detached child re-execs and inherits cwd). The child log must NEVER
/// contain the flake-root error; any failure must be sandbox/KVM-related.
///
/// Uses the committed 5-workload fixture (example-litellm is a plain
/// registry image with no flake artifacts) and the same isolation pattern
/// as `lifecycle_detached.rs`.
#[tokio::test]
#[ignore = "needs KVM + a loaded python:3.12-slim image; run manually with --ignored"]
async fn detached_example_litellm_up_from_flake_less_cwd_passes_project_root_gate() {
    let home = uniq_dir("kvm-home");
    let cwd = uniq_dir("kvm-cwd");
    std::fs::create_dir_all(&home).expect("create isolated HOME");
    std::fs::create_dir_all(&cwd).expect("create flake-less cwd");
    let fixture: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("config");

    let mut up = gate_cmd(&home, &fixture, &cwd);
    // Dummy non-placeholder values for every required fixture secret so the
    // child passes the secrets gate on a provisioned host.
    for (k, v) in [
        ("LITELLM_MASTER_KEY", "sk-test-master-key"),
        ("ODYSSEUS_ADMIN_PASSWORD", "test-admin-password"),
    ] {
        up.env(k, v);
    }
    let up = up
        .args(["workload", "up", "example-litellm"])
        .output()
        .expect("spawn workload up example-litellm");
    let up_stderr = String::from_utf8_lossy(&up.stderr).to_string();

    // The detached child's per-sandbox log is the source of truth for how
    // far it got (the parent returns after a 500ms grace window).
    let log_path = home.join(".microsandbox/sandboxes/example-litellm/workestrate.log");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut log = String::new();
    while std::time::Instant::now() < deadline {
        if let Ok(content) = std::fs::read_to_string(&log_path) {
            if content.contains("===== workestrate ") {
                log = content;
                break;
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }
    // Give the child a moment to write its outcome past the delimiter.
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    if let Ok(content) = std::fs::read_to_string(&log_path) {
        log = content;
    }

    assert!(
        log.contains("===== workestrate "),
        "detached child never wrote its run delimiter; up stderr:\n{up_stderr}\nlog:\n{log}"
    );
    assert!(
        !log.contains("flake.nix") && !log.contains("project root"),
        "REGRESSION: example-litellm (registry image) hit the flake-root gate \
         from a flake-less cwd; log:\n{log}"
    );

    // Best-effort cleanup on KVM hosts where the sandbox may have started.
    let _ = gate_cmd(&home, &fixture, &cwd)
        .args(["workload", "down", "example-litellm"])
        .output();
    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&cwd);
}
