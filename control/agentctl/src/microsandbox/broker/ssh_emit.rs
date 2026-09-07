//! Guest SSH policy emission: compile a workload's [`CredentialsPlan`]
//! into the fork's guest-visible [`SshConfig`].
//!
//! The emitted policy carries confinement mode plus the allowance set only.
//! The broker unix-socket path and the transport CID are host-side and never
//! enter the spec: they live in the shim handle and the CID registry, and
//! the runtime joins them with this policy view at enforcement time.

use crate::config::SecretViolationPolicy;
use crate::microsandbox::plan::CredentialsPlan;
use microsandbox_types::{HostPattern, PortRange, SshConfig, SshGrant, ViolationAction};

/// Map one grant's effective violation policy onto the fork's guest SSH
/// deny strength. `Passthrough` carries no host set on the SSH view (the
/// divert prelude has no placeholder to forward), so it emits an empty
/// passthrough set; the network engine coerces that to `Block` at
/// enforcement, keeping the wire intent distinct from the deny default.
pub fn ssh_violation_action(policy: SecretViolationPolicy) -> ViolationAction {
    match policy {
        SecretViolationPolicy::Passthrough => ViolationAction::Passthrough(Vec::new()),
        SecretViolationPolicy::Block => ViolationAction::Block,
        SecretViolationPolicy::BlockAndLog => ViolationAction::BlockAndLog,
        SecretViolationPolicy::BlockAndTerminate => ViolationAction::BlockAndTerminate,
    }
}

/// Severity rank for the single-slot SSH deny strength: passthrough is no
/// enforcement (weakest), then block-and-log below block below
/// block-and-terminate. Matches the broker scan ordering so the SSH view
/// and the DLP action reduction agree on which policy is strictest.
fn ssh_policy_priority(policy: SecretViolationPolicy) -> u8 {
    match policy {
        SecretViolationPolicy::Passthrough => 0,
        SecretViolationPolicy::BlockAndLog => 1,
        SecretViolationPolicy::Block => 2,
        SecretViolationPolicy::BlockAndTerminate => 3,
    }
}

/// Reduce a grant set's effective policies to the single guest deny
/// strength: the strictest grant wins, so one tightening grant tightens
/// the whole SSH view. Order-independent (a pure max over the rank).
/// Empty input falls back to the [`ViolationAction`] default, preserving
/// the strict-only emission.
pub fn ssh_violation_for_grants(
    policies: impl IntoIterator<Item = SecretViolationPolicy>,
) -> ViolationAction {
    let mut best: Option<SecretViolationPolicy> = None;
    for policy in policies {
        let replace = match best {
            None => true,
            Some(current) => ssh_policy_priority(policy) > ssh_policy_priority(current),
        };
        if replace {
            best = Some(policy);
        }
    }
    match best {
        None => ViolationAction::default(),
        Some(policy) => ssh_violation_action(policy),
    }
}

/// Compile the workload's SSH credential view into the guest-visible SSH
/// policy, or `None` when the workload carries neither SSH grants nor
/// strict confinement (silent, so grant-less plans stay byte-identical).
///
/// Mapping notes:
/// - Each grant plan fans out to one [`SshGrant`] per declared host entry
///   (parsed via [`HostPattern::parse`]); each declared port becomes a
///   single-port [`PortRange`].
/// - The guest view carries a single deny strength, so the grants'
///   effective policies reduce strictest-wins
///   ([`ssh_violation_for_grants`]); a grant-less strict-only plan keeps
///   the [`ViolationAction`] default.
/// - Grant `users` have no counterpart in the guest policy (the divert
///   prelude carries only host and port), so they are dropped here. They
///   still ride the provenance hash as declared allowance intent.
pub fn ssh_config_for_plan(plan: &CredentialsPlan) -> Option<SshConfig> {
    if plan.ssh.is_empty() && !plan.strict {
        return None;
    }
    let mut grants = Vec::new();
    for grant in &plan.ssh {
        let ports: Vec<PortRange> = grant
            .ports
            .iter()
            .map(|p| PortRange { start: *p, end: *p })
            .collect();
        for host in &grant.hosts {
            grants.push(SshGrant {
                host: HostPattern::parse(host),
                ports: ports.clone(),
            });
        }
    }
    Some(SshConfig {
        strict: plan.strict,
        grants,
        on_violation: ssh_violation_for_grants(plan.ssh.iter().map(|g| g.on_violation)),
    })
}

