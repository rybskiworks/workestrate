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
/// Used by `home clone` (ADR 0025): reproducing a home needs the full
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

/// Check out `rev` in `repo` (detached HEAD). Used by `home clone` to
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

/// True when `repo` has a resolvable HEAD (at least one commit). A
/// git-init'd repo with no commits is still a repo on disk, but cloning it
/// yields an empty tree — callers use this to gate git-clone paths.
pub fn git_has_head(repo: &std::path::Path) -> bool {
    git_rev_parse(repo).is_ok()
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

/// Resolve `refname` in `repo` to a commit sha (`git rev-parse
/// <refname>^{commit}` — the `^{commit}` peel forces a commit answer for
/// tags and other ref-ish input). Errors name BOTH the repo and the ref.
/// (A5/ADR 0032 addendum: default-ref resolution and archive pinning.)
pub fn git_rev_parse_ref(repo: &std::path::Path, refname: &str) -> Result<String> {
    let spec = format!("{}^{{commit}}", refname);
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", &spec])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("git rev-parse '{}' failed for {}", spec, repo.display());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// The default branch of the `origin` remote (`git symbolic-ref
/// refs/remotes/origin/HEAD`), stripped of the `refs/remotes/origin/`
/// prefix. Returns `Ok(None)` when the symref is absent (a repo without an
/// `origin/HEAD` — e.g. a fresh local init, or a clone whose remote never
/// advertised a HEAD); other command failures also read as `Ok(None)` (the
/// caller's fallback chain decides whether that is fatal).
pub fn git_remote_default_branch(repo: &std::path::Path) -> Result<Option<String>> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["symbolic-ref", "refs/remotes/origin/HEAD"])
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let symref = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match symref.strip_prefix("refs/remotes/origin/") {
        Some(branch) => Ok(Some(branch.to_string())),
        None => anyhow::bail!(
            "git symbolic-ref refs/remotes/origin/HEAD for {} returned unexpected target '{}'",
            repo.display(),
            symref
        ),
    }
}

/// The branch `checkout` currently has checked out (`git symbolic-ref
/// HEAD`), stripped of the `refs/heads/` prefix. Returns `Ok(None)` on a
/// DETACHED HEAD (symbolic-ref exits non-zero there) and on a non-repo.
pub fn git_checkout_branch(repo: &std::path::Path) -> Result<Option<String>> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["symbolic-ref", "HEAD"])
        .output()?;
    if !output.status.success() {
        return Ok(None);
    }
    let symref = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match symref.strip_prefix("refs/heads/") {
        Some(branch) => Ok(Some(branch.to_string())),
        None => anyhow::bail!(
            "git symbolic-ref HEAD for {} returned unexpected target '{}'",
            repo.display(),
            symref
        ),
    }
}

