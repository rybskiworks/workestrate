//! Broker DLP pattern registration: compile broker-bound SSH credential
//! material into Raw match patterns for the relay scanner.
//!
//! One admitted credential emits exactly one [`BrokerPattern`] with
//! [`Decoder::Raw`]; the fork's brokerd derives the hex/base64 needles
//! itself at ingest. Short material is excluded with a compile-time
//! warning and a coverage note — never fail-closed, so one weak
//! credential cannot block the up. Grants sharing one material form a
//! group whose action is the strictest contributor (severity max,
//! audit/count union).
//!
//! The emission shape reuses the fork's typed `BrokerPatterns`: credential
//! ID, decoder, match bytes and coalesced action. These values contain raw
//! secrets and are only for the custody broker. Exclusion from a serialized
//! guest spec alone is insufficient: bootstrap also enters its target VM.
//! This module deliberately does not accept an ordinary workload builder.

use crate::config::SecretViolationPolicy;
use crate::microsandbox::plan::{CredentialBinding, CredentialsPlan, SshGrantPlan};
use crate::microsandbox::secrets::SecretDefinition;
use microsandbox_protocol::bootstrap::{BrokerPattern, BrokerPatterns};
use microsandbox_scan::{ActionSet, Decoder};
use std::collections::HashMap;

/// Minimum raw credential length kept for matching. Shorter material is
/// excluded at compile time with a warning: below 8 bytes literal matches
/// are too collision-prone for binary SSH streams.
pub const MIN_RAW_BYTES: usize = 8;

/// Why a credential contributed no pattern. Exclusions warn at compile
/// time and are reported back to the caller; they never fail closed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DlpExclusionReason {
    /// The resolved raw credential is shorter than the minimum match
    /// length (too collision-prone to enforce).
    TooShort { len: usize },
    /// No material bytes resolved for the grant's secret (absent from the
    /// decrypted map or whitespace-only). Key custody still fails closed
    /// separately; DLP alone never blocks the up.
    MaterialAbsent,
}

/// A credential excluded at compile time, with its grant name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DlpExclusion {
    pub credential_id: String,
    pub reason: DlpExclusionReason,
}

impl std::fmt::Display for DlpExclusionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { len } => write!(
                f,
                "resolved credential is {len} bytes, below the minimum match length"
            ),
            Self::MaterialAbsent => f.write_str("no material bytes resolved for the grant secret"),
        }
    }
}

impl std::fmt::Display for DlpExclusion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "credential '{}': {}", self.credential_id, self.reason)
    }
}

/// Map one grant's effective violation policy onto the coalesced DLP
/// action. Passthrough contributes no enforcement but still counts;
/// block closes silently, block-and-log closes with an audit record,
/// block-and-terminate tears down the relay session with an audit record.
pub fn policy_to_action(policy: SecretViolationPolicy) -> ActionSet {
    match policy {
        SecretViolationPolicy::Passthrough => ActionSet::passthrough(),
        SecretViolationPolicy::Block => ActionSet {
            enforce: Some(microsandbox_scan::Severity::Block),
            audit: false,
            count: true,
        },
        SecretViolationPolicy::BlockAndLog => ActionSet {
            enforce: Some(microsandbox_scan::Severity::BlockAndLog),
            audit: true,
            count: true,
        },
        SecretViolationPolicy::BlockAndTerminate => ActionSet {
            enforce: Some(microsandbox_scan::Severity::BlockAndTerminate),
            audit: true,
            count: true,
        },
    }
}

/// Compile-time warning for an excluded credential: grant name and reason
/// only, never bytes.
fn warn_excluded(credential_id: &str, reason: &DlpExclusionReason) {
    eprintln!("ssh-patterns: excluding DLP pattern for credential '{credential_id}': {reason}");
}

