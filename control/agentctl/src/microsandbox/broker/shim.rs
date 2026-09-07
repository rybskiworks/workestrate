//! Broker shim: host-side unix-socket listener + transport trait.
//!
//! Topology: guests dial CID 2:<port> (HOST_CID); the msb bridge maps
//! that to a host unix socket. The shim listens on that socket and hands
//! decoded requests to the [`SigningService`](super::signing::SigningService).
//! The [`BrokerTransport`] trait separates the shim from the in-VM service
//! path so follow-up work can plug a relay transport into the broker VM without
//! touching dispatch logic.
//!
//! Authoritative identity: [`dispatch`] stamps the transport-supplied CID
//! and resolves the instance via the [`CidRegistry`](super::registry::CidRegistry),
//! overriding any body-claimed instance. It then verifies the presented
//! epoch against the registry-bound epoch before the request reaches the
//! service. A spoofed claim can therefore never escalate: at worst the
//! attacker receives another instance's DENIAL.
//!
//! Wire framing on the unix socket: `u32` big-endian length prefix + one
//! CBOR [`WireEnvelope`]. CBOR via the [`super::signing`] codec helpers.

use crate::microsandbox::broker::audit::{AuditLog, AuditRecord, payload_digest_hex};
use crate::microsandbox::broker::epoch::EpochToken;
use crate::microsandbox::broker::registry::CidRegistry;
use crate::microsandbox::broker::signing::{
    Denial, GrantStore, KeyBackend, SignRequest, SignResponse, SigningService, decode_cbor,
    encode_cbor,
};
use microsandbox_network::ssh::gateway::{SshDivertPrelude, decode_ssh_divert_prelude};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Default broker port (guest dials `CID 2:<port>`; msb bridges to the host
/// unix socket). This may become config in follow-up work; this change fixes the constant.
pub const BROKER_PORT: u32 = 22099;

/// Maximum single envelope frame (8 MiB — comfortably above the 1 MiB
/// default payload cap plus envelope overhead; a fail-closed abuse bound on
/// the socket read, checked BEFORE allocation loops).
pub const MAX_FRAME_BYTES: usize = 8 * 1024 * 1024;

/// Maximum accepted wall-clock skew between a divert prelude's epoch and
/// the broker clock (seconds, in EITHER direction). Past it the prelude is
/// stale (replay) or from the future (clock lie); either way fail closed.
/// A zero broker clock also fails closed — without a clock, freshness is
/// unverifiable.
pub const MAX_DIVERT_EPOCH_SKEW_SECS: u64 = 300;

/// The peer identity the transport vouches for. The CID comes from the
/// transport (vsock bridge metadata in production, the test/fake transport
/// in unit tests) — it is AUTHORITATIVE and overrides `claimed_instance`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportPeer {
    pub cid: u32,
}

/// One wire message: the guest's (untrusted) instance claim, its epoch
/// prelude, and the signing request body. The shim resolves the REAL
/// instance from `peer.cid` and never trusts `claimed_instance`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireEnvelope {
    pub claimed_instance: String,
    pub epoch: EpochToken,
    pub body: SignRequest,
}

/// Outcome of one dispatch: a signed response, a typed denial, or a
/// transport/protocol failure (CBOR decode, unknown CID, stale epoch).
/// Denials and auth failures audit; undecodable frames cannot (no trustworthy
/// fields) and are stderr-loud instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchOutcome {
    Signed(SignResponse),
    Denied(Denial),
    Rejected { reason: String, re_attest: bool },
}

impl DispatchOutcome {
    /// Whether the guest should re-attest (stale epoch / unknown CID).
    pub fn re_attest(&self) -> bool {
        match self {
            DispatchOutcome::Signed(_) | DispatchOutcome::Denied(_) => false,
            DispatchOutcome::Rejected { re_attest, .. } => *re_attest,
        }
    }
}

/// Byte transport between the shim and the in-VM side. This change ships
/// [`UnixSocketTransport`]; follow-up work adds the in-VM relay behind this trait.
pub trait BrokerTransport: Send + Sync {
    /// Accept one connection: the vouched peer CID plus the decoded
    /// envelope. Implementations perform framing + CBOR decode; CID
    /// resolution is transport-specific (see [`CidResolver`]).
    fn accept(&self) -> std::io::Result<(TransportPeer, WireEnvelope)>;
}

/// Resolves the authoritative peer CID for an accepted unix-socket
/// connection. A production vsock-relay bridge derives this from the vsock-relay
/// metadata; tests pin a fixed CID. Returning `None` rejects the
/// connection before any byte is trusted.
pub trait CidResolver: Send + Sync + std::fmt::Debug {
    fn resolve_cid(&self, stream: &std::os::unix::net::UnixStream) -> Option<u32>;
}

