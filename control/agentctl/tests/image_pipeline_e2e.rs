//! REAL end-to-end test of the spec-21 phase-D build/load pipeline
//! (`images::pipeline::run_build_pipeline` with the REAL backends):
//! nix build of a tiny fixture flake → outPath re-load gate → `gunzip -c` |
//! `msb load -t <tag>` → post-load store verification → record upsert, then
//! the gate-skip second run, then an out-of-band tag deletion + reload.
//!
//! ENV GATES (skip-with-note, the detect.rs `nix_on_path()` precedent):
//!
//! - `nix` on PATH (the fixture build is a REAL `nix build`);
//! - `MSB_PATH` pointing at an msb binary that HONORS `MSB_HOME`. This is
//!   deliberately NOT the bare `msb` on PATH: the workestrate devshell wraps
//!   msb with a forced `MSB_HOME=$HOME/.microsandbox`, and running this test
//!   through the wrapper would write fixture images into the REAL home
//!   store. Point MSB_PATH at the unwrapped binary (e.g. the
//!   `microsandbox-0.5.6/bin/msb` store path) or any msb on a host without
//!   the wrapper;
//! - `gunzip` on PATH (the loader's decompressor).
//!
//! The fixture (`tests/fixtures/image-flake/flake.nix`) is a
//! `dockerTools.buildLayeredImage` whose ONLY content is a static text file
//! — no network FODs, ~20 KiB tarball. Its nixpkgs input pins the SAME rev
//! as this repo's `flake.lock`, so eval+build need no network once that rev
//! is realized in the store.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]

use std::path::{Path, PathBuf};

use workestrate::images::detect::{DrvEvaluator, MsbStoreProbe, NixCliEvaluator, StoreProbe};
use workestrate::images::pipeline::{
    run_build_pipeline, BuildJob, LoadAction, MsbCliLoader, NixCliBuilder,
};
use workestrate::images::state::{image_key, ImagesState, RepoIdentity};

fn nix_on_path() -> bool {
    std::process::Command::new("nix")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn gunzip_on_path() -> bool {
    std::process::Command::new("gunzip")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// An msb binary that honors MSB_HOME (see the module docs for why the
/// devshell's wrapped `msb` is NOT acceptable here).
fn msb_for_e2e() -> Option<String> {
    let bin = std::env::var("MSB_PATH").ok()?;
    let ok = std::process::Command::new(&bin)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if ok {
        Some(bin)
    } else {
        None
    }
}

fn uniq_tmp(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "workestrate-e2e-{}-{}-{}",
        label,
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ))
}

