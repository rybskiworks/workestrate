//! CID registry: CID→instance mapping, persisted and thread-safe.
//!
//! Persistence follows the port registry's pattern (JSON files under the
//! state dir, mutated under an exclusive file lock — see
//! `port_registry/store.rs` + `lock.rs`).
//!
//! Two deliberate deviations:
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
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Grant-cache TTL: an expired CID cannot be rebound within this window
/// (seconds). Monotonic no-reuse horizon for restart/fork races.
pub const GRANT_CACHE_TTL_SECS: u64 = 300;

/// Supervisor allocation base. Reserve before creating a VM, then assign the
/// returned CID to libkrun through the SDK's per-launch `guest_cid` input.
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

/// File name of the broker VM divert socket under the broker dir: the shim
/// dials here to hand a decided session to the broker VM, whose guest
/// brokerd listens on its divert vsock port. Singleton per state dir, like
/// [`broker_socket_path`].
pub const BROKER_VM_SOCKET_FILE_NAME: &str = "broker-vm.sock";

/// Resolve the host-side broker VM divert socket path. Host-side only —
/// this path must never enter the guest-visible network spec.
pub fn broker_vm_socket_path(state_dir: &Path) -> PathBuf {
    state_dir
        .join("var")
        .join("run")
        .join(BROKER_DIR_NAME)
        .join(BROKER_VM_SOCKET_FILE_NAME)
}

/// File name of the broker egress socket under the broker dir: the broker
/// VM dials here for upstream TCP egress and the host-side forwarder
/// answers. Singleton per state dir. This is the only egress path for the
/// broker VM, whose guest IP stack stays down.
pub const BROKER_EGRESS_SOCKET_FILE_NAME: &str = "broker-egress.sock";

/// Resolve the host-side broker egress socket path. Host-side only.
pub fn broker_egress_socket_path(state_dir: &Path) -> PathBuf {
    state_dir
        .join("var")
        .join("run")
        .join(BROKER_DIR_NAME)
        .join(BROKER_EGRESS_SOCKET_FILE_NAME)
}

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
    /// Last console-provisioned wire epoch for this CID (0 = never
    /// provisioned — the unprovisioned sentinel, never emitted on the wire).
    ///
    /// Mapping note: the console `core.ssh_epoch.provision` frame carries a
    /// `u64` epoch while the launch [`EpochToken`] is 32 bytes of opaque
    /// entropy, so the two are NOT interchangeable encodings of one value.
    /// The token stays the shim-side authentication secret (presented in
    /// every signing envelope and verified by dispatch); this counter is
    /// the guest-visible sequence number provisioned over the console agent
    /// channel. It is monotonic within one binding: initialized to 0 at
    /// bind and bumped (persisted) by every provision emission, so a
    /// retried or re-issued provision always supersedes — never replays —
    /// the previous one. A rebind after the tombstone ages out starts a new
    /// sequence under a new token. Missing in entries written before this
    /// field existed ([`serde(default)`] → 0: unprovisioned).
    #[serde(default)]
    pub wire_epoch: u64,
}

impl CidEntry {
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

