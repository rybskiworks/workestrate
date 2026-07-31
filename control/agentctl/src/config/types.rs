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

/// One secret reference in a workload's `secret_env` list: names an entry in
/// the top-level `secrets` map whose resolved value is injected into the
/// sandbox environment.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct SecretEnvConfig {
    pub secret: String,
}

/// Serde/schemars-boundary-only helper for the `secret_env` string-or-table
/// shorthand (spec 13): a bare string `"NAME"` is shorthand for the inline
/// table `{ secret = "NAME" }`. This enum is NEVER stored —
/// [`deserialize_secret_env`] normalizes every element to [`SecretEnvConfig`]
/// at parse time, so merge/validation/plan code sees the identical post-parse
/// struct it saw before. The `Full` (table) variant is kept as forward-compat
/// for future per-entry fields.
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)] // serde/schemars-boundary-only: variants are never read in Rust code
enum SecretEnvShorthand {
    Bare(String),
    Full(SecretEnvConfig),
}

/// Deserialize `secret_env`, accepting both bare secret-name strings
/// (shorthand for `{ secret = "NAME" }`) and inline tables, normalizing to
/// `Vec<SecretEnvConfig>`. Elements that are neither produce a precise error
/// naming the element index, the offending value's type, and the two expected
/// forms (untagged's default "did not match any variant" error is too vague).
fn deserialize_secret_env<'de, D>(deserializer: D) -> Result<Vec<SecretEnvConfig>, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct SecretEnvSeqVisitor;

    impl<'de> de::Visitor<'de> for SecretEnvSeqVisitor {
        type Value = Vec<SecretEnvConfig>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str(
                "a sequence of bare secret name strings and/or inline tables like \
                 { secret = \"NAME\" }",
            )
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut entries = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            let mut index = 0usize;
            while let Some(entry) = seq.next_element::<SecretEnvElement>().map_err(|e| {
                de::Error::custom(format_args!("secret_env entry at index {index}: {e}"))
            })? {
                entries.push(entry.0);
                index += 1;
            }
            Ok(entries)
        }
    }

    deserializer.deserialize_seq(SecretEnvSeqVisitor)
}

/// One `secret_env` array element during deserialization: either a bare
/// secret-name string or an inline [`SecretEnvConfig`] table. Any other
/// element type (integer, boolean, array, …) is rejected via serde's
/// `invalid_type` machinery with [`SecretEnvElementVisitor`]'s `expecting`
/// message.
struct SecretEnvElement(SecretEnvConfig);

struct SecretEnvElementVisitor;

impl<'de> de::Visitor<'de> for SecretEnvElementVisitor {
    type Value = SecretEnvElement;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a bare secret name string, or an inline table like { secret = \"NAME\" }")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(SecretEnvElement(SecretEnvConfig {
            secret: v.to_string(),
        }))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(SecretEnvElement(SecretEnvConfig { secret: v }))
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        // Delegate to the real struct so `deny_unknown_fields` errors are
        // preserved verbatim for table elements.
        SecretEnvConfig::deserialize(de::value::MapAccessDeserializer::new(map))
            .map(SecretEnvElement)
    }
}

impl<'de> Deserialize<'de> for SecretEnvElement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(SecretEnvElementVisitor)
    }
}

/// One secret reference as an `env` map value (spec 14): in the map form
/// `env = { KEY = { secret = "NAME" } }`, the inline table may carry ONLY a
/// secret reference — never a `name` (the map key IS the name) and never a
/// literal `value`. Deliberately distinct from [`EnvVarConfig`] so
/// `deny_unknown_fields` rejects `name`/`value`/typos inside a map value.
#[derive(Debug, Clone, Deserialize, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
struct EnvSecretRef {
    secret: String,
}

/// Serde/schemars-boundary-only helper for the `env` map value forms
/// (spec 14): a bare string `"value"` is a literal value, an inline table
/// `{ secret = "NAME" }` a secret reference. This enum is NEVER stored —
/// [`deserialize_env`] normalizes every map entry to [`EnvVarConfig`] at
/// parse time, so merge/validation/plan code sees the identical post-parse
/// struct it saw before. It exists so the generated schema can show both
/// value forms.
#[derive(Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)] // serde/schemars-boundary-only: variants are never read in Rust code
enum EnvValueShorthand {
    Bare(String),
    Full(EnvSecretRef),
}