/// Test/fixture resolver: every connection vouches one fixed CID.
#[derive(Debug, Clone)]
pub struct FixedCidResolver(pub u32);

impl CidResolver for FixedCidResolver {
    fn resolve_cid(&self, _stream: &std::os::unix::net::UnixStream) -> Option<u32> {
        Some(self.0)
    }
}

/// Host-side unix-socket listener implementing [`BrokerTransport`].
/// Binds `socket_path`, reads one length-prefixed CBOR envelope per
/// connection, and stamps the CID from its [`CidResolver`].
pub struct UnixSocketTransport<R: CidResolver> {
    listener: std::os::unix::net::UnixListener,
    resolver: R,
}

impl<R: CidResolver> UnixSocketTransport<R> {
    /// Bind the listener (removing a stale socket file first — a leftover
    /// from an unclean shutdown must not wedge the next launch).
    pub fn bind(socket_path: &Path, resolver: R) -> std::io::Result<Self> {
        if socket_path.exists() {
            std::fs::remove_file(socket_path)?;
        }
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let listener = std::os::unix::net::UnixListener::bind(socket_path)?;
        Ok(Self { listener, resolver })
    }

    /// Read one length-prefixed frame (fail-closed at [`MAX_FRAME_BYTES`]).
    fn read_frame(stream: &mut std::os::unix::net::UnixStream) -> std::io::Result<Vec<u8>> {
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf)?;
        let len = u32::from_be_bytes(len_buf) as usize;
        if len == 0 || len > MAX_FRAME_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("broker frame length {len} out of bounds (max {MAX_FRAME_BYTES})"),
            ));
        }
        let mut buf = vec![0u8; len];
        stream.read_exact(&mut buf)?;
        Ok(buf)
    }

    /// Write one length-prefixed frame.
    pub fn write_frame(
        stream: &mut std::os::unix::net::UnixStream,
        bytes: &[u8],
    ) -> std::io::Result<()> {
        if bytes.len() > MAX_FRAME_BYTES {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "broker response frame exceeds bound",
            ));
        }
        stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
        stream.write_all(bytes)?;
        Ok(())
    }
}

impl<R: CidResolver + Send + Sync> BrokerTransport for UnixSocketTransport<R> {
    fn accept(&self) -> std::io::Result<(TransportPeer, WireEnvelope)> {
        let (mut stream, _) = self.listener.accept()?;
        let cid = self.resolver.resolve_cid(&stream).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "broker: no CID vouched",
            )
        })?;
        let frame = Self::read_frame(&mut stream)?;
        let envelope: WireEnvelope = decode_cbor(&frame)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        Ok((TransportPeer { cid }, envelope))
    }
}

impl<R: CidResolver> UnixSocketTransport<R> {
    /// Accept one connection and read its SSH divert prelude, KEEPING the
    /// stream for the relay (unlike [`BrokerTransport::accept`], which
    /// consumes it). The prelude decoder consumes the framed
    /// (length-prefixed) form, so the prefix stripped by the shared
    /// bounds-checked read is re-attached before decoding.
    pub fn accept_divert(
        &self,
    ) -> std::io::Result<(
        TransportPeer,
        std::os::unix::net::UnixStream,
        SshDivertPrelude,
    )> {
        let (mut stream, _) = self.listener.accept()?;
        let cid = self.resolver.resolve_cid(&stream).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "broker: no CID vouched",
            )
        })?;
        let payload = Self::read_frame(&mut stream)?;
        let mut framed = Vec::with_capacity(4 + payload.len());
        framed.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        framed.extend_from_slice(&payload);
        let (prelude, _) = decode_ssh_divert_prelude(&framed)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        Ok((TransportPeer { cid }, stream, prelude))
    }
}

