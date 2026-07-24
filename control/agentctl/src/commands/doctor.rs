//! `workestrate doctor` — environment/tool health checks and the
//! `DoctorCheck` result struct.

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config;
use crate::scaffold;

/// One doctor check result. `status` is "OK", "WARN", or "FAIL"; a FAIL
/// anywhere flips the overall verdict and the process exit code to 1.
#[derive(serde::Serialize)]
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
