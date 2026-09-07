//! CID registry: CID→instance mapping, persisted and thread-safe.
//!
//! Persistence follows the port registry's pattern (JSON files under the
//! state dir, mutated under an exclusive file lock — see
//! `port_registry/store.rs` + `lock.rs`).
//!
//! Two deliberate deviations, documented:
//!
//! 1. Location is `${state_dir}/var/run/broker/` (a SUBDIRECTORY), not
//!    `var/run/` directly: the port registry globs `var/run/*.json` and
//!    would log loud corruption warnings for foreign files. The subdir
//!    keeps both registries' globs clean.
//! 2. Locking reuses [`crate::microsandbox::port_registry::lock::PortRegistryLock`]
//!    instead of a second lock protocol: one lock for the whole `var/run`
//!    tree means broker and port operations serialize rather than deadlock.
//!
//! No-reuse rule: a CID bound to (instance, epoch) can never be rebound to
//! a different binding while live, and a tombstoned (expired) CID stays
//! unbindable until [`GRANT_CACHE_TTL_SECS`] elapses — this closes
//! CID-reuse-after-restart inside the grant-cache window.

use crate::microsandbox::broker::epoch::EpochToken;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Grant-cache TTL: an expired CID cannot be rebound within this window
/// (seconds). Monotonic no-reuse horizon for restart/fork races.
pub const GRANT_CACHE_TTL_SECS: u64 = 300;

/// Synthetic allocator base: real guest CIDs come from libkrun
/// (small integers); broker-side synthetic allocation stays out of that
/// range. [`CidRegistry::allocate`] is for tests and future bookkeeping —
/// production binds arrive via [`CidRegistry::bind`] with the libkrun CID.
pub const CID_ALLOC_BASE: u32 = 1 << 16;

/// Minimum bindable CID: 0 (invalid), 1 (hypervisor), and 2 (host) are
/// reserved by vsock and can never identify a workload VM.
pub const MIN_GUEST_CID: u32 = 3;

/// Directory name under `${state_dir}/var/run/` holding CID entries.
pub const BROKER_DIR_NAME: &str = "broker";

/// File name of the broker unix socket under the broker dir (singleton
/// topology — one socket per state dir, no port glob).
pub const BROKER_SOCKET_FILE_NAME: &str = "broker.sock";

/// Resolve the host-side broker socket path: the shim listens here and the
/// runtime dials it for diverted sessions. Host-side only — this path must
/// never enter the guest-visible network spec.
pub fn broker_socket_path(state_dir: &Path) -> PathBuf {
    state_dir
        .join("var")
        .join("run")
        .join(BROKER_DIR_NAME)
        .join(BROKER_SOCKET_FILE_NAME)
}

/// File holding the synthetic allocator's next-candidate CID.
const NEXT_CID_FILE_NAME: &str = "next-cid";

/// One CID→instance binding with its launch epoch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CidEntry {
    pub cid: u32,
    pub instance: String,
    /// Hex epoch token bound at launch ([`EpochToken::to_hex`]).
    pub epoch_hex: String,
    /// Unix epoch seconds when bound.
    pub bound_at_secs: u64,
    /// Unix epoch seconds when expired (`None` = live). A tombstoned entry
    /// blocks rebinding until the TTL elapses.
    #[serde(default)]
    pub expired_at_secs: Option<u64>,
}

impl CidEntry {
    /// Whether the entry is live (not tombstoned).
    pub fn is_live(&self) -> bool {
        self.expired_at_secs.is_none()
    }

    /// Whether the tombstone is old enough at `now_secs` to allow rebinding.
    pub fn tombstone_aged_out(&self, now_secs: u64, ttl_secs: u64) -> bool {
        match self.expired_at_secs {
            None => false,
            Some(expired) => now_secs.saturating_sub(expired) >= ttl_secs,
        }
    }
}

/// Current Unix epoch seconds (test seam: time-injectable `*_at` variants
/// take the clock explicitly; this is the production default).
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Thread-safe (`Mutex` + file lock), persisted CID→instance registry.
/// File-authoritative across processes: [`Self::refresh`] reloads the
/// snapshot from disk (the shim calls it per dispatch so a bind performed
/// by a concurrent launch process is visible).
pub struct CidRegistry {
    state_dir: PathBuf,
    ttl_secs: u64,
    entries: Mutex<HashMap<u32, CidEntry>>,
}

