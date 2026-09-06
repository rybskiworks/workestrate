//! M1 credential-broker plan fixtures: rendered-plan goldens for the three
//! binding/confinement shapes.
//!
//! - (a) `broker-bound`: a grant with NO env binding (omission =
//!   broker-bound, the default, secure).
//! - (b) `guest-bound`: the same grant with a `bound = "guest"` consumption
//!   of its material via the EXISTING env idiom (real material via the
//!   existing secret-delivery path; the catalog entry is untouched).
//! - (c) `strict`: layer-level `[policy.ssh] strict = true` with no ssh
//!   grant — legal via a port-22 egress allowance; the plan renders the
//!   confinement flag with its ladder origin.
//!
//! Hermetic like `golden_plans.rs`: each case points
//! `WORKESTRATE_CONFIG_DIR` at its fixture dir (single dev layer, no
//! registry/project/local layers), serialized by `ENV_TEST_LOCK`.
//!
//! Regenerate after an INTENDED display change with
//! `CREDENTIALS_GOLDEN_UPDATE=1 cargo test --test credentials_broker`
//! (the `just golden-generate` convention, scoped to this broker suite).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

use std::path::PathBuf;

use workestrate::config::test_support::{ENV_TEST_LOCK, EnvGuard};
use workestrate::microsandbox::workload::{ConfigWorkload, Workload};

/// (fixture dir, workload name): the rendered plan must match the committed
/// `tests/credentials_broker/<case>.plan.txt` byte-for-byte.
const CASES: [(&str, &str); 3] = [
    ("broker-bound", "deployer"),
    ("guest-bound", "verifier"),
    ("strict", "hardened"),
];

fn case_dir(case: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("credentials_broker")
        .join(case)
}

fn golden_path(case: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("credentials_broker")
        .join(format!("{case}.plan.txt"))
}

/// Render `ConfigWorkload::plan` for one fixture case (its Display is the
/// plan/config output surface the goldens pin).
#[allow(unsafe_code)]
fn render_plan(case: &str, workload: &str) -> String {
    let _lock = ENV_TEST_LOCK.lock().unwrap();
    let _guard = EnvGuard::capture(&["WORKESTRATE_CONFIG_DIR"]);
    // SAFETY: serialized by ENV_TEST_LOCK (held for this function's
    // lifetime via _lock); EnvGuard restores the prior value on drop.
    unsafe { std::env::set_var("WORKESTRATE_CONFIG_DIR", case_dir(case)) };
    let wl = ConfigWorkload::new(workload)
        .unwrap_or_else(|e| panic!("fixture '{case}' workload '{workload}' failed to load: {e}"));
    wl.plan().to_string()
}

#[test]
fn credential_broker_plans_match_goldens() {
    for (case, workload) in CASES {
        let rendered = render_plan(case, workload);
        if std::env::var("CREDENTIALS_GOLDEN_UPDATE").is_ok() {
            std::fs::write(golden_path(case), &rendered)
                .unwrap_or_else(|e| panic!("failed to write golden for '{case}': {e}"));
            continue;
        }
        let expected = std::fs::read_to_string(golden_path(case))
            .unwrap_or_else(|e| panic!("failed to read golden for '{case}': {e}"));
        assert!(
            rendered == expected,
            "broker plan drift for '{case}': rendered plan no longer matches \
             control/agentctl/tests/credentials_broker/{case}.plan.txt.\n--- rendered ---\n{rendered}--- expected ---\n{expected}If the Display change is intended, regenerate with CREDENTIALS_GOLDEN_UPDATE=1.",
        );
    }
}
