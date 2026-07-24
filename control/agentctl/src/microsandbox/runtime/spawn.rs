use anyhow::Result;
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
    let mut cmd = std::process::Command::new(&exe);
    cmd.args(args)
        .stdin(std::process::Stdio::null())
        .stdout(log_file.try_clone()?)
        .stderr(log_file);
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
        anyhow::bail!(
            "detached service '{}' exited immediately ({status}); see log: {}",
            name,
            log_path.display()
        );
    }
    Ok(child)
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
            "no logs found for '{}'; not started? run: workestrate {} up",
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

        let _ = std::fs::remove_dir_all(&home);
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
