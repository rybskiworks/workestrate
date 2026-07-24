//! Git subprocess helpers.
//!
//! Thin wrappers around `git` CLI invocations used by config-repo management
//! (`workestrate config add/update/new/remove`, `check`). Each helper runs a
//! single `git` command and maps a non-zero exit status to an `anyhow` error.

use anyhow::Result;

pub(crate) fn git_clone(url: &str, dest: &std::path::Path, branch: Option<&str>) -> Result<()> {
    let mut cmd = std::process::Command::new("git");
    cmd.args(["clone", "--depth", "1"]);
    if let Some(branch) = branch {
        cmd.args(["--branch", branch]);
    }
    let status = cmd.arg(url).arg(dest).status()?;
    if !status.success() {
        anyhow::bail!("git clone failed for {}", url);
    }
    Ok(())
}

/// Initialize a new git repo at `dir` (no commit, mirrors `cargo new`).
/// Used by `workestrate config new` to make the scaffold immediately
/// committable. Returns a distinct error kind when the `git` binary is
/// absent so the caller can warn-and-continue rather than fail the whole
/// scaffold (the files are already written and valid).
pub(crate) fn git_init(dir: &std::path::Path) -> Result<()> {
    let status = std::process::Command::new("git")
        .arg("init")
        .arg(dir)
        .status()
        .map_err(|e| anyhow::anyhow!("git binary not found: {}", e))?;
    if !status.success() {
        anyhow::bail!("git init failed in '{}'", dir.display());
    }
    Ok(())
}

pub(crate) fn git_rev_parse(repo: &std::path::Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("git rev-parse failed for {}", repo.display());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub(crate) fn git_is_dirty(repo: &std::path::Path) -> Result<bool> {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["diff", "--quiet", "HEAD"])
        .status()?;
    Ok(!status.success())
}

pub(crate) fn git_pull(repo: &std::path::Path, branch: &str) -> Result<()> {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["pull", "origin", branch])
        .status()?;
    if !status.success() {
        anyhow::bail!("git pull failed for {}", repo.display());
    }
    Ok(())
}

pub(crate) fn git_checkout_dot(repo: &std::path::Path) -> Result<()> {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["checkout", "."])
        .status()?;
    if !status.success() {
        anyhow::bail!("git checkout failed for {}", repo.display());
    }
    Ok(())
}

pub(crate) fn short_rev(rev: &str) -> String {
    rev.chars().take(7).collect()
}

/// On-disk status of one registered config repo. Plain-data result of
/// [`collect_repo_statuses`]; each caller renders it (doctor → JSON, check →
/// text). `rev` is the registry-recorded revision (or "unknown"); `short` is
/// its 7-char prefix; `exists` is whether the clone dir is present; `dirty`
/// is the git working-tree dirty flag (`true` when missing or undeterminable,
/// matching both callers' prior fail-closed default).
#[derive(Debug, Clone)]
pub(crate) struct RepoStatus {
    pub name: String,
    pub rev: String,
    pub short: String,
    pub exists: bool,
    pub dirty: bool,
}

/// Collect the on-disk status of every registered config repo. Shared by
/// `workestrate doctor` and `workestrate check`; behavior is identical to the
/// two former inline copies (same dir resolution, rev fallback, dirty probe).
pub(crate) fn collect_repo_statuses(registry: &crate::config::Registry) -> Vec<RepoStatus> {
    registry
        .configs
        .iter()
        .map(|(name, entry)| {
            let dest = crate::config::config_repo_dir(name);
            let rev = entry.rev.as_deref().unwrap_or("unknown").to_string();
            let short = short_rev(&rev);
            let exists = dest.exists();
            let dirty = git_is_dirty(&dest).unwrap_or(true);
            RepoStatus {
                name: name.clone(),
                rev,
                short,
                exists,
                dirty,
            }
        })
        .collect()
}
