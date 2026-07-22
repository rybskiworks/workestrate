use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::policy;
use crate::recipes::EgressRecipeRef;

/// The resolved active context.
/// `name` is None when no contexts are defined (bare-layers backward-compat).
#[derive(Debug, Clone)]
pub struct ActiveContext {
    pub name: Option<String>,
    pub layers: Vec<String>,
}

thread_local! {
    static ACTIVE_CONTEXT: std::cell::RefCell<Option<ActiveContext>> =
        const { std::cell::RefCell::new(None) };
}

/// Store the active context for the current thread.
pub fn set_active_context(ctx: Option<ActiveContext>) {
    ACTIVE_CONTEXT.with(|c| {
        *c.borrow_mut() = ctx;
    });
}

/// Get the active context name (None = bare-layers backward-compat).
pub fn active_context_name() -> Option<String> {
    ACTIVE_CONTEXT.with(|c| c.borrow().as_ref().and_then(|ctx| ctx.name.clone()))
}

pub fn project_root() -> Result<PathBuf> {
    // 1. AGENTCTL_ROOT env var
    let root = if let Ok(root) = std::env::var("AGENTCTL_ROOT") {
        PathBuf::from(root)
    }
    // 2. Walk up from CARGO_MANIFEST_DIR (compile-time, works in cargo run)
    else if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut path = PathBuf::from(manifest);
        if path.pop() && path.pop() {
            path
        } else {
            std::env::current_dir()?
        }
    }
    // 3. Current working directory
    else {
        std::env::current_dir()?
    };

    // Validate: the root must contain flake.nix
    if !root.join("flake.nix").exists() {
        anyhow::bail!(
            "resolved project root '{}' does not contain flake.nix.\n\
             Set AGENTCTL_ROOT or run from the workbench root directory.",
            root.display()
        );
    }

    Ok(root)
}

/// Best-effort variant of [`project_root`] for the standalone-installed-tool
/// model (review finding E1, WP5). Returns `None` instead of bailing when no
/// workbench checkout can be located — letting callers like `cmd_check` and
/// `find_reference_config` degrade gracefully.
///
/// Callers that genuinely require a workbench root (e.g. workload source /
/// build resolution at sandbox-start time) should keep using [`project_root`]
/// so the hard failure surfaces at the operation that needs it.
pub fn project_root_optional() -> Option<PathBuf> {
    // 1. AGENTCTL_ROOT env var
    if let Ok(root) = std::env::var("AGENTCTL_ROOT") {
        let p = PathBuf::from(root);
        if p.join("flake.nix").exists() {
            return Some(p);
        }
    }
    // 2. Walk up from CARGO_MANIFEST_DIR (cargo run / cargo test).
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut path = PathBuf::from(manifest);
        if path.pop() && path.pop() && path.join("flake.nix").exists() {
            return Some(path);
        }
    }
    // 3. Current working directory + flake.nix check.
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join("flake.nix").exists() {
            return Some(cwd);
        }
    }
    None
}

/// One row in the `agentctl check` report.
#[derive(Debug, Clone)]
pub struct CheckEntry {
    /// Human-readable label printed in the report.
    pub label: String,
    /// Whether the required artifact is present.
    pub ok: bool,
    /// When `true`, a missing artifact is reported as a warning and does
    /// not cause the command to exit non-zero. Used for optional local
    /// overrides such as `agents/<name>/repo` checkouts.
    pub optional: bool,
}

struct CheckSpec {
    label: &'static str,
    path: PathBuf,
    optional: bool,
}

fn required(label: &'static str, path: PathBuf) -> CheckSpec {
    CheckSpec {
        label,
        path,
        optional: false,
    }
}

fn optional(label: &'static str, path: PathBuf) -> CheckSpec {
    CheckSpec {
        label,
        path,
        optional: true,
    }
}

/// Run the full set of sanity checks for the workbench layout.
///
/// `agents/<name>/repo` directories are documented as optional local
/// overrides (the agents can also be supplied via flake inputs), so missing
/// directories are reported as `[MISSING] (optional)` and do not fail the
/// command. Any other missing artifact is fatal.
pub fn check_required_files(root: &Path) -> Result<Vec<CheckEntry>> {
    let specs: Vec<CheckSpec> = vec![
        required("ai-workbench root", root.to_path_buf()),
        required("flake.nix", root.join("flake.nix")),
        required(
            "config.reference/workestrate.toml",
            root.join("config.reference").join("workestrate.toml"),
        ),
        required(
            "infra/microsandbox/sdk-notes.md",
            root.join("infra/microsandbox/sdk-notes.md"),
        ),
        required("profiles/litellm.md", root.join("profiles/litellm.md")),
        required("profiles/agents/pi.md", root.join("profiles/agents/pi.md")),
        required(
            "profiles/agents/odysseus.md",
            root.join("profiles/agents/odysseus.md"),
        ),
        required(
            "profiles/agents/opencode.md",
            root.join("profiles/agents/opencode.md"),
        ),
        required(
            "profiles/agents/tempest.md",
            root.join("profiles/agents/tempest.md"),
        ),
        optional("workspaces/", root.join("workspaces")),
        optional("var/", root.join("var")),
        // Optional: agent repos are typically supplied via flake
        // inputs. A fresh clone may legitimately omit local
        // `agents/<name>/repo` checkouts.
        optional("agents/pi/repo", root.join("agents/pi/repo")),
        optional("agents/odysseus/repo", root.join("agents/odysseus/repo")),
        optional("agents/opencode/repo", root.join("agents/opencode/repo")),
        optional("agents/pi/build", root.join("agents/pi/build")),
        optional("agents/odysseus/build", root.join("agents/odysseus/build")),
        optional("agents/opencode/build", root.join("agents/opencode/build")),
        optional("agents/tempest/repo", root.join("agents/tempest/repo")),
        optional("agents/tempest/build", root.join("agents/tempest/build")),
    ];

    Ok(specs
        .into_iter()
        .map(|spec| CheckEntry {
            label: spec.label.to_string(),
            ok: spec.path.exists(),
            optional: spec.optional,
        })
        .collect())
}

// ---------------------------------------------------------------------------
// workestrate.toml schema
// ---------------------------------------------------------------------------

use crate::microsandbox::plan::{DenyDomainRule, IngressRule, MountPlan, PortMapping};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
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

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[allow(dead_code)]
pub struct BinarySpec {
    pub recipe: String,
    pub src: String,
    pub entrypoint: Option<String>,
    pub worker: Option<String>,
    pub npm_deps_hash: Option<String>,
    pub install_layout: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[allow(dead_code)]
pub struct BakedFileSpec {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[allow(dead_code)]
pub struct EnvVarConfig {
    pub name: String,
    pub value: Option<String>,
    pub secret: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[allow(dead_code)]
pub struct SecretEnvConfig {
    pub secret: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
#[allow(dead_code)]
pub struct SeedFileConfig {
    pub source: String,
    pub target: String,
    pub only_if_missing: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
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

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
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

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
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

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
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

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, schemars::JsonSchema)]
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
// Tool home resolution (ADR 0023 single-home layout)
// ---------------------------------------------------------------------------

/// How the workestrate tool home was resolved (ADR 0023 single-home layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeKind {
    /// `WORKESTRATE_HOME` env var — new single-home layout.
    Env,
    /// Auto-discovered `.workestrate/config.toml` inside a trusted project — new layout.
    Discovered,
    /// Legacy XDG layout (`XDG_*_HOME` set) — compatibility, read/write as before.
    LegacyXdg,
    /// Default `~/.workestrate` — new single-home layout.
    Default,
}

/// One-time stderr migration note for the legacy XDG layout.
static LEGACY_NOTE: std::sync::Once = std::sync::Once::new();

fn emit_legacy_xdg_note() {
    LEGACY_NOTE.call_once(|| {
        eprintln!(
            "note: using legacy XDG workestrate layout; run 'workestrate migrate-home' to \
             consolidate into a single WORKESTRATE_HOME"
        );
    });
}

/// One-time stderr warning when an untrusted `.workestrate/config.toml` is found
/// during discovery. `resolve_home_with_kind` is called many times per command
/// (cmd_check, registry_path, store/state resolution, ...); without this guard
/// the warning would print once per call.
static DISCOVERY_WARN: std::sync::Once = std::sync::Once::new();

fn emit_untrusted_discovery_warn(dir: &Path) {
    DISCOVERY_WARN.call_once(|| {
        eprintln!(
            ".workestrate/config.toml found in {} but it is not a trusted project; \
             ignoring (run 'workestrate config trust <dir>' to trust it)",
            dir.display()
        );
    });
}

fn xdg_var_set(name: &str) -> bool {
    std::env::var(name).map(|v| !v.is_empty()).unwrap_or(false)
}

/// Base home resolution WITHOUT discovery (Env/LegacyXdg/Default only).
///
/// Used by the discovery trust-check ([`is_dir_trusted_via_base_registry`]) so
/// that loading the global trust registry cannot recurse back into discovery.
fn resolve_home_base_with_kind() -> (PathBuf, HomeKind) {
    // (a) Env: WORKESTRATE_HOME
    if let Ok(value) = std::env::var("WORKESTRATE_HOME") {
        if !value.is_empty() {
            return (expand_tilde(&value), HomeKind::Env);
        }
    }
    // (c) Legacy XDG
    if xdg_var_set("XDG_CONFIG_HOME")
        || xdg_var_set("XDG_DATA_HOME")
        || xdg_var_set("XDG_STATE_HOME")
    {
        return (xdg_config_dir(), HomeKind::LegacyXdg);
    }
    // (d) Default
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    (PathBuf::from(home).join(".workestrate"), HomeKind::Default)
}

/// The registry path computed from the *base* resolution (no discovery).
///
/// Location of the global trust list, independent of any discovered project
/// home — so a hostile `.workestrate/` cannot self-trust.
fn base_registry_path() -> PathBuf {
    let (home, kind) = resolve_home_base_with_kind();
    match kind {
        HomeKind::LegacyXdg => xdg_config_dir().join("config.toml"),
        _ => home.join("config.toml"),
    }
}

/// Trust-check used ONLY inside discovery; reads the base registry directly to
/// avoid recursing through [`registry_path`] → [`resolve_home_with_kind`].
fn is_dir_trusted_via_base_registry(dir: &Path) -> bool {
    let path = base_registry_path();
    if !path.exists() {
        return false;
    }
    let reg = match std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| toml::from_str::<Registry>(&c).ok())
    {
        Some(reg) => reg,
        None => return false,
    };
    let canonical_dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    reg.trusted_projects.iter().any(|p| {
        let expanded = expand_tilde(&p.path);
        let canonical_p = std::fs::canonicalize(&expanded)
            .or_else(|_| std::fs::canonicalize(&p.path))
            .unwrap_or_else(|_| expanded.clone());
        canonical_p == canonical_dir || expanded == dir || Path::new(&p.path) == dir
    })
}

/// Resolve the workestrate tool home and how it was chosen (ADR 0023).
///
/// Precedence (first match wins):
/// 1. **Env** — `WORKESTRATE_HOME` (used verbatim, leading `~/` expanded).
/// 2. **Discovered** — a `.workestrate/config.toml` in a *trusted* ancestor of
///    the cwd, but only when no `XDG_*_HOME` var is set; an explicit XDG var is
///    a deliberate legacy-layout signal that discovery must not override. An
///    untrusted discovery prints a one-time warning and *stops* walking (does
///    not keep looking higher), then falls through.
/// 3. **LegacyXdg** — any of `XDG_CONFIG_HOME`/`XDG_DATA_HOME`/`XDG_STATE_HOME`
///    set and non-empty (compatibility; emits a one-time migration note).
/// 4. **Default** — `~/.workestrate`.
pub fn resolve_home_with_kind() -> (PathBuf, HomeKind) {
    // (a) Env: WORKESTRATE_HOME
    if let Ok(value) = std::env::var("WORKESTRATE_HOME") {
        if !value.is_empty() {
            return (expand_tilde(&value), HomeKind::Env);
        }
    }

    let xdg_explicit = xdg_var_set("XDG_CONFIG_HOME")
        || xdg_var_set("XDG_DATA_HOME")
        || xdg_var_set("XDG_STATE_HOME");

    // (b) Discovery — only when XDG is NOT explicitly set. An explicit XDG
    // var is a deliberate legacy-layout choice that discovery must not
    // override (keeps XDG-pinned environments and tests working even when
    // a trusted .workestrate/config.toml exists in an ancestor).
    if !xdg_explicit {
        if let Ok(cwd) = std::env::current_dir() {
            let mut dir: &Path = &cwd;
            loop {
                let candidate = dir.join(".workestrate").join("config.toml");
                if candidate.exists() {
                    if is_dir_trusted_via_base_registry(dir) {
                        return (dir.join(".workestrate"), HomeKind::Discovered);
                    }
                    // Untrusted: warn (once per process), STOP walking, fall through.
                    emit_untrusted_discovery_warn(dir);
                    break;
                }
                match dir.parent() {
                    Some(parent) => dir = parent,
                    None => break,
                }
            }
        }
    }

    // (c) Legacy XDG
    if xdg_explicit {
        emit_legacy_xdg_note();
        return (xdg_config_dir(), HomeKind::LegacyXdg);
    }

    // (d) Default
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    (PathBuf::from(home).join(".workestrate"), HomeKind::Default)
}

/// Resolve the workestrate tool home (ADR 0023).
pub fn resolve_home() -> PathBuf {
    resolve_home_with_kind().0
}

// ---------------------------------------------------------------------------
// XDG path resolution
// ---------------------------------------------------------------------------

/// XDG config dir for workestrate: $XDG_CONFIG_HOME/workestrate/ or ~/.config/workestrate/
pub fn xdg_config_dir() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(".config")
        });
    base.join("workestrate")
}