/// Schemars-boundary-only helper for the `env` field shape (spec 14): the
/// field accepts EITHER the classic sequence of `[[env]]` entry tables OR
/// the map form `{ KEY = "value", KEY2 = { secret = "NAME" } }`. This enum
/// is NEVER constructed or deserialized — [`deserialize_env`] normalizes
/// both forms to `Vec<EnvVarConfig>` at parse time; it exists only so
/// `#[schemars(with = "EnvFieldShape")]` renders the field as
/// `anyOf [array-of-EnvVarConfig, object-map]`.
#[derive(schemars::JsonSchema)]
#[serde(untagged)]
#[allow(dead_code)] // schemars-boundary-only: variants are never constructed in Rust code
enum EnvFieldShape {
    Seq(Vec<EnvVarConfig>),
    Map(HashMap<String, EnvValueShorthand>),
}

/// Deserialize `env`, accepting both the classic sequence of `[[env]]` entry
/// tables and the map form `{ KEY = "value", KEY2 = { secret = "NAME" } }`
/// (spec 14), normalizing to `Vec<EnvVarConfig>`. Sequence elements are
/// parsed as [`EnvVarConfig`] exactly as the derived default did (so
/// `deny_unknown_fields`/unknown-field errors inside `[[env]]` tables are
/// preserved verbatim). Map entries are collected in DOCUMENT ORDER —
/// toml_edit's `MapAccess` preserves it, and entries are pushed straight
/// into the output vec in iteration order (no intermediate map, no sorting;
/// duplicate keys are a free TOML-level error). Map values that are neither
/// a bare string literal nor an inline table produce a precise error naming
/// the entry key, the offending value's type, and the two expected forms.
fn deserialize_env<'de, D>(deserializer: D) -> Result<Vec<EnvVarConfig>, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct EnvVisitor;

    impl<'de> de::Visitor<'de> for EnvVisitor {
        type Value = Vec<EnvVarConfig>;

        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str(
                "a sequence of env entry tables, or a map of env names to bare string \
                 literals and/or inline tables like { secret = \"NAME\" }",
            )
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let mut entries = Vec::with_capacity(seq.size_hint().unwrap_or(0));
            let mut index = 0usize;
            while let Some(entry) = seq
                .next_element::<EnvVarConfig>()
                .map_err(|e| de::Error::custom(format_args!("env entry at index {index}: {e}")))?
            {
                entries.push(entry);
                index += 1;
            }
            Ok(entries)
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: de::MapAccess<'de>,
        {
            let mut entries = Vec::with_capacity(map.size_hint().unwrap_or(0));
            while let Some(key) = map.next_key::<String>()? {
                let value = map
                    .next_value::<EnvMapValue>()
                    .map_err(|e| de::Error::custom(format_args!("env map entry '{key}': {e}")))?;
                entries.push(EnvVarConfig {
                    name: key,
                    value: value.value,
                    secret: value.secret,
                });
            }
            Ok(entries)
        }
    }

    deserializer.deserialize_any(EnvVisitor)
}

/// One `env` map value during deserialization: either a bare string literal
/// (the variable's value) or an inline [`EnvSecretRef`] table. Any other
/// value type (integer, boolean, array, …) is rejected via serde's
/// `invalid_type` machinery with [`EnvMapValueVisitor`]'s `expecting`
/// message.
struct EnvMapValue {
    value: Option<String>,
    secret: Option<String>,
}

struct EnvMapValueVisitor;

impl<'de> de::Visitor<'de> for EnvMapValueVisitor {
    type Value = EnvMapValue;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a bare string literal, or an inline table like { secret = \"NAME\" }")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(EnvMapValue {
            value: Some(v.to_string()),
            secret: None,
        })
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(EnvMapValue {
            value: Some(v),
            secret: None,
        })
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        // Delegate to the real struct so `deny_unknown_fields` errors are
        // preserved verbatim for inline-table values.
        let secret_ref = EnvSecretRef::deserialize(de::value::MapAccessDeserializer::new(map))?;
        Ok(EnvMapValue {
            value: None,
            secret: Some(secret_ref.secret),
        })
    }
}

impl<'de> Deserialize<'de> for EnvMapValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_any(EnvMapValueVisitor)
    }
}

