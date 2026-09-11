//! Broker shim: host-side unix-socket listener + transport trait.
//!
//! Topology: guests dial CID 2:<port> (HOST_CID); the msb bridge maps
//! that to a host unix socket. The shim listens on that socket and hands
//! decoded requests to the service. The [`BrokerTransport`] trait separates
//! the shim from the in-VM path.
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
use crate::microsandbox::plan::{CredentialBinding, SshGrantPlan};
use microsandbox_network::ssh::gateway::{
    SshDivertPrelude, decode_ssh_divert_prelude, encode_ssh_divert_prelude,
};
use microsandbox_protocol::broker as managed_wire;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Default broker port (guest dials `CID 2:<port>`; msb bridges to the host
/// unix socket).
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

/// Byte transport separating the shim from the in-VM path.
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
        Self::bind_fresh(socket_path, resolver)
    }

    /// Bind a newly reserved launch endpoint without unlinking an existing
    /// file or listener. Collision is an error, never permission to take over.
    pub fn bind_fresh(socket_path: &Path, resolver: R) -> std::io::Result<Self> {
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let listener = std::os::unix::net::UnixListener::bind(socket_path)?;
        Ok(Self { listener, resolver })
    }

    /// Allow an owned loop to observe cancellation without dialing its pathname.
    pub(super) fn set_nonblocking(&self) -> std::io::Result<()> {
        self.listener.set_nonblocking(true)
    }

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
        self.accept_envelope(None)
    }
}

impl<R: CidResolver> UnixSocketTransport<R> {
    fn accept_envelope(
        &self,
        stop: Option<&AtomicBool>,
    ) -> std::io::Result<(TransportPeer, WireEnvelope)> {
        let (mut stream, _) = self.listener.accept()?;
        let cid = self.resolver.resolve_cid(&stream).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "broker: no CID vouched",
            )
        })?;
        let frame = super::frame_io::read_frame(&mut stream, MAX_FRAME_BYTES, stop)?;
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
        self.accept_divert_with_stop(None)
    }

    fn accept_divert_with_stop(
        &self,
        stop: Option<&AtomicBool>,
    ) -> std::io::Result<(
        TransportPeer,
        std::os::unix::net::UnixStream,
        SshDivertPrelude,
    )> {
        let (mut stream, _) = self.listener.accept()?;
        // The listener may be nonblocking for cancellable acceptance. Frame
        // deadlines and relays use blocking streams on every supported host.
        stream.set_nonblocking(false)?;
        let cid = self.resolver.resolve_cid(&stream).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "broker: no CID vouched",
            )
        })?;
        let payload = super::frame_io::read_frame(&mut stream, MAX_FRAME_BYTES, stop)?;
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

/// Endpoint decision handed from the shim to its relay.
///
/// The credential records come from the registry-resolved instance's compiled
/// plan, never from the guest's prelude. They retain all available information
/// for later session authorization; an allowed endpoint is not permission to
/// authenticate with any of its keys or usernames. This in-process context is
/// not a wire message and contains material references, not secret bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DivertDestination {
    pub instance: String,
    pub cid: u32,
    pub dest_host: String,
    pub dest_port: u16,
    pub credentials: Vec<SshGrantPlan>,
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

    /// Owned-loop entry. Legacy implementations may not observe cancellation;
    /// their worker remains reserved until it actually returns and is joined.
    fn relay_until(
        &self,
        stream: std::os::unix::net::UnixStream,
        dest: &DivertDestination,
        stop: &AtomicBool,
    ) -> std::io::Result<()> {
        if stop.load(Ordering::Acquire) {
            return Err(relay_cancelled());
        }
        self.relay(stream, dest)
    }
}

fn relay_cancelled() -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Interrupted, "SSH relay cancelled")
}

