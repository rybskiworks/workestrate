//! Build/load pipeline (spec 21 phase D) — SEAM ONLY in phase C.
//!
//! ==== SPEC 21 PHASE D SEAM ====
//!
//! Phase D implements the real pipeline here, in this exact slot, taking the
//! fully-resolved [`BuildJob`] (everything the pipeline needs, assembled by
//! the phase-C build verb inside the still-held per-tag lock):
//!
//! 1. `nix build <repo.flake_root>#<attr>` — realizes the drv recorded in
//!    `drv_path`; the **outPath re-load gate** (spec §3.1): re-load into msb
//!    only when the realized outPath differs from the recorded one, so eval
//!    churn that resolves to an already-realized outPath costs no `msb load`.
//! 2. `msb load` of the resulting tarball — under the STILL-HELD per-tag
//!    [`crate::images::lock::ImageTagLock`] (spec §3.3: the lock spans the
//!    whole eval → build → load → record critical section; msb's internal
//!    per-ref/per-layer locks compose below it — §11 HOST-VERIFY item 3).
//! 3. Record upsert (`drv_path`, the realized `out_path`, `digest` when the
//!    msb digest surface lands — spec §3.5/§11) + `ImagesState::save` INSIDE
//!    the lock (spec §3.3 atomicity).
//!
//! Phase C callers: `build_cmd` routes [`SkewDecision::Build`] /
//! [`SkewDecision::Rebuild`] / [`SkewDecision::RebuildForced`] here in build
//! mode (fail-fast on the error). `--check` NEVER reaches this seam — it
//! reports the structured "would build" decision instead.

use anyhow::Result;

use crate::images::state::RepoIdentity;

/// Everything the phase-D pipeline needs, assembled by the phase-C build
/// verb inside the per-tag lock.
#[derive(Debug, Clone)]
pub struct BuildJob {
    /// Workload name from the merged config.
    pub workload: String,
    /// Config-repo identity (name + path + flake_root) for the record.
    pub repo: RepoIdentity,
    /// Flake attribute that builds the image tarball (e.g. `workestrate-pi`).
    pub attr: String,
    /// Stable verbatim `name:tag` loaded into the msb store (USER DECISION D2).
    pub tag: String,
    /// The current drvPath eval (spec §3.1) — the pipeline realizes this drv.
    pub drv_path: String,
    /// The build verb's `--force` flag (RebuildForced verdicts).
    pub force: bool,
}

/// The phase-D pipeline entry point. Unimplemented in phase C: the error is
/// the honest, named refusal the build verb surfaces when change detection
/// concludes a build is required.
pub fn run_build_pipeline(job: &BuildJob) -> Result<()> {
    let _ = job;
    Err(anyhow::anyhow!(
        "build pipeline not yet implemented (spec 21 phase D)"
    ))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// The seam's phase-C refusal is named and stable (the build verb's error
    /// path asserts on it; phase D replaces the body, not the contract).
    #[test]
    fn seam_refusal_names_phase_d() {
        let job = BuildJob {
            workload: "pi".to_string(),
            repo: RepoIdentity {
                name: "personal".to_string(),
                path: PathBuf::from("/tmp/repo"),
                flake_root: PathBuf::from("/tmp/repo"),
            },
            attr: "workestrate-pi".to_string(),
            tag: "workestrate-pi:latest".to_string(),
            drv_path: "/nix/store/abc-workestrate-pi.tar.gz.drv".to_string(),
            force: false,
        };
        let err = run_build_pipeline(&job).expect_err("phase C: the seam must refuse");
        assert!(
            err.to_string()
                .contains("build pipeline not yet implemented (spec 21 phase D)"),
            "the refusal names the phase: {err}"
        );
    }
}
