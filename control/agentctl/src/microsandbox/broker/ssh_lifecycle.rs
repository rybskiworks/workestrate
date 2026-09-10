//! SSH shim lifecycle: per-launch listener setup and teardown.
//!
//! When a workload emits an SSH policy with at least one grant,
//! [`ensure_ssh_shim`] allocates the sandbox's transport CID, binds the
//! broker socket, and serves divert decisions on a background thread until
//! the foreground service ends. Strict-only confinement emits a policy but
//! binds no listener — nothing can divert to it, so it takes no handle.

use crate::microsandbox::broker::audit::AuditLog;
use crate::microsandbox::broker::epoch::EpochToken;
use crate::microsandbox::broker::epoch_provision::{
    AgentConsoleChannel, CONSOLE_PROVISION_TIMEOUT_SECS, EpochProvisionError,
    provision_with_deadline,
};
use crate::microsandbox::broker::registry::{
    CidRegistry, broker_socket_path, broker_vm_socket_path,
};
use crate::microsandbox::broker::shim::{
    BrokerFirstRelay, FixedCidResolver, UnixSocketTransport, run_divert_until,
};
use crate::microsandbox::broker::signing::GrantStore;
use crate::microsandbox::plan::CredentialsPlan;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Deliver the launch epoch to the guest over the console agent channel:
/// bump the CID's persisted wire sequence (every emission goes through the
/// bump, so a value is never sent twice even across a failed send followed
/// by a retry), dial the sandbox's agent relay, gate on the negotiated
/// generation, send the provision, and check the ack. The sandbox identity
/// for the dial and the provision payload both come from the CID's live
/// registry binding (single source of truth — the caller only names the
/// CID). Returns the provisioned wire epoch.
///
/// Unknown or tombstoned CIDs fail with [`EpochProvisionError::NotBound`]
/// before any dial — there is nothing to provision.
/// After the registry bump, one deadline bounds the asynchronous connection,
/// handshake, write and acknowledgement. A timeout consumes that wire epoch;
/// retries still bump the sequence rather than replaying it.
pub async fn provision_epoch_via_console(
    registry: &CidRegistry,
    cid: u32,
) -> Result<u64, EpochProvisionError> {
    let entry = registry
        .lookup(cid)
        .map_err(|e| EpochProvisionError::SendFailed {
            detail: format!("CID {cid} lookup failed: {e}"),
        })?
        .ok_or(EpochProvisionError::NotBound { cid })?;
    let wire_epoch =
        registry
            .bump_wire_epoch(cid)
            .map_err(|e| EpochProvisionError::SendFailed {
                detail: format!("wire epoch bump for CID {cid} failed after live lookup: {e}"),
            })?;
    let deadline = tokio::time::Instant::now()
        + std::time::Duration::from_secs(CONSOLE_PROVISION_TIMEOUT_SECS);
    provision_with_deadline(
        AgentConsoleChannel::connect(&entry.instance),
        &entry.instance,
        cid,
        wire_epoch,
        deadline,
    )
    .await
}

/// Re-issue the epoch for a live CID over the console agent channel: the
/// repeatable provision path for re-attestation and fork recovery. Bumps the
/// persisted wire sequence first, so the re-issue always supersedes — never
/// replays — the previous provision. Fails with
/// [`EpochProvisionError::NotBound`] when the CID has no live binding.
pub async fn reprovision_epoch(state_dir: &Path, cid: u32) -> Result<u64, EpochProvisionError> {
    let registry = CidRegistry::open(state_dir).map_err(|e| EpochProvisionError::SendFailed {
        detail: format!("CID registry open failed: {e}"),
    })?;
    provision_epoch_via_console(&registry, cid).await
}

/// Owns one workload's SSH shim: the bound socket, the allocated
/// transport CID, and the background divert loop. Release via
/// [`SshShimHandle::shutdown`] when the foreground service ends — the
/// socket, thread, and CID binding are all relinquished there. The divert
/// path needs no signing backend, so this type is deliberately
/// non-generic.
#[derive(Debug)]
pub struct SshShimHandle {
    /// Host-side broker socket path (never guest-visible).
    pub socket_path: PathBuf,
    /// Transport CID attributed to this sandbox's diverted sessions.
    pub transport_cid: u64,
    /// Sandbox identity owning the CID binding (teardown).
    pub instance: String,
    /// Allocated CID (teardown).
    pub cid: u32,
    state_dir: PathBuf,
    stop: Arc<AtomicBool>,
    join: Option<std::thread::JoinHandle<()>>,
}