async fn relay_cancellation(stop: &AtomicBool) {
    while !stop.load(Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
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

/// Managed connect/header deadline and legacy dial-result wait. A legacy OS
/// connect is still joined and may outlive this result deadline; its reserved
/// worker cannot be reported retired until that join actually completes.
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
    managed: Option<ManagedBrokerRelay>,
}

/// A live owner query, not a cached ready receipt. The callback must check its
/// retained endpoint, source launch and native generation, then return the
/// controller's current Applied observation for this exact destination. It may
/// not derive authority from the guest prelude or a pathname alone. Keep the
/// synchronous query bounded; this adapter runs on a dedicated relay thread.
pub(crate) type ManagedAdmission =
    dyn Fn(&DivertDestination) -> std::io::Result<managed_wire::Observation> + Send + Sync;

#[derive(Clone)]
struct ManagedBrokerRelay {
    launch: managed_wire::LaunchRef,
    cid: u32,
    runtime: tokio::runtime::Handle,
    admission: Arc<ManagedAdmission>,
}

impl std::fmt::Debug for ManagedBrokerRelay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ManagedBrokerRelay")
            .field("launch", &self.launch)
            .field("cid", &self.cid)
            .finish_non_exhaustive()
    }
}

fn managed_refusal() -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::PermissionDenied,
        "managed SSH relay admission is not current",
    )
}

impl ManagedBrokerRelay {
    fn header(&self, dest: &DivertDestination) -> std::io::Result<managed_wire::ManagedDivert> {
        if dest.instance != self.launch.instance.instance
            || dest.cid != self.cid
            || !dest
                .credentials
                .iter()
                .any(|c| c.binding == CredentialBinding::Broker)
        {
            return Err(managed_refusal());
        }
        // Deliberately discard arbitrary callback diagnostics: the trusted
        // resolver may have encountered credentials or private source paths.
        let observation = (self.admission)(dest).map_err(|_| managed_refusal())?;
        if observation.outcome != managed_wire::Outcome::Applied
            || observation.failure.is_some()
            || observation.transaction.launch != self.launch
        {
            return Err(managed_refusal());
        }
        let header = managed_wire::ManagedDivert {
            version: managed_wire::VERSION,
            transaction: observation.transaction,
            // Preserve the original hostname; resolving it is not authority to
            // substitute an IP or a different host in the policy selection.
            destination_host: dest.dest_host.clone(),
            destination_port: dest.dest_port,
        };
        header.validate().map_err(|_| managed_refusal())?;
        Ok(header)
    }

    fn encode(&self, header: &managed_wire::ManagedDivert) -> std::io::Result<Vec<u8>> {
        // Called only on a dedicated synchronous relay thread. Reuse the
        // owner's handle instead of constructing a runtime per connection.
        let mut bytes = Vec::new();
        self.runtime
            .block_on(managed_wire::write_divert(&mut bytes, header))
            .map_err(|_| managed_refusal())?;
        Ok(bytes)
    }
}

impl BrokerSocketRelay {
    /// Build the legacy prelude relay. This does not select managed service mode.
    pub fn new(broker_socket: PathBuf) -> Self {
        Self {
            broker_socket,
            connect_timeout: Duration::from_secs(BROKER_SOCKET_CONNECT_TIMEOUT_SECS),
            managed: None,
        }
    }

    /// Build a relay with an explicit bound on one broker-socket dial.
    pub fn with_timeout(broker_socket: PathBuf, connect_timeout: Duration) -> Self {
        Self {
            broker_socket,
            connect_timeout,
            managed: None,
        }
    }

    /// Explicit managed-service adapter for a trusted, retained source launch.
    /// The owner supplies a live Applied query; saving an observation in the
    /// callback would lose revocation, rotation and endpoint-loss fencing.
    /// No listener/provisioning is performed and legacy constructors stay legacy.
    pub(crate) fn managed(
        broker_socket: PathBuf,
        launch: managed_wire::LaunchRef,
        cid: u32,
        runtime: tokio::runtime::Handle,
        admission: Arc<ManagedAdmission>,
    ) -> std::io::Result<Self> {
        if cid < super::registry::MIN_GUEST_CID
            || cid == u32::MAX
            || !broker_socket.is_absolute()
            || broker_socket
                .components()
                .any(|p| matches!(p, std::path::Component::ParentDir))
        {
            return Err(managed_refusal());
        }
        Ok(Self {
            broker_socket,
            connect_timeout: Duration::from_secs(BROKER_SOCKET_CONNECT_TIMEOUT_SECS),
            managed: Some(ManagedBrokerRelay {
                launch,
                cid,
                runtime,
                admission,
            }),
        })
    }