    pub fn bind_at(
        &self,
        cid: u32,
        instance: &str,
        epoch: &EpochToken,
        now_secs: u64,
    ) -> anyhow::Result<()> {
        validate_binding(cid, instance)?;
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
            // A fresh binding starts unprovisioned; every provision emission
            // bumps this via [`Self::bump_wire_epoch`] (see the field docs).
            // Rebinding over an aged-out tombstone restarts the sequence
            // under the new token.
            wire_epoch: 0,
        };
        // Even an explicitly bound CID initializes allocator state before its
        // first record. Losing an established counter must not look like a
        // fresh registry on the next automatic allocation.
        let next = read_next_cid(&self.next_cid_path(), &entries)?.max(cid + 1);
        write_next_cid(&self.next_cid_path(), next)?;
        write_entry(&self.broker_dir(), &entry)?;
        entries.insert(cid, entry);
        Ok(())
    }

    /// Allocate a fresh CID (≥ [`CID_ALLOC_BASE`], monotonic) and bind it.
    /// The caller must assign this reservation to the VM before guest execution.
    pub fn allocate(&self, instance: &str, epoch: &EpochToken) -> anyhow::Result<u32> {
        self.allocate_at(instance, epoch, now_secs())
    }

    pub fn allocate_at(
        &self,
        instance: &str,
        epoch: &EpochToken,
        now_secs: u64,
    ) -> anyhow::Result<u32> {
        self.allocate_with_writer(instance, epoch, now_secs, write_entry)
    }

    fn allocate_with_writer(
        &self,
        instance: &str,
        epoch: &EpochToken,
        now_secs: u64,
        persist_entry: impl FnOnce(&Path, &CidEntry) -> anyhow::Result<()>,
    ) -> anyhow::Result<u32> {
        if instance.is_empty() {
            anyhow::bail!("CID registry allocate refused: instance name must not be empty");
        }
        let _lock =
            crate::microsandbox::port_registry::lock::PortRegistryLock::acquire(&self.state_dir)?;
        let mut entries = self.lock_entries()?;
        *entries = read_all_entries(&self.broker_dir())?;
        let candidate = read_next_cid(&self.next_cid_path(), &entries)?;
        if candidate == u32::MAX {
            anyhow::bail!("CID registry allocate: guest CID space exhausted");
        }
        let entry = CidEntry {
            cid: candidate,
            instance: instance.to_string(),
            epoch_hex: epoch.to_hex(),
            bound_at_secs: now_secs,
            expired_at_secs: None,
            // Fresh binding (or a new sequence over an aged-out tombstone
            // slot): starts unprovisioned, see [`Self::bind_at`].
            wire_epoch: 0,
        };
        // Reserve durably before publishing the binding. A failed entry write
        // may leave a gap, but must never permit a later launch to reuse a CID
        // whose reservation might already have escaped this process.
        write_next_cid(
            &self.next_cid_path(),
            candidate.checked_add(1).ok_or_else(|| {
                anyhow::anyhow!("CID registry allocate: guest CID space exhausted")
            })?,
        )?;
        persist_entry(&self.broker_dir(), &entry)?;
        entries.insert(candidate, entry);
        Ok(candidate)
    }

    /// Bump the console wire epoch for a live `cid` and persist it,
    /// returning the new (emittable, nonzero) sequence value.
    ///
    /// Every console provision emission — initial and re-issue — goes
    /// through here BEFORE sending, so a value is never emitted twice even
    /// across a failed send followed by a retry: at-least-once delivery
    /// stays monotonic and newer provisions always supersede older ones.
    /// Refuses unknown or tombstoned CIDs (re-provisioning a dead binding
    /// is a caller error, never silently provisioned).
    pub fn bump_wire_epoch(&self, cid: u32) -> anyhow::Result<u64> {
        let _lock =
            crate::microsandbox::port_registry::lock::PortRegistryLock::acquire(&self.state_dir)?;
        let mut entries = self.lock_entries()?;
        // Re-read under the lock so a concurrent process's bind is visible.
        *entries = read_all_entries(&self.broker_dir())?;
        let mut entry = entries.get(&cid).cloned().ok_or_else(|| {
            anyhow::anyhow!("CID registry bump refused: CID {cid} has no binding")
        })?;
        if !entry.is_live() {
            anyhow::bail!("CID registry bump refused: CID {cid} is expired (tombstoned)");
        }
        entry.wire_epoch = entry.wire_epoch.checked_add(1).ok_or_else(|| {
            anyhow::anyhow!("CID registry bump refused: wire epoch for CID {cid} exhausted u64")
        })?;
        write_entry(&self.broker_dir(), &entry)?;
        let next = entry.wire_epoch;
        entries.insert(cid, entry);
        Ok(next)
    }

    /// Expire the binding for `cid` (tombstone; missing CID is a no-op).
    /// The CID stays unbindable until the TTL elapses.
    pub fn expire(&self, cid: u32) -> anyhow::Result<()> {
        self.expire_at(cid, now_secs())
    }

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

    /// Expire only the exact owned launch. A delayed cleanup must not revoke
    /// a replacement that has since acquired the same numeric CID.
    pub fn expire_binding(
        &self,
        cid: u32,
        instance: &str,
        epoch: &EpochToken,
    ) -> anyhow::Result<bool> {
        let _lock =
            crate::microsandbox::port_registry::lock::PortRegistryLock::acquire(&self.state_dir)?;
        let mut entries = self.lock_entries()?;
        *entries = read_all_entries(&self.broker_dir())?;
        let Some(entry) = entries.get_mut(&cid) else {
            return Ok(false);
        };
        if !entry.is_live() || entry.instance != instance || entry.epoch_hex != epoch.to_hex() {
            return Ok(false);
        }
        entry.expired_at_secs = Some(now_secs());
        write_entry(&self.broker_dir(), entry)?;
        Ok(true)
    }

    /// Returns the number of entries tombstoned.
    pub fn expire_instance(&self, instance: &str) -> anyhow::Result<usize> {
        self.expire_instance_at(instance, now_secs())
    }

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

    pub fn sweep_expired(&self) -> anyhow::Result<usize> {
        self.sweep_expired_at(now_secs())
    }

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
/// A missing, unreadable, malformed or misnamed entry fails the whole snapshot.
/// Silently dropping an authorization record could make its CID reusable.
fn read_all_entries(dir: &Path) -> anyhow::Result<HashMap<u32, CidEntry>> {
    let mut out = HashMap::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(error) => return Err(error).context("read broker CID registry directory"),
    };
    for entry in entries {
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
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("read broker CID record {}", path.display()))?;
        let record: CidEntry = serde_json::from_str(&content)
            .with_context(|| format!("decode broker CID record {}", path.display()))?;
        validate_binding(record.cid, &record.instance)
            .with_context(|| format!("invalid broker CID record {}", path.display()))?;
        EpochToken::from_hex(&record.epoch_hex)
            .with_context(|| format!("invalid launch token in {}", path.display()))?;
        if path.file_name().and_then(|name| name.to_str())
            != Some(format!("cid-{}.json", record.cid).as_str())
        {
            anyhow::bail!(
                "broker CID record filename does not match its CID: {}",
                path.display()
            );
        }
        if out.insert(record.cid, record).is_some() {
            anyhow::bail!("duplicate broker CID record: {}", path.display());
        }
    }
    Ok(out)
}

