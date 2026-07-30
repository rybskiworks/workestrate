//! Git subprocess helpers.
//!
//! Thin wrappers around `git` CLI invocations used by config-repo management
//! (`workestrate config add/update/new/remove`, `check`). Each helper runs a
//! single `git` command and maps a non-zero exit status to an `anyhow` error.

use anyhow::Result;

pub fn git_clone(url: &str, dest: &std::path::Path, branch: Option<&str>) -> Result<()> {
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

/// Full (non-shallow) clone of `url` into `dest` on the default branch.
/// Used by `home init --from` (ADR 0025): reproducing a home needs the full
/// history so a recorded registry `rev` can be checked out (a `--depth 1`
/// clone only carries the branch tip).
pub fn git_clone_full(url: &str, dest: &std::path::Path) -> Result<()> {
    let status = std::process::Command::new("git")
        .args(["clone"])
        .arg(url)
        .arg(dest)
        .status()?;
    if !status.success() {
        anyhow::bail!("git clone failed for {}", url);
    }
    Ok(())
}

/// Check out `rev` in `repo` (detached HEAD). Used by `home init --from` to
/// pin a reproduced config repo to the source home's recorded revision.
pub fn git_checkout_rev(repo: &std::path::Path, rev: &str) -> Result<()> {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["checkout", rev])
        .status()?;
    if !status.success() {
        anyhow::bail!("git checkout {} failed for {}", rev, repo.display());
    }
    Ok(())
}

/// Read a remote's URL (`git remote get-url <name>`). Returns `Ok(None)`
/// when the remote does not exist (git exits 2); other failures are errors.
pub fn git_remote_get_url(repo: &std::path::Path, name: &str) -> Result<Option<String>> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["remote", "get-url", name])
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    Ok(Some(
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
    ))
}

/// Add or repoint a remote: `git remote add <name> <url>` when the remote is
/// absent, else `git remote set-url <name> <url>`.
pub fn git_remote_add_or_set_url(repo: &std::path::Path, name: &str, url: &str) -> Result<()> {
    let verb = if git_remote_get_url(repo, name)?.is_some() {
        "set-url"
    } else {
        "add"
    };
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["remote", verb, name, url])
        .status()?;
    if !status.success() {
        anyhow::bail!("git remote {} failed for {}", verb, repo.display());
    }
    Ok(())
}

/// Initialize a new git repo at `dir` (no commit, mirrors `cargo new`).
/// Used by `workestrate config new` to make the scaffold immediately
/// committable. Returns a distinct error kind when the `git` binary is
/// absent so the caller can warn-and-continue rather than fail the whole
/// scaffold (the files are already written and valid).
pub fn git_init(dir: &std::path::Path) -> Result<()> {
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

pub fn git_rev_parse(repo: &std::path::Path) -> Result<String> {
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

pub fn git_is_dirty(repo: &std::path::Path) -> Result<bool> {
    // `git status --porcelain` reports tracked modifications AND untracked
    // files; `git diff --quiet HEAD` misses untracked files entirely (a repo
    // whose only change is a new, never-added file would read as clean).
    // Empty output = clean; any output = dirty.
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["status", "--porcelain"])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("git status failed for {}", repo.display());
    }
    Ok(!output.stdout.is_empty())
}

pub fn git_pull(repo: &std::path::Path, branch: &str) -> Result<()> {
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

pub fn git_checkout_dot(repo: &std::path::Path) -> Result<()> {
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

pub fn short_rev(rev: &str) -> String {
    rev.chars().take(7).collect()
}

/// On-disk status of one registered config repo. Plain-data result of
/// [`collect_repo_statuses`]; each caller renders it (doctor → JSON, check →
/// text). `rev` is the registry-recorded revision (or "unknown"); `short` is
/// its 7-char prefix; `exists` is whether the clone dir is present; `dirty`
/// is the git working-tree dirty flag (`true` when missing or undeterminable,
/// matching both callers' prior fail-closed default).
#[derive(Debug, Clone)]
pub struct RepoStatus {
    pub name: String,
    pub rev: String,
    pub short: String,
    pub exists: bool,
    pub dirty: bool,
}

/// Collect the on-disk status of every registered config repo. Shared by
/// `workestrate doctor` and `workestrate check`; behavior is identical to the
/// two former inline copies (same dir resolution, rev fallback, dirty probe).
pub fn collect_repo_statuses(registry: &crate::config::Registry) -> Vec<RepoStatus> {
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
    use super::*;
    use crate::config::test_support::uniq_dir;

    /// Init a git repo in `dir` with one committed file, using local
    /// (repo-scoped) identity so the test is independent of global git config.
    fn init_repo_with_commit(dir: &std::path::Path) {
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(args)
                .status()
                .expect("git must be runnable");
            assert!(status.success(), "git {:?} failed", args);
        };
        std::fs::create_dir_all(dir).expect("create repo dir");
        run(&["init", "--quiet"]);
        run(&["config", "user.email", "wp6@test.invalid"]);
        run(&["config", "user.name", "wp6-test"]);
        std::fs::write(dir.join("tracked.txt"), "committed").expect("write tracked file");
        run(&["add", "tracked.txt"]);
        run(&["commit", "--quiet", "-m", "init"]);
    }

    #[test]
    fn git_is_dirty_reports_untracked_only_repo_as_dirty() {
        // FN-1 regression: a repo whose ONLY change is an untracked file must
        // read as dirty (the old `git diff --quiet HEAD` probe missed it).
        let dir = uniq_dir("git-dirty-untracked");
        init_repo_with_commit(&dir);
        assert!(
            !git_is_dirty(&dir).expect("dirty check on clean repo"),
            "fresh commit with no changes must be clean"
        );
        std::fs::write(dir.join("new-untracked.txt"), "never added").expect("write untracked file");
        assert!(
            git_is_dirty(&dir).expect("dirty check with untracked file"),
            "repo with only an untracked file must be dirty"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn git_is_dirty_reports_tracked_modification_as_dirty() {
        let dir = uniq_dir("git-dirty-modified");
        init_repo_with_commit(&dir);
        std::fs::write(dir.join("tracked.txt"), "modified").expect("modify tracked file");
        assert!(
            git_is_dirty(&dir).expect("dirty check with modified file"),
            "repo with a tracked modification must be dirty"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
