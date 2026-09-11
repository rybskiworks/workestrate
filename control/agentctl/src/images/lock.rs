//! Per-tag image lock: `state/image-locks/<sanitized-key>.lock` (spec 21 §3.3).
//!
//! The guard spans the whole **eval → build → load → record** critical
//! section, which phases C/D hold across MINUTES-LONG `nix build` + `msb
//! load` runs. Acquisition is therefore BLOCKING-UNTIL-ACQUIRED — there is no
//! give-up timeout (the port-registry lock's ~2s timeout would be wrong here:
//! a racing second `up` must WAIT for the in-flight build, then re-check the
//! skew matrix inside the lock — spec §7 "Concurrent same-tag build" row).
//!
//! **Why an O_EXCL lock file and not `flock(2)`:** the crate sets
//! `[lints.rust] unsafe_code = "forbid"`, and `libc::flock` is an unsafe FFI
//! call — unusable without a lint change, which phase B deliberately does not
//! make (the pre-approved `libc` direct-dependency plan is therefore NOT
//! taken: no new dependency is added). The trade: flock(2) would give
//! kernel-side auto-release on crash, whereas a lock FILE survives a crashed
//! holder. That is compensated by the same stale dead-PID recovery the
//! port-registry lock uses (the lock body records `pid=` + epoch; a holder
//! whose PID is dead is provably stale and the file is reclaimed — see
//! [`crate::microsandbox::port_registry::lock::lock_file_is_stale`], reused
//! rather than duplicated). Upgrading to kernel-release flock(2) — gated on a
//! lint/unsafe policy decision — is a recorded follow-up for phases C/D.
//!
//! This is a cooperative advisory lock between workestrate processes (msb has
//! its own internal per-ref/per-layer locks below it — spec §3.3); `Drop`
//! removes the file.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};

use crate::images::state::image_locks_dir;
use crate::microsandbox::port_registry::lock::lock_file_is_stale;

/// Backoff between lock-acquisition attempts. Small enough that a released
/// lock is picked up promptly, large enough that a minutes-long hold does not
/// busy-spin.
const LOCK_ACQUIRE_BACKOFF: Duration = Duration::from_millis(25);

/// Lock-file name for a `<repo>#<tag>` key: `<sanitized-key>.lock`.
pub fn lock_file_name(key: &str) -> String {
    format!("{}.lock", sanitize_lock_key(key))
}

/// Sanitize a `<repo>#<tag>` key for filesystem safety. The key contains
/// path-hostile characters by construction (`#`, `:`, and `/` when the
/// repo_key is a canonical path), so the filename is a percent-encoding:
/// `[A-Za-z0-9._-]` pass through (the allowlist spirit of
/// `microsandbox::port_registry::slug`'s alphabet), every other BYTE becomes
/// `%XX` (uppercase hex). Unlike a bare replace-with-dash, percent-encoding
/// is INJECTIVE — distinct keys never collide on one lock file — and
/// UTF-8-safe (encoding is over bytes).
pub fn sanitize_lock_key(key: &str) -> String {
    let mut out = String::with_capacity(key.len());
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    for b in key.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'.' | b'_' | b'-' => out.push(b as char),
            _ => {
                out.push('%');
                out.push(HEX[(b >> 4) as usize] as char);
                out.push(HEX[(b & 0x0F) as usize] as char);
            }
        }
    }
    out
}

/// RAII guard for the per-tag image lock.
///
/// Acquisition O_EXCL-creates `<state_dir>/image-locks/<sanitized-key>.lock`
/// (the create fails while another holder's file exists) and writes a
/// `pid=`/`created_at_epoch=` body for the staleness probe; [`Drop`] removes
/// the file.
///
/// **Blocking:** retries forever (with [`LOCK_ACQUIRE_BACKOFF`]) until the
/// lock is acquired — a live holder means a build/load is in flight and the
/// waiter must queue behind it (spec §3.3 duplicate-work avoidance). Only
/// non-`AlreadyExists` I/O errors bail.
///
/// **Staleness:** if the file exists but its recorded PID is dead (holder
/// crashed mid-build), the file is removed and acquisition retried — the
/// O_EXCL-not-flock(2) compensation described in the module docs.
pub struct ImageTagLock {
    path: PathBuf,
}

