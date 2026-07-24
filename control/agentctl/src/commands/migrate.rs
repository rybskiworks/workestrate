//! `workestrate migrate-home` (ADR 0023) plus its summary renderer and
//! sizing helpers.

use std::path::Path;

use anyhow::Result;

use crate::config;

/// Recursive byte size of a directory tree (files only); 0 on error.
///
/// FS-25: read_dir failures mid-recursion no longer silently truncate the
/// total — a one-time stderr note names the unreadable directory (the size
/// still returns the readable partial total; this is a best-effort
/// informational report, not an error path).
pub fn entry_bytes(path: &Path) -> u64 {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return 0,
    };
    if meta.is_file() {
        return meta.len();
    }
    if meta.is_dir() {
        let mut total: u64 = 0;
        match std::fs::read_dir(path) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    total += entry_bytes(&entry.path());
                }
            }
            Err(e) => {
                emit_entry_bytes_warn(path, &e);
            }
        }
        return total;
    }
    meta.len()
}

/// One-time stderr note for a directory that could not be read mid-recursion
/// (FS-25). migrate-home sizing is informational; the note keeps a
/// permission-denied subdir from silently shrinking the reported total.
static ENTRY_BYTES_WARN: std::sync::Once = std::sync::Once::new();

fn emit_entry_bytes_warn(path: &Path, e: &std::io::Error) {
    ENTRY_BYTES_WARN.call_once(|| {
        eprintln!(
            "note: could not fully read {} while sizing ({}); reported byte total is a partial sum",
            path.display(),
            e
        );
    });
}

pub fn render_migrate_summary_human(summary: &config::MigrateSummary) {
    println!("migrate-home");
    println!("  source layout: {}", summary.from);
    println!("  destination:   {}", summary.dest);
    println!(
        "  mode:          {}",
        if summary.dry_run {
            "dry-run (nothing moved)"
        } else {
            "applied"
        }
    );
    println!("  entries:       {}", summary.moved.len());
    for m in &summary.moved {
        let src = Path::new(&m.src);
        let (exists, bytes) = match std::fs::metadata(src) {
            Ok(_) => (true, entry_bytes(src)),
            Err(_) => (false, 0),
        };
        println!(
            "    {} -> {} (exists: {}, bytes: {})",
            m.src, m.dst, exists, bytes
        );
    }
    if summary.partial {
        println!("  WARNING:       partial failure — some entries could not be moved");
        println!(
            "  failed at:     {}",
            summary.failed_at.as_deref().unwrap_or("(unknown)")
        );
    }
    if !summary.dry_run {
        println!(
            "  registry updated: {} (home_version: {})",
            summary.registry_updated,
            match summary.home_version {
                Some(v) => v.to_string(),
                None => "absent".to_string(),
            }
        );
        let urls = if summary.urls_rewritten.is_empty() {
            "(none)".to_string()
        } else {
            summary.urls_rewritten.join(", ")
        };
        println!("  URLs rewritten:  {}", urls);
    }
}

/// `workestrate migrate-home`: consolidate a legacy layout into a single
/// `WORKESTRATE_HOME` (ADR 0023). See `config::run_migrate_home` for the
/// mechanics; this fn only resolves the destination and renders the summary.
pub fn cmd_migrate_home(from: Option<&str>, dry_run: bool, json: bool, force: bool) -> Result<()> {
    let wh = std::env::var("WORKESTRATE_HOME")
        .ok()
        .filter(|v| !v.is_empty());
    let dest = if let Some(home) = wh {
        config::expand_tilde(&home)
    } else if from == Some("bundle") {
        let cwd = std::env::current_dir()?;
        let candidate = cwd.join(".workestrate");
        if candidate.is_dir() {
            candidate
        } else {
            config::resolve_home()
        }
    } else {
        config::resolve_home()
    };

    let summary = config::run_migrate_home(from, &dest, dry_run, force)?;

    if json {
        let body = serde_json::to_string(&summary)
            .map_err(|e| anyhow::anyhow!("failed to serialize migrate summary: {}", e))?;
        println!("{}", body);
    } else {
        render_migrate_summary_human(&summary);
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

    // ---- FS-25: entry_bytes counts readable files and survives recursion ----

    /// A small tree of known sizes sums exactly; this pins the counting
    /// contract that the FS-25 read_dir error note must not perturb on the
    /// happy path.
    #[test]
    fn entry_bytes_sums_files_recursively() -> Result<()> {
        let root = std::env::temp_dir().join(format!(
            "workestrate-fs25-bytes-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(root.join("sub").join("deep"))?;
        std::fs::write(root.join("a.bin"), vec![0u8; 100])?;
        std::fs::write(root.join("sub").join("b.bin"), vec![0u8; 250])?;
        std::fs::write(root.join("sub").join("deep").join("c.bin"), vec![0u8; 50])?;

        assert_eq!(entry_bytes(&root), 400, "files only, summed recursively");

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    /// An unreadable path contributes 0 (pre-existing contract, unchanged);
    /// a mid-recursion read_dir failure yields the PARTIAL readable sum (the
    /// one-time stderr note fires but is not captured here).
    #[test]
    fn entry_bytes_missing_path_is_zero() -> Result<()> {
        let missing = std::env::temp_dir().join(format!(
            "workestrate-fs25-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        assert_eq!(entry_bytes(&missing), 0);
        Ok(())
    }

    /// Mid-recursion failure: a permission-denied subdirectory contributes 0
    /// while its readable sibling still counts — the total is partial but
    /// the function does not panic or swallow the whole tree.
    #[cfg(unix)]
    #[test]
    fn entry_bytes_unreadable_subdir_yields_partial_sum() -> Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let root = std::env::temp_dir().join(format!(
            "workestrate-fs25-denied-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let open_dir = root.join("open");
        let denied = root.join("denied");
        std::fs::create_dir_all(&open_dir)?;
        std::fs::create_dir_all(&denied)?;
        std::fs::write(open_dir.join("a.bin"), vec![0u8; 128])?;
        std::fs::write(denied.join("secret.bin"), vec![0u8; 999])?;
        std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o000))?;

        let total = entry_bytes(&root);
        assert_eq!(
            total, 128,
            "unreadable subdir contributes 0; readable sibling still counts"
        );

        // Restore permissions so cleanup can remove the tree.
        std::fs::set_permissions(&denied, std::fs::Permissions::from_mode(0o755))?;
        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }
}