/// XDG data dir for workestrate: $XDG_DATA_HOME/workestrate/ or ~/.local/share/workestrate/
pub fn xdg_data_dir() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(".local").join("share")
        });
    base.join("workestrate")
}

/// XDG state dir for workestrate: $XDG_STATE_HOME/workestrate/ or ~/.local/state/workestrate/
pub fn xdg_state_dir() -> PathBuf {
    let base = std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(".local").join("state")
        });
    base.join("workestrate")
}

/// Registry path: the active tool home's `config.toml`.
///
/// In legacy XDG mode this is `$XDG_CONFIG_HOME/workestrate/config.toml`
/// (unchanged from pre-ADR-0023); in every other mode it is `<home>/config.toml`.
pub fn registry_path() -> PathBuf {
    let (home, kind) = resolve_home_with_kind();
    match kind {
        HomeKind::LegacyXdg => xdg_config_dir().join("config.toml"),
        _ => home.join("config.toml"),
    }
}

/// Overrides path: the active tool home's `overrides.toml`.
pub fn overrides_path() -> PathBuf {
    let (home, kind) = resolve_home_with_kind();
    match kind {
        HomeKind::LegacyXdg => xdg_config_dir().join("overrides.toml"),
        _ => home.join("overrides.toml"),
    }
}

/// Config repo store: resolve_store_dir()/repos/<name>/
pub fn config_repo_dir(name: &str) -> PathBuf {
    resolve_store_dir().join("repos").join(name)
}

/// Source override store: resolve_store_dir()/sources/<name>/
pub fn source_store_dir(name: &str) -> PathBuf {
    resolve_store_dir().join("sources").join(name)
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default, schemars::JsonSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
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

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TrustedProject {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, schemars::JsonSchema)]
pub struct Context {
    #[serde(default)]
    pub layers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, schemars::JsonSchema)]
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

pub fn load_registry() -> Result<Option<Registry>> {
    let path = registry_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("failed to read registry {}: {}", path.display(), e))?;
    let registry: Registry = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("failed to parse registry {}: {}", path.display(), e))?;
    Ok(Some(registry))
}

