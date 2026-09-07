//! Guest SSH policy emission: compile a workload's [`CredentialsPlan`]
//! into the fork's guest-visible [`SshConfig`].
//!
//! The emitted policy carries confinement mode plus the allowance set only.
//! The broker unix-socket path and the transport CID are host-side and never
//! enter the spec: they live in the shim handle and the CID registry, and
//! the runtime joins them with this policy view at enforcement time.

use crate::microsandbox::plan::CredentialsPlan;
use microsandbox_types::{HostPattern, PortRange, SshConfig, SshGrant, ViolationAction};

/// Compile the workload's SSH credential view into the guest-visible SSH
/// policy, or `None` when the workload carries neither SSH grants nor
/// strict confinement (silent, so grant-less plans stay byte-identical).
///
/// Mapping notes:
/// - Each grant plan fans out to one [`SshGrant`] per declared host entry
///   (parsed via [`HostPattern::parse`]); each declared port becomes a
///   single-port [`PortRange`].
/// - The plan carries no SSH violation policy, so the deny strength is the
///   [`ViolationAction`] default.
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
        on_violation: ViolationAction::default(),
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
                    PortRange { start: 2222, end: 2222 },
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
        let builder =
            apply_ssh_policy(microsandbox::Sandbox::builder("ssh-emit"), Some(&plan));
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
        let builder =
            apply_ssh_policy(microsandbox::Sandbox::builder("ssh-emit-none"), None);
        assert!(builder.spec().network.ssh.is_none());
        let plain = plan_with(Vec::new(), false);
        let builder = apply_ssh_policy(
            microsandbox::Sandbox::builder("ssh-emit-empty"),
            Some(&plain),
        );
        assert!(builder.spec().network.ssh.is_none());
    }
}