impl ImageTagLock {
    /// Acquire the per-tag lock, blocking until acquired. `key` is the
    /// `<repo>#<tag>` composite ([`crate::images::state::image_key`]).
    pub fn acquire(state_dir: &Path, key: &str) -> Result<Self> {
        let dir = image_locks_dir(state_dir);
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;
        let path = dir.join(lock_file_name(key));
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
                    // only feeds the staleness probe.
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
                    // Live holder: a build/load is in flight for this tag.
                    // Block and retry — no give-up deadline (see module docs).
                    std::thread::sleep(LOCK_ACQUIRE_BACKOFF);
                }
                Err(e) => {
                    return Err(e).with_context(|| {
                        format!("failed to acquire image lock {}", path.display())
                    });
                }
            }
        }
    }

    /// Path of the held lock file (inspection/testing).
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for ImageTagLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
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
    use crate::config::test_support::unique_state_dir;

    #[test]
    fn sanitize_lock_key_encodes_path_hostile_chars_injectively() {
        assert_eq!(
            sanitize_lock_key("personal#workestrate-pi:latest"),
            "personal%23workestrate-pi%3Alatest",
            "`#` and `:` percent-encode; letters/`.`/`-` pass through"
        );
        // Unregistered repo_keys are canonical paths: `/` encodes too.
        assert_eq!(
            sanitize_lock_key("/home/node/repo#img:1"),
            "%2Fhome%2Fnode%2Frepo%23img%3A1"
        );
        // Injectivity: keys that a replace-with-dash scheme would collapse
        // stay distinct.
        assert_ne!(
            sanitize_lock_key("a#b:c"),
            sanitize_lock_key("a-b-c"),
            "percent-encoding must never collide distinct keys"
        );
        // `%` itself encodes, so encoded output never spoof-matches.
        assert_eq!(sanitize_lock_key("a%23b"), "a%2523b");
    }

    #[test]
    fn lock_is_released_on_drop_and_reacquirable() -> Result<()> {
        let state_dir = unique_state_dir("img-lock-release");
        let key = "personal#workestrate-pi:latest";
        {
            let guard = ImageTagLock::acquire(&state_dir, key)?;
            assert!(guard.path().exists(), "lock file exists while held");
            assert!(
                guard.path().starts_with(image_locks_dir(&state_dir)),
                "lock lives under state/image-locks/"
            );
            // The body records the holder PID for the staleness probe.
            let body = std::fs::read_to_string(guard.path())?;
            assert!(
                body.contains(&format!("pid={}", std::process::id())),
                "lock body records the holder pid: {body}"
            );
        }
        // A second acquisition must succeed immediately after the drop.
        let guard2 = ImageTagLock::acquire(&state_dir, key)?;
        drop(guard2);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// Spec §3.3 duplicate-work avoidance: a second acquirer BLOCKS until the
    /// first holder drops (no timeout failure).
    #[test]
    fn contending_guard_blocks_until_first_drops() -> Result<()> {
        let state_dir = unique_state_dir("img-lock-contention");
        let key = "personal#tempest:latest".to_string();
        let first = ImageTagLock::acquire(&state_dir, &key)?;

        let dir = state_dir.clone();
        let (acquired_tx, acquired_rx) = std::sync::mpsc::channel::<()>();
        let waiter = std::thread::spawn(move || {
            let second = ImageTagLock::acquire(&dir, &key).expect("waiter acquires eventually");
            acquired_tx.send(()).expect("signal acquisition");
            second
        });

        // While the first guard is held, the waiter must NOT have acquired —
        // 250ms is ~10x the acquire backoff, so a non-blocking implementation
        // would have failed or won by now.
        std::thread::sleep(Duration::from_millis(250));
        assert!(
            acquired_rx.try_recv().is_err(),
            "waiter must block while the first guard is held"
        );

        drop(first);
        let second = waiter.join().expect("waiter thread panicked");
        acquired_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("waiter acquired after the first guard dropped");
        drop(second);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// Stale-lock recovery: a lock file whose recorded holder PID is dead
    /// (holder crashed mid-build) is reclaimed instead of blocking forever.
    /// Mirrors the dead-PID derivation trick in
    /// `microsandbox::port_registry::lock` tests: spawn `true`, reap it, reuse
    /// the provably-defunct PID.
    #[test]
    fn stale_lock_from_dead_pid_is_recovered() -> Result<()> {
        let state_dir = unique_state_dir("img-lock-stale");
        let key = "personal#workestrate-pi:latest";
        let dir = image_locks_dir(&state_dir);
        std::fs::create_dir_all(&dir)?;

        let dead_pid = {
            let mut child = std::process::Command::new("true")
                .spawn()
                .expect("spawn 'true'");
            let pid = child.id();
            child.wait().expect("wait for 'true'");
            pid
        };
        std::fs::write(
            dir.join(lock_file_name(key)),
            format!("pid={dead_pid}\ncreated_at_epoch=1\n"),
        )?;

        // Acquisition must detect the dead PID, reclaim the file, and proceed
        // — a naive blocking loop would hang here forever.
        let guard = ImageTagLock::acquire(&state_dir, key)?;
        drop(guard);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// Distinct tags have distinct lock files and do NOT serialize against
    /// each other.
    #[test]
    fn distinct_tags_lock_independently() -> Result<()> {
        let state_dir = unique_state_dir("img-lock-distinct");
        let a = ImageTagLock::acquire(&state_dir, "personal#a:1")?;
        let b = ImageTagLock::acquire(&state_dir, "personal#b:1")?;
        assert_ne!(a.path(), b.path());
        drop(a);
        drop(b);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }
}
