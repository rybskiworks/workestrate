//! Config-domain type definitions: the `workestrate.toml` schema structs and
//! the tool-home registry schema structs, plus the field-name tables used for
//! unknown-field detection in overrides.
//!
//! NOTE (WP6(e)/C10): the MAIN config structs below all carry
//! #[serde(deny_unknown_fields)] so unknown fields in a config LAYER hard-error
//! at parse time. The user-global overrides path stays lenient:
//! `process_override_section` warns about AND strips unknown ConfigFile-level /
//! workload-level keys BEFORE the fragment is re-parsed via
//! `merge::Layer::from_string`, so override typos remain warnings, not errors.

use serde::ser::SerializeMap;
use serde::{de, Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

use crate::microsandbox::plan::{DenyDomainRule, IngressRule, MountPlan, PortMapping};
use crate::mount_policy::MountsFragment;
use crate::recipes::EgressRecipeRef;

/// How to obtain the sandbox image for a workload (`image = { ... }` inline
/// table in workestrate.toml). `recipe` selects the acquisition strategy
/// (e.g. `registry`, `local`); the remaining optional fields narrow it
/// (`ref`/`name`/`tag`/`contents`) or extend the image (`binary`,
/// `baked_files`, `features`). Unknown fields are rejected at parse time.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct ImageSpec {
    pub recipe: String,
    #[serde(rename = "ref")]
    pub reference: Option<String>,
    pub name: Option<String>,
    pub tag: Option<String>,
    pub contents: Option<Vec<String>>,
    pub binary: Option<BinarySpec>,
    pub baked_files: Option<Vec<BakedFileSpec>>,
    pub features: Option<Vec<String>>,
    /// Keep-last-N rung for this capsule's nix-layered image (ADR 0032
    /// §Image tags, RESOLVED user decision 3): the top rung of the cascade
    /// `crate::images::gc::DEFAULT_IMAGE_KEEP_LAST` < home settings
    /// (`RegistrySettings.image_keep_last`) < config-repo entry
    /// (`ConfigRepoEntry.image_keep_last`) < THIS field. First Some wins;
    /// an explicit 0 is a hard error (the just-loaded tag always counts
    /// toward N). Enforced at load time by prune-on-load.
    #[schemars(range(min = 1))]
    pub keep_last: Option<u32>,
}

/// A binary built from source and baked into the image (`image.binary`).
/// `recipe` selects the build strategy and `src` locates the source; the
/// optional fields tune the produced artifact (`entrypoint`, `worker`,
/// `npm_deps_hash`).
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct BinarySpec {
    pub recipe: String,
    pub src: String,
    pub entrypoint: Option<String>,
    pub worker: Option<String>,
    pub npm_deps_hash: Option<String>,
}

/// A single file baked into the image at build time (`image.baked_files`).
/// `path` is the in-image destination and `content` is the verbatim file body.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct BakedFileSpec {
    pub path: String,
    pub content: String,
}

/// One environment-variable entry in a workload's `env` list. Exactly one of
/// `value` (literal) or `secret` (reference to a `secrets.<name>` entry) is
/// expected to be set; `name` must be a valid shell env identifier.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct EnvVarConfig {
    pub name: String,
    pub value: Option<String>,
    pub secret: Option<String>,
}

/// Exposure mode of a workload env binding to a secret (spec 16). `host`
/// (the default; secure-by-default) renders the placeholder — the real value
/// is substituted by the egress rewrite only for hosts in the credential's
/// `allowed_hosts`. `guest` injects the real resolved value as a plain
/// sandbox env var (the explicit opt-in for verifier workloads).
///
/// TOML values: `bound = "host"` / `bound = "guest"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Bound {
    Host,
    Guest,
}

/// One secret reference as an `env` map value (specs 14/16): in the map form
/// `env = { KEY = { secret = "NAME", bound = "guest" } }`, the inline table
/// may carry ONLY a `secret` reference and a `bound` exposure mode — never a
/// `name` (the map key IS the name) and never a literal `value`. `secret` is
/// rename-only on the wire (it defaults to the map key at parse, so it is
/// always populated HERE); `bound` defaults to `host` at plan resolution
/// (defaults-after-merge — it stays `None` through parse and merge).
/// Deliberately distinct from [`EnvVarConfig`] so `deny_unknown_fields`
/// rejects `name`/`value`/typos inside a map value.
#[derive(Debug, Clone, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnvSecretRef {
    pub secret: String,
    #[serde(default)]
    pub bound: Option<Bound>,
}

/// Schemars-boundary-only shape of the secret-binding inline table (spec 16):
/// BOTH fields are optional on the wire (`secret` defaults to the map key,
/// `bound` defaults to `host` — neither default is visible in the wire
/// format). Distinct from the real [`EnvSecretRef`], whose `secret` is
/// required in Rust because the key-name default has already been applied at
/// parse time.
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)] // schemars-boundary-only: fields are never read in Rust code
struct EnvSecretRefShape {
    secret: Option<String>,
    bound: Option<Bound>,
}

/// Serde/schemars-boundary-only helper for the `env` map value forms
/// (specs 14/16): a bare string `"value"` is a literal value, a bare `true`
/// the same-name host-bound sugar, an inline table
/// `{ secret = "NAME", bound = "guest" }` a secret reference. This enum is
/// NEVER stored — it backs the schemars rendering of [`EnvBinding`] (the map
/// value in the env map), so the generated schema shows all value forms
/// (`string | boolean | { secret?, bound? }`).
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)] // serde/schemars-boundary-only: variants are never read in Rust code
enum EnvValueShorthand {
    Bare(String),
    Flag(bool),
    Full(EnvSecretRefShape),
}

/// Schemars-boundary-only helper for the `env` field shape (spec 14): the
/// field accepts EITHER the classic sequence of `[[env]]` entry tables OR
/// the map form `{ KEY = "value", KEY2 = { secret = "NAME" } }`. This enum
/// is NEVER constructed or deserialized — [`EnvBindings`]'s custom
/// deserializer normalizes both forms at parse time; it exists only so
/// `#[schemars(with = "EnvFieldShape")]` renders the field as
/// `anyOf [array-of-EnvVarConfig, object-map-of-EnvBinding]`.
#[derive(schemars::JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)] // schemars-boundary-only: variants are never constructed in Rust code
enum EnvFieldShape {
    Seq(Vec<EnvVarConfig>),
    Map(HashMap<String, EnvBinding>),
}

/// A single workload env binding (spec 16): the value of one entry in the
/// name-keyed, document-order-ordered `workloads.<name>.env` map. A bare
/// string is a literal value; `true` or an inline table
/// `{ secret = "ID", bound = "guest" }` is a secret reference whose
/// per-binding `bound` decides exposure — `host` (the default) renders a
/// host-bound placeholder plan entry, `guest` a real-value plan env entry
/// (the explicit opt-in for verifier workloads).
#[derive(Debug, Clone, PartialEq)]
pub enum EnvBinding {
    /// Literal value: `env = { KEY = "value" }`.
    Literal(String),
    /// Secret reference: `env = { KEY = { secret = "ID", bound = "guest" } }`
    /// (also the desugared form of `KEY = true` / `KEY = { bound = "guest" }`).
    Secret(EnvSecretRef),
}

/// Parse-time intermediate for the inline-table env map value (spec 16 §2):
/// identical to [`EnvSecretRef`] except `secret` is OPTIONAL — the key-name
/// default (`secret` defaults to the map KEY) is applied by
/// [`EnvBindingsVisitor::visit_map`] AFTER the raw value is parsed, which is
/// the only point where the key is known. `deny_unknown_fields` still
/// hard-rejects stray keys inside the inline table.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSecretRef {
    secret: Option<String>,
    bound: Option<Bound>,
}

/// Parse-time intermediate for one env map value: the wire forms a binding
/// accepts BEFORE the key-name default is applied (spec 16 §2). A bare
/// string is a literal; `true` is the same-name host-bound sugar; an inline
/// table is a (bound-only and/or rename) secret reference. `false` is a
/// hard error with a corrective hint.
enum RawBinding {
    Literal(String),
    Secret(RawSecretRef),
}

struct RawBindingVisitor;

impl<'de> de::Visitor<'de> for RawBindingVisitor {
    type Value = RawBinding;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(
            "a bare string literal, true (same-name host-bound secret), or an inline table \
             like { secret = \"NAME\" } / { bound = \"guest\" }",
        )
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(RawBinding::Literal(v.to_string()))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(RawBinding::Literal(v))
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if v {
            // Same-name host-bound sugar: `KEY = true` — the key-name default
            // fills `secret` at the EnvBindingsVisitor level.
            Ok(RawBinding::Secret(RawSecretRef {
                secret: None,
                bound: None,
            }))
        } else {
            Err(de::Error::custom(
                "env binding `false` has no meaning (a binding either binds or is absent); \
                 did you mean true?",
            ))
        }
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        // Delegate to the raw struct so `deny_unknown_fields` errors are
        // preserved verbatim for inline-table values.
        let raw = RawSecretRef::deserialize(de::value::MapAccessDeserializer::new(map))?;
        Ok(RawBinding::Secret(raw))
    }
}

impl<'de> Deserialize<'de> for RawBinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(RawBindingVisitor)
    }
}

impl Serialize for EnvBinding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            EnvBinding::Literal(value) => serializer.serialize_str(value),
            EnvBinding::Secret(ref_) => {
                let mut map =
                    serializer.serialize_map(Some(if ref_.bound.is_some() { 2 } else { 1 }))?;
                map.serialize_entry("secret", &ref_.secret)?;
                if let Some(bound) = ref_.bound {
                    map.serialize_entry("bound", &bound)?;
                }
                map.end()
            }
        }
    }
}

impl schemars::JsonSchema for EnvBinding {
    fn schema_name() -> String {
        "EnvBinding".to_string()
    }

    fn json_schema(gen: &mut schemars::SchemaGenerator) -> schemars::schema::Schema {
        // Render as the accepted wire forms (bare string | boolean | inline
        // table with optional `secret`/`bound`).
        EnvValueShorthand::json_schema(gen)
    }
}

/// The name-keyed, document-order-ordered `workloads.<name>.env` map,
/// implemented as a Vec of `(name, binding)` pairs so document order is
/// preserved exactly (no `indexmap` dependency — the same trick the spec-14
/// map visitor used).
///
/// The custom deserializer accepts BOTH:
///   (a) the map form `env = { KEY = "literal", KEY2 = true, KEY3 = { secret = "ID", bound = "guest" } }`
///       (spec 16: the map visitor applies the key-name default — a secret
///       binding that never names a secret binds the secret whose ID IS the
///       env name), and
///   (b) the legacy `[[env]]` array-of-[`EnvVarConfig`] form, normalized per
///       entry: value only → [`EnvBinding::Literal`], secret only →
///       [`EnvBinding::Secret`] (bound-less), neither → `Literal("")`, both →
///       a precise hard error ("cannot have both value and secret"). The
///       array form CANNOT express `bound`.
/// Serialization always emits the map form.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EnvBindings(Vec<(String, EnvBinding)>);

impl EnvBindings {
    /// Ordered iteration over `(name, binding)` pairs in document/merge order.
    pub fn iter(&self) -> std::slice::Iter<'_, (String, EnvBinding)> {
        self.0.iter()
    }

    /// Look up a binding by env-var name.
    pub fn get(&self, name: &str) -> Option<&EnvBinding> {
        self.0.iter().find(|(k, _)| k == name).map(|(_, v)| v)
    }

    /// Replace the binding for `name` in place (preserving position) when it
    /// exists, else append it. This is the merge primitive (union-by-name,
    /// last layer wins per key).
    pub fn upsert(&mut self, name: &str, binding: EnvBinding) {
        if let Some(slot) = self.0.iter_mut().find(|(k, _)| k == name) {
            slot.1 = binding;
        } else {
            self.0.push((name.to_string(), binding));
        }
    }

    /// Whether the map holds no bindings.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// The number of bindings.
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

struct EnvBindingsVisitor;

