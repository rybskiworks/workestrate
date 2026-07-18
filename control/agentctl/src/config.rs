use anyhow::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::policy;
use crate::recipes::EgressRecipeRef;

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
    pub kind: String,
    pub image: ImageSpec,
    pub workdir: Option<String>,
    pub cpus: Option<u8>,
    pub memory_mib: Option<u32>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedProject {
    pub path: String,
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
/// 3. Trusted project config (`./workestrate.toml` in cwd if trusted).
/// 4. Local overrides (`./workestrate.local.toml` in cwd).
///
/// The `$WORKESTRATE_CONFIG_DIR` environment variable bypasses discovery and
/// loads a single dev/testing layer directly.
pub fn load_config() -> Result<ConfigFile> {
    // 1. Dev/testing override: single layer, no merging.
    if let Ok(dir) = std::env::var("WORKESTRATE_CONFIG_DIR") {
        let path = PathBuf::from(dir).join("workestrate.toml");
        if path.exists() {
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

    // 3. Registry layers.
    if let Ok(Some(registry)) = load_registry() {
        for name in &registry.layers {
            let path = resolve_store_dir()
                .join("repos")
                .join(name)
                .join("workestrate.toml");
            if path.exists() {
                layers.push(crate::merge::Layer::load(name, &path)?);
            }
        }
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

/// Validate a loaded config against the policy.rs allowlists.
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
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn project_root_with_agentctl_root_env() {
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
}
