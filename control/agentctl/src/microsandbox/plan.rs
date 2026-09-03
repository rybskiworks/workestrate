use serde::{Deserialize, Serialize};
use std::fmt;
use std::net::{IpAddr, Ipv4Addr};

use crate::config::SecretViolationPolicy;
use crate::microsandbox::secrets::SecretDefinition;
use crate::mount_policy::{AxisFragment, MountsFragment};

/// Default host bind address for published ports (ADR 0026): `127.0.0.1`,
/// the shared singleton bind. Parallel slots bind per-instance loopbacks
/// (`127.0.0.N`, `N >= 2`) drawn from the port registry's loopback allocator.
pub fn default_bind_ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::LOCALHOST)
}

/// Network protocol for ingress/egress rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Protocol {
    Tcp,
    Udp,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Protocol::Tcp => write!(f, "tcp"),
            Protocol::Udp => write!(f, "udp"),
        }
    }
}

/// Network scope for ingress rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Local,
    /// Public ingress (not yet used by any workload).
    #[allow(dead_code)]
    Public,
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Scope::Local => write!(f, "local"),
            Scope::Public => write!(f, "public"),
        }
    }
}

/// Egress destination: host bridge or specific DNS domains.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum EgressTarget {
    /// Host bridge networking (microsandbox host, including host.microsandbox.internal).
    Host,
    /// Specific DNS domains.
    Domains(Vec<String>),
}

impl fmt::Display for EgressTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EgressTarget::Host => write!(f, "host"),
            EgressTarget::Domains(hosts) => write!(f, "{}", hosts.join(", ")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct SandboxPlan {
    pub name: String,
    pub image: Option<String>,
    pub workdir: Option<String>,
    pub command: Vec<String>,
    pub cpus: Option<u8>,
    pub memory_mib: Option<u32>,
    pub env: Vec<EnvVar>,
    pub secret_env: Vec<HostBoundSecret>,
    pub ports: Vec<PortMapping>,
    pub mounts: Vec<MountPlan>,
    pub network: NetworkPlan,
    /// The workload's instance policy (ADR 0030 §4.1), when declared.
    /// `None` = current behavior (no policy block). Additive serde default so
    /// legacy plan JSON parses cleanly and legacy output stays byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_policy: Option<crate::config::InstancePolicy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PortMapping {
    /// Host port to publish on `bind_ip`. `0` = auto-allocate a free port at
    /// boot (probed from the port registry; the effective port is recorded in
    /// the instance record and shown by `ps`).
    pub host: u16,
    pub guest: u16,
    /// Host bind address (ADR 0026). Defaults to 127.0.0.1 (the shared singleton bind).
    #[serde(default = "default_bind_ip")]
    pub bind_ip: IpAddr,
    /// Optional port name (namespaced ports): gives the port a stable identity
    /// that `depends_on.<dep>.exports = { <name> = "<ENV_VAR>" }` resolves
    /// (one exported env var per named port). Unnamed = the legacy primary
    /// port, addressed via `depends_on.<dep>.env`. Additive serde default so
    /// older plans/configs parse cleanly; skipped when `None` so legacy JSON
    /// is byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl PortMapping {
    /// A host:guest mapping on the default shared singleton bind (127.0.0.1).
    pub fn new(host: u16, guest: u16) -> Self {
        Self {
            host,
            guest,
            bind_ip: default_bind_ip(),
            name: None,
        }
    }
}

/// Mount access mode (spec §"Mount `mode` field"). Closed vocabulary: the
/// only accepted wire values are `"ro"` (read-only) and `"rw"` (read-write,
/// the default). This replaces the `read_only = <bool>` field, which remains
/// a DEPRECATED parse-time alias (see [`MountPlan`]'s custom `Deserialize`).
/// Rationale (ADR 0030 addendum): booleans don't grow — a future third state
/// (e.g. masked / append-only) fits a mode enum, not a bool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "lowercase")]
pub enum MountMode {
    Ro,
    #[default]
    Rw,
}

impl fmt::Display for MountMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MountMode::Ro => write!(f, "ro"),
            MountMode::Rw => write!(f, "rw"),
        }
    }
}

/// A mount row (`[[mounts]]` / `[[workloads.<name>.mounts]]`), BOTH the
/// config shape AND the plan-JSON wire shape. The canonical field is `mode`
/// ("ro" | "rw", default "rw"); `read_only = <bool>` is accepted as a
/// DEPRECATED alias at parse time only — it is normalized into `mode` by the
/// custom `Deserialize` below and is NEVER serialized. Alias semantics:
///   - only `mode` set            → use it;
///   - only `read_only` set       → `true` ≡ "ro", `false` ≡ "rw", plus one
///     deprecation warning per process (stderr, matching the repo's other
///     deprecation notices; plan-JSON re-parses never carry `read_only`, so
///     they stay quiet);
///   - both set and AGREE         → accept, same single deprecation warning;
///   - both set and CONFLICT      → hard deserialization error naming both
///     fields and both values.
///
/// Because the alias normalizes at parse time, merged config layers only
/// ever carry `mode` (mounts merge whole-array, last layer wins). The same
/// parse-time normalization applies to the mount-policy sugar sub-tables
/// (`read`/`write` directly on the row, in the `[policy.mounts]` axis
/// shape): they are concatenated INTO `policy` by the custom `Deserialize`
/// below, so merged layers only ever carry the canonical `policy` fragment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountPlan {
    pub host: String,
    pub guest: String,
    pub mode: MountMode,
    /// Mount-entry policy is collected from the same declaring layer as this
    /// row; it is intentionally not part of mount-row merging.
    pub policy: Option<MountsFragment>,
    pub policy_file: Option<std::path::PathBuf>,
}

