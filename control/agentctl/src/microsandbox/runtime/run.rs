use super::super::env::{resolve_templated_value_with, resolve_templated_value_with_env_fallback};
use super::super::mounts::{apply_plan_mounts, ensure_mount_sources};
use super::super::plan::{PortMapping, SandboxPlan};
use super::super::workload::{EntrypointSpec, SandboxCommand, Workload};
use super::{ForegroundConfig, InstanceSpec, check_occupied_or_replace};
use crate::config::SecretViolationPolicy;
use anyhow::Result;
use microsandbox::Sandbox;
use microsandbox::sandbox::{SandboxBuilder, exec::ExecEvent};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::path::{Path, PathBuf};

/// Forward explicit PID 1 selection without changing the separate exec-stream
/// command. Agentd remains the default; no OCI auto-detection is requested.
pub(crate) fn apply_plan_init(
    builder: SandboxBuilder,
    plan: &SandboxPlan,
) -> Result<SandboxBuilder> {
    let Some(init) = &plan.init else {
        return Ok(builder);
    };
    crate::config::validation::validate_init(init)?;
    Ok(match init {
        crate::config::InitConfig::Agentd {} => builder,
        crate::config::InitConfig::Handoff { cmd, args, env } => {
            builder.init_with(cmd, |options| {
                env.iter()
                    .fold(options.args(args), |options, (name, value)| {
                        options.env(name, value)
                    })
            })
        }
    })
}

/// Resolve the host bind IP for the slot `instance` runs in (ADR 0026(a)).
///
/// - PARALLEL slot (`<slot>@<id>`): a per-instance loopback (`127.0.0.N`,
///   `N >= 2`) drawn from the port registry's locked allocator.
/// - SINGLETON slot: the shared bind `127.0.0.1` (UNCHANGED — the well-known
///   address static configs use).
///
/// The allocation happens for parallel slots EVEN when the workload publishes
/// no ports: every parallel instance holds a per-instance IP as its uniform
/// identity (the registry record reserves it until `down`), so addressing
/// does not depend on whether this particular workload exposes a port.
pub(crate) fn slot_bind_ip(instance: &str, state_dir: &Path) -> Result<IpAddr> {
    if crate::microsandbox::slots::instance_id_of(instance).is_some() {
        super::super::port_registry::allocate_loopback_ip(state_dir)
    } else {
        Ok(IpAddr::V4(Ipv4Addr::LOCALHOST))
    }
}

fn reject_if_placeholder(value: &str, placeholder: &Option<String>, label: &str) -> Result<()> {
    if let Some(ph) = placeholder
        && value.trim() == ph.trim()
    {
        anyhow::bail!(
            "{} is set to the placeholder value '{}'. \
             Replace it with a real secret before running this command.",
            label,
            ph
        );
    }
    Ok(())
}

/// Whether a host-bound secret requires verified TLS identity before the
/// host egress proxy substitutes it.
///
/// A secret bound to the local proxy alias may be substituted over plain
/// HTTP (pi → `host.microsandbox.internal:4000`). External hosts keep
/// `require_tls_identity = true` so substitution only fires under TLS
/// interception.
pub(crate) fn secret_requires_tls_identity(host: &str) -> bool {
    !host.eq_ignore_ascii_case(crate::microsandbox::discovery::GUEST_HOST_ALIAS)
}

pub(crate) fn apply_plan_secrets(
    builder: SandboxBuilder,
    plan: &SandboxPlan,
    secrets: &std::collections::HashMap<String, String>,
) -> Result<SandboxBuilder> {
    let mut b = builder;
    for s in &plan.secret_env {
        // FN-9: resolve ${VAR} templates against the merged secrets map
        // returned by load_secrets FIRST; fall back to process env only for
        // variables the map does not carry (ad-hoc user exports, runtime
        // vars). No secret value is read back out of process-global env.
        match resolve_templated_value_with(&s.value, secrets) {
            Ok(value) => {
                reject_if_placeholder(&value, &s.reject_placeholder, &s.name)?;
                if s.required && value.trim().is_empty() {
                    anyhow::bail!(
                        "required secret '{}' is set but empty.\n\
                         Set a real value via `nix develop -c setup-secrets --config <name> update` (or `just setup-secrets --config <name> update`).",
                        s.name
                    );
                }
                for host in &s.allowed_hosts {
                    // Build the SecretEntry explicitly so each host carries
                    // its own require_tls_identity: false for the local proxy
                    // alias (plain HTTP substitution), true for external hosts.
                    // No explicit .placeholder() — the auto-generated
                    // `$MSB_<name>` must stay unchanged.
                    let require_tls = secret_requires_tls_identity(host);
                    b = b.secret(|sb| {
                        sb.env(s.name.clone())
                            .value(value.clone())
                            .allow_host(host.clone())
                            .require_tls_identity(require_tls)
                            // Per-secret violation policy: what happens when
                            // the placeholder reaches a NON-allowed host.
                            // Passthrough maps to all-hosts placeholder
                            // forwarding — the proxy still only substitutes
                            // the real value for allowed hosts (upstream
                            // microsandbox#1354 workaround: a placeholder
                            // quoted in a request body no longer poisons the
                            // session).
                            .on_violation(|v| match s.on_violation {
                                SecretViolationPolicy::Passthrough => v.passthrough_all_hosts(true),
                                SecretViolationPolicy::Block => v.block(),
                                SecretViolationPolicy::BlockAndLog => v.block_and_log(),
                                SecretViolationPolicy::BlockAndTerminate => v.block_and_terminate(),
                            })
                    });
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
                     Set it in the environment or run `nix develop -c setup-secrets --config <name> update` (or `just setup-secrets --config <name> update`)",
                    s.name,
                    e
                ));
            }
        }
    }
    Ok(b)
}

/// Resolve every plan env entry to its final `(name, value)` pair.
///
/// Resolution order is SPLIT by `is_secret`:
///
/// - `is_secret == true` entries resolve `${VAR}` references against the
///   merged `secrets` map ONLY (map lookup, NO process-env fallback — FN-9
///   semantics, matching `apply_plan_secrets`). This is load-bearing: a
///   self-referential secret entry like
///   `EnvVar { name: "LITELLM_MASTER_KEY", value: "${LITELLM_MASTER_KEY}" }`
///   must NOT resolve against the plan env map, where it would self-match
///   its own RAW template and emit the literal `${LITELLM_MASTER_KEY}`
///   string, shadowing the decrypted secret.
/// - `is_secret == false` entries resolve `${VAR}` references against a map
///   of ALL plan env entries FIRST — including depends_on-injected vars
///   appended by `discovery::apply_resolution` (spec 12 §4: the templated
///   composition is the declared env consuming the injected var) — falling
///   back to the process env for names the plan does not carry.
///
/// KNOWN LIMITATION: map values are RAW (unresolved) — a var referencing
/// another templated plan var gets its raw `${...}` form; there is no
/// recursive resolution.
pub(crate) fn resolve_plan_envs(
    plan: &SandboxPlan,
    secrets: &HashMap<String, String>,
) -> Result<Vec<(String, String)>> {
    let vars: HashMap<String, String> = plan
        .env
        .iter()
        .map(|e| (e.name.clone(), e.value.clone()))
        .collect();
    let mut resolved = Vec::with_capacity(plan.env.len());
    for e in &plan.env {
        let value = if e.is_secret {
            resolve_templated_value_with(&e.value, secrets)?
        } else {
            resolve_templated_value_with_env_fallback(&e.value, &vars)?
        };
        reject_if_placeholder(&value, &e.reject_placeholder, &e.name)?;
        resolved.push((e.name.clone(), value));
    }
    Ok(resolved)
}

pub(crate) fn apply_plan_envs(
    builder: SandboxBuilder,
    plan: &SandboxPlan,
    secrets: &HashMap<String, String>,
) -> Result<SandboxBuilder> {
    let mut b = builder;
    for (name, value) in resolve_plan_envs(plan, secrets)? {
        b = b.env(name, value);
    }
    Ok(b)
}

pub(crate) async fn run_service_foreground(
    sandbox: &Sandbox,
    mut config: ForegroundConfig,
) -> Result<()> {
    // Taken before `exec_stream` moves the command args below: the shim and
    // the broker reservation are relinquished on every exit path via
    // `shutdown_ssh_shim` / `shutdown_broker_vm`.
    let mut ssh_shim = config.ssh_shim.take();
    let mut broker = config.broker.take();
    let mut exec_handle = match sandbox
        .exec_stream(&config.command.binary, config.command.arguments)
        .await
    {
        Ok(handle) => handle,
        Err(e) => {
            shutdown_ssh_shim(&mut ssh_shim);
            shutdown_broker_vm(&mut broker);
            return Err(anyhow::anyhow!(
                "failed to start {} process: {}",
                config.service_label,
                e
            ));
        }
    };

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
            shutdown_ssh_shim(&mut ssh_shim);
            shutdown_broker_vm(&mut broker);
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
            shutdown_ssh_shim(&mut ssh_shim);
            shutdown_broker_vm(&mut broker);
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
    for (host, guest) in &config.mounts {
        println!("mount: {} -> {}", host, guest);
    }
    if let Err(e) = tokio::signal::ctrl_c().await {
        // Even if the signal handler fails, attempt to stop the sandbox
        // before propagating the error.
        let _ = sandbox.stop().await;
        let _ = drain_handle.await;
        shutdown_ssh_shim(&mut ssh_shim);
        shutdown_broker_vm(&mut broker);
        return Err(anyhow::anyhow!("signal handler error: {}", e));
    }
    // Post-session teardown is REQUIRED: a failed stop means a silently
    // leaked sandbox, so it must fail the command (nonzero exit), not just
    // eprintln. The `log_stop_errors` knob still controls the eprintln; the
    // error propagates either way.
    let stop_err = match sandbox.stop().await {
        Ok(()) => None,
        Err(e) => {
            if config.log_stop_errors {
                eprintln!("failed to stop sandbox '{}': {}", config.sandbox_name, e);
            }
            Some(e)
        }
    };

    // Drain any remaining buffered log events before returning.
    // Use a timeout so a hung exec channel doesn't block forever.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), drain_handle).await;

    // The foreground service ends here on every path below: relinquish the
    // SSH shim (socket, thread, CID binding) and the broker reservation
    // with the sandbox.
    shutdown_ssh_shim(&mut ssh_shim);
    shutdown_broker_vm(&mut broker);

    match stop_err {
        None => {
            println!("Sandbox '{}' stopped", config.sandbox_name);
            Ok(())
        }
        Some(e) => Err(anyhow::anyhow!(
            "failed to stop sandbox '{}' after session end: {}",
            config.sandbox_name,
            e
        )),
    }
}

/// Start `exec_program` with `exec_args` inside `sandbox` via an interactive
/// TTY attach, blocking until the guest process exits OR the user detaches
/// (ctrl-]). Then stop the sandbox.
///
/// This is the interactive counterpart to `run_service_foreground`, used by
/// interactive TUI agent workloads. The SDK's `attach_with` handles raw mode,
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
        mounts,
        ssh_shim,
        broker,
    } = config;
    let SandboxCommand { binary, arguments } = command;

    println!(
        "Sandbox '{}' started (interactive TUI; ctrl-] to detach)",
        sandbox_name
    );
    for (host, guest) in &mounts {
        println!("mount: {} -> {}", host, guest);
    }

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

    // Post-session teardown is REQUIRED: a failed stop means a silently
    // leaked sandbox, so it must fail the command (nonzero exit), not just
    // eprintln. The `log_stop_errors` knob still controls the eprintln; the
    // error propagates either way.
    let stop_err = match sandbox.stop().await {
        Ok(()) => None,
        Err(e) => {
            if log_stop_errors {
                eprintln!("failed to stop sandbox '{}': {}", sandbox_name, e);
            }
            Some(e)
        }
    };

    // The interactive service ends here: relinquish the SSH shim (socket,
    // thread, CID binding) and the broker reservation with the sandbox.
    if let Some(shim) = ssh_shim {
        shim.shutdown();
    }
    if let Some(broker) = broker {
        broker.shutdown();
    }

    match (outcome, stop_err) {
        (Ok(()), None) => {
            println!("Sandbox '{}' stopped", sandbox_name);
            Ok(())
        }
        (Ok(()), Some(e)) => Err(anyhow::anyhow!(
            "failed to stop sandbox '{}' after session end: {}",
            sandbox_name,
            e
        )),
        // When BOTH fail, the session error is the primary failure and is
        // kept; the stop failure rides along as context so it is not lost.
        (Err(e), None) => Err(e),
        (Err(e), Some(stop)) => Err(e.context(format!(
            "also failed to stop sandbox '{}': {}",
            sandbox_name, stop
        ))),
    }
}

/// Assign a probed free port to every mapping whose `host == 0` (auto).
///
/// Each 0-marked port is probed individually via
/// [`crate::microsandbox::port_registry::probe_free_ports`] on `bind`. The
/// probed value is NOT a reservation (same TOCTOU contract as --port-auto:
/// the post-create atomic check+register closes the registry-side window).
/// A candidate is REJECTED (re-probed) when it equals any already-concrete
/// host in `ports` — either a declared non-zero port of this workload or a
/// previously assigned auto port — so a workload mixing `host = 4000` and
/// `host = 0` can never double-publish. Bounded retries; fail-closed on
/// exhaustion.
fn apply_auto_ports(state_dir: &Path, bind: IpAddr, ports: &mut [PortMapping]) -> Result<()> {
    let auto_indices: Vec<usize> = ports
        .iter()
        .enumerate()
        .filter(|(_, p)| p.host == 0)
        .map(|(i, _)| i)
        .collect();
    if auto_indices.is_empty() {
        return Ok(());
    }
    const MAX_AUTO_ATTEMPTS: usize = 16;
    let mut attempts = 0usize;
    let mut next = 0usize;
    while next < auto_indices.len() {
        attempts += 1;
        if attempts > MAX_AUTO_ATTEMPTS {
            anyhow::bail!(
                "host=0 auto port(s): failed to probe {} distinct free port(s) on {} after {} attempts; \
                 retry, or publish explicit host ports",
                auto_indices.len(),
                bind,
                MAX_AUTO_ATTEMPTS
            );
        }
        let probed = super::super::port_registry::probe_free_ports(state_dir, bind, 1)?[0];
        let collides = ports.iter().any(|p| p.host == probed);
        if collides {
            continue;
        }
        ports[auto_indices[next]].host = probed;
        next += 1;
    }
    Ok(())
}

