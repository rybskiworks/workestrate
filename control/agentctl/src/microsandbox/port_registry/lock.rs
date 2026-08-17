use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

// ---------------------------------------------------------------------------
// Registry lock (WP10/A17: check-then-register TOCTOU mitigation)
// ---------------------------------------------------------------------------

/// Lock-file name inside `${state_dir}/var/run/`.
pub(crate) const PORT_REGISTRY_LOCK_NAME: &str = ".port-registry.lock";

/// How long to retry lock acquisition before giving up (~2s), so normal
/// contention between two near-simultaneous `up` invocations does not produce
/// a spurious failure.
const LOCK_ACQUIRE_TIMEOUT: Duration = Duration::from_millis(2000);

/// Backoff between lock-acquisition attempts.
const LOCK_ACQUIRE_BACKOFF: Duration = Duration::from_millis(25);

/// RAII guard for exclusive access to the port registry.
///
/// Creation acquires the lock by creating
/// `${state_dir}/var/run/.port-registry.lock` with
/// `OpenOptions::create_new(true)` (O_EXCL semantics — the create fails while
/// another holder's file exists); [`Drop`] removes the file. While held, the
/// holder has exclusive rights to check + register ports.
///
/// **Staleness rule (Linux-only):** the file body records the creating PID
/// and an epoch-second timestamp. If acquisition fails because the file
/// already exists AND the recorded PID is no longer alive (no `/proc/<pid>` —
/// e.g. the holder crashed), the file is treated as stale: it is removed and
/// acquisition is retried. A live PID (or an unreadable/foreign-format lock
/// file) is never removed — it just keeps retrying until the timeout.
///
/// Note this is a cross-process advisory lock on the registry DIRECTORY, not
/// an `flock`: it needs no extra deps and the registry is only mutated by
/// workestrate itself.
pub struct PortRegistryLock {
    path: PathBuf,
}

impl PortRegistryLock {
    /// Acquire the registry lock, retrying briefly and recovering from a
    /// stale (dead-PID) lock file.
    pub fn acquire(state_dir: &Path) -> Result<Self> {
        let run_dir = state_dir.join("var").join("run");
        std::fs::create_dir_all(&run_dir)
            .with_context(|| format!("failed to create {}", run_dir.display()))?;
        let path = run_dir.join(PORT_REGISTRY_LOCK_NAME);
        let deadline = Instant::now() + LOCK_ACQUIRE_TIMEOUT;
        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(mut f) => {
                    use std::io::Write;
                    let body = format!(
                        "pid={}\ncreated_at_epoch={}\n",
                        std::process::id(),
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0)
                    );
                    // Best-effort: the file's EXISTENCE is the lock; the body
                    // only feeds the staleness check.
                    let _ = f.write_all(body.as_bytes());
                    return Ok(Self { path });
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if lock_file_is_stale(&path) {
                        // Stale lock: the creating process is dead. Remove and
                        // immediately retry acquisition.
                        let _ = std::fs::remove_file(&path);
                        continue;
                    }
                    if Instant::now() >= deadline {
                        anyhow::bail!(
                            "timed out acquiring port-registry lock {} after {:?}; \
                             another workestrate process is registering ports. \
                             Retry, or remove the file if no workestrate process is running.",
                            path.display(),
                            LOCK_ACQUIRE_TIMEOUT
                        );
                    }
                    std::thread::sleep(LOCK_ACQUIRE_BACKOFF);
                }
                Err(e) => {
                    return Err(e).with_context(|| {
                        format!("failed to acquire port-registry lock {}", path.display())
                    });
                }
            }
        }
    }
}

impl Drop for PortRegistryLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Whether the lock file at `path` is stale: it exists, records a `pid=N`
/// line, and that PID is no longer alive (Linux liveness probe).
///
/// Conservative: any parse failure, missing PID, or live PID counts as NOT
/// stale (never remove a lock we cannot prove is dead).
///
/// `pub(crate)` so `images::lock` (spec 21 §3.3) shares the ONE stale-lock
/// recovery probe — the pid+timestamp body format is identical.
pub(crate) fn lock_file_is_stale(path: &Path) -> bool {
    lock_file_is_stale_with(path, pid_is_alive)
}

/// Testable core of [`lock_file_is_stale`] with the liveness probe injected.
fn lock_file_is_stale_with(path: &Path, is_alive: fn(u32) -> bool) -> bool {
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    let pid = content
        .lines()
        .find_map(|line| line.strip_prefix("pid="))
        .and_then(|s| s.trim().parse::<u32>().ok());
    match pid {
        Some(pid) => !is_alive(pid),
        None => false,
    }
}

