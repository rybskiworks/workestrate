//! Credential-broker grant resolution (M1): compile a workload's
//! `[workloads.<name>.credentials]` allowlist against the repo-global
//! `[credentials.*]` catalog into the plan's [`CredentialsPlan`].
//!
//! Binding reuses the EXISTING secret-delivery path: omission of any guest
//! binding renders the grant broker-bound (the default, secure); an env
//! secret consumption of the material secret with `bound = "guest"`
//! renders it guest-bound. No new machinery, no `via`/`recipe` vocabulary.

use crate::config::{Bound, ConfigFile, EnvBinding, SshPolicyFragment, WorkloadConfig};
use crate::microsandbox::plan::{
    CredentialBinding, CredentialsPlan, SigningGrantPlan, SshGrantPlan,
};
use crate::microsandbox::secrets::SecretDefinition;
use anyhow::Result;
use std::collections::HashMap;

/// Default SSH ports when a catalog entry omits `ports`
/// (defaults-after-merge — the field stays `None` through parse and merge,
/// the same idiom as `EnvSecretRef.bound`).
pub const DEFAULT_SSH_PORTS: [u16; 1] = [22];

/// Walk the SSH confinement-policy ladder for ONE workload and return
/// `(effective strict, deciding-rung origin)` — the strict edition of
/// [`crate::microsandbox::workload::secrets::resolve_on_violation`]: the
/// built-in default is `false`; each rung's `strict` (when present) becomes
/// the effective value; `final = true` is a terminal freeze — the walk stops
/// and every lower rung is frozen out. A final rung without `strict`
/// freezes the value resolved so far.
pub(crate) fn resolve_ssh_strict(rungs: &[(&str, &SshPolicyFragment)]) -> (bool, String) {
    let mut effective = false;
    let mut origin = "built-in".to_string();
    for (label, fragment) in rungs {
        if let Some(value) = fragment.strict {
            effective = value;
            origin = (*label).to_string();
        }
        if fragment.r#final {
            return (effective, origin);
        }
    }
    (effective, origin)
}

/// Resolve the effective SSH confinement for one workload against the
/// collected ladder rungs in authority-ascending order (home registry, then
/// config-repo layers in stack order, then this workload's capsule rungs —
/// from the process-global stored at load time). An empty or absent ladder
/// degrades to the built-in default (`false`), so synthetic/test paths
/// without a load stay correct.
pub(crate) fn resolve_ssh_for_workload(workload_name: &str) -> (bool, String) {
    let ladder = crate::merge::get_ssh_policy_ladder().unwrap_or_default();
    let mut rungs: Vec<(String, SshPolicyFragment)> = Vec::new();
    if let Some((origin, fragment)) = &ladder.home {
        rungs.push((origin.clone(), fragment.clone()));
    }
    for (origin, fragment) in &ladder.layers {
        rungs.push((origin.clone(), fragment.clone()));
    }
    if let Some(workload_rungs) = ladder.workloads.get(workload_name) {
        for (origin, fragment) in workload_rungs {
            rungs.push((origin.clone(), fragment.clone()));
        }
    }
    let rung_refs: Vec<(&str, &SshPolicyFragment)> =
        rungs.iter().map(|(o, f)| (o.as_str(), f)).collect();
    resolve_ssh_strict(&rung_refs)
}

/// Resolve one grant's exposure from the workload's EXISTING env secret
/// consumptions: guest-bound iff some env binding consumes the material
/// secret with `bound = "guest"`; anything else — including omission — is
/// broker-bound (the default, secure). This is the secret-delivery path's
/// own dispatch, reused — `bound` lives ONLY at the env binding site, never
/// on a credential definition.
fn resolve_binding(workload: &WorkloadConfig, material: &str) -> CredentialBinding {
    let guest = workload.env.iter().any(|(_, binding)| match binding {
        EnvBinding::Secret(ref_) => ref_.secret == material && ref_.bound == Some(Bound::Guest),
        EnvBinding::Literal(_) => false,
    });
    if guest {
        CredentialBinding::Guest
    } else {
        CredentialBinding::Broker
    }
}