/// Apply the workload's declared `instance.port` policy to the plan's ports
/// (ADR 0030 Phase 3 / addendum 2 U6):
///   - Strict(n): current behavior — the declared host ports stand; an
///     occupied fixed port errors at create (check_port_collisions_locked).
///   - Auto: every declared port behaves as host=0 (auto-allocate at boot).
///   - Preferred { preferred, on_occupied }: try preferred; if occupied, walk
///     the on_occupied chain (increment band / auto / fail).
///
/// The effective ports land in the plan (and thus the registry record).
///
/// `exclude_instance` is the `--replace` target, when one is in play: the
/// instance being replaced never blocks its OWN preferred port. Selection
/// here runs BEFORE the replace teardown (`teardown_for_replace` /
/// `check_occupied_or_replace`, below in `build_sandbox`), so without the
/// exclusion the predecessor's registry record and OS listener would make
/// its own preferred port look occupied and force an increment on every
/// `up --replace` cycle (4000 → 4001 → 4002).
fn apply_instance_port_policy<W: Workload>(
    state_dir: &Path,
    workload: &W,
    bind: IpAddr,
    ports: &mut [PortMapping],
    exclude_instance: Option<&str>,
) -> Result<()> {
    let Some(port) = workload.instance_port() else {
        return Ok(());
    };
    match port {
        crate::config::InstancePort::Strict(_) => {
            // Current behavior: declared hosts stand; occupied -> hard error
            // at create (check_port_collisions_locked). No-op here.
        }
        crate::config::InstancePort::Auto => {
            // Every declared port auto-allocates (host=0 semantics).
            for p in ports.iter_mut() {
                p.host = 0;
            }
            apply_auto_ports(state_dir, bind, ports)?;
        }
        crate::config::InstancePort::Preferred(pref) => {
            let chain = pref
                .on_occupied
                .clone()
                .map(|c| c.0)
                .unwrap_or_else(|| crate::config::PortOccupiedChain::default_on_occupied().0);
            // The occupied predicate: a registry record on the same bind, OR
            // an OS bind, OR a declared host in the plan — EXCLUDING the
            // primary port being selected: its declared host is typically
            // `preferred` itself (the canonical `host == preferred` config
            // shape, e.g. litellm's `host = 4000` + `preferred = 4000`), so
            // counting it would permanently self-occupy the preferred port.
            // Sibling declared hosts still count, so the increment band can
            // never steal another port the workload declares.
            let siblings = ports.get(1..).unwrap_or(&[]);
            let is_occupied = |candidate: u16| {
                port_is_occupied(state_dir, bind, candidate, siblings, exclude_instance)
            };
            let auto_allocate = || {
                super::super::port_registry::probe_free_ports(state_dir, bind, 1)
                    .ok()
                    .and_then(|v| v.into_iter().next())
            };
            // Only apply to the port(s) the policy governs. The policy is
            // per-workload; apply to the FIRST declared port (the primary)
            // and leave named/other ports as declared. For a single-port
            // workload (litellm) this is the api port.
            let chosen =
                match select_preferred_port(pref.preferred, &chain, &is_occupied, &auto_allocate) {
                    PortSelection::Chosen(chosen) => chosen,
                    PortSelection::Exhausted(msg) => anyhow::bail!("{msg}"),
                };
            if let Some(p) = ports.first_mut() {
                p.host = chosen;
            }
        }
    }
    Ok(())
}

/// Whether `candidate` is occupied on `bind`: a registry record on the same
/// bind holds it, an OS bind succeeds, or a declared host in `ports` equals it.
///
/// `exclude_instance` (the `--replace` target): records belonging to the
/// instance being replaced are skipped — its teardown releases them. When the
/// candidate is registered ONLY to the excluded instance the OS-bind probe is
/// bypassed too: the listener is the predecessor's own, which the pending
/// teardown releases. A failed OS bind with NO matching excluded-instance
/// record is a FOREIGN live listener and still counts as occupied —
/// fail-closed, unchanged.
fn port_is_occupied(
    state_dir: &Path,
    bind: IpAddr,
    candidate: u16,
    ports: &[PortMapping],
    exclude_instance: Option<&str>,
) -> bool {
    if ports.iter().any(|p| p.host == candidate) {
        return true;
    }
    // Registry-recorded ports on the same bind (defense-in-depth).
    if let Ok(records) = super::super::port_registry::list_records(state_dir) {
        let mut self_hold = false;
        for r in &records {
            if r.bind_ip != bind || !r.ports.contains(&candidate) {
                continue;
            }
            if Some(r.instance.as_str()) == exclude_instance {
                self_hold = true;
                continue;
            }
            return true; // a FOREIGN record holds the candidate
        }
        if self_hold {
            // Only the replace target's own record lists the candidate; its
            // OS listener (if any) is the predecessor's own and the pending
            // teardown releases it. Skip the OS-bind probe.
            return false;
        }
    }
    // OS bind probe (SO_REUSEADDR off — a plain TcpListener::bind).
    std::net::TcpListener::bind((bind, candidate)).is_err()
}

/// The outcome of a preferred-port selection walk (ADR 0030 addendum 2 U6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PortSelection {
    /// A concrete port was chosen (preferred, an increment candidate, or an
    /// auto-allocated ephemeral).
    Chosen(u16),
    /// The chain exhausted without a free port; the error lists the attempts.
    Exhausted(String),
}

/// Decide the port for a preferred-port workload given an OCCUPIED predicate
/// (ADR 0030 addendum 2 U6 behavior matrix). PURE — the caller supplies the
/// occupancy check so the decision logic is unit-testable without real binds.
///
/// `preferred` is the port to try first. `chain` is the on_occupied chain
/// (default ["increment", "auto"] when None). `is_occupied(port)` returns
/// true when the port is taken (by a registry record on the same bind, an
/// OS bind, or a declared host in the plan). `auto_allocate()` returns a
/// fresh ephemeral port (or None if none available).
///
/// Walk: try `preferred`; if free -> Chosen(preferred). Else iterate the
/// chain in order:
///   - increment: probe the band in order (bare = preferred+1..preferred+100;
///     {limit=N} = preferred+1..preferred+N; {range=[S,E]} = S..=E), each
///     candidate probe-before-bind (skip occupied); the first free wins.
///   - auto: auto_allocate() -> Chosen.
///   - fail: terminal — the chain ends here (nothing after it, validated).
///
/// If the chain ends without a choice -> Exhausted(error listing the attempts
/// in order, e.g. "increment (band exhausted), auto (none available)").
pub(crate) fn select_preferred_port(
    preferred: u16,
    chain: &[crate::config::PortOccupiedStep],
    is_occupied: &dyn Fn(u16) -> bool,
    auto_allocate: &dyn Fn() -> Option<u16>,
) -> PortSelection {
    if !is_occupied(preferred) {
        return PortSelection::Chosen(preferred);
    }
    let mut attempts: Vec<String> = Vec::new();
    for step in chain {
        match step {
            crate::config::PortOccupiedStep::Bare(crate::config::PortOccupiedBare::Increment) => {
                let band = preferred.saturating_add(1)..=preferred.saturating_add(100);
                if let Some(p) = first_free_in_band(band, is_occupied) {
                    return PortSelection::Chosen(p);
                }
                attempts.push("increment (band exhausted)".to_string());
            }
            crate::config::PortOccupiedStep::Increment(
                crate::config::types::ParameterizedIncrement { increment },
            ) => {
                let band: Vec<u16> = match (&increment.limit, &increment.range) {
                    (Some(limit), None) => {
                        (preferred.saturating_add(1)..=preferred.saturating_add(*limit)).collect()
                    }
                    (None, Some((start, end))) => (*start..=*end).collect(),
                    _ => Vec::new(), // validated elsewhere; treat as empty
                };
                if let Some(p) = first_free_in_band(band.into_iter(), is_occupied) {
                    return PortSelection::Chosen(p);
                }
                attempts.push("increment (band exhausted)".to_string());
            }
            crate::config::PortOccupiedStep::Bare(crate::config::PortOccupiedBare::Auto) => {
                if let Some(p) = auto_allocate() {
                    return PortSelection::Chosen(p);
                }
                attempts.push("auto (none available)".to_string());
            }
            crate::config::PortOccupiedStep::Bare(crate::config::PortOccupiedBare::Fail) => {
                attempts.push("fail".to_string());
                break;
            }
        }
    }
    PortSelection::Exhausted(format!(
        "port selection exhausted for preferred {}: {} — no free port found",
        preferred,
        attempts.join(", ")
    ))
}

fn first_free_in_band<I: Iterator<Item = u16>>(
    band: I,
    is_occupied: &dyn Fn(u16) -> bool,
) -> Option<u16> {
    band.into_iter().find(|p| !is_occupied(*p))
}

/// Write every per-mount policy file beneath the loader-approved root and
/// record the loader-RELATIVE token on each plan mount.
///
/// ORDERING INVARIANT (cross-link: [`super::teardown_for_replace`] /
/// `policy_file::remove_policy_dir`): this write must be the LAST writer of
/// the instance's policy dir before `builder.create()` / `handle.start()`.
/// The replace teardowns wipe the policy dir, so this helper is only ever
/// called (a) in the `StartExisting` branch — an earlier `down` may have
/// removed the dir while the sandbox was stopped, and `handle.start()`
/// re-loads the policy — and (b) AFTER the conflict-chain match and the
/// `--replace` teardown, immediately before builder assembly. Running it
/// earlier (before teardown) makes the fork loader fail closed with
/// "mount policy file not found" on every replace boot.
fn write_mount_policy_files<W: Workload>(
    spec: &InstanceSpec,
    workload: &W,
    plan: &mut SandboxPlan,
) -> Result<()> {
    for m in &mut plan.mounts {
        if let Some(program) = workload.mount_policy_for(&m.guest) {
            let slug = crate::microsandbox::policy_file::mount_slug(&m.guest);
            let (_abs, rel) = crate::microsandbox::policy_file::write_policy_file(
                &spec.instance,
                &slug,
                program,
            )?;
            // The plan carries the loader-RELATIVE token; the fork loader
            // resolves it beneath the MSB_HOME-anchored approved root.
            m.policy_file = Some(rel);
        }
    }
    Ok(())
}

/// Outcome of [`build_sandbox`] (ADR 0030 Phase 0): the caller either gets a
/// live sandbox + foreground config to exec the workload into, or learns the
/// slot was REUSED (already running healthy/booting) and has nothing to do.
///
/// `Sandbox` is boxed so the enum is small (the `Reused` variant carries no
/// data; an unboxed `Sandbox` would make the whole enum ~1.4KB). The config
/// rides boxed too since it owns the SSH shim handle (socket path, thread
/// join handle, registry bindings).
pub(crate) enum BuildOutcome {
    Ready(Box<Sandbox>, Box<ForegroundConfig>),
    Reused,
}

/// The `on_skew` disposition (ADR 0030 V-addendum §V4, wired by ADR 0032
/// A3 provenance stamps) — PURE, exhaustively matrix-tested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkewDisposition {
    /// Adopt the running instance (reuse/start per the conflict chain).
    Proceed,
    /// Adopt AND print the divergence notice (the warn default).
    Warn(String),
    /// Tear down the skewed instance and create fresh on the new inputs.
    ReplaceNow,
}

/// The ONE canonical `on_skew` decision (ADR 0030 V-addendum §V4 + ADR 0032
/// §Provenance stamps). BOTH stamps must be present: an absent stamp is a
/// PRE-STAMP (unknown-version) record — it NEVER warns and NEVER replaces
/// for any policy (pinned; old records are never auto-stale). Equal stamps
/// proceed for every policy. On a real divergence:
/// - `Warn` (the default when `on_skew` is None) → [`SkewDisposition::Warn`]
///   with the PINNED notice naming the instance and both 4-char hash
///   prefixes;
/// - `Replace` → [`SkewDisposition::ReplaceNow`] (teardown + fresh create);
/// - `ReuseSilently` → [`SkewDisposition::Proceed`] without comment.
pub(crate) fn skew_disposition(
    on_skew: Option<crate::config::OnSkew>,
    instance: &str,
    recorded: Option<&str>,
    current: Option<&str>,
) -> SkewDisposition {
    let policy = on_skew.unwrap_or_default();
    let (Some(recorded), Some(current)) = (recorded, current) else {
        // Unknown-version posture: either stamp missing → never fire.
        return SkewDisposition::Proceed;
    };
    if recorded == current {
        return SkewDisposition::Proceed;
    }
    match policy {
        crate::config::OnSkew::Warn => SkewDisposition::Warn(skew_warn_message(
            instance,
            crate::microsandbox::provenance::short_hash(recorded),
            crate::microsandbox::provenance::short_hash(current),
        )),
        crate::config::OnSkew::Replace => SkewDisposition::ReplaceNow,
        crate::config::OnSkew::ReuseSilently => SkewDisposition::Proceed,
    }
}

/// The PINNED divergence-notice wording (ADR 0032 §Provenance stamps /
/// ADR 0030 §V4): `warning: instance '<instance>' was built from config
/// <rec4>; current inputs config <cur4> (on_skew = "warn": proceeding with
/// reuse)` — 4-char hash prefixes per [`PROVENANCE_DISPLAY_LEN`].
fn skew_warn_message(instance: &str, recorded4: &str, current4: &str) -> String {
    format!(
        "warning: instance '{instance}' was built from config {recorded4}; \
         current inputs config {current4} (on_skew = \"warn\": proceeding with reuse)"
    )
}

/// The CURRENT config-hash input view of a workload for `instance`: the
/// freshly-built plan with exactly the deterministic mutations
/// [`build_sandbox`] applies BEFORE any chain decision — instance-scoped
/// state mounts for `per-dir` strategies (keyed on the instance id's
/// @-suffix) and the name override to the spec's instance name. The
/// detached-up PARENT short-circuit (`up_service_with_spec`) computes its
/// skew comparison through this helper while the child (`build_sandbox`)
/// hashes its already-mutated plan directly; host ports (mutated later by
/// port policy/probing) are excluded from the hash, so parent and child
/// CANNOT diverge (equivalence pinned by test).
pub(crate) fn current_config_hash_for_workload<W: Workload>(
    workload: &W,
    instance: &str,
) -> String {
    let mut plan = workload.plan();
    apply_current_view_mutations(&mut plan, workload, instance);
    crate::microsandbox::provenance::config_hash_of_plan(&plan)
}

/// The shared pre-chain plan mutations of [`build_sandbox`] (instance-state
/// scoping + name override), extracted so the parent-side skew comparison
/// applies the identical view.
fn apply_current_view_mutations<W: Workload>(plan: &mut SandboxPlan, workload: &W, instance: &str) {
    let instance_state_key: Option<&str> =
        if workload.instance_strategy() == crate::config::InstanceStrategy::PerDir {
            crate::microsandbox::slots::instance_id_of(instance)
        } else {
            None
        };
    if let Some(key) = instance_state_key {
        for m in &mut plan.mounts {
            m.host = super::super::mounts::instance_scoped_state_path(
                &m.host,
                workload.name(),
                Some(key),
            );
        }
    }
    plan.name = instance.to_string();
}