fn validate_binding(cid: u32, instance: &str) -> anyhow::Result<()> {
    if cid < MIN_GUEST_CID || cid == u32::MAX {
        anyhow::bail!("CID registry bind refused: CID {cid} is reserved");
    }
    if instance.is_empty() {
        anyhow::bail!("CID registry bind refused: instance name must not be empty");
    }
    Ok(())
}

/// Persist one complete entry under the already-held registry lock.
fn write_entry(dir: &Path, entry: &CidEntry) -> anyhow::Result<()> {
    let path = dir.join(format!("cid-{}.json", entry.cid));
    let content = serde_json::to_string_pretty(entry)?;
    write_atomic(&path, content.as_bytes())
}

/// A fresh empty registry may initialize its counter. Existing records without
/// a counter require explicit repair; malformed counters never reset allocation.
/// `u32::MAX` is the persisted exhaustion sentinel, not an allocatable CID.
fn read_next_cid(path: &Path, entries: &HashMap<u32, CidEntry>) -> anyhow::Result<u32> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound && entries.is_empty() => {
            return Ok(CID_ALLOC_BASE);
        }
        Err(e) => return Err(e).context("read broker CID allocation counter"),
    };
    let next: u32 = content
        .trim()
        .parse()
        .context("invalid broker CID allocation counter")?;
    if next < CID_ALLOC_BASE {
        anyhow::bail!("broker CID allocation counter is below its reserved range");
    }
    // Both live and tombstoned records prove that their numbers were already
    // reserved. A parseable but rolled-back counter is not a fresh allocator.
    if entries.keys().any(|cid| *cid >= next) {
        anyhow::bail!("broker CID allocation counter is behind existing reservations");
    }
    Ok(next)
}

fn write_next_cid(path: &Path, next: u32) -> anyhow::Result<()> {
    write_atomic(path, next.to_string().as_bytes())
}