/// Emit the Raw pattern for one material group's grants: one pattern per
/// grant carrying the literal bytes and the group's coalesced action.
/// Missing or short material excludes the whole group with a warning and
/// a coverage note per grant — never fail-closed.
fn flush_material_group(
    members: &mut Vec<&SshGrantPlan>,
    material: &str,
    material_bytes: &HashMap<String, Vec<u8>>,
    action: ActionSet,
    patterns: &mut Vec<BrokerPattern>,
    excluded: &mut Vec<DlpExclusion>,
) {
    let Some(raw) = material_bytes.get(material) else {
        for grant in members.iter() {
            let reason = DlpExclusionReason::MaterialAbsent;
            warn_excluded(&grant.name, &reason);
            excluded.push(DlpExclusion {
                credential_id: grant.name.clone(),
                reason: reason.clone(),
            });
        }
        members.clear();
        return;
    };
    if raw.len() < MIN_RAW_BYTES {
        for grant in members.iter() {
            let reason = DlpExclusionReason::TooShort { len: raw.len() };
            warn_excluded(&grant.name, &reason);
            excluded.push(DlpExclusion {
                credential_id: grant.name.clone(),
                reason: reason.clone(),
            });
        }
        members.clear();
        return;
    }
    for grant in members.iter() {
        patterns.push(BrokerPattern {
            credential_id: grant.name.clone(),
            decoder: Decoder::Raw,
            bytes: raw.clone(),
            action,
        });
    }
    members.clear();
}

/// Compile broker-bound grants into Raw match patterns.
///
/// `material_bytes` maps the grant's material secret name to its resolved
/// raw bytes. Guest-bound grants are skipped silently (their sessions
/// relay direct, outside broker scanning); broker-bound grants without
/// material warn and contribute a coverage note instead of failing the up.
///
/// Per material group the action is the strictest contributor, so the
/// result is independent of grant order. Every admitted credential emits
/// exactly one Raw pattern; brokerd derives the encoded needles at ingest.
pub fn compile_ssh_patterns(
    grants: &[SshGrantPlan],
    material_bytes: &HashMap<String, Vec<u8>>,
) -> (BrokerPatterns, Vec<DlpExclusion>) {
    let mut brokered: Vec<&SshGrantPlan> = grants
        .iter()
        .filter(|g| g.binding == CredentialBinding::Broker)
        .collect();
    brokered.sort_by(|a, b| {
        a.material
            .cmp(&b.material)
            .then_with(|| a.name.cmp(&b.name))
    });

    let mut patterns = Vec::new();
    let mut excluded = Vec::new();
    let mut group_material = String::new();
    let mut group_action = ActionSet::passthrough();
    let mut group_members: Vec<&SshGrantPlan> = Vec::new();

    for grant in brokered {
        if group_members.is_empty() {
            group_material = grant.material.clone();
            group_action = policy_to_action(grant.on_violation);
        } else if grant.material != group_material {
            flush_material_group(
                &mut group_members,
                &group_material,
                material_bytes,
                group_action,
                &mut patterns,
                &mut excluded,
            );
            group_material = grant.material.clone();
            group_action = policy_to_action(grant.on_violation);
        } else {
            group_action = ActionSet::strictest(group_action, policy_to_action(grant.on_violation));
        }
        group_members.push(grant);
    }
    if !group_members.is_empty() {
        let material = group_material.clone();
        flush_material_group(
            &mut group_members,
            &material,
            material_bytes,
            group_action,
            &mut patterns,
            &mut excluded,
        );
    }

    (BrokerPatterns { patterns }, excluded)
}