    fn relay_with_dial(
        &self,
        stream: std::os::unix::net::UnixStream,
        dest: &DivertDestination,
        dial: impl FnOnce() -> std::io::Result<std::os::unix::net::UnixStream>,
    ) -> std::io::Result<()> {
        if let Some(managed) = &self.managed {
            return self.managed_with_connect(
                stream,
                dest,
                &AtomicBool::new(false),
                || async {
                    let broker = dial()?;
                    broker.set_nonblocking(true)?;
                    tokio::net::UnixStream::from_std(broker)
                },
                managed,
            );
        }
        let mut broker = dial()?;
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

    /// A single future owns both sockets through connect, header and copy.
    /// Handle::block_on runs on the existing dedicated relay thread; there is
    /// no dial task, nested pump thread, or detached completion to infer.
    fn managed_relay_until(
        &self,
        stream: std::os::unix::net::UnixStream,
        dest: &DivertDestination,
        stop: &AtomicBool,
    ) -> std::io::Result<()> {
        let managed = self.managed.as_ref().ok_or_else(managed_refusal)?;
        self.managed_with_connect(
            stream,
            dest,
            stop,
            || tokio::net::UnixStream::connect(&self.broker_socket),
            managed,
        )
    }

    fn managed_with_connect<F, C>(
        &self,
        stream: std::os::unix::net::UnixStream,
        dest: &DivertDestination,
        stop: &AtomicBool,
        connect: C,
        managed: &ManagedBrokerRelay,
    ) -> std::io::Result<()>
    where
        C: FnOnce() -> F,
        F: std::future::Future<Output = std::io::Result<tokio::net::UnixStream>>,
    {
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(managed_refusal());
        }
        if stop.load(Ordering::Acquire) {
            return Err(relay_cancelled());
        }
        let header = managed.header(dest)?;
        // Validate with the shared codec before any connection side effect.
        let encoded = managed.encode(&header)?;
        stream.set_nonblocking(true)?;
        managed.runtime.block_on(async {
            let mut guest = tokio::net::UnixStream::from_std(stream)?;
            tokio::select! {
                biased;
                _ = relay_cancellation(stop) => Err(relay_cancelled()),
                result = async {
                    // One absolute setup deadline covers connect AND header.
                    let mut broker = tokio::time::timeout(self.connect_timeout, async {
                        let mut broker = connect().await?;
                        let current = managed.header(dest)?;
                        if stop.load(Ordering::Acquire) {
                            return Err(relay_cancelled());
                        }
                        if current.transaction != header.transaction {
                            return Err(managed_refusal());
                        }
                        tokio::io::AsyncWriteExt::write_all(&mut broker, &encoded).await?;
                        tokio::io::AsyncWriteExt::flush(&mut broker).await?;
                        Ok::<_, std::io::Error>(broker)
                    }).await.map_err(|_| std::io::Error::new(
                        std::io::ErrorKind::TimedOut, "managed SSH setup timed out",
                    ))??;
                    tokio::io::copy_bidirectional(&mut guest, &mut broker).await?;
                    Ok(())
                } => result,
            }
        })
    }

    /// Dial the broker socket with a bounded wait. Unix connects carry no
    /// timeout knob, so the caller's wait for a worker result is bounded.
    /// The legacy worker is joined even after the result deadline. This may
    /// remain pending in an OS connect, but cannot falsely certify retirement
    /// of a detached dial. Managed mode uses a cancellable owned future instead.
    fn dial(&self) -> std::io::Result<std::os::unix::net::UnixStream> {
        let path = self.broker_socket.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let worker = std::thread::Builder::new()
            .name("ssh-broker-dial".to_string())
            .spawn(move || {
                let _ = tx.send(std::os::unix::net::UnixStream::connect(&path));
            })
            .map_err(|e| std::io::Error::other(format!("broker dial worker spawn failed: {e}")))?;
        let result = match rx.recv_timeout(self.connect_timeout) {
            Ok(Ok(stream)) => Ok(stream),
            Ok(Err(e)) => Err(std::io::Error::other(format!(
                "ssh broker dial failed for {}: {e}",
                self.broker_socket.display()
            ))),
            Err(_) => Err(std::io::Error::other(format!(
                "ssh broker dial timed out for {}",
                self.broker_socket.display()
            ))),
        };
        let joined = worker.join();
        // Preserve the primary dial/timeout failure, but always visit the join.
        let stream = result?;
        joined.map_err(|_| std::io::Error::other("broker dial worker panicked"))?;
        Ok(stream)
    }
}

