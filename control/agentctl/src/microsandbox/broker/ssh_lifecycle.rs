//! SSH shim lifecycle: per-launch listener setup and teardown.
//!
//! When a workload emits an SSH policy with at least one grant,
//! [`ensure_ssh_shim`] allocates the sandbox's transport CID, binds the
//! broker socket, and serves divert decisions on a background thread until
//! the foreground service ends. Strict-only confinement emits a policy but
//! binds no listener — nothing can divert to it, so it takes no handle.

use crate::microsandbox::broker::audit::AuditLog;
use crate::microsandbox::broker::epoch::EpochToken;
use crate::microsandbox::broker::registry::{CidRegistry, broker_socket_path};
use crate::microsandbox::broker::shim::{
    DivertDestination, FixedCidResolver, SshRelay, UnixSocketTransport, run_divert_until,
};
use crate::microsandbox::broker::signing::GrantStore;
use crate::microsandbox::plan::CredentialsPlan;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Epoch delivery failure: the console agent handshake that carries the
/// launch epoch to the guest is not wired yet, so delivery always fails
/// until that channel lands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpochProvisionError {
    /// The console handshake channel is not wired: no epoch material
    /// crosses to the guest by any other path (fail-closed — guests
    /// simply cannot present an epoch until the handshake lands).
    NotWired { instance: String, cid: u32 },
}

impl std::fmt::Display for EpochProvisionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EpochProvisionError::NotWired { instance, cid } => write!(
                f,
                "epoch delivery for instance '{instance}' (CID {cid}) is not wired: \
                 the console agent handshake does not exist yet"
            ),
        }
    }
}

impl std::error::Error for EpochProvisionError {}

/// Deliver the launch epoch to the guest over the console agent
/// handshake. That channel does not exist yet and is gated on KVM, so
/// this always returns [`EpochProvisionError::NotWired`]: epoch material
/// never crosses to the guest by any other path.
pub fn provision_epoch_via_console(
    instance: &str,
    cid: u32,
    _epoch: &EpochToken,
) -> Result<(), EpochProvisionError> {
    Err(EpochProvisionError::NotWired {
        instance: instance.to_string(),
        cid,
    })
}

/// Fail-closed placeholder relay: divert decisions are enforced and
/// audited, but no production upstream path exists yet, so allowed
/// sessions close instead of flowing anywhere. The in-VM relay replaces
/// this behind the [`SshRelay`] trait without touching the loop.
#[derive(Debug, Default)]
struct FailClosedRelay;

impl SshRelay for FailClosedRelay {
    fn relay(
        &self,
        stream: std::os::unix::net::UnixStream,
        dest: &DivertDestination,
    ) -> std::io::Result<()> {
        eprintln!(
            "WARNING: ssh divert to {}:{} for instance '{}' closed: no upstream relay is wired",
            dest.dest_host, dest.dest_port, dest.instance
        );
        drop(stream);
        Ok(())
    }
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
/// The epoch handshake is not wired yet, so delivery is noticed and
/// skipped: the registry bind and listener still stand, keeping the
/// shim-side checks live while guest-initiated requests fail closed.
pub fn ensure_ssh_shim(
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
    match provision_epoch_via_console(instance, cid, &epoch) {
        Ok(()) => {}
        Err(e) => eprintln!(
            "WARNING: ssh shim for instance '{instance}': {e} (continuing host-side setup)"
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
                let relay = FailClosedRelay;
                run_divert_until(
                    &transport,
                    &loop_registry,
                    &grants,
                    &loop_audit,
                    &relay,
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::microsandbox::broker::epoch::EPOCH_BYTES;
    use crate::microsandbox::plan::CredentialBinding;

    fn ssh_credentials() -> CredentialsPlan {
        CredentialsPlan {
            ssh: vec![crate::microsandbox::plan::SshGrantPlan {
                name: "deploy".to_string(),
                material: "DEPLOY_KEY".to_string(),
                hosts: vec!["github.com".to_string()],
                users: vec!["git".to_string()],
                ports: vec![22],
                binding: CredentialBinding::Broker,
            }],
            signing: Vec::new(),
            strict: false,
            strict_origin: None,
        }
    }

    #[test]
    fn provision_epoch_is_not_wired() {
        let epoch = EpochToken::from_bytes([1u8; EPOCH_BYTES]);
        let err = provision_epoch_via_console("personal-pi", 7, &epoch).unwrap_err();
        assert_eq!(
            err,
            EpochProvisionError::NotWired {
                instance: "personal-pi".to_string(),
                cid: 7,
            }
        );
    }

    #[test]
    fn ensure_returns_none_without_ssh_grants() {
        let dir = crate::config::test_support::unique_state_dir("ssh-shim-none");
        let strict_only = CredentialsPlan {
            ssh: Vec::new(),
            signing: Vec::new(),
            strict: true,
            strict_origin: None,
        };
        assert!(
            ensure_ssh_shim(&dir, "personal-pi", &strict_only)
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
    fn ensure_binds_socket_and_registry_then_shutdown_releases() {
        let dir = crate::config::test_support::unique_state_dir("ssh-shim-life");
        let credentials = ssh_credentials();
        let handle = ensure_ssh_shim(&dir, "personal-pi", &credentials)
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
}
