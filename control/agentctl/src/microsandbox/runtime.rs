use super::env::resolve_templated_value;
use super::mounts::{apply_plan_mounts, ensure_mount_sources};
use super::plan::{EgressTarget, NetworkPlan, PortMapping, Protocol, SandboxPlan, Scope};
use super::slots;
use super::workload::{EntrypointSpec, SandboxCommand, Workload};
use anyhow::Result;
use microsandbox::sandbox::{exec::ExecEvent, SandboxBuilder, SandboxHandle, SandboxStatus};
use microsandbox::{MicrosandboxError, NetworkPolicy, Sandbox};
use std::path::{Path, PathBuf};

fn reject_if_placeholder(value: &str, placeholder: &Option<String>, label: &str) -> Result<()> {
    if let Some(ref ph) = placeholder {
        if value.trim() == ph.trim() {
            anyhow::bail!(
                "{} is set to the placeholder value '{}'. \
                 Replace it with a real secret before running this command.",
                label,
                ph
            );
        }
    }
    Ok(())
}

/// Convert a declarative `NetworkPlan` into a Microsandbox SDK `NetworkPolicy`.
pub(crate) fn network_plan_to_policy(plan: &NetworkPlan) -> Result<NetworkPolicy> {
    let mut builder = NetworkPolicy::builder();

    if plan.default_deny {
        builder = builder.default_deny();
    }

    for rule in &plan.ingress_rules {
        let port = rule.port;
        match (rule.protocol, rule.scope) {
            (Protocol::Tcp, Scope::Local) => {
                builder = builder.ingress(|i| i.tcp().port(port).allow_local());
            }
            (Protocol::Tcp, Scope::Public) => {
                builder = builder.ingress(|i| i.tcp().port(port).allow_public());
            }
            (proto, scope) => {
                anyhow::bail!("unsupported ingress rule: {}/{}", proto, scope);
            }
        }
    }

    for rule in &plan.egress_rules {
        let port = rule.port;
        match (&rule.protocol, &rule.target) {
            (Protocol::Tcp, EgressTarget::Host) => {
                builder = builder.egress(|e| e.tcp().port(port).allow_host());
            }
            (Protocol::Udp, EgressTarget::Host) => {
                builder = builder.egress(|e| e.udp().port(port).allow_host());
            }
            (Protocol::Tcp, EgressTarget::Domains(hosts)) => {
                let hosts = hosts.clone();
                builder = builder.egress(move |e| {
                    e.tcp()
                        .port(port)
                        .allow_domains(hosts.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                });
            }
            (Protocol::Udp, EgressTarget::Domains(hosts)) => {
                let hosts = hosts.clone();
                builder = builder.egress(move |e| {
                    e.udp()
                        .port(port)
                        .allow_domains(hosts.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                });
            }
        }
    }

    for rule in &plan.deny_rules {
        let suffix = rule.domain_suffix.clone();
        builder = builder.egress(move |e| e.deny_domain_suffixes([&suffix]));
    }

    builder.build().map_err(Into::into)
}

pub(crate) fn spawn_detached_service(name: &str, args: &[String]) -> Result<std::process::Child> {
    let exe = std::env::current_exe()?;
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| anyhow::anyhow!("HOME not set"))?;
    let log_dir = home.join(".microsandbox/sandboxes").join(name);
    std::fs::create_dir_all(&log_dir)?;
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_dir.join("workestrate.log"))?;
    let mut cmd = std::process::Command::new(&exe);
    cmd.args(args)
        .stdin(std::process::Stdio::null())
        .stdout(log_file.try_clone()?)
        .stderr(log_file);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    Ok(cmd.spawn()?)
}

pub(crate) fn apply_plan_secrets(
    builder: SandboxBuilder,
    plan: &SandboxPlan,
) -> Result<SandboxBuilder> {
    let mut b = builder;
    for s in &plan.secret_env {
        match resolve_templated_value(&s.value) {
            Ok(value) => {
                reject_if_placeholder(&value, &s.reject_placeholder, &s.name)?;
                if s.required && value.trim().is_empty() {
                    anyhow::bail!(
                        "required secret '{}' is set but empty.\n\
                         Set a real value via setup-secrets update.",
                        s.name
                    );
                }
                for host in &s.allowed_hosts {
                    b = b.secret_env(&s.name, value.clone(), host);
                }
            }
            Err(_) if !s.required => {
                eprintln!(
                    "warning: optional secret '{}' is not set; skipping (hosts: {})",
                    s.name,
                    s.allowed_hosts.join(", ")
                );
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "required secret '{}' is not set: {}\n\
                     Set it in the environment or run setup-secrets update",
                    s.name,
                    e
                ));
            }
        }
    }
    Ok(b)
}

pub(crate) fn apply_plan_envs(
    builder: SandboxBuilder,
    plan: &SandboxPlan,
) -> Result<SandboxBuilder> {
    let mut b = builder;
    for e in &plan.env {
        let value = resolve_templated_value(&e.value)?;
        reject_if_placeholder(&value, &e.reject_placeholder, &e.name)?;
        b = b.env(&e.name, value);
    }
    Ok(b)
}

pub(crate) async fn stop_and_remove(handle: SandboxHandle) -> Result<()> {
    match handle.status() {
        SandboxStatus::Running | SandboxStatus::Draining | SandboxStatus::Paused => {
            if let Err(e) = handle.stop().await {
                eprintln!("stop failed ({}), attempting kill", e);
                handle.kill().await?;
            }
        }
        _ => {}
    }
    handle.remove().await?;
    Ok(())
}

/// Start `exec_program` with `exec_args` inside `sandbox`, stream its logs to
/// stderr, and block until Ctrl-C — then stop the sandbox.
///
/// This is the shared foreground path used by `up_litellm`, `up_pi`, and
/// `up_odysseus`. The service label is used in user-facing messages
/// (e.g. "litellm", "pi", "odysseus").
pub(crate) struct ForegroundConfig {
    pub sandbox_name: String,
    pub service_label: String,
    pub command: SandboxCommand,
    pub log_stop_errors: bool,
}

