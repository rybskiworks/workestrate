//! `workestrate versions` — runtime artifact identities and provenance.
//!
//! Prints the four versioned runtime artifacts plus provenance in one place:
//! the baked workestrate rev (`WORKESTRATE_VERSION`), the msb binary version,
//! the agentd content hash, and the libkrunfw soname, plus the
//! msb store DB schema marker and the pinned fork rev.
//!
//! Every probe is read-only and degrades gracefully: a missing binary, a
//! missing DB, or a missing `sqlite3` yields an `"unavailable"`-style marker,
//! never an error. Nothing here migrates or writes anything.

use std::io::Read;
use std::path::Path;

use anyhow::Result;
use sha2::{Digest, Sha256};

use crate::commands::doctor::{msb_binary, probe_version, resolve_msb_home};

/// Expected Microsandbox version. The static pin guard checks the Cargo
/// dependency pins; Nix also checks the fork's package and SDK versions.
pub const MSB_VERSION_PIN: &str = "0.6.18";

/// Hand-maintained fork rev pin (full rev). Must agree with the
/// `microsandbox-fork` input in `flake.nix` / `flake.lock` (enforced by
/// `scripts/check-msb-versions.sh`).
pub const FORK_REV_PIN: &str = "8ae14c22963c0680b231f61280f43db364693a5c";

/// Expected libkrunfw soname shipped by the fork's runtime package.
pub const LIBKRUNFW_SONAME: &str = "libkrunfw.so.5.6.1";

/// Short (8-char) form of [`FORK_REV_PIN`] for human output.
pub fn fork_rev_short() -> String {
    FORK_REV_PIN.chars().take(8).collect()
}

/// Parse a baked `WORKESTRATE_VERSION` string (`"<version>-<rev>"`, see
/// `control/agentctl/build.rs`) into `(version, rev)`. A string without a
/// `-` separator yields `(s, "")` instead of failing.
pub fn parse_workestrate_version(s: &str) -> (String, String) {
    match s.split_once('-') {
        Some((version, rev)) => (version.to_string(), rev.to_string()),
        None => (s.to_string(), String::new()),
    }
}

/// Pure version-match classifier: true when the probed version string
/// contains the baked pin (case-insensitive). Empty inputs never match.
pub fn classify_version_match(baked_pin: &str, probed: &str) -> bool {
    if baked_pin.is_empty() || probed.is_empty() {
        return false;
    }
    probed.to_lowercase().contains(&baked_pin.to_lowercase())
}

/// Basename of a path string (everything after the last `/`).
fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Pure libkrunfw pick logic over a directory listing (file names or full
/// paths): prefer the exact expected soname, then the ABI-major link, then
/// the generic link, else the first sorted `libkrunfw.so*` entry.
pub fn pick_libkrunfw(candidates: &[String]) -> Option<String> {
    let mut hits: Vec<&String> = candidates
        .iter()
        .filter(|c| file_name(c).starts_with("libkrunfw.so"))
        .collect();
    hits.sort();
    for want in [LIBKRUNFW_SONAME, "libkrunfw.so.5", "libkrunfw.so"] {
        if let Some(hit) = hits.iter().find(|c| file_name(c) == want) {
            return Some((*hit).clone());
        }
    }
    hits.first().map(|s| (*s).clone())
}

/// Resolve a bare binary name against `PATH` (or verify an explicit path).
/// Returns `None` when the binary cannot be located.
fn resolve_on_path(bin: &str) -> Option<String> {
    if bin.contains('/') {
        let p = Path::new(bin);
        if p.is_file() {
            if let Ok(canonical) = p.canonicalize() {
                return Some(canonical.to_string_lossy().into_owned());
            }
            return Some(bin.to_string());
        }
        return None;
    }
    let path_env = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_env) {
        let candidate = dir.join(bin);
        if candidate.is_file() {
            if let Ok(canonical) = candidate.canonicalize() {
                return Some(canonical.to_string_lossy().into_owned());
            }
            return Some(candidate.to_string_lossy().into_owned());
        }
    }
    None
}

