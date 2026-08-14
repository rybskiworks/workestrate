use super::super::env::{resolve_templated_value_with, resolve_templated_value_with_env_fallback};
use super::super::mounts::{apply_plan_mounts, ensure_mount_sources};
use super::super::plan::{PortMapping, SandboxPlan};
use super::super::workload::{EntrypointSpec, SandboxCommand, Workload};
use super::{check_occupied_or_replace, ForegroundConfig, InstanceSpec};
use anyhow::Result;
use microsandbox::sandbox::{exec::ExecEvent, SandboxBuilder};
use microsandbox::Sandbox;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::path::Path;

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

/// Prepare, resolve, and create the sandbox plus the foreground config used
/// to run the workload's real command.
pub(crate) async fn build_sandbox<W: Workload>(
    workload: &W,
    spec: &InstanceSpec,
) -> Result<(Sandbox, ForegroundConfig)> {
    // Load secrets from .env.enc across the resolved layers. FN-9: the
    // merged map is threaded into env/secret resolution below — it is NOT
    // written into process-global env (parallel build_sandbox calls would
    // race on shared keys). Only called for exec/up paths — plan/check
    // never reach here.
    let secrets = crate::microsandbox::secrets_loader::load_secrets()?;

    let mut plan = workload.plan();

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
    workload.prepare(&env_view)?;

    // Hoist state_dir before the occupancy check so it can be reused for
    // collision detection and lifecycle registration below.
    let state_dir = crate::config::resolve_state_dir();
    for m in &mut plan.mounts {
        if let Some(program) = workload.mount_policy_for(&m.guest) {
            let slug = crate::microsandbox::policy_file::mount_slug(&m.guest);
            let path = crate::microsandbox::policy_file::write_policy_file(
                &state_dir,
                &spec.instance,
                &slug,
                program,
            )?;
            m.policy_file = Some(path);
        }
    }

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

    check_occupied_or_replace(spec, &state_dir).await?;

    ensure_mount_sources(&mount_roots, &plan)?;

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

    let builder = if spec.replace {
        builder.replace()
    } else {
        builder
    };
    let sandbox = builder.create().await?;

    // Host ports from the (possibly --port-auto-mutated) plan: single source
    // of truth for the collision check and the legacy `ports` field in the
    // lifecycle state record, so the record always carries the EFFECTIVE
    // (probed) ports — `ps`/`down` recover them.
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
    let created_at = super::time::current_rfc3339_utc();
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
    )?;
    let config = ForegroundConfig {
        sandbox_name: sandbox.name().to_string(),
        service_label: workload.name().to_string(),
        command: workload.exec(),
        log_stop_errors: workload.log_stop_errors(),
    };
    Ok((sandbox, config))
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

pub async fn exec_agent_with_spec<W: Workload>(workload: &W, spec: &InstanceSpec) -> Result<()> {
    let (sandbox, config) = build_sandbox(workload, spec).await?;
    run_service_interactive(&sandbox, config).await
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
    use crate::config::test_support::unique_state_dir;
    use crate::microsandbox::plan::{EnvVar, HostBoundSecret, NetworkPlan};

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
            ports: Vec::new(),
            mounts: Vec::new(),
            network: NetworkPlan {
                default_deny: false,
                egress_rules: Vec::new(),
                deny_rules: Vec::new(),
                ingress_rules: Vec::new(),
            },
        }
    }

    fn secrets_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
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
            ports: Vec::new(),
            mounts: Vec::new(),
            network: NetworkPlan {
                default_deny: false,
                egress_rules: Vec::new(),
                deny_rules: Vec::new(),
                ingress_rules: Vec::new(),
            },
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
        std::env::set_var(unique, "process-value");
        let plan =
            empty_plan_with_env(vec![EnvVar::literal("AD_HOC", &format!("${{{}}}", unique))]);
        let resolved = resolve_plan_envs(&plan, &secrets_map(&[]))?;
        std::env::remove_var(unique);
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
            },
            HostBoundSecret {
                name: "LITELLM_MASTER_KEY".to_string(),
                value: "${LITELLM_MASTER_KEY}".to_string(),
                allowed_hosts: vec!["openrouter.ai".to_string()],
                required: true,
                reject_placeholder: None,
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
            None,
            "litellm",
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)),
            &[4000],
            &[PortMapping::new(4000, 4000)],
            "2026-07-30T00:00:00Z",
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
            None,
            "test",
            bind,
            &host_ports,
            &port_pairs,
            "2026-08-10T00:00:00Z",
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
}