impl SshRelay for BrokerSocketRelay {
    fn relay(
        &self,
        stream: std::os::unix::net::UnixStream,
        dest: &DivertDestination,
    ) -> std::io::Result<()> {
        self.relay_until(stream, dest, &AtomicBool::new(false))
    }

    fn relay_until(
        &self,
        stream: std::os::unix::net::UnixStream,
        dest: &DivertDestination,
        stop: &AtomicBool,
    ) -> std::io::Result<()> {
        if self.managed.is_some() {
            self.managed_relay_until(stream, dest, stop)
        } else if stop.load(Ordering::Acquire) {
            Err(relay_cancelled())
        } else {
            self.relay_with_dial(stream, dest, || self.dial())
        }
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
/// exactly as before. The binding comes from the complete credential records
/// retained by the endpoint decision: any broker-bound covering record selects
/// custody. This does not select an upstream username or key.
#[derive(Debug, Clone)]
pub struct BrokerFirstRelay {
    broker: BrokerSocketRelay,
    direct: TcpUpstreamRelay,
}

impl BrokerFirstRelay {
    /// Build the dispatcher: `broker_socket` names the broker VM divert
    /// socket. The endpoint decision supplies the credential context.
    pub fn new(broker_socket: PathBuf) -> Self {
        Self {
            broker: BrokerSocketRelay::new(broker_socket),
            direct: TcpUpstreamRelay::default(),
        }
    }

    /// Opt into managed broker framing while preserving explicit guest-bound
    /// direct forwarding. The existing constructor keeps the legacy contract.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "workload launch does not yet activate managed diversion from a live Applied custody observation"
        )
    )]
    pub(crate) fn managed(
        broker_socket: PathBuf,
        launch: managed_wire::LaunchRef,
        cid: u32,
        runtime: tokio::runtime::Handle,
        admission: Arc<ManagedAdmission>,
    ) -> std::io::Result<Self> {
        Ok(Self {
            broker: BrokerSocketRelay::managed(broker_socket, launch, cid, runtime, admission)?,
            direct: TcpUpstreamRelay::default(),
        })
    }
}

#[cfg(test)]
#[path = "shim_managed_tests.rs"]
mod managed_tests;

impl SshRelay for BrokerFirstRelay {
    fn relay(
        &self,
        stream: std::os::unix::net::UnixStream,
        dest: &DivertDestination,
    ) -> std::io::Result<()> {
        self.relay_until(stream, dest, &AtomicBool::new(false))
    }

