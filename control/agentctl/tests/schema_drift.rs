//! Standing CI guard for the JSON Schema artifact (ADR 0021 §8).
//!
//! Invokes the built `workestrate` binary's `generate-schema` command and
//! diffs its stdout against the committed schema at
//! `<workspace_root>/schemas/workestrate.schema.json`.
//!
//! Bootstrap semantics: when the committed file is still the hand-written
//! PLACEHOLDER (no nix host has regenerated it yet), the test passes with a
//! warning. After `just generate-schema` runs once on a nix-capable host,
//! the placeholder is replaced and the test enforces strict byte-for-byte
//! parity on every subsequent run.
//!
//! This mirrors the `spec_examples_parse` guard from ADR 0020 Ruling 4.

use std::path::Path;
use std::process::Command;

/// Substring the placeholder schema carries. Matched with `.contains` so
/// minor JSON formatting differences don't defeat the bootstrap detector.
const PLACEHOLDER_MARKER: &str = r#""$comment": "PLACEHOLDER""#;

fn committed_schema_path() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = .../ai-workbench/control/agentctl
    // Workspace root    = .../ai-workbench
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("schemas")
        .join("workestrate.schema.json")
}

#[test]
fn committed_schema_matches_generate_schema_output() {
    let bin = env!("CARGO_BIN_EXE_workestrate");

    // MSB_HOME / vendor issues don't affect generate-schema (no SDK calls),
    // but defensive: surface stderr if the binary fails to start at all.
    let output = Command::new(bin)
        .arg("generate-schema")
        .output()
        .unwrap_or_else(|e| panic!("failed to invoke `workestrate generate-schema`: {e}"));

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "`workestrate generate-schema` exited with status {:?}; stderr:\n{}",
            output.status.code(),
            stderr
        );
    }

    let generated = String::from_utf8(output.stdout)
        .expect("generate-schema stdout was not valid UTF-8");

    let committed_path = committed_schema_path();
    let committed = std::fs::read_to_string(&committed_path).unwrap_or_else(|_| {
        panic!(
            "missing committed schema at {}; the file should exist at the workspace root",
            committed_path.display()
        )
    });

    if committed.contains(PLACEHOLDER_MARKER) {
        eprintln!(
            "schema_drift: SKIP strict diff — committed schema at {} is still the \
             PLACEHOLDER. Run `just generate-schema` on a nix-capable host to populate \
             the canonical file; subsequent runs will enforce strict drift.",
            committed_path.display()
        );
        return;
    }

    // Both sides carry a trailing newline (cmd_generate_schema writes
    // "{}\n" to --out; println! adds "\n" to stdout). Trim both to make
    // the comparison robust against trailing-newline differences.
    assert_eq!(
        generated.trim(),
        committed.trim(),
        "schema drift: `workestrate generate-schema` output no longer matches the \
         committed schema at {}. Run `just generate-schema` to update the committed file.",
        committed_path.display()
    );
}

#[test]
fn committed_schema_path_exists() {
    // Belt-and-suspenders: catches a accidental schema-file move/rename
    // independent of the drift check.
    let path = committed_schema_path();
    assert!(
        path.exists(),
        "committed schema is missing at {}; the file is required for the drift guard",
        path.display()
    );
}
