//! Change detection for the image build/load lifecycle (spec 21 §3, phase C).
//!
//! Two seams, both trait-based so unit tests drive fakes while production
//! uses the real backends:
//!
//! - [`DrvEvaluator`] — the eval-only §3.1 signals: `nix eval --raw
//!   <flake_root>#<attr>.drvPath` (provenance) and, under A2,
//!   `<attr>.outPath` (the content-addressed tag source). Eval-only means NO
//!   build is triggered and
//!   fakeHash placeholders eval fine (a placeholder FOD hash perturbs neither
//!   the derivation structure nor its drvPath — the fixture test below proves
//!   it with a fixed-output derivation carrying a placeholder hash, and the
//!   phase-C smoke verified it against the personal repo's `tempest` attr,
//!   which carries a HOST-GATE placeholder `npm_deps_hash`). The real backend
//!   shells out to the `nix` CLI; [`DrvEvalError::NixAbsent`] (spawn
//!   `NotFound`) drives the §7 "nix absent from PATH" ladder.
//! - [`StoreProbe`] — msb store-tag presence (ground truth for what is
//!   loaded, spec §3.2). The real backend is `microsandbox::Image::get`; an
//!   unreachable store is a named ERROR per spec §7 ("msb store unreachable"
//!   row), reusing the `ps.rs` unreachable-DB vocabulary (the
//!   [`crate::microsandbox::runtime::ps::probe_liveness`] Io/Http/Database
//!   reachability class) — it is NOT a third `StoreTag` variant: the skew
//!   matrix decides between present/gone, and "cannot tell" fails the
//!   command.
//!
//! [`record_state_for`] derives the [`RecordState`] half of the skew matrix.
//! A2 (ADR 0032 §Image tags — DECIDED 2026-08-24): the freshness signal is
//! the CONTENT-ADDRESSED tag — `nix eval --raw <flake>#<attr>.outPath`
//! (eval-only, no build) decides the `<name>:<ctx>.<sha>` store tag BEFORE
//! the store probe, and a record under that computed key is Fresh by
//! construction. The drvPath eval is still recorded for provenance (spec §8)
//! but no longer drives the decision; the outPath re-load gate (re-load only
//! when the realized outPath differs, e.g. out-of-band tag deletion) stays
//! phase D.

use std::path::{Path, PathBuf};

use crate::images::skew::{RecordState, StoreTag};
use crate::images::state::ImageRecord;

// ---------------------------------------------------------------------------
// drvPath evaluation (spec §3.1)
// ---------------------------------------------------------------------------

/// Failure modes of a drvPath eval. `Clone` so fakes can queue responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrvEvalError {
    /// `nix` could not be spawned (not on PATH). The caller applies the §7
    /// ladder: store tag present → proceed with a stderr note (degrade, don't
    /// block); store tag missing → hard error + remediation.
    NixAbsent,
    /// The flake evaluated but does not provide the attribute (carries the
    /// attr and nix's stderr detail).
    AttrMissing { attr: String, detail: String },
    /// Any other eval failure (flake syntax/eval errors); carries nix's
    /// stderr verbatim — the named error must surface it (§7 spirit: the
    /// operator sees nix's own diagnosis, not a flattened "eval failed").
    EvalFailed { detail: String },
}

impl std::fmt::Display for DrvEvalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DrvEvalError::NixAbsent => write!(f, "nix not found on PATH"),
            DrvEvalError::AttrMissing { attr, .. } => {
                write!(f, "flake does not provide attribute '{attr}'")
            }
            DrvEvalError::EvalFailed { .. } => write!(f, "nix eval failed"),
        }
    }
}

impl std::error::Error for DrvEvalError {}

/// The drvPath/outPath-eval seam. `&mut self` so fakes can record calls.
pub trait DrvEvaluator {
    /// `nix eval --raw <flake_root>#<attr>.drvPath` — eval-only (no build).
    /// Recorded for provenance/diagnostics (spec §8 `drv_path`); no longer
    /// the freshness signal (A2: the content-addressed tag is).
    fn eval_drv_path(&mut self, flake_root: &Path, attr: &str) -> Result<String, DrvEvalError>;

