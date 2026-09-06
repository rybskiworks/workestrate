//! Signing service skeleton: request/response types, the grant-enforcement
//! pipeline, and a deterministic test backend. No real crypto.
//!
//! Normative check order (BEFORE any signing): instance grant → key
//! authorization → signature type → namespace → request limits. Denials are
//! typed ([`Denial`]) so the shim and audit log render precise reasons.
//!
//! SSHSIG domain separation: a grant for `namespace = "git"` authorizes
//! ONLY that exact namespace string. `namespace = "file"` (or any other)
//! against a `git` grant is denied, and the `raw` scheme is denied always —
//! the existing credential-schema grant vocabulary ([`crate::microsandbox::plan::SigningGrantPlan`])
//! carries no scheme field, so no grant can authorize raw: fail-closed.
//!
//! CBOR: [`SignRequest`]/[`SignResponse`] are plain serde structs decoded
//! with [`ciborium`] (already in the lockfile via the microsandbox SDK —
//! promoted to a direct dep). Codec helpers ([`encode_cbor`]/[`decode_cbor`])
//! are the single wiring point the shim uses. Note: `payload: Vec<u8>`
//! encodes as a CBOR array of ints (correct, slightly verbose); switching
//! to `serde_bytes::ByteBuf` for a byte-string encoding is a future
//! optimization, out of scope for this change.

use crate::microsandbox::broker::audit::{AuditLog, AuditRecord, payload_digest_hex};
use crate::microsandbox::plan::{CredentialsPlan, SigningGrantPlan};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Limits (configurable via LimitsConfig; constants are the defaults)
// ---------------------------------------------------------------------------

/// Default cap on a single signing payload (1 MiB — SSHSIG payloads are
/// orders of magnitude smaller; this is a fail-closed abuse bound).
pub const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;
/// Default per-instance signing rate (requests per 60s sliding window).
pub const DEFAULT_RATE_PER_MINUTE: u32 = 60;
/// Default per-instance concurrent in-flight signing cap.
pub const DEFAULT_MAX_CONCURRENT_PER_INSTANCE: usize = 8;
/// Default per-request backend timeout (seconds).
pub const DEFAULT_REQUEST_TIMEOUT_SECS: u64 = 30;
/// Sliding-window length for rate limiting (seconds).
pub const RATE_WINDOW_SECS: u64 = 60;

/// Tunable request limits. `Default` yields the `DEFAULT_*` constants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LimitsConfig {
    pub max_payload_bytes: usize,
    pub max_requests_per_minute: u32,
    pub max_concurrent_per_instance: usize,
    /// Sync-backend enforcement here is post-hoc (elapsed > timeout ⇒
    /// [`Denial::Timeout`], signature discarded — fail-closed). True
    /// cancellation of a hung backend is follow-up async work.
    pub request_timeout: Duration,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_payload_bytes: MAX_PAYLOAD_BYTES,
            max_requests_per_minute: DEFAULT_RATE_PER_MINUTE,
            max_concurrent_per_instance: DEFAULT_MAX_CONCURRENT_PER_INSTANCE,
            request_timeout: Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECS),
        }
    }
}

// ---------------------------------------------------------------------------
// Wire types
// ---------------------------------------------------------------------------

/// Signature scheme requested. Only [`SignatureScheme::SshSig`] is
/// authorizable (see module docs); `Raw` exists so its denial is explicit
/// and typed rather than a parse error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SignatureScheme {
    SshSig,
    Raw,
}

impl std::fmt::Display for SignatureScheme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SignatureScheme::SshSig => write!(f, "ssh-sig"),
            SignatureScheme::Raw => write!(f, "raw"),
        }
    }
}

/// One signing request (guest → broker). No instance field by design: the
/// shim stamps the AUTHORITATIVE instance from the transport CID and passes
/// it separately to [`SigningService::sign`] — a body-claimed identity
/// could never be trusted here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignRequest {
    pub operation_id: String,
    pub key_reference: String,
    pub signature_scheme: SignatureScheme,
    pub namespace: String,
    pub payload: Vec<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional_context: Option<String>,
}

/// One signing response (broker → guest).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignResponse {
    pub operation_id: String,
    pub signature: Vec<u8>,
    pub key_id: String,
}

