//! Integration tests for `workestrate down-all` confirmation. Verifies the
//! single-line confirm read does not block waiting for EOF: a piped "y\n"
//! proceeds, and a piped refusal aborts. Uses an isolated HOME so no real
//! state dir is touched (down_all on an empty state dir is a no-op).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

use std::path::PathBuf;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_workestrate");

struct IsolatedHome {
    dir: PathBuf,
}

impl IsolatedHome {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "workestrate-cmd-down-all-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("create isolated HOME");
        std::fs::create_dir_all(dir.join(".config")).expect("create .config");
        std::fs::create_dir_all(dir.join(".local").join("share")).expect("create .local/share");
        std::fs::create_dir_all(dir.join(".local").join("state")).expect("create .local/state");
        Self { dir }
    }

    /// Build a Command with HOME / XDG pointed at this isolated root.
    fn cmd(&self) -> Command {
        let mut c = Command::new(BIN);
        c.env("HOME", &self.dir);
        c.env("XDG_CONFIG_HOME", self.dir.join(".config"));
        c.env("XDG_DATA_HOME", self.dir.join(".local").join("share"));
        c.env("XDG_STATE_HOME", self.dir.join(".local").join("state"));
        c.env_remove("WORKESTRATE_CONFIG_DIR");
        c.env_remove("WORKESTRATE_NO_PROJECT_CONFIG");
        c.env_remove("WORKESTRATE_HOME");
        c
    }
}

/// Piping "y\n" to `down-all` confirms and proceeds without waiting for EOF.
/// With an empty state dir, down_all is a no-op and the command succeeds.
#[test]
fn down_all_piped_yes_proceeds_without_eof() {
    let home = IsolatedHome::new();
    let mut c = home.cmd();
    c.args(["down-all"]);
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
        "down-all with piped 'y' should proceed; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// Piping "n\n" to `down-all` aborts (non-zero exit) with an "aborted" message.
#[test]
fn down_all_piped_no_aborts() {
    let home = IsolatedHome::new();
    let mut c = home.cmd();
    c.args(["down-all"]);
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
        "down-all with piped 'n' should abort; got success"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("aborted"),
        "expected aborted message; got: {}",
        stderr
    );
}