/// Core dispatch: stamp the authoritative instance from the transport CID,
/// verify the epoch prelude, then hand to the service. Pure over the
/// registry/service/audit handles — the socket loop and the tests share it.
pub fn dispatch<B: KeyBackend>(
    registry: &CidRegistry,
    service: &SigningService<B>,
    audit: &AuditLog,
    peer: TransportPeer,
    envelope: &WireEnvelope,
    timestamp: &str,
) -> DispatchOutcome {
    // Cross-process visibility: a concurrent launch may have bound this CID.
    if registry.refresh().is_err() {
        return DispatchOutcome::Rejected {
            reason: "CID registry unavailable".to_string(),
            re_attest: false,
        };
    }
    // 1. Transport CID is AUTHORITATIVE: resolve, never trust the claim.
    let entry = match registry.lookup(peer.cid) {
        Ok(Some(e)) => e,
        Ok(None) => {
            return DispatchOutcome::Rejected {
                reason: format!("unknown CID {}", peer.cid),
                re_attest: true,
            };
        }
        Err(e) => {
            return DispatchOutcome::Rejected {
                reason: format!("CID lookup failed: {e}"),
                re_attest: false,
            };
        }
    };
    let instance = entry.instance.clone();
    // 2. Epoch prelude: presented token must equal the registry-bound epoch
    //    for this CID (defeats restart-reuse and snapshot-fork clones).
    let expected = match EpochToken::from_hex(&entry.epoch_hex) {
        Ok(t) => t,
        Err(e) => {
            return DispatchOutcome::Rejected {
                reason: format!("registry epoch corrupt for CID {}: {e}", peer.cid),
                re_attest: true,
            };
        }
    };
    if let Err(e) = envelope.epoch.verify(peer.cid, &expected, peer.cid) {
        // Audit the rejected attempt under the AUTHORITATIVE instance —
        // the claim is untrusted, but the CID→instance binding is real.
        let record = AuditRecord::deny(
            timestamp,
            &instance,
            peer.cid,
            &envelope.body.key_reference,
            envelope.body.signature_scheme.to_string(),
            &envelope.body.namespace,
            e.to_string(),
            payload_digest_hex(&envelope.body.payload),
        );
        if let Err(ae) = audit.append(record) {
            eprintln!("WARNING: broker audit append failed: {ae}");
        }
        return DispatchOutcome::Rejected {
            reason: e.to_string(),
            re_attest: e.re_attest(),
        };
    }
    // 3. Hand to the service under the stamped instance (the claim is
    //    dropped here — spoof attempts resolve to the CID owner's grants).
    match service.sign(&instance, peer.cid, &envelope.body, audit, timestamp) {
        Ok(resp) => DispatchOutcome::Signed(resp),
        Err(denial) => DispatchOutcome::Denied(denial),
    }
}

/// Allowed divert target: the registry-resolved instance plus the
/// prelude's destination. The relay dials `dest_host:dest_port` upstream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DivertDestination {
    pub instance: String,
    pub cid: u32,
    pub dest_host: String,
    pub dest_port: u16,
}

/// Outcome of one divert decision: an allowed destination (hand the kept
/// stream to the [`SshRelay`]) or a denial (close the stream). Every
/// outcome audits via the `ssh-divert` record constructors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DivertDecision {
    Allow { dest: DivertDestination },
    Deny { reason: String, re_attest: bool },
}

/// Byte relay for an allowed divert: carries the guest session between the
/// accepted broker stream and a freshly dialed upstream. The production
/// in-VM relay plugs in behind this trait; the divert decision and its
/// audit stay untouched.
pub trait SshRelay: Send + Sync + std::fmt::Debug {
    fn relay(
        &self,
        stream: std::os::unix::net::UnixStream,
        dest: &DivertDestination,
    ) -> std::io::Result<()>;
}

/// Test relay: echoes accepted bytes back until EOF. Proves the hook hands
/// over a live stream; not a production upstream path.
#[derive(Debug, Default)]
pub struct EchoRelay;

impl SshRelay for EchoRelay {
    fn relay(
        &self,
        mut stream: std::os::unix::net::UnixStream,
        _dest: &DivertDestination,
    ) -> std::io::Result<()> {
        let mut buf = [0u8; 8192];
        loop {
            let n = stream.read(&mut buf)?;
            if n == 0 {
                return Ok(());
            }
            stream.write_all(&buf[..n])?;
        }
    }
}