impl<'de> de::Visitor<'de> for EnvBindingsVisitor {
    type Value = EnvBindings;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(
            "a sequence of env entry tables, or a map of env names to bare string \
             literals, true, and/or inline tables like { secret = \"NAME\" } / \
             { bound = \"guest\" }",
        )
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        // Legacy `[[env]]` array-of-tables form. Elements are parsed as
        // `EnvVarConfig` exactly as the derived default did (so
        // `deny_unknown_fields`/unknown-field errors inside `[[env]]` tables
        // are preserved verbatim), then normalized to bindings. The array
        // form has no `bound` — secret-only entries are host-bound by
        // construction (bound: None; the real-value opt-in requires the map
        // form).
        let mut entries = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        let mut index = 0usize;
        while let Some(entry) = seq
            .next_element::<EnvVarConfig>()
            .map_err(|e| de::Error::custom(format_args!("env entry at index {index}: {e}")))?
        {
            let binding = match (entry.value, entry.secret) {
                (Some(value), None) => EnvBinding::Literal(value),
                (None, Some(secret)) => EnvBinding::Secret(EnvSecretRef {
                    secret,
                    bound: None,
                }),
                (None, None) => EnvBinding::Literal(String::new()),
                (Some(_), Some(_)) => {
                    return Err(de::Error::custom(format_args!(
                        "env '{}' cannot have both value and secret",
                        entry.name
                    )));
                }
            };
            entries.push((entry.name, binding));
            index += 1;
        }
        Ok(EnvBindings(entries))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        // Map form. Entries are collected in DOCUMENT ORDER — toml_edit's
        // `MapAccess` preserves it, and entries are pushed straight into the
        // output vec in iteration order (no intermediate map, no sorting;
        // duplicate keys are a free TOML-level error).
        //
        // Each value parses as a `RawBinding` (the pre-default wire forms);
        // the key-name default is then applied HERE — the only point where
        // the map key is known: a secret binding that never names a secret
        // (`KEY = true`, `KEY = { bound = "guest" }`) binds the secret whose
        // ID IS the env name (spec 16 §2). `bound` stays `Option` through
        // parse and merge; the `host` default applies only at plan
        // resolution (defaults-after-merge).
        let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(0));
        while let Some(key) = map.next_key::<String>()? {
            let raw = map
                .next_value::<RawBinding>()
                .map_err(|e| de::Error::custom(format_args!("env map entry '{key}': {e}")))?;
            let binding = match raw {
                RawBinding::Literal(value) => EnvBinding::Literal(value),
                RawBinding::Secret(raw_ref) => EnvBinding::Secret(EnvSecretRef {
                    secret: raw_ref.secret.unwrap_or_else(|| key.clone()),
                    bound: raw_ref.bound,
                }),
            };
            entries.push((key, binding));
        }
        Ok(EnvBindings(entries))
    }
}

impl<'de> Deserialize<'de> for EnvBindings {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(EnvBindingsVisitor)
    }
}

impl Serialize for EnvBindings {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, binding) in &self.0 {
            map.serialize_entry(key, binding)?;
        }
        map.end()
    }
}

/// A file copied from the host into the sandbox at start time
/// (`workloads.<name>.seed_files`). `source` is the host path (validated at
/// the trust boundary); it is `None` when the entry uses `glob` instead —
/// exactly one of `source`|`glob` per entry is enforced by
/// `config::validation`. `target` is rendered on the HOST: targets starting
/// with `workspaces/` or `var/` resolve under the XDG state dir, anything
/// else resolves under the declaring config layer's content root. The file
/// is guest-visible ONLY through a declared mount whose host path is a
/// component-wise prefix of the target (seeds are mount-backed by design);
/// validation fails closed on an uncovered target — and `prepare()` re-checks
/// before any write — so a seed can never silently render on the host where
/// the guest never sees it. `only_if_missing` skips the copy when the target
/// already exists. When
/// `template` is true, the source TEXT is rendered as a `${VAR}` template
/// against the workload's guest-visible env view at seed time (`$$` emits a
/// literal `$`, so `$${FOO}` renders as `${FOO}`; map-only resolver; missing
/// var = hard error; no process env). `glob` is a glob pattern relative to
/// the declaring layer's content dir: `target` becomes a directory and each
/// regular-file match seeds to `target/<rel-path>`, sorted; a glob with no
/// matches is a hard error at seed time; mutually exclusive with `source`.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct SeedFileConfig {
    pub source: Option<String>,
    pub target: String,
    pub only_if_missing: Option<bool>,
    /// When true, render the source TEXT as a `${VAR}` template against the
    /// workload's guest-visible env view at seed time (`$$` emits a literal
    /// `$`; a missing var is a hard error).
    #[serde(default)]
    pub template: bool,
    /// Glob pattern (relative to the declaring layer's content dir): `target`
    /// becomes a directory and each sorted regular-file match seeds to
    /// `target/<rel-path>`. No match = hard error at seed time. Mutually
    /// exclusive with `source`.
    #[serde(default)]
    pub glob: Option<String>,
}

/// Build the workload from a local source checkout instead of pulling an
/// image (`workloads.<name>.local_build`). `recipe` selects the build
/// strategy, `source` the checkout location; the optional fields drive
/// incremental-build gating and fallback behaviour.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct LocalBuildConfig {
    pub recipe: String,
    pub source: String,
    pub requirements_file: Option<String>,
    pub target: Option<String>,
    pub gating_file: Option<String>,
    pub env_override: Option<String>,
    pub fallback: Option<String>,
}

/// Default per-direction action for a workload's network policy
/// (`[...network.defaults] egress|ingress = "allow" | "deny"`). `Deny` is
/// the fail-closed default; `Allow` requires the workload to declare the
/// matching `default_egress_allow` / `default_ingress_allow` entitlement
/// (checked at merge and validate time).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DefaultAction {
    /// Fail-open: all traffic in that direction allowed unless a `deny`
    /// rule matches.
    Allow,
    /// Fail-closed: all traffic in that direction denied unless a matching
    /// recipe/rule allows it.
    Deny,
}

/// Per-workload network defaults (`[...network.defaults]`). `egress` and
/// `ingress` are the per-direction fail-closed switches (absent = `deny`;
/// tightening to `deny` is always allowed across layers, relaxing to `allow`
/// requires the workload's declared `default_egress_allow` /
/// `default_ingress_allow` entitlement).
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct NetworkDefaultsConfig {
    pub egress: Option<DefaultAction>,
    pub ingress: Option<DefaultAction>,
}

/// Friendly-cutover deserializer for the REMOVED `default_deny` key: any
/// workload TOML still carrying `default_deny` hard-errors with a targeted
/// migration message instead of a bare serde unknown-field error. Only runs
/// when the key is present (absent hits `#[serde(default)]`).
fn removed_default_deny<'de, D>(deserializer: D) -> Result<Option<()>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let _ = serde::de::IgnoredAny::deserialize(deserializer)?;
    Err(serde::de::Error::custom(
        "network.default_deny was removed — use [network.defaults] egress = \"deny\" (absent = deny; \"allow\" requires entitlements = [\"default_egress_allow\"])",
    ))
}

/// Per-workload network policy (`workloads.<name>.network`). `defaults.egress`
/// / `defaults.ingress` are the per-direction fail-closed switches (relaxing
/// to `allow` requires the declared `default_egress_allow` /
/// `default_ingress_allow` entitlement); `egress` lists allowed egress
/// recipes, `deny` explicit domain-suffix denials, and `ingress` inbound
/// exposure rules.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct NetworkConfig {
    #[serde(default, deserialize_with = "removed_default_deny", skip_serializing)]
    #[schemars(skip)]
    #[doc(hidden)]
    pub default_deny: Option<()>,
    pub defaults: Option<NetworkDefaultsConfig>,
    #[serde(default)]
    pub egress: Vec<EgressRecipeRef>,
    #[serde(default)]
    pub deny: Vec<DenyDomainRule>,
    #[serde(default)]
    pub ingress: Vec<IngressRule>,
}

/// One step in a conflict chain (ADR 0030 addendum 2). A disposition a
/// lifecycle verb tries against an occupied slot, in chain order, until one
/// applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConflictStep {
    /// Use the running instance when healthy (500ms host TCP port probe) or
    /// still booting (<30s record). Not applicable to a stopped/crashed row.
    Reuse,
    /// Start a stopped/crashed sandbox (msb `handle.start()`); on failure the
    /// executor continues the chain.
    Start,
    /// Always down/remove + fresh create.
    Replace,
    /// Refuse with the standard occupied-instance message (terminal; nothing
    /// may follow it in a chain).
    Fail,
}

/// Schemars-boundary-only helper for the `on_conflict` field shape (ADR 0030
/// addendum 2): the field accepts EITHER a scalar string (back-compat,
/// d452575) OR an ordered list of strings. Never constructed/deserialized —
/// `DepConflict`'s custom deserializer normalizes both forms at parse time.
#[derive(schemars::JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum ConflictChainShape {
    Scalar(ConflictStep),
    Chain(Vec<ConflictStep>),
}

/// Auto-start conflict policy for a dependency (`depends_on.<dep>.on_conflict`;
/// ADR 0026 addendum 2026-08-16; ADR 0030 addendum 2). An ORDERED CHAIN of
/// steps attempted until one succeeds. TOML accepts a scalar (back-compat:
/// d452575's `"reuse"` parses as `["reuse"]`) or an ordered list over
/// {reuse, start, replace, fail}.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DepConflict(pub Vec<ConflictStep>);

impl schemars::JsonSchema for DepConflict {
    fn schema_name() -> String {
        "DepConflict".to_string()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        // The spec's `#[schemars(with = "ConflictChainShape")]` attribute only
        // takes effect through the derive macro; with the manual Deserialize
        // impl the struct cannot derive JsonSchema, so delegate to the
        // boundary helper directly (same rendered shape: scalar | list).
        ConflictChainShape::json_schema(gen)
    }
}

/// Validate a conflict chain per ADR 0030 addendum 2 (U3): non-empty, no
/// duplicates, no elements after `fail` (terminal). Unknown elements are
/// rejected by serde's closed vocabulary before this runs.
pub(crate) fn validate_conflict_chain(steps: &[ConflictStep]) -> Result<(), String> {
    if steps.is_empty() {
        return Err(
            "conflict chain must not be empty (on_conflict needs at least one element)".to_string(),
        );
    }
    let mut seen = std::collections::HashSet::new();
    for (i, step) in steps.iter().enumerate() {
        if !seen.insert(*step) {
            return Err(format!(
                "conflict chain contains duplicate element '{}' at position {}",
                step_name(*step),
                i
            ));
        }
        if *step == ConflictStep::Fail && i + 1 < steps.len() {
            return Err(format!(
                "conflict chain: 'fail' is terminal; no elements may follow it (element {} is after 'fail')",
                step_name(steps[i + 1])
            ));
        }
    }
    Ok(())
}

fn step_name(step: ConflictStep) -> &'static str {
    match step {
        ConflictStep::Reuse => "reuse",
        ConflictStep::Start => "start",
        ConflictStep::Replace => "replace",
        ConflictStep::Fail => "fail",
    }
}

impl fmt::Display for ConflictStep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Reuse => "reuse",
            Self::Start => "start",
            Self::Replace => "replace",
            Self::Fail => "fail",
        };
        f.write_str(s)
    }
}

impl DepConflict {
    /// d452575 back-compat singleton: `on_conflict = "reuse"`.
    pub fn reuse() -> Self {
        Self(vec![ConflictStep::Reuse])
    }
    /// d452575 back-compat singleton: `on_conflict = "replace"`.
    pub fn replace() -> Self {
        Self(vec![ConflictStep::Replace])
    }
    /// d452575 back-compat singleton: `on_conflict = "fail"`.
    pub fn fail() -> Self {
        Self(vec![ConflictStep::Fail])
    }
    /// ADR 0030 default chain: reuse-if-healthy, start-if-stopped,
    /// replace-if-zombie/stale.
    pub fn default_chain() -> Self {
        Self(vec![
            ConflictStep::Reuse,
            ConflictStep::Start,
            ConflictStep::Replace,
        ])
    }
}

