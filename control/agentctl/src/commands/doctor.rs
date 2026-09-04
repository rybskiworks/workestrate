//! `workestrate doctor` — environment/tool health checks and the
//! `DoctorCheck` result struct.

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config;
use crate::images::state::{ImageRecord, ImagesState};
use crate::scaffold;

/// One doctor check result. `status` is "OK", "WARN", or "FAIL"; a FAIL
/// anywhere flips the overall verdict and the process exit code to 1.
#[derive(Debug, serde::Serialize)]
pub struct DoctorCheck {
    name: &'static str,
    status: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    remediation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repos: Option<Vec<serde_json::Value>>,
}

impl DoctorCheck {
    fn new(name: &'static str, status: &'static str, message: String) -> Self {
        Self {
            name,
            status,
            message,
            remediation: None,
            repos: None,
        }
    }

    fn with_remediation(mut self, remediation: &str) -> Self {
        self.remediation = Some(remediation.to_string());
        self
    }

    fn with_repos(mut self, repos: Vec<serde_json::Value>) -> Self {
        self.repos = Some(repos);
        self
    }
}

/// Resolve the default age key file path — identical fallback chain to
/// `secrets_loader::decrypt_layer()`: `SOPS_AGE_KEY_FILE` env, else `$HOME` +
/// `scaffold::AGE_KEY_DEFAULT_PATH` with the `~/` prefix stripped.
pub fn default_age_key_path() -> PathBuf {
    if let Ok(env_key) = std::env::var("SOPS_AGE_KEY_FILE") {
        return PathBuf::from(env_key);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let rel = scaffold::AGE_KEY_DEFAULT_PATH
        .strip_prefix("~/")
        .unwrap_or(scaffold::AGE_KEY_DEFAULT_PATH);
    PathBuf::from(home).join(rel)
}

/// The msb binary to probe: `MSB_PATH` when set, else `msb` on PATH.
pub fn msb_binary() -> String {
    std::env::var("MSB_PATH").unwrap_or_else(|_| "msb".to_string())
}

/// Run `<bin> --version` and return the first line of stdout, trimmed.
/// Returns `None` when the binary is missing (NotFound) or exits non-zero.
pub fn probe_version(bin: &str) -> Option<String> {
    let output = std::process::Command::new(bin)
        .arg("--version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let first = stdout.lines().next().unwrap_or("").trim();
    if first.is_empty() {
        None
    } else {
        Some(first.to_string())
    }
}

pub fn doctor_check_kvm() -> DoctorCheck {
    let kvm = Path::new("/dev/kvm");
    if !kvm.exists() {
        return DoctorCheck::new("dev_kvm", "FAIL", "not found".to_string()).with_remediation(
            "Install KVM support; on Linux: ensure virtualization is enabled in BIOS \
                 and kvm module is loaded (sudo modprobe kvm)",
        );
    }
    match std::fs::File::open(kvm) {
        Ok(_) => DoctorCheck::new("dev_kvm", "OK", "accessible".to_string()),
        Err(_) => DoctorCheck::new("dev_kvm", "WARN", "exists but not accessible".to_string())
            .with_remediation(
                "Add your user to the kvm group (sudo usermod -aG kvm $USER) or fix \
                 /dev/kvm permissions",
            ),
    }
}

/// Nested-virtualization inventory (ADR 0036 §4): additive next to the
/// verbatim `dev_kvm` check above (which is NEVER renamed — external
/// tooling greps for it). Reads the shared advisory probe
/// (`microsandbox::nested::read_nested_probe`): OK = /dev/kvm accessible +
/// vmx|svm flag + nested parameter affirmatively enabled; WARN = KVM works
/// but nested is disabled/unknown; FAIL = no /dev/kvm. Each carries
/// remediation. The trailing pin note records which fork rev carries the
/// Track 1 VMM flag — honestly marked INERT until the Phase 2 firmware
/// rebuild lands (plan D6: no "nested works" claim until Track 3).
pub fn doctor_check_nested_virt() -> DoctorCheck {
    const PIN_NOTE: &str =
        "fork rev 78fb3ed1 carries the Track 1 VMM nested_virt flag (inert until the Phase 2 libkrunfw rebuild w/ CONFIG_KVM lands)";
    const REMEDIATION: &str = "Enable virtualization in BIOS + sudo modprobe kvm(_intel|_amd) + \
         sudo usermod -aG kvm $USER (re-login); guest nesting additionally needs the host \
         kvm_intel/kvm_amd nested parameter at Y (sudo modprobe kvm_intel nested=1)";
    let probe = crate::microsandbox::nested::read_nested_probe();
    if !probe.kvm_present {
        return DoctorCheck::new(
            "nested_virt",
            "FAIL",
            format!("no /dev/kvm; {PIN_NOTE}"),
        )
        .with_remediation(REMEDIATION);
    }
    if !probe.kvm_accessible {
        // WARN, not FAIL (matches `dev_kvm` severity: fixable perms, and a
        // plain `up` without a nested ask is unaffected) — but the message
        // is explicit that every nested ask refuses until fixed.
        return DoctorCheck::new(
            "nested_virt",
            "WARN",
            format!("host /dev/kvm exists but not accessible (nested asks refuse); {PIN_NOTE}"),
        )
        .with_remediation(REMEDIATION);
    }
    match (probe.cpu_flag, probe.nested_param) {
        (true, Some(true)) => DoctorCheck::new(
            "nested_virt",
            "OK",
            format!("kvm + vmx/svm + nested=Y; {PIN_NOTE}"),
        ),
        (_, nested) => {
            let detail = match nested {
                Some(false) => "nested=N (disabled)",
                _ => "nested param unknown",
            };
            let cpu = if probe.cpu_flag { "vmx/svm" } else { "no vmx/svm flag" };
            DoctorCheck::new(
                "nested_virt",
                "WARN",
                format!("kvm ok ({cpu}), {detail}; {PIN_NOTE}"),
            )
            .with_remediation(REMEDIATION)
        }
    }
}

pub fn doctor_check_tool(name: &'static str, bin: &str, remediation: &str) -> DoctorCheck {
    match probe_version(bin) {
        Some(version) => DoctorCheck::new(name, "OK", version),
        None => DoctorCheck::new(name, "FAIL", format!("'{}' not found on PATH", bin))
            .with_remediation(remediation),
    }
}

pub fn doctor_check_age_key_file() -> DoctorCheck {
    use std::os::unix::fs::PermissionsExt;
    let path = default_age_key_path();
    let metadata = match std::fs::metadata(&path) {
        Ok(m) => m,
        Err(_) => {
            return DoctorCheck::new(
                "age_key_file",
                "WARN",
                format!("{} not found", path.display()),
            )
            .with_remediation("Run 'setup-secrets init' to generate the age key");
        }
    };
    let mode = metadata.permissions().mode() & 0o777;
    if mode & 0o077 > 0 {
        DoctorCheck::new(
            "age_key_file",
            "WARN",
            format!("{} (mode {:o}, too permissive)", path.display(), mode),
        )
        .with_remediation(&format!("chmod 600 {}", path.display()))
    } else {
        DoctorCheck::new(
            "age_key_file",
            "OK",
            format!("{} (mode {:o})", path.display(), mode),
        )
    }
}

pub fn doctor_check_msb() -> DoctorCheck {
    let bin = msb_binary();
    let home = resolve_msb_home();
    let version = match probe_version(&bin) {
        Some(v) => v,
        None => {
            return DoctorCheck::new(
                "msb",
                "FAIL",
                format!("MSB_HOME={} '{}' not reachable", home.display(), bin),
            )
            .with_remediation(
                "Ensure msb is installed; run 'nix develop' or check MSB_HOME/MSB_PATH",
            )
        }
    };
    // Downgrade probe first: a canonical DB newer than the msb binary is a
    // hard FAIL (it beats any skew WARN below).
    if let Some(fail) = check_msb_store_downgrade(&bin, &home, &version) {
        return fail;
    }
    let legacy_db = legacy_msb_home().join("db").join("msb.db");
    let canonical_db = home.join("db").join("msb.db");
    match msb_home_skew_status(&legacy_db, &canonical_db) {
        MsbHomeSkew::Skewed => DoctorCheck::new(
            "msb",
            "WARN",
            format!(
                "{}; MSB homes skewed: legacy {} exists alongside canonical {}",
                msb_ok_message(&home, &version),
                legacy_db.display(),
                canonical_db.display()
            ),
        )
        .with_remediation(
            "Run 'scripts/migrate-msb-home.sh --check-only' then without flags to migrate",
        ),
        MsbHomeSkew::Unmigrated => DoctorCheck::new(
            "msb",
            "WARN",
            format!(
                "{}; legacy MSB home {} exists but canonical {} is missing (unmigrated)",
                msb_ok_message(&home, &version),
                legacy_db.display(),
                canonical_db.display()
            ),
        )
        .with_remediation(
            "Run 'scripts/migrate-msb-home.sh --check-only' then without flags to migrate",
        ),
        MsbHomeSkew::Clean => DoctorCheck::new("msb", "OK", msb_ok_message(&home, &version)),
    }
}

/// Resolve the canonical msb home: non-empty `MSB_HOME` verbatim (empty
/// treated as unset), else `$HOME/.microsandbox`, else `./.microsandbox`.
/// Mirrors `microsandbox_utils::resolve_home` (the SDK default).
pub fn resolve_msb_home() -> PathBuf {
    if let Some(path) = std::env::var_os("MSB_HOME").filter(|v| !v.is_empty()) {
        return PathBuf::from(path);
    }
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".microsandbox")
}

/// The legacy pre-convergence devshell home (`$HOME/.cache/ai-workbench-msb`).
pub fn legacy_msb_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".cache")
        .join("ai-workbench-msb")
}

