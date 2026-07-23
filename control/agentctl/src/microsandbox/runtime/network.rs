use super::super::plan::{EgressTarget, NetworkPlan, Protocol, Scope};
use anyhow::Result;
use microsandbox::NetworkPolicy;

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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::network_plan_to_policy;
    use crate::config::test_support::TestConfigGuard;
    use crate::microsandbox::plan::{EgressTarget, Protocol};
    use crate::microsandbox::workload::{ConfigWorkload, Workload};

    #[test]
    fn litellm_network_plan_converts_without_error() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("litellm")?.plan();
        let result = network_plan_to_policy(&plan.network);
        assert!(
            result.is_ok(),
            "litellm conversion failed: {:?}",
            result.err()
        );
        Ok(())
    }

    #[test]
    fn pi_network_plan_converts_without_error() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("pi")?.plan();
        let result = network_plan_to_policy(&plan.network);
        assert!(result.is_ok(), "pi conversion failed: {:?}", result.err());
        Ok(())
    }

    #[test]
    fn pi_plan_uses_nix_built_image() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        // The pi-bun binary's PT_INTERP points at nix glibc 2.42; the sandbox
        // image must be the nix-built `workestrator-pi:latest` (loaded via
        // `just load-pi-image`), NOT node:24-bookworm-slim (glibc 2.36 → crash).
        let plan = ConfigWorkload::new("pi")?.plan();
        assert_eq!(plan.image.as_deref(), Some("workestrator-pi:latest"));
        Ok(())
    }

    #[test]
    fn odysseus_network_plan_converts_without_error() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("odysseus")?.plan();
        let result = network_plan_to_policy(&plan.network);
        assert!(
            result.is_ok(),
            "odysseus conversion failed: {:?}",
            result.err()
        );
        Ok(())
    }

    #[test]
    fn odysseus_plan_includes_admin_password_secret() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("odysseus")?.plan();
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
        Ok(())
    }

    #[test]
    fn odysseus_plan_has_expected_data_mount() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("odysseus")?.plan();
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
        Ok(())
    }

    #[test]
    fn odysseus_plan_uses_data_dir_env_and_drops_hardcoded_db_url() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("odysseus")?.plan();
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
        Ok(())
    }

    #[test]
    fn opencode_network_plan_converts_without_error() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("opencode")?.plan();
        let result = network_plan_to_policy(&plan.network);
        assert!(
            result.is_ok(),
            "opencode conversion failed: {:?}",
            result.err()
        );
        Ok(())
    }

    #[test]
    fn pi_plan_has_expected_egress() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("pi")?.plan();
        assert!(plan.network.default_deny);
        assert_eq!(plan.network.egress_rules.len(), 4);
        assert_eq!(plan.network.egress_rules[0].protocol, Protocol::Tcp);
        assert_eq!(plan.network.egress_rules[0].port, 53);
        assert_eq!(plan.network.egress_rules[0].target, EgressTarget::Host);
        let github_rules: Vec<_> = plan
            .network
            .egress_rules
            .iter()
            .filter(|r| r.port == 443)
            .collect();
        assert_eq!(
            github_rules.len(),
            1,
            "expected exactly one port 443 egress rule"
        );
        let github_rule = github_rules[0];
        assert!(
            matches!(github_rule.target, EgressTarget::Domains(_)),
            "expected domains, got host"
        );
        if let EgressTarget::Domains(hosts) = &github_rule.target {
            assert!(hosts.contains(&"github.com".to_string()));
            assert!(hosts.contains(&"api.github.com".to_string()));
        }
        Ok(())
    }

    #[test]
    fn pi_plan_redirects_config_to_data_dir() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("pi")?.plan();
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
        Ok(())
    }

    #[test]
    fn pi_plan_exposes_litellm_master_key_as_env_not_host_bound() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("pi")?.plan();
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
        Ok(())
    }

    #[test]
    fn pi_plan_has_expected_mounts() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("pi")?.plan();
        assert_eq!(plan.mounts.len(), 2, "pi should have 2 mounts");
        // /app is no longer mounted — the bun binary + assets are baked into
        // the workestrator-pi image (the daemon can't bind-mount from
        // /nix/store). /app/bin/pi resolves via the symlink in the image.
        assert!(
            plan.mounts.iter().all(|m| m.guest != "/app"),
            "pi must NOT have a /app mount (binary baked into image)"
        );
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
        Ok(())
    }
}