/// Encode any serde value to CBOR bytes (the wire codec).
pub fn encode_cbor<T: Serialize>(value: &T) -> anyhow::Result<Vec<u8>> {
    let mut buf = Vec::new();
    ciborium::into_writer(value, &mut buf)
        .map_err(|e| anyhow::anyhow!("CBOR encode failed: {e}"))?;
    Ok(buf)
}

/// Decode CBOR bytes into any serde value.
pub fn decode_cbor<T: DeserializeOwned>(bytes: &[u8]) -> anyhow::Result<T> {
    ciborium::from_reader(bytes).map_err(|e| anyhow::anyhow!("CBOR decode failed: {e}"))
}

// ---------------------------------------------------------------------------
// Typed denials
// ---------------------------------------------------------------------------

/// Every way the pipeline can refuse BEFORE signing. Each renders a stable
/// audit reason via [`Display`](std::fmt::Display).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Denial {
    /// No grant store entry for this instance at all.
    UnknownInstance { instance: String },
    /// Instance known but holds zero signing grants.
    NoGrant { instance: String },
    /// Instance holds signing grants, but none for this key reference.
    UnknownKey { key_reference: String },
    /// The matching grant is guest-bound: real material already lives in
    /// the guest via secret delivery, so the broker path stays closed
    /// (avoids dual custody of one key).
    GuestBoundMaterial { key_reference: String },
    /// Only `ssh-sig` is authorizable; `raw` has no grant vocabulary.
    SchemeNotAuthorized { scheme: String },
    /// Exact-match namespace failure (SSHSIG domain separation).
    NamespaceMismatch { expected: String, got: String },
    /// Payload exceeds the configured cap.
    PayloadTooLarge { bytes: usize, max: usize },
    /// Sliding-window rate exceeded.
    RateLimited,
    /// Per-instance in-flight cap reached.
    ConcurrencyExhausted,
    /// Backend exceeded the request timeout (signature discarded).
    Timeout,
    /// The key backend itself failed (no real crypto in this change — only the
    /// test backend's synthetic failures).
    BackendError { detail: String },
}

impl std::fmt::Display for Denial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Denial::UnknownInstance { instance } => {
                write!(f, "unknown instance '{instance}': no broker grants")
            }
            Denial::NoGrant { instance } => {
                write!(f, "instance '{instance}' holds no signing grants")
            }
            Denial::UnknownKey { key_reference } => {
                write!(
                    f,
                    "unknown key reference '{key_reference}' for this instance"
                )
            }
            Denial::GuestBoundMaterial { key_reference } => write!(
                f,
                "key '{key_reference}' is guest-bound: material lives in the guest, broker signing closed"
            ),
            Denial::SchemeNotAuthorized { scheme } => {
                write!(
                    f,
                    "signature scheme '{scheme}' is not authorized by any grant"
                )
            }
            Denial::NamespaceMismatch { expected, got } => write!(
                f,
                "namespace mismatch: grant authorizes '{expected}', request asked '{got}'"
            ),
            Denial::PayloadTooLarge { bytes, max } => {
                write!(f, "payload too large: {bytes} bytes exceeds cap of {max}")
            }
            Denial::RateLimited => write!(f, "rate limit exceeded"),
            Denial::ConcurrencyExhausted => write!(f, "per-instance concurrency limit reached"),
            Denial::Timeout => write!(f, "signing backend timed out"),
            Denial::BackendError { detail } => write!(f, "signing backend failed: {detail}"),
        }
    }
}

impl std::error::Error for Denial {}

// ---------------------------------------------------------------------------
// Grant store (compiled from the existing credential-schema plan types — consumed, never modified)
// ---------------------------------------------------------------------------

/// Per-instance compiled signing grants.
#[derive(Debug, Clone, Default)]
pub struct InstanceGrants {
    pub signing: HashMap<String, SigningGrantPlan>,
}

/// The broker's grant store: instance → compiled grants. Built once at
/// plan-render time from each workload's [`CredentialsPlan`].
#[derive(Debug, Clone, Default)]
pub struct GrantStore {
    instances: HashMap<String, InstanceGrants>,
}

impl GrantStore {
    /// Compile `(instance, plan)` pairs into the store. Guest-bound grants
    /// are INCLUDED (they exist in the plan) — the pipeline denies them at
    /// signing time with [`Denial::GuestBoundMaterial`] so the denial is
    /// audited rather than invisible.
    pub fn compile(plans: &[(&str, &CredentialsPlan)]) -> Self {
        let mut store = Self::default();
        for (instance, plan) in plans {
            let grants = InstanceGrants {
                signing: plan
                    .signing
                    .iter()
                    .map(|g| (g.name.clone(), g.clone()))
                    .collect(),
            };
            store.instances.insert((*instance).to_string(), grants);
        }
        store
    }

