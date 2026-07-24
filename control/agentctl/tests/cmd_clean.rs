//! Integration tests for `workestrate clean` — state-dir content removal,
//! non-interactive refusal, JSON shape, and absent-dir tolerance. Uses an
//! isolated HOME + XDG per test so the user's real state dir is never touched.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::IsolatedHome;
/// `clean --yes` removes the contents of workspaces/ and var/run/ but leaves
/// the directories themselves in place.
#[test]
fn clean_yes_removes_state_contents() {
    let home = IsolatedHome::new("cmd-clean");
    let state = home.state_dir();
    let workspaces = state.join("workspaces");
    let var_run = state.join("var").join("run");
    std::fs::create_dir_all(&workspaces).expect("create workspaces");
    std::fs::create_dir_all(&var_run).expect("create var/run");
    std::fs::write(workspaces.join("ws-file.txt"), "ws").expect("seed workspaces file");
    std::fs::write(var_run.join("litellm.json"), "{}").expect("seed var/run file");

    let out = home
        .cmd()
        .args(["clean", "--yes"])
        .output()
        .expect("invoke clean");
    assert!(
        out.status.success(),
        "clean --yes failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Dirs still exist, but are empty.
    assert!(workspaces.exists(), "workspaces dir should still exist");
    assert!(state.join("var").exists(), "var dir should still exist");
    let ws_entries: Vec<_> = std::fs::read_dir(&workspaces)
        .expect("read workspaces")
        .collect();
    assert!(ws_entries.is_empty(), "workspaces should be empty");
    let var_entries: Vec<_> = std::fs::read_dir(state.join("var"))
        .expect("read var")
        .collect();
    assert!(var_entries.is_empty(), "var should be empty");

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("workspaces") && stdout.contains("removed 1 entries"),
        "expected per-dir report; got: {}",
        stdout
    );
    assert!(
        stdout.contains("Cleaned") && stdout.contains("directories"),
        "expected summary line; got: {}",
        stdout
    );
}

/// Without --yes and with non-interactive stdin (/dev/null), clean refuses
/// and points at --yes.
#[test]
fn clean_without_yes_non_interactive_refuses() {
    let home = IsolatedHome::new("cmd-clean");
    let out = home.cmd().args(["clean"]).output().expect("invoke clean");
    assert!(
        !out.status.success(),
        "clean without --yes on non-interactive stdin should fail"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("non-interactive") && stderr.contains("--yes"),
        "expected refusal mentioning --yes; got: {}",
        stderr
    );
}

/// `clean --json` emits the documented JSON shape.
#[test]
fn clean_json_shape() {
    let home = IsolatedHome::new("cmd-clean");
    let state = home.state_dir();
    let workspaces = state.join("workspaces");
    std::fs::create_dir_all(&workspaces).expect("create workspaces");
    std::fs::write(workspaces.join("a"), "a").expect("seed a");
    std::fs::write(workspaces.join("b"), "b").expect("seed b");

    let out = home
        .cmd()
        .args(["clean", "--yes", "--json"])
        .output()
        .expect("invoke clean --json");
    assert!(
        out.status.success(),
        "clean --json failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert!(
        stdout.contains("\"state_dir\""),
        "missing state_dir key; got: {}",
        stdout
    );
    assert!(
        stdout.contains("\"cleaned\""),
        "missing cleaned key; got: {}",
        stdout
    );
    assert!(
        stdout.contains("\"dir\": \"workspaces\""),
        "missing workspaces entry; got: {}",
        stdout
    );
    assert!(
        stdout.contains("\"entries_removed\": 2"),
        "expected 2 removed entries for workspaces; got: {}",
        stdout
    );
    assert!(
        stdout.contains("\"status\": \"cleaned\""),
        "expected cleaned status; got: {}",
        stdout
    );
    // var and run were never created → absent.
    assert!(
        stdout.contains("\"status\": \"absent\""),
        "expected absent status for missing dirs; got: {}",
        stdout
    );
}

/// When the state dir does not exist at all, clean still succeeds and
/// reports every subdir as absent.
#[test]
fn clean_absent_state_dir_is_ok() {
    let home = IsolatedHome::new("cmd-clean");
    // Do NOT create state dir.
    let out = home
        .cmd()
        .args(["clean", "--yes"])
        .output()
        .expect("invoke clean");
    assert!(
        out.status.success(),
        "clean with absent state dir should succeed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("(absent)"),
        "expected absent markers; got: {}",
        stdout
    );
    assert!(
        stdout.contains("Cleaned 0 directories"),
        "expected 0 cleaned; got: {}",
        stdout
    );
}
