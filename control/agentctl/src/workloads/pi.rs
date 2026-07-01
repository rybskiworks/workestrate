use crate::microsandbox::plan::*;
use crate::microsandbox::secrets;
use crate::microsandbox::workload::{ExecMode, SandboxCommand, Workload};
use anyhow::Result;
use std::path::Path;

#[derive(Debug)]
pub struct Pi;

impl Workload for Pi {
    fn name(&self) -> &str {
        "pi"
    }

    fn plan(&self) -> SandboxPlan {
        // A coding agent cannot operate without a project directory; if the host
        // cwd is unavailable there is nothing useful to do but fail loudly.
        #[allow(clippy::expect_used)]
        let work_host = std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .expect("failed to determine current working directory for /work mount");

        SandboxPlan {
            name: self.name().into(),
            image: Some("node:24-bookworm-slim".into()),
            workdir: Some("/work".into()),
            command: {
                let cmd = self.exec();
                std::iter::once(cmd.binary).chain(cmd.arguments).collect()
            },
            cpus: Some(2),
            memory_mib: Some(2048),
            env: vec![
                EnvVar::literal("PI_CODING_AGENT_DIR", "/data/agent"),
                EnvVar::literal("PI_TELEMETRY", "0"),
                // LITELLM_MASTER_KEY must be a process env var (not host-bound)
                // so Pi's `${LITELLM_MASTER_KEY}` in models.json resolves to the
                // actual key. Host-bound secrets are injected into the network
                // proxy (TLS interception) but NOT into the guest env, which
                // left the substitution empty and caused "No connected db".
                // Network policy (default-deny + allow only the LiteLLM proxy)
                // remains the authoritative security control; this mirrors
                // litellm.rs, which uses EnvVar::secret for the same key.
                EnvVar::secret(&secrets::LITELLM_MASTER_KEY),
            ],
            secret_env: vec![HostBoundSecret::from(&secrets::GITHUB_TOKEN)],
            ports: vec![],
            mounts: vec![
                MountPlan::readonly(self.build_path(), "/app"),
                MountPlan::readwrite("workspaces/pi-state", "/data"),
                MountPlan::readwrite(work_host, "/work"),
            ],
            network: NetworkPlan {
                default_deny: true,
                egress_rules: EgressRule::agent_base(),
                deny_rules: vec![DenyDomainRule {
                    domain_suffix: ".pi.dev".into(),
                }],
                ingress_rules: vec![],
            },
        }
    }

    fn exec(&self) -> SandboxCommand {
        SandboxCommand::with_args("node", &["/app/packages/coding-agent/dist/cli.js"])
    }

    fn exec_mode(&self) -> ExecMode {
        ExecMode::Interactive
    }

    fn detach_args(&self) -> Vec<String> {
        vec![self.name().into(), "up".into()]
    }

    fn log_stop_errors(&self) -> bool {
        false
    }

    fn prepare(&self) -> Result<()> {
        let root = crate::config::project_root()?;
        seed_models_json(
            &root.join("workspaces/pi-state/agent"),
            &root.join(self.config_path("models.json")),
        )
    }
}

/// Seed `workspaces/pi-state/agent/models.json` from the tracked source
/// (`agents/pi/config/models.json`) only when the target is missing, so Pi
/// picks up the LiteLLM provider config on first start while preserving any
/// runtime edits across restarts. Pi reads this file from
/// `$PI_CODING_AGENT_DIR/models.json` (= `/data/agent/models.json`).
fn seed_models_json(state_dir: &Path, source: &Path) -> Result<()> {
    std::fs::create_dir_all(state_dir)?;
    let target = state_dir.join("models.json");
    if !target.exists() {
        std::fs::copy(source, &target).map_err(|e| {
            anyhow::anyhow!(
                "failed to seed models.json from {} to {}: {}",
                source.display(),
                target.display(),
                e
            )
        })?;
    }
    Ok(())
}