impl MountPlan {
    /// Canonical internal accessor: `true` iff `mode == "ro"`. Replaces the
    /// removed `read_only: bool` field at all internal call sites.
    pub fn is_read_only(&self) -> bool {
        self.mode == MountMode::Ro
    }
}

/// Serde wire shape for [`MountPlan`]: both `mode` and the deprecated
/// `read_only` alias are optional on the wire so the reconciliation logic in
/// `TryFrom<MountPlanWire>` can distinguish "unset" from the default. Also
/// the schemars source: the generated JSON Schema for `MountPlan` is derived
/// from this struct (see the manual `JsonSchema` impl below), so the schema
/// matches exactly what the parser accepts. `deny_unknown_fields` keeps the
/// removed sugar words (`mask`/`unmask`/`protect`/`writes_deny`) hard parse
/// errors rather than silently ignored keys.
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
struct MountPlanWire {
    host: String,
    guest: String,
    /// Mount access mode: "ro" (read-only) or "rw" (read-write, the default).
    #[serde(default)]
    mode: Option<MountMode>,
    /// DEPRECATED alias for `mode` (`true` ≡ "ro", `false` ≡ "rw"); accepted
    /// at parse time with a deprecation warning, never serialized.
    #[serde(default)]
    read_only: Option<bool>,
    /// Mount-entry policy is collected from the same declaring layer as this
    /// row; it is intentionally not part of mount-row merging.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "schema", schemars(with = "Option<MountsFragment>"))]
    policy: Option<MountsFragment>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    policy_file: Option<std::path::PathBuf>,
    /// Mount-policy sugar: the read axis (visibility) directly on the mount
    /// row, in the same shape `[policy.mounts.read]` accepts (TOML
    /// dotted-key form: `read.deny = [...]` / `read.allow = [...]`).
    /// Parse-time-only: normalized INTO `policy.read` by
    /// `TryFrom<MountPlanWire>`, never serialized.
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(with = "Option<AxisFragment>"))]
    read: Option<AxisFragment>,
    /// Mount-policy sugar: the write axis (write admission) directly on the
    /// mount row (same shape as `read`; `write.deny = [...]` /
    /// `write.allow = [...]`). Parse-time-only, normalized into
    /// `policy.write`.
    #[serde(default)]
    #[cfg_attr(feature = "schema", schemars(with = "Option<AxisFragment>"))]
    write: Option<AxisFragment>,
}

/// One deprecation warning per process is enough (a config may declare many
/// legacy rows); the CLI's other deprecation notices are likewise
/// once-per-invocation, which this reduces to.
static READ_ONLY_ALIAS_WARNED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

fn warn_read_only_alias() {
    if !READ_ONLY_ALIAS_WARNED.swap(true, std::sync::atomic::Ordering::Relaxed) {
        eprintln!(
            "warning: mount field `read_only` is deprecated; use `mode = \"ro\"` / \
             `mode = \"rw\"` instead (`read_only = true` ≡ `mode = \"ro\"`)"
        );
    }
}

impl TryFrom<MountPlanWire> for MountPlan {
    type Error = String;

    fn try_from(wire: MountPlanWire) -> Result<Self, Self::Error> {
        let mode = match (wire.mode, wire.read_only) {
            (Some(mode), None) => mode,
            (None, Some(read_only)) => {
                warn_read_only_alias();
                if read_only {
                    MountMode::Ro
                } else {
                    MountMode::Rw
                }
            }
            (Some(mode), Some(read_only)) => {
                let alias_mode = if read_only {
                    MountMode::Ro
                } else {
                    MountMode::Rw
                };
                if mode != alias_mode {
                    return Err(format!(
                        "conflicting mount fields: read_only = {read_only} (≡ mode = \
                         \"{alias_mode}\") but mode = \"{mode}\"; remove the deprecated \
                         `read_only` field (or make the two agree)"
                    ));
                }
                warn_read_only_alias();
                mode
            }
            (None, None) => MountMode::default(),
        };
        // Mount-policy sugar normalization: `read`/`write` sub-tables on the
        // row concatenate INTO the mount's `policy` fragment (AFTER any
        // explicitly-declared `policy` table entries; both forms may mix).
        // Per-scope compile groups read.deny-before-read.allow and keeps the
        // write axis a separate rule set (spec 22 §4), so the concatenation
        // order is semantics-free. Entries stay raw `PolicyValue<String>` —
        // pattern validation happens at compile time with the declaring
        // origin named, exactly as the policy-table form.
        let mut policy = wire.policy;
        if wire.read.is_some() || wire.write.is_some() {
            let fragment = policy.get_or_insert_with(MountsFragment::default);
            if let Some(read) = wire.read {
                let target = fragment.read.get_or_insert_with(AxisFragment::default);
                target.deny.extend(read.deny);
                target.allow.extend(read.allow);
            }
            if let Some(write) = wire.write {
                let target = fragment.write.get_or_insert_with(AxisFragment::default);
                target.deny.extend(write.deny);
                target.allow.extend(write.allow);
            }
        }
        Ok(Self {
            host: wire.host,
            guest: wire.guest,
            mode,
            policy,
            policy_file: wire.policy_file,
        })
    }
}