    /// Whether the store knows this instance at all.
    pub fn knows_instance(&self, instance: &str) -> bool {
        self.instances.contains_key(instance)
    }
}

// ---------------------------------------------------------------------------
// Key backends (no real crypto in this change)
// ---------------------------------------------------------------------------

/// Signs authorized payloads. This change ships only [`TestBackend`]; a real
/// SSH-agent/HSM backend implements this trait as follow-up work.
pub trait KeyBackend: Send + Sync + std::fmt::Debug {
    fn sign(
        &self,
        key_ref: &str,
        scheme: &SignatureScheme,
        namespace: &str,
        payload: &[u8],
    ) -> Result<Vec<u8>, String>;
}

/// Deterministic fake signatures for tests and control-plane bring-up:
/// `TESTSIGv1:<key_ref>:<namespace>:<hex-sha256(payload)>`. Deterministic
/// so golden assertions are possible; obviously not a signature.
#[derive(Debug, Clone, Default)]
pub struct TestBackend;

impl KeyBackend for TestBackend {
    fn sign(
        &self,
        key_ref: &str,
        _scheme: &SignatureScheme,
        namespace: &str,
        payload: &[u8],
    ) -> Result<Vec<u8>, String> {
        Ok(format!(
            "TESTSIGv1:{key_ref}:{namespace}:{}",
            payload_digest_hex(payload)
        )
        .into_bytes())
    }
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

/// Mutable limit state (behind the service's `Mutex`).
#[derive(Debug, Default)]
struct ServiceState {
    /// Per-instance sliding-window request timestamps (epoch secs, ascending).
    windows: HashMap<String, VecDeque<u64>>,
    /// Per-instance in-flight signing count.
    in_flight: HashMap<String, usize>,
}

/// Grant-enforcing signing service over a [`KeyBackend`].
pub struct SigningService<B: KeyBackend> {
    grants: GrantStore,
    limits: LimitsConfig,
    backend: B,
    state: Mutex<ServiceState>,
}

impl<B: KeyBackend> std::fmt::Debug for SigningService<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SigningService")
            .field("limits", &self.limits)
            .field("backend", &self.backend)
            .finish()
    }
}

impl<B: KeyBackend> SigningService<B> {
    pub fn new(grants: GrantStore, limits: LimitsConfig, backend: B) -> Self {
        Self {
            grants,
            limits,
            backend,
            state: Mutex::new(ServiceState::default()),
        }
    }

    /// Sign one request for the AUTHORITATIVE instance (stamped by the shim
    /// from the transport CID — never guest-claimed). Every outcome appends
    /// exactly one audit record (allow or deny) carrying the payload digest.
    pub fn sign(
        &self,
        instance: &str,
        cid: u32,
        request: &SignRequest,
        audit: &AuditLog,
        timestamp: &str,
    ) -> Result<SignResponse, Denial> {
        self.sign_at(instance, cid, request, audit, timestamp, current_secs())
    }

