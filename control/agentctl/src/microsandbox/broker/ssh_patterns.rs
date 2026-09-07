//! Broker DLP pattern registration: compile broker-bound SSH credential
//! material into literal match patterns for the relay scanner.
//!
//! Each broker-bound grant contributes its material's raw bytes plus the
//! text encodings a relayed session may carry them in (hex, standard
//! base64). Short material is excluded with a compile-time warning and a
//! coverage note — never fail-closed, so one weak credential cannot block
//! the up. Grants sharing one material form a group whose action is the
//! strictest contributor (severity max, audit/count union).
//!
//! The emission shape mirrors the fork's typed bootstrap (`BrokerPatterns`:
//! credential id, authored decoder, match bytes, coalesced action) so the
//! pin advance that vendors the scan/protocol crates can thread this value
//! straight into the builder. Until then the patterns are staged host-side:
//! they never enter the guest-visible spec.

use crate::config::SecretViolationPolicy;
use crate::microsandbox::plan::{CredentialBinding, CredentialsPlan, SshGrantPlan};
use std::collections::HashMap;

/// Truncated hash length for pattern ids, in bytes: enough to correlate
/// audit records, too short to brute-force content from.
pub const ID_BYTES: usize = 16;

/// Minimum raw credential length kept for matching. Shorter material is
/// excluded at compile time with a warning: below 8 bytes literal matches
/// are too collision-prone for binary SSH streams.
pub const MIN_RAW_BYTES: usize = 8;

/// Minimum raw length that derives hex needles.
pub const MIN_HEX_RAW_BYTES: usize = 12;

/// Minimum raw length that derives base64 needles.
pub const MIN_BASE64_RAW_BYTES: usize = 8;

/// Authored encoding of a pattern's bytes. Lowercase wire names match the
/// fork scan vocabulary where they overlap (`raw`, `base64`); `hex` is the
/// workestrate derived form the broker compiles the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DlpDecoder {
    Raw,
    Base64,
    Hex,
}

/// Enforcement strength for a DLP hit, weakest to strongest.
/// Declaration order is NOT the severity order; see
/// [`DlpSeverity::priority`].
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize,
)]
#[serde(rename_all = "kebab-case")]
pub enum DlpSeverity {
    #[default]
    BlockAndLog,
    Block,
    BlockAndTerminate,
}

/// Coalesced action for a material group. Reduction rule: the maximum
/// [`DlpSeverity`] over the contributors wins, and `audit`/`count` are the
/// OR-union. A passthrough policy contributes no enforcement but still
/// counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Deserialize, serde::Serialize)]
pub struct DlpAction {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enforce: Option<DlpSeverity>,
    #[serde(default)]
    pub audit: bool,
    #[serde(default)]
    pub count: bool,
}

/// Truncated SHA-256 over (credential id, decoder, bytes). Safe to log: a
/// hash over the pattern, never content.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PatternId([u8; ID_BYTES]);

/// One DLP pattern: audit identity plus match bytes and the coalesced
/// action contributed on a hit. The `Debug` impl redacts `bytes`; only the
/// id, credential name, decoder, action, and byte length render.
#[derive(Clone, PartialEq, Eq)]
pub struct SshPattern {
    pub credential_id: String,
    pub decoder: DlpDecoder,
    pub bytes: Vec<u8>,
    pub action: DlpAction,
}

/// The host-side emission shape: every compiled pattern for one launch.
/// Additive wire shape mirroring the fork bootstrap grouping; an empty
/// value means no DLP scanning and relayed sessions pass through unchanged.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct SshPatterns {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub patterns: Vec<SshPattern>,
}

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

impl DlpDecoder {
    /// Stable tag byte mixed into the pattern-id hash.
    fn tag(self) -> u8 {
        match self {
            Self::Raw => 0,
            Self::Base64 => 1,
            Self::Hex => 2,
        }
    }
}

impl DlpSeverity {
    /// Severity rank: higher wins the strictest reduction. Passthrough
    /// (no enforcement) ranks 0 and never wins over an enforcer.
    pub fn priority(self) -> u8 {
        match self {
            Self::BlockAndLog => 1,
            Self::Block => 2,
            Self::BlockAndTerminate => 3,
        }
    }
}