impl std::fmt::Debug for CidRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CidRegistry")
            .field("state_dir", &self.state_dir)
            .field("ttl_secs", &self.ttl_secs)
            .finish()
    }
}

impl CidRegistry {
    /// Open (creating dirs as needed) and load the persisted snapshot,
    /// with the default grant-cache TTL.
    pub fn open(state_dir: &Path) -> anyhow::Result<Self> {
        Self::open_with_ttl(state_dir, GRANT_CACHE_TTL_SECS)
    }

    /// Open with an explicit TTL (tests use short TTLs with `*_at` clocks).
    pub fn open_with_ttl(state_dir: &Path, ttl_secs: u64) -> anyhow::Result<Self> {
        let registry = Self {
            state_dir: state_dir.to_path_buf(),
            ttl_secs,
            entries: Mutex::new(HashMap::new()),
        };
        registry.refresh()?;
        Ok(registry)
    }

    fn broker_dir(&self) -> PathBuf {
        self.state_dir.join("var").join("run").join(BROKER_DIR_NAME)
    }

    fn entry_path(&self, cid: u32) -> PathBuf {
        self.broker_dir().join(format!("cid-{cid}.json"))
    }

    fn next_cid_path(&self) -> PathBuf {
        self.broker_dir().join(NEXT_CID_FILE_NAME)
    }

