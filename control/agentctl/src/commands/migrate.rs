//! `workestrate migrate-home` (ADR 0023) plus its summary renderer and
//! sizing helpers.

use std::path::Path;

use anyhow::Result;

use crate::config;

/// Recursive byte size of a directory tree (files only); 0 on error.
pub(crate) fn entry_bytes(path: &Path) -> u64 {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return 0,
    };
    if meta.is_file() {
        return meta.len();
    }
    if meta.is_dir() {
        let mut total: u64 = 0;
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                total += entry_bytes(&entry.path());
            }
        }
        return total;
    }
    meta.len()
}

pub(crate) fn render_migrate_summary_human(summary: &config::MigrateSummary) {
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
pub(crate) fn cmd_migrate_home(
    from: Option<&str>,
    dry_run: bool,
    json: bool,
    force: bool,
) -> Result<()> {
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
