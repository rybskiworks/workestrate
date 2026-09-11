//! Explicit host-only integration of the retained adapter and broker transport.
//! Ordinary Nix tests compile this fixture but never boot its VM. The selected
//! packaged library-test executable is run by a separately reviewed resource and
//! descendant supervisor, with a disposable spec and no ambient credentials.

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use anyhow::{Context, Result, ensure};
use microsandbox::backend::LocalBackend;
use microsandbox::image::Image;
use microsandbox::sandbox::{PullPolicy, SecurityProfile};
use serde::Deserialize;
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use tokio::time::{Instant, timeout, timeout_at};

use super::super::authorization::AuthenticatedCaller;
use super::super::broker_link::BrokerLink;
use super::super::custody::SshController;
use super::super::dispatcher::ControlDispatcher;
use super::super::types::{ControlRequest, ControlResponse, ExecRequest, ExecState, ExecStatus};
use crate::microsandbox::broker::host_trust::BrokerHostPrincipal;
use crate::microsandbox::plan::CredentialsPlan;

const SYSTEMCTL: &str = "/run/current-system/systemd/bin/systemctl";
const JOURNALCTL: &str = "/run/current-system/systemd/bin/journalctl";
const SERVICE: &str = "workestrate-broker.service";
const NAME: &str = "native-broker-fixture";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VmSpec {
    version: u32,
    root: PathBuf,
    archive: PathBuf,
    image_reference: String,
    principal: String,
}

impl VmSpec {
    fn validate_shape(&self) -> Result<()> {
        ensure!(self.version == 1, "unsupported fixture spec");
        ensure!(
            self.root.parent() == Some(Path::new("/tmp")),
            "fixture root must be directly under /tmp"
        );
        let name = self
            .root
            .file_name()
            .and_then(|value| value.to_str())
            .context("invalid root")?;
        let suffix = name
            .strip_prefix("broker-vm.")
            .context("wrong fixture root prefix")?;
        ensure!(
            suffix.len() >= 6 && suffix.bytes().all(|byte| byte.is_ascii_alphanumeric()),
            "invalid fixture root nonce"
        );
        ensure!(
            self.archive == self.root.join("image.tar"),
            "archive must be the owned plain tar"
        );
        let tag = self
            .image_reference
            .strip_prefix("localhost:9/workestrate-broker:")
            .context("fixture image must use a loopback-only tag")?;
        ensure!(
            !tag.is_empty()
                && tag.len() <= 64
                && tag
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'),
            "invalid fixture image tag"
        );
        ensure!(
            self.principal == "broker.workestrate.internal",
            "unexpected broker principal"
        );
        Ok(())
    }

    fn read() -> Result<Self> {
        let path =
            std::env::var_os("WORKESTRATE_BROKER_VM_SPEC").context("explicit VM spec required")?;
        let bytes = read_small_file(Path::new(&path))?;
        let spec: Self = serde_json::from_slice(&bytes)?;
        spec.validate_shape()?;
        ensure!(
            spec.root.canonicalize()? == spec.root,
            "fixture root must be canonical"
        );
        // SAFETY: geteuid has no pointer arguments or memory-safety preconditions.
        #[allow(unsafe_code)]
        let uid = unsafe { libc::geteuid() };
        private_listener_directory(&std::fs::symlink_metadata(&spec.root)?, uid)?;
        ensure!(
            std::env::var_os("MSB_HOME") == Some(spec.root.join("msb").into_os_string()),
            "MSB_HOME is not the private fixture home"
        );
        ensure!(
            std::env::var_os("MSB_CONFIG_PATH")
                == Some(spec.root.join("msb.json").into_os_string()),
            "MSB_CONFIG_PATH is not private"
        );
        let config = read_small_file(&spec.root.join("msb.json"))?;
        ensure!(
            serde_json::from_slice::<serde_json::Value>(&config)? == serde_json::json!({}),
            "fixture config must be empty"
        );
        let runtime = Path::new(
            option_env!("MSB_BUILD_RUNTIME").context("fixture lacks immutable compiled runtime")?,
        );
        ensure!(
            std::env::var_os("MSB_PATH") == Some(runtime.join("bin/msb").into_os_string()),
            "runtime wrapper mismatch"
        );
        ensure!(
            std::env::var_os("MSB_AGENTD_PATH")
                == Some(runtime.join("libexec/agentd").into_os_string()),
            "agentd wrapper mismatch"
        );
        let archive = std::fs::symlink_metadata(&spec.archive)?;
        ensure!(
            archive.is_file()
                && archive.nlink() == 1
                && archive.uid() == uid
                && archive.len() > 0
                && archive.len() <= 4 * 1024 * 1024 * 1024,
            "invalid owned plain archive"
        );
        for relative in ["msb", "run/bad", "run/good", "bootstrap/good"] {
            let directory = spec.root.join(relative);
            ensure!(
                directory.canonicalize()? == directory,
                "aliased fixture directory"
            );
            private_listener_directory(&std::fs::symlink_metadata(&directory)?, uid)?;
            ensure!(
                std::fs::read_dir(directory)?.next().is_none(),
                "fixture directory is not empty"
            );
        }
        Ok(spec)
    }
}