/// Compile a workload's grant allowlist against the catalog: SSH grants
/// resolve material→secret plus the confinement scope (hosts/users/ports,
/// ports defaulting to `[22]`) and the violation policy (the entry's
/// `on_violation`, else the material secret's ladder-resolved policy —
/// the SSH edition of the signing grant's inheritance); signing grants
/// resolve material→secret plus namespace and violation policy (the entry's
/// `on_violation`, else the material secret's ladder-resolved policy).
///
/// Unknown grant names and unknown material secrets are hard errors naming
/// the workload and the credential — [`crate::config::validate_config`]
/// reports them first on every load path; this is defense-in-depth for
/// synthetic constructions (mirroring `build_env_and_secret_env`).
pub(crate) fn build_credential_grants(
    config: &ConfigFile,
    workload_name: &str,
    workload: &WorkloadConfig,
    secrets: &HashMap<String, SecretDefinition>,
) -> Result<(Vec<SshGrantPlan>, Vec<SigningGrantPlan>)> {
    let mut ssh = Vec::with_capacity(workload.credentials.ssh.len());
    for name in &workload.credentials.ssh {
        let entry = config.credentials.ssh.get(name).ok_or_else(|| {
            anyhow::anyhow!(
                "workload '{workload_name}' credentials.ssh references undefined credential '{name}'"
            )
        })?;
        let def = secrets.get(&entry.material).ok_or_else(|| {
            anyhow::anyhow!(
                "credential ssh '{name}' references undefined secret '{}'",
                entry.material
            )
        })?;
        ssh.push(SshGrantPlan {
            name: name.clone(),
            material: entry.material.clone(),
            hosts: entry.hosts.clone(),
            users: entry.users.clone(),
            ports: entry
                .ports
                .clone()
                .unwrap_or_else(|| DEFAULT_SSH_PORTS.to_vec()),
            binding: resolve_binding(workload, &entry.material),
            on_violation: entry.on_violation.unwrap_or(def.on_violation),
        });
    }

    let mut signing = Vec::with_capacity(workload.credentials.signing.len());
    for name in &workload.credentials.signing {
        let entry = config.credentials.signing.ssh.get(name).ok_or_else(|| {
            anyhow::anyhow!(
                "workload '{workload_name}' credentials.signing references undefined credential '{name}'"
            )
        })?;
        let def = secrets.get(&entry.material).ok_or_else(|| {
            anyhow::anyhow!(
                "credential signing '{name}' references undefined secret '{}'",
                entry.material
            )
        })?;
        signing.push(SigningGrantPlan {
            name: name.clone(),
            material: entry.material.clone(),
            namespace: entry.namespace.clone(),
            on_violation: entry.on_violation.unwrap_or(def.on_violation),
            binding: resolve_binding(workload, &entry.material),
        });
    }

    Ok((ssh, signing))
}