/// msb state generations fail-closed up gate (see
/// [`crate::microsandbox::generation`]): refuses an up whose resolved msb
/// home is pinned to a DIFFERENT generation than the baked msb store-path
/// key, a pre-generation legacy root (`db/` at `$HOME/.microsandbox`), or
/// an ambiguous multi-generation root with no `current` symlink — every
/// refusal names `./scripts/host-provision.sh` (generation converge). An
/// explicit MSB_HOME is verbatim for STATE placement but its generation
/// IDENTITY is canonicalized (`generation_key_of_resolved_home`), so the
/// nix wrapper's `$HOME/.microsandbox/current` default resolves through
/// the symlink and the mismatch check engages under the wrapper. An
/// UNMANAGED baked msb (no nix store path) skips the gate (legacy
/// single-generation behavior). Returns the generation dir that should
/// receive the `.booted-ok` marker on a successful up (Current/Healed with
/// the matching key, or an explicit home canonicalizing to a matching
/// `generations/<key12>` dir — including via `current`); `None` for the
/// unmanaged posture, a non-generation explicit override, or a fresh root.
///
/// ORDERING INVARIANT: the verdict is EVALUATED at the TOP of
/// [`build_sandbox`] and at the top of the detached-up parent
/// (`up_service_with_spec` non-foreground block) — before gather_facts /
/// chain decisions / ANY teardown or mount-policy write. Evaluation is
/// cheap and side-effect-safe (its only side effect is the rule-3
/// best-effort heal); the REFUSAL is then applied lazily at each mutation
/// point via [`gate_refusal`] — never partial: a mismatch refuses before
/// any state change:
/// - chain-Replace and on_skew="replace" teardowns (child AND detached
///   parent) refuse BEFORE `teardown_for_replace`;
/// - StartExisting refuses BEFORE the policy write + `handle.start()` —
///   starting a STOPPED old-generation sandbox with the NEW baked binary
///   would forward-mutate the old generation in place;
/// - the create path consumes the verdict before
///   `check_occupied_or_replace` / builder assembly / `create()`;
/// - REUSE of an already-RUNNING sandbox is deliberately UNGATED
///   (adjudication): reusing a live sandbox mutates no msb state, and
///   converge's quiesce gate separately refuses while it is live.
fn check_generation_up_gate() -> Result<Option<PathBuf>> {
    use crate::microsandbox::generation;
    let baked = generation::baked_generation_key();
    if baked == generation::UNMANAGED_KEY {
        return Ok(None);
    }
    match generation::resolve_msb_home_generation() {
        generation::HomeResolution::Current { gen_dir, key }
        | generation::HomeResolution::Healed { gen_dir, key } => {
            if key == baked {
                Ok(Some(gen_dir))
            } else {
                anyhow::bail!(
                    "refusing up: msb state generation mismatch — the resolved home is generation \
                     '{key}' but the baked msb is generation '{baked}'; \
                     run ./scripts/host-provision.sh (generation converge)"
                )
            }
        }
        generation::HomeResolution::Explicit(path) => {
            match generation::generation_key_of_resolved_home(&path) {
                Some((_, key)) if key != baked => anyhow::bail!(
                    "refusing up: MSB_HOME resolves to generation '{key}' but the baked msb \
                     is generation '{baked}'; run ./scripts/host-provision.sh (generation converge)"
                ),
                Some((gen_dir, _)) => Ok(Some(gen_dir)),
                None => Ok(None),
            }
        }
        generation::HomeResolution::LegacyRoot(_) => anyhow::bail!(
            "refusing up: pre-generation msb home (db/ at $HOME/.microsandbox root); \
             run ./scripts/host-provision.sh to absorb it as generations/legacy"
        ),
        generation::HomeResolution::Ambiguous(keys) => anyhow::bail!(
            "refusing up: multiple msb state generations ({}) and no current symlink; \
             run ./scripts/host-provision.sh (generation converge)",
            keys.join(", ")
        ),
        generation::HomeResolution::Fresh(_) => Ok(None),
    }
}

/// The refusal half of a stored [`check_generation_up_gate`] verdict:
/// `Some(error)` (message preserved) when the gate refused, WITHOUT
/// consuming the verdict — the create path later consumes it for the
/// `.booted-ok` marker dir. Applied at every pre-mutation point (chain /
/// on_skew / parent-side teardowns, StartExisting) so a generation
/// mismatch refuses BEFORE any state change while the adjudicated-ungated
/// Reuse path never consults it.
fn gate_refusal(gate: &Result<Option<PathBuf>>) -> Option<anyhow::Error> {
    gate.as_ref().err().map(|e| anyhow::anyhow!("{e}"))
}

/// msb state generations: write the empty `.booted-ok` marker in the
/// resolved generation dir after a successful up — best-effort only: write
/// errors are logged and ignored, never fail the up.
fn mark_generation_booted(gen_dir: &Path) {
    if let Err(e) = std::fs::write(gen_dir.join(".booted-ok"), b"") {
        eprintln!(
            "warning: could not write .booted-ok marker in {}: {e}",
            gen_dir.display()
        );
    }
}

/// ADR 0036 §4 pre-create gate evaluation (pure given the `probe`):
/// `build_sandbox` passes the live `read_nested_probe()` immediately before
/// `builder.create()`; unit tests pass mocked probes. Off skips silently
/// (legacy workloads never touch this path); otherwise the shared
/// `nested_up_decision` applies — `prefer` degrades, `require` refuses
/// fail-closed, a home-final seal refuses either ask.
///
/// The returned bool reports the nested-ON verdict that the create path
/// threads into `SandboxBuilder::nested_virt(...)` (the fork's first-class
/// `resources.nested_virt` spec option, default OFF): `require`
/// that passed the decision is ON; `prefer` is ON only when the
/// probe reports the full host nested offering (a DEGRADED prefer must NOT
/// be ON); `off` is never ON. This CONSUMES the decision result — the
/// probe is not re-read.
fn check_nested_up_gate<W: Workload + ?Sized>(
    workload: &W,
    probe: &crate::microsandbox::nested::NestedProbe,
) -> Result<bool> {
    use crate::microsandbox::nested::{host_offers_nested, nested_up_decision};
    let resolution = workload.virtualization_resolution();
    if resolution.effective == crate::config::NestedMode::Off {
        return Ok(false);
    }
    nested_up_decision(
        workload.name(),
        resolution.effective,
        probe,
        resolution.frozen_by.as_deref(),
    )?;
    Ok(match resolution.effective {
        crate::config::NestedMode::Require => true,
        crate::config::NestedMode::Prefer => host_offers_nested(probe),
        crate::config::NestedMode::Off => false,
    })
}

/// Thread the [`check_nested_up_gate`] verdict into the sandbox builder as
/// the fork's first-class `nested_virt` spec option (supersedes the
/// same-day env flag): per-sandbox data on the persisted spec, not
/// process-global state — an ambient operator export can no longer leak
/// nested virt into an off/degraded workload, and parallel creates cannot
/// race on a shared env key.
fn apply_nested_virt(builder: SandboxBuilder, nested_on: bool) -> SandboxBuilder {
    builder.nested_virt(nested_on)
}

/// Release the foreground SSH shim, if any. Best-effort (see
/// [`crate::microsandbox::broker::SshShimHandle::shutdown`]): teardown
/// must not fail the service exit it follows.
fn shutdown_ssh_shim(shim: &mut Option<crate::microsandbox::broker::SshShimHandle>) {
    if let Some(shim) = shim.take() {
        shim.shutdown();
    }
}

/// Release the foreground broker VM reservation, if any. Best-effort (see
/// [`crate::microsandbox::broker::BrokerVmHandle::shutdown`]): teardown
/// must not fail the service exit it follows.
fn shutdown_broker_vm(broker: &mut Option<crate::microsandbox::broker::BrokerVmHandle>) {
    if let Some(broker) = broker.take() {
        broker.shutdown();
    }
}