/// True for a regular file with any executable bit set. Read-only metadata stat.
#[cfg(unix)]
fn is_executable(path: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Non-Unix fallback: a regular file. Read-only metadata stat.
#[cfg(not(unix))]
fn is_executable(path: &str) -> bool {
    Path::new(path).is_file()
}

/// Short content hash (first 12 hex characters), without spawning a process.
/// Reads with bounded memory and returns `None` if the file cannot be read.
fn short_sha256(path: &str) -> Option<String> {
    let mut file = std::fs::File::open(path).ok()?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 8192];
    loop {
        let count = file.read(&mut buffer).ok()?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Some(format!("{:x}", digest.finalize())[..12].to_owned())
}

/// The agentd binary path: `MSB_AGENTD_PATH` when set and non-empty, else an
/// `agentd` resolved on `PATH`, else an `"unavailable ..."` note.
pub fn agentd_path() -> String {
    if let Ok(v) = std::env::var("MSB_AGENTD_PATH")
        && !v.trim().is_empty()
    {
        return v;
    }
    if let Some(p) = resolve_on_path("agentd") {
        return p;
    }
    "unavailable (MSB_AGENTD_PATH unset, agentd not on PATH)".to_string()
}

/// The agentd identity: a `sha256:<12>` content hash when executable, otherwise
/// `executable:<bool>`. Never execute agentd: it is a guest bootstrap binary,
/// not a command with a version-only entry point.
/// Never fails — degrades to `"unavailable"` for non-path inputs.
pub fn agentd_version_or_sha(path: &str) -> String {
    if path.is_empty() || path.starts_with("unavailable") {
        return "unavailable".to_string();
    }
    let executable = is_executable(path);
    if executable && let Some(sha) = short_sha256(path) {
        return format!("sha256:{sha}");
    }
    format!("executable:{executable}")
}

/// The libkrunfw search directory: the `bin/` parent of the msb binary path
/// with `../lib` appended (`$MSB_PATH` when set, else the resolved msb).
/// Returns `"unavailable"` when no msb path resolves.
pub fn libkrunfw_search_dir() -> String {
    let msb = std::env::var("MSB_PATH")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .or_else(|| resolve_on_path(&msb_binary()));
    match msb {
        Some(bin) => match Path::new(&bin).parent() {
            Some(parent) => parent.join("../lib").to_string_lossy().into_owned(),
            None => "unavailable".to_string(),
        },
        None => "unavailable".to_string(),
    }
}

/// Resolve the libkrunfw soname chain inside `search_dir`: read the
/// directory, pick via [`pick_libkrunfw`], and follow a symlink one level
/// for display. Returns `"unavailable"` when the dir is unreadable or holds
/// no `libkrunfw.so*` entry. Read-only.
pub fn resolve_libkrunfw(search_dir: &str) -> String {
    if search_dir.is_empty() || search_dir == "unavailable" {
        return "unavailable".to_string();
    }
    let dir = Path::new(search_dir);
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return "unavailable".to_string(),
    };
    let mut names: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        names.push(entry.path().to_string_lossy().into_owned());
    }
    let picked = match pick_libkrunfw(&names) {
        Some(p) => p,
        None => return "unavailable".to_string(),
    };
    let picked_path = Path::new(&picked);
    match std::fs::read_link(picked_path) {
        Ok(target) => {
            let resolved = if target.is_absolute() {
                target
            } else if let Some(parent) = picked_path.parent() {
                parent.join(&target)
            } else {
                target.clone()
            };
            format!("{} -> {}", picked, resolved.to_string_lossy().into_owned())
        }
        Err(_) => picked,
    }
}

/// Read-only DB schema marker for a msb store DB path: the newest
/// `seaql_migrations` name via `sqlite3` when available (`mig:<name>`), else
/// the file mtime (`mtime:<secs>`), else `"unavailable ..."` for a missing
/// DB. The DB is opened with `sqlite3 -readonly` so the probe can never
/// write, migrate, or create; the query is a plain SELECT and the `is_file`
/// precheck prevents creation.
pub fn db_schema_marker(db: &Path) -> String {
    if !db.is_file() {
        return "unavailable (no DB)".to_string();
    }
    let out = std::process::Command::new("sqlite3")
        .arg("-readonly")
        .arg(db)
        .arg("SELECT name FROM seaql_migrations ORDER BY name DESC LIMIT 1;")
        .output();
    if let Ok(out) = out
        && out.status.success()
    {
        let first = String::from_utf8_lossy(&out.stdout)
            .lines()
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        if !first.is_empty() {
            return format!("mig:{first}");
        }
    }
    match std::fs::metadata(db) {
        Ok(meta) => match meta.modified() {
            Ok(mtime) => match mtime.duration_since(std::time::UNIX_EPOCH) {
                Ok(d) => format!("mtime:{}", d.as_secs()),
                Err(_) => "mtime:0".to_string(),
            },
            Err(_) => "mtime:0".to_string(),
        },
        Err(_) => "unavailable".to_string(),
    }
}

/// Baked workestrate version (`WORKESTRATE_VERSION` = `<version>-<rev>`).
#[derive(Debug, Clone, serde::Serialize)]
pub struct WorkestrateVersion {
    pub version_string: String,
    pub version: String,
    pub rev: String,
}

/// msb probe: the configured binary, its resolved path, and its version.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MsbInfo {
    pub path_used: String,
    pub resolved_path: String,
    pub version: String,
}

/// agentd probe: the binary path and its version-or-identity.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentdInfo {
    pub path: String,
    pub version_or_sha: String,
}

