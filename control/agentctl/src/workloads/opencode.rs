use crate::microsandbox::plan::*;
use crate::microsandbox::secrets;
use crate::microsandbox::workload::{ExecMode, SandboxCommand, Workload};

#[derive(Debug)]
pub struct Opencode;

impl Workload for Opencode {
    fn name(&self) -> &str {
        "opencode"
    }

    fn plan(&self) -> SandboxPlan {
        SandboxPlan {
            name: self.name().into(),
            image: Some("node:24-bookworm-slim".into()),
            workdir: Some("/home/node".into()),
            command: {
                let cmd = self.exec();
                std::iter::once(cmd.binary).chain(cmd.arguments).collect()
            },
            cpus: Some(2),
            memory_mib: Some(2048),
            env: vec![
                EnvVar::literal(
                    "OPENAI_BASE_URL",
                    "http://host.microsandbox.internal:4000/v1",
                ),
                EnvVar::literal("OPENAI_MODEL", "coding"),
            ],
            secret_env: vec![
                HostBoundSecret::remapped(&secrets::LITELLM_AUTH),
                HostBoundSecret::from(&secrets::GITHUB_TOKEN),
            ],
            ports: vec![PortMapping::same(3000)],
            mounts: vec![
                MountPlan::readonly(self.build_path(), "/app"),
                MountPlan::readwrite("workspaces/opencode", "/workspace"),
                MountPlan::readonly(
                    self.config_path("opencode.jsonc"),
                    "/home/node/.config/opencode/opencode.jsonc",
                ),
                MountPlan::readwrite(
                    "${MSB_HOME}/sandboxes/opencode/state",
                    "/home/node/.local/share/opencode",
                ),
            ],
            network: NetworkPlan {
                default_deny: true,
                egress_rules: EgressRule::agent_base(),
                deny_rules: vec![],
                ingress_rules: vec![IngressRule::local_tcp(3000)],
            },
        }
    }

    fn exec(&self) -> SandboxCommand {
        SandboxCommand::with_args("opencode", &[])
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

    // entrypoint defaults to Shell, no override needed
}
