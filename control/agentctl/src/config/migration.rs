//! `workestrate migrate-home` (ADR 0023): consolidate legacy layouts into a
//! single tool home.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::config::paths::{xdg_config_dir, xdg_data_dir, xdg_state_dir};
use crate::config::types::Registry;

/// One moved file/dir recorded by [`run_migrate_home`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct MovedEntry {
    pub src: String,
    pub dst: String,
}

/// Structured summary of a `workestrate migrate-home` run (ADR 0023).
///
/// The migration is **non-transactional**: if a move fails mid-loop, the
/// entries already moved are not rolled back. In that case `partial` is
/// `true`, `failed_at` names the destination that could not be moved, and
/// `moved` lists the entries that succeeded up to that point. The remaining
/// planned entries (not in `moved`) were skipped.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MigrateSummary {
    pub from: String,
    pub dest: String,
    pub dry_run: bool,
    pub moved: Vec<MovedEntry>,
    pub registry_updated: bool,
    pub home_version: Option<u32>,
    /// Names of `configs.<name>` entries whose local `url` pointed into the
    /// old layout and was rewritten to the new `config-repos/<name>` path.
    /// Empty in
    /// dry-run (no editing happens) and when no local urls matched.
    pub urls_rewritten: Vec<String>,
    /// True when the migration moved some entries but aborted mid-loop (see
    /// `failed_at`). The move is NOT transactional: entries already moved stay
    /// moved. Always `false` on full success and in dry-run mode.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub partial: bool,
    /// Destination path of the entry whose move failed when `partial` is true.
    /// `None` on full success and in dry-run mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_at: Option<String>,
}

/// Source layout for a migration (paths read FROM, into `dest`).
struct MigrateSources {
    registry: PathBuf,
    overrides: PathBuf,
    secrets: PathBuf,
    repos_root: PathBuf,
    sources_root: PathBuf,
    state_root: PathBuf,
    /// Old subtrees to clean up after a successful in-place move (bundle only).
    cleanup_dirs: Vec<PathBuf>,
}

/// Resolve the source layout. `dest` is the destination single-home dir.
///
/// - `from == "xdg"`: legacy `XDG_*_HOME` dirs.
/// - `from == "bundle"`: the old `.workestrate/{config,data,state}/workestrate/`
///   triplication rooted at `dest`'s parent (in-place bundle).
fn resolve_migrate_sources(from: &str, dest: &Path) -> Result<MigrateSources> {
    match from {
        "xdg" => Ok(MigrateSources {
            registry: xdg_config_dir().join("config.toml"),
            overrides: xdg_config_dir().join("overrides.toml"),
            secrets: xdg_config_dir().join(".env.local.enc"),
            repos_root: xdg_data_dir().join("repos"),
            sources_root: xdg_data_dir().join("sources"),
            state_root: xdg_state_dir(),
            cleanup_dirs: Vec::new(),
        }),
        "bundle" => {
            // In-place bundle: dest == <bundle_root>/.workestrate, so the old
            // triplication lives directly under dest as {config,data,state}/workestrate.
            Ok(MigrateSources {
                registry: dest.join("config").join("workestrate").join("config.toml"),
                overrides: dest
                    .join("config")
                    .join("workestrate")
                    .join("overrides.toml"),
                secrets: dest
                    .join("config")
                    .join("workestrate")
                    .join(".env.local.enc"),
                repos_root: dest.join("data").join("workestrate").join("repos"),
                sources_root: dest.join("data").join("workestrate").join("sources"),
                state_root: dest.join("state").join("workestrate"),
                // NOTE: dest/state is BOTH the old state subtree's parent and
                // the NEW state destination (new layout moves files INTO
                // dest/state). So we clean up only the old `state/workestrate`
                // subdir, never `dest/state` itself, to avoid nuking the
                // just-moved state. dest/config and dest/data have no new
                // layout files under them and can be removed wholesale.
                cleanup_dirs: vec![
                    dest.join("config"),
                    dest.join("data"),
                    dest.join("state").join("workestrate"),
                ],
            })
        }
        other => anyhow::bail!(
            "unknown --from value '{}' (expected \"xdg\" or \"bundle\")",
            other
        ),
    }
}