/// The OK/WARN message core: resolved home plus the probed msb version.
pub fn msb_ok_message(home: &Path, version: &str) -> String {
    format!("MSB_HOME={} {}", home.display(), version)
}

/// Skew verdict for the legacy-vs-canonical msb homes (pure core).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsbHomeSkew {
    /// Canonical only (or neither home has a DB yet) — nothing to do.
    Clean,
    /// Only the legacy DB exists — unmigrated.
    Unmigrated,
    /// Both DBs exist — migration needed / divergence risk.
    Skewed,
}

/// Pure skew classifier over DB-file existence (unit-testable without the
/// filesystem).
pub fn classify_msb_home_skew(legacy_db_exists: bool, canonical_db_exists: bool) -> MsbHomeSkew {
    match (legacy_db_exists, canonical_db_exists) {
        (true, true) => MsbHomeSkew::Skewed,
        (true, false) => MsbHomeSkew::Unmigrated,
        _ => MsbHomeSkew::Clean,
    }
}

/// Path-taking skew wrapper: stats both `db/msb.db` files. When the resolved
/// canonical home IS the legacy path (custom MSB_HOME edge), there is no
/// second home to skew against → Clean.
pub fn msb_home_skew_status(legacy_db: &Path, canonical_db: &Path) -> MsbHomeSkew {
    if legacy_db == canonical_db {
        return MsbHomeSkew::Clean;
    }
    classify_msb_home_skew(legacy_db.is_file(), canonical_db.is_file())
}