impl<'de> Deserialize<'de> for MountPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = MountPlanWire::deserialize(deserializer)?;
        MountPlan::try_from(wire).map_err(serde::de::Error::custom)
    }
}

impl Serialize for MountPlan {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        // Canonical form only: `mode` is always emitted; `read_only` is NEVER
        // serialized. `policy`/`policy_file` stay skip-when-None so plan JSON
        // without them is byte-identical to the legacy form.
        let mut n = 3;
        if self.policy.is_some() {
            n += 1;
        }
        if self.policy_file.is_some() {
            n += 1;
        }
        let mut s = serializer.serialize_struct("MountPlan", n)?;
        s.serialize_field("host", &self.host)?;
        s.serialize_field("guest", &self.guest)?;
        s.serialize_field("mode", &self.mode)?;
        if let Some(policy) = &self.policy {
            s.serialize_field("policy", policy)?;
        }
        if let Some(policy_file) = &self.policy_file {
            s.serialize_field("policy_file", policy_file)?;
        }
        s.end()
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for MountPlan {
    fn schema_name() -> String {
        "MountPlan".to_string()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        // Start from the wire shape (both fields optional, read_only present
        // but deprecated) so the schema matches exactly what the parser
        // accepts, then pin `mode` to its canonical closed-vocabulary form.
        let mut schema = MountPlanWire::json_schema(gen).into_object();
        schema.metadata = Some(Box::new(schemars::schema::Metadata {
            description: Some(
                "A mount row (`[[mounts]]` / `[[workloads.<name>.mounts]]`). Canonical \
                 field: `mode` (\"ro\" | \"rw\", default \"rw\"); `read_only = <bool>` \
                 is a deprecated parse-time alias (normalized into `mode`, never \
                 serialized). `read`/`write` are parse-time mount-policy sugar in the \
                 `[policy.mounts]` axis shapes, normalized into the row's `policy` \
                 fragment (never serialized)."
                    .to_string(),
            ),
            ..Default::default()
        }));
        if let Some(object) = schema.object.as_mut() {
            object.properties.insert(
                "mode".to_string(),
                schemars::schema::Schema::Object(schemars::schema::SchemaObject {
                    metadata: Some(Box::new(schemars::schema::Metadata {
                        description: Some(
                            "Mount access mode: \"ro\" (read-only) or \"rw\" \
                             (read-write, the default)."
                                .to_string(),
                        ),
                        default: Some(serde_json::json!("rw")),
                        ..Default::default()
                    })),
                    instance_type: Some(schemars::schema::SingleOrVec::Single(Box::new(
                        schemars::schema::InstanceType::String,
                    ))),
                    enum_values: Some(vec![serde_json::json!("ro"), serde_json::json!("rw")]),
                    ..Default::default()
                }),
            );
            object.properties.insert(
                "read_only".to_string(),
                schemars::schema::Schema::Object(schemars::schema::SchemaObject {
                    metadata: Some(Box::new(schemars::schema::Metadata {
                        description: Some(
                            "DEPRECATED alias for `mode` (`true` ≡ \"ro\", `false` ≡ \
                             \"rw\"); accepted at parse time with a deprecation \
                             warning, never serialized."
                                .to_string(),
                        ),
                        deprecated: true,
                        ..Default::default()
                    })),
                    instance_type: Some(schemars::schema::SingleOrVec::Single(Box::new(
                        schemars::schema::InstanceType::Boolean,
                    ))),
                    ..Default::default()
                }),
            );
        }
        schemars::schema::Schema::Object(schema)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct HostBoundSecret {
    pub name: String,
    pub value: String,
    pub allowed_hosts: Vec<String>,
    pub required: bool,
    pub reject_placeholder: Option<String>,
    /// Egress violation policy for traffic carrying the placeholder to a
    /// non-allowed host. Additive serde default so plans serialized before
    /// this field existed still parse (mirrors `EnvVar.injected_by`).
    #[serde(default)]
    pub on_violation: SecretViolationPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EnvVar {
    pub name: String,
    pub value: String,
    pub is_secret: bool,
    pub reject_placeholder: Option<String>,
    /// ADR 0026(d) discovery-lite: the `depends_on` dependency this var was
    /// injected for (`None` = declared env). Additive serde default so plans
    /// serialized before this field existed still parse; skipped when `None`
    /// so declared-env JSON is byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub injected_by: Option<String>,
    /// ADR 0026(d) discovery-lite: the named port this var was injected for
    /// (`None` = the primary/unnamed port). Additive serde default; skipped
    /// when `None` so declared-env JSON is byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub injected_port: Option<String>,
}

impl fmt::Display for EnvVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = if self.is_secret {
            "(redacted)"
        } else {
            self.value.as_str()
        };
        // ADR 0026(d): an injected var is marked with its origin dependency
        // (and, when present, the named port it was injected for); declared
        // env keeps the legacy `NAME=value` form byte-identical.
        match (&self.injected_by, &self.injected_port) {
            (Some(dep), Some(port)) => write!(
                f,
                "{}={} (injected: depends_on '{}' port '{}')",
                self.name, value, dep, port
            ),
            (Some(dep), None) => write!(
                f,
                "{}={} (injected: depends_on '{}')",
                self.name, value, dep
            ),
            (None, _) => write!(f, "{}={}", self.name, value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct NetworkPlan {
    pub egress_default_deny: bool,
    pub ingress_default_deny: bool,
    pub egress_rules: Vec<EgressRule>,
    pub deny_rules: Vec<DenyDomainRule>,
    pub ingress_rules: Vec<IngressRule>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct EgressRule {
    pub protocol: Protocol,
    pub port: u16,
    pub target: EgressTarget,
    /// ADR 0026(d) discovery-lite: the `depends_on` dependency this rule was
    /// derived for (`None` = declared/recipe-expanded egress). Additive serde
    /// default; skipped when `None` so declared-egress JSON is byte-identical.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub derived_from: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DenyDomainRule {
    pub domain_suffix: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct IngressRule {
    pub protocol: Protocol,
    pub port: u16,
    pub scope: Scope,
}

impl fmt::Display for SandboxPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "name: {}", self.name)?;
        if let Some(img) = &self.image {
            writeln!(f, "image: {}", img)?;
        }
        if let Some(wd) = &self.workdir {
            writeln!(f, "workdir: {}", wd)?;
        }
        if !self.command.is_empty() {
            writeln!(f, "command: {}", self.command.join(" "))?;
        }
        if let Some(cpus) = self.cpus {
            writeln!(f, "cpus: {}", cpus)?;
        }
        if let Some(mem) = self.memory_mib {
            writeln!(f, "memory: {} MiB", mem)?;
        }
        for e in &self.env {
            writeln!(f, "env: {}", e)?;
        }
        for se in &self.secret_env {
            let req = if se.required { "required" } else { "optional" };
            writeln!(
                f,
                "secret_env: {} (value redacted, allowed: {}, {})",
                se.name,
                se.allowed_hosts.join(", "),
                req
            )?;
        }
        for p in &self.ports {
            // ADR 0026/C2: render the bind ONLY when it is not the default
            // shared singleton bind (127.0.0.1). Default-bind plans keep the
            // legacy `port: <host>:<guest>` line BYTE-IDENTICAL (the golden
            // plans pin this); non-default binds render
            // `port: <bind_ip>:<host>:<guest>`.
            // P1: a port name (when present) prefixes the line
            // (`port: <name>:<host>:<guest>`); `host = 0` marks an auto port
            // and renders an `(auto)` suffix (the allocation happens at boot,
            // so 0 is the honest prospective view).
            let name_prefix = p
                .name
                .as_deref()
                .map(|n| format!("{n}:"))
                .unwrap_or_default();
            let auto_suffix = if p.host == 0 { " (auto)" } else { "" };
            if p.bind_ip == default_bind_ip() {
                writeln!(
                    f,
                    "port: {}{}:{}{}",
                    name_prefix, p.host, p.guest, auto_suffix
                )?;
            } else {
                writeln!(
                    f,
                    "port: {}{}:{}:{}{}",
                    name_prefix, p.bind_ip, p.host, p.guest, auto_suffix
                )?;
            }
        }
        for m in &self.mounts {
            let ro = if m.is_read_only() { " (ro)" } else { "" };
            writeln!(f, "mount: {}:{}{}", m.host, m.guest, ro)?;
            if let Some(pf) = &m.policy_file {
                writeln!(f, "  policy_file: {}", pf.display())?;
            }
        }
        writeln!(
            f,
            "network: egress_default={} ingress_default={}",
            if self.network.egress_default_deny {
                "deny"
            } else {
                "allow"
            },
            if self.network.ingress_default_deny {
                "deny"
            } else {
                "allow"
            }
        )?;
        for rule in &self.network.ingress_rules {
            writeln!(
                f,
                "  ingress: {}:{} {}",
                rule.protocol, rule.port, rule.scope
            )?;
        }
        for rule in &self.network.egress_rules {
            // ADR 0026(d): a rule derived from a depends_on resolution is
            // marked with its origin dependency; declared/recipe-expanded
            // rules keep the legacy line byte-identical (the golden plans
            // pin this).
            match &rule.derived_from {
                Some(dep) => writeln!(
                    f,
                    "  egress: {}:{} -> {} (derived: depends_on '{}')",
                    rule.protocol, rule.port, rule.target, dep
                )?,
                None => writeln!(
                    f,
                    "  egress: {}:{} -> {}",
                    rule.protocol, rule.port, rule.target
                )?,
            }
        }
        for rule in &self.network.deny_rules {
            writeln!(f, "  egress: deny domain suffix {}", rule.domain_suffix)?;
        }
        if let Some(policy) = &self.instance_policy {
            writeln!(f, "instance: strategy={}", policy.strategy)?;
            if let Some(chain) = &policy.on_conflict {
                let steps: Vec<String> = chain.0.iter().map(ToString::to_string).collect();
                writeln!(f, "instance: on_conflict=[{}]", steps.join(", "))?;
            }
            if let Some(port) = &policy.port {
                writeln!(f, "instance: port={}", render_instance_port(port))?;
            }
            if let Some(label) = &policy.label {
                writeln!(f, "instance: label={label}")?;
            }
        }
        Ok(())
    }
}

/// Render an instance port policy for the plan display (ADR 0030 addendum 2
/// U6 forms; the on_occupied DEFAULT chain is not rendered — only declared
/// values).
pub(crate) fn render_instance_port(port: &crate::config::InstancePort) -> String {
    match port {
        crate::config::InstancePort::Strict(n) => n.to_string(),
        crate::config::InstancePort::Auto => "auto".to_string(),
        crate::config::InstancePort::Preferred(p) => {
            let mut s = format!("preferred={}", p.preferred);
            if let Some(chain) = &p.on_occupied {
                let steps: Vec<String> = chain.0.iter().map(render_occupied_step).collect();
                s.push_str(&format!(", on_occupied=[{}]", steps.join(", ")));
            }
            s
        }
    }
}

/// Render one port `on_occupied` chain step for the plan display (ADR 0030
/// addendum 2 U6).
pub(crate) fn render_occupied_step(step: &crate::config::PortOccupiedStep) -> String {
    match step {
        crate::config::PortOccupiedStep::Bare(b) => b.to_string(),
        crate::config::PortOccupiedStep::Increment(p) => {
            match (&p.increment.limit, &p.increment.range) {
                (Some(l), None) => format!("increment(limit={l})"),
                (None, Some((s, e))) => format!("increment(range={s}..{e})"),
                _ => "increment".to_string(),
            }
        }
    }
}

impl EnvVar {
    pub fn literal(name: &str, value: &str) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            is_secret: false,
            reject_placeholder: None,
            injected_by: None,
            injected_port: None,
        }
    }
}

impl HostBoundSecret {
    /// Host-bound binding of a secret definition under its own resolved
    /// name (`source_env_var`). All metadata comes from the definition.
    pub fn from(definition: &SecretDefinition) -> Self {
        Self {
            name: definition.source_env_var.clone(),
            value: format!("${{{}}}", definition.source_env_var),
            allowed_hosts: definition.allowed_hosts.clone(),
            required: definition.required,
            reject_placeholder: definition.placeholder.clone(),
            on_violation: definition.on_violation,
        }
    }
}

impl EgressRule {
    pub fn dns() -> Vec<Self> {
        vec![
            Self {
                protocol: Protocol::Tcp,
                port: 53,
                target: EgressTarget::Host,
                derived_from: None,
            },
            Self {
                protocol: Protocol::Udp,
                port: 53,
                target: EgressTarget::Host,
                derived_from: None,
            },
        ]
    }
    /// TCP 4000 to the host bridge: the OSS litellm proxy's default port
    /// (generic tool vocabulary — no personal models/hosts encoded).
    /// `agent_base` embeds this rule, so workloads using `agent_base`
    /// assume a host-reachable litellm proxy on port 4000.
    pub fn litellm_proxy() -> Self {
        Self {
            protocol: Protocol::Tcp,
            port: 4000,
            target: EgressTarget::Host,
            derived_from: None,
        }
    }
    pub fn https(domains: &[&str]) -> Self {
        Self {
            protocol: Protocol::Tcp,
            port: 443,
            target: EgressTarget::Domains(domains.iter().map(|d| d.to_string()).collect()),
            derived_from: None,
        }
    }
    /// dns + litellm_proxy + github: the baseline agent egress set. Note the
    /// embedded litellm_proxy rule means every `agent_base` workload may
    /// reach a litellm proxy on the host bridge at its default port 4000.
    pub fn agent_base() -> Vec<Self> {
        let mut rules = Self::dns();
        rules.push(Self::litellm_proxy());
        rules.push(Self::https(&["github.com", "api.github.com"]));
        rules
    }
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

    fn definition(env_var: &str) -> SecretDefinition {
        SecretDefinition {
            source_env_var: env_var.to_string(),
            allowed_hosts: vec!["example.com".to_string()],
            required: true,
            placeholder: Some("CHANGEME".to_string()),
            on_violation: SecretViolationPolicy::BlockAndLog,
        }
    }

    // ---- Display stability (golden-inline; guards Display drift) ----

    #[test]
    fn sandbox_plan_display_is_stable() {
        let plan = SandboxPlan {
            name: "demo".to_string(),
            image: Some("img:1".to_string()),
            workdir: Some("/app".to_string()),
            command: vec!["run".to_string(), "--fast".to_string()],
            cpus: Some(2),
            memory_mib: Some(512),
            env: vec![
                EnvVar::literal("PLAIN", "value"),
                EnvVar {
                    name: "TOKEN".to_string(),
                    value: "supersecret".to_string(),
                    is_secret: true,
                    reject_placeholder: None,
                    injected_by: None,
                    injected_port: None,
                },
            ],
            secret_env: vec![HostBoundSecret {
                name: "API_KEY".to_string(),
                value: "${API_KEY}".to_string(),
                allowed_hosts: vec!["example.com".to_string()],
                required: false,
                reject_placeholder: None,
                on_violation: SecretViolationPolicy::Passthrough,
            }],
            ports: vec![PortMapping::new(8080, 80)],
            mounts: vec![
                MountPlan {
                    host: "/data".to_string(),
                    guest: "/mnt".to_string(),
                    mode: MountMode::Rw,
                    policy: None,
                    policy_file: None,
                },
                MountPlan {
                    host: "/cfg".to_string(),
                    guest: "/etc/cfg".to_string(),
                    mode: MountMode::Ro,
                    policy: None,
                    policy_file: None,
                },
            ],
            network: NetworkPlan {
                egress_default_deny: true,
                ingress_default_deny: true,
                egress_rules: vec![
                    EgressRule::litellm_proxy(),
                    EgressRule::https(&["example.com"]),
                ],
                deny_rules: vec![DenyDomainRule {
                    domain_suffix: ".evil".to_string(),
                }],
                ingress_rules: vec![IngressRule {
                    protocol: Protocol::Tcp,
                    port: 80,
                    scope: Scope::Local,
                }],
            },
            instance_policy: None,
        };
        let expected = "\
name: demo
image: img:1
workdir: /app
command: run --fast
cpus: 2
memory: 512 MiB
env: PLAIN=value
env: TOKEN=(redacted)
secret_env: API_KEY (value redacted, allowed: example.com, optional)
port: 8080:80
mount: /data:/mnt
mount: /cfg:/etc/cfg (ro)
network: egress_default=deny ingress_default=deny
  ingress: tcp:80 local
  egress: tcp:4000 -> host
  egress: tcp:443 -> example.com
  egress: deny domain suffix .evil
";
        assert_eq!(format!("{plan}"), expected);
    }

    /// ADR 0026/C2: a non-default bind renders `port: <bind_ip>:<host>:<guest>`;
    /// the default shared bind (127.0.0.1) keeps the legacy two-field line so
    /// existing golden plans stay byte-identical.
    #[test]
    fn sandbox_plan_display_renders_bind_only_when_non_default() {
        let base = |ports: Vec<PortMapping>| SandboxPlan {
            name: "binds".to_string(),
            image: None,
            workdir: None,
            command: vec![],
            cpus: None,
            memory_mib: None,
            env: vec![],
            secret_env: vec![],
            ports,
            mounts: vec![],
            network: NetworkPlan {
                egress_default_deny: false,
                ingress_default_deny: false,
                egress_rules: vec![],
                deny_rules: vec![],
                ingress_rules: vec![],
            },
            instance_policy: None,
        };

        // Non-default bind → three-field line with the bind IP.
        let parallel = base(vec![PortMapping {
            host: 8080,
            guest: 80,
            bind_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)),
            name: None,
        }]);
        let rendered = format!("{parallel}");
        assert!(
            rendered.contains("port: 127.0.0.2:8080:80\n"),
            "non-default bind must render the bind IP; got:\n{rendered}"
        );

        // Default bind → legacy two-field line (golden-plan invariant).
        let singleton = base(vec![PortMapping::new(8080, 80)]);
        let rendered = format!("{singleton}");
        assert!(
            rendered.contains("port: 8080:80\n"),
            "default bind must keep the legacy line; got:\n{rendered}"
        );
        assert!(
            !rendered.contains("127.0.0.1"),
            "default bind must NOT print the bind IP; got:\n{rendered}"
        );
    }

    /// P1 (namespaced ports): a port name prefixes the line and `host = 0`
    /// renders an `(auto)` suffix; unnamed non-auto ports keep the legacy
    /// byte-identical format (golden-plan invariant).
    #[test]
    fn sandbox_plan_display_renders_names_and_auto_marker() {
        let base = |ports: Vec<PortMapping>| SandboxPlan {
            name: "names".to_string(),
            image: None,
            workdir: None,
            command: vec![],
            cpus: None,
            memory_mib: None,
            env: vec![],
            secret_env: vec![],
            ports,
            mounts: vec![],
            network: NetworkPlan {
                egress_default_deny: false,
                ingress_default_deny: false,
                egress_rules: vec![],
                deny_rules: vec![],
                ingress_rules: vec![],
            },
            instance_policy: None,
        };

        // Named port on the default bind → `port: <name>:<host>:<guest>`.
        let named = base(vec![PortMapping {
            host: 4000,
            guest: 4000,
            bind_ip: default_bind_ip(),
            name: Some("api".to_string()),
        }]);
        let rendered = format!("{named}");
        assert!(
            rendered.contains("port: api:4000:4000\n"),
            "named port must render a name prefix; got:\n{rendered}"
        );

        // Named port on a non-default bind → the bind IP joins the line.
        let named_parallel = base(vec![PortMapping {
            host: 8080,
            guest: 80,
            bind_ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 2)),
            name: Some("api".to_string()),
        }]);
        let rendered = format!("{named_parallel}");
        assert!(
            rendered.contains("port: api:127.0.0.2:8080:80\n"),
            "named non-default-bind port must render name + bind IP; got:\n{rendered}"
        );

