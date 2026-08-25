//! Provenance stamps (ADR 0032 §Provenance stamps): content hashes over the
//! RUNTIME-RELEVANT build inputs of a sandbox plan, recorded on registry
//! records so staleness is visible (`ps`) and dispositions can react
//! (ADR 0030 V-addendum §V4 `on_skew`).
//!
//! This module currently hosts the shared hand-rolled FNV-1a 64-bit hash
//! (the per-dir shorthash precedent, ADR 0030 V-addendum §V1 — no new crate
//! dependency, nix vendor surface unchanged). The config-hash canonicalizer
//! lands with the A3 stamp wiring.
//!
//! NOTE: `crate::microsandbox::slots` carries its own private
//! `fnv1a64_hex8` (first-8-hex truncation for instance ids). It predates
//! this module and renders a DIFFERENT output shape; unifying it here would
//! churn a landed module for no behavioral gain, so both exist and any NEW
//! caller should use [`fnv1a64`].

/// FNV-1a 64-bit hash of `bytes` (hand-rolled — see the module doc).
pub fn fnv1a64(data: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET_BASIS;
    for &b in data {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    /// Reference implementation copied from the FNV-1a pseudocode (the same
    /// cross-check shape the slots.rs per-dir shorthash test uses).
    fn fnv_reference(bytes: &[u8]) -> u64 {
        let mut hash: u64 = 0xcbf29ce484222325;
        for &b in bytes {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(0x100000001b3);
        }
        hash
    }

    #[test]
    fn fnv1a64_matches_reference_implementation() {
        for input in [
            "".as_bytes(),
            "a".as_bytes(),
            "/home/node/work".as_bytes(),
            "/da/ta".as_bytes(),
            "/da_ta".as_bytes(),
            "img-pi:personal:aaaaaaaaaaaa".as_bytes(),
        ] {
            assert_eq!(
                fnv1a64(input),
                fnv_reference(input),
                "fnv1a64 drifted from the FNV-1a-64 reference for {input:?}"
            );
        }
    }

    #[test]
    fn fnv1a64_known_vectors() {
        // Standard FNV-1a 64-bit test vectors.
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn fnv1a64_is_deterministic_and_input_sensitive() {
        let a = fnv1a64(b"/da/ta");
        assert_eq!(a, fnv1a64(b"/da/ta"), "same input → same hash");
        assert_ne!(a, fnv1a64(b"/da_ta"), "distinct inputs → distinct hashes");
    }
}