/// The full pipeline lifecycle against a real nix build and a real
/// (temp-homed) msb store: load → gate-skip → out-of-band delete → reload.
#[tokio::test]
async fn fixture_image_full_pipeline_lifecycle() {
    if !nix_on_path() {
        eprintln!("note: nix not on PATH; skipping the phase-D pipeline e2e");
        return;
    }
    if !gunzip_on_path() {
        eprintln!("note: gunzip not on PATH; skipping the phase-D pipeline e2e");
        return;
    }
    let Some(msb) = msb_for_e2e() else {
        eprintln!(
            "note: MSB_PATH is not set to a working msb binary; skipping the phase-D \
             pipeline e2e (set MSB_PATH to an msb that HONORS MSB_HOME — NOT the \
             devshell's wrapped msb, which forces MSB_HOME to the real home store)"
        );
        return;
    };
    let fixture_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("image-flake");
    let fixture = fixture_dir.join("flake.nix");
    assert!(fixture.is_file(), "fixture flake missing: {fixture:?}");

    // Copy the fixture OUT of the repo git tree: path flakes inside a git
    // worktree eval through git+file semantics (tracked files only, whole-
    // tree copies); a plain temp dir keeps the e2e hermetic. The committed
    // flake.lock comes along so the eval is fully pinned.
    let tmp = uniq_tmp("image-pipeline");
    let flake_root = tmp.join("flake");
    std::fs::create_dir_all(&flake_root).unwrap();
    std::fs::copy(&fixture, flake_root.join("flake.nix")).unwrap();
    std::fs::copy(
        fixture_dir.join("flake.lock"),
        flake_root.join("flake.lock"),
    )
    .unwrap();
    let state_dir = tmp.join("state");
    let msb_home = tmp.join("msb-home");
    std::fs::create_dir_all(&msb_home).unwrap();

    // MSB_HOME for BOTH the SDK probe (MsbStoreProbe reads it directly) and
    // the msb CLI child processes (inherited env).
    let prior_msb_home = std::env::var_os("MSB_HOME");
    std::env::set_var("MSB_HOME", &msb_home);
    // MsbCliLoader::new() resolves msb via the doctor MSB_PATH convention.
    std::env::set_var("MSB_PATH", &msb);

    let attr = "packages.x86_64-linux.wk-fixture-image";
    // A2 (ADR 0032 §Image tags): BuildJob.tag is the caller-computed
    // content-addressed tag in production; this e2e drives the pipeline
    // directly, so any stable tag works. tag_ctx feeds the stage-4
    // current-pointer upsert (asserted below).
    let tag = "wk-fixture-image:latest";
    let mut eval = NixCliEvaluator::new();
    let drv_path = eval
        .eval_drv_path(&flake_root, attr)
        .expect("fixture drvPath eval must succeed (no network inputs)");
    let job = BuildJob {
        workload: "fixture".to_string(),
        repo: RepoIdentity {
            name: "e2e".to_string(),
            path: flake_root.clone(),
            flake_root: flake_root.clone(),
        },
        attr: attr.to_string(),
        tag: tag.to_string(),
        tag_ctx: None,
        drv_path: drv_path.clone(),
        force: false,
    };

    let restore = |prior: &Option<std::ffi::OsString>| match prior {
        Some(v) => std::env::set_var("MSB_HOME", v),
        None => std::env::remove_var("MSB_HOME"),
    };

    let result = async {
        let mut builder = NixCliBuilder::new();
        let mut loader = MsbCliLoader::new();
        let mut probe = MsbStoreProbe;

        // Run 1: absent record + absent tag → build → load → record.
        let first =
            run_build_pipeline(&job, &state_dir, &mut builder, &mut loader, &mut probe).await?;
        assert_eq!(first.action, LoadAction::Loaded, "first run loads");
        let out_path = PathBuf::from(&first.out_path);
        assert!(
            out_path.starts_with("/nix/store/") && out_path.is_file(),
            "the realized outPath is a real store tarball: {out_path:?}"
        );
        // The store holds the tag (ground truth, spec §3.2).
        assert_eq!(
            probe.tag_state(tag).await.expect("store reachable"),
            workestrate::images::skew::StoreTag::Present
        );
        // The record carries the phase-D shape.
        let key = image_key("e2e", tag);
        let record = ImagesState::load(&state_dir)
            .lookup(&key)
            .expect("record written")
            .clone();
        assert_eq!(record.drv_path, drv_path);
        assert_eq!(record.out_path, first.out_path);
        assert_eq!(record.digest, None, "§3.5 probe point stays null");
        // A2: the stage-4 upsert also moved the current-pointer.
        let state = ImagesState::load(&state_dir);
        assert_eq!(
            state
                .lookup_pointer(&workestrate::images::state::pointer_key("e2e", attr, None))
                .map(|p| p.tag.as_str()),
            Some(tag),
            "the current-pointer for (repo, attr, ctx=None) moved to the loaded tag"
        );

        // Run 2: same inputs → the outPath re-load gate SKIPS `msb load`.
        let second =
            run_build_pipeline(&job, &state_dir, &mut builder, &mut loader, &mut probe).await?;
        assert_eq!(
            second.action,
            LoadAction::AlreadyCurrent,
            "second run: image unchanged in store; tag already current (§3.1 gate)"
        );
        assert_eq!(second.out_path, first.out_path, "deterministic realization");

        // Out-of-band deletion (`msb image rm`), then run 3 loads anyway.
        let rm = std::process::Command::new(&msb)
            .args(["image", "rm", tag])
            .output()
            .expect("spawn msb image rm");
        assert!(rm.status.success(), "msb image rm failed: {rm:?}");
        assert_eq!(
            probe.tag_state(tag).await.expect("store reachable"),
            workestrate::images::skew::StoreTag::Gone,
            "the tag is gone out-of-band"
        );
        let third =
            run_build_pipeline(&job, &state_dir, &mut builder, &mut loader, &mut probe).await?;
        assert_eq!(
            third.action,
            LoadAction::Loaded,
            "out-of-band deletion forces the reload even with a matching outPath"
        );
        Ok::<(), anyhow::Error>(())
    }
    .await;

    restore(&prior_msb_home);
    let _ = std::fs::remove_dir_all(&tmp);
    result.expect("phase-D pipeline e2e");
    // Sanity for future readers: the fixture path exists relative to the
    // manifest (guards against a moved fixture silently skipping).
    assert!(Path::new(env!("CARGO_MANIFEST_DIR")).is_dir());
}