pub fn save_registry(registry: &Registry) -> Result<()> {
    let path = registry_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(registry)
        .map_err(|e| anyhow::anyhow!("failed to serialize registry: {}", e))?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Insert/replace a config repo entry in the registry. If `layers` is empty,
/// push `name` as the default layer (mirrors cmd_config_add's behavior).
/// Shared by `cmd_config_add` (clone + register) and `cmd_config_new`
/// (local path + register). For local-path scaffolds, pass `git_ref = None`
/// and `rev = None` — `cmd_config_update` recognizes this as a local-path
/// repo and skips the pull step.
pub fn register_config(
    name: &str,
    url: &str,
    git_ref: Option<&str>,
    rev: Option<&str>,
) -> Result<()> {
    let mut registry = load_registry()?.unwrap_or_default();
    registry.configs.insert(
        name.to_string(),
        ConfigRepoEntry {
            url: url.to_string(),
            r#ref: git_ref.map(|s| s.to_string()),
            rev: rev.map(|s| s.to_string()),
            secrets: None,
            secrets_file: None,
            age_key_file: None,
        },
    );
    if registry.layers.is_empty() {
        registry.layers.push(name.to_string());
    }
    save_registry(&registry)
}

/// Validate a config repo name for `workestrate config new`. Same safe-set
/// as workload names: names flow into both filesystem paths (the registry
/// store dir) and registry TOML keys, so the intersection `[a-z0-9-]` is
/// the only safe charset.
///
/// Pattern: `^[a-z0-9][a-z0-9-]{0,62}$` — starts with an alphanumeric, allows
/// lowercase letters / digits / hyphens, max 63 characters (DNS-label length).
/// Rejects empty / overlong / uppercase / underscores / dots / slashes / shell
/// metacharacters / leading hyphen.
///
/// "Escape nothing — reject instead" is the policy. This is a thin wrapper
/// around [`validate_identifier`] shared with [`validate_workload_name`] in
/// main.rs (mirrored here to keep config-domain logic in config.rs without a
/// cross-module dependency from main.rs's validator).
pub fn validate_config_name(name: &str) -> Result<()> {
    validate_identifier(name, "config name")
}

/// Shared identifier validator. `label` is interpolated into error messages
/// ("workload name ...", "config name ..."). Pattern:
/// `^[a-z0-9][a-z0-9-]{0,62}$`.
fn validate_identifier(name: &str, label: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("{label} cannot be empty");
    }
    if name.len() > 63 {
        anyhow::bail!(
            "{label} cannot exceed 63 characters (got {}): '{}'",
            name.len(),
            name
        );
    }
    let mut chars = name.chars();
    let first_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    if !first_ok {
        anyhow::bail!(
            "{label} must start with [a-z0-9]; \
             pattern: ^[a-z0-9][a-z0-9-]{{0,62}}$; got: '{name}'"
        );
    }
    for c in chars {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' {
            anyhow::bail!(
                "{label} contains invalid character '{}' (allowed: [a-z0-9-]); \
                 pattern: ^[a-z0-9][a-z0-9-]{{0,62}}$; got: '{}'",
                c,
                name
            );
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// User-global overrides
// ---------------------------------------------------------------------------

/// Known top-level ConfigFile fields (for unknown-field detection in overrides).
const CONFIG_FIELDS: &[&str] = &["schema_version", "secrets", "workloads"];

/// Known WorkloadConfig fields (for unknown-field detection in overrides).
const WORKLOAD_FIELDS: &[&str] = &[
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

/// Load user-global override layers from overrides.toml.
///
/// Returns layers in precedence order: [global] first, then [configs.<name>]
/// for each name in `context_layers` (in order). Missing overrides.toml →
/// empty vec (silently absent).
///
/// LENIENT semantics:
/// - Unknown config section ([configs.team] when team not in context) → skip + INFO log
/// - Unknown workload section ([configs.team.workloads.nonexistent]) → skip + INFO log
/// - Unknown field in a matched section → loud WARNING (probable typo)
pub fn load_overrides(
    overrides_path: &Path,
    context_layers: &[String],
    existing_workloads: &std::collections::HashSet<String>,
) -> Result<Vec<crate::merge::Layer>> {
    if !overrides_path.exists() {
        return Ok(vec![]); // silently absent
    }
    let content = std::fs::read_to_string(overrides_path).map_err(|e| {
        anyhow::anyhow!(
            "failed to read overrides {}: {}",
            overrides_path.display(),
            e
        )
    })?;
    let raw: toml::Value = toml::from_str(&content).map_err(|e| {
        anyhow::anyhow!(
            "failed to parse overrides {}: {}",
            overrides_path.display(),
            e
        )
    })?;

    let mut layers = Vec::new();

    // [global] section — applied to every context.
    if let Some(global) = raw.get("global").and_then(|v| v.as_table()) {
        let processed = process_override_section(global, "global", existing_workloads)?;
        if !processed.is_empty() {
            layers.push(crate::merge::Layer::from_string(
                "global-override",
                &processed,
            )?);
        }
    }

    // [configs.<name>] sections — only when name is in context_layers.
    if let Some(configs) = raw.get("configs").and_then(|v| v.as_table()) {
        for (name, value) in configs {
            let table = match value.as_table() {
                Some(t) => t,
                None => continue,
            };
            if !context_layers.contains(name) {
                eprintln!(
                    "INFO: override section [configs.{}] has no matching config in this context, skipping",
                    name
                );
                continue;
            }
            let section_path = format!("configs.{}", name);
            let processed = process_override_section(table, &section_path, existing_workloads)?;
            if !processed.is_empty() {
                layers.push(crate::merge::Layer::from_string(
                    &format!("configs.{}-override", name),
                    &processed,
                )?);
            }
        }
    }

    Ok(layers)
}

/// Process an override section: check unknown fields, filter unknown workloads,
/// return the processed TOML string.
fn process_override_section(
    table: &toml::map::Map<String, toml::Value>,
    section_path: &str,
    existing_workloads: &std::collections::HashSet<String>,
) -> Result<String> {
    // Check unknown fields at ConfigFile level.
    for key in table.keys() {
        if !CONFIG_FIELDS.contains(&key.as_str()) {
            eprintln!(
                "WARNING: unknown field '{}' in override section [{}] (probable typo)",
                key, section_path
            );
        }
    }

    // Clone the table for filtering.
    let mut value = toml::Value::Table(table.clone());

    // Process workloads sub-table: filter unknown workloads + check fields.
    if let Some(workloads) = value.get_mut("workloads").and_then(|v| v.as_table_mut()) {
        let unknown: Vec<String> = workloads
            .keys()
            .filter(|k| !existing_workloads.contains(k.as_str()))
            .cloned()
            .collect();
        for k in &unknown {
            eprintln!(
                "INFO: override section [{}.workloads.{}] has no matching workload in this context, skipping",
                section_path, k
            );
            workloads.remove(k);
        }

        // Check unknown fields in each remaining workload sub-table.
        for (wl_name, wl_value) in workloads.iter() {
            if let Some(wl_table) = wl_value.as_table() {
                for key in wl_table.keys() {
                    if !WORKLOAD_FIELDS.contains(&key.as_str()) {
                        eprintln!(
                            "WARNING: unknown field '{}' in override section [{}.workloads.{}] (probable typo)",
                            key, section_path, wl_name
                        );
                    }
                }
            }
        }
    }

    toml::to_string(&value).map_err(|e| {
        anyhow::anyhow!(
            "failed to serialize override section [{}]: {}",
            section_path,
            e
        )
    })
}

/// Resolve the active context.
///
/// Precedence:
/// 1. WORKESTRATE_CONTEXT env (set by --context flag or by user)
/// 2. [settings] default_context
/// 3. If NO contexts defined: bare `layers` (backward compat)
///
/// Returns ActiveContext { name: None, layers: registry.layers } when no
/// contexts are defined (backward-compat). Returns an error if contexts are
/// defined but neither env nor default_context resolves to a valid context.
pub fn resolve_active_context() -> Result<ActiveContext> {
    let registry = match load_registry()? {
        Some(r) => r,
        None => {
            return Ok(ActiveContext {
                name: None,
                layers: vec![],
            })
        }
    };

    // Backward-compat: no contexts defined → use bare layers.
    if registry.contexts.is_empty() {
        return Ok(ActiveContext {
            name: None,
            layers: registry.layers,
        });
    }

    // 1. WORKESTRATE_CONTEXT env (set by --context flag or by user)
    if let Ok(name) = std::env::var("WORKESTRATE_CONTEXT") {
        if let Some(ctx) = registry.contexts.get(&name) {
            return Ok(ActiveContext {
                name: Some(name),
                layers: ctx.layers.clone(),
            });
        }
        anyhow::bail!(
            "context '{}' not found in registry; available contexts: {}",
            name,
            registry
                .contexts
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // 2. [settings] default_context
    if let Some(ref default) = registry.settings.default_context {
        if let Some(ctx) = registry.contexts.get(default) {
            return Ok(ActiveContext {
                name: Some(default.clone()),
                layers: ctx.layers.clone(),
            });
        }
        anyhow::bail!(
            "default_context '{}' not found in registry contexts; available: {}",
            default,
            registry
                .contexts
                .keys()
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    // 3. Contexts exist but no env/default → error
    anyhow::bail!(
        "contexts are defined but no default_context is set; use --context <name> or set WORKESTRATE_CONTEXT env. Available contexts: {}",
        registry.contexts.keys().cloned().collect::<Vec<_>>().join(", ")
    );
}

pub fn resolve_state_dir() -> PathBuf {
    if let Ok(Some(registry)) = load_registry() {
        if let Some(ref state_dir) = registry.settings.state_dir {
            return expand_tilde(state_dir);
        }
    }
    let (home, kind) = resolve_home_with_kind();
    match kind {
        HomeKind::LegacyXdg => xdg_state_dir(),
        _ => home.join("state"),
    }
}

pub fn resolve_store_dir() -> PathBuf {
    if let Ok(Some(registry)) = load_registry() {
        if let Some(ref store_dir) = registry.settings.store_dir {
            return expand_tilde(store_dir);
        }
    }
    let (home, kind) = resolve_home_with_kind();
    match kind {
        HomeKind::LegacyXdg => xdg_data_dir(),
        _ => home,
    }
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(home).join(rest)
    } else {
        PathBuf::from(path)
    }
}

// ---------------------------------------------------------------------------
// Trust gating
// ---------------------------------------------------------------------------

/// Whether `dir` is in the registry's `[trusted_projects]` list.
///
/// Path comparison canonicalizes both sides first (closes review finding A18),
/// so a project registered via a symlink or a `..`-containing path matches
/// queries through any equivalent path. Falls back to lexical comparison
/// (with `expand_tilde`) when canonicalization fails (e.g. broken symlink,
/// non-existent path), preserving backward compatibility with pre-A18
/// registries that stored non-canonical paths.
pub fn is_trusted_project(dir: &Path) -> bool {
    let canonical_dir = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    if let Ok(Some(registry)) = load_registry() {
        registry.trusted_projects.iter().any(|p| {
            let expanded = expand_tilde(&p.path);
            let canonical_p = std::fs::canonicalize(&expanded)
                .or_else(|_| std::fs::canonicalize(&p.path))
                .unwrap_or_else(|_| expanded.clone());
            canonical_p == canonical_dir || expanded == dir || Path::new(&p.path) == dir
        })
    } else {
        false
    }
}

/// Register `dir` as trusted. The path is canonicalized before storage (closes
/// review finding A18) so future queries through symlinks or `..`-containing
/// paths match consistently. Dedup considers both canonical and lexical forms
/// of existing entries.
pub fn trust_project(dir: &Path) -> Result<()> {
    let mut registry = load_registry()?.unwrap_or_default();
    let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let dir_str = canonical.to_string_lossy().to_string();
    let already = registry.trusted_projects.iter().any(|p| {
        if p.path == dir_str {
            return true;
        }
        let expanded = expand_tilde(&p.path);
        std::fs::canonicalize(&expanded)
            .map(|c| c == canonical)
            .unwrap_or(false)
    });
    if !already {
        registry
            .trusted_projects
            .push(TrustedProject { path: dir_str });
        save_registry(&registry)?;
    }
    Ok(())
}

/// Remove `dir` from the trusted list. Matches by canonical OR lexical form
/// (closes review finding A18) so untrusting via a different-but-equivalent
/// path still works.
pub fn untrust_project(dir: &Path) -> Result<()> {
    let mut registry = load_registry()?.unwrap_or_default();
    let canonical = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    let canonical_str = canonical.to_string_lossy().to_string();
    registry.trusted_projects.retain(|p| {
        if p.path == canonical_str {
            return false;
        }
        let expanded = expand_tilde(&p.path);
        let p_canonical = std::fs::canonicalize(&expanded).unwrap_or_else(|_| expanded.clone());
        p_canonical != canonical
    });
    save_registry(&registry)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// migrate-home (ADR 0023): consolidate legacy layouts into a single home
// ---------------------------------------------------------------------------

/// One moved file/dir recorded by [`run_migrate_home`].
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct MovedEntry {
    pub src: String,
    pub dst: String,
}

/// Structured summary of a `workestrate migrate-home` run (ADR 0023).
#[derive(Debug, Clone, serde::Serialize)]
pub(crate) struct MigrateSummary {
    pub from: String,
    pub dest: String,
    pub dry_run: bool,
    pub moved: Vec<MovedEntry>,
    pub registry_updated: bool,
    pub home_version: Option<u32>,
    /// Names of `configs.<name>` entries whose local `url` pointed into the
    /// old layout and was rewritten to the new `repos/<name>` path. Empty in
    /// dry-run (no editing happens) and when no local urls matched.
    pub urls_rewritten: Vec<String>,
}

/// Source layout for a migration (paths read FROM, into `dest`).
struct MigrateSources {
    registry: PathBuf,
    overrides: PathBuf,
    secrets: PathBuf,
    repos_root: PathBuf,
    sources_root: PathBuf,
    state_root: PathBuf,
    /// Old subtrees to clean up after a successful in-place move (bundle only).
    cleanup_dirs: Vec<PathBuf>,
}

/// Resolve the source layout. `dest` is the destination single-home dir.
///
/// - `from == "xdg"`: legacy `XDG_*_HOME` dirs.
/// - `from == "bundle"`: the old `.workestrate/{config,data,state}/workestrate/`
///   triplication rooted at `dest`'s parent (in-place bundle).
fn resolve_migrate_sources(from: &str, dest: &Path) -> Result<MigrateSources> {
    match from {
        "xdg" => Ok(MigrateSources {
            registry: xdg_config_dir().join("config.toml"),
            overrides: xdg_config_dir().join("overrides.toml"),
            secrets: xdg_config_dir().join(".env.local.enc"),
            repos_root: xdg_data_dir().join("repos"),
            sources_root: xdg_data_dir().join("sources"),
            state_root: xdg_state_dir(),
            cleanup_dirs: Vec::new(),
        }),
        "bundle" => {
            // In-place bundle: dest == <bundle_root>/.workestrate, so the old
            // triplication lives directly under dest as {config,data,state}/workestrate.
            Ok(MigrateSources {
                registry: dest.join("config").join("workestrate").join("config.toml"),
                overrides: dest
                    .join("config")
                    .join("workestrate")
                    .join("overrides.toml"),
                secrets: dest
                    .join("config")
                    .join("workestrate")
                    .join(".env.local.enc"),
                repos_root: dest.join("data").join("workestrate").join("repos"),
                sources_root: dest.join("data").join("workestrate").join("sources"),
                state_root: dest.join("state").join("workestrate"),
                // NOTE: dest/state is BOTH the old state subtree's parent and
                // the NEW state destination (new layout moves files INTO
                // dest/state). So we clean up only the old `state/workestrate`
                // subdir, never `dest/state` itself, to avoid nuking the
                // just-moved state. dest/config and dest/data have no new
                // layout files under them and can be removed wholesale.
                cleanup_dirs: vec![
                    dest.join("config"),
                    dest.join("data"),
                    dest.join("state").join("workestrate"),
                ],
            })
        }
        other => anyhow::bail!(
            "unknown --from value '{}' (expected \"xdg\" or \"bundle\")",
            other
        ),
    }
}

/// Recursively copy an entry (file/dir/symlink). Cross-filesystem fallback for
/// [`move_entry`].
fn copy_entry_recursive(src: &Path, dst: &Path) -> Result<()> {
    let meta = std::fs::symlink_metadata(src)?;
    if meta.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            copy_entry_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        }
    } else if meta.file_type().is_symlink() {
        let target = std::fs::read_link(src)?;
        #[cfg(unix)]
        {
            let _ = std::os::unix::fs::symlink(&target, dst);
            if !dst.exists() && std::fs::symlink_metadata(dst).is_err() {
                std::fs::copy(src, dst)?;
            }
        }
        #[cfg(not(unix))]
        {
            std::fs::copy(src, dst)?;
        }
    } else {
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

/// Move a filesystem entry, falling back to recursive copy + delete when a
/// simple `rename` fails (e.g. crossing a filesystem boundary).
fn move_entry(src: &Path, dst: &Path) -> Result<()> {
    if std::fs::rename(src, dst).is_ok() {
        return Ok(());
    }
    copy_entry_recursive(src, dst)?;
    let meta = std::fs::symlink_metadata(src)
        .map_err(|e| anyhow::anyhow!("stat moved source {}: {}", src.display(), e))?;
    if meta.is_dir() {
        std::fs::remove_dir_all(src)?;
    } else {
        std::fs::remove_file(src)?;
    }
    Ok(())
}

/// Collect the planned (src, dst) moves for a given source layout + dest.
fn plan_moves(sources: &MigrateSources, dest: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut moves: Vec<(PathBuf, PathBuf)> = Vec::new();
    if sources.registry.exists() {
        moves.push((sources.registry.clone(), dest.join("config.toml")));
    }
    if sources.overrides.exists() {
        moves.push((sources.overrides.clone(), dest.join("overrides.toml")));
    }
    if sources.secrets.exists() {
        moves.push((
            sources.secrets.clone(),
            dest.join("secrets").join(".env.local.enc"),
        ));
    }
    if sources.repos_root.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&sources.repos_root) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let p = entry.path();
                moves.push((p, dest.join("repos").join(name)));
            }
        }
    }
    if sources.sources_root.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&sources.sources_root) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let p = entry.path();
                moves.push((p, dest.join("sources").join(name)));
            }
        }
    }
    if sources.state_root.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&sources.state_root) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let p = entry.path();
                moves.push((p, dest.join("state").join(name)));
            }
        }
    }
    moves
}

/// Auto-detect the source layout when `--from` is omitted.
///
/// Prefers "bundle" when a `.workestrate/config/workestrate/config.toml` exists
/// in the cwd, otherwise treats the source as the legacy XDG layout.
fn detect_layout() -> &'static str {
    if let Ok(cwd) = std::env::current_dir() {
        if cwd
            .join(".workestrate")
            .join("config")
            .join("workestrate")
            .join("config.toml")
            .exists()
        {
            return "bundle";
        }
    }
    "xdg"
}

/// Detect whether a string looks like a remote URL (carries a scheme) rather
/// than a local filesystem path. Used by [`run_migrate_home`] to avoid
/// rewriting genuine remote git urls stored in `configs.<name>.url`.
///
/// Returns `true` for `http://`, `https://`, `ssh://`, `git@`, `flake://`, or
/// anything else containing a `://` scheme separator.
fn looks_like_remote_url(s: &str) -> bool {
    s.starts_with("http://")
        || s.starts_with("https://")
        || s.starts_with("ssh://")
        || s.starts_with("git@")
        || s.starts_with("flake://")
        || s.contains("://")
}

