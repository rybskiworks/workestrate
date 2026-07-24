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

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

/// A single workload definition (`workloads.<name>` in workestrate.toml).
/// `kind` is `"agent"` (interactive TUI attach) or `"service"` (headless,
/// detached by default); the remaining fields describe the image, resources,
/// command, env/secret wiring, mounts, ports, seed files, local-build
/// override, and network policy. All fields merge layer-by-layer via
/// `merge::merge_layers`.
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
    pub env: Vec<EnvVarConfig>,
    #[serde(default)]
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
}
