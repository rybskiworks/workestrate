//! Append-only broker audit log.
//!
//! Every signing decision — allow AND deny — appends one record carrying
//! the payload DIGEST (hex SHA-256), never the payload. The record type's
//! construction API only accepts a precomputed digest string, so a full
//! payload cannot reach the log by accident: there is no field for it.
//!
//! Persistence: JSONL appended under `${state_dir}/var/log/broker-audit.jsonl`
//! (the `var/` state-tree convention the port registry's `var/run/` lives
//! in; no prior audit/log infra existed to reuse). In-memory records are
//! always kept (tests + `ps`-style inspection); the file is best-effort —
//! an append failure is reported to the caller but never fails the signing
//! decision it records.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// File name of the JSONL audit log under `${state_dir}/var/log/`.
pub const AUDIT_FILE_NAME: &str = "broker-audit.jsonl";

/// SHA-256 hex digest of a signing payload. The digest is what the audit
/// log stores; the full payload never leaves the request path.
pub fn payload_digest_hex(payload: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(payload);
    hex::encode(hasher.finalize())
}

/// The outcome recorded for one signing decision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditResult {
    Allow,
    Deny { reason: String },
}

/// One append-only audit record: timestamp, instance, key id, scheme,
/// namespace, result, payload DIGEST. There is deliberately no payload
/// field — see the module docs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditRecord {
    pub timestamp: String,
    pub instance: String,
    pub cid: u32,
    pub key_id: String,
    pub scheme: String,
    pub namespace: String,
    pub result: AuditResult,
    pub payload_digest: String,
}

impl AuditRecord {
    /// Build an allow record. `payload_digest` must come from
    /// [`payload_digest_hex`] — the type is a plain String (serde-friendly
    /// for JSONL), but no constructor accepts raw payload bytes.
    #[allow(clippy::too_many_arguments)]
    pub fn allow(
        timestamp: impl Into<String>,
        instance: impl Into<String>,
        cid: u32,
        key_id: impl Into<String>,
        scheme: impl Into<String>,
        namespace: impl Into<String>,
        payload_digest: impl Into<String>,
    ) -> Self {
        Self {
            timestamp: timestamp.into(),
            instance: instance.into(),
            cid,
            key_id: key_id.into(),
            scheme: scheme.into(),
            namespace: namespace.into(),
            result: AuditResult::Allow,
            payload_digest: payload_digest.into(),
        }
    }

    /// Build a deny record (same digest rule as [`Self::allow`]).
    #[allow(clippy::too_many_arguments)]
    pub fn deny(
        timestamp: impl Into<String>,
        instance: impl Into<String>,
        cid: u32,
        key_id: impl Into<String>,
        scheme: impl Into<String>,
        namespace: impl Into<String>,
        reason: impl Into<String>,
        payload_digest: impl Into<String>,
    ) -> Self {
        Self {
            timestamp: timestamp.into(),
            instance: instance.into(),
            cid,
            key_id: key_id.into(),
            scheme: scheme.into(),
            namespace: namespace.into(),
            result: AuditResult::Deny {
                reason: reason.into(),
            },
            payload_digest: payload_digest.into(),
        }
    }

    /// Build an allow record for one SSH divert decision. The diverted
    /// destination (`host:port`) rides `key_id` as the routing identity —
    /// the same way signing records carry the key reference there — while
    /// the digest field carries only the digest of that string, never raw
    /// bytes. Note the destination is low-entropy (a dictionary attack
    /// recovers it trivially): the digest keeps the record shape uniform,
    /// it is not secrecy.
    pub fn ssh_divert_allow(
        timestamp: impl Into<String>,
        instance: impl Into<String>,
        cid: u32,
        host: &str,
        port: u16,
    ) -> Self {
        let destination = format!("{host}:{port}");
        let digest = payload_digest_hex(destination.as_bytes());
        Self::allow(
            timestamp,
            instance,
            cid,
            destination,
            "ssh-divert",
            "divert",
            digest,
        )
    }

    /// Build a deny record for one SSH divert decision (same destination
    /// and digest conventions as [`Self::ssh_divert_allow`]).
    pub fn ssh_divert_deny(
        timestamp: impl Into<String>,
        instance: impl Into<String>,
        cid: u32,
        host: &str,
        port: u16,
        reason: impl Into<String>,
    ) -> Self {
        let destination = format!("{host}:{port}");
        let digest = payload_digest_hex(destination.as_bytes());
        Self::deny(
            timestamp,
            instance,
            cid,
            destination,
            "ssh-divert",
            "divert",
            reason,
            digest,
        )
    }
}