/// Thread an optional credential view into the sandbox builder as the
/// fork's first-class `network.ssh` spec option, following the
/// `apply_nested_virt` threading style (small pure helper + one call site
/// just before create). `None` (or a grant-less, non-strict plan) leaves
/// the builder untouched.
pub fn apply_ssh_policy(
    builder: microsandbox::sandbox::SandboxBuilder,
    credentials: Option<&CredentialsPlan>,
) -> microsandbox::sandbox::SandboxBuilder {
    let Some(config) = credentials.and_then(ssh_config_for_plan) else {
        return builder;
    };
    builder.overlay(ssh_overlay_patch(&config))
}

/// Build the `SandboxConfig → network → ssh` overlay patch carrying a
/// compiled [`SshConfig`]. Split from [`apply_ssh_policy`] so tests can
/// assert the patch content via `SandboxBuilder::overlay` + `spec()`
/// without booting a VM.
pub fn ssh_overlay_patch(config: &SshConfig) -> microsandbox_types::SandboxConfigPatch {
    use microsandbox_types::{NetworkSpecPatch, SandboxConfigPatch, SshConfigPatch};
    SandboxConfigPatch::new().network(
        NetworkSpecPatch::new().ssh(
            SshConfigPatch::new()
                .strict(config.strict)
                .grants(config.grants.clone())
                .on_violation(config.on_violation.clone()),
        ),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::microsandbox::plan::{CredentialBinding, CredentialsPlan, SshGrantPlan};

    fn ssh_grant(hosts: &[&str], ports: Vec<u16>) -> SshGrantPlan {
        SshGrantPlan {
            name: "deploy".to_string(),
            material: "DEPLOY_KEY".to_string(),
            hosts: hosts.iter().map(|h| h.to_string()).collect(),
            users: vec!["git".to_string()],
            ports,
            binding: CredentialBinding::Broker,
            on_violation: crate::config::SecretViolationPolicy::Passthrough,
        }
    }

    fn plan_with(ssh: Vec<SshGrantPlan>, strict: bool) -> CredentialsPlan {
        CredentialsPlan {
            ssh,
            signing: Vec::new(),
            strict,
            strict_origin: None,
        }
    }

    #[test]
    fn empty_non_strict_plan_emits_nothing() {
        assert!(ssh_config_for_plan(&plan_with(Vec::new(), false)).is_none());
    }

    #[test]
    fn strict_only_plan_emits_policy_without_grants() {
        // Strict-only confinement still changes guest enforcement, so it
        // emits a policy (no listener is bound for it — that needs grants).
        let config = ssh_config_for_plan(&plan_with(Vec::new(), true)).expect("strict emits");
        assert!(config.strict);
        assert!(config.grants.is_empty());
        assert_eq!(config.on_violation, ViolationAction::default());
    }

    #[test]
    fn grant_fans_out_per_host_with_single_port_ranges() {
        let plan = plan_with(
            vec![ssh_grant(&["github.com", "*.example.com"], vec![22, 2222])],
            false,
        );
        let config = ssh_config_for_plan(&plan).expect("grants emit");
        assert!(!config.strict);
        assert_eq!(config.grants.len(), 2);
        assert_eq!(
            config.grants[0].host,
            HostPattern::Exact("github.com".to_string())
        );
        assert_eq!(
            config.grants[1].host,
            HostPattern::Wildcard("*.example.com".to_string())
        );
        for grant in &config.grants {
            assert_eq!(
                grant.ports,
                vec![
                    PortRange { start: 22, end: 22 },
                    PortRange {
                        start: 2222,
                        end: 2222
                    },
                ]
            );
        }
    }

    #[test]
    fn star_host_parses_to_any() {
        let plan = plan_with(vec![ssh_grant(&["*"], vec![22])], false);
        let config = ssh_config_for_plan(&plan).expect("grants emit");
        assert_eq!(config.grants.len(), 1);
        assert_eq!(config.grants[0].host, HostPattern::Any);
    }

    #[test]
    fn overlay_patch_lands_on_the_built_spec() {
        let plan = plan_with(vec![ssh_grant(&["github.com"], vec![22])], true);
        let config = ssh_config_for_plan(&plan).expect("grants emit");
        let builder = apply_ssh_policy(microsandbox::Sandbox::builder("ssh-emit"), Some(&plan));
        let ssh = builder
            .spec()
            .network
            .ssh
            .clone()
            .expect("overlay must carry the ssh policy");
        assert_eq!(ssh, config);
        assert!(ssh.strict);
        assert_eq!(ssh.grants.len(), 1);
    }

    #[test]
    fn apply_ssh_policy_leaves_grant_less_builder_untouched() {
        let builder = apply_ssh_policy(microsandbox::Sandbox::builder("ssh-emit-none"), None);
        assert!(builder.spec().network.ssh.is_none());
        let plain = plan_with(Vec::new(), false);
        let builder = apply_ssh_policy(
            microsandbox::Sandbox::builder("ssh-emit-empty"),
            Some(&plain),
        );
        assert!(builder.spec().network.ssh.is_none());
    }

    fn ssh_grant_with_policy(
        hosts: &[&str],
        ports: Vec<u16>,
        policy: crate::config::SecretViolationPolicy,
    ) -> SshGrantPlan {
        SshGrantPlan {
            on_violation: policy,
            ..ssh_grant(hosts, ports)
        }
    }

    #[test]
    fn single_grant_policy_threads_to_guest_deny_strength() {
        use crate::config::SecretViolationPolicy as Policy;
        for (policy, expected) in [
            (Policy::Block, ViolationAction::Block),
            (Policy::BlockAndLog, ViolationAction::BlockAndLog),
            (
                Policy::BlockAndTerminate,
                ViolationAction::BlockAndTerminate,
            ),
        ] {
            let plan = plan_with(
                vec![ssh_grant_with_policy(&["github.com"], vec![22], policy)],
                false,
            );
            let config = ssh_config_for_plan(&plan).expect("grants emit");
            assert_eq!(
                config.on_violation, expected,
                "policy {policy:?} must thread"
            );
        }
        let plan = plan_with(
            vec![ssh_grant_with_policy(
                &["github.com"],
                vec![22],
                Policy::Passthrough,
            )],
            false,
        );
        let config = ssh_config_for_plan(&plan).expect("grants emit");
        assert_eq!(
            config.on_violation,
            ViolationAction::Passthrough(Vec::new()),
            "passthrough keeps its wire shape (the engine coerces it to block)"
        );
    }

    #[test]
    fn strictest_grant_wins_independent_of_order() {
        use crate::config::SecretViolationPolicy as Policy;
        let forward = vec![
            ssh_grant_with_policy(&["a.example"], vec![22], Policy::BlockAndLog),
            ssh_grant_with_policy(&["b.example"], vec![22], Policy::Block),
            ssh_grant_with_policy(&["c.example"], vec![22], Policy::Passthrough),
        ];
        let mut reverse = forward.clone();
        reverse.reverse();
        let first = ssh_config_for_plan(&plan_with(forward, false)).expect("grants emit");
        let second = ssh_config_for_plan(&plan_with(reverse, false)).expect("grants emit");
        assert_eq!(first.on_violation, ViolationAction::Block);
        assert_eq!(second.on_violation, first.on_violation);
        let terminating = vec![
            ssh_grant_with_policy(&["a.example"], vec![22], Policy::Block),
            ssh_grant_with_policy(&["b.example"], vec![22], Policy::BlockAndTerminate),
        ];
        let config = ssh_config_for_plan(&plan_with(terminating, false)).expect("grants emit");
        assert_eq!(config.on_violation, ViolationAction::BlockAndTerminate);
    }
}
