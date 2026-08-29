//! Build/load pipeline (spec 21 phase D) — nix build → outPath re-load gate
//! → `msb load` → record upsert, all inside the caller's still-held per-tag
//! lock (spec §3.3).
//!
//! The pipeline consumes the fully-resolved [`BuildJob`] assembled by the
//! phase-C build verb ([`crate::images::build_cmd::process_target`]) and runs
//! FOUR stages, in order:
//!
//! 1. **nix build** ([`ImageBuilder`]; real backend [`NixCliBuilder`]):
//!    `nix build <flake_root>#<attr> --no-link --print-out-paths
//!    --extra-experimental-features "nix-command flakes"` with
//!    `current_dir(flake_root)` (the [`crate::images::detect::NixCliEvaluator`]
//!    conventions — hermetic against the ambient nix config and cwd).
//!    `--print-out-paths` writes the realized outPath to stdout; build
//!    progress goes to stderr, which is TEED: copied live to the operator's
//!    TTY (this is a parent-side, user-facing, minutes-long operation) AND
//!    retained for §7 failure classification ([`classify_build_stderr`]).
//!    Spawn `NotFound` maps to the SAME "nix absent" vocabulary as the
//!    phase-C eval ladder ([`BuildError::NixAbsent`] — see stage mapping in
//!    [`run_build_pipeline`]).
//! 2. **outPath re-load gate** (spec §3.1): `ImagesState` is re-loaded from
//!    `state_dir` (fresh, inside the lock) and the store tag is probed via
//!    [`StoreProbe`]. The pure [`reload_decision`] decides: skip `msb load`
//!    when the recorded outPath is non-empty AND equals the fresh outPath AND
//!    the tag is still in the store (eval churn resolving to an
//!    already-realized outPath costs no `msb load`) — BUT load anyway when
//!    the tag is GONE (out-of-band deletion) or the record's `out_path` is
//!    empty (phase-C trust records) or differs.
//! 3. **msb load** ([`ImageLoader`]; real backend [`MsbCliLoader`]):
//!    `gunzip -c <outPath>` piped into `msb load -t <tag>` — two
//!    `std::process::Command`s, no shell, no tarball staged on disk (the
//!    anti-accumulation posture). msb is located via
//!    [`crate::commands::doctor::msb_binary`] (the `MSB_PATH` convention);
//!    `MSB_HOME` is honored implicitly (msb reads it). On failure the named
//!    §7 "load failure" error surfaces msb's stderr. After a load, the store
//!    is probed AGAIN: `msb load` reporting success while the tag stays gone
//!    is a named error, never silent.
//! 4. **record upsert INSIDE the lock** (spec §3.3 atomicity): `drv_path` =
//!    the job's current eval, `out_path` = the realized outPath, `built_at`
//!    captured right after the nix build, `loaded_at` captured after the load
//!    (or, on the gate-skip path, after the store probe that verified the
//!    tag still present — the load-equivalent guarantee the record tracks),
//!    `digest = None` (the §3.5 upgrade hook — a documented probe point,
//!    pending §11 HOST-VERIFY item 1), then `ImagesState::save(state_dir)`.
//!
//! Both process seams are traits so unit tests drive fakes (mock-free, per
//! repo convention — see `test_fakes` under `cfg(test)`); only the real
//! backends spawn processes.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};

use anyhow::Result;

use crate::images::detect::StoreProbe;
use crate::images::skew::StoreTag;
use crate::images::state::{image_key, ImageRecord, ImagesState, Provenance, RepoIdentity};

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
    /// The store tag to load under. A2 (ADR 0032 §Image tags — DECIDED
    /// 2026-08-24): the caller computes the IMMUTABLE content-addressed tag
    /// (`<name>:<ctx>.<sha>`, or `<name>:<sha>` without a tag context) from
    /// the evaluated out_path; the capsule's declared `tag` field is no
    /// longer loaded into the store for nix-layered images.
    pub tag: String,
    /// The tag-context segment baked into [`BuildJob::tag`] (ADR 0032 —
    /// [`crate::images::state::image_tag_context`] at ensure time). Also the
    /// pointer-key segment for the stage-4 current-pointer upsert.
    pub tag_ctx: Option<String>,
    /// The current drvPath eval (spec §3.1) — the pipeline realizes this drv.
    pub drv_path: String,
    /// The build verb's `--force` flag (RebuildForced verdicts).
    pub force: bool,
}

/// What the load stage did (drives the build verb's `action_taken` wording).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadAction {
    /// The tarball was (re)loaded into the msb store and the tag was
    /// verified present afterwards.
    Loaded,
    /// The outPath re-load gate skipped `msb load`: the recorded outPath
    /// already equals the realized one and the tag is still in the store
    /// (spec §3.1 — eval churn costs no `msb load`).
    AlreadyCurrent,
}

/// The pipeline result: the realized outPath plus what the load stage did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PipelineOutcome {
    pub out_path: String,
    pub action: LoadAction,
}

// ---------------------------------------------------------------------------
// Stage 1: nix build (spec §3.1 realization)
// ---------------------------------------------------------------------------

/// Failure modes of `nix build`. `Clone` so fakes can queue responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// `nix` could not be spawned (not on PATH). Mapped by
    /// [`run_build_pipeline`] to the SAME §7 "nix absent" wording the
    /// phase-C eval ladder uses (build and `--check` share vocabulary).
    NixAbsent,
    /// §7 "fakeHash placeholder" row: nix's fixed-output hash-mismatch
    /// pattern. `detail` carries the extracted mismatch lines (NOT the raw
    /// error wall).
    FakeHashPlaceholder { reference: String, detail: String },
    /// §7 "Offline" row: a fetch/substituter failure. `detail` carries the
    /// stderr tail with the offline context noted in the message.
    OfflineFetch { reference: String, detail: String },
    /// Any other build failure; `detail` carries the stderr tail.
    Failed { reference: String, detail: String },
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::NixAbsent => write!(f, "nix not found on PATH"),
            BuildError::FakeHashPlaceholder { reference, detail } => write!(
                f,
                "nix build failed for '{reference}': the flake carries a placeholder hash \
                 (fakeHash) for a fixed-output derivation; run the declaring repo's \
                 'update-hashes' recipe to fill real hashes, then retry (spec 21 §7)\n{detail}"
            ),
            BuildError::OfflineFetch { reference, detail } => write!(
                f,
                "nix build failed for '{reference}': a fetch/substituter failure — the \
                 offline context applies (locked inputs eval fine, but builds proceed only \
                 when all inputs are already realized in the store; spec 21 §7). Check the \
                 network or pre-realize the inputs, then retry\n{detail}"
            ),
            BuildError::Failed { reference, detail } => {
                write!(f, "nix build failed for '{reference}':\n{detail}")
            }
        }
    }
}