impl<'de> Deserialize<'de> for DepConflict {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct DepConflictVisitor;

        impl<'de> de::Visitor<'de> for DepConflictVisitor {
            type Value = DepConflict;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(
                    "a conflict chain: a scalar \"reuse\"/\"start\"/\"replace\"/\"fail\" \
                     or an ordered list of them",
                )
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                // Scalar back-compat (d452575): normalize to a singleton chain.
                let step = ConflictStep::deserialize(de::value::StrDeserializer::new(v))?;
                Ok(DepConflict(vec![step]))
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut steps = Vec::new();
                while let Some(step) = seq.next_element::<ConflictStep>()? {
                    steps.push(step);
                }
                validate_conflict_chain(&steps).map_err(de::Error::custom)?;
                Ok(DepConflict(steps))
            }
        }

        deserializer.deserialize_any(DepConflictVisitor)
    }
}

// ---------------------------------------------------------------------------
// ADR 0030 §4.1 + addendum 2 U6: the per-workload instance policy
// ---------------------------------------------------------------------------

/// `[workloads.<name>.instance]` — the per-workload instance policy (ADR 0030
/// §4.1): the instance model the workload defaults to, the conflict chain when
/// the target slot is occupied, the port strategy, and an optional display
/// label. Absent = current behavior. NOTE: named `InstancePolicy` (not
/// "InstanceSpec") to avoid colliding with the runtime `InstanceSpec`.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstancePolicy {
    /// Instance model the workload defaults to (default "singleton" — current
    /// behavior). `parallel`/`replace`/`reuse` SEMANTICS land with Phase 2
    /// (selection/disposition wiring); Phase 1 parses + validates the closed
    /// vocabulary.
    #[serde(default)]
    pub strategy: InstanceStrategy,
    /// Conflict chain when the target slot is occupied (scalar-or-list over
    /// reuse|start|replace|fail; `None` → the built-in default
    /// ["reuse","start","replace"] at decision time; ADR 0030 U11 precedence:
    /// CLI flag > this declared chain > built-in default).
    #[serde(default)]
    pub on_conflict: Option<DepConflict>,
    /// Port strategy: strict integer | "auto" | { preferred, on_occupied }
    /// (ADR 0030 addendum 2 U6). Selection behavior is Phase 3; Phase 1
    /// parses + validates.
    #[serde(default)]
    pub port: Option<InstancePort>,
    /// Config/image divergence policy for a RUNNING instance (ADR 0030
    /// V-addendum §V4): warn (default) | replace | reuse-silently. `None` →
    /// warn at decision time (mirrors the `on_conflict` Option pattern).
    #[serde(default)]
    pub on_skew: Option<OnSkew>,
    /// Optional display/version identity label (ADR 0030 §4.5 / Q4 lighter
    /// option). Free-form string.
    #[serde(default)]
    pub label: Option<String>,
}

/// The instance model a workload defaults to (ADR 0030 §4.1 `strategy`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum InstanceStrategy {
    #[default]
    Singleton,
    Parallel,
    Replace,
    Reuse,
    /// One instance per working directory (ADR 0030 V-addendum §V1): the
    /// instance id is `<dirname-slug>-<shorthash>` of the CANONICALIZED
    /// invocation cwd. `rename_all = "snake_case"` would render this
    /// `per_dir` — the pinned config vocabulary is kebab-case `per-dir`.
    #[serde(rename = "per-dir")]
    PerDir,
}

impl fmt::Display for InstanceStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Singleton => "singleton",
            Self::Parallel => "parallel",
            Self::Replace => "replace",
            Self::Reuse => "reuse",
            Self::PerDir => "per-dir",
        };
        f.write_str(s)
    }
}

/// The `instance.on_skew` divergence policy knob (ADR 0030 V-addendum §V4):
/// what to do when the current build inputs diverge from a RUNNING
/// instance's recorded provenance stamps. `Warn` is the default (divergence
/// is common and usually benign; silent reuse hides real drift; auto-replace
/// destroys instances the operator may want).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, schemars::JsonSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum OnSkew {
    /// Proceed (reuse/start per the conflict chain) and print the divergence.
    #[default]
    Warn,
    /// Tear down and start fresh on the new inputs.
    Replace,
    /// Adopt the running instance without comment.
    ReuseSilently,
}

impl fmt::Display for OnSkew {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Warn => "warn",
            Self::Replace => "replace",
            Self::ReuseSilently => "reuse-silently",
        };
        f.write_str(s)
    }
}

/// `workloads.<name>.instance.port` — the workload's port strategy (ADR 0030
/// addendum 2 U6). Untagged union: strict integer | "auto" | { preferred,
/// on_occupied }. Manual Serialize/Deserialize (the spec's derived untagged
/// shape cannot parse the UNIT variant `Auto` from the string "auto", and
/// untagged enums swallow the nested `on_occupied` chain's helpful validation
/// errors into a generic "did not match any variant" message).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstancePort {
    /// `port = N`: strict — declared ports as-is; an occupied preferred port
    /// fails (the ADR's "current behavior"; semantically
    /// `{ preferred = N, on_occupied = "fail" }`).
    Strict(u16),
    /// `port = "auto"`: every declared port auto-allocates at boot.
    Auto,
    /// `port = { preferred = N, on_occupied = ... }`.
    Preferred(PreferredPort),
}

/// Schemars-boundary-only helper for the `instance.port` field shape (ADR
/// 0030 addendum 2 U6): strict integer | "auto" | { preferred, on_occupied }.
/// The `auto` form is a newtype over a string enum so it renders as the
/// string `"auto"` (a unit variant in an untagged enum renders as `null`).
/// Never constructed/deserialized.
#[derive(schemars::JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum InstancePortShape {
    Strict(u16),
    Auto(PortAuto),
    Preferred(PreferredPort),
}

/// The `"auto"` string form of `instance.port` (schema-rendering only).
#[derive(schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum PortAuto {
    Auto,
}

impl schemars::JsonSchema for InstancePort {
    fn schema_name() -> String {
        "InstancePort".to_string()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        InstancePortShape::json_schema(gen)
    }
}

impl Serialize for InstancePort {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Manual: the derived untagged impl would serialize the unit variant
        // `Auto` as a null/unit — the wire form is the string "auto".
        match self {
            InstancePort::Strict(n) => serializer.serialize_u16(*n),
            InstancePort::Auto => serializer.serialize_str("auto"),
            InstancePort::Preferred(p) => p.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for InstancePort {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct InstancePortVisitor;

        impl<'de> de::Visitor<'de> for InstancePortVisitor {
            type Value = InstancePort;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(
                    "a strict integer port, \"auto\", or a { preferred, on_occupied } table",
                )
            }

            fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
                u16::try_from(v)
                    .map(InstancePort::Strict)
                    .map_err(de::Error::custom)
            }

            fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
                u16::try_from(v)
                    .map(InstancePort::Strict)
                    .map_err(de::Error::custom)
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                if v == "auto" {
                    Ok(InstancePort::Auto)
                } else {
                    Err(de::Error::custom(format!(
                        "unknown instance port strategy '{v}' (expected \"auto\")"
                    )))
                }
            }

            fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                PreferredPort::deserialize(de::value::MapAccessDeserializer::new(map))
                    .map(InstancePort::Preferred)
            }
        }

        deserializer.deserialize_any(InstancePortVisitor)
    }
}

/// The preferred-port form of `instance.port` (ADR 0030 addendum 2 U6).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreferredPort {
    /// The port to try first.
    pub preferred: u16,
    /// What to do when `preferred` is occupied: scalar-or-list chain over
    /// auto|increment|fail. `None` → the default chain ["increment", "auto"]
    /// at decision time (Phase 3).
    #[serde(default)]
    pub on_occupied: Option<PortOccupiedChain>,
}

/// A port `on_occupied` chain: scalar-or-list over {auto, increment, fail}
/// (ADR 0030 addendum 2 U6). Scalar normalizes to a singleton; validation:
/// non-empty, no duplicates (by KIND), no elements after the terminal 'fail'.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortOccupiedChain(pub Vec<PortOccupiedStep>);

/// Schemars-boundary-only helper for the `on_occupied` field shape (ADR 0030
/// addendum 2 U6): scalar or ordered list. Never constructed/deserialized.
#[derive(schemars::JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)]
enum PortOccupiedChainShape {
    Scalar(PortOccupiedStep),
    Chain(Vec<PortOccupiedStep>),
}

impl schemars::JsonSchema for PortOccupiedChain {
    fn schema_name() -> String {
        "PortOccupiedChain".to_string()
    }

    fn json_schema(gen: &mut schemars::gen::SchemaGenerator) -> schemars::schema::Schema {
        // Same boundary-helper pattern as `DepConflict`: the manual
        // Deserialize impl prevents the struct from deriving JsonSchema, so
        // delegate to the shape helper (scalar | list).
        PortOccupiedChainShape::json_schema(gen)
    }
}

impl PortOccupiedChain {
    /// Singleton `"auto"` chain.
    pub fn auto() -> Self {
        Self(vec![PortOccupiedStep::Bare(PortOccupiedBare::Auto)])
    }
    /// Singleton `"increment"` chain.
    pub fn increment() -> Self {
        Self(vec![PortOccupiedStep::Bare(PortOccupiedBare::Increment)])
    }
    /// Singleton `"fail"` chain.
    pub fn fail() -> Self {
        Self(vec![PortOccupiedStep::Bare(PortOccupiedBare::Fail)])
    }
    /// The U6 recommended default when `preferred` is occupied:
    /// `["increment", "auto"]`.
    pub fn default_on_occupied() -> Self {
        Self(vec![
            PortOccupiedStep::Bare(PortOccupiedBare::Increment),
            PortOccupiedStep::Bare(PortOccupiedBare::Auto),
        ])
    }
}

impl<'de> Deserialize<'de> for PortOccupiedChain {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PortOccupiedChainVisitor;

        impl<'de> de::Visitor<'de> for PortOccupiedChainVisitor {
            type Value = PortOccupiedChain;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(
                    "a port on_occupied chain: a scalar \"auto\"/\"increment\"/\"fail\" \
                     or { increment = ... }, or an ordered list of them",
                )
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                // Scalar string form: normalize to a singleton chain.
                let step = PortOccupiedStep::deserialize(de::value::StrDeserializer::new(v))?;
                let steps = vec![step];
                validate_port_occupied_chain(&steps).map_err(de::Error::custom)?;
                Ok(PortOccupiedChain(steps))
            }

            fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                // Scalar table form: `{ increment = { limit|range = ... } }`.
                let step =
                    PortOccupiedStep::deserialize(de::value::MapAccessDeserializer::new(map))?;
                let steps = vec![step];
                validate_port_occupied_chain(&steps).map_err(de::Error::custom)?;
                Ok(PortOccupiedChain(steps))
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut steps = Vec::new();
                while let Some(step) = seq.next_element::<PortOccupiedStep>()? {
                    steps.push(step);
                }
                validate_port_occupied_chain(&steps).map_err(de::Error::custom)?;
                Ok(PortOccupiedChain(steps))
            }
        }

        deserializer.deserialize_any(PortOccupiedChainVisitor)
    }
}

/// One step in a port `on_occupied` chain (ADR 0030 addendum 2 U6): a bare
/// string auto|increment|fail, or a parameterized increment table. Manual
/// Deserialize (the untagged derive swallows an unknown element's "unknown
/// variant" error into a generic "did not match any variant" message).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum PortOccupiedStep {
    /// `"auto"` / `"increment"` / `"fail"` (bare).
    Bare(PortOccupiedBare),
    /// `{ increment = { limit = N } }` or `{ increment = { range = [S, E] } }`.
    Increment(ParameterizedIncrement),
}

impl<'de> Deserialize<'de> for PortOccupiedStep {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct PortOccupiedStepVisitor;