fn read_small_file(path: &Path) -> Result<Vec<u8>> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)?;
    ensure!(
        file.metadata()?.is_file(),
        "fixture input is not a regular file"
    );
    let mut bytes = Vec::new();
    file.take(4097).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 4096, "fixture input too large");
    Ok(bytes)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn bootstrap(spec: &VmSpec) -> Result<()> {
    use ssh_key::{Algorithm, LineEnding, PrivateKey};
    let ca = PrivateKey::random(&mut ssh_key::rand_core::OsRng, Algorithm::Ed25519)?;
    let host = PrivateKey::random(&mut ssh_key::rand_core::OsRng, Algorithm::Ed25519)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let issued = BrokerHostPrincipal::new(&spec.principal)?.issue(
        &ca,
        host.public_key(),
        1,
        now.saturating_sub(60),
        now + 3600,
    )?;
    let directory = spec.root.join("bootstrap/good");
    write_new(
        &directory.join("host-key"),
        host.to_openssh(LineEnding::LF)?.as_bytes(),
    )?;
    write_new(
        &directory.join("host-certificate"),
        issued.certificate().as_bytes(),
    )?;
    write_new(
        &directory.join("host-ca.pub"),
        issued.ca_public_key().as_bytes(),
    )?;
    ensure!(
        std::fs::read_dir(directory)?.count() == 3,
        "unexpected bootstrap file"
    );
    // The CA private key is never serialized or mounted. Its ordinary owner
    // drops it here; only the unrelated host key enters the readonly mount.
    Ok(())
}

fn instance() -> InstanceRef {
    InstanceRef {
        workload: super::super::types::WorkloadRef {
            context: Some("native-fixture".into()),
            name: "broker".into(),
        },
        instance: "one".into(),
    }
}

struct OwnedVm {
    sandbox: Sandbox,
    egress: std::os::unix::net::UnixListener,
    phase: &'static str,
}

async fn create_vm(spec: &VmSpec, good: bool) -> Result<OwnedVm> {
    let phase = if good { "good" } else { "bad" };
    let directory = spec.root.join("run").join(phase);
    let egress = std::os::unix::net::UnixListener::bind(directory.join("egress.sock"))?;
    egress.set_nonblocking(true)?;
    let mut builder = Sandbox::builder(NAME)
        .image(spec.image_reference.as_str())
        .pull_policy(PullPolicy::Never)
        .cpus(1)
        .memory(2048u32)
        .root_disk(4096u32)
        .max_duration(300)
        .security(SecurityProfile::Default)
        .workdir("/")
        .disable_network()
        .init_with("/init", |init| init.env("container", "microsandbox"))
        .vsock_host_listen(directory.join("management.sock"), 3024)
        .vsock_host_listen(directory.join("divert.sock"), 3022)
        .vsock(directory.join("egress.sock"), 3023);
    if good {
        builder = builder.volume("/broker-credentials", |mount| {
            mount.bind(spec.root.join("bootstrap/good")).readonly()
        });
    }
    // Missing-bootstrap negative deliberately has no mount at all: an absent
    // host bind path would fail before the guest service could be exercised.
    let sandbox = timeout(Duration::from_secs(60), builder.create())
        .await
        .context("VM create deadline")??;
    Ok(OwnedVm {
        sandbox,
        egress,
        phase,
    })
}

async fn stop_current(current: &mut Option<OwnedVm>) -> Result<()> {
    if let Some(owned) = current.as_ref() {
        let launch = owned.sandbox.launch_identity()?;
        let deadline = Instant::now() + Duration::from_secs(45);
        timeout_at(deadline, owned.sandbox.stop())
            .await
            .context("exact VM stop deadline")??;
        // This attached Sandbox owns/reaps the runtime. The outer supervisor
        // cannot observe that same child's normal wait status independently.
        let status = timeout_at(deadline, owned.sandbox.wait())
            .await
            .context("exact VM owner wait deadline")??;
        eprintln!(
            "{}",
            owner_exit_marker(owned.phase, launch.as_bytes(), status)?
        );
        // This receiver method retains the original database/run/process fence;
        // no name-addressed administrative deletion is used.
        timeout(Duration::from_secs(15), owned.sandbox.remove_persisted())
            .await
            .context("exact VM removal deadline")??;
        let no_egress = matches!(owned.egress.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock);
        eprintln!("native-vm: exact stopped runtime removed; no egress connection: {no_egress}");
        current.take();
        ensure!(no_egress, "unexpected guest-to-host egress connection");
    }
    Ok(())
}

fn owner_exit_marker(
    phase: &str,
    launch: &[u8; 32],
    status: std::process::ExitStatus,
) -> Result<String> {
    use std::os::unix::process::ExitStatusExt;
    ensure!(matches!(phase, "bad" | "good"), "unknown fixture phase");
    ensure!(
        launch.iter().any(|byte| *byte != 0),
        "empty launch identity"
    );
    ensure!(
        status.success() && status.code() == Some(0) && status.signal().is_none(),
        "runtime owner observed non-successful termination: {status}"
    );
    let identity: String = launch.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!(
        "native-vm: owner-exit phase={phase} launch={identity} code=0 signal=none"
    ))
}

