//! Rust-side byte-equality guard for the golden plan files.
//!
//! The justfile `golden-check` shells out to the binary and diffs — but that
//! path is not a Rust test, so a `Display` drift in `SandboxPlan` could slip
//! past `cargo test`. This suite closes that blind spot: for each workload
//! it runs `workestrate <name> plan` against the committed `config.reference`
//! fixture and asserts the stdout is BYTE-identical to the committed golden
//! file (loaded via `include_str!`).
//!
//! Hermetic: the child gets `WORKESTRATE_FLEET_DIR` pointed at
//! `config.reference`; all layering/registry env vars are removed so the
//! host's own workestrate state can never leak into the render.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_workestrate");

const WORKLOADS: [&str; 3] = ["example-service", "example-agent", "example-offensive"];

/// `<workspace_root>/config.reference` (manifest dir = control/agentctl).
fn config_reference_dir() -> PathBuf {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("config.reference");
    std::fs::canonicalize(&dir).unwrap_or(dir)
}

/// Render `workestrate workload plan <name>` against config.reference.
fn render_plan(name: &str) -> Vec<u8> {
    let out = Command::new(BIN)
        .args(["workload", "plan", name])
        .env("WORKESTRATE_FLEET_DIR", config_reference_dir())
        .env_remove("WORKESTRATE_NO_PROJECT_CONFIG")
        .env_remove("WORKESTRATE_CONFIG")
        .env_remove("WORKESTRATE_CONTEXT")
        .output()
        .unwrap_or_else(|e| panic!("failed to invoke `workestrate workload plan {name}`: {e}"));
    assert!(
        out.status.success(),
        "`workestrate workload plan {name}` failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}

/// Locate the first byte position where `actual` and `expected` differ and
/// render both surrounding lines for the failure message.
fn first_diff_context(actual: &[u8], expected: &[u8]) -> String {
    let split = actual
        .iter()
        .zip(expected.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| actual.len().min(expected.len()));
    let a_lines: Vec<&str> = std::str::from_utf8(actual)
        .unwrap_or("<non-utf8>")
        .lines()
        .collect();
    let e_lines: Vec<&str> = std::str::from_utf8(expected)
        .unwrap_or("<non-utf8>")
        .lines()
        .collect();
    let upto = std::str::from_utf8(&actual[..split]).unwrap_or("");
    let line_no = upto.matches('\n').count();
    format!(
        "first difference at byte {split} (line {}):\n  actual  : {}\n  expected: {}\n  (actual {} bytes / {} lines, expected {} bytes / {} lines)",
        line_no + 1,
        a_lines.get(line_no).unwrap_or(&"<eof>"),
        e_lines.get(line_no).unwrap_or(&"<eof>"),
        actual.len(),
        a_lines.len(),
        expected.len(),
        e_lines.len(),
    )
}

#[test]
fn golden_plans_match_byte_for_byte() {
    for name in WORKLOADS {
        let golden: &str = match name {
            "example-service" => include_str!("golden/example-service.plan.txt"),
            "example-agent" => include_str!("golden/example-agent.plan.txt"),
            "example-offensive" => include_str!("golden/example-offensive.plan.txt"),
            other => unreachable!("unknown workload {other}"),
        };
        let stdout = render_plan(name);
        assert!(
            stdout == golden.as_bytes(),
            "golden drift for `{name}`: `workestrate {name} plan` output no longer \
             matches control/agentctl/tests/golden/{name}.plan.txt byte-for-byte.\n{}\n\
             If the Display change is intended, regenerate the goldens \
             (just golden-check / golden-update).",
            first_diff_context(&stdout, golden.as_bytes()),
        );
    }
}

/// ADR 0026/C2: `plan --instance <id>` renders the prospective parallel-slot
/// plan — `<slot>@<id>` name plus the prospective per-instance bind
/// (`127.0.0.2` on an empty registry) on every port.
///
/// Hermetic: `WORKESTRATE_STATE_DIR` points at a fresh temp dir so the
/// prospective bind comes from an EMPTY registry view (never the dev's real
/// state home); the temp dir is removed afterwards.
#[test]
fn golden_parallel_instance_plan_matches_byte_for_byte() {
    let golden: &str = include_str!("golden/example-service.plan-instance.txt");

    let state_dir = std::env::temp_dir().join(format!(
        "workestrate-golden-instance-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&state_dir)
        .unwrap_or_else(|e| panic!("failed to create temp state dir: {e}"));

    let out = Command::new(BIN)
        .args(["workload", "plan", "example-service", "--instance", "canary"])
        .env("WORKESTRATE_FLEET_DIR", config_reference_dir())
        .env("WORKESTRATE_STATE_DIR", &state_dir)
        .env_remove("WORKESTRATE_NO_PROJECT_CONFIG")
        .env_remove("WORKESTRATE_CONFIG")
        .env_remove("WORKESTRATE_CONTEXT")
        .output()
        .unwrap_or_else(|e| {
            panic!("failed to invoke `workestrate workload plan example-service --instance canary`: {e}")
        });
    let _ = std::fs::remove_dir_all(&state_dir);

    assert!(
        out.status.success(),
        "`workestrate workload plan example-service --instance canary` failed: stderr=\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stdout == golden.as_bytes(),
        "golden drift for `workload plan example-service --instance canary`: output no longer \
         matches control/agentctl/tests/golden/example-service.plan-instance.txt \
         byte-for-byte.\n{}\nIf the Display change is intended, regenerate the fixture \
         (WORKESTRATE_FLEET_DIR=config.reference WORKESTRATE_STATE_DIR=$(mktemp -d) \
         cargo run --manifest-path control/agentctl/Cargo.toml -- workload plan \
         example-service --instance canary).",
        first_diff_context(&out.stdout, golden.as_bytes()),
    );
}