/// A file copied from the host into the sandbox at start time
/// (`workloads.<name>.seed_files`). `source` is the host path (validated at
/// the trust boundary), `target` the in-sandbox destination; `only_if_missing`
/// skips the copy when the target already exists.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct SeedFileConfig {
    pub source: String,
    pub target: String,
    pub only_if_missing: Option<bool>,
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
/// discovery-lite). `env` names the environment variable the resolved address
/// of dependency `<dep>` is injected as; `required` (default false) makes a
/// not-running dependency a plan-time refusal instead of a skip.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct DependsOnSpec {
    pub env: String,
    #[serde(default)]
    pub required: bool,
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
    #[serde(default, deserialize_with = "deserialize_env")]
    #[schemars(with = "EnvFieldShape")]
    pub env: Vec<EnvVarConfig>,
    #[serde(default, deserialize_with = "deserialize_secret_env")]
    #[schemars(with = "Vec<SecretEnvShorthand>")]
    pub secret_env: Vec<SecretEnvConfig>,
    #[serde(default)]
    pub ports: Vec<PortMapping>,
    #[serde(default)]
    pub mounts: Vec<MountPlan>,
    #[serde(default)]
    pub seed_files: Vec<SeedFileConfig>,
    pub local_build: Option<LocalBuildConfig>,
    #[serde(default)]
    pub network: NetworkConfig,
    /// Dependency declarations (`workloads.<name>.depends_on.<dep>`; ADR
    /// 0026(d)): each entry names another workload whose address is resolved
    /// from the port registry and injected as the declared env var at plan
    /// time. Layers merge union-by-dependency-name, last layer wins per dep.
    #[serde(default)]
    pub depends_on: HashMap<String, DependsOnSpec>,
}

/// Definition of one named secret (`secrets.<name>` in workestrate.toml).
/// `env_var` is the environment variable the resolved value is injected as;
/// `hosts` constrains which egress hosts may receive it; `required` makes a
/// missing value a hard error; `source`/`exposed_as`/`placeholder`/
/// `description` drive resolution and UX.
#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
pub struct SecretDefConfig {
    pub env_var: Option<String>,
    #[serde(default)]
    pub hosts: Option<Vec<String>>,
    pub required: Option<bool>,
    pub placeholder: Option<String>,
    pub source: Option<String>,
    pub exposed_as: Option<String>,
    pub description: Option<String>,
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
    "secret_env",
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
        assert_eq!(spec.env, "LITELLM_URL");
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
        assert_eq!(spec.env, "LITELLM_URL");
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

    // ---- Spec 13: secret_env string-or-table shorthand ----

    /// Bare strings are shorthand for `{ secret = "NAME" }`: an all-string
    /// array must parse to the equivalent `SecretEnvConfig` entries.
    #[test]
    fn secret_env_bare_strings_parse() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
secret_env = ["A", "B"]
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let secret_env = &config.workloads["pi"].secret_env;
        assert_eq!(
            secret_env,
            &vec![
                SecretEnvConfig {
                    secret: "A".to_string()
                },
                SecretEnvConfig {
                    secret: "B".to_string()
                },
            ]
        );
    }

    /// The existing table form keeps parsing unchanged.
    #[test]
    fn secret_env_table_form_parses() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[[workloads.pi.secret_env]]
secret = "A"
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let secret_env = &config.workloads["pi"].secret_env;
        assert_eq!(
            secret_env,
            &vec![SecretEnvConfig {
                secret: "A".to_string()
            }]
        );
    }

    /// Mixed bare-string and inline-table elements are legal and preserve
    /// order.
    #[test]
    fn secret_env_mixed_forms_parse_in_order() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
secret_env = ["A", { secret = "B" }]
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let secret_env = &config.workloads["pi"].secret_env;
        assert_eq!(
            secret_env,
            &vec![
                SecretEnvConfig {
                    secret: "A".to_string()
                },
                SecretEnvConfig {
                    secret: "B".to_string()
                },
            ]
        );
    }

    /// An explicit empty array parses to an empty vec.
    #[test]
    fn secret_env_empty_array_parses() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