    /// `nix eval --raw <flake_root>#<attr>.outPath` — eval-only (no build).
    /// A2 (ADR 0032 §Image tags): the evaluated outPath decides the
    /// content-addressed store tag BEFORE any store probe (unchanged inputs
    /// → same outPath → same tag → Present → skip).
    fn eval_out_path(&mut self, flake_root: &Path, attr: &str) -> Result<String, DrvEvalError>;
}

/// Real backend: the `nix` CLI. `program` is injectable so tests can point
/// it at a nonexistent path to exercise the [`DrvEvalError::NixAbsent`]
/// mapping hermetically.
pub struct NixCliEvaluator {
    program: PathBuf,
}

impl NixCliEvaluator {
    pub fn new() -> Self {
        Self {
            program: PathBuf::from("nix"),
        }
    }

    pub fn with_program(program: impl Into<PathBuf>) -> Self {
        Self {
            program: program.into(),
        }
    }
}

impl Default for NixCliEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl NixCliEvaluator {
    /// Shared `nix eval --raw <flake_root>#<attr>.<suffix>` spawn (eval-only,
    /// hermetic against the ambient nix config and cwd — see the comment at
    /// the Command construction).
    fn eval_raw(
        &self,
        flake_root: &Path,
        attr: &str,
        suffix: &str,
    ) -> Result<String, DrvEvalError> {
        let reference = format!("{}#{}{}", flake_root.display(), attr, suffix);
        // `--extra-experimental-features` is passed EXPLICITLY (additive — a
        // user nix.conf that already enables them is unaffected) so the eval
        // is hermetic against the ambient nix config: the fixture tests run
        // in a process where parallel env-mutating tests can hide
        // ~/.config/nix/nix.conf (HOME mutation), and on hosts the feature
        // flags may live only in the user's config. `current_dir` is pinned
        // to the flake root for the same reason: the eval needs no cwd, and
        // the inherited process cwd can be a directory another parallel test
        // already deleted ("cannot get cwd" failures).
        let output = std::process::Command::new(&self.program)
            .args([
                "eval",
                "--raw",
                "--extra-experimental-features",
                "nix-command flakes",
                &reference,
            ])
            .current_dir(flake_root)
            .output()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    DrvEvalError::NixAbsent
                } else {
                    DrvEvalError::EvalFailed {
                        detail: format!("failed to spawn nix: {e}"),
                    }
                }
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            // nix 2.x names a missing output attr as "does not provide
            // attribute" — distinguishable from a flake EVAL error, and the
            // distinction is cheap, so keep it (§7: named errors).
            if stderr.contains("does not provide attribute") {
                return Err(DrvEvalError::AttrMissing {
                    attr: attr.to_string(),
                    detail: stderr,
                });
            }
            return Err(DrvEvalError::EvalFailed { detail: stderr });
        }
        let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if value.is_empty() {
            return Err(DrvEvalError::EvalFailed {
                detail: format!("nix eval {reference} succeeded but printed an empty value"),
            });
        }
        Ok(value)
    }
}

impl DrvEvaluator for NixCliEvaluator {
    fn eval_drv_path(&mut self, flake_root: &Path, attr: &str) -> Result<String, DrvEvalError> {
        self.eval_raw(flake_root, attr, ".drvPath")
    }

    fn eval_out_path(&mut self, flake_root: &Path, attr: &str) -> Result<String, DrvEvalError> {
        self.eval_raw(flake_root, attr, ".outPath")
    }
}

/// Derive the record half of the skew matrix (spec §3.4) for the
/// A2 content-addressed tag scheme (ADR 0032 §Image tags): the caller looks
/// up the record for the COMPUTED tag (`<repo>#<name:ctx.sha>`), so record
/// PRESENCE is the freshness signal — a record under that key exists only
/// when exactly this content was built+loaded (or D1-trusted). `Absent`
/// when no record exists for the key (first run, changed content → a new
/// tag, or another home loaded it — see the TRUST branch, D1); `Fresh`
/// otherwise. [`RecordState::Stale`] is unreachable under content-addressed
/// tags (stale content keys under a DIFFERENT tag) and remains only in the
/// matrix for the documented row-2 semantics.
pub fn record_state_for(record: Option<&ImageRecord>) -> RecordState {
    match record {
        None => RecordState::Absent,
        Some(_) => RecordState::Fresh,
    }
}

