//! Integration tests for `workestrate context list` and `workestrate context
//! current` — context enumeration, default marking, and resolution-source
//! classification (env / default / bare). Uses an isolated HOME + XDG per
//! test so the user's real registry is never touched.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::IsolatedHome;
/// Parse a top-level string field out of a small flat JSON object. (Avoids
/// pulling serde_json into the test binary.)
fn json_field<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{}\"", key);
    let idx = json.find(&pat)?;
    let rest = &json[idx + pat.len()..];
    let colon = rest.find(':')?;
    let after = rest[colon + 1..].trim_start();
    if let Some(inner) = after.strip_prefix('"') {
        let end = inner.find('"')?;
        Some(&inner[..end])
    } else {
        let end = after.find([',', '}', '\n']).unwrap_or(after.len());
        Some(after[..end].trim())
    }
}

/// With no registry at all, `context list` reports empty.
#[test]
fn context_list_no_registry() {
    let home = IsolatedHome::new("cmd-context");

    // Human output
    let out = home
        .cmd()
        .args(["context", "list"])
        .output()
        .expect("invoke context list");
    assert!(
        out.status.success(),
        "context list failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("(no registry found)"),
        "expected no-registry notice; got: {}",
        stdout
    );

    // JSON output
    let out = home
        .cmd()
        .args(["context", "list", "--json"])
        .output()
        .expect("invoke context list --json");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"contexts\": []"),
        "expected empty contexts array; got: {}",
        stdout
    );
}

/// Defined contexts are listed with their layers and the default marked.
#[test]
fn context_list_shows_contexts_and_default() {
    let home = IsolatedHome::new("cmd-context");
    home.write_registry(
        r#"
[settings]
default_context = "personal"

[contexts.personal]
layers = ["personal", "team"]

[contexts.work]
layers = ["work"]
"#,
    );

    // Human
    let out = home
        .cmd()
        .args(["context", "list"])
        .output()
        .expect("invoke context list");
    assert!(
        out.status.success(),
        "context list failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Contexts:"),
        "missing header; got: {}",
        stdout
    );
    assert!(
        stdout.contains("personal (default)"),
        "personal should be marked default; got: {}",
        stdout
    );
    assert!(
        stdout.contains("layers: personal, team"),
        "personal layers; got: {}",
        stdout
    );
    assert!(stdout.contains("work"), "work context; got: {}", stdout);

    // JSON
    let out = home
        .cmd()
        .args(["context", "list", "--json"])
        .output()
        .expect("invoke context list --json");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"name\": \"personal\""),
        "missing personal entry; got: {}",
        stdout
    );
    assert!(
        stdout.contains("\"is_default\": true"),
        "personal should be default; got: {}",
        stdout
    );
    assert!(
        stdout.contains("\"name\": \"work\""),
        "missing work entry; got: {}",
        stdout
    );
}

/// WORKESTRATE_CONTEXT env (or --context flag) classifies the source as "env".
#[test]
fn context_current_env_source() {
    let home = IsolatedHome::new("cmd-context");
    home.write_registry(
        r#"
[settings]
default_context = "work"

[contexts.personal]
layers = ["personal", "team"]

[contexts.work]
layers = ["work"]
"#,
    );

    let out = home
        .cmd()
        .env("WORKESTRATE_CONTEXT", "personal")
        .args(["context", "current", "--json"])
        .output()
        .expect("invoke context current --json");
    assert!(
        out.status.success(),
        "context current failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        json_field(&stdout, "name"),
        Some("personal"),
        "name mismatch; got: {}",
        stdout
    );
    assert_eq!(
        json_field(&stdout, "source"),
        Some("env"),
        "source mismatch; got: {}",
        stdout
    );
}

/// With a default_context set and no env, source is "default".
#[test]
fn context_current_default_source() {
    let home = IsolatedHome::new("cmd-context");
    home.write_registry(
        r#"
[settings]
default_context = "work"

[contexts.work]
layers = ["work"]
"#,
    );

    let out = home
        .cmd()
        .args(["context", "current", "--json"])
        .output()
        .expect("invoke context current --json");
    assert!(
        out.status.success(),
        "context current failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        json_field(&stdout, "name"),
        Some("work"),
        "name mismatch; got: {}",
        stdout
    );
    assert_eq!(
        json_field(&stdout, "source"),
        Some("default"),
        "source mismatch; got: {}",
        stdout
    );
}

/// With no contexts defined (bare-layers backward-compat), name is null and
/// source is "bare".
#[test]
fn context_current_bare_source() {
    let home = IsolatedHome::new("cmd-context");
    home.write_registry(
        r#"
layers = []

[fleets.personal]
url = "https://example.com/repo.git"
ref = "main"
"#,
    );

    let out = home
        .cmd()
        .args(["context", "current", "--json"])
        .output()
        .expect("invoke context current --json");
    assert!(
        out.status.success(),
        "context current failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        json_field(&stdout, "name"),
        Some("null"),
        "name should be null; got: {}",
        stdout
    );
    assert_eq!(
        json_field(&stdout, "source"),
        Some("bare"),
        "source mismatch; got: {}",
        stdout
    );

    // Human form mentions bare layers.
    let out = home
        .cmd()
        .args(["context", "current"])
        .output()
        .expect("invoke context current");
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Active context: (none — using bare layers)"),
        "expected bare notice; got: {}",
        stdout
    );
    assert!(
        stdout.contains("source: bare"),
        "expected bare source; got: {}",
        stdout
    );
}