impl SshShimHandle {
    /// Stop the divert loop (stop flag plus a dummy connection to unblock
    /// `accept`), join the thread, remove the socket, and expire the CID
    /// binding. Best-effort throughout: failures are stderr-loud, never
    /// propagated — teardown must not fail the service exit it follows.
    pub fn shutdown(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        // Unblock the loop's `accept`: the dummy connection wakes it, the
        // resulting EOF fails the frame read, and the loop sees `stop`.
        // `connect` queues in the listener backlog, so this cannot hang
        // even if the loop already exited.
        if let Ok(stream) = std::os::unix::net::UnixStream::connect(&self.socket_path) {
            drop(stream);
        }
        if let Some(join) = self.join.take()
            && join.join().is_err()
        {
            eprintln!(
                "WARNING: ssh shim thread for instance '{}' failed during shutdown",
                self.instance
            );
        }
        if let Err(e) = std::fs::remove_file(&self.socket_path)
            && e.kind() != std::io::ErrorKind::NotFound
        {
            eprintln!(
                "WARNING: failed to remove ssh shim socket {}: {e}",
                self.socket_path.display()
            );
        }
        match CidRegistry::open(&self.state_dir).and_then(|registry| registry.expire(self.cid)) {
            Ok(()) => {}
            Err(e) => eprintln!(
                "WARNING: failed to expire CID {} for instance '{}': {e}",
                self.cid, self.instance
            ),
        }
    }
}

/// Set up the SSH shim for one workload launch, or `None` when the
/// workload carries no SSH grants (strict-only confinement emits a policy
/// but binds no listener — nothing can divert to it).
///
/// The launch epoch is provisioned over the console agent channel right
/// after the CID is allocated. A failed provision warns and continues the
/// host-side setup: the guest agent boots with the VM and may simply not
/// answer yet. The complete asynchronous exchange is bounded, including an
/// accepted console connection that never acknowledges the provision.
/// Enforcement stays fail-closed meanwhile — an unprovisioned guest fails
/// the shim-side checks — and [`reprovision_epoch`] retries explicitly on
/// re-attestation.
pub async fn ensure_ssh_shim(
    state_dir: &Path,
    instance: &str,
    credentials: &CredentialsPlan,
) -> anyhow::Result<Option<SshShimHandle>> {
    if credentials.ssh.is_empty() {
        return Ok(None);
    }
    // The issue-time CID is documentation only (the token is opaque
    // entropy); the real (instance, CID) binding persists at allocate.
    let epoch = EpochToken::issue(instance, 0)
        .map_err(|e| anyhow::anyhow!("ssh shim: epoch issue failed: {e}"))?;
    let registry = CidRegistry::open(state_dir)?;
    let cid = registry.allocate(instance, &epoch)?;
    match provision_epoch_via_console(&registry, cid).await {
        Ok(_) => {}
        Err(e) => eprintln!(
            "WARNING: ssh shim for instance '{instance}': {e} (continuing host-side setup; \
             guest checks stay fail-closed until the epoch is re-provisioned)"
        ),
    }
    let socket_path = broker_socket_path(state_dir);
    let transport =
        UnixSocketTransport::bind(&socket_path, FixedCidResolver(cid)).map_err(|e| {
            anyhow::anyhow!(
                "ssh shim: socket bind failed for {}: {e}",
                socket_path.display()
            )
        })?;
    let grants = GrantStore::compile(&[(instance, credentials)]);
    let audit = Arc::new(AuditLog::with_state_dir(state_dir));
    let stop = Arc::new(AtomicBool::new(false));
    // Custody-first relay: broker-bound sessions ride the broker VM socket
    // (fail-closed while it is absent), guest-bound sessions relay direct.
    let broker_socket = broker_vm_socket_path(state_dir);
    let join = std::thread::Builder::new()
        .name(format!("ssh-shim-{instance}"))
        .spawn({
            let state_dir = state_dir.to_path_buf();
            let instance = instance.to_string();
            let loop_stop = Arc::clone(&stop);
            let loop_audit = Arc::clone(&audit);
            move || {
                let loop_registry = match CidRegistry::open(&state_dir) {
                    Ok(loop_registry) => loop_registry,
                    Err(e) => {
                        eprintln!(
                            "WARNING: ssh shim for instance '{instance}': registry open failed: {e}"
                        );
                        return;
                    }
                };
                let relay = BrokerFirstRelay::new(broker_socket, grants.clone());
                run_divert_until(
                    &transport,
                    &loop_registry,
                    &grants,
                    &loop_audit,
                    relay,
                    &loop_stop,
                );
            }
        })
        .map_err(|e| anyhow::anyhow!("ssh shim: divert thread spawn failed: {e}"))?;
    Ok(Some(SshShimHandle {
        socket_path,
        transport_cid: u64::from(cid),
        instance: instance.to_string(),
        cid,
        state_dir: state_dir.to_path_buf(),
        stop,
        join: Some(join),
    }))
}

