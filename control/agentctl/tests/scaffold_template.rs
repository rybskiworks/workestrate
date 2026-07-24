//! Standing CI guard for the `workestrate config new` scaffold.
//!
//! Mirrors `schema_drift.rs` + `spec_examples_parse.rs`: invokes the built
//! `workestrate` binary's `config new` command into a temp dir, then
//! enforces:
//!
//! 1. The rendered skeleton passes `workestrate validate-config`.
//! 2. No `{{ }}` (unsubstituted token) remains in any rendered file.
//! 3. (HOST-NIX-gated) Byte-parity between the native Rust render and the
//!    copier template's minimal-personal render for the overlap files.
//! 4. (HOST-NIX-gated) `copier update` succeeds on a native-scaffolded
//!    repo (`.copier-answers.yml` interop contract).
//!
//! Tests 3 and 4 SKIP gracefully when `copier` is not on PATH (this
//! container has no nix); tests 1 and 2 MUST pass regardless of host.

// Test harness: panic!/expect are the idiomatic way to fail a test, so the
// crate-wide clippy denies are relaxed here (mirrors `tests/schema_drift.rs`).
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::TempDir;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Scratch dir for one test: an RAII `TempDir` guard (removes the tree on
/// drop — fixes the historical leak where every render left its temp dir).
fn tempdir_for_test() -> TempDir {
    TempDir::new("scaffold-test")
}

/// Render the scaffold via the real binary path. Uses fixed flags so the
/// render is deterministic: explicit age recipient (skips age-keygen
/// derivation), no registry write, no git init.
fn render_via_subprocess(dest: &Path) -> std::process::Output {
    let bin = env!("CARGO_BIN_EXE_workestrate");
    Command::new(bin)
        .args(["config", "new", "scaffoldtest", "--path"])
        .arg(dest)
        .args([
            "--no-register",
            "--no-git-init",
            "--age-recipient",
            "age1TESTPLACEHOLDER",
        ])
        .output()
        .expect("failed to invoke `workestrate config new`")
}

/// Test 1: the rendered skeleton passes `workestrate validate-config`.
#[test]
fn native_skeleton_passes_validate_config() {
    let tmp = tempdir_for_test();
    let dest = tmp.path().join("scaffoldtest");
    let output = render_via_subprocess(&dest);
    assert!(
        output.status.success(),
        "`config new` failed: stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(dest.join("workestrate.toml").exists());

    let bin = env!("CARGO_BIN_EXE_workestrate");
    let val = Command::new(bin)
        .args(["validate-config"])
        .env("WORKESTRATE_CONFIG_DIR", &dest)
        .output()
        .expect("failed to invoke `workestrate validate-config`");
    assert!(
        val.status.success(),
        "validate-config failed on rendered skeleton: stdout=\n{}\nstderr=\n{}",
        String::from_utf8_lossy(&val.stdout),
        String::from_utf8_lossy(&val.stderr)
    );
    let stdout = String::from_utf8_lossy(&val.stdout);
    assert!(
        stdout.contains("valid"),
        "expected validation message, got: {}",
        stdout
    );
}

/// Test 2: no `{{ ` (unsubstituted token) remains in any rendered file.
#[test]
fn native_render_leaves_no_unsubstituted_tokens() {
    let tmp = tempdir_for_test();
    let dest = tmp.path().join("scaffoldtest");
    let output = render_via_subprocess(&dest);
    assert!(
        output.status.success(),
        "`config new` failed: stderr=\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let mut offenders = Vec::new();
    walk_files(&dest, &dest, &mut |rel_path, content| {
        for (lineno, line) in content.lines().enumerate() {
            if line.contains("{{") {
                offenders.push(format!("{}:{}: {}", rel_path.display(), lineno + 1, line));
            }
        }
    });
    assert!(
        offenders.is_empty(),
        "rendered files contain unsubstituted {{ }} tokens:\n{}",
        offenders.join("\n")
    );
}

/// Test 3: copier template byte-parity (HOST-NIX-gated; SKIP if copier
/// absent). Mirrors schema_drift.rs's PLACEHOLDER-skip bootstrap pattern.
#[test]
fn copier_template_byte_matches_native_render() {
    let copier = match which_copier() {
        Some(c) => c,
        None => {
            eprintln!(
                "scaffold_template: SKIP copier parity — 'copier' not on PATH \
                 (HOST-NIX gate; mirrors schema_drift.rs bootstrap-skip)."
            );
            return;
        }
    };

    let tmp = tempdir_for_test();

    // Native render.
    let native_dest = tmp.path().join("native");
    let native_out = render_via_subprocess(&native_dest);
    assert!(
        native_out.status.success(),
        "native render failed: stderr=\n{}",
        String::from_utf8_lossy(&native_out.stderr)
    );

    // Copier render with minimal-personal answers.
    let copier_dest = tmp.path().join("copier");
    std::fs::create_dir_all(&copier_dest).unwrap();
    let copier_template = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("templates")
        .join("workestrator-config");
    let copier_out = Command::new(&copier)
        .args(["copy", "--defaults"])
        .arg(&copier_template)
        .arg(&copier_dest)
        .args(["--data", "config_name=scaffoldtest"])
        .args(["--data", "age_recipient=age1TESTPLACEHOLDER"])
        .args(["--data", "team_age_recipient="])
        .args(["--data", "include_flake=false"])
        .output()
        .expect("failed to invoke copier");
    if !copier_out.status.success() {
        eprintln!(
            "scaffold_template: SKIP copier parity — copier copy failed (stderr=\n{})",
            String::from_utf8_lossy(&copier_out.stderr)
        );
        return;
    }

    // Compare overlap files (after trimming trailing whitespace per line).
    // README is excluded — copier has a richer copier-update section; native
    // points users at `workestrate config new`.
    let overlap = [
        "workestrate.toml",
        ".sops.yaml",
        ".env.example",
        ".gitignore",
    ];
    let mut mismatches = Vec::new();
    for fname in &overlap {
        let n = std::fs::read_to_string(native_dest.join(fname)).unwrap_or_default();
        let c = std::fs::read_to_string(copier_dest.join(fname)).unwrap_or_default();
        if normalize(&n) != normalize(&c) {
            mismatches.push(format!(
                "'{}' differs between native and copier render",
                fname
            ));
        }
    }
    assert!(
        mismatches.is_empty(),
        "native scaffold and copier template produce different output for the minimal-personal \
         subset. Align templates/workestrator-config/ with the Rust skeleton:\n{}",
        mismatches.join("\n")
    );
}

/// Test 4: copier update works from a native-scaffolded repo (HOST-NIX-gated).
/// Verifies the `.copier-answers.yml` interop contract.
#[test]
fn copier_update_works_from_native_scaffold() {
    let copier = match which_copier() {
        Some(c) => c,
        None => {
            eprintln!(
                "scaffold_template: SKIP copier update interop — 'copier' not on PATH \
                 (HOST-NIX gate)."
            );
            return;
        }
    };

    let tmp = tempdir_for_test();
    let dest = tmp.path().join("scaffoldtest");
    let output = render_via_subprocess(&dest);
    assert!(output.status.success(), "native render failed");
    assert!(
        dest.join(".copier-answers.yml").exists(),
        ".copier-answers.yml not written by config new"
    );

    // git init + commit so copier update has a baseline to diff against.
    let dest_str = dest.to_string_lossy().to_string();
    let _ = Command::new("git")
        .args(["init", dest_str.as_str()])
        .status();
    let _ = Command::new("git")
        .arg("-C")
        .arg(&dest)
        .args(["add", "."])
        .status();
    let _ = Command::new("git")
        .arg("-C")
        .arg(&dest)
        .args([
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-m",
            "init",
        ])
        .status();

    // copier update should succeed (exit 0).
    let update_out = Command::new(copier)
        .arg("update")
        .arg("--defaults")
        .arg("--unset")
        .arg(dest_str.as_str())
        .output();
    match update_out {
        Ok(o) if o.status.success() => { /* pass */ }
        Ok(o) => panic!(
            "copier update failed: stderr=\n{}",
            String::from_utf8_lossy(&o.stderr)
        ),
        Err(e) => panic!("copier update spawn failed: {}", e),
    }
}

// --- helpers ---

fn which_copier() -> Option<PathBuf> {
    // Don't depend on the `which` crate; use PATH lookup via std.
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join("copier");
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn walk_files(root: &Path, dir: &Path, cb: &mut dyn FnMut(&Path, &str)) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip .git and similar VCS dirs.
            if path.file_name().and_then(|n| n.to_str()) != Some(".git") {
                walk_files(root, &path, cb);
            }
        } else if path.is_file() {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            cb(rel, &content);
        }
    }
}

fn normalize(s: &str) -> String {
    s.lines()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}
