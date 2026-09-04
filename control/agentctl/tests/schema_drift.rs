//! Standing CI guard for the JSON Schema artifacts (ADR 0021 §8).
//!
//! Invokes the built `workestrate` binary's `generate-schema` command with
//! `--output` + `--output-workload` + `--output-registry` and diffs all
//! three written files against the committed schemas at
//! `<workspace_root>/schemas/`:
//!   - `workestrate.schema.json` (full schema, from `ConfigFile`),
//!   - `workestrate-workload.schema.json` (workload subschema, from
//!     `WorkloadConfig`),
//!   - `registry.schema.json` (tool-home registry schema, from `Registry`).
//!
//! Bootstrap semantics: when a committed file is still the hand-written
//! PLACEHOLDER (no nix host has regenerated it yet), that artifact passes
//! with a warning. After `just generate-schema` runs once on a nix-capable
//! host, the placeholders are replaced and the test enforces strict
//! byte-for-byte parity on every subsequent run.
//!
//! This mirrors the `spec_examples_parse` guard from ADR 0020 Ruling 4. The
//! sibling `schema_subschema_drift.rs` guard covers the workload subschema
//! alone (kept passing; overlap is intentional, not confusion).

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

/// The three committed schema artifacts: (flag-file stem, committed name).
/// The full schema is written via `--output`; the other two via their
/// dedicated `--output-*` flags (both require `--output`).
const ARTIFACTS: &[(&str, &str, &str)] = &[
    ("full", "workestrate.schema.json", "--output"),
    (
        "workload",
        "workestrate-workload.schema.json",
        "--output-workload",
    ),
    ("registry", "registry.schema.json", "--output-registry"),
];

fn schemas_dir() -> std::path::PathBuf {
    // CARGO_MANIFEST_DIR = .../ai-workbench/control/agentctl
    // Workspace root    = .../ai-workbench
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("schemas")
}

#[test]
fn committed_schemas_match_generate_schema_output() {
    let bin = env!("CARGO_BIN_EXE_workestrate");

    // Scratch outputs under the system temp dir (a drift harness should not
    // depend on the shared helper; the unique name + cleanup is
    // self-contained).
    let tmp = std::env::temp_dir().join(format!(
        "workestrate-schema-drift-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&tmp).expect("create scratch dir for schema drift diff");

    let mut cmd = Command::new(bin);
    cmd.arg("generate-schema");
    let mut scratch_paths: Vec<(String, std::path::PathBuf, std::path::PathBuf)> = Vec::new();
    for (stem, committed_name, flag) in ARTIFACTS {
        let scratch = tmp.join(format!("{stem}.json"));
        cmd.arg(flag).arg(&scratch);
        scratch_paths.push((
            (*committed_name).to_string(),
            scratch,
            schemas_dir().join(committed_name),
        ));
    }

    // MSB_HOME / vendor issues don't affect generate-schema (no SDK calls),
    // but defensive: surface stderr if the binary fails to start at all.
    let output = cmd
        .output()
        .unwrap_or_else(|e| panic!("failed to invoke `workestrate generate-schema`: {e}"));

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "`workestrate generate-schema --output ... --output-workload ... --output-registry ...` \
             exited with status {:?}; stderr:\n{}",
            output.status.code(),
            stderr
        );
    }

    let mut failures: Vec<String> = Vec::new();
    for (name, scratch, committed_path) in &scratch_paths {
        let generated = std::fs::read_to_string(scratch)
            .unwrap_or_else(|_| panic!("generate-schema did not write {}", scratch.display()));
        let committed = std::fs::read_to_string(committed_path).unwrap_or_else(|_| {
            panic!(
                "missing committed schema at {}; the file should exist at the workspace root",
                committed_path.display()
            )
        });

        if committed.contains(PLACEHOLDER_MARKER) {
            eprintln!(
                "schema_drift: SKIP strict diff for {name} — committed schema at {} is still the \
                 PLACEHOLDER. Run `just generate-schema` on a nix-capable host to populate \
                 the canonical file; subsequent runs will enforce strict drift.",
                committed_path.display()
            );
            continue;
        }

        // Both sides carry a trailing newline (cmd_generate_schema writes
        // "{}\n" to --output*; the committed files were written the same
        // way). Trim both to make the comparison robust against
        // trailing-newline differences.
        if generated.trim() != committed.trim() {
            failures.push(format!(
                "schema drift for {name}: `workestrate generate-schema` output no longer matches \
                 the committed file at {}. Run `just generate-schema` to update the committed file.",
                committed_path.display()
            ));
        }
    }

    // Best-effort scratch cleanup (must not mask the assertions below).
    let _ = std::fs::remove_dir_all(&tmp);

    assert!(
        failures.is_empty(),
        "schema drift detected:\n{}",
        failures.join("\n")
    );
}

#[test]
fn committed_schema_paths_exist() {
    // Belt-and-suspenders: catches an accidental schema-file move/rename
    // independent of the drift check.
    for (_, committed_name, _) in ARTIFACTS {
        let path = schemas_dir().join(committed_name);
        assert!(
            path.exists(),
            "committed schema is missing at {}; the file is required for the drift guard",
            path.display()
        );
    }
}