/// Resolve the broker socket path the sandbox builder must dial for SSH
/// divert, or `None` when the workload carries no SSH grants.
///
/// Pure derivation: the path is a fixed function of the state dir, so the
/// pre-create builder input and the post-registration bind agree without
/// binding anything early. Strict-only confinement resolves to `None` — its
/// policy needs no listener because nothing can divert to it.
pub fn ssh_broker_socket_for_plan(
    state_dir: &Path,
    credentials: Option<&CredentialsPlan>,
) -> Option<PathBuf> {
    match credentials {
        Some(credentials) if !credentials.ssh.is_empty() => Some(broker_socket_path(state_dir)),
        _ => None,
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::microsandbox::plan::CredentialBinding;
    use std::io::{Read, Write};

    fn ssh_credentials() -> CredentialsPlan {
        CredentialsPlan {
            ssh: vec![crate::microsandbox::plan::SshGrantPlan {
                name: "deploy".to_string(),
                material: "DEPLOY_KEY".to_string(),
                hosts: vec!["github.com".to_string()],
                users: vec!["git".to_string()],
                ports: vec![22],
                binding: CredentialBinding::Broker,
                on_violation: crate::config::SecretViolationPolicy::Passthrough,
            }],
            signing: Vec::new(),
            strict: false,
            strict_origin: None,
        }
    }

    #[tokio::test]
    async fn reprovision_unknown_cid_fails_closed_without_dialing() {
        use crate::microsandbox::broker::epoch_provision::EpochProvisionError;
        let dir = crate::config::test_support::unique_state_dir("ssh-reprov-unknown");
        // No binding exists: typed rejection before any console dial (there
        // is no sandbox here at all — this must not hang or dial).
        let err = reprovision_epoch(&dir, 4242).await.unwrap_err();
        assert_eq!(err, EpochProvisionError::NotBound { cid: 4242 });
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn ensure_returns_none_without_ssh_grants() {
        let dir = crate::config::test_support::unique_state_dir("ssh-shim-none");
        let strict_only = CredentialsPlan {
            ssh: Vec::new(),
            signing: Vec::new(),
            strict: true,
            strict_origin: None,
        };
        assert!(
            ensure_ssh_shim(&dir, "personal-pi", &strict_only)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            !broker_socket_path(&dir).exists(),
            "strict-only workloads bind no listener"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn broker_socket_resolves_for_grant_plans_only() {
        let dir = crate::config::test_support::unique_state_dir("ssh-broker-socket");
        let credentials = ssh_credentials();
        // Grants resolve to the same host-side path the shim binds: the
        // pre-create builder input and the post-registration bind agree.
        let socket = ssh_broker_socket_for_plan(&dir, Some(&credentials))
            .expect("grants resolve a dial path");
        assert_eq!(socket, broker_socket_path(&dir));
        assert!(
            socket.is_absolute(),
            "the dial path must be absolute for the builder endpoint"
        );
        // Strict-only confinement takes no listener, so it resolves none.
        let strict_only = CredentialsPlan {
            ssh: Vec::new(),
            signing: Vec::new(),
            strict: true,
            strict_origin: None,
        };
        assert_eq!(ssh_broker_socket_for_plan(&dir, Some(&strict_only)), None);
        // Absent policy resolves none (fail-closed unchanged).
        assert_eq!(ssh_broker_socket_for_plan(&dir, None), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn ensure_binds_socket_and_registry_then_shutdown_releases() {
        let dir = crate::config::test_support::unique_state_dir("ssh-shim-life");
        let credentials = ssh_credentials();
        let handle = ensure_ssh_shim(&dir, "personal-pi", &credentials)
            .await
            .unwrap()
            .expect("grants bind a shim");
        assert_eq!(handle.socket_path, broker_socket_path(&dir));
        assert!(handle.socket_path.exists(), "listener socket file exists");
        assert_eq!(handle.transport_cid, u64::from(handle.cid));
        // The registry lookup resolves the allocated CID to the instance.
        let registry = CidRegistry::open(&dir).unwrap();
        let entry = registry
            .lookup(handle.cid)
            .unwrap()
            .expect("CID must be bound");
        assert_eq!(entry.instance, "personal-pi");
        // The overlay helper carries the same workload's SSH policy.
        let builder = crate::microsandbox::broker::apply_ssh_policy(
            microsandbox::Sandbox::builder("ssh-shim-life"),
            Some(&credentials),
        );
        let ssh = builder
            .spec()
            .network
            .ssh
            .clone()
            .expect("overlay must carry the ssh policy");
        assert_eq!(ssh.grants.len(), 1);
        assert!(!ssh.strict);
        // Shutdown removes the socket and expires the binding.
        let cid = handle.cid;
        handle.shutdown();
        assert!(
            !broker_socket_path(&dir).exists(),
            "shutdown removes the socket"
        );
        let registry = CidRegistry::open(&dir).unwrap();
        assert!(
            registry.lookup(cid).unwrap().is_none(),
            "shutdown expires the binding"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn ensure_provision_failure_does_not_fail_launch() {
        // No sandbox agent answers in the test env, so the console
        // provision always fails here — the launch setup must still stand
        // (warn-and-continue; enforcement stays fail-closed).
        let dir = crate::config::test_support::unique_state_dir("ssh-shim-nocons");
        let credentials = ssh_credentials();
        let handle = ensure_ssh_shim(&dir, "personal-pi", &credentials)
            .await
            .unwrap()
            .expect("grants bind a shim even when the console is unreachable");
        assert!(handle.socket_path.exists(), "listener still binds");
        let registry = CidRegistry::open(&dir).unwrap();
        assert!(
            registry.lookup(handle.cid).unwrap().is_some(),
            "CID still binds"
        );
        let cid = handle.cid;
        handle.shutdown();
        assert!(
            CidRegistry::open(&dir)
                .unwrap()
                .lookup(cid)
                .unwrap()
                .is_none()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn ensure_recovers_from_stale_socket_file() {
        // A leftover regular file at the socket path (unclean shutdown)
        // must not wedge the next launch: bind unlinks before listening.
        // A stale path is a dead route (bind failure is silent-skip in the
        // guest muxer), so recovery here is load-bearing, not cosmetic.
        let dir = crate::config::test_support::unique_state_dir("ssh-shim-stale");
        let socket_path = broker_socket_path(&dir);
        if let Some(parent) = socket_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&socket_path, b"stale").unwrap();
        let credentials = ssh_credentials();
        let handle = ensure_ssh_shim(&dir, "personal-pi", &credentials)
            .await
            .unwrap()
            .expect("stale socket file must not wedge the bind");
        assert_eq!(handle.socket_path, socket_path);
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

    #[tokio::test]
    async fn shutdown_without_socket_file_is_quiet() {
        // Shutdown when the socket file is already gone (crashed listener,
        // double teardown): no panic, and the CID binding still expires.
        let dir = crate::config::test_support::unique_state_dir("ssh-shim-nosock");
        let credentials = ssh_credentials();
        let handle = ensure_ssh_shim(&dir, "personal-pi", &credentials)
            .await
            .unwrap()
            .expect("grants bind a shim");
        let cid = handle.cid;
        std::fs::remove_file(&handle.socket_path).unwrap();
        handle.shutdown();
        let registry = CidRegistry::open(&dir).unwrap();
        assert!(
            registry.lookup(cid).unwrap().is_none(),
            "shutdown expires the binding even with the socket already gone"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// SSH grants covering one loopback TCP port (the upstream stub below).
    /// Guest-bound: the key lives in the guest, so the divert thread relays
    /// these sessions direct to the granted upstream.
    fn loopback_credentials(port: u16) -> CredentialsPlan {
        CredentialsPlan {
            ssh: vec![crate::microsandbox::plan::SshGrantPlan {
                name: "stub".to_string(),
                material: "TEST_KEY".to_string(),
                hosts: vec!["127.0.0.1".to_string()],
                users: vec!["git".to_string()],
                ports: vec![port],
                binding: CredentialBinding::Guest,
                on_violation: crate::config::SecretViolationPolicy::Passthrough,
            }],
            signing: Vec::new(),
            strict: false,
            strict_origin: None,
        }
    }

    /// Broker-bound grants covering one loopback TCP port. The key lives in
    /// broker custody, so these sessions must ride the broker VM socket.
    fn broker_bound_credentials(port: u16) -> CredentialsPlan {
        CredentialsPlan {
            ssh: vec![crate::microsandbox::plan::SshGrantPlan {
                name: "stub".to_string(),
                material: "TEST_KEY".to_string(),
                hosts: vec!["127.0.0.1".to_string()],
                users: vec!["git".to_string()],
                ports: vec![port],
                binding: CredentialBinding::Broker,
                on_violation: crate::config::SecretViolationPolicy::Passthrough,
            }],
            signing: Vec::new(),
            strict: false,
            strict_origin: None,
        }
    }

    /// The divert thread the launch binds hands allowed guest-bound sessions
    /// to the direct TCP relay: a full guest→shim→stub round trip through
    /// [`ensure_ssh_shim`], ending with exactly one persisted allow record.
    /// Broker-bound sessions never take this path (they ride the broker VM
    /// socket instead — see below).
    #[tokio::test]
    async fn ensure_divert_thread_relays_allowed_sessions_to_the_granted_upstream() {
        use microsandbox_network::ssh::gateway::{SshDivertPrelude, encode_ssh_divert_prelude};

        // The granted upstream: a local TCP stub speaking fixed banners.
        // Nonblocking accept with a deadline so a never-dialing relay
        // fails the test instead of hanging it.
        let stub = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let stub_port = stub.local_addr().unwrap().port();
        stub.set_nonblocking(true).unwrap();
        let stub_worker = std::thread::spawn(move || {
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
            let (mut conn, _) = loop {
                match stub.accept() {
                    Ok(v) => break v,
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if std::time::Instant::now() > deadline {
                            panic!("upstream stub accepted nothing: the relay never dialed");
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(e) => panic!("upstream stub accept failed: {e}"),
                }
            };
            conn.set_read_timeout(Some(std::time::Duration::from_secs(15)))
                .unwrap();
            let mut banner = [0u8; 9];
            conn.read_exact(&mut banner).unwrap();
            assert_eq!(&banner, b"SSH-GUEST");
            conn.write_all(b"SSH-STUB!").unwrap();
            let mut rest = Vec::new();
            conn.read_to_end(&mut rest).unwrap();
            assert!(rest.is_empty(), "guest must send nothing after its banner");
        });

        let dir = crate::config::test_support::unique_state_dir("ssh-relay-wire");
        let credentials = loopback_credentials(stub_port);
        let handle = ensure_ssh_shim(&dir, "personal-pi", &credentials)
            .await
            .unwrap()
            .expect("grants bind a shim");
        let socket_path = handle.socket_path.clone();
        let cid = handle.cid;

        // Guest side: connect to the shim socket and divert to the stub.
        let mut guest = std::os::unix::net::UnixStream::connect(&socket_path).unwrap();
        guest
            .set_read_timeout(Some(std::time::Duration::from_secs(15)))
            .unwrap();
        let prelude = SshDivertPrelude {
            dest_host: "127.0.0.1".to_string(),
            dest_port: stub_port,
            transport_cid: u64::from(cid),
            epoch: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        guest
            .write_all(&encode_ssh_divert_prelude(&prelude))
            .unwrap();
        // Post-prelude bytes reach the upstream through the relay, and the
        // upstream answer flows back: the second banner of a diverted session.
        guest.write_all(b"SSH-GUEST").unwrap();
        let mut answer = [0u8; 9];
        guest.read_exact(&mut answer).unwrap();
        assert_eq!(&answer, b"SSH-STUB!");
        // The guest half-close propagates to the stub; the stub close
        // surfaces here as EOF.
        guest.shutdown(std::net::Shutdown::Write).unwrap();
        let mut tail = Vec::new();
        guest.read_to_end(&mut tail).unwrap();
        assert!(tail.is_empty(), "upstream close must surface as guest EOF");
        stub_worker.join().unwrap();

        handle.shutdown();

        // Relay establishment audits exactly one allow record: the handoff
        // from decision to relay is observable in the persisted log.
        let log =
            std::fs::read_to_string(crate::microsandbox::broker::audit::audit_file_path(&dir))
                .unwrap();
        let records: Vec<serde_json::Value> = log
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            records.len(),
            1,
            "one divert decision per session: {records:?}"
        );
        assert_eq!(records[0]["instance"], "personal-pi");
        assert_eq!(
            records[0]["key_id"],
            format!("127.0.0.1:{stub_port}").as_str()
        );
        assert_eq!(records[0]["scheme"], "ssh-divert");
        assert_eq!(records[0]["namespace"], "divert");
        assert_eq!(records[0]["result"], "allow");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Broker-bound sessions fail closed while no broker VM serves its
    /// socket: the decision still allows (exactly one persisted allow
    /// record), but the custody relay cannot dial and the guest observes
    /// EOF — never a direct-TCP dial to the granted upstream.
    #[tokio::test]
    async fn ensure_divert_thread_fails_broker_bound_sessions_closed_without_broker() {
        use microsandbox_network::ssh::gateway::{SshDivertPrelude, encode_ssh_divert_prelude};

        // No TCP stub listens here on purpose: any direct dial would refuse
        // loudly, but the custody path must not dial at all — the guest
        // must see EOF from the dropped relay stream.
        let closed = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = closed.local_addr().unwrap().port();
        drop(closed);

        let dir = crate::config::test_support::unique_state_dir("ssh-relay-nobroker");
        // No broker VM socket exists under this state dir.
        assert!(
            !crate::microsandbox::broker::registry::broker_vm_socket_path(&dir).exists(),
            "the test needs no broker VM running"
        );
        let credentials = broker_bound_credentials(port);
        let handle = ensure_ssh_shim(&dir, "personal-pi", &credentials)
            .await
            .unwrap()
            .expect("grants bind a shim");
        let socket_path = handle.socket_path.clone();
        let cid = handle.cid;

        let mut guest = std::os::unix::net::UnixStream::connect(&socket_path).unwrap();
        guest
            .set_read_timeout(Some(std::time::Duration::from_secs(20)))
            .unwrap();
        let prelude = SshDivertPrelude {
            dest_host: "127.0.0.1".to_string(),
            dest_port: port,
            transport_cid: u64::from(cid),
            epoch: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };
        guest
            .write_all(&encode_ssh_divert_prelude(&prelude))
            .unwrap();
        // The custody relay drops the session: EOF, not upstream bytes.
        let mut tail = Vec::new();
        guest.read_to_end(&mut tail).unwrap();
        assert!(
            tail.is_empty(),
            "broker-bound session without a broker must close"
        );

        // Give the worker a moment to finish its audit write, then shut down.
        std::thread::sleep(std::time::Duration::from_millis(200));
        handle.shutdown();

        let log =
            std::fs::read_to_string(crate::microsandbox::broker::audit::audit_file_path(&dir))
                .unwrap();
        let records: Vec<serde_json::Value> = log
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(
            records.len(),
            1,
            "one divert decision per session: {records:?}"
        );
        assert_eq!(records[0]["result"], "allow");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
