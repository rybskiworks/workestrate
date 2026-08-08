//! Standing CI guard for the JSON Schema workload subschema artifact
//! (ADR 0021 §8).
//!
//! Invokes the built `workestrate` binary's `generate-schema` command with
//! `--output` + `--output-workload` and diffs the written workload subschema
//! against the committed schema at
//! `<workspace_root>/schemas/workestrate-workload.schema.json`.
//!
//! Bootstrap semantics: when the committed file is still the hand-written
//! PLACEHOLDER (no nix host has regenerated it yet), the test passes with a
//! warning. After `just generate-schema` runs once on a nix-capable host,
//! the placeholder is replaced and the test enforces strict byte-for-byte
//! parity on every subsequent run.
//!
//! This mirrors the `spec_examples_parse` guard from ADR 0020 Ruling 4 and
//! the sibling `schema_drift.rs` guard for the full schema.

// Test harness: panic!/expect are the idiomatic way to fail a test, so the
// crate-wide clippy denies are relaxed here (mirrors the `#[cfg(test)]`
// module allow at the foot of `src/microsandbox/runtime.rs`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

use std::path::Path;
use std::process::Command;

/// Substring the placeholder schema carries. Matched with `.contains` so
/// minor JSON formatting differences don't defeat the bootstrap detector.
const PLACEHOLDER_MARKER: &str = r#""$comment": "PLACEHOLDER""#;

fn committed_workload_schema_path() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = .../ai-workbench/control/agentctl
    // Workspace root    = .../ai-workbench
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("schemas")
        .join("workestrate-workload.schema.json")
}

#[test]
fn committed_workload_schema_matches_generate_schema_output() {
    let bin = env!("CARGO_BIN_EXE_workestrate");

    // Scratch outputs under the system temp dir (the sibling schema_drift
    // guard avoids `common::TempDir` for the same reason: a drift harness
    // should not depend on the shared helper; the unique name + cleanup is
    // self-contained).
    let tmp = std::env::temp_dir().join(format!(
        "workestrate-schema-subschema-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&tmp).expect("create scratch dir for schema subschema diff");
    let full_path = tmp.join("full.json");
    let workload_path = tmp.join("workload.json");

    // MSB_HOME / vendor issues don't affect generate-schema (no SDK calls),
    // but defensive: surface stderr if the binary fails to start at all.
    let output = Command::new(bin)
        .arg("generate-schema")
        .arg("--output")
        .arg(&full_path)
        .arg("--output-workload")
        .arg(&workload_path)
        .output()
        .unwrap_or_else(|e| panic!("failed to invoke `workestrate generate-schema`: {e}"));

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "`workestrate generate-schema --output ... --output-workload ...` exited with \
             status {:?}; stderr:\n{}",
            output.status.code(),
            stderr
        );
    }

    let generated = std::fs::read_to_string(&workload_path).unwrap_or_else(|_| {
        panic!(
            "generate-schema --output-workload did not write {}",
            workload_path.display()
        )
    });

    let committed_path = committed_workload_schema_path();
    let committed = std::fs::read_to_string(&committed_path).unwrap_or_else(|_| {
        panic!(
            "missing committed workload schema at {}; the file should exist at the workspace root",
            committed_path.display()
        )
    });

    // Best-effort scratch cleanup (must not mask the assertions below).
    let _ = std::fs::remove_dir_all(&tmp);

    if committed.contains(PLACEHOLDER_MARKER) {
        eprintln!(
            "schema_subschema_drift: SKIP strict diff — committed workload schema at {} is \
             still the PLACEHOLDER. Run `just generate-schema` on a nix-capable host to \
             populate the canonical file; subsequent runs will enforce strict drift.",
            committed_path.display()
        );
        return;
    }

    // Both sides carry a trailing newline (cmd_generate_schema writes
    // "{}\n" to --output-workload; the committed file was written the same
    // way). Trim both to make the comparison robust against
    // trailing-newline differences.
    assert_eq!(
        generated.trim(),
        committed.trim(),
        "schema drift: `workestrate generate-schema` workload subschema no longer matches \
         the committed file at {}. Run `just generate-schema` to update the committed file.",
        committed_path.display()
    );
}

#[test]
fn committed_workload_schema_path_exists() {
    // Belt-and-suspenders: catches a accidental schema-file move/rename
    // independent of the drift check.
    let path = committed_workload_schema_path();
    assert!(
        path.exists(),
        "committed workload schema is missing at {}; the file is required for the drift guard",
        path.display()
    );
}
