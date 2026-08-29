use super::super::plan::{EgressTarget, NetworkPlan, Protocol, Scope};
use anyhow::Result;
use microsandbox::{NetworkAction, NetworkPolicy};

/// Convert a declarative `NetworkPlan` into a Microsandbox SDK `NetworkPolicy`.
pub fn network_plan_to_policy(plan: &NetworkPlan) -> Result<NetworkPolicy> {
    let mut builder = NetworkPolicy::builder();

    // Per-direction defaults. NOTE: without the explicit allow mapping the
    // msb builder falls back to Deny, so a plan with a relaxed default
    // (`egress = "allow"` / `ingress = "allow"`) would still enforce
    // deny-all in that direction. Map each direction explicitly.
    let egress_action = if plan.egress_default_deny {
        NetworkAction::Deny
    } else {
        NetworkAction::Allow
    };
    let ingress_action = if plan.ingress_default_deny {
        NetworkAction::Deny
    } else {
        NetworkAction::Allow
    };
    builder = builder
        .default_egress(egress_action)
        .default_ingress(ingress_action);

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
    use crate::microsandbox::plan::{EgressTarget, NetworkPlan, Protocol};
    use crate::microsandbox::workload::{ConfigWorkload, Workload};

    /// Regression: the plan's per-direction defaults must map to the SDK
    /// builder's `default_egress`/`default_ingress` — previously neither
    /// builder method was called and the policy silently fell back to
    /// deny-all in BOTH directions.
    #[test]
    fn direction_defaults_map_to_sdk_per_direction() -> anyhow::Result<()> {
        for egress_deny in [true, false] {
            for ingress_deny in [true, false] {
                let plan = NetworkPlan {
                    egress_default_deny: egress_deny,
                    ingress_default_deny: ingress_deny,
                    egress_rules: vec![],
                    deny_rules: vec![],
                    ingress_rules: vec![],
                };
                let policy = network_plan_to_policy(&plan)?;
                assert_eq!(
                    policy.default_egress.is_allow(),
                    !egress_deny,
                    "egress_default_deny={egress_deny} must produce default_egress {}",
                    if egress_deny { "deny" } else { "allow" }
                );
                assert_eq!(
                    policy.default_ingress.is_allow(),
                    !ingress_deny,
                    "ingress_default_deny={ingress_deny} must produce default_ingress {}",
                    if ingress_deny { "deny" } else { "allow" }
                );
            }
        }
        Ok(())
    }

    /// Regression: explicit ingress allow rules survive under a deny ingress
    /// default — the default must not swallow declared `[[network.ingress]]`
    /// rules.
    #[test]
    fn ingress_allow_rule_survives_under_deny_default() -> anyhow::Result<()> {
        let plan = NetworkPlan {
            egress_default_deny: true,
            ingress_default_deny: true,
            egress_rules: vec![],
            deny_rules: vec![],
            ingress_rules: vec![crate::microsandbox::plan::IngressRule {
                protocol: Protocol::Tcp,
                port: 7000,
                scope: crate::microsandbox::plan::Scope::Local,
            }],
        };
        let policy = network_plan_to_policy(&plan)?;
        assert!(
            !policy.default_ingress.is_allow(),
            "ingress default must stay deny"
        );
        // `Direction` is not re-exported by the SDK facade; assert on the
        // serialized rule shape instead (snake_case direction, port range).
        let has_ingress_allow = policy.rules.iter().any(|r| {
            let v = serde_json::to_value(r).expect("rule serializes");
            v["direction"] == "ingress"
                && r.action.is_allow()
                && r.ports.iter().any(|p| p.start == 7000)
        });
        assert!(
            has_ingress_allow,
            "policy must carry the tcp/7000 ingress allow rule; got {:?}",
            policy.rules
        );
        Ok(())
    }

    #[test]
    fn example_litellm_network_plan_converts_without_error() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("example-litellm")?.plan();
        let result = network_plan_to_policy(&plan.network);
        assert!(
            result.is_ok(),
            "example-litellm conversion failed: {:?}",
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
        // image must be the nix-built `workestrate-pi:latest` (built by the
        // config repo flake and loaded into the msb store there), NOT
        // node:24-bookworm-slim (glibc 2.36 → crash).
        let plan = ConfigWorkload::new("pi")?.plan();
        assert_eq!(plan.image.as_deref(), Some("workestrate-pi:latest"));
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
            assert!(!m.is_read_only(), "/data mount must be readwrite");
        }
        let app_mount = plan.mounts.iter().find(|m| m.guest == "/app");
        assert!(app_mount.is_some(), "odysseus must have a /app mount");
        if let Some(m) = app_mount {
            assert_eq!(m.host, "agents/odysseus/build");
            assert!(m.is_read_only(), "/app mount must be readonly");
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
        assert!(plan.network.egress_default_deny);
        assert!(plan.network.ingress_default_deny);
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
    fn pi_plan_host_binds_litellm_master_key_as_placeholder_not_env() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("pi")?.plan();
        // Spec 16: pi is a PRESENTER workload — it binds the master key as a
        // host-bound placeholder (`LITELLM_MASTER_KEY = true`), never the real
        // value. The v2 def-global `delivery = "env"` over-exposed the real
        // key to pi; per-binding `bound` fixes that (only verifier workloads
        // opt into `bound = "guest"`).
        let in_env = plan
            .env
            .iter()
            .any(|e| e.name == "LITELLM_MASTER_KEY" && e.is_secret);
        assert!(
            !in_env,
            "pi must NOT expose LITELLM_MASTER_KEY as a guest env var (presenter posture: placeholder only)"
        );
        let in_secret_env = plan
            .secret_env
            .iter()
            .any(|s| s.name == "LITELLM_MASTER_KEY");
        assert!(
            in_secret_env,
            "pi must host-bind LITELLM_MASTER_KEY (placeholder in-sandbox; substitution at egress)"
        );
        Ok(())
    }

    #[test]
    fn pi_plan_has_expected_mounts() -> anyhow::Result<()> {
        let _guard = TestConfigGuard::new();
        let plan = ConfigWorkload::new("pi")?.plan();
        assert_eq!(plan.mounts.len(), 2, "pi should have 2 mounts");
        // /app is no longer mounted — the bun binary + assets are baked into
        // the workestrate-pi image (the daemon can't bind-mount from
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
            assert!(!m.is_read_only(), "/data mount must be readwrite");
        }
        let work_mount = plan.mounts.iter().find(|m| m.guest == "/work");
        assert!(
            work_mount.is_some(),
            "pi must have a /work mount (user project)"
        );
        if let Some(m) = work_mount {
            assert!(!m.is_read_only(), "/work mount must be readwrite");
        }
        assert!(
            plan.mounts.iter().all(|m| m.guest != "/workspace"),
            "pi must not mount /workspace (replaced by /work + /data)"
        );
        Ok(())
    }
}