/// Pure downgrade classifier over combined msb store-command output
/// (case-insensitive): true when the output indicates the store DB is newer
/// than the msb binary understands.
pub fn classify_msb_store_error(output: &str) -> bool {
    let lower = output.to_lowercase();
    [
        "downgrade",
        "newer",
        "unsupported schema",
        "migration",
        "database version",
    ]
    .iter()
    .any(|pat| lower.contains(pat))
}

/// Downgrade probe: run `<bin> list`; a failing run whose output matches
/// [`classify_msb_store_error`] becomes a FAIL check (DB newer than binary).
/// Success or any non-matching failure yields `None` (no verdict).
fn check_msb_store_downgrade(bin: &str, home: &Path, version: &str) -> Option<DoctorCheck> {
    let out = std::process::Command::new(bin).arg("list").output().ok()?;
    if out.status.success() {
        return None;
    }
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    if classify_msb_store_error(&combined) {
        Some(
            DoctorCheck::new(
                "msb",
                "FAIL",
                format!(
                    "{}; msb store error indicates the DB is newer than the binary: {}",
                    msb_ok_message(home, version),
                    combined.trim().lines().next().unwrap_or("").trim(),
                ),
            )
            .with_remediation(
                "DB newer than msb binary; downgrade msb or restore backup; \
                 see 'scripts/migrate-msb-home.sh --rollback <ts>'",
            ),
        )
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Image records vs store presence (spec 21 §3.2 — read-only)
// ---------------------------------------------------------------------------

/// What the read-only `msb image ls` probe returned. Separated from the
/// check builder so the verdict logic is unit-testable without spawning msb.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreListing {
    /// Reference strings from the listing's first column.
    Tags(Vec<String>),
    /// The listing could not be read (msb missing/unreachable/failed) —
    /// always WARN, never FAIL: this hook is read-only and the records are
    /// advisory (spec §3.2).
    Unreachable(String),
}

/// Parse the reference column of `msb image ls` output (pure): the first
/// whitespace-separated token of each non-header, non-empty row. The
/// "No images found." empty-store line yields an empty vec.
pub fn parse_image_ls_references(output: &str) -> Vec<String> {
    let mut refs = Vec::new();
    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line == "No images found." {
            continue;
        }
        let Some(first) = line.split_whitespace().next() else {
            continue;
        };
        if first == "REFERENCE" {
            continue; // the header row
        }
        refs.push(first.to_string());
    }
    refs
}