/// Resolve broker-bound material for the custody broker's scanner.
///
/// Use the same secret-ID to source-variable mapping as sealed key custody.
/// An explicit mapping never falls back to the ID's value or process-wide
/// environment. No guest builder is accepted or mutated. The caller must
/// send the returned secret-bearing patterns only to the managed broker.
pub fn resolve_ssh_patterns(
    credentials: Option<&CredentialsPlan>,
    secrets: &HashMap<String, String>,
    definitions: &HashMap<String, SecretDefinition>,
) -> (BrokerPatterns, Vec<DlpExclusion>) {
    let Some(plan) = credentials else {
        return (BrokerPatterns { patterns: vec![] }, vec![]);
    };
    if !plan
        .ssh
        .iter()
        .any(|g| g.binding == CredentialBinding::Broker)
    {
        return (BrokerPatterns { patterns: vec![] }, vec![]);
    }
    let mut material_bytes: HashMap<String, Vec<u8>> = HashMap::new();
    for grant in &plan.ssh {
        if grant.binding != CredentialBinding::Broker {
            continue;
        }
        if material_bytes.contains_key(&grant.material) {
            continue;
        }
        let source = definitions
            .get(&grant.material)
            .map_or(grant.material.as_str(), |definition| {
                definition.source_env_var.as_str()
            });
        if let Some(value) = secrets.get(source)
            && !value.trim().is_empty()
        {
            material_bytes.insert(grant.material.clone(), value.as_bytes().to_vec());
        }
    }
    compile_ssh_patterns(&plan.ssh, &material_bytes)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::config::SecretViolationPolicy as Policy;
    use microsandbox_scan::Severity;

    fn grant(
        name: &str,
        material: &str,
        policy: Policy,
        binding: CredentialBinding,
    ) -> SshGrantPlan {
        SshGrantPlan {
            name: name.to_string(),
            material: material.to_string(),
            hosts: vec!["github.com".to_string()],
            users: vec!["git".to_string()],
            ports: vec![22],
            binding,
            on_violation: policy,
        }
    }

    fn broker_grant(name: &str, material: &str, policy: Policy) -> SshGrantPlan {
        grant(name, material, policy, CredentialBinding::Broker)
    }

    fn materials(pairs: &[(&str, &[u8])]) -> HashMap<String, Vec<u8>> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_vec()))
            .collect()
    }

    #[test]
    fn sub_eight_byte_material_is_excluded_with_coverage_note() {
        let grants = vec![broker_grant("deploy", "DEPLOY_KEY", Policy::Block)];
        let (patterns, excluded) =
            compile_ssh_patterns(&grants, &materials(&[("DEPLOY_KEY", b"1234567")]));
        assert!(patterns.patterns.is_empty());
        assert_eq!(excluded.len(), 1);
        assert_eq!(excluded[0].credential_id, "deploy");
        assert_eq!(excluded[0].reason, DlpExclusionReason::TooShort { len: 7 });
    }

    #[test]
    fn eight_bytes_emit_exactly_one_raw_pattern() {
        let grants = vec![broker_grant("deploy", "DEPLOY_KEY", Policy::Block)];
        let (patterns, excluded) =
            compile_ssh_patterns(&grants, &materials(&[("DEPLOY_KEY", b"12345678")]));
        assert!(excluded.is_empty());
        assert_eq!(patterns.patterns.len(), 1);
        let pattern = &patterns.patterns[0];
        assert_eq!(pattern.decoder, Decoder::Raw);
        assert_eq!(pattern.bytes, b"12345678");
        assert_eq!(pattern.credential_id, "deploy");
    }

    #[test]
    fn twelve_bytes_still_emit_exactly_one_raw_pattern() {
        let grants = vec![broker_grant("deploy", "DEPLOY_KEY", Policy::BlockAndLog)];
        let (patterns, excluded) =
            compile_ssh_patterns(&grants, &materials(&[("DEPLOY_KEY", b"123456789012")]));
        assert!(excluded.is_empty());
        assert_eq!(
            patterns.patterns.len(),
            1,
            "brokerd derives encoded needles at ingest; emission stays Raw-only"
        );
        let pattern = &patterns.patterns[0];
        assert_eq!(pattern.decoder, Decoder::Raw);
        assert_eq!(pattern.bytes, b"123456789012");
        assert_eq!(pattern.action.enforce, Some(Severity::BlockAndLog));
    }

    #[test]
    fn material_group_coalescing_is_order_independent() {
        let forward = vec![
            broker_grant("a", "SHARED", Policy::Block),
            broker_grant("b", "SHARED", Policy::BlockAndTerminate),
            broker_grant("c", "SHARED", Policy::Passthrough),
        ];
        let mut reverse = forward.clone();
        reverse.reverse();
        let map = materials(&[("SHARED", b"shared-credential-bytes")]);
        let (first, _) = compile_ssh_patterns(&forward, &map);
        let (second, _) = compile_ssh_patterns(&reverse, &map);
        assert_eq!(first.patterns.len(), 3);
        assert_eq!(first.patterns.len(), second.patterns.len());
        for pattern in &first.patterns {
            assert_eq!(
                pattern.action.enforce,
                Some(Severity::BlockAndTerminate),
                "terminate is strictest regardless of grant order"
            );
            assert!(pattern.action.audit);
            assert!(pattern.action.count);
            assert_eq!(pattern.decoder, Decoder::Raw);
        }
        let first_actions: Vec<ActionSet> = first.patterns.iter().map(|p| p.action).collect();
        let second_actions: Vec<ActionSet> = second.patterns.iter().map(|p| p.action).collect();
        assert_eq!(first_actions, second_actions);
    }

    #[test]
    fn passthrough_group_counts_without_enforcing() {
        let grants = vec![broker_grant("deploy", "DEPLOY_KEY", Policy::Passthrough)];
        let (patterns, _) =
            compile_ssh_patterns(&grants, &materials(&[("DEPLOY_KEY", b"counted-bytes-001")]));
        assert_eq!(patterns.patterns.len(), 1);
        for pattern in &patterns.patterns {
            assert_eq!(pattern.action, ActionSet::passthrough());
            assert_eq!(pattern.action.enforce, None);
            assert!(!pattern.action.audit);
            assert!(pattern.action.count);
        }
    }

    #[test]
    fn guest_bound_grants_are_skipped_silently() {
        let grants = vec![grant(
            "deploy",
            "DEPLOY_KEY",
            Policy::Block,
            CredentialBinding::Guest,
        )];
        let (patterns, excluded) = compile_ssh_patterns(
            &grants,
            &materials(&[("DEPLOY_KEY", b"guest-bound-bytes-001")]),
        );
        assert!(patterns.patterns.is_empty());
        assert!(excluded.is_empty());
    }

    #[test]
    fn missing_material_is_a_coverage_note_not_a_failure() {
        let grants = vec![
            broker_grant("missing", "ABSENT", Policy::Block),
            broker_grant("present", "HERE", Policy::Block),
        ];
        let (patterns, excluded) =
            compile_ssh_patterns(&grants, &materials(&[("HERE", b"present-bytes-00001")]));
        assert_eq!(excluded.len(), 1);
        assert_eq!(excluded[0].credential_id, "missing");
        assert_eq!(excluded[0].reason, DlpExclusionReason::MaterialAbsent);
        assert_eq!(patterns.patterns.len(), 1);
        assert_eq!(patterns.patterns[0].credential_id, "present");
    }

    #[test]
    fn emission_shape_round_trips_fork_wire() {
        let grants = vec![broker_grant(
            "deploy",
            "DEPLOY_KEY",
            Policy::BlockAndTerminate,
        )];
        let (patterns, _) = compile_ssh_patterns(
            &grants,
            &materials(&[("DEPLOY_KEY", b"round-trip-bytes-01")]),
        );
        let value = serde_json::to_value(&patterns).expect("patterns serialize");
        assert!(value.get("patterns").is_some());
        let back: BrokerPatterns = serde_json::from_value(value).expect("patterns deserialize");
        assert_eq!(back, patterns);
        let raw = &patterns.patterns[0];
        let wire = serde_json::to_value(raw).expect("pattern serializes");
        assert_eq!(
            wire.get("credential_id").and_then(|v| v.as_str()),
            Some("deploy")
        );
        assert_eq!(wire.get("decoder").and_then(|v| v.as_str()), Some("raw"));
    }

    #[test]
    fn debug_impls_never_carry_secret_text() {
        let grants = vec![broker_grant("deploy", "DEPLOY_KEY", Policy::Block)];
        let secret = b"super-secret-credential-value-001";
        let (patterns, _) = compile_ssh_patterns(&grants, &materials(&[("DEPLOY_KEY", secret)]));
        let rendered = format!("{patterns:?}");
        assert!(!rendered.contains("super-secret"));
        assert!(rendered.contains("deploy"));
        let rendered = format!("{:?}", patterns.patterns[0]);
        assert!(!rendered.contains("super-secret"));
    }

    #[test]
    fn resolve_without_grants_emits_no_broker_material() {
        let (patterns, exclusions) = resolve_ssh_patterns(None, &HashMap::new(), &HashMap::new());
        assert!(patterns.patterns.is_empty());
        assert!(exclusions.is_empty());
    }

    fn plan(ssh: Vec<SshGrantPlan>) -> CredentialsPlan {
        CredentialsPlan {
            ssh,
            signing: vec![],
            strict: true,
            strict_origin: None,
        }
    }

    fn mapped_definition(source: &str) -> SecretDefinition {
        SecretDefinition {
            source_env_var: source.to_owned(),
            allowed_hosts: vec![],
            required: true,
            placeholder: None,
            on_violation: Policy::Block,
        }
    }

    #[test]
    fn resolver_uses_the_custody_secret_mapping_without_fallback() {
        let plan = plan(vec![broker_grant("deploy", "KEY", Policy::Block)]);
        let secrets = HashMap::from([
            ("KEY".to_owned(), "wrong-material-sentinel".to_owned()),
            (
                "KEY_SOURCE".to_owned(),
                "broker-material-sentinel".to_owned(),
            ),
        ]);
        let definitions = HashMap::from([("KEY".to_owned(), mapped_definition("KEY_SOURCE"))]);
        let (patterns, exclusions) = resolve_ssh_patterns(Some(&plan), &secrets, &definitions);
        assert!(exclusions.is_empty());
        assert_eq!(patterns.patterns.len(), 1);
        assert_eq!(patterns.patterns[0].bytes, b"broker-material-sentinel");
        let rendered = format!("{patterns:?}");
        assert!(!rendered.contains("broker-material-sentinel"));
        assert!(!rendered.contains("wrong-material-sentinel"));

        let unmapped = HashMap::from([("KEY".to_owned(), "wrong-material-sentinel".to_owned())]);
        let (patterns, exclusions) = resolve_ssh_patterns(Some(&plan), &unmapped, &definitions);
        assert!(patterns.patterns.is_empty());
        assert_eq!(exclusions.len(), 1);
        assert_eq!(exclusions[0].reason, DlpExclusionReason::MaterialAbsent);
    }

    #[test]
    fn resolver_excludes_guest_material_even_in_mixed_plans() {
        let plan = plan(vec![
            broker_grant("deploy", "BROKER", Policy::Block),
            grant("guest", "GUEST", Policy::Block, CredentialBinding::Guest),
        ]);
        let secrets = HashMap::from([
            ("BROKER".to_owned(), "broker-material-sentinel".to_owned()),
            ("GUEST".to_owned(), "guest-material-sentinel".to_owned()),
        ]);
        let (patterns, exclusions) = resolve_ssh_patterns(Some(&plan), &secrets, &HashMap::new());
        assert!(exclusions.is_empty());
        assert_eq!(patterns.patterns.len(), 1);
        assert_eq!(patterns.patterns[0].credential_id, "deploy");
        assert_eq!(patterns.patterns[0].bytes, b"broker-material-sentinel");
    }

    #[test]
    fn ordinary_workload_construction_has_no_broker_payload_setters() {
        // This source guard complements broker transport tests; inspecting
        // SandboxSpec alone cannot detect secret-bearing bootstrap fields.
        let source = include_str!("../runtime/run.rs");
        for setter in [
            ".broker_key(",
            ".broker_upstream(",
            ".broker_patterns(",
            "apply_ssh_patterns(",
        ] {
            assert!(!source.contains(setter), "ordinary workload calls {setter}");
        }
    }
}