    fn relay_until(
        &self,
        stream: std::os::unix::net::UnixStream,
        dest: &DivertDestination,
        stop: &AtomicBool,
    ) -> std::io::Result<()> {
        if dest.credentials.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "SSH relay lacks compiled credential context",
            ));
        }
        if dest
            .credentials
            .iter()
            .any(|credential| credential.binding == CredentialBinding::Broker)
        {
            self.broker.relay_until(stream, dest, stop)
        } else {
            self.direct.relay_until(stream, dest, stop)
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
    if prelude.transport_cid != u64::from(cid) {
        return deny(
            &instance,
            "divert CID does not match the reserved runtime context".to_string(),
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
    let credentials: Vec<_> = grants
        .ssh_credentials_for_destination(&instance, &prelude.dest_host, prelude.dest_port)
        .cloned()
        .collect();
    if credentials.is_empty() {
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
        credentials,
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
    stop: Option<&AtomicBool>,
    clock: impl FnOnce() -> (String, u64),
) -> std::io::Result<(std::os::unix::net::UnixStream, DivertDecision)> {
    let ((peer, stream, prelude), (timestamp, now_secs)) =
        super::receipt_clock::receive_with_clock(
            || transport.accept_divert_with_stop(stop),
            clock,
        )?;
    let decision = decide_divert(
        registry, grants, audit, peer.cid, &prelude, now_secs, &timestamp,
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
    let (stream, decision) = accept_and_decide(transport, registry, grants, audit, None, || {
        (timestamp.to_owned(), now_secs)
    })?;
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
/// caller can use a nonblocking listener and unpark this thread at shutdown,
/// without resolving a pathname that may have been removed or replaced.
///
/// The existing loop retains at most 64 relay workers, reaping completed joins
/// before admitting another. Shutdown fences admission, cancels owned sockets,
/// and does not return until all workers join. A legacy resolver or relay may
/// remain pending; the lifecycle handle must retain its reservation in that case.
pub fn run_divert_until<R: CidResolver, L: SshRelay + Clone + 'static>(
    transport: &UnixSocketTransport<R>,
    registry: &CidRegistry,
    grants: &GrantStore,
    audit: &AuditLog,
    relay: L,
    stop: &AtomicBool,
) -> std::io::Result<()> {
    let mut workers = RelayWorkers::new();
    while !stop.load(Ordering::Relaxed) {
        workers.reap();
        if workers.failed {
            stop.store(true, Ordering::Release);
            break;
        }
        let (stream, decision) =
            match accept_and_decide(transport, registry, grants, audit, Some(stop), || {
                let timestamp = crate::microsandbox::runtime::time::current_rfc3339_utc();
                let now_secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                (timestamp, now_secs)
            }) {
                Ok(v) => v,
                Err(e) if stop.load(Ordering::Relaxed) => {
                    let _ = e;
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::park_timeout(std::time::Duration::from_millis(100));
                    continue;
                }
                Err(e) => {
                    eprintln!("WARNING: ssh divert accept failed: {e}");
                    continue;
                }
            };
        match decision {
            DivertDecision::Allow { dest } => {
                if stop.load(Ordering::Acquire) {
                    drop(stream);
                    break;
                }
                if let Err(e) = workers.spawn(relay.clone(), stream, dest) {
                    // Capacity/spawn refusal closes this session before relay
                    // admission or dialing. The audit above records only the
                    // destination decision, not relay establishment.
                    eprintln!("WARNING: SSH relay refused: {e} (session closed)");
                }
            }
            DivertDecision::Deny { .. } => {
                // Close: dropping the stream refuses the session.
                drop(stream);
            }
        }
    }
    workers.drain()
}

const MAX_RELAY_WORKERS: usize = 64;

struct RelayWorker {
    join: std::thread::JoinHandle<()>,
    guest: std::os::unix::net::UnixStream,
}

/// Local ownership inside the one accept loop, not an admission registry.
struct RelayWorkers {
    entries: Vec<RelayWorker>,
    stop: Arc<AtomicBool>,
    failed: bool,
}

impl RelayWorkers {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            stop: Arc::new(AtomicBool::new(false)),
            failed: false,
        }
    }

    fn reap(&mut self) {
        let mut index = 0;
        while index < self.entries.len() {
            if self.entries[index].join.is_finished() {
                let worker = self.entries.swap_remove(index);
                if worker.join.join().is_err() {
                    self.failed = true;
                }
            } else {
                index += 1;
            }
        }
    }

    fn spawn<L: SshRelay + 'static>(
        &mut self,
        relay: L,
        stream: std::os::unix::net::UnixStream,
        dest: DivertDestination,
    ) -> std::io::Result<()> {
        self.reap();
        if self.failed || self.stop.load(Ordering::Acquire) {
            return Err(relay_cancelled());
        }
        if self.entries.len() >= MAX_RELAY_WORKERS {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "SSH relay worker capacity exhausted",
            ));
        }
        let guest = stream.try_clone()?;
        let stop = Arc::clone(&self.stop);
        let join = std::thread::Builder::new()
            .name(format!("ssh-relay-{}-{}", dest.instance, dest.cid))
            .spawn(move || {
                if let Err(error) = relay.relay_until(stream, &dest, &stop) {
                    eprintln!("WARNING: SSH relay ended: {error}");
                }
            })?;
        self.entries.push(RelayWorker { join, guest });
        Ok(())
    }

    fn cancel(&self) {
        self.stop.store(true, Ordering::Release);
        for worker in &self.entries {
            let _ = worker.guest.shutdown(Shutdown::Both);
        }
    }

    fn drain(&mut self) -> std::io::Result<()> {
        self.cancel();
        while !self.entries.is_empty() {
            self.reap();
            if !self.entries.is_empty() {
                std::thread::park_timeout(Duration::from_millis(20));
            }
        }
        if self.failed {
            Err(std::io::Error::other("SSH relay retirement failed"))
        } else {
            Ok(())
        }
    }
}

