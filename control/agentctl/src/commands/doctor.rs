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
    match probe_version(&bin) {
        Some(version) => DoctorCheck::new("msb", "OK", version),
        None => DoctorCheck::new("msb", "FAIL", format!("'{}' not reachable", bin))
            .with_remediation(
                "Ensure msb is installed; run 'nix develop' or check MSB_HOME/MSB_PATH",
            ),
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

/// `workestrate doctor` — run environment/tool health checks (KVM, nix,
/// sops, age, msb, home resolution, config repos), print a human report or a
/// machine-readable JSON document, and exit non-zero when any check FAILs.
pub fn cmd_doctor(json: bool) -> Result<()> {
    let checks = vec![
        doctor_check_kvm(),
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
                    let name = repo["name"].as_str().unwrap_or("unknown");
                    let status = repo["status"].as_str().unwrap_or("unknown");
                    let message = repo["message"].as_str().unwrap_or("");
                    println!("  {}: {} ({})", name, status, message);
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
}