/// Publish a complete owner-only file by same-directory rename. Sync the file
/// before publication and its parent before reporting success. Existing readers
/// see the old or new complete value, never a truncated record/counter.
fn write_atomic(path: &Path, content: &[u8]) -> anyhow::Result<()> {
    let parent = path
        .parent()
        .context("broker registry path has no parent")?;
    std::fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(content)?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    std::fs::File::open(parent)?.sync_all()?;
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
    fn old_owner_cleanup_cannot_expire_a_replacement_binding() {
        let dir = crate::config::test_support::unique_state_dir("broker-owner-cleanup");
        let reg = CidRegistry::open_with_ttl(&dir, 0).unwrap();
        reg.bind(7, "personal-pi", &token(1)).unwrap();
        assert!(!reg.expire_binding(7, "other", &token(1)).unwrap());
        assert!(!reg.expire_binding(7, "personal-pi", &token(2)).unwrap());
        assert!(reg.expire_binding(7, "personal-pi", &token(1)).unwrap());
        reg.bind(7, "personal-pi", &token(2)).unwrap();
        assert!(!reg.expire_binding(7, "personal-pi", &token(1)).unwrap());
        assert_eq!(reg.lookup(7).unwrap().unwrap().epoch_hex, token(2).to_hex());
        assert!(reg.expire_binding(7, "personal-pi", &token(2)).unwrap());
        assert!(!reg.expire_binding(7, "personal-pi", &token(2)).unwrap());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reserved_cids_and_empty_instance_refused() {
        let dir = crate::config::test_support::unique_state_dir("broker-reserved");
        let reg = CidRegistry::open(&dir).unwrap();
        for cid in [0, 1, 2, u32::MAX] {
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
        reg.allocate("work-pi", &token(2)).unwrap();
        reg.bind(7, "personal-pi", &token(1)).unwrap();
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

    #[test]
    fn allocator_refuses_empty_owner_without_publishing_state() {
        let dir = tempfile::tempdir().unwrap();
        let reg = CidRegistry::open(dir.path()).unwrap();
        assert!(reg.allocate("", &token(1)).is_err());
        assert!(!reg.next_cid_path().exists());
        assert_eq!(reg.live_count().unwrap(), 0);
    }

    #[test]
    fn allocator_refuses_corrupt_out_of_range_and_missing_counters() {
        let dir = tempfile::tempdir().unwrap();
        let reg = CidRegistry::open(dir.path()).unwrap();
        let cid = reg.allocate("first", &token(1)).unwrap();
        for invalid in ["", "broken", "0", "2", "65535", "65536", "4294967296"] {
            std::fs::write(reg.next_cid_path(), invalid).unwrap();
            assert!(reg.allocate("second", &token(2)).is_err(), "{invalid:?}");
            assert_eq!(
                std::fs::read_to_string(reg.next_cid_path()).unwrap(),
                invalid
            );
            assert_eq!(reg.live_count().unwrap(), 1);
            assert_eq!(reg.lookup(cid).unwrap().unwrap().instance, "first");
        }
        std::fs::remove_file(reg.next_cid_path()).unwrap();
        assert!(reg.allocate("second", &token(2)).is_err());
        assert!(!reg.next_cid_path().exists());
    }

    #[test]
    fn exhausted_counter_never_publishes_the_wildcard_cid() {
        let dir = tempfile::tempdir().unwrap();
        let reg = CidRegistry::open(dir.path()).unwrap();
        write_next_cid(&reg.next_cid_path(), u32::MAX - 1).unwrap();
        assert_eq!(reg.allocate("last", &token(1)).unwrap(), u32::MAX - 1);
        assert_eq!(
            read_next_cid(&reg.next_cid_path(), &HashMap::new()).unwrap(),
            u32::MAX
        );
        assert!(reg.allocate("overflow", &token(2)).is_err());
        assert!(!reg.entry_path(u32::MAX).exists());
        assert_eq!(reg.live_count().unwrap(), 1);
    }

    #[test]
    fn counter_io_failure_cannot_publish_a_new_binding() {
        let dir = tempfile::tempdir().unwrap();
        let reg = CidRegistry::open(dir.path()).unwrap();
        std::fs::create_dir_all(reg.next_cid_path()).unwrap();
        assert!(reg.allocate("first", &token(1)).is_err());
        assert!(!reg.entry_path(CID_ALLOC_BASE).exists());
        assert_eq!(reg.live_count().unwrap(), 0);
    }

    #[test]
    fn explicit_high_binding_advances_the_same_allocator() {
        let dir = tempfile::tempdir().unwrap();
        let reg = CidRegistry::open(dir.path()).unwrap();
        let explicit = CID_ALLOC_BASE + 37;
        reg.bind(explicit, "explicit", &token(1)).unwrap();
        assert_eq!(reg.allocate("automatic", &token(2)).unwrap(), explicit + 1);
    }

    #[test]
    fn failed_binding_publication_consumes_the_reservation_without_returning_it() {
        let dir = tempfile::tempdir().unwrap();
        let reg = CidRegistry::open(dir.path()).unwrap();
        let error = reg.allocate_with_writer("failed", &token(1), 1000, |_, _| {
            anyhow::bail!("injected entry publication failure")
        });
        assert!(error.is_err());
        assert_eq!(reg.live_count().unwrap(), 0);
        assert!(!reg.entry_path(CID_ALLOC_BASE).exists());
        assert_eq!(
            read_next_cid(&reg.next_cid_path(), &HashMap::new()).unwrap(),
            CID_ALLOC_BASE + 1
        );
        let reopened = CidRegistry::open(dir.path()).unwrap();
        assert_eq!(
            reopened.allocate("next", &token(2)).unwrap(),
            CID_ALLOC_BASE + 1
        );
    }

    #[test]
    fn failed_atomic_replacement_preserves_the_target_and_cleans_scratch() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("occupied");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("marker"), b"preserved").unwrap();
        assert!(write_atomic(&target, b"replacement").is_err());
        assert_eq!(std::fs::read(target.join("marker")).unwrap(), b"preserved");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn corrupt_or_misattributed_records_fail_the_entire_snapshot() {
        let dir = tempfile::tempdir().unwrap();
        let reg = CidRegistry::open(dir.path()).unwrap();
        let cid = reg.allocate("first", &token(1)).unwrap();
        let original = reg.lookup(cid).unwrap().unwrap();
        let path = reg.entry_path(cid);
        std::fs::write(&path, "{").unwrap();
        assert!(reg.refresh().is_err());
        assert!(CidRegistry::open(dir.path()).is_err());
        assert!(reg.allocate("second", &token(2)).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{");

        let mut malformed = original.clone();
        malformed.cid += 1;
        let mut wildcard = original.clone();
        wildcard.cid = u32::MAX;
        let mut unnamed = original.clone();
        unnamed.instance.clear();
        let mut invalid_token = original;
        invalid_token.epoch_hex = "bad-token".into();
        for record in [malformed, wildcard, unnamed, invalid_token] {
            let content = serde_json::to_string(&record).unwrap();
            std::fs::write(&path, &content).unwrap();
            assert!(reg.refresh().is_err());
            assert!(reg.allocate("second", &token(2)).is_err());
            assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
        }
    }

    #[cfg(unix)]
    #[test]
    fn atomic_records_are_owner_only_and_leave_no_scratch_files() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let reg = CidRegistry::open(dir.path()).unwrap();
        let cid = reg.allocate("first", &token(1)).unwrap();
        reg.bump_wire_epoch(cid).unwrap();
        for path in [reg.entry_path(cid), reg.next_cid_path()] {
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert_eq!(std::fs::read_dir(reg.broker_dir()).unwrap().count(), 2);
        assert_eq!(
            CidRegistry::open(dir.path())
                .unwrap()
                .lookup(cid)
                .unwrap()
                .unwrap()
                .wire_epoch,
            1
        );
    }

    #[test]
    fn fresh_bindings_start_unprovisioned() {
        let dir = crate::config::test_support::unique_state_dir("broker-wireinit");
        let reg = CidRegistry::open(&dir).unwrap();
        reg.bind(7, "personal-pi", &token(1)).unwrap();
        let entry = reg.lookup(7).unwrap().expect("bound CID must resolve");
        assert_eq!(entry.wire_epoch, 0, "fresh binds start unprovisioned");
        let allocated = reg.allocate("work-pi", &token(2)).unwrap();
        let entry = reg
            .lookup(allocated)
            .unwrap()
            .expect("allocated CID must resolve");
        assert_eq!(entry.wire_epoch, 0, "fresh allocations start unprovisioned");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bump_wire_epoch_is_monotonic_and_persisted() {
        let dir = crate::config::test_support::unique_state_dir("broker-wirebump");
        let reg = CidRegistry::open(&dir).unwrap();
        reg.bind(7, "personal-pi", &token(1)).unwrap();
        assert_eq!(reg.bump_wire_epoch(7).unwrap(), 1);
        assert_eq!(reg.bump_wire_epoch(7).unwrap(), 2);
        // The bump survives reopen (at-least-once retries never replay).
        let reg2 = CidRegistry::open(&dir).unwrap();
        assert_eq!(reg2.bump_wire_epoch(7).unwrap(), 3);
        let entry = reg2.lookup(7).unwrap().expect("binding must survive");
        assert_eq!(entry.wire_epoch, 3);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn idempotent_rebind_preserves_wire_epoch() {
        let dir = crate::config::test_support::unique_state_dir("broker-wireidem");
        let reg = CidRegistry::open(&dir).unwrap();
        reg.bind(7, "personal-pi", &token(1)).unwrap();
        assert_eq!(reg.bump_wire_epoch(7).unwrap(), 1);
        // The identical re-bind is a no-op: it must not reset the sequence
        // under a guest that already acked it.
        reg.bind(7, "personal-pi", &token(1)).unwrap();
        let entry = reg.lookup(7).unwrap().expect("binding must survive");
        assert_eq!(entry.wire_epoch, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bump_refuses_unknown_and_tombstoned_cids() {
        let dir = crate::config::test_support::unique_state_dir("broker-wiredeny");
        let reg = CidRegistry::open_with_ttl(&dir, 60).unwrap();
        let err = reg.bump_wire_epoch(4242).unwrap_err();
        assert!(err.to_string().contains("no binding"), "got: {err}");
        reg.bind_at(7, "personal-pi", &token(1), 1000).unwrap();
        reg.expire_at(7, 1100).unwrap();
        let err = reg.bump_wire_epoch(7).unwrap_err();
        assert!(err.to_string().contains("tombstoned"), "got: {err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rebind_after_ttl_restarts_the_sequence() {
        let dir = crate::config::test_support::unique_state_dir("broker-wirerebind");
        let reg = CidRegistry::open_with_ttl(&dir, 60).unwrap();
        reg.bind_at(7, "personal-pi", &token(1), 1000).unwrap();
        assert_eq!(reg.bump_wire_epoch(7).unwrap(), 1);
        reg.expire_at(7, 1100).unwrap();
        assert_eq!(reg.sweep_expired_at(1200).unwrap(), 1);
        // A new binding after the tombstone ages out starts a new sequence
        // under the new token (monotonic within a binding, not across).
        reg.bind_at(7, "work-pi", &token(2), 1200).unwrap();
        let entry = reg.lookup(7).unwrap().expect("rebound CID must resolve");
        assert_eq!(entry.wire_epoch, 0);
        assert_eq!(reg.bump_wire_epoch(7).unwrap(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn legacy_entries_without_wire_epoch_read_as_unprovisioned() {
        let dir = crate::config::test_support::unique_state_dir("broker-wirelegacy");
        std::fs::create_dir_all(dir.join("var").join("run").join("broker")).unwrap();
        // An entry written before the field existed carries no wire_epoch.
        std::fs::write(
            dir.join("var")
                .join("run")
                .join("broker")
                .join("cid-7.json"),
            serde_json::json!({
                "cid": 7,
                "instance": "personal-pi",
                "epoch_hex": token(1).to_hex(),
                "bound_at_secs": 1000,
                "expired_at_secs": null,
            })
            .to_string(),
        )
        .unwrap();
        let reg = CidRegistry::open(&dir).unwrap();
        let entry = reg.lookup(7).unwrap().expect("legacy entry must load");
        assert_eq!(
            entry.wire_epoch, 0,
            "missing field defaults to unprovisioned"
        );
        assert_eq!(reg.bump_wire_epoch(7).unwrap(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