        impl<'de> de::Visitor<'de> for PortOccupiedStepVisitor {
            type Value = PortOccupiedStep;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str(
                    "a bare \"auto\"/\"increment\"/\"fail\" string or a { increment = ... } table",
                )
            }

            fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
                PortOccupiedBare::deserialize(de::value::StrDeserializer::new(v))
                    .map(PortOccupiedStep::Bare)
            }

            fn visit_map<A: de::MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                ParameterizedIncrement::deserialize(de::value::MapAccessDeserializer::new(map))
                    .map(PortOccupiedStep::Increment)
            }
        }

        deserializer.deserialize_any(PortOccupiedStepVisitor)
    }
}

/// Bare `on_occupied` elements (ADR 0030 addendum 2 U6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PortOccupiedBare {
    Auto,
    Increment,
    Fail,
}

impl fmt::Display for PortOccupiedBare {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Auto => "auto",
            Self::Increment => "increment",
            Self::Fail => "fail",
        };
        f.write_str(s)
    }
}

/// The parameterized `increment` element: `on_occupied = { increment = ... }`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ParameterizedIncrement {
    pub increment: IncrementSpec,
}

/// The increment band: either `{ limit = N }` (count-bound, relative:
/// preferred+1 .. preferred+N) or `{ range = [START, END] }` (absolute band,
/// scanned in order). Exactly ONE of the two must be present (validated in
/// config/validation.rs).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IncrementSpec {
    /// Count-bound relative form: candidates `preferred+1 .. preferred+N`.
    pub limit: Option<u16>,
    /// Absolute band form: candidates `START..=END`.
    pub range: Option<(u16, u16)>,
}

/// The semantic kind of an `on_occupied` step (dup check is by KIND: bare
/// "increment" and `{ increment = {...} }` are the same kind).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PortOccupiedKind {
    Auto,
    Increment,
    Fail,
}

pub(crate) fn port_occupied_kind(step: &PortOccupiedStep) -> PortOccupiedKind {
    match step {
        PortOccupiedStep::Bare(b) => match b {
            PortOccupiedBare::Auto => PortOccupiedKind::Auto,
            PortOccupiedBare::Increment => PortOccupiedKind::Increment,
            PortOccupiedBare::Fail => PortOccupiedKind::Fail,
        },
        PortOccupiedStep::Increment(_) => PortOccupiedKind::Increment,
    }
}

/// Validate a port `on_occupied` chain (ADR 0030 addendum 2 U6, same rules as
/// on_conflict): non-empty, no duplicates (by kind), nothing after 'fail'.
pub(crate) fn validate_port_occupied_chain(steps: &[PortOccupiedStep]) -> Result<(), String> {
    if steps.is_empty() {
        return Err("on_occupied chain must not be empty (needs at least one element)".to_string());
    }
    let mut seen = std::collections::HashSet::new();
    for (i, step) in steps.iter().enumerate() {
        let kind = port_occupied_kind(step);
        if !seen.insert(kind) {
            return Err(format!(
                "on_occupied chain contains duplicate element at position {}",
                i
            ));
        }
        if kind == PortOccupiedKind::Fail && i + 1 < steps.len() {
            return Err(format!(
                "on_occupied chain: 'fail' is terminal; no elements may follow it (position {})",
                i + 1
            ));
        }
    }
    Ok(())
}

/// The instance model a `depends_on.<dep>` entry selects for its dependency
/// (ADR 0030 §4.1 T2). `Shared` (default) = the dependency's singleton slot in
/// the dependent's namespace (today's model); `Scoped` = a dep instance named
/// after the DEPENDENT's instance id (`litellm@<dependent-id>` for dependent
/// `prime@<id>`); `Fresh` = a fresh auto-slugged dep instance per start.
///
/// When unset, the mode derives from the DEP's own `instance.strategy`:
/// singleton/replace/reuse → [`DepInstanceMode::Shared`]; parallel →
/// [`DepInstanceMode::Fresh`] (see `commands::deps::dep_instance_mode`).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize, Serialize, schemars::JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum DepInstanceMode {
    #[default]
    Shared,
    Scoped,
    Fresh,
}

impl fmt::Display for DepInstanceMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Shared => "shared",
            Self::Scoped => "scoped",
            Self::Fresh => "fresh",
        };
        f.write_str(s)
    }
}

/// A single dependency declaration of a workload
/// (`workloads.<name>.depends_on.<dep>` in workestrate.toml; ADR 0026(d)
/// discovery-lite). `env` (optional) names the environment variable the
/// resolved address of dependency `<dep>`'s primary/unnamed port is injected
/// as; `required` (default false) makes a not-running dependency a plan-time
/// refusal instead of a skip; `exports` maps named-port -> env var, injecting
/// the resolved host address of each named port (one env var per named port).
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct DependsOnSpec {
    /// Env var the resolved address of dependency `<dep>`'s primary/unnamed
    /// port is injected as (legacy primary form). Optional: an exports-only
    /// dependency may omit it.
    pub env: Option<String>,
    #[serde(default)]
    pub required: bool,
    /// Named-port exports: port name -> env var name. Each entry injects the
    /// resolved host address of that named port as one env var per named
    /// port; every key must name a port the dependency declares.
    #[serde(default)]
    pub exports: HashMap<String, String>,
    /// Auto-start conflict policy when the dependency's singleton slot is
    /// already occupied (ADR 0026 addendum 2026-08-16; ADR 0030 addendum 2).
    /// Accepts a scalar (back-compat: d452575's `"reuse"` parses as
    /// `["reuse"]`) or an ordered list over {reuse, start, replace, fail}.
    /// `None` (default) resolves to the default chain
    /// `["reuse", "start", "replace"]` at decision time: reuse the running
    /// instance when healthy (host port probe) or still booting, start a
    /// stopped/crashed sandbox, then replace (down + start fresh) a
    /// keep-alive zombie or stale record.
    #[serde(default)]
    pub on_conflict: Option<DepConflict>,
    /// The instance model this dependency entry selects (ADR 0030 §4.1 T2).
    /// `None` (default) derives from the DEP's own `instance.strategy` at
    /// decision time (parallel → fresh, else shared).
    #[serde(default)]
    pub instance: Option<DepInstanceMode>,
}

/// A single workload definition (`workloads.<name>` in workestrate.toml).
/// `kind` is `"agent"` (interactive TUI attach) or `"service"` (headless,
/// detached by default); the remaining fields describe the image, resources,
/// command, env/secret wiring, mounts, ports, seed files, local-build
/// override, network policy, and dependency declarations. All fields merge
/// layer-by-layer via `merge::merge_layers`.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct WorkloadConfig {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub image: ImageSpec,
    pub workdir: Option<String>,
    pub cpus: Option<u8>,
    pub memory_mib: Option<u32>,
    #[serde(default)]
    pub command: Vec<String>,
    pub log_stop_errors: Option<bool>,
    #[serde(default)]
    #[schemars(with = "EnvFieldShape")]
    pub env: EnvBindings,
    #[serde(default)]
    pub ports: Vec<PortMapping>,
    #[serde(default)]
    pub mounts: Vec<MountPlan>,
    #[serde(default)]
    pub seed_files: Vec<SeedFileConfig>,
    pub local_build: Option<LocalBuildConfig>,
    #[serde(default)]
    pub network: NetworkConfig,
    #[serde(default)]
    pub policy: PolicyConfig,
    /// Per-workload instance policy (`[workloads.<name>.instance]`; ADR 0030
    /// §4.1). Absent = current behavior; Phase 1 parses + validates, the
    /// semantics land in Phases 2–3.
    #[serde(default)]
    pub instance: InstancePolicy,
    /// Config-declared entitlements (`workloads.<name>.entitlements`). The
    /// closed vocabulary core understands lives in `config::validation`
    /// (currently only `"default_egress_allow"`, which permits
    /// `network.defaults.egress = "allow"` — fail-closed: setting
    /// `egress = "allow"` WITHOUT the declared entitlement is a hard
    /// error at merge and validate time). Layers merge union-style with
    /// dedup (an entitlement, once granted by any layer, cannot be revoked
    /// by a later layer).
    #[serde(default)]
    pub entitlements: Vec<String>,
    /// Dependency declarations (`workloads.<name>.depends_on.<dep>`; ADR
    /// 0026(d)): each entry names another workload whose address is resolved
    /// from the port registry and injected as the declared env var at plan
    /// time. Layers merge union-by-dependency-name, last layer wins per dep.
    #[serde(default)]
    pub depends_on: HashMap<String, DependsOnSpec>,
}

/// Per-secret egress violation policy (`secrets.<name>.on_violation`): what
/// the host-side network proxy does when the secret's `$MSB_<NAME>`
/// placeholder appears in traffic to a host that is NOT in the secret's
/// `allowed_hosts` (including request BODIES, where a quoted placeholder
/// would otherwise poison the session — upstream microsandbox#1354).
///
/// The sandbox only ever sees the placeholder; the proxy substitutes the
/// real value exclusively for allowed hosts, so every variant here governs
/// the fate of the harmless placeholder TEXT, never the credential itself.
/// Serde kebab-case yields the TOML strings `"passthrough"` | `"block"` |
/// `"block-and-log"` | `"block-and-terminate"`, matching the SDK's
/// `ViolationAction` naming.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, Default, PartialEq, Eq, schemars::JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum SecretViolationPolicy {
    /// Forward the placeholder unchanged to the non-allowed host (the
    /// engine's connection is not reset). DEFAULT — safe because the proxy
    /// never substitutes the real value for non-allowed hosts: passthrough
    /// only permits the harmless placeholder text to leave the sandbox;
    /// real-secret substitution still requires an allowed host + enabled
    /// injection location. Also works around upstream microsandbox#1354,
    /// where a placeholder quoted inside a request body (e.g. an LLM
    /// conversation echoing `$MSB_GITHUB_TOKEN`) blocks every subsequent
    /// request under the engine default.
    #[default]
    Passthrough,
    /// Drop/reset the connection carrying the placeholder to the
    /// non-allowed host, silently (no host-side warning).
    Block,
    /// Drop/reset the connection AND emit a host-side warning — the engine
    /// default when no per-secret policy is configured in the SDK.
    BlockAndLog,
    /// Drop/reset the connection, log, and terminate the sandbox session.
    BlockAndTerminate,
}

/// Definition of one named secret (`secrets.<name>` in workestrate.toml) —
/// the final unified model (spec 16): a pure catalog of the credential's
/// intrinsic properties. `env_var` is the host environment variable the
/// resolved value is read from (default: the secret ID); `allowed_hosts`
/// constrains which egress hosts may receive the value by substitution
/// (omitted → deny-all; valid regardless of binding mode); `required` makes
/// a missing value a hard error; `placeholder` is a known-bad value to
/// reject; `on_violation` selects the egress violation policy applied when
/// the placeholder reaches a non-allowed host (absent → passthrough,
/// post-merge). Exposure mode is NOT a def property — it lives at the binding
/// site (`EnvSecretRef.bound`). The retired v1/v2 keys (`hosts`, `delivery`,
/// `source`, `exposed_as`, `description`, stray `bound`) hard-error at parse
/// via `deny_unknown_fields`.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct SecretDefConfig {
    pub env_var: Option<String>,
    #[serde(default)]
    pub allowed_hosts: Option<Vec<String>>,
    pub required: Option<bool>,
    pub placeholder: Option<String>,
}

/// Top-level schema root of a `workestrate.toml` layer: `schema_version`
/// (must equal `EXPECTED_SCHEMA_VERSION`; absent warns and is treated as
/// legacy), plus the `secrets` and `workloads` maps. Layers are merged by
/// `merge::merge_layers` into one effective `ConfigFile`.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct ConfigFile {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub secrets: HashMap<String, SecretDefConfig>,
    #[serde(default)]
    pub workloads: HashMap<String, WorkloadConfig>,
    #[serde(default)]
    pub policy: PolicyConfig,
}