impl std::error::Error for BuildError {}

/// The §7 failure classification over captured nix stderr (pure).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NixFailureClass {
    /// Fixed-output hash mismatch — the fakeHash placeholder row.
    FakeHashPlaceholder,
    /// Fetch/substituter failure — the offline row.
    OfflineFetch,
    /// Anything else — the generic build-failure row.
    Generic,
}

/// Markers of a fetch/substituter failure (§7 offline row): nix's download
/// and DNS failure wordings.
const OFFLINE_MARKERS: &[&str] = &[
    "unable to download",
    "Couldn't resolve host",
    "Temporary failure in name resolution",
    "Failed to fetch",
];

/// Classify captured `nix build` stderr into the §7 failure rows. The
/// fakeHash pattern is checked FIRST: a fixed-output hash mismatch is nix's
/// `hash mismatch … specified: sha256-… / got: sha256-…` report, and it must
/// point at the declaring repo's `update-hashes` recipe, not read as a
/// generic failure.
pub fn classify_build_stderr(stderr: &str) -> NixFailureClass {
    if stderr.contains("hash mismatch")
        && ((stderr.contains("got:") && stderr.contains("sha256-"))
            || (stderr.contains("specified:") && stderr.contains("sha256-")))
    {
        NixFailureClass::FakeHashPlaceholder
    } else if OFFLINE_MARKERS.iter().any(|m| stderr.contains(m)) {
        NixFailureClass::OfflineFetch
    } else {
        NixFailureClass::Generic
    }
}

/// The last `max` lines of captured stderr, trimmed — the "tail" surfaced by
/// the generic/offline rows (bounded, never the whole wall).
pub fn stderr_tail(stderr: &str, max: usize) -> String {
    let lines: Vec<&str> = stderr.trim().lines().collect();
    let start = lines.len().saturating_sub(max);
    lines[start..].join("\n")
}

/// The lines of a hash-mismatch report (the `hash mismatch` / `specified:` /
/// `got:` triple), for the fakeHash row's extracted detail.
fn hash_mismatch_lines(stderr: &str) -> String {
    let lines: Vec<&str> = stderr
        .lines()
        .map(str::trim)
        .filter(|l| {
            l.contains("hash mismatch") || l.starts_with("got:") || l.starts_with("specified:")
        })
        .collect();
    if lines.is_empty() {
        stderr_tail(stderr, 10)
    } else {
        lines.join("\n")
    }
}

/// Parse the realized outPath from `nix build --print-out-paths` stdout: the
/// LAST `/nix/store/…` line (pure).
pub fn parse_out_path(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .rev()
        .map(str::trim)
        .find(|l| l.starts_with("/nix/store/"))
        .map(str::to_string)
}

/// The nix-build seam. Sync (the real backend is a process spawn); `&mut
/// self` so fakes can record calls.
pub trait ImageBuilder {
    /// `nix build <flake_root>#<attr>`; returns the realized outPath.
    fn build_out_path(&mut self, flake_root: &Path, attr: &str) -> Result<String, BuildError>;
}

/// Real backend: the `nix` CLI. `program` is injectable so tests can point
/// it at a nonexistent path to exercise the [`BuildError::NixAbsent`]
/// mapping hermetically.
pub struct NixCliBuilder {
    program: PathBuf,
}

impl NixCliBuilder {
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

impl Default for NixCliBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageBuilder for NixCliBuilder {
    fn build_out_path(&mut self, flake_root: &Path, attr: &str) -> Result<String, BuildError> {
        let reference = format!("{}#{}", flake_root.display(), attr);
        let spawn_failed = |detail: String| BuildError::Failed {
            reference: reference.clone(),
            detail,
        };
        let mut child = Command::new(&self.program)
            .args([
                "build",
                &reference,
                "--no-link",
                "--print-out-paths",
                "--extra-experimental-features",
                "nix-command flakes",
            ])
            // Same hermeticity rationale as NixCliEvaluator: the build needs
            // no cwd, and the inherited process cwd can be a directory
            // another parallel test already deleted.
            .current_dir(flake_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    BuildError::NixAbsent
                } else {
                    spawn_failed(format!("failed to spawn nix: {e}"))
                }
            })?;
        // The stderr TEE: a drain thread copies build progress to the
        // operator's TTY live (parent-side, user-facing, minutes-long) AND
        // accumulates it for §7 classification. Without the drain, a verbose
        // nix build would deadlock on a full stderr pipe buffer.
        let mut stderr_pipe = child
            .stderr
            .take()
            .ok_or_else(|| spawn_failed("nix stderr was not piped".to_string()))?;
        let captured: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
        let sink = Arc::clone(&captured);
        let tee = std::thread::spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match stderr_pipe.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        let chunk = &buf[..n];
                        let _ = std::io::stderr().write_all(chunk);
                        let _ = std::io::stderr().flush();
                        if let Ok(mut guard) = sink.lock() {
                            guard.push_str(&String::from_utf8_lossy(chunk));
                        }
                    }
                }
            }
        });
        let mut stdout = Vec::new();
        if let Some(mut out) = child.stdout.take() {
            let _ = out.read_to_end(&mut stdout);
        }
        let status = child
            .wait()
            .map_err(|e| spawn_failed(format!("failed to wait on nix: {e}")))?;
        let _ = tee.join();
        let stderr = captured
            .lock()
            .map(|g| g.clone())
            .unwrap_or_else(|_| String::new());
        if !status.success() {
            return Err(match classify_build_stderr(&stderr) {
                NixFailureClass::FakeHashPlaceholder => BuildError::FakeHashPlaceholder {
                    reference: reference.clone(),
                    detail: hash_mismatch_lines(&stderr),
                },
                NixFailureClass::OfflineFetch => BuildError::OfflineFetch {
                    reference: reference.clone(),
                    detail: stderr_tail(&stderr, 20),
                },
                NixFailureClass::Generic => BuildError::Failed {
                    reference: reference.clone(),
                    detail: stderr_tail(&stderr, 20),
                },
            });
        }
        let stdout = String::from_utf8_lossy(&stdout);
        parse_out_path(&stdout).ok_or_else(|| {
            spawn_failed(format!(
                "nix build succeeded but printed no outPath on stdout (tail: {})",
                stderr_tail(&stdout, 5)
            ))
        })
    }
}