async fn service_state(sandbox: &Sandbox, expected: &[u8]) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(45);
    loop {
        let (output, stderr) = guest_diagnostic(
            sandbox,
            SYSTEMCTL,
            &["show", "--property=ActiveState", "--value", SERVICE],
        )
        .await?;
        ensure!(stderr.is_empty(), "service diagnostic stderr");
        if output == expected {
            return Ok(());
        }
        ensure!(
            Instant::now() < deadline,
            "systemd service did not reach expected diagnostic state"
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
}

async fn store_ready(sandbox: &Sandbox) -> Result<()> {
    let started = Instant::now();
    eprintln!("native-vm: activation observation started; deadline_seconds=45");
    // Agent transport readiness precedes the parent's /init activation. Only
    // this fixed, read-only probe may retry the enumerated early-boot outcomes.
    wait_for_activation(started + Duration::from_secs(45), || async {
        activation_observation(
            guest_diagnostic(
                sandbox,
                SYSTEMCTL,
                &[
                    "show",
                    "--property=ActiveState",
                    "--value",
                    "guest-store-ready.target",
                ],
            )
            .await,
        )
    })
    .await?;
    eprintln!(
        "native-vm: activation observed; elapsed_ms={}",
        started.elapsed().as_millis()
    );
    // Socket activation need not leave nix-daemon.service already active. A
    // real daemon protocol connection is the acceptance boundary, not its PID.
    let (stdout, stderr) = guest_diagnostic(
        sandbox,
        "/run/current-system/sw/bin/nix",
        &[
            "--extra-experimental-features",
            "nix-command",
            "store",
            "info",
            "--store",
            "daemon",
            "--json",
        ],
    )
    .await?;
    let info: serde_json::Value =
        serde_json::from_slice(&stdout).context("daemon info is not bounded JSON")?;
    eprintln!(
        "native-vm: store target active; daemon info exited zero; json={info}; stderr={:?}",
        String::from_utf8_lossy(&stderr)
    );
    Ok(())
}

#[derive(Debug)]
struct DiagnosticSpawnFailure {
    failure: microsandbox_protocol::exec::ExecFailed,
    before_started: bool,
}

impl std::fmt::Display for DiagnosticSpawnFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "guest diagnostic spawn failure ({:?}, before_started={}): {}",
            self.failure.kind,
            self.before_started,
            bounded_cause(&self.failure.message)
        )
    }
}

impl std::error::Error for DiagnosticSpawnFailure {}

#[derive(Debug)]
struct DiagnosticExitFailure {
    code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

impl std::fmt::Display for DiagnosticExitFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "guest diagnostic exited {}; stdout={:?}; stderr={:?}",
            self.code,
            bounded_cause(&String::from_utf8_lossy(&self.stdout)),
            bounded_cause(&String::from_utf8_lossy(&self.stderr)),
        )
    }
}

impl std::error::Error for DiagnosticExitFailure {}

#[derive(Debug, PartialEq, Eq)]
enum ActivationObservation {
    Ready,
    Pending(String),
}

fn bounded_cause(value: &str) -> String {
    let mut end = value.len().min(512);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value[..end].to_owned()
}

fn activation_observation(result: Result<(Vec<u8>, Vec<u8>)>) -> Result<ActivationObservation> {
    use microsandbox_protocol::exec::ExecFailureKind;
    match result {
        Ok((state, stderr)) => {
            ensure!(stderr.is_empty(), "store target diagnostic stderr");
            match state.as_slice() {
                b"active\n" => Ok(ActivationObservation::Ready),
                b"inactive\n" | b"activating\n" => Ok(ActivationObservation::Pending(format!(
                    "guest-store-ready.target: {}",
                    String::from_utf8_lossy(&state)
                ))),
                _ => anyhow::bail!(
                    "unexpected store target diagnostic: {}",
                    bounded_cause(&String::from_utf8_lossy(&state))
                ),
            }
        }
        Err(error) => {
            // Repeating this fixed, idempotent probe after an observed ordinary
            // nonzero exit is readiness polling, not replay of an uncertain
            // operation. Unknown/negative statuses are not ordinary exit codes.
            if let Some(exited) = error.downcast_ref::<DiagnosticExitFailure>()
                && (1..=255).contains(&exited.code)
            {
                return Ok(ActivationObservation::Pending(bounded_cause(
                    &error.to_string(),
                )));
            }
            if let Some(failed) = error.downcast_ref::<DiagnosticSpawnFailure>() {
                let failure = &failed.failure;
                // The pinned agent's implicit HOME lookup uses 0:0 even when
                // no user is requested. Keep that lookup intact; wait for the
                // real activation to publish account files instead of bypassing it.
                let accounts_pending = failure.kind == ExecFailureKind::Other
                    && failure.errno.is_none()
                    && failure.errno_name.is_none()
                    && failure.stage.is_none()
                    && failure.message
                        == "exec session error: failed to resolve guest uid 0: No such file or directory (os error 2)";
                // /run/current-system is published at activation completion.
                // This exact fixed program and cwd=/ are the only spawn lookup
                // eligible here; unrelated command failures remain fatal.
                let systemctl_pending = failure.kind == ExecFailureKind::NotFound
                    && failure.errno == Some(libc::ENOENT)
                    && failure.errno_name.as_deref() == Some("ENOENT")
                    && failure.stage.as_deref() == Some("Command::spawn")
                    && failure.message
                        == format!("spawn {SYSTEMCTL:?}: No such file or directory (os error 2)");
                if failed.before_started && (accounts_pending || systemctl_pending) {
                    return Ok(ActivationObservation::Pending(bounded_cause(
                        &error.to_string(),
                    )));
                }
            }
            Err(error)
        }
    }
}

