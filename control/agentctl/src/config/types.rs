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
/// `config::validation`. `target` is the in-sandbox destination;
/// `only_if_missing` skips the copy when the target already exists. When
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

/// Per-workload network policy (`workloads.<name>.network`). `default_deny`
/// is the egress fail-closed switch (monotonic-true across layers for
/// non-entitled workloads); `egress` lists allowed egress recipes, `deny`
/// explicit domain-suffix denials, and `ingress` inbound exposure rules.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct NetworkConfig {
    pub default_deny: Option<bool>,
    #[serde(default)]
    pub egress: Vec<EgressRecipeRef>,
    #[serde(default)]
    pub deny: Vec<DenyDomainRule>,
    #[serde(default)]
    pub ingress: Vec<IngressRule>,
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
    /// Config-declared entitlements (`workloads.<name>.entitlements`). The
    /// closed vocabulary core understands lives in `config::validation`
    /// (currently only `"default_deny_false"`, which permits
    /// `network.default_deny = false` — fail-closed: setting
    /// `default_deny = false` WITHOUT the declared entitlement is a hard
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

/// Definition of one named secret (`secrets.<name>` in workestrate.toml) —
/// the final unified model (spec 16): a pure catalog of the credential's
/// intrinsic properties. `env_var` is the host environment variable the
/// resolved value is read from (default: the secret ID); `allowed_hosts`
/// constrains which egress hosts may receive the value by substitution
/// (omitted → deny-all; valid regardless of binding mode); `required` makes
/// a missing value a hard error; `placeholder` is a known-bad value to
/// reject. Exposure mode is NOT a def property — it lives at the binding
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
}

// ---------------------------------------------------------------------------
// User-global overrides field tables
// ---------------------------------------------------------------------------

/// Known top-level ConfigFile fields (for unknown-field detection in overrides).
pub(crate) const CONFIG_FIELDS: &[&str] = &["schema_version", "secrets", "workloads"];

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
}