/// Run a `workestrate migrate-home` consolidation into a single home (ADR 0023).
///
/// `from` is `"xdg"`, `"bundle"`, or `None` (auto-detect). `dest` is the
/// destination single-home dir. In dry-run mode nothing is moved; the returned
/// [`MigrateSummary`] lists the planned moves. On a real run the registry at
/// `dest/config.toml` has `store_dir`/`state_dir` cleared and `home_version`
/// set to `Some(2)`.
pub(crate) fn run_migrate_home(
    from: Option<&str>,
    dest: &Path,
    dry_run: bool,
    force: bool,
) -> Result<MigrateSummary> {
    let layout: &str = match from {
        Some(s) => s,
        None => detect_layout(),
    };
    let sources = resolve_migrate_sources(layout, dest)?;
    let planned = plan_moves(&sources, dest);

    if dry_run {
        let moved = planned
            .iter()
            .map(|(src, dst)| MovedEntry {
                src: src.display().to_string(),
                dst: dst.display().to_string(),
            })
            .collect();
        return Ok(MigrateSummary {
            from: layout.to_string(),
            dest: dest.display().to_string(),
            dry_run: true,
            moved,
            registry_updated: false,
            home_version: None,
            urls_rewritten: Vec::new(),
        });
    }

    // Refuse to clobber an existing home unless --force.
    if dest.join("config.toml").exists() && !force {
        anyhow::bail!(
            "destination {} already contains config.toml; pass --force to overwrite",
            dest.display()
        );
    }

    std::fs::create_dir_all(dest)?;
    std::fs::create_dir_all(dest.join("secrets"))?;
    std::fs::create_dir_all(dest.join("state"))?;
    std::fs::create_dir_all(dest.join("repos"))?;
    std::fs::create_dir_all(dest.join("sources"))?;

    let mut moved: Vec<MovedEntry> = Vec::new();
    for (src, dst) in &planned {
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent)?;
        }
        move_entry(src, dst)?;
        moved.push(MovedEntry {
            src: src.display().to_string(),
            dst: dst.display().to_string(),
        });
    }

    // Update the relocated registry: drop store_dir/state_dir (derivation from
    // the new home takes over) and stamp home_version = 2.
    let mut registry_updated = false;
    let mut home_version = None;
    let mut urls_rewritten: Vec<String> = Vec::new();
    let reg_path = dest.join("config.toml");
    if reg_path.exists() {
        if let Some(mut reg) = std::fs::read_to_string(&reg_path)
            .ok()
            .and_then(|c| toml::from_str::<Registry>(&c).ok())
        {
            reg.settings.store_dir = None;
            reg.settings.state_dir = None;
            reg.settings.home_version = Some(2);

            // Rewrite `configs.<name>.url` fields that still point into the
            // OLD layout that was just migrated. Only local filesystem paths
            // are considered (remote URLs are left untouched). A url matches
            // when it equals, or lives under, the old repo base
            // `sources.repos_root/<name>`; it is then rewritten to the new
            // `dest/repos/<name>` path.
            for (name, entry) in reg.configs.iter_mut() {
                if looks_like_remote_url(&entry.url) {
                    continue;
                }
                let url_path = PathBuf::from(&entry.url);
                let old_base = sources.repos_root.join(name);
                // Prefer canonical comparison when both paths still resolve,
                // else fall back to lexical (component-wise) matching. In a
                // real run the old repo dir has already been moved, so both
                // canonicalizations fail and we use the lexical branch.
                let matches = match (url_path.canonicalize(), old_base.canonicalize()) {
                    (Ok(u), Ok(b)) => u == b || u.starts_with(&b),
                    _ => url_path == old_base || url_path.starts_with(&old_base),
                };
                if matches {
                    entry.url = dest.join("repos").join(name).to_string_lossy().to_string();
                    urls_rewritten.push(name.clone());
                }
            }
            // Deterministic (alphabetical) output ordering regardless of
            // HashMap iteration order.
            urls_rewritten.sort();

            let toml_str = toml::to_string_pretty(&reg)
                .map_err(|e| anyhow::anyhow!("failed to serialize migrated registry: {}", e))?;
            std::fs::write(&reg_path, toml_str)?;
            registry_updated = true;
            home_version = Some(2);
        }
    }

    // Best-effort cleanup of now-empty old subtrees (bundle in-place).
    for dir in &sources.cleanup_dirs {
        if dir.exists() {
            if let Err(e) = std::fs::remove_dir_all(dir) {
                eprintln!(
                    "warning: could not remove old subtree {}: {}",
                    dir.display(),
                    e
                );
            }
        }
    }

    Ok(MigrateSummary {
        from: layout.to_string(),
        dest: dest.display().to_string(),
        dry_run: false,
        moved,
        registry_updated,
        home_version,
        urls_rewritten,
    })
}

/// Load the active `workestrate.toml`.
///
/// Resolution order (lowest to highest precedence):
/// 1. Reference config (`config.reference/workestrate.toml`).
/// 2. Registry layers (`registry.layers` ordered list, each a config repo).
/// 3. User-global overrides (`$XDG_CONFIG_HOME/workestrate/overrides.toml`):
///    `[global]` is applied to every context, then `[configs.<name>]` for each
///    active context layer.
/// 4. Trusted project config (`./workestrate.toml` in cwd if trusted).
/// 5. Local overrides (`./workestrate.local.toml` in cwd).
///
/// The `$WORKESTRATE_CONFIG_DIR` environment variable bypasses discovery and
/// loads a single dev/testing layer directly.
pub fn load_config() -> Result<ConfigFile> {
    // 1. Dev/testing override: single layer, no merging.
    if let Ok(dir) = std::env::var("WORKESTRATE_CONFIG_DIR") {
        let path = PathBuf::from(dir).join("workestrate.toml");
        if path.exists() {
            set_active_context(None);
            let layer = crate::merge::Layer::load("local", &path)?;
            let (merged, provenance) = crate::merge::merge_layers(&[layer])?;
            crate::merge::set_provenance(Some(provenance));
            validate_config(&merged)?;
            return Ok(merged);
        }
    }

    let mut layers: Vec<crate::merge::Layer> = Vec::new();

    // 2. Reference config as the base layer.
    if let Some(path) = reference_config_path() {
        if path.exists() {
            layers.push(crate::merge::Layer::load("reference", &path)?);
        }
    }

    // 3. Resolve active context and load its layers.
    let active_context = resolve_active_context()?;
    set_active_context(Some(active_context.clone()));
    for name in &active_context.layers {
        let path = resolve_store_dir()
            .join("repos")
            .join(name)
            .join("workestrate.toml");
        if path.exists() {
            layers.push(crate::merge::Layer::load(name, &path)?);
        }
    }

    // 3.5. User-global overrides (between context layers and trusted project).
    {
        let existing_workloads: std::collections::HashSet<String> = layers
            .iter()
            .flat_map(|l| l.config.workloads.keys().cloned())
            .collect();
        let override_layers = load_overrides(
            &overrides_path(),
            &active_context.layers,
            &existing_workloads,
        )?;
        layers.extend(override_layers);
    }

    // 4. Trusted project layer.
    let skip_project = std::env::var("WORKESTRATE_NO_PROJECT_CONFIG").is_ok();
    if !skip_project {
        let cwd = std::env::current_dir()?;
        let project_path = cwd.join("workestrate.toml");
        if project_path.exists() {
            match load_registry()? {
                Some(_) => {
                    if is_trusted_project(&cwd) {
                        layers.push(crate::merge::Layer::load("project", &project_path)?);
                    } else {
                        eprintln!("project config ./workestrate.toml found but not trusted; run 'workestrate config trust <dir>' to trust it");
                    }
                }
                None => {
                    layers.push(crate::merge::Layer::load("project", &project_path)?);
                }
            }
        }
    }

    // 5. Local overrides — gated by the SAME trust check as the project layer.
    //    Closes review finding A1: previously `workestrate.local.toml` was
    //    loaded unconditionally from cwd, which (because local is the
    //    highest-precedence layer) meant a hostile `git clone` followed by
    //    `cd` and any workestrate invocation could compromise the host
    //    without any explicit operator action.
    let skip_local = skip_project;
    if !skip_local {
        let local_cwd = std::env::current_dir()?;
        let local_path = local_cwd.join("workestrate.local.toml");
        if local_path.exists() {
            match load_registry()? {
                Some(_) => {
                    if is_trusted_project(&local_cwd) {
                        layers.push(crate::merge::Layer::load("local", &local_path)?);
                    } else {
                        eprintln!(
                            "local config ./workestrate.local.toml found but not trusted; \
                             run 'workestrate config trust <dir>' to trust it"
                        );
                    }
                }
                None => {
                    // Bootstrap mode (no registry yet): allow local layer.
                    layers.push(crate::merge::Layer::load("local", &local_path)?);
                }
            }
        }
    }

    if layers.is_empty() {
        anyhow::bail!("no config found; run 'workestrate init' or set WORKESTRATE_CONFIG_DIR");
    }

    let (merged, provenance) = crate::merge::merge_layers(&layers)?;
    crate::merge::set_provenance(Some(provenance));
    validate_config(&merged)?;
    Ok(merged)
}

fn reference_config_path() -> Option<PathBuf> {
    if let Ok(root) = project_root() {
        let path = root.join("config.reference").join("workestrate.toml");
        if path.exists() {
            return Some(path);
        }
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut path = PathBuf::from(manifest);
        if path.pop() && path.pop() {
            let reference = path.join("config.reference").join("workestrate.toml");
            if reference.exists() {
                return Some(reference);
            }
        }
    }
    None
}

/// Resolve the directory where new config entries should be written.
///
/// Resolution order (same spirit as load_config):
/// 1. WORKESTRATE_CONFIG_DIR env var
/// 2. Trusted project ./workestrate.toml (cwd)
/// 3. Registry single layer (default config repo)
/// 4. Error: no active config repo
pub fn resolve_active_config_dir() -> Result<PathBuf> {
    // 1. WORKESTRATE_CONFIG_DIR (must exist)
    if let Ok(dir) = std::env::var("WORKESTRATE_CONFIG_DIR") {
        let path = PathBuf::from(dir);
        if path.exists() {
            return Ok(path);
        }
    }

    // 2. Trusted project (cwd)
    let skip_project = std::env::var("WORKESTRATE_NO_PROJECT_CONFIG").is_ok();
    if !skip_project {
        let cwd = std::env::current_dir()?;
        let project_path = cwd.join("workestrate.toml");
        if project_path.exists() {
            match load_registry()? {
                Some(_) => {
                    if is_trusted_project(&cwd) {
                        return Ok(cwd);
                    }
                }
                None => {
                    return Ok(cwd);
                }
            }
        }
    }

    // 3. Registry context's first layer
    if let Ok(Some(_registry)) = load_registry() {
        let active_context = resolve_active_context()?;
        if let Some(name) = active_context.layers.first() {
            return Ok(resolve_store_dir().join("repos").join(name));
        }
    }

    anyhow::bail!(
        "no active config repo; run 'workestrate init' or 'workestrate config add <url> <name>' first"
    );
}

