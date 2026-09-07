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
//! - [`key_material`]: sealed SOPS-backed Ed25519 custody + the real SSHSIG
//!   [`key_material::SealedKeyBackend`] behind the same trait.
//! - [`shim`]: host-side unix-socket listener + transport trait.
//! - [`audit`]: append-only audit records (digests, never payloads).
//!
//! Identity model (normative): the transport-supplied CID is AUTHORITATIVE.
//! A body-claimed instance is always overridden by the registry lookup —
//! never trusted. The epoch token (provisioned per sandbox launch) defeats
//! CID-reuse-after-restart and snapshot-fork clones.

pub mod audit;
pub mod epoch;
pub mod key_material;
pub mod registry;
pub mod shim;
pub mod signing;

pub use audit::{AuditLog, AuditRecord, AuditResult, payload_digest_hex};
pub use epoch::{EpochError, EpochToken};
pub use key_material::{
    KeyMaterialError, SealedKeyBackend, SopsKeyMaterial, SshSigVerifyError, verify_sshsig,
};
pub use registry::{CidEntry, CidRegistry, GRANT_CACHE_TTL_SECS};
pub use shim::{BrokerShim, BrokerTransport, DispatchOutcome, TransportPeer, WireEnvelope};
pub use signing::{
    Denial, GrantStore, KeyBackend, LimitsConfig, SignRequest, SignResponse, SignatureScheme,
    SigningService, TestBackend,
};
