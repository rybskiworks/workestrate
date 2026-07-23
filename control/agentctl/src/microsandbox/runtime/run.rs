use super::super::env::resolve_templated_value;
use super::super::mounts::{apply_plan_mounts, ensure_mount_sources};
use super::super::plan::{PortMapping, SandboxPlan};
use super::super::slots;
use super::super::workload::{EntrypointSpec, SandboxCommand, Workload};
use super::{check_occupied_or_replace, ForegroundConfig, InstanceSpec};
use anyhow::Result;
use microsandbox::sandbox::{exec::ExecEvent, SandboxBuilder};
use microsandbox::Sandbox;

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
    super::super::port_registry::check_port_collisions(&state_dir, &spec.instance, &host_ports)?;

    ensure_mount_sources(&root, &plan)?;

    let policy = super::network_plan_to_policy(&plan.network)?;

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
    let created_at = super::time::current_rfc3339_utc();
    super::super::port_registry::register_sandbox_lifecycle(
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
        let child = super::spawn_detached_service(&instance, &workload.detach_args(spec))?;
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