// ---------------------------------------------------------------------------
// Stage 2: the outPath re-load gate (spec §3.1) — pure
// ---------------------------------------------------------------------------

/// The gate verdict (spec §3.1 "Re-load gate").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReloadDecision {
    /// Recorded outPath is non-empty, equals the fresh outPath, and the tag
    /// is still in the store → skip `msb load`; the tag is already current.
    SkipLoad,
    /// Anything else: record outPath empty (phase-C trust record) or
    /// different, OR the tag is gone from the store (out-of-band deletion —
    /// load anyway even when the outPath matches).
    Load,
}

/// The re-load gate, pure over the record's outPath, the fresh outPath, and
/// the store-tag state — unit-testable without any process spawns.
pub fn reload_decision(record_out: &str, fresh_out: &str, store: StoreTag) -> ReloadDecision {
    match store {
        // Out-of-band deletion: the recorded bits are not in the store even
        // when the outPath matches — load anyway.
        StoreTag::Gone => ReloadDecision::Load,
        StoreTag::Present if !record_out.is_empty() && record_out == fresh_out => {
            ReloadDecision::SkipLoad
        }
        // record_out empty (phase-C trust record) or stale → load.
        StoreTag::Present => ReloadDecision::Load,
    }
}

// ---------------------------------------------------------------------------
// Stage 3: msb load (spec §5/§7 "load failure" row)
// ---------------------------------------------------------------------------

/// Failure modes of the gunzip | `msb load` stage. `Clone` so fakes can
/// queue responses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadError {
    /// `gunzip -c <outPath>` failed to spawn or exited non-zero.
    GunzipFailed { out_path: String, detail: String },
    /// The msb binary could not be spawned (not on PATH).
    MsbAbsent { program: String },
    /// `msb load` exited non-zero; `detail` carries msb's stderr (§7: the
    /// named load failure surfaces msb's own diagnosis).
    Failed { tag: String, detail: String },
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::GunzipFailed { out_path, detail } => write!(
                f,
                "failed to decompress image tarball {}: {detail}",
                out_path
            ),
            LoadError::MsbAbsent { program } => write!(
                f,
                "msb binary '{program}' not found on PATH — install msb, or set MSB_PATH \
                 to the msb binary, then retry (spec 21 §7)"
            ),
            LoadError::Failed { tag, detail } => {
                write!(f, "msb load failed for tag '{tag}': {detail}")
            }
        }
    }
}

impl std::error::Error for LoadError {}

/// The msb-load seam. Sync (the real backend is a process spawn).
pub trait ImageLoader {
    /// Stream the gzipped tarball at `out_path` into the msb store under
    /// `tag`.
    fn load(&mut self, out_path: &Path, tag: &str) -> Result<(), LoadError>;
}

/// Real backend: `gunzip -c <outPath>` piped into `msb load -t <tag>` — two
/// `std::process::Command`s, no shell, no tarball staged on disk (the
/// anti-accumulation posture: nothing accumulates under the config repo or
/// the state dir). [`MsbCliLoader::new`] locates msb via
/// [`crate::commands::doctor::msb_binary`] (the `MSB_PATH` convention);
/// `MSB_HOME` is honored implicitly by msb itself. Program paths are
/// injectable for hermetic failure-mapping tests.
pub struct MsbCliLoader {
    gunzip: PathBuf,
    msb: PathBuf,
}

impl MsbCliLoader {
    pub fn new() -> Self {
        Self {
            gunzip: PathBuf::from("gunzip"),
            msb: PathBuf::from(crate::commands::doctor::msb_binary()),
        }
    }

    pub fn with_programs(gunzip: impl Into<PathBuf>, msb: impl Into<PathBuf>) -> Self {
        Self {
            gunzip: gunzip.into(),
            msb: msb.into(),
        }
    }
}