/// Build the compiled credential-broker view for ONE workload, or `None`
/// when the workload carries no grants and no confinement (silent, so
/// legacy plan output stays byte-identical).
pub(crate) fn build_credentials_plan(
    config: &ConfigFile,
    workload_name: &str,
    workload: &WorkloadConfig,
    secrets: &HashMap<String, SecretDefinition>,
) -> Result<Option<CredentialsPlan>> {
    let (ssh, signing) = build_credential_grants(config, workload_name, workload, secrets)?;
    let (strict, origin) = resolve_ssh_for_workload(workload_name);
    if ssh.is_empty() && signing.is_empty() && !strict {
        return Ok(None);
    }
    Ok(Some(CredentialsPlan {
        ssh,
        signing,
        strict,
        strict_origin: Some(origin),
    }))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;
    use crate::config::SecretViolationPolicy;

    fn ssh_toml(catalog: &str, workload: &str) -> String {
        format!(
            "schema_version = 1\n\n{catalog}\n[workloads.pi]\nkind = \"agent\"\nimage = {{ recipe = \"registry\", ref = \"node:24\" }}\ncommand = []\n{workload}\n\n[workloads.pi.network.defaults]\negress = \"deny\""
        )
    }

    fn secrets_for(config: &ConfigFile) -> HashMap<String, SecretDefinition> {
        crate::microsandbox::workload::secrets::build_secret_definitions(config).unwrap()
    }

    // ---- resolve_ssh_strict: ladder walk ----

    #[test]
    fn strict_defaults_false_with_no_rungs() {
        let (strict, origin) = resolve_ssh_strict(&[]);
        assert!(!strict);
        assert_eq!(origin, "built-in");
    }

    #[test]
    fn strict_last_value_wins_authority_ascending() {
        let home = SshPolicyFragment {
            strict: Some(false),
            r#final: false,
        };
        let layer = SshPolicyFragment {
            strict: Some(true),
            r#final: false,
        };
        let rungs = [("home-registry", &home), ("team", &layer)];
        let refs: Vec<(&str, &SshPolicyFragment)> = rungs.iter().map(|(o, f)| (*o, *f)).collect();
        let (strict, origin) = resolve_ssh_strict(&refs);
        assert!(strict);
        assert_eq!(origin, "team");
    }

    #[test]
    fn strict_final_freezes_lower_rungs() {
        let home = SshPolicyFragment {
            strict: Some(false),
            r#final: true,
        };
        let layer = SshPolicyFragment {
            strict: Some(true),
            r#final: false,
        };
        let rungs = [("home-registry", &home), ("team", &layer)];
        let refs: Vec<(&str, &SshPolicyFragment)> = rungs.iter().map(|(o, f)| (*o, *f)).collect();
        let (strict, origin) = resolve_ssh_strict(&refs);
        assert!(!strict, "home final=false freezes the later strict=true");
        assert_eq!(origin, "home-registry");
    }

    #[test]
    fn strict_bare_final_freezes_value_resolved_so_far() {
        let home = SshPolicyFragment {
            strict: Some(true),
            r#final: false,
        };
        let bare = SshPolicyFragment {
            strict: None,
            r#final: true,
        };
        let layer = SshPolicyFragment {
            strict: Some(false),
            r#final: false,
        };
        let rungs = [
            ("home-registry", &home),
            ("team", &bare),
            ("capsule", &layer),
        ];
        let refs: Vec<(&str, &SshPolicyFragment)> = rungs.iter().map(|(o, f)| (*o, *f)).collect();
        let (strict, origin) = resolve_ssh_strict(&refs);
        assert!(strict, "bare final freezes the true resolved above it");
        assert_eq!(origin, "home-registry");
    }

    // ---- build_credential_grants: ports default, binding, signing ----

    #[test]
    fn ports_default_to_22_when_absent() -> Result<()> {
        let layer = crate::merge::Layer::from_string(
            "base",
            &ssh_toml(
                "[secrets.DEPLOY_KEY]\n[credentials.ssh.deploy]\nmaterial = \"DEPLOY_KEY\"\nhosts = [\"github.com\"]\nusers = [\"git\"]\n",
                "[workloads.pi.credentials]\nssh = [\"deploy\"]\n",
            ),
        )?;
        let (config, _) = crate::merge::merge_layers(&[layer])?;
        let secrets = secrets_for(&config);
        let workload = config.workloads.get("pi").unwrap();
        let (ssh, _) = build_credential_grants(&config, "pi", workload, &secrets)?;
        assert_eq!(ssh.len(), 1);
        assert_eq!(ssh[0].ports, vec![22]);
        assert_eq!(ssh[0].binding, CredentialBinding::Broker);
        Ok(())
    }

    #[test]
    fn omission_is_broker_bound_guest_binding_is_guest_bound() -> Result<()> {
        // Same catalog, two workloads: one omits any binding (broker-bound),
        // one consumes the material with bound="guest" (guest-bound).
        let layer = crate::merge::Layer::from_string(
            "base",
            &ssh_toml(
                "[secrets.DEPLOY_KEY]\n[credentials.ssh.deploy]\nmaterial = \"DEPLOY_KEY\"\nhosts = [\"github.com\"]\nusers = [\"git\"]\nports = [22, 2222]\n",
                "[workloads.pi.credentials]\nssh = [\"deploy\"]\n[workloads.pi.env]\nSSH_KEY = { secret = \"DEPLOY_KEY\", bound = \"guest\" }\n",
            ),
        )?;
        let (config, _) = crate::merge::merge_layers(&[layer])?;
        let secrets = secrets_for(&config);
        let workload = config.workloads.get("pi").unwrap();
        let (ssh, _) = build_credential_grants(&config, "pi", workload, &secrets)?;
        assert_eq!(ssh[0].binding, CredentialBinding::Guest);
        assert_eq!(ssh[0].ports, vec![22, 2222]);
        Ok(())
    }

    #[test]
    fn signing_inherits_secret_policy_unless_overridden() -> Result<()> {
        let layer = crate::merge::Layer::from_string(
            "base",
            &ssh_toml(
                "[secrets.SIGN_KEY]\non_violation = \"block\"\n[credentials.signing.ssh.rel]\nmaterial = \"SIGN_KEY\"\nnamespace = \"release\"\n[credentials.signing.ssh.rel-strict]\nmaterial = \"SIGN_KEY\"\nnamespace = \"release\"\non_violation = \"block-and-terminate\"\n",
                "[workloads.pi.credentials]\nsigning = [\"rel\", \"rel-strict\"]\n",
            ),
        )?;
        let (config, _) = crate::merge::merge_layers(&[layer])?;
        let secrets = secrets_for(&config);
        let workload = config.workloads.get("pi").unwrap();
        let (_, signing) = build_credential_grants(&config, "pi", workload, &secrets)?;
        assert_eq!(signing.len(), 2);
        assert_eq!(
            signing[0].on_violation,
            SecretViolationPolicy::Block,
            "absent entry policy inherits the material secret's"
        );
        assert_eq!(
            signing[1].on_violation,
            SecretViolationPolicy::BlockAndTerminate
        );
        Ok(())
    }

    #[test]
    fn ssh_grant_inherits_material_policy_unless_overridden() -> Result<()> {
        let layer = crate::merge::Layer::from_string(
            "base",
            &ssh_toml(
                "[secrets.DEPLOY_KEY]\non_violation = \"block\"\n[credentials.ssh.deploy]\nmaterial = \"DEPLOY_KEY\"\nhosts = [\"github.com\"]\nusers = [\"git\"]\n[credentials.ssh.deploy-strict]\nmaterial = \"DEPLOY_KEY\"\nhosts = [\"github.com\"]\nusers = [\"git\"]\non_violation = \"block-and-terminate\"\n",
                "[workloads.pi.credentials]\nssh = [\"deploy\", \"deploy-strict\"]\n",
            ),
        )?;
        let (config, _) = crate::merge::merge_layers(&[layer])?;
        let secrets = secrets_for(&config);
        let workload = config.workloads.get("pi").unwrap();
        let (ssh, _) = build_credential_grants(&config, "pi", workload, &secrets)?;
        assert_eq!(ssh.len(), 2);
        assert_eq!(
            ssh[0].on_violation,
            SecretViolationPolicy::Block,
            "absent entry policy inherits the material secret's ladder-resolved policy"
        );
        assert_eq!(
            ssh[1].on_violation,
            SecretViolationPolicy::BlockAndTerminate,
            "a declared entry policy wins over the material ladder"
        );
        Ok(())
    }

    #[test]
    fn unknown_grant_and_material_are_hard_errors() -> Result<()> {
        let layer = crate::merge::Layer::from_string(
            "base",
            &ssh_toml(
                "[secrets.DEPLOY_KEY]\n",
                "[workloads.pi.credentials]\nssh = [\"nope\"]\n",
            ),
        )?;
        let (config, _) = crate::merge::merge_layers(&[layer])?;
        let secrets = secrets_for(&config);
        let workload = config.workloads.get("pi").unwrap();
        let err = build_credential_grants(&config, "pi", workload, &secrets).unwrap_err();
        assert!(
            err.to_string().contains("nope") && err.to_string().contains("'pi'"),
            "unknown grant names workload and credential: {err}"
        );
        Ok(())
    }
}