/// Linux liveness probe (pure std, no extra deps): a PID counts as ALIVE
/// when `/proc/<pid>` exists AND its `/proc/<pid>/stat` `state` field is not
/// `X` (dead) or `Z` (zombie). Reading `stat` filters out the defunct-PID
/// case that plain directory-existence misses when PID recycling is slow.
fn pid_is_alive(pid: u32) -> bool {
    let stat = match std::fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(s) => s,
        Err(_) => return false, // no /proc entry at all → dead
    };
    // Field 3 (after the comm, which is parenthesized and may contain
    // spaces) is the single-char state. Parse robustly: state is the first
    // token after the final ')'.
    let Some((_, after_comm)) = stat.rsplit_once(") ") else {
        return false;
    };
    let state = after_comm.chars().next().unwrap_or('?');
    !matches!(state, 'X' | 'Z')
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::super::store::{check_and_register_sandbox_lifecycle, list_records};
    use super::*;
    use crate::config::test_support::unique_state_dir;
    use std::net::IpAddr;

    /// Small helper: a lifecycle registration for `instance` on `port` bound
    /// to the shared singleton 127.0.0.1 (ADR 0026: same-(ip, port) race).
    fn combined_register(
        state_dir: &Path,
        instance: &str,
        workload: &str,
        port: u16,
    ) -> Result<()> {
        check_and_register_sandbox_lifecycle(
            state_dir,
            instance,
            None,
            workload,
            IpAddr::V4(std::net::Ipv4Addr::LOCALHOST),
            &[port],
            &[crate::microsandbox::plan::PortMapping::new(port, port)],
            "2026-07-23T00:00:00Z",
            "default",
        )
    }

    #[test]
    fn lock_is_released_after_guard_drop() -> Result<()> {
        let state_dir = unique_state_dir("lock-release");
        {
            let _guard = PortRegistryLock::acquire(&state_dir)?;
            assert!(
                state_dir
                    .join("var")
                    .join("run")
                    .join(PORT_REGISTRY_LOCK_NAME)
                    .exists(),
                "lock file exists while held"
            );
        }
        // A second acquisition (via the combined function) must succeed.
        combined_register(&state_dir, "personal-pi", "pi", 3000)?;
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn concurrent_combined_register_exactly_one_wins_per_port() -> Result<()> {
        let state_dir = unique_state_dir("combined-race");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let mut handles = Vec::new();
        for i in 0..4u32 {
            let dir = state_dir.clone();
            let b = std::sync::Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                b.wait(); // release all threads at once
                combined_register(&dir, &format!("inst-{i}"), "litellm", 4000)
            }));
        }
        let results: Vec<Result<()>> = handles
            .into_iter()
            .map(|h| h.join().expect("worker thread panicked"))
            .collect();
        let wins = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(
            wins, 1,
            "exactly one concurrent registration should win the port; results: {results:?}"
        );
        for r in &results {
            if let Err(e) = r {
                let msg = e.to_string();
                assert!(
                    msg.contains("port collision") || msg.contains("timed out acquiring"),
                    "losers must fail with a collision or lock-timeout, got: {msg}"
                );
            }
        }
        // Only one record may hold port 4000.
        let records = list_records(&state_dir)?;
        let holders: Vec<_> = records.iter().filter(|r| r.ports.contains(&4000)).collect();
        assert_eq!(holders.len(), 1, "exactly one record may hold port 4000");
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn stale_lock_from_dead_pid_is_recovered() -> Result<()> {
        let state_dir = unique_state_dir("stale-lock");
        let run_dir = state_dir.join("var").join("run");
        std::fs::create_dir_all(&run_dir)?;
        // Derive a PROVABLY dead PID: spawn a short-lived child, reap it, then
        // use its (now-defunct) PID. A hardcoded large PID is unreliable —
        // containers can set pid_max high enough for it to be a live PID.
        let dead_pid = {
            let mut child = std::process::Command::new("true")
                .spawn()
                .expect("spawn 'true'");
            let pid = child.id();
            child.wait().expect("wait for 'true'");
            pid
        };
        assert!(
            !pid_is_alive(dead_pid),
            "test precondition: pid {dead_pid} must be dead"
        );
        // Write a lock file whose recorded holder is the dead PID.
        std::fs::write(
            run_dir.join(PORT_REGISTRY_LOCK_NAME),
            format!("pid={dead_pid}\ncreated_at_epoch=1\n"),
        )?;
        // Acquisition must detect the dead PID, remove the stale file, and
        // proceed (not time out).
        let guard = PortRegistryLock::acquire(&state_dir)?;
        drop(guard);
        // … and a normal combined call works afterwards.
        combined_register(&state_dir, "personal-litellm", "litellm", 4000)?;
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }
}
