//! Host-side TCP forwarder for broker VM egress.
//!
//! The broker VM keeps its guest IP stack down, so upstream SSH egress
//! leaves through its vsock egress port, which the launcher projects onto
//! the host egress socket ([`crate::microsandbox::broker::registry::broker_egress_socket_path`]).
//! The forwarder answers the egress-port framing there: it reads one framed
//! connect request naming the upstream destination, enforces the compiled
//! grant allowlist, dials `dest_host:dest_port` on allow, and then pumps
//! bytes both ways until EOF or error. This is the only egress path for
//! the broker VM.
//!
//! Framing mirrors the guest-side egress codec (length-prefixed CBOR): the
//! struct field names and types here must stay identical to the brokerd
//! `EgressConnect` / `EgressAck` shapes so both sides decode the same CBOR
//! map without sharing a crate.

use crate::microsandbox::broker::audit::{AuditLog, AuditRecord};
use crate::microsandbox::broker::registry::broker_egress_socket_path;
use crate::microsandbox::broker::signing::GrantStore;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

/// Maximum egress connect/ack frame size (matches the guest-side cap so a
/// corrupt length prefix cannot force a huge allocation).
const MAX_EGRESS_FRAME_BYTES: usize = 64 * 1024;

/// Bound on one upstream TCP dial. A diverted session must fail closed fast
/// when the granted upstream is unroutable — an unbounded connect would park
/// the tunnel worker and leave the guest hanging instead of closing it.
pub const EGRESS_CONNECT_TIMEOUT_SECS: u64 = 10;

/// Connect request opening one egress tunnel to an upstream destination.
/// Field names and types mirror the guest-side codec.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EgressConnect {
    /// Upstream hostname or IP string (matches the divert prelude dest).
    pub host: String,
    /// Upstream TCP port.
    pub port: u16,
}

/// Forwarder acknowledgement for one egress connect request. Mirrors the
/// guest-side codec; `detail` defaults so older senders decode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EgressAck {
    /// Whether the forwarder now carries TCP bytes on this stream.
    pub ok: bool,
    /// Human-readable detail for refused tunnels (identifiers only).
    #[serde(default)]
    pub detail: String,
}

/// Encode a value as `u32` big-endian length plus CBOR payload. Encoding
/// an in-memory struct is infallible in practice; on the impossible error
/// path an empty frame is emitted so the peer rejects it fail-closed
/// instead of panicking the forwarder.
fn frame_cbor<T: Serialize>(value: &T) -> Vec<u8> {
    let mut payload = Vec::new();
    if ciborium::into_writer(value, &mut payload).is_err() {
        return vec![0, 0, 0, 0];
    }
    let Ok(len) = u32::try_from(payload.len()) else {
        return vec![0, 0, 0, 0];
    };
    let mut framed = Vec::with_capacity(4 + payload.len());
    framed.extend_from_slice(&len.to_be_bytes());
    framed.extend_from_slice(&payload);
    framed
}

/// Encode an egress acknowledgement.
pub fn encode_egress_ack(ack: &EgressAck) -> Vec<u8> {
    frame_cbor(ack)
}

/// Encode an egress connect request (client side of the contract; the
/// broker VM encodes its own, tests use this).
pub fn encode_egress_connect(request: &EgressConnect) -> Vec<u8> {
    frame_cbor(request)
}

/// Read one framed connect request from the tunnel stream (fail-closed at
/// [`MAX_EGRESS_FRAME_BYTES`]).
fn read_connect(stream: &mut std::os::unix::net::UnixStream) -> std::io::Result<EgressConnect> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let len = u32::from_be_bytes(len_buf) as usize;
    if len == 0 || len > MAX_EGRESS_FRAME_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("egress frame length {len} out of bounds (max {MAX_EGRESS_FRAME_BYTES})"),
        ));
    }
    let mut payload = vec![0u8; len];
    stream.read_exact(&mut payload)?;
    crate::microsandbox::broker::signing::decode_cbor(&payload)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))
}

/// Write one framed acknowledgement to the tunnel stream.
fn write_ack(stream: &mut std::os::unix::net::UnixStream, ack: &EgressAck) -> std::io::Result<()> {
    let framed = encode_egress_ack(ack);
    stream.write_all(&framed)?;
    stream.flush()?;
    Ok(())
}

/// Dial the requested destination, trying each resolved address in order
/// until one connects, with the timeout bounding every attempt.
fn dial(host: &str, port: u16, timeout: Duration) -> std::io::Result<TcpStream> {
    let addrs = (host, port).to_socket_addrs().map_err(|e| {
        std::io::Error::other(format!("egress resolve failed for {host}:{port}: {e}"))
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
            "egress resolve returned no addresses for {host}:{port}"
        ))
    }))
}