/// Prepare, resolve, and create the sandbox plus the foreground config used
/// to run the workload's real command.
pub(crate) async fn build_sandbox<W: Workload>(
    workload: &W,
    spec: &InstanceSpec,
) -> Result<BuildOutcome> {
    // msb state generations fail-closed up gate: EVALUATED FIRST — before
    // secrets/plan/gather_facts/chain decisions and therefore before ANY
    // teardown or mount-policy write (ordering invariant: see
    // `check_generation_up_gate`). The REFUSAL is applied lazily at each
    // mutation point (`gate_refusal`) so the adjudicated-ungated Reuse
    // path returns untouched; the create path consumes the verdict for the
    // `.booted-ok` marker dir.
    let generation_gate = check_generation_up_gate();
    // Load secrets from .env.enc across the resolved layers. FN-9: the
    // merged map is threaded into env/secret resolution below — it is NOT
    // written into process-global env (parallel build_sandbox calls would
    // race on shared keys). Only called for exec/up paths — plan/check
    // never reach here.
    let secrets = crate::microsandbox::secrets_loader::load_secrets()?;

    let mut plan = workload.plan();

    // ADR 0030 V-addendum §V2: instance-scoped state mounts. For a
    // `per-dir`-strategy workload the state mount root
    // (`workspaces/<name>-state[/...]`) gains the instance-key segment (the
    // instance id's @-suffix) so each per-dir instance gets its OWN state
    // subdir; singleton/non-per-dir workloads pass key=None and every host
    // is byte-identical to before (no layout churn). Applied to the plan
    // BEFORE `resolve_mount_roots_owned` so the plan-time existence
    // preflight and the runtime mount resolution agree on the scoped path.
    let instance_state_key: Option<&str> =
        if workload.instance_strategy() == crate::config::InstanceStrategy::PerDir {
            crate::microsandbox::slots::instance_id_of(&spec.instance)
        } else {
            None
        };
    if let Some(key) = instance_state_key {
        for m in &mut plan.mounts {
            m.host = super::super::mounts::instance_scoped_state_path(
                &m.host,
                workload.name(),
                Some(key),
            );
        }
    }

    // F1/F2 mount-root resolution is shared with the plan-time existence
    // preflight via `resolve_mount_roots_owned` (mounts.rs) so the runtime
    // build path and the plan path agree on where a mount host resolves.
    // See that helper for the F2 lazy flake-root gate + F1 content-root
    // fallback rationale (spec 17 / spec 21 §6.1).
    let mount_roots_owned = super::super::mounts::resolve_mount_roots_owned(workload, &plan)?;
    let mount_roots = mount_roots_owned.as_roots();

    // Override the plan name with the spec instance name so display matches
    // the actual sandbox identity (slot for singleton, slot@id for parallel).
    plan.name = spec.instance.clone();

    // NEW: build the guest-visible env view and seed BEFORE sandbox create.
    // Defined-but-unbound secret detection needs the merged config's secret IDs.
    let defined_secrets: std::collections::HashSet<String> = crate::config::load_config()?
        .secrets
        .keys()
        .cloned()
        .collect();
    let env_view =
        crate::microsandbox::env::build_seed_env_view(&plan, &secrets, &defined_secrets)?;
    // `--reseed` rides the spec: template seeds re-render over existing
    // targets; without it the seed step is byte-identical to before.
    // `instance_state_key` scopes seed targets to the per-instance state
    // subdir exactly like the mount hosts above (ADR 0030 V-addendum §V2).
    workload.prepare(&env_view, spec.reseed, instance_state_key)?;

    // Hoist state_dir before the occupancy check so it can be reused for
    // collision detection and lifecycle registration below.
    let state_dir = crate::config::resolve_state_dir();

    // ADR 0030 stale-record GC: drop registry records whose backing msb
    // sandbox is definitively gone BEFORE any registry-reading selection
    // below (bind-IP allocation, instance.port occupancy, --port-auto/auto
    // probing). A record left behind by a crash or an out-of-band `msb rm`
    // would otherwise block its ports/loopback IP forever — occupancy checks
    // treat every record as authoritative with no liveness reconciliation.
    // Fail-closed: when msb is unreachable nothing is pruned, and a prune
    // failure never blocks the up.
    if let Err(e) = super::reconcile::prune_stale_records(&state_dir).await {
        eprintln!("warning: stale registry record prune failed: {e}");
    }

    // NOTE: the per-mount policy-file write does NOT live here. It runs after
    // the conflict-chain decision and any replace teardown, immediately before
    // builder assembly — see [`write_mount_policy_files`] for the ordering
    // invariant.

    // ADR 0026(a)/C2: resolve the slot's bind IP BEFORE the builder port
    // loop. Parallel slots draw a per-instance loopback from the locked
    // allocator; the singleton keeps the shared 127.0.0.1 bind.
    //
    // TOCTOU honesty: this allocation is NOT a reservation — it happens
    // pre-create, so two concurrent parallel `up`s of the same workload can
    // draw the same IP before either registers. The post-create atomic
    // check+register (FN-6) keyed on (bind_ip, port) closes the window: the
    // loser surfaces a clear port-collision error at registration.
    let bind_ip = slot_bind_ip(&spec.instance, &state_dir)?;

    // ADR 0026(c)/C3: --port-auto replaces every declared host port with a
    // lock-probed free port on the slot's bind (guest unchanged). The probed
    // ports are NOT a reservation — see probe_free_ports' doc comment; the
    // post-create atomic check+register (FN-6) closes the remaining window
    // and the chosen ports are recorded in the instance record. Composes
    // with per-instance binds: a parallel slot probes on its 127.0.0.N, the
    // singleton probes on the shared 127.0.0.1.
    //
    // ADR 0030 Phase 3: the workload's declared instance.port drives
    // selection. --port-auto (all ports) still wins when both are used.
    if !spec.port_auto {
        // --replace: the instance being replaced never blocks its own
        // preferred port. Selection runs BEFORE the replace teardown
        // (ChainStep::Replace / check_occupied_or_replace below), so the
        // predecessor's registry record and OS listener would otherwise
        // force an increment on every `up --replace` cycle. Foreign holders
        // still block (fail-closed). NOTE: keyed on the EXPLICIT
        // `spec.replace`; the conflict chain's Replace disposition is only
        // known after `decide_step` below, so a chain-driven replace of a
        // zombie/stale predecessor still increments (documented limitation).
        let replace_target = spec.replace.then_some(spec.instance.as_str());
        apply_instance_port_policy(
            &state_dir,
            workload,
            bind_ip,
            &mut plan.ports,
            replace_target,
        )?;
    }
    if spec.port_auto && !plan.ports.is_empty() {
        let probed =
            super::super::port_registry::probe_free_ports(&state_dir, bind_ip, plan.ports.len())?;
        for (p, host) in plan.ports.iter_mut().zip(probed) {
            p.host = host;
        }
    } else {
        // P1: host = 0 marks a per-port auto allocation (namespaced ports).
        // Each 0-marked port is probed individually on the slot's bind. This
        // runs even when --port-auto is absent; --port-auto above wins when
        // both are used (it overwrites every host, including 0-marked ones).
        apply_auto_ports(&state_dir, bind_ip, &mut plan.ports)?;
    }

    // Host ports from the (possibly --port-auto-mutated) plan: single source
    // of truth for the collision check and the legacy `ports` field in the
    // lifecycle state record, so the record always carries the EFFECTIVE
    // (probed) ports — `ps`/`down` recover them. Hoisted BEFORE the
    // occupancy gate so the chain's `start` element (StartExisting) and the
    // create path share the same values.
    let host_ports: Vec<u16> = plan.ports.iter().map(|p| p.host).collect();

    // Register with full lifecycle metadata so `ps` and `down --all` work.
    // Each pair carries the slot's bind IP (ADR 0026): the record feeds `ps`
    // and the (bind_ip, port)-keyed collision model.
    let port_pairs: Vec<PortMapping> = plan
        .ports
        .iter()
        .map(|p| PortMapping {
            bind_ip,
            ..p.clone()
        })
        .collect();

    // ADR 0030 Phase 0: status+dir-aware occupancy routed through the
    // workload's conflict chain — the per-workload
    // `instance.on_conflict` default when declared (ADR 0030 U11
    // precedence), otherwise the built-in reuse → start → replace chain —
    // instead of the status-blind refuse gate. The operator's EXPLICIT
    // `--replace` (`spec.replace`) wins over the chain on EVERY path — the
    // detached parent's short-circuit (`up_service_with_spec`) and this
    // child/foreground build gate alike — forcing ChainStep::Replace
    // (teardown + fresh create) even when the chain would reuse or fail.
    let declared_ports: Vec<u16> = plan.ports.iter().map(|p| p.host).collect();
    let facts = super::reconcile::gather_facts(&state_dir, &spec.instance, &declared_ports).await?;
    let chain = workload.instance_conflict_chain();
    let step = super::reconcile::decide_step(&chain, &facts, &spec.instance, spec.replace)?;
    // ADR 0032 A3: an on_skew = "replace" disposition decided on the Reuse
    // arm tears down here and falls through to the create path with replace
    // semantics (the slot must be re-created fresh).
    let mut skew_replaced = false;
    match step {
        super::reconcile::ChainStep::Reuse => {
            // msb state generations adjudication: reusing an
            // already-RUNNING sandbox is deliberately UNGATED — it mutates
            // no msb state, and converge's quiesce gate separately refuses
            // while the sandbox is live. Only the on_skew="replace"
            // fall-through mutates (teardown), so IT consults the gate.
            // A1/P3: adopting a record registered under a different context
            // than the active one is allowed but surfaced (warn-and-proceed).
            super::reconcile::warn_on_context_drift(
                &spec.instance,
                facts.record.as_ref().and_then(|r| r.context.as_deref()),
                crate::config::active_context_name().as_deref(),
            );
            // ADR 0030 V-addendum §V4 wired (ADR 0032 A3): compare the
            // reused instance's recorded config stamp against the CURRENT
            // build inputs. `plan` at this point carries exactly the view
            // the create path would hash (instance-scoped mounts applied,
            // name overridden; host ports are excluded from the hash), so
            // this matches the detached parent's helper-derived comparison
            // by construction.
            let recorded = facts.record.as_ref().and_then(|r| r.config_hash.as_deref());
            let current = crate::microsandbox::provenance::config_hash_of_plan(&plan);
            match skew_disposition(
                workload.instance_on_skew(),
                &spec.instance,
                recorded,
                Some(&current),
            ) {
                SkewDisposition::Proceed => return Ok(BuildOutcome::Reused),
                SkewDisposition::Warn(message) => {
                    eprintln!("{message}");
                    return Ok(BuildOutcome::Reused);
                }
                SkewDisposition::ReplaceNow => {
                    // msb state generations gate: refuse BEFORE the
                    // teardown (never partial — a mismatch must not destroy
                    // the old-generation sandbox).
                    if let Some(e) = gate_refusal(&generation_gate) {
                        return Err(e);
                    }
                    eprintln!(
                        "on_skew = \"replace\": replacing instance '{}' (build inputs diverged)",
                        spec.instance
                    );
                    super::teardown_for_replace(&state_dir, &spec.instance).await?;
                    skew_replaced = true;
                }
            }
        }
        super::reconcile::ChainStep::Fail => {
            anyhow::bail!(
                "{}",
                super::format_refuse_message(&spec.workload, &spec.instance)
            );
        }
        super::reconcile::ChainStep::StartExisting => {
            // msb state generations gate: refuse BEFORE the policy write +
            // `handle.start()` — starting a STOPPED old-generation sandbox
            // with the NEW baked binary would forward-mutate the old
            // generation in place.
            if let Some(e) = gate_refusal(&generation_gate) {
                return Err(e);
            }
            // A1/P3: adopting a record registered under a different context
            // than the active one is allowed but surfaced (warn-and-proceed).
            super::reconcile::warn_on_context_drift(
                &spec.instance,
                facts.record.as_ref().and_then(|r| r.context.as_deref()),
                crate::config::active_context_name().as_deref(),
            );
            // The policy dir may be gone (an earlier `down` removed it while
            // the sandbox was stopped); `handle.start()` re-loads the policy,
            // so the write must cover this path too (ordering invariant: see
            // [`write_mount_policy_files`]).
            write_mount_policy_files(spec, workload, &mut plan)?;
            // A3 carry-forward (ADR 0032 §Provenance stamps): starting a
            // STOPPED sandbox does not change its build inputs — the prior
            // record's stamps ride forward unchanged (absent prior → None,
            // the pre-stamp posture).
            let (prior_image_out_hash, prior_config_hash) = facts
                .record
                .as_ref()
                .map(|r| (r.image_out_hash.clone(), r.config_hash.clone()))
                .unwrap_or((None, None));
            let (sandbox, mut config) = start_existing_sandbox(
                &state_dir,
                spec,
                workload,
                bind_ip,
                &host_ports,
                &port_pairs,
                prior_image_out_hash.as_deref(),
                prior_config_hash.as_deref(),
            )
            .await?;
            config.mounts = plan
                .mounts
                .iter()
                .map(|m| (m.host.clone(), m.guest.clone()))
                .collect();
            return Ok(BuildOutcome::Ready(Box::new(sandbox), Box::new(config)));
        }
        super::reconcile::ChainStep::Replace => {
            // msb state generations gate: refuse BEFORE the teardown
            // (never partial — a mismatch must not destroy the
            // old-generation sandbox).
            if let Some(e) = gate_refusal(&generation_gate) {
                return Err(e);
            }
            super::teardown_for_replace(&state_dir, &spec.instance).await?;
        }
        super::reconcile::ChainStep::Start => {}
    }
    // msb state generations gate: the create path CONSUMES the verdict
    // here — still before any create-path mutation
    // (`check_occupied_or_replace`, the policy write, builder assembly,
    // `create()`). The Ok generation dir rides to the `.booted-ok` marker
    // write after registration below.
    let generation_dir = generation_gate?;
    if spec.replace {
        check_occupied_or_replace(spec, &state_dir).await?;
    }

    // ORDERING INVARIANT: write the per-mount policy files ONLY here — after
    // the chain decision + any replace teardown (`teardown_for_replace` wipes
    // the policy dir via `remove_policy_dir`), immediately before builder
    // assembly / `create()`. The write must be the last writer before create;
    // writing earlier lets the replace teardown delete the freshly written
    // file and the fork loader then fails closed with "mount policy file not
    // found". See [`write_mount_policy_files`].
    write_mount_policy_files(spec, workload, &mut plan)?;

    ensure_mount_sources(&mount_roots, &plan)?;

    let policy = super::network_plan_to_policy(&plan.network)?;

    // ADR 0030 addendum 2026-08-26: the SDK validates sandbox names
    // (`@` is illegal), so the BUILDER gets the encoded msb name — via the
    // ONE SDK-boundary wrapper [`super::builder_for`]; every registry/
    // record surface keeps the workestrate identity.
    let mut builder = super::builder_for(&spec.instance)
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
    builder = apply_plan_init(builder, &plan)?;

    // ADR 0026(a): the singleton publishes on the shared bind via `.port`
    // (`.port_bind(127.0.0.1, ...)`); parallel slots publish on their
    // per-instance loopback via `.port_bind`.
    let is_parallel = crate::microsandbox::slots::instance_id_of(&spec.instance).is_some();
    for port in &plan.ports {
        builder = if is_parallel {
            builder.port_bind(bind_ip, port.host, port.guest)
        } else {
            builder.port(port.host, port.guest)
        };
    }

    builder = apply_plan_envs(builder, &plan, &secrets)?;
    builder = apply_plan_mounts(builder, &mount_roots, &plan)?;
    builder = apply_plan_secrets(builder, &plan, &secrets)?;

    // Chain-driven Replace (no explicit `--replace`) tore down state above,
    // but `ensure_mount_sources` can re-create the sandbox dir for rw mounts
    // self-mounted under it (e.g. `${MSB_HOME}/sandboxes/<name>/logs`); the
    // fork's non-replace create gate (`prepare_create_target`) refuses any
    // pre-existing dir with `SandboxAlreadyExists`, so the create must carry
    // replace semantics whenever the build DECIDED Replace, not only on the
    // explicit flag. An explicit `--replace` keeps working unchanged. A3:
    // an on_skew = "replace" disposition (the Reuse arm's fall-through)
    // joins the same rule — the skewed instance was torn down above.
    let create_with_replace = should_create_with_replace(spec.replace, step) || skew_replaced;
    let builder = if create_with_replace {
        builder.replace()
    } else {
        builder
    };
    // ADR 0036 §4: fail-closed nested-virt pre-create gate. Evaluated HERE —
    // after the policy-file write + `ensure_mount_sources`, immediately
    // before `builder.create()` — so a refusal leaves NO partial sandbox
    // behind (no create was attempted, no record written). `prefer` never
    // refuses (it degrades; the degrade is already in the plan provenance);
    // `off` skips silently. The decision shares
    // `microsandbox::nested::nested_up_decision` with the `plan` warn path
    // so warn and refuse can never disagree. The gate ALSO threads its
    // nested-ON verdict into the builder via the fork's first-class
    // `nested_virt` spec-field option (default OFF): the builder call is
    // per-sandbox data on the persisted spec, not process-global state, so
    // an ambient shell env can no longer leak nested virt into an
    // off/degraded workload and parallel creates cannot race on a shared
    // env key; a refused workload never reaches create and never sets the
    // option.
    let nested_on =
        check_nested_up_gate(workload, &crate::microsandbox::nested::read_nested_probe())?;
    let builder = apply_nested_virt(builder, nested_on);
    // Guest SSH policy: thread the workload's credential view into the
    // builder as the fork's first-class `network.ssh` spec option (policy
    // only — the socket path stays host-side). Grant-less plans leave the
    // builder untouched.
    let builder = crate::microsandbox::broker::apply_ssh_policy(builder, plan.credentials.as_ref());
    // Reserve the launch identity before the runtime exists. The SDK assigns
    // this CID to libkrun and verifies it before guest execution. Each listener
    // is launch-specific, and dropping its handle cleans up on failed creation
    // or registration; no network-slot identity or shared-listener takeover.
    let ssh_shim = match plan.credentials.as_ref() {
        Some(credentials) => {
            crate::microsandbox::broker::ensure_ssh_shim(&state_dir, &spec.instance, credentials)
                .await?
        }
        None => None,
    };
    let builder = match ssh_shim.as_ref() {
        Some(shim) => builder.guest_cid(shim.cid).ssh_broker_endpoint(
            shim.socket_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("SSH launch endpoint must be valid UTF-8"))?,
        ),
        None => builder,
    };
    // This builder launches the workload, not the custody broker. Raw DLP
    // patterns contain credential material and belong only on the direct
    // broker management path, never in this workload's bootstrap.
    let sandbox = builder.create().await?;

    let created_at = super::time::current_rfc3339_utc();
    // A3 (ADR 0032 §Provenance stamps): the create path knows the FULL
    // build-input view — compute the config hash over the FINAL mutated
    // plan (instance-scoped mounts, name override, port policy applied) and
    // take the image out-hash from the resolved tag. Recorded so reuse
    // decisions and `ps` can compare against current inputs.
    let config_hash = crate::microsandbox::provenance::config_hash_of_plan(&plan);
    let image_out_hash = plan
        .image
        .as_deref()
        .and_then(crate::microsandbox::provenance::image_out_hash_from_tag);
    // FN-6: atomic check + register under ONE registry-lock hold. The
    // collision check must not run as a separate pre-create call: it
    // released the lock before `create().await`, letting a concurrent
    // `up` claim the same port in between (check-then-register TOCTOU).
    // The sandbox create cannot move inside the lock (it is async and
    // would deadlock the lock file), so a same-port race is still
    // possible mid-create; this closes the post-create registration
    // window, and the loser surfaces a clear port-collision error here.
    //
    // ADR 0026/C2: the resolved slot bind (shared 127.0.0.1 for the
    // singleton, per-instance 127.0.0.N for parallel slots) is registered
    // with the record.
    super::super::port_registry::check_and_register_sandbox_lifecycle(
        &state_dir,
        &spec.instance,
        spec.context.as_deref(),
        workload.name(),
        bind_ip,
        &host_ports,
        &port_pairs,
        &created_at,
        &workload.namespace(),
        spec.source_dir.as_deref(),
        // A2 (ADR 0032 §Image tags): the CREATE path knows the image —
        // record the resolved store tag so the keep-last-N GC never prunes
        // a tag a running sandbox was created with.
        plan.image.as_deref(),
        // A3 (ADR 0032 §Provenance stamps): the create-time stamps.
        image_out_hash.as_deref(),
        Some(&config_hash),
    )?;
    // msb state generations: the create + registration succeeded — record
    // the first verified up for this generation. Best-effort; a write
    // failure never fails the up (see `mark_generation_booted`). This is
    // the shared create-success point for both `up_service_with_spec` and
    // `exec_agent_with_spec` (both funnel through `build_sandbox`).
    if let Some(dir) = &generation_dir {
        mark_generation_booted(dir);
    }
    // Broker VM: reserve the shared host handles only once the sandbox
    // identity is durably registered, beside the shim. Grant-less and
    // guest-bound-only plans take no reservation. The KVM boot itself is
    // still deferred: until it lands, the custody relay fails broker-bound
    // sessions closed.
    let broker = match plan.credentials.as_ref() {
        Some(credentials) => {
            crate::microsandbox::broker::ensure_broker_vm(&state_dir, &spec.instance, credentials)?
        }
        _ => None,
    };
    let config = ForegroundConfig {
        sandbox_name: sandbox.name().to_string(),
        service_label: workload.name().to_string(),
        command: workload.exec(),
        log_stop_errors: workload.log_stop_errors(),
        mounts: plan
            .mounts
            .iter()
            .map(|m| (m.host.clone(), m.guest.clone()))
            .collect(),
        ssh_shim,
        broker,
    };
    Ok(BuildOutcome::Ready(Box::new(sandbox), Box::new(config)))
}

/// ADR 0030 Phase 0 `start` element: start a stopped/crashed sandbox via
/// msb `handle.start()` (the capability the CLI never used), register the
/// lifecycle record (the stopped sandbox may have no record), and return the
/// live sandbox + foreground config so the caller execs the workload command
/// into it. Preserves the sandbox state (filesystem/config); the service
/// process is re-run by the caller.
//
// too_many_arguments: the positional tail mirrors the registry's lifecycle
// entry point (identity, bind, ports, metadata); the A3 carry-forward stamps
// (ADR 0032) complete it, exactly as they completed
// check_and_register_sandbox_lifecycle. A params struct is deferred to the
// C2 wiring commit.
#[allow(clippy::too_many_arguments)]
async fn start_existing_sandbox<W: Workload>(
    state_dir: &Path,
    spec: &InstanceSpec,
    workload: &W,
    bind_ip: IpAddr,
    host_ports: &[u16],
    port_pairs: &[PortMapping],
    image_out_hash: Option<&str>,
    config_hash: Option<&str>,
) -> Result<(Sandbox, ForegroundConfig)> {
    // ADR 0030 addendum 2026-08-26: single encoded-name lookup — records
    // drive the re-START, so a legacy raw-@ sandbox simply reads as gone.
    // The encoding lives in the ONE SDK-boundary wrapper [`get_sandbox`].
    let handle = super::get_sandbox(&spec.instance).await?;
    let sandbox = handle.start().await?;
    let created_at = super::time::current_rfc3339_utc();
    super::super::port_registry::check_and_register_sandbox_lifecycle(
        state_dir,
        &spec.instance,
        spec.context.as_deref(),
        workload.name(),
        bind_ip,
        host_ports,
        port_pairs,
        &created_at,
        &workload.namespace(),
        spec.source_dir.as_deref(),
        // A re-START of an existing sandbox: the running tag is unknown
        // here (ADR 0032 §Image tags — documented None posture).
        None,
        // A3 carry-forward: the caller passes the PRIOR record's stamps
        // (starting a stopped sandbox does not change its build inputs);
        // absent prior → None (pre-stamp posture).
        image_out_hash,
        config_hash,
    )?;
    let config = ForegroundConfig {
        sandbox_name: sandbox.name().to_string(),
        service_label: workload.name().to_string(),
        command: workload.exec(),
        log_stop_errors: workload.log_stop_errors(),
        mounts: Vec::new(),
        // Restarting a stopped sandbox reuses its prior identity without a
        // fresh launch epoch or CID: no new listener is bound here.
        ssh_shim: None,
        // Same for the broker reservation: a restart claims no new host
        // handles.
        broker: None,
    };
    Ok((sandbox, config))
}

