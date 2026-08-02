//! Image build/load lifecycle state (spec 21 phase B —
//! `docs/validation-and-improvements/06-improvements/21-image-build-lifecycle.md`).
//!
//! This module is LIBRARY-ONLY in phase B: nothing here is wired to a CLI
//! verb yet — phases C (change detection + `workload build`), D (build/load
//! pipeline), and E (lifecycle pre-flight wiring) are the intended callers.
//! Every public item is exercised from in-module `#[cfg(test)]` tests only.
//!
//! Intended usage (spec §2/§5): the phase-C/D ensure-images pre-flight will
//!
//! 1. resolve the declaring layer's [`repo_key`] (`repo_key.rs`) and the
//!    record key via [`state::image_key`];
//! 2. acquire the per-tag [`lock::ImageTagLock`] (`lock.rs`), which spans the
//!    whole eval → build → load → record critical section (spec §3.3);
//! 3. consult the [`skew`] matrix (`skew.rs`) against the record in
//!    [`state::ImagesState`] plus the msb store-tag presence;
//! 4. on rebuild/trust, upsert the record and save `state/images.json`
//!    (`state.rs`) INSIDE the lock (spec §3.3 atomicity).
//!
//! Two phase-B decisions, recorded here because they diverge from first-cut
//! tasking (both spec-faithful):
//!
//! - **Key separator is `#`** — `<repo>#<tag>` (e.g.
//!   `personal#workestrate-pi:latest`) per spec §8, NOT `|`.
//! - **The per-tag lock is an O_EXCL lock file, NOT `flock(2)`.** The spec
//!   says "flock", but `[lints.rust] unsafe_code = "forbid"` crate-wide makes
//!   the unsafe `libc::flock` FFI uncallable without a lint change (which
//!   phase B does not make). `lock.rs` documents the trade; upgrading to
//!   kernel-release `flock(2)` is a follow-up decision for phases C/D.

pub mod lock;
pub mod repo_key;
pub mod skew;
pub mod state;