/// The pure verdict core of [`doctor_check_image_records`]: recorded tags vs
/// store presence. No records → OK note; unreachable store → WARN (never
/// FAIL a read-only hook); a recorded tag missing from the store → WARN with
/// remediation; all present → OK.
pub fn image_records_check(state: &ImagesState, listing: &StoreListing) -> DoctorCheck {
    if state.images.is_empty() {
        return DoctorCheck::new(
            "image_records",
            "OK",
            "no image records (no nix-layered image has been built/loaded via 'workload \
             build' yet)"
                .to_string(),
        );
    }
    let count = state.images.len();
    let listing = match listing {
        StoreListing::Tags(tags) => tags,
        StoreListing::Unreachable(detail) => {
            return DoctorCheck::new(
                "image_records",
                "WARN",
                format!(
                    "{count} image record(s) but the msb store listing could not be read: \
                     {detail}"
                ),
            )
            .with_remediation(
                "Check that msb is installed and MSB_HOME/MSB_PATH are set correctly; the \
                 records are advisory — the store is ground truth (spec 21 §3.2)",
            );
        }
    };
    let mut missing: Vec<&ImageRecord> = state
        .images
        .values()
        .filter(|r| !listing.iter().any(|t| t == &r.tag))
        .collect();
    missing.sort_by(|a, b| a.tag.cmp(&b.tag));
    if missing.is_empty() {
        DoctorCheck::new(
            "image_records",
            "OK",
            format!("{count} image record(s); all recorded tags present in the msb store"),
        )
    } else {
        let names = missing
            .iter()
            .map(|r| format!("{}#{}", r.repo.name, r.tag))
            .collect::<Vec<_>>()
            .join(", ");
        let first_repo = missing[0].repo.name.clone();
        DoctorCheck::new(
            "image_records",
            "WARN",
            format!("recorded tag(s) missing from the msb store: {names}"),
        )
        .with_remediation(&format!(
            "re-run 'workestrate workload build --repo {first_repo}' (or load the image \
             manually via the declaring repo's 'load-images' recipe) to re-populate the \
             store; records are advisory — the store is ground truth (spec 21 §3.2)"
        ))
    }
}

/// `image_records` check (spec 21 §3.2, read-only): the per-home
/// `state/images.json` records against the live msb store listing (`msb
/// image ls` via [`msb_binary`]). Never FAILs: a missing recorded tag or an
/// unreadable store is WARN (the records are advisory; the store is ground
/// truth). nix is not consulted at all — the separate `nix` doctor check
/// owns that verdict.
pub fn doctor_check_image_records() -> DoctorCheck {
    let state = ImagesState::load(&config::resolve_state_dir());
    if state.images.is_empty() {
        // No records → OK note without touching the store at all.
        return image_records_check(&state, &StoreListing::Tags(Vec::new()));
    }
    let bin = msb_binary();
    let listing = match std::process::Command::new(&bin)
        .args(["image", "ls"])
        .output()
    {
        Ok(out) if out.status.success() => StoreListing::Tags(parse_image_ls_references(
            &String::from_utf8_lossy(&out.stdout),
        )),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            StoreListing::Unreachable(if stderr.is_empty() {
                format!("'{bin} image ls' exited with {}", out.status)
            } else {
                stderr
            })
        }
        Err(e) => StoreListing::Unreachable(format!("failed to spawn '{bin}': {e}")),
    };
    image_records_check(&state, &listing)
}

