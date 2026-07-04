use crate::microsandbox::plan::*;
use crate::microsandbox::secrets;
use crate::microsandbox::workload::{SandboxCommand, Workload};
use anyhow::Result;
use std::path::Path;

#[derive(Debug)]
pub struct Odysseus;

impl Workload for Odysseus {
    fn name(&self) -> &str {
        "odysseus"
    }

    fn plan(&self) -> SandboxPlan {
        SandboxPlan {
            name: self.name().into(),
            image: Some("python:3.12-slim".into()),
            workdir: Some("/app".into()),
            command: {
                let cmd = self.exec();
                std::iter::once(cmd.binary).chain(cmd.arguments).collect()
            },
            cpus: Some(2),
            memory_mib: Some(2048),
            env: vec![
                EnvVar::literal("APP_PORT", "7000"),
                EnvVar::literal("AUTH_ENABLED", "true"),
                EnvVar::literal("LOCALHOST_BYPASS", "false"),
                EnvVar::literal("ODYSSEUS_DATA_DIR", "/data"),
                EnvVar::literal(
                    "OPENAI_BASE_URL",
                    "http://host.microsandbox.internal:4000/v1",
                ),
                EnvVar::literal("OPENAI_MODEL", "coding"),
                EnvVar::literal("PYTHONPATH", "/app/.deps"),
                EnvVar::secret(&secrets::ODYSSEUS_ADMIN_PASSWORD),
            ],
            secret_env: vec![
                HostBoundSecret::remapped(&secrets::LITELLM_AUTH),
                HostBoundSecret::from(&secrets::GITHUB_TOKEN),
            ],
            ports: vec![PortMapping::same(7000)],
            mounts: vec![
                MountPlan::readonly(self.build_path(), "/app"),
                MountPlan::readwrite("workspaces/odysseus-state", "/data"),
            ],
            network: NetworkPlan {
                default_deny: true,
                egress_rules: {
                    let mut rules = EgressRule::agent_base();
                    rules.push(EgressRule::https(&[
                        "huggingface.co",
                        "cdn-lfs.huggingface.co",
                        "cdn-lfs-us-1.huggingface.co",
                    ]));
                    rules
                },
                deny_rules: vec![],
                ingress_rules: vec![IngressRule::local_tcp(7000)],
            },
        }
    }

    fn exec(&self) -> SandboxCommand {
        SandboxCommand::with_args(
            "python",
            &[
                "-m", "uvicorn", "app:app", "--host", "0.0.0.0", "--port", "7000",
            ],
        )
    }

    fn log_stop_errors(&self) -> bool {
        false
    }

    fn prepare(&self) -> Result<()> {
        let root = crate::config::project_root()?;
        seed_settings_json(
            &root.join("workspaces/odysseus-state"),
            &root.join(self.config_path("settings.json")),
        )
    }
}

/// Seed `workspaces/odysseus-state/settings.json` from the tracked source
/// (`agents/odysseus/config/settings.json`) only when the target is missing,
/// so runtime setting changes persist across restarts (matching upstream
/// Docker semantics where settings.json lives writable under DATA_DIR).
fn seed_settings_json(state_dir: &Path, source_settings: &Path) -> Result<()> {
    std::fs::create_dir_all(state_dir)?;
    let target = state_dir.join("settings.json");
    if !target.exists() {
        std::fs::copy(source_settings, &target).map_err(|e| {
            anyhow::anyhow!(
                "failed to seed settings.json from {} to {}: {}",
                source_settings.display(),
                target.display(),
                e
            )
        })?;
    }
    Ok(())
}