/// Pump bytes both ways between the tunnel stream and the dialed upstream
/// until EOF or error, propagating half-close both ways so each side
/// observes the other's EOF instead of half-hanging.
fn pump(
    tunnel: std::os::unix::net::UnixStream,
    upstream: TcpStream,
    label: &str,
) -> std::io::Result<()> {
    let mut tunnel_in = tunnel.try_clone()?;
    let mut tunnel_out = tunnel;
    let mut upstream_in = upstream.try_clone()?;
    let mut upstream_out = upstream;
    // Upstream→tunnel runs on a worker; tunnel→upstream runs here. Each
    // side shuts the far write half when its copy ends so EOF propagates.
    let worker = std::thread::Builder::new()
        .name(format!("egress-up-{label}"))
        .spawn(move || {
            let res = std::io::copy(&mut upstream_in, &mut tunnel_out);
            let _ = tunnel_out.shutdown(Shutdown::Write);
            res
        })
        .map_err(|e| std::io::Error::other(format!("egress pump worker spawn failed: {e}")))?;
    let downstream = std::io::copy(&mut tunnel_in, &mut upstream_out);
    let _ = upstream_out.shutdown(Shutdown::Write);
    let upstream_res = match worker.join() {
        Ok(res) => res,
        Err(_) => Err(std::io::Error::other("egress pump worker did not finish")),
    };
    downstream?;
    upstream_res?;
    Ok(())
}

/// Dial upstream for one allowed tunnel and pump it: answer `ok: true`,
/// then carry bytes until EOF or error. Dial failures answer `ok: false`
/// and close — fail-closed, never a hang. Runs on the loop's worker
/// threads (and inline in [`serve_egress_once`]).
fn open_tunnel(
    mut stream: std::os::unix::net::UnixStream,
    request: &EgressConnect,
) -> std::io::Result<()> {
    let upstream = match dial(
        &request.host,
        request.port,
        Duration::from_secs(EGRESS_CONNECT_TIMEOUT_SECS),
    ) {
        Ok(upstream) => upstream,
        Err(e) => {
            let reason = format!(
                "egress dial to {}:{} failed: {e}",
                request.host, request.port
            );
            eprintln!("WARNING: {reason}");
            let _ = write_ack(
                &mut stream,
                &EgressAck {
                    ok: false,
                    detail: reason,
                },
            );
            return Err(std::io::Error::other("egress upstream dial failed"));
        }
    };
    if let Err(e) = write_ack(
        &mut stream,
        &EgressAck {
            ok: true,
            detail: String::new(),
        },
    ) {
        eprintln!(
            "WARNING: egress ack write failed for {}:{}: {e}",
            request.host, request.port
        );
        return Err(e);
    }
    let label = format!("{}:{}", request.host, request.port);
    pump(stream, upstream, &label)
}

/// Serve one egress connection: read the connect request, enforce the
/// allowlist, and on allow dial upstream and pump. Refusals answer
/// `ok: false` and close; undecodable frames close stderr-loud without an
/// audit record (no trustworthy fields). The tunnel runs INLINE here (the
/// single-shot and test path); the loop in [`run_egress_until`] pumps on
/// worker threads instead.
///
/// Returns `true` when a tunnel carried bytes, `false` when the connection
/// was refused or failed before the pump.
pub fn serve_egress_once(
    mut stream: std::os::unix::net::UnixStream,
    grants: &GrantStore,
    audit: &AuditLog,
    timestamp: &str,
) -> bool {
    let request = match read_connect(&mut stream) {
        Ok(request) => request,
        Err(e) => {
            eprintln!("WARNING: egress connect read failed: {e}");
            return false;
        }
    };
    if !grants.ssh_authorized_any(&request.host, request.port) {
        let reason = format!(
            "egress to {}:{} is not granted for any instance",
            request.host, request.port
        );
        let record = AuditRecord::ssh_divert_deny(
            timestamp,
            "broker",
            0,
            &request.host,
            request.port,
            &reason,
        );
        if let Err(e) = audit.append(record) {
            eprintln!("WARNING: broker audit append failed: {e}");
        }
        eprintln!("WARNING: {reason} (tunnel refused)");
        let _ = write_ack(
            &mut stream,
            &EgressAck {
                ok: false,
                detail: reason,
            },
        );
        return false;
    }
    let record = AuditRecord::ssh_divert_allow(timestamp, "broker", 0, &request.host, request.port);
    if let Err(e) = audit.append(record) {
        eprintln!("WARNING: broker audit append failed: {e}");
    }
    match open_tunnel(stream, &request) {
        Ok(()) => true,
        Err(e) => {
            eprintln!(
                "WARNING: egress tunnel for {}:{} ended: {e}",
                request.host, request.port
            );
            false
        }
    }
}