impl DlpAction {
    /// The passthrough action: forward unchanged, still counted.
    pub fn passthrough() -> Self {
        Self {
            enforce: None,
            audit: false,
            count: true,
        }
    }

    /// Reduce two contributing actions: maximum severity wins, and
    /// `audit`/`count` are the OR-union.
    pub fn strictest(first: Self, second: Self) -> Self {
        let enforce = match (first.enforce, second.enforce) {
            (None, None) => None,
            (Some(action), None) | (None, Some(action)) => Some(action),
            (Some(a), Some(b)) => Some(if a.priority() >= b.priority() { a } else { b }),
        };
        Self {
            enforce,
            audit: first.audit || second.audit,
            count: first.count || second.count,
        }
    }
}

impl Default for DlpAction {
    /// The wire default is passthrough: forward unchanged, still counted.
    fn default() -> Self {
        Self::passthrough()
    }
}

impl std::fmt::Display for DlpSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::BlockAndLog => "block-and-log",
            Self::Block => "block",
            Self::BlockAndTerminate => "block-and-terminate",
        };
        f.write_str(value)
    }
}

impl std::fmt::Display for DlpAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.enforce {
            Some(severity) => write!(f, "{severity}"),
            None => f.write_str("passthrough"),
        }
    }
}

impl PatternId {
    /// Derive the pattern id over length-prefixed fields, so
    /// concatenations cannot collide across field boundaries.
    pub fn derive(credential_id: &str, decoder: DlpDecoder, bytes: &[u8]) -> Self {
        use sha2::Digest;
        let mut hash = sha2::Sha256::new();
        hash.update((credential_id.len() as u64).to_be_bytes());
        hash.update(credential_id.as_bytes());
        hash.update([decoder.tag()]);
        hash.update((bytes.len() as u64).to_be_bytes());
        hash.update(bytes);
        let digest = hash.finalize();
        let mut id = [0u8; ID_BYTES];
        id.copy_from_slice(&digest[..ID_BYTES]);
        Self(id)
    }

    /// Hex rendering for audit records and logs.
    pub fn to_hex(self) -> String {
        hex::encode(self.0)
    }
}

impl std::fmt::Display for PatternId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl std::fmt::Debug for PatternId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("PatternId").field(&self.to_hex()).finish()
    }
}

impl SshPattern {
    /// Pattern identity (hash, safe to log).
    pub fn id(&self) -> PatternId {
        PatternId::derive(&self.credential_id, self.decoder, &self.bytes)
    }
}

impl std::fmt::Debug for SshPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SshPattern")
            .field("id", &self.id())
            .field("credential_id", &self.credential_id)
            .field("decoder", &self.decoder)
            .field("action", &self.action)
            .field("bytes_len", &self.bytes.len())
            .finish_non_exhaustive()
    }
}

impl serde::Serialize for SshPattern {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("SshPattern", 4)?;
        state.serialize_field("credential_id", &self.credential_id)?;
        state.serialize_field("decoder", &self.decoder)?;
        state.serialize_field("bytes", &serde_bytes::ByteBuf::from(self.bytes.clone()))?;
        state.serialize_field("action", &self.action)?;
        state.end()
    }
}

impl<'de> serde::Deserialize<'de> for SshPattern {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(serde::Deserialize)]
        struct Wire {
            credential_id: String,
            decoder: DlpDecoder,
            #[serde(with = "serde_bytes")]
            bytes: Vec<u8>,
            #[serde(default)]
            action: DlpAction,
        }
        let wire = Wire::deserialize(deserializer)?;
        Ok(Self {
            credential_id: wire.credential_id,
            decoder: wire.decoder,
            bytes: wire.bytes,
            action: wire.action,
        })
    }
}

impl SshPatterns {
    /// Number of compiled patterns.
    pub fn len(&self) -> usize {
        self.patterns.len()
    }

