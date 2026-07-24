//! Rust-side byte-equality guard for the golden plan files.
//!
//! The justfile `golden-check` shells out to the binary and diffs — but that
//! path is not a Rust test, so a `Display` drift in `SandboxPlan` could slip
//! past `cargo test`. This suite closes that blind spot: for each workload
//! it runs `workestrate <name> plan` against the committed `config.reference`
//! fixture and asserts the stdout is BYTE-identical to the committed golden
//! file (loaded via `include_str!`).
//!
//! Hermetic: the child gets `WORKESTRATE_CONFIG_DIR` pointed at
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

/// Render `workestrate <name> plan` against config.reference.
fn render_plan(name: &str) -> Vec<u8> {
    let out = Command::new(BIN)
        .args([name, "plan"])
        .env("WORKESTRATE_CONFIG_DIR", config_reference_dir())
        .env_remove("WORKESTRATE_NO_PROJECT_CONFIG")
        .env_remove("WORKESTRATE_HOME")
        .env_remove("WORKESTRATE_CONTEXT")
        .output()
        .unwrap_or_else(|e| panic!("failed to invoke `workestrate {name} plan`: {e}"));
    assert!(
        out.status.success(),
        "`workestrate {name} plan` failed: stderr=\n{}",
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