    /// [`Self::sign`] with an explicit clock (tests drive the rate window).
    pub fn sign_at(
        &self,
        instance: &str,
        cid: u32,
        request: &SignRequest,
        audit: &AuditLog,
        timestamp: &str,
        now_secs: u64,
    ) -> Result<SignResponse, Denial> {
        let digest = payload_digest_hex(&request.payload);
        // Deny helper: audits the denial, then returns it. Audit-write
        // failure is stderr-loud but never masks the denial itself.
        let deny = |denial: Denial| -> Result<SignResponse, Denial> {
            let record = AuditRecord::deny(
                timestamp,
                instance,
                cid,
                &request.key_reference,
                request.signature_scheme.to_string(),
                &request.namespace,
                denial.to_string(),
                &digest,
            );
            if let Err(e) = audit.append(record) {
                eprintln!("WARNING: broker audit append failed: {e}");
            }
            Err(denial)
        };

        // 1. Instance grant.
        let instance_grants = match self.grants.instances.get(instance) {
            Some(g) => g,
            None => {
                return deny(Denial::UnknownInstance {
                    instance: instance.to_string(),
                });
            }
        };
        if instance_grants.signing.is_empty() {
            return deny(Denial::NoGrant {
                instance: instance.to_string(),
            });
        }
        // 2. Key authorization.
        let grant = match instance_grants.signing.get(&request.key_reference) {
            Some(g) => g,
            None => {
                return deny(Denial::UnknownKey {
                    key_reference: request.key_reference.clone(),
                });
            }
        };
        if grant.binding == crate::microsandbox::plan::CredentialBinding::Guest {
            return deny(Denial::GuestBoundMaterial {
                key_reference: request.key_reference.clone(),
            });
        }
        // 3. Signature type.
        if request.signature_scheme != SignatureScheme::SshSig {
            return deny(Denial::SchemeNotAuthorized {
                scheme: request.signature_scheme.to_string(),
            });
        }
        // 4. Namespace (exact match — domain separation).
        if grant.namespace != request.namespace {
            return deny(Denial::NamespaceMismatch {
                expected: grant.namespace.clone(),
                got: request.namespace.clone(),
            });
        }
        // 5. Request limits (payload cap → rate → concurrency).
        if request.payload.len() > self.limits.max_payload_bytes {
            return deny(Denial::PayloadTooLarge {
                bytes: request.payload.len(),
                max: self.limits.max_payload_bytes,
            });
        }
        if self.rate_limited(instance, now_secs) {
            return deny(Denial::RateLimited);
        }
        if !self.try_acquire(instance) {
            return deny(Denial::ConcurrencyExhausted);
        }
        // Backend call with post-hoc timeout (see LimitsConfig docs).
        let started = std::time::Instant::now();
        let backend_outcome = self.backend.sign(
            &request.key_reference,
            &request.signature_scheme,
            &request.namespace,
            &request.payload,
        );
        self.release(instance);
        if started.elapsed() > self.limits.request_timeout {
            return deny(Denial::Timeout);
        }
        let signature = match backend_outcome {
            Ok(sig) => sig,
            Err(detail) => return deny(Denial::BackendError { detail }),
        };

        let response = SignResponse {
            operation_id: request.operation_id.clone(),
            signature,
            key_id: grant.material.clone(),
        };
        let record = AuditRecord::allow(
            timestamp,
            instance,
            cid,
            &request.key_reference,
            request.signature_scheme.to_string(),
            &request.namespace,
            &digest,
        );
        if let Err(e) = audit.append(record) {
            eprintln!("WARNING: broker audit append failed: {e}");
        }
        Ok(response)
    }

    /// Sliding-window check-and-record: prune timestamps older than
    /// [`RATE_WINDOW_SECS`]; deny (without recording) when the window is
    /// full, else record `now_secs` and allow. Lock-poisoned ⇒ fail-closed
    /// (deny) — a broken limiter must not become an open limiter.
    fn rate_limited(&self, instance: &str, now_secs: u64) -> bool {
        let mut state = match self.state.lock() {
            Ok(s) => s,
            Err(_) => return true,
        };
        let window = state.windows.entry(instance.to_string()).or_default();
        while window
            .front()
            .is_some_and(|t| now_secs.saturating_sub(*t) >= RATE_WINDOW_SECS)
        {
            window.pop_front();
        }
        if window.len() >= self.limits.max_requests_per_minute as usize {
            return true;
        }
        window.push_back(now_secs);
        false
    }

    /// Acquire one in-flight slot; false when at cap (or lock-poisoned —
    /// fail-closed).
    fn try_acquire(&self, instance: &str) -> bool {
        let mut state = match self.state.lock() {
            Ok(s) => s,
            Err(_) => return false,
        };
        let count = state.in_flight.entry(instance.to_string()).or_insert(0);
        if *count >= self.limits.max_concurrent_per_instance {
            return false;
        }
        *count += 1;
        true
    }

    /// Release one in-flight slot (best-effort on poisoned lock).
    fn release(&self, instance: &str) {
        if let Ok(mut state) = self.state.lock()
            && let Some(count) = state.in_flight.get_mut(instance)
        {
            *count = count.saturating_sub(1);
        }
    }
}