/// libkrunfw probe: the search dir and the resolved soname chain.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LibkrunfwInfo {
    pub search_dir: String,
    pub resolved: String,
}

/// msb store DB schema probe: the home, DB path, and schema marker.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DbSchemaInfo {
    pub home: String,
    pub db_path: String,
    pub marker: String,
}

/// The full Phase-0 observability quadruple plus provenance.
#[derive(Debug, Clone, serde::Serialize)]
pub struct VersionsReport {
    pub workestrate: WorkestrateVersion,
    pub msb: MsbInfo,
    pub agentd: AgentdInfo,
    pub libkrunfw: LibkrunfwInfo,
    pub db_schema: DbSchemaInfo,
    pub msb_home: String,
    pub fork_rev_pin: String,
}

/// Collect every version probe. Total and read-only: each leg degrades to an
/// `"unavailable"`-style marker instead of failing.
pub fn collect_versions() -> VersionsReport {
    let baked = env!("WORKESTRATE_VERSION");
    let (version, rev) = parse_workestrate_version(baked);
    let msb_path = msb_binary();
    let msb_resolved = resolve_on_path(&msb_path).unwrap_or_else(|| "unavailable".to_string());
    let msb_version = probe_version(&msb_path).unwrap_or_else(|| "unavailable".to_string());
    let agentd = agentd_path();
    let agentd_v = agentd_version_or_sha(&agentd);
    let search_dir = libkrunfw_search_dir();
    let libkrunfw_resolved = resolve_libkrunfw(&search_dir);
    let home = resolve_msb_home();
    let db_path = home.join("db").join("msb.db");
    let marker = db_schema_marker(&db_path);
    VersionsReport {
        workestrate: WorkestrateVersion {
            version_string: baked.to_string(),
            version,
            rev,
        },
        msb: MsbInfo {
            path_used: msb_path,
            resolved_path: msb_resolved,
            version: msb_version,
        },
        agentd: AgentdInfo {
            path: agentd,
            version_or_sha: agentd_v,
        },
        libkrunfw: LibkrunfwInfo {
            search_dir,
            resolved: libkrunfw_resolved,
        },
        db_schema: DbSchemaInfo {
            home: home.to_string_lossy().into_owned(),
            db_path: db_path.to_string_lossy().into_owned(),
            marker,
        },
        msb_home: home.to_string_lossy().into_owned(),
        fork_rev_pin: FORK_REV_PIN.to_string(),
    }
}

