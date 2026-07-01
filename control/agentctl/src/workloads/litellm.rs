use crate::microsandbox::plan::*;
use crate::microsandbox::secrets;
use crate::microsandbox::workload::{SandboxCommand, Workload};

#[derive(Debug)]
pub struct Litellm;

impl Workload for Litellm {
    fn name(&self) -> &str {
        "litellm"
    }

    fn plan(&self) -> SandboxPlan {
        SandboxPlan {
            name: self.name().into(),
            image: Some("ghcr.io/berriai/litellm:v1.89.4".into()),
            workdir: Some("/app".into()),
            command: {
                let cmd = self.exec();
                std::iter::once(cmd.binary).chain(cmd.arguments).collect()
            },
            cpus: Some(2),
            memory_mib: Some(2048),
            env: vec![
                EnvVar::literal("PORT", "4000"),
                EnvVar::literal("LITELLM_LOCAL_MODEL_COST_MAP", "True"),
                EnvVar::secret(&secrets::LITELLM_MASTER_KEY),
            ],
            secret_env: vec![
                HostBoundSecret::from(&secrets::OPENROUTER),
                HostBoundSecret::from(&secrets::KIMI),
                HostBoundSecret::from(&secrets::NEURALWATT),
                HostBoundSecret::from(&secrets::MINIMAX),
            ],
            ports: vec![PortMapping::same(4000)],
            mounts: vec![
                MountPlan::readwrite("${MSB_HOME}/sandboxes/litellm/logs", "/var/log/litellm"),
                MountPlan::readonly("infra/litellm", "/app/config"),
            ],
            network: NetworkPlan {
                default_deny: true,
                egress_rules: {
                    let mut rules = EgressRule::dns();
                    rules.push(EgressRule::https(&[
                        "openrouter.ai",
                        "api.kimi.com",
                        "api.neuralwatt.com",
                        "api.minimax.io",
                    ]));
                    rules
                },
                deny_rules: vec![],
                ingress_rules: vec![IngressRule::local_tcp(4000)],
            },
        }
    }

    fn exec(&self) -> SandboxCommand {
        SandboxCommand::with_args(
            "/app/.venv/bin/litellm",
            &["--config", "/app/config/config.yaml", "--host", "0.0.0.0"],
        )
    }

    fn detach_args(&self) -> Vec<String> {
        vec![self.name().into(), "up".into()]
    }

    // entrypoint defaults to Shell, no override needed
}