impl Default for MsbCliLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageLoader for MsbCliLoader {
    fn load(&mut self, out_path: &Path, tag: &str) -> Result<(), LoadError> {
        let mut gunzip = Command::new(&self.gunzip)
            .arg("-c")
            .arg(out_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| LoadError::GunzipFailed {
                out_path: out_path.display().to_string(),
                detail: format!("failed to spawn gunzip: {e}"),
            })?;
        let tarball = gunzip
            .stdout
            .take()
            .ok_or_else(|| LoadError::GunzipFailed {
                out_path: out_path.display().to_string(),
                detail: "gunzip stdout was not piped".to_string(),
            })?;
        let msb = Command::new(&self.msb)
            .args(["load", "-t", tag])
            .stdin(Stdio::from(tarball))
            // stdout inherited: msb's "✓ Loaded <tag>" line is operator-facing.
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    LoadError::MsbAbsent {
                        program: self.msb.display().to_string(),
                    }
                } else {
                    LoadError::Failed {
                        tag: tag.to_string(),
                        detail: format!("failed to spawn msb: {e}"),
                    }
                }
            })?;
        // `wait_with_output` collects the piped stderr while waiting; msb's
        // stdin (the gunzip pipe) is closed by the spawn handoff, so gunzip
        // sees EOF/SIGPIPE as msb exits. Wait msb FIRST (it is the verdict
        // that matters), then reap gunzip.
        let output = msb.wait_with_output().map_err(|e| LoadError::Failed {
            tag: tag.to_string(),
            detail: format!("failed to wait on msb: {e}"),
        })?;
        let gunzip_status = gunzip.wait().map_err(|e| LoadError::GunzipFailed {
            out_path: out_path.display().to_string(),
            detail: format!("failed to wait on gunzip: {e}"),
        })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(LoadError::Failed {
                tag: tag.to_string(),
                detail: if stderr.is_empty() {
                    format!("exit status {}", output.status)
                } else {
                    stderr
                },
            });
        }
        if !gunzip_status.success() {
            return Err(LoadError::GunzipFailed {
                out_path: out_path.display().to_string(),
                detail: format!("gunzip exited with {gunzip_status}"),
            });
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// msb tag removal (ADR 0032 §Image tags — keep-last-N GC, RESOLVED decision 3)
// ---------------------------------------------------------------------------

/// Failure modes of the `msb` tag-removal stage. `Clone` so fakes can queue
/// responses. Style mirrors [`LoadError`]: named variants carrying the tag,
/// with Display impls surfacing msb's own diagnosis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoveError {
    /// msb refused to remove the tag (e.g. a backend-level refusal for a
    /// reference it considers protected); `detail` carries msb's own
    /// explanation.
    Refused { tag: String, detail: String },
    /// The msb store could not be reached (or the SDK call failed for a
    /// non-not-found reason); `source` carries the error text.
    Unreachable { tag: String, source: String },
}

impl std::fmt::Display for RemoveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RemoveError::Refused { tag, detail } => {
                write!(f, "msb refused to remove image tag '{tag}': {detail}")
            }
            RemoveError::Unreachable { tag, source } => write!(
                f,
                "msb image store unreachable while removing tag '{tag}' \
                 (db unreachable: {source}) — check that msb is installed and its \
                 database is readable (MSB_HOME), then retry",
                tag = tag,
                source = source
            ),
        }
    }
}

impl std::error::Error for RemoveError {}

/// The msb tag-removal seam (ADR 0032 §Image tags — the keep-last-N GC
/// cascade's process seam, next to [`ImageLoader`]). Async because the real
/// backend is the async microsandbox SDK; `&mut self` so fakes can record
/// calls.
pub trait ImageRemover {
    fn remove_tag(
        &mut self,
        tag: &str,
    ) -> impl std::future::Future<Output = Result<(), RemoveError>> + Send;
}

/// Real backend: `microsandbox::Image::remove(tag, false)` — never forced
/// (a referenced image is msb's business, not ours). Mapping:
/// `MicrosandboxError::ImageNotFound(_)` → `Ok(())` (already gone — removal
/// is idempotent); every other error → [`RemoveError::Unreachable`] with the
/// SDK's own text.
pub struct MsbCliRemover;

