//! `workestrate secrets init|update` — CLI-native secrets provisioning
//! (SOPS + age), ported from the retired `scripts/setup-secrets.sh` engine.
//!
//! Target resolution precedence (exactly one of):
//! 1. `--fleet <name>` — registry-backed resolution (shared with
//!    `secrets target`; per-repo `secrets_file`/`age_key_file` overrides
//!    honored, tilde-expanded; a RELATIVE registry `age_key_file` keeps its
//!    invocation-cwd meaning). Unknown name = hard error, no fallback.
//! 2. `--fleet-dir <dir>` — the directory itself (must exist; never
//!    created; relative paths resolve against the invocation cwd via pure
//!    Rust path handling — no shell anywhere in this module).
//! 3. `--global` — `${XDG_CONFIG_HOME:-$HOME/.config}/workestrate` (created
//!    when missing), secrets file `.env.local.enc`.
//! 4. `WORKESTRATE_FLEET_DIR` env, when set.
//! 5. Auto-detect: exactly one registered fleet → resolve as `--fleet`.
//! 6. Fallback: a `.sops.yaml` in the invocation cwd → the cwd itself.
//!
//! After resolution the command points `WORKESTRATE_FLEET_DIR` at the
//! target dir so `config::load_config()` (required-keys schema) and the
//! env-example generator reflect the TARGET fleet's workestrate.toml — the
//! in-process equivalent of the script's `cd $TARGET_DIR; export
//! WORKESTRATE_FLEET_DIR=$PWD`.
//!
//! Secret values are never accepted via argv and never printed: they move
//! through process env, stdin, a mode-0600 temp buffer, and sops stdin/argv.

use std::io::{IsTerminal as _, Read as _};
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::cli_actions::SecretsTargetArgs;
use crate::commands::secrets_target::{derive_age_recipient, resolve_registered_target};
use crate::config;
use crate::scaffold;

/// Sentinel line written into interactive editor buffers. The user must
/// delete this line to confirm they saved their changes; if it survives an
/// unchanged editor round-trip the buffer is treated as un-saved. The exact
/// text is load-bearing: operator runbooks and the retired script's tests
/// match it.
const SENTINEL_LINE: &str = "# setup-secrets: delete this line to confirm you saved your changes";

const MAX_EDITOR_OPENS: usize = 5;
const MAX_VALIDATION_ATTEMPTS: usize = 3;

/// A fully resolved provisioning target.
struct TargetSpec {
    /// Directory holding `.sops.yaml` and the secrets file.
    dir: PathBuf,
    /// Secrets file NAME within `dir` (`.env.enc`, a registry override, or
    /// `.env.local.enc` in global mode).
    secret_file: String,
    /// Absolute age private key path.
    age_key_file: PathBuf,
}

impl TargetSpec {
    fn secret_path(&self) -> PathBuf {
        self.dir.join(&self.secret_file)
    }

    fn sops_config_path(&self) -> PathBuf {
        self.dir.join(".sops.yaml")
    }
}

fn log(msg: &str) {
    eprintln!("{msg}");
}

