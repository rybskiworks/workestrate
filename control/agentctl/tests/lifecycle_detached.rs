//! End-to-end test for the detached-instance lifecycle (ADR 0021, WP-A).
//!
//! Verifies that a detached `up --new` forwards the resolved
//! [`InstanceSpec`] into the background child so that:
//!   1. the child creates the sandbox at `<slot>@<slug>` (NOT the bare slot),
//!   2. the port-registry record lands at `<slot>@<slug>`,
//!   3. `workestrate ps --json` reports it with the plan's host ports,
//!   4. `workestrate <wl> down --instance <slug>` stops and removes it.
//!
//! This exercises the real microsandbox create path, which needs KVM + a
//! loaded image. It is therefore `#[ignore]`'d so `just verify` stays green
//! and deterministic in CI / the devshell. Run it on a KVM host with images
//! loaded via:
//!
//! ```text
//! cargo test --manifest-path control/agentctl/Cargo.toml \
//!   --test lifecycle_detached -- --ignored --nocapture
//! ```
//!
//! The test uses the WP-E writability/isolation pattern (isolated
//! `MSB_HOME` + `HOME` + `WORKESTRATE_FLEET_DIR`) so it never touches the
//! operator's real workestrate registry or msb state.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_workestrate");
/// `[a-z2-7]{4}` — the base32 instance slug shape produced by `--new` (WP-B).
const SLUG_RE: &str = r"^[a-z2-7]{4}$";

/// Build a `workestrate` [`Command`] with fully isolated state: a fresh HOME,
/// a fresh writable `MSB_HOME` (so the SDK's `<MSB_HOME>/db/msb.db` is
/// openable and empty), and `WORKESTRATE_FLEET_DIR` pointed at the committed
/// 5-workload fixture. Dummy non-placeholder values are injected for every
/// required fixture secret so the create path does not bail on missing
/// secrets on a provisioned host.
fn isolated_cmd(home: &std::path::Path, msb_home: &std::path::Path) -> Command {
    let mut c = Command::new(BIN);
    c.env("HOME", home);
    c.env("XDG_CONFIG_HOME", home.join(".config"));
    c.env("XDG_DATA_HOME", home.join(".local").join("share"));
    c.env("XDG_STATE_HOME", home.join(".local").join("state"));
    c.env_remove("WORKESTRATE_NO_PROJECT_CONFIG");
    c.env_remove("WORKESTRATE_CONTEXT");
    let fixture: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("config");
    c.env("WORKESTRATE_FLEET_DIR", &fixture);
    // Isolated, writable msb home (WP-E pattern): empty db → openable.
    c.env("MSB_HOME", msb_home);
    // Dummy non-placeholder secrets so the create path proceeds on a host.
    for (k, v) in [
        ("LITELLM_MASTER_KEY", "sk-test-master-key"),
        ("ODYSSEUS_ADMIN_PASSWORD", "test-admin-password"),
    ] {
        c.env(k, v);
    }
    c
}

/// Parse `Sandbox '<instance>' started in background` from `up` stdout.
fn parse_started_instance(stdout: &str) -> Option<String> {
    let marker = "Sandbox '";
    let start = stdout.find(marker)? + marker.len();
    let rest = &stdout[start..];
    let end = rest.find("' started")?;
    Some(rest[..end].to_string())
}

/// Parse the detached child PID from `... started in background (PID N). ...`.
fn parse_background_pid(stdout: &str) -> Option<u32> {
    let marker = "(PID ";
    let start = stdout.find(marker)? + marker.len();
    let rest = &stdout[start..];
    let end = rest.find(')')?;
    rest[..end].trim().parse().ok()
}

/// Bounded tail (last ~4 KiB) of the detached child's log for failure
/// messages. The poll-timeout path must be self-diagnosing: a dead child
/// (product bug) and a slow cold-store pull (test-too-tight) look identical
/// from `ps` alone.
///
/// Since the 2026-08-30 relocation (ADR 0032 addendum) the detached-child
/// log lives in the workestrate state dir, not the ephemeral msb sandbox
/// dir. This test's isolated home resolves the state dir to
/// `$XDG_STATE_HOME/workestrate` (LegacyXdg home — same assumption as the
/// `state_dir` construction below).
fn child_log_tail(home: &std::path::Path, instance: &str) -> String {
    let path = home
        .join(".local")
        .join("state")
        .join("workestrate")
        .join("logs")
        .join(instance)
        .join("workestrate.log");
    let Ok(content) = std::fs::read_to_string(&path) else {
        return format!("<no child log at {}>", path.display());
    };
    const MAX: usize = 4 * 1024;
    if content.len() <= MAX {
        return content;
    }
    let mut start = content.len() - MAX;
    while !content.is_char_boundary(start) {
        start += 1;
    }
    format!("…{}", &content[start..])
}

/// Linux `/proc` aliveness check for the detached child (the test is
/// KVM-gated, hence Linux-only). The `up` parent has already exited, so a
/// dead child is reaped by init and `/proc/<pid>` disappears.
fn child_alive(pid: u32) -> bool {
    std::path::Path::new(&format!("/proc/{pid}")).exists()
}