pub fn doctor_check_home() -> DoctorCheck {
    let (home, kind) = config::resolve_home_with_kind();
    let message = format!("{} ({:?})", home.display(), kind);
    if home.exists() {
        DoctorCheck::new("home", "OK", message)
    } else {
        DoctorCheck::new("home", "WARN", format!("{} (does not exist)", message))
            .with_remediation("Run 'workestrate init' to create the tool home")
    }
}

pub fn doctor_check_config_repos() -> Result<DoctorCheck> {
    let registry = match config::load_registry()? {
        Some(r) => r,
        None => {
            return Ok(
                DoctorCheck::new("config_repos", "WARN", "no registry found".to_string())
                    .with_remediation("run 'workestrate init'"),
            );
        }
    };
    let mut repos: Vec<serde_json::Value> = Vec::new();
    for status in crate::git::collect_repo_statuses(&registry) {
        let dest = config::config_repo_dir(&status.name);
        if !status.exists {
            repos.push(serde_json::json!({
                "name": status.name,
                "status": "FAIL",
                "rev": status.rev,
                "dirty": false,
                "message": format!("clone missing at {}", dest.display()),
            }));
            continue;
        }
        let check = if status.dirty { "WARN" } else { "OK" };
        let message = if status.dirty {
            format!("rev {}, dirty", status.short)
        } else {
            format!("rev {}, clean", status.short)
        };
        repos.push(serde_json::json!({
            "name": status.name,
            "status": check,
            "rev": status.rev,
            "dirty": status.dirty,
            "message": message,
        }));
    }
    let worst = if repos.iter().any(|r| r["status"] == "FAIL") {
        "FAIL"
    } else if repos.iter().any(|r| r["status"] == "WARN") {
        "WARN"
    } else {
        "OK"
    };
    let message = if repos.is_empty() {
        "no config repos registered".to_string()
    } else {
        format!("{} repo(s) checked", repos.len())
    };
    Ok(DoctorCheck::new("config_repos", worst, message).with_repos(repos))
}

/// `schemas` check: generate both schema artifacts in-process and compare
/// every known consumer location (tool template, tool home, registered config
/// repos with a schemas/ dir) byte-for-byte — the same freshness rule as
/// `schemas update --check`. A missing or mismatched file is STALE per
/// target; the check-level status is WARN when any copy is stale (stale
/// copies never break the tool — validate-config uses the Rust types; only
/// editor/tombi UX is affected), never FAIL.
pub fn doctor_check_schemas() -> Result<DoctorCheck> {
    let (full, workload, registry) = crate::commands::diagnostics::generate_schema_triple()?;
    let artifacts: [(&str, &str); 3] = [
        ("workestrate.schema.json", full.as_str()),
        ("workestrate-workload.schema.json", workload.as_str()),
        ("registry.schema.json", registry.as_str()),
    ];
    let targets = crate::commands::schemas::schema_targets(None)?;
    let mut entries: Vec<serde_json::Value> = Vec::new();
    let mut stale = 0usize;
    for t in &targets {
        let mut t_stale = false;
        for (name, content) in artifacts {
            let p = t.dir.join(name);
            // The canonical on-disk form carries the trailing newline (the
            // same form `schemas update` writes and `--check` compares), so
            // the freshness rule matches `schemas update --check` exactly.
            let canonical = format!("{}\n", content);
            let ok = std::fs::read(&p)
                .map(|b| b == canonical.as_bytes())
                .unwrap_or(false);
            if !ok {
                t_stale = true;
            }
        }
        entries.push(serde_json::json!({
            "target": t.label,
            "status": if t_stale { "STALE" } else { "OK" },
            "path": t.dir.display().to_string(),
        }));
        if t_stale {
            stale += 1;
        }
    }
    let status = if stale == 0 { "OK" } else { "WARN" };
    let message = if targets.is_empty() {
        "no consumer schema locations found".to_string()
    } else {
        format!(
            "{} consumer location(s) checked, {} stale",
            targets.len(),
            stale
        )
    };
    Ok(DoctorCheck::new("schemas", status, message)
        .with_remediation("Run 'workestrate schemas update' to refresh consumer schema copies")
        .with_repos(entries))
}

