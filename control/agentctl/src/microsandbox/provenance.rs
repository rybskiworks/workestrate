//! Provenance stamps (ADR 0032 §Provenance stamps): content hashes over the
//! RUNTIME-RELEVANT build inputs of a sandbox plan, recorded on registry
//! records so staleness is visible (`ps`) and dispositions can react
//! (ADR 0030 V-addendum §V4 `on_skew`).
//!
//! # The config hash (pinned canonical serialization)
//!
//! [`config_hash_of_plan`] hashes a CANONICAL labeled serialization of the
//! plan with FNV-1a 64-bit ([`fnv1a64`], the per-dir shorthash precedent —
//! hand-rolled, no new crate dependency, nix vendor surface unchanged) and
//! renders the digest as 16-char lowercase hex. Separators: `\x1f`
//! (unit-sep) within a field group, `\x1e` (record-sep) between groups;
//! every group carries its label. Field order is PINNED (changing it
//! changes every hash):
//!
//! 1. `image` — `plan.image`, the RESOLVED tag/ref string (image out-path
//!    identity IS in the ADR hash scope)
//! 2. `workdir`
//! 3. `command` — args in declared order
//! 4. `cpus`, `memory_mib` (resources)
//! 5. `env` — pairs SORTED by name, `NAME=value`; `plan.env` is the
//!    effective post-merge view including baked env, hashed exactly as it
//!    carries it
//! 6. `secret_names` — SORTED secret NAMES only, never values
//! 7. `ports` — GUEST port + optional name ONLY, DECLARED order.
//!    HOST PORTS ARE EXCLUDED (PINNED): they are allocation-dependent
//!    (`--port-auto` probes, increment chains, per-slot binds) and must not
//!    churn the hash; staleness keys on build inputs, not on which host port
//!    the allocator happened to draw.
//! 8. `mounts` — per mount in DECLARED order: guest, host AS PLANNED (post
//!    instance-state scoping — NOT the FS-resolved root, so the hash stays
//!    computable without filesystem I/O and parent and child compute
//!    identically), the mount mode, plus the mount's policy fragment
//!    canonically (sorted read/write entries with their final flags) when
//!    present
//! 9. `network` — NetworkPlan canonically: the egress default (`dd=1`/`dd=0`
//!    — kept byte-identical to the retired `default_deny` bool so existing
//!    instance records do not drift) + the ingress default (`id=1`/`id=0`,
//!    appended when per-direction ingress defaults landed — a one-time hash
//!    change vs the pre-ingress format) + egress / deny / ingress rules
//!    SORTED by their canonical encoding (rule sets are semantically
//!    unordered; domain lists inside a rule are sorted too)
//!
//! # Explicitly NOT hashed (each would churn on non-runtime edits)
//!
//! - **host ports and bind IPs** — allocation-dependent (see ports above);
//! - **`policy_file` PATHS** — loader-relative tokens set late at boot;
//!   the mount's policy CONTENT rides the fragment in `mounts`;
//! - **seed_files declarations + content** — a `--reseed` concern, never a
//!   silent staleness trigger (ADR 0032 A3 resolution): seeds refresh via
//!   the explicit `--reseed` flow, and folding them into identity would
//!   stale running instances on template edits without any runtime-input
//!   change. The mount a seed lands IN remains hashed via `mounts`;
//! - **`created_at`**, instance/slot/context names (`plan.name`),
//!   namespace — identity/registry metadata, not build inputs;
//! - **`instance_policy`** — orchestration policy (conflict chain, port
//!   strategy, on_skew), not a sandbox build input;
//! - **`virtualization`** (ADR 0036 plan provenance) — orchestration policy
//!   (nested ask + degraded/frozen state), not a sandbox build input; the
//!   machine-readable record rides the plan JSON, and `up` re-resolves at
//!   gate time, so no record write is needed;
//! - **env metadata beyond `NAME=value`** (`is_secret`,
//!   `reject_placeholder`, `injected_by`, `injected_port`) and **secret
//!   metadata beyond names** (`allowed_hosts`, `required`,
//!   `reject_placeholder`) — pinned name/value-only scope;
//! - **egress `derived_from`** — depends_on display provenance; the RULES
//!   themselves are hashed;
//! - **comments / docs / formatting / unrelated-workload edits** — cannot
//!   reach a plan, so they cannot churn the hash by construction.
//!
//! Unknown-version posture: records created before this landed carry no
//! stamps (`None`); they parse fine and are NEVER auto-stale and never
//! trigger replace (ADR 0030 §V4 wiring pins this).
//!
//! NOTE: `crate::microsandbox::slots` carries its own private
//! `fnv1a64_hex8` (first-8-hex truncation for instance ids). It predates
//! this module and renders a DIFFERENT output shape; unifying it here would
//! churn a landed module for no behavioral gain, so both exist and any NEW
//! caller should use [`fnv1a64`].

