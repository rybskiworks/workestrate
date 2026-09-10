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
//! - [`epoch_provision`]: console sender for the per-CID wire epoch
//!   (generation-8 provision/ack over the sandbox's agent channel).
//! - [`registry`]: CID→instance mapping, persisted under the state dir.
//! - [`signing`]: grant-enforcement pipeline + [`signing::KeyBackend`].
//! - [`key_material`]: sealed SOPS-backed Ed25519 custody + the real SSHSIG
//!   [`key_material::SealedKeyBackend`] behind the same trait.
//! - [`shim`]: host-side unix-socket listener + transport trait.
//! - [`forwarder`]: host-side TCP forwarder for broker VM egress.
//! - [`broker_vm`]: shared broker VM lifecycle (spawn once per host).
//! - [`ssh_emit`]: credential-plan → guest SSH policy compilation.
//! - [`ssh_lifecycle`]: per-launch SSH shim setup and teardown.
//! - [`audit`]: append-only audit records (digests, never payloads).
//!
//! Identity model (normative): the transport-supplied CID is AUTHORITATIVE.
//! A body-claimed instance is always overridden by the registry lookup —
//! never trusted. The epoch token (provisioned per sandbox launch) defeats
//! CID-reuse-after-restart and snapshot-fork clones.

pub mod audit;
pub mod broker_vm;
pub mod epoch;
pub mod epoch_provision;
pub mod forwarder;
mod frame_io;
pub mod key_material;
mod receipt_clock;
pub mod registry;
pub mod shim;
pub mod signing;
pub mod ssh_emit;
pub mod ssh_lifecycle;
pub mod ssh_patterns;

pub use audit::{AuditLog, AuditRecord, AuditResult, payload_digest_hex};
pub use broker_vm::{BrokerVmHandle, broker_has_broker_bound_grants, ensure_broker_vm};
pub use epoch::{EpochError, EpochToken};
pub use epoch_provision::{
    CONSOLE_CONNECT_TIMEOUT_SECS, EPOCH_INTRO_GENERATION, EpochProvisionError,
    MAX_PROVISION_CLOCK_SKEW_SECS, SshEpochAck, SshEpochProvision,
};
pub use forwarder::{
    EGRESS_CONNECT_TIMEOUT_SECS, EgressAck, EgressConnect, EgressForwarderHandle,
    ensure_egress_forwarder, run_egress_until, serve_egress_once,
};
pub use key_material::{
    KeyMaterialError, SealedKeyBackend, SopsKeyMaterial, SshSigVerifyError, verify_sshsig,
};
pub use registry::{
    CidEntry, CidRegistry, GRANT_CACHE_TTL_SECS, broker_egress_socket_path, broker_socket_path,
    broker_vm_socket_path,
};
pub use shim::{
    BrokerFirstRelay, BrokerShim, BrokerSocketRelay, BrokerTransport, DispatchOutcome,
    DivertDecision, DivertDestination, EchoRelay, MAX_DIVERT_EPOCH_SKEW_SECS, SshRelay,
    TCP_UPSTREAM_CONNECT_TIMEOUT_SECS, TcpUpstreamRelay, TransportPeer, WireEnvelope,
    decide_divert, run_divert_until, serve_divert_once,
};
pub use signing::{
    Denial, GrantStore, KeyBackend, LimitsConfig, SignRequest, SignResponse, SignatureScheme,
    SigningService, TestBackend,
};
pub use ssh_emit::{apply_ssh_policy, ssh_config_for_plan, ssh_overlay_patch};
pub use ssh_lifecycle::{
    SshShimHandle, ensure_ssh_shim, provision_epoch_via_console, reprovision_epoch,
    ssh_broker_socket_for_plan,
};
pub use ssh_patterns::{
    DlpExclusion, DlpExclusionReason, apply_ssh_patterns, compile_ssh_patterns, policy_to_action,
};
