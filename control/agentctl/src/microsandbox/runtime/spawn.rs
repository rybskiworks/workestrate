use anyhow::Result;
use std::path::Path;
use std::path::PathBuf;

/// Grace period after spawn during which an immediate child exit is treated
/// as a startup FAILURE (FS-8). ~500ms: long enough to catch an arg-parse /
/// config-load abort, short enough to not slow down `up` noticeably.
const SPAWN_GRACE: std::time::Duration = std::time::Duration::from_millis(500);

pub fn spawn_detached_service(name: &str, args: &[String]) -> Result<std::process::Child> {
    let exe = std::env::current_exe()?;
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| anyhow::anyhow!("HOME not set"))?;
    let log_dir = home.join(".microsandbox/sandboxes").join(name);
    std::fs::create_dir_all(&log_dir)?;
    let log_path = log_dir.join("workestrate.log");
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?;
    // Per-run delimiter (F4): the log is append-mode across runs AND binary
    // versions, so without a marker an operator can misattribute a stale
    // error from an older binary to the current run. One cheap write before
    // the child's stdout/stderr is redirected here. Failure is non-fatal —
    // log hygiene must never block a service start.
    {
        use std::io::Write;
        let _ = writeln!(
            &log_file,
            "===== workestrate {} spawn {} pid {} =====",
            env!("CARGO_PKG_VERSION"),
            super::time::current_rfc3339_utc(),
            std::process::id()
        );
    }
    let mut cmd = std::process::Command::new(&exe);
    cmd.args(args)
        .stdin(std::process::Stdio::null())
        .stdout(log_file.try_clone()?)
        .stderr(log_file);
    // Explicit cwd contract for the detached child (wrong-CWD `${CWD}` mount
    // fix): pin the ORIGINAL operator invocation cwd into the child's env so
    // `${CWD}` mount hosts resolve to the operator's directory even if the
    // parent's env was scrubbed. The child's own `ensure_invoke_cwd_env()`
    // keeps this inherited value (inherited-wins).
    if let Some(cwd) = crate::config::invoke_cwd() {
        cmd.env(crate::config::INVOKE_CWD_ENV, cwd);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let mut child = cmd.spawn()?;

    // FS-8 readiness check: previously the caller printed "started in
    // background (PID …)" with no verification, so a child that immediately
    // aborted (bad args, config error, missing secrets) still reported
    // success. Give the child a short grace window; if it has ALREADY exited,
    // surface the failure and point at the log file instead of returning a
    // healthy-looking Child.
    std::thread::sleep(SPAWN_GRACE);
    if let Some(status) = child.try_wait()? {
        let hint = extract_last_run_error(&log_path);
        match hint {
            Some(h) => anyhow::bail!(
                "detached service '{}' exited immediately ({status}); see log: {}; last error: {}",
                name,
                log_path.display(),
                h
            ),
            None => anyhow::bail!(
                "detached service '{}' exited immediately ({status}); see log: {}",
                name,
                log_path.display()
            ),
        }
    }
    Ok(child)
}

