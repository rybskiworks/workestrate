mod env;
mod mounts;
mod plan;
mod runtime;

use std::path::PathBuf;

use anyhow::Result;
use microsandbox::{MicrosandboxError, Sandbox};

pub use plan::{build_litellm_plan, build_odysseus_plan, build_pi_plan};

use env::require_env_vars;
use mounts::{apply_plan_mounts, ensure_mount_sources};
use runtime::{
    apply_plan_envs, apply_plan_secrets, network_plan_to_policy, run_service_foreground,
    spawn_detached_service, stop_and_remove,
};

pub async fn up_litellm(background: bool) -> Result<()> {
    if background {
        let child = spawn_detached_service("litellm", &["litellm".into(), "up".into()])?;
        println!(
            "Sandbox 'litellm' started in background (PID {}). Logs: ~/.microsandbox/sandboxes/litellm/agentctl.log",
            child.id()
        );
        return Ok(());
    }

    require_env_vars(
        "agentctl litellm up",
        &[
            "LITELLM_MASTER_KEY",
            "OPENROUTER_API_KEY",
            "KIMI_CODE_API_KEY",
            "MINIMAX_CODING_API_KEY",
            "INCEPTION_API_KEY",
        ],
    )?;
    let root = crate::config::project_root()?;
    let plan = build_litellm_plan();
    ensure_mount_sources(&root, &plan)?;

    let logs_dir = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| anyhow::anyhow!("HOME not set"))?
        .join(".microsandbox/sandboxes/litellm/logs");
    std::fs::create_dir_all(&logs_dir)?;

    let policy = network_plan_to_policy(&plan.network)?;

    let mut builder = Sandbox::builder(&plan.name)
        .image(plan.image.as_deref().unwrap_or("alpine:latest"))
        .cpus(plan.cpus.unwrap_or(2))
        .memory(plan.memory_mib.unwrap_or(2048))
        .workdir(plan.workdir.as_deref().unwrap_or("/app"))
        .entrypoint(["/bin/sh"])
        .port(4000, 4000)
        .network(|n| n.policy(policy))
        .detached(true);

    builder = apply_plan_envs(builder, &plan)?;
    builder = apply_plan_mounts(builder, &root, &plan);
    builder = apply_plan_secrets(builder, &plan)?;

    let sandbox = builder.replace().create().await?;
    let sandbox_name = sandbox.name().to_string();

    run_service_foreground(
        &sandbox,
        &sandbox_name,
        "litellm",
        "/app/.venv/bin/litellm",
        vec![
            "--config".to_string(),
            "/app/config.yaml".to_string(),
            "--host".to_string(),
            "0.0.0.0".to_string(),
        ],
        true,
    )
    .await
}