    /// Whether no pattern was admitted (relay passes through unchanged).
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }
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
pub fn policy_to_action(policy: SecretViolationPolicy) -> DlpAction {
    match policy {
        SecretViolationPolicy::Passthrough => DlpAction::passthrough(),
        SecretViolationPolicy::Block => DlpAction {
            enforce: Some(DlpSeverity::Block),
            audit: false,
            count: true,
        },
        SecretViolationPolicy::BlockAndLog => DlpAction {
            enforce: Some(DlpSeverity::BlockAndLog),
            audit: true,
            count: true,
        },
        SecretViolationPolicy::BlockAndTerminate => DlpAction {
            enforce: Some(DlpSeverity::BlockAndTerminate),
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

/// Emit every derived form for one material group's grants: the literal
/// bytes always, hex (lower- and uppercase) from 12 bytes up, standard
/// base64 (padded, plus the unpadded form when it differs) from 8 bytes
/// up. Missing or short material excludes the whole group with a warning
/// and a coverage note per grant — never fail-closed.
fn flush_material_group(
    members: &mut Vec<&SshGrantPlan>,
    material: &str,
    material_bytes: &HashMap<String, Vec<u8>>,
    action: DlpAction,
    patterns: &mut Vec<SshPattern>,
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
        patterns.push(SshPattern {
            credential_id: grant.name.clone(),
            decoder: DlpDecoder::Raw,
            bytes: raw.clone(),
            action,
        });
        if raw.len() >= MIN_HEX_RAW_BYTES {
            let lower = hex::encode(raw.as_slice()).into_bytes();
            let upper = lower.to_ascii_uppercase();
            patterns.push(SshPattern {
                credential_id: grant.name.clone(),
                decoder: DlpDecoder::Hex,
                bytes: lower,
                action,
            });
            patterns.push(SshPattern {
                credential_id: grant.name.clone(),
                decoder: DlpDecoder::Hex,
                bytes: upper,
                action,
            });
        }
        if raw.len() >= MIN_BASE64_RAW_BYTES {
            use base64::Engine;
            let padded = base64::engine::general_purpose::STANDARD.encode(raw.as_slice());
            let pad = padded
                .as_bytes()
                .iter()
                .rev()
                .take_while(|b| **b == b'=')
                .count();
            patterns.push(SshPattern {
                credential_id: grant.name.clone(),
                decoder: DlpDecoder::Base64,
                bytes: padded.as_bytes().to_vec(),
                action,
            });
            if pad > 0 {
                let unpadded = padded.as_bytes()[..padded.len() - pad].to_vec();
                patterns.push(SshPattern {
                    credential_id: grant.name.clone(),
                    decoder: DlpDecoder::Base64,
                    bytes: unpadded,
                    action,
                });
            }
        }
    }
    members.clear();
}

/// Compile broker-bound grants into match patterns.
///
/// `material_bytes` maps the grant's material secret name to its resolved
/// raw bytes. Guest-bound grants are skipped silently (their sessions
/// relay direct, outside broker scanning); broker-bound grants without
/// material warn and contribute a coverage note instead of failing the up.
///
/// Per material group the action is the strictest contributor, so the
/// result is independent of grant order. Every admitted raw credential
/// emits its literal bytes; hex (lower- and uppercase) derives from 12
/// bytes up and standard base64 (padded, plus the unpadded form when it
/// differs) from 8 bytes up.
pub fn compile_ssh_patterns(
    grants: &[SshGrantPlan],
    material_bytes: &HashMap<String, Vec<u8>>,
) -> (SshPatterns, Vec<DlpExclusion>) {
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
    let mut group_action = DlpAction::passthrough();
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
            group_action = DlpAction::strictest(group_action, policy_to_action(grant.on_violation));
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

    (SshPatterns { patterns }, excluded)
}

/// Host-side builder setter for the compiled DLP patterns, mirroring the
/// `ssh_broker_endpoint` precedent: the patterns never enter the
/// guest-visible spec. The vendored pin predates the fork's builder
/// threading, so this stages the value host-side (coverage note on stderr)
/// and returns the builder unchanged; the pin advance that vendors the
/// scan/protocol crates delegates this call to the fork setter with no
/// call-site change.
pub trait SshPatternsExt {
    fn ssh_patterns(self, patterns: SshPatterns) -> Self;
}

impl SshPatternsExt for microsandbox::sandbox::SandboxBuilder {
    fn ssh_patterns(self, patterns: SshPatterns) -> Self {
        if patterns.is_empty() {
            return self;
        }
        let mut credentials: Vec<&str> = patterns
            .patterns
            .iter()
            .map(|p| p.credential_id.as_str())
            .collect();
        credentials.sort_unstable();
        credentials.dedup();
        eprintln!(
            "ssh-patterns: staging {} DLP pattern(s) for credential(s) {}",
            patterns.len(),
            credentials.join(",")
        );
        self
    }
}

/// Compile a workload's broker-bound SSH material and stage it on the
/// builder. Grant-less, guest-only, and fully-excluded plans leave the
/// builder untouched, preserving pass-through relay behavior.
///
/// Material resolves by secret name against the decrypted map; a custom
/// `env_var` on the material secret is not followed here (that grant
/// warns as absent while key custody resolves it through the definitions
/// and still fails closed on its own path).
pub fn apply_ssh_patterns(
    builder: microsandbox::sandbox::SandboxBuilder,
    credentials: Option<&CredentialsPlan>,
    secrets: &HashMap<String, String>,
) -> microsandbox::sandbox::SandboxBuilder {
    let Some(plan) = credentials else {
        return builder;
    };
    if !plan
        .ssh
        .iter()
        .any(|g| g.binding == CredentialBinding::Broker)
    {
        return builder;
    }
    let mut material_bytes: HashMap<String, Vec<u8>> = HashMap::new();
    for grant in &plan.ssh {
        if grant.binding != CredentialBinding::Broker {
            continue;
        }
        if material_bytes.contains_key(&grant.material) {
            continue;
        }
        if let Some(value) = secrets.get(&grant.material)
            && !value.trim().is_empty()
        {
            material_bytes.insert(grant.material.clone(), value.as_bytes().to_vec());
        }
    }
    let (patterns, _excluded) = compile_ssh_patterns(&plan.ssh, &material_bytes);
    builder.ssh_patterns(patterns)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::config::SecretViolationPolicy as Policy;

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
        assert!(patterns.is_empty());
        assert_eq!(excluded.len(), 1);
        assert_eq!(excluded[0].credential_id, "deploy");
        assert_eq!(excluded[0].reason, DlpExclusionReason::TooShort { len: 7 });
    }

    #[test]
    fn eight_bytes_admit_raw_and_base64_without_hex() {
        let grants = vec![broker_grant("deploy", "DEPLOY_KEY", Policy::Block)];
        let (patterns, excluded) =
            compile_ssh_patterns(&grants, &materials(&[("DEPLOY_KEY", b"12345678")]));
        assert!(excluded.is_empty());
        let decoders: Vec<DlpDecoder> = patterns.patterns.iter().map(|p| p.decoder).collect();
        assert!(decoders.contains(&DlpDecoder::Raw));
        assert!(decoders.contains(&DlpDecoder::Base64));
        assert!(!decoders.contains(&DlpDecoder::Hex));
        let raws: Vec<&SshPattern> = patterns
            .patterns
            .iter()
            .filter(|p| p.decoder == DlpDecoder::Raw)
            .collect();
        assert_eq!(raws.len(), 1);
        assert_eq!(raws[0].bytes, b"12345678");
    }

    #[test]
    fn twelve_bytes_derive_hex_and_base64_forms() {
        let grants = vec![broker_grant("deploy", "DEPLOY_KEY", Policy::BlockAndLog)];
        let (patterns, excluded) =
            compile_ssh_patterns(&grants, &materials(&[("DEPLOY_KEY", b"123456789012")]));
        assert!(excluded.is_empty());
        let hexes: Vec<&SshPattern> = patterns
            .patterns
            .iter()
            .filter(|p| p.decoder == DlpDecoder::Hex)
            .collect();
        assert_eq!(hexes.len(), 2);
        assert_eq!(hexes[0].bytes, b"313233343536373839303132");
        assert_eq!(
            hexes[1].bytes,
            b"313233343536373839303132".to_ascii_uppercase()
        );
        let b64: Vec<&SshPattern> = patterns
            .patterns
            .iter()
            .filter(|p| p.decoder == DlpDecoder::Base64)
            .collect();
        assert_eq!(b64.len(), 1, "12 bytes encode with no padding");
        assert_eq!(b64[0].bytes, b"MTIzNDU2Nzg5MDEy");
        assert!(
            b64.iter()
                .all(|p| p.action.enforce == Some(DlpSeverity::BlockAndLog)),
            "every derived form carries the coalesced action"
        );
    }

    #[test]
    fn base64_without_padding_emits_a_single_form() {
        let grants = vec![broker_grant("deploy", "DEPLOY_KEY", Policy::Block)];
        let (patterns, _) =
            compile_ssh_patterns(&grants, &materials(&[("DEPLOY_KEY", b"123456789012")]));
        let b64: Vec<&SshPattern> = patterns
            .patterns
            .iter()
            .filter(|p| p.decoder == DlpDecoder::Base64)
            .collect();
        assert_eq!(b64.len(), 1, "12 bytes encode with no padding");
        let (padded, _) = compile_ssh_patterns(&grants, &materials(&[("DEPLOY_KEY", b"12345678")]));
        let b64: Vec<&SshPattern> = padded
            .patterns
            .iter()
            .filter(|p| p.decoder == DlpDecoder::Base64)
            .collect();
        assert_eq!(b64.len(), 2);
        assert_eq!(b64[0].bytes, b"MTIzNDU2Nzg=");
        assert_eq!(b64[1].bytes, b"MTIzNDU2Nzg");
    }

    #[test]
    fn pattern_id_binds_credential_decoder_and_bytes() {
        let bytes = b"credential-bytes-001";
        let raw = PatternId::derive("cred-a", DlpDecoder::Raw, bytes);
        assert_ne!(raw, PatternId::derive("cred-b", DlpDecoder::Raw, bytes));
        assert_ne!(raw, PatternId::derive("cred-a", DlpDecoder::Base64, bytes));
        assert_ne!(raw, PatternId::derive("cred-a", DlpDecoder::Hex, bytes));
        assert_ne!(
            raw,
            PatternId::derive("cred-a", DlpDecoder::Raw, b"credential-bytes-002")
        );
        assert_eq!(raw, PatternId::derive("cred-a", DlpDecoder::Raw, bytes));
        assert_eq!(raw.to_hex().len(), ID_BYTES * 2);
    }

    #[test]
    fn pattern_id_uses_length_prefixing() {
        let left = PatternId::derive("ab", DlpDecoder::Raw, b"c");
        let right = PatternId::derive("a", DlpDecoder::Raw, b"bc");
        assert_ne!(left, right);
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
        assert!(!first.is_empty());
        assert_eq!(first.patterns.len(), second.patterns.len());
        for pattern in &first.patterns {
            assert_eq!(
                pattern.action.enforce,
                Some(DlpSeverity::BlockAndTerminate),
                "terminate is strictest regardless of grant order"
            );
            assert!(pattern.action.audit);
            assert!(pattern.action.count);
        }
        let first_actions: Vec<DlpAction> = first.patterns.iter().map(|p| p.action).collect();
        let second_actions: Vec<DlpAction> = second.patterns.iter().map(|p| p.action).collect();
        assert_eq!(first_actions, second_actions);
    }

    #[test]
    fn passthrough_group_counts_without_enforcing() {
        let grants = vec![broker_grant("deploy", "DEPLOY_KEY", Policy::Passthrough)];
        let (patterns, _) =
            compile_ssh_patterns(&grants, &materials(&[("DEPLOY_KEY", b"counted-bytes-001")]));
        assert!(!patterns.is_empty());
        for pattern in &patterns.patterns {
            assert_eq!(pattern.action, DlpAction::passthrough());
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
        assert!(patterns.is_empty());
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
        assert!(
            patterns
                .patterns
                .iter()
                .all(|p| p.credential_id == "present")
        );
    }

    #[test]
    fn emission_shape_round_trips_serde() {
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
        let back: SshPatterns = serde_json::from_value(value).expect("patterns deserialize");
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
    fn debug_impls_never_carry_secret_bytes() {
        let grants = vec![broker_grant("deploy", "DEPLOY_KEY", Policy::Block)];
        let secret = b"super-secret-credential-value-001";
        let (patterns, _) = compile_ssh_patterns(&grants, &materials(&[("DEPLOY_KEY", secret)]));
        let rendered = format!("{patterns:?}");
        assert!(!rendered.contains("super-secret"));
        assert!(rendered.contains("deploy"));
        let rendered = format!("{:?}", patterns.patterns[0]);
        assert!(!rendered.contains("super-secret"));
        assert!(rendered.contains(&patterns.patterns[0].id().to_hex()));
    }

    #[test]
    fn ssh_patterns_setter_leaves_grant_less_builder_untouched() {
        let builder = microsandbox::Sandbox::builder("ssh-patterns-none");
        let kept = builder.ssh_patterns(SshPatterns::default());
        assert!(kept.spec().network.ssh.is_none());
    }
}