/// Whether the detached-`up` PARENT must run the replace teardown itself
/// before spawning the child: true ONLY for [`super::reconcile::ChainStep::Replace`].
/// `Start` is a free slot and `StartExisting` starts the stopped/crashed
/// sandbox in place via `handle.start()` (reconcile.rs `decide_step` docs) —
/// neither tears down. Pure decision so the contract is unit-testable; the
/// `up_service_with_spec` flow itself is not (it spawns processes).
pub(crate) fn should_teardown_in_parent(step: super::reconcile::ChainStep) -> bool {
    matches!(step, super::reconcile::ChainStep::Replace)
}

/// Whether the sandbox create must run with replace semantics: true when the
/// explicit `--replace` flag is set OR when the build DECIDED
/// [`super::reconcile::ChainStep::Replace`] (chain-driven replace). The
/// Replace arm tears down state (`teardown_for_replace`), but
/// `ensure_mount_sources` can then RE-CREATE the sandbox dir for rw mounts
/// self-mounted under it (e.g. `${MSB_HOME}/sandboxes/<name>/logs`); the
/// fork's non-replace create gate (`prepare_create_target`) refuses any
/// pre-existing dir with `SandboxAlreadyExists`, so the create must use
/// replace semantics whenever the Replace teardown ran — otherwise a
/// chain-driven replace fails at create. Pure decision so the contract is
/// unit-testable (mirrors [`should_teardown_in_parent`]).
pub(crate) fn should_create_with_replace(
    spec_replace: bool,
    step: super::reconcile::ChainStep,
) -> bool {
    spec_replace || matches!(step, super::reconcile::ChainStep::Replace)
}

pub async fn up_service_with_spec<W: Workload>(
    workload: &W,
    spec: &InstanceSpec,
    foreground: bool,
) -> Result<()> {
    if !foreground {
        // msb state generations fail-closed up gate: EVALUATED before any
        // parent-side mutation (the pre-spawn teardowns below); refusal is
        // applied lazily (`gate_refusal`) so the adjudicated-ungated Reuse
        // short-circuit returns untouched — ordering invariant: see
        // `check_generation_up_gate`.
        let generation_gate = check_generation_up_gate();
        // ADR 0030 Phase 0: short-circuit reuse/fail in the PARENT — a child
        // that reconciles to reuse would exit within the FS-8 grace window
        // and be misreported as an immediate failure. An explicit `--replace`
        // preempts the chain (decide_step): the Reuse/Fail short-circuit
        // never fires, the child is always spawned, and the child re-derives
        // Replace (`--replace` rides detach_args) to tear down + recreate.
        let state_dir = crate::config::resolve_state_dir();
        let declared_ports: Vec<u16> = workload.plan().ports.iter().map(|p| p.host).collect();
        let facts =
            super::reconcile::gather_facts(&state_dir, &spec.instance, &declared_ports).await?;
        let chain = workload.instance_conflict_chain();
        let step = super::reconcile::decide_step(&chain, &facts, &spec.instance, spec.replace)?;
        match step {
            super::reconcile::ChainStep::Reuse => {
                // msb state generations adjudication: the reuse
                // short-circuit is deliberately UNGATED (reusing a live
                // sandbox mutates no msb state — see
                // `check_generation_up_gate`); only the on_skew="replace"
                // teardown below consults the gate.
                // A1/P3: adopting a record registered under a different
                // context than the active one is allowed but surfaced
                // (warn-and-proceed).
                super::reconcile::warn_on_context_drift(
                    &spec.instance,
                    facts.record.as_ref().and_then(|r| r.context.as_deref()),
                    crate::config::active_context_name().as_deref(),
                );
                // ADR 0032 A3 — the PARENT-side skew site (closes the
                // per-dir landed note: this short-circuit is the ONLY path
                // detached-up reuses take; the child's build_sandbox Reuse
                // arm is unreachable for them). The comparison uses the
                // same helper-derived current view the child would hash.
                let recorded = facts.record.as_ref().and_then(|r| r.config_hash.as_deref());
                let current = current_config_hash_for_workload(workload, &spec.instance);
                match skew_disposition(
                    workload.instance_on_skew(),
                    &spec.instance,
                    recorded,
                    Some(&current),
                ) {
                    SkewDisposition::ReplaceNow => {
                        // on_skew = "replace": tear down here (the same
                        // hardened teardown the should_teardown_in_parent
                        // arm runs) and CONTINUE to spawn the child — the
                        // slot is now free, so the child re-derives Start
                        // and creates fresh with new stamps. msb state
                        // generations gate: refuse BEFORE the teardown
                        // (never partial).
                        if let Some(e) = gate_refusal(&generation_gate) {
                            return Err(e);
                        }
                        eprintln!(
                            "on_skew = \"replace\": replacing instance '{}' (build inputs diverged)",
                            spec.instance
                        );
                        super::teardown_for_replace(&state_dir, &spec.instance).await?;
                    }
                    SkewDisposition::Warn(message) => {
                        eprintln!("{message}");
                        println!("instance '{}' is already running — reusing", spec.instance);
                        return Ok(());
                    }
                    SkewDisposition::Proceed => {
                        println!("instance '{}' is already running — reusing", spec.instance);
                        return Ok(());
                    }
                }
            }
            super::reconcile::ChainStep::Fail => {
                anyhow::bail!(
                    "{}",
                    super::format_refuse_message(&spec.workload, &spec.instance)
                );
            }
            step => {
                if should_teardown_in_parent(step) {
                    // msb state generations gate: refuse BEFORE the
                    // parent-side teardown (never partial).
                    if let Some(e) = gate_refusal(&generation_gate) {
                        return Err(e);
                    }
                    // The FS-8 grace (spawn.rs: 500ms) only catches an IMMEDIATE
                    // child exit; a replace teardown takes seconds, so a
                    // child-side teardown failure would be logged by the child
                    // while the parent exited 0 (the 2026-08-23 dev-host
                    // swallow). Run the idempotent teardown HERE so its failure
                    // is a nonzero parent exit; the child re-derives Replace
                    // (`--replace` rides detach_args) and re-runs the now no-op
                    // teardown (`teardown_for_replace`: Sandbox::get →
                    // SandboxNotFound → Ok path).
                    println!(
                        "tearing down existing instance '{}' before replace",
                        spec.instance
                    );
                    super::teardown_for_replace(&state_dir, &spec.instance).await?;
                }
            }
        }
        // msb state generations gate: the SPAWNING paths (Start /
        // StartExisting / post-teardown replace — the ungated Reuse
        // short-circuit already returned) consume the verdict HERE so a
        // refusal exits the parent nonzero instead of spawning a child that
        // would fail at its own gate. The generation dir is unused in the
        // parent (the `.booted-ok` marker write is child-side, in
        // `build_sandbox`).
        let _ = generation_gate?;
        let instance = spec.instance.clone();
        let child = super::spawn_detached_service(&instance, &workload.detach_args(spec))?;
        println!(
            "Sandbox '{}' started in background (PID {}). Logs: {}",
            instance,
            child.id(),
            super::detached_log_path(&instance).display()
        );
        return Ok(());
    }
    match build_sandbox(workload, spec).await? {
        BuildOutcome::Ready(sandbox, config) => run_service_foreground(&sandbox, *config).await,
        BuildOutcome::Reused => {
            println!("instance '{}' is already running — reusing", spec.instance);
            Ok(())
        }
    }
}