/// Resolved identity + flags for a single `up`/`exec` invocation (ADR 0021).
///
/// Built by the CLI layer from `--replace` / `--instance <id>` / `--new` /
/// `--port-offset N` plus the active context. Consumed by [`build_sandbox`].
pub(crate) struct InstanceSpec {
    /// Singleton slot: `<workload>` or `<context>-<workload>`.
    //
    // Carried for diagnostics/state-file parity; the runtime keys off
    // `instance` (which is `slot` or `slot@<id>`), so `slot` itself is not
    // read on the current hot path.
    #[allow(dead_code)]
    pub slot: String,
    /// The sandbox name to create: `slot` (singleton) or `slot@<id>` (parallel).
    pub instance: String,
    /// Bare workload name (e.g. `litellm`). Used in user-facing messages.
    pub workload: String,
    /// Active context name, if any.
    pub context: Option<String>,
    /// `--port-offset N`. Added to every HOST port. 0 = no shift.
    pub port_offset: u16,
    /// `--replace`. If true, occupancy is torn down before create; otherwise
    /// an occupied slot REFUSES (fail-closed default).
    pub replace: bool,
}

/// Result of an occupancy probe against the state registry (no msb call).
#[derive(Debug, Clone)]
pub enum Occupancy {
    /// No state record for the instance.
    Free,
    /// A state record exists; the named instance may still be running.
    //
    // Fields carry the occupying identity for diagnostics/refuse messages;
    // the live refuse path currently formats from separately-resolved args
    // (see [`format_refuse_message`]), so these fields are read mainly by
    // tests until the formatter is migrated to consume the enum directly.
    #[allow(dead_code)]
    Occupied {
        occupying_instance: String,
        workload: String,
        context: Option<String>,
    },
}

/// Outcome of stopping one instance. Used by `down --instance`, `down
/// --all-instances`, and `down --all`.
#[derive(Debug, Clone)]
pub struct DownResult {
    pub instance: String,
    pub status: DownStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownStatus {
    /// Sandbox was running and was stopped+removed cleanly.
    Stopped,
    /// No sandbox found (state record cleared if present).
    NotFound,
    /// msb returned a hard error during stop/remove.
    Error,
}

/// One row of `workestrate ps` output. Pure: no msb calls.
#[derive(Debug, Clone)]
pub struct PsEntry {
    pub instance: String,
    pub workload: String,
    pub context: Option<String>,
    /// Effective host:guest port pairs (post-offset host). Legacy records
    /// synthesize `{host: p, guest: p}` from the bare host-port list.
    pub ports: Vec<PortMapping>,
    /// RFC3339 timestamp the instance was registered, or empty for legacy.
    pub created: String,
    /// Best-effort staleness flag; populated by callers that can reach msb.
    /// The pure [`ps`] function always sets this to `false`.
    //
    // Surfaced to future `ps --json` / dashboard consumers; the current text
    // renderer doesn't print it, so it's exercised by tests until the JSON
    // view lands.
    #[allow(dead_code)]
    pub stale: bool,
}

/// The canonical refuse message (text mode). Pinned by ADR 0021 §2.
///
/// Note: the message references the occupying *instance name* (which is the
/// slot for the singleton case, or `slot@id` for a parallel instance). The
/// caller passes the workload's bare name so the suggested `down` command
/// reads naturally (`workestrate litellm down`).
pub fn format_refuse_message(workload: &str, occupying_instance: &str) -> String {
    format!(
        "instance '{instance}' is already running. \
         Use --replace to replace it, --instance <id> for a parallel instance, \
         or '{wl} down' to stop it first.",
        instance = occupying_instance,
        wl = workload,
    )
}

/// Pure state-record occupancy probe. Does NOT call msb.
///
/// Returns `Occupied` when a state file exists for `instance` (regardless of
/// whether the backing sandbox is still running). The async caller MUST
/// further verify via `Sandbox::get` to distinguish truly-running from stale.
pub fn occupancy_from_state(state_dir: &Path, instance: &str) -> Result<Occupancy> {
    Ok(
        match super::port_registry::find_record(state_dir, instance)? {
            Some(r) => Occupancy::Occupied {
                occupying_instance: r.instance,
                workload: r.workload,
                context: r.context,
            },
            None => Occupancy::Free,
        },
    )
}

/// Pure listing for `workestrate ps`. Reads the registry state files; does
/// NOT call msb (callers may post-process to populate `stale`).
pub fn ps(state_dir: &Path) -> Result<Vec<PsEntry>> {
    let records = super::port_registry::list_records(state_dir)?;
    Ok(records
        .into_iter()
        .map(|r| {
            let ports = if r.port_pairs.is_empty() {
                r.ports
                    .iter()
                    .map(|&h| PortMapping { host: h, guest: h })
                    .collect()
            } else {
                r.port_pairs
            };
            PsEntry {
                instance: r.instance,
                workload: r.workload,
                context: r.context,
                ports,
                created: r.created_at,
                stale: false,
            }
        })
        .collect())
}

/// Compute the offset-shifted host port, failing on overflow past u16.
fn offset_port(host: u16, offset: u16) -> Result<u16> {
    host.checked_add(offset).ok_or_else(|| {
        anyhow::anyhow!(
            "port offset {} applied to host port {} overflows u16; \
             reduce --port-offset",
            offset,
            host
        )
    })
}

/// Current time as an RFC3339 UTC string. Best-effort: falls back to the
/// Unix timestamp when SystemTime fails (should not happen in practice).
fn current_rfc3339_utc() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Format as a stable RFC3339-ish UTC timestamp using a tiny ad-hoc
    // formatter (no chrono dep). The format is `YYYY-MM-DDTHH:MM:SSZ`. This
    // is good enough for `ps` display; callers needing finer precision can
    // post-process. The date math is the standard days-from-civil algorithm
    // (Howard Hinnant, http://howardhinnant.github.io/date_algorithms.html).
    let days = (now / 86_400) as i64;
    let secs = (now % 86_400) as u32;
    let (y, m, d) = days_to_ymd(days);
    let hh = secs / 3600;
    let mm = (secs % 3600) / 60;
    let ss = secs % 60;
    format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z", y, m, d, hh, mm, ss)
}