/// Core divert decision: resolve the authoritative instance from the
/// transport CID, range-check the prelude's transport CID, verify epoch
/// freshness, then check the destination against the instance's SSH
/// grants. Pure over the registry/grant/audit handles — the socket loop
/// and the tests share it.
///
/// Denials close the connection at the call site; only the unknown-CID
/// denial asks the guest to re-attest (its binding is gone — a fresh
/// launch re-binds). Stale/future epochs and ungranted destinations do
/// not: re-attestation provisions a new epoch token, which cannot fix a
/// wall clock or a grant set.
pub fn decide_divert(
    registry: &CidRegistry,
    grants: &GrantStore,
    audit: &AuditLog,
    cid: u32,
    prelude: &SshDivertPrelude,
    now_secs: u64,
    timestamp: &str,
) -> DivertDecision {
    // Deny helper: audits under the resolved instance when known, else
    // under a synthetic `cid-<n>` label (no registry binding vouches an
    // identity there), then returns the denial. Audit-write failure is
    // stderr-loud but never masks the denial itself.
    let deny = |instance: &str, reason: String, re_attest: bool| -> DivertDecision {
        let record = AuditRecord::ssh_divert_deny(
            timestamp,
            instance,
            cid,
            &prelude.dest_host,
            prelude.dest_port,
            reason.clone(),
        );
        if let Err(e) = audit.append(record) {
            eprintln!("WARNING: broker audit append failed: {e}");
        }
        DivertDecision::Deny { reason, re_attest }
    };
    // Cross-process visibility: a concurrent launch may have bound this CID.
    if registry.refresh().is_err() {
        return deny(
            &format!("cid-{cid}"),
            "CID registry unavailable".to_string(),
            false,
        );
    }
    // Transport CID is AUTHORITATIVE: resolve, never trust a claim.
    let instance = match registry.lookup(cid) {
        Ok(Some(entry)) => entry.instance,
        Ok(None) => {
            return deny(
                &format!("cid-{cid}"),
                format!("unknown CID {cid}: no live binding; re-attestation required"),
                true,
            );
        }
        Err(e) => {
            return deny(
                &format!("cid-{cid}"),
                format!("CID lookup failed: {e}"),
                false,
            );
        }
    };
    // The prelude's transport CID must fit the 32-bit CID space (fail
    // closed — an overflowing attribution claim is never honored).
    if u32::try_from(prelude.transport_cid).is_err() {
        return deny(
            &instance,
            format!(
                "divert prelude transport CID {} exceeds the u32 CID range",
                prelude.transport_cid
            ),
            false,
        );
    }
    // Epoch freshness: a zero broker clock fails closed (freshness is
    // unverifiable without one); otherwise the prelude must sit within
    // the skew window in EITHER direction (stale = replay, future =
    // clock lie).
    if now_secs == 0 {
        return deny(
            &instance,
            "divert prelude rejected: broker clock unavailable, freshness cannot be verified"
                .to_string(),
            false,
        );
    }
    let skew = prelude.epoch.abs_diff(now_secs);
    if skew > MAX_DIVERT_EPOCH_SKEW_SECS {
        let direction = if prelude.epoch > now_secs {
            "in the future"
        } else {
            "stale"
        };
        return deny(
            &instance,
            format!(
                "divert prelude epoch {} is {direction} (skew {skew}s exceeds {MAX_DIVERT_EPOCH_SKEW_SECS}s)",
                prelude.epoch
            ),
            false,
        );
    }
    // Destination allowance: the instance's SSH grants must cover the
    // dialed host and port.
    if !grants.ssh_authorized(&instance, &prelude.dest_host, prelude.dest_port) {
        return deny(
            &instance,
            format!(
                "SSH destination {}:{} is not granted for instance '{instance}'",
                prelude.dest_host, prelude.dest_port
            ),
            false,
        );
    }
    let dest = DivertDestination {
        instance: instance.clone(),
        cid,
        dest_host: prelude.dest_host.clone(),
        dest_port: prelude.dest_port,
    };
    let record = AuditRecord::ssh_divert_allow(
        timestamp,
        &instance,
        cid,
        &prelude.dest_host,
        prelude.dest_port,
    );
    if let Err(e) = audit.append(record) {
        eprintln!("WARNING: broker audit append failed: {e}");
    }
    DivertDecision::Allow { dest }
}

/// Serve one divert connection: accept, decide, then relay on allow or
/// close on deny. Accept/protocol failures (no vouched CID, undecodable
/// prelude) carry no trustworthy fields, so they close stderr-loud
/// without an audit record — the signing path's undecodable-frame rule.
pub fn serve_divert_once<R: CidResolver>(
    transport: &UnixSocketTransport<R>,
    registry: &CidRegistry,
    grants: &GrantStore,
    audit: &AuditLog,
    relay: &dyn SshRelay,
    timestamp: &str,
    now_secs: u64,
) -> std::io::Result<DivertDecision> {
    let (peer, stream, prelude) = transport.accept_divert()?;
    let decision = decide_divert(
        registry, grants, audit, peer.cid, &prelude, now_secs, timestamp,
    );
    match &decision {
        DivertDecision::Allow { dest } => {
            if let Err(e) = relay.relay(stream, dest) {
                eprintln!(
                    "WARNING: ssh divert relay failed for instance '{}': {e}",
                    dest.instance
                );
            }
        }
        DivertDecision::Deny { .. } => {
            // Close: dropping the stream refuses the session.
            drop(stream);
        }
    }
    Ok(decision)
}

/// Serve divert connections until `stop` is set (the divert counterpart to
/// [`BrokerShim::run_until`]). Decided connections never kill the loop;
/// after any accept/protocol failure a set `stop` exits instead — the
/// shutdown dummy connection surfaces exactly such a failure to unblock
/// `accept`.
pub fn run_divert_until<R: CidResolver>(
    transport: &UnixSocketTransport<R>,
    registry: &CidRegistry,
    grants: &GrantStore,
    audit: &AuditLog,
    relay: &dyn SshRelay,
    stop: &AtomicBool,
) {
    while !stop.load(Ordering::Relaxed) {
        let timestamp = crate::microsandbox::runtime::time::current_rfc3339_utc();
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        match serve_divert_once(
            transport, registry, grants, audit, relay, &timestamp, now_secs,
        ) {
            Ok(_) => {}
            Err(e) if stop.load(Ordering::Relaxed) => {
                let _ = e;
                break;
            }
            Err(e) => {
                eprintln!("WARNING: ssh divert accept failed: {e}");
            }
        }
    }
}