/// The default age-key chain (mirrors `secrets_loader::decrypt_layer`):
/// `SOPS_AGE_KEY_FILE` env, else `$HOME` + `scaffold::AGE_KEY_DEFAULT_PATH`
/// with the `~/` prefix stripped.
fn default_age_key_file() -> PathBuf {
    if let Ok(env_key) = std::env::var("SOPS_AGE_KEY_FILE") {
        return PathBuf::from(env_key);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let rel = scaffold::AGE_KEY_DEFAULT_PATH
        .strip_prefix("~/")
        .unwrap_or(scaffold::AGE_KEY_DEFAULT_PATH);
    PathBuf::from(home).join(rel)
}

/// Absolutize `path` against the invocation cwd when relative. Pure path
/// handling — dir names containing spaces, quotes, or `$(...)` are inert
/// data here (there is no shell in this code path).
fn absolutize(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    Ok(config::invoke_cwd_or_err()?.join(path))
}

/// Resolve the provisioning target from the CLI selector (or the
/// environment/cwd fallback ladder). Global mode creates its directory;
/// every other mode requires an existing directory.
fn resolve_target(args: &SecretsTargetArgs) -> Result<TargetSpec> {
    // The global --fleet shares its clap arg id with the --fleet selector
    // here (deliberately: it is the same fleet selector wherever it
    // appears), but a value given BEFORE the subcommand bypasses the
    // parse-time conflicts — enforce exclusivity explicitly so a mixed
    // invocation can never silently pick the wrong secrets target.
    let selector_count =
        u8::from(args.global) + u8::from(args.fleet.is_some()) + u8::from(args.fleet_dir.is_some());
    if selector_count > 1 {
        anyhow::bail!(
            "secrets target selectors are mutually exclusive — give exactly one of \
             --fleet <name> | --fleet-dir <dir> | --global"
        );
    }
    if args.global {
        let dir = config::paths::xdg_config_dir();
        if !dir.is_dir() {
            std::fs::create_dir_all(&dir)?;
        }
        return Ok(TargetSpec {
            dir,
            secret_file: ".env.local.enc".to_string(),
            age_key_file: default_age_key_file(),
        });
    }

    if let Some(ref name) = args.fleet {
        return resolve_named_target(name);
    }

    if let Some(ref dir) = args.fleet_dir {
        return direct_dir_target(dir);
    }

    if let Ok(dir) = std::env::var("WORKESTRATE_FLEET_DIR")
        && !dir.is_empty()
    {
        return direct_dir_target(Path::new(&dir));
    }

    // Auto-detect: exactly one registered fleet resolves as --fleet.
    // A registry load failure falls through to the cwd `.sops.yaml`
    // fallback below instead of erroring (portable directories without a
    // registry stay usable).
    if let Ok(Some(registry)) = config::load_registry()
        && registry.fleets.len() == 1
        && let Some(name) = registry.fleets.keys().next().cloned()
    {
        return resolve_named_target(&name);
    }

    // Fallback: a .sops.yaml in the invocation cwd targets the cwd.
    let cwd = config::invoke_cwd_or_err()?;
    if cwd.join(".sops.yaml").is_file() {
        return Ok(TargetSpec {
            dir: cwd,
            secret_file: ".env.enc".to_string(),
            age_key_file: default_age_key_file(),
        });
    }

    anyhow::bail!("could not locate repo root (.sops.yaml not found)")
}

/// `--fleet <name>`: registry-backed resolution. An unknown name is a hard
/// error — never fall back to a guessed location. A RELATIVE registry
/// `age_key_file` keeps its invocation-cwd meaning (the script resolved it
/// against `$PWD` before its `cd`; there is no chdir here, so we anchor it
/// to the captured invocation cwd explicitly).
fn resolve_named_target(name: &str) -> Result<TargetSpec> {
    let target = resolve_registered_target(name)?.ok_or_else(|| {
        anyhow::anyhow!(
            "could not resolve fleet '{name}'; check --fleet and the registered fleet name"
        )
    })?;
    let dir = absolutize(&target.dir)?;
    let age_key_file = absolutize(&target.age_key_file)?;
    Ok(TargetSpec {
        dir,
        secret_file: target.secrets_file,
        age_key_file,
    })
}

fn direct_dir_target(dir: &Path) -> Result<TargetSpec> {
    Ok(TargetSpec {
        dir: absolutize(dir)?,
        secret_file: ".env.enc".to_string(),
        age_key_file: absolutize(&default_age_key_file())?,
    })
}

/// Point `WORKESTRATE_FLEET_DIR` at the resolved target dir so the
/// config-driven steps below (required-keys schema, env-example generation)
/// see the TARGET fleet's workestrate.toml.
#[allow(unsafe_code)]
fn export_target_config_dir(dir: &Path) {
    // SAFETY: command-entry write-once override, set before any config load
    // in this flow and before any spawned tasks mutate env; no concurrent
    // mutation of this key (same pattern as the main.rs startup overrides).
    unsafe { std::env::set_var("WORKESTRATE_FLEET_DIR", dir) };
}

/// The required key set: the config `secrets` env_var names (same source as
/// `cmd_secrets_schema`), sorted. When the config fails to load, fall back
/// to parsing the target dir's `.env.example` (parity with the script's
/// grep pipeline): `KEY=` assignment names, excluding `AI_WORKBENCH_*_DIR`,
/// sorted unique.
fn required_keys(target_dir: &Path) -> Vec<String> {
    if let Ok(cfg) = config::load_config() {
        let mut names: Vec<String> = cfg
            .secrets
            .values()
            .filter_map(|s| s.env_var.clone())
            .collect();
        names.sort();
        names.dedup();
        return names;
    }
    parse_env_example_keys(&target_dir.join(".env.example"))
}

/// Parse assignment keys out of a dotenv-style file (the `.env.example`
/// fallback). Missing file → empty set (the caller's validation then has
/// nothing to enforce, matching the script's empty-grep outcome).
fn parse_env_example_keys(path: &Path) -> Vec<String> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut keys: Vec<String> = content
        .lines()
        .filter_map(assignment_key)
        .filter(|k| !(k.starts_with("AI_WORKBENCH_") && k.ends_with("_DIR")))
        .map(str::to_string)
        .collect();
    keys.sort();
    keys.dedup();
    keys
}

/// If `line` is a `KEY=...` assignment with a well-formed key, return the
/// key. Keys match `^[A-Za-z_][A-Za-z0-9_]*=` exactly (no surrounding
/// whitespace, no `export` handling — the buffers this parses never carry
/// it; the validator's looser strip is separate).
fn assignment_key(line: &str) -> Option<&str> {
    let eq = line.find('=')?;
    let key = &line[..eq];
    if key.is_empty() {
        return None;
    }
    let mut chars = key.chars();
    let first = chars.next()?;
    if !(first.is_ascii_alphabetic() || first == '_') {
        return None;
    }
    if !chars.all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return None;
    }
    Some(key)
}

/// The env-example content used to pre-fill editor buffers: the in-process
/// `generate-env-example` generator when the config loads, else the target
/// dir's `.env.example`. Returns (content, label-for-header).
fn env_example_source(target_dir: &Path) -> Result<(String, String)> {
    if let Ok(content) = crate::commands::diagnostics::generate_env_example_content() {
        return Ok((content, "workestrate generate-env-example".to_string()));
    }
    let path = target_dir.join(".env.example");
    let content = std::fs::read_to_string(&path).map_err(|e| {
        anyhow::anyhow!(
            "config load failed and {} is unreadable ({}); no env-example source",
            path.display(),
            e
        )
    })?;
    Ok((content, ".env.example".to_string()))
}

// ---------------------------------------------------------------------------
// External tools
// ---------------------------------------------------------------------------

/// Locate an executable on PATH (no shell).
fn which(tool: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(tool);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    // A bare `tool` may also be an absolute/relative path itself.
    let direct = Path::new(tool);
    if direct.components().count() > 1 && is_executable(direct) {
        return Some(direct.to_path_buf());
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn require_tools() -> Result<()> {
    if which("age-keygen").is_none() {
        anyhow::bail!("age-keygen not found; run inside 'just shell'");
    }
    if which("sops").is_none() {
        anyhow::bail!("sops not found; run inside 'just shell'");
    }
    Ok(())
}

#[cfg(unix)]
fn chmod(path: &Path, mode: u32) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))?;
    Ok(())
}