/// Convert days-since-1970-01-01 to (year, month, day). Pure.
fn days_to_ymd(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 {
        z / 146_097
    } else {
        (z - 146_096) / 146_097
    };
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

/// Occupancy gate for `up`/`exec`. When `spec.replace` is true, the existing
/// sandbox at `spec.instance` is torn down (best-effort) and stale state is
/// cleared. When false, the function REFUSES (returns Err) if any of:
///   - msb reports the sandbox running, OR
///   - a state record exists AND msb is unavailable (fail-closed).
///
/// Returns Ok(()) when the slot is free (or has been cleared by --replace).
pub(crate) async fn check_occupied_or_replace(spec: &InstanceSpec, state_dir: &Path) -> Result<()> {
    if spec.replace {
        match Sandbox::get(&spec.instance).await {
            Ok(handle) => {
                stop_and_remove(handle).await?;
            }
            Err(MicrosandboxError::SandboxNotFound(_)) => {
                let _ = super::port_registry::unregister_sandbox(state_dir, &spec.instance);
            }
            Err(e) => {
                eprintln!(
                    "warning: could not verify sandbox '{}' via msb ({}); \
                     proceeding with --replace and clearing any stale state",
                    spec.instance, e
                );
                let _ = super::port_registry::unregister_sandbox(state_dir, &spec.instance);
            }
        }
        return Ok(());
    }

    match Sandbox::get(&spec.instance).await {
        Ok(_handle) => {
            // Truly running — refuse. The handle drops without stopping;
            // the existing sandbox keeps running (this is the desired
            // fail-closed behavior).
            anyhow::bail!("{}", format_refuse_message(&spec.workload, &spec.instance));
        }
        Err(MicrosandboxError::SandboxNotFound(_)) => {
            // Not running. A state record, if any, is stale.
            if matches!(
                occupancy_from_state(state_dir, &spec.instance)?,
                Occupancy::Occupied { .. }
            ) {
                eprintln!(
                    "warning: state record for '{}' exists but msb reports the sandbox \
                     is not running (stale record); refusing by default. \
                     Use --replace to clear.",
                    spec.instance
                );
                anyhow::bail!("{}", format_refuse_message(&spec.workload, &spec.instance));
            }
            Ok(())
        }
        Err(e) => {
            // msb unavailable. Fall back to state-record verdict (fail-closed).
            if matches!(
                occupancy_from_state(state_dir, &spec.instance)?,
                Occupancy::Occupied { .. }
            ) {
                eprintln!(
                    "warning: could not verify sandbox '{}' via msb ({}); \
                     treating state record as authoritative and refusing.",
                    spec.instance, e
                );
                anyhow::bail!("{}", format_refuse_message(&spec.workload, &spec.instance));
            }
            Ok(())
        }
    }
}

pub(crate) async fn run_service_foreground(
    sandbox: &Sandbox,
    config: ForegroundConfig,
) -> Result<()> {
    let mut exec_handle = sandbox
        .exec_stream(&config.command.binary, config.command.arguments)
        .await
        .map_err(|e| anyhow::anyhow!("failed to start {} process: {}", config.service_label, e))?;

    match exec_handle.recv().await {
        Some(ExecEvent::Started { pid }) => {
            eprintln!(
                "{} process started (guest PID {})",
                config.service_label, pid
            );
        }
        Some(ExecEvent::Failed(err)) => {
            if let Err(stop_err) = sandbox.stop().await {
                eprintln!("failed to stop sandbox after service failure: {}", stop_err);
            }
            return Err(anyhow::anyhow!(
                "{} process failed to start: {:?}",
                config.service_label,
                err
            ));
        }
        other => {
            if let Err(stop_err) = sandbox.stop().await {
                eprintln!(
                    "failed to stop sandbox after unexpected event: {}",
                    stop_err
                );
            }
            return Err(anyhow::anyhow!(
                "unexpected exec event waiting for {} start: {:?}",
                config.service_label,
                other
            ));
        }
    }

    // The ExecHandle must outlive `sandbox` for the streaming session to stay
    // open, so we move it into a background task. We intentionally do NOT use
    // `Sandbox::detach()` here: the foreground path wants a clean stop on
    // Ctrl-C, not a fire-and-forget background sandbox whose process group
    // outlives this CLI invocation.
    let drain_service = config.service_label.clone();
    let drain_handle = tokio::spawn(async move {
        while let Some(event) = exec_handle.recv().await {
            match event {
                ExecEvent::Stdout(data) => {
                    eprint!("{}", String::from_utf8_lossy(&data));
                }
                ExecEvent::Stderr(data) => {
                    eprint!("{}", String::from_utf8_lossy(&data));
                }
                ExecEvent::Exited { code } => {
                    eprintln!("{} exited with code {}", drain_service, code);
                }
                ExecEvent::Failed(err) => {
                    eprintln!("{} failed: {:?}", drain_service, err);
                }
                _ => {}
            }
        }
    });

    println!("Sandbox '{}' started (Ctrl-C to stop)", config.sandbox_name);
    if let Err(e) = tokio::signal::ctrl_c().await {
        // Even if the signal handler fails, attempt to stop the sandbox
        // before propagating the error.
        let _ = sandbox.stop().await;
        let _ = drain_handle.await;
        return Err(anyhow::anyhow!("signal handler error: {}", e));
    }
    if config.log_stop_errors {
        if let Err(e) = sandbox.stop().await {
            eprintln!("failed to stop sandbox '{}': {}", config.sandbox_name, e);
        }
    } else {
        let _ = sandbox.stop().await;
    }

    // Drain any remaining buffered log events before returning.
    // Use a timeout so a hung exec channel doesn't block forever.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), drain_handle).await;

    println!("Sandbox '{}' stopped", config.sandbox_name);
    Ok(())
}

/// Start `exec_program` with `exec_args` inside `sandbox` via an interactive
/// TTY attach, blocking until the guest process exits OR the user detaches
/// (ctrl-]). Then stop the sandbox.
///
/// This is the interactive counterpart to `run_service_foreground`, used by
/// TUI workloads (Pi, OpenCode). The SDK's `attach_with` handles raw mode,
/// stdin, resize, and TUI output coalescing internally.
pub(crate) async fn run_service_interactive(
    sandbox: &Sandbox,
    config: ForegroundConfig,
) -> Result<()> {
    let ForegroundConfig {
        sandbox_name,
        service_label,
        command,
        log_stop_errors,
    } = config;
    let SandboxCommand { binary, arguments } = command;

    println!(
        "Sandbox '{}' started (interactive TUI; ctrl-] to detach)",
        sandbox_name
    );

    let attach_result = sandbox
        .attach_with(binary.as_str(), |a| a.args(arguments))
        .await;

    let outcome = match attach_result {
        Ok(code) => {
            eprintln!("{} exited with code {}", service_label, code);
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!(
            "failed to attach to {} process: {}",
            service_label,
            e
        )),
    };

    if log_stop_errors {
        if let Err(e) = sandbox.stop().await {
            eprintln!("failed to stop sandbox '{}': {}", sandbox_name, e);
        }
    } else {
        let _ = sandbox.stop().await;
    }

    println!("Sandbox '{}' stopped", sandbox_name);
    outcome
}

/// Prepare, resolve, and create the sandbox plus the foreground config used
/// to run the workload's real command.
pub(crate) async fn build_sandbox<W: Workload>(
    workload: &W,
    spec: &InstanceSpec,
) -> Result<(Sandbox, ForegroundConfig)> {
    workload.prepare()?;

    // Load secrets from .env.enc if not already in env.
    // Only called for exec/up paths — plan/check never reach here.
    crate::microsandbox::secrets_loader::load_secrets()?;

    let root = crate::config::project_root()?;
    let mut plan = workload.plan();

    // Override the plan name with the spec instance name so display matches
    // the actual sandbox identity (slot for singleton, slot@id for parallel).
    plan.name = spec.instance.clone();

    // Compute offset-adjusted host ports (post-offset). `host_ports` is used
    // both for collision checking and for the legacy `ports` field in the
    // lifecycle state record.
    let host_ports: Vec<u16> = plan
        .ports
        .iter()
        .map(|p| offset_port(p.host, spec.port_offset))
        .collect::<Result<Vec<u16>>>()?;

    // Hoist state_dir before the occupancy check so it can be reused for
    // collision detection and lifecycle registration below.
    let state_dir = crate::config::resolve_state_dir();
    check_occupied_or_replace(spec, &state_dir).await?;

    // Port collision detection: check against already-running workestrate sandboxes.
    super::port_registry::check_port_collisions(&state_dir, &spec.instance, &host_ports)?;

    ensure_mount_sources(&root, &plan)?;

    let policy = network_plan_to_policy(&plan.network)?;

    let mut builder = Sandbox::builder(&spec.instance)
        .image(plan.image.as_deref().unwrap_or("alpine:latest"))
        .cpus(plan.cpus.unwrap_or(2))
        .memory(plan.memory_mib.unwrap_or(2048))
        .workdir(plan.workdir.as_deref().unwrap_or("/app"))
        .network(|n| n.policy(policy))
        .detached(true);

    let EntrypointSpec::Shell = workload.entrypoint();
    // Bare `/bin/sh` does not reliably block: the image's inherited CMD is
    // appended (Docker ENTRYPOINT+CMD semantics), so e.g. `python:3.12-slim`
    // (CMD `python3`) runs `/bin/sh python3` and exits before `exec_stream`
    // can start the service. `tail -f /dev/null` blocks forever regardless of
    // any appended CMD, keeping the sandbox alive for the relay's exec_stream.
    builder = builder.entrypoint(["/bin/sh", "-c", "tail -f /dev/null"]);

    for port in &plan.ports {
        let host = offset_port(port.host, spec.port_offset)?;
        builder = builder.port(host, port.guest);
    }

    builder = apply_plan_envs(builder, &plan)?;
    builder = apply_plan_mounts(builder, &root, &plan)?;
    builder = apply_plan_secrets(builder, &plan)?;

    let builder = if spec.replace {
        builder.replace()
    } else {
        builder
    };
    let sandbox = builder.create().await?;

    // Register with full lifecycle metadata so `ps` and `down --all` work.
    let port_pairs: Vec<PortMapping> = plan
        .ports
        .iter()
        .map(|p| {
            Ok(PortMapping {
                host: offset_port(p.host, spec.port_offset)?,
                guest: p.guest,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let created_at = current_rfc3339_utc();
    super::port_registry::register_sandbox_lifecycle(
        &state_dir,
        &spec.instance,
        spec.context.as_deref(),
        workload.name(),
        &host_ports,
        &port_pairs,
        spec.port_offset,
        &created_at,
    )?;
    let config = ForegroundConfig {
        sandbox_name: sandbox.name().to_string(),
        service_label: workload.name().to_string(),
        command: workload.exec(),
        log_stop_errors: workload.log_stop_errors(),
    };
    Ok((sandbox, config))
}

/// Build the default singleton InstanceSpec for `workload` in the active
/// context. Used by the legacy no-flag entry points and as the base for the
/// CLI flag-aware variants.
///
/// `replace: false` is the ADR 0021 fail-closed default — `up`/`exec` on an
/// occupied slot REFUSES unless `--replace` is passed. The legacy
/// `replace: true` behavior (silent replace) is opt-in via `--replace`.
//
// Retained for the legacy no-flag entry points ([`up_service`] / [`exec_agent`])
// which the CLI no longer dispatches through directly (it resolves flags via
// `resolve_instance_spec` and calls the `_with_spec` variants). Kept on the
// migration branch so external/ scripted callers can still reach the simple
// default-spec path; remove once the migration fully retires the legacy API.
#[allow(dead_code)]
fn default_spec<W: Workload>(workload: &W) -> InstanceSpec {
    let context = crate::config::active_context_name();
    let slot = slots::slot_for(workload.name(), context.as_deref());
    let instance = slot.clone();
    InstanceSpec {
        slot,
        instance,
        workload: workload.name().to_string(),
        context,
        port_offset: 0,
        replace: false,
    }
}

/// Start a service workload. Detached by default; pass `foreground = true` to
/// block until Ctrl-C.
//
// Legacy no-flag entry point; the CLI resolves flags and calls
// [`up_service_with_spec`]. Retained on the migration branch for scripted /
// external callers of the simple default-spec path.
#[allow(dead_code)]
pub async fn up_service<W: Workload>(workload: &W, foreground: bool) -> Result<()> {
    let spec = default_spec(workload);
    up_service_with_spec(workload, &spec, foreground).await
}

pub async fn up_service_with_spec<W: Workload>(
    workload: &W,
    spec: &InstanceSpec,
    foreground: bool,
) -> Result<()> {
    if !foreground {
        let instance = spec.instance.clone();
        let child = spawn_detached_service(&instance, &workload.detach_args(spec))?;
        println!(
            "Sandbox '{}' started in background (PID {}). Logs: ~/.microsandbox/sandboxes/{}/workestrate.log",
            instance, child.id(), instance
        );
        return Ok(());
    }
    let (sandbox, config) = build_sandbox(workload, spec).await?;
    run_service_foreground(&sandbox, config).await
}

/// Attach to an agent workload interactively (TUI).
//
// Legacy no-flag entry point; the CLI resolves flags and calls
// [`exec_agent_with_spec`]. Retained on the migration branch for scripted /
// external callers of the simple default-spec path.
#[allow(dead_code)]
pub async fn exec_agent<W: Workload>(workload: &W) -> Result<()> {
    let spec = default_spec(workload);
    exec_agent_with_spec(workload, &spec).await
}

pub async fn exec_agent_with_spec<W: Workload>(workload: &W, spec: &InstanceSpec) -> Result<()> {
    let (sandbox, config) = build_sandbox(workload, spec).await?;
    run_service_interactive(&sandbox, config).await
}

/// Tail the detached service's log file.
pub async fn logs(name: &str) -> Result<()> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| anyhow::anyhow!("HOME not set"))?;
    let path = home
        .join(".microsandbox/sandboxes")
        .join(name)
        .join("workestrate.log");

    if !path.exists() {
        anyhow::bail!(
            "no logs found for '{}'; not started? run: workestrate {} up",
            name,
            name
        );
    }

    let status = std::process::Command::new("tail")
        .args(["-n", "200", "-f", &path.to_string_lossy()])
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("tail exited with status: {}", status))
    }
}

/// Generic lifecycle: stop and remove any sandbox by name.
//
// Legacy single-name teardown; the CLI's `down` subcommand routes through the
// state-dir-explicit [`down_instance`] / [`down_all`] / [`down_all_instances`]
// variants so it can clean parallel-instance state. Retained on the migration
// branch as the resolved-state-dir convenience wrapper documented by
// [`down_instance`].
#[allow(dead_code)]
pub async fn down(name: &str) -> Result<()> {
    match Sandbox::get(name).await {
        Ok(handle) => {
            stop_and_remove(handle).await?;
            let state_dir = crate::config::resolve_state_dir();
            super::port_registry::unregister_sandbox(&state_dir, name)?;
            println!("Sandbox '{}' stopped and removed", name);
            Ok(())
        }
        Err(MicrosandboxError::SandboxNotFound(_)) => {
            // Sandbox not running, but there may be a stale state file.
            let state_dir = crate::config::resolve_state_dir();
            super::port_registry::unregister_sandbox(&state_dir, name)?;
            println!("Sandbox '{}' not found", name);
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

/// Stop a single instance by name (state-dir-explicit). The legacy
/// [`down`] wraps this with the resolved state dir for back-compat.
pub async fn down_instance(state_dir: &Path, instance: &str) -> DownResult {
    match down_one(state_dir, instance).await {
        Ok(DownOutcome::Stopped) => DownResult {
            instance: instance.to_string(),
            status: DownStatus::Stopped,
            message: None,
        },
        Ok(DownOutcome::NotFound) => DownResult {
            instance: instance.to_string(),
            status: DownStatus::NotFound,
            message: None,
        },
        Err(e) => DownResult {
            instance: instance.to_string(),
            status: DownStatus::Error,
            message: Some(e.to_string()),
        },
    }
}

#[derive(Debug, Clone, Copy)]
enum DownOutcome {
    Stopped,
    NotFound,
}

async fn down_one(state_dir: &Path, instance: &str) -> Result<DownOutcome> {
    match Sandbox::get(instance).await {
        Ok(handle) => {
            stop_and_remove(handle).await?;
            let _ = super::port_registry::unregister_sandbox(state_dir, instance);
            Ok(DownOutcome::Stopped)
        }
        Err(MicrosandboxError::SandboxNotFound(_)) => {
            let _ = super::port_registry::unregister_sandbox(state_dir, instance);
            Ok(DownOutcome::NotFound)
        }
        Err(e) => Err(e.into()),
    }
}

/// Stop every instance whose workload matches `workload`. Used by
/// `workestrate <wl> down --all-instances`.
pub async fn down_all_instances(state_dir: &Path, workload: &str) -> Result<Vec<DownResult>> {
    let records = super::port_registry::list_records_for_workload(state_dir, workload)?;
    let mut results = Vec::with_capacity(records.len());
    for r in records {
        results.push(down_instance(state_dir, &r.instance).await);
    }
    Ok(results)
}

/// Stop every workestrate-tracked instance. Used by `workestrate down --all`.
pub async fn down_all(state_dir: &Path) -> Result<Vec<DownResult>> {
    let records = super::port_registry::list_records(state_dir)?;
    let mut results = Vec::with_capacity(records.len());
    for r in records {
        results.push(down_instance(state_dir, &r.instance).await);
    }
    Ok(results)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::super::plan::{EgressTarget, Protocol};
    use super::network_plan_to_policy;
    use crate::microsandbox::workload::{ConfigWorkload, Workload};
    use std::path::PathBuf;

    /// RAII guard that points `WORKESTRATE_CONFIG_DIR` at the committed test
    /// fixture (a copy of the pre-strip-down 5-workload config) and restores the
    /// previous state on drop. Holds a global lock so env-var tests do not race
    /// when Cargo runs them in parallel.
    struct TestConfigGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl TestConfigGuard {
        fn new() -> Self {
            let lock = crate::config::tests::ENV_TEST_LOCK.lock().unwrap();
            let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join("config");
            std::env::set_var("WORKESTRATE_CONFIG_DIR", fixture);
            Self { _lock: lock }
        }
    }

    impl Drop for TestConfigGuard {
        fn drop(&mut self) {
            std::env::remove_var("WORKESTRATE_CONFIG_DIR");
        }
    }

    #[test]
    fn litellm_network_plan_converts_without_error() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("litellm")?.plan();
        let result = network_plan_to_policy(&plan.network);
        assert!(
            result.is_ok(),
            "litellm conversion failed: {:?}",
            result.err()
        );
        Ok(())
    }

    #[test]
    fn pi_network_plan_converts_without_error() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("pi")?.plan();
        let result = network_plan_to_policy(&plan.network);
        assert!(result.is_ok(), "pi conversion failed: {:?}", result.err());
        Ok(())
    }

    #[test]
    fn pi_plan_uses_nix_built_image() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        // The pi-bun binary's PT_INTERP points at nix glibc 2.42; the sandbox
        // image must be the nix-built `workestrator-pi:latest` (loaded via
        // `just load-pi-image`), NOT node:24-bookworm-slim (glibc 2.36 → crash).
        let plan = ConfigWorkload::new("pi")?.plan();
        assert_eq!(plan.image.as_deref(), Some("workestrator-pi:latest"));
        Ok(())
    }

    #[test]
    fn odysseus_network_plan_converts_without_error() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("odysseus")?.plan();
        let result = network_plan_to_policy(&plan.network);
        assert!(
            result.is_ok(),
            "odysseus conversion failed: {:?}",
            result.err()
        );
        Ok(())
    }

    #[test]
    fn odysseus_plan_includes_admin_password_secret() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("odysseus")?.plan();
        let has_admin_pw = plan
            .env
            .iter()
            .any(|e| e.name == "ODYSSEUS_ADMIN_PASSWORD" && e.is_secret);
        assert!(
            has_admin_pw,
            "odysseus plan must include ODYSSEUS_ADMIN_PASSWORD as a secret env var"
        );
        // The admin password is an internal credential, not egress-bound:
        // it must NOT appear in secret_env (which is host-bound).
        let in_secret_env = plan
            .secret_env
            .iter()
            .any(|s| s.name == "ODYSSEUS_ADMIN_PASSWORD");
        assert!(
            !in_secret_env,
            "ODYSSEUS_ADMIN_PASSWORD must not be host-bound (it is an internal admin credential)"
        );
        Ok(())
    }

    #[test]
    fn odysseus_plan_has_expected_data_mount() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("odysseus")?.plan();
        assert_eq!(plan.mounts.len(), 2, "odysseus should have 2 mounts");
        let data_mount = plan.mounts.iter().find(|m| m.guest == "/data");
        assert!(data_mount.is_some(), "odysseus must have a /data mount");
        if let Some(m) = data_mount {
            assert_eq!(m.host, "workspaces/odysseus-state");
            assert!(!m.read_only, "/data mount must be readwrite");
        }
        let app_mount = plan.mounts.iter().find(|m| m.guest == "/app");
        assert!(app_mount.is_some(), "odysseus must have a /app mount");
        if let Some(m) = app_mount {
            assert_eq!(m.host, "agents/odysseus/build");
            assert!(m.read_only, "/app mount must be readonly");
        }
        assert!(
            plan.mounts.iter().all(|m| m.guest != "/app/data"),
            "odysseus must NOT mount /app/data (relocated to /data via ODYSSEUS_DATA_DIR)"
        );
        Ok(())
    }

    #[test]
    fn odysseus_plan_uses_data_dir_env_and_drops_hardcoded_db_url() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("odysseus")?.plan();
        let has_data_dir = plan
            .env
            .iter()
            .any(|e| e.name == "ODYSSEUS_DATA_DIR" && !e.is_secret && e.value == "/data");
        assert!(
            has_data_dir,
            "odysseus must set ODYSSEUS_DATA_DIR=/data so all writes land on the rw /data mount"
        );
        let has_db_url = plan.env.iter().any(|e| e.name == "DATABASE_URL");
        assert!(
            !has_db_url,
            "odysseus must NOT hardcode DATABASE_URL (let it default to $ODYSSEUS_DATA_DIR/app.db)"
        );
        Ok(())
    }

    #[test]
    fn opencode_network_plan_converts_without_error() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("opencode")?.plan();
        let result = network_plan_to_policy(&plan.network);
        assert!(
            result.is_ok(),
            "opencode conversion failed: {:?}",
            result.err()
        );
        Ok(())
    }

    #[test]
    fn pi_plan_has_expected_egress() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("pi")?.plan();
        assert!(plan.network.default_deny);
        assert_eq!(plan.network.egress_rules.len(), 4);
        assert_eq!(plan.network.egress_rules[0].protocol, Protocol::Tcp);
        assert_eq!(plan.network.egress_rules[0].port, 53);
        assert_eq!(plan.network.egress_rules[0].target, EgressTarget::Host);
        let github_rules: Vec<_> = plan
            .network
            .egress_rules
            .iter()
            .filter(|r| r.port == 443)
            .collect();
        assert_eq!(
            github_rules.len(),
            1,
            "expected exactly one port 443 egress rule"
        );
        let github_rule = github_rules[0];
        assert!(
            matches!(github_rule.target, EgressTarget::Domains(_)),
            "expected domains, got host"
        );
        if let EgressTarget::Domains(hosts) = &github_rule.target {
            assert!(hosts.contains(&"github.com".to_string()));
            assert!(hosts.contains(&"api.github.com".to_string()));
        }
        Ok(())
    }

    #[test]
    fn pi_plan_redirects_config_to_data_dir() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("pi")?.plan();
        let has_agent_dir = plan
            .env
            .iter()
            .any(|e| e.name == "PI_CODING_AGENT_DIR" && !e.is_secret && e.value == "/data/agent");
        assert!(
            has_agent_dir,
            "pi must set PI_CODING_AGENT_DIR=/data/agent so config is read from the rw /data mount"
        );
        let has_offline = plan.env.iter().any(|e| e.name == "PI_OFFLINE");
        assert!(
            !has_offline,
            "pi must not set PI_OFFLINE (egress policy handles security; tools ship in the image)"
        );
        Ok(())
    }

    #[test]
    fn pi_plan_exposes_litellm_master_key_as_env_not_host_bound() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("pi")?.plan();
        // The key must be a process env var so `${LITELLM_MASTER_KEY}` in
        // models.json resolves; otherwise Pi sends no auth key and LiteLLM
        // rejects with "No connected db".
        let in_env = plan
            .env
            .iter()
            .any(|e| e.name == "LITELLM_MASTER_KEY" && e.is_secret);
        assert!(
            in_env,
            "pi must expose LITELLM_MASTER_KEY as a secret EnvVar so models.json substitution resolves"
        );
        let in_secret_env = plan
            .secret_env
            .iter()
            .any(|s| s.name == "LITELLM_MASTER_KEY");
        assert!(
            !in_secret_env,
            "pi must NOT host-bind LITELLM_MASTER_KEY (host-bound secrets are not exposed as guest env vars)"
        );
        Ok(())
    }

    #[test]
    fn pi_plan_has_expected_mounts() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("pi")?.plan();
        assert_eq!(plan.mounts.len(), 2, "pi should have 2 mounts");
        // /app is no longer mounted — the bun binary + assets are baked into
        // the workestrator-pi image (the daemon can't bind-mount from
        // /nix/store). /app/bin/pi resolves via the symlink in the image.
        assert!(
            plan.mounts.iter().all(|m| m.guest != "/app"),
            "pi must NOT have a /app mount (binary baked into image)"
        );
        let data_mount = plan.mounts.iter().find(|m| m.guest == "/data");
        assert!(
            data_mount.is_some(),
            "pi must have a /data mount (persistent state)"
        );
        if let Some(m) = data_mount {
            assert_eq!(m.host, "workspaces/pi-state");
            assert!(!m.read_only, "/data mount must be readwrite");
        }
        let work_mount = plan.mounts.iter().find(|m| m.guest == "/work");
        assert!(
            work_mount.is_some(),
            "pi must have a /work mount (user project)"
        );
        if let Some(m) = work_mount {
            assert!(!m.read_only, "/work mount must be readwrite");
        }
        assert!(
            plan.mounts.iter().all(|m| m.guest != "/workspace"),
            "pi must not mount /workspace (replaced by /work + /data)"
        );
        Ok(())
    }

    // ---- ADR 0021 instance-lifecycle tests ----

    use super::{
        current_rfc3339_utc, days_to_ymd, down_all_instances, format_refuse_message,
        occupancy_from_state, ps, DownStatus, Occupancy,
    };
    use crate::microsandbox::plan::PortMapping;
    // The msb / MSB_HOME-mutating tests below mutate the process-global
    // MSB_HOME env var. They MUST run single-file with every other env-mutating
    // test (notably the `config::tests` family, which serialize via
    // `crate::config::tests::ENV_TEST_LOCK`): an unsynchronized `set_var` racing
    // with a config test's env read panics that test while it holds
    // ENV_TEST_LOCK, poisoning the mutex and cascading ~30 PoisonError failures.
    // Acquiring ENV_TEST_LOCK (the SAME lock the config tests use) makes the
    // whole env-mutating group mutually exclusive. `serial_test`'s separate
    // group cannot help here because the config tests do not use it.

    fn unique_state_dir_runtime(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "workestrate-runtime-{}-{}-{}",
            label,
            std::process::id(),
            nanos,
        ))
    }

    #[test]
    fn refuse_message_is_byte_identical_to_pinned_text() {
        // The text below is pinned by ADR 0021 §2 and asserted by external
        // tooling; do not rephrase without coordinating with the docs track.
        let msg = format_refuse_message("litellm", "personal-litellm");
        let expected = "instance 'personal-litellm' is already running. \
         Use --replace to replace it, --instance <id> for a parallel instance, \
         or 'litellm down' to stop it first.";
        assert_eq!(msg, expected, "refuse message drifted from pinned text");
    }

    #[test]
    fn refuse_message_for_parallel_instance_names_slot_at_id() {
        let msg = format_refuse_message("litellm", "personal-litellm@canary");
        assert!(
            msg.contains("instance 'personal-litellm@canary' is already running"),
            "refuse message must name the occupying instance verbatim: {msg}"
        );
    }

    #[test]
    fn occupancy_from_state_free_when_no_record() -> anyhow::Result<()> {
        let dir = unique_state_dir_runtime("occ-free");
        match occupancy_from_state(&dir, "absent")? {
            Occupancy::Free => {}
            other => panic!("expected Free, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn occupancy_from_state_occupied_when_record_present() -> anyhow::Result<()> {
        let dir = unique_state_dir_runtime("occ-set");
        crate::microsandbox::port_registry::register_sandbox(
            &dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        match occupancy_from_state(&dir, "personal-litellm")? {
            Occupancy::Occupied {
                occupying_instance,
                workload,
                context,
            } => {
                assert_eq!(occupying_instance, "personal-litellm");
                assert_eq!(workload, "litellm");
                assert_eq!(context.as_deref(), Some("personal"));
            }
            other => panic!("expected Occupied, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn occupancy_from_state_free_for_corrupt_record() -> anyhow::Result<()> {
        let dir = unique_state_dir_runtime("occ-corrupt");
        let run_dir = dir.join("var").join("run");
        std::fs::create_dir_all(&run_dir)?;
        std::fs::write(run_dir.join("corrupt.json"), "not json")?;
        match occupancy_from_state(&dir, "corrupt")? {
            Occupancy::Free => {}
            other => panic!("corrupt record should be treated as Free, got {other:?}"),
        }
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn ps_returns_empty_when_no_records() -> anyhow::Result<()> {
        let dir = unique_state_dir_runtime("ps-empty");
        let entries = ps(&dir)?;
        assert!(entries.is_empty(), "expected empty ps list");
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn ps_lists_lifecycle_records_with_port_pairs() -> anyhow::Result<()> {
        let dir = unique_state_dir_runtime("ps-lifecycle");
        let pairs = vec![
            PortMapping {
                host: 14000,
                guest: 4000,
            },
            PortMapping {
                host: 14001,
                guest: 4001,
            },
        ];
        crate::microsandbox::port_registry::register_sandbox_lifecycle(
            &dir,
            "personal-litellm@canary",
            Some("personal"),
            "litellm",
            &[14000, 14001],
            &pairs,
            10000,
            "2026-07-20T14:05:42Z",
        )?;
        let entries = ps(&dir)?;
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.instance, "personal-litellm@canary");
        assert_eq!(e.workload, "litellm");
        assert_eq!(e.context.as_deref(), Some("personal"));
        assert_eq!(e.ports.len(), 2);
        assert_eq!(e.ports[0].host, 14000);
        assert_eq!(e.ports[0].guest, 4000);
        assert_eq!(e.created, "2026-07-20T14:05:42Z");
        assert!(!e.stale);
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn ps_synthesizes_port_pairs_for_legacy_records() -> anyhow::Result<()> {
        let dir = unique_state_dir_runtime("ps-legacy");
        // Legacy record: only the bare host-port list; no port_pairs.
        crate::microsandbox::port_registry::register_sandbox(
            &dir,
            "legacy-litellm",
            None,
            "litellm",
            &[4000, 4001],
        )?;
        let entries = ps(&dir)?;
        assert_eq!(entries.len(), 1);
        let e = &entries[0];
        assert_eq!(e.ports.len(), 2);
        // Legacy: host==guest in the synthesized pairs.
        assert_eq!(e.ports[0].host, 4000);
        assert_eq!(e.ports[0].guest, 4000);
        assert_eq!(e.ports[1].host, 4001);
        assert_eq!(e.ports[1].guest, 4001);
        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn current_rfc3339_utc_ends_with_z_and_has_expected_length() {
        // Sanity: format is YYYY-MM-DDTHH:MM:SSZ = 20 chars.
        let ts = current_rfc3339_utc();
        assert!(
            ts.len() == 20 && ts.ends_with('Z'),
            "unexpected rfc3339 shape: {ts}"
        );
    }

    #[test]
    fn days_to_ymd_known_epoch() {
        // 1970-01-01 is day 0.
        assert_eq!(days_to_ymd(0), (1970, 1, 1));
        // 1970-01-02 is day 1.
        assert_eq!(days_to_ymd(1), (1970, 1, 2));
        // 1971-01-01 is day 365 (1970 was NOT a leap year).
        assert_eq!(days_to_ymd(365), (1971, 1, 1));
        // 2026-01-01: count of days from 1970-01-01.
        // (20454 days; cross-checked against `date -d 2026-01-01 +%s` /86400.)
        assert_eq!(days_to_ymd(20_454), (2026, 1, 1));
    }

    /// RAII guard: point `MSB_HOME` at `path` for the duration of a test and
    /// restore the prior value (or unset it) on drop. The microsandbox SDK
    /// resolves its DB home from `MSB_HOME` inside `db::init_global`, which is
    /// a process-global `OnceCell` — so the value MUST be set before the first
    /// `Sandbox::get` in the process, and restored afterwards so other tests /
    /// the devshell are not disturbed.
    struct MsbHomeGuard {
        prior: Option<std::ffi::OsString>,
    }

    impl MsbHomeGuard {
        fn set(path: &std::path::Path) -> Self {
            let prior = std::env::var_os("MSB_HOME");
            std::env::set_var("MSB_HOME", path);
            Self { prior }
        }
    }

    impl Drop for MsbHomeGuard {
        fn drop(&mut self) {
            match &self.prior {
                Some(v) => std::env::set_var("MSB_HOME", v),
                None => std::env::remove_var("MSB_HOME"),
            }
        }
    }

    /// down_all_instances with an UNREACHABLE msb db must deterministically
    /// surface DownStatus::Error for each attempted record (never panic, never
    /// NotFound). The msb DB is made unreachable by pointing MSB_HOME at a path
    /// under a regular file, so the SDK's `create_dir_all(<MSB_HOME>/db)` fails
    /// with ENOTDIR and `Sandbox::get` errors out.
    ///
    /// The prior incarnation (`down_all_instances_without_msb_returns_error_results`)
    /// was non-deterministic: it asserted Error but actually returned NotFound
    /// whenever a real msb happened to be reachable in the test environment
    /// (the devshell has one). Pinning MSB_HOME at an unwritable path fixes the
    /// outcome to Error regardless of environment.
    #[tokio::test]
    // ENV_TEST_LOCK (std::sync::Mutex) is held across the `.await` below. This
    // is safe because #[tokio::test] uses a single-threaded current-thread
    // runtime with no spawned tasks, so the await can never yield to a task
    // that contends on the lock (no deadlock). The lock MUST span the await so
    // no concurrent env-mutating test (config::tests) races this set_var/read.
    #[allow(clippy::await_holding_lock)]
    async fn down_all_instances_returns_error_when_msb_db_unreachable() -> anyhow::Result<()> {
        // Serialize with ALL env-mutating tests (see the ENV_TEST_LOCK note
        // above) so this set_var(MSB_HOME) cannot race a config test's env read.
        // Declared before `_msb` so the lock is released AFTER MsbHomeGuard
        // restores MSB_HOME on drop (incl. panic).
        let _env_lock = crate::config::tests::ENV_TEST_LOCK.lock().unwrap();
        // `<tmp>/blocker` is a regular file, so `<MSB_HOME>/db` (=
        // `<tmp>/blocker/db`) cannot be created → init_global fails →
        // Sandbox::get returns a hard error.
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-msb-unreachable-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::create_dir_all(&tmp)?;
        std::fs::write(tmp.join("blocker"), b"x")?;
        let _msb = MsbHomeGuard::set(&tmp.join("blocker"));

        let dir = unique_state_dir_runtime("down-all-inst-msb-unreachable");
        crate::microsandbox::port_registry::register_sandbox(
            &dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        let results = down_all_instances(&dir, "litellm").await?;
        assert_eq!(results.len(), 1, "one record was attempted");
        assert!(
            matches!(results[0].status, DownStatus::Error),
            "expected Error status when the msb db is unreachable; got {:?} ({:?})",
            results[0].status,
            results[0].message,
        );
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    /// Positive control: with a writable, empty MSB_HOME the SDK opens its DB
    /// fine and `down_all_instances` resolves to DownStatus::NotFound (msb
    /// reachable, but no such sandbox is registered there).
    ///
    /// `#[ignore]`'d because the SDK pins its DB pool in a process-global
    /// `OnceCell` on the first *successful* `init_global`. If this test and the
    /// Error test both ran in one `cargo test` invocation, whichever
    /// initialized first would fix the home for the whole process and the pair
    /// would be non-deterministic (a writable pin makes the Error test see
    /// NotFound). Keeping this `#[ignore]`'d means:
    ///   - `cargo test`           → Error test runs alone (deterministic Error);
    ///   - `cargo test --ignored` → this test runs alone (deterministic
    ///                               NotFound), the Error test is skipped.
    #[tokio::test]
    #[allow(clippy::await_holding_lock)] // single-threaded test runtime; see Error test
    #[ignore = "shares the SDK process-global DB pool with the Error test; run alone with --ignored"]
    async fn down_all_instances_returns_notfound_when_msb_db_empty_but_openable(
    ) -> anyhow::Result<()> {
        // Serialize with ALL env-mutating tests (see the ENV_TEST_LOCK note above).
        let _env_lock = crate::config::tests::ENV_TEST_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-msb-empty-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::create_dir_all(&tmp)?;
        let _msb = MsbHomeGuard::set(&tmp);

        let dir = unique_state_dir_runtime("down-all-inst-msb-empty");
        crate::microsandbox::port_registry::register_sandbox(
            &dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        let results = down_all_instances(&dir, "litellm").await?;
        assert_eq!(results.len(), 1, "one record was attempted");
        assert!(
            matches!(results[0].status, DownStatus::NotFound),
            "expected NotFound when msb db is openable but empty; got {:?} ({:?})",
            results[0].status,
            results[0].message,
        );
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }
}