    fn lock_entries(&self) -> anyhow::Result<std::sync::MutexGuard<'_, HashMap<u32, CidEntry>>> {
        self.entries
            .lock()
            .map_err(|e| anyhow::anyhow!("CID registry lock poisoned: {e}"))
    }

    /// Reload the in-memory snapshot from disk (cross-process visibility).
    pub fn refresh(&self) -> anyhow::Result<()> {
        // Shared file lock: one lock for the var/run tree (see module docs).
        let _lock =
            crate::microsandbox::port_registry::lock::PortRegistryLock::acquire(&self.state_dir)?;
        let snapshot = read_all_entries(&self.broker_dir())?;
        *self.lock_entries()? = snapshot;
        Ok(())
    }

    /// Look up the live entry for `cid` (`None` = unknown or tombstoned).
    pub fn lookup(&self, cid: u32) -> anyhow::Result<Option<CidEntry>> {
        Ok(self
            .lock_entries()?
            .get(&cid)
            .filter(|e| e.is_live())
            .cloned())
    }

    /// Number of live entries in the snapshot.
    pub fn live_count(&self) -> anyhow::Result<usize> {
        Ok(self
            .lock_entries()?
            .values()
            .filter(|e| e.is_live())
            .count())
    }

    /// Authoritatively bind `cid` → (`instance`, epoch) at launch.
    /// Idempotent for the identical binding; refuses any rebinding of a
    /// live CID and any rebinding inside the grant-cache TTL.
    pub fn bind(&self, cid: u32, instance: &str, epoch: &EpochToken) -> anyhow::Result<()> {
        self.bind_at(cid, instance, epoch, now_secs())
    }

    /// [`Self::bind`] with an explicit clock (tests).
    pub fn bind_at(
        &self,
        cid: u32,
        instance: &str,
        epoch: &EpochToken,
        now_secs: u64,
    ) -> anyhow::Result<()> {
        if cid < MIN_GUEST_CID {
            anyhow::bail!("CID registry bind refused: CID {cid} is reserved (vsock 0/1/2)");
        }
        if instance.is_empty() {
            anyhow::bail!("CID registry bind refused: instance name must not be empty");
        }
        let _lock =
            crate::microsandbox::port_registry::lock::PortRegistryLock::acquire(&self.state_dir)?;
        let mut entries = self.lock_entries()?;
        // Re-read under the lock so a concurrent process's bind is visible.
        *entries = read_all_entries(&self.broker_dir())?;
        if let Some(existing) = entries.get(&cid) {
            if existing.is_live() {
                if existing.instance == instance && existing.epoch_hex == epoch.to_hex() {
                    return Ok(()); // idempotent re-bind of the identical launch
                }
                anyhow::bail!(
                    "CID registry bind refused: CID {cid} is live-bound to instance '{}' \
                     (no-reuse while live)",
                    existing.instance
                );
            }
            if !existing.tombstone_aged_out(now_secs, self.ttl_secs) {
                anyhow::bail!(
                    "CID registry bind refused: CID {cid} expired recently — rebind blocked \
                     within the grant-cache TTL ({}s)",
                    self.ttl_secs
                );
            }
        }
        let entry = CidEntry {
            cid,
            instance: instance.to_string(),
            epoch_hex: epoch.to_hex(),
            bound_at_secs: now_secs,
            expired_at_secs: None,
        };
        write_entry(&self.broker_dir(), &entry)?;
        entries.insert(cid, entry);
        Ok(())
    }

    /// Allocate a fresh synthetic CID (≥ [`CID_ALLOC_BASE`], monotonic) and
    /// bind it. Production launches use [`Self::bind`] with the libkrun CID.
    pub fn allocate(&self, instance: &str, epoch: &EpochToken) -> anyhow::Result<u32> {
        self.allocate_at(instance, epoch, now_secs())
    }

    /// [`Self::allocate`] with an explicit clock (tests).
    pub fn allocate_at(
        &self,
        instance: &str,
        epoch: &EpochToken,
        now_secs: u64,
    ) -> anyhow::Result<u32> {
        let _lock =
            crate::microsandbox::port_registry::lock::PortRegistryLock::acquire(&self.state_dir)?;
        let mut entries = self.lock_entries()?;
        *entries = read_all_entries(&self.broker_dir())?;
        let mut candidate = read_next_cid(&self.next_cid_path())?;
        loop {
            let blocked = match entries.get(&candidate) {
                None => false,
                Some(e) => e.is_live() || !e.tombstone_aged_out(now_secs, self.ttl_secs),
            };
            if !blocked {
                break;
            }
            candidate = candidate.checked_add(1).ok_or_else(|| {
                anyhow::anyhow!("CID registry allocate: synthetic CID space exhausted")
            })?;
        }
        let entry = CidEntry {
            cid: candidate,
            instance: instance.to_string(),
            epoch_hex: epoch.to_hex(),
            bound_at_secs: now_secs,
            expired_at_secs: None,
        };
        write_entry(&self.broker_dir(), &entry)?;
        entries.insert(candidate, entry);
        write_next_cid(
            &self.next_cid_path(),
            candidate.checked_add(1).ok_or_else(|| {
                anyhow::anyhow!("CID registry allocate: synthetic CID space exhausted")
            })?,
        )?;
        Ok(candidate)
    }

    /// Expire the binding for `cid` (tombstone; missing CID is a no-op).
    /// The CID stays unbindable until the TTL elapses.
    pub fn expire(&self, cid: u32) -> anyhow::Result<()> {
        self.expire_at(cid, now_secs())
    }

    /// [`Self::expire`] with an explicit clock (tests).
    pub fn expire_at(&self, cid: u32, now_secs: u64) -> anyhow::Result<()> {
        let _lock =
            crate::microsandbox::port_registry::lock::PortRegistryLock::acquire(&self.state_dir)?;
        let mut entries = self.lock_entries()?;
        *entries = read_all_entries(&self.broker_dir())?;
        if let Some(entry) = entries.get_mut(&cid) {
            entry.expired_at_secs = Some(now_secs);
            let updated = entry.clone();
            write_entry(&self.broker_dir(), &updated)?;
        }
        Ok(())
    }

    /// Expire every binding for `instance` (launch-teardown path).
    /// Returns the number of entries tombstoned.
    pub fn expire_instance(&self, instance: &str) -> anyhow::Result<usize> {
        self.expire_instance_at(instance, now_secs())
    }

    /// [`Self::expire_instance`] with an explicit clock (tests).
    pub fn expire_instance_at(&self, instance: &str, now_secs: u64) -> anyhow::Result<usize> {
        let _lock =
            crate::microsandbox::port_registry::lock::PortRegistryLock::acquire(&self.state_dir)?;
        let mut entries = self.lock_entries()?;
        *entries = read_all_entries(&self.broker_dir())?;
        let mut count = 0;
        for entry in entries
            .values_mut()
            .filter(|e| e.instance == instance && e.is_live())
        {
            entry.expired_at_secs = Some(now_secs);
            let updated = entry.clone();
            write_entry(&self.broker_dir(), &updated)?;
            count += 1;
        }
        Ok(count)
    }

    /// Delete tombstones older than the TTL. Returns the number removed.
    pub fn sweep_expired(&self) -> anyhow::Result<usize> {
        self.sweep_expired_at(now_secs())
    }

    /// [`Self::sweep_expired`] with an explicit clock (tests).
    pub fn sweep_expired_at(&self, now_secs: u64) -> anyhow::Result<usize> {
        let _lock =
            crate::microsandbox::port_registry::lock::PortRegistryLock::acquire(&self.state_dir)?;
        let mut entries = self.lock_entries()?;
        *entries = read_all_entries(&self.broker_dir())?;
        let aged: Vec<u32> = entries
            .values()
            .filter(|e| !e.is_live() && e.tombstone_aged_out(now_secs, self.ttl_secs))
            .map(|e| e.cid)
            .collect();
        let mut removed = 0;
        for cid in aged {
            let path = self.entry_path(cid);
            if path.exists() {
                std::fs::remove_file(&path)?;
            }
            entries.remove(&cid);
            removed += 1;
        }
        Ok(removed)
    }
}

