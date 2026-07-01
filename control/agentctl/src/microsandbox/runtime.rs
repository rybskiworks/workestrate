use super::env::resolve_templated_value;
use super::mounts::{apply_plan_mounts, ensure_mount_sources};
use super::plan::{EgressTarget, NetworkPlan, Protocol, SandboxPlan, Scope};
use super::workload::{EntrypointSpec, ExecMode, SandboxCommand, Workload};
use anyhow::Result;
use microsandbox::sandbox::{exec::ExecEvent, SandboxBuilder, SandboxHandle, SandboxStatus};
use microsandbox::{MicrosandboxError, NetworkPolicy, Sandbox};
use std::path::PathBuf;

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
        .open(log_dir.join("agentctl.log"))?;
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
                         Set a real value via with-secrets or setup-secrets update.",
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
                     Set it in the environment or run via with-secrets",
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

pub(crate) async fn run_service_foreground(
    sandbox: &Sandbox,
    config: ForegroundConfig,
) -> Result<()> {
    let mut exec_handle = sandbox
        .exec_stream(&config.command.binary, config.command.arguments)
        .await
        .map_err(|e| anyhow::anyhow!("failed to start {} process: {}", &config.service_label, e))?;

    match exec_handle.recv().await {
        Some(ExecEvent::Started { pid }) => {
            eprintln!(
                "{} process started (guest PID {})",
                &config.service_label, pid
            );
        }
        Some(ExecEvent::Failed(err)) => {
            if let Err(stop_err) = sandbox.stop().await {
                eprintln!("failed to stop sandbox after service failure: {}", stop_err);
            }
            return Err(anyhow::anyhow!(
                "{} process failed to start: {:?}",
                &config.service_label,
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
                &config.service_label,
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

    println!(
        "Sandbox '{}' started (Ctrl-C to stop)",
        &config.sandbox_name
    );
    if let Err(e) = tokio::signal::ctrl_c().await {
        // Even if the signal handler fails, attempt to stop the sandbox
        // before propagating the error.
        let _ = sandbox.stop().await;
        let _ = drain_handle.await;
        return Err(anyhow::anyhow!("signal handler error: {}", e));
    }
    if config.log_stop_errors {
        if let Err(e) = sandbox.stop().await {
            eprintln!("failed to stop sandbox '{}': {}", &config.sandbox_name, e);
        }
    } else {
        let _ = sandbox.stop().await;
    }

    // Drain any remaining buffered log events before returning.
    // Use a timeout so a hung exec channel doesn't block forever.
    let _ = tokio::time::timeout(std::time::Duration::from_secs(2), drain_handle).await;

    println!("Sandbox '{}' stopped", &config.sandbox_name);
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
        &sandbox_name
    );

    let attach_result = sandbox
        .attach_with(binary.as_str(), |a| a.args(arguments))
        .await;

    let outcome = match attach_result {
        Ok(code) => {
            eprintln!("{} exited with code {}", &service_label, code);
            Ok(())
        }
        Err(e) => Err(anyhow::anyhow!(
            "failed to attach to {} process: {}",
            &service_label,
            e
        )),
    };

    if log_stop_errors {
        if let Err(e) = sandbox.stop().await {
            eprintln!("failed to stop sandbox '{}': {}", &sandbox_name, e);
        }
    } else {
        let _ = sandbox.stop().await;
    }

    println!("Sandbox '{}' stopped", &sandbox_name);
    outcome
}

/// Generic lifecycle: start any workload's sandbox.
pub async fn up(workload: &dyn Workload, background: bool) -> Result<()> {
    if background {
        let child = spawn_detached_service(workload.name(), &workload.detach_args())?;
        println!(
            "Sandbox '{}' started in background (PID {}). Logs: ~/.microsandbox/sandboxes/{}/agentctl.log",
            workload.name(), child.id(), workload.name()
        );
        return Ok(());
    }

    workload.prepare()?;

    let root = crate::config::project_root()?;
    let plan = workload.plan();
    ensure_mount_sources(&root, &plan)?;

    let policy = network_plan_to_policy(&plan.network)?;

    let mut builder = Sandbox::builder(&plan.name)
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
        builder = builder.port(port.host, port.guest);
    }

    builder = apply_plan_envs(builder, &plan)?;
    builder = apply_plan_mounts(builder, &root, &plan)?;
    builder = apply_plan_secrets(builder, &plan)?;

    let sandbox = builder.replace().create().await?;
    let config = ForegroundConfig {
        sandbox_name: sandbox.name().to_string(),
        service_label: workload.name().to_string(),
        command: workload.exec(),
        log_stop_errors: workload.log_stop_errors(),
    };
    match workload.exec_mode() {
        ExecMode::Headless => run_service_foreground(&sandbox, config).await,
        ExecMode::Interactive => run_service_interactive(&sandbox, config).await,
    }
}

/// Generic lifecycle: stop and remove any sandbox by name.
pub async fn down(name: &str) -> Result<()> {
    match Sandbox::get(name).await {
        Ok(handle) => {
            stop_and_remove(handle).await?;
            println!("Sandbox '{}' stopped and removed", name);
            Ok(())
        }
        Err(MicrosandboxError::SandboxNotFound(_)) => {
            println!("Sandbox '{}' not found", name);
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::super::plan::{EgressTarget, Protocol};
    use super::network_plan_to_policy;
    use crate::microsandbox::workload::Workload;
    use crate::workloads::{Litellm, Odysseus, Pi};

    #[test]
    fn litellm_network_plan_converts_without_error() {
        let plan = Litellm.plan();
        let result = network_plan_to_policy(&plan.network);
        assert!(
            result.is_ok(),
            "litellm conversion failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn pi_network_plan_converts_without_error() {
        let plan = Pi.plan();
        let result = network_plan_to_policy(&plan.network);
        assert!(result.is_ok(), "pi conversion failed: {:?}", result.err());
    }

    #[test]
    fn odysseus_network_plan_converts_without_error() {
        let plan = Odysseus.plan();
        let result = network_plan_to_policy(&plan.network);
        assert!(
            result.is_ok(),
            "odysseus conversion failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn odysseus_plan_includes_admin_password_secret() {
        let plan = Odysseus.plan();
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
    }

    #[test]
    fn odysseus_plan_has_expected_data_mount() {
        let plan = Odysseus.plan();
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
    }

    #[test]
    fn odysseus_plan_uses_data_dir_env_and_drops_hardcoded_db_url() {
        let plan = Odysseus.plan();
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
    }

    #[test]
    fn opencode_network_plan_converts_without_error() {
        let plan = crate::workloads::Opencode.plan();
        let result = network_plan_to_policy(&plan.network);
        assert!(
            result.is_ok(),
            "opencode conversion failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn pi_plan_has_expected_egress() {
        let plan = Pi.plan();
        assert!(plan.network.default_deny);
        assert_eq!(plan.network.egress_rules.len(), 4);
        assert_eq!(plan.network.egress_rules[0].protocol, Protocol::Tcp);
        assert_eq!(plan.network.egress_rules[0].port, 53);
        assert_eq!(plan.network.egress_rules[0].target, EgressTarget::Host);
        let github_rule = plan
            .network
            .egress_rules
            .iter()
            .find(|r| r.port == 443)
            .expect("expected port 443 egress rule");
        assert_eq!(github_rule.protocol, Protocol::Tcp);
        match &github_rule.target {
            EgressTarget::Domains(hosts) => {
                assert!(hosts.contains(&"github.com".to_string()));
                assert!(hosts.contains(&"api.github.com".to_string()));
            }
            EgressTarget::Host => panic!("expected domains, got host"),
        }
    }

    #[test]
    fn pi_plan_redirects_config_to_data_dir() {
        let plan = Pi.plan();
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
    }

    #[test]
    fn pi_plan_exposes_litellm_master_key_as_env_not_host_bound() {
        let plan = Pi.plan();
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
    }

    #[test]
    fn pi_plan_has_expected_mounts() {
        let plan = Pi.plan();
        assert_eq!(plan.mounts.len(), 3, "pi should have 3 mounts");
        let app_mount = plan.mounts.iter().find(|m| m.guest == "/app");
        assert!(app_mount.is_some(), "pi must have a /app mount (Pi code)");
        if let Some(m) = app_mount {
            assert_eq!(m.host, "agents/pi/build");
            assert!(m.read_only, "/app mount must be readonly");
        }
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
    }
}
