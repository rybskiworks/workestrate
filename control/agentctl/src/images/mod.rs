//! Image build/load lifecycle state (spec 21 —
//! `docs/validation-and-improvements/06-improvements/21-image-build-lifecycle.md`).
//!
//! Phase B landed the library-only state layer (`state.rs`, `lock.rs`,
//! `repo_key.rs`, `skew.rs` — see the phase-B notes below). Phase C wired it
//! to the CLI: `build_cmd.rs` implements `workestrate workload build`
//! (selectors, the lock → probe → eval → skew → act flow, `--check`, JSON),
//! and `detect.rs` carries the two change-detection seams ([`detect::DrvEvaluator`]
//! — the §3.1 eval-only drvPath signal — and [`detect::StoreProbe`] — msb
//! store-tag presence with the §7 unreachable vocabulary). Phase D landed the
//! build/load pipeline (`pipeline.rs`): nix build (stderr teed to the TTY +
//! retained for §7 classification) → outPath re-load gate (spec §3.1, pure
//! [`pipeline::reload_decision`]) → `gunzip -c` | `msb load -t <tag>` →
//! post-load store verification → record upsert, all inside the caller's
//! still-held per-tag lock (spec §3.3), with the `ImageBuilder`/`ImageLoader`
//! trait seams and cfg(test) fakes matching the detect.rs pattern. Phase E
//! landed the ensure-images pre-flight (`ensure.rs`): the parent-side wiring
//! of `workload up` / `exec` / bare-up to the phase-C/D flow, the
//! `images_ready` detach token (`InstanceSpec::images_ready` + the hidden
//! `--images-ready` flag `detach_args` appends unconditionally), and the
//! `--reload-images` force on the lifecycle verbs (USER DECISION D3:
//! batch-scoped, never forwarded to the detached child).
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
//!    ([`detect::StoreProbe`]) — keyed, under A2 (ADR 0032 §Image tags), by
//!    the COMPUTED content-addressed tag from the eval-only outPath eval
//!    ([`detect::DrvEvaluator`]);
//! 4. on rebuild/trust, upsert the record AND the current-pointer
//!    (A2: [`state::PointerRecord`] — the mutable per-context resolution
//!    target replacing any mutable registry tag) and save
//!    `state/images.json` (`state.rs`) INSIDE the lock (spec §3.3
//!    atomicity).
//!
//! A2 stage 2 (ADR 0032 §Image tags — RESOLVED user decision 3) landed the
//! keep-last-N GC cascade (`gc.rs`): the capsule < repo-entry < settings <
//! default N resolution, prune-on-load inside `process_target`'s held lock,
//! the `workestrate images gc` manual sweep, running-sandbox protection via
//! the port-registry `image_tag` field, and the `ImageRemover` seam
//! (`pipeline.rs`).
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
pub mod ensure;
pub mod gc;
pub mod lock;
pub mod pipeline;
pub mod repo_key;
pub mod skew;
pub mod state;