async fn wait_for_activation<F, A>(deadline: Instant, mut attempt: F) -> Result<()>
where
    F: FnMut() -> A,
    A: std::future::Future<Output = Result<ActivationObservation>>,
{
    let mut first = None;
    let mut last = None;
    let mut attempts = 0u32;
    loop {
        if Instant::now() >= deadline {
            anyhow::bail!(
                "guest activation deadline; attempts={attempts}; first={first:?}; last={last:?}"
            );
        }
        attempts += 1;
        match timeout_at(deadline, attempt()).await {
            Ok(Ok(ActivationObservation::Ready)) => {
                eprintln!(
                    "native-vm: activation attempts={attempts}; first={first:?}; last={last:?}"
                );
                return Ok(());
            }
            Ok(Ok(ActivationObservation::Pending(cause))) => {
                let cause = bounded_cause(&cause);
                first.get_or_insert_with(|| cause.clone());
                last = Some(cause);
            }
            Ok(Err(error)) => {
                return Err(error.context(format!(
                    "guest activation refused; attempts={attempts}; first={first:?}; last={last:?}"
                )));
            }
            Err(_) => anyhow::bail!(
                "guest activation deadline during probe; attempts={attempts}; first={first:?}; last={last:?}"
            ),
        }
        tokio::time::sleep_until(deadline.min(Instant::now() + Duration::from_millis(200))).await;
    }
}

async fn missing_credentials(sandbox: &Sandbox) -> Result<()> {
    service_state(sandbox, b"failed\n").await?;
    let (properties, stderr) = guest_diagnostic(
        sandbox,
        SYSTEMCTL,
        &[
            "show",
            "--property=LoadState,ActiveState,Result,ExecMainCode,ExecMainStatus",
            SERVICE,
        ],
    )
    .await?;
    ensure!(stderr.is_empty(), "credential property diagnostic stderr");
    let lines: Vec<_> = std::str::from_utf8(&properties)?.lines().collect();
    let expected = [
        "LoadState=loaded",
        "ActiveState=failed",
        "Result=exit-code",
        "ExecMainCode=1",
        "ExecMainStatus=243",
    ];
    ensure!(
        lines.len() == expected.len()
            && expected
                .iter()
                .all(|value| lines.iter().filter(|line| *line == value).count() == 1),
        "unexpected credential failure properties: {:?}",
        String::from_utf8_lossy(&properties)
    );
    // systemd's credential setup failure is EXIT_CREDENTIALS (243), distinct
    // from executing the broker and finding a malformed key or certificate.
    // The missing absolute LoadCredential source is fatal without a fallback;
    // its error-level diagnostic survives the default journal log level.
    let (journal, stderr) = guest_diagnostic(
        sandbox,
        JOURNALCTL,
        &[
            "--unit",
            SERVICE,
            "--boot",
            "--no-pager",
            "--lines=20",
            "--output=cat",
            "--grep=Failed to set up credentials:",
        ],
    )
    .await?;
    ensure!(stderr.is_empty(), "credential journal diagnostic stderr");
    ensure!(
        std::str::from_utf8(&journal)?
            .contains("Failed to set up credentials: No such file or directory"),
        "missing credential failure cause: {:?}",
        String::from_utf8_lossy(&journal)
    );
    eprintln!(
        "native-vm: loaded broker unit failed with EXIT_CREDENTIALS and missing-file setup cause"
    );
    Ok(())
}

async fn guest_diagnostic(
    sandbox: &Sandbox,
    program: &str,
    args: &[&str],
) -> Result<(Vec<u8>, Vec<u8>)> {
    use microsandbox::ExecEvent as Event;
    let mut handle = timeout(
        Duration::from_secs(6),
        sandbox.exec_stream_with(program, |options| {
            options
                .args(args.iter().copied())
                .env("LC_ALL", "C")
                .timeout(Duration::from_secs(5))
        }),
    )
    .await
    .context("service diagnostic request deadline")??;
    let observed = timeout(Duration::from_secs(6), async {
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        let mut started = false;
        loop {
            match handle.recv().await {
                Some(Event::Started { .. }) if !started => started = true,
                Some(Event::Stdout(chunk)) if started => {
                    ensure!(
                        stdout.len() + stderr.len() + chunk.len() <= 4096,
                        "service diagnostic output limit"
                    );
                    stdout.extend_from_slice(&chunk);
                }
                Some(Event::Stderr(chunk)) if started => {
                    ensure!(
                        stdout.len() + stderr.len() + chunk.len() <= 4096,
                        "guest diagnostic output limit"
                    );
                    stderr.extend_from_slice(&chunk);
                }
                Some(Event::Exited { code: 0 }) if started => return Ok((stdout, stderr)),
                Some(Event::Exited { code }) if started => {
                    return Err(DiagnosticExitFailure {
                        code,
                        stdout,
                        stderr,
                    }
                    .into());
                }
                Some(Event::Failed(failure)) => {
                    return Err(DiagnosticSpawnFailure {
                        failure,
                        before_started: !started,
                    }
                    .into());
                }
                _ => anyhow::bail!("service diagnostic did not exit cleanly"),
            }
        }
    })
    .await;
    if !matches!(&observed, Ok(Ok(_))) {
        let cleanup = timeout(Duration::from_secs(6), handle.cancel()).await;
        eprintln!(
            "native-vm: failed diagnostic cancellation observation: {}",
            bounded_cause(&format!("{cleanup:?}"))
        );
    }
    observed.context("service diagnostic observation deadline")?
}