/// `workestrate versions` — print the quadruple plus provenance lines, or a
/// pretty JSON object with `--json`.
pub fn cmd_versions(json: bool) -> Result<()> {
    let v = collect_versions();
    if json {
        println!("{}", serde_json::to_string_pretty(&v)?);
    } else {
        println!(
            "workestrate: {} (rev {})",
            v.workestrate.version_string, v.workestrate.rev
        );
        println!("msb: {} ({})", v.msb.path_used, v.msb.version);
        println!("  resolved: {}", v.msb.resolved_path);
        println!("agentd: {} ({})", v.agentd.path, v.agentd.version_or_sha);
        println!(
            "libkrunfw: {} (soname {}, search {})",
            v.libkrunfw.resolved, LIBKRUNFW_SONAME, v.libkrunfw.search_dir
        );
        println!(
            "db-schema: {} ({})",
            v.db_schema.marker, v.db_schema.db_path
        );
        println!("msb-home: {}", v.msb_home);
        println!(
            "fork-rev-pin: {} (short {})",
            v.fork_rev_pin,
            fork_rev_short()
        );
        println!("msb-pin: {MSB_VERSION_PIN}");
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

    fn strings(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    // ---- parse_workestrate_version (pure) ----

    #[test]
    fn parses_baked_version_string_into_version_and_rev() {
        assert_eq!(
            parse_workestrate_version("0.1.0-abc12345"),
            ("0.1.0".to_string(), "abc12345".to_string())
        );
        assert_eq!(
            parse_workestrate_version("0.1.0"),
            ("0.1.0".to_string(), String::new())
        );
        assert_eq!(
            parse_workestrate_version("0.1.0-dirty"),
            ("0.1.0".to_string(), "dirty".to_string())
        );
    }

    // ---- classify_version_match (pure) ----

    #[test]
    fn version_match_is_case_insensitive_contains() {
        assert!(classify_version_match("0.6.16", "msb 0.6.16"));
        assert!(classify_version_match("0.6.16", "MSB 0.6.16 (release)"));
        assert!(!classify_version_match("0.6.16", "msb 0.5.6"));
        assert!(!classify_version_match("0.6.16", "unavailable"));
        assert!(!classify_version_match("", "msb 0.6.16"));
        assert!(!classify_version_match("0.6.16", ""));
    }

    // ---- pick_libkrunfw (pure, over fake dir listings) ----

    #[test]
    fn libkrunfw_pick_prefers_the_exact_soname() {
        let listing = strings(&[
            "/lib/libkrunfw.so",
            "/lib/libkrunfw.so.5",
            "/lib/libkrunfw.so.5.6.1",
        ]);
        assert_eq!(
            pick_libkrunfw(&listing),
            Some("/lib/libkrunfw.so.5.6.1".to_string())
        );
    }

    #[test]
    fn libkrunfw_pick_falls_back_down_the_link_chain() {
        let listing = strings(&["/lib/libkrunfw.so", "/lib/libkrunfw.so.5"]);
        assert_eq!(
            pick_libkrunfw(&listing),
            Some("/lib/libkrunfw.so.5".to_string())
        );
        let listing = strings(&["/lib/libkrunfw.so"]);
        assert_eq!(
            pick_libkrunfw(&listing),
            Some("/lib/libkrunfw.so".to_string())
        );
    }

    #[test]
    fn libkrunfw_pick_ignores_non_matches_and_empty_listings() {
        let listing = strings(&["/lib/libcap.so", "/lib/other.so.1"]);
        assert_eq!(pick_libkrunfw(&listing), None);
        let empty: Vec<String> = Vec::new();
        assert_eq!(pick_libkrunfw(&empty), None);
    }

    // ---- fork_rev_short (pure) ----

    #[test]
    fn fork_short_rev_is_the_first_eight_chars() {
        assert_eq!(fork_rev_short(), &FORK_REV_PIN[..8]);
        assert_eq!(fork_rev_short().len(), 8);
    }

    #[cfg(unix)]
    #[test]
    fn agentd_identity_hashes_without_executing_the_guest_binary() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let binary = directory.path().join("agentd");
        std::fs::write(&binary, b"#!/bin/sh\nprintf executed > \"$0.executed\"\n").unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o755)).unwrap();
        let expected = format!("sha256:{}", short_sha256(binary.to_str().unwrap()).unwrap());
        let marker = binary.with_extension("executed");

        // Prove the synthetic fixture would expose the old execution path;
        // a missing interpreter must not make the negative assertion vacuous.
        assert!(
            std::process::Command::new(&binary)
                .arg("--version")
                .status()
                .unwrap()
                .success()
        );
        assert!(marker.is_file());
        std::fs::remove_file(&marker).unwrap();

        assert_eq!(agentd_version_or_sha(binary.to_str().unwrap()), expected);
        assert!(!marker.exists());
    }

    #[test]
    fn agentd_hash_matches_known_sha256() {
        let directory = tempfile::tempdir().unwrap();
        let binary = directory.path().join("agentd");
        std::fs::write(&binary, b"abc").unwrap();
        assert_eq!(
            short_sha256(binary.to_str().unwrap()),
            Some("ba7816bf8f01".to_owned())
        );
    }

    #[test]
    fn absent_agentd_and_directories_have_no_executable_identity() {
        let directory = tempfile::tempdir().unwrap();
        assert_eq!(agentd_version_or_sha(""), "unavailable");
        assert_eq!(
            agentd_version_or_sha("unavailable (no agentd)"),
            "unavailable"
        );
        assert_eq!(
            agentd_version_or_sha(directory.path().to_str().unwrap()),
            "executable:false"
        );
        assert_eq!(
            agentd_version_or_sha(directory.path().join("missing").to_str().unwrap()),
            "executable:false"
        );
    }

    #[cfg(unix)]
    #[test]
    fn nonexecutable_agentd_remains_a_metadata_result() {
        use std::os::unix::fs::PermissionsExt;

        let directory = tempfile::tempdir().unwrap();
        let binary = directory.path().join("agentd");
        std::fs::write(&binary, b"abc").unwrap();
        std::fs::set_permissions(&binary, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(
            agentd_version_or_sha(binary.to_str().unwrap()),
            "executable:false"
        );
    }

    // ---- collect_versions shape (host-independent) ----

    #[test]
    fn versions_report_carries_the_pins_and_quadruple_shape() {
        let v = collect_versions();
        assert_eq!(v.fork_rev_pin, FORK_REV_PIN);
        assert!(!v.workestrate.version_string.is_empty());
        assert!(!v.msb.path_used.is_empty());
        assert!(!v.agentd.path.is_empty());
        assert!(!v.msb_home.is_empty());
        // JSON shape: the doctor versions payload keys.
        let json = serde_json::to_value(&v).unwrap();
        for key in [
            "workestrate",
            "msb",
            "agentd",
            "libkrunfw",
            "db_schema",
            "msb_home",
            "fork_rev_pin",
        ] {
            assert!(json.get(key).is_some(), "missing versions key: {key}");
        }
    }
}