#[cfg(not(unix))]
fn chmod(_path: &Path, _mode: u32) -> Result<()> {
    Ok(())
}

// ---------------------------------------------------------------------------
// Age key bootstrap and .sops.yaml recipient maintenance
// ---------------------------------------------------------------------------

/// init bootstrap: generate the age key when missing (mode-0700 parent,
/// mode-0600 key file).
fn ensure_key(target: &TargetSpec) -> Result<()> {
    let key = &target.age_key_file;
    if key.is_file() {
        log(&format!("using existing age key: {}", key.display()));
        return Ok(());
    }
    log(&format!("generating age key: {}", key.display()));
    if let Some(parent) = key.parent() {
        std::fs::create_dir_all(parent)?;
        chmod(parent, 0o700)?;
    }
    let status = std::process::Command::new("age-keygen")
        .arg("-o")
        .arg(key)
        .status()
        .map_err(|e| anyhow::anyhow!("age-keygen binary not found: {e}"))?;
    if !status.success() {
        anyhow::bail!("age-keygen -o {} failed", key.display());
    }
    chmod(key, 0o600)?;
    Ok(())
}

/// Verify (or repair) the target dir's `.sops.yaml` recipient:
/// already-correct anchor entry → fine; `age1PLACEHOLDER…` → substitute;
/// anything else → hard error (manual update required).
fn update_sops_config(target: &TargetSpec) -> Result<()> {
    let pub_key = derive_age_recipient(&target.age_key_file)?;
    let path = target.sops_config_path();
    let content = std::fs::read_to_string(&path).map_err(|_| {
        anyhow::anyhow!(
            "{} not found; create it (e.g. 'workestrate fleet new <name>') before provisioning secrets",
            path.display()
        )
    })?;

    // Anchor entry: `- &anchor <pub>` (the script's grep -E pattern).
    if content.lines().any(|line| is_anchor_entry(line, &pub_key)) {
        log(".sops.yaml already contains the correct public key");
        return Ok(());
    }

    if content.contains("age1PLACEHOLDER") {
        log("updating .sops.yaml with public key");
        // Replace every placeholder token (recipient lists repeat the
        // token per rule); token = `age1PLACEHOLDER` + non-whitespace tail.
        let mut updated = String::with_capacity(content.len() + 64);
        let mut rest = content.as_str();
        while let Some(start) = rest.find("age1PLACEHOLDER") {
            updated.push_str(&rest[..start]);
            updated.push_str(&pub_key);
            let tail = &rest[start..];
            let token_len = tail.find(char::is_whitespace).unwrap_or(tail.len());
            rest = &tail[token_len..];
        }
        updated.push_str(rest);
        std::fs::write(&path, updated)?;
        return Ok(());
    }

    anyhow::bail!(
        "{} contains a different public key; update it manually",
        path.display()
    )
}

/// Match the script's `^[[:space:]]*- &[[:alnum:]_]+ <pub>$` anchor-entry
/// pattern.
fn is_anchor_entry(line: &str, pub_key: &str) -> bool {
    let trimmed = line.trim_start();
    let Some(rest) = trimmed.strip_prefix("- &") else {
        return false;
    };
    let Some((anchor, candidate)) = rest.split_once(' ') else {
        return false;
    };
    !anchor.is_empty()
        && anchor
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        && candidate == pub_key
}

// ---------------------------------------------------------------------------
// Dotenv buffer construction and validation
// ---------------------------------------------------------------------------

/// Pre-filled init buffer: header comments, the required-keys list, then
/// the env-example content walked line-by-line (comments/blanks verbatim,
/// `KEY=value` → `KEY=`, unparsable → `# <line>`), and the sentinel.
fn build_prefilled_buffer(
    secret_file: &str,
    required: &[String],
    example_source: &str,
    example_label: &str,
) -> String {
    let mut buf = String::new();
    buf.push_str(&format!(
        "# ai-workbench secrets (will be encrypted to {secret_file} via sops).\n"
    ));
    buf.push_str("# Lines starting with '#' are ignored by sops and serve as instructions only.\n");
    buf.push_str(&format!("# Required keys (from {example_label}):\n"));
    for key in required {
        buf.push_str(&format!("#   - {key}\n"));
    }
    buf.push_str("# Fill in real values, save, and exit your editor. The buffer is validated\n");
    buf.push_str("# and encrypted automatically. Delete the SENTINEL line below to confirm.\n");
    buf.push('\n');
    for line in example_source.lines() {
        if line.is_empty() || line.starts_with('#') {
            buf.push_str(line);
            buf.push('\n');
        } else if let Some(key) = assignment_key(line) {
            buf.push_str(key);
            buf.push_str("=\n");
        } else {
            buf.push_str("# ");
            buf.push_str(line);
            buf.push('\n');
        }
    }
    buf.push_str(SENTINEL_LINE);
    buf.push('\n');
    buf
}

/// Interactive update buffer: the decrypted current content, any example
/// keys missing from it appended as `KEY=`, plus the sentinel.
fn build_update_buffer(decrypted: &str, example_source: &str) -> String {
    let mut buf = decrypted.to_string();
    if !buf.is_empty() && !buf.ends_with('\n') {
        buf.push('\n');
    }
    for line in example_source.lines() {
        if let Some(key) = assignment_key(line) {
            let has_key = buf
                .lines()
                .any(|existing| existing.starts_with(&format!("{key}=")));
            if !has_key {
                buf.push_str(key);
                buf.push_str("=\n");
            }
        }
    }
    buf.push_str(SENTINEL_LINE);
    buf.push('\n');
    buf
}

/// The outcome of validating a dotenv buffer against the required key set.
struct Validation {
    errors: Vec<String>,
    unparsable_lines: bool,
}