async fn dispatch_exec(
    dispatcher: &mut ControlDispatcher<MicrosandboxControl>,
    caller: &AuthenticatedCaller,
    launch: &LaunchRef,
    request: ExecRequest,
) -> Result<ExecStatus> {
    let outcome = dispatcher
        .dispatch(
            caller,
            ControlRequest::GuestExec {
                launch: launch.clone(),
                request,
            },
        )
        .await?;
    ensure!(
        outcome.custody_transaction.is_none(),
        "exec unexpectedly changed custody"
    );
    match outcome.response {
        ControlResponse::GuestExec(status) => Ok(status),
        _ => anyhow::bail!("unexpected dispatcher response"),
    }
}

async fn command(
    dispatcher: &mut ControlDispatcher<MicrosandboxControl>,
    caller: &AuthenticatedCaller,
    launch: &LaunchRef,
    program: &str,
    args: &[&str],
    timeout_ms: u32,
    cancel: bool,
) -> Result<(ExecState, Vec<u8>)> {
    let mut status = dispatch_exec(
        dispatcher,
        caller,
        launch,
        ExecRequest::Start {
            command: ExecCommand {
                program: program.into(),
                args: args.iter().map(|value| (*value).into()).collect(),
                cwd: Some("/".into()),
                env: BTreeMap::new(),
                stdin: false,
                tty: false,
                timeout_ms,
            },
        },
    )
    .await?;
    let id = status.id.clone();
    let deadline = Instant::now() + Duration::from_secs(12);
    let mut stdout = Vec::new();
    let mut started = false;
    let mut cancelled = false;
    loop {
        for event in &status.events {
            match event {
                ExecEvent::Started => started = true,
                ExecEvent::Stdout { bytes } => {
                    ensure!(stdout.len() + bytes.len() <= 8192, "fixture output limit");
                    stdout.extend_from_slice(bytes);
                }
                ExecEvent::Stderr { bytes } => ensure!(bytes.is_empty(), "unexpected guest stderr"),
                _ => {}
            }
        }
        if status.state.terminal() {
            ensure!(started, "terminal arrived without actual start");
            let state = status.state;
            dispatch_exec(dispatcher, caller, launch, ExecRequest::Release { id }).await?;
            return Ok((state, stdout));
        }
        ensure!(
            Instant::now() < deadline,
            "dispatcher execution observation deadline"
        );
        if cancel && started && !cancelled {
            status = dispatch_exec(
                dispatcher,
                caller,
                launch,
                ExecRequest::Cancel { id: id.clone() },
            )
            .await?;
            cancelled = true;
        } else {
            tokio::time::sleep(Duration::from_millis(20)).await;
            status = dispatch_exec(
                dispatcher,
                caller,
                launch,
                ExecRequest::Read {
                    id: id.clone(),
                    max_bytes: 8192,
                },
            )
            .await?;
        }
    }
}

