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

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[allow(dead_code)]
pub struct BinarySpec {
    pub recipe: String,
    pub src: String,
    pub entrypoint: Option<String>,
    pub worker: Option<String>,
    pub npm_deps_hash: Option<String>,
    pub install_layout: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[allow(dead_code)]
pub struct BakedFileSpec {
    pub path: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[allow(dead_code)]
pub struct EnvVarConfig {
    pub name: String,
    pub value: Option<String>,
    pub secret: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[allow(dead_code)]
pub struct SecretEnvConfig {
    pub secret: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
#[allow(dead_code)]
pub struct SeedFileConfig {
    pub source: String,
    pub target: String,
    pub only_if_missing: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
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

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq)]
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

/// Registry path: ~/.config/workestrate/config.toml
pub fn registry_path() -> PathBuf {
    xdg_config_dir().join("config.toml")
}

/// Overrides path: $XDG_CONFIG_HOME/workestrate/overrides.toml
pub fn overrides_path() -> PathBuf {
    xdg_config_dir().join("overrides.toml")
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegistrySettings {
    pub default_context: Option<String>,
    pub store_dir: Option<String>,
    pub state_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedProject {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Context {
    #[serde(default)]
    pub layers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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
    xdg_state_dir()
}

pub fn resolve_store_dir() -> PathBuf {
    if let Ok(Some(registry)) = load_registry() {
        if let Some(ref store_dir) = registry.settings.store_dir {
            return expand_tilde(store_dir);
        }
    }
    xdg_data_dir()
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

pub fn is_trusted_project(dir: &Path) -> bool {
    if let Ok(Some(registry)) = load_registry() {
        registry.trusted_projects.iter().any(|p| {
            let expanded = expand_tilde(&p.path);
            expanded == dir || Path::new(&p.path) == dir
        })
    } else {
        false
    }
}

pub fn trust_project(dir: &Path) -> Result<()> {
    let mut registry = load_registry()?.unwrap_or_default();
    let dir_str = dir.to_string_lossy().to_string();
    if !registry.trusted_projects.iter().any(|p| p.path == dir_str) {
        registry
            .trusted_projects
            .push(TrustedProject { path: dir_str });
        save_registry(&registry)?;
    }
    Ok(())
}

pub fn untrust_project(dir: &Path) -> Result<()> {
    let mut registry = load_registry()?.unwrap_or_default();
    let dir_str = dir.to_string_lossy().to_string();
    registry.trusted_projects.retain(|p| p.path != dir_str);
    save_registry(&registry)?;
    Ok(())
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

    // 5. Local overrides.
    let local_path = std::env::current_dir()?.join("workestrate.local.toml");
    if local_path.exists() {
        layers.push(crate::merge::Layer::load("local", &local_path)?);
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

    // 4. User-global secrets layer (.env.local.enc in XDG config dir).
    // Applied per-key AFTER the context's domain layers, BEFORE project layers.
    // Optional — missing file is handled gracefully by decrypt_layer().
    layers.push(SecretsLayer {
        name: "user-global".to_string(),
        dir: xdg_config_dir(),
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

    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    #![allow(clippy::unwrap_used)]
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
}