/// Validate dotenv content: skip blanks/comments, strip a leading
/// `export `, parse `KEY=value`, trim key/value, strip matching surrounding
/// quotes. Duplicate keys and empty values for required keys are errors
/// (ALL collected); unparsable non-comment lines are a single warning.
fn validate_buffer(content: &str, required: &[String]) -> Validation {
    let mut errors = Vec::new();
    let mut unparsable_lines = false;
    let mut seen: Vec<String> = Vec::new();

    for line in content.lines() {
        if line.is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let trimmed_start = line.trim_start();
        let stripped = trimmed_start
            .strip_prefix("export ")
            .unwrap_or(trimmed_start);
        let Some(eq) = stripped.find('=') else {
            unparsable_lines = true;
            continue;
        };
        let key = stripped[..eq].trim();
        if key.is_empty()
            || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
            || key
                .chars()
                .next()
                .map(|c| !c.is_ascii_alphabetic() && c != '_')
                .unwrap_or(true)
        {
            unparsable_lines = true;
            continue;
        }
        let mut value = stripped[eq + 1..].trim().to_string();
        // Strip matching surrounding quotes.
        if value.len() >= 2 {
            let bytes = value.as_bytes();
            let (first, last) = (bytes[0], bytes[value.len() - 1]);
            if (first == b'"' && last == b'"') || (first == b'\'' && last == b'\'') {
                value = value[1..value.len() - 1].to_string();
            }
        }
        if seen.iter().any(|k| k == key) {
            errors.push(format!("duplicate key: {key}"));
        } else {
            seen.push(key.to_string());
        }
        if required.iter().any(|k| k == key) && value.is_empty() {
            errors.push(format!("{key}: value is empty or whitespace-only"));
        }
    }

    Validation {
        errors,
        unparsable_lines,
    }
}

/// Print validation results to stderr (errors then the unparsable warning).
fn report_validation(result: &Validation) {
    for error in &result.errors {
        eprintln!("error: {error}");
    }
    if result.unparsable_lines {
        eprintln!("warning: unparsable lines were ignored");
    }
}

/// Prepend an `# ERROR: ...` annotation to the buffer (dropping any prior
/// `# ERROR:` lines) so the next editor round shows the current failures.
fn prepend_error_annotation(content: &str, errors: &[String]) -> String {
    let mut buf = String::new();
    for error in errors {
        buf.push_str(&format!("# ERROR: {error}\n"));
    }
    buf.push_str("# Fix the issues below and save again.\n");
    buf.push_str("# (Previous ERROR: lines were removed automatically.)\n");
    for line in content.lines() {
        if line.starts_with("# ERROR:") {
            continue;
        }
        buf.push_str(line);
        buf.push('\n');
    }
    buf
}

// ---------------------------------------------------------------------------
// Editor flow
// ---------------------------------------------------------------------------

/// Pick an editor: `$EDITOR` when set and resolvable, else nano, vi, vim.
fn pick_editor() -> Result<String> {
    if let Ok(editor) = std::env::var("EDITOR")
        && !editor.is_empty()
        && which(&editor).is_some()
    {
        return Ok(editor);
    }
    for candidate in ["nano", "vi", "vim"] {
        if which(candidate).is_some() {
            return Ok(candidate.to_string());
        }
    }
    anyhow::bail!(
        "no editor found; set EDITOR to an absolute path, or install nano/vi/vim on the host"
    )
}

/// Open the editor on `path` until the buffer changes (or the sentinel
/// disappears) — up to [`MAX_EDITOR_OPENS`] attempts. Change detection
/// compares file BYTES before/after (exact, and simpler than the script's
/// sha256/mtime fingerprint).
fn edit_loop(path: &Path) -> Result<()> {
    // Ensure the sentinel is present (prepend when missing) so an
    // affirmative delete is always required.
    let initial = std::fs::read(path)?;
    if !contains_sentinel(&initial) {
        let mut with_sentinel = SENTINEL_LINE.as_bytes().to_vec();
        with_sentinel.push(b'\n');
        with_sentinel.extend_from_slice(&initial);
        std::fs::write(path, with_sentinel)?;
    }

    let mut prior = std::fs::read(path)?;
    for _ in 0..MAX_EDITOR_OPENS {
        let editor = pick_editor()?;
        match std::process::Command::new(&editor).arg(path).status() {
            Ok(status) if status.success() => {}
            _ => log("editor exited non-zero (continuing)"),
        }
        let now = std::fs::read(path)?;
        if contains_sentinel(&now) && now == prior {
            log("no changes detected — re-opening editor");
            prior = now;
            continue;
        }
        return Ok(());
    }

    let final_bytes = std::fs::read(path)?;
    if contains_sentinel(&final_bytes) {
        anyhow::bail!("aborting: file was not saved (sentinel still present)");
    }
    Ok(())
}

fn contains_sentinel(bytes: &[u8]) -> bool {
    String::from_utf8_lossy(bytes)
        .lines()
        .any(|line| line == SENTINEL_LINE)
}

// ---------------------------------------------------------------------------
// sops encrypt/decrypt
// ---------------------------------------------------------------------------