async fn exercise(
    spec: &VmSpec,
    backend: Arc<dyn microsandbox::Backend>,
    current: &mut Option<OwnedVm>,
) -> Result<()> {
    let local = backend.as_local().context("fixture backend is not local")?;
    // The unoptimized test importer processes the full cold, high-entry-count
    // image. This allowance remains inside the unchanged whole-body deadline.
    let import_started = Instant::now();
    eprintln!("native fixture image import started; deadline_seconds=180");
    let imported = timeout(
        Duration::from_secs(180),
        Image::load_local(local, &spec.archive, vec![spec.image_reference.clone()]),
    )
    .await;
    let import_outcome = match &imported {
        Ok(Ok(_)) => "completed",
        Ok(Err(_)) => "failed",
        Err(_) => "deadline",
    };
    eprintln!(
        "native fixture image import {import_outcome}; elapsed_ms={}",
        import_started.elapsed().as_millis()
    );
    let images = imported.context("image import deadline")??;
    ensure!(
        images
            .iter()
            .any(|image| image.reference() == spec.image_reference),
        "exact local image tag missing"
    );
    bootstrap(spec)?;
    let mut native = MicrosandboxControl::new(backend.clone());
    *current = Some(create_vm(spec, false).await?);
    let bad = &current.as_ref().context("missing owned VM")?.sandbox;
    let old = native.retain(instance(), NAME, bad.clone()).await?;
    store_ready(bad).await?;
    missing_credentials(bad).await?;
    let old_route = native
        .capture_host_listener(
            &old,
            &spec.root.join("run/bad/management.sock"),
            Instant::now() + Duration::from_secs(5),
        )
        .await?;
    let bad_stream = native
        .connect_host_listener(&old, &old_route, Instant::now() + Duration::from_secs(5))
        .await?;
    let mut incarnation = [0; 32];
    getrandom::fill(&mut incarnation)
        .map_err(|_| anyhow::anyhow!("controller entropy unavailable"))?;
    let mut controller = SshController::new(OpaqueId::from_bytes(incarnation));
    let negative = BrokerLink::connect_verified(
        bad_stream,
        || old_route.check_current(&old),
        &mut controller,
    )
    .await;
    match negative {
        Ok((mut link, _)) => {
            link.close(&mut controller);
            anyhow::bail!("missing bootstrap unexpectedly accepted Hello");
        }
        Err(error) => ensure!(
            error == ControlError::BrokerUnavailable,
            "unexpected bootstrap failure category"
        ),
    }
    eprintln!("native-vm: missing bootstrap failed in booted guest; direct Hello refused");
    stop_current(current).await?;

    *current = Some(create_vm(spec, true).await?);
    let good = &current.as_ref().context("missing owned VM")?.sandbox;
    let selected = native.retain(instance(), NAME, good.clone()).await?;
    ensure!(
        selected != old,
        "same-name replacement reused native generation"
    );
    ensure!(
        matches!(
            native.verify_launch(&old).await,
            Err(ControlError::StaleLaunch)
        ),
        "old actual SDK launch was not fenced"
    );
    ensure!(
        matches!(
            native
                .connect_host_listener(&old, &old_route, Instant::now() + Duration::from_secs(5))
                .await,
            Err(ControlError::StaleLaunch)
        ),
        "old route connected after replacement"
    );
    store_ready(good).await?;
    service_state(good, b"active\n").await?;
    let route = native
        .capture_host_listener(
            &selected,
            &spec.root.join("run/good/management.sock"),
            Instant::now() + Duration::from_secs(5),
        )
        .await?;
    let stream = native
        .connect_host_listener(&selected, &route, Instant::now() + Duration::from_secs(5))
        .await?;
    let (mut link, pending) =
        BrokerLink::connect_verified(stream, || route.check_current(&selected), &mut controller)
            .await?;
    let probe = link.probe(&mut controller).await;
    link.close(&mut controller);
    probe?;
    ensure!(
        pending.is_empty(),
        "empty fixture unexpectedly had pending policy"
    );
    ensure!(
        good.status().await? == SandboxStatus::Running,
        "native ownership lost after Hello"
    );
    eprintln!(
        "native-vm: actual kernel-bound management Hello and Probe passed; no policy/upstream requests"
    );

    // Swap only this fixture-owned endpoint, retain/restore the original inode
    // for libkrun's own cleanup, and never recapture a token for the old handle.
    let original = route.path.with_file_name("original-management.sock");
    std::fs::rename(&route.path, &original)?;
    let replacement = std::os::unix::net::UnixListener::bind(&route.path);
    let refusal = match replacement {
        Ok(listener) => {
            let check = (|| -> Result<()> {
                listener.set_nonblocking(true)?;
                let mut connection = Box::pin(native.connect_host_listener(
                    &selected,
                    &route,
                    Instant::now() + Duration::from_secs(5),
                ));
                // Exact replacement must refuse in the first poll, before
                // native async proof or connect. No await while paths differ.
                let result = connection
                    .as_mut()
                    .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()));
                ensure!(
                    matches!(
                        result,
                        std::task::Poll::Ready(Err(ControlError::StaleLaunch))
                    ),
                    "replacement was not refused synchronously"
                );
                ensure!(
                    matches!(listener.accept(), Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
                    "replacement endpoint received a connection"
                );
                Ok(())
            })();
            drop(listener);
            let removed = std::fs::remove_file(&route.path);
            let restored = std::fs::rename(&original, &route.path);
            removed?;
            restored?;
            check
        }
        Err(error) => {
            std::fs::rename(&original, &route.path)?;
            return Err(error.into());
        }
    };
    refusal?;
    ensure!(
        route.check_current(&selected) == Err(ControlError::StaleLaunch),
        "observed loss was revived"
    );

    let credentials = CredentialsPlan {
        ssh: Vec::new(),
        signing: Vec::new(),
        strict: true,
        strict_origin: None,
    };
    controller.register_launch(selected.clone(), &credentials, None)?;
    controller.confirm_launch(&selected)?;
    let mut dispatcher = ControlDispatcher::with_custody(native, controller);
    let uid = std::fs::metadata(&spec.root)?.uid();
    let caller = AuthenticatedCaller::local_operator(uid);
    let (state, bytes) = command(
        &mut dispatcher,
        &caller,
        &selected,
        SYSTEMCTL,
        &["show", "--property=ActiveState", "--value", SERVICE],
        5000,
        false,
    )
    .await?;
    ensure!(
        matches!(state, ExecState::Exited { code: 0 }) && bytes == b"active\n",
        "real dispatcher stdout/exit mismatch"
    );
    for (milliseconds, cancel, reason) in [
        (250, false, ExecInterruptionReason::Timeout),
        (10000, true, ExecInterruptionReason::Cancelled),
    ] {
        let (state, bytes) = command(
            &mut dispatcher,
            &caller,
            &selected,
            "/run/current-system/sw/bin/sleep",
            &["30"],
            milliseconds,
            cancel,
        )
        .await?;
        ensure!(bytes.is_empty(), "sleep emitted unexpected bytes");
        ensure!(
            matches!(state, ExecState::Interrupted { reason: actual, termination: ExecTermination::Exited { .. } } if actual == reason),
            "interruption lacks correct reason or observed exit: {state:?}"
        );
    }
    eprintln!(
        "native-vm: actual dispatcher output, timeout and cancellation passed with independent exit evidence"
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires an explicit disposable broker VM spec, pinned test wrapper and owned KVM/resource supervisor"]
async fn native_broker_vm_management_and_exec() -> Result<()> {
    let spec = VmSpec::read()?;
    let backend: Arc<dyn microsandbox::Backend> = Arc::new(
        LocalBackend::builder()
            .home(spec.root.join("msb"))
            .try_build_lazy()?,
    );
    let mut current = None;
    let result = microsandbox::with_backend(
        backend.clone(),
        timeout(
            Duration::from_secs(300),
            exercise(&spec, backend, &mut current),
        ),
    )
    .await;
    let cleanup = stop_current(&mut current).await;
    if let Err(error) = &cleanup {
        eprintln!("native-vm: cleanup incomplete: {error:#}");
    }
    match result {
        Err(_) => anyhow::bail!("native VM fixture deadline; cleanup: {cleanup:?}"),
        Ok(Err(error)) => {
            anyhow::bail!("native VM fixture failed: {error:#}; cleanup: {cleanup:?}")
        }
        Ok(Ok(())) => cleanup?,
    }
    eprintln!(
        "native-vm: PASS; two sequential launches stopped and removed; no CLI/shared-owner claim"
    );
    Ok(())
}

#[test]
fn native_vm_spec_rejects_ambient_or_ambiguous_inputs() {
    let base = serde_json::json!({"version":1,"root":"/tmp/broker-vm.Abc123","archive":"/tmp/broker-vm.Abc123/image.tar","image_reference":"localhost:9/workestrate-broker:fixture","principal":"broker.workestrate.internal"});
    serde_json::from_value::<VmSpec>(base.clone())
        .unwrap()
        .validate_shape()
        .unwrap();
    for (field, value) in [
        ("version", serde_json::json!(2)),
        ("root", serde_json::json!("/tmp")),
        ("root", serde_json::json!("/tmp/broker-vm.a/../other")),
        ("archive", serde_json::json!("/etc/passwd")),
        (
            "image_reference",
            serde_json::json!("docker.io/broker:latest"),
        ),
        ("principal", serde_json::json!("*")),
    ] {
        let mut changed = base.clone();
        changed[field] = value;
        assert!(
            serde_json::from_value::<VmSpec>(changed)
                .unwrap()
                .validate_shape()
                .is_err()
        );
    }
    let mut changed = base;
    changed["credential"] = serde_json::json!("must not be accepted");
    assert!(serde_json::from_value::<VmSpec>(changed).is_err());
}

fn pending_account_failure() -> DiagnosticSpawnFailure {
    DiagnosticSpawnFailure {
        before_started: true,
        failure: microsandbox_protocol::exec::ExecFailed {
            kind: microsandbox_protocol::exec::ExecFailureKind::Other,
            errno: None,
            errno_name: None,
            stage: None,
            message: "exec session error: failed to resolve guest uid 0: No such file or directory (os error 2)".into(),
        },
    }
}

#[test]
fn native_activation_classifies_only_exact_prespawn_readiness() {
    use microsandbox_protocol::exec::ExecFailureKind;
    assert!(matches!(
        activation_observation(Err(pending_account_failure().into())).unwrap(),
        ActivationObservation::Pending(_)
    ));
    for change in 0..6 {
        let mut failure = pending_account_failure();
        match change {
            0 => failure.before_started = false,
            1 => failure.failure.message.push('!'),
            2 => failure.failure.errno = Some(libc::EACCES),
            3 => failure.failure.stage = Some("unknown".into()),
            4 => failure.failure.kind = ExecFailureKind::UserSetupFailed,
            _ => failure.failure.errno_name = Some("ENOENT".into()),
        }
        assert!(activation_observation(Err(failure.into())).is_err());
    }
    let mut failure = pending_account_failure();
    failure.failure.kind = ExecFailureKind::NotFound;
    failure.failure.errno = Some(libc::ENOENT);
    failure.failure.errno_name = Some("ENOENT".into());
    failure.failure.stage = Some("Command::spawn".into());
    failure.failure.message =
        format!("spawn {SYSTEMCTL:?}: No such file or directory (os error 2)");
    assert!(matches!(
        activation_observation(Err(failure.into())).unwrap(),
        ActivationObservation::Pending(_)
    ));
    let mut failure = pending_account_failure();
    failure.failure.message = "unrelated missing binary".into();
    assert!(activation_observation(Err(failure.into())).is_err());
}

#[test]
fn native_activation_requires_observed_target_not_transport_or_interruption() {
    assert_eq!(
        activation_observation(Ok((b"active\n".to_vec(), Vec::new()))).unwrap(),
        ActivationObservation::Ready
    );
    for state in [b"inactive\n".as_slice(), b"activating\n"] {
        assert!(matches!(
            activation_observation(Ok((state.to_vec(), Vec::new()))).unwrap(),
            ActivationObservation::Pending(_)
        ));
    }
    for state in [b"failed\n".as_slice(), b"active\nextra\n", b""] {
        assert!(activation_observation(Ok((state.to_vec(), Vec::new()))).is_err());
    }
    assert!(activation_observation(Ok((b"active\n".to_vec(), b"warning".to_vec()))).is_err());
    for code in [1, 255] {
        assert!(matches!(
            activation_observation(Err(DiagnosticExitFailure {
                code,
                stdout: Vec::new(),
                stderr: b"observed nonzero diagnostic".to_vec(),
            }
            .into()))
            .unwrap(),
            ActivationObservation::Pending(_)
        ));
    }
    for code in [-1, 0, 256] {
        assert!(
            activation_observation(Err(DiagnosticExitFailure {
                code,
                stdout: Vec::new(),
                stderr: Vec::new(),
            }
            .into()))
            .is_err()
        );
    }
    for error in [
        "transport lost",
        "post-Started interrupted",
        "unconfirmed termination",
    ] {
        assert!(activation_observation(Err(anyhow::anyhow!(error))).is_err());
    }
}

#[tokio::test(start_paused = true)]
async fn native_activation_retries_only_until_actual_ready() {
    let calls = std::cell::Cell::new(0);
    let started = Instant::now();
    wait_for_activation(started + Duration::from_secs(45), || {
        let call = calls.get();
        calls.set(call + 1);
        async move {
            Ok(if call < 2 {
                ActivationObservation::Pending(format!("early-{call}"))
            } else {
                ActivationObservation::Ready
            })
        }
    })
    .await
    .unwrap();
    assert_eq!(calls.get(), 3);
    assert_eq!(started.elapsed(), Duration::from_millis(400));
}

#[tokio::test(start_paused = true)]
async fn native_activation_unknown_failure_is_not_replayed() {
    let calls = std::cell::Cell::new(0);
    let error = wait_for_activation(Instant::now() + Duration::from_secs(45), || {
        calls.set(calls.get() + 1);
        async { anyhow::bail!("unknown spawn failure") }
    })
    .await
    .unwrap_err();
    assert_eq!(calls.get(), 1);
    assert!(format!("{error:#}").contains("unknown spawn failure"));
}

#[tokio::test(start_paused = true)]
async fn native_activation_deadline_retains_bounded_first_and_last_causes() {
    let calls = std::cell::Cell::new(0);
    let started = Instant::now();
    let error = wait_for_activation(started + Duration::from_secs(1), || {
        let call = calls.get();
        calls.set(call + 1);
        async move {
            let label = if call == 0 { "first" } else { "last" };
            Ok(ActivationObservation::Pending(format!(
                "{label}:{}",
                "é".repeat(1000)
            )))
        }
    })
    .await
    .unwrap_err();
    assert_eq!(started.elapsed(), Duration::from_secs(1));
    assert_eq!(calls.get(), 5);
    let text = error.to_string();
    assert!(text.contains("first=Some(\"first:"));
    assert!(text.contains("last=Some(\"last:"));
    assert!(text.len() < 1200);
    assert!(bounded_cause(&"é".repeat(1000)).len() <= 512);
}

#[tokio::test(start_paused = true)]
async fn native_activation_deadline_covers_an_inflight_probe() {
    let calls = std::cell::Cell::new(0);
    let started = Instant::now();
    let error = wait_for_activation(started + Duration::from_secs(1), || {
        calls.set(calls.get() + 1);
        std::future::pending::<Result<ActivationObservation>>()
    })
    .await
    .unwrap_err();
    assert_eq!(started.elapsed(), Duration::from_secs(1));
    assert_eq!(calls.get(), 1);
    assert!(error.to_string().contains("deadline during probe"));
}

#[tokio::test(start_paused = true)]
async fn native_activation_cancellation_has_no_detached_retry() {
    use std::future::Future;
    let calls = std::cell::Cell::new(0);
    let mut wait = Box::pin(wait_for_activation(
        Instant::now() + Duration::from_secs(45),
        || {
            calls.set(calls.get() + 1);
            std::future::pending::<Result<ActivationObservation>>()
        },
    ));
    assert!(
        wait.as_mut()
            .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
            .is_pending()
    );
    assert_eq!(calls.get(), 1);
    drop(wait);
    tokio::time::advance(Duration::from_secs(60)).await;
    assert_eq!(calls.get(), 1);
}

#[test]
fn native_owner_exit_marker_requires_exact_successful_retained_status() {
    use std::os::unix::process::ExitStatusExt;
    let marker = owner_exit_marker("bad", &[1; 32], std::process::ExitStatus::from_raw(0)).unwrap();
    assert_eq!(
        marker,
        format!(
            "native-vm: owner-exit phase=bad launch={} code=0 signal=none",
            "01".repeat(32)
        )
    );
    assert!(owner_exit_marker("good", &[2; 32], std::process::ExitStatus::from_raw(0)).is_ok());
    for raw in [1 << 8, libc::SIGKILL, (libc::SIGSTOP << 8) | 0x7f] {
        assert!(
            owner_exit_marker("bad", &[1; 32], std::process::ExitStatus::from_raw(raw)).is_err()
        );
    }
    assert!(owner_exit_marker("other", &[1; 32], std::process::ExitStatus::from_raw(0)).is_err());
    assert!(owner_exit_marker("bad", &[0; 32], std::process::ExitStatus::from_raw(0)).is_err());
}