/// A policy namespace. Keeping the `mounts` table nested makes the config
/// surface match `[policy.mounts]` while leaving policy fragments outside the
/// ordinary merge algebra.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PolicyConfig {
    #[serde(default)]
    pub mounts: Option<MountsFragment>,
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Tool-wide settings section of the tool-home `registry.toml` (`[settings]`).
/// `default_context` selects the active context when none is given;
/// `store_dir`/`state_dir` override the derived store/state locations.
#[derive(Debug, Clone, Serialize, Deserialize, Default, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RegistrySettings {
    pub default_context: Option<String>,
    pub store_dir: Option<String>,
    pub state_dir: Option<String>,
    /// Layout version of the tool home. Absent ⇒ 1 (legacy XDG-derived layout).
    /// Set to 2 by `workestrate migrate-home` after consolidating into a single
    /// `WORKESTRATE_HOME` (ADR 0023). Purely informational/forward-compat: the
    /// [`HomeKind`] resolution already determines the active layout.
    #[serde(default)]
    pub home_version: Option<u32>,
    /// Home-wide keep-last-N rung for nix-layered image tags (ADR 0032
    /// §Image tags, RESOLVED user decision 3): middle rung of the cascade —
    /// beats the built-in default, loses to a config-repo entry and a
    /// workload capsule `keep_last`. `None` = not configured at this rung.
    #[serde(default)]
    pub image_keep_last: Option<u32>,
}

/// One registered config repo in the tool-home registry (`[configs.<name>]`).
/// `url` is the clone source (git URL, or a filesystem path for `config new`
/// repos); `ref`/`rev` track the checked-out branch and commit; the
/// `secrets*` fields locate that repo's encrypted secrets material.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfigRepoEntry {
    pub url: String,
    pub r#ref: Option<String>,
    pub rev: Option<String>,
    #[serde(default)]
    pub secrets: Option<String>, // "file" (default) | "none"
    #[serde(default)]
    pub secrets_file: Option<String>, // default ".env.enc"
    #[serde(default)]
    pub age_key_file: Option<String>, // default: SOPS_AGE_KEY_FILE env or default path
    /// Per-repo keep-last-N rung for nix-layered image tags (ADR 0032
    /// §Image tags, RESOLVED user decision 3): beats the home-settings and
    /// built-in-default rungs, loses to a workload capsule `keep_last`.
    /// `None` = not configured at this rung.
    #[serde(default)]
    pub image_keep_last: Option<u32>,
}

/// A resolved secrets layer for multi-layer secret loading.
#[derive(Debug, Clone)]
pub struct SecretsLayer {
    pub name: String,
    pub dir: PathBuf,
    pub secrets_file: String,
    pub age_key_file: Option<PathBuf>,
    pub skip: bool, // secrets = "none"
}

/// A project directory trusted for project-layer config loading
/// (`[[trusted_projects]]` in the registry). `path` is the canonicalized
/// directory; only trusted projects' `workestrate.toml`/`.workestrate/` are
/// honored during config/home resolution.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TrustedProject {
    pub path: String,
}

/// A named context in the registry (`[contexts.<name>]`): an ordered list of
/// config-layer names merged (earlier = lower precedence) when the context is
/// active.
#[derive(Debug, Clone, Serialize, Deserialize, Default, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Context {
    #[serde(default)]
    pub layers: Vec<String>,
}

/// Schema root of the tool-home `registry.toml`: global `[settings]`, the
/// registered config repos (`configs`), the default layer stack (`layers`),
/// named contexts (`contexts`), and the trusted-project list. Written
/// atomically by `config::save_registry`.
#[derive(Debug, Clone, Serialize, Deserialize, Default, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    #[serde(default)]
    pub settings: RegistrySettings,
    #[serde(default)]
    pub configs: HashMap<String, ConfigRepoEntry>,
    #[serde(default)]
    pub layers: Vec<String>,
    #[serde(default)]
    pub contexts: HashMap<String, Context>,
    #[serde(default)]
    pub trusted_projects: Vec<TrustedProject>,
    #[serde(default)]
    pub policy: PolicyConfig,
}

// ---------------------------------------------------------------------------
// User-global overrides field tables
// ---------------------------------------------------------------------------

/// Known top-level ConfigFile fields (for unknown-field detection in overrides).
pub(crate) const CONFIG_FIELDS: &[&str] = &["schema_version", "secrets", "workloads", "policy"];

/// Known WorkloadConfig fields (for unknown-field detection in overrides).
pub(crate) const WORKLOAD_FIELDS: &[&str] = &[
    "kind",
    "image",
    "workdir",
    "cpus",
    "memory_mib",
    "command",
    "log_stop_errors",
    "env",
    "ports",
    "mounts",
    "seed_files",
    "local_build",
    "network",
    "policy",
    "depends_on",
];

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;
    use crate::config::test_support::MINIMAL_VALID_TOML;

    // ---- FS-4: registry structs reject unknown fields (fail loudly on typos) ----

    /// A typo'd top-level registry key must hard-error at parse time, not be
    /// silently ignored (the same policy the workload config structs already
    /// enforce).
    #[test]
    fn registry_rejects_unknown_top_level_field() {
        let raw = "layers = []\nunknown_top = 1\n";
        let err = toml::from_str::<Registry>(raw).unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "expected unknown-field error, got: {err}"
        );
    }

    /// Nested unknown fields must fail at every registry sub-struct.
    #[test]
    fn registry_rejects_unknown_nested_fields() {
        // [settings] typo.
        let err =
            toml::from_str::<Registry>("[settings]\ndefault_contex = \"personal\"\n").unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "settings typo must fail: {err}"
        );

        // [configs.<name>] typo.
        let err = toml::from_str::<Registry>("[configs.personal]\nurl = \"x\"\nrevv = \"abc\"\n")
            .unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "configs typo must fail: {err}"
        );

        // [contexts.<name>] typo.
        let err = toml::from_str::<Registry>("[contexts.personal]\nlayerz = []\n").unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "contexts typo must fail: {err}"
        );

        // [[trusted_projects]] typo.
        let err =
            toml::from_str::<Registry>("[[trusted_projects]]\npathz = \"/tmp/x\"\n").unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "trusted_projects typo must fail: {err}"
        );
    }

    /// `ConfigRepoEntry` uses a raw identifier (`r#ref`) for the TOML key
    /// `ref`; verify serde sees the plain name "ref" (not "r#ref") BOTH ways:
    /// the real key still parses, and a misspelling of it is rejected.
    #[test]
    fn config_repo_entry_raw_identifier_ref_round_trips() {
        let entry: ConfigRepoEntry =
            toml::from_str("url = \"https://example.invalid/x.git\"\nref = \"main\"\n").unwrap();
        assert_eq!(entry.r#ref.as_deref(), Some("main"));

        let err = toml::from_str::<ConfigRepoEntry>("url = \"x\"\nreff = \"main\"\n").unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "misspelled raw-identifier key must fail: {err}"
        );
    }

    /// A fully-populated, valid registry still parses (the new deny rule must
    /// not reject any known field).
    #[test]
    fn registry_accepts_all_known_fields() {
        let raw = r#"
layers = ["personal"]

[settings]
default_context = "personal"
store_dir = "/tmp/store"
state_dir = "/tmp/state"
home_version = 2

[configs.personal]
url = "https://example.invalid/personal.git"
ref = "main"
rev = "abc123"
secrets = "file"
secrets_file = ".env.enc"
age_key_file = "~/.config/sops/age/keys.txt"

[contexts.personal]
layers = ["personal"]

[[trusted_projects]]
path = "/tmp/project"
"#;
        let registry: Registry = toml::from_str(raw).unwrap();
        assert_eq!(registry.settings.home_version, Some(2));
        assert_eq!(registry.configs["personal"].r#ref.as_deref(), Some("main"));
        assert_eq!(registry.trusted_projects.len(), 1);
    }

    // ---- image.binary (BinarySpec): worker is optional ----

    /// A `BinarySpec` with the worker omitted (prime-agent compiles workerless,
    /// no image-resize-worker) must parse with `worker == None`.
    #[test]
    fn binary_spec_worker_omitted_parses_as_none() {
        let raw = r#"
recipe = "bun-compile"
src = "flake://prime"
entrypoint = "packages/coding-agent/dist/bun/cli.js"
"#;
        let spec: BinarySpec = toml::from_str(raw).unwrap();
        assert_eq!(spec.recipe, "bun-compile");
        assert_eq!(spec.src, "flake://prime");
        assert_eq!(
            spec.entrypoint.as_deref(),
            Some("packages/coding-agent/dist/bun/cli.js")
        );
        assert_eq!(
            spec.worker, None,
            "worker must default to None when omitted"
        );
    }

    /// A `BinarySpec` with a worker present (pi passes its
    /// image-resize-worker.ts) must parse with `worker == Some(...)` and the
    /// full inline `image.binary` form inside a workload must also parse.
    #[test]
    fn binary_spec_worker_present_parses_as_some() {
        let raw = r#"
recipe = "bun-compile"
src = "flake://pi"
entrypoint = "packages/coding-agent/dist/bun/cli.js"
worker = "packages/coding-agent/src/utils/image-resize-worker.ts"
"#;
        let spec: BinarySpec = toml::from_str(raw).unwrap();
        assert_eq!(
            spec.worker.as_deref(),
            Some("packages/coding-agent/src/utils/image-resize-worker.ts")
        );

        // The same shape inside a workload's inline image table.
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "nix-layered", name = "pi", tag = "latest", binary = { recipe = "bun-compile", src = "flake://pi", entrypoint = "packages/coding-agent/dist/bun/cli.js", worker = "packages/coding-agent/src/utils/image-resize-worker.ts" } }
command = []

[workloads.pi.network.defaults]
egress = "deny"
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let binary = config.workloads["pi"].image.binary.as_ref().unwrap();
        assert_eq!(binary.recipe, "bun-compile");
        assert_eq!(
            binary.worker.as_deref(),
            Some("packages/coding-agent/src/utils/image-resize-worker.ts")
        );
    }

    // ---- ADR 0026(d): depends_on dependency declarations ----

    /// A `[workloads.pi.depends_on.litellm]` table carrying only `env` must
    /// parse with `required` defaulting to false.
    #[test]
    fn depends_on_parses_with_required_defaulting_false() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let spec = &config.workloads["pi"].depends_on["litellm"];
        assert_eq!(spec.env.as_deref(), Some("LITELLM_URL"));
        assert!(!spec.required, "required must default to false");
    }

    /// An explicit `required = true` survives a serialize/deserialize
    /// round-trip.
    #[test]
    fn depends_on_required_true_round_trips() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
required = true
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let spec = &config.workloads["pi"].depends_on["litellm"];
        assert_eq!(spec.env.as_deref(), Some("LITELLM_URL"));
        assert!(spec.required);

        let serialized = toml::to_string(&config).unwrap();
        let reparsed: ConfigFile = toml::from_str(&serialized).unwrap();
        assert_eq!(reparsed, config);
    }

    /// A typo inside a depends_on spec must hard-error (closed vocabulary).
    #[test]
    fn depends_on_rejects_unknown_spec_field() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
evn = "X"
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "depends_on spec typo must fail: {err}"
        );
    }

    /// `depends_on.<dep>.on_conflict` accepts the three closed variants and
    /// defaults to None (the default chain is applied at decision time).
    #[test]
    fn depends_on_on_conflict_variants_parse_and_default_none() {
        for (value, expected) in [
            ("reuse", DepConflict::reuse()),
            ("replace", DepConflict::replace()),
            ("fail", DepConflict::fail()),
        ] {
            let raw = format!(
                "schema_version = 1\n\n\
                 [workloads.pi]\n\
                 kind = \"agent\"\n\
                 image = {{ recipe = \"registry\", ref = \"node:24\" }}\n\
                 command = []\n\n\
                 [workloads.pi.depends_on.litellm]\n\
                 env = \"LITELLM_URL\"\n\
                 on_conflict = \"{value}\"\n"
            );
            let config: ConfigFile = toml::from_str(&raw).unwrap();
            assert_eq!(
                config.workloads["pi"].depends_on["litellm"].on_conflict,
                Some(expected),
                "on_conflict = \"{value}\" must parse"
            );
        }
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(
            config.workloads["pi"].depends_on["litellm"].on_conflict, None,
            "omitted on_conflict must default to None"
        );
    }

    /// An unknown on_conflict variant is rejected by serde (closed
    /// vocabulary), naming the offending value.
    #[test]
    fn depends_on_on_conflict_rejects_unknown_variant() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
