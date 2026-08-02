//! Image build/load lifecycle state (spec 21 —
//! `docs/validation-and-improvements/06-improvements/21-image-build-lifecycle.md`).
//!
//! Phase B landed the library-only state layer (`state.rs`, `lock.rs`,
//! `repo_key.rs`, `skew.rs` — see the phase-B notes below). Phase C wires it
//! to the CLI: `build_cmd.rs` implements `workestrate workload build`
//! (selectors, the lock → probe → eval → skew → act flow, `--check`, JSON),
//! `detect.rs` carries the two change-detection seams ([`detect::DrvEvaluator`]
//! — the §3.1 eval-only drvPath signal — and [`detect::StoreProbe`] — msb
//! store-tag presence with the §7 unreachable vocabulary), and `pipeline.rs`
//! is the **SPEC 21 PHASE D SEAM** (nix build → outPath re-load gate → msb
//! load → record upsert inside the still-held lock; a named refusal in
//! phase C). Phase E wires the ensure-images pre-flight into the lifecycle
//! verbs.
//!
//! Phase-C usage (spec §3/§5, implemented in `build_cmd::process_target`):
//!
//! 1. resolve the declaring layer's [`repo_key`] (`repo_key.rs`) and the
//!    record key via [`state::image_key`];
//! 2. acquire the per-tag [`lock::ImageTagLock`] (`lock.rs`), which spans the
//!    whole eval → build → load → record critical section (spec §3.3) —
//!    `--check` takes NO lock (read-only report);
//! 3. consult the [`skew`] matrix (`skew.rs`) against the record in
//!    [`state::ImagesState`] plus the msb store-tag presence
//!    ([`detect::StoreProbe`]) and the drvPath eval ([`detect::DrvEvaluator`]);
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

pub mod build_cmd;
pub mod detect;
pub mod lock;
pub mod pipeline;
pub mod repo_key;
pub mod skew;
pub mod state;