/// Best-effort extraction of the child's first error line for the FS-8
/// early-exit path. The append-mode log carries a per-run delimiter
/// (`===== workestrate <ver> spawn <ts> pid <pid> =====`); find the LAST
/// delimiter and scan lines after it for the first error/panic/usage line,
/// else the first non-empty line. Bounded: last 8 KiB, `<=32` lines,
/// `<=300` chars (truncated with a trailing `…`). `None` if the log is
/// missing/empty, has no delimiter, or no candidate line follows it.
fn extract_last_run_error(log_path: &Path) -> Option<String> {
    // bounded tail read: last 8 KiB
    const TAIL: u64 = 8 * 1024;
    let mut file = std::fs::File::open(log_path).ok()?;
    let len = std::fs::metadata(log_path).ok()?.len();
    if len == 0 {
        return None;
    }
    let start = len.saturating_sub(TAIL);
    use std::io::{Read, Seek, SeekFrom};
    if start > 0 {
        file.seek(SeekFrom::Start(start)).ok()?;
    }
    let mut buf = Vec::with_capacity(TAIL as usize);
    file.read_to_end(&mut buf).ok()?;
    let tail = String::from_utf8_lossy(&buf);

    // Split into lines; if we sliced mid-line (start>0), drop the first
    // partial line so we only consider whole lines.
    let mut lines: Vec<&str> = tail.lines().collect();
    if start > 0 && !lines.is_empty() {
        lines.remove(0);
    }

    // Find the LAST delimiter line (a line that starts with the marker).
    const MARK: &str = "===== workestrate ";
    let last_delim = lines.iter().rposition(|l| l.starts_with(MARK))?;
    let after: Vec<&str> = lines[last_delim + 1..]
        .iter()
        .rev()
        .take(32)
        .copied()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    // First candidate: a line containing error/panic/usage (case-insensitive).
    let candidate = after
        .iter()
        .find(|l| {
            let low = l.to_ascii_lowercase();
            low.contains("error") || low.contains("panic") || low.contains("usage")
        })
        .or_else(|| after.iter().find(|l| !l.is_empty()))?;
    let s = candidate.trim();
    if s.is_empty() {
        return None;
    }
    const MAX: usize = 300;
    if s.chars().count() <= MAX {
        Some(s.to_string())
    } else {
        // truncate at a char boundary <= MAX chars, append ellipsis
        let mut end = 0usize;
        for (i, (bidx, _)) in s.char_indices().enumerate() {
            if i == MAX {
                end = bidx;
                break;
            }
        }
        Some(format!("{}…", &s[..end]))
    }
}