/// Host-side broker shim: owns the socket path plus the dispatch handles.
/// `run_until` serves one envelope per connection until `stop` is set.
pub struct BrokerShim<B: KeyBackend> {
    pub socket_path: PathBuf,
    registry: CidRegistry,
    service: SigningService<B>,
    audit: AuditLog,
}

impl<B: KeyBackend> BrokerShim<B> {
    pub fn new(
        socket_path: PathBuf,
        registry: CidRegistry,
        service: SigningService<B>,
        audit: AuditLog,
    ) -> Self {
        Self {
            socket_path,
            registry,
            service,
            audit,
        }
    }

    /// Serve connections until `stop` is set. Each connection carries one
    /// envelope; the outcome is framed back on the same connection
    /// (CBOR `DispatchReply`). Transport errors on one connection never
    /// kill the loop — they are stderr-loud and the loop continues.
    pub fn run_until<R: CidResolver + Send + Sync>(
        &self,
        transport: &UnixSocketTransport<R>,
        stop: &AtomicBool,
    ) {
        while !stop.load(Ordering::Relaxed) {
            let (peer, envelope) = match transport.accept() {
                Ok(v) => v,
                Err(e) if stop.load(Ordering::Relaxed) => {
                    let _ = e;
                    break;
                }
                Err(e) => {
                    eprintln!("WARNING: broker accept failed: {e}");
                    continue;
                }
            };
            let timestamp = crate::microsandbox::runtime::time::current_rfc3339_utc();
            let outcome = dispatch(
                &self.registry,
                &self.service,
                &self.audit,
                peer,
                &envelope,
                &timestamp,
            );
            let _ = outcome;
            // This skeleton serves dispatch only; reply framing over the
            // accepted stream inside the transport remains follow-up work
            // (the accepted `UnixStream` is consumed by `accept`).
        }
    }
}

/// CBOR response frame helpers (kept beside the transport framing).
pub fn encode_envelope(envelope: &WireEnvelope) -> anyhow::Result<Vec<u8>> {
    encode_cbor(envelope)
}