pub async fn down_litellm() -> Result<()> {
    match Sandbox::get("litellm").await {
        Ok(handle) => {
            stop_and_remove(handle).await?;
            println!("Sandbox 'litellm' stopped and removed");
            Ok(())
        }
        Err(MicrosandboxError::SandboxNotFound(_)) => {
            println!("Sandbox 'litellm' not found");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn up_pi(background: bool) -> Result<()> {
    if background {
        let child = spawn_detached_service("pi", &["agent".into(), "up".into(), "pi".into()])?;
        println!(
            "Sandbox 'pi' started in background (PID {}). Logs: ~/.microsandbox/sandboxes/pi/agentctl.log",
            child.id()
        );
        return Ok(());
    }

    require_env_vars("agentctl agent up pi", &["LITELLM_MASTER_KEY"])?;
    let root = crate::config::project_root()?;
    let plan = build_pi_plan();
    ensure_mount_sources(&root, &plan)?;

    let policy = network_plan_to_policy(&plan.network)?;

    let mut builder = Sandbox::builder(&plan.name)
        .image(plan.image.as_deref().unwrap_or("alpine:latest"))
        .cpus(plan.cpus.unwrap_or(2))
        .memory(plan.memory_mib.unwrap_or(2048))
        .workdir(plan.workdir.as_deref().unwrap_or("/app"))
        .entrypoint(plan.command.iter().map(String::as_str))
        .network(|n| n.policy(policy))
        .detached(true);

    builder = apply_plan_envs(builder, &plan)?;

    builder = apply_plan_mounts(builder, &root, &plan);
    builder = apply_plan_secrets(builder, &plan)?;

    let sandbox = builder.replace().create().await?;
    let sandbox_name = sandbox.name().to_string();

    run_service_foreground(
        &sandbox,
        &sandbox_name,
        "pi",
        "pi",
        vec!["--mode".to_string(), "rpc".to_string()],
        false,
    )
    .await
}

pub async fn down_pi() -> Result<()> {
    match Sandbox::get("pi").await {
        Ok(handle) => {
            stop_and_remove(handle).await?;
            println!("Sandbox 'pi' stopped and removed");
            Ok(())
        }
        Err(MicrosandboxError::SandboxNotFound(_)) => {
            println!("Sandbox 'pi' not found");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn up_odysseus(background: bool) -> Result<()> {
    if background {
        let child = spawn_detached_service(
            "odysseus",
            &["agent".into(), "up".into(), "odysseus".into()],
        )?;
        println!(
            "Sandbox 'odysseus' started in background (PID {}). Logs: ~/.microsandbox/sandboxes/odysseus/agentctl.log",
            child.id()
        );
        return Ok(());
    }

    let data_dir = std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| anyhow::anyhow!("HOME not set"))?
        .join(".microsandbox/sandboxes/odysseus/data");
    std::fs::create_dir_all(&data_dir)?;

    // Odysseus uses `data/settings.json` for per-provider configuration.
    // For milestone 1 the `${LITELLM_MASTER_KEY}` literal is acceptable
    // because agentctl injects the real key via `OPENAI_API_KEY`. Odysseus
    // may need the actual key injected here depending on its implementation.
    let settings_path = data_dir.join("settings.json");
    std::fs::write(
        &settings_path,
        r#"{
  "providers": {
    "litellm": {
      "base_url": "http://host.microsandbox.internal:4000/v1",
      "api_key": "${LITELLM_MASTER_KEY}",
      "model": "chat"
    }
  }
}
"#,
    )?;

    require_env_vars("agentctl agent up odysseus", &["LITELLM_MASTER_KEY"])?;
    let root = crate::config::project_root()?;
    let plan = build_odysseus_plan();
    ensure_mount_sources(&root, &plan)?;

    let policy = network_plan_to_policy(&plan.network)?;

    let mut builder = Sandbox::builder(&plan.name)
        .image(plan.image.as_deref().unwrap_or("alpine:latest"))
        .cpus(plan.cpus.unwrap_or(2))
        .memory(plan.memory_mib.unwrap_or(2048))
        .workdir(plan.workdir.as_deref().unwrap_or("/app"))
        .entrypoint(plan.command.iter().map(String::as_str))
        .port(7000, 7000)
        .network(|n| n.policy(policy))
        .detached(true);

    builder = apply_plan_envs(builder, &plan)?;

    builder = apply_plan_mounts(builder, &root, &plan);
    builder = apply_plan_secrets(builder, &plan)?;

    let sandbox = builder.replace().create().await?;
    let sandbox_name = sandbox.name().to_string();

    run_service_foreground(
        &sandbox,
        &sandbox_name,
        "odysseus",
        "uvicorn",
        vec![
            "app:app".to_string(),
            "--host".to_string(),
            "0.0.0.0".to_string(),
            "--port".to_string(),
            "7000".to_string(),
        ],
        false,
    )
    .await
}

pub async fn down_odysseus() -> Result<()> {
    match Sandbox::get("odysseus").await {
        Ok(handle) => {
            stop_and_remove(handle).await?;
            println!("Sandbox 'odysseus' stopped and removed");
            Ok(())
        }
        Err(MicrosandboxError::SandboxNotFound(_)) => {
            println!("Sandbox 'odysseus' not found");
            Ok(())
        }
        Err(e) => Err(e.into()),
    }
}