        // Auto port (host = 0) → `(auto)` suffix; named auto prefixes too.
        let auto = base(vec![PortMapping::new(0, 4000)]);
        let rendered = format!("{auto}");
        assert!(
            rendered.contains("port: 0:4000 (auto)\n"),
            "host=0 port must render the (auto) marker; got:\n{rendered}"
        );

        let named_auto = base(vec![PortMapping {
            host: 0,
            guest: 4000,
            bind_ip: default_bind_ip(),
            name: Some("api".to_string()),
        }]);
        let rendered = format!("{named_auto}");
        assert!(
            rendered.contains("port: api:0:4000 (auto)\n"),
            "named auto port must render name + (auto); got:\n{rendered}"
        );

        // Legacy unnamed non-auto stays byte-identical AND the named/auto
        // formats never leak into it.
        let legacy = base(vec![PortMapping::new(4000, 4000)]);
        let rendered = format!("{legacy}");
        assert_eq!(
            rendered,
            "name: names\nport: 4000:4000\nnetwork: egress_default=allow ingress_default=allow\n"
        );
        assert!(
            !rendered.contains("(auto)"),
            "legacy unnamed non-auto port must not render (auto); got:\n{rendered}"
        );
        assert!(
            !rendered.contains("port: api:"),
            "legacy output must not contain the named-port format; got:\n{rendered}"
        );
    }

    #[test]
    fn sandbox_plan_display_omits_absent_optional_fields() {
        let plan = SandboxPlan {
            name: "bare".to_string(),
            image: None,
            workdir: None,
            command: vec![],
            cpus: None,
            memory_mib: None,
            env: vec![],
            secret_env: vec![],
            ports: vec![],
            mounts: vec![],
            network: NetworkPlan {
                egress_default_deny: false,
                ingress_default_deny: false,
                egress_rules: vec![],
                deny_rules: vec![],
                ingress_rules: vec![],
            },
            instance_policy: None,
        };
        assert_eq!(
            format!("{plan}"),
            "name: bare\nnetwork: egress_default=allow ingress_default=allow\n"
        );
    }

    #[test]
    fn env_var_display_redacts_secrets_only() {
        let plain = EnvVar::literal("A", "1");
        assert_eq!(format!("{plain}"), "A=1");
        let secret = EnvVar {
            name: "S".to_string(),
            value: "hunter2".to_string(),
            is_secret: true,
            reject_placeholder: None,
            injected_by: None,
            injected_port: None,
        };
        let shown = format!("{secret}");
        assert_eq!(shown, "S=(redacted)");
        assert!(!shown.contains("hunter2"), "secret value must not leak");
    }

    // ---- constructors ----

    #[test]
    fn env_var_literal_marks_non_secret_and_no_placeholder() {
        let v = EnvVar::literal("K", "v");
        assert_eq!(v.name, "K");
        assert_eq!(v.value, "v");
        assert!(!v.is_secret);
        assert_eq!(v.reject_placeholder, None);
        assert_eq!(v.injected_by, None, "declared env is never marked");
    }

    // ---- ADR 0026(d): injected-env / derived-egress render forms ----

    /// An injected env var renders `NAME=value (injected: depends_on
    /// '<dep>')`; the value is a URL (NOT a secret — never redacted). The
    /// declared form stays byte-identical (`None` marker).
    #[test]
    fn env_var_display_marks_injected_by() {
        let injected = EnvVar {
            name: "LITELLM_URL".to_string(),
            value: "host.microsandbox.internal:4000".to_string(),
            is_secret: false,
            reject_placeholder: None,
            injected_by: Some("litellm".to_string()),
            injected_port: None,
        };
        assert_eq!(
            format!("{injected}"),
            "LITELLM_URL=host.microsandbox.internal:4000 (injected: depends_on 'litellm')"
        );
        let declared = EnvVar::literal("PLAIN", "value");
        assert_eq!(format!("{declared}"), "PLAIN=value");
    }

    /// P2 (namespaced exports): an injected var carrying a NAMED port renders
    /// `(injected: depends_on '<dep>' port '<port>')`; the primary-port form
    /// (no port) stays the byte-identical legacy `(injected: depends_on
    /// '<dep>')` line, and declared env never renders a marker at all.
    #[test]
    fn env_var_display_marks_injected_port_when_present() {
        let named = EnvVar {
            name: "LITELLM_API_URL".to_string(),
            value: "host.microsandbox.internal:14000".to_string(),
            is_secret: false,
            reject_placeholder: None,
            injected_by: Some("litellm".to_string()),
            injected_port: Some("api".to_string()),
        };
        assert_eq!(
            format!("{named}"),
            "LITELLM_API_URL=host.microsandbox.internal:14000 (injected: depends_on 'litellm' port 'api')"
        );
        let primary = EnvVar {
            name: "LITELLM_URL".to_string(),
            value: "host.microsandbox.internal:4000".to_string(),
            is_secret: false,
            reject_placeholder: None,
            injected_by: Some("litellm".to_string()),
            injected_port: None,
        };
        assert_eq!(
            format!("{primary}"),
            "LITELLM_URL=host.microsandbox.internal:4000 (injected: depends_on 'litellm')"
        );
    }

    /// A derived egress rule renders the ` (derived: depends_on '<dep>')`
    /// suffix; declared/recipe-expanded rules (`None`) keep the legacy line.
    #[test]
    fn egress_rule_display_marks_derived_from() {
        let plan = |rules: Vec<EgressRule>| SandboxPlan {
            name: "derived".to_string(),
            image: None,
            workdir: None,
            command: vec![],
            cpus: None,
            memory_mib: None,
            env: vec![],
            secret_env: vec![],
            ports: vec![],
            mounts: vec![],
            network: NetworkPlan {
                egress_default_deny: true,
                ingress_default_deny: true,
                egress_rules: rules,
                deny_rules: vec![],
                ingress_rules: vec![],
            },
            instance_policy: None,
        };
        let rendered = format!(
            "{}",
            plan(vec![
                EgressRule::litellm_proxy(),
                EgressRule {
                    protocol: Protocol::Tcp,
                    port: 5432,
                    target: EgressTarget::Host,
                    derived_from: Some("db".to_string()),
                },
            ])
        );
        assert!(
            rendered.contains("  egress: tcp:4000 -> host\n"),
            "declared rule keeps the legacy line; got:\n{rendered}"
        );
        assert!(
            rendered.contains("  egress: tcp:5432 -> host (derived: depends_on 'db')\n"),
            "derived rule renders the marker suffix; got:\n{rendered}"
        );
    }

    #[test]
    fn egress_rule_dns_is_tcp_and_udp_53_to_host() {
        let rules = EgressRule::dns();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].protocol, Protocol::Tcp);
        assert_eq!(rules[1].protocol, Protocol::Udp);
        for r in &rules {
            assert_eq!(r.port, 53);
            assert_eq!(r.target, EgressTarget::Host);
        }
    }

    #[test]
    fn egress_rule_litellm_proxy_is_tcp_4000_to_host() {
        let r = EgressRule::litellm_proxy();
        assert_eq!(r.protocol, Protocol::Tcp);
        assert_eq!(r.port, 4000);
        assert_eq!(r.target, EgressTarget::Host);
    }

    #[test]
    fn egress_rule_https_is_tcp_443_to_domains() {
        let r = EgressRule::https(&["a.com", "b.com"]);
        assert_eq!(r.protocol, Protocol::Tcp);
        assert_eq!(r.port, 443);
        assert_eq!(
            r.target,
            EgressTarget::Domains(vec!["a.com".to_string(), "b.com".to_string()])
        );
    }

    #[test]
    fn egress_rule_agent_base_composes_dns_litellm_github() {
        let rules = EgressRule::agent_base();
        assert_eq!(rules.len(), 4);
        assert_eq!(rules[2], EgressRule::litellm_proxy());
        assert_eq!(rules[3], EgressRule::https(&["github.com", "api.github.com"]));
    }

    // ---- HostBoundSecret ----

    #[test]
    fn host_bound_secret_from_definition_templates_value_and_propagates_meta() {
        let def = definition("MY_SECRET");
        let bound = HostBoundSecret::from(&def);
        assert_eq!(bound.name, "MY_SECRET");
        assert_eq!(bound.value, "${MY_SECRET}");
        assert_eq!(bound.allowed_hosts, vec!["example.com".to_string()]);
        assert!(bound.required);
        assert_eq!(bound.reject_placeholder, Some("CHANGEME".to_string()));
        assert_eq!(
            bound.on_violation,
            SecretViolationPolicy::BlockAndLog,
            "the violation policy propagates from the definition"
        );
    }

    // ---- ADR 0030 Phase 1: instance policy rendering ----

    /// A plan carrying a declared instance policy renders the four
    /// `instance:` lines (strategy / on_conflict / port / label).
    #[test]
    fn plan_display_renders_instance_policy() {
        let plan = SandboxPlan {
            name: "demo".to_string(),
            image: None,
            workdir: None,
            command: vec![],
            cpus: None,
            memory_mib: None,
            env: vec![],
            secret_env: vec![],
            ports: vec![],
            mounts: vec![],
            network: NetworkPlan {
                egress_default_deny: false,
                ingress_default_deny: false,
                egress_rules: vec![],
                deny_rules: vec![],
                ingress_rules: vec![],
            },
            instance_policy: Some(crate::config::InstancePolicy {
                strategy: crate::config::InstanceStrategy::Parallel,
                on_conflict: Some(crate::config::DepConflict(vec![
                    crate::config::ConflictStep::Reuse,
                    crate::config::ConflictStep::Start,
                    crate::config::ConflictStep::Replace,
                ])),
                port: Some(crate::config::InstancePort::Preferred(
                    crate::config::types::PreferredPort {
                        preferred: 4000,
                        on_occupied: Some(crate::config::PortOccupiedChain(vec![
                            crate::config::PortOccupiedStep::Bare(
                                crate::config::PortOccupiedBare::Increment,
                            ),
                            crate::config::PortOccupiedStep::Bare(
                                crate::config::PortOccupiedBare::Auto,
                            ),
                        ])),
                    },
                )),
                on_skew: None,
                label: Some("dev".to_string()),
            }),
        };
        let rendered = format!("{plan}");
        assert!(
            rendered.contains("instance: strategy=parallel\n"),
            "strategy line: {rendered}"
        );
        assert!(
            rendered.contains("instance: on_conflict=[reuse, start, replace]\n"),
            "on_conflict line: {rendered}"
        );
        assert!(
            rendered.contains("instance: port=preferred=4000, on_occupied=[increment, auto]\n"),
            "port line: {rendered}"
        );
        assert!(
            rendered.contains("instance: label=dev\n"),
            "label line: {rendered}"
        );
    }

    /// A plan with `instance_policy: None` renders NO `instance:` lines —
    /// legacy output stays byte-identical (the existing golden plan).
    #[test]
    fn plan_display_omits_policy_when_none() {
        let plan = SandboxPlan {
            name: "bare".to_string(),
            image: None,
            workdir: None,
            command: vec![],
            cpus: None,
            memory_mib: None,
            env: vec![],
            secret_env: vec![],
            ports: vec![],
            mounts: vec![],
            network: NetworkPlan {
                egress_default_deny: false,
                ingress_default_deny: false,
                egress_rules: vec![],
                deny_rules: vec![],
                ingress_rules: vec![],
            },
            instance_policy: None,
        };
        let rendered = format!("{plan}");
        assert_eq!(
            rendered,
            "name: bare\nnetwork: egress_default=allow ingress_default=allow\n"
        );
        assert!(
            !rendered.contains("instance:"),
            "no policy must render no instance lines: {rendered}"
        );
    }
}