/// Recursively copy an entry (file/dir/symlink). Cross-filesystem fallback for
/// [`move_entry`].
fn copy_entry_recursive(src: &Path, dst: &Path) -> Result<()> {
    let meta = std::fs::symlink_metadata(src)?;
    if meta.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            copy_entry_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        }
    } else if meta.file_type().is_symlink() {
        let target = std::fs::read_link(src)?;
        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(&target, dst);
            if !dst.exists() && std::fs::symlink_metadata(dst).is_err() {
                std::fs::copy(src, dst)?;
            }
        }
        #[cfg(not(unix))]
        {
            std::fs::copy(src, dst)?;
        }
    } else {
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

/// Move a filesystem entry, falling back to recursive copy + delete when a
/// simple `rename` fails (e.g. crossing a filesystem boundary).
fn move_entry(src: &Path, dst: &Path) -> Result<()> {
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    copy_entry_recursive(src, dst)?;
    let meta = std::fs::symlink_metadata(src)
        .map_err(|e| anyhow::anyhow!("stat moved source {}: {}", src.display(), e))?;
    if meta.is_dir() {
        std::fs::remove_dir_all(src)?;
    } else {
        std::fs::remove_file(src)?;
    }
    Ok(())
}

/// Collect the planned (src, dst) moves for a given source layout + dest.
fn plan_moves(sources: &MigrateSources, dest: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut moves: Vec<(PathBuf, PathBuf)> = Vec::new();
    if sources.registry.exists() {
        moves.push((sources.registry.clone(), dest.join("config.toml")));
    }
    if sources.overrides.exists() {
        moves.push((sources.overrides.clone(), dest.join("overrides.toml")));
    }
    if sources.secrets.exists() {
        moves.push((
            sources.secrets.clone(),
            dest.join("secrets").join(".env.local.enc"),
        ));
    }
    if sources.repos_root.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&sources.repos_root) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let p = entry.path();
                moves.push((p, dest.join("config-repos").join(name)));
            }
        }
    }
    if sources.sources_root.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&sources.sources_root) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let p = entry.path();
                moves.push((p, dest.join("sources").join(name)));
            }
        }
    }
    if sources.state_root.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&sources.state_root) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let p = entry.path();
                moves.push((p, dest.join("state").join(name)));
            }
        }
    }
    moves
}

/// Auto-detect the source layout when `--from` is omitted.
///
/// Prefers "bundle" when a `.workestrate/config/workestrate/config.toml` exists
/// in the cwd, otherwise treats the source as the legacy XDG layout.
fn detect_layout() -> &'static str {
    if let Ok(cwd) = std::env::current_dir() {
        if cwd
            .join(".workestrate")
            .join("config")
            .join("workestrate")
            .join("config.toml")
            .exists()
        {
            return "bundle";
        }
    }
    "xdg"
}

/// Detect whether a string looks like a remote URL (carries a scheme) rather
/// than a local filesystem path. Used by [`run_migrate_home`] to avoid
/// rewriting genuine remote git urls stored in `configs.<name>.url`.
///
/// Returns `true` for `http://`, `https://`, `ssh://`, `git@`, `flake://`, or
/// anything else containing a `://` scheme separator.
fn looks_like_remote_url(s: &str) -> bool {
    s.starts_with("http://")
        || s.starts_with("https://")
        || s.starts_with("ssh://")
        || s.starts_with("git@")
        || s.starts_with("flake://")
        || s.contains("://")
}