secret_env = []
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        assert!(config.workloads["pi"].secret_env.is_empty());
    }

    /// A non-string/non-table element must fail with a precise error naming
    /// the element index, the offending value's type, and the two expected
    /// forms.
    #[test]
    fn secret_env_bad_element_error_names_index_type_and_forms() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
secret_env = [42]
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("index 0"), "error must name the index: {msg}");
        assert!(
            msg.contains("integer"),
            "error must name the offending type: {msg}"
        );
        assert!(
            msg.contains("bare secret name string"),
            "error must name the bare-string form: {msg}"
        );
        assert!(
            msg.contains("{ secret = \"NAME\" }"),
            "error must name the inline-table form: {msg}"
        );
    }

    /// Unknown fields inside an inline-table element still hard-error
    /// (`deny_unknown_fields` is preserved through the shorthand path).
    #[test]
    fn secret_env_table_element_rejects_unknown_field() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
secret_env = [{ secert = "A" }]
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "inline-table typo must fail: {err}"
        );
    }

    /// The table form serializes via `toml::to_string` and re-parses
    /// identical, exactly as before the shorthand was added.
    #[test]
    fn secret_env_table_form_serializes_and_round_trips() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[[workloads.pi.secret_env]]
secret = "A"

[[workloads.pi.secret_env]]
secret = "B"
"#;
        let config: ConfigFile = toml::from_str(raw).unwrap();
        let serialized = toml::to_string(&config).unwrap();
        let reparsed: ConfigFile = toml::from_str(&serialized).unwrap();
        assert_eq!(reparsed, config);
    }

    // ---- Spec 14: env map form ----

    /// Bare map literals: both the inline-table form `env = { FOO = "bar" }`
    /// and the standard-table form `[workloads.pi.env]` parse to the
    /// identical `EnvVarConfig` entry (name from the key, value set, no
    /// secret).
    #[test]
    fn env_map_bare_literals_parse() {
        let expected = vec![EnvVarConfig {
            name: "FOO".to_string(),
            value: Some("bar".to_string()),
            secret: None,
        }];
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

    /// A map value that is an inline table is a secret reference:
    /// `env = { API_KEY = { secret = "MY_SECRET" } }` sets `secret`,
    /// leaving `value` unset.
    #[test]
    fn env_map_secret_ref_parses() {
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
            vec![EnvVarConfig {
                name: "API_KEY".to_string(),
                value: None,
                secret: Some("MY_SECRET".to_string()),
            }]
        );
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
            vec![
                EnvVarConfig {
                    name: "ZEBRA".to_string(),
                    value: Some("z".to_string()),
                    secret: None,
                },
                EnvVarConfig {
                    name: "MIDDLE".to_string(),
                    value: None,
                    secret: Some("M_SECRET".to_string()),
                },
                EnvVarConfig {
                    name: "ALPHA".to_string(),
                    value: Some("a".to_string()),
                    secret: None,
                },
            ]
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

    /// The classic array-of-tables form parses unchanged (backward compat).
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
            vec![
                EnvVarConfig {
                    name: "FOO".to_string(),
                    value: Some("bar".to_string()),
                    secret: None,
                },
                EnvVarConfig {
                    name: "BAZ".to_string(),
                    value: None,
                    secret: Some("BAZ_SECRET".to_string()),
                },
            ]
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

    /// The array-of-tables form serializes via `toml::to_string` and
    /// re-parses identical, exactly as before the map form was added.
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

    /// A non-string/non-table map value must fail with a precise error
    /// naming the entry key, the offending value's type, and the two
    /// expected forms.
    #[test]
    fn env_map_bad_value_error_names_key_type_and_forms() {
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
        assert!(
            msg.contains("bare string literal"),
            "error must name the bare-string form: {msg}"
        );
        assert!(
            msg.contains("{ secret = \"NAME\" }"),
            "error must name the inline-table form: {msg}"
        );
    }

    /// Unknown fields inside an inline-table map value still hard-error
    /// (`deny_unknown_fields` is preserved through the map-form path).
    #[test]
    fn env_map_inline_table_rejects_unknown_field() {
        let raw = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []
env = { API_KEY = { secert = "MY_SECRET" } }
"#;
        let err = toml::from_str::<ConfigFile>(raw).unwrap_err();
        assert!(
            err.to_string().contains("unknown field"),
            "inline-table typo must fail: {err}"
        );
    }
}
