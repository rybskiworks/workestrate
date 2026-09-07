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
use microsandbox_network::ssh::gateway::{
    SshDivertPrelude, decode_ssh_divert_prelude, encode_ssh_divert_prelude,
};
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

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

/// Bound on one upstream TCP dial (seconds). A diverted session must fail
/// closed fast when the granted upstream is unroutable — an unbounded
/// connect would park the relay worker and leave the guest hanging instead
/// of closing the session.
pub const TCP_UPSTREAM_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Test relay: echoes accepted bytes back until EOF. Proves the hook hands
/// over a live stream; not a production upstream path.
#[derive(Debug, Default, Clone, Copy)]
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

/// Production relay: reoriginates a fresh TCP dial to the decided
/// destination and pumps bytes both ways until EOF or error.
///
/// Framing note: the divert prelude lives ONLY on the guest→shim leg (the
/// msb proxy sends it, [`UnixSocketTransport::accept_divert`] consumes
/// it). The upstream is a real SSH server expecting a fresh SSH stream,
/// so no prelude or other framing is sent there — the guest observes the
/// upstream banner a second time through this pump (the diverted
/// session's double-banner fingerprint). The shim's own broker socket is
/// the accept side and is never dialed here: a self-dial would loop back
/// into the divert accept path instead of reaching an SSH server.
///
/// Half-close propagates both ways: guest EOF shuts the upstream write
/// half (sshd sees EOF) and upstream EOF shuts the guest write half.
#[derive(Debug, Clone)]
pub struct TcpUpstreamRelay {
    connect_timeout: Duration,
}

impl TcpUpstreamRelay {
    /// Build a relay with an explicit bound on one upstream dial.
    pub fn new(connect_timeout: Duration) -> Self {
        Self { connect_timeout }
    }

    /// Dial the decided destination, trying each resolved address in order
    /// until one connects (the [`TcpStream::connect`] behavior) with the
    /// configured timeout bounding every attempt.
    fn dial(dest: &DivertDestination, timeout: Duration) -> std::io::Result<TcpStream> {
        let addrs = (dest.dest_host.as_str(), dest.dest_port)
            .to_socket_addrs()
            .map_err(|e| {
                std::io::Error::other(format!(
                    "ssh divert upstream resolve failed for {}:{}: {e}",
                    dest.dest_host, dest.dest_port
                ))
            })?;
        let mut last_err = None;
        for addr in addrs {
            match TcpStream::connect_timeout(&addr, timeout) {
                Ok(stream) => return Ok(stream),
                Err(e) => last_err = Some(e),
            }
        }
        Err(last_err.unwrap_or_else(|| {
            std::io::Error::other(format!(
                "ssh divert upstream resolve returned no addresses for {}:{}",
                dest.dest_host, dest.dest_port
            ))
        }))
    }
}

impl Default for TcpUpstreamRelay {
    fn default() -> Self {
        Self::new(Duration::from_secs(TCP_UPSTREAM_CONNECT_TIMEOUT_SECS))
    }
}

impl SshRelay for TcpUpstreamRelay {
    fn relay(
        &self,
        stream: std::os::unix::net::UnixStream,
        dest: &DivertDestination,
    ) -> std::io::Result<()> {
        let upstream = Self::dial(dest, self.connect_timeout)?;
        let mut guest_in = stream.try_clone()?;
        let mut guest_out = stream;
        let mut upstream_in = upstream.try_clone()?;
        let mut upstream_out = upstream;
        // Upstream→guest runs on a worker; guest→upstream runs here. Each
        // side shuts the far write half when its copy ends so EOF
        // propagates instead of half-hanging the session.
        let worker = std::thread::Builder::new()
            .name(format!("ssh-relay-up-{}-{}", dest.instance, dest.cid))
            .spawn(move || {
                let res = std::io::copy(&mut upstream_in, &mut guest_out);
                let _ = guest_out.shutdown(Shutdown::Write);
                res
            })
            .map_err(|e| {
                std::io::Error::other(format!("ssh divert relay worker spawn failed: {e}"))
            })?;
        let downstream = std::io::copy(&mut guest_in, &mut upstream_out);
        let _ = upstream_out.shutdown(Shutdown::Write);
        let upstream_res = match worker.join() {
            Ok(res) => res,
            Err(_) => Err(std::io::Error::other(
                "ssh divert relay worker did not finish",
            )),
        };
        downstream?;
        upstream_res?;
        Ok(())
    }
}

