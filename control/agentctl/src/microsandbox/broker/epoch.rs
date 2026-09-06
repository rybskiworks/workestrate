//! Epoch tokens: per-sandbox-launch unguessable handshake preludes.
//!
//! Each workload VM gets a fresh 256-bit token at launch, provisioned over
//! the existing console agent handshake (provisioning over that handshake
//! remains follow-up work; here is the token type + issue/verify core).
//! Every broker request must present
//! the current epoch for its CID as a vsock handshake prelude. A stale
//! (post-restart reuse) or forked (snapshot-clone) epoch is rejected with a
//! re-attestation signal, never silently accepted.
//!
//! Entropy source: [`getrandom`] — the platform secure-random source,
//! already in the dependency graph (promoted to a direct dep). No
//! new RNG dependency was introduced.

use serde::{Deserialize, Serialize};

/// Byte length of an epoch token (256 bits).
pub const EPOCH_BYTES: usize = 32;

/// An unguessable per-launch token binding (instance, CID) to one sandbox
/// generation. Serializes as a 32-byte array (CBOR byte-string-friendly via
/// `Vec<u8>` conversions at the wire boundary; JSON renders the number
/// array — tokens are never written to the audit log in any form).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EpochToken([u8; EPOCH_BYTES]);

impl EpochToken {
    /// Issue a fresh token for `(instance, cid)` from the platform
    /// secure-random source. The arguments are not embedded in the token —
    /// they document the binding the CALLER must persist (see
    /// [`crate::microsandbox::broker::registry::CidRegistry::bind`]); the
    /// token itself is opaque entropy.
    pub fn issue(_instance: &str, _cid: u32) -> anyhow::Result<Self> {
        let mut bytes = [0u8; EPOCH_BYTES];
        getrandom::fill(&mut bytes)
            .map_err(|e| anyhow::anyhow!("epoch issue: secure-random source failed: {e}"))?;
        Ok(Self(bytes))
    }

    /// Test/synthetic constructor from known bytes (deterministic fixtures).
    /// Production code must use [`Self::issue`].
    pub fn from_bytes(bytes: [u8; EPOCH_BYTES]) -> Self {
        Self(bytes)
    }

    /// Hex rendering (registry persistence + debug surfaces only — never
    /// the audit log).
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parse [`Self::to_hex`] output.
    pub fn from_hex(s: &str) -> anyhow::Result<Self> {
        let raw =
            hex::decode(s).map_err(|e| anyhow::anyhow!("epoch from_hex: invalid hex: {e}"))?;
        if raw.len() != EPOCH_BYTES {
            anyhow::bail!(
                "epoch from_hex: expected {EPOCH_BYTES} bytes, got {}",
                raw.len()
            );
        }
        let mut bytes = [0u8; EPOCH_BYTES];
        bytes.copy_from_slice(&raw);
        Ok(Self(bytes))
    }

    /// Verify a presented token against the expected token for `cid`.
    /// Comparison is constant-time over the token bytes (no early exit);
    /// the CID comparison is a plain equality (CIDs are not secret).
    pub fn verify(
        &self,
        cid: u32,
        expected: &EpochToken,
        expected_cid: u32,
    ) -> Result<(), EpochError> {
        if cid != expected_cid {
            return Err(EpochError::CidMismatch { re_attest: true });
        }
        if !constant_time_eq(&self.0, &expected.0) {
            return Err(EpochError::StaleEpoch { re_attest: true });
        }
        Ok(())
    }
}

/// Constant-time byte equality (accumulates the XOR diff — no early exit).
/// Hand-rolled over std so this change adds no `subtle`-family dependency for one
/// 32-byte comparison; the shape mirrors `subtle::ConstantTimeEq`.
fn constant_time_eq(a: &[u8; EPOCH_BYTES], b: &[u8; EPOCH_BYTES]) -> bool {
    let mut diff = 0u8;
    for i in 0..EPOCH_BYTES {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

/// Epoch verification failure. Every variant carries `re_attest: true` —
/// the guest's epoch is no longer valid for this CID and it must
/// re-attest over the console agent handshake before retrying.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EpochError {
    /// Presented epoch differs from the registry-bound epoch for this CID:
    /// restart-reuse or snapshot-fork. Re-attest.
    StaleEpoch { re_attest: bool },
    /// The CID itself is unknown or mismatched. Re-attest (a fresh launch
    /// binds a fresh CID).
    CidMismatch { re_attest: bool },
}

impl EpochError {
    /// Whether the guest should re-attest (always true by construction).
    pub fn re_attest(&self) -> bool {
        match self {
            EpochError::StaleEpoch { re_attest } | EpochError::CidMismatch { re_attest } => {
                *re_attest
            }
        }
    }
}

impl std::fmt::Display for EpochError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EpochError::StaleEpoch { .. } => {
                write!(
                    f,
                    "stale epoch for this CID (restart-reuse or fork); re-attestation required"
                )
            }
            EpochError::CidMismatch { .. } => {
                write!(f, "unknown or mismatched CID; re-attestation required")
            }
        }
    }
}

impl std::error::Error for EpochError {}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn issue_produces_unique_tokens() {
        let a = EpochToken::issue("pi", 7).unwrap();
        let b = EpochToken::issue("pi", 7).unwrap();
        assert_ne!(a, b, "two issued tokens must differ");
    }

    #[test]
    fn verify_accepts_matching_token_and_cid() {
        let tok = EpochToken::issue("pi", 7).unwrap();
        assert!(tok.verify(7, &tok, 7).is_ok());
    }

    #[test]
    fn verify_rejects_stale_token_with_re_attest() {
        let current = EpochToken::from_bytes([1u8; EPOCH_BYTES]);
        let presented = EpochToken::from_bytes([2u8; EPOCH_BYTES]);
        let err = presented.verify(7, &current, 7).unwrap_err();
        assert_eq!(err, EpochError::StaleEpoch { re_attest: true });
        assert!(err.re_attest(), "stale epoch must signal re-attestation");
    }

    #[test]
    fn verify_rejects_cid_mismatch_with_re_attest() {
        let tok = EpochToken::from_bytes([1u8; EPOCH_BYTES]);
        let err = tok.verify(8, &tok, 7).unwrap_err();
        assert_eq!(err, EpochError::CidMismatch { re_attest: true });
        assert!(err.re_attest());
    }

    #[test]
    fn hex_round_trip() {
        let tok = EpochToken::from_bytes([0xabu8; EPOCH_BYTES]);
        let back = EpochToken::from_hex(&tok.to_hex()).unwrap();
        assert_eq!(tok, back);
        assert!(EpochToken::from_hex("zz").is_err());
        assert!(EpochToken::from_hex("00").is_err(), "short input must fail");
    }
}
