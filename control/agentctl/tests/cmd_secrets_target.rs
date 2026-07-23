//! Integration tests for `workestrate secrets-target` — registry resolution,
//! per-repo overrides, unregistered names, and exists detection. Uses an
//! isolated HOME + XDG_CONFIG_HOME per test so the user's real workestrate
//! registry is never touched.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_workestrate");

/// Isolated sandbox: creates a fresh HOME under std::env::temp_dir() and
/// returns it along with the env vars to set. Drop is the caller's job
/// (these tests don't need cleanup — the temp_dir is process-id-namespaced
/// and the OS reaps it eventually).
struct IsolatedHome {
    dir: PathBuf,
}

impl IsolatedHome {
    fn new() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "workestrate-cmd-secrets-target-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("create isolated HOME");
        std::fs::create_dir_all(dir.join(".config")).expect("create .config");
        std::fs::create_dir_all(dir.join(".local").join("share")).expect("create .local/share");
        Self { dir }
    }

    /// Build a Command with HOME / XDG pointed at this isolated root.
    fn cmd(&self) -> Command {
        let mut c = Command::new(BIN);
        c.env("HOME", &self.dir);
        c.env("XDG_CONFIG_HOME", self.dir.join(".config"));
        c.env("XDG_DATA_HOME", self.dir.join(".local").join("share"));
        c.env_remove("WORKESTRATE_CONFIG_DIR");
        c.env_remove("WORKESTRATE_NO_PROJECT_CONFIG");
        c.env_remove("SOPS_AGE_KEY_FILE");
        c
    }

    /// Path to the registry file (`$XDG_CONFIG_HOME/workestrate/config.toml`).
    fn registry_path(&self) -> PathBuf {
        self.dir
            .join(".config")
            .join("workestrate")
            .join("config.toml")
    }

    /// Path to the store dir (`$XDG_DATA_HOME/workestrate`).
    fn store_dir(&self) -> PathBuf {
        self.dir.join(".local").join("share").join("workestrate")
    }

    /// Write a registry TOML with a single `[configs.<name>]` entry.
    fn write_registry(&self, name: &str, extra_lines: &str) {
        let workestrate_dir = self.dir.join(".config").join("workestrate");
        std::fs::create_dir_all(&workestrate_dir).expect("create workestrate config dir");
        let content = format!(
            "[configs.{name}]\nurl = \"https://example.com/repo.git\"\nref = \"main\"\n{extra_lines}"
        );
        std::fs::write(self.registry_path(), content).expect("write registry");
    }

    /// Create the repo checkout dir in the store so `dir` exists.
    fn create_repo_dir(&self, name: &str) -> PathBuf {
        let dir = self.store_dir().join("repos").join(name);
        std::fs::create_dir_all(&dir).expect("create repo dir");
        dir
    }
}

/// Parse a top-level string field out of a small flat JSON object.
/// (Avoids pulling serde_json into the test binary; the CLI output shape is
/// fixed and flat.)
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

/// A registered repo with no overrides resolves to the default secrets_file
/// (".env.enc"), the default age key path, and exists=false when the file
/// has not been written yet.
#[test]
fn secrets_target_defaults_for_registered_repo() {
    let home = IsolatedHome::new();
    home.write_registry("personal", "");
    let repo_dir = home.create_repo_dir("personal");

    let out = home
        .cmd()
        .args(["secrets-target", "personal", "--json"])
        .output()
        .expect("invoke secrets-target");
    assert!(
        out.status.success(),
        "secrets-target failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        json_field(&stdout, "dir"),
        Some(repo_dir.to_string_lossy().as_ref()),
        "dir mismatch; stdout: {}",
        stdout
    );
    assert_eq!(
        json_field(&stdout, "secrets_file"),
        Some(".env.enc"),
        "secrets_file mismatch; stdout: {}",
        stdout
    );
    let expected_age_key = home
        .dir
        .join(".config")
        .join("sops")
        .join("age")
        .join("ai-workbench-secrets.txt");
    assert_eq!(
        json_field(&stdout, "age_key_file"),
        Some(expected_age_key.to_string_lossy().as_ref()),
        "age_key_file mismatch; stdout: {}",
        stdout
    );
    assert_eq!(
        json_field(&stdout, "exists"),
        Some("false"),
        "exists mismatch; stdout: {}",
        stdout
    );
}

/// Per-repo secrets_file / age_key_file overrides are reflected verbatim in
/// the JSON output.
#[test]
fn secrets_target_honors_per_repo_overrides() {
    let home = IsolatedHome::new();
    home.write_registry(
        "personal",
        "secrets_file = \".env.custom.enc\"\nage_key_file = \"/custom/key/path\"\n",
    );
    let repo_dir = home.create_repo_dir("personal");

    let out = home
        .cmd()
        .args(["secrets-target", "personal", "--json"])
        .output()
        .expect("invoke secrets-target");
    assert!(
        out.status.success(),
        "secrets-target failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);

    assert_eq!(
        json_field(&stdout, "dir"),
        Some(repo_dir.to_string_lossy().as_ref()),
        "dir mismatch; stdout: {}",
        stdout
    );
    assert_eq!(
        json_field(&stdout, "secrets_file"),
        Some(".env.custom.enc"),
        "secrets_file mismatch; stdout: {}",
        stdout
    );
    assert_eq!(
        json_field(&stdout, "age_key_file"),
        Some("/custom/key/path"),
        "age_key_file mismatch; stdout: {}",
        stdout
    );
    assert_eq!(
        json_field(&stdout, "exists"),
        Some("false"),
        "exists mismatch; stdout: {}",
        stdout
    );
}

/// A name that is not in the registry produces a non-zero exit and an
/// explanatory error on stderr.
#[test]
fn secrets_target_rejects_unregistered_name() {
    let home = IsolatedHome::new();
    home.write_registry("personal", "");

    let out = home
        .cmd()
        .args(["secrets-target", "ghost", "--json"])
        .output()
        .expect("invoke secrets-target");
    assert!(
        !out.status.success(),
        "unregistered name should fail; got success"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("config repo 'ghost' not registered"),
        "expected not-registered error, got: {}",
        stderr
    );
}

/// When the resolved secrets file exists on disk, exists=true.
#[test]
fn secrets_target_reports_exists_true_when_file_present() {
    let home = IsolatedHome::new();
    home.write_registry("personal", "");
    let repo_dir = home.create_repo_dir("personal");
    std::fs::write(repo_dir.join(".env.enc"), "sops-encrypted-placeholder").expect("seed .env.enc");

    let out = home
        .cmd()
        .args(["secrets-target", "personal", "--json"])
        .output()
        .expect("invoke secrets-target");
    assert!(
        out.status.success(),
        "secrets-target failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        json_field(&stdout, "exists"),
        Some("true"),
        "exists mismatch; stdout: {}",
        stdout
    );
}
