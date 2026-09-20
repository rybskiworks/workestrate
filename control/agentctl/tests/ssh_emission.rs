//! SSH policy emission: the `ssh-divert` fixture plan compiles to the exact
//! guest-visible policy shape the fork's network engine consumes.
//!
//! Hermetic like `credentials_broker.rs`: points `WORKESTRATE_FLEET_DIR`
//! at the fixture dir (single dev layer), serialized by `ENV_TEST_LOCK`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

use std::path::PathBuf;

use workestrate::config::test_support::{ENV_TEST_LOCK, EnvGuard};
use workestrate::microsandbox::broker::ssh_config_for_plan;
use workestrate::microsandbox::workload::{ConfigWorkload, Workload};

/// Load the `ssh-divert` fixture plan (ssh grant + strict confinement).
#[allow(unsafe_code)]
fn diverter_plan() -> workestrate::microsandbox::plan::SandboxPlan {
    let _lock = ENV_TEST_LOCK.lock().unwrap();
    let _guard = EnvGuard::capture(&["WORKESTRATE_FLEET_DIR"]);
    let dir: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("credentials_broker")
        .join("ssh-divert");
    // SAFETY: serialized by ENV_TEST_LOCK (held for this function's
    // lifetime via _lock); EnvGuard restores the prior value on drop.
    unsafe { std::env::set_var("WORKESTRATE_FLEET_DIR", dir) };
    ConfigWorkload::new("diverter")
        .unwrap_or_else(|e| panic!("ssh-divert fixture failed to load: {e}"))
        .plan()
}

#[test]
fn ssh_divert_fixture_emits_exact_guest_policy() {
    let plan = diverter_plan();
    let credentials = plan
        .credentials
        .expect("diverter carries a credentials view");
    let config = ssh_config_for_plan(&credentials).expect("grants emit a policy");
    let value = serde_json::to_value(&config).unwrap();
    assert_eq!(
        value,
        serde_json::json!({
            "strict": true,
            "grants": [
                {"host": {"exact": "github.com"}, "ports": [{"start": 22, "end": 22}]},
            ],
            "on_violation": "block",
        }),
        "the emitted guest policy must match the fork's SshConfig shape exactly"
    );
    // Round-trip through the fork's domain type: the emitted value IS the
    // wire shape the runtime consumes.
    let back: microsandbox_types::SshConfig = serde_json::from_value(value).unwrap();
    assert_eq!(back, config);
}