/// True when `branch` names an existing BRANCH ref in `repo` —
/// `refs/heads/<branch>` or `refs/remotes/origin/<branch>` (a thin
/// `git show-ref --verify --quiet` probe per form: read-only, no fetch, no
/// output). A purely-sha input resolves as a commit but is NOT a branch, so
/// this returns false for it — that distinction is exactly what the A5
/// context-derivation ladder (ADR 0032 addendum §Selection ladder) keys on
/// when it asks whether `--config-ref <ref>` names a branch.
pub fn git_branch_ref_exists(repo: &std::path::Path, branch: &str) -> Result<bool> {
    for full_ref in [
        format!("refs/heads/{branch}"),
        format!("refs/remotes/origin/{branch}"),
    ] {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["show-ref", "--verify", "--quiet", &full_ref])
            .status()?;
        if status.success() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Produce a plain-directory export of `rev` from `repo` into `dest`:
/// `git archive --format=tar` to a sibling `<dest>.tmp.tar`, then `tar -xf`
/// into `dest` (created first). The tmp tar is removed on BOTH the success
/// and the failure path. Errors name the repo and the rev. (A5/ADR 0032
/// addendum: the content-addressed archive store is built from these;
/// `tar` on unix is the pinned mechanism, a zip fallback is the noted
/// Windows path.)
pub fn git_archive(repo: &std::path::Path, rev: &str, dest: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(dest)?;
    let tmp = std::path::PathBuf::from(format!("{}.tmp.tar", dest.display()));
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["archive", "--format=tar"])
        .arg(format!("--output={}", tmp.display()))
        .arg(rev)
        .status()?;
    if !status.success() {
        let _ = std::fs::remove_file(&tmp);
        anyhow::bail!("git archive {} failed for {}", rev, repo.display());
    }
    let status = std::process::Command::new("tar")
        .arg("-xf")
        .arg(&tmp)
        .arg("-C")
        .arg(dest)
        .status();
    match status {
        Ok(s) if s.success() => {
            let _ = std::fs::remove_file(&tmp);
            Ok(())
        }
        Ok(_) => {
            let _ = std::fs::remove_file(&tmp);
            anyhow::bail!(
                "tar extraction of archive {} failed for {}",
                rev,
                repo.display()
            );
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            Err(anyhow::anyhow!(
                "failed to run tar for archive {} of {}: {}",
                rev,
                repo.display(),
                e
            ))
        }
    }
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

    // ---- A5: rev-parse-ref / default-branch / checkout-branch / archive ----
    // Git availability: like the dirty-probe tests above and the lockfile
    // tests, these expect `git` (and `tar`, for git_archive) on PATH — the
    // pinned toolchain guarantees both; there is no host-gating precedent in
    // this file.

    #[test]
    fn git_rev_parse_ref_resolves_branch_and_sha_to_commit() {
        let dir = uniq_dir("git-rev-parse-ref");
        init_repo_with_commit(&dir);
        let head = git_rev_parse(&dir).expect("HEAD sha");
        let branch = git_checkout_branch(&dir)
            .expect("symbolic-ref HEAD")
            .expect("fresh commit repo is on a branch");
        assert_eq!(
            git_rev_parse_ref(&dir, &branch).expect("rev-parse branch"),
            head,
            "branch name must resolve to the same commit"
        );
        assert_eq!(
            git_rev_parse_ref(&dir, &head).expect("rev-parse sha"),
            head,
            "a sha resolves to itself"
        );
        let err = git_rev_parse_ref(&dir, "no-such-ref").expect_err("bogus ref must fail");
        let msg = err.to_string();
        assert!(
            msg.contains("no-such-ref") && msg.contains(&dir.display().to_string()),
            "error must name the ref AND the repo: {msg}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn git_remote_default_branch_reads_origin_head() {
        // Holds ENV_TEST_LOCK: the clone probes HOME for git config, and
        // parallel env-mutating tests can point HOME at a removed dir.
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let src = uniq_dir("git-remote-head-src");
        init_repo_with_commit(&src);
        // A clone carries refs/remotes/origin/HEAD; the source repo does not.
        let clone = uniq_dir("git-remote-head-clone");
        let out = std::process::Command::new("git")
            .args(["clone", "--quiet"])
            .arg(&src)
            .arg(&clone)
            .output()
            .expect("git must be runnable");
        assert!(
            out.status.success(),
            "git clone failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let src_branch = git_checkout_branch(&src)
            .expect("symbolic-ref HEAD")
            .expect("src is on a branch");
        assert_eq!(
            git_remote_default_branch(&clone).expect("origin/HEAD probe"),
            Some(src_branch),
            "origin/HEAD must resolve to the source's default branch"
        );
        assert_eq!(
            git_remote_default_branch(&src).expect("absent origin/HEAD probe"),
            None,
            "a repo without refs/remotes/origin/HEAD yields None"
        );
        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&clone);
    }

    #[test]
    fn git_checkout_branch_reports_branch_and_none_when_detached() {
        let dir = uniq_dir("git-checkout-branch");
        init_repo_with_commit(&dir);
        let branch = git_checkout_branch(&dir).expect("symbolic-ref HEAD");
        assert!(
            branch.as_deref().is_some_and(|b| !b.is_empty()),
            "a fresh commit repo must report its branch"
        );
        let head = git_rev_parse(&dir).expect("HEAD sha");
        git_checkout_rev(&dir, &head).expect("detach HEAD");
        assert_eq!(
            git_checkout_branch(&dir).expect("symbolic-ref HEAD detached"),
            None,
            "detached HEAD must yield None"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn git_branch_ref_exists_distinguishes_branch_from_sha_and_bogus() {
        // Holds ENV_TEST_LOCK: the clone probes HOME for git config, and
        // parallel env-mutating tests can point HOME at a removed dir.
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let dir = uniq_dir("git-branch-ref");
        init_repo_with_commit(&dir);
        let head = git_rev_parse(&dir).expect("HEAD sha");
        let branch = git_checkout_branch(&dir)
            .expect("symbolic-ref HEAD")
            .expect("fresh commit repo is on a branch");

        assert!(
            git_branch_ref_exists(&dir, &branch).expect("local branch probe"),
            "refs/heads/<branch> must read as an existing branch"
        );
        assert!(
            !git_branch_ref_exists(&dir, &head).expect("sha probe"),
            "a purely-sha ref resolves as a commit but is NOT a branch"
        );
        assert!(
            !git_branch_ref_exists(&dir, "no-such-branch").expect("bogus probe"),
            "an absent ref is not a branch"
        );

        // A clone carries the source's branch under refs/remotes/origin/<b>
        // (and its own local refs/heads/<b>); both forms count.
        let clone = uniq_dir("git-branch-ref-clone");
        let out = std::process::Command::new("git")
            .args(["clone", "--quiet"])
            .arg(&dir)
            .arg(&clone)
            .output()
            .expect("git must be runnable");
        assert!(
            out.status.success(),
            "git clone failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            git_branch_ref_exists(&clone, &branch).expect("clone branch probe"),
            "refs/remotes/origin/<branch> in a clone must read as an existing branch"
        );

        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&clone);
    }

    #[test]
    fn git_archive_exports_rev_content_and_cleans_tmp() {
        let repo = uniq_dir("git-archive-repo");
        init_repo_with_commit(&repo);
        let head = git_rev_parse(&repo).expect("HEAD sha");
        let dest = uniq_dir("git-archive-dest");

        git_archive(&repo, &head, &dest).expect("archive HEAD");

        assert_eq!(
            std::fs::read_to_string(dest.join("tracked.txt")).expect("archived file"),
            "committed",
            "the archive must contain the committed tree"
        );
        assert!(
            !std::path::PathBuf::from(format!("{}.tmp.tar", dest.display())).exists(),
            "the tmp tar must be removed on success"
        );
        assert!(
            !dest.join(".git").exists(),
            "an archive is a plain directory, not a repo"
        );

        let bogus_dest = uniq_dir("git-archive-bogus");
        let err = git_archive(&repo, "0".repeat(40).as_str(), &bogus_dest)
            .expect_err("a nonexistent rev must fail");
        let msg = err.to_string();
        assert!(
            msg.contains(&repo.display().to_string()) && msg.contains(&"0".repeat(40)),
            "error must name repo AND rev: {msg}"
        );
        assert!(
            !std::path::PathBuf::from(format!("{}.tmp.tar", bogus_dest.display())).exists(),
            "the tmp tar must be removed on failure"
        );
        let _ = std::fs::remove_dir_all(&repo);
        let _ = std::fs::remove_dir_all(&dest);
        let _ = std::fs::remove_dir_all(&bogus_dest);
    }
}