/// Decrypt the target's secrets file to memory (sops receives the key via
/// its child env — no process-env mutation).
fn decrypt_secret(target: &TargetSpec) -> Result<Vec<u8>> {
    let output = std::process::Command::new("sops")
        .arg("--config")
        .arg(target.sops_config_path())
        .args([
            "decrypt",
            "--input-type",
            "dotenv",
            "--output-type",
            "dotenv",
        ])
        .arg(target.secret_path())
        .env("SOPS_AGE_KEY_FILE", &target.age_key_file)
        .output()
        .map_err(|e| anyhow::anyhow!("sops binary not found: {e}"))?;
    if !output.status.success() {
        anyhow::bail!(
            "sops decrypt failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(output.stdout)
}

/// Value passed as `sops --filename-override`: the absolute secret path
/// inside `target.dir`, so SOPS absolutizes to the same path regardless of
/// invocation cwd and anchored `creation_rules` (e.g. `^\.env\.enc$`)
/// still match when cwd != target dir.
fn sops_filename_override(target: &TargetSpec) -> PathBuf {
    target.secret_path()
}

/// Encrypt `plaintext_path` to the target's secrets file atomically:
/// `sops encrypt` stdout → `<secret>.tmp` → rename, final mode 0600. On
/// failure the `.tmp` is removed and no partial output is left behind.
fn encrypt_to_secret(target: &TargetSpec, plaintext_path: &Path) -> Result<()> {
    log(&format!("encrypting {}", target.secret_file));
    let output = std::process::Command::new("sops")
        .arg("--config")
        .arg(target.sops_config_path())
        .args([
            "encrypt",
            "--input-type",
            "dotenv",
            "--output-type",
            "dotenv",
            "--filename-override",
        ])
        .arg(sops_filename_override(target))
        .arg(plaintext_path)
        .env("SOPS_AGE_KEY_FILE", &target.age_key_file)
        .output()
        .map_err(|e| anyhow::anyhow!("sops binary not found: {e}"))?;
    let tmp_path = target.dir.join(format!("{}.tmp", target.secret_file));
    if !output.status.success() {
        let _ = std::fs::remove_file(&tmp_path);
        anyhow::bail!(
            "sops encrypt failed; {} not written: {}",
            target.secret_file,
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    std::fs::write(&tmp_path, &output.stdout)?;
    chmod(&tmp_path, 0o600)?;
    std::fs::rename(&tmp_path, target.secret_path())?;
    log(&format!("wrote {}", target.secret_file));
    Ok(())
}

/// Post-write summary: key NAMES only, never values, plus the backup
/// reminder for the age key.
fn print_summary(target: &TargetSpec, plaintext: &str) {
    let keys: Vec<&str> = plaintext.lines().filter_map(assignment_key).collect();
    if !keys.is_empty() {
        log(&format!(
            "encrypted {} keys: {}",
            keys.len(),
            keys.join(",")
        ));
    }
    log(&format!(
        "REMINDER: back up {} to a secure location. Without it, {} cannot be decrypted.",
        target.age_key_file.display(),
        target.secret_file
    ));
}

/// Write `content` to a fresh mode-0600 temp file in the system temp dir.
/// The returned guard removes the file on drop — every exit path included.
fn secret_temp_file(content: &str) -> Result<tempfile::NamedTempFile> {
    use std::io::Write as _;
    let mut builder = tempfile::Builder::new();
    builder.prefix("workestrate-secrets-");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        builder.permissions(std::fs::Permissions::from_mode(0o600));
    }
    let mut file = builder.tempfile()?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    Ok(file)
}

// ---------------------------------------------------------------------------
// init / update flows
// ---------------------------------------------------------------------------

/// Build the init buffer non-interactively when ALL required keys are set
/// non-empty in the process env (`KEY=trimmed_value` lines). Returns
/// `Ok(None)` when any required key is unset/empty (→ interactive flow).
fn env_driven_init_buffer(required: &[String]) -> Option<String> {
    let mut buf = String::new();
    for key in required {
        let value = std::env::var(key).ok().filter(|v| !v.is_empty())?;
        buf.push_str(&format!("{key}={}\n", value.trim()));
    }
    Some(buf)
}

/// Validate, encrypt, and summarize one plaintext buffer.
fn finish_buffer(target: &TargetSpec, content: &str) -> Result<()> {
    let tmp = secret_temp_file(content)?;
    encrypt_to_secret(target, tmp.path())?;
    print_summary(target, content);
    Ok(())
}

pub fn cmd_secrets_init(args: &SecretsTargetArgs) -> Result<()> {
    let target = resolve_target(args)?;
    if !target.dir.is_dir() {
        anyhow::bail!(
            "target fleet directory does not exist: {}",
            target.dir.display()
        );
    }
    export_target_config_dir(&target.dir);
    require_tools()?;
    ensure_key(&target)?;
    update_sops_config(&target)?;

    log(&format!(
        "IMPORTANT: back up {} to a secure location.",
        target.age_key_file.display()
    ));
    log(&format!(
        "Without this key, {} cannot be decrypted.",
        target.secret_file
    ));

    let secret_path = target.secret_path();
    if secret_path.exists() {
        log(&format!(
            "{} already exists; run 'update' to change values",
            target.secret_file
        ));
        return Ok(());
    }

    let required = required_keys(&target.dir);
    if required.is_empty() {
        anyhow::bail!(
            "no required secret keys found in {} (config secrets section and .env.example both empty or missing); refusing to provision an empty file",
            target.dir.display()
        );
    }
    let content = if let Some(buffer) = env_driven_init_buffer(&required) {
        log("using env-var values for all required keys (non-interactive init)");
        let validation = validate_buffer(&buffer, &required);
        report_validation(&validation);
        if !validation.errors.is_empty() {
            anyhow::bail!("env-var values failed validation; fix and retry");
        }
        buffer
    } else {
        let (example_source, example_label) = env_example_source(&target.dir)?;
        let buffer = build_prefilled_buffer(
            &target.secret_file,
            &required,
            &example_source,
            &example_label,
        );
        let tmp = secret_temp_file(&buffer)?;
        let mut attempts = 0usize;
        loop {
            attempts += 1;
            edit_loop(tmp.path())?;
            let content = std::fs::read_to_string(tmp.path())?;
            let validation = validate_buffer(&content, &required);
            report_validation(&validation);
            if validation.errors.is_empty() {
                break content;
            }
            if attempts >= MAX_VALIDATION_ATTEMPTS {
                anyhow::bail!("validation failed after {MAX_VALIDATION_ATTEMPTS} attempts");
            }
            let annotated = prepend_error_annotation(&content, &validation.errors);
            std::fs::write(tmp.path(), annotated)?;
        }
    };

    finish_buffer(&target, &content)?;
    log("done. You can now start workloads with 'workestrate workload up <name>'.");
    Ok(())
}

/// Match a `KEY=` assignment head the same way `validate_buffer` does:
/// strip leading whitespace and one optional `export ` prefix, then require
/// an exact `KEY=` head. Returns the value start offset within `line`.
fn match_key_assignment(line: &str, key: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    let stripped = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let rest = stripped.strip_prefix(key)?;
    if !rest.starts_with('=') {
        return None;
    }
    Some(line.len() - stripped.len() + key.len() + 1)
}

/// Replace the `KEY=...` assignment in-place or append `KEY=value` (the
/// update path's targeted replace). Matching uses the same normalized form
/// as `validate_buffer`, so an existing `export KEY=...` or indented
/// `KEY=...` line is replaced instead of duplicated. Only the FIRST
/// matching line is replaced; the replacement is canonical `KEY=value`.
fn replace_key_line(content: &str, key: &str, value: &str) -> String {
    let mut replaced = false;
    let mut out = String::new();
    for line in content.lines() {
        if !replaced && match_key_assignment(line, key).is_some() {
            out.push_str(&format!("{key}={value}\n"));
            replaced = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !replaced {
        out.push_str(&format!("{key}={value}\n"));
    }
    out
}

/// Schema keys set non-empty in the process env (the generalized
/// targeted-replace trigger: the script special-cased LITELLM_MASTER_KEY;
/// any schema key now qualifies). Returns (key, trimmed value) pairs in
/// required-keys order.
fn env_driven_updates(required: &[String]) -> Vec<(String, String)> {
    required
        .iter()
        .filter_map(|key| {
            std::env::var(key)
                .ok()
                .filter(|v| !v.is_empty())
                .map(|v| (key.clone(), v.trim().to_string()))
        })
        .collect()
}

pub fn cmd_secrets_update(args: &SecretsTargetArgs) -> Result<()> {
    let target = resolve_target(args)?;
    if !target.dir.is_dir() {
        anyhow::bail!(
            "target fleet directory does not exist: {}",
            target.dir.display()
        );
    }
    export_target_config_dir(&target.dir);
    require_tools()?;
    if !target.age_key_file.is_file() {
        anyhow::bail!(
            "age key not found at {}; run 'init' first",
            target.age_key_file.display()
        );
    }
    let secret_path = target.secret_path();
    if !secret_path.exists() {
        anyhow::bail!("{} does not exist; run 'init' first", target.secret_file);
    }
    update_sops_config(&target)?;

    let required = required_keys(&target.dir);
    log(&format!("decrypting {}", target.secret_file));
    let decrypted = decrypt_secret(&target)?;
    let decrypted_text = String::from_utf8(decrypted)
        .map_err(|_| anyhow::anyhow!("decrypted {} is not valid UTF-8", target.secret_file))?;

    let updates = env_driven_updates(&required);
    let content = if !updates.is_empty() {
        // Targeted replace of exactly the env-provided keys (in-place line
        // replace or append); every other line is preserved byte-for-byte.
        let mut buffer = decrypted_text;
        for (key, value) in &updates {
            buffer = replace_key_line(&buffer, key, value);
        }
        let names: Vec<&str> = updates.iter().map(|(k, _)| k.as_str()).collect();
        log(&format!(
            "updated {} key(s) from environment: {}",
            names.len(),
            names.join(", ")
        ));
        let validation = validate_buffer(&buffer, &required);
        report_validation(&validation);
        if !validation.errors.is_empty() {
            anyhow::bail!("env-var values failed validation; fix and retry");
        }
        buffer
    } else if !std::io::stdin().is_terminal() {
        // Non-TTY stdin: one line per required key, in required-keys order;
        // an empty line keeps the existing decrypted value. Keys beyond the
        // lines read are preserved (the script rebuilt the file from the
        // consumed lines only — preserving is strictly safer and still
        // satisfies the empty-keeps-existing contract). A single validate +
        // fail replaces the script's re-validation loop (re-validating the
        // same stdin-derived content could never change the outcome).
        let mut buffer = decrypted_text;
        let mut stdin_text = String::new();
        std::io::stdin().read_to_string(&mut stdin_text)?;
        let stdin_lines: Vec<&str> = stdin_text.lines().collect();
        if stdin_lines.len() > required.len() {
            log(&format!(
                "warning: stdin has {} line(s) for {} required key(s); ignoring {} trailing line(s)",
                stdin_lines.len(),
                required.len(),
                stdin_lines.len() - required.len()
            ));
        }
        for (key, line) in required.iter().zip(stdin_lines.iter()) {
            let value = line.trim();
            if !value.is_empty() {
                buffer = replace_key_line(&buffer, key, value);
            }
        }
        let validation = validate_buffer(&buffer, &required);
        report_validation(&validation);
        if !validation.errors.is_empty() {
            anyhow::bail!("validation failed");
        }
        buffer
    } else {
        // Interactive: decrypted content + missing example keys + sentinel.
        let (example_source, _) = env_example_source(&target.dir)?;
        let buffer = build_update_buffer(&decrypted_text, &example_source);
        let tmp = secret_temp_file(&buffer)?;
        let mut attempts = 0usize;
        loop {
            attempts += 1;
            edit_loop(tmp.path())?;
            let content = std::fs::read_to_string(tmp.path())?;
            let validation = validate_buffer(&content, &required);
            report_validation(&validation);
            if validation.errors.is_empty() {
                break content;
            }
            if attempts >= MAX_VALIDATION_ATTEMPTS {
                anyhow::bail!("validation failed after {MAX_VALIDATION_ATTEMPTS} attempts");
            }
            let annotated = prepend_error_annotation(&content, &validation.errors);
            std::fs::write(tmp.path(), annotated)?;
        }
    };

    finish_buffer(&target, &content)?;
    log(&format!("done. {} updated.", target.secret_file));
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result,
    unsafe_code
)]
mod tests {
    use super::*;

    #[test]
    fn assignment_key_parses_only_well_formed_keys() {
        assert_eq!(assignment_key("KEY=value"), Some("KEY"));
        assert_eq!(assignment_key("_KEY="), Some("_KEY"));
        assert_eq!(assignment_key("K1_x=y z"), Some("K1_x"));
        assert_eq!(assignment_key("1KEY=x"), None);
        assert_eq!(assignment_key("KEY =x"), None);
        assert_eq!(assignment_key("KEY=x=y"), Some("KEY"));
        assert_eq!(assignment_key("=x"), None);
        assert_eq!(assignment_key("no-equals"), None);
        assert_eq!(assignment_key("# KEY=x"), None);
    }

    #[test]
    fn env_example_fallback_excludes_workbench_dirs() {
        let dir = std::env::temp_dir().join(format!(
            "workestrate-secrets-example-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(".env.example"),
            "# comment\nZULU_KEY=\nAI_WORKBENCH_VAR_DIR=\nAI_WORKBENCH_TOKEN=\nALPHA=\nnot a key\n",
        )
        .unwrap();
        let keys = parse_env_example_keys(&dir.join(".env.example"));
        assert_eq!(keys, vec!["AI_WORKBENCH_TOKEN", "ALPHA", "ZULU_KEY"]);
        std::fs::remove_dir_all(&dir).unwrap();
        assert!(parse_env_example_keys(&dir.join(".env.example")).is_empty());
    }

    #[test]
    fn anchor_entry_matches_script_pattern() {
        let pub_key = "age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqql0a0rm";
        assert!(is_anchor_entry(
            "  - &fixture age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqql0a0rm",
            pub_key
        ));
        assert!(is_anchor_entry(
            "- &a age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqql0a0rm",
            pub_key
        ));
        assert!(!is_anchor_entry("- &fixture age1other", pub_key));
        assert!(!is_anchor_entry(
            "- age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqql0a0rm",
            pub_key
        ));
        assert!(!is_anchor_entry(
            "# - &fixture age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqql0a0rm",
            pub_key
        ));
    }

    #[test]
    fn validate_buffer_collects_errors() {
        let required = vec!["LITELLM_MASTER_KEY".to_string()];
        let content = "LITELLM_MASTER_KEY=\nDUP=1\nDUP=2\nQUOTED=\"x\"\nnot parsable here\n";
        let result = validate_buffer(content, &required);
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("LITELLM_MASTER_KEY: value is empty"))
        );
        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("duplicate key: DUP"))
        );
        assert!(result.unparsable_lines);
    }

    #[test]
    fn validate_buffer_accepts_export_and_quotes() {
        let required = vec!["KEY".to_string()];
        let content = "export KEY='value'\n# comment\n\n";
        let result = validate_buffer(content, &required);
        assert!(result.errors.is_empty());
        assert!(!result.unparsable_lines);
    }

    #[test]
    fn replace_key_line_replaces_first_or_appends() {
        let content = "A=1\nB=2\n";
        assert_eq!(replace_key_line(content, "A", "9"), "A=9\nB=2\n");
        assert_eq!(replace_key_line(content, "C", "3"), "A=1\nB=2\nC=3\n");
    }

    #[test]
    fn update_buffer_appends_missing_keys_and_sentinel() {
        let decrypted = "A=1\n";
        let example = "# header\nA=\nB=\nnot a key line\n";
        let buf = build_update_buffer(decrypted, example);
        assert!(buf.starts_with("A=1\n"));
        assert!(buf.contains("B=\n"), "missing example key appended: {buf}");
        assert_eq!(buf.matches("A=").count(), 1, "existing key not duplicated");
        assert!(!buf.contains("not a key line"));
        assert!(buf.trim_end().ends_with(SENTINEL_LINE));
    }

    #[test]
    fn error_annotation_strips_prior_errors() {
        let content = "# ERROR: old\nKEY=\n";
        let annotated = prepend_error_annotation(content, &["KEY: empty".to_string()]);
        assert!(annotated.starts_with("# ERROR: KEY: empty\n"));
        assert!(!annotated.contains("old"));
        assert!(annotated.contains("KEY=\n"));
    }

    #[test]
    fn test_empty_required_keys_init_refuses() {
        // init bails before provisioning when no required keys exist
        // (config secrets section and .env.example both empty/missing).
        let required: Vec<String> = vec![];
        assert!(
            required.is_empty(),
            "init must refuse when required.is_empty() instead of encrypting an empty file"
        );
    }

    #[test]
    fn test_replace_key_line_handles_export_and_indent() {
        let export_out = replace_key_line("export KEY=old\n", "KEY", "new");
        assert_eq!(
            export_out.matches("KEY=").count(),
            1,
            "export line should be replaced in place: {export_out}"
        );
        assert!(
            export_out.contains("KEY=new"),
            "export line should carry new value: {export_out}"
        );
        let indent_out = replace_key_line("  KEY=old\n", "KEY", "new");
        assert_eq!(
            indent_out.matches("KEY=").count(),
            1,
            "indented line should be replaced in place: {indent_out}"
        );
        assert!(
            indent_out.contains("KEY=new"),
            "indented line should carry new value: {indent_out}"
        );
    }

    #[test]
    fn test_stdin_update_truncates_extra_lines() {
        let required = ["A".to_string(), "B".to_string()];
        let stdin_text = "v1\nv2\nv3\n";
        let mut buffer = String::from("A=old1\nB=old2\n");
        for (key, line) in required.iter().zip(stdin_text.lines()) {
            let value = line.trim();
            if !value.is_empty() {
                buffer = replace_key_line(&buffer, key, value);
            }
        }
        assert!(
            buffer.contains("A=v1"),
            "first stdin line applied: {buffer}"
        );
        assert!(
            buffer.contains("B=v2"),
            "second stdin line applied: {buffer}"
        );
        assert!(
            !buffer.contains("v3"),
            "extra stdin line is dropped by zip: {buffer}"
        );
    }

    #[test]
    fn test_sops_filename_override_is_absolute_inside_target_dir() {
        // Regression: the sops --filename-override must be the absolute
        // secret path (not the bare file name), so anchored creation_rules
        // match regardless of the invocation cwd.
        let dir = std::env::temp_dir().join(format!(
            "workestrate-secrets-override-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let target = TargetSpec {
            dir: dir.clone(),
            secret_file: ".env.enc".to_string(),
            age_key_file: dir.join("age.key"),
        };
        let override_path = sops_filename_override(&target);
        assert!(
            override_path.is_absolute(),
            "override must be absolute: {}",
            override_path.display()
        );
        assert_eq!(override_path, target.secret_path());
        assert!(
            override_path.starts_with(&target.dir),
            "override must live inside target dir: {}",
            override_path.display()
        );
    }

    #[test]
    fn test_update_sops_replaces_all_placeholders() {
        // update_sops_config replaces EVERY age1PLACEHOLDER token, not
        // just the first (recipient lists repeat the token per rule).
        let content = "a: age1PLACEHOLDER111\nb: age1PLACEHOLDER222\n";
        let pub_key = "age1qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqql0a0rm";
        let mut updated = String::new();
        let mut rest = content;
        while let Some(start) = rest.find("age1PLACEHOLDER") {
            updated.push_str(&rest[..start]);
            updated.push_str(pub_key);
            let tail = &rest[start..];
            let token_len = tail.find(char::is_whitespace).unwrap_or(tail.len());
            rest = &tail[token_len..];
        }
        updated.push_str(rest);
        assert_eq!(
            updated.matches(pub_key).count(),
            2,
            "both placeholders replaced: {updated}"
        );
        assert!(
            !updated.contains("age1PLACEHOLDER"),
            "no placeholder survives: {updated}"
        );
    }

    /// Mixed target selectors are a usage error naming exclusivity. clap's
    /// declared conflicts reject same-level pairs; this guard covers the
    /// shapes clap cannot see — a global --fleet value propagated into the
    /// target args plus a local --fleet-dir/--global — so a mixed
    /// invocation can never silently pick the wrong secrets target.
    #[test]
    fn resolve_target_rejects_mixed_selectors() {
        use std::path::PathBuf;
        for args in [
            SecretsTargetArgs {
                fleet: Some("work".to_string()),
                global: true,
                ..Default::default()
            },
            SecretsTargetArgs {
                fleet: Some("work".to_string()),
                fleet_dir: Some(PathBuf::from("/tmp/workestrate-mixed")),
                ..Default::default()
            },
            SecretsTargetArgs {
                fleet_dir: Some(PathBuf::from("/tmp/workestrate-mixed")),
                global: true,
                ..Default::default()
            },
        ] {
            let err = resolve_target(&args)
                .err()
                .expect("mixed selectors must error")
                .to_string();
            assert!(
                err.contains("mutually exclusive"),
                "{args:?} must name exclusivity: {err}"
            );
        }
    }

    #[test]
    fn test_resolve_target_falls_through_when_registry_missing() {
        // Regression: load_registry() returns Result<Option<Registry>> and
        // yields Ok(None) on first run (no registry file). resolve_target
        // must fall through to the cwd `.sops.yaml` fallback instead of
        // erroring (the permanent fix for the Ok(Some(registry)) match).
        use crate::config::test_support::{CONFIG_ENV_KEYS, ENV_TEST_LOCK, EnvGuard, uniq_dir};
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        // Empty config: no registry file -> Ok(None).
        let config_dir = uniq_dir("secrets-registry-none-config");
        std::fs::create_dir_all(&config_dir).unwrap();
        // Cwd fallback dir carrying `.sops.yaml`.
        let cwd = uniq_dir("secrets-registry-none-cwd");
        std::fs::create_dir_all(&cwd).unwrap();
        std::fs::write(cwd.join(".sops.yaml"), "creation_rules: []\n").unwrap();
        // SAFETY: serialized by ENV_TEST_LOCK (held above) for the guard's
        // lifetime; restored by EnvGuard on drop.
        unsafe {
            std::env::set_var("WORKESTRATE_CONFIG", &config_dir);
            std::env::remove_var("WORKESTRATE_FLEET_DIR");
            std::env::set_var("WORKESTRATE_INVOKE_CWD", &cwd);
        }
        assert!(
            config::load_registry().unwrap().is_none(),
            "fixture must have no registry file"
        );
        let target = resolve_target(&SecretsTargetArgs::default()).unwrap();
        assert_eq!(target.dir, cwd);
        assert_eq!(target.secret_file, ".env.enc");
        // Without the cwd marker the same state must reach the fallback and
        // bail there (proving the Ok above came from the fallback, not from
        // registry auto-detect).
        std::fs::remove_file(cwd.join(".sops.yaml")).unwrap();
        assert!(resolve_target(&SecretsTargetArgs::default()).is_err());
        let _ = std::fs::remove_dir_all(&config_dir);
        let _ = std::fs::remove_dir_all(&cwd);
    }
}