// ---------------------------------------------------------------------------
// msb store-tag presence (spec §3.2)
// ---------------------------------------------------------------------------

/// The §7 "msb store unreachable" named error. Mirrors the `ps.rs`
/// unreachable-DB vocabulary (`probe_liveness`'s Io/Http/Database
/// reachability class): same "db unreachable" shape, plus the remediation
/// wording for the image-store context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreUnreachable {
    pub tag: String,
    pub source: String,
}

impl std::fmt::Display for StoreUnreachable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "msb image store unreachable while probing tag '{}' (db unreachable: {}); \
             the msb store is ground truth for loaded images, so `workload build` cannot \
             proceed without it — check that msb is installed and its database is readable \
             (MSB_HOME), then retry (spec 21 §7)",
            self.tag, self.source
        )
    }
}

impl std::error::Error for StoreUnreachable {}

/// The store-presence seam. Async because the real backend is the async
/// microsandbox SDK (the build handlers are async).
pub trait StoreProbe {
    /// Present iff the tag exists in the msb image store. An unreachable
    /// store is [`StoreUnreachable`], NEVER silently treated as Gone.
    fn tag_state(
        &mut self,
        tag: &str,
    ) -> impl std::future::Future<Output = Result<StoreTag, StoreUnreachable>> + Send;
}

/// Real backend: `microsandbox::Image::get`.
pub struct MsbStoreProbe;

