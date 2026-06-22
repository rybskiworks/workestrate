use super::env::resolve_secret_value;
use super::plan::{NetworkPlan, SandboxPlan};
use anyhow::Result;
use microsandbox::sandbox::{exec::ExecEvent, SandboxBuilder, SandboxHandle, SandboxStatus};
use microsandbox::{NetworkPolicy, Sandbox};
use std::path::PathBuf;

/// Convert a declarative `NetworkPlan` into a Microsandbox SDK `NetworkPolicy`.
///
/// The string `"host"` in `EgressRule.allow_hosts` is a sentinel that maps to
/// the SDK's `allow_host()` method (host bridge networking, including the
/// microsandbox host bridge needed for `host.microsandbox.internal`). All other
/// strings map to `allow_domains()`.
pub(crate) fn network_plan_to_policy(plan: &NetworkPlan) -> Result<NetworkPolicy> {
    let mut builder = NetworkPolicy::builder();

    if plan.default_deny {
        builder = builder.default_deny();
    }

    for rule in &plan.ingress_rules {
        let port = rule.port;
        match (rule.protocol.as_str(), rule.scope.as_str()) {
            ("tcp", "local") => {
                builder = builder.ingress(|i| i.tcp().port(port).allow_local());
            }
            ("tcp", "public") => {
                builder = builder.ingress(|i| i.tcp().port(port).allow_public());
            }
            (proto, scope) => {
                anyhow::bail!("unsupported ingress rule: {proto}/{scope}")
            }
        }
    }

    for rule in &plan.egress_rules {
        let port = rule
            .port
            .ok_or_else(|| anyhow::anyhow!("egress rule missing port"))?;
        let is_host = rule.allow_hosts.iter().all(|h| h == "host");

        match rule.protocol.as_str() {
            "tcp" => {
                if is_host {
                    builder = builder.egress(|e| e.tcp().port(port).allow_host());
                } else {
                    let hosts: Vec<String> = rule.allow_hosts.clone();
                    builder = builder.egress(move |e| {
                        e.tcp()
                            .port(port)
                            .allow_domains(hosts.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                    });
                }
            }
            "udp" => {
                if is_host {
                    builder = builder.egress(|e| e.udp().port(port).allow_host());
                } else {
                    let hosts: Vec<String> = rule.allow_hosts.clone();
                    builder = builder.egress(move |e| {
                        e.udp()
                            .port(port)
                            .allow_domains(hosts.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                    });
                }
            }
            other => anyhow::bail!("unsupported egress protocol: {}", other),
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
        let value = resolve_secret_value(&s.value)?;
        b = b.secret_env(&s.var, value, &s.allowed_host);
    }
    Ok(b)
}

pub(crate) fn apply_plan_envs(
    builder: SandboxBuilder,
    plan: &SandboxPlan,
) -> Result<SandboxBuilder> {
    let mut b = builder;
    for e in &plan.env {
        let value = resolve_secret_value(&e.value)?;
        b = b.env(&e.key, value);
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
pub(crate) async fn run_service_foreground(
    sandbox: &Sandbox,
    sandbox_name: &str,
    service_label: &str,
    exec_program: &str,
    exec_args: Vec<String>,
    log_stop_errors: bool,
) -> Result<()> {
    let mut exec_handle = sandbox
        .exec_stream(exec_program, exec_args)
        .await
        .map_err(|e| anyhow::anyhow!("failed to start {} process: {}", service_label, e))?;

    match exec_handle.recv().await {
        Some(ExecEvent::Started { pid }) => {
            eprintln!("{} process started (guest PID {})", service_label, pid);
        }
        Some(ExecEvent::Failed(err)) => {
            let _ = sandbox.stop().await;
            return Err(anyhow::anyhow!(
                "{} process failed to start: {:?}",
                service_label,
                err
            ));
        }
        other => {
            let _ = sandbox.stop().await;
            return Err(anyhow::anyhow!(
                "unexpected exec event waiting for {} start: {:?}",
                service_label,
                other
            ));
        }
    }

    // The ExecHandle must outlive `sandbox` for the streaming session to stay
    // open, so we move it into a background task. We intentionally do NOT use
    // `Sandbox::detach()` here: the foreground path wants a clean stop on
    // Ctrl-C, not a fire-and-forget background sandbox whose process group
    // outlives this CLI invocation.
    let drain_service = service_label.to_string();
    tokio::spawn(async move {
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

    println!("Sandbox '{}' started (Ctrl-C to stop)", sandbox_name);
    tokio::signal::ctrl_c().await?;
    if log_stop_errors {
        if let Err(e) = sandbox.stop().await {
            eprintln!("failed to stop sandbox '{}': {}", sandbox_name, e);
        }
    } else {
        let _ = sandbox.stop().await;
    }
    println!("Sandbox '{}' stopped", sandbox_name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::plan::{build_litellm_plan, build_odysseus_plan, build_pi_plan};
    use super::network_plan_to_policy;

    #[test]
    fn litellm_network_plan_converts_without_error() {
        let plan = build_litellm_plan();
        let result = network_plan_to_policy(&plan.network);
        assert!(
            result.is_ok(),
            "litellm conversion failed: {:?}",
            result.err()
        );
    }

    #[test]
    fn pi_network_plan_converts_without_error() {
        let plan = build_pi_plan();
        let result = network_plan_to_policy(&plan.network);
        assert!(result.is_ok(), "pi conversion failed: {:?}", result.err());
    }

    #[test]
    fn odysseus_network_plan_converts_without_error() {
        let plan = build_odysseus_plan();
        let result = network_plan_to_policy(&plan.network);
        assert!(
            result.is_ok(),
            "odysseus conversion failed: {:?}",
            result.err()
        );
    }
}