/// Serve egress connections until `stop` is set. Decided connections never
/// kill the loop; after any accept failure a set `stop` exits instead — the
/// shutdown dummy connection surfaces exactly such a failure to unblock
/// `accept`.
///
/// Allowed tunnels pump on detached worker threads: the pump blocks for the
/// whole session, so serving it inline would stall every later tunnel
/// behind it. Each worker owns its stream and request — nothing borrowed —
/// so the loop keeps accepting while tunnels flow. Shutdown still only
/// joins this loop thread: in-flight tunnels drain on their own EOF after
/// the socket is removed instead of hanging teardown.
pub fn run_egress_until(
    listener: &std::os::unix::net::UnixListener,
    grants: &GrantStore,
    audit: &AuditLog,
    stop: &AtomicBool,
) {
    while !stop.load(Ordering::Relaxed) {
        let (mut stream, _) = match listener.accept() {
            Ok(v) => v,
            Err(e) if stop.load(Ordering::Relaxed) => {
                let _ = e;
                break;
            }
            Err(e) => {
                eprintln!("WARNING: egress accept failed: {e}");
                continue;
            }
        };
        let request = match read_connect(&mut stream) {
            Ok(request) => request,
            Err(e) if stop.load(Ordering::Relaxed) => {
                let _ = e;
                break;
            }
            Err(e) => {
                eprintln!("WARNING: egress connect read failed: {e}");
                continue;
            }
        };
        let timestamp = crate::microsandbox::runtime::time::current_rfc3339_utc();
        if !grants.ssh_authorized_any(&request.host, request.port) {
            let reason = format!(
                "egress to {}:{} is not granted for any instance",
                request.host, request.port
            );
            let record = AuditRecord::ssh_divert_deny(
                &timestamp,
                "broker",
                0,
                &request.host,
                request.port,
                &reason,
            );
            if let Err(e) = audit.append(record) {
                eprintln!("WARNING: broker audit append failed: {e}");
            }
            eprintln!("WARNING: {reason} (tunnel refused)");
            let _ = write_ack(
                &mut stream,
                &EgressAck {
                    ok: false,
                    detail: reason,
                },
            );
            continue;
        }
        let record =
            AuditRecord::ssh_divert_allow(&timestamp, "broker", 0, &request.host, request.port);
        if let Err(e) = audit.append(record) {
            eprintln!("WARNING: broker audit append failed: {e}");
        }
        let label = format!("egress-{}:{}", request.host, request.port);
        match std::thread::Builder::new().name(label.clone()).spawn({
            let value = label.clone();
            move || {
                if let Err(e) = open_tunnel(stream, &request) {
                    eprintln!("WARNING: {value} ended: {e}");
                }
            }
        }) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("WARNING: {label} spawn failed: {e} (tunnel closed)");
            }
        }
    }
}

