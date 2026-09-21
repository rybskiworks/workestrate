//! Integration tests for `workestrate secrets init|update` — the Rust port of
//! `scripts/test-setup-secrets.py`. Real sops/age crypto against disposable
//! operator configs and freshly generated keys; the secret value is asserted to
//! NEVER appear in stdout/stderr. Crypto-dependent tests skip (with a note)
//! when `sops`/`age-keygen` are missing from PATH or cannot complete a real
//! key generation, so tool-less or broken-tool environments stay green.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

mod common;

use common::{BIN, TempDir};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const INITIAL_VALUE: &str = "sk-test-initial-target-selection";
const UPDATED_VALUE: &str = "sk-test-updated-target-selection";

fn on_path(tool: &str) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| {
        let candidate = dir.join(tool);
        candidate.is_file()
    })
}

/// Presence on PATH is not enough: an `age-keygen` can spawn yet fail at
/// runtime (locked-down sandbox, missing entropy, broken binary). Probe a
/// real key generation plus recipient derivation in a throwaway directory so
/// a broken toolchain takes the skip path instead of panicking mid-test.
fn crypto_tools_available() -> bool {
    if !on_path("sops") || !on_path("age-keygen") {
        return false;
    }
    let probe_guard = TempDir::new("cmd-secrets-probe");
    let probe_key = probe_guard.path().join("probe.key");
    let generated = Command::new("age-keygen")
        .arg("-o")
        .arg(&probe_key)
        .output();
    let Ok(generated) = generated else {
        return false;
    };
    if !generated.status.success() {
        return false;
    }
    let derived = Command::new("age-keygen")
        .arg("-y")
        .arg(&probe_key)
        .output();
    match derived {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

/// Skip helper: returns true (and the caller returns early) when the crypto
/// toolchain is unavailable.
fn skip_without_crypto(test: &str) -> bool {
    if crypto_tools_available() {
        return false;
    }
    eprintln!("skipping {test}: sops/age-keygen unavailable or nonfunctional");
    true
}

/// Disposable fixture mirroring the python suite's setUp: an "operator config"
/// holding the registry + fleet clone, a separate "user home", unrelated XDG
/// dirs, and a fresh age key.
struct Fixture {
    #[allow(dead_code)]
    root_guard: TempDir,
    root: PathBuf,
    config: PathBuf,
    fleet: PathBuf,
    key: PathBuf,
}

impl Fixture {
    fn new() -> Option<Self> {
        if !crypto_tools_available() {
            return None;
        }
        let root_guard = TempDir::new("cmd-secrets");
        let root = root_guard.path().to_path_buf();
        let config = root.join("operator config");
        let fleet = config.join("fleets/personal");
        let key = root.join("private keys/age.txt");
        std::fs::create_dir_all(key.parent().unwrap()).expect("create key dir");
        let status = Command::new("age-keygen")
            .arg("-o")
            .arg(&key)
            .output()
            .expect("run age-keygen");
        assert!(
            status.status.success(),
            "age-keygen failed: {}",
            String::from_utf8_lossy(&status.stderr)
        );
        let fx = Self {
            root_guard,
            root,
            config,
            fleet,
            key,
        };
        let recipient = fx.recipient();
        fx.make_fleet(&fx.fleet.clone(), &recipient);
        fx.write_registry("", "");
        Some(fx)
    }

    fn recipient(&self) -> String {
        let out = Command::new("age-keygen")
            .arg("-y")
            .arg(&self.key)
            .output()
            .expect("run age-keygen -y");
        assert!(
            out.status.success(),
            "age-keygen -y failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap().trim().to_string()
    }

    fn make_fleet(&self, path: &Path, recipient: &str) {
        std::fs::create_dir_all(path).expect("create fleet dir");
        std::fs::write(
            path.join(".sops.yaml"),
            format!(
                "keys:\n  - &fixture {recipient}\ncreation_rules:\n  - path_regex: .*\\.enc$\n    key_groups:\n      - age:\n          - *fixture\n"
            ),
        )
        .expect("write .sops.yaml");
        std::fs::write(path.join(".env.example"), "LITELLM_MASTER_KEY=\n")
            .expect("write .env.example");
        std::fs::write(
            path.join("workestrate.toml"),
            "schema_version = 1\n[secrets.LITELLM_MASTER_KEY]\nenv_var = \"LITELLM_MASTER_KEY\"\nrequired = false\n",
        )
        .expect("write workestrate.toml");
    }

    fn write_registry(&self, settings: &str, overrides: &str) {
        std::fs::create_dir_all(&self.config).expect("create operator config");
        std::fs::write(
            self.config.join("config.toml"),
            format!("layers = [\"personal\"]\n{settings}\n[fleets.personal]\nurl = \"fleets/personal\"\n{overrides}\n"),
        )
        .expect("write registry");
    }

    /// Build a CLI invocation with the python suite's env: detached user
    /// home, unrelated XDG dirs, the test key, and the initial secret value
    /// in the process env (stdin is /dev/null → non-interactive).
    fn cmd(&self) -> Command {
        let mut c = Command::new(BIN);
        c.current_dir(&self.root);
        c.env("HOME", self.root.join("user home"));
        c.env("XDG_CONFIG_HOME", self.root.join("unrelated xdg config"));
        c.env("XDG_DATA_HOME", self.root.join("unrelated xdg data"));
        c.env("XDG_STATE_HOME", self.root.join("unrelated xdg state"));
        c.env("SOPS_AGE_KEY_FILE", &self.key);
        c.env("TMPDIR", &self.root);
        c.env("LITELLM_MASTER_KEY", INITIAL_VALUE);
        c.env("LC_ALL", "C");
        // Fail deterministically if a flow unexpectedly goes interactive.
        c.env("EDITOR", "false");
        c.env_remove("WORKESTRATE_CONFIG");
        c.env_remove("WORKESTRATE_FLEET_DIR");
        c.env_remove("WORKESTRATE_NO_PROJECT_CONFIG");
        c.env_remove("WORKESTRATE_REFERENCE_CONFIG");
        c.env_remove("WORKESTRATE_INVOKE_CWD");
        c.stdin(std::process::Stdio::null());
        c
    }

    /// Run the CLI, asserting success and — on every invocation — that the
    /// active secret values never leak into stdout/stderr.
    fn run_ok(&self, args: &[&str], extra_env: &[(&str, &str)]) -> Output {
        self.run(args, extra_env, true)
    }

    fn run(&self, args: &[&str], extra_env: &[(&str, &str)], expect_success: bool) -> Output {
        let mut c = self.cmd();
        c.args(args);
        for (k, v) in extra_env {
            c.env(k, v);
        }
        let out = c.output().expect("invoke workestrate secrets");
        let combined = format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        for value in [INITIAL_VALUE, UPDATED_VALUE] {
            assert!(
                !combined.contains(value),
                "secret value leaked into CLI output: {combined}"
            );
        }
        if expect_success {
            assert!(
                out.status.success(),
                "expected success for {args:?}; stderr: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        } else {
            assert!(
                !out.status.success(),
                "expected failure for {args:?}; stdout: {}",
                String::from_utf8_lossy(&out.stdout)
            );
        }
        out
    }

    /// Decrypt a secrets file with the real sops (walk-up .sops.yaml
    /// discovery from the file's directory, exactly as the loader does).
    fn decrypt(&self, path: &Path) -> String {
        let out = Command::new("sops")
            .args([
                "decrypt",
                "--input-type",
                "dotenv",
                "--output-type",
                "dotenv",
            ])
            .arg(path)
            .env("SOPS_AGE_KEY_FILE", &self.key)
            .env("HOME", self.root.join("user home"))
            .output()
            .expect("run sops decrypt");
        assert!(
            out.status.success(),
            "sops decrypt failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8(out.stdout).unwrap()
    }
}

/// 1. Named config: env-driven init + env-var targeted-replace update; the
///    unselected fleet's ciphertext stays untouched and no unrelated XDG dirs
///    are created.
#[test]
fn named_config_initializes_and_updates_only_selected_fleet() {
    if skip_without_crypto("named_config_initializes_and_updates_only_selected_fleet") {
        return;
    }
    let fx = Fixture::new().unwrap();
    let other = fx.config.join("fleets/other");
    fx.make_fleet(&other, &fx.recipient());
    std::fs::write(other.join(".env.enc"), "untouched ciphertext fixture").unwrap();

    let init_args = [
        "secrets",
        "init",
        "--config",
        fx.config.to_str().unwrap(),
        "--fleet",
        "personal",
    ];
    fx.run_ok(&init_args, &[]);
    assert!(
        fx.decrypt(&fx.fleet.join(".env.enc"))
            .contains(INITIAL_VALUE),
        "init must encrypt the env-provided value"
    );

    let update_args = [
        "secrets",
        "update",
        "--config",
        fx.config.to_str().unwrap(),
        "--fleet",
        "personal",
    ];
    let out = fx.run_ok(&update_args, &[("LITELLM_MASTER_KEY", UPDATED_VALUE)]);
    assert!(
        fx.decrypt(&fx.fleet.join(".env.enc"))
            .contains(UPDATED_VALUE)
    );
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !combined.contains(UPDATED_VALUE),
        "update must not print the value"
    );
    assert!(
        combined.contains("LITELLM_MASTER_KEY"),
        "update reports the replaced key NAMES: {combined}"
    );

    assert_eq!(
        std::fs::read_to_string(other.join(".env.enc")).unwrap(),
        "untouched ciphertext fixture"
    );
    assert!(!fx.root.join("unrelated xdg data").exists());
}

/// 2. Relative --config and equals-form options after the subcommand.
#[test]
fn relative_config_and_equals_options_after_command() {
    if skip_without_crypto("relative_home_and_equals_options_after_command") {
        return;
    }
    let fx = Fixture::new().unwrap();
    fx.run_ok(
        &[
            "secrets",
            "init",
            "--config=operator config",
            "--fleet=personal",
        ],
        &[],
    );
    assert!(fx.fleet.join(".env.enc").is_file());
}

/// 3. WORKESTRATE_CONFIG env + explicit --config beats WORKESTRATE_FLEET_DIR.
#[test]
fn config_environment_and_explicit_name_override_direct_environment() {
    if skip_without_crypto("home_environment_and_explicit_name_override_direct_environment") {
        return;
    }
    let fx = Fixture::new().unwrap();
    let other = fx.root.join("unselected fleet");
    fx.make_fleet(&other, &fx.recipient());
    fx.run_ok(
        &["secrets", "init", "--fleet", "personal"],
        &[
            ("WORKESTRATE_CONFIG", fx.config.to_str().unwrap()),
            ("WORKESTRATE_FLEET_DIR", other.to_str().unwrap()),
        ],
    );
    assert!(fx.fleet.join(".env.enc").is_file());
    assert!(!other.join(".env.enc").exists());
}

/// 4. The --config flag beats the WORKESTRATE_CONFIG env var.
#[test]
fn config_flag_overrides_config_environment() {
    if skip_without_crypto("home_flag_overrides_home_environment") {
        return;
    }
    let fx = Fixture::new().unwrap();
    let wrong = fx.root.join("wrong config");
    fx.run_ok(
        &[
            "secrets",
            "init",
            "--config",
            fx.config.to_str().unwrap(),
            "--fleet",
            "personal",
        ],
        &[("WORKESTRATE_CONFIG", wrong.to_str().unwrap())],
    );
    assert!(fx.fleet.join(".env.enc").is_file());
    assert!(!wrong.exists());
}

/// 5. Registry store_dir / secrets_file / age_key_file overrides win; a
///    wrong SOPS_AGE_KEY_FILE and SECRET_FILE env are ignored for the named
///    target.
#[test]
fn registered_store_file_and_key_overrides() {
    if skip_without_crypto("registered_store_file_and_key_overrides") {
        return;
    }
    let fx = Fixture::new().unwrap();
    let store = fx.root.join("custom \"store\"");
    let selected = store.join("fleets/personal");
    fx.make_fleet(&selected, &fx.recipient());
    let settings = format!(
        "[settings]\nstore_dir = \"{}\"\n",
        store.display().to_string().replace('"', "\\\"")
    );
    let overrides = format!(
        "secrets_file = \"custom secrets.enc\"\nage_key_file = \"{}\"",
        fx.key.display()
    );
    fx.write_registry(&settings, &overrides);
    let wrong_key = fx.root.join("wrong key");
    fx.run_ok(
        &[
            "secrets",
            "init",
            "--config",
            fx.config.to_str().unwrap(),
            "--fleet",
            "personal",
        ],
        &[
            ("SOPS_AGE_KEY_FILE", wrong_key.to_str().unwrap()),
            ("SECRET_FILE", "wrong-file.enc"),
        ],
    );
    assert!(
        fx.decrypt(&selected.join("custom secrets.enc"))
            .contains(INITIAL_VALUE),
        "registry secrets_file override must be used"
    );
    assert!(!selected.join("wrong-file.enc").exists());
    assert!(!fx.fleet.join(".env.enc").exists());
}

/// 6. --fleet-dir with spaces, quotes, and shell metacharacters in the
///    name: pure path handling, no injection, file lands in the right place.
#[test]
fn direct_directory_handles_relative_paths_and_shell_metacharacters() {
    if skip_without_crypto("direct_directory_handles_relative_paths_and_shell_metacharacters") {
        return;
    }
    let fx = Fixture::new().unwrap();
    let name = "fleet with spaces \"quoted\" $(touch INJECTED)";
    let selected = fx.root.join(name);
    fx.make_fleet(&selected, &fx.recipient());
    fx.run_ok(
        &["secrets", "init", "--fleet-dir", name],
        &[("WORKESTRATE_FLEET_DIR", fx.fleet.to_str().unwrap())],
    );
    assert!(
        fx.decrypt(&selected.join(".env.enc"))
            .contains(INITIAL_VALUE)
    );
    assert!(!fx.fleet.join(".env.enc").exists());
    assert!(
        !fx.root.join("INJECTED").exists(),
        "shell metacharacters must never be evaluated"
    );
}

/// 7. A relative registry age_key_file keeps its invocation-cwd meaning.
#[test]
fn relative_registry_key_is_resolved_against_invocation_cwd() {
    if skip_without_crypto("relative_registry_key_is_resolved_against_invocation_cwd") {
        return;
    }
    let fx = Fixture::new().unwrap();
    fx.write_registry("", "age_key_file = \"private keys/age.txt\"");
    fx.run_ok(
        &[
            "secrets",
            "init",
            "--config",
            fx.config.to_str().unwrap(),
            "--fleet",
            "personal",
        ],
        &[],
    );
    assert!(
        fx.decrypt(&fx.fleet.join(".env.enc"))
            .contains(INITIAL_VALUE)
    );
}

/// 8. --fleet-dir=<path> equals form.
#[test]
fn direct_directory_equals_form() {
    if skip_without_crypto("direct_directory_equals_form") {
        return;
    }
    let fx = Fixture::new().unwrap();
    let arg = format!("--fleet-dir={}", fx.fleet.display());
    fx.run_ok(&["secrets", "init", &arg], &[]);
    assert!(fx.fleet.join(".env.enc").exists());
}

/// 9. An unknown fleet name is a hard error (no environment fallback) and
///    nothing is written.
#[test]
fn unknown_name_does_not_fall_back_to_environment() {
    if skip_without_crypto("unknown_name_does_not_fall_back_to_environment") {
        return;
    }
    let fx = Fixture::new().unwrap();
    let out = fx.run(
        &[
            "secrets",
            "init",
            "--config",
            fx.config.to_str().unwrap(),
            "--fleet",
            "missing",
        ],
        &[("WORKESTRATE_FLEET_DIR", fx.fleet.to_str().unwrap())],
        false,
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("could not resolve fleet 'missing'"),
        "expected could-not-resolve-fleet error, got: {stderr}"
    );
    assert!(!fx.fleet.join(".env.enc").exists());
}

/// 10. A missing --fleet-dir directory on update errors and is NOT created.
#[test]
fn missing_directory_is_not_created() {
    if skip_without_crypto("missing_directory_is_not_created") {
        return;
    }
    let fx = Fixture::new().unwrap();
    let missing = fx.root.join("missing directory");
    let out = fx.run(
        &[
            "secrets",
            "update",
            "--fleet-dir",
            missing.to_str().unwrap(),
        ],
        &[],
        false,
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("target fleet directory does not exist"),
        "expected missing-directory error, got: {stderr}"
    );
    assert!(!missing.exists());
}

/// 11. Invalid argument combinations fail (clap or resolution) before any
///     writes. No crypto tools required.
#[test]
fn invalid_arguments_fail_before_writes() {
    let root_guard = TempDir::new("cmd-secrets-invalid");
    let root = root_guard.path();
    let config_dir = root.join("operator config");
    let fleet = config_dir.join("fleets/personal");
    std::fs::create_dir_all(&fleet).unwrap();

    let cases: Vec<Vec<String>> = vec![
        vec!["--config".into()],
        vec!["--config=".into()],
        vec!["--fleet-dir".into()],
        vec!["--fleet-dir=".into()],
        vec!["--fleet".into(), "--global".into()],
        vec!["--fleet".into(), "personal".into(), "--global".into()],
        vec![
            "--fleet-dir".into(),
            fleet.display().to_string(),
            "--global".into(),
        ],
        vec![
            "--fleet-dir".into(),
            fleet.display().to_string(),
            "--fleet".into(),
            "personal".into(),
        ],
        vec!["--unknown".into()],
        vec!["init".into(), "update".into()],
        vec!["typo".into()],
    ];
    for verb in ["init", "update"] {
        for mut case in cases.clone() {
            let mut args: Vec<String> = vec!["secrets".into(), verb.into()];
            args.append(&mut case);
            let out = Command::new(BIN)
                .args(&args)
                .current_dir(root)
                .env("HOME", root.join("user home"))
                .env("XDG_CONFIG_HOME", root.join("unrelated xdg config"))
                .env("LC_ALL", "C")
                .env_remove("WORKESTRATE_CONFIG")
                .env_remove("WORKESTRATE_FLEET_DIR")
                .stdin(std::process::Stdio::null())
                .output()
                .expect("invoke workestrate secrets");
            assert!(
                !out.status.success(),
                "{args:?} must fail; stdout: {}",
                String::from_utf8_lossy(&out.stdout)
            );
            assert!(
                !fleet.join(".env.enc").exists(),
                "{args:?} must not write the secrets file"
            );
        }
    }
}

/// 12. Global mode: --global init writes .env.local.enc under the scratch
///     XDG_CONFIG_HOME/workestrate and round-trips an update.
#[test]
fn global_mode_roundtrip() {
    if skip_without_crypto("global_mode_roundtrip") {
        return;
    }
    let fx = Fixture::new().unwrap();
    let global_dir = fx.root.join("unrelated xdg config").join("workestrate");
    // Global mode still verifies the target dir's .sops.yaml recipient —
    // seed one with the placeholder for init to substitute.
    fx.make_fleet(&global_dir, &fx.recipient());
    std::fs::write(
        global_dir.join(".sops.yaml"),
        "keys:\n  - &fixture age1PLACEHOLDER0000000000000000000000000000000000000000000000000000\ncreation_rules:\n  - path_regex: .*\\.enc$\n    key_groups:\n      - age:\n          - *fixture\n",
    )
    .unwrap();

    fx.run_ok(&["secrets", "init", "--global"], &[]);
    let secret = global_dir.join(".env.local.enc");
    assert!(secret.is_file(), "global init must write .env.local.enc");
    assert!(fx.decrypt(&secret).contains(INITIAL_VALUE));
    assert!(
        std::fs::read_to_string(global_dir.join(".sops.yaml"))
            .unwrap()
            .contains(&fx.recipient()),
        "placeholder recipient must be substituted"
    );

    fx.run_ok(
        &["secrets", "update", "--global"],
        &[("LITELLM_MASTER_KEY", UPDATED_VALUE)],
    );
    assert!(fx.decrypt(&secret).contains(UPDATED_VALUE));
}

/// 13. The produced secrets file is mode 0600 and no temp files are left
///     behind in the target dir or TMPDIR.
#[test]
fn secret_file_permissions_and_no_temp_leftovers() {
    if skip_without_crypto("secret_file_permissions_and_no_temp_leftovers") {
        return;
    }
    let fx = Fixture::new().unwrap();
    fx.run_ok(
        &[
            "secrets",
            "init",
            "--config",
            fx.config.to_str().unwrap(),
            "--fleet",
            "personal",
        ],
        &[],
    );
    fx.run_ok(
        &[
            "secrets",
            "update",
            "--config",
            fx.config.to_str().unwrap(),
            "--fleet",
            "personal",
        ],
        &[("LITELLM_MASTER_KEY", UPDATED_VALUE)],
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(fx.fleet.join(".env.enc"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600, ".env.enc must be mode 0600, got {mode:o}");
    }

    // Target dir: only the expected artifacts, no *.tmp leftovers.
    let mut entries: Vec<String> = std::fs::read_dir(&fx.fleet)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    entries.sort();
    assert_eq!(
        entries,
        vec![".env.enc", ".env.example", ".sops.yaml", "workestrate.toml"],
        "unexpected files in the target dir"
    );

    // TMPDIR (the scratch root): no leftover provisioning temp files.
    let strays: Vec<String> = std::fs::read_dir(&fx.root)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains("workestrate-secrets-") || name.ends_with(".tmp"))
        .collect();
    assert!(strays.is_empty(), "temp files left behind: {strays:?}");
}