/// Bound on one broker-socket dial. A diverted session must fail closed fast
/// when the broker VM is down — an unbounded connect would park the relay
/// worker and leave the guest hanging instead of closing the session.
pub const BROKER_SOCKET_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Custody relay: hands a decided session to the broker VM over its host
/// divert socket.
///
/// Dials the broker VM socket, sends the framed divert prelude, then pumps
/// bytes both ways (guest→shim→broker). The broker terminates the session
/// and reoriginates upstream under sealed custody; the shim never dials
/// upstream TCP itself on this path. A missing or unreachable broker socket
/// (no broker VM running) fails the relay, and the caller drops the guest
/// stream — fail-closed, never a direct-TCP fallback for broker-bound
/// sessions.
#[derive(Debug, Clone)]
pub struct BrokerSocketRelay {
    broker_socket: PathBuf,
    connect_timeout: Duration,
}

impl BrokerSocketRelay {
    /// Build a relay dialing `broker_socket` for every decided session.
    pub fn new(broker_socket: PathBuf) -> Self {
        Self {
            broker_socket,
            connect_timeout: Duration::from_secs(BROKER_SOCKET_CONNECT_TIMEOUT_SECS),
        }
    }

    /// Build a relay with an explicit bound on one broker-socket dial.
    pub fn with_timeout(broker_socket: PathBuf, connect_timeout: Duration) -> Self {
        Self {
            broker_socket,
            connect_timeout,
        }
    }

    /// Dial the broker socket with a bounded wait. Unix connects carry no
    /// timeout knob, so the dial runs on a worker joined with the bound —
    /// an unresponsive listener fails closed instead of parking the relay.
    fn dial(&self) -> std::io::Result<std::os::unix::net::UnixStream> {
        let path = self.broker_socket.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("ssh-broker-dial".to_string())
            .spawn(move || {
                let _ = tx.send(std::os::unix::net::UnixStream::connect(&path));
            })
            .map_err(|e| std::io::Error::other(format!("broker dial worker spawn failed: {e}")))?;
        match rx.recv_timeout(self.connect_timeout) {
            Ok(Ok(stream)) => Ok(stream),
            Ok(Err(e)) => Err(std::io::Error::other(format!(
                "ssh broker dial failed for {}: {e}",
                self.broker_socket.display()
            ))),
            Err(_) => Err(std::io::Error::other(format!(
                "ssh broker dial timed out for {}",
                self.broker_socket.display()
            ))),
        }
    }
}