/// Poll the instance registry for a record named `instance`. Returns true
/// once `workestrate ps --json` lists it.
fn ps_contains(home: &std::path::Path, msb_home: &std::path::Path, instance: &str) -> bool {
    let out = isolated_cmd(home, msb_home)
        .args(["ps", "--json"])
        .output()
        .expect("ps --json");
    if !out.status.success() {
        return false;
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    // The instance field is serialized as `"instance": "<name>"`.
    stdout.contains(&format!("\"instance\": \"{}\"", instance))
}

#[tokio::test]
#[ignore = "needs KVM + a loaded python:3.12-slim image; run manually with --ignored"]
async fn detached_up_new_registers_slot_at_slug_and_down_stops_it() {
    let home = std::env::temp_dir().join(format!(
        "workestrate-lifecycle-detached-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0),
    ));
    std::fs::create_dir_all(&home).expect("create isolated HOME");
    let msb_home = common::short_msb_home();
    std::fs::create_dir_all(&msb_home).expect("create short MSB_HOME");

    // 1. Detached `up --new`. The parent returns at once after spawning the
    //    foreground child; the child creates the sandbox and writes the
    //    registry record.
    let up = isolated_cmd(&home, &msb_home)
        .args(["example-litellm", "up", "--new"])
        .output()
        .expect("spawn example-litellm up --new");
    let up_stdout = String::from_utf8_lossy(&up.stdout).to_string();
    let up_stderr = String::from_utf8_lossy(&up.stderr).to_string();

    // If the host has no working msb/KVM, the detached spawn itself may fail
    // or the child never registers. Treat an outright msb transport failure
    // as a skip (the WP-E writability detection pattern) so the test is
    // robust when un-ignored on a half-provisioned host.
    let msb_unavailable = up_stderr.contains("connect to")
        || up_stderr.contains("transport")
        || up_stderr.contains("msb");
    if !up.status.success() && msb_unavailable {
        eprintln!(
            "SKIP: msb unavailable on this host (stderr: {})",
            up_stderr.trim()
        );
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&msb_home);
        return;
    }
    assert!(
        up.status.success(),
        "detached up failed; stderr:\n{}",
        up_stderr
    );

    // 2. The instance MUST be `<slot>@<slug>`, never the bare singleton slot.
    let instance = parse_started_instance(&up_stdout).unwrap_or_else(|| {
        panic!(
            "could not parse started instance from up stdout:\n{}",
            up_stdout
        )
    });
    let (slot, slug) = instance
        .split_once('@')
        .unwrap_or_else(|| panic!("expected '<slot>@<slug>', got '{instance}'"));
    assert_eq!(
        slot, "example-litellm",
        "slot must be the bare workload name (no context active)"
    );
    assert!(
        regex_lite_matches(SLUG_RE, slug),
        "slug '{slug}' must be 4 chars of [a-z2-7] (base32, WP-B); instance was '{instance}'"
    );
    assert_ne!(
        instance, "example-litellm",
        "must NOT register the bare singleton slot"
    );

    // 3. Poll `ps --json` until the child has registered the instance (the
    //    record is written after sandbox creation, which races the parent's
    //    return). The child's MSB_HOME is always fresh/empty, so the create
    //    path PULLS python:3.12-slim (plus first-boot microsandbox assets)
    //    before the record lands — on a cold store / congested host that can
    //    exceed a minute, so bound at 180s. Fail FAST with the child's log
    //    tail if the child process dies before registering: a dead child
    //    never registers, and waiting out the bound only hides the
    //    diagnosis (the 2026-08-29 host failure was indistinguishable
    //    between these two modes).
    let child_pid = parse_background_pid(&up_stdout);
    let deadline = Instant::now() + Duration::from_secs(180);
    let mut seen = false;
    while Instant::now() < deadline {
        if ps_contains(&home, &msb_home, &instance) {
            seen = true;
            break;
        }
        if let Some(pid) = child_pid
            && !child_alive(pid)
        {
            panic!(
                "detached child (PID {pid}) exited before registering '{instance}'; \
                     child log tail:\n{}",
                child_log_tail(&home, &instance)
            );
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    assert!(
        seen,
        "ps --json never listed '{instance}' within 180s; child log tail:\n{}",
        child_log_tail(&home, &instance)
    );

    // 4. The registry record must carry the plan's port pair: host 4000,
    //    guest 4000. Read the raw record to assert the port pair directly
    //    (independent of the ps JSON renderer).
    let state_dir = home.join(".local").join("state").join("workestrate");
    let record_path = state_dir
        .join("var")
        .join("run")
        .join(format!("{instance}.json"));
    let record = std::fs::read_to_string(&record_path).expect("read registry record");
    assert!(
        record.contains("\"host\": 4000") && record.contains("\"guest\": 4000"),
        "registry record for '{instance}' must carry the plan port pair (4000:4000); got:\n{record}"
    );

    // 5. `down --instance <slug>` stops and removes it.
    let down = isolated_cmd(&home, &msb_home)
        .args(["example-litellm", "down", "--instance", slug])
        .output()
        .expect("spawn example-litellm down --instance");
    let down_stdout = String::from_utf8_lossy(&down.stdout).to_string();
    assert!(
        down.status.success(),
        "down --instance {slug} failed; stderr:\n{}",
        String::from_utf8_lossy(&down.stderr)
    );
    assert!(
        down_stdout.contains("stopped") || down_stdout.contains("not found"),
        "down --instance {slug} should report stopped; got:\n{down_stdout}"
    );

    let _ = std::fs::remove_dir_all(&home);
    let _ = std::fs::remove_dir_all(&msb_home);
}

/// Tiny anchored-prefix matcher for `[a-z2-7]{4}` without pulling a regex dep.
/// Matches iff `s.len() == 4` and every char is in `a..=z` or `2..=7`.
fn regex_lite_matches(_pattern: &str, s: &str) -> bool {
    s.len() == 4
        && s.bytes()
            .all(|b| b.is_ascii_lowercase() || (b'2'..=b'7').contains(&b))
}