/// Resolve all secrets layers in precedence order (lowest first).
///
/// Same resolution as `load_config()`, but returns layer directories for
/// `.env.enc` loading.
pub fn resolve_secrets_layers() -> Result<Vec<SecretsLayer>> {
    let mut layers: Vec<SecretsLayer> = Vec::new();

    // 1. WORKESTRATE_CONFIG_DIR env var: single override layer.
    if let Ok(dir) = std::env::var("WORKESTRATE_CONFIG_DIR") {
        let path = PathBuf::from(dir);
        if path.exists() {
            layers.push(SecretsLayer {
                name: "local".to_string(),
                dir: path,
                secrets_file: ".env.enc".to_string(),
                age_key_file: None,
                skip: false,
            });
            return Ok(layers);
        }
    }

    // 2. Reference config dir (shipped with the tool — no .env.enc expected,
    //    but included so the layer list mirrors load_config()).
    if let Some(path) = reference_config_path() {
        if let Some(parent) = path.parent() {
            layers.push(SecretsLayer {
                name: "reference".to_string(),
                dir: parent.to_path_buf(),
                secrets_file: ".env.enc".to_string(),
                age_key_file: None,
                skip: false,
            });
        }
    }

    // 3. Context layers in declared order, each with its own .env.enc.
    let registry = load_registry()?;
    let active_context = resolve_active_context()?;
    if let Some(registry) = registry {
        for name in &active_context.layers {
            let dir = resolve_store_dir().join("repos").join(name);
            let entry = registry.configs.get(name);
            let secrets_mode = entry.and_then(|e| e.secrets.as_deref()).unwrap_or("file");
            let secrets_file = entry
                .and_then(|e| e.secrets_file.as_deref())
                .unwrap_or(".env.enc")
                .to_string();
            let age_key_file = entry
                .and_then(|e| e.age_key_file.as_deref())
                .map(expand_tilde);
            layers.push(SecretsLayer {
                name: name.clone(),
                dir,
                secrets_file,
                age_key_file,
                skip: secrets_mode == "none",
            });
        }
    }

    // 4. User-global secrets layer (.env.local.enc in the active tool home's
    // secrets dir). Applied per-key AFTER the context's domain layers, BEFORE
    // project layers. Optional — missing file is handled gracefully by decrypt_layer().
    let (home, kind) = resolve_home_with_kind();
    let global_dir = match kind {
        HomeKind::LegacyXdg => xdg_config_dir(),
        _ => home.join("secrets"),
    };
    layers.push(SecretsLayer {
        name: "user-global".to_string(),
        dir: global_dir,
        secrets_file: ".env.local.enc".to_string(),
        age_key_file: None, // uses SOPS_AGE_KEY_FILE env or default
        skip: false,
    });

    // 5. Trusted project dir (cwd) — may have .env.enc.
    let skip_project = std::env::var("WORKESTRATE_NO_PROJECT_CONFIG").is_ok();
    if !skip_project {
        let cwd = std::env::current_dir()?;
        let project_path = cwd.join("workestrate.toml");
        if project_path.exists() {
            match load_registry()? {
                Some(_) => {
                    if is_trusted_project(&cwd) {
                        layers.push(SecretsLayer {
                            name: "project".to_string(),
                            dir: cwd,
                            secrets_file: ".env.enc".to_string(),
                            age_key_file: None,
                            skip: false,
                        });
                    }
                }
                None => {
                    layers.push(SecretsLayer {
                        name: "project".to_string(),
                        dir: cwd,
                        secrets_file: ".env.enc".to_string(),
                        age_key_file: None,
                        skip: false,
                    });
                }
            }
        }
    }

    Ok(layers)
}