on_conflict = "nuke"
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown variant"),
            "unknown on_conflict variant must fail: {msg}"
        );
        assert!(msg.contains("nuke"), "error must name the value: {msg}");
    }

    /// An explicit on_conflict survives a serialize/deserialize round-trip.
    #[test]
    fn depends_on_on_conflict_round_trips() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
on_conflict = "replace"
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let serialized = toml::to_string(&config).unwrap();
        let reparsed: ConfigFile = toml::from_str(&serialized).unwrap();
        assert_eq!(reparsed, config);
        assert_eq!(
            reparsed.workloads["pi"].depends_on["litellm"].on_conflict,
            Some(DepConflict::replace())
        );
    }

    /// An ordered `on_conflict` LIST parses into the chain, preserving order.
    #[test]
    fn on_conflict_chain_list_parses_and_normalizes() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
on_conflict = ["reuse", "start", "replace"]
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(
            config.workloads["pi"].depends_on["litellm"].on_conflict,
            Some(DepConflict(vec![
                ConflictStep::Reuse,
                ConflictStep::Start,
                ConflictStep::Replace,
            ])),
            "an ordered list must parse into the chain in order"
        );
    }

    /// A scalar `on_conflict` normalizes to a SINGLETON chain (back-compat).
    #[test]
    fn on_conflict_scalar_normalizes_to_singleton() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
on_conflict = "reuse"
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let conflict = config.workloads["pi"].depends_on["litellm"]
            .on_conflict
            .as_ref()
            .unwrap();
        assert_eq!(
            conflict.0.len(),
            1,
            "scalar must normalize to a singleton chain"
        );
        assert_eq!(conflict.0[0], ConflictStep::Reuse);
    }

    /// An empty `on_conflict` list is rejected (a chain needs at least one
    /// element).
    #[test]
    fn on_conflict_empty_list_rejected() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
on_conflict = []
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("must not be empty"),
            "empty chain must be rejected: {msg}"
        );
    }

    /// A duplicate element in the chain is rejected.
    #[test]
    fn on_conflict_duplicate_rejected() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
on_conflict = ["reuse", "reuse"]
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("duplicate"),
            "duplicate chain element must be rejected: {msg}"
        );
    }

    /// `fail` is terminal: nothing may follow it. A singleton `["fail"]`
    /// chain parses fine.
    #[test]
    fn on_conflict_after_fail_rejected() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
on_conflict = ["reuse", "fail", "replace"]
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("terminal"),
            "elements after 'fail' must be rejected: {msg}"
        );

        let ok = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
on_conflict = ["fail"]
"#;
        let config: ConfigFile = toml::from_str(ok).unwrap();
        assert_eq!(
            config.workloads["pi"].depends_on["litellm"].on_conflict,
            Some(DepConflict::fail()),
            "a singleton ['fail'] chain must parse"
        );
    }

    /// An unknown chain element is rejected by serde's closed vocabulary.
    #[test]
    fn on_conflict_unknown_element_rejected() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
on_conflict = ["reuse", "nuke"]
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown variant"),
            "unknown chain element must be rejected: {msg}"
        );
        assert!(msg.contains("nuke"), "error must name the value: {msg}");
    }

    /// Scalar and singleton-list forms parse to EQUAL chains (round-trip
    /// equality — the merge last-layer-wins test depends on this).
    #[test]
    fn dep_conflict_scalar_list_round_trip_equality() {
        let scalar = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
on_conflict = "reuse"
"#;
        let list = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
on_conflict = ["reuse"]
"#;
        let a: ConfigFile = toml::from_str(scalar).unwrap();
        let b: ConfigFile = toml::from_str(list).unwrap();
        assert_eq!(
            a.workloads["pi"].depends_on["litellm"].on_conflict,
            b.workloads["pi"].depends_on["litellm"].on_conflict,
            "scalar and singleton-list forms must normalize to equal chains"
        );
    }

    /// A camelCase `dependsOn` at workload level is not a known field and
    /// must be rejected like any other typo.
    #[test]
    fn workload_rejects_unknown_depends_on_casing() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
dependsOn = {}
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "workload-level 'dependsOn' must fail: {err}"
        );
    }

    // ---- ADR 0030 Phase 1: the per-workload instance policy ----

    /// A workload with NO instance table parses to the default policy
    /// (strategy Singleton, on_conflict None, port None, label None) — the
    /// "absent = current behavior" invariant.
    #[test]
    fn instance_policy_defaults_are_current_behavior() {
        let config: ConfigFile = toml::from_str(MINIMAL_VALID_TOML).unwrap();
        let policy = &config.workloads["pi"].instance;
        assert_eq!(policy, &InstancePolicy::default());
        assert_eq!(policy.strategy, InstanceStrategy::Singleton);
        assert_eq!(policy.on_conflict, None);
        assert_eq!(policy.port, None);
        assert_eq!(policy.label, None);
    }

    /// A full `[workloads.<name>.instance]` block parses into the typed
    /// fields.
    #[test]
    fn instance_policy_full_block_parses() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
strategy = "parallel"
on_conflict = ["reuse", "fail"]
label = "dev"

[workloads.pi.instance.port]
preferred = 4000
on_occupied = ["increment", "auto"]
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let policy = &config.workloads["pi"].instance;
        assert_eq!(policy.strategy, InstanceStrategy::Parallel);
        assert_eq!(
            policy.on_conflict,
            Some(DepConflict(vec![ConflictStep::Reuse, ConflictStep::Fail]))
        );
        match &policy.port {
            Some(InstancePort::Preferred(p)) => {
                assert_eq!(p.preferred, 4000);
                assert_eq!(
                    p.on_occupied,
                    Some(PortOccupiedChain(vec![
                        PortOccupiedStep::Bare(PortOccupiedBare::Increment),
                        PortOccupiedStep::Bare(PortOccupiedBare::Auto),
                    ]))
                );
            }
            other => panic!("expected Preferred port, got {other:?}"),
        }
        assert_eq!(policy.label.as_deref(), Some("dev"));
    }

    /// The strategy vocabulary is closed: an unknown strategy is a parse
    /// error (unknown variant).
    #[test]
    fn instance_policy_strategy_closed_vocabulary() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
strategy = "bogus"
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown variant"),
            "unknown strategy must be rejected: {msg}"
        );
        assert!(msg.contains("bogus"), "error must name the value: {msg}");
    }

    /// Unknown instance-policy fields are rejected by deny_unknown_fields.
    #[test]
    fn instance_policy_unknown_field_rejected() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
bogus = 1
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "instance block must reject unknown fields: {err}"
        );
    }

    /// `port = N` (integer) parses as the strict form.
    #[test]
    fn instance_port_strict_integer_parses() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = 4000
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(
            config.workloads["pi"].instance.port,
            Some(InstancePort::Strict(4000))
        );
    }

    /// `port = "auto"` parses as the auto form.
    #[test]
    fn instance_port_auto_parses() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = "auto"
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(
            config.workloads["pi"].instance.port,
            Some(InstancePort::Auto)
        );
    }

    /// `port = { preferred = N }` (no on_occupied) parses to Preferred with a
    /// None chain.
    #[test]
    fn instance_port_preferred_parses() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000 }
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(
            config.workloads["pi"].instance.port,
            Some(InstancePort::Preferred(PreferredPort {
                preferred: 4000,
                on_occupied: None,
            }))
        );
    }

    /// `port = { preferred = N, on_occupied = [...] }` parses the chain.
    #[test]
    fn instance_port_preferred_with_chain_parses() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = ["increment", "auto"] }
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        match &config.workloads["pi"].instance.port {
            Some(InstancePort::Preferred(p)) => {
                assert_eq!(p.preferred, 4000);
                assert_eq!(
                    p.on_occupied,
                    Some(PortOccupiedChain(vec![
                        PortOccupiedStep::Bare(PortOccupiedBare::Increment),
                        PortOccupiedStep::Bare(PortOccupiedBare::Auto),
                    ]))
                );
            }
            other => panic!("expected Preferred port, got {other:?}"),
        }
    }

    /// A scalar TABLE form `on_occupied = { increment = { limit = N } }`
    /// parses as a singleton parameterized increment.
    #[test]
    fn instance_port_preferred_parameterized_increment_limit() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = { increment = { limit = 100 } } }
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        match &config.workloads["pi"].instance.port {
            Some(InstancePort::Preferred(p)) => {
                assert_eq!(
                    p.on_occupied,
                    Some(PortOccupiedChain(vec![PortOccupiedStep::Increment(
                        ParameterizedIncrement {
                            increment: IncrementSpec {
                                limit: Some(100),
                                range: None,
                            },
                        }
                    )]))
                );
            }
            other => panic!("expected Preferred port, got {other:?}"),
        }
    }

    /// The scalar TABLE form supports the absolute `range` band.
    #[test]
    fn instance_port_preferred_parameterized_increment_range() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = { increment = { range = [5000, 5100] } } }
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        match &config.workloads["pi"].instance.port {
            Some(InstancePort::Preferred(p)) => {
                assert_eq!(
                    p.on_occupied,
                    Some(PortOccupiedChain(vec![PortOccupiedStep::Increment(
                        ParameterizedIncrement {
                            increment: IncrementSpec {
                                limit: None,
                                range: Some((5000, 5100)),
                            },
                        }
                    )]))
                );
            }
            other => panic!("expected Preferred port, got {other:?}"),
        }
    }

    /// A chain may mix the bare and parameterized increment forms.
    #[test]
    fn instance_port_preferred_chain_mixes_bare_and_parameterized() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = [{ increment = { range = [5000, 5100] } }, "auto"] }
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        match &config.workloads["pi"].instance.port {
            Some(InstancePort::Preferred(p)) => {
                assert_eq!(
                    p.on_occupied,
                    Some(PortOccupiedChain(vec![
                        PortOccupiedStep::Increment(ParameterizedIncrement {
                            increment: IncrementSpec {
                                limit: None,
                                range: Some((5000, 5100)),
                            },
                        }),
                        PortOccupiedStep::Bare(PortOccupiedBare::Auto),
                    ]))
                );
            }
            other => panic!("expected Preferred port, got {other:?}"),
        }
    }

    /// A scalar string `on_occupied = "auto"` normalizes to a singleton chain.
    #[test]
    fn on_occupied_scalar_normalizes_to_singleton() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = "auto" }
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        match &config.workloads["pi"].instance.port {
            Some(InstancePort::Preferred(p)) => {
                assert_eq!(
                    p.on_occupied,
                    Some(PortOccupiedChain::auto()),
                    "scalar must normalize to a singleton chain"
                );
            }
            other => panic!("expected Preferred port, got {other:?}"),
        }

        // The U6 chain constructors (used by the Phase 3 decision path):
        // singletons + the recommended default chain.
        assert_eq!(
            PortOccupiedChain::increment().0,
            vec![PortOccupiedStep::Bare(PortOccupiedBare::Increment)]
        );
        assert_eq!(
            PortOccupiedChain::fail().0,
            vec![PortOccupiedStep::Bare(PortOccupiedBare::Fail)]
        );
        assert_eq!(
            PortOccupiedChain::default_on_occupied().0,
            vec![
                PortOccupiedStep::Bare(PortOccupiedBare::Increment),
                PortOccupiedStep::Bare(PortOccupiedBare::Auto),
            ]
        );
    }

    /// An empty `on_occupied` list is rejected (a chain needs at least one
    /// element).
    #[test]
    fn on_occupied_empty_rejected() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = [] }
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("must not be empty"),
            "empty chain must be rejected: {msg}"
        );
    }

    /// Duplicate elements (by KIND) in the chain are rejected — a bare
    /// "increment" and a parameterized `{ increment = ... }` are the same
    /// kind.
    #[test]
    fn on_occupied_duplicate_kind_rejected() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = ["increment", "auto", "increment"] }
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("duplicate"),
            "duplicate chain element must be rejected: {msg}"
        );
    }

    /// `fail` is terminal: nothing may follow it. A singleton `["fail"]`
    /// chain parses fine.
    #[test]
    fn on_occupied_after_fail_rejected() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = ["auto", "fail", "auto"] }
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("terminal"),
            "elements after 'fail' must be rejected: {msg}"
        );

        let ok = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = ["fail"] }
