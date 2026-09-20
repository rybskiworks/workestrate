//! Integration tests for the config-scope `workestrate down --all`
//! confirmation (ADR 0032 addendum §Down scope ladder; the former
//! `down-all` verb survives as a hidden alias). Verifies the single-line
//! confirm read does not block waiting for EOF: a piped "y\n" proceeds, and
//! a piped refusal aborts. Uses an isolated HOME so no real state dir is
//! touched (down on an empty state dir is a no-op).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::IsolatedHome;
use std::process::Stdio;
/// Piping "y\n" to `down --all` confirms and proceeds without waiting for
/// EOF. With an empty state dir, the config scope is a no-op and the command
/// succeeds.
#[test]
fn down_all_piped_yes_proceeds_without_eof() {
    let home = IsolatedHome::new("cmd-down-all");
    let mut c = home.cmd();
    c.args(["down-all", "--all"]); // hidden alias kept (back-compat)
    c.stdin(Stdio::piped());
    c.stdout(Stdio::piped());
    c.stderr(Stdio::piped());
    let mut child = c.spawn().expect("spawn down-all");
    use std::io::Write;
    {
        let mut stdin = child.stdin.take().expect("take stdin");
        stdin.write_all(b"y\n").expect("write y");
    } // drop stdin → closes pipe; single-line read already returned
    let out = child.wait_with_output().expect("wait");
    assert!(
        out.status.success(),
        "down --all with piped 'y' should proceed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Piping "n\n" to `down --all` aborts (non-zero exit) with an "aborted"
/// message.
#[test]
fn down_all_piped_no_aborts() {
    let home = IsolatedHome::new("cmd-down-all");
    let mut c = home.cmd();
    c.args(["down", "--all"]);
    c.stdin(Stdio::piped());
    c.stdout(Stdio::piped());
    c.stderr(Stdio::piped());
    let mut child = c.spawn().expect("spawn down-all");
    use std::io::Write;
    {
        let mut stdin = child.stdin.take().expect("take stdin");
        stdin.write_all(b"n\n").expect("write n");
    }
    let out = child.wait_with_output().expect("wait");
    assert!(
        !out.status.success(),
        "down --all with piped 'n' should abort; got success"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("aborted"),
        "expected aborted message; got: {}",
        stderr
    );
}