pub async fn exec_agent_with_spec<W: Workload>(workload: &W, spec: &InstanceSpec) -> Result<()> {
    match build_sandbox(workload, spec).await? {
        BuildOutcome::Ready(sandbox, config) => run_service_interactive(&sandbox, *config).await,
        BuildOutcome::Reused => {
            println!("instance '{}' is already running — reusing", spec.instance);
            Ok(())
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
#[allow(unsafe_code)]
mod tests {
    use super::*;
    use crate::config::test_support::unique_state_dir;
    use crate::microsandbox::plan::{EnvVar, HostBoundSecret, NetworkPlan};

    // ---- ADR 0036: pre-create gate via mocked probes (no host I/O) ----

    /// Fake workload carrying only a nested ask (+ optional frozen seal):
    /// the gate under test resolves through the workload's own
    /// `virtualization_resolution`, exactly like `build_sandbox`.
    #[derive(Debug)]
    struct NestedGateWorkload {
        ask: Option<crate::config::NestedMode>,
        frozen_by: Option<String>,
    }

    impl Workload for NestedGateWorkload {
        fn name(&self) -> &str {
            "kvm-job"
        }
        fn plan(&self) -> SandboxPlan {
            empty_plan_with_env(Vec::new())
        }
        fn exec(&self) -> SandboxCommand {
            SandboxCommand::with_args("", &[])
        }
        fn virtualization_nested(&self) -> Option<crate::config::NestedMode> {
            self.ask
        }
        fn virtualization_resolution(
            &self,
        ) -> crate::microsandbox::nested::VirtualizationResolution {
            crate::microsandbox::nested::VirtualizationResolution {
                effective: self.ask.unwrap_or(crate::config::NestedMode::Off),
                allowed: self.frozen_by.is_none(),
                frozen_out: self.frozen_by.is_some(),
                frozen_by: self.frozen_by.clone(),
                origin: "personal".to_string(),
                sealed: self.frozen_by.is_some(),
            }
        }
    }

    /// Legacy workloads (no ask) skip the gate on EVERY probe — the gate is
    /// a no-op unless a workload opts in with an explicit nested≠off.
    #[test]
    fn nested_gate_skips_off_workloads() {
        use crate::microsandbox::nested::NestedProbe;
        let wl = NestedGateWorkload {
            ask: None,
            frozen_by: None,
        };
        for probe in [NestedProbe::absent(), NestedProbe::full()] {
            check_nested_up_gate(&wl, &probe)
                .unwrap_or_else(|e| panic!("off must skip the gate: {e}"));
        }
    }

    /// Require refuses fail-closed on a lacking host with the exact
    /// plan-§4 shape, passes on a full host; prefer never refuses.
    #[test]
    fn nested_gate_require_refuses_prefer_degrades() {
        use crate::config::NestedMode;
        use crate::microsandbox::nested::NestedProbe;
        let require = NestedGateWorkload {
            ask: Some(NestedMode::Require),
            frozen_by: None,
        };
        let err = check_nested_up_gate(&require, &NestedProbe::absent())
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("workload 'kvm-job' requires KVM")
                && err.contains("virtualization.nested=\"require\"")
                && err.contains("ADR 0036 §4")
                && err.contains("hint:"),
            "refusal carries the exact error shape: {err}"
        );
        check_nested_up_gate(&require, &NestedProbe::full())
            .unwrap_or_else(|e| panic!("require on a full host passes: {e}"));
        let prefer = NestedGateWorkload {
            ask: Some(NestedMode::Prefer),
            frozen_by: None,
        };
        check_nested_up_gate(&prefer, &NestedProbe::absent())
            .unwrap_or_else(|e| panic!("prefer never refuses: {e}"));
    }

    /// A home-final seal refuses even on a full host, citing the seal.
    #[test]
    fn nested_gate_seal_refuses_on_full_host() {
        use crate::config::NestedMode;
        use crate::microsandbox::nested::NestedProbe;
        let sealed = NestedGateWorkload {
            ask: Some(NestedMode::Require),
            frozen_by: Some("home-registry".to_string()),
        };
        let err = check_nested_up_gate(&sealed, &NestedProbe::full())
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("seal forbids it") && err.contains("home-registry"),
            "seal refusal cites the seal: {err}"
        );
    }

    /// The gate's bool = "the `nested_virt` builder option for the create":
    /// require/prefer on a FULL host set it; off (any probe) and a DEGRADED
    /// prefer (kvm absent, or the nested module param not affirmatively Y)
    /// must NOT; require on a lacking host refuses (exact-error shape
    /// pinned above).
    #[test]
    fn nested_gate_nested_on_bool_matrix() {
        use crate::config::NestedMode;
        use crate::microsandbox::nested::NestedProbe;
        let workload = |ask| NestedGateWorkload {
            ask: Some(ask),
            frozen_by: None,
        };
        // Full host: require AND prefer resolve nested-ON.
        assert!(
            check_nested_up_gate(&workload(NestedMode::Require), &NestedProbe::full()).unwrap()
        );
        assert!(check_nested_up_gate(&workload(NestedMode::Prefer), &NestedProbe::full()).unwrap());
        // Degraded prefer never sets the option: kvm absent...
        assert!(
            !check_nested_up_gate(&workload(NestedMode::Prefer), &NestedProbe::absent()).unwrap()
        );
        // ...or the nested module param not affirmatively Y.
        let degraded_param = NestedProbe {
            kvm_present: true,
            kvm_accessible: true,
            cpu_flag: true,
            nested_param: None,
            arch_supported: true,
        };
        assert!(!check_nested_up_gate(&workload(NestedMode::Prefer), &degraded_param).unwrap());
        // Require on a lacking host refuses (no option is ever applied —
        // a refusal never reaches create).
        assert!(
            check_nested_up_gate(&workload(NestedMode::Require), &NestedProbe::absent()).is_err()
        );
        // Off is option-less on EVERY probe.
        let off = NestedGateWorkload {
            ask: None,
            frozen_by: None,
        };
        for probe in [NestedProbe::absent(), NestedProbe::full()] {
            assert!(!check_nested_up_gate(&off, &probe).unwrap());
        }
    }

    /// The verdict reaches the fork as the spec-field option: ON sets
    /// `resources.nested_virt` on the built spec, OFF leaves it at the
    /// fork's default-off (which the ambient shell env can no longer
    /// override — the env gate is gone).
    #[test]
    fn apply_nested_virt_sets_the_spec_option() {
        let on = apply_nested_virt(Sandbox::builder("nested-on"), true);
        assert!(on.spec().resources.nested_virt);
        let off = apply_nested_virt(Sandbox::builder("nested-off"), false);
        assert!(!off.spec().resources.nested_virt);
    }

    fn empty_plan_with_env(env: Vec<EnvVar>) -> SandboxPlan {
        SandboxPlan {
            name: "test".to_string(),
            image: None,
            workdir: None,
            command: Vec::new(),
            cpus: None,
            memory_mib: None,
            env,
            secret_env: Vec::new(),
            credentials: None,
            ports: Vec::new(),
            mounts: Vec::new(),
            network: NetworkPlan {
                egress_default_deny: false,
                ingress_default_deny: false,
                egress_rules: Vec::new(),
                deny_rules: Vec::new(),
                ingress_rules: Vec::new(),
                egress_defaults_seal: None,
                ingress_defaults_seal: None,
            },
            instance_policy: None,
            virtualization: None,
            init: None,
        }
    }

    fn secrets_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn explicit_init_reaches_sdk_without_replacing_workload_entrypoint() {
        let mut plan = empty_plan_with_env(vec![]);
        plan.command = vec!["/app/workload".into()];
        plan.init = Some(crate::config::InitConfig::Handoff {
            cmd: "/init".into(),
            args: vec!["a b".into(), "${literal}".into()],
            env: std::collections::BTreeMap::from([
                ("Z".into(), "last".into()),
                ("A".into(), "first".into()),
            ]),
        });
        let builder =
            Sandbox::builder("init-test").entrypoint(["/bin/sh", "-c", "tail -f /dev/null"]);
        let builder = apply_plan_init(builder, &plan).unwrap();
        let init = builder.spec().init.as_ref().unwrap();
        assert_eq!(init.cmd, "/init");
        assert_eq!(init.args, ["a b", "${literal}"]);
        assert_eq!(
            init.env,
            [("A".into(), "first".into()), ("Z".into(), "last".into())]
        );
        assert_eq!(
            builder.spec().runtime.entrypoint.as_ref().unwrap(),
            &["/bin/sh", "-c", "tail -f /dev/null"]
        );
        assert_eq!(plan.command, ["/app/workload"]);
    }

    #[test]
    fn omitted_or_agentd_init_keeps_default_sdk_pid_one() {
        let mut plan = empty_plan_with_env(vec![]);
        for init in [None, Some(crate::config::InitConfig::Agentd {})] {
            plan.init = init;
            let builder = apply_plan_init(Sandbox::builder("init-default"), &plan).unwrap();
            assert!(builder.spec().init.is_none());
        }
    }

    #[test]
    fn invalid_init_plan_is_rejected_at_sdk_boundary() {
        let mut plan = empty_plan_with_env(vec![]);
        plan.init = Some(crate::config::InitConfig::Handoff {
            cmd: "auto".into(),
            args: vec![],
            env: Default::default(),
        });
        assert!(apply_plan_init(Sandbox::builder("init-invalid"), &plan).is_err());
    }

    fn empty_plan_with_secrets(secret_env: Vec<HostBoundSecret>) -> SandboxPlan {
        SandboxPlan {
            name: "test".to_string(),
            image: None,
            workdir: None,
            command: Vec::new(),
            cpus: None,
            memory_mib: None,
            env: Vec::new(),
            secret_env,
            credentials: None,
            ports: Vec::new(),
            mounts: Vec::new(),
            network: NetworkPlan {
                egress_default_deny: false,
                ingress_default_deny: false,
                egress_rules: Vec::new(),
                deny_rules: Vec::new(),
                ingress_rules: Vec::new(),
                egress_defaults_seal: None,
                ingress_defaults_seal: None,
            },
            instance_policy: None,
            virtualization: None,
            init: None,
        }
    }

    // ---- spec 12 §4: injected depends_on vars are visible to templated
    // declared env (W5) ----

    #[test]
    fn resolve_plan_envs_sees_injected_depends_on_var() -> Result<()> {
        // Declared env FIRST (templated, consuming the injected var), the
        // depends_on-injected var appended AFTER — the exact ordering
        // `discovery::apply_resolution` produces.
        let plan = empty_plan_with_env(vec![
            EnvVar::literal("OPENAI_BASE_URL", "http://${LITELLM_ADDR}/v1"),
            EnvVar {
                name: "LITELLM_ADDR".to_string(),
                value: "host.microsandbox.internal:4000".to_string(),
                is_secret: false,
                reject_placeholder: None,
                injected_by: Some("litellm".to_string()),
                injected_port: None,
            },
        ]);
        let resolved = resolve_plan_envs(&plan, &secrets_map(&[]))?;
        assert_eq!(
            resolved,
            vec![
                (
                    "OPENAI_BASE_URL".to_string(),
                    "http://host.microsandbox.internal:4000/v1".to_string()
                ),
                (
                    "LITELLM_ADDR".to_string(),
                    "host.microsandbox.internal:4000".to_string()
                ),
            ]
        );
        Ok(())
    }

    #[test]
    fn resolve_plan_envs_falls_back_to_process_env_for_names_not_in_plan() -> Result<()> {
        let unique = "WORKESTRATE_TEST_PLAN_ENV_FALLBACK";
        // SAFETY: unique per-test var name; no concurrent accessor; removed before test end.
        unsafe { std::env::set_var(unique, "process-value") };
        let plan =
            empty_plan_with_env(vec![EnvVar::literal("AD_HOC", &format!("${{{}}}", unique))]);
        let resolved = resolve_plan_envs(&plan, &secrets_map(&[]))?;
        // SAFETY: unique per-test var name; no concurrent accessor; removed before test end.
        unsafe { std::env::remove_var(unique) };
        assert_eq!(
            resolved,
            vec![("AD_HOC".to_string(), "process-value".to_string())]
        );
        Ok(())
    }

    // ---- P0 regression: secret-backed plan env entries resolve against the
    // merged secrets map ONLY, never self-matching their own raw template in
    // the plan env map ----

    #[test]
    fn resolve_plan_envs_self_referential_secret_uses_secrets_map() -> Result<()> {
        // The entry's name matches its own template var: resolving against
        // the plan env map would return the raw literal "${LITELLM_MASTER_KEY}".
        let plan = empty_plan_with_env(vec![EnvVar {
            name: "LITELLM_MASTER_KEY".to_string(),
            value: "${LITELLM_MASTER_KEY}".to_string(),
            is_secret: true,
            reject_placeholder: None,
            injected_by: None,
            injected_port: None,
        }]);
        let secrets = secrets_map(&[("LITELLM_MASTER_KEY", "real-key")]);
        let resolved = resolve_plan_envs(&plan, &secrets)?;
        assert_eq!(
            resolved,
            vec![("LITELLM_MASTER_KEY".to_string(), "real-key".to_string())]
        );
        Ok(())
    }

    #[test]
    fn resolve_plan_envs_remapped_secret_uses_secrets_map() -> Result<()> {
        // The entry's name DIFFERS from its template var: still resolves to
        // the secret value from the secrets map.
        let plan = empty_plan_with_env(vec![EnvVar {
            name: "TEMPEST_LOCAL_API_KEY".to_string(),
            value: "${LITELLM_MASTER_KEY}".to_string(),
            is_secret: true,
            reject_placeholder: None,
            injected_by: None,
            injected_port: None,
        }]);
        let secrets = secrets_map(&[("LITELLM_MASTER_KEY", "real-key")]);
        let resolved = resolve_plan_envs(&plan, &secrets)?;
        assert_eq!(
            resolved,
            vec![("TEMPEST_LOCAL_API_KEY".to_string(), "real-key".to_string())]
        );
        Ok(())
    }

    #[test]
    fn resolve_plan_envs_non_secret_still_uses_injected_plan_var() -> Result<()> {
        // W5 preserved: a non-secret templated entry consumes a
        // depends_on-injected plan var.
        let plan = empty_plan_with_env(vec![
            EnvVar::literal("OPENAI_BASE_URL", "http://${LITELLM_ADDR}/v1"),
            EnvVar {
                name: "LITELLM_ADDR".to_string(),
                value: "127.0.0.1:4000".to_string(),
                is_secret: false,
                reject_placeholder: None,
                injected_by: Some("litellm".to_string()),
                injected_port: None,
            },
        ]);
        let resolved = resolve_plan_envs(&plan, &secrets_map(&[]))?;
        assert_eq!(
            resolved,
            vec![
                (
                    "OPENAI_BASE_URL".to_string(),
                    "http://127.0.0.1:4000/v1".to_string()
                ),
                ("LITELLM_ADDR".to_string(), "127.0.0.1:4000".to_string()),
            ]
        );
        Ok(())
    }

    #[test]
    fn resolve_plan_envs_composition_of_secret_injected_and_templated() -> Result<()> {
        // One call resolves all three shapes correctly: the injected var,
        // the non-secret templated consumer, and the self-referential secret.
        let plan = empty_plan_with_env(vec![
            EnvVar::literal("OPENAI_BASE_URL", "http://${LITELLM_ADDR}/v1"),
            EnvVar {
                name: "LITELLM_ADDR".to_string(),
                value: "127.0.0.1:4000".to_string(),
                is_secret: false,
                reject_placeholder: None,
                injected_by: Some("litellm".to_string()),
                injected_port: None,
            },
            EnvVar {
                name: "LITELLM_MASTER_KEY".to_string(),
                value: "${LITELLM_MASTER_KEY}".to_string(),
                is_secret: true,
                reject_placeholder: None,
                injected_by: None,
                injected_port: None,
            },
        ]);
        let secrets = secrets_map(&[("LITELLM_MASTER_KEY", "real-key")]);
        let resolved = resolve_plan_envs(&plan, &secrets)?;
        assert_eq!(
            resolved,
            vec![
                (
                    "OPENAI_BASE_URL".to_string(),
                    "http://127.0.0.1:4000/v1".to_string()
                ),
                ("LITELLM_ADDR".to_string(), "127.0.0.1:4000".to_string()),
                ("LITELLM_MASTER_KEY".to_string(), "real-key".to_string()),
            ]
        );
        Ok(())
    }

    // ---- host-bound secret substitution: require_tls_identity ----

    #[test]
    fn secret_requires_tls_identity_local_proxy_alias_is_false() {
        assert!(!secret_requires_tls_identity("host.microsandbox.internal"));
        // Case variation: the alias match is case-insensitive.
        assert!(!secret_requires_tls_identity("HOST.MICROSANDBOX.INTERNAL"));
    }

    #[test]
    fn secret_requires_tls_identity_external_host_is_true() {
        assert!(secret_requires_tls_identity("openrouter.ai"));
        assert!(secret_requires_tls_identity("github.com"));
    }

    #[test]
    fn apply_plan_secrets_builds_secret_for_alias_and_external_hosts() -> Result<()> {
        // One secret bound to the local proxy alias (plain-HTTP substitution
        // allowed) and the same secret bound to an external host (keeps
        // require_tls_identity = true).
        let plan = empty_plan_with_secrets(vec![
            HostBoundSecret {
                name: "LITELLM_MASTER_KEY".to_string(),
                value: "${LITELLM_MASTER_KEY}".to_string(),
                allowed_hosts: vec!["host.microsandbox.internal".to_string()],
                required: true,
                reject_placeholder: None,
                on_violation: SecretViolationPolicy::Passthrough,
            },
            HostBoundSecret {
                name: "LITELLM_MASTER_KEY".to_string(),
                value: "${LITELLM_MASTER_KEY}".to_string(),
                allowed_hosts: vec!["openrouter.ai".to_string()],
                required: true,
                reject_placeholder: None,
                on_violation: SecretViolationPolicy::Passthrough,
            },
        ]);
        let secrets = secrets_map(&[("LITELLM_MASTER_KEY", "real")]);
        let builder = apply_plan_secrets(Sandbox::builder("test"), &plan, &secrets)?;
        // LIMITATION: the SDK's built config internals are pub(crate), not
        // inspectable from workestrate, so we cannot assert the per-entry
        // require_tls_identity on the produced config. Instead we assert the
        // pure helper derives the value for both host kinds and that the
        // builder call (which applies the helper per host) succeeds.
        assert!(!secret_requires_tls_identity("host.microsandbox.internal"));
        assert!(secret_requires_tls_identity("openrouter.ai"));
        let _ = builder;
        Ok(())
    }

    /// Every violation policy variant wires through `SecretBuilder::
    /// on_violation` without error. LIMITATION: the SDK's built config
    /// internals are pub(crate), not inspectable from workestrate, so the
    /// assertion is that the wiring call succeeds per policy (the
    /// parse/default tests in workload/secrets.rs pin the value plumbing
    /// from TOML → definition → plan entry).
    #[test]
    fn apply_plan_secrets_wires_every_violation_policy() -> Result<()> {
        for policy in [
            SecretViolationPolicy::Passthrough,
            SecretViolationPolicy::Block,
            SecretViolationPolicy::BlockAndLog,
            SecretViolationPolicy::BlockAndTerminate,
        ] {
            let plan = empty_plan_with_secrets(vec![HostBoundSecret {
                name: "GITHUB_TOKEN".to_string(),
                value: "${GITHUB_TOKEN}".to_string(),
                allowed_hosts: vec!["github.com".to_string()],
                required: true,
                reject_placeholder: None,
                on_violation: policy,
            }]);
            let secrets = secrets_map(&[("GITHUB_TOKEN", "real")]);
            let builder = apply_plan_secrets(Sandbox::builder("test"), &plan, &secrets)?;
            let _ = builder;
        }
        Ok(())
    }

    // ---- detached-`up` parent teardown decision (should_teardown_in_parent) ----
    //
    // `up_service_with_spec` itself cannot run under unit tests (it spawns
    // processes), so the decision-level contract is pinned at the pure seam:
    // only ChainStep::Replace tears down in the parent — Start is a free
    // slot, StartExisting starts the stopped/crashed sandbox in place, and
    // Reuse/Fail return before the seam is consulted (reconcile.rs
    // decide_step docs).

    #[test]
    fn should_teardown_in_parent_true_only_for_replace() {
        use crate::microsandbox::runtime::ChainStep;
        assert!(should_teardown_in_parent(ChainStep::Replace));
        for step in [
            ChainStep::Start,
            ChainStep::StartExisting,
            ChainStep::Reuse,
            ChainStep::Fail,
        ] {
            assert!(
                !should_teardown_in_parent(step),
                "{step:?} must not tear down in the parent"
            );
        }
    }

    // ---- create-with-replace decision (should_create_with_replace) ----
    //
    // The create must carry replace semantics whenever the build DECIDED
    // Replace, not only on the explicit `--replace` flag: a chain-chosen
    // Replace tears down state, but `ensure_mount_sources` can re-create the
    // sandbox dir (self-mounted rw subdirs, e.g.
    // `${MSB_HOME}/sandboxes/<name>/logs`), and the fork's non-replace create
    // gate (`prepare_create_target`) refuses any pre-existing dir with
    // `SandboxAlreadyExists`.

    // ---- ADR 0030 V-addendum §V4 wired: skew_disposition truth table ----

    /// Default-absent policy (None → warn) with BOTH stamps present and
    /// DIFFERENT yields the PINNED divergence notice (instance + 4-char
    /// hash prefixes).
    #[test]
    fn skew_disposition_warns_on_divergence_under_default_policy() {
        let notice = match skew_disposition(
            None,
            "personal-litellm",
            Some("a1b2c3d4e5f60718"),
            Some("d4e5f60718273a4b"),
        ) {
            SkewDisposition::Warn(m) => m,
            other => panic!("warn + differing stamps must notice, got {other:?}"),
        };
        assert_eq!(
            notice,
            "warning: instance 'personal-litellm' was built from config a1b2; \
             current inputs config d4e5 (on_skew = \"warn\": proceeding with reuse)",
            "the notice wording is PINNED (ADR 0032 §Provenance stamps)"
        );
        let notice = match skew_disposition(
            Some(crate::config::OnSkew::Warn),
            "x",
            Some("aaaa"),
            Some("bbbb"),
        ) {
            SkewDisposition::Warn(m) => m,
            other => panic!("explicit warn behaves like the default, got {other:?}"),
        };
        assert!(notice.contains("aaaa") && notice.contains("bbbb"));
    }

    /// replace → ReplaceNow; reuse-silently → silent Proceed.
    #[test]
    fn skew_disposition_replace_and_silent_policies() {
        for policy in [
            crate::config::OnSkew::Replace,
            crate::config::OnSkew::ReuseSilently,
        ] {
            let d = skew_disposition(Some(policy), "x", Some("a"), Some("b"));
            if matches!(policy, crate::config::OnSkew::Replace) {
                assert_eq!(d, SkewDisposition::ReplaceNow, "{policy} must replace");
            } else {
                assert_eq!(
                    d,
                    SkewDisposition::Proceed,
                    "{policy} must proceed silently"
                );
            }
        }
    }

    /// Unknown-version posture: EITHER stamp missing (pre-stamp record or
    /// no current view) → Proceed for EVERY policy — never warns, never
    /// replaces. Equal stamps → Proceed for every policy too.
    #[test]
    fn skew_disposition_missing_or_equal_stamps_never_fire() {
        for policy in [
            None,
            Some(crate::config::OnSkew::Warn),
            Some(crate::config::OnSkew::Replace),
            Some(crate::config::OnSkew::ReuseSilently),
        ] {
            assert_eq!(
                skew_disposition(policy, "x", None, None),
                SkewDisposition::Proceed,
                "both stamps missing: {policy:?}"
            );
            assert_eq!(
                skew_disposition(policy, "x", Some("a"), None),
                SkewDisposition::Proceed,
                "recorded missing: {policy:?}"
            );
            assert_eq!(
                skew_disposition(policy, "x", None, Some("b")),
                SkewDisposition::Proceed,
                "current missing: {policy:?}"
            );
            assert_eq!(
                skew_disposition(policy, "x", Some("same"), Some("same")),
                SkewDisposition::Proceed,
                "equal stamps: {policy:?}"
            );
        }
    }

    // ---- ADR 0032 A3: parent/child current-view equivalence ----

    /// A per-dir-shaped workload whose plan carries THIS workload's state
    /// mount (the only plan dimension the pre-chain mutations touch).
    #[derive(Debug)]
    struct PerDirWorkload {
        strategy: crate::config::InstanceStrategy,
    }

    impl Workload for PerDirWorkload {
        fn name(&self) -> &str {
            "pd"
        }
        fn plan(&self) -> SandboxPlan {
            let mut p = empty_plan_with_env(Vec::new());
            p.mounts = vec![crate::microsandbox::plan::MountPlan {
                host: "workspaces/pd-state".to_string(),
                guest: "/data".to_string(),
                mode: crate::microsandbox::plan::MountMode::Rw,
                policy: None,
                policy_file: None,
            }];
            p.ports = vec![PortMapping::new(4000, 4000)];
            p
        }
        fn exec(&self) -> SandboxCommand {
            SandboxCommand::with_args("", &[])
        }
        fn instance_strategy(&self) -> crate::config::InstanceStrategy {
            self.strategy
        }
    }

    /// THE non-divergence pin: the detached PARENT hashes through
    /// `current_config_hash_for_workload` (fresh plan + mutations) while
    /// the CHILD hashes its already-mutated build_sandbox plan. Both views
    /// must produce the SAME hash — including under per-dir instance-state
    /// scoping — so a skewed reuse cannot be judged differently by the two
    /// sites.
    #[test]
    fn parent_helper_matches_child_mutated_plan_hash() -> Result<()> {
        use crate::microsandbox::plan::MountMode;
        let workload = PerDirWorkload {
            strategy: crate::config::InstanceStrategy::PerDir,
        };
        let instance = "pd@work-1234abcd";

        // Parent view (helper).
        let parent_hash = current_config_hash_for_workload(&workload, instance);

        // Child view: replicate build_sandbox's pre-chain mutations on a
        // fresh plan (scoping keyed on the @-suffix + name override).
        let mut child_plan = workload.plan();
        let key = crate::microsandbox::slots::instance_id_of(instance).expect("per-dir id");
        for m in &mut child_plan.mounts {
            m.host = crate::microsandbox::mounts::instance_scoped_state_path(
                &m.host,
                workload.name(),
                Some(key),
            );
        }
        child_plan.name = instance.to_string();
        // Host-port probing happens after these mutations in build_sandbox;
        // it must NOT affect the comparison (host ports are excluded), so
        // simulate a probe redrawing the host while the guest stays put.
        child_plan.ports[0].host = 54321;

        assert_eq!(
            parent_hash,
            crate::microsandbox::provenance::config_hash_of_plan(&child_plan),
            "parent helper and child mutated-plan views must hash identically"
        );

        // And the scoping itself is visible: a DIFFERENT per-dir key yields
        // a different hash (state mounts differ), while a singleton
        // strategy ignores the key entirely.
        let other = current_config_hash_for_workload(&workload, "pd@work-9999zzzz");
        assert_ne!(other, parent_hash, "per-dir keys scope the state mount");
        let singleton = PerDirWorkload {
            strategy: crate::config::InstanceStrategy::Singleton,
        };
        let s1 = current_config_hash_for_workload(&singleton, "pd");
        let mut expected_singleton = singleton.plan();
        expected_singleton.name = "pd".to_string();
        assert_eq!(
            s1,
            crate::microsandbox::provenance::config_hash_of_plan(&expected_singleton)
        );
        let _ = MountMode::Rw; // keep the import honest when fixtures evolve
        Ok(())
    }

    #[test]
    fn should_create_with_replace_true_for_chain_driven_replace() {
        use crate::microsandbox::runtime::ChainStep;
        // THE regression: a chain-chosen Replace with no explicit flag must
        // still create with replace semantics, or the fork's non-replace
        // create gate refuses the dir `ensure_mount_sources` re-creates.
        assert!(should_create_with_replace(false, ChainStep::Replace));
    }

    #[test]
    fn should_create_with_replace_true_for_explicit_flag_any_step() {
        use crate::microsandbox::runtime::ChainStep;
        for step in [
            ChainStep::Start,
            ChainStep::Reuse,
            ChainStep::StartExisting,
            ChainStep::Replace,
            ChainStep::Fail,
        ] {
            assert!(
                should_create_with_replace(true, step),
                "explicit --replace must force replace semantics for {step:?}"
            );
        }
    }

    #[test]
    fn should_create_with_replace_false_otherwise() {
        use crate::microsandbox::runtime::ChainStep;
        for step in [
            ChainStep::Start,
            ChainStep::StartExisting,
            ChainStep::Reuse,
            ChainStep::Fail,
        ] {
            assert!(
                !should_create_with_replace(false, step),
                "{step:?} without --replace must not create with replace semantics"
            );
        }
    }

    // ---- ADR 0026(a)/C2: slot_bind_ip ----

    #[test]
    fn slot_bind_ip_singleton_uses_shared_localhost() -> Result<()> {
        let state_dir = unique_state_dir("slot-bind-singleton");
        // Singleton slot (no `@`): the shared bind, and the allocator is
        // never consulted (no records needed).
        let ip = slot_bind_ip("personal-litellm", &state_dir)?;
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::LOCALHOST));
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn slot_bind_ip_parallel_allocates_lowest_free_loopback() -> Result<()> {
        let state_dir = unique_state_dir("slot-bind-parallel");
        // Empty registry → 127.0.0.2 (the lowest allocatable N >= 2).
        let ip = slot_bind_ip("personal-litellm@canary", &state_dir)?;
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)));
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn slot_bind_ip_parallel_skips_registered_ips() -> Result<()> {
        let state_dir = unique_state_dir("slot-bind-skip");
        // A registered parallel instance holding 127.0.0.2 …
        super::super::super::port_registry::check_and_register_sandbox_lifecycle(
            &state_dir,
            "personal-litellm@canary",
            Some("personal"),
            "litellm",
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)),
            &[4000],
            &[PortMapping::new(4000, 4000)],
            "2026-07-30T00:00:00Z",
            "default",
            None,
            None,
            None,
            None,
        )?;
        // … makes the next parallel slot draw 127.0.0.3.
        let ip = slot_bind_ip("personal-litellm@blue", &state_dir)?;
        assert_eq!(ip, IpAddr::V4(Ipv4Addr::new(127, 0, 0, 3)));
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- P1: host = 0 per-port auto allocation (namespaced ports) ----

    /// Apply `apply_auto_ports` to `original` and assert every auto-marked
    /// host satisfies the P1 invariants: non-zero, mutually distinct, distinct
    /// from the declared (non-auto) hosts, and bindable on `bind` RIGHT NOW.
    ///
    /// FLAKE-GUARD (TOCTOU): `probe_free_ports` is deliberately NOT a
    /// reservation — under `cargo test`'s parallel harness another thread (or
    /// an unrelated process) can claim a probed port between the probe and
    /// this assertion's bind. Retrying the whole apply+assert cycle a few
    /// times distinguishes that benign race from a real regression (mirrors
    /// store.rs `assert_probed_ports_bindable`).
    fn assert_auto_ports_assign(
        state_dir: &Path,
        bind: IpAddr,
        original: Vec<PortMapping>,
        auto_indices: &[usize],
    ) -> Vec<PortMapping> {
        let declared: Vec<u16> = original
            .iter()
            .enumerate()
            .filter(|(i, _)| !auto_indices.contains(i))
            .map(|(_, p)| p.host)
            .collect();
        const MAX_CYCLES: usize = 8;
        for cycle in 1..=MAX_CYCLES {
            let mut ports = original.clone();
            apply_auto_ports(state_dir, bind, &mut ports).expect("apply_auto_ports must succeed");
            let assigned: Vec<u16> = auto_indices.iter().map(|&i| ports[i].host).collect();
            // Deterministic invariants (a violation is a real regression, not
            // a flake): assert directly, no retry.
            assert!(
                assigned.iter().all(|p| *p != 0),
                "auto ports must be non-zero: {assigned:?}"
            );
            let distinct: std::collections::HashSet<u16> = assigned.iter().copied().collect();
            assert_eq!(
                distinct.len(),
                assigned.len(),
                "auto ports must be mutually distinct: {assigned:?}"
            );
            for p in &assigned {
                assert!(
                    !declared.contains(p),
                    "auto port {p} collides with a declared host {declared:?}"
                );
            }
            // Bindability is racy (the probe is not a reservation): hold every
            // successfully bound listener while attempting the rest, so the
            // assertion is "all assigned ports are SIMULTANEOUSLY bindable".
            let mut held = Vec::with_capacity(assigned.len());
            let mut conflict = None;
            for p in &assigned {
                match std::net::TcpListener::bind((bind, *p)) {
                    Ok(listener) => held.push(listener),
                    Err(e) => {
                        conflict = Some((*p, e));
                        break;
                    }
                }
            }
            drop(held);
            if let Some((port, err)) = conflict {
                assert!(
                    cycle < MAX_CYCLES,
                    "auto-assigned port {port} on {bind} must be bindable after the probe \
                     (failed all {MAX_CYCLES} apply/assert cycles): {err:?}"
                );
                eprintln!(
                    "auto-port cycle {cycle}/{MAX_CYCLES}: auto port {port} on {bind} was \
                     claimed before the assertion bind ({err}); re-applying"
                );
            } else {
                return ports;
            }
        }
        unreachable!("the loop returns on success or asserts on exhaustion");
    }

    #[test]
    fn apply_auto_ports_no_auto_marked_ports_is_noop() -> Result<()> {
        let state_dir = unique_state_dir("auto-noop");
        let mut ports = vec![PortMapping::new(4000, 4000), PortMapping::new(3000, 3000)];
        apply_auto_ports(&state_dir, IpAddr::V4(Ipv4Addr::LOCALHOST), &mut ports)?;
        assert_eq!(ports[0].host, 4000);
        assert_eq!(ports[1].host, 3000);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn apply_auto_ports_assigns_one_auto_port_distinct_from_declared() -> Result<()> {
        let state_dir = unique_state_dir("auto-one");
        let bind = IpAddr::V4(Ipv4Addr::LOCALHOST);
        // One declared host, one host=0 auto port: the auto host must never
        // double-publish the declared 4000.
        let original = vec![PortMapping::new(4000, 4000), PortMapping::new(0, 8080)];
        let ports = assert_auto_ports_assign(&state_dir, bind, original, &[1]);
        assert_ne!(
            ports[1].host, 4000,
            "auto host must not equal the declared host"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn apply_auto_ports_assigns_all_auto_ports_distinct() -> Result<()> {
        let state_dir = unique_state_dir("auto-multi");
        let bind = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let original = vec![
            PortMapping::new(4000, 4000),
            PortMapping::new(0, 8080),
            PortMapping::new(0, 9090),
            PortMapping::new(0, 7070),
        ];
        // The helper asserts every P1 invariant (non-zero, distinct, distinct
        // from declared hosts, bindable right now) for all assigned hosts.
        assert_auto_ports_assign(&state_dir, bind, original, &[1, 2, 3]);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn apply_auto_ports_assigned_hosts_are_recorded_on_registration() -> Result<()> {
        let state_dir = unique_state_dir("auto-record");
        let bind = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let original = vec![PortMapping::new(4000, 4000), PortMapping::new(0, 8080)];
        let ports = assert_auto_ports_assign(&state_dir, bind, original, &[1]);
        // Mirror build_sandbox: the (mutated) plan is the single source of
        // truth for the record's `ports` and `port_pairs`.
        let host_ports: Vec<u16> = ports.iter().map(|p| p.host).collect();
        let port_pairs: Vec<PortMapping> = ports
            .iter()
            .map(|p| PortMapping {
                bind_ip: bind,
                ..p.clone()
            })
            .collect();
        super::super::super::port_registry::check_and_register_sandbox_lifecycle(
            &state_dir,
            "personal-test",
            Some("personal"),
            "test",
            bind,
            &host_ports,
            &port_pairs,
            "2026-08-10T00:00:00Z",
            "default",
            None,
            None,
            None,
            None,
        )?;
        let record = super::super::super::port_registry::find_record(&state_dir, "personal-test")?
            .expect("record must exist after registration");
        assert_eq!(
            record.ports, host_ports,
            "record.ports must carry the assigned hosts"
        );
        let recorded_hosts: Vec<u16> = record.port_pairs.iter().map(|p| p.host).collect();
        assert_eq!(
            recorded_hosts, host_ports,
            "record.port_pairs hosts must carry the assigned hosts"
        );
        assert!(
            record.ports.contains(&ports[1].host),
            "the auto-assigned host must be recorded (not 0)"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- ADR 0030 Phase 3: pure preferred-port selection (closure-based
    // occupied sets — NO real binds) ----

    use crate::config::types::{IncrementSpec, ParameterizedIncrement};
    use crate::config::{PortOccupiedBare, PortOccupiedStep};

    /// Build an `is_occupied` predicate from a set of occupied ports.
    fn occupied_set(ports: &[u16]) -> impl Fn(u16) -> bool + '_ {
        let set: std::collections::HashSet<u16> = ports.iter().copied().collect();
        move |p| set.contains(&p)
    }

    #[test]
    fn select_preferred_port_preferred_free_chooses_preferred() {
        let chain = vec![PortOccupiedStep::Bare(PortOccupiedBare::Increment)];
        let is_occupied = occupied_set(&[]);
        let result = select_preferred_port(4000, &chain, &is_occupied, &|| None);
        assert_eq!(result, PortSelection::Chosen(4000));
    }

    #[test]
    fn select_preferred_port_preferred_occupied_increments() {
        let chain = vec![PortOccupiedStep::Bare(PortOccupiedBare::Increment)];
        let is_occupied = occupied_set(&[4000]);
        let result = select_preferred_port(4000, &chain, &is_occupied, &|| None);
        assert_eq!(result, PortSelection::Chosen(4001));
    }

    #[test]
    fn select_preferred_port_increment_skips_occupied() {
        let chain = vec![PortOccupiedStep::Bare(PortOccupiedBare::Increment)];
        let is_occupied = occupied_set(&[4000, 4001]);
        let result = select_preferred_port(4000, &chain, &is_occupied, &|| None);
        assert_eq!(result, PortSelection::Chosen(4002));
    }

    #[test]
    fn select_preferred_port_range_scans_in_order() {
        let chain = vec![PortOccupiedStep::Increment(ParameterizedIncrement {
            increment: IncrementSpec {
                limit: None,
                range: Some((5000, 5100)),
            },
        })];
        let is_occupied = occupied_set(&[4000, 5000]);
        let result = select_preferred_port(4000, &chain, &is_occupied, &|| None);
        assert_eq!(result, PortSelection::Chosen(5001));
    }

    #[test]
    fn select_preferred_port_range_disjoint_band() {
        let chain = vec![PortOccupiedStep::Increment(ParameterizedIncrement {
            increment: IncrementSpec {
                limit: None,
                range: Some((5000, 5100)),
            },
        })];
        let is_occupied = occupied_set(&[4000]);
        let result = select_preferred_port(4000, &chain, &is_occupied, &|| None);
        assert_eq!(result, PortSelection::Chosen(5000));
    }

    #[test]
    fn select_preferred_port_range_exhausted_then_auto() {
        let chain = vec![
            PortOccupiedStep::Increment(ParameterizedIncrement {
                increment: IncrementSpec {
                    limit: None,
                    range: Some((5000, 5001)),
                },
            }),
            PortOccupiedStep::Bare(PortOccupiedBare::Auto),
        ];
        let is_occupied = occupied_set(&[4000, 5000, 5001]);
        let result = select_preferred_port(4000, &chain, &is_occupied, &|| Some(7000));
        assert_eq!(result, PortSelection::Chosen(7000));
    }

    #[test]
    fn select_preferred_port_chain_exhausted_lists_attempts() {
        let chain = vec![PortOccupiedStep::Bare(PortOccupiedBare::Increment)];
        // Occupy preferred AND the entire bare band (preferred+1 .. preferred+100).
        let occupied: Vec<u16> = (4000..=4100).collect();
        let is_occupied = occupied_set(&occupied);
        let result = select_preferred_port(4000, &chain, &is_occupied, &|| None);
        match result {
            PortSelection::Exhausted(msg) => {
                assert!(
                    msg.contains("increment (band exhausted)"),
                    "message must list the increment attempt: {msg}"
                );
            }
            other => panic!("expected Exhausted, got {other:?}"),
        }
    }

    #[test]
    fn select_preferred_port_fail_terminal() {
        let chain = vec![PortOccupiedStep::Bare(PortOccupiedBare::Fail)];
        let is_occupied = occupied_set(&[4000]);
        let result = select_preferred_port(4000, &chain, &is_occupied, &|| Some(7000));
        match result {
            PortSelection::Exhausted(msg) => {
                assert!(
                    msg.contains("fail"),
                    "message must list the fail attempt: {msg}"
                );
            }
            other => panic!("expected Exhausted, got {other:?}"),
        }
    }

    #[test]
    fn select_preferred_port_limit_form() {
        let chain = vec![PortOccupiedStep::Increment(ParameterizedIncrement {
            increment: IncrementSpec {
                limit: Some(3),
                range: None,
            },
        })];
        // Occupy preferred AND preferred+1 .. preferred+3 (the whole limit band).
        let occupied: Vec<u16> = (4000..=4003).collect();
        let is_occupied = occupied_set(&occupied);
        let result = select_preferred_port(4000, &chain, &is_occupied, &|| None);
        match result {
            PortSelection::Exhausted(msg) => {
                assert!(
                    msg.contains("increment (band exhausted)"),
                    "message must list the increment attempt: {msg}"
                );
            }
            other => panic!("expected Exhausted, got {other:?}"),
        }
    }

    #[test]
    fn select_preferred_port_auto_direct() {
        let chain = vec![PortOccupiedStep::Bare(PortOccupiedBare::Auto)];
        let is_occupied = occupied_set(&[4000]);
        let result = select_preferred_port(4000, &chain, &is_occupied, &|| Some(7000));
        assert_eq!(result, PortSelection::Chosen(7000));
    }

    // ---- ADR 0030 Phase 3: apply_instance_port_policy (Auto) ----

    /// A minimal Workload whose `instance_port()` returns the given policy.
    #[derive(Debug)]
    struct PolicyWorkload {
        port: Option<crate::config::InstancePort>,
    }

    impl Workload for PolicyWorkload {
        fn name(&self) -> &str {
            "policy-test"
        }
        fn plan(&self) -> SandboxPlan {
            empty_plan_with_env(Vec::new())
        }
        fn exec(&self) -> SandboxCommand {
            SandboxCommand::with_args("", &[])
        }
        fn instance_port(&self) -> Option<crate::config::InstancePort> {
            self.port.clone()
        }
    }

    #[test]
    fn apply_instance_port_policy_auto_sets_all_hosts_to_auto() -> Result<()> {
        let state_dir = unique_state_dir("policy-auto");
        let bind = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let workload = PolicyWorkload {
            port: Some(crate::config::InstancePort::Auto),
        };
        let original = vec![
            PortMapping::new(4000, 4000),
            PortMapping::new(3000, 3000),
            PortMapping::new(0, 8080),
        ];
        let mut ports = original.clone();
        apply_instance_port_policy(&state_dir, &workload, bind, &mut ports, None)?;
        // Every declared port must now be a probed non-zero host.
        assert!(
            ports.iter().all(|p| p.host != 0),
            "all hosts must be auto-assigned (non-zero): {:?}",
            ports.iter().map(|p| p.host).collect::<Vec<_>>()
        );
        // Distinct from each other and bindable right now (the
        // assert_auto_ports_assign invariants).
        let assigned: Vec<u16> = ports.iter().map(|p| p.host).collect();
        let distinct: std::collections::HashSet<u16> = assigned.iter().copied().collect();
        assert_eq!(
            distinct.len(),
            assigned.len(),
            "auto hosts must be mutually distinct: {assigned:?}"
        );
        let mut held = Vec::new();
        for p in &assigned {
            held.push(std::net::TcpListener::bind((bind, *p))?);
        }
        drop(held);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- ADR 0030 Phase 3: apply_instance_port_policy (Preferred) — the
    // REAL occupied predicate (registry + OS bind + plan-declared hosts) ----

    /// Regression: the PRIMARY port's own declared host must not self-occupy
    /// the preferred port. The canonical config shape declares
    /// `host == preferred` (litellm: `host = 4000` +
    /// `port = { preferred = 4000 }`); counting the primary in the
    /// plan-declared check made `Preferred(n)` permanently skip n and walk
    /// the on_occupied chain on every up. With a free preferred port the
    /// selection must land on it.
    #[test]
    fn apply_instance_port_policy_preferred_free_selects_preferred() -> Result<()> {
        let state_dir = unique_state_dir("policy-preferred-free");
        let bind = IpAddr::V4(Ipv4Addr::LOCALHOST);
        // Probe-and-release a free port so the OS-bind occupancy check passes.
        let preferred = std::net::TcpListener::bind((bind, 0))?.local_addr()?.port();
        let workload = PolicyWorkload {
            port: Some(crate::config::InstancePort::Preferred(
                crate::config::types::PreferredPort {
                    preferred,
                    on_occupied: None,
                },
            )),
        };
        // The canonical shape: the plan declares host == preferred.
        let mut ports = vec![PortMapping::new(preferred, 4000)];
        apply_instance_port_policy(&state_dir, &workload, bind, &mut ports, None)?;
        assert_eq!(
            ports[0].host, preferred,
            "a free preferred port must be selected even when the plan declares host == preferred"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// With the preferred port OS-occupied (a live listener held for the
    /// duration), the increment chain must walk — and must SKIP the plan's
    /// SIBLING declared host (a multi-port workload's other port), landing on
    /// the next free candidate. The sibling itself is left untouched.
    #[test]
    fn apply_instance_port_policy_preferred_occupied_increments_skipping_siblings() -> Result<()> {
        let state_dir = unique_state_dir("policy-preferred-occupied");
        let bind = IpAddr::V4(Ipv4Addr::LOCALHOST);
        // Probe-and-release to pick a preferred whose +1/+2 are very likely
        // free, then HOLD preferred so the OS-bind probe judges it occupied.
        let preferred = std::net::TcpListener::bind((bind, 0))?.local_addr()?.port();
        let _held = std::net::TcpListener::bind((bind, preferred))?;
        let workload = PolicyWorkload {
            port: Some(crate::config::InstancePort::Preferred(
                crate::config::types::PreferredPort {
                    preferred,
                    on_occupied: None,
                },
            )),
        };
        let sibling = preferred.saturating_add(1);
        let expected = preferred.saturating_add(2);
        let mut ports = vec![
            PortMapping::new(preferred, 4000),
            PortMapping::new(sibling, 4001),
        ];
        apply_instance_port_policy(&state_dir, &workload, bind, &mut ports, None)?;
        assert_eq!(
            ports[0].host, expected,
            "increment must skip the sibling declared host {sibling} and land on {expected}"
        );
        assert_eq!(
            ports[1].host, sibling,
            "the policy governs only the primary port; siblings stay as declared"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- --replace target exclusion: the instance being replaced never
    // blocks its OWN preferred port (selection runs BEFORE teardown) ----

    fn register(dir: &std::path::Path, instance: &str, ports: &[u16]) -> Result<()> {
        // A1: keep (instance, workload, context) consistent — the fixtures
        // below use `<ctx>-<wl>` instance names, so register under
        // Some(<ctx>) with workload `<wl>`.
        if let Some(wl) = instance.strip_prefix("personal-") {
            crate::microsandbox::port_registry::register_sandbox(
                dir,
                instance,
                Some("personal"),
                wl,
                ports,
            )
        } else {
            crate::microsandbox::port_registry::register_sandbox(
                dir, instance, None, instance, ports,
            )
        }
    }

    /// Regression: with --replace in play, the preferred port occupied ONLY
    /// by the replaced instance's OWN registry record AND its live OS
    /// listener (the predecessor still runs at selection time — teardown is
    /// later) must be RECLAIMED, not incremented. Successive
    /// `up litellm --replace` cycles hold 4000 steady instead of walking
    /// 4000 → 4001 → 4002.
    #[test]
    fn apply_instance_port_policy_replace_reclaims_own_preferred() -> Result<()> {
        let state_dir = unique_state_dir("policy-replace-reclaim");
        let bind = IpAddr::V4(Ipv4Addr::LOCALHOST);
        // Probe-and-release a free preferred port, then stand in for the
        // LIVE PREDECESSOR: its registry record plus its held OS listener.
        let preferred = std::net::TcpListener::bind((bind, 0))?.local_addr()?.port();
        let _predecessor_listener = std::net::TcpListener::bind((bind, preferred))?;
        register(&state_dir, "personal-litellm", &[preferred])?;
        let workload = PolicyWorkload {
            port: Some(crate::config::InstancePort::Preferred(
                crate::config::types::PreferredPort {
                    preferred,
                    on_occupied: None,
                },
            )),
        };
        let mut ports = vec![PortMapping::new(preferred, 4000)];
        apply_instance_port_policy(
            &state_dir,
            &workload,
            bind,
            &mut ports,
            Some("personal-litellm"),
        )?;
        assert_eq!(
            ports[0].host, preferred,
            "the replace target's own record+listener must not block its preferred port"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// Fail-closed: with --replace in play, a preferred port held by a
    /// FOREIGN instance's registry record (+ its live listener) still blocks
    /// — the exclusion covers only the replace target's own holdings, so the
    /// increment chain walks.
    #[test]
    fn apply_instance_port_policy_replace_increments_past_foreign_holder() -> Result<()> {
        let state_dir = unique_state_dir("policy-replace-foreign");
        let bind = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let preferred = std::net::TcpListener::bind((bind, 0))?.local_addr()?.port();
        let _foreign_listener = std::net::TcpListener::bind((bind, preferred))?;
        register(&state_dir, "personal-other", &[preferred])?;
        let workload = PolicyWorkload {
            port: Some(crate::config::InstancePort::Preferred(
                crate::config::types::PreferredPort {
                    preferred,
                    on_occupied: None,
                },
            )),
        };
        let expected = preferred.saturating_add(1);
        let mut ports = vec![PortMapping::new(preferred, 4000)];
        apply_instance_port_policy(
            &state_dir,
            &workload,
            bind,
            &mut ports,
            Some("personal-litellm"),
        )?;
        assert_eq!(
            ports[0].host, expected,
            "a foreign holder still blocks the preferred port — increment to {expected}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// Fail-closed on the OS probe: with --replace in play, a live listener
    /// with NO registry record at all (a process outside the registry) still
    /// blocks the preferred port — the exclusion bypasses the OS-bind probe
    /// ONLY when the candidate is registered to the replace target itself.
    #[test]
    fn apply_instance_port_policy_replace_foreign_listener_without_record_increments() -> Result<()>
    {
        let state_dir = unique_state_dir("policy-replace-foreign-listener");
        let bind = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let preferred = std::net::TcpListener::bind((bind, 0))?.local_addr()?.port();
        let _foreign_listener = std::net::TcpListener::bind((bind, preferred))?;
        let workload = PolicyWorkload {
            port: Some(crate::config::InstancePort::Preferred(
                crate::config::types::PreferredPort {
                    preferred,
                    on_occupied: None,
                },
            )),
        };
        let expected = preferred.saturating_add(1);
        let mut ports = vec![PortMapping::new(preferred, 4000)];
        apply_instance_port_policy(
            &state_dir,
            &workload,
            bind,
            &mut ports,
            Some("personal-litellm"),
        )?;
        assert_eq!(
            ports[0].host, expected,
            "a record-less foreign listener still blocks — fail-closed, increment to {expected}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// Ordering: a STALE record for the replaced instance (msb reports Gone)
    /// is pruned by `prune_stale_records` BEFORE selection (build_sandbox:
    /// prune first, then apply_instance_port_policy), so the preferred port
    /// is reclaimed via the PRUNE path — the replace exclusion is not even
    /// needed (exclude_instance is None here). The unregister below is
    /// exactly what `prune_by_verdicts` does for a Gone verdict (covered by
    /// reconcile.rs's prune tests).
    #[test]
    fn apply_instance_port_policy_pruned_predecessor_record_reclaims_preferred() -> Result<()> {
        let state_dir = unique_state_dir("policy-prune-reclaim");
        let bind = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let preferred = std::net::TcpListener::bind((bind, 0))?.local_addr()?.port();
        register(&state_dir, "personal-litellm", &[preferred])?;
        // prune_stale_records' Gone verdict unregisters the stale record
        // BEFORE selection runs (no listener: the sandbox is gone).
        crate::microsandbox::port_registry::unregister_sandbox(&state_dir, "personal-litellm")?;
        let workload = PolicyWorkload {
            port: Some(crate::config::InstancePort::Preferred(
                crate::config::types::PreferredPort {
                    preferred,
                    on_occupied: None,
                },
            )),
        };
        let mut ports = vec![PortMapping::new(preferred, 4000)];
        apply_instance_port_policy(&state_dir, &workload, bind, &mut ports, None)?;
        assert_eq!(
            ports[0].host, preferred,
            "a pruned (Gone) predecessor record frees the preferred port before selection"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }
}
