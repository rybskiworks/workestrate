use crate::microsandbox::plan::*;
use crate::microsandbox::secrets;
use crate::microsandbox::workload::{SandboxCommand, Workload};

#[derive(Debug)]
pub struct Tempest;

impl Workload for Tempest {
    fn name(&self) -> &str {
        "tempest"
    }

    fn plan(&self) -> SandboxPlan {
        // An interactive agent cannot operate without a project directory; if
        // the host cwd is unavailable there is nothing useful to do but fail
        // loudly.
        #[allow(clippy::expect_used)]
        let work_host = std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .expect("failed to determine current working directory for /work mount");

        SandboxPlan {
            name: self.name().into(),
            // nix-built image (dockerTools.buildLayeredImage); provides nodejs_24
            // + nmap + dnsutils + the compiled T3MP3ST tree + the baked
            // defaultProvider:"local" config. Load via: just load-images.
            image: Some("tempest:latest".into()),
            workdir: Some("/work".into()),
            command: {
                let cmd = self.exec();
                std::iter::once(cmd.binary).chain(cmd.arguments).collect()
            },
            cpus: Some(2),
            memory_mib: Some(2048),
            env: vec![
                // LLM backbone — all via env vars, local provider.
                // defaultProvider:"local" is baked into the image config
                // (root/.config/t3mp3st/config.json) so T3MP3ST uses the
                // env-var-driven local provider without a conf-store secret.
                EnvVar::literal(
                    "TEMPEST_LOCAL_BASE_URL",
                    "http://host.microsandbox.internal:4000/v1",
                ),
                EnvVar::literal("TEMPEST_LOCAL_MODEL", "coding"),
                // Remap LITELLM_MASTER_KEY → TEMPEST_LOCAL_API_KEY.
                // The local provider reads the API key from TEMPEST_LOCAL_API_KEY
                // (see agents/tempest/repo/src/config/index.ts getApiKey('local')).
                // There is no env-var remapping helper (only HostBoundSecret::remapped
                // exists, which is host-bound and NOT exposed as a guest env var), so
                // construct the EnvVar directly with the ${LITELLM_MASTER_KEY}
                // placeholder, mirroring how EnvVar::secret builds the value.
                EnvVar {
                    name: "TEMPEST_LOCAL_API_KEY".into(),
                    value: format!("${{{}}}", secrets::LITELLM_MASTER_KEY.env_var),
                    is_secret: true,
                    reject_placeholder: secrets::LITELLM_MASTER_KEY.placeholder.map(|p| p.to_string()),
                },
                // Loopback only — the Express API server (if started) stays
                // inside the microVM and is never exposed to the host.
                EnvVar::literal("T3MP3ST_HOST", "127.0.0.1"),
            ],
            secret_env: vec![],
            ports: vec![],
            mounts: vec![
                MountPlan::readwrite("workspaces/tempest-state", "/data"),
                MountPlan::readwrite(work_host, "/work"),
            ],
            network: NetworkPlan {
                // T3MP3ST is an offensive-security tool that needs to reach
                // arbitrary targets for scanning (nmap, dig, etc.). The current
                // EgressTarget enum only supports Host and Domains — there is no
                // "allow all destinations" variant. Using default_deny: false is
                // the pragmatic approach for an offensive tool: the microVM
                // boundary itself is the containment, and broad egress is the
                // whole point (scanning arbitrary targets). Future: per-mission
                // scope configuration to tighten egress to specific target ranges.
                default_deny: false,
                egress_rules: vec![],
                deny_rules: vec![],
                ingress_rules: vec![],
            },
        }
    }

    fn exec(&self) -> SandboxCommand {
        // `node dist/cli.js` is the interactive CLI TUI (Agent workload, like
        // Pi). The compiled T3MP3ST tree (dist/ + node_modules/ + package.json)
        // is baked into the tempest image, so `node dist/cli.js`
        // resolves imports from the image's working directory.
        SandboxCommand::with_args("node", &["dist/cli.js"])
    }

    fn log_stop_errors(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use crate::microsandbox::workload::Workload;
    use crate::workloads::Tempest;

    #[test]
    fn exec_targets_node_cli() {
        let tempest = Tempest;
        let cmd = tempest.exec();
        assert_eq!(cmd.binary, "node");
        assert_eq!(cmd.arguments, vec!["dist/cli.js"]);
    }
}