pub fn decode_envelope(bytes: &[u8]) -> anyhow::Result<WireEnvelope> {
    decode_cbor(bytes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::config::SecretViolationPolicy;
    use crate::microsandbox::broker::audit::AuditResult;
    use crate::microsandbox::broker::epoch::EPOCH_BYTES;
    use crate::microsandbox::broker::signing::{
        GrantStore, LimitsConfig, SignatureScheme, TestBackend,
    };
    use crate::microsandbox::plan::{
        CredentialBinding, CredentialsPlan, SigningGrantPlan, SshGrantPlan,
    };
    use microsandbox_network::ssh::gateway::{SshDivertPrelude, encode_ssh_divert_prelude};

    fn test_service() -> SigningService<TestBackend> {
        let plan = CredentialsPlan {
            ssh: vec![],
            signing: vec![SigningGrantPlan {
                name: "rel".to_string(),
                material: "SIGN_KEY".to_string(),
                namespace: "git".to_string(),
                on_violation: SecretViolationPolicy::Block,
                binding: CredentialBinding::Broker,
            }],
            strict: false,
            strict_origin: None,
        };
        // Only "real-instance" holds grants — the spoof claim ("evil") holds none.
        let store = GrantStore::compile(&[("real-instance", &plan)]);
        SigningService::new(store, LimitsConfig::default(), TestBackend)
    }

    fn envelope_for(claimed: &str, epoch: &EpochToken) -> WireEnvelope {
        WireEnvelope {
            claimed_instance: claimed.to_string(),
            epoch: epoch.clone(),
            body: SignRequest {
                operation_id: "op-1".to_string(),
                key_reference: "rel".to_string(),
                signature_scheme: SignatureScheme::SshSig,
                namespace: "git".to_string(),
                payload: b"data".to_vec(),
                optional_context: None,
            },
        }
    }

    #[test]
    fn dispatch_stamps_transport_cid_over_spoofed_claim() {
        let dir = crate::config::test_support::unique_state_dir("broker-spoof");
        let registry = CidRegistry::open(&dir).unwrap();
        // CID 7 is bound to "real-instance". The guest claims "evil".
        let epoch = EpochToken::from_bytes([7u8; EPOCH_BYTES]);
        registry.bind(7, "real-instance", &epoch).unwrap();
        let service = test_service();
        let audit = AuditLog::memory();
        let envelope = envelope_for("evil", &epoch);
        let outcome = dispatch(
            &registry,
            &service,
            &audit,
            TransportPeer { cid: 7 },
            &envelope,
            "t",
        );
        // Success proves the stamped identity won: "evil" holds no grants,
        // so a claim-trusting shim would have denied with UnknownInstance.
        assert!(
            matches!(outcome, DispatchOutcome::Signed(_)),
            "transport-CID identity must win over the spoofed claim: {outcome:?}"
        );
        let records = audit.snapshot();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].instance, "real-instance");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dispatch_rejects_unknown_cid_with_re_attest() {
        let dir = crate::config::test_support::unique_state_dir("broker-unknowncid");
        let registry = CidRegistry::open(&dir).unwrap();
        let service = test_service();
        let audit = AuditLog::memory();
        let envelope = envelope_for("ghost", &EpochToken::from_bytes([1u8; EPOCH_BYTES]));
        let outcome = dispatch(
            &registry,
            &service,
            &audit,
            TransportPeer { cid: 4242 },
            &envelope,
            "t",
        );
        assert!(
            matches!(
                outcome,
                DispatchOutcome::Rejected {
                    re_attest: true,
                    ..
                }
            ),
            "unknown CID must reject with re-attestation: {outcome:?}"
        );
        assert!(outcome.re_attest());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dispatch_rejects_stale_epoch_with_re_attest_and_audits() {
        let dir = crate::config::test_support::unique_state_dir("broker-stale");
        let registry = CidRegistry::open(&dir).unwrap();
        registry
            .bind(
                7,
                "real-instance",
                &EpochToken::from_bytes([7u8; EPOCH_BYTES]),
            )
            .unwrap();
        let service = test_service();
        let audit = AuditLog::memory();
        // Fork/clone presents an old epoch for the same CID.
        let envelope = envelope_for("real-instance", &EpochToken::from_bytes([0u8; EPOCH_BYTES]));
        let outcome = dispatch(
            &registry,
            &service,
            &audit,
            TransportPeer { cid: 7 },
            &envelope,
            "t",
        );
        assert!(
            matches!(
                outcome,
                DispatchOutcome::Rejected {
                    re_attest: true,
                    ..
                }
            ),
            "stale epoch must reject with re-attestation: {outcome:?}"
        );
        // Rejected auth attempts audit under the authoritative instance.
        let records = audit.snapshot();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].instance, "real-instance");
        assert!(matches!(records[0].result, AuditResult::Deny { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dispatch_denies_ungranted_stamped_instance() {
        let dir = crate::config::test_support::unique_state_dir("broker-deny");
        let registry = CidRegistry::open(&dir).unwrap();
        let epoch = EpochToken::from_bytes([3u8; EPOCH_BYTES]);
        registry.bind(9, "nogrants-here", &epoch).unwrap();
        let service = test_service();
        let audit = AuditLog::memory();
        let envelope = envelope_for("real-instance", &epoch);
        // Claim says "real-instance" (which HAS grants) but CID 9 stamps
        // "nogrants-here" (which has none) → denial proves stamping.
        let outcome = dispatch(
            &registry,
            &service,
            &audit,
            TransportPeer { cid: 9 },
            &envelope,
            "t",
        );
        assert!(
            matches!(
                outcome,
                DispatchOutcome::Denied(Denial::UnknownInstance { .. })
            ),
            "stamped ungranted instance must deny: {outcome:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unix_transport_round_trips_envelope_with_fixed_cid() {
        let dir = crate::config::test_support::unique_state_dir("broker-socket");
        let socket_path = dir.join("broker.sock");
        let transport = UnixSocketTransport::bind(&socket_path, FixedCidResolver(7)).unwrap();
        let epoch = EpochToken::from_bytes([7u8; EPOCH_BYTES]);
        let envelope = envelope_for("real-instance", &epoch);
        let bytes = encode_envelope(&envelope).unwrap();
        // Client side: connect + write one frame.
        let mut client = std::os::unix::net::UnixStream::connect(&socket_path).unwrap();
        UnixSocketTransport::<FixedCidResolver>::write_frame(&mut client, &bytes).unwrap();
        // Server side: accept resolves the fixed CID + decodes the envelope.
        let (peer, got) = transport.accept().unwrap();
        assert_eq!(peer, TransportPeer { cid: 7 });
        assert_eq!(got, envelope);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unix_transport_rejects_oversized_frame() {
        let dir = crate::config::test_support::unique_state_dir("broker-bigframe");
        let socket_path = dir.join("broker.sock");
        let transport = UnixSocketTransport::bind(&socket_path, FixedCidResolver(7)).unwrap();
        let mut client = std::os::unix::net::UnixStream::connect(&socket_path).unwrap();
        client
            .write_all(&(MAX_FRAME_BYTES as u32 + 1).to_be_bytes())
            .unwrap();
        let err = transport.accept().unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- SSH divert decisions ----

    /// Fixed wall clock for divert tests (explicit `now_secs`, no time I/O).
    const DIVERT_NOW: u64 = 1_700_000_000;

    fn divert_store() -> GrantStore {
        let plan = CredentialsPlan {
            ssh: vec![SshGrantPlan {
                name: "deploy".to_string(),
                material: "DEPLOY_KEY".to_string(),
                hosts: vec!["github.com".to_string()],
                users: vec!["git".to_string()],
                ports: vec![22],
                binding: CredentialBinding::Broker,
            }],
            signing: vec![],
            strict: false,
            strict_origin: None,
        };
        // Only "real-instance" holds SSH grants.
        GrantStore::compile(&[("real-instance", &plan)])
    }

    fn divert_prelude(host: &str, port: u16, transport_cid: u64, epoch: u64) -> SshDivertPrelude {
        SshDivertPrelude {
            dest_host: host.to_string(),
            dest_port: port,
            transport_cid,
            epoch,
        }
    }

    fn bound_registry(dir: &std::path::Path) -> CidRegistry {
        let registry = CidRegistry::open(dir).unwrap();
        registry
            .bind(
                7,
                "real-instance",
                &EpochToken::from_bytes([7u8; EPOCH_BYTES]),
            )
            .unwrap();
        registry
    }

    #[test]
    fn decide_divert_allows_granted_destination_and_audits_allow() {
        let dir = crate::config::test_support::unique_state_dir("divert-allow");
        let registry = bound_registry(&dir);
        let audit = AuditLog::memory();
        let prelude = divert_prelude("github.com", 22, 7, DIVERT_NOW);
        let decision = decide_divert(
            &registry,
            &divert_store(),
            &audit,
            7,
            &prelude,
            DIVERT_NOW,
            "t",
        );
        assert_eq!(
            decision,
            DivertDecision::Allow {
                dest: DivertDestination {
                    instance: "real-instance".to_string(),
                    cid: 7,
                    dest_host: "github.com".to_string(),
                    dest_port: 22,
                }
            }
        );
        let records = audit.snapshot();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].scheme, "ssh-divert");
        assert_eq!(records[0].namespace, "divert");
        assert_eq!(records[0].key_id, "github.com:22");
        assert_eq!(
            records[0].payload_digest,
            payload_digest_hex("github.com:22".as_bytes())
        );
        assert!(matches!(records[0].result, AuditResult::Allow));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn decide_divert_rejects_unknown_cid_with_re_attest_and_audits() {
        let dir = crate::config::test_support::unique_state_dir("divert-unknowncid");
        let registry = CidRegistry::open(&dir).unwrap();
        let audit = AuditLog::memory();
        let prelude = divert_prelude("github.com", 22, 4242, DIVERT_NOW);
        let decision = decide_divert(
            &registry,
            &divert_store(),
            &audit,
            4242,
            &prelude,
            DIVERT_NOW,
            "t",
        );
        match &decision {
            DivertDecision::Deny { re_attest, .. } => assert!(*re_attest),
            DivertDecision::Allow { .. } => panic!("unknown CID must deny: {decision:?}"),
        }
        let records = audit.snapshot();
        assert_eq!(records.len(), 1, "unknown-CID diverts audit");
        assert!(matches!(records[0].result, AuditResult::Deny { .. }));
        assert_eq!(records[0].instance, "cid-4242");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn decide_divert_rejects_transport_cid_overflow_fail_closed() {
        let dir = crate::config::test_support::unique_state_dir("divert-overflow");
        let registry = bound_registry(&dir);
        let audit = AuditLog::memory();
        let prelude = divert_prelude("github.com", 22, u64::from(u32::MAX) + 1, DIVERT_NOW);
        let decision = decide_divert(
            &registry,
            &divert_store(),
            &audit,
            7,
            &prelude,
            DIVERT_NOW,
            "t",
        );
        match &decision {
            DivertDecision::Deny { re_attest, .. } => assert!(!re_attest),
            DivertDecision::Allow { .. } => panic!("overflowing transport CID must deny"),
        }
        assert_eq!(audit.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn decide_divert_rejects_stale_and_future_epochs() {
        let dir = crate::config::test_support::unique_state_dir("divert-skew");
        let registry = bound_registry(&dir);
        let store = divert_store();
        for epoch in [
            DIVERT_NOW - MAX_DIVERT_EPOCH_SKEW_SECS - 1,
            DIVERT_NOW + MAX_DIVERT_EPOCH_SKEW_SECS + 1,
        ] {
            let audit = AuditLog::memory();
            let prelude = divert_prelude("github.com", 22, 7, epoch);
            let decision = decide_divert(&registry, &store, &audit, 7, &prelude, DIVERT_NOW, "t");
            assert!(
                matches!(decision, DivertDecision::Deny { .. }),
                "epoch {epoch} must deny"
            );
            assert_eq!(audit.len(), 1, "skewed preludes audit");
        }
        // The window edge itself still allows.
        for epoch in [
            DIVERT_NOW - MAX_DIVERT_EPOCH_SKEW_SECS,
            DIVERT_NOW + MAX_DIVERT_EPOCH_SKEW_SECS,
        ] {
            let audit = AuditLog::memory();
            let prelude = divert_prelude("github.com", 22, 7, epoch);
            let decision = decide_divert(&registry, &store, &audit, 7, &prelude, DIVERT_NOW, "t");
            assert!(
                matches!(decision, DivertDecision::Allow { .. }),
                "epoch {epoch} sits on the window edge and must allow"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn decide_divert_rejects_zero_clock_fail_closed() {
        let dir = crate::config::test_support::unique_state_dir("divert-zeroclock");
        let registry = bound_registry(&dir);
        let audit = AuditLog::memory();
        let prelude = divert_prelude("github.com", 22, 7, 0);
        let decision = decide_divert(&registry, &divert_store(), &audit, 7, &prelude, 0, "t");
        match decision {
            DivertDecision::Deny { reason, .. } => assert!(reason.contains("clock")),
            DivertDecision::Allow { .. } => {
                panic!("a zero broker clock cannot verify freshness and must deny")
            }
        }
        assert_eq!(audit.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn decide_divert_denies_ungranted_destination() {
        let dir = crate::config::test_support::unique_state_dir("divert-ungranted");
        let registry = bound_registry(&dir);
        let audit = AuditLog::memory();
        let prelude = divert_prelude("evil.example", 22, 7, DIVERT_NOW);
        let decision = decide_divert(
            &registry,
            &divert_store(),
            &audit,
            7,
            &prelude,
            DIVERT_NOW,
            "t",
        );
        match decision {
            DivertDecision::Deny { reason, re_attest } => {
                assert!(!re_attest);
                assert!(reason.contains("evil.example"));
            }
            DivertDecision::Allow { .. } => panic!("ungranted destination must deny"),
        }
        let records = audit.snapshot();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].instance, "real-instance");
        assert!(matches!(records[0].result, AuditResult::Deny { .. }));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn serve_divert_once_closes_denied_connection() {
        let dir = crate::config::test_support::unique_state_dir("divert-close");
        let socket_path = dir.join("broker.sock");
        let transport = UnixSocketTransport::bind(&socket_path, FixedCidResolver(7)).unwrap();
        let registry = bound_registry(&dir);
        let store = divert_store();
        let audit = AuditLog::memory();
        let relay = EchoRelay;
        // Client side: connect + send an ungranted prelude before the
        // server accepts (listener backlog makes this deterministic).
        let mut client = std::os::unix::net::UnixStream::connect(&socket_path).unwrap();
        let framed = encode_ssh_divert_prelude(&divert_prelude("evil.example", 22, 7, DIVERT_NOW));
        client.write_all(&framed).unwrap();
        let decision = serve_divert_once(
            &transport, &registry, &store, &audit, &relay, "t", DIVERT_NOW,
        )
        .unwrap();
        assert!(matches!(decision, DivertDecision::Deny { .. }));
        // Deny closes: the client reads EOF.
        let mut buf = [0u8; 1];
        assert_eq!(
            client.read(&mut buf).unwrap(),
            0,
            "denied stream must close"
        );
        assert_eq!(audit.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn serve_divert_once_relays_allowed_connection_via_echo() {
        let dir = crate::config::test_support::unique_state_dir("divert-echo");
        let socket_path = dir.join("broker.sock");
        let transport = UnixSocketTransport::bind(&socket_path, FixedCidResolver(7)).unwrap();
        let registry = bound_registry(&dir);
        let store = divert_store();
        let audit = AuditLog::memory();
        let relay = EchoRelay;
        std::thread::scope(|s| {
            let server = s.spawn(|| {
                serve_divert_once(
                    &transport, &registry, &store, &audit, &relay, "t", DIVERT_NOW,
                )
            });
            let mut client = std::os::unix::net::UnixStream::connect(&socket_path).unwrap();
            let framed =
                encode_ssh_divert_prelude(&divert_prelude("github.com", 22, 7, DIVERT_NOW));
            client.write_all(&framed).unwrap();
            // Post-prelude bytes flow through the relay (echoed here).
            client.write_all(b"ping").unwrap();
            let mut echo = [0u8; 4];
            client.read_exact(&mut echo).unwrap();
            assert_eq!(&echo, b"ping");
            client.shutdown(std::net::Shutdown::Write).unwrap();
            let decision = server.join().unwrap().unwrap();
            assert!(
                matches!(decision, DivertDecision::Allow { .. }),
                "granted destination must relay: {decision:?}"
            );
        });
        assert_eq!(audit.len(), 1);
        assert!(matches!(audit.snapshot()[0].result, AuditResult::Allow));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