/// Thread-safe append-only audit sink: in-memory records plus an optional
/// JSONL file under the state dir.
pub struct AuditLog {
    records: Mutex<Vec<AuditRecord>>,
    file: Option<PathBuf>,
}

impl std::fmt::Debug for AuditLog {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuditLog")
            .field("file", &self.file)
            .finish()
    }
}

impl AuditLog {
    /// In-memory-only log (tests, synthetic paths).
    pub fn memory() -> Self {
        Self {
            records: Mutex::new(Vec::new()),
            file: None,
        }
    }

    /// Persistent log: `${state_dir}/var/log/broker-audit.jsonl`.
    /// The directory is created on first append, not here.
    pub fn with_state_dir(state_dir: &Path) -> Self {
        Self {
            records: Mutex::new(Vec::new()),
            file: Some(audit_file_path(state_dir)),
        }
    }

    /// Append one record to memory and (best-effort) to the JSONL file.
    /// A file-append failure is returned as Err AND stashed on stderr, but
    /// the in-memory record is always kept — audit write failure must never
    /// lose the decision it records.
    pub fn append(&self, record: AuditRecord) -> anyhow::Result<()> {
        let line = serde_json::to_string(&record)?;
        {
            let mut records = self
                .records
                .lock()
                .map_err(|e| anyhow::anyhow!("audit log lock poisoned: {e}"))?;
            records.push(record);
        }
        if let Some(path) = &self.file {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)?;
            if let Err(e) = writeln!(file, "{line}") {
                eprintln!(
                    "WARNING: broker audit append failed for {}: {e}",
                    path.display()
                );
                return Err(e.into());
            }
        }
        Ok(())
    }

    /// Snapshot of the in-memory records (inspection order = append order).
    pub fn snapshot(&self) -> Vec<AuditRecord> {
        self.records.lock().map(|r| r.clone()).unwrap_or_default()
    }

    /// Number of in-memory records.
    pub fn len(&self) -> usize {
        self.records.lock().map(|r| r.len()).unwrap_or(0)
    }

    /// Whether any in-memory records exist.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Resolve the JSONL audit path for a state dir.
pub fn audit_file_path(state_dir: &Path) -> PathBuf {
    state_dir.join("var").join("log").join(AUDIT_FILE_NAME)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn sample_record() -> AuditRecord {
        AuditRecord::allow(
            "2026-09-06T00:00:00Z",
            "personal-pi",
            7,
            "deploy",
            "ssh-sig",
            "git",
            payload_digest_hex(b"hello"),
        )
    }

    #[test]
    fn digest_is_hex_sha256_not_payload() {
        let digest = payload_digest_hex(b"hello");
        assert_eq!(
            digest,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        assert!(!digest.contains("hello"));
    }

    #[test]
    fn serialized_record_contains_digest_never_payload() {
        let record = sample_record();
        let json = serde_json::to_string(&record).unwrap();
        assert!(
            json.contains("payload_digest"),
            "digest field must exist: {json}"
        );
        assert!(json.contains("2cf24dba5fb0a30e26e83b2ac5b9e29e"), "{json}");
        assert!(
            !json.contains("hello"),
            "raw payload must never appear: {json}"
        );
        // Shape check: exactly the normative fields, no payload field.
        for field in [
            "timestamp",
            "instance",
            "key_id",
            "scheme",
            "namespace",
            "result",
            "payload_digest",
        ] {
            assert!(json.contains(field), "missing field {field}: {json}");
        }
        assert!(!json.contains("payload\":"), "no raw-payload field: {json}");
    }

    #[test]
    fn memory_append_and_snapshot() {
        let log = AuditLog::memory();
        assert!(log.is_empty());
        log.append(sample_record()).unwrap();
        assert_eq!(log.len(), 1);
        let snap = log.snapshot();
        assert_eq!(snap[0].instance, "personal-pi");
        assert!(matches!(snap[0].result, AuditResult::Allow));
    }

    #[test]
    fn file_append_writes_jsonl() {
        let state_dir = crate::config::test_support::unique_state_dir("broker-audit-file");
        let log = AuditLog::with_state_dir(&state_dir);
        log.append(sample_record()).unwrap();
        log.append(AuditRecord::deny(
            "2026-09-06T00:00:01Z",
            "personal-pi",
            7,
            "deploy",
            "ssh-sig",
            "file",
            "namespace mismatch",
            payload_digest_hex(b"hello"),
        ))
        .unwrap();
        let content = std::fs::read_to_string(audit_file_path(&state_dir)).unwrap();
        assert_eq!(content.lines().count(), 2, "one JSON object per line");
        assert!(!content.contains("hello"), "file must never hold payloads");
        let _ = std::fs::remove_dir_all(&state_dir);
    }
}