use crate::microsandbox::plan::{EgressTarget, NetworkPlan, SandboxPlan};
use crate::mount_policy::{AxisFragment, MountsFragment, PolicyValue};

/// How many hex chars of a FULL stored hash the human surfaces show
/// (ADR 0032 §Provenance stamps: `stale (config a1b2 → current d4e5)`).
/// Stored hashes stay full-length (16 hex).
pub const PROVENANCE_DISPLAY_LEN: usize = 4;

/// FNV-1a 64-bit hash of `bytes` (hand-rolled — see the module doc).
pub fn fnv1a64(data: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET_BASIS;
    for &b in data {
        hash ^= u64::from(b);
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

/// The display form of a full provenance hash: the FIRST
/// [`PROVENANCE_DISPLAY_LEN`] hex chars (renderers truncate; storage does
/// not).
pub fn short_hash(hash: &str) -> &str {
    &hash[..hash.len().min(PROVENANCE_DISPLAY_LEN)]
}

/// The image out-path hash for a RESOLVED tag (ADR 0032 §Provenance
/// stamps): the sha segment of a computed-shape `<name>:<sha>` /
/// `<name>:<ctx>.<sha>` tag (ADR 0032 §Image tags, AMENDED 2026-08-28 —
/// dot separator), `None` otherwise (registry refs and legacy
/// declared tags have no content hash — staleness then rides the config
/// hash alone). Shape-checks via the GC's parser; NEVER re-hashes the
/// image.
pub fn image_out_hash_from_tag(tag: &str) -> Option<String> {
    crate::images::gc::split_computed_tag(tag)?;
    // split_computed_tag validated the shape: the sha follows the LAST
    // `.` (ctx form) or the LAST `:` (ctx-less form); both segments are
    // dot/colon-free, so splitting on either is unambiguous.
    tag.rsplit([':', '.']).next().map(str::to_string)
}

/// The config hash over the runtime-relevant view of `plan` (16-char
/// lowercase hex). See the module doc for the pinned serialization.
pub fn config_hash_of_plan(plan: &SandboxPlan) -> String {
    format!("{:016x}", fnv1a64(canonical_plan_bytes(plan).as_bytes()))
}

// ---------------------------------------------------------------------------
// Canonical serialization (private; the shapes are pinned in the module doc)
// ---------------------------------------------------------------------------

const UNIT: char = '\u{1f}';
const REC: char = '\u{1e}';

/// Injective Option encoding: `Some(v)` → `1:v`, `None` → `0:` (a bare
/// value could collide with an absent field).
fn push_opt(out: &mut String, value: Option<&str>) {
    match value {
        Some(v) => {
            out.push_str("1:");
            out.push_str(v);
        }
        None => out.push_str("0:"),
    }
}

fn push_opt_number<T: std::fmt::Display>(out: &mut String, value: Option<T>) {
    match value {
        Some(v) => {
            out.push_str("1:");
            out.push_str(&v.to_string());
        }
        None => out.push_str("0:"),
    }
}

fn canonical_plan_bytes(plan: &SandboxPlan) -> String {
    let mut s = String::new();

    // 1. image (resolved tag/ref string)
    s.push_str("image");
    s.push(UNIT);
    push_opt(&mut s, plan.image.as_deref());
    s.push(REC);

    // 2. workdir
    s.push_str("workdir");
    s.push(UNIT);
    push_opt(&mut s, plan.workdir.as_deref());
    s.push(REC);

    // 3. command (declared order)
    s.push_str("command");
    for arg in &plan.command {
        s.push(UNIT);
        s.push_str(arg);
    }
    s.push(REC);

    // 4. resources
    s.push_str("cpus");
    s.push(UNIT);
    push_opt_number(&mut s, plan.cpus);
    s.push(REC);
    s.push_str("memory_mib");
    s.push(UNIT);
    push_opt_number(&mut s, plan.memory_mib);
    s.push(REC);

    // 5. env — SORTED by name, NAME=value (the effective post-merge view).
    let mut env: Vec<(&str, &str)> = plan
        .env
        .iter()
        .map(|e| (e.name.as_str(), e.value.as_str()))
        .collect();
    env.sort_unstable();
    s.push_str("env");
    for (name, value) in env {
        s.push(UNIT);
        s.push_str(name);
        s.push('=');
        s.push_str(value);
    }
    s.push(REC);

    // 6. secret NAMES only, sorted — never values.
    let mut secrets: Vec<&str> = plan.secret_env.iter().map(|h| h.name.as_str()).collect();
    secrets.sort_unstable();
    s.push_str("secret_names");
    for name in secrets {
        s.push(UNIT);
        s.push_str(name);
    }
    s.push(REC);

    // 7. ports — guest + optional name ONLY, declared order. Host ports and
    //    bind IPs are allocation-dependent and PINNED OUT of the hash.
    s.push_str("ports");
    for p in &plan.ports {
        s.push(UNIT);
        match &p.name {
            Some(name) => {
                s.push_str("1:");
                s.push_str(&p.guest.to_string());
                s.push(':');
                s.push_str(name);
            }
            None => {
                s.push_str("0:");
                s.push_str(&p.guest.to_string());
            }
        }
    }
    s.push(REC);

    // 8. mounts — declared order: guest, host AS PLANNED, mode, policy
    //    fragment canonically when present. policy_file PATHS excluded.
    s.push_str("mounts");
    for m in &plan.mounts {
        s.push(UNIT);
        s.push_str(&m.guest);
        s.push(UNIT);
        s.push_str(&m.host);
        s.push(UNIT);
        s.push_str(m.mode.to_string().as_str());
        s.push(UNIT);
        match &m.policy {
            None => s.push_str("0:"),
            Some(fragment) => {
                s.push_str("1:");
                s.push_str(&mounts_fragment_canonical(fragment));
            }
        }
    }
    s.push(REC);

    // 9. network — canonical, sorted.
    s.push_str("network");
    s.push(UNIT);
    s.push_str(&network_canonical(&plan.network));

    s
}

fn mounts_fragment_canonical(f: &MountsFragment) -> String {
    let mut s = String::new();
    match &f.read {
        None => s.push_str("0:"),
        Some(axis) => {
            s.push_str("1:");
            axis_canonical(&mut s, axis);
        }
    }
    s.push(UNIT);
    match &f.write {
        None => s.push_str("0:"),
        Some(axis) => {
            s.push_str("1:");
            axis_canonical(&mut s, axis);
        }
    }
    s.push(UNIT);
    match &f.case_sensitivity {
        None => s.push_str("0:"),
        Some(c) => {
            s.push_str("1:");
            s.push_str(c);
        }
    }
    s
}

/// One policy axis: deny entries then allow entries, each list SORTED so
/// declaration order cannot churn the hash (per-scope compile semantics are
/// deny-then-allow regardless of entry order within a list).
fn axis_canonical(out: &mut String, axis: &AxisFragment) {
    out.push('d');
    let mut deny: Vec<String> = axis.deny.iter().map(policy_value_canonical).collect();
    deny.sort();
    for d in deny {
        out.push(UNIT);
        out.push_str(&d);
    }
    out.push(UNIT);
    out.push('a');
    let mut allow: Vec<String> = axis.allow.iter().map(policy_value_canonical).collect();
    allow.sort();
    for a in allow {
        out.push(UNIT);
        out.push_str(&a);
    }
}

fn policy_value_canonical(v: &PolicyValue<String>) -> String {
    format!("{}:{}", if v.terminal { "1" } else { "0" }, v.value)
}

fn network_canonical(n: &NetworkPlan) -> String {
    // `dd=` is the EGRESS default — kept byte-identical to the retired
    // `default_deny` bool. `id=` is the INGRESS default, appended when the
    // per-direction `network.defaults.ingress` key landed: two configs
    // differing only in ingress default MUST hash differently (accepted
    // one-time instance drift vs the pre-ingress format after upgrade).
    let mut s = String::from(if n.egress_default_deny {
        "dd=1"
    } else {
        "dd=0"
    });
    s.push_str(if n.ingress_default_deny {
        " id=1"
    } else {
        " id=0"
    });

    let mut egress: Vec<String> = n.egress_rules.iter().map(egress_rule_canonical).collect();
    egress.sort();
    s.push(UNIT);
    s.push('e');
    for r in egress {
        s.push(UNIT);
        s.push_str(&r);
    }

    // FIX2/FIX3: deny canonical includes port/protocol so port-scoped vs
    // port-agnostic denies hash differently; without port stays legacy string.
    let mut denies: Vec<String> = n
        .deny_rules
        .iter()
        .map(|d| {
            let mut s = d.domain_suffix.clone();
            if let Some(p) = d.port {
                s.push(':');
                s.push_str(&p.to_string());
            }
            if let Some(proto) = d.protocol {
                s.push(':');
                s.push_str(&proto.to_string());
            }
            s
        })
        .collect();
    denies.sort();
    s.push(UNIT);
    s.push_str("dn");
    for d in denies {
        s.push(UNIT);
        s.push_str(&d);
    }

    let mut ingress: Vec<String> = n
        .ingress_rules
        .iter()
        .map(|r| format!("{}:{}:{}", r.protocol, r.port, r.scope))
        .collect();
    ingress.sort();
    s.push(UNIT);
    s.push('i');
    for r in ingress {
        s.push(UNIT);
        s.push_str(&r);
    }
    s
}

fn egress_rule_canonical(rule: &crate::microsandbox::plan::EgressRule) -> String {
    // derived_from is display provenance (which dependency produced the
    // rule), not a runtime input — excluded; the RULE itself is hashed.
    format!(
        "{}:{}:{}",
        rule.protocol,
        rule.port,
        match &rule.target {
            EgressTarget::Host => "host".to_string(),
            EgressTarget::Domains(domains) => {
                let mut sorted: Vec<&str> = domains.iter().map(String::as_str).collect();
                sorted.sort_unstable();
                format!("dom({})", sorted.join(","))
            }
        }
    )
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
    use crate::microsandbox::plan::{
        DenyDomainRule, EgressRule, EnvVar, HostBoundSecret, IngressRule, MountMode, MountPlan,
        PortMapping, Protocol, Scope,
    };
    use crate::mount_policy::PolicyValue;

    fn empty_plan() -> SandboxPlan {
        SandboxPlan {
            name: "test".to_string(),
            image: None,
            workdir: None,
            command: Vec::new(),
            cpus: None,
            memory_mib: None,
            env: Vec::new(),
            secret_env: Vec::new(),
            ports: Vec::new(),
            mounts: Vec::new(),
            network: NetworkPlan {
                egress_default_deny: false,
                ingress_default_deny: false,
                egress_rules: Vec::new(),
                deny_rules: Vec::new(),
                ingress_rules: Vec::new(),
                egress_defaults_seal: None,
                ingress_defaults_seal: None,
            },
            instance_policy: None,
            virtualization: None,
        }
    }

    fn mount(host: &str, guest: &str, mode: MountMode) -> MountPlan {
        MountPlan {
            host: host.to_string(),
            guest: guest.to_string(),
            mode,
            policy: None,
            policy_file: None,
        }
    }

    #[test]
    fn fnv1a64_matches_reference_implementation() {
        /// Reference implementation copied from the FNV-1a pseudocode (the
        /// same cross-check shape the slots.rs per-dir shorthash test uses).
        fn fnv_reference(bytes: &[u8]) -> u64 {
            let mut hash: u64 = 0xcbf29ce484222325;
            for &b in bytes {
                hash ^= u64::from(b);
                hash = hash.wrapping_mul(0x100000001b3);
            }
            hash
        }
        for input in [
            "".as_bytes(),
            "a".as_bytes(),
            "/home/node/work".as_bytes(),
            "/da/ta".as_bytes(),
            "/da_ta".as_bytes(),
            "img-pi:personal.aaaaaaaaaaaa".as_bytes(),
        ] {
            assert_eq!(
                fnv1a64(input),
                fnv_reference(input),
                "fnv1a64 drifted from the FNV-1a-64 reference for {input:?}"
            );
        }
    }

    #[test]
    fn fnv1a64_known_vectors() {
        // Standard FNV-1a 64-bit test vectors.
        assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
        assert_eq!(fnv1a64(b"foobar"), 0x8594_4171_f739_67e8);
    }

    #[test]
    fn fnv1a64_is_deterministic_and_input_sensitive() {
        let a = fnv1a64(b"/da/ta");
        assert_eq!(a, fnv1a64(b"/da/ta"), "same input → same hash");
        assert_ne!(a, fnv1a64(b"/da_ta"), "distinct inputs → distinct hashes");
    }

    // ---- config_hash_of_plan: stability ----

    #[test]
    fn same_plan_hashes_identically() {
        let mut plan = empty_plan();
        plan.image = Some("img-pi:personal.aaaaaaaaaaaa".to_string());
        plan.workdir = Some("/app".to_string());
        plan.command = vec!["run".to_string(), "--fast".to_string()];
        plan.env = vec![EnvVar::literal("B", "2"), EnvVar::literal("A", "1")];
        assert_eq!(
            config_hash_of_plan(&plan),
            config_hash_of_plan(&plan),
            "the hash is a pure function of the plan"
        );
    }

    /// THE ADR invariant: doc/comment/formatting/unrelated-workload edits
    /// cannot reach the plan, and non-runtime plan dimensions do not churn
    /// the hash. Pinned exclusion proofs:
    #[test]
    fn excluded_dimensions_do_not_churn_the_hash() {
        let base = empty_plan();

        // Host-port probe difference (--port-auto drew another port) → SAME.
        let mut probed = empty_plan();
        probed.ports = vec![PortMapping::new(54321, 4000)];
        let mut declared = empty_plan();
        declared.ports = vec![PortMapping::new(4000, 4000)];
        assert_eq!(
            config_hash_of_plan(&probed),
            config_hash_of_plan(&declared),
            "host ports are allocation-dependent and MUST be excluded"
        );

        // Parallel-slot bind difference → SAME.
        let mut bound = empty_plan();
        bound.ports = vec![PortMapping {
            host: 14000,
            guest: 4000,
            bind_ip: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 2)),
            name: None,
        }];
        let mut plain = empty_plan();
        plain.ports = vec![PortMapping::new(14000, 4000)];
        assert_eq!(
            config_hash_of_plan(&bound),
            config_hash_of_plan(&plain),
            "bind IPs are allocation-dependent and MUST be excluded"
        );

        // Instance/slot name (plan.name) → SAME.
        let mut renamed = empty_plan();
        renamed.name = "personal-litellm@canary".to_string();
        assert_eq!(config_hash_of_plan(&renamed), config_hash_of_plan(&base));

        // instance_policy (orchestration, not build inputs) → SAME.
        let mut policy = empty_plan();
        policy.instance_policy = Some(crate::config::InstancePolicy {
            strategy: crate::config::InstanceStrategy::Parallel,
            on_conflict: None,
            port: None,
            on_skew: Some(crate::config::OnSkew::Replace),
            label: None,
        });
        assert_eq!(
            config_hash_of_plan(&policy),
            config_hash_of_plan(&base),
            "instance_policy is orchestration policy, not a build input"
        );

        // virtualization provenance (ADR 0036 orchestration posture) → SAME.
        let mut virt = empty_plan();
        virt.virtualization = Some(crate::microsandbox::plan::VirtualizationPlan {
            nested: crate::config::NestedMode::Require,
            degraded: false,
            frozen_out: false,
            frozen_by: None,
            origin: "personal".to_string(),
        });
        assert_eq!(
            config_hash_of_plan(&virt),
            config_hash_of_plan(&base),
            "virtualization is orchestration policy, not a build input"
        );

        // Late-bound policy_file token → SAME.
        let mut tokened = empty_plan();
        tokened.mounts = vec![MountPlan {
            host: "/data".to_string(),
            guest: "/mnt".to_string(),
            mode: MountMode::Rw,
            policy: None,
            policy_file: Some(std::path::PathBuf::from("inst/slug.json")),
        }];
        let mut untokened = empty_plan();
        untokened.mounts = vec![mount("/data", "/mnt", MountMode::Rw)];
        assert_eq!(
            config_hash_of_plan(&tokened),
            config_hash_of_plan(&untokened),
            "policy_file PATHS are late-bound tokens and MUST be excluded"
        );

        // Secret VALUE never enters: two HostBoundSecrets with the same
        // name but different values → SAME hash.
        let secret = |value: &str| HostBoundSecret {
            name: "API_KEY".to_string(),
            value: value.to_string(),
            allowed_hosts: vec!["example.com".to_string()],
            required: true,
            reject_placeholder: None,
            on_violation: SecretViolationPolicy::Passthrough,
        };
        let mut with_a = empty_plan();
        with_a.secret_env = vec![secret("${A}")];
        let mut with_b = empty_plan();
        with_b.secret_env = vec![secret("real-key-value")];
        assert_eq!(
            config_hash_of_plan(&with_a),
            config_hash_of_plan(&with_b),
            "secret VALUES must never enter the hash"
        );

        // Env declaration ORDER → SAME (the canonical form sorts by name).
        let mut one_order = empty_plan();
        one_order.env = vec![EnvVar::literal("A", "1"), EnvVar::literal("B", "2")];
        let mut other_order = empty_plan();
        other_order.env = vec![EnvVar::literal("B", "2"), EnvVar::literal("A", "1")];
        assert_eq!(
            config_hash_of_plan(&one_order),
            config_hash_of_plan(&other_order),
            "env is canonically sorted by name"
        );

        // Network rule declaration order → SAME (rule sets sort).
        let mut net_a = empty_plan();
        net_a.network.egress_rules =
            vec![EgressRule::dns()[0].clone(), EgressRule::litellm_proxy()];
        let mut net_b = empty_plan();
        net_b.network.egress_rules =
            vec![EgressRule::litellm_proxy(), EgressRule::dns()[0].clone()];
        assert_eq!(
            config_hash_of_plan(&net_a),
            config_hash_of_plan(&net_b),
            "network rules are canonically sorted"
        );

        // Egress derived_from marker → SAME (display provenance only).
        let mut derived = empty_plan();
        derived.network.egress_rules = vec![EgressRule {
            protocol: Protocol::Tcp,
            port: 4000,
            target: EgressTarget::Host,
            derived_from: Some("litellm".to_string()),
        }];
        let mut declared_rule = empty_plan();
        declared_rule.network.egress_rules = vec![EgressRule::litellm_proxy()];
        assert_eq!(
            config_hash_of_plan(&derived),
            config_hash_of_plan(&declared_rule),
            "derived_from is not a runtime input"
        );
    }

    /// Runtime-relevant edits DO churn the hash.
    #[test]
    fn runtime_relevant_edits_churn_the_hash() {
        let base_hash = config_hash_of_plan(&empty_plan());

        let mut edited = empty_plan();
        edited.image = Some("python:3.12-slim".to_string());
        assert_ne!(config_hash_of_plan(&edited), base_hash, "image");

        let mut edited = empty_plan();
        edited.workdir = Some("/app".to_string());
        assert_ne!(config_hash_of_plan(&edited), base_hash, "workdir");

        let mut edited = empty_plan();
        edited.command = vec!["run".to_string()];
        assert_ne!(config_hash_of_plan(&edited), base_hash, "command");

        let mut edited = empty_plan();
        edited.cpus = Some(4);
        assert_ne!(config_hash_of_plan(&edited), base_hash, "cpus");

        let mut edited = empty_plan();
        edited.memory_mib = Some(4096);
        assert_ne!(config_hash_of_plan(&edited), base_hash, "memory_mib");

        let mut edited = empty_plan();
        edited.env = vec![EnvVar::literal("A", "1")];
        assert_ne!(config_hash_of_plan(&edited), base_hash, "env edit");
        let mut edited = empty_plan();
        edited.env = vec![EnvVar::literal("A", "1")];
        edited.env[0].value = "2".to_string();
        assert_ne!(
            config_hash_of_plan(&edited),
            config_hash_of_plan(&empty_plan_with_env_a1()),
            "env value edit"
        );

        let mut edited = empty_plan();
        edited.secret_env = vec![HostBoundSecret {
            name: "OTHER_KEY".to_string(),
            value: "${X}".to_string(),
            allowed_hosts: vec![],
            required: false,
            reject_placeholder: None,
            on_violation: SecretViolationPolicy::Passthrough,
        }];
        assert_ne!(
            config_hash_of_plan(&edited),
            config_hash_of_plan(&empty_plan_with_secret_api_key()),
            "secret NAME edit"
        );

        // Guest port change → DIFFERENT (host ports excluded, guests are
        // runtime-relevant).
        let mut guest_edited = empty_plan();
        guest_edited.ports = vec![PortMapping::new(4000, 4001)];
        let mut guest_base = empty_plan();
        guest_base.ports = vec![PortMapping::new(4000, 4000)];
        assert_ne!(
            config_hash_of_plan(&guest_edited),
            config_hash_of_plan(&guest_base),
            "guest port edit"
        );

        // Port NAME change → DIFFERENT (names drive depends_on exports).
        let mut named = empty_plan();
        named.ports = vec![PortMapping {
            host: 4000,
            guest: 4000,
            bind_ip: crate::microsandbox::plan::default_bind_ip(),
            name: Some("api".to_string()),
        }];
        let mut unnamed = empty_plan();
        unnamed.ports = vec![PortMapping::new(4000, 4000)];
        assert_ne!(
            config_hash_of_plan(&named),
            config_hash_of_plan(&unnamed),
            "port name edit"
        );

        // Mount host/guest/mode edits → DIFFERENT.
        let mut m_edited = empty_plan();
        m_edited.mounts = vec![mount("/other", "/mnt", MountMode::Rw)];
        let mut m_base = empty_plan();
        m_base.mounts = vec![mount("/data", "/mnt", MountMode::Rw)];
        assert_ne!(
            config_hash_of_plan(&m_edited),
            config_hash_of_plan(&m_base),
            "mount host"
        );
        let mut g_edited = empty_plan();
        g_edited.mounts = vec![mount("/data", "/other", MountMode::Rw)];
        assert_ne!(
            config_hash_of_plan(&g_edited),
            config_hash_of_plan(&m_base),
            "mount guest"
        );
        let mut ro = empty_plan();
        ro.mounts = vec![mount("/data", "/mnt", MountMode::Ro)];
        assert_ne!(
            config_hash_of_plan(&ro),
            config_hash_of_plan(&m_base),
            "mount mode is runtime-relevant"
        );

        // Mount-policy fragment edit → DIFFERENT.
        let mut frag = mount("/data", "/mnt", MountMode::Rw);
        frag.policy = Some(crate::mount_policy::MountsFragment {
            read: Some(crate::mount_policy::AxisFragment {
                deny: vec![PolicyValue::relaxable("secret/**".to_string())],
                allow: vec![],
            }),
            write: None,
            case_sensitivity: None,
        });
        let mut with_frag = empty_plan();
        with_frag.mounts = vec![frag];
        assert_ne!(
            config_hash_of_plan(&with_frag),
            config_hash_of_plan(&m_base),
            "mount-policy fragment edit"
        );

        // Network edits → DIFFERENT.
        let mut dd = empty_plan();
        dd.network.egress_default_deny = true;
        assert_ne!(config_hash_of_plan(&dd), base_hash, "egress_default_deny");
        // Two configs differing ONLY in the ingress default must hash
        // differently (the `id=` token).
        let mut id = empty_plan();
        id.network.ingress_default_deny = true;
        assert_ne!(config_hash_of_plan(&id), base_hash, "ingress_default_deny");
        let mut eg = empty_plan();
        eg.network.egress_rules = vec![EgressRule::https(&["example.com"])];
        assert_ne!(config_hash_of_plan(&eg), base_hash, "egress rule");
        let mut dn = empty_plan();
        dn.network.deny_rules = vec![DenyDomainRule::suffix(".evil")];
        assert_ne!(config_hash_of_plan(&dn), base_hash, "deny rule");
        let mut ing = empty_plan();
        ing.network.ingress_rules = vec![IngressRule {
            protocol: Protocol::Tcp,
            port: 80,
            scope: Scope::Local,
        }];
        assert_ne!(config_hash_of_plan(&ing), base_hash, "ingress rule");
    }

    fn empty_plan_with_env_a1() -> SandboxPlan {
        let mut p = empty_plan();
        p.env = vec![EnvVar::literal("A", "1")];
        p
    }

    fn empty_plan_with_secret_api_key() -> SandboxPlan {
        let mut p = empty_plan();
        p.secret_env = vec![HostBoundSecret {
            name: "API_KEY".to_string(),
            value: "${A}".to_string(),
            allowed_hosts: vec![],
            required: false,
            reject_placeholder: None,
            on_violation: SecretViolationPolicy::Passthrough,
        }];
        p
    }

    /// The empty-plan vector: pin the exact digest so accidental
    /// serialization drift (field order, separators, encodings) fails loud.
    /// Recompute by hand ONLY alongside a deliberate ADR addendum.
    /// (Recomputed when the `id=` ingress-default token was appended to
    /// `network_canonical` — a deliberate one-time hash change.)
    #[test]
    fn empty_plan_vector_is_pinned() {
        assert_eq!(config_hash_of_plan(&empty_plan()), "df56de50cdf4435f");
    }

    #[test]
    fn hashes_render_as_16_lowercase_hex_and_truncate_to_display_len() {
        let h = config_hash_of_plan(&empty_plan());
        assert_eq!(h.len(), 16);
        assert!(
            h.bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()),
            "lowercase hex only: {h}"
        );
        assert_eq!(short_hash(&h), &h[..PROVENANCE_DISPLAY_LEN]);
        assert_eq!(short_hash("abc"), "abc", "shorter inputs pass through");
    }

    // ---- image_out_hash_from_tag ----

    #[test]
    fn image_out_hash_takes_the_sha_segment_of_computed_tags_only() {
        assert_eq!(
            image_out_hash_from_tag("img-pi:aaaaaaaaaaaa"),
            Some("aaaaaaaaaaaa".to_string()),
            "ctx-less computed tag"
        );
        assert_eq!(
            image_out_hash_from_tag("img-pi:feat-x.bbbbbbbbbbbb"),
            Some("bbbbbbbbbbbb".to_string()),
            "ctx-present computed tag (dot separator)"
        );
        // Dots in the NAME are legal: the sha still splits off the LAST dot.
        assert_eq!(
            image_out_hash_from_tag("img.with.dots:cccccccccccc"),
            Some("cccccccccccc".to_string()),
            "dotted name, ctx-less computed tag"
        );
        for legacy in [
            "img-pi:latest",
            "python:3.12-slim",
            "",
            "img:aaaaaaaaaaaaa",
            "img:AAAAAAAAAAAA",
            // The pre-amendment two-colon shape (host Bug B) never parses.
            "img-pi:feat-x:bbbbbbbbbbbb",
        ] {
            assert_eq!(
                image_out_hash_from_tag(legacy),
                None,
                "legacy/registry tags have no content hash: {legacy}"
            );
        }
    }
}