/// Tail the detached service's log file.
pub async fn logs(name: &str) -> Result<()> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| anyhow::anyhow!("HOME not set"))?;
    let path = home
        .join(".microsandbox/sandboxes")
        .join(name)
        .join("workestrate.log");

    if !path.exists() {
        anyhow::bail!(
            "no logs found for '{}'; not started? run: `workestrate workload up {}`",
            name,
            name
        );
    }

    let status = std::process::Command::new("tail")
        .args(["-n", "200", "-f", &path.to_string_lossy()])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("tail exited with status: {}", status))
    }
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

    // ---- FS-8: spawn_detached_service surfaces immediate child failure ----

    /// FS-8: a child that exits within the grace window must surface an Err
    /// pointing at the log file, NOT a healthy-looking Child (which the
    /// caller would report as "started in background (PID …)").
    ///
    /// `--definitely-not-a-real-flag` makes the re-exec'd workestrate binary
    /// abort inside clap arg parsing (exit code 2), deterministically within
    /// the grace period. This test mutates HOME (log-dir root) and therefore
    /// holds ENV_TEST_LOCK like every other env-mutating test.
    #[test]
    fn spawn_detached_service_fails_when_child_exits_immediately() -> anyhow::Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(
            crate::config::test_support::HOME_ENV_KEYS,
        );

        let home = crate::config::test_support::uniq_dir("fs8-spawn-home");
        std::fs::create_dir_all(&home)?;
        std::env::set_var("HOME", &home);

        let args = vec!["--definitely-not-a-real-flag".to_string()];
        let result = spawn_detached_service("fs8-immediate-fail", &args);

        assert!(
            result.is_err(),
            "an immediately-exiting child must surface Err, got Ok"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("exited immediately"),
            "error should name the immediate-exit failure mode: {err}"
        );
        assert!(
            err.contains("workestrate.log"),
            "error should point at the log file: {err}"
        );
        assert!(
            err.contains("unexpected argument")
                || err.contains("Usage:")
                || err.contains("Unrecognized option"),
            "the early-exit message should surface the child's clap error via the last-error hint: {err}"
        );
        // The log file captured the child's clap usage error.
        let log = std::fs::read_to_string(
            home.join(".microsandbox/sandboxes")
                .join("fs8-immediate-fail")
                .join("workestrate.log"),
        )?;
        assert!(
            !log.is_empty(),
            "the child's stderr must land in the log file"
        );
        // F4: a per-run delimiter precedes the child's output so stale
        // entries from older runs/binaries are distinguishable.
        assert!(
            log.contains("===== workestrate "),
            "log must open with the per-run delimiter: {log}"
        );
        assert!(
            log.contains(" pid "),
            "delimiter carries version, timestamp and pid: {log}"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn extract_last_run_error_finds_first_error_after_last_delimiter() -> anyhow::Result<()> {
        let dir = crate::config::test_support::uniq_dir("extract-first-error");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("workestrate.log");
        std::fs::write(
            &path,
            "===== workestrate 0.0.0 spawn 2026-01-01T00:00:00Z pid 1 =====\nerror: stale run failure\n===== workestrate 0.0.0 spawn 2026-01-01T00:00:00Z pid 2 =====\nerror: current run failure\nUsage: workestrate ...\n",
        )?;
        assert_eq!(
            extract_last_run_error(&path),
            Some("error: current run failure".to_string())
        );
        std::fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn extract_last_run_error_falls_back_to_first_nonempty_when_no_error_marker(
    ) -> anyhow::Result<()> {
        let dir = crate::config::test_support::uniq_dir("extract-fallback");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("workestrate.log");
        std::fs::write(
            &path,
            "===== workestrate 0.0.0 spawn 2026-01-01T00:00:00Z pid 1 =====\nbooting workestrate\nstarted ok\n",
        )?;
        assert_eq!(
            extract_last_run_error(&path),
            Some("booting workestrate".to_string())
        );
        std::fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn extract_last_run_error_none_for_missing_empty_delimiter_only() -> anyhow::Result<()> {
        let dir = crate::config::test_support::uniq_dir("extract-none");
        std::fs::create_dir_all(&dir)?;
        let missing = dir.join("missing.log");
        assert_eq!(extract_last_run_error(&missing), None);
        let empty = dir.join("empty.log");
        std::fs::write(&empty, "")?;
        assert_eq!(extract_last_run_error(&empty), None);
        let delimiter = dir.join("delimiter.log");
        std::fs::write(
            &delimiter,
            "===== workestrate 0.0.0 spawn 2026-01-01T00:00:00Z pid 1 =====\n",
        )?;
        assert_eq!(extract_last_run_error(&delimiter), None);
        std::fs::remove_dir_all(&dir)?;
        Ok(())
    }

    #[test]
    fn extract_last_run_error_is_bounded_and_truncates() -> anyhow::Result<()> {
        let dir = crate::config::test_support::uniq_dir("extract-bounded");
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("long.log");
        let log = format!(
            "===== workestrate 0.0.0 spawn 2026-01-01T00:00:00Z pid 1 =====\nerror: {}\n",
            "x".repeat(1000)
        );
        std::fs::write(&path, log)?;
        let result = extract_last_run_error(&path).expect("long error line should be extracted");
        assert!(result.chars().count() <= 301);
        assert!(result.ends_with('…'));

        let many = dir.join("many.log");
        let mut log =
            String::from("===== workestrate 0.0.0 spawn 2026-01-01T00:00:00Z pid 1 =====\n");
        for _ in 0..999 {
            log.push_str("x\n");
        }
        std::fs::write(&many, log)?;
        assert!(extract_last_run_error(&many).is_some());
        std::fs::remove_dir_all(&dir)?;
        Ok(())
    }

    /// FS-8 companion: even on the immediate-failure path the log file must
    /// exist, so the pointer in the failure message is valid. (The
    /// still-running success path is inherently timing-dependent under the
    /// 500ms grace and is covered end-to-end by the KVM-gated
    /// `lifecycle_detached` integration test.)
    #[test]
    fn spawn_detached_service_log_file_is_created() -> anyhow::Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(
            crate::config::test_support::HOME_ENV_KEYS,
        );

        let home = crate::config::test_support::uniq_dir("fs8-log-home");
        std::fs::create_dir_all(&home)?;
        std::env::set_var("HOME", &home);

        // Even on the failure path, the log file must exist (the failure
        // message points at it).
        let args = vec!["--definitely-not-a-real-flag".to_string()];
        let _ = spawn_detached_service("fs8-log-check", &args);
        assert!(
            home.join(".microsandbox/sandboxes")
                .join("fs8-log-check")
                .join("workestrate.log")
                .exists(),
            "log file must exist so the failure message's pointer is valid"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }
}