/// Run a `workestrate migrate-home` consolidation into a single home (ADR 0023).
///
/// `from` is `"xdg"`, `"bundle"`, or `None` (auto-detect). `dest` is the
/// destination single-home dir. In dry-run mode nothing is moved; the returned
/// [`MigrateSummary`] lists the planned moves. On a real run the registry at
/// `dest/config.toml` has `store_dir`/`state_dir` cleared and `home_version`
/// set to `Some(2)`.
pub fn run_migrate_home(
    from: Option<&str>,
    dest: &Path,
    dry_run: bool,
    force: bool,
) -> Result<MigrateSummary> {
    let layout: &str = match from {
        Some(s) => s,
        None => detect_layout(),
    };
    let sources = resolve_migrate_sources(layout, dest)?;
    let planned = plan_moves(&sources, dest);

    if dry_run {
        let moved = planned
            .iter()
            .map(|(src, dst)| MovedEntry {
                src: src.display().to_string(),
                dst: dst.display().to_string(),
            })
            .collect();
        return Ok(MigrateSummary {
            from: layout.to_string(),
            dest: dest.display().to_string(),
            dry_run: true,
            moved,
            registry_updated: false,
            home_version: None,
            urls_rewritten: Vec::new(),
            partial: false,
            failed_at: None,
        });
    }

    // Refuse to clobber an existing home unless --force.
    if dest.join("config.toml").exists() && !force {
        anyhow::bail!(
            "destination {} already contains config.toml; pass --force to overwrite",
            dest.display()
        );
    }

    // Clobber guard: scan ALL planned dst paths (overrides.toml,
    // secrets/.env.local.enc, every config-repos/<name>, sources/<name>,
    // state/<name>). The config.toml primary check above is the fast-path
    // refusal; this catches every other pre-existing destination. --force
    // overrides.
    let existing_dsts: Vec<String> = planned
        .iter()
        // lstat, not stat: `dst.exists()` follows symlinks and returns FALSE
        // for a dangling symlink, so rename(2) would silently clobber one
        // planted at a planned destination (FN-11). `symlink_metadata` is Ok
        // for anything occupying the path, including dangling links.
        .filter(|(_, dst)| std::fs::symlink_metadata(dst).is_ok())
        .map(|(_, dst)| dst.display().to_string())
        .collect();
    if !existing_dsts.is_empty() && !force {
        anyhow::bail!(
            "destination {} already contains {} existing entr{}; \
             pass --force to overwrite:\n  {}",
            dest.display(),
            existing_dsts.len(),
            if existing_dsts.len() == 1 { "y" } else { "ies" },
            existing_dsts.join("\n  ")
        );
    }

    std::fs::create_dir_all(dest)?;
    std::fs::create_dir_all(dest.join("secrets"))?;
    std::fs::create_dir_all(dest.join("state"))?;
    std::fs::create_dir_all(dest.join("config-repos"))?;
    std::fs::create_dir_all(dest.join("sources"))?;

    // Pre-flight: verify every src exists and every dst parent is writable
    // before moving anything. This catches the common failure modes (missing
    // source, unwritable destination parent) up front so we don't move half
    // the tree and then discover a problem.
    for (src, dst) in &planned {
        if !src.exists() {
            anyhow::bail!(
                "pre-flight: source {} does not exist (planned move to {})",
                src.display(),
                dst.display()
            );
        }
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
            // Writability check: create then remove a probe file in the parent.
            let probe = parent.join(format!(".mig-write-probe-{}", std::process::id()));
            match std::fs::write(&probe, b"") {
                Ok(_) => {
                    let _ = std::fs::remove_file(&probe);
                }
                Err(e) => {
                    anyhow::bail!(
                        "pre-flight: destination parent {} is not writable: {}",
                        parent.display(),
                        e
                    );
                }
            }
        }
    }

    // Execute moves. Non-transactional: on mid-loop failure, entries already
    // moved stay moved; we return a partial summary instead of propagating the
    // error so the caller knows what succeeded.
    let mut moved: Vec<MovedEntry> = Vec::new();
    for (src, dst) in &planned {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if let Err(e) = move_entry(src, dst) {
            let moved_so_far = moved.len();
            let remaining = planned.len() - moved_so_far - 1;
            eprintln!(
                "warning: migrate-home failed moving {} -> {}: {} \
                 (moved {} entr{}, {} remaining, non-transactional)",
                src.display(),
                dst.display(),
                e,
                moved_so_far,
                if moved_so_far == 1 { "y" } else { "ies" },
                remaining
            );
            return Ok(MigrateSummary {
                from: layout.to_string(),
                dest: dest.display().to_string(),
                dry_run: false,
                moved,
                registry_updated: false,
                home_version: None,
                urls_rewritten: Vec::new(),
                partial: true,
                failed_at: Some(dst.display().to_string()),
            });
        }
        moved.push(MovedEntry {
            src: src.display().to_string(),
            dst: dst.display().to_string(),
        });
    }

    // Update the relocated registry: drop store_dir/state_dir (derivation from
    // the new home takes over) and stamp home_version = 2.
    let mut registry_updated = false;
    let mut home_version = None;
    let mut urls_rewritten: Vec<String> = Vec::new();
    let reg_path = dest.join("config.toml");
    if reg_path.exists() {
        if let Some(mut reg) = std::fs::read_to_string(&reg_path)
            .ok()
            .and_then(|c| toml::from_str::<Registry>(&c).ok())
        {
            reg.settings.store_dir = None;
            reg.settings.state_dir = None;
            reg.settings.home_version = Some(2);

            // Rewrite `configs.<name>.url` fields that still point into the
            // OLD layout that was just migrated. Only local filesystem paths
            // are considered (remote URLs are left untouched). A url matches
            // when it equals, or lives under, the old repo base
            // `sources.repos_root/<name>`; it is then rewritten to the new
            // `dest/config-repos/<name>` path.
            for (name, entry) in reg.configs.iter_mut() {
                if looks_like_remote_url(&entry.url) {
                    continue;
                }
                let url_path = PathBuf::from(&entry.url);
                let old_base = sources.repos_root.join(name);
                // This site REWRITES urls from the old layout to the new one;
                // it does not resolve a checkout for reading, so
                // `local_entry_checkout_dir` (home-relative resolution) does
                // not apply here.
                // Prefer canonical comparison when both paths still resolve,
                // else fall back to lexical (component-wise) matching. In a
                // real run the old repo dir has already been moved, so both
                // canonicalizations fail and we use the lexical branch.
                let matches = match (url_path.canonicalize(), old_base.canonicalize()) {
                    (Ok(u), Ok(b)) => u == b || u.starts_with(&b),
                    _ => url_path == old_base || url_path.starts_with(&old_base),
                };
                if matches {
                    entry.url = dest
                        .join("config-repos")
                        .join(name)
                        .to_string_lossy()
                        .to_string();
                    urls_rewritten.push(name.clone());
                }
            }
            // Deterministic (alphabetical) output ordering regardless of
            // HashMap iteration order.
            urls_rewritten.sort();

            let toml_str = toml::to_string_pretty(&reg)
                .map_err(|e| anyhow::anyhow!("failed to serialize migrated registry: {}", e))?;
            std::fs::write(&reg_path, toml_str)?;
            registry_updated = true;
            home_version = Some(2);
        }
    }

    // Best-effort cleanup of now-empty old subtrees (bundle in-place).
    for dir in &sources.cleanup_dirs {
        if dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(dir) {
                eprintln!(
                    "warning: could not remove old subtree {}: {}",
                    dir.display(),
                    e
                );
            }
        }
    }

    Ok(MigrateSummary {
        from: layout.to_string(),
        dest: dest.display().to_string(),
        dry_run: false,
        moved,
        registry_updated,
        home_version,
        urls_rewritten,
        partial: false,
        failed_at: None,
    })
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_in_result
    )]
    use super::*;
    use crate::config::test_support::*;

    // ---- ADR 0023: migrate-home coverage ----
    //
    // The resolve_home_* / discovery / xdg home-resolution tests moved to
    // `config::paths::tests` (WP1-1 file split). The migrate-home family below
    // moved here with the migrate code (WP1-3 file split).

    /// Build a full legacy XDG layout under `root` and return the computed
    /// xdg config/data/state dirs. Pins XDG_*_HOME env vars.
    fn build_xdg_layout(root: &Path) -> Result<(PathBuf, PathBuf, PathBuf)> {
        std::env::set_var("XDG_CONFIG_HOME", root.join("xdg-config"));
        std::env::set_var("XDG_DATA_HOME", root.join("xdg-data"));
        std::env::set_var("XDG_STATE_HOME", root.join("xdg-state"));
        std::env::set_var("HOME", root.join("home"));

        let xcfg = xdg_config_dir();
        let xdata = xdg_data_dir();
        let xstate = xdg_state_dir();
        std::fs::create_dir_all(&xcfg)?;
        std::fs::write(
            xcfg.join("config.toml"),
            "[settings]\nstore_dir = \"/old/store\"\n",
        )?;
        std::fs::write(xcfg.join("overrides.toml"), "[global]\n")?;
        std::fs::write(xcfg.join(".env.local.enc"), "ENCRYPTED-BYTES")?;

        std::fs::create_dir_all(xdata.join("repos").join("personal"))?;
        std::fs::write(
            xdata.join("repos").join("personal").join("file.txt"),
            "repo-data",
        )?;
        std::fs::create_dir_all(xdata.join("sources").join("foo"))?;
        std::fs::write(xdata.join("sources").join("foo").join("f.txt"), "src-data")?;

        std::fs::create_dir_all(xstate.join("workspaces"))?;
        std::fs::write(xstate.join("workspaces").join("ws.txt"), "ws-data")?;
        Ok((xcfg, xdata, xstate))
    }

    #[test]
    fn migrate_home_dry_run_xdg() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let root = uniq_dir("mig-dry");
        std::fs::create_dir_all(&root)?;
        let (xcfg, xdata, xstate) = build_xdg_layout(&root)?;
        let dest = root.join("dest");

        let summary = run_migrate_home(Some("xdg"), &dest, true, false)?;
        assert!(summary.dry_run);
        assert!(!summary.registry_updated);
        assert!(summary.moved.iter().any(|m| m.dst.ends_with("config.toml")));
        assert!(summary
            .moved
            .iter()
            .any(|m| m.dst.ends_with("overrides.toml")));
        assert!(summary
            .moved
            .iter()
            .any(|m| m.dst.ends_with(".env.local.enc")));
        assert!(summary
            .moved
            .iter()
            .any(|m| m.dst.ends_with("config-repos/personal")));
        assert!(summary.moved.iter().any(|m| m.dst.ends_with("sources/foo")));
        assert!(summary
            .moved
            .iter()
            .any(|m| m.dst.ends_with("state/workspaces")));

        // Dry-run must touch nothing.
        assert!(xcfg.join("config.toml").exists());
        assert!(xdata
            .join("repos")
            .join("personal")
            .join("file.txt")
            .exists());
        assert!(xstate.join("workspaces").join("ws.txt").exists());
        assert!(!dest.exists());

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn migrate_home_real_move_xdg() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let root = uniq_dir("mig-real");
        std::fs::create_dir_all(&root)?;
        let (xcfg, xdata, _xstate) = build_xdg_layout(&root)?;
        let dest = root.join("dest");

        let summary = run_migrate_home(Some("xdg"), &dest, false, false)?;
        assert!(!summary.dry_run);
        assert!(summary.registry_updated);
        assert_eq!(summary.home_version, Some(2));

        // New single-home layout.
        assert!(dest.join("config.toml").exists());
        assert!(dest.join("overrides.toml").exists());
        assert!(dest.join("secrets").join(".env.local.enc").exists());
        assert!(dest
            .join("config-repos")
            .join("personal")
            .join("file.txt")
            .exists());
        assert!(dest.join("sources").join("foo").join("f.txt").exists());
        assert!(dest
            .join("state")
            .join("workspaces")
            .join("ws.txt")
            .exists());

        // Registry consolidated: store_dir cleared, home_version stamped.
        let reg: Registry = toml::from_str(&std::fs::read_to_string(dest.join("config.toml"))?)?;
        assert_eq!(reg.settings.store_dir, None);
        assert_eq!(reg.settings.state_dir, None);
        assert_eq!(reg.settings.home_version, Some(2));

        // Old sources moved away.
        assert!(!xcfg.join("config.toml").exists());
        assert!(!xcfg.join(".env.local.enc").exists());
        assert!(!xdata
            .join("repos")
            .join("personal")
            .join("file.txt")
            .exists());

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn migrate_home_bundle_inplace() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let root = uniq_dir("mig-bundle");
        let dest = root.join(".workestrate");
        // Old bundle triplication under dest/{config,data,state}/workestrate.
        let bcfg = dest.join("config").join("workestrate");
        let bdata = dest.join("data").join("workestrate");
        let bstate = dest.join("state").join("workestrate");
        std::fs::create_dir_all(&bcfg)?;
        std::fs::write(
            bcfg.join("config.toml"),
            "[settings]\nstore_dir = \"/old\"\n",
        )?;
        std::fs::write(bcfg.join("overrides.toml"), "[global]\n")?;
        std::fs::write(bcfg.join(".env.local.enc"), "ENC")?;
        std::fs::create_dir_all(bdata.join("repos").join("personal"))?;
        std::fs::write(
            bdata.join("repos").join("personal").join("file.txt"),
            "repo",
        )?;
        std::fs::create_dir_all(bdata.join("sources").join("foo"))?;
        std::fs::write(bdata.join("sources").join("foo").join("f.txt"), "src")?;
        std::fs::create_dir_all(bstate.join("workspaces"))?;
        std::fs::write(bstate.join("workspaces").join("ws.txt"), "ws")?;

        let summary = run_migrate_home(Some("bundle"), &dest, false, false)?;
        assert!(summary.registry_updated);
        assert_eq!(summary.from, "bundle");

        // New single-home layout under dest.
        assert!(dest.join("config.toml").exists());
        assert!(dest.join("secrets").join(".env.local.enc").exists());
        assert!(dest
            .join("config-repos")
            .join("personal")
            .join("file.txt")
            .exists());
        assert!(dest.join("sources").join("foo").join("f.txt").exists());
        // State: old state/workestrate moved INTO dest/state (overlap case).
        assert!(dest
            .join("state")
            .join("workspaces")
            .join("ws.txt")
            .exists());

        // Old subtrees removed; dest/state (new) survives.
        assert!(!dest.join("config").exists());
        assert!(!dest.join("data").exists());
        assert!(!dest.join("state").join("workestrate").exists());
        assert!(dest.join("state").exists());

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn migrate_home_rewrites_repo_url_pointing_into_old_layout() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let root = uniq_dir("mig-url");
        let dest = root.join(".workestrate");
        // Old bundle triplication under dest/{config,data}/workestrate.
        let bcfg = dest.join("config").join("workestrate");
        let bdata = dest.join("data").join("workestrate");
        std::fs::create_dir_all(&bcfg)?;

        // Old repo base for <personal> under the bundle layout:
        // dest/data/workestrate/repos/personal
        let old_repo = bdata.join("repos").join("personal");
        std::fs::create_dir_all(&old_repo)?;
        std::fs::write(old_repo.join("file.txt"), "repo")?;
        // Simulate a git checkout so it looks like a real repo dir.
        std::fs::create_dir_all(old_repo.join(".git"))?;
        std::fs::write(old_repo.join(".git").join("HEAD"), "ref: refs/heads/main\n")?;

        // Registry entry whose url points into the OLD layout.
        let old_url = old_repo.to_string_lossy().to_string();
        let toml_reg = format!(
            "[settings]\nstore_dir = \"/old\"\n\
             [configs.personal]\nurl = \"{}\"\nref = \"main\"\n",
            old_url
        );
        std::fs::write(bcfg.join("config.toml"), toml_reg)?;

        let summary = run_migrate_home(Some("bundle"), &dest, false, false)?;
        assert!(summary.registry_updated);
        assert_eq!(summary.from, "bundle");

        // url rewritten to the new repo path.
        assert!(
            summary.urls_rewritten.contains(&"personal".to_string()),
            "expected personal in urls_rewritten, got {:?}",
            summary.urls_rewritten
        );

        // Reload the relocated registry and confirm the url was rewritten.
        let new_reg_text = std::fs::read_to_string(dest.join("config.toml"))?;
        let new_reg: Registry = toml::from_str(&new_reg_text)?;
        let expected_new = dest.join("config-repos").join("personal");
        let entry = new_reg
            .configs
            .get("personal")
            .ok_or_else(|| anyhow::anyhow!("personal config missing after migrate"))?;
        assert_eq!(
            entry.url,
            expected_new.to_string_lossy(),
            "url should have been rewritten to the new repo path"
        );
        // store_dir cleared as before.
        assert_eq!(new_reg.settings.store_dir, None);
        assert_eq!(new_reg.settings.home_version, Some(2));

        // The repo itself landed at the new path.
        assert!(expected_new.join("file.txt").exists());

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn migrate_home_refuses_when_dest_repo_pre_exists() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let root = uniq_dir("mig-clobber");
        std::fs::create_dir_all(&root)?;
        let _ = build_xdg_layout(&root)?;
        let dest = root.join("dest");

        // Pre-create TWO dst paths that would be clobbered.
        std::fs::create_dir_all(dest.join("config-repos").join("personal"))?;
        std::fs::write(
            dest.join("config-repos").join("personal").join("stale.txt"),
            "stale",
        )?;
        std::fs::create_dir_all(dest.join("sources").join("foo"))?;
        std::fs::write(dest.join("sources").join("foo").join("stale.txt"), "stale")?;

        let err = run_migrate_home(Some("xdg"), &dest, false, false).unwrap_err();
        let msg = format!("{err:#}");
        eprintln!("TEST_A_REFUSAL_MSG:\n{msg}");
        assert!(
            msg.contains("already contains"),
            "expected clobber refusal, got: {msg}"
        );
        assert!(
            msg.contains("config-repos/personal"),
            "expected existing dst config-repos/personal listed in refusal, got: {msg}"
        );
        assert!(
            msg.contains("sources/foo"),
            "expected existing dst sources/foo listed in refusal, got: {msg}"
        );
        // The stale files must be untouched (refusal happens before any move).
        assert!(dest
            .join("config-repos")
            .join("personal")
            .join("stale.txt")
            .exists());
        assert!(dest.join("sources").join("foo").join("stale.txt").exists());

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn migrate_home_force_overrides_clobber_guard() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let root = uniq_dir("mig-force");
        std::fs::create_dir_all(&root)?;
        let _ = build_xdg_layout(&root)?;
        let dest = root.join("dest");

        // Pre-create a dst repo dir that would be clobbered.
        std::fs::create_dir_all(dest.join("config-repos").join("personal"))?;
        std::fs::write(
            dest.join("config-repos").join("personal").join("stale.txt"),
            "stale",
        )?;

        let summary = run_migrate_home(Some("xdg"), &dest, false, true)?;
        assert!(
            !summary.partial,
            "force should complete without partial failure"
        );
        assert!(summary.failed_at.is_none());
        // The moved repo content overwrites the stale file.
        assert!(dest
            .join("config-repos")
            .join("personal")
            .join("file.txt")
            .exists());

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn migrate_home_partial_failure_reports_partial_and_failed_at() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let root = uniq_dir("mig-partial");
        std::fs::create_dir_all(&root)?;
        let _ = build_xdg_layout(&root)?;
        let dest = root.join("dest");

        // Plant a regular FILE at dest/config-repos/personal where the src is
        // a DIRECTORY. --force bypasses the clobber guard; the pre-flight
        // passes (src exists, dst parent dest/config-repos/ is writable) but
        // the actual move of the config-repos/personal dir onto a file path
        // fails.
        std::fs::create_dir_all(dest.join("config-repos"))?;
        std::fs::write(dest.join("config-repos").join("personal"), "BLOCKER")?;

        let summary = run_migrate_home(Some("xdg"), &dest, false, true)?;
        eprintln!(
            "TEST_C_PARTIAL: partial={} failed_at={:?} moved_len={}",
            summary.partial,
            summary.failed_at,
            summary.moved.len()
        );
        assert!(
            summary.partial,
            "expected partial=true on mid-loop failure, got partial={}",
            summary.partial
        );
        assert!(
            summary
                .failed_at
                .as_deref()
                .is_some_and(|s| s.contains("personal")),
            "expected failed_at to contain 'personal', got {:?}",
            summary.failed_at
        );
        // Some entries before config-repos/personal should have moved
        // (registry, overrides, secrets come first in plan_moves ordering).
        assert!(
            !summary.moved.is_empty(),
            "expected at least one moved entry before the failure"
        );

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn home_version_defaults_to_absent() -> Result<()> {
        let toml_no_version = "[settings]\ndefault_context = \"personal\"\n";
        let reg: Registry = toml::from_str(toml_no_version)?;
        assert_eq!(reg.settings.home_version, None);
        // Round-trip preserves absence.
        let round = toml::to_string(&reg)?;
        let reg2: Registry = toml::from_str(&round)?;
        assert_eq!(reg2.settings.home_version, None);
        Ok(())
    }
    #[cfg(unix)]
    #[test]
    fn migrate_home_refuses_when_planned_dst_is_dangling_symlink() -> Result<()> {
        // FN-11 regression: a DANGLING symlink at a planned dst must trip the
        // clobber guard. `dst.exists()` follows the link and returns false,
        // so the old guard missed it and rename(2) silently replaced the link.
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let root = uniq_dir("mig-dangling");
        std::fs::create_dir_all(&root)?;
        let _ = build_xdg_layout(&root)?;
        let dest = root.join("dest");

        // Plant a dangling symlink where config-repos/personal is planned to
        // land.
        std::fs::create_dir_all(dest.join("config-repos"))?;
        let dangling = dest.join("config-repos").join("personal");
        std::os::unix::fs::symlink(root.join("nonexistent-target"), &dangling)?;
        // Precondition: the link is dangling (stat fails) but lstat sees it.
        assert!(!dangling.exists(), "test precondition: link must dangle");
        assert!(std::fs::symlink_metadata(&dangling).is_ok());

        let err = run_migrate_home(Some("xdg"), &dest, false, false).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("already contains"),
            "expected clobber refusal for the dangling symlink, got: {msg}"
        );
        assert!(
            msg.contains("config-repos/personal"),
            "expected the dangling dst listed in the refusal, got: {msg}"
        );
        // The dangling symlink must be untouched (refusal precedes any move).
        assert!(
            std::fs::symlink_metadata(&dangling)
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false),
            "dangling symlink must survive the refused migration"
        );

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }
}
