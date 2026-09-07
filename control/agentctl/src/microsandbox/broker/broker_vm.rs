//! Shared broker VM lifecycle: ensure once per host, own in the foreground.
//!
//! When any workload carries broker-bound SSH credentials, the custody path
//! needs a broker VM: a network-isolated sandbox running the broker init,
//! holding the sealed key in custody and reoriginating diverted sessions
//! upstream. The VM is shared per host (one divert socket and one egress
//! socket per state dir), while the foreground config owns the host-side
//! handle so the service end tears down the shim, the broker reservation,
//! and the forwarder together.
//!
//! The actual VM boot (broker rootfs image, sealed key and upstream pins
//! through the sandbox builder, vsock routes) is KVM-gated and lands with
//! the KVM smoke run: this module owns the host-side sockets and the
//! egress forwarder, which are fully exercisable without KVM. Until the
//! boot lands, the broker divert socket is absent and the custody relay
//! fails broker-bound sessions closed.

use crate::microsandbox::broker::forwarder::{EgressForwarderHandle, ensure_egress_forwarder};
use crate::microsandbox::broker::registry::broker_vm_socket_path;
use crate::microsandbox::broker::signing::GrantStore;
use crate::microsandbox::plan::{CredentialBinding, CredentialsPlan};
use std::path::{Path, PathBuf};

/// Sandbox name reserved for the shared broker VM. Workload instances never
/// take this name: instance names are slot identities, and no slot is named
/// `broker`.
pub const BROKER_VM_NAME: &str = "broker";

/// Whether the workload needs the shared broker VM: at least one SSH grant
/// keeps its key material in broker custody (the default, secure binding).
/// Guest-bound-only plans need no broker — their sessions relay direct.
pub fn broker_has_broker_bound_grants(credentials: Option<&CredentialsPlan>) -> bool {
    credentials
        .map(|plan| {
            plan.ssh
                .iter()
                .any(|grant| grant.binding == CredentialBinding::Broker)
        })
        .unwrap_or(false)
}

/// Owns the host side of one shared broker VM reservation: the divert
/// socket path the custody relay dials and the egress forwarder answering
/// the broker's upstream tunnels. Release via [`BrokerVmHandle::shutdown`]
/// when the foreground service ends.
///
/// Sharing: the first reservation binds the egress listener and owns the
/// forwarder; a later reservation while it lives shares the sockets
/// (`forwarder` is `None`) so its shutdown never removes a live listener
/// out from under the owner. Best-effort throughout: failures are
/// stderr-loud, never propagated.
#[derive(Debug)]
pub struct BrokerVmHandle {
    /// Reserved broker VM sandbox name (the KVM boot claims it).
    pub vm_name: String,
    /// Host-side divert socket the custody relay dials (bound by the broker
    /// VM boot once it lands; absent until then).
    pub vm_socket_path: PathBuf,
    /// Egress forwarder when this reservation owns it (`None` when shared).
    pub forwarder: Option<EgressForwarderHandle>,
}

impl BrokerVmHandle {
    /// Tear down the reservation: stop the owned forwarder (shared
    /// reservations own none and only drop their path). The broker VM
    /// itself stops with the KVM boot's sandbox handle once it lands;
    /// until then there is no VM to stop.
    pub fn shutdown(self) {
        if let Some(forwarder) = self.forwarder {
            forwarder.shutdown();
        }
    }
}

/// Ensure the shared broker VM reservation for one workload launch, or
/// `None` when the workload carries no broker-bound SSH grants.
///
/// Idempotent per host: when the egress listener is already bound (an
/// earlier reservation owns it), the returned handle shares the sockets
/// without owning the forwarder. A failed workload create or registration
/// must never leave a live forwarder behind, so callers ensure this only
/// once the sandbox identity is durably registered — beside
/// [`crate::microsandbox::broker::ssh_lifecycle::ensure_ssh_shim`].
pub fn ensure_broker_vm(
    state_dir: &Path,
    instance: &str,
    credentials: &CredentialsPlan,
) -> anyhow::Result<Option<BrokerVmHandle>> {
    if !broker_has_broker_bound_grants(Some(credentials)) {
        return Ok(None);
    }
    let grants = GrantStore::compile(&[(instance, credentials)]);
    let forwarder = match ensure_egress_forwarder(state_dir, grants) {
        Ok(handle) => Some(handle),
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => None,
        Err(e) => {
            return Err(anyhow::anyhow!(
                "broker egress forwarder bind failed for instance '{instance}': {e}"
            ));
        }
    };
    if forwarder.is_none() {
        eprintln!("broker VM for instance '{instance}': sharing the live host reservation");
    }
    Ok(Some(BrokerVmHandle {
        vm_name: BROKER_VM_NAME.to_string(),
        vm_socket_path: broker_vm_socket_path(state_dir),
        forwarder,
    }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn broker_bound() -> CredentialsPlan {
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

    fn guest_bound() -> CredentialsPlan {
        let mut plan = broker_bound();
        plan.ssh[0].binding = CredentialBinding::Guest;
        plan
    }

    #[test]
    fn broker_reservation_triggers_on_broker_bound_grants_only() {
        assert!(broker_has_broker_bound_grants(Some(&broker_bound())));
        assert!(!broker_has_broker_bound_grants(Some(&guest_bound())));
        let strict_only = CredentialsPlan {
            ssh: Vec::new(),
            signing: Vec::new(),
            strict: true,
            strict_origin: None,
        };
        assert!(!broker_has_broker_bound_grants(Some(&strict_only)));
        assert!(!broker_has_broker_bound_grants(None));
    }

    #[test]
    fn ensure_returns_none_without_broker_bound_grants() {
        let dir = crate::config::test_support::unique_state_dir("broker-vm-none");
        assert!(
            ensure_broker_vm(&dir, "personal-pi", &guest_bound())
                .unwrap()
                .is_none()
        );
        assert!(
            !crate::microsandbox::broker::registry::broker_egress_socket_path(&dir).exists(),
            "grant-less plans bind no forwarder"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ensure_binds_forwarder_then_second_reservation_shares() {
        let dir = crate::config::test_support::unique_state_dir("broker-vm-shared");
        let credentials = broker_bound();
        let first = ensure_broker_vm(&dir, "personal-pi", &credentials)
            .unwrap()
            .expect("broker-bound binds a reservation");
        assert_eq!(first.vm_name, BROKER_VM_NAME);
        assert_eq!(
            first.vm_socket_path,
            crate::microsandbox::broker::registry::broker_vm_socket_path(&dir)
        );
        assert!(
            first.forwarder.is_some(),
            "first reservation owns the forwarder"
        );
        let second = ensure_broker_vm(&dir, "other-pi", &credentials)
            .unwrap()
            .expect("second workload shares the reservation");
        assert!(
            second.forwarder.is_none(),
            "shared reservation owns no forwarder"
        );
        // The shared shutdown removes nothing; the owner shutdown removes
        // the socket.
        let egress = crate::microsandbox::broker::registry::broker_egress_socket_path(&dir);
        second.shutdown();
        assert!(egress.exists(), "shared shutdown keeps the live listener");
        first.shutdown();
        assert!(!egress.exists(), "owner shutdown removes the socket");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