"#;
        let config: ConfigFile = toml::from_str(ok).unwrap();
        match &config.workloads["pi"].instance.port {
            Some(InstancePort::Preferred(p)) => {
                assert_eq!(p.on_occupied, Some(PortOccupiedChain::fail()));
            }
            other => panic!("expected Preferred port, got {other:?}"),
        }
    }

    /// An unknown chain element is rejected by serde's closed vocabulary.
    #[test]
    fn on_occupied_unknown_rejected() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = ["auto", "nuke"] }
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown variant"),
            "unknown chain element must be rejected: {msg}"
        );
        assert!(msg.contains("nuke"), "error must name the value: {msg}");
    }

    /// An instance policy block survives a serialize/deserialize round-trip
    /// (ConfigFile equality).
    #[test]
    fn instance_policy_round_trip() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
strategy = "parallel"
on_conflict = ["reuse", "fail"]
label = "dev"

[workloads.pi.instance.port]
preferred = 4000
on_occupied = ["increment", "auto"]
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let serialized = toml::to_string(&config).unwrap();
        let reparsed: ConfigFile = toml::from_str(&serialized).unwrap();
        assert_eq!(reparsed, config);
        assert_eq!(
            reparsed.workloads["pi"].instance.port,
            Some(InstancePort::Preferred(PreferredPort {
                preferred: 4000,
                on_occupied: Some(PortOccupiedChain(vec![
                    PortOccupiedStep::Bare(PortOccupiedBare::Increment),
                    PortOccupiedStep::Bare(PortOccupiedBare::Auto),
                ])),
            }))
        );
    }

    /// `{ increment = {} }` (neither limit nor range) PARSES — the struct
    /// fields are both optional; the exactly-one rejection is
    /// validation-level (config/validation.rs tests).
    #[test]
    fn increment_spec_exactly_one_of_limit_range() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = { increment = {} } }
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        match &config.workloads["pi"].instance.port {
            Some(InstancePort::Preferred(p)) => {
                let steps = p.on_occupied.as_ref().unwrap();
                let PortOccupiedStep::Increment(ParameterizedIncrement { increment }) = &steps.0[0]
                else {
                    panic!("expected a parameterized increment");
                };
                assert_eq!(increment.limit, None);
                assert_eq!(increment.range, None);
            }
            other => panic!("expected Preferred port, got {other:?}"),
        }
    }

    /// The strategy Display strings match the config vocabulary.
    #[test]
    fn instance_strategy_display() {
        assert_eq!(InstanceStrategy::Singleton.to_string(), "singleton");
        assert_eq!(InstanceStrategy::Parallel.to_string(), "parallel");
        assert_eq!(InstanceStrategy::Replace.to_string(), "replace");
        assert_eq!(InstanceStrategy::Reuse.to_string(), "reuse");
        assert_eq!(ConflictStep::Reuse.to_string(), "reuse");
        assert_eq!(ConflictStep::Start.to_string(), "start");
        assert_eq!(ConflictStep::Replace.to_string(), "replace");
        assert_eq!(ConflictStep::Fail.to_string(), "fail");
        assert_eq!(PortOccupiedBare::Auto.to_string(), "auto");
        assert_eq!(PortOccupiedBare::Increment.to_string(), "increment");
        assert_eq!(PortOccupiedBare::Fail.to_string(), "fail");
    }

    // ---- P0: seed_files source|glob + template/glob fields ----

    /// A `[[workloads.svc.seed_files]]` entry carrying only source/target/
    /// only_if_missing parses with `template` defaulting to false and `glob`
    /// defaulting to None.
    #[test]
    fn seed_entry_parses_with_template_defaulting_false() {
        let raw = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = []

[[workloads.svc.seed_files]]
source = "seed/a.json"
target = "workspaces/svc-state/a.json"
only_if_missing = true
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let seed = &config.workloads["svc"].seed_files[0];
        assert_eq!(seed.source.as_deref(), Some("seed/a.json"));
        assert_eq!(seed.target, "workspaces/svc-state/a.json");
        assert_eq!(seed.only_if_missing, Some(true));
        assert!(!seed.template, "template must default to false");
        assert!(seed.glob.is_none(), "glob must default to None");
    }

    /// A glob entry with `template = true` parses and survives a
    /// serialize/deserialize round-trip byte-for-byte (template/glob fields
    /// must round-trip).
    #[test]
    fn seed_entry_glob_round_trips() {
        let raw = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = []

[[workloads.svc.seed_files]]
glob = "seed/**/*.json"
target = "workspaces/svc-state"
template = true
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let seed = &config.workloads["svc"].seed_files[0];
        assert_eq!(seed.glob.as_deref(), Some("seed/**/*.json"));
        assert_eq!(seed.target, "workspaces/svc-state");
        assert!(seed.template);
        assert!(seed.source.is_none());

        let serialized = toml::to_string(&config).unwrap();
        let reparsed: ConfigFile = toml::from_str(&serialized).unwrap();
        assert_eq!(reparsed, config);
    }

    /// A typo inside a seed_files entry must hard-error (closed vocabulary).
    #[test]
    fn seed_entry_rejects_unknown_field() {
        let raw = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = []

[[workloads.svc.seed_files]]
source = "seed/a.json"
target = "workspaces/svc-state/a.json"
gloob = "x"
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "seed_files typo must fail: {err}"
        );
    }

    /// Any non-`template` casing of the template key must be rejected as an
    /// unknown field.
    #[test]
    fn seed_entry_rejects_template_casing_typo() {
        for typo in ["templated = true", "Template = true"] {
            let raw = format!(
                "schema_version = 1\n\n\
                 [workloads.svc]\n\
                 kind = \"service\"\n\
                 image = {{ recipe = \"registry\", ref = \"python:3.12-slim\" }}\n\
                 command = []\n\n\
                 [[workloads.svc.seed_files]]\n\
                 source = \"seed/a.json\"\n\
                 target = \"workspaces/svc-state/a.json\"\n\
                 {typo}\n"
            );
            let err = toml::from_str::<ConfigFile>(&raw).unwrap_err();
            assert!(
                err.to_string().contains("unknown field"),
                "seed_files template casing typo '{typo}' must fail: {err}"
            );
        }
    }

    // ---- Spec 16: final unified secret/env model — the desugar table ----

    /// Row 1: `KEY = true` desugars to a host-bound (bound-less) secret
    /// binding whose secret ID IS the key name (key-name default).
    #[test]
    fn env_map_true_sugar_binds_same_named_secret_host_bound() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
env = { GITHUB_TOKEN = true }
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(
            config.workloads["pi"].env,
            EnvBindings(vec![(
                "GITHUB_TOKEN".to_string(),
                EnvBinding::Secret(EnvSecretRef {
                    secret: "GITHUB_TOKEN".to_string(),
                    bound: None,
                })
            )])
        );
    }

    /// Row 2: `KEY = { bound = "guest" }` is the bound-only object — the
    /// key-name default fills `secret`; no name repeat is needed for a
    /// same-name real value.
    #[test]
    fn env_map_bound_only_object_defaults_secret_to_key() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
env = { MASTER = { bound = "guest" } }
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(
            config.workloads["pi"].env,
            EnvBindings(vec![(
                "MASTER".to_string(),
                EnvBinding::Secret(EnvSecretRef {
                    secret: "MASTER".to_string(),
                    bound: Some(Bound::Guest),
                })
            )])
        );
    }

    /// Row 3: `KEY = { secret = "ID" }` is the rename form — placeholder
    /// (bound-less) for ID ≠ KEY.
    #[test]
    fn env_map_secret_rename_defaults_bound_to_none() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
env = { API_KEY = { secret = "MY_SECRET" } }
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(
            config.workloads["pi"].env,
            EnvBindings(vec![(
                "API_KEY".to_string(),
                EnvBinding::Secret(EnvSecretRef {
                    secret: "MY_SECRET".to_string(),
                    bound: None,
                })
            )])
        );
    }

    /// Row 4: the full form `KEY = { secret = "ID", bound = "guest" }` —
    /// renamed real value.
    #[test]
    fn env_map_full_form_parses_secret_and_bound() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
env = { API_KEY = { secret = "MY_SECRET", bound = "guest" } }
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(
            config.workloads["pi"].env,
            EnvBindings(vec![(
                "API_KEY".to_string(),
                EnvBinding::Secret(EnvSecretRef {
                    secret: "MY_SECRET".to_string(),
                    bound: Some(Bound::Guest),
                })
            )])
        );
    }

    /// An explicit `bound = "host"` parses (it is the default, made
    /// explicit) and stays `Some(Host)` through parse — the host-default is
    /// NOT collapsed to None.
    #[test]
    fn env_map_explicit_host_bound_parses() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
env = { API_KEY = { secret = "MY_SECRET", bound = "host" } }
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(
            config.workloads["pi"].env,
            EnvBindings(vec![(
                "API_KEY".to_string(),
                EnvBinding::Secret(EnvSecretRef {
                    secret: "MY_SECRET".to_string(),
                    bound: Some(Bound::Host),
                })
            )])
        );
    }

    /// `KEY = false` is a hard error with the corrective "did you mean
    /// true?" hint (false has no meaning in this desugar — a binding either
    /// binds or is absent).
    #[test]
    fn env_map_false_is_hard_error_with_did_you_mean_true() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
env = { API_KEY = false }
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("did you mean true?"),
            "error must carry the corrective hint: {msg}"
        );
        assert!(
            msg.contains("env map entry 'API_KEY'"),
            "error must name the entry key: {msg}"
        );
    }

    /// Unknown fields inside an inline-table map value still hard-error
    /// (`deny_unknown_fields` is preserved through the raw-ref path).
    #[test]
    fn env_map_inline_table_rejects_unknown_field() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
env = { API_KEY = { secret = "X", name = "Y" } }
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "inline-table unknown field must fail: {err}"
        );
    }

    /// An unknown `bound` variant is rejected by serde's unknown-variant
    /// machinery, naming the offending value and the known variants.
    #[test]
    fn env_map_rejects_unknown_bound_variant() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
env = { API_KEY = { bound = "universe" } }
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("unknown variant"),
            "unknown bound variant must fail: {msg}"
        );
        assert!(msg.contains("universe"), "error names the value: {msg}");
    }

    // ---- Spec 16: secret defs are a closed catalog — retired keys hard-error ----

    /// Every retired v1/v2 def key (`hosts`, `delivery`, `source`,
    /// `exposed_as`, `description`) plus a stray `bound` on a def is an
    /// unknown-field parse error under `deny_unknown_fields`.
    #[test]
    fn secret_def_rejects_retired_keys() {
        for snippet in [
            "hosts = [\"example.com\"]",
            "delivery = \"env\"",
            "source = \"OTHER\"",
            "exposed_as = \"OPENAI_API_KEY\"",
            "description = \"doc\"",
            "bound = \"guest\"",
        ] {
            let raw = format!("[secrets.A]\nenv_var = \"A\"\n{snippet}\n");
            let err = toml::from_str::<ConfigFile>(&raw).unwrap_err();
            assert!(
                err.to_string().contains("unknown field"),
                "def key '{snippet}' must be an unknown-field error: {err}"
            );
        }
    }

    /// A `secret_env` workload key is GONE from the schema — it is an
    /// unknown-field parse error (the required hard-reject), not a
    /// normalizable legacy form.
    #[test]
    fn workload_rejects_secret_env_key() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
