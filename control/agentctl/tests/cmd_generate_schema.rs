//! Integration tests for `workestrate generate-schema` — W1: canonical
//! `-o/--output` writes the schema to a file, `--out` remains a hidden
//! back-compat alias, and `--help` shows `--output` but not `--out`.
//! generate-schema makes no SDK/HOME calls; `common::TempDir` provides
//! scratch output paths.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use std::process::Command;

use common::{TempDir, BIN};

fn assert_schema_file_written(flag: &str) {
    let tmp = TempDir::new("cmd-generate-schema");
    let path = tmp.path().join("schema.json");
    let out = Command::new(BIN)
        .args(["generate-schema", flag])
        .arg(&path)
        .output()
        .unwrap_or_else(|e| panic!("invoke generate-schema {flag}: {e}"));
    assert!(
        out.status.success(),
        "generate-schema {flag} failed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let body = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("schema file must exist at {}: {e}", path.display()));
    let doc: serde_json::Value =
        serde_json::from_str(&body).expect("written schema must be valid JSON");
    assert!(doc.is_object(), "written schema must be a JSON object");
}

/// `generate-schema --output <path>` writes the schema file.
#[test]
fn generate_schema_output_writes_schema_file() {
    assert_schema_file_written("--output");
}

/// W1 back-compat: `generate-schema --out <path>` still writes the schema file.
#[test]
fn generate_schema_out_alias_still_works() {
    assert_schema_file_written("--out");
}

/// `--help` shows the canonical `--output` and hides the `--out` alias.
#[test]
fn generate_schema_help_shows_output_hides_out() {
    let out = Command::new(BIN)
        .args(["generate-schema", "--help"])
        .output()
        .expect("invoke generate-schema --help");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("--output"),
        "help must show --output; got: {stdout}"
    );
    // "--output" contains "--out" as a substring, so match the alias with a
    // trailing delimiter: `--out ` (space before a value name / padding) or
    // `--out,` cannot appear when the alias is hidden.
    assert!(
        !stdout.contains("--out "),
        "help must NOT show the hidden --out alias; got: {stdout}"
    );
}