/// Read every `cid-*.json` file in `dir` (missing dir = empty snapshot).
/// A corrupt entry file is a loud warning + skip — the port registry's
/// WP10/A12 `read_record_loud` idiom, applied to the broker subtree.
fn read_all_entries(dir: &Path) -> anyhow::Result<HashMap<u32, CidEntry>> {
    let mut out = HashMap::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .is_none_or(|n| !n.starts_with("cid-"))
        {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                eprintln!(
                    "WARNING: unreadable broker CID record {}: {e}",
                    path.display()
                );
                continue;
            }
        };
        match serde_json::from_str::<CidEntry>(&content) {
            Ok(record) => {
                out.insert(record.cid, record);
            }
            Err(e) => {
                eprintln!("WARNING: corrupt broker CID record {}: {e}", path.display());
            }
        }
    }
    Ok(out)
}

/// Persist one entry (plain write under the already-held file lock — the
/// port registry's `store.rs` idiom).
fn write_entry(dir: &Path, entry: &CidEntry) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("cid-{}.json", entry.cid));
    let content = serde_json::to_string_pretty(entry)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Read the allocator counter (missing/corrupt = [`CID_ALLOC_BASE`],
/// fail-open to the base — allocation scans for a free CID regardless).
fn read_next_cid(path: &Path) -> anyhow::Result<u32> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(CID_ALLOC_BASE),
        Err(_) => return Ok(CID_ALLOC_BASE),
    };
    Ok(content.trim().parse().unwrap_or(CID_ALLOC_BASE))
}