impl MsbCliRemover {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MsbCliRemover {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageRemover for MsbCliRemover {
    async fn remove_tag(&mut self, tag: &str) -> Result<(), RemoveError> {
        match microsandbox::Image::remove(tag, false).await {
            Ok(()) => Ok(()),
            Err(microsandbox::MicrosandboxError::ImageNotFound(_)) => Ok(()),
            Err(e) => Err(RemoveError::Unreachable {
                tag: tag.to_string(),
                source: e.to_string(),
            }),
        }
    }
}

// ---------------------------------------------------------------------------
// The pipeline (stages 1–4)
// ---------------------------------------------------------------------------

/// The phase-D pipeline entry point. Runs stages 1–4 (module docs) with the
/// caller's per-tag [`crate::images::lock::ImageTagLock`] still held — the
/// build verb acquires it before change detection and holds it across this
/// call (spec §3.3: the lock spans eval → build → load → record).
pub async fn run_build_pipeline<B: ImageBuilder, L: ImageLoader, P: StoreProbe>(
    job: &BuildJob,
    state_dir: &Path,
    builder: &mut B,
    loader: &mut L,
    probe: &mut P,
) -> Result<PipelineOutcome> {
    // Stage 1: nix build. NixAbsent maps to the SAME §7 wording the phase-C
    // eval ladder uses for its hard-error row ("install nix / config-repo
    // ritual") — build and --check share the nix-absent vocabulary.
    let out_path = builder
        .build_out_path(&job.repo.flake_root, &job.attr)
        .map_err(|e| match e {
            BuildError::NixAbsent => anyhow::anyhow!(
                "nix is required to build '{}': nix was not found on PATH — install nix, \
                 or load the image manually via the config-repo ritual (the declaring \
                 repo's 'load-images' recipe), then retry (spec 21 §7)",
                job.tag
            ),
            other => anyhow::anyhow!("{other}"),
        })?;
    let built_prov = Provenance::capture();

    // Stage 2: the outPath re-load gate (spec §3.1). The state is re-loaded
    // FRESH here (inside the lock) — the caller's copy predates the build.
    let key = image_key(&job.repo.name, &job.tag);
    let state = ImagesState::load(state_dir);
    let record_out = state
        .lookup(&key)
        .map(|r| r.out_path.clone())
        .unwrap_or_default();
    let store = probe
        .tag_state(&job.tag)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let decision = reload_decision(&record_out, &out_path, store);

    // Stage 3: msb load + post-load store verification.
    let action = match decision {
        ReloadDecision::SkipLoad => LoadAction::AlreadyCurrent,
        ReloadDecision::Load => {
            loader
                .load(Path::new(&out_path), &job.tag)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            match probe
                .tag_state(&job.tag)
                .await
                .map_err(|e| anyhow::anyhow!("{e}"))?
            {
                StoreTag::Present => LoadAction::Loaded,
                StoreTag::Gone => anyhow::bail!(
                    "msb load reported success but tag '{}' is not in the store — the msb \
                     store is ground truth (spec 21 §3.2); inspect the store ('msb image \
                     ls') and retry the load",
                    job.tag
                ),
            }
        }
    };
    // `loaded_at`: after the load on the Load path; on the gate-skip path,
    // after the store probe that verified the tag still present — the
    // load-equivalent guarantee the record tracks (no NEW bits were loaded,
    // but the tag was confirmed current at this time).
    let loaded_prov = Provenance::capture();

    // Stage 4: record upsert + save INSIDE the caller's lock (spec §3.3
    // atomicity). `digest = None`: the §3.5 upgrade hook — msb 0.5.6 exposes
    // a manifest digest (§11 HOST-VERIFY item 1 verified in-container), but
    // capturing it here is deferred until the digest-COMPARISON design (the
    // §3.5 one-way signal in the D1 trust branch) lands; this write site is
    // the documented probe point.
    //
    // A2 (ADR 0032 §Image tags): the same locked critical section ALSO moves
    // the state-dir current-pointer for (repo, attr, tag_ctx) to the
    // just-loaded immutable tag — record + pointer upsert are atomic under
    // the lock, so plan/spawn-time resolution never observes a record
    // without its pointer.
    let mut state = state;
    state.upsert(
        key,
        ImageRecord {
            repo: job.repo.clone(),
            attr: job.attr.clone(),
            tag: job.tag.clone(),
            drv_path: job.drv_path.clone(),
            out_path: out_path.clone(),
            digest: None,
            built_at: built_prov.now,
            loaded_at: loaded_prov.now.clone(),
            loader: loaded_prov.loader,
            host: loaded_prov.host,
            user: loaded_prov.user,
        },
    );
    state.upsert_pointer(
        crate::images::state::pointer_key(&job.repo.name, &job.attr, job.tag_ctx.as_deref()),
        crate::images::state::PointerRecord {
            tag: job.tag.clone(),
            updated_at: loaded_prov.now,
        },
    );
    state.save(state_dir)?;

    Ok(PipelineOutcome { out_path, action })
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
pub mod test_fakes {
    //! Fakes shared by `pipeline.rs` and `build_cmd.rs` tests (`pub` under
    //! `cfg(test)` so the sibling module's tests reuse them) — the
    //! detect.rs fake pattern applied to the two phase-D process seams.

    use super::*;
    use std::collections::VecDeque;

    /// Queue-driven fake builder: each call pops one queued response and
    /// records the (flake_root, attr) call.
    pub struct FakeBuilder {
        pub responses: VecDeque<Result<String, BuildError>>,
        pub calls: Vec<(PathBuf, String)>,
    }

    impl FakeBuilder {
        pub fn new() -> Self {
            Self {
                responses: VecDeque::new(),
                calls: Vec::new(),
            }
        }

        pub fn push_ok(&mut self, out_path: &str) {
            self.responses.push_back(Ok(out_path.to_string()));
        }

        pub fn push_err(&mut self, err: BuildError) {
            self.responses.push_back(Err(err));
        }
    }

    impl Default for FakeBuilder {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ImageBuilder for FakeBuilder {
        fn build_out_path(&mut self, flake_root: &Path, attr: &str) -> Result<String, BuildError> {
            self.calls
                .push((flake_root.to_path_buf(), attr.to_string()));
            self.responses
                .pop_front()
                .expect("FakeBuilder: no queued response")
        }
    }

    /// Queue-driven fake loader.
    pub struct FakeLoader {
        pub responses: VecDeque<Result<(), LoadError>>,
        pub calls: Vec<(PathBuf, String)>,
    }

    impl FakeLoader {
        pub fn new() -> Self {
            Self {
                responses: VecDeque::new(),
                calls: Vec::new(),
            }
        }

        pub fn push_ok(&mut self) {
            self.responses.push_back(Ok(()));
        }

        pub fn push_err(&mut self, err: LoadError) {
            self.responses.push_back(Err(err));
        }
    }

    impl Default for FakeLoader {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ImageLoader for FakeLoader {
        fn load(&mut self, out_path: &Path, tag: &str) -> Result<(), LoadError> {
            self.calls.push((out_path.to_path_buf(), tag.to_string()));
            self.responses
                .pop_front()
                .expect("FakeLoader: no queued response")
        }
    }

    /// Queue-driven fake remover (the A2 GC seam): each call pops one queued
    /// response and records the tag. An EMPTY queue answers `Ok(())` — the
    /// common "removal succeeds" case needs no boilerplate.
    pub struct FakeRemover {
        pub responses: VecDeque<Result<(), RemoveError>>,
        pub calls: Vec<String>,
    }

    impl FakeRemover {
        pub fn new() -> Self {
            Self {
                responses: VecDeque::new(),
                calls: Vec::new(),
            }
        }

        pub fn push_ok(&mut self) {
            self.responses.push_back(Ok(()));
        }

        pub fn push_err(&mut self, err: RemoveError) {
            self.responses.push_back(Err(err));
        }
    }

    impl Default for FakeRemover {
        fn default() -> Self {
            Self::new()
        }
    }

    impl ImageRemover for FakeRemover {
        async fn remove_tag(&mut self, tag: &str) -> Result<(), RemoveError> {
            self.calls.push(tag.to_string());
            match self.responses.pop_front() {
                Some(r) => r,
                None => Ok(()),
            }
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
    use super::super::detect::test_fakes::FakeStoreProbe;
    use super::test_fakes::*;
    use super::*;
    use crate::config::test_support::unique_state_dir;

    fn job_fixture() -> BuildJob {
        BuildJob {
            workload: "pi".to_string(),
            repo: RepoIdentity {
                name: "personal".to_string(),
                path: PathBuf::from("/tmp/repo"),
                flake_root: PathBuf::from("/tmp/repo"),
            },
            attr: "workestrate-pi".to_string(),
            tag: "workestrate-pi:latest".to_string(),
            tag_ctx: None,
            drv_path: "/nix/store/abc-workestrate-pi.tar.gz.drv".to_string(),
            force: false,
        }
    }

    // ---- reload_decision (the §3.1 re-load gate, pure) ----

    /// Exhaustive gate table: record_out empty / differs / matches × store
    /// present / gone.
    #[test]
    fn reload_decision_table_matches_spec_3_1() {
        // record_out empty (phase-C trust record) → load, either store state.
        assert_eq!(
            reload_decision("", "/nix/store/out-A", StoreTag::Present),
            ReloadDecision::Load
        );
        assert_eq!(
            reload_decision("", "/nix/store/out-A", StoreTag::Gone),
            ReloadDecision::Load
        );
        // record_out differs → load, either store state.
        assert_eq!(
            reload_decision("/nix/store/out-A", "/nix/store/out-B", StoreTag::Present),
            ReloadDecision::Load
        );
        assert_eq!(
            reload_decision("/nix/store/out-A", "/nix/store/out-B", StoreTag::Gone),
            ReloadDecision::Load
        );
        // record_out matches + tag present → SKIP the load (the gate).
        assert_eq!(
            reload_decision("/nix/store/out-A", "/nix/store/out-A", StoreTag::Present),
            ReloadDecision::SkipLoad
        );
        // record_out matches + tag GONE → load anyway (out-of-band deletion).
        assert_eq!(
            reload_decision("/nix/store/out-A", "/nix/store/out-A", StoreTag::Gone),
            ReloadDecision::Load,
            "out-of-band deletion forces the load even when outPaths match"
        );
    }

    // ---- classify_build_stderr (the §7 rows, pure over fixture stderr) ----

    /// Realistic nix fixed-output hash-mismatch stderr (the fakeHash row).
    const FAKEHASH_STDERR: &str = r#"error: hash mismatch in fixed-output derivation '/nix/store/xyz-npm-deps.drv':
         specified: sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=
            got:    sha256-9mB2HwD2BTz3vEpNmKuOp2ObqTzpqL9n0fEwO8W0k9E=
"#;

    #[test]
    fn fakehash_hash_mismatch_classifies_as_placeholder_row() {
        assert_eq!(
            classify_build_stderr(FAKEHASH_STDERR),
            NixFailureClass::FakeHashPlaceholder
        );
        // The extracted detail keeps the got/specified lines, not the wall.
        let detail = hash_mismatch_lines(FAKEHASH_STDERR);
        assert!(detail.contains("specified: sha256-"), "{detail}");
        assert!(detail.contains("got:"), "{detail}");
    }

    #[test]
    fn offline_and_substituter_failures_classify_as_offline_row() {
        for marker in [
            "error: unable to download 'https://cache.nixos.org/xyz': Couldn't resolve host name",
            "curl: (6) Couldn't resolve host 'github.com'",
            "error: Temporary failure in name resolution",
            "error: Failed to fetch archive ...",
        ] {
            assert_eq!(
                classify_build_stderr(marker),
                NixFailureClass::OfflineFetch,
                "marker: {marker}"
            );
        }
    }

    #[test]
    fn generic_build_failure_is_the_default_class() {
        let stderr = "error: builder for '/nix/store/xyz.drv' failed with exit code 1\n\
                      last 10 log lines:\n> make: *** [Makefile:2: all] Error 1";
        assert_eq!(classify_build_stderr(stderr), NixFailureClass::Generic);
        // stderr_tail keeps the LAST N lines.
        let tail = stderr_tail("l1\nl2\nl3\nl4", 2);
        assert_eq!(tail, "l3\nl4");
    }

    // ---- parse_out_path ----

    #[test]
    fn parse_out_path_picks_the_last_store_line() {
        let stdout = "/nix/store/aaa-first\n/nix/store/bbb-last.tar.gz\n";
        assert_eq!(
            parse_out_path(stdout).as_deref(),
            Some("/nix/store/bbb-last.tar.gz")
        );
        assert_eq!(parse_out_path("no store paths here\n"), None);
    }

    // ---- the pipeline over fakes (mock-free, per repo convention) ----

    /// Full happy path: build → gate (record absent → load) → load →
    /// post-load verify → record upsert with the phase-D field shape.
    #[tokio::test]
    async fn build_load_record_happy_path() -> Result<()> {
        let state_dir = unique_state_dir("pipe-happy");
        let job = job_fixture();
        let mut builder = FakeBuilder::new();
        builder.push_ok("/nix/store/def-workestrate-pi.tar.gz");
        let mut loader = FakeLoader::new();
        loader.push_ok();
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Gone); // pre-gate probe: nothing in the store yet
        probe.push(StoreTag::Present); // post-load verification

        let outcome =
            run_build_pipeline(&job, &state_dir, &mut builder, &mut loader, &mut probe).await?;

        assert_eq!(outcome.out_path, "/nix/store/def-workestrate-pi.tar.gz");
        assert_eq!(outcome.action, LoadAction::Loaded);
        assert_eq!(
            loader.calls,
            vec![(
                PathBuf::from("/nix/store/def-workestrate-pi.tar.gz"),
                "workestrate-pi:latest".to_string()
            )]
        );

        let key = image_key("personal", "workestrate-pi:latest");
        let state = ImagesState::load(&state_dir);
        let record = state
            .lookup(&key)
            .expect("record written inside the pipeline");
        assert_eq!(record.repo.name, "personal");
        assert_eq!(record.attr, "workestrate-pi");
        assert_eq!(record.tag, "workestrate-pi:latest");
        assert_eq!(record.drv_path, job.drv_path, "drv_path = the job's eval");
        assert_eq!(record.out_path, outcome.out_path, "out_path = realized");
        assert_eq!(record.digest, None, "§3.5 probe point stays null");
        assert!(record.loader.starts_with("workestrate "));
        assert!(!record.host.is_empty() && !record.user.is_empty());
        assert!(
            record.built_at.len() == 20 && record.built_at.ends_with('Z'),
            "rfc3339 provenance: {}",
            record.built_at
        );
        assert!(record.loaded_at.len() == 20 && record.loaded_at.ends_with('Z'));

        // A2 (ADR 0032 §Image tags): the stage-4 upsert ALSO moved the
        // current-pointer for (repo, attr, ctx=None) to the loaded tag —
        // atomically under the same lock, with the loaded_at stamp.
        let pointer = state
            .lookup_pointer(&crate::images::state::pointer_key(
                "personal",
                "workestrate-pi",
                None,
            ))
            .expect("pointer upserted alongside the record");
        assert_eq!(pointer.tag, "workestrate-pi:latest");
        assert_eq!(pointer.updated_at, record.loaded_at);
        assert!(
            pointer.updated_at.len() == 20 && pointer.updated_at.ends_with('Z'),
            "rfc3339 stamp: {}",
            pointer.updated_at
        );

        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// A2: a tag-context job (e.g. an armed inline override `prime:feat-x`)
    /// moves ONLY the `(name, ctx)` pointer — the home context's pointer
    /// (ctx=None) is untouched.
    #[tokio::test]
    async fn ctx_tagged_job_moves_only_the_ctx_pointer() -> Result<()> {
        let state_dir = unique_state_dir("pipe-ctx-pointer");
        let mut job = job_fixture();
        job.tag = "workestrate-pi:feat-x.abcdefghijkl".to_string();
        job.tag_ctx = Some("feat-x".to_string());
        // Seed a home-context pointer; it must survive the ctx build.
        let mut state = ImagesState::default();
        state.upsert_pointer(
            crate::images::state::pointer_key("personal", "workestrate-pi", None),
            crate::images::state::PointerRecord {
                tag: "workestrate-pi:000000000000".to_string(),
                updated_at: "2026-08-24T09:00:00Z".to_string(),
            },
        );
        state.save(&state_dir)?;

        let mut builder = FakeBuilder::new();
        builder.push_ok("/nix/store/abcdefghijklmnopqrstuvwxyz012345-workestrate-pi.tar.gz");
        let mut loader = FakeLoader::new();
        loader.push_ok();
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Gone);
        probe.push(StoreTag::Present);

        run_build_pipeline(&job, &state_dir, &mut builder, &mut loader, &mut probe).await?;

        let state = ImagesState::load(&state_dir);
        assert_eq!(
            state
                .lookup_pointer(&crate::images::state::pointer_key(
                    "personal",
                    "workestrate-pi",
                    Some("feat-x")
                ))
                .map(|p| p.tag.as_str()),
            Some("workestrate-pi:feat-x.abcdefghijkl"),
            "the override-ctx pointer moved to the fresh tag"
        );
        assert_eq!(
            state
                .lookup_pointer(&crate::images::state::pointer_key(
                    "personal",
                    "workestrate-pi",
                    None
                ))
                .map(|p| p.tag.as_str()),
            Some("workestrate-pi:000000000000"),
            "the home-context pointer NEVER flaps under an override build"
        );

        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// Gate-skip: the record's outPath already equals the realized one and
    /// the tag is present → NO loader call, the record is still upserted
    /// (drv_path refreshes), outcome is AlreadyCurrent.
    #[tokio::test]
    async fn outpath_gate_skips_the_load_but_refreshes_the_record() -> Result<()> {
        let state_dir = unique_state_dir("pipe-gate-skip");
        let job = job_fixture();
        // Seed a phase-D record with the SAME outPath the builder will
        // realize (drv churned: drv-B recorded, job carries drv-C eval).
        let key = image_key("personal", "workestrate-pi:latest");
        let mut state = ImagesState::default();
        state.upsert(
            key.clone(),
            ImageRecord {
                repo: job.repo.clone(),
                attr: job.attr.clone(),
                tag: job.tag.clone(),
                drv_path: "/nix/store/drv-B.drv".to_string(),
                out_path: "/nix/store/def-workestrate-pi.tar.gz".to_string(),
                digest: None,
                built_at: "2026-08-02T10:15:00Z".to_string(),
                loaded_at: "2026-08-02T10:16:12Z".to_string(),
                loader: "workestrate 0.1.0".to_string(),
                host: "devbox".to_string(),
                user: "node".to_string(),
            },
        );
        state.save(&state_dir)?;

        let mut builder = FakeBuilder::new();
        builder.push_ok("/nix/store/def-workestrate-pi.tar.gz");
        let mut loader = FakeLoader::new(); // no responses queued: must NOT be called
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Present);

        let outcome =
            run_build_pipeline(&job, &state_dir, &mut builder, &mut loader, &mut probe).await?;

        assert_eq!(outcome.action, LoadAction::AlreadyCurrent);
        assert!(loader.calls.is_empty(), "the gate skipped msb load");
        let record = ImagesState::load(&state_dir)
            .lookup(&key)
            .expect("record still upserted on the gate-skip path")
            .clone();
        assert_eq!(
            record.drv_path, job.drv_path,
            "drv_path refreshes to the current eval"
        );
        assert_eq!(record.out_path, "/nix/store/def-workestrate-pi.tar.gz");

        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// Out-of-band deletion: record outPath matches but the tag is GONE →
    /// load anyway (the gate's store probe is what forces it).
    #[tokio::test]
    async fn tag_gone_out_of_band_loads_anyway() -> Result<()> {
        let state_dir = unique_state_dir("pipe-oob-delete");
        let job = job_fixture();
        let key = image_key("personal", "workestrate-pi:latest");
        let mut state = ImagesState::default();
        state.upsert(
            key,
            ImageRecord {
                repo: job.repo.clone(),
                attr: job.attr.clone(),
                tag: job.tag.clone(),
                drv_path: job.drv_path.clone(),
                out_path: "/nix/store/def-workestrate-pi.tar.gz".to_string(),
                digest: None,
                built_at: "2026-08-02T10:15:00Z".to_string(),
                loaded_at: "2026-08-02T10:16:12Z".to_string(),
                loader: "workestrate 0.1.0".to_string(),
                host: "devbox".to_string(),
                user: "node".to_string(),
            },
        );
        state.save(&state_dir)?;

        let mut builder = FakeBuilder::new();
        builder.push_ok("/nix/store/def-workestrate-pi.tar.gz");
        let mut loader = FakeLoader::new();
        loader.push_ok();
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Gone); // pre-gate: out-of-band deletion
        probe.push(StoreTag::Present); // post-load verification

        let outcome =
            run_build_pipeline(&job, &state_dir, &mut builder, &mut loader, &mut probe).await?;

        assert_eq!(outcome.action, LoadAction::Loaded, "loads anyway");
        assert_eq!(loader.calls.len(), 1);

        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// Post-load verification: `msb load` succeeded but the tag is still
    /// gone → the named error, and NO record is written.
    #[tokio::test]
    async fn successful_load_with_tag_still_gone_is_a_named_error() -> Result<()> {
        let state_dir = unique_state_dir("pipe-phantom-load");
        let job = job_fixture();
        let mut builder = FakeBuilder::new();
        builder.push_ok("/nix/store/def-workestrate-pi.tar.gz");
        let mut loader = FakeLoader::new();
        loader.push_ok();
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Gone);
        probe.push(StoreTag::Gone); // STILL gone after the load

        let err = run_build_pipeline(&job, &state_dir, &mut builder, &mut loader, &mut probe)
            .await
            .expect_err("phantom load must fail");
        let msg = err.to_string();
        assert!(
            msg.contains(
                "msb load reported success but tag 'workestrate-pi:latest' is not in the store"
            ),
            "{msg}"
        );
        assert!(
            !crate::images::state::images_state_path(&state_dir).exists(),
            "no record is written when the store verification fails"
        );

        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// §7 load failure: msb's stderr surfaces in the named error.
    #[tokio::test]
    async fn load_failure_surfaces_msb_stderr() -> Result<()> {
        let state_dir = unique_state_dir("pipe-load-fail");
        let job = job_fixture();
        let mut builder = FakeBuilder::new();
        builder.push_ok("/nix/store/def-workestrate-pi.tar.gz");
        let mut loader = FakeLoader::new();
        loader.push_err(LoadError::Failed {
            tag: "workestrate-pi:latest".to_string(),
            detail: "invalid tar header".to_string(),
        });
        let mut probe = FakeStoreProbe::new();
        probe.push(StoreTag::Gone);

        let err = run_build_pipeline(&job, &state_dir, &mut builder, &mut loader, &mut probe)
            .await
            .expect_err("load failure propagates");
        let msg = err.to_string();
        assert!(
            msg.contains("msb load failed for tag 'workestrate-pi:latest'"),
            "{msg}"
        );
        assert!(msg.contains("invalid tar header"), "msb's stderr: {msg}");

        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// Nix-absent unification: the pipeline's own nix spawn maps NotFound to
    /// the SAME §7 wording as the phase-C eval ladder.
    #[tokio::test]
    async fn nix_absent_maps_to_the_shared_section7_vocabulary() -> Result<()> {
        let state_dir = unique_state_dir("pipe-nix-absent");
        let job = job_fixture();
        let mut builder = FakeBuilder::new();
        builder.push_err(BuildError::NixAbsent);
        let mut loader = FakeLoader::new();
        let mut probe = FakeStoreProbe::new();

        let err = run_build_pipeline(&job, &state_dir, &mut builder, &mut loader, &mut probe)
            .await
            .expect_err("nix absent fails the pipeline");
        let msg = err.to_string();
        assert!(msg.contains("nix is required to build"), "{msg}");
        assert!(msg.contains("install nix"), "remediation: {msg}");
        assert!(
            msg.contains("load-images"),
            "config-repo ritual pointer: {msg}"
        );
        assert!(
            msg.contains("workestrate-pi:latest"),
            "names the tag: {msg}"
        );
        assert!(probe.calls.is_empty(), "the store is never probed");

        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// The §7 fakeHash error points at the declaring repo's `update-hashes`
    /// recipe, NOT a raw nix error wall.
    #[test]
    fn fakehash_error_points_at_update_hashes_recipe() {
        let err = BuildError::FakeHashPlaceholder {
            reference: "/repo#workestrate-pi".to_string(),
            detail: "specified: sha256-AAA…\ngot: sha256-BBB…".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("update-hashes"), "{msg}");
        assert!(msg.contains("placeholder hash"), "{msg}");
        assert!(msg.contains("spec 21 §7"), "{msg}");
    }

    #[test]
    fn offline_error_notes_the_offline_context() {
        let err = BuildError::OfflineFetch {
            reference: "/repo#workestrate-pi".to_string(),
            detail: "…".to_string(),
        };
        let msg = err.to_string();
        assert!(msg.contains("offline"), "{msg}");
        assert!(msg.contains("spec 21 §7"), "{msg}");
    }

    // ---- real-backend error mapping (hermetic) ----

    /// An unspawnable nix maps to BuildError::NixAbsent (the caller's §7
    /// mapping keys on this variant).
    #[test]
    fn unspawnable_nix_maps_to_nix_absent() {
        let mut builder = NixCliBuilder::with_program("/definitely/not/on/path/nix");
        match builder.build_out_path(Path::new("/tmp"), "attr") {
            Err(BuildError::NixAbsent) => {}
            other => panic!("expected NixAbsent, got {other:?}"),
        }
    }

    /// A failing msb maps to the named load failure with its stderr; an
    /// unspawnable msb maps to MsbAbsent with the MSB_PATH remediation.
    #[test]
    fn msb_failure_modes_map_to_named_errors() {
        let out_path = Path::new("/definitely/not/a/tarball.tar.gz");

        let mut loader = MsbCliLoader::with_programs("/bin/true", "/bin/false");
        match loader.load(out_path, "t:latest") {
            Err(LoadError::Failed { tag, .. }) => assert_eq!(tag, "t:latest"),
            other => panic!("expected Failed, got {other:?}"),
        }

        let mut loader = MsbCliLoader::with_programs("/bin/true", "/definitely/not/msb");
        match loader.load(out_path, "t:latest") {
            Err(LoadError::MsbAbsent { program }) => {
                assert_eq!(program, "/definitely/not/msb")
            }
            other => panic!("expected MsbAbsent, got {other:?}"),
        }

        let mut loader = MsbCliLoader::with_programs("/bin/false", "/bin/true");
        match loader.load(out_path, "t:latest") {
            Err(LoadError::GunzipFailed { .. }) => {}
            other => panic!("expected GunzipFailed, got {other:?}"),
        }
    }
}