impl StoreProbe for MsbStoreProbe {
    async fn tag_state(&mut self, tag: &str) -> Result<StoreTag, StoreUnreachable> {
        match microsandbox::Image::get(tag).await {
            Ok(_) => Ok(StoreTag::Present),
            Err(microsandbox::MicrosandboxError::ImageNotFound(_)) => Ok(StoreTag::Gone),
            // §7 "msb store unreachable": probe_liveness's reachability class
            // is Io / Http / Database. Any OTHER SDK error variant likewise
            // means the tag state could not be determined — the same named
            // error, never a silent Gone.
            Err(e) => Err(StoreUnreachable {
                tag: tag.to_string(),
                source: e.to_string(),
            }),
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
pub mod test_fakes {
    //! Fakes shared by `detect.rs` and `build_cmd.rs` tests (`pub` under
    //! `cfg(test)` so the sibling module's tests reuse them).

    use super::*;
    use std::collections::VecDeque;

    /// Queue-driven fake evaluator: each call pops one queued response and
    /// records the (flake_root, attr) call. drvPath and outPath evals have
    /// SEPARATE queues (`responses`/`calls` vs `out_responses`/`out_calls`)
    /// so tests drive the A2 out-path-first flow deterministically.
    pub struct FakeEvaluator {
        pub responses: VecDeque<Result<String, DrvEvalError>>,
        pub calls: Vec<(PathBuf, String)>,
        pub out_responses: VecDeque<Result<String, DrvEvalError>>,
        pub out_calls: Vec<(PathBuf, String)>,
    }

    impl FakeEvaluator {
        pub fn new() -> Self {
            Self {
                responses: VecDeque::new(),
                calls: Vec::new(),
                out_responses: VecDeque::new(),
                out_calls: Vec::new(),
            }
        }

        pub fn push_ok(&mut self, drv: &str) {
            self.responses.push_back(Ok(drv.to_string()));
        }

        pub fn push_err(&mut self, err: DrvEvalError) {
            self.responses.push_back(Err(err));
        }

        pub fn push_out_ok(&mut self, out_path: &str) {
            self.out_responses.push_back(Ok(out_path.to_string()));
        }

        pub fn push_out_err(&mut self, err: DrvEvalError) {
            self.out_responses.push_back(Err(err));
        }
    }

    impl Default for FakeEvaluator {
        fn default() -> Self {
            Self::new()
        }
    }

    impl DrvEvaluator for FakeEvaluator {
        fn eval_drv_path(&mut self, flake_root: &Path, attr: &str) -> Result<String, DrvEvalError> {
            self.calls
                .push((flake_root.to_path_buf(), attr.to_string()));
            self.responses
                .pop_front()
                .expect("FakeEvaluator: no queued response")
        }

        fn eval_out_path(&mut self, flake_root: &Path, attr: &str) -> Result<String, DrvEvalError> {
            self.out_calls
                .push((flake_root.to_path_buf(), attr.to_string()));
            self.out_responses
                .pop_front()
                .expect("FakeEvaluator: no queued out_path response")
        }
    }

    /// Queue-driven fake store probe.
    pub struct FakeStoreProbe {
        pub responses: VecDeque<Result<StoreTag, StoreUnreachable>>,
        pub calls: Vec<String>,
    }

    impl FakeStoreProbe {
        pub fn new() -> Self {
            Self {
                responses: VecDeque::new(),
                calls: Vec::new(),
            }
        }

        pub fn push(&mut self, state: StoreTag) {
            self.responses.push_back(Ok(state));
        }

        pub fn push_unreachable(&mut self, tag: &str, source: &str) {
            self.responses.push_back(Err(StoreUnreachable {
                tag: tag.to_string(),
                source: source.to_string(),
            }));
        }
    }

    impl Default for FakeStoreProbe {
        fn default() -> Self {
            Self::new()
        }
    }

    impl StoreProbe for FakeStoreProbe {
        async fn tag_state(&mut self, tag: &str) -> Result<StoreTag, StoreUnreachable> {
            self.calls.push(tag.to_string());
            self.responses
                .pop_front()
                .expect("FakeStoreProbe: no queued response")
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::test_fakes::*;
    use super::*;
    use crate::config::test_support::uniq_dir;

    // ---- record_state_for (the A2 presence-based freshness predicate) ----

    fn record_with_drv(drv: &str) -> ImageRecord {
        ImageRecord {
            repo: crate::images::state::RepoIdentity {
                name: "personal".to_string(),
                path: PathBuf::from("/tmp/repo"),
                flake_root: PathBuf::from("/tmp/repo"),
            },
            attr: "workestrate-pi".to_string(),
            tag: "workestrate-pi:abcdefghijkl".to_string(),
            drv_path: drv.to_string(),
            out_path: "/nix/store/abcdefghijklmnopqrstuvwxyz012345-workestrate-pi.tar.gz"
                .to_string(),
            digest: None,
            built_at: "2026-08-02T10:15:00Z".to_string(),
            loaded_at: "2026-08-02T10:15:00Z".to_string(),
            loader: "workestrate 0.1.0".to_string(),
            host: "devbox".to_string(),
            user: "node".to_string(),
        }
    }

    /// A2 (ADR 0032 §Image tags): the caller keys the lookup by the COMPUTED
    /// content-addressed tag, so record presence IS freshness — drvPath is
    /// no longer consulted (a drv-text churn with unchanged outPath yields
    /// the same tag and must Skip, not rebuild).
    #[test]
    fn record_state_absent_or_fresh_by_presence() {
        assert_eq!(record_state_for(None), RecordState::Absent);
        let record = record_with_drv("drv-A");
        assert_eq!(
            record_state_for(Some(&record)),
            RecordState::Fresh,
            "a record under the computed tag is Fresh regardless of drv text"
        );
    }

    // ---- NixCliEvaluator error mapping (hermetic) ----

    /// §7 mapping: a nix binary that cannot be spawned maps to NixAbsent
    /// (the caller's degrade/hard-error ladder keys on this variant).
    #[test]
    fn unspawnable_nix_maps_to_nix_absent() {
        let mut eval = NixCliEvaluator::with_program("/definitely/not/on/path/nix");
        match eval.eval_drv_path(Path::new("/tmp/repo"), "workestrate-pi") {
            Err(DrvEvalError::NixAbsent) => {}
            other => panic!("expected NixAbsent, got {other:?}"),
        }
        // The A2 outPath eval maps identically (same spawn path).
        match eval.eval_out_path(Path::new("/tmp/repo"), "workestrate-pi") {
            Err(DrvEvalError::NixAbsent) => {}
            other => panic!("expected NixAbsent, got {other:?}"),
        }
    }

    // ---- Real nix eval against a zero-network fixture flake ----

    fn nix_on_path() -> bool {
        std::process::Command::new("nix")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Write a fixture flake with ZERO network inputs: a plain
    /// `builtins.derivation` plus a FIXED-OUTPUT derivation carrying a
    /// placeholder hash (the fakeHash case of spec §3.1 — eval must succeed
    /// without realizing the FOD). drvPath eval needs no build and no
    /// network. Returns (dir, plain_attr, fod_attr).
    fn write_fixture_flake(label: &str, name_suffix: &str) -> (PathBuf, String, String) {
        let dir = uniq_dir(label);
        std::fs::create_dir_all(&dir).unwrap();
        let plain_attr = format!("x86_64-linux.wk-fixture-img{name_suffix}");
        let fod_attr = format!("x86_64-linux.wk-fixture-fod{name_suffix}");
        let flake = format!(
            r#"{{
  outputs = {{ self }}: {{
    {plain_attr} = builtins.derivation {{
      name = "wk-fixture-img{name_suffix}";
      system = "x86_64-linux";
      builder = "/bin/sh";
      args = [ "-c" "echo hi > $out" ];
    }};
    {fod_attr} = builtins.derivation {{
      name = "wk-fixture-fod{name_suffix}";
      system = "x86_64-linux";
      builder = "/bin/sh";
      args = [ "-c" "echo hi > $out" ];
      outputHashMode = "recursive";
      outputHashAlgo = "sha256";
      outputHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";
    }};
  }};
}}
"#
        );
        std::fs::write(dir.join("flake.nix"), flake).unwrap();
        (dir, plain_attr, fod_attr)
    }

    /// REAL integration test (spec 21 §3.1): drvPath is stable across repeat
    /// evals, changes when the derivation definition changes, and eval works
    /// with a placeholder FOD hash (fakeHash) — all with zero network and no
    /// build. Skipped (with a note) when nix is absent from PATH.
    #[test]
    fn fixture_flake_drv_path_eval_is_stable_changes_on_edit_and_tolerates_fakehash() {
        if !nix_on_path() {
            eprintln!(
                "note: nix not on PATH; skipping the real-eval fixture-flake integration test"
            );
            return;
        }
        let (dir, plain_attr, fod_attr) = write_fixture_flake("drv-eval-fixture", "");
        let mut eval = NixCliEvaluator::new();

        let first = eval
            .eval_drv_path(&dir, &plain_attr)
            .expect("drvPath eval must succeed with zero network inputs");
        assert!(
            first.starts_with("/nix/store/") && first.ends_with(".drv"),
            "unexpected drvPath shape: {first}"
        );
        // Stable across repeat evals (change detection must not flap).
        let second = eval.eval_drv_path(&dir, &plain_attr).expect("re-eval");
        assert_eq!(first, second, "drvPath must be stable across repeat evals");

        // fakeHash placeholder (spec §3.1: "eval works with fakeHash
        // placeholders") — the FOD attr evals WITHOUT realizing the hash.
        let fod = eval
            .eval_drv_path(&dir, &fod_attr)
            .expect("eval-only drvPath must work with a placeholder FOD hash");
        assert!(fod.ends_with(".drv"));

        // A2: the outPath eval (the content-addressed tag source) is
        // eval-only too — succeeds with zero network, is stable across
        // repeat evals, and yields a /nix/store/<hash>-<name> shape whose
        // 12-char hash prefix feeds compute_image_tag.
        let out = eval
            .eval_out_path(&dir, &plain_attr)
            .expect("outPath eval must succeed with zero network inputs");
        assert!(
            out.starts_with("/nix/store/") && !out.ends_with(".drv"),
            "unexpected outPath shape: {out}"
        );
        assert_eq!(
            crate::images::state::store_hash_prefix(&out).map(|s| s.len()),
            Some(crate::images::state::OUT_PATH_HASH_PREFIX_LEN),
            "the fixture out_path has a >=12-char store-hash segment: {out}"
        );
        let out2 = eval
            .eval_out_path(&dir, &plain_attr)
            .expect("re-eval outPath");
        assert_eq!(out, out2, "outPath must be stable across repeat evals");
        // fakeHash placeholder FOD: outPath evals WITHOUT realizing the hash
        // (the tag tracks the placeholder until update-hashes fills it).
        let fod_out = eval
            .eval_out_path(&dir, &fod_attr)
            .expect("eval-only outPath must work with a placeholder FOD hash");
        assert!(fod_out.starts_with("/nix/store/"));

        // Editing the derivation definition changes the drvPath — THE
        // change-detection signal.
        let edited_content = std::fs::read_to_string(dir.join("flake.nix"))
            .unwrap()
            .replace("wk-fixture-img\"", "wk-fixture-img-v2\"");
        std::fs::write(dir.join("flake.nix"), edited_content).unwrap();
        let edited = eval
            .eval_drv_path(&dir, &plain_attr)
            .expect("edited derivation must still eval");
        assert_ne!(
            first, edited,
            "a derivation-definition edit must change the drvPath"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A missing attribute maps to AttrMissing (distinct from a flake eval
    /// error — §7 named errors), gated on nix presence.
    #[test]
    fn missing_attribute_maps_to_attr_missing() {
        if !nix_on_path() {
            eprintln!("note: nix not on PATH; skipping the attr-missing eval test");
            return;
        }
        let (dir, _, _) = write_fixture_flake("drv-eval-attr", "");
        let mut eval = NixCliEvaluator::new();
        match eval.eval_drv_path(&dir, "x86_64-linux.no-such-attr") {
            Err(DrvEvalError::AttrMissing { attr, detail }) => {
                assert!(attr.contains("no-such-attr"));
                assert!(
                    detail.contains("does not provide attribute"),
                    "nix's own wording must surface: {detail}"
                );
            }
            other => panic!("expected AttrMissing, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    // ---- StoreUnreachable vocabulary (§7) ----

    /// The §7 "msb store unreachable" wording is pinned: same "db
    /// unreachable" shape as ps.rs, plus the remediation.
    #[test]
    fn store_unreachable_message_matches_ps_vocabulary() {
        let e = StoreUnreachable {
            tag: "workestrate-pi:latest".to_string(),
            source: "io error: not a directory".to_string(),
        };
        let msg = e.to_string();
        assert!(msg.contains("msb image store unreachable"), "{msg}");
        assert!(
            msg.contains("db unreachable"),
            "ps.rs unreachable-DB vocabulary: {msg}"
        );
        assert!(msg.contains("workestrate-pi:latest"), "{msg}");
        assert!(msg.contains("MSB_HOME"), "remediation wording: {msg}");
    }

    /// Fake probe queue semantics (the build_cmd integration tests drive the
    /// whole flow through these).
    #[tokio::test]
    async fn fake_store_probe_pops_queued_responses() {
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present);
        probe.push(StoreTag::Gone);
        assert_eq!(probe.tag_state("a:1").await, Ok(StoreTag::Present));
        assert_eq!(probe.tag_state("a:1").await, Ok(StoreTag::Gone));
        assert_eq!(probe.calls, vec!["a:1", "a:1"]);
    }

    /// REAL SDK test: an unreachable msb DB (MSB_HOME pointed under a
    /// regular file → ENOTDIR on `<MSB_HOME>/db`, the ps.rs trick) maps to
    /// the named §7 error, never to a silent Gone.
    ///
    /// Deterministic per the ps.rs note: a FAILING `init_global` does not
    /// pin the SDK's process-global DB pool, so this cannot contaminate (or
    /// be contaminated by) other SDK tests.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn msb_store_probe_unreachable_db_maps_to_named_error() {
        let _env_lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let tmp = uniq_dir("store-probe-unreachable");
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("blocker"), b"x").unwrap();
        let prior = std::env::var_os("MSB_HOME");
        std::env::set_var("MSB_HOME", tmp.join("blocker"));

        let mut probe = MsbStoreProbe;
        let result = probe.tag_state("workestrate-pi:latest").await;

        match &prior {
            Some(v) => std::env::set_var("MSB_HOME", v),
            None => std::env::remove_var("MSB_HOME"),
        }
        let _ = std::fs::remove_dir_all(&tmp);

        match result {
            Err(StoreUnreachable { tag, .. }) => {
                assert_eq!(tag, "workestrate-pi:latest");
            }
            other => panic!("an unreachable DB must be the named §7 error, got {other:?}"),
        }
    }
}
