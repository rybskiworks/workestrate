//! Regression: `workestrate workload plan <w> --use <dep>@<id>` must run
//! depends_on resolution exactly ONCE per invocation. The per-IP bind
//! warning (ADR 0026(f) DEFERRED-PENDING-E1) previously printed TWICE:
//! main.rs constructs the workload with the parsed `--use` overrides (first
//! resolution) and cmd_plan's `plan_holder` block re-constructed it (second
//! resolution). cmd_plan now renders the caller-constructed workload, so the
//! warning prints exactly once while the plan still reflects the SELECTED
//! parallel record's port.
//!
//! Fixture: the committed reference config (via `WORKESTRATE_REFERENCE_CONFIG=1`
//! in `IsolatedHome::cmd`) — `example-service` declares
//! `depends_on.example-litellm env = "LITELLM_ADDR"` — plus a hand-seeded
//! parallel record `example-litellm@prime-canary` on the per-IP bind
//! 127.0.0.2 publishing host port 14000.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::IsolatedHome;

/// `plan --use` against a parallel (per-IP bind) record prints the bind
/// warning EXACTLY once and renders the selected record's port.
#[test]
fn plan_use_prints_per_ip_bind_warning_exactly_once() {
    let home = IsolatedHome::new("plan-use-single-warning");
    let run_dir = home.state_dir().join("var").join("run");
    std::fs::create_dir_all(&run_dir).expect("create var/run");
    // Parallel record on a per-IP bind — selecting it via --use triggers the
    // DEFERRED-PENDING-E1 warning arm in discovery resolution.
    std::fs::write(
        run_dir.join("example-litellm@prime-canary.json"),
        r#"{"instance":"example-litellm@prime-canary","context":null,"workload":"example-litellm","ports":[14000],"port_pairs":[],"created_at":"","bind_ip":"127.0.0.2"}"#,
    )
    .expect("seed parallel record");

    let out = home
        .cmd()
        .args([
            "workload",
            "plan",
            "example-service",
            "--use",
            "example-litellm@prime-canary",
        ])
        .output()
        .expect("invoke plan");
    assert!(
        out.status.success(),
        "plan failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stderr = String::from_utf8(out.stderr).expect("utf8 stderr");
    let warnings = stderr.matches("per-IP bind").count();
    assert_eq!(
        warnings, 1,
        "the per-IP bind warning must print EXACTLY once per invocation (resolution runs once); stderr:\n{stderr}"
    );

    // The plan itself reflects the SELECTED parallel record (port 14000) —
    // dropping the cmd_plan re-construction did not lose the --use override.
    let stdout = String::from_utf8(out.stdout).expect("utf8 stdout");
    assert!(
        stdout.contains("host.microsandbox.internal:14000"),
        "the plan must inject the selected parallel record's port; stdout:\n{stdout}"
    );
}