fn write_next_cid(path: &Path, next: u32) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, next.to_string())?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::microsandbox::broker::epoch::EPOCH_BYTES;

    fn token(byte: u8) -> EpochToken {
        EpochToken::from_bytes([byte; EPOCH_BYTES])
    }

    #[test]
    fn bind_and_lookup_round_trip() {
        let dir = crate::config::test_support::unique_state_dir("broker-bind");
        let reg = CidRegistry::open(&dir).unwrap();
        reg.bind(7, "personal-pi", &token(1)).unwrap();
        let entry = reg.lookup(7).unwrap().expect("bound CID must resolve");
        assert_eq!(entry.instance, "personal-pi");
        assert_eq!(entry.epoch_hex, token(1).to_hex());
        assert!(entry.is_live());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bind_is_idempotent_for_identical_binding() {
        let dir = crate::config::test_support::unique_state_dir("broker-idem");
        let reg = CidRegistry::open(&dir).unwrap();
        reg.bind(7, "personal-pi", &token(1)).unwrap();
        reg.bind(7, "personal-pi", &token(1)).unwrap();
        assert_eq!(reg.live_count().unwrap(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn live_cid_cannot_be_rebound() {
        let dir = crate::config::test_support::unique_state_dir("broker-noreuse");
        let reg = CidRegistry::open(&dir).unwrap();
        reg.bind(7, "personal-pi", &token(1)).unwrap();
        let err = reg.bind(7, "work-pi", &token(2)).unwrap_err();
        assert!(err.to_string().contains("no-reuse"), "got: {err}");
        // Same instance, rotated epoch (restart without expire) also refused.
        let err = reg.bind(7, "personal-pi", &token(9)).unwrap_err();
        assert!(err.to_string().contains("no-reuse"), "got: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reserved_cids_and_empty_instance_refused() {
        let dir = crate::config::test_support::unique_state_dir("broker-reserved");
        let reg = CidRegistry::open(&dir).unwrap();
        for cid in [0, 1, 2] {
            assert!(reg.bind(cid, "personal-pi", &token(1)).is_err());
        }
        assert!(reg.bind(7, "", &token(1)).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn snapshot_persists_across_handles() {
        let dir = crate::config::test_support::unique_state_dir("broker-persist");
        {
            let reg = CidRegistry::open(&dir).unwrap();
            reg.bind(7, "personal-pi", &token(1)).unwrap();
        }
        let reg2 = CidRegistry::open(&dir).unwrap();
        let entry = reg2.lookup(7).unwrap().expect("must survive reopen");
        assert_eq!(entry.instance, "personal-pi");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expired_cid_blocked_within_ttl_then_rebindable_after_sweep() {
        let dir = crate::config::test_support::unique_state_dir("broker-expiry");
        let reg = CidRegistry::open_with_ttl(&dir, 60).unwrap();
        reg.bind_at(7, "personal-pi", &token(1), 1000).unwrap();
        reg.expire_at(7, 1100).unwrap();
        assert!(reg.lookup(7).unwrap().is_none(), "tombstoned CID hides");
        // Within TTL: rebind refused (restart race closed).
        let err = reg.bind_at(7, "work-pi", &token(2), 1150).unwrap_err();
        assert!(err.to_string().contains("grant-cache TTL"), "got: {err}");
        // Sweep before TTL: nothing removed.
        assert_eq!(reg.sweep_expired_at(1150).unwrap(), 0);
        // After TTL: sweep removes the tombstone, rebind succeeds.
        assert_eq!(reg.sweep_expired_at(1200).unwrap(), 1);
        reg.bind_at(7, "work-pi", &token(2), 1200).unwrap();
        assert_eq!(reg.lookup(7).unwrap().expect("rebound").instance, "work-pi");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn expire_instance_tombstones_all_of_instance() {
        let dir = crate::config::test_support::unique_state_dir("broker-exp-inst");
        let reg = CidRegistry::open(&dir).unwrap();
        reg.bind(7, "personal-pi", &token(1)).unwrap();
        reg.bind(8, "personal-pi", &token(1)).unwrap();
        reg.bind(9, "work-pi", &token(2)).unwrap();
        assert_eq!(reg.expire_instance("personal-pi").unwrap(), 2);
        assert!(reg.lookup(7).unwrap().is_none());
        assert!(reg.lookup(9).unwrap().is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn allocate_is_monotonic_and_persisted() {
        let dir = crate::config::test_support::unique_state_dir("broker-alloc");
        let reg = CidRegistry::open(&dir).unwrap();
        let a = reg.allocate("personal-pi", &token(1)).unwrap();
        let b = reg.allocate("work-pi", &token(2)).unwrap();
        assert!(a >= CID_ALLOC_BASE);
        assert!(b > a, "allocator must be monotonic");
        // Counter survives reopen (no-reuse across restarts).
        let reg2 = CidRegistry::open(&dir).unwrap();
        let c = reg2.allocate("other-pi", &token(3)).unwrap();
        assert!(c > b);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn broker_files_do_not_pollute_port_registry_glob() {
        let dir = crate::config::test_support::unique_state_dir("broker-isolation");
        let reg = CidRegistry::open(&dir).unwrap();
        reg.bind(7, "personal-pi", &token(1)).unwrap();
        reg.allocate("work-pi", &token(2)).unwrap();
        let records = crate::microsandbox::port_registry::list_records(&dir).unwrap();
        assert!(
            records.is_empty(),
            "broker files must stay out of var/run/*.json"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn concurrent_binds_from_threads_all_succeed() {
        let dir = crate::config::test_support::unique_state_dir("broker-threads");
        let reg = std::sync::Arc::new(CidRegistry::open(&dir).unwrap());
        let mut handles = Vec::new();
        for i in 0..8u32 {
            let reg = std::sync::Arc::clone(&reg);
            handles.push(std::thread::spawn(move || {
                reg.bind(10 + i, &format!("inst-{i}"), &token(i as u8))
            }));
        }
        for h in handles {
            h.join().expect("worker panicked").unwrap();
        }
        assert_eq!(reg.live_count().unwrap(), 8);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