pub fn validate_config(config: &ConfigFile) -> Result<()> {
    // Egress hosts in https recipes must be in ALLOWED_EGRESS_HOSTS.
    for (workload_name, workload) in &config.workloads {
        for egress in &workload.network.egress {
            if let EgressRecipeRef::Https { hosts } = egress {
                for host in hosts {
                    if !policy::ALLOWED_EGRESS_HOSTS.contains(&host.as_str()) {
                        anyhow::bail!(
                            "workload '{}' egress host '{}' is not in the core egress allowlist",
                            workload_name,
                            host
                        );
                    }
                }
            }
        }
    }

    // Secret bindings must match SECRET_HOST_BINDINGS.
    for (secret_name, secret) in &config.secrets {
        if let Some(ref env_var) = secret.env_var {
            let allowed_hosts = policy::SECRET_HOST_BINDINGS
                .iter()
                .find(|(key, _)| key == env_var)
                .map(|(_, hosts)| *hosts);

            match allowed_hosts {
                Some(allowed_hosts) => {
                    for host in secret.hosts.as_deref().unwrap_or(&[]) {
                        if !allowed_hosts.contains(&host.as_str()) {
                            anyhow::bail!(
                                "secret '{}' host '{}' is not in the core binding allowlist for '{}'",
                                secret_name,
                                host,
                                env_var
                            );
                        }
                    }
                }
                None => {
                    anyhow::bail!(
                        "secret '{}' env_var '{}' has no core secret binding allowlist entry",
                        secret_name,
                        env_var
                    );
                }
            }
        }
    }

    // Package names in nix-layered images must be in ALLOWED_PACKAGES.
    for (workload_name, workload) in &config.workloads {
        if let Some(ref contents) = workload.image.contents {
            for pkg in contents {
                if !policy::ALLOWED_PACKAGES.contains(&pkg.as_str()) {
                    anyhow::bail!(
                        "workload '{}' package '{}' is not in the core package allowlist",
                        workload_name,
                        pkg
                    );
                }
            }
        }
    }

    // Only entitled workloads may use default_deny = false.
    for (workload_name, workload) in &config.workloads {
        if workload.network.default_deny == Some(false)
            && !policy::DEFAULT_DENY_FALSE_ENTITLEMENT.contains(&workload_name.as_str())
        {
            anyhow::bail!(
                "workload '{}' is not entitled to default_deny=false",
                workload_name
            );
        }
    }

    // Secret references must be defined in the secrets section.
    for (workload_name, workload) in &config.workloads {
        for env in &workload.env {
            if let Some(ref secret_name) = env.secret {
                if !config.secrets.contains_key(secret_name) {
                    anyhow::bail!(
                        "workload '{}' env references undefined secret '{}'",
                        workload_name,
                        secret_name
                    );
                }
            }
        }
        for se in &workload.secret_env {
            if !config.secrets.contains_key(&se.secret) {
                anyhow::bail!(
                    "workload '{}' secret_env references undefined secret '{}'",
                    workload_name,
                    se.secret
                );
            }
        }
    }

    // WP1 trust-boundary validators (closes A2, C2, C3, C4). These run at
    // config-load time so hostile layers are rejected BEFORE merge / plan /
    // sandbox-start. See `80-remediation-plan.md` WP1.
    use crate::microsandbox::{
        validate_env_override, validate_mount_guest, validate_mount_host, validate_seed_source,
    };
    for (workload_name, workload) in &config.workloads {
        for m in &workload.mounts {
            validate_mount_host(&m.host).map_err(|e| {
                anyhow::anyhow!("workload '{workload_name}' mount host validation failed: {e}")
            })?;
            validate_mount_guest(&m.guest, m.read_only).map_err(|e| {
                anyhow::anyhow!("workload '{workload_name}' mount guest validation failed: {e}")
            })?;
        }
        for seed in &workload.seed_files {
            validate_seed_source(&seed.source).map_err(|e| {
                anyhow::anyhow!(
                    "workload '{workload_name}' seed_files.source validation failed: {e}"
                )
            })?;
        }
        if let Some(build) = &workload.local_build {
            if let Some(name) = &build.env_override {
                validate_env_override(name).map_err(|e| {
                    anyhow::anyhow!(
                        "workload '{workload_name}' local_build.env_override validation failed: {e}"
                    )
                })?;
            }
        }
    }

    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_in_result
    )]
    use super::*;

    /// Global lock for tests that mutate process env vars.
    ///
    /// Cargo runs unit tests in parallel by default, and tests that set
    /// `WORKESTRATE_CONFIG_DIR` or similar env vars would otherwise race.
    pub static ENV_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn project_root_with_agentctl_root_env() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let old = std::env::var("AGENTCTL_ROOT").ok();
        std::env::set_var("AGENTCTL_ROOT", "/tmp");
        let result = project_root();
        match old {
            Some(v) => std::env::set_var("AGENTCTL_ROOT", v),
            None => std::env::remove_var("AGENTCTL_ROOT"),
        }
        // /tmp doesn't have flake.nix, so this should error
        assert!(result.is_err(), "expected error when flake.nix missing");
    }

    #[test]
    fn project_root_rejects_missing_flake_nix() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let old = std::env::var("AGENTCTL_ROOT").ok();
        std::env::set_var("AGENTCTL_ROOT", "/tmp/nonexistent-ai-workbench-test");
        let result = project_root();
        match old {
            Some(v) => std::env::set_var("AGENTCTL_ROOT", v),
            None => std::env::remove_var("AGENTCTL_ROOT"),
        }
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("flake.nix"),
            "error should mention flake.nix: {err}"
        );
    }

    #[test]
    fn check_required_files_finds_missing() {
        let tmp = std::env::temp_dir();
        let entries = check_required_files(&tmp).unwrap();
        // temp dir won't have flake.nix etc
        let missing_required: Vec<_> = entries.iter().filter(|e| !e.ok && !e.optional).collect();
        assert!(
            !missing_required.is_empty(),
            "expected missing required files"
        );
    }

    #[test]
    fn check_required_files_marks_optional_correctly() {
        let tmp = std::env::temp_dir();
        let entries = check_required_files(&tmp).unwrap();
        let optional_entries: Vec<_> = entries.iter().filter(|e| e.optional).collect();
        // Optional local overrides: 4 agent repos + 4 agent builds + workspaces/ + var/ = 10.
        assert_eq!(
            optional_entries.len(),
            10,
            "expected 10 optional checks, got {}",
            optional_entries.len()
        );
    }

    #[test]
    fn resolve_active_context_no_registry_uses_empty_layers() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-no-reg-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp_home)?;
        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let ctx = resolve_active_context()?;

        // Restore
        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        assert_eq!(ctx.name, None);
        assert!(ctx.layers.is_empty());
        Ok(())
    }

    #[test]
    fn resolve_active_context_no_contexts_uses_bare_layers() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-bare-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        // Registry with bare layers, no contexts
        std::fs::write(
            config_dir.join("config.toml"),
            "layers = [\"work\", \"personal\"]\n\n[settings]\ndefault_context = \"personal\"\n",
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let ctx = resolve_active_context()?;

        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        // No contexts defined → bare layers, name is None (backward compat)
        assert_eq!(ctx.name, None);
        assert_eq!(ctx.layers, vec!["work", "personal"]);
        Ok(())
    }

    #[test]
    fn resolve_active_context_env_overrides_default() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-env-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        std::fs::write(
            config_dir.join("config.toml"),
            "[settings]\ndefault_context = \"personal\"\n\n[contexts.personal]\nlayers = [\"personal\"]\n\n[contexts.work]\nlayers = [\"team\", \"personal\"]\n",
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::set_var("WORKESTRATE_CONTEXT", "work");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let ctx = resolve_active_context()?;

        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        assert_eq!(ctx.name.as_deref(), Some("work"));
        assert_eq!(ctx.layers, vec!["team", "personal"]);
        Ok(())
    }

    #[test]
    fn resolve_active_context_default_when_no_env() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-default-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        std::fs::write(
            config_dir.join("config.toml"),
            "[settings]\ndefault_context = \"personal\"\n\n[contexts.personal]\nlayers = [\"personal\"]\n\n[contexts.work]\nlayers = [\"team\", \"personal\"]\n",
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let ctx = resolve_active_context()?;

        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        assert_eq!(ctx.name.as_deref(), Some("personal"));
        assert_eq!(ctx.layers, vec!["personal"]);
        Ok(())
    }

    #[test]
    fn resolve_active_context_unknown_env_errors() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-unknown-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        std::fs::write(
            config_dir.join("config.toml"),
            "[settings]\ndefault_context = \"personal\"\n\n[contexts.personal]\nlayers = [\"personal\"]\n",
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::set_var("WORKESTRATE_CONTEXT", "nonexistent");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let result = resolve_active_context();

        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found"),
            "error should mention 'not found': {err}"
        );
        Ok(())
    }

    #[test]
    fn resolve_active_context_contexts_but_no_default_no_env_errors() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-nodflt-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        // Contexts defined but no default_context and no env
        std::fs::write(
            config_dir.join("config.toml"),
            "[contexts.personal]\nlayers = [\"personal\"]\n\n[contexts.work]\nlayers = [\"team\"]\n",
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let result = resolve_active_context();

        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("default_context") || err.contains("--context"),
            "error should mention default_context or --context: {err}"
        );
        Ok(())
    }

    // --- User-global overrides tests ---

    fn write_overrides(dir: &Path, content: &str) -> PathBuf {
        let path = dir.join("overrides.toml");
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn load_overrides_missing_file_returns_empty() -> Result<()> {
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-ov-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        let path = tmp.join("overrides.toml");
        // File does NOT exist.
        let layers = load_overrides(
            &path,
            &["personal".to_string()],
            &std::collections::HashSet::new(),
        )?;
        assert!(
            layers.is_empty(),
            "missing overrides.toml should return empty vec"
        );
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn load_overrides_global_section_applied() -> Result<()> {
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-ov-global-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        let path = write_overrides(&tmp, "[global.workloads.pi]\ncpus = 4\n");
        let existing: std::collections::HashSet<String> = ["pi".to_string()].into_iter().collect();
        let layers = load_overrides(&path, &["personal".to_string()], &existing)?;
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].name, "global-override");
        let pi = layers[0].config.workloads.get("pi").unwrap();
        assert_eq!(pi.cpus, Some(4));
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn load_overrides_configs_section_only_when_in_context() -> Result<()> {
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-ov-match-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        let path = write_overrides(
            &tmp,
            "[configs.team.workloads.pi]\ncpus = 2\n\n[configs.other.workloads.pi]\ncpus = 8\n",
        );
        let existing: std::collections::HashSet<String> = ["pi".to_string()].into_iter().collect();
        // Only "team" is in context_layers; "other" should be skipped.
        let layers = load_overrides(&path, &["team".to_string()], &existing)?;
        assert_eq!(
            layers.len(),
            1,
            "only the matching config section should produce a layer"
        );
        assert_eq!(layers[0].name, "configs.team-override");
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn load_overrides_unknown_workload_skipped() -> Result<()> {
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-ov-unknown-wl-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        let path = write_overrides(
            &tmp,
            "[global.workloads.pi]\ncpus = 4\n\n[global.workloads.nonexistent]\ncpus = 8\n",
        );
        // Only "pi" exists; "nonexistent" should be skipped.
        let existing: std::collections::HashSet<String> = ["pi".to_string()].into_iter().collect();
        let layers = load_overrides(&path, &[], &existing)?;
        assert_eq!(layers.len(), 1);
        let pi = layers[0].config.workloads.get("pi").unwrap();
        assert_eq!(pi.cpus, Some(4));
        // "nonexistent" should NOT be in the layer.
        assert!(!layers[0].config.workloads.contains_key("nonexistent"));
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn load_overrides_most_specific_wins() -> Result<()> {
        // [global.workloads.pi] cpus=4 is overridden by [configs.team.workloads.pi] cpus=2
        // because configs.team-override comes AFTER global-override in the layer list.
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-ov-specific-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        let path = write_overrides(
            &tmp,
            "[global.workloads.pi]\ncpus = 4\n\n[configs.team.workloads.pi]\ncpus = 2\n",
        );
        let existing: std::collections::HashSet<String> = ["pi".to_string()].into_iter().collect();
        let layers = load_overrides(&path, &["team".to_string()], &existing)?;
        assert_eq!(layers.len(), 2);
        assert_eq!(layers[0].name, "global-override");
        assert_eq!(layers[1].name, "configs.team-override");

        // Merge: base (no cpus) + global (cpus=4) + configs.team (cpus=2) → cpus=2
        let base = crate::merge::Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.network]\ndefault_deny = true",
        )?;
        let mut all = vec![base];
        all.extend(layers);
        let (merged, _) = crate::merge::merge_layers(&all)?;
        let pi = merged.workloads.get("pi").unwrap();
        assert_eq!(
            pi.cpus,
            Some(2),
            "configs.team override should win over global"
        );
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn load_overrides_policy_violation_default_deny_false_hard_fails() -> Result<()> {
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-ov-policy-dd-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        // Override sets default_deny=false on pi (not entitled).
        let path = write_overrides(
            &tmp,
            "[global.workloads.pi.network]\ndefault_deny = false\n",
        );
        let existing: std::collections::HashSet<String> = ["pi".to_string()].into_iter().collect();
        let layers = load_overrides(&path, &[], &existing)?;
        assert_eq!(layers.len(), 1);

        // Base layer has pi with default_deny=true.
        let base = crate::merge::Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.network]\ndefault_deny = true",
        )?;
        let mut all = vec![base];
        all.extend(layers);
        let result = crate::merge::merge_layers(&all);
        assert!(
            result.is_err(),
            "override setting default_deny=false on non-entitled workload should hard-fail"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("monotonic-true") || err.contains("entitlement"),
            "error should mention monotonic-true or entitlement: {err}"
        );
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn load_overrides_policy_violation_egress_host_hard_fails() -> Result<()> {
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-ov-policy-egress-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        // Override adds a non-allowlisted egress host.
        let path = write_overrides(
            &tmp,
            "[[global.workloads.pi.network.egress]]\nrecipe = \"https\"\nhosts = [\"evil.com\"]\n",
        );
        let existing: std::collections::HashSet<String> = ["pi".to_string()].into_iter().collect();
        let layers = load_overrides(&path, &[], &existing)?;
        assert_eq!(layers.len(), 1);

        let base = crate::merge::Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.network]\ndefault_deny = true",
        )?;
        let mut all = vec![base];
        all.extend(layers);
        let result = crate::merge::merge_layers(&all);
        assert!(
            result.is_err(),
            "override with non-allowlisted egress host should hard-fail"
        );
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("allowlist"),
            "error should mention allowlist: {err}"
        );
        assert!(
            err.contains("evil.com"),
            "error should mention the host: {err}"
        );
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn load_overrides_global_applied_in_two_contexts() -> Result<()> {
        // [global] should produce a layer regardless of which context is active.
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-ov-two-ctx-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        let path = write_overrides(&tmp, "[global.workloads.pi]\ncpus = 4\n");
        let existing: std::collections::HashSet<String> = ["pi".to_string()].into_iter().collect();

        // Context 1: personal
        let layers1 = load_overrides(&path, &["personal".to_string()], &existing)?;
        assert_eq!(layers1.len(), 1);
        assert_eq!(layers1[0].name, "global-override");

        // Context 2: work
        let layers2 = load_overrides(&path, &["team".to_string()], &existing)?;
        assert_eq!(layers2.len(), 1);
        assert_eq!(layers2[0].name, "global-override");

        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn resolve_secrets_layers_includes_user_global_after_context() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-secrets-global-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        let data_dir = tmp_home.join(".local").join("share").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        std::fs::create_dir_all(&data_dir)?;

        // Registry with a context and a config repo
        std::fs::write(
            config_dir.join("config.toml"),
            "[settings]\ndefault_context = \"personal\"\n\n[contexts.personal]\nlayers = [\"personal\"]\n\n[configs.personal]\nurl = \"git@example.com:personal.git\"\nref = \"main\"\n",
        )?;

        // Config repo dir with a workestrate.toml (so the layer is loaded)
        let repo_dir = data_dir.join("repos").join("personal");
        std::fs::create_dir_all(&repo_dir)?;
        std::fs::write(repo_dir.join("workestrate.toml"), "schema_version = 1\n")?;

        // Create a dummy .env.local.enc in the XDG config dir
        std::fs::write(config_dir.join(".env.local.enc"), "# dummy")?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg_config = std::env::var("XDG_CONFIG_HOME").ok();
        let old_xdg_data = std::env::var("XDG_DATA_HOME").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var("XDG_CONFIG_HOME", tmp_home.join(".config"));
        std::env::set_var("XDG_DATA_HOME", tmp_home.join(".local").join("share"));
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");
        std::env::remove_var("WORKESTRATE_CONTEXT");

        let layers = resolve_secrets_layers()?;

        // Restore env
        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg_config {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_xdg_data {
            Some(v) => std::env::set_var("XDG_DATA_HOME", v),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        // Expected layers: reference, personal (context layer), user-global
        // (trusted project is not present because cwd has no workestrate.toml)
        let names: Vec<&str> = layers.iter().map(|l| l.name.as_str()).collect();
        assert!(
            names.contains(&"personal"),
            "context layer 'personal' should be present: {:?}",
            names
        );
        assert!(
            names.contains(&"user-global"),
            "user-global layer should be present: {:?}",
            names
        );

        // user-global should come AFTER personal (context layer)
        let personal_idx = names.iter().position(|n| *n == "personal").unwrap();
        let global_idx = names.iter().position(|n| *n == "user-global").unwrap();
        assert!(
            global_idx > personal_idx,
            "user-global should come after context layers: {:?}",
            names
        );

        // user-global should use .env.local.enc
        let global_layer = layers.iter().find(|l| l.name == "user-global").unwrap();
        assert_eq!(global_layer.secrets_file, ".env.local.enc");
        assert!(!global_layer.skip, "user-global should not be skipped");

        Ok(())
    }

    #[test]
    fn resolve_secrets_layers_user_global_silent_when_missing() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-secrets-global-missing-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        let data_dir = tmp_home.join(".local").join("share").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        std::fs::create_dir_all(&data_dir)?;

        // Registry with a context
        std::fs::write(
            config_dir.join("config.toml"),
            "[settings]\ndefault_context = \"personal\"\n\n[contexts.personal]\nlayers = [\"personal\"]\n\n[configs.personal]\nurl = \"git@example.com:personal.git\"\nref = \"main\"\n",
        )?;

        let repo_dir = data_dir.join("repos").join("personal");
        std::fs::create_dir_all(&repo_dir)?;
        std::fs::write(repo_dir.join("workestrate.toml"), "schema_version = 1\n")?;

        // NO .env.local.enc — should still include the layer (decrypt handles missing file)
        let old_home = std::env::var("HOME").ok();
        let old_xdg_config = std::env::var("XDG_CONFIG_HOME").ok();
        let old_xdg_data = std::env::var("XDG_DATA_HOME").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var("XDG_CONFIG_HOME", tmp_home.join(".config"));
        std::env::set_var("XDG_DATA_HOME", tmp_home.join(".local").join("share"));
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");
        std::env::remove_var("WORKESTRATE_CONTEXT");

        let layers = resolve_secrets_layers()?;

        // Restore env
        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg_config {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_xdg_data {
            Some(v) => std::env::set_var("XDG_DATA_HOME", v),
            None => std::env::remove_var("XDG_DATA_HOME"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        // user-global layer should still be present (file is optional, layer is always added)
        let names: Vec<&str> = layers.iter().map(|l| l.name.as_str()).collect();
        assert!(
            names.contains(&"user-global"),
            "user-global layer should be present even when .env.local.enc is missing: {:?}",
            names
        );

        Ok(())
    }

    // ---- A1 regression: workestrate.local.toml requires trust gate ----

    /// Helper: write a minimal registry into XDG_CONFIG_HOME that marks the
    /// given project as trusted (or NOT trusted, if no paths are passed).
    fn write_test_registry(home: &Path, trusted_paths: &[&Path]) -> std::io::Result<()> {
        let cfg_dir = home.join(".config").join("workestrate");
        std::fs::create_dir_all(&cfg_dir)?;
        let mut s = String::from("layers = []\n\n");
        for p in trusted_paths {
            s.push_str(&format!(
                "[[trusted_projects]]\npath = \"{}\"\n",
                p.display()
            ));
        }
        std::fs::write(cfg_dir.join("config.toml"), s)?;
        Ok(())
    }

    /// Build a minimal 1-workload ConfigFile TOML string. The workload name
    /// is the discriminator for the trust-gate test.
    fn one_workload_toml(name: &str) -> String {
        format!(
            "schema_version = 1\n\n\
             [workloads.{name}]\n\
             kind = \"agent\"\n\
             image = {{ recipe = \"registry\", ref = \"node:24\" }}\n\
             command = []\n\n\
             [workloads.{name}.network]\n\
             default_deny = true\n"
        )
    }

    /// A1 regression: with a registry present but cwd NOT trusted, a
    /// workestrate.local.toml in cwd must NOT be loaded. After trusting
    /// the project dir, the local layer MUST be loaded.
    #[test]
    fn local_toml_requires_trust_gate() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();

        let root = std::env::temp_dir().join(format!(
            "workestrate-a1-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(root.join("cwd"))?;
        std::fs::create_dir_all(root.join("home").join(".config").join("workestrate"))?;

        std::fs::write(
            root.join("cwd").join("workestrate.local.toml"),
            one_workload_toml("a1_hostile_local_marker"),
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        let old_no_project = std::env::var("WORKESTRATE_NO_PROJECT_CONFIG").ok();
        let old_cwd = std::env::current_dir().ok();

        std::env::set_var("HOME", root.join("home"));
        std::env::set_var(
            "XDG_CONFIG_HOME",
            root.join("home").join(".config").to_string_lossy().as_ref(),
        );
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");
        std::env::remove_var("WORKESTRATE_NO_PROJECT_CONFIG");
        std::env::set_current_dir(root.join("cwd"))?;

        // Case 1: registry exists, cwd NOT trusted -> local layer gated out.
        write_test_registry(&root.join("home"), &[])?;
        let cfg = load_config()?;
        assert!(
            !cfg.workloads.contains_key("a1_hostile_local_marker"),
            "A1 regression: local.toml loaded without trust! workloads: {:?}",
            cfg.workloads.keys().collect::<Vec<_>>()
        );

        // Case 2: trust cwd -> local layer loads.
        let cwd_canonical = std::fs::canonicalize(root.join("cwd"))?;
        trust_project(&cwd_canonical)?;
        let cfg2 = load_config()?;
        assert!(
            cfg2.workloads.contains_key("a1_hostile_local_marker"),
            "A1 regression: local.toml NOT loaded after trust! workloads: {:?}",
            cfg2.workloads.keys().collect::<Vec<_>>()
        );

        // Restore env
        if let Some(c) = old_cwd {
            std::env::set_current_dir(c)?;
        }
        for (k, v) in [
            ("HOME", old_home),
            ("XDG_CONFIG_HOME", old_xdg),
            ("WORKESTRATE_CONTEXT", old_ctx),
            ("WORKESTRATE_CONFIG_DIR", old_config_dir),
            ("WORKESTRATE_NO_PROJECT_CONFIG", old_no_project),
        ] {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    /// A1 corollary: in bootstrap mode (no registry yet), local.toml IS
    /// loaded — this matches the project-layer bootstrap behavior.
    #[test]
    fn local_toml_loaded_in_bootstrap_mode() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();

        let root = std::env::temp_dir().join(format!(
            "workestrate-a1boot-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(root.join("cwd"))?;
        std::fs::create_dir_all(root.join("home"))?;

        std::fs::write(
            root.join("cwd").join("workestrate.local.toml"),
            one_workload_toml("a1_bootstrap_marker"),
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        let old_no_project = std::env::var("WORKESTRATE_NO_PROJECT_CONFIG").ok();
        let old_cwd = std::env::current_dir().ok();

        std::env::set_var("HOME", root.join("home"));
        std::env::set_var(
            "XDG_CONFIG_HOME",
            root.join("home").join(".config").to_string_lossy().as_ref(),
        );
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");
        std::env::remove_var("WORKESTRATE_NO_PROJECT_CONFIG");
        std::env::set_current_dir(root.join("cwd"))?;

        // NO registry file present -> load_registry returns None -> bootstrap.
        let cfg = load_config()?;
        assert!(
            cfg.workloads.contains_key("a1_bootstrap_marker"),
            "A1 bootstrap: local.toml should load when no registry exists; got {:?}",
            cfg.workloads.keys().collect::<Vec<_>>()
        );

        if let Some(c) = old_cwd {
            std::env::set_current_dir(c)?;
        }
        for (k, v) in [
            ("HOME", old_home),
            ("XDG_CONFIG_HOME", old_xdg),
            ("WORKESTRATE_CONTEXT", old_ctx),
            ("WORKESTRATE_CONFIG_DIR", old_config_dir),
            ("WORKESTRATE_NO_PROJECT_CONFIG", old_no_project),
        ] {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    // ---- A18 regression: trust paths are canonicalized ----

    /// A18 regression: trust a project via a path containing a `.` or via a
    /// symlink, then query trust via a different-but-equivalent path — both
    /// must report trusted.
    #[test]
    fn trust_paths_are_canonicalized() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();

        let root = std::env::temp_dir().join(format!(
            "workestrate-a18-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let real = root.join("real");
        let linked = root.join("link");
        std::fs::create_dir_all(&real)?;
        std::os::unix::fs::symlink(&real, &linked).or_else(|_| std::fs::create_dir_all(&linked))?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("HOME", &root);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            root.join(".config").to_string_lossy().as_ref(),
        );
        std::fs::create_dir_all(root.join(".config").join("workestrate"))?;

        std::fs::write(
            root.join(".config").join("workestrate").join("config.toml"),
            "layers = []\n",
        )?;

        // Trust via the symlink path; query via the canonical path.
        trust_project(&linked)?;
        let canonical_real = std::fs::canonicalize(&real)?;
        assert!(
            is_trusted_project(&canonical_real),
            "A18: trust via symlink '{}' did not match canonical '{}'",
            linked.display(),
            canonical_real.display()
        );

        // Query via a `..`-containing equivalent path through a parent reference.
        let sibling = root.join("sibling");
        std::fs::create_dir_all(&sibling)?;
        let dotted = sibling.join("..").join("real");
        assert!(
            is_trusted_project(&dotted),
            "A18: trust query via dotted path '{}' failed",
            dotted.display()
        );

        // untrust via canonical, then verify gone via symlink.
        untrust_project(&canonical_real)?;
        assert!(
            !is_trusted_project(&linked),
            "A18: untrust via canonical did not remove symlink-form trust"
        );

        for (k, v) in [("HOME", old_home), ("XDG_CONFIG_HOME", old_xdg)] {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    // ---- ADR 0023: resolve_home + migrate-home coverage ----

    /// Snapshot env vars (+ cwd) and restore them on drop, even on panic.
    /// Mirrors the manual save/restore in older tests but panic-safe.
    struct EnvGuard {
        vars: Vec<(&'static str, Option<String>)>,
        cwd: Option<PathBuf>,
    }
    impl EnvGuard {
        fn capture(keys: &'static [&'static str]) -> Self {
            let vars = keys.iter().map(|&k| (k, std::env::var(k).ok())).collect();
            EnvGuard {
                vars,
                cwd: std::env::current_dir().ok(),
            }
        }
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            for (k, v) in &self.vars {
                match v {
                    Some(val) => std::env::set_var(k, val),
                    None => std::env::remove_var(k),
                }
            }
            if let Some(cwd) = &self.cwd {
                let _ = std::env::set_current_dir(cwd);
            }
        }
    }

    const HOME_ENV_KEYS: &[&str] = &[
        "WORKESTRATE_HOME",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_STATE_HOME",
        "WORKESTRATE_CONFIG_DIR",
        "WORKESTRATE_NO_PROJECT_CONFIG",
        "HOME",
    ];

    fn uniq_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "workestrate-{}-{}-{}",
            label,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ))
    }

    #[test]
    fn resolve_home_env_wins() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let env_home = uniq_dir("rh-env");
        let xdg = uniq_dir("rh-env-xdg");
        std::fs::create_dir_all(&env_home)?;
        std::env::set_var("HOME", &env_home);
        std::env::set_var("XDG_CONFIG_HOME", &xdg);
        std::env::set_var("WORKESTRATE_HOME", &env_home);

        let (home, kind) = resolve_home_with_kind();
        assert_eq!(kind, HomeKind::Env, "WORKESTRATE_HOME must win over XDG");
        assert_eq!(home, env_home);

        let _ = std::fs::remove_dir_all(&env_home);
        let _ = std::fs::remove_dir_all(&xdg);
        Ok(())
    }

    #[test]
    fn resolve_home_discovery_trusted() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let base_home = uniq_dir("rh-disc-base");
        let project = uniq_dir("rh-disc-proj");
        std::fs::create_dir_all(base_home.join(".workestrate"))?;
        std::fs::create_dir_all(project.join(".workestrate"))?;

        // Seed a project-local config.toml so discovery notices it.
        std::fs::write(
            project.join(".workestrate").join("config.toml"),
            "layers = []\n",
        )?;

        // Trust list lives in the Default base registry (<HOME>/.workestrate).
        let canonical_project = std::fs::canonicalize(&project)?;
        let trust_toml = format!(
            "[[trusted_projects]]\npath = \"{}\"\n",
            canonical_project.display()
        );
        std::fs::write(
            base_home.join(".workestrate").join("config.toml"),
            trust_toml,
        )?;

        std::env::set_var("HOME", &base_home);
        std::env::set_current_dir(&project)?;

        let (home, kind) = resolve_home_with_kind();
        assert_eq!(kind, HomeKind::Discovered);
        assert_eq!(home, project.join(".workestrate"));

        let _ = std::fs::remove_dir_all(&base_home);
        let _ = std::fs::remove_dir_all(&project);
        Ok(())
    }

    /// Precedence regression: an explicit XDG var must win over discovery even
    /// when a *trusted* `.workestrate/config.toml` exists in the cwd. Without
    /// this guarantee, discovery overrides a deliberately-pinned legacy layout
    /// whenever a trusted project config is present in an ancestor.
    #[test]
    fn discovery_does_not_override_explicit_xdg() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let base_home = uniq_dir("rh-xdg-base");
        let project = uniq_dir("rh-xdg-proj");
        let xdg = uniq_dir("rh-xdg-explicit");
        std::fs::create_dir_all(base_home.join(".workestrate"))?;
        std::fs::create_dir_all(project.join(".workestrate"))?;
        std::fs::create_dir_all(&xdg)?;

        // Project-local config.toml so discovery WOULD notice it if it ran.
        std::fs::write(
            project.join(".workestrate").join("config.toml"),
            "layers = []\n",
        )?;

        // Base registry at <HOME>/.workestrate trusts the project dir, so
        // discovery would return Discovered if it were allowed to run.
        let canonical_project = std::fs::canonicalize(&project)?;
        let trust_toml = format!(
            "[[trusted_projects]]\npath = \"{}\"\n",
            canonical_project.display()
        );
        std::fs::write(
            base_home.join(".workestrate").join("config.toml"),
            trust_toml,
        )?;

        std::env::set_var("HOME", &base_home);
        std::env::set_var("XDG_CONFIG_HOME", &xdg);
        std::env::remove_var("WORKESTRATE_HOME");
        std::env::set_current_dir(&project)?;

        let (_home, kind) = resolve_home_with_kind();
        assert_ne!(
            kind,
            HomeKind::Discovered,
            "explicit XDG var must override discovery even for a trusted project"
        );
        assert_eq!(kind, HomeKind::LegacyXdg);

        let _ = std::fs::remove_dir_all(&base_home);
        let _ = std::fs::remove_dir_all(&project);
        let _ = std::fs::remove_dir_all(&xdg);
        Ok(())
    }

    #[test]
    fn resolve_home_discovery_untrusted_ignored() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let base_home = uniq_dir("rh-untr-base");
        let project = uniq_dir("rh-untr-proj");
        std::fs::create_dir_all(base_home.join(".workestrate"))?;
        std::fs::create_dir_all(project.join(".workestrate"))?;
        std::fs::write(
            project.join(".workestrate").join("config.toml"),
            "layers = []\n",
        )?;
        // Base registry exists but does NOT trust the project.
        std::fs::write(
            base_home.join(".workestrate").join("config.toml"),
            "[[trusted_projects]]\npath = \"/some/other/dir\"\n",
        )?;

        std::env::set_var("HOME", &base_home);
        std::env::set_current_dir(&project)?;

        let (home, kind) = resolve_home_with_kind();
        assert_ne!(
            kind,
            HomeKind::Discovered,
            "untrusted .workestrate must be ignored"
        );
        // Fell through to Default (<HOME>/.workestrate).
        assert_eq!(kind, HomeKind::Default);
        assert_eq!(home, base_home.join(".workestrate"));

        let _ = std::fs::remove_dir_all(&base_home);
        let _ = std::fs::remove_dir_all(&project);
        Ok(())
    }

    #[test]
    fn resolve_home_legacy_xdg_when_xdg_set() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let xdg = uniq_dir("rh-xdg");
        let home = uniq_dir("rh-xdg-home");
        std::fs::create_dir_all(&xdg)?;
        std::env::set_var("HOME", &home);
        std::env::set_var("XDG_CONFIG_HOME", &xdg);

        let (resolved, kind) = resolve_home_with_kind();
        assert_eq!(kind, HomeKind::LegacyXdg);
        assert_eq!(resolved, xdg.join("workestrate"));

        let _ = std::fs::remove_dir_all(&xdg);
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn resolve_home_default_when_nothing_set() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let home = uniq_dir("rh-default-home");
        std::fs::create_dir_all(&home)?;
        std::env::set_var("HOME", &home);

        let (resolved, kind) = resolve_home_with_kind();
        assert_eq!(kind, HomeKind::Default);
        assert_eq!(resolved, home.join(".workestrate"));

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Build a full legacy XDG layout under `root` and return the computed
    /// xdg config/data/state dirs. Pins XDG_*_HOME env vars.
    fn build_xdg_layout(root: &Path) -> Result<(PathBuf, PathBuf, PathBuf)> {
        std::env::set_var("XDG_CONFIG_HOME", root.join("xdg-config"));
        std::env::set_var("XDG_DATA_HOME", root.join("xdg-data"));
        std::env::set_var("XDG_STATE_HOME", root.join("xdg-state"));
        std::env::set_var("HOME", root.join("home"));

        let xcfg = xdg_config_dir();
        let xdata = xdg_data_dir();
        let xstate = xdg_state_dir();
        std::fs::create_dir_all(&xcfg)?;
        std::fs::write(
            xcfg.join("config.toml"),
            "[settings]\nstore_dir = \"/old/store\"\n",
        )?;
        std::fs::write(xcfg.join("overrides.toml"), "[global]\n")?;
        std::fs::write(xcfg.join(".env.local.enc"), "ENCRYPTED-BYTES")?;

        std::fs::create_dir_all(xdata.join("repos").join("personal"))?;
        std::fs::write(
            xdata.join("repos").join("personal").join("file.txt"),
            "repo-data",
        )?;
        std::fs::create_dir_all(xdata.join("sources").join("foo"))?;
        std::fs::write(xdata.join("sources").join("foo").join("f.txt"), "src-data")?;

        std::fs::create_dir_all(xstate.join("workspaces"))?;
        std::fs::write(xstate.join("workspaces").join("ws.txt"), "ws-data")?;
        Ok((xcfg, xdata, xstate))
    }

    #[test]
    fn migrate_home_dry_run_xdg() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let root = uniq_dir("mig-dry");
        std::fs::create_dir_all(&root)?;
        let (xcfg, xdata, xstate) = build_xdg_layout(&root)?;
        let dest = root.join("dest");

        let summary = run_migrate_home(Some("xdg"), &dest, true, false)?;
        assert!(summary.dry_run);
        assert!(!summary.registry_updated);
        assert!(summary.moved.iter().any(|m| m.dst.ends_with("config.toml")));
        assert!(summary
            .moved
            .iter()
            .any(|m| m.dst.ends_with("overrides.toml")));
        assert!(summary
            .moved
            .iter()
            .any(|m| m.dst.ends_with(".env.local.enc")));
        assert!(summary
            .moved
            .iter()
            .any(|m| m.dst.ends_with("repos/personal")));
        assert!(summary.moved.iter().any(|m| m.dst.ends_with("sources/foo")));
        assert!(summary
            .moved
            .iter()
            .any(|m| m.dst.ends_with("state/workspaces")));

        // Dry-run must touch nothing.
        assert!(xcfg.join("config.toml").exists());
        assert!(xdata
            .join("repos")
            .join("personal")
            .join("file.txt")
            .exists());
        assert!(xstate.join("workspaces").join("ws.txt").exists());
        assert!(!dest.exists());

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn migrate_home_real_move_xdg() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let root = uniq_dir("mig-real");
        std::fs::create_dir_all(&root)?;
        let (xcfg, xdata, _xstate) = build_xdg_layout(&root)?;
        let dest = root.join("dest");

        let summary = run_migrate_home(Some("xdg"), &dest, false, false)?;
        assert!(!summary.dry_run);
        assert!(summary.registry_updated);
        assert_eq!(summary.home_version, Some(2));

        // New single-home layout.
        assert!(dest.join("config.toml").exists());
        assert!(dest.join("overrides.toml").exists());
        assert!(dest.join("secrets").join(".env.local.enc").exists());
        assert!(dest
            .join("repos")
            .join("personal")
            .join("file.txt")
            .exists());
        assert!(dest.join("sources").join("foo").join("f.txt").exists());
        assert!(dest
            .join("state")
            .join("workspaces")
            .join("ws.txt")
            .exists());

        // Registry consolidated: store_dir cleared, home_version stamped.
        let reg: Registry = toml::from_str(&std::fs::read_to_string(dest.join("config.toml"))?)?;
        assert_eq!(reg.settings.store_dir, None);
        assert_eq!(reg.settings.state_dir, None);
        assert_eq!(reg.settings.home_version, Some(2));

        // Old sources moved away.
        assert!(!xcfg.join("config.toml").exists());
        assert!(!xcfg.join(".env.local.enc").exists());
        assert!(!xdata
            .join("repos")
            .join("personal")
            .join("file.txt")
            .exists());

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn migrate_home_bundle_inplace() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let root = uniq_dir("mig-bundle");
        let dest = root.join(".workestrate");
        // Old bundle triplication under dest/{config,data,state}/workestrate.
        let bcfg = dest.join("config").join("workestrate");
        let bdata = dest.join("data").join("workestrate");
        let bstate = dest.join("state").join("workestrate");
        std::fs::create_dir_all(&bcfg)?;
        std::fs::write(
            bcfg.join("config.toml"),
            "[settings]\nstore_dir = \"/old\"\n",
        )?;
        std::fs::write(bcfg.join("overrides.toml"), "[global]\n")?;
        std::fs::write(bcfg.join(".env.local.enc"), "ENC")?;
        std::fs::create_dir_all(bdata.join("repos").join("personal"))?;
        std::fs::write(
            bdata.join("repos").join("personal").join("file.txt"),
            "repo",
        )?;
        std::fs::create_dir_all(bdata.join("sources").join("foo"))?;
        std::fs::write(bdata.join("sources").join("foo").join("f.txt"), "src")?;
        std::fs::create_dir_all(bstate.join("workspaces"))?;
        std::fs::write(bstate.join("workspaces").join("ws.txt"), "ws")?;

        let summary = run_migrate_home(Some("bundle"), &dest, false, false)?;
        assert!(summary.registry_updated);
        assert_eq!(summary.from, "bundle");

        // New single-home layout under dest.
        assert!(dest.join("config.toml").exists());
        assert!(dest.join("secrets").join(".env.local.enc").exists());
        assert!(dest
            .join("repos")
            .join("personal")
            .join("file.txt")
            .exists());
        assert!(dest.join("sources").join("foo").join("f.txt").exists());
        // State: old state/workestrate moved INTO dest/state (overlap case).
        assert!(dest
            .join("state")
            .join("workspaces")
            .join("ws.txt")
            .exists());

        // Old subtrees removed; dest/state (new) survives.
        assert!(!dest.join("config").exists());
        assert!(!dest.join("data").exists());
        assert!(!dest.join("state").join("workestrate").exists());
        assert!(dest.join("state").exists());

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn migrate_home_rewrites_repo_url_pointing_into_old_layout() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let root = uniq_dir("mig-url");
        let dest = root.join(".workestrate");
        // Old bundle triplication under dest/{config,data}/workestrate.
        let bcfg = dest.join("config").join("workestrate");
        let bdata = dest.join("data").join("workestrate");
        std::fs::create_dir_all(&bcfg)?;

        // Old repo base for <personal> under the bundle layout:
        // dest/data/workestrate/repos/personal
        let old_repo = bdata.join("repos").join("personal");
        std::fs::create_dir_all(&old_repo)?;
        std::fs::write(old_repo.join("file.txt"), "repo")?;
        // Simulate a git checkout so it looks like a real repo dir.
        std::fs::create_dir_all(old_repo.join(".git"))?;
        std::fs::write(old_repo.join(".git").join("HEAD"), "ref: refs/heads/main\n")?;

        // Registry entry whose url points into the OLD layout.
        let old_url = old_repo.to_string_lossy().to_string();
        let toml_reg = format!(
            "[settings]\nstore_dir = \"/old\"\n\
             [configs.personal]\nurl = \"{}\"\nref = \"main\"\n",
            old_url
        );
        std::fs::write(bcfg.join("config.toml"), toml_reg)?;

        let summary = run_migrate_home(Some("bundle"), &dest, false, false)?;
        assert!(summary.registry_updated);
        assert_eq!(summary.from, "bundle");

        // url rewritten to the new repo path.
        assert!(
            summary.urls_rewritten.contains(&"personal".to_string()),
            "expected personal in urls_rewritten, got {:?}",
            summary.urls_rewritten
        );

        // Reload the relocated registry and confirm the url was rewritten.
        let new_reg_text = std::fs::read_to_string(dest.join("config.toml"))?;
        let new_reg: Registry = toml::from_str(&new_reg_text)?;
        let expected_new = dest.join("repos").join("personal");
        let entry = new_reg
            .configs
            .get("personal")
            .ok_or_else(|| anyhow::anyhow!("personal config missing after migrate"))?;
        assert_eq!(
            entry.url,
            expected_new.to_string_lossy(),
            "url should have been rewritten to the new repo path"
        );
        // store_dir cleared as before.
        assert_eq!(new_reg.settings.store_dir, None);
        assert_eq!(new_reg.settings.home_version, Some(2));

        // The repo itself landed at the new path.
        assert!(expected_new.join("file.txt").exists());

        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    #[test]
    fn home_version_defaults_to_absent() -> Result<()> {
        let toml_no_version = "[settings]\ndefault_context = \"personal\"\n";
        let reg: Registry = toml::from_str(toml_no_version)?;
        assert_eq!(reg.settings.home_version, None);
        // Round-trip preserves absence.
        let round = toml::to_string(&reg)?;
        let reg2: Registry = toml::from_str(&round)?;
        assert_eq!(reg2.settings.home_version, None);
        Ok(())
    }

    #[test]
    fn legacy_xdg_registry_path_compat() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let xdg = uniq_dir("rh-compat");
        std::fs::create_dir_all(&xdg)?;
        std::env::set_var("XDG_CONFIG_HOME", &xdg);

        // Backward-compat guarantee: existing XDG-pinned callers see the
        // same registry path as before ADR 0023.
        assert_eq!(registry_path(), xdg.join("workestrate").join("config.toml"));

        let _ = std::fs::remove_dir_all(&xdg);
        Ok(())
    }
}