secret_env = ["A"]
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "secret_env must be an unknown-field error: {err}"
        );
    }

    // ---- Spec 14: env map form ----

    /// Bare map literals: both the inline-table form `env = { FOO = "bar" }`
    /// and the standard-table form `[workloads.pi.env]` parse to the
    /// identical binding (name from the key, literal value).
    #[test]
    fn env_map_bare_literals_parse() {
        let expected = EnvBindings(vec![(
            "FOO".to_string(),
            EnvBinding::Literal("bar".to_string()),
        )]);
        for raw in [
            r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
env = { FOO = "bar" }
"#,
            r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.env]
FOO = "bar"
"#,
        ] {
            let config: ConfigFile = toml::from_str(raw).unwrap();
            assert_eq!(config.workloads["pi"].env, expected);
        }
    }

    /// Mixed literal and secret-ref map entries are legal and preserve
    /// DOCUMENT order — the deliberately non-alphabetical keys prove no
    /// sorting (and no map intermediate) happens during normalization.
    #[test]
    fn env_map_mixed_entries_preserve_document_order() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
env = { ZEBRA = "z", MIDDLE = { secret = "M_SECRET" }, ALPHA = "a" }
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(
            config.workloads["pi"].env,
            EnvBindings(vec![
                ("ZEBRA".to_string(), EnvBinding::Literal("z".to_string())),
                (
                    "MIDDLE".to_string(),
                    EnvBinding::Secret(EnvSecretRef {
                        secret: "M_SECRET".to_string(),
                        bound: None,
                    })
                ),
                ("ALPHA".to_string(), EnvBinding::Literal("a".to_string())),
            ])
        );
    }

    /// Duplicate keys in a map are a free TOML-level error — no code needed.
    #[test]
    fn env_map_duplicate_key_fails() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
env = { FOO = "a", FOO = "b" }
"#;
        assert!(
            toml::from_str::<ConfigFile>(raw).is_err(),
            "duplicate map key must fail at the TOML level"
        );
    }

    /// The legacy array-of-tables form still parses: value-only entries
    /// normalize to Literal, secret-only to a bound-less Secret (the array
    /// form cannot express `bound`).
    #[test]
    fn env_array_of_tables_still_parses() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[[workloads.pi.env]]
name = "FOO"
value = "bar"

[[workloads.pi.env]]
name = "BAZ"
secret = "BAZ_SECRET"
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(
            config.workloads["pi"].env,
            EnvBindings(vec![
                ("FOO".to_string(), EnvBinding::Literal("bar".to_string())),
                (
                    "BAZ".to_string(),
                    EnvBinding::Secret(EnvSecretRef {
                        secret: "BAZ_SECRET".to_string(),
                        bound: None,
                    })
                ),
            ])
        );
    }

    /// A legacy `[[env]]` entry with neither value nor secret normalizes to
    /// an empty literal; one with BOTH is a precise hard error (the message
    /// the pre-map-form plan builder emitted, moved to parse time).
    #[test]
    fn env_array_entry_normalization_edge_cases() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[[workloads.pi.env]]
name = "EMPTY"
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(
            config.workloads["pi"].env,
            EnvBindings(vec![(
                "EMPTY".to_string(),
                EnvBinding::Literal(String::new())
            )])
        );

        let both = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[[workloads.pi.env]]
name = "BAD"
value = "x"
secret = "S"
"#;
        let err = toml::from_str::<ConfigFile>(both).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("env 'BAD' cannot have both value and secret"),
            "both-fields entry must fail with the precise message: {msg}"
        );
    }

    /// Mixing the array-of-tables form and the map form for the same
    /// workload is a TOML redefinition error.
    #[test]
    fn env_array_plus_map_form_fails() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[[workloads.pi.env]]
name = "FOO"
value = "bar"

[workloads.pi.env]
BAZ = "qux"
"#;
        assert!(
            toml::from_str::<ConfigFile>(raw).is_err(),
            "redefining env as a table after an array of tables must fail"
        );
    }

    /// The array-of-tables form serializes via `toml::to_string` (always the
    /// MAP form) and re-parses to the identical bindings.
    #[test]
    fn env_array_form_serializes_and_round_trips() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[[workloads.pi.env]]
name = "FOO"
value = "bar"

[[workloads.pi.env]]
name = "BAZ"
secret = "BAZ_SECRET"
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let serialized = toml::to_string(&config).unwrap();
        let reparsed: ConfigFile = toml::from_str(&serialized).unwrap();
        assert_eq!(reparsed, config);
    }

    /// A bound secret binding serializes with BOTH keys and re-parses
    /// identical; a bound-less one serializes `{ secret = "ID" }` only.
    #[test]
    fn env_map_secret_binding_serializes_and_round_trips() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.env]
MASTER = { bound = "guest" }
API_KEY = { secret = "MY_SECRET" }
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let serialized = toml::to_string(&config).unwrap();
        assert!(
            serialized.contains("secret = \"MASTER\"") && serialized.contains("bound = \"guest\""),
            "bound serializes alongside secret: {serialized}"
        );
        let reparsed: ConfigFile = toml::from_str(&serialized).unwrap();
        assert_eq!(reparsed, config);
    }

    /// A non-string/non-bool/non-table map value must fail with a precise
    /// error naming the entry key and the offending value's type.
    #[test]
    fn env_map_bad_value_error_names_key_and_type() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
env = { FOO = 42 }
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("env map entry 'FOO'"),
            "error must name the entry key: {msg}"
        );
        assert!(
            msg.contains("integer"),
            "error must name the offending type: {msg}"
        );
    }

    // ---- EnvBindings methods ----

    /// `get`/`upsert`/`len`/`is_empty`: upsert replaces in place (position
    /// preserved) or appends, matching the merge union-by-name semantics.
    #[test]
    fn env_bindings_upsert_replaces_in_place_or_appends() {
        let mut bindings = EnvBindings(vec![
            ("A".to_string(), EnvBinding::Literal("1".to_string())),
            ("B".to_string(), EnvBinding::Literal("2".to_string())),
        ]);
        assert_eq!(bindings.len(), 2);
        assert!(!bindings.is_empty());
        assert_eq!(
            bindings.get("A"),
            Some(&EnvBinding::Literal("1".to_string()))
        );
        assert_eq!(bindings.get("MISSING"), None);

        // Replace in place: A keeps position 0.
        bindings.upsert(
            "A",
            EnvBinding::Secret(EnvSecretRef {
                secret: "S".to_string(),
                bound: None,
            }),
        );
        // Append: C lands at the end.
        bindings.upsert("C", EnvBinding::Literal("3".to_string()));

        let names: Vec<&str> = bindings.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(names, vec!["A", "B", "C"], "order must be preserved");
        assert_eq!(
            bindings.get("A"),
            Some(&EnvBinding::Secret(EnvSecretRef {
                secret: "S".to_string(),
                bound: None,
            }))
        );
        assert_eq!(
            bindings.get("C"),
            Some(&EnvBinding::Literal("3".to_string()))
        );

        let empty = EnvBindings::default();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);
    }

    // ---- ADR 0030 Phase 2: depends_on.<dep>.instance (DepInstanceMode) ----

    /// The closed vocabulary parses: shared/scoped/fresh each map to their
    /// enum variant, and the default is Shared.
    #[test]
    fn depends_on_instance_mode_parses() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
instance = "shared"

[workloads.pi.depends_on.redis]
env = "REDIS_URL"
instance = "scoped"

[workloads.pi.depends_on.pg]
env = "PG_URL"
instance = "fresh"
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let deps = &config.workloads["pi"].depends_on;
        assert_eq!(deps["litellm"].instance, Some(DepInstanceMode::Shared));
        assert_eq!(deps["redis"].instance, Some(DepInstanceMode::Scoped));
        assert_eq!(deps["pg"].instance, Some(DepInstanceMode::Fresh));
        // Display strings match the config vocabulary.
        assert_eq!(DepInstanceMode::Shared.to_string(), "shared");
        assert_eq!(DepInstanceMode::Scoped.to_string(), "scoped");
        assert_eq!(DepInstanceMode::Fresh.to_string(), "fresh");
    }

    /// An unknown instance value is a parse error (closed vocabulary).
    #[test]
    fn depends_on_instance_mode_closed_vocabulary() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
instance = "ephemeral"
"#;
        assert!(
            toml::from_str::<ConfigFile>(raw).is_err(),
            "an unknown instance mode must be a parse error"
        );
    }

    /// Omitting `instance` leaves it None (the strategy-derived default
    /// applies at decision time).
    #[test]
    fn depends_on_instance_mode_defaults_none() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(config.workloads["pi"].depends_on["litellm"].instance, None);
    }

    // ---- ADR 0030 V-addendum §V1: strategy = "per-dir" ----

    /// `strategy = "per-dir"` parses (kebab-case — snake_case would be
    /// `per_dir` and must NOT parse) and Displays as "per-dir".
    #[test]
    fn instance_strategy_per_dir_parses_and_displays() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
strategy = "per-dir"
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(
            config.workloads["pi"].instance.strategy,
            InstanceStrategy::PerDir
        );
        assert_eq!(InstanceStrategy::PerDir.to_string(), "per-dir");
        // The serde spelling is kebab-case; snake_case is rejected.
        let snake = raw.replace("\"per-dir\"", "\"per_dir\"");
        assert!(
            toml::from_str::<ConfigFile>(&snake).is_err(),
            "per_dir (snake_case) must NOT parse — the vocabulary is per-dir"
        );
    }

    // ---- ADR 0030 V-addendum §V4: instance.on_skew ----

    /// Each on_skew value parses; the default-absent case is None (the warn
    /// default applies at decision time).
    #[test]
    fn on_skew_variants_parse_and_default_none() {
        for (value, expected) in [
            ("warn", OnSkew::Warn),
            ("replace", OnSkew::Replace),
            ("reuse-silently", OnSkew::ReuseSilently),
        ] {
            let raw = format!(
                "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = {{ recipe = \"registry\", ref = \"node:24\" }}\ncommand = []\n\n[workloads.pi.instance]\non_skew = \"{value}\"\n"
            );
            let config: ConfigFile = toml::from_str(&raw)
                .unwrap_or_else(|e| panic!("on_skew = \"{value}\" must parse: {e}"));
            assert_eq!(
                config.workloads["pi"].instance.on_skew,
                Some(expected),
                "on_skew = \"{value}\""
            );
        }
        // Absent → None (warn applies at decision time).
        let raw = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n";
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert_eq!(config.workloads["pi"].instance.on_skew, None);
        // Display strings match the config vocabulary.
        assert_eq!(OnSkew::Warn.to_string(), "warn");
        assert_eq!(OnSkew::Replace.to_string(), "replace");
        assert_eq!(OnSkew::ReuseSilently.to_string(), "reuse-silently");
        assert_eq!(OnSkew::default(), OnSkew::Warn);
    }

    /// An unknown on_skew value is rejected by serde's closed vocabulary
    /// (kebab-case; `reuse_silently` must NOT parse).
    #[test]
    fn on_skew_unknown_value_rejected() {
        for bad in ["nuke", "reuse_silently", "WARN"] {
            let raw = format!(
                "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = {{ recipe = \"registry\", ref = \"node:24\" }}\ncommand = []\n\n[workloads.pi.instance]\non_skew = \"{bad}\"\n"
            );
            assert!(
                toml::from_str::<ConfigFile>(&raw).is_err(),
                "unknown on_skew value '{bad}' must fail"
            );
        }
    }

    /// An explicit on_skew survives a serialize/deserialize round-trip.
    #[test]
    fn on_skew_round_trips() {
        let raw = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.instance]\non_skew = \"reuse-silently\"\n";
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let serialized = toml::to_string(&config).unwrap();
        let reparsed: ConfigFile = toml::from_str(&serialized).unwrap();
        assert_eq!(
            reparsed.workloads["pi"].instance.on_skew,
            Some(OnSkew::ReuseSilently)
        );
    }
}
