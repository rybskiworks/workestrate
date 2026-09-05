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

    // FIX2/FIX3: ordered emission — single global sort by specificity desc,
    // deny-before-allow ties, port-scoped before any-port. We expand egress
    // allows (Domains vec) into per-domain items so each domain's specificity
    // is represented; host rules stay as one item each. Deny rules carry
    // port/protocol per FIX2. Merge both vectors into one ordered list and
    // emit in that order so SDK first-match-wins reproduces ADR resolution.
    #[derive(Debug)]
    enum OrderedItem {
        Host {
            protocol: Protocol,
            port: u16,
        },
        AllowDomain {
            domain: String,
            port: u16,
            protocol: Protocol,
        },
        DenyDomain {
            suffix: String,
            port: Option<u16>,
            protocol: Option<Protocol>,
        },
    }
    fn domain_specificity_rank(domain: &str) -> (u8, usize) {
        if domain == "all" {
            return (0, 0);
        }
        if let Some(stripped) = domain.strip_prefix('.') {
            (1, stripped.len())
        } else {
            (2, domain.len())
        }
    }
    let mut ordered: Vec<OrderedItem> = Vec::new();
    for rule in &plan.egress_rules {
        let port = rule.port;
        let proto = rule.protocol;
        match &rule.target {
            EgressTarget::Host => ordered.push(OrderedItem::Host {
                protocol: proto,
                port,
            }),
            EgressTarget::Domains(hosts) => {
                for h in hosts {
                    ordered.push(OrderedItem::AllowDomain {
                        domain: h.clone(),
                        port,
                        protocol: proto,
                    });
                }
            }
        }
    }
    for rule in &plan.deny_rules {
        ordered.push(OrderedItem::DenyDomain {
            suffix: rule.domain_suffix.clone(),
            port: rule.port,
            protocol: rule.protocol,
        });
    }
    // Sort by: host first, then specificity desc, then port-scoped (Some) before any-port, then deny before allow — carve-out wins per ADR §6; identical-coverage ties resolve deny-first (fail-closed)
    ordered.sort_by(|a, b| {
        let a_is_host = matches!(a, OrderedItem::Host { .. });
        let b_is_host = matches!(b, OrderedItem::Host { .. });
        if a_is_host != b_is_host {
            // Host first
            return if a_is_host {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        // Both host or both domain: for host, keep port ordering
        if a_is_host && b_is_host {
            if let (
                OrderedItem::Host {
                    port: pa,
                    protocol: prota,
                },
                OrderedItem::Host {
                    port: pb,
                    protocol: protb,
                },
            ) = (a, b)
            {
                match pa.cmp(pb) {
                    std::cmp::Ordering::Equal => {
                        return format!("{:?}", prota).cmp(&format!("{:?}", protb));
                    }
                    other => return other,
                }
            }
        }
        // Domain items: compute specificity and port/action ranks
        // AllowDomain is always port-scoped (port required) so rank 1; DenyDomain rank 1 if port-scoped else 0 — this achieves carve-out semantics where narrower coverage sorts first.
        let (a_spec, a_len, a_port_rank, a_is_deny) = match a {
            OrderedItem::AllowDomain {
                domain,
                port: _,
                protocol: _,
            } => {
                let (rank, len) = domain_specificity_rank(domain);
                (rank, len, 1, false)
            }
            OrderedItem::DenyDomain {
                suffix,
                port,
                protocol: _,
            } => {
                let (rank, len) = domain_specificity_rank(suffix);
                let pr = if port.is_some() { 1 } else { 0 };
                (rank, len, pr, true)
            }
            OrderedItem::Host { .. } => (0, 0, 0, false),
        };
        let (b_spec, b_len, b_port_rank, b_is_deny) = match b {
            OrderedItem::AllowDomain {
                domain,
                port: _,
                protocol: _,
            } => {
                let (rank, len) = domain_specificity_rank(domain);
                (rank, len, 1, false)
            }
            OrderedItem::DenyDomain {
                suffix,
                port,
                protocol: _,
            } => {
                let (rank, len) = domain_specificity_rank(suffix);
                let pr = if port.is_some() { 1 } else { 0 };
                (rank, len, pr, true)
            }
            OrderedItem::Host { .. } => (0, 0, 0, false),
        };
        // Specificity desc
        match b_spec.cmp(&a_spec) {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }
        match b_len.cmp(&a_len) {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }
        // narrower coverage (port-scoped) sorts before broader (any-port) at equal domain specificity — carve-out wins per ADR §6; identical-coverage ties resolve deny-first (fail-closed)
        match b_port_rank.cmp(&a_port_rank) {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }
        if a_is_deny != b_is_deny {
            return if a_is_deny {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            };
        }
        // Deterministic fallbacks: domain string, port
        match a {
            OrderedItem::AllowDomain {
                domain: da,
                port: pa,
                ..
            } => {
                if let OrderedItem::AllowDomain {
                    domain: db,
                    port: pb,
                    ..
                } = b
                {
                    match da.cmp(db) {
                        std::cmp::Ordering::Equal => pa.cmp(pb),
                        other => other,
                    }
                } else {
                    std::cmp::Ordering::Equal
                }
            }
            OrderedItem::DenyDomain {
                suffix: sa,
                port: pa,
                ..
            } => {
                if let OrderedItem::DenyDomain {
                    suffix: sb,
                    port: pb,
                    ..
                } = b
                {
                    match sa.cmp(sb) {
                        std::cmp::Ordering::Equal => pa.cmp(pb),
                        other => other,
                    }
                } else {
                    std::cmp::Ordering::Equal
                }
            }
            _ => std::cmp::Ordering::Equal,
        }
    });
    for item in ordered {
        match item {
            OrderedItem::Host { protocol, port } => match protocol {
                Protocol::Tcp => {
                    builder = builder.egress(|e| e.tcp().port(port).allow_host());
                }
                Protocol::Udp => {
                    builder = builder.egress(|e| e.udp().port(port).allow_host());
                }
            },
            OrderedItem::AllowDomain {
                domain,
                port,
                protocol,
            } => {
                let is_suffix = domain.starts_with('.');
                match protocol {
                    Protocol::Tcp => {
                        if is_suffix {
                            let d = domain.clone();
                            builder = builder.egress(move |e| {
                                e.tcp().port(port).allow_domain_suffixes([d.as_str()])
                            });
                        } else {
                            let d = domain.clone();
                            builder = builder
                                .egress(move |e| e.tcp().port(port).allow_domains([d.as_str()]));
                        }
                    }
                    Protocol::Udp => {
                        if is_suffix {
                            let d = domain.clone();
                            builder = builder.egress(move |e| {
                                e.udp().port(port).allow_domain_suffixes([d.as_str()])
                            });
                        } else {
                            let d = domain.clone();
                            builder = builder
                                .egress(move |e| e.udp().port(port).allow_domains([d.as_str()]));
                        }
                    }
                }
            }
            OrderedItem::DenyDomain {
                suffix,
                port,
                protocol,
            } => {
                let is_suffix = suffix.starts_with('.');
                // FIX2: port-scoped deny uses protocol+port prefix; any-port without protocol uses any-protocol rule.
                match (port, protocol) {
                    (Some(p), Some(Protocol::Tcp)) => {
                        if is_suffix {
                            let s = suffix.clone();
                            builder = builder.egress(move |e| {
                                e.tcp().port(p).deny_domain_suffixes([s.as_str()])
                            });
                        } else {
                            let s = suffix.clone();
                            builder =
                                builder.egress(move |e| e.tcp().port(p).deny_domains([s.as_str()]));
                        }
                    }
                    (Some(p), Some(Protocol::Udp)) => {
                        if is_suffix {
                            let s = suffix.clone();
                            builder = builder.egress(move |e| {
                                e.udp().port(p).deny_domain_suffixes([s.as_str()])
                            });
                        } else {
                            let s = suffix.clone();
                            builder =
                                builder.egress(move |e| e.udp().port(p).deny_domains([s.as_str()]));
                        }
                    }
                    (Some(p), None) => {
                        // Port-scoped but no protocol (legacy any-protocol port-scoped) — use tcp as default? But any-protocol port-scoped is distinct.
                        // Use tcp as fallback for now, but ideally would be any-protocol port filter. For v1, default to tcp.
                        if is_suffix {
                            let s = suffix.clone();
                            builder = builder.egress(move |e| {
                                e.tcp().port(p).deny_domain_suffixes([s.as_str()])
                            });
                        } else {
                            let s = suffix.clone();
                            builder =
                                builder.egress(move |e| e.tcp().port(p).deny_domains([s.as_str()]));
                        }
                    }
                    (None, Some(Protocol::Tcp)) => {
                        if is_suffix {
                            let s = suffix.clone();
                            builder =
                                builder.egress(move |e| e.tcp().deny_domain_suffixes([s.as_str()]));
                        } else {
                            let s = suffix.clone();
                            builder = builder.egress(move |e| e.tcp().deny_domains([s.as_str()]));
                        }
                    }
                    (None, Some(Protocol::Udp)) => {
                        if is_suffix {
                            let s = suffix.clone();
                            builder =
                                builder.egress(move |e| e.udp().deny_domain_suffixes([s.as_str()]));
                        } else {
                            let s = suffix.clone();
                            builder = builder.egress(move |e| e.udp().deny_domains([s.as_str()]));
                        }
                    }
                    (None, None) => {
                        if is_suffix {
                            let s = suffix.clone();
                            builder = builder.egress(move |e| e.deny_domain_suffixes([s.as_str()]));
                        } else {
                            let s = suffix.clone();
                            builder = builder.egress(move |e| e.deny_domains([s.as_str()]));
                        }
                    }
                }
            }
        }
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
                    egress_defaults_seal: None,
                    ingress_defaults_seal: None,
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
            egress_defaults_seal: None,
            ingress_defaults_seal: None,
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

    #[test]
    fn port_scoped_deny_emits_tcp_port_filter() -> anyhow::Result<()> {
        use crate::microsandbox::plan::{DenyDomainRule, NetworkPlan};
        let plan = NetworkPlan {
            egress_default_deny: true,
            ingress_default_deny: true,
            egress_rules: vec![],
            deny_rules: vec![DenyDomainRule {
                domain_suffix: ".evil.com".to_string(),
                port: Some(443),
                protocol: Some(Protocol::Tcp),
            }],
            ingress_rules: vec![],
            egress_defaults_seal: None,
            ingress_defaults_seal: None,
        };
        let policy = network_plan_to_policy(&plan)?;
        // Should have one egress deny rule with tcp 443 and DomainSuffix
        let rule = policy
            .rules
            .iter()
            .find(|r| r.action == microsandbox::NetworkAction::Deny)
            .expect("deny rule must exist");
        assert!(
            rule.ports.iter().any(|p| p.start == 443),
            "deny rule must be port 443, got {:?}",
            rule.ports
        );
        let v = serde_json::to_value(rule).unwrap();
        // protocols field should contain tcp
        assert!(
            v["protocols"].to_string().contains("tcp"),
            "deny must be tcp, got {:?}",
            v["protocols"]
        );
        // SDK canonicalizes DomainName by stripping leading dot, so .evil.com becomes evil.com
        assert_eq!(v["destination"]["domain_suffix"], "evil.com");
        Ok(())
    }

    #[test]
    fn ordered_egress_emission_respects_specificity_and_deny_before_allow() -> anyhow::Result<()> {
        use crate::microsandbox::plan::{DenyDomainRule, EgressRule, EgressTarget, NetworkPlan};
        // Setup: allow suffix .evil.com and deny exact evil.com, plus allow github.com
        // Specificity: exact evil.com (deny) > suffix .evil.com (allow) > exact github.com (allow)
        // Global order should be: deny exact evil.com, allow suffix .evil.com, allow exact github.com
        // But also test port-scoped vs any-port: port-scoped deny before any-port deny at same specificity
        let plan = NetworkPlan {
            egress_default_deny: true,
            ingress_default_deny: true,
            egress_rules: vec![
                EgressRule {
                    protocol: Protocol::Tcp,
                    port: 443,
                    target: EgressTarget::Domains(vec![".evil.com".to_string()]),
                    derived_from: None,
                },
                EgressRule {
                    protocol: Protocol::Tcp,
                    port: 443,
                    target: EgressTarget::Domains(vec!["github.com".to_string()]),
                    derived_from: None,
                },
            ],
            deny_rules: vec![
                DenyDomainRule {
                    domain_suffix: "evil.com".to_string(), // exact (no leading dot)
                    port: Some(443),
                    protocol: Some(Protocol::Tcp),
                },
                DenyDomainRule {
                    domain_suffix: ".tracker.io".to_string(),
                    port: None,
                    protocol: None,
                },
            ],
            ingress_rules: vec![],
            egress_defaults_seal: None,
            ingress_defaults_seal: None,
        };
        let policy = network_plan_to_policy(&plan)?;
        // Find order of rules: first deny exact evil.com should be before allow suffix .evil.com
        let positions: Vec<(String, String, bool)> = policy
            .rules
            .iter()
            .map(|r| {
                let v = serde_json::to_value(r).unwrap();
                let dest = v["destination"].clone();
                let is_deny = r.action == microsandbox::NetworkAction::Deny;
                (
                    dest.to_string(),
                    r.ports
                        .iter()
                        .map(|p| p.start.to_string())
                        .collect::<Vec<_>>()
                        .join(","),
                    is_deny,
                )
            })
            .collect();
        // Ensure deny evil.com appears before allow .evil.com in the ordered list
        let deny_pos = positions
            .iter()
            .position(|(d, _, is_deny)| {
                *is_deny && d.contains("evil.com") && !d.contains(".tracker.io")
            })
            .unwrap();
        let allow_suffix_pos = positions
            .iter()
            .position(|(d, _, is_deny)| !*is_deny && d.contains("evil.com"))
            .unwrap();
        assert!(
            deny_pos < allow_suffix_pos,
            "deny exact must be before allow suffix: deny {} vs allow {} in {:?}",
            deny_pos,
            allow_suffix_pos,
            positions
        );
        // Port-scoped deny .evil.com:443 should be before any-port deny .tracker.io
        let tracker_pos = positions
            .iter()
            .position(|(d, _, _)| d.contains("tracker.io"))
            .unwrap();
        assert!(
            deny_pos < tracker_pos,
            "port-scoped deny should be before any-port deny"
        );
        Ok(())
    }

    #[test]
    fn identical_coverage_tie_deny_before_allow() -> anyhow::Result<()> {
        use crate::microsandbox::plan::{DenyDomainRule, EgressRule, EgressTarget, NetworkPlan};
        // T1: identical coverage tie — same domain exact evil.com, same port 443 tcp,
        // deny and allow at equal specificity+port must resolve deny-first (fail-closed)
        let plan = NetworkPlan {
            egress_default_deny: true,
            ingress_default_deny: true,
            egress_rules: vec![EgressRule {
                protocol: Protocol::Tcp,
                port: 443,
                target: EgressTarget::Domains(vec!["evil.com".to_string()]),
                derived_from: None,
            }],
            deny_rules: vec![DenyDomainRule {
                domain_suffix: "evil.com".to_string(),
                port: Some(443),
                protocol: Some(Protocol::Tcp),
            }],
            ingress_rules: vec![],
            egress_defaults_seal: None,
            ingress_defaults_seal: None,
        };
        let policy = network_plan_to_policy(&plan)?;
        assert_eq!(policy.rules.len(), 2, "expected deny + allow");
        // Order must be deny before allow at identical coverage
        assert!(
            policy.rules[0].action.is_deny(),
            "first rule must be deny, got {:?} {:?}",
            policy.rules[0].action,
            serde_json::to_value(&policy.rules[0]).unwrap()
        );
        assert!(
            policy.rules[1].action.is_allow(),
            "second rule must be allow, got {:?}",
            policy.rules[1].action
        );
        // Both target same domain; verify via serde shape
        for r in &policy.rules {
            let v = serde_json::to_value(r).unwrap();
            assert!(
                v["destination"]["domain"] == "evil.com"
                    || v["destination"]["domain_suffix"] == "evil.com",
                "both rules must target evil.com, got {}",
                v["destination"]
            );
            assert!(
                r.ports.iter().any(|p| p.start == 443),
                "port 443, got {:?}",
                r.ports
            );
        }
        // Explicit position check via serde for robustness
        let vals: Vec<serde_json::Value> = policy
            .rules
            .iter()
            .map(|r| serde_json::to_value(r).unwrap())
            .collect();
        assert!(
            vals[0]["destination"]["domain"] == "evil.com"
                || vals[0]["destination"]["domain_suffix"] == "evil.com"
        );
        assert!(
            vals[1]["destination"]["domain"] == "evil.com"
                || vals[1]["destination"]["domain_suffix"] == "evil.com"
        );
        Ok(())
    }

    #[test]
    fn carve_out_exact_before_suffix() -> anyhow::Result<()> {
        use crate::microsandbox::plan::{DenyDomainRule, EgressRule, EgressTarget, NetworkPlan};
        // T2: carve-out — exact api.evil.com allow (rank 2) must sort before suffix .evil.com deny (rank 1)
        // at equal port, specificity wins over deny-first
        let plan = NetworkPlan {
            egress_default_deny: true,
            ingress_default_deny: true,
            egress_rules: vec![EgressRule {
                protocol: Protocol::Tcp,
                port: 443,
                target: EgressTarget::Domains(vec!["api.evil.com".to_string()]),
                derived_from: None,
            }],
            deny_rules: vec![DenyDomainRule {
                domain_suffix: ".evil.com".to_string(),
                port: Some(443),
                protocol: Some(Protocol::Tcp),
            }],
            ingress_rules: vec![],
            egress_defaults_seal: None,
            ingress_defaults_seal: None,
        };
        let policy = network_plan_to_policy(&plan)?;
        assert_eq!(policy.rules.len(), 2);
        let v0 = serde_json::to_value(&policy.rules[0]).unwrap();
        let v1 = serde_json::to_value(&policy.rules[1]).unwrap();
        // First must be allow exact api.evil.com
        assert!(
            policy.rules[0].action.is_allow(),
            "exact allow must be first, got deny"
        );
        assert_eq!(v0["destination"]["domain"], "api.evil.com");
        // Second must be deny suffix evil.com
        assert!(
            policy.rules[1].action.is_deny(),
            "suffix deny must be second"
        );
        assert_eq!(v1["destination"]["domain_suffix"], "evil.com");
        Ok(())
    }

    #[test]
    fn port_carve_out_scoped_before_any_port() -> anyhow::Result<()> {
        use crate::microsandbox::plan::{DenyDomainRule, EgressRule, EgressTarget, NetworkPlan};
        // T3: port carve-out — at equal domain specificity (.evil.com suffix), port-scoped allow :443 (rank1) sorts before any-port deny (rank0)
        let plan = NetworkPlan {
            egress_default_deny: true,
            ingress_default_deny: true,
            egress_rules: vec![EgressRule {
                protocol: Protocol::Tcp,
                port: 443,
                target: EgressTarget::Domains(vec![".evil.com".to_string()]),
                derived_from: None,
            }],
            deny_rules: vec![DenyDomainRule {
                domain_suffix: ".evil.com".to_string(),
                port: None,
                protocol: None,
            }],
            ingress_rules: vec![],
            egress_defaults_seal: None,
            ingress_defaults_seal: None,
        };
        let policy = network_plan_to_policy(&plan)?;
        assert_eq!(policy.rules.len(), 2);
        let v0 = serde_json::to_value(&policy.rules[0]).unwrap();
        let v1 = serde_json::to_value(&policy.rules[1]).unwrap();
        // Allow must be first (narrower coverage carve-out)
        assert!(
            policy.rules[0].action.is_allow(),
            "port-scoped allow must be first"
        );
        assert_eq!(v0["destination"]["domain_suffix"], "evil.com");
        assert!(policy.rules[0].ports.iter().any(|p| p.start == 443));
        // Deny any-port must be second (broader)
        assert!(
            policy.rules[1].action.is_deny(),
            "any-port deny must be second"
        );
        assert_eq!(v1["destination"]["domain_suffix"], "evil.com");
        assert!(
            policy.rules[1].ports.is_empty(),
            "any-port deny should have no port filter, got {:?}",
            policy.rules[1].ports
        );
        Ok(())
    }

    #[test]
    fn port_scoped_deny_before_broader_allow() -> anyhow::Result<()> {
        use crate::microsandbox::plan::{DenyDomainRule, EgressRule, EgressTarget, NetworkPlan};
        // T4: port-scoped deny evil.com:443 vs broader allow evil.com:80 (same exact specificity, both port-scoped rank1)
        // identical-coverage tie resolves deny-first even though allow port 80 < deny port 443 numerically
        let plan = NetworkPlan {
            egress_default_deny: true,
            ingress_default_deny: true,
            egress_rules: vec![EgressRule {
                protocol: Protocol::Tcp,
                port: 80,
                target: EgressTarget::Domains(vec!["evil.com".to_string()]),
                derived_from: None,
            }],
            deny_rules: vec![DenyDomainRule {
                domain_suffix: "evil.com".to_string(),
                port: Some(443),
                protocol: Some(Protocol::Tcp),
            }],
            ingress_rules: vec![],
            egress_defaults_seal: None,
            ingress_defaults_seal: None,
        };
        let policy = network_plan_to_policy(&plan)?;
        assert_eq!(policy.rules.len(), 2);
        let v0 = serde_json::to_value(&policy.rules[0]).unwrap();
        let v1 = serde_json::to_value(&policy.rules[1]).unwrap();
        // Deny :443 must be first (deny-first tie at identical coverage, overriding numeric port order)
        assert!(
            policy.rules[0].action.is_deny(),
            "deny :443 must be first, got allow"
        );
        assert_eq!(v0["destination"]["domain"], "evil.com");
        assert!(policy.rules[0].ports.iter().any(|p| p.start == 443));
        // Allow :80 second
        assert!(
            policy.rules[1].action.is_allow(),
            "allow :80 must be second"
        );
        assert_eq!(v1["destination"]["domain"], "evil.com");
        assert!(policy.rules[1].ports.iter().any(|p| p.start == 80));
        Ok(())
    }
}
