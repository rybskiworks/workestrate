//! Credential-broker control plane.
//!
//! Placement: `microsandbox::broker` sits alongside `port_registry`,
//! `workload`, and `runtime` because the broker is a microsandbox-lifecycle
//! concern (per-sandbox identity + grant enforcement), not a top-level CLI
//! concern. The alternative (`crate::broker`) was rejected: every broker
//! input (plan grants, instance records, state dir) already lives under
//! `microsandbox`, and a future in-VM relay will need the same locality.
//!
//! Layout:
//! - [`epoch`]: per-launch epoch tokens (anti-replay / anti-fork).
//! - [`registry`]: CID→instance mapping, persisted under the state dir.
//! - [`signing`]: grant-enforcement pipeline + [`signing::KeyBackend`].
//! - [`shim`]: host-side unix-socket listener + transport trait.
//! - [`ssh_emit`]: credential-plan → guest SSH policy compilation.
//! - [`ssh_lifecycle`]: per-launch SSH shim setup and teardown.
//! - [`audit`]: append-only audit records (digests, never payloads).
//!
//! Identity model (normative): the transport-supplied CID is AUTHORITATIVE.
//! A body-claimed instance is always overridden by the registry lookup —
//! never trusted. The epoch token (provisioned per sandbox launch) defeats
//! CID-reuse-after-restart and snapshot-fork clones.

pub mod audit;
pub mod epoch;
pub mod registry;
pub mod shim;
pub mod signing;
pub mod ssh_emit;
pub mod ssh_lifecycle;

pub use audit::{AuditLog, AuditRecord, AuditResult, payload_digest_hex};
pub use epoch::{EpochError, EpochToken};
pub use registry::{CidEntry, CidRegistry, GRANT_CACHE_TTL_SECS, broker_socket_path};
pub use shim::{
    BrokerShim, BrokerTransport, DispatchOutcome, DivertDecision, DivertDestination, EchoRelay,
    MAX_DIVERT_EPOCH_SKEW_SECS, SshRelay, TransportPeer, WireEnvelope, decide_divert,
    run_divert_until, serve_divert_once,
};
pub use signing::{
    Denial, GrantStore, KeyBackend, LimitsConfig, SignRequest, SignResponse, SignatureScheme,
    SigningService, TestBackend,
};
pub use ssh_emit::{apply_ssh_policy, ssh_config_for_plan, ssh_overlay_patch};
pub use ssh_lifecycle::{
    EpochProvisionError, SshShimHandle, ensure_ssh_shim, provision_epoch_via_console,
};