/// Owns the host egress listener: the bound socket, the background accept
/// loop, and the compiled allowlist. Release via
/// [`EgressForwarderHandle::shutdown`] when the broker VM ends — the socket
/// and thread are relinquished there. Best-effort throughout: failures are
/// stderr-loud, never propagated.
#[derive(Debug)]
pub struct EgressForwarderHandle {
    /// Host-side egress socket path (never guest-visible).
    pub socket_path: PathBuf,
    stop: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl EgressForwarderHandle {
    /// Stop the accept loop (stop flag plus a dummy connection to unblock
    /// `accept`), join the thread, and remove the socket. Best-effort:
    /// teardown must not fail the service exit it follows.
    pub fn shutdown(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Unblock the loop's `accept`: the dummy connection wakes it, the
        // resulting EOF fails the frame read, and the loop sees `stop`.
        if let Ok(stream) = std::os::unix::net::UnixStream::connect(&self.socket_path) {
            drop(stream);
        }
        if let Some(join) = self.join.take()
            && join.join().is_err()
        {
            eprintln!(
                "WARNING: egress forwarder thread for {} failed during shutdown",
                self.socket_path.display()
            );
        }
        if let Err(e) = std::fs::remove_file(&self.socket_path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            eprintln!(
                "WARNING: failed to remove egress socket {}: {e}",
                self.socket_path.display()
            );
        }
    }
}

/// Bind the host egress listener and serve the allowlist on a background
/// thread until [`EgressForwarderHandle::shutdown`].
///
/// A live listener at the path (an earlier reservation owns it) surfaces
/// [`std::io::ErrorKind::AddrInUse`] so the caller can share instead of
/// stealing; only a stale socket file from an unclean shutdown is unlinked
/// before binding, so it cannot wedge the next launch. The liveness probe
/// is a connect: success vouches a live listener, refusal proves stale.
pub fn ensure_egress_forwarder(
    state_dir: &Path,
    grants: GrantStore,
) -> std::io::Result<EgressForwarderHandle> {
    let socket_path = broker_egress_socket_path(state_dir);
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let listener = match std::os::unix::net::UnixListener::bind(&socket_path) {
        Ok(listener) => listener,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            if std::os::unix::net::UnixStream::connect(&socket_path).is_ok() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::AddrInUse,
                    format!(
                        "egress socket {} already serves a live reservation",
                        socket_path.display()
                    ),
                ));
            }
            std::fs::remove_file(&socket_path)?;
            std::os::unix::net::UnixListener::bind(&socket_path)?
        }
        Err(e) => return Err(e),
    };
    let audit = Arc::new(AuditLog::with_state_dir(state_dir));
    let stop = Arc::new(AtomicBool::new(false));
    let join = std::thread::Builder::new()
        .name("broker-egress".to_string())
        .spawn({
            let loop_stop = Arc::clone(&stop);
            move || {
                run_egress_until(&listener, &grants, &audit, &loop_stop);
            }
        })
        .map_err(|e| std::io::Error::other(format!("egress forwarder thread spawn failed: {e}")))?;
    Ok(EgressForwarderHandle {
        socket_path,
        stop,
        join: Some(join),
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::microsandbox::plan::{CredentialBinding, CredentialsPlan, SshGrantPlan};

    fn granted_store(port: u16) -> GrantStore {
        let plan = CredentialsPlan {
            ssh: vec![SshGrantPlan {
                name: "deploy".to_string(),
                material: "DEPLOY_KEY".to_string(),
                hosts: vec!["127.0.0.1".to_string()],
                users: vec!["git".to_string()],
                ports: vec![port],
                binding: CredentialBinding::Broker,
                on_violation: crate::config::SecretViolationPolicy::Passthrough,
            }],
            signing: Vec::new(),
            strict: false,
            strict_origin: None,
        };
        GrantStore::compile(&[("personal-pi", &plan)])
    }

    fn read_ack(stream: &mut std::os::unix::net::UnixStream) -> EgressAck {
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf).unwrap();
        let len = u32::from_be_bytes(len_buf) as usize;
        let mut payload = vec![0u8; len];
        stream.read_exact(&mut payload).unwrap();
        ciborium::from_reader(&payload[..]).unwrap()
    }

    #[test]
    fn egress_framing_round_trips() {
        let request = EgressConnect {
            host: "example.com".to_string(),
            port: 22,
        };
        let framed = encode_egress_connect(&request);
        let mut len_buf = [0u8; 4];
        len_buf.copy_from_slice(&framed[..4]);
        assert_eq!(u32::from_be_bytes(len_buf) as usize, framed.len() - 4);
        let decoded: EgressConnect = ciborium::from_reader(&framed[4..]).unwrap();
        assert_eq!(decoded, request);

        let ack = EgressAck {
            ok: false,
            detail: "denied by policy".to_string(),
        };
        let framed = encode_egress_ack(&ack);
        let decoded: EgressAck = ciborium::from_reader(&framed[4..]).unwrap();
        assert_eq!(decoded, ack);
    }

    #[test]
    fn forwarder_allows_granted_destination_and_pumps() {
        // Granted upstream stub: checks the tunnel bytes, answers, then
        // expects the tunnel half-close as EOF.
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
            let mut rest = Vec::new();
            conn.read_to_end(&mut rest).unwrap();
            assert!(rest.is_empty(), "tunnel must send nothing after its reply");
        });
        let dir = crate::config::test_support::unique_state_dir("egress-allow");
        let socket_path = dir.join("egress.sock");
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let listener = std::os::unix::net::UnixListener::bind(&socket_path).unwrap();
        let audit = AuditLog::memory();
        std::thread::scope(|s| {
            let server = s.spawn(|| {
                let (stream, _) = listener.accept().unwrap();
                serve_egress_once(stream, &granted_store(port), &audit, "t")
            });
            let mut client = std::os::unix::net::UnixStream::connect(&socket_path).unwrap();
            client
                .set_read_timeout(Some(std::time::Duration::from_secs(15)))
                .unwrap();
            client
                .write_all(&encode_egress_connect(&EgressConnect {
                    host: "127.0.0.1".to_string(),
                    port,
                }))
                .unwrap();
            let ack = read_ack(&mut client);
            assert!(ack.ok, "granted egress must ack ok: {ack:?}");
            client.write_all(b"hello").unwrap();
            let mut reply = [0u8; 5];
            client.read_exact(&mut reply).unwrap();
            assert_eq!(&reply, b"world");
            client.shutdown(Shutdown::Write).unwrap();
            let mut tail = Vec::new();
            client.read_to_end(&mut tail).unwrap();
            assert!(tail.is_empty(), "upstream close must surface as tunnel EOF");
            assert!(
                server.join().unwrap(),
                "allowed tunnel must report established"
            );
        });
        stub_worker.join().unwrap();
        assert_eq!(audit.len(), 1);
        assert!(matches!(
            audit.snapshot()[0].result,
            crate::microsandbox::broker::audit::AuditResult::Allow
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn forwarder_refuses_non_granted_destination_with_nack_and_close() {
        let dir = crate::config::test_support::unique_state_dir("egress-deny");
        let socket_path = dir.join("egress.sock");
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let listener = std::os::unix::net::UnixListener::bind(&socket_path).unwrap();
        let audit = AuditLog::memory();
        // The store grants only 127.0.0.1:22; this dial names an ungranted
        // host and must be refused before any dial is attempted.
        std::thread::scope(|s| {
            let server = s.spawn(|| {
                let (stream, _) = listener.accept().unwrap();
                serve_egress_once(stream, &granted_store(22), &audit, "t")
            });
            let mut client = std::os::unix::net::UnixStream::connect(&socket_path).unwrap();
            client
                .set_read_timeout(Some(std::time::Duration::from_secs(15)))
                .unwrap();
            client
                .write_all(&encode_egress_connect(&EgressConnect {
                    host: "evil.example".to_string(),
                    port: 22,
                }))
                .unwrap();
            let ack = read_ack(&mut client);
            assert!(!ack.ok, "non-granted egress must nack: {ack:?}");
            assert!(
                ack.detail.contains("evil.example"),
                "refusal detail names the destination: {ack:?}"
            );
            // Refused tunnels close: the client reads EOF after the nack.
            let mut buf = [0u8; 1];
            assert_eq!(
                client.read(&mut buf).unwrap(),
                0,
                "refused tunnel must close"
            );
            assert!(
                !server.join().unwrap(),
                "refused tunnel must not report established"
            );
        });
        assert_eq!(audit.len(), 1);
        assert!(matches!(
            audit.snapshot()[0].result,
            crate::microsandbox::broker::audit::AuditResult::Deny { .. }
        ));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn forwarder_handle_shutdown_removes_socket() {
        let dir = crate::config::test_support::unique_state_dir("egress-life");
        // Bind through the real ensure path so stale-file recovery and the
        // background loop are covered, then shut down immediately.
        let handle = ensure_egress_forwarder(&dir, granted_store(22)).unwrap();
        assert_eq!(
            handle.socket_path,
            broker_egress_socket_path(&dir),
            "forwarder binds the derived egress path"
        );
        assert!(handle.socket_path.exists(), "listener socket file exists");
        handle.shutdown();
        assert!(
            !broker_egress_socket_path(&dir).exists(),
            "shutdown removes the socket"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn forwarder_recovers_from_stale_socket_file() {
        // A leftover regular file at the egress path (unclean shutdown)
        // must not wedge the next launch: ensure unlinks the stale file
        // before binding, like the SSH shim does.
        let dir = crate::config::test_support::unique_state_dir("egress-stale");
        let socket_path = broker_egress_socket_path(&dir);
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&socket_path, b"stale").unwrap();
        let handle = ensure_egress_forwarder(&dir, granted_store(22)).unwrap();
        assert_eq!(
            handle.socket_path, socket_path,
            "forwarder binds the derived egress path"
        );
        let file_type = std::fs::symlink_metadata(&socket_path).unwrap().file_type();
        assert!(
            std::os::unix::fs::FileTypeExt::is_socket(&file_type),
            "the stale file is replaced by a live socket"
        );
        handle.shutdown();
        assert!(
            !socket_path.exists(),
            "shutdown removes the socket it bound over the stale file"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