impl Drop for RelayWorkers {
    fn drop(&mut self) {
        // Unwinding must fence streams, but dropping a JoinHandle is not a
        // completed join. The enclosing loop panic prevents CID retirement.
        self.cancel();
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

    /// Serve until stop; transport errors never kill loop — stderr-loud, continue.
    pub fn run_until<R: CidResolver + Send + Sync>(
        &self,
        transport: &UnixSocketTransport<R>,
        stop: &AtomicBool,
    ) {
        while !stop.load(Ordering::Relaxed) {
            let (peer, envelope) = match transport.accept_envelope(Some(stop)) {
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
            // Dispatch only; reply framing not yet implemented.
        }
    }
}

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

    #[test]
    fn signing_and_divert_loops_stop_during_an_accepted_incomplete_frame() {
        #[derive(Debug)]
        struct AcceptedResolver(std::sync::mpsc::Sender<()>);

        impl CidResolver for AcceptedResolver {
            fn resolve_cid(&self, _stream: &std::os::unix::net::UnixStream) -> Option<u32> {
                self.0.send(()).unwrap();
                Some(7)
            }
        }

        for signing in [false, true] {
            for prefix in [vec![], vec![0, 0], vec![0, 0, 0, 5, 1]] {
                let dir = tempfile::tempdir().unwrap();
                let path = dir.path().join("broker.sock");
                let (accepted, ready) = std::sync::mpsc::channel();
                let transport =
                    UnixSocketTransport::bind(&path, AcceptedResolver(accepted)).unwrap();
                let registry = bound_registry(dir.path());
                let audit = AuditLog::memory();
                let shim = BrokerShim::new(path.clone(), registry, test_service(), audit);
                let stop = AtomicBool::new(false);
                let mut client = std::os::unix::net::UnixStream::connect(&path).unwrap();
                client.write_all(&prefix).unwrap();
                let (finished, done) = std::sync::mpsc::channel();
                std::thread::scope(|scope| {
                    let server = scope.spawn(|| {
                        if signing {
                            shim.run_until(&transport, &stop);
                        } else {
                            let _ = run_divert_until(
                                &transport,
                                &shim.registry,
                                &divert_store(),
                                &shim.audit,
                                EchoRelay,
                                &stop,
                            );
                        }
                        finished.send(()).unwrap();
                    });
                    let was_accepted = ready.recv_timeout(Duration::from_secs(2));
                    stop.store(true, Ordering::Relaxed);
                    // Keep the stalled peer open until after measuring shutdown.
                    // On failure, close/wake before joining to bound the test too.
                    let stopped = done.recv_timeout(Duration::from_secs(2));
                    drop(client);
                    drop(std::os::unix::net::UnixStream::connect(&path));
                    server.join().unwrap();
                    assert!(
                        was_accepted.is_ok(),
                        "fixture must reach an accepted stream"
                    );
                    assert!(stopped.is_ok(), "signing={signing}, prefix={prefix:?}");
                });
                assert!(shim.audit.is_empty());
            }
        }
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
                on_violation: SecretViolationPolicy::Passthrough,
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
                    credentials: divert_store()
                        .ssh_credentials_for_destination("real-instance", "github.com", 22)
                        .cloned()
                        .collect(),
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
    fn decide_divert_carries_complete_credential_context_to_relay() {
        let dir = crate::config::test_support::unique_state_dir("divert-context");
        let registry = bound_registry(&dir);
        let broker = SshGrantPlan {
            name: "broker-key".to_string(),
            material: "BROKER_MATERIAL".to_string(),
            hosts: vec!["github.com".to_string(), "*.internal".to_string()],
            users: vec!["git".to_string(), "review".to_string()],
            ports: vec![22, 2222],
            binding: CredentialBinding::Broker,
            on_violation: SecretViolationPolicy::BlockAndLog,
        };
        let guest = SshGrantPlan {
            name: "guest-key".to_string(),
            material: "GUEST_MATERIAL".to_string(),
            users: vec!["deploy".to_string()],
            binding: CredentialBinding::Guest,
            on_violation: SecretViolationPolicy::BlockAndTerminate,
            ..broker.clone()
        };
        let unrelated = SshGrantPlan {
            hosts: vec!["unrelated.example".to_string()],
            ..broker.clone()
        };
        let plan = CredentialsPlan {
            ssh: vec![broker.clone(), guest.clone(), unrelated],
            signing: vec![],
            strict: false,
            strict_origin: None,
        };
        let store = GrantStore::compile(&[("real-instance", &plan)]);
        let decision = decide_divert(
            &registry,
            &store,
            &AuditLog::memory(),
            7,
            &divert_prelude("github.com", 22, 7, DIVERT_NOW),
            DIVERT_NOW,
            "t",
        );
        let DivertDecision::Allow { dest } = decision else {
            panic!("configured endpoint must be allowed");
        };
        assert_eq!(dest.instance, "real-instance");
        assert_eq!(dest.credentials, vec![broker, guest]);
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
    fn decide_divert_refuses_claims_for_a_different_reserved_runtime() {
        let dir = crate::config::test_support::unique_state_dir("divert-cid-mismatch");
        let registry = bound_registry(&dir);
        for claimed in [0, 1, 2, 3, 6, 8, 65_536, u64::from(u32::MAX)] {
            let audit = AuditLog::memory();
            let decision = decide_divert(
                &registry,
                &divert_store(),
                &audit,
                7,
                &divert_prelude("github.com", 22, claimed, DIVERT_NOW),
                DIVERT_NOW,
                "t",
            );
            assert!(matches!(
                decision,
                DivertDecision::Deny {
                    re_attest: false,
                    ..
                }
            ));
            assert_eq!(audit.len(), 1);
        }
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
    fn accept_and_decide_samples_receipt_clock_and_preserves_skew_limits() {
        use std::sync::Arc;
        use std::sync::atomic::AtomicU64;

        #[derive(Debug)]
        struct ReceiptClockResolver {
            clock: Arc<AtomicU64>,
            received_at: u64,
        }

        impl CidResolver for ReceiptClockResolver {
            fn resolve_cid(&self, _stream: &std::os::unix::net::UnixStream) -> Option<u32> {
                // Simulate time spent waiting for a client, without wall-clock sleeps.
                self.clock.store(self.received_at, Ordering::SeqCst);
                Some(7)
            }
        }

        let receipt_time = DIVERT_NOW + 600;
        for offset in [-301_i64, -300, 0, 300, 301] {
            let dir = tempfile::tempdir().unwrap();
            let socket_path = dir.path().join("broker.sock");
            let clock = Arc::new(AtomicU64::new(DIVERT_NOW));
            let transport = UnixSocketTransport::bind(
                &socket_path,
                ReceiptClockResolver {
                    clock: Arc::clone(&clock),
                    received_at: receipt_time,
                },
            )
            .unwrap();
            let registry = bound_registry(dir.path());
            let audit = AuditLog::memory();
            let mut client = std::os::unix::net::UnixStream::connect(&socket_path).unwrap();
            let epoch = receipt_time.checked_add_signed(offset).unwrap();
            let framed = encode_ssh_divert_prelude(&divert_prelude("github.com", 22, 7, epoch));
            client.write_all(&framed).unwrap();
            client.shutdown(Shutdown::Write).unwrap();

            let (_stream, decision) =
                accept_and_decide(&transport, &registry, &divert_store(), &audit, None, || {
                    let now = clock.load(Ordering::SeqCst);
                    (format!("receipt-{now}"), now)
                })
                .unwrap();
            assert_eq!(
                matches!(decision, DivertDecision::Allow { .. }),
                offset.unsigned_abs() <= MAX_DIVERT_EPOCH_SKEW_SECS,
                "offset {offset}: {decision:?}"
            );
            let records = audit.snapshot();
            assert_eq!(records.len(), 1);
            assert_eq!(records[0].timestamp, format!("receipt-{receipt_time}"));
        }
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
            credentials: mixed_binding_store(port)
                .ssh_credentials_for_destination("real-instance", "127.0.0.1", port)
                .cloned()
                .collect(),
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
            credentials: Vec::new(),
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

    /// Bind the broker stub listener on the CALLING thread and serve it on
    /// a worker: the relay under test dials as soon as it runs, so binding
    /// inside the spawned stub worker raced the dial under parallel load.
    /// When the dial won, it failed with ENOENT, the relay dropped the
    /// session, and the guest observed ECONNRESET instead of the stub
    /// reply. Binding first makes stub readiness deterministic.
    fn bind_broker_stub(socket_path: &std::path::Path) -> std::os::unix::net::UnixListener {
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::os::unix::net::UnixListener::bind(socket_path).unwrap()
    }

    fn serve_broker_stub(
        listener: std::os::unix::net::UnixListener,
        expect_host: String,
        expect_port: u16,
    ) {
        use microsandbox_network::ssh::gateway::decode_ssh_divert_prelude;
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
        // Bind before spawning so the stub socket exists before the relay
        // dials (see `bind_broker_stub`).
        let stub_listener = bind_broker_stub(&socket_path);
        let stub_worker = std::thread::spawn(move || {
            serve_broker_stub(stub_listener, "broker.example".to_string(), 22)
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
            credentials: mixed_binding_store(1)
                .ssh_credentials_for_destination("real-instance", "broker.example", 22)
                .cloned()
                .collect(),
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
            credentials: mixed_binding_store(1)
                .ssh_credentials_for_destination("real-instance", "broker.example", 22)
                .cloned()
                .collect(),
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
                    on_violation: SecretViolationPolicy::Passthrough,
                },
                SshGrantPlan {
                    name: "local".to_string(),
                    material: "LOCAL_KEY".to_string(),
                    hosts: vec!["127.0.0.1".to_string()],
                    users: vec!["git".to_string()],
                    ports: vec![tcp_port],
                    binding: CredentialBinding::Guest,
                    on_violation: SecretViolationPolicy::Passthrough,
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
        // Bind before spawning so the stub socket exists before the relay
        // dials (see `bind_broker_stub`).
        let stub_listener = bind_broker_stub(&socket_path);
        let stub_worker = std::thread::spawn(move || {
            serve_broker_stub(stub_listener, "broker.example".to_string(), 22)
        });
        let relay = BrokerFirstRelay::new(socket_path);
        let (mut guest, shuttle) = std::os::unix::net::UnixStream::pair().unwrap();
        guest
            .set_read_timeout(Some(std::time::Duration::from_secs(15)))
            .unwrap();
        let dest = DivertDestination {
            instance: "real-instance".to_string(),
            cid: 7,
            dest_host: "broker.example".to_string(),
            dest_port: 22,
            credentials: mixed_binding_store(1)
                .ssh_credentials_for_destination("real-instance", "broker.example", 22)
                .cloned()
                .collect(),
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
    fn broker_first_relay_without_credential_context_never_dials() {
        let stub = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        stub.set_nonblocking(true).unwrap();
        let mut dest = tcp_dest(stub.local_addr().unwrap().port());
        dest.credentials.clear();
        let dir = crate::config::test_support::unique_state_dir("broker-no-context");
        let socket = broker_stub_path(&dir);
        let broker = bind_broker_stub(&socket);
        broker.set_nonblocking(true).unwrap();
        let relay = BrokerFirstRelay::new(socket);
        let (mut guest, shuttle) = std::os::unix::net::UnixStream::pair().unwrap();
        guest
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        assert_eq!(
            relay.relay(shuttle, &dest).unwrap_err().kind(),
            std::io::ErrorKind::PermissionDenied
        );
        assert_eq!(guest.read(&mut [0]).unwrap(), 0);
        assert_eq!(
            stub.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        assert_eq!(
            broker.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        drop(broker);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn broker_first_relay_mixed_custody_never_falls_back_to_direct() {
        let stub = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        stub.set_nonblocking(true).unwrap();
        let mut dest = tcp_dest(stub.local_addr().unwrap().port());
        let brokered = SshGrantPlan {
            name: "broker-key".to_string(),
            material: "BROKER_KEY".to_string(),
            binding: CredentialBinding::Broker,
            ..dest.credentials[0].clone()
        };
        dest.credentials.push(brokered);
        let dir = crate::config::test_support::unique_state_dir("broker-mixed-context");
        let relay = BrokerFirstRelay::new(dir.join("absent.sock"));
        for _ in 0..2 {
            let (mut guest, shuttle) = std::os::unix::net::UnixStream::pair().unwrap();
            guest
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            relay.relay(shuttle, &dest).unwrap_err();
            assert_eq!(guest.read(&mut [0]).unwrap(), 0);
            assert_eq!(
                stub.accept().unwrap_err().kind(),
                std::io::ErrorKind::WouldBlock
            );
            dest.credentials.reverse();
        }
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
        let relay = BrokerFirstRelay::new(dir.join("no-broker.sock"));
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