impl SshRelay for BrokerSocketRelay {
    fn relay(
        &self,
        stream: std::os::unix::net::UnixStream,
        dest: &DivertDestination,
    ) -> std::io::Result<()> {
        let mut broker = self.dial()?;
        // Re-stamp the prelude for the broker leg: the destination and
        // transport attribution carry over, the epoch is now (the broker
        // checks freshness against its provisioned floor).
        let prelude = SshDivertPrelude {
            dest_host: dest.dest_host.clone(),
            dest_port: dest.dest_port,
            transport_cid: u64::from(dest.cid),
            epoch: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        broker.write_all(&encode_ssh_divert_prelude(&prelude))?;
        broker.flush()?;
        pump_unix_streams(
            stream,
            broker,
            &format!("ssh-broker-{}-{}", dest.instance, dest.cid),
        )
    }
}

/// Pump bytes both ways between two unix streams until EOF or error,
/// propagating half-close both ways.
fn pump_unix_streams(
    guest: std::os::unix::net::UnixStream,
    broker: std::os::unix::net::UnixStream,
    label: &str,
) -> std::io::Result<()> {
    let mut guest_in = guest.try_clone()?;
    let mut guest_out = guest;
    let mut broker_in = broker.try_clone()?;
    let mut broker_out = broker;
    let worker = std::thread::Builder::new()
        .name(format!("{label}-up"))
        .spawn(move || {
            let res = std::io::copy(&mut broker_in, &mut guest_out);
            let _ = guest_out.shutdown(Shutdown::Write);
            res
        })
        .map_err(|e| std::io::Error::other(format!("broker pump worker spawn failed: {e}")))?;
    let downstream = std::io::copy(&mut guest_in, &mut broker_out);
    let _ = broker_out.shutdown(Shutdown::Write);
    let upstream_res = match worker.join() {
        Ok(res) => res,
        Err(_) => Err(std::io::Error::other("broker pump worker did not finish")),
    };
    downstream?;
    upstream_res?;
    Ok(())
}

/// Binding-aware divert relay: broker-bound sessions ride broker custody,
/// guest-bound sessions relay direct.
///
/// Broker-bound key material lives in the broker VM, so those sessions must
/// reach [`BrokerSocketRelay`] — a missing broker fails the relay closed
/// with no direct-TCP fallback. Guest-bound material lives in the guest
/// itself, so those sessions relay direct through [`TcpUpstreamRelay`]
/// exactly as before. The binding comes from the compiled
/// [`GrantStore`](super::signing::GrantStore): any broker-bound covering
/// grant selects custody.
#[derive(Debug, Clone)]
pub struct BrokerFirstRelay {
    broker: BrokerSocketRelay,
    grants: GrantStore,
    direct: TcpUpstreamRelay,
}

impl BrokerFirstRelay {
    /// Build the dispatcher: `broker_socket` names the broker VM divert
    /// socket, `grants` is the compiled store the divert decision used.
    pub fn new(broker_socket: PathBuf, grants: GrantStore) -> Self {
        Self {
            broker: BrokerSocketRelay::new(broker_socket),
            grants,
            direct: TcpUpstreamRelay::default(),
        }
    }
}

impl SshRelay for BrokerFirstRelay {
    fn relay(
        &self,
        stream: std::os::unix::net::UnixStream,
        dest: &DivertDestination,
    ) -> std::io::Result<()> {
        if self
            .grants
            .ssh_is_broker_bound(&dest.instance, &dest.dest_host, dest.dest_port)
        {
            self.broker.relay(stream, dest)
        } else {
            self.direct.relay(stream, dest)
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

/// Accept one divert connection and decide it, returning the kept stream
/// with its decision. Accept/protocol failures (no vouched CID,
/// undecodable prelude) carry no trustworthy fields, so they fail without
/// an audit record — the signing path's undecodable-frame rule. Shared by
/// the single-shot server and the loop below.
fn accept_and_decide<R: CidResolver>(
    transport: &UnixSocketTransport<R>,
    registry: &CidRegistry,
    grants: &GrantStore,
    audit: &AuditLog,
    timestamp: &str,
    now_secs: u64,
) -> std::io::Result<(std::os::unix::net::UnixStream, DivertDecision)> {
    let (peer, stream, prelude) = transport.accept_divert()?;
    let decision = decide_divert(
        registry, grants, audit, peer.cid, &prelude, now_secs, timestamp,
    );
    Ok((stream, decision))
}

/// Serve one divert connection: accept, decide, then relay on allow or
/// close on deny. Accept/protocol failures (no vouched CID, undecodable
/// prelude) carry no trustworthy fields, so they close stderr-loud
/// without an audit record — the signing path's undecodable-frame rule.
/// The relay runs INLINE here (the single-shot and test path); the loop
/// in [`run_divert_until`] relays on worker threads instead.
pub fn serve_divert_once<R: CidResolver>(
    transport: &UnixSocketTransport<R>,
    registry: &CidRegistry,
    grants: &GrantStore,
    audit: &AuditLog,
    relay: &dyn SshRelay,
    timestamp: &str,
    now_secs: u64,
) -> std::io::Result<DivertDecision> {
    let (stream, decision) =
        accept_and_decide(transport, registry, grants, audit, timestamp, now_secs)?;
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
///
/// Allowed sessions relay on detached worker threads: the relay blocks for
/// the whole session (a full SSH login), so serving it inline would stall
/// every later divert behind it. Each worker owns its stream, decision,
/// and relay clone — nothing borrowed — so the loop keeps accepting while
/// sessions flow. Shutdown still only joins this loop thread: in-flight
/// sessions drain on their own EOF after the socket is removed instead of
/// hanging teardown.
pub fn run_divert_until<R: CidResolver, L: SshRelay + Clone + 'static>(
    transport: &UnixSocketTransport<R>,
    registry: &CidRegistry,
    grants: &GrantStore,
    audit: &AuditLog,
    relay: L,
    stop: &AtomicBool,
) {
    while !stop.load(Ordering::Relaxed) {
        let timestamp = crate::microsandbox::runtime::time::current_rfc3339_utc();
        let now_secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let (stream, decision) =
            match accept_and_decide(transport, registry, grants, audit, &timestamp, now_secs) {
                Ok(v) => v,
                Err(e) if stop.load(Ordering::Relaxed) => {
                    let _ = e;
                    break;
                }
                Err(e) => {
                    eprintln!("WARNING: ssh divert accept failed: {e}");
                    continue;
                }
            };
        match decision {
            DivertDecision::Allow { dest } => {
                let worker_relay = relay.clone();
                let label = format!("ssh-relay-{}-{}", dest.instance, dest.cid);
                match std::thread::Builder::new()
                    .name(label.clone())
                    .spawn(move || {
                        if let Err(e) = worker_relay.relay(stream, &dest) {
                            eprintln!(
                                "WARNING: ssh divert relay failed for instance '{}': {e}",
                                dest.instance
                            );
                        }
                    }) {
                    Ok(_) => {}
                    Err(e) => {
                        // Spawn failure fails closed: the unspawned closure
                        // drops the stream unrelayed (the allow audit already
                        // records the decision).
                        eprintln!("WARNING: {label} spawn failed: {e} (session closed)");
                    }
                }
            }
            DivertDecision::Deny { .. } => {
                // Close: dropping the stream refuses the session.
                drop(stream);
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

    // ---- TCP upstream relay ----

    fn tcp_dest(port: u16) -> DivertDestination {
        DivertDestination {
            instance: "real-instance".to_string(),
            cid: 7,
            dest_host: "127.0.0.1".to_string(),
            dest_port: port,
        }
    }

    /// A local TCP stub stands in for the upstream SSH server: it checks
    /// the guest-bound bytes, answers, then expects the guest half-close
    /// as EOF before closing (which the guest must observe as its EOF).
    #[test]
    fn tcp_relay_pumps_both_directions_and_propagates_eof() {
        let stub = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = stub.local_addr().unwrap().port();
        let stub_worker = std::thread::spawn(move || {
            let (mut conn, _) = stub.accept().unwrap();
            conn.set_read_timeout(Some(std::time::Duration::from_secs(15)))
                .unwrap();
            // Guest→upstream direction arrives first.
            let mut hello = [0u8; 5];
            conn.read_exact(&mut hello).unwrap();
            assert_eq!(&hello, b"hello");
            // Upstream→guest direction flows back.
            conn.write_all(b"world").unwrap();
            // The guest half-close arrives as EOF; anything else is a leak.
            let mut rest = Vec::new();
            conn.read_to_end(&mut rest).unwrap();
            assert!(rest.is_empty(), "guest must send nothing after its reply");
        });
        let (mut guest, shuttle) = std::os::unix::net::UnixStream::pair().unwrap();
        guest
            .set_read_timeout(Some(std::time::Duration::from_secs(15)))
            .unwrap();
        let relay = TcpUpstreamRelay::default();
        let dest = tcp_dest(port);
        std::thread::scope(|s| {
            let server = s.spawn(|| relay.relay(shuttle, &dest));
            guest.write_all(b"hello").unwrap();
            let mut reply = [0u8; 5];
            guest.read_exact(&mut reply).unwrap();
            assert_eq!(&reply, b"world");
            guest.shutdown(std::net::Shutdown::Write).unwrap();
            let mut tail = Vec::new();
            guest.read_to_end(&mut tail).unwrap();
            assert!(tail.is_empty(), "upstream close must surface as guest EOF");
            server.join().unwrap().unwrap();
        });
        stub_worker.join().unwrap();
    }

    #[test]
    fn tcp_relay_refused_upstream_fails_closed_with_guest_eof() {
        // Reserve then release a loopback port so the dial refuses.
        let closed = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = closed.local_addr().unwrap().port();
        drop(closed);
        let (mut guest, shuttle) = std::os::unix::net::UnixStream::pair().unwrap();
        let relay = TcpUpstreamRelay::new(std::time::Duration::from_millis(500));
        let err = relay.relay(shuttle, &tcp_dest(port)).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::ConnectionRefused);
        // Fail-closed: the guest side observes EOF, never a hang.
        let mut buf = [0u8; 1];
        assert_eq!(guest.read(&mut buf).unwrap(), 0);
    }

    #[test]
    fn tcp_relay_resolves_hostnames_before_dialing() {
        let stub = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = stub.local_addr().unwrap().port();
        let stub_worker = std::thread::spawn(move || {
            let (mut conn, _) = stub.accept().unwrap();
            conn.set_read_timeout(Some(std::time::Duration::from_secs(15)))
                .unwrap();
            let mut ping = [0u8; 4];
            conn.read_exact(&mut ping).unwrap();
            assert_eq!(&ping, b"ping");
        });
        let (mut guest, shuttle) = std::os::unix::net::UnixStream::pair().unwrap();
        // `localhost` exercises hostname resolution (and the multi-address
        // fallback when it resolves to ::1 first) before the dial.
        let dest = DivertDestination {
            instance: "real-instance".to_string(),
            cid: 7,
            dest_host: "localhost".to_string(),
            dest_port: port,
        };
        let relay = TcpUpstreamRelay::new(std::time::Duration::from_secs(5));
        let worker = std::thread::spawn(move || relay.relay(shuttle, &dest));
        guest.write_all(b"ping").unwrap();
        // Half-close so the guest→upstream copy reaches EOF: the stub
        // closes after its read, ending the other direction.
        guest.shutdown(std::net::Shutdown::Write).unwrap();
        worker.join().unwrap().unwrap();
        stub_worker.join().unwrap();
    }

    // ---- Broker-socket relay ----

    /// A broker stub stands in for the broker VM: it reads the framed
    /// divert prelude, checks the attribution, then echoes post-prelude
    /// bytes until the guest half-close.
    fn broker_stub_path(dir: &std::path::Path) -> PathBuf {
        dir.join("broker-stub.sock")
    }

    fn run_broker_stub(socket_path: PathBuf, expect_host: String, expect_port: u16) {
        use microsandbox_network::ssh::gateway::decode_ssh_divert_prelude;
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let listener = std::os::unix::net::UnixListener::bind(&socket_path).unwrap();
        let (mut conn, _) = listener.accept().unwrap();
        conn.set_read_timeout(Some(std::time::Duration::from_secs(15)))
            .unwrap();
        let mut len_buf = [0u8; 4];
        conn.read_exact(&mut len_buf).unwrap();
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut payload = vec![0u8; len];
        conn.read_exact(&mut payload).unwrap();
        let mut framed = Vec::with_capacity(4 + payload.len());
        framed.extend_from_slice(&len_buf);
        framed.extend_from_slice(&payload);
        let (prelude, consumed) = decode_ssh_divert_prelude(&framed).unwrap();
        assert_eq!(consumed, framed.len(), "prelude must be exactly one frame");
        assert_eq!(prelude.dest_host, expect_host);
        assert_eq!(prelude.dest_port, expect_port);
        assert_eq!(prelude.transport_cid, 7);
        assert!(prelude.epoch > 0, "prelude must carry a wall-clock epoch");
        let mut ping = [0u8; 4];
        conn.read_exact(&mut ping).unwrap();
        assert_eq!(&ping, b"ping");
        conn.write_all(b"pong").unwrap();
        let mut rest = Vec::new();
        conn.read_to_end(&mut rest).unwrap();
        assert!(rest.is_empty(), "guest must send nothing after its reply");
    }

    #[test]
    fn broker_socket_relay_forwards_prelude_and_pumps() {
        let dir = crate::config::test_support::unique_state_dir("broker-relay");
        let socket_path = broker_stub_path(&dir);
        let stub_worker = std::thread::spawn({
            let socket_path = socket_path.clone();
            move || run_broker_stub(socket_path, "broker.example".to_string(), 22)
        });
        let (mut guest, shuttle) = std::os::unix::net::UnixStream::pair().unwrap();
        guest
            .set_read_timeout(Some(std::time::Duration::from_secs(15)))
            .unwrap();
        let relay = BrokerSocketRelay::new(socket_path);
        let dest = DivertDestination {
            instance: "real-instance".to_string(),
            cid: 7,
            dest_host: "broker.example".to_string(),
            dest_port: 22,
        };
        std::thread::scope(|s| {
            let server = s.spawn(|| relay.relay(shuttle, &dest));
            guest.write_all(b"ping").unwrap();
            let mut reply = [0u8; 4];
            guest.read_exact(&mut reply).unwrap();
            assert_eq!(&reply, b"pong");
            guest.shutdown(std::net::Shutdown::Write).unwrap();
            let mut tail = Vec::new();
            guest.read_to_end(&mut tail).unwrap();
            assert!(tail.is_empty(), "broker close must surface as guest EOF");
            server.join().unwrap().unwrap();
        });
        stub_worker.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn broker_socket_relay_missing_socket_fails_closed_with_guest_eof() {
        let dir = crate::config::test_support::unique_state_dir("broker-relay-missing");
        let missing = dir.join("no-broker.sock");
        let (mut guest, shuttle) = std::os::unix::net::UnixStream::pair().unwrap();
        let relay = BrokerSocketRelay::with_timeout(missing, std::time::Duration::from_millis(500));
        let dest = DivertDestination {
            instance: "real-instance".to_string(),
            cid: 7,
            dest_host: "broker.example".to_string(),
            dest_port: 22,
        };
        relay.relay(shuttle, &dest).unwrap_err();
        // Fail-closed: the guest side observes EOF, never a hang and never
        // a direct-TCP dial.
        let mut buf = [0u8; 1];
        assert_eq!(guest.read(&mut buf).unwrap(), 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn mixed_binding_store(tcp_port: u16) -> GrantStore {
        let plan = CredentialsPlan {
            ssh: vec![
                SshGrantPlan {
                    name: "brokered".to_string(),
                    material: "BROKER_KEY".to_string(),
                    hosts: vec!["broker.example".to_string()],
                    users: vec!["git".to_string()],
                    ports: vec![22],
                    binding: CredentialBinding::Broker,
                },
                SshGrantPlan {
                    name: "local".to_string(),
                    material: "LOCAL_KEY".to_string(),
                    hosts: vec!["127.0.0.1".to_string()],
                    users: vec!["git".to_string()],
                    ports: vec![tcp_port],
                    binding: CredentialBinding::Guest,
                },
            ],
            signing: Vec::new(),
            strict: false,
            strict_origin: None,
        };
        GrantStore::compile(&[("real-instance", &plan)])
    }

    #[test]
    fn broker_first_relay_routes_broker_bound_to_broker_socket() {
        let dir = crate::config::test_support::unique_state_dir("broker-first-custody");
        let socket_path = broker_stub_path(&dir);
        let stub_worker = std::thread::spawn({
            let socket_path = socket_path.clone();
            move || run_broker_stub(socket_path, "broker.example".to_string(), 22)
        });
        // The guest-bound port is unused here; any free port keeps the
        // store shape realistic.
        let relay = BrokerFirstRelay::new(socket_path, mixed_binding_store(1));
        let (mut guest, shuttle) = std::os::unix::net::UnixStream::pair().unwrap();
        guest
            .set_read_timeout(Some(std::time::Duration::from_secs(15)))
            .unwrap();
        let dest = DivertDestination {
            instance: "real-instance".to_string(),
            cid: 7,
            dest_host: "broker.example".to_string(),
            dest_port: 22,
        };
        std::thread::scope(|s| {
            let server = s.spawn(|| relay.relay(shuttle, &dest));
            guest.write_all(b"ping").unwrap();
            let mut reply = [0u8; 4];
            guest.read_exact(&mut reply).unwrap();
            assert_eq!(&reply, b"pong", "broker-bound must ride the broker socket");
            guest.shutdown(std::net::Shutdown::Write).unwrap();
            server.join().unwrap().unwrap();
        });
        stub_worker.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn broker_first_relay_routes_guest_bound_direct() {
        let stub = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = stub.local_addr().unwrap().port();
        let stub_worker = std::thread::spawn(move || {
            let (mut conn, _) = stub.accept().unwrap();
            conn.set_read_timeout(Some(std::time::Duration::from_secs(15)))
                .unwrap();
            let mut hello = [0u8; 5];
            conn.read_exact(&mut hello).unwrap();
            assert_eq!(&hello, b"hello");
            conn.write_all(b"world").unwrap();
        });
        let dir = crate::config::test_support::unique_state_dir("broker-first-direct");
        // No broker stub binds here: a guest-bound session must reach its
        // TCP upstream without touching the (absent) broker socket.
        let relay = BrokerFirstRelay::new(dir.join("no-broker.sock"), mixed_binding_store(port));
        let (mut guest, shuttle) = std::os::unix::net::UnixStream::pair().unwrap();
        guest
            .set_read_timeout(Some(std::time::Duration::from_secs(15)))
            .unwrap();
        let dest = tcp_dest(port);
        std::thread::scope(|s| {
            let server = s.spawn(|| relay.relay(shuttle, &dest));
            guest.write_all(b"hello").unwrap();
            let mut reply = [0u8; 5];
            guest.read_exact(&mut reply).unwrap();
            assert_eq!(&reply, b"world", "guest-bound must relay direct");
            guest.shutdown(std::net::Shutdown::Write).unwrap();
            server.join().unwrap().unwrap();
        });
        stub_worker.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