/// `workestrate doctor` — run environment/tool health checks (KVM, nix,
/// sops, age, msb, home resolution, config repos), print a human report or a
/// machine-readable JSON document, and exit non-zero when any check FAILs.
pub fn cmd_doctor(json: bool) -> Result<()> {
    let checks = vec![
        doctor_check_kvm(),
        doctor_check_nested_virt(),
        doctor_check_tool(
            "nix",
            "nix",
            "Install Nix: sh <(curl -L https://nixos.org/nix/install) or use the \
             multi-user installer",
        ),
        doctor_check_tool(
            "sops",
            "sops",
            "Install sops; run inside 'nix develop' or: go install \
             github.com/getsops/sops/v3/cmd/sops@latest",
        ),
        doctor_check_tool(
            "age_keygen",
            "age-keygen",
            "Install age; run inside 'nix develop' or: brew install age",
        ),
        doctor_check_age_key_file(),
        doctor_check_msb(),
        doctor_check_home(),
        doctor_check_config_repos()?,
        doctor_check_schemas()?,
        doctor_check_image_records(),
    ];
    let overall = if checks.iter().any(|c| c.status == "FAIL") {
        "FAIL"
    } else if checks.iter().any(|c| c.status == "WARN") {
        "WARN"
    } else {
        "OK"
    };

    if json {
        let body = serde_json::json!({
            "checks": checks,
            "overall": overall,
        });
        println!("{}", serde_json::to_string_pretty(&body)?);
    } else {
        println!("=== workestrate doctor ===\n");
        for check in &checks {
            println!("{}: {} ({})", check.name, check.status, check.message);
            if let Some(ref remediation) = check.remediation {
                println!("  → {}", remediation);
            }
            if let Some(ref repos) = check.repos {
                for repo in repos {
                    // config_repos entries carry name/message; the schemas
                    // row carries target/path instead.
                    let name = repo["name"]
                        .as_str()
                        .or_else(|| repo["target"].as_str())
                        .unwrap_or("unknown");
                    let status = repo["status"].as_str().unwrap_or("unknown");
                    let message = repo["message"].as_str().unwrap_or("");
                    let detail = if message.is_empty() {
                        repo["path"].as_str().unwrap_or("")
                    } else {
                        message
                    };
                    println!("  {}: {} ({})", name, status, detail);
                }
            }
        }
        println!();
        match overall {
            "OK" => println!("All checks passed."),
            "WARN" => println!("All checks passed with warnings."),
            _ => println!("Some checks failed."),
        }
    }

    if overall == "FAIL" {
        std::process::exit(1);
    }
    Ok(())
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;
    use crate::images::state::{image_key, RepoIdentity};

    fn record(repo: &str, tag: &str) -> ImageRecord {
        ImageRecord {
            repo: RepoIdentity {
                name: repo.to_string(),
                path: PathBuf::from("/tmp/repo"),
                flake_root: PathBuf::from("/tmp/repo"),
            },
            attr: "img".to_string(),
            tag: tag.to_string(),
            drv_path: "/nix/store/drv.drv".to_string(),
            out_path: "/nix/store/out.tar.gz".to_string(),
            digest: None,
            built_at: "2026-08-02T10:15:00Z".to_string(),
            loaded_at: "2026-08-02T10:16:12Z".to_string(),
            loader: "workestrate 0.1.0".to_string(),
            host: "devbox".to_string(),
            user: "node".to_string(),
        }
    }

    fn state_with(records: &[(&str, &str)]) -> ImagesState {
        let mut state = ImagesState::default();
        for (repo, tag) in records {
            state.upsert(image_key(repo, tag), record(repo, tag));
        }
        state
    }

    // ---- parse_image_ls_references (pure) ----

    #[test]
    fn parse_image_ls_reads_the_reference_column() {
        let output = "REFERENCE                  DIGEST                 SIZE        CREATED\n\
                      wk-fixture-image:latest    sha256:3a48c1e72d5a    20.0 KiB    2026-08-02 20:59:28\n\
                      tempest:latest             sha256:9f8e7d6c5b4a    1.2 GiB     2026-08-01 09:00:00\n";
        assert_eq!(
            parse_image_ls_references(output),
            vec!["wk-fixture-image:latest", "tempest:latest"]
        );
        assert!(parse_image_ls_references("No images found.").is_empty());
        assert!(parse_image_ls_references("").is_empty());
    }

    // ---- image_records_check (pure verdict core) ----

    /// No records → OK note (the store is never consulted).
    #[test]
    fn no_records_is_an_ok_note() {
        let check = image_records_check(&ImagesState::default(), &StoreListing::Tags(vec![]));
        assert_eq!(check.name, "image_records");
        assert_eq!(check.status, "OK");
        assert!(check.message.contains("no image records"), "{check:?}");
    }

    /// All recorded tags present in the store → OK.
    #[test]
    fn all_recorded_tags_present_is_ok() {
        let state = state_with(&[("personal", "workestrate-pi:latest")]);
        let check = image_records_check(
            &state,
            &StoreListing::Tags(vec!["workestrate-pi:latest".to_string()]),
        );
        assert_eq!(check.status, "OK");
        assert!(check.message.contains("1 image record(s)"), "{check:?}");
    }

    /// A recorded tag missing from the store → WARN (never FAIL) with the
    /// remediation naming the repo and the build verb.
    #[test]
    fn missing_recorded_tag_is_warn_with_remediation() {
        let state = state_with(&[
            ("personal", "workestrate-pi:latest"),
            ("work", "tempest:latest"),
        ]);
        let check = image_records_check(
            &state,
            &StoreListing::Tags(vec!["workestrate-pi:latest".to_string()]),
        );
        assert_eq!(check.status, "WARN", "read-only hook never FAILs");
        assert!(check.message.contains("work#tempest:latest"), "{check:?}");
        let remediation = check.remediation.expect("WARN carries remediation");
        assert!(
            remediation.contains("workestrate workload build --repo work"),
            "{remediation}"
        );
        assert!(remediation.contains("load-images"), "{remediation}");
    }

    /// An unreadable store with records present → WARN, never FAIL.
    #[test]
    fn unreachable_store_is_warn_never_fail() {
        let state = state_with(&[("personal", "workestrate-pi:latest")]);
        let check = image_records_check(
            &state,
            &StoreListing::Unreachable("failed to spawn 'msb': No such file".to_string()),
        );
        assert_eq!(check.status, "WARN");
        assert!(check.message.contains("could not be read"), "{check:?}");
        assert!(check.remediation.is_some());
    }

    // ---- MSB_HOME convergence (Step 1): skew + downgrade + message ----

    /// Downgrade classifier: matches schema/newer/migration signals
    /// case-insensitively, ignores unrelated failures.
    #[test]
    fn store_error_classifier_matches_downgrade_signals() {
        for signal in [
            "database version 7 is newer than supported (max 5)",
            "DOWNGRADE detected: store schema unsupported",
            "unsupported schema version 9",
            "migration required before open",
            "Error: Database Version mismatch",
        ] {
            assert!(
                classify_msb_store_error(signal),
                "must match downgrade signal: {signal}"
            );
        }
        for benign in [
            "",
            "connection refused",
            "no such file or directory",
            "permission denied",
            "No sandboxes found.",
        ] {
            assert!(
                !classify_msb_store_error(benign),
                "must not match benign output: {benign}"
            );
        }
    }

    /// Pure skew classifier over DB-file existence.
    #[test]
    fn skew_classifier_covers_all_existence_combos() {
        assert_eq!(
            classify_msb_home_skew(true, true),
            MsbHomeSkew::Skewed,
            "both DBs → skewed"
        );
        assert_eq!(
            classify_msb_home_skew(true, false),
            MsbHomeSkew::Unmigrated,
            "legacy only → unmigrated"
        );
        assert_eq!(
            classify_msb_home_skew(false, true),
            MsbHomeSkew::Clean,
            "canonical only → clean"
        );
        assert_eq!(
            classify_msb_home_skew(false, false),
            MsbHomeSkew::Clean,
            "neither → clean (fresh)"
        );
    }

    /// Path-taking skew wrapper: same-path edge is Clean; real skew and
    /// unmigrated layouts are detected via temp dirs.
    #[test]
    fn skew_status_reads_db_files() {
        let base = std::env::temp_dir().join(format!(
            "workestrate-doctor-skew-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        let legacy_db = base.join("legacy").join("db").join("msb.db");
        let canon_db = base.join("canon").join("db").join("msb.db");
        // Same path → Clean even when the file exists.
        std::fs::create_dir_all(legacy_db.parent().unwrap()).unwrap();
        std::fs::write(&legacy_db, b"db").unwrap();
        assert_eq!(
            msb_home_skew_status(&legacy_db, &legacy_db),
            MsbHomeSkew::Clean
        );
        // Legacy only → Unmigrated.
        assert_eq!(
            msb_home_skew_status(&legacy_db, &canon_db),
            MsbHomeSkew::Unmigrated
        );
        // Both → Skewed.
        std::fs::create_dir_all(canon_db.parent().unwrap()).unwrap();
        std::fs::write(&canon_db, b"db").unwrap();
        assert_eq!(
            msb_home_skew_status(&legacy_db, &canon_db),
            MsbHomeSkew::Skewed
        );
        // Canonical only → Clean.
        std::fs::remove_file(&legacy_db).unwrap();
        assert_eq!(
            msb_home_skew_status(&legacy_db, &canon_db),
            MsbHomeSkew::Clean
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    /// The OK message carries both the resolved MSB_HOME and the probed
    /// msb version; resolve_msb_home honors non-empty MSB_HOME and treats
    /// empty as unset (SDK resolve_home mirror).
    #[test]
    fn msb_message_carries_home_and_version() {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _guard = crate::config::test_support::EnvGuard::capture(&["MSB_HOME", "HOME"]);
        let fake_home = crate::config::test_support::uniq_dir("doctor-msb-home");
        std::env::set_var("HOME", &fake_home);
        std::env::remove_var("MSB_HOME");
        assert_eq!(
            resolve_msb_home(),
            fake_home.join(".microsandbox"),
            "unset MSB_HOME → $HOME/.microsandbox"
        );
        std::env::set_var("MSB_HOME", "");
        assert_eq!(
            resolve_msb_home(),
            fake_home.join(".microsandbox"),
            "empty MSB_HOME → treated as unset"
        );
        let custom = crate::config::test_support::uniq_dir("doctor-msb-custom");
        std::env::set_var("MSB_HOME", &custom);
        assert_eq!(resolve_msb_home(), custom, "set MSB_HOME → verbatim");
        let msg = msb_ok_message(&custom, "msb 0.6.16");
        assert!(msg.contains(&custom.display().to_string()), "{msg}");
        assert!(msg.contains("msb 0.6.16"), "{msg}");
        assert_eq!(
            legacy_msb_home(),
            fake_home.join(".cache").join("ai-workbench-msb"),
            "legacy home is anchored at $HOME/.cache"
        );
    }

    // ---- nested_virt inventory check (ADR 0036 §4) ----

    /// The check reads the LIVE host probe, so the test pins SHAPE only
    /// (host-independent): the name, the closed status vocabulary, the pin
    /// note in every message, and remediation on every non-OK verdict.
    #[test]
    fn nested_virt_check_shape_is_host_independent() {
        let check = doctor_check_nested_virt();
        assert_eq!(check.name, "nested_virt");
        assert!(
            ["OK", "WARN", "FAIL"].contains(&check.status),
            "closed status vocabulary: {:?}",
            check
        );
        assert!(
            check.message.contains("78fb3ed1"),
            "every verdict carries the fork pin note: {check:?}"
        );
        // Honest Phase 1 strings: the VMM flag is inert until fw lands.
        assert!(
            check.message.contains("inert until the Phase 2"),
            "pin note states Phase honesty: {check:?}"
        );
        if check.status != "OK" {
            assert!(
                check.remediation.as_deref().unwrap_or("").contains("modprobe"),
                "non-OK carries remediation: {check:?}"
            );
        }
    }
}