fn current_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::config::SecretViolationPolicy;
    use crate::microsandbox::broker::audit::AuditResult;
    use crate::microsandbox::plan::{CredentialBinding, SigningGrantPlan};

    fn grant(name: &str, namespace: &str, binding: CredentialBinding) -> SigningGrantPlan {
        SigningGrantPlan {
            name: name.to_string(),
            material: "SIGN_KEY".to_string(),
            namespace: namespace.to_string(),
            on_violation: SecretViolationPolicy::Block,
            binding,
        }
    }

    fn plan_with(grants: Vec<SigningGrantPlan>) -> CredentialsPlan {
        CredentialsPlan {
            ssh: vec![],
            signing: grants,
            strict: false,
            strict_origin: None,
        }
    }

    fn request(key: &str, scheme: SignatureScheme, namespace: &str, payload: &[u8]) -> SignRequest {
        SignRequest {
            operation_id: "op-1".to_string(),
            key_reference: key.to_string(),
            signature_scheme: scheme,
            namespace: namespace.to_string(),
            payload: payload.to_vec(),
            optional_context: None,
        }
    }

    fn service_for(
        grants: Vec<SigningGrantPlan>,
        limits: LimitsConfig,
    ) -> SigningService<TestBackend> {
        let plan = plan_with(grants);
        let store = GrantStore::compile(&[("personal-pi", &plan)]);
        SigningService::new(store, limits, TestBackend)
    }

    #[test]
    fn happy_path_signs_and_audits_allow() {
        let svc = service_for(
            vec![grant("rel", "git", CredentialBinding::Broker)],
            LimitsConfig::default(),
        );
        let audit = AuditLog::memory();
        let req = request("rel", SignatureScheme::SshSig, "git", b"payload-bytes");
        let resp = svc
            .sign_at("personal-pi", 7, &req, &audit, "2026-09-06T00:00:00Z", 1000)
            .unwrap();
        assert_eq!(resp.operation_id, "op-1");
        assert_eq!(resp.key_id, "SIGN_KEY");
        assert!(
            resp.signature.starts_with(b"TESTSIGv1:rel:git:"),
            "deterministic fake shape"
        );
        let records = audit.snapshot();
        assert_eq!(records.len(), 1);
        assert!(matches!(records[0].result, AuditResult::Allow));
        assert_eq!(
            records[0].payload_digest,
            payload_digest_hex(b"payload-bytes")
        );
    }

    #[test]
    fn ungranted_instance_denied() {
        let svc = service_for(
            vec![grant("rel", "git", CredentialBinding::Broker)],
            LimitsConfig::default(),
        );
        let audit = AuditLog::memory();
        let err = svc
            .sign_at(
                "unknown-box",
                9,
                &request("rel", SignatureScheme::SshSig, "git", b"x"),
                &audit,
                "t",
                1000,
            )
            .unwrap_err();
        assert!(matches!(err, Denial::UnknownInstance { .. }), "got: {err}");
        assert_eq!(audit.len(), 1, "denials are audited too");
    }

    #[test]
    fn instance_without_signing_grants_denied() {
        let plan = plan_with(vec![]);
        let store = GrantStore::compile(&[("personal-pi", &plan)]);
        let svc = SigningService::new(store, LimitsConfig::default(), TestBackend);
        let audit = AuditLog::memory();
        let err = svc
            .sign_at(
                "personal-pi",
                7,
                &request("rel", SignatureScheme::SshSig, "git", b"x"),
                &audit,
                "t",
                1000,
            )
            .unwrap_err();
        assert!(matches!(err, Denial::NoGrant { .. }), "got: {err}");
    }

    #[test]
    fn unknown_key_and_guest_bound_denied() {
        let svc = service_for(
            vec![
                grant("rel", "git", CredentialBinding::Broker),
                grant("local", "git", CredentialBinding::Guest),
            ],
            LimitsConfig::default(),
        );
        let audit = AuditLog::memory();
        let err = svc
            .sign_at(
                "personal-pi",
                7,
                &request("nope", SignatureScheme::SshSig, "git", b"x"),
                &audit,
                "t",
                1000,
            )
            .unwrap_err();
        assert!(matches!(err, Denial::UnknownKey { .. }), "got: {err}");
        let err = svc
            .sign_at(
                "personal-pi",
                7,
                &request("local", SignatureScheme::SshSig, "git", b"x"),
                &audit,
                "t",
                1000,
            )
            .unwrap_err();
        assert!(
            matches!(err, Denial::GuestBoundMaterial { .. }),
            "got: {err}"
        );
        assert_eq!(audit.len(), 2);
    }

    #[test]
    fn namespace_domain_separation_git_vs_file_and_raw() {
        let svc = service_for(
            vec![grant("rel", "git", CredentialBinding::Broker)],
            LimitsConfig::default(),
        );
        let audit = AuditLog::memory();
        // git grant must NOT authorize namespace=file.
        let err = svc
            .sign_at(
                "personal-pi",
                7,
                &request("rel", SignatureScheme::SshSig, "file", b"x"),
                &audit,
                "t",
                1000,
            )
            .unwrap_err();
        assert!(
            matches!(err, Denial::NamespaceMismatch { .. }),
            "file namespace against git grant must deny: {err}"
        );
        // raw scheme is never authorized, even with a matching namespace.
        let err = svc
            .sign_at(
                "personal-pi",
                7,
                &request("rel", SignatureScheme::Raw, "git", b"x"),
                &audit,
                "t",
                1000,
            )
            .unwrap_err();
        assert!(
            matches!(err, Denial::SchemeNotAuthorized { .. }),
            "raw scheme must deny: {err}"
        );
        // Namespace comparison is exact: prefix tricks fail.
        let err = svc
            .sign_at(
                "personal-pi",
                7,
                &request("rel", SignatureScheme::SshSig, "git-extra", b"x"),
                &audit,
                "t",
                1000,
            )
            .unwrap_err();
        assert!(
            matches!(err, Denial::NamespaceMismatch { .. }),
            "got: {err}"
        );
    }

    #[test]
    fn oversized_payload_denied_before_backend() {
        let limits = LimitsConfig {
            max_payload_bytes: 8,
            ..LimitsConfig::default()
        };
        let svc = service_for(vec![grant("rel", "git", CredentialBinding::Broker)], limits);
        let audit = AuditLog::memory();
        let err = svc
            .sign_at(
                "personal-pi",
                7,
                &request("rel", SignatureScheme::SshSig, "git", b"nine-byte"),
                &audit,
                "t",
                1000,
            )
            .unwrap_err();
        assert!(matches!(err, Denial::PayloadTooLarge { .. }), "got: {err}");
    }

    #[test]
    fn rate_limit_sliding_window() {
        let limits = LimitsConfig {
            max_requests_per_minute: 2,
            ..LimitsConfig::default()
        };
        let svc = service_for(vec![grant("rel", "git", CredentialBinding::Broker)], limits);
        let audit = AuditLog::memory();
        let req = || request("rel", SignatureScheme::SshSig, "git", b"x");
        assert!(
            svc.sign_at("personal-pi", 7, &req(), &audit, "t", 1000)
                .is_ok()
        );
        assert!(
            svc.sign_at("personal-pi", 7, &req(), &audit, "t", 1000)
                .is_ok()
        );
        let err = svc
            .sign_at("personal-pi", 7, &req(), &audit, "t", 1000)
            .unwrap_err();
        assert!(matches!(err, Denial::RateLimited), "got: {err}");
        // Window slides: 61s later the budget is back.
        assert!(
            svc.sign_at("personal-pi", 7, &req(), &audit, "t", 1061)
                .is_ok()
        );
    }

    #[test]
    fn concurrency_cap_denies_when_full() {
        let limits = LimitsConfig {
            max_concurrent_per_instance: 1,
            ..LimitsConfig::default()
        };
        let svc = service_for(vec![grant("rel", "git", CredentialBinding::Broker)], limits);
        assert!(svc.try_acquire("personal-pi"));
        assert!(
            !svc.try_acquire("personal-pi"),
            "second slot must fail at cap 1"
        );
        svc.release("personal-pi");
        assert!(svc.try_acquire("personal-pi"), "released slot reusable");
        svc.release("personal-pi");
    }

    #[test]
    fn request_cbor_round_trip() {
        let req = SignRequest {
            operation_id: "op-9".to_string(),
            key_reference: "rel".to_string(),
            signature_scheme: SignatureScheme::SshSig,
            namespace: "git".to_string(),
            payload: b"data".to_vec(),
            optional_context: Some("ctx".to_string()),
        };
        let bytes = encode_cbor(&req).unwrap();
        let back: SignRequest = decode_cbor(&bytes).unwrap();
        assert_eq!(req, back);
        assert!(decode_cbor::<SignRequest>(b"\xff\xfe not cbor").is_err());
    }
}
