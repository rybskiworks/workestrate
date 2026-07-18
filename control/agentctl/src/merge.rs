use crate::config::{ConfigFile, SecretDefConfig, WorkloadConfig};
use crate::policy;
use crate::recipes::EgressRecipeRef;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

/// Provenance: which layer set each field.
///
/// Key is a dot-path like `workloads.pi.cpus` or `workloads.pi.network.default_deny`.
/// Value is the layer name that set the final value.
pub type Provenance = HashMap<String, String>;

/// A loaded layer with its name and config.
pub struct Layer {
    pub name: String,
    pub config: ConfigFile,
    raw: toml::Value,
}

impl Layer {
    /// Load a layer from a `workestrate.toml` file.
    pub fn load(name: &str, path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read layer config {}", path.display()))?;
        Self::from_string(name, &content)
    }

    /// Load a layer from a TOML string.
    pub fn from_string(name: &str, content: &str) -> Result<Self> {
        let raw: toml::Value = toml::from_str(content)
            .with_context(|| format!("failed to parse raw TOML for layer '{}'", name))?;
        let config: ConfigFile = toml::from_str(content)
            .with_context(|| format!("failed to parse typed config for layer '{}'", name))?;
        Ok(Self {
            name: name.to_string(),
            config,
            raw,
        })
    }
}

/// Merge an ordered list of layers (earlier = lower precedence).
///
/// Returns the merged config and a provenance map describing which layer set
/// each final field value.
pub fn merge_layers(layers: &[Layer]) -> Result<(ConfigFile, Provenance)> {
    let mut merged = ConfigFile::default();
    let mut provenance = Provenance::new();

    for layer in layers {
        merge_schema_version(&mut merged, layer, &mut provenance);
        merge_secrets(&mut merged, layer, &mut provenance)?;
        merge_workloads(&mut merged, layer, &mut provenance)?;
    }

    Ok((merged, provenance))
}

// ---------------------------------------------------------------------------
// Provenance thread-local storage
// ---------------------------------------------------------------------------

thread_local! {
    static MERGED_PROVENANCE: std::cell::RefCell<Option<Provenance>> =
        const { std::cell::RefCell::new(None) };
}

/// Store the provenance for the most recent merge.
pub fn set_provenance(provenance: Option<Provenance>) {
    MERGED_PROVENANCE.with(|p| {
        *p.borrow_mut() = provenance;
    });
}

/// Take ownership of the stored provenance, leaving the thread-local empty.
pub fn take_provenance() -> Option<Provenance> {
    MERGED_PROVENANCE.with(|p| p.borrow_mut().take())
}

// ---------------------------------------------------------------------------
// Merge helpers
// ---------------------------------------------------------------------------

fn merge_schema_version(merged: &mut ConfigFile, layer: &Layer, provenance: &mut Provenance) {
    if layer.raw.get("schema_version").is_some() {
        merged.schema_version = layer.config.schema_version;
        provenance.insert("schema_version".to_string(), layer.name.clone());
    }
}

fn merge_secrets(
    merged: &mut ConfigFile,
    layer: &Layer,
    provenance: &mut Provenance,
) -> Result<()> {
    let secrets_table = match layer.raw.get("secrets").and_then(|v| v.as_table()) {
        Some(t) => t,
        None => return Ok(()),
    };

    for (name, _) in secrets_table {
        let layer_secret = layer.config.secrets.get(name).cloned().unwrap_or_default();
        let merged_secret = merged.secrets.entry(name.clone()).or_default();
        merge_secret_def(merged_secret, &layer_secret, name, layer, provenance)?;
    }

    Ok(())
}

fn merge_secret_def(
    merged: &mut SecretDefConfig,
    layer: &SecretDefConfig,
    name: &str,
    layer_ctx: &Layer,
    provenance: &mut Provenance,
) -> Result<()> {
    let raw_secret = layer_ctx
        .raw
        .get("secrets")
        .and_then(|v| v.get(name))
        .and_then(|v| v.as_table());

    let Some(table) = raw_secret else {
        return Ok(());
    };

    if table.contains_key("env_var") {
        merged.env_var = layer.env_var.clone();
        provenance.insert(format!("secrets.{name}.env_var"), layer_ctx.name.clone());
    }
    if table.contains_key("hosts") {
        merged.hosts = layer.hosts.clone();
        provenance.insert(format!("secrets.{name}.hosts"), layer_ctx.name.clone());
    }
    if table.contains_key("required") {
        merged.required = layer.required;
        provenance.insert(format!("secrets.{name}.required"), layer_ctx.name.clone());
    }
    if table.contains_key("placeholder") {
        merged.placeholder = layer.placeholder.clone();
        provenance.insert(
            format!("secrets.{name}.placeholder"),
            layer_ctx.name.clone(),
        );
    }
    if table.contains_key("source") {
        merged.source = layer.source.clone();
        provenance.insert(format!("secrets.{name}.source"), layer_ctx.name.clone());
    }
    if table.contains_key("exposed_as") {
        merged.exposed_as = layer.exposed_as.clone();
        provenance.insert(format!("secrets.{name}.exposed_as"), layer_ctx.name.clone());
    }
    if table.contains_key("description") {
        merged.description = layer.description.clone();
        provenance.insert(
            format!("secrets.{name}.description"),
            layer_ctx.name.clone(),
        );
    }

    Ok(())
}

fn merge_workloads(
    merged: &mut ConfigFile,
    layer: &Layer,
    provenance: &mut Provenance,
) -> Result<()> {
    let workloads_table = match layer.raw.get("workloads").and_then(|v| v.as_table()) {
        Some(t) => t,
        None => return Ok(()),
    };

    for (name, _) in workloads_table {
        let layer_wl = layer
            .config
            .workloads
            .get(name)
            .cloned()
            .unwrap_or_default();
        let merged_wl = merged.workloads.entry(name.clone()).or_default();
        merge_workload(merged_wl, &layer_wl, name, layer, provenance)?;
    }

    Ok(())
}

fn merge_workload(
    merged: &mut WorkloadConfig,
    layer: &WorkloadConfig,
    name: &str,
    layer_ctx: &Layer,
    provenance: &mut Provenance,
) -> Result<()> {
    let raw_wl = layer_ctx
        .raw
        .get("workloads")
        .and_then(|v| v.get(name))
        .and_then(|v| v.as_table());

    let Some(table) = raw_wl else {
        return Ok(());
    };

    if table.contains_key("kind") {
        merged.kind = layer.kind.clone();
        provenance.insert(format!("workloads.{name}.kind"), layer_ctx.name.clone());
    }
    if table.contains_key("image") {
        merged.image = layer.image.clone();
        provenance.insert(format!("workloads.{name}.image"), layer_ctx.name.clone());
    }
    if table.contains_key("workdir") {
        merged.workdir = layer.workdir.clone();
        provenance.insert(format!("workloads.{name}.workdir"), layer_ctx.name.clone());
    }
    if table.contains_key("cpus") {
        merged.cpus = layer.cpus;
        provenance.insert(format!("workloads.{name}.cpus"), layer_ctx.name.clone());
    }
    if table.contains_key("memory_mib") {
        merged.memory_mib = layer.memory_mib;
        provenance.insert(
            format!("workloads.{name}.memory_mib"),
            layer_ctx.name.clone(),
        );
    }
    if table.contains_key("command") {
        merged.command = layer.command.clone();
        provenance.insert(format!("workloads.{name}.command"), layer_ctx.name.clone());
    }
    if table.contains_key("log_stop_errors") {
        merged.log_stop_errors = layer.log_stop_errors;
        provenance.insert(
            format!("workloads.{name}.log_stop_errors"),
            layer_ctx.name.clone(),
        );
    }
    if table.contains_key("env") {
        merged.env = layer.env.clone();
        provenance.insert(format!("workloads.{name}.env"), layer_ctx.name.clone());
    }
    if table.contains_key("ports") {
        merged.ports = layer.ports.clone();
        provenance.insert(format!("workloads.{name}.ports"), layer_ctx.name.clone());
    }
    if table.contains_key("mounts") {
        merged.mounts = layer.mounts.clone();
        provenance.insert(format!("workloads.{name}.mounts"), layer_ctx.name.clone());
    }
    if table.contains_key("seed_files") {
        merged.seed_files = layer.seed_files.clone();
        provenance.insert(
            format!("workloads.{name}.seed_files"),
            layer_ctx.name.clone(),
        );
    }
    if table.contains_key("local_build") {
        merged.local_build = layer.local_build.clone();
        provenance.insert(
            format!("workloads.{name}.local_build"),
            layer_ctx.name.clone(),
        );
    }
    if table.contains_key("secret_env") {
        for se in &layer.secret_env {
            if !merged.secret_env.iter().any(|m| m.secret == se.secret) {
                merged.secret_env.push(se.clone());
                provenance.insert(
                    format!("workloads.{name}.secret_env.{}", se.secret),
                    layer_ctx.name.clone(),
                );
            }
        }
    }

    if table.contains_key("network") {
        let raw_network = table.get("network").and_then(|v| v.as_table());
        merge_network(
            &mut merged.network,
            &layer.network,
            name,
            layer_ctx,
            raw_network,
            provenance,
        )?;
    }

    Ok(())
}

fn merge_network(
    merged: &mut crate::config::NetworkConfig,
    layer: &crate::config::NetworkConfig,
    name: &str,
    layer_ctx: &Layer,
    raw_network: Option<&toml::map::Map<String, toml::Value>>,
    provenance: &mut Provenance,
) -> Result<()> {
    let Some(raw_network) = raw_network else {
        return Ok(());
    };

    if raw_network.contains_key("default_deny") {
        match layer.default_deny {
            Some(false) => {
                if merged.default_deny == Some(true) {
                    anyhow::bail!(
                        "layer '{}' cannot set default_deny=false for workload '{}'; \
                         an earlier layer set default_deny=true (monotonic-true invariant)",
                        layer_ctx.name,
                        name
                    );
                }
                if !policy::DEFAULT_DENY_FALSE_ENTITLEMENT.contains(&name) {
                    anyhow::bail!(
                        "workload '{}' is not entitled to default_deny=false (core entitlement: DEFAULT_DENY_FALSE_ENTITLEMENT)",
                        name
                    );
                }
                merged.default_deny = Some(false);
            }
            Some(true) => {
                merged.default_deny = Some(true);
            }
            None => {
                merged.default_deny = None;
            }
        }
        provenance.insert(
            format!("workloads.{name}.network.default_deny"),
            layer_ctx.name.clone(),
        );
    }

    if raw_network.contains_key("egress") {
        for recipe in &layer.egress {
            if let EgressRecipeRef::Https { hosts } = recipe {
                for host in hosts {
                    if !policy::ALLOWED_EGRESS_HOSTS.contains(&host.as_str()) {
                        anyhow::bail!(
                            "layer '{}' egress host '{}' for workload '{}' is not in the core egress allowlist",
                            layer_ctx.name,
                            host,
                            name
                        );
                    }
                }
            }
            if !merged.egress.contains(recipe) {
                merged.egress.push(recipe.clone());
                let idx = merged.egress.len().saturating_sub(1);
                provenance.insert(
                    format!("workloads.{name}.network.egress.{idx}"),
                    layer_ctx.name.clone(),
                );
            }
        }
    }

    if raw_network.contains_key("deny") {
        for rule in &layer.deny {
            if !merged
                .deny
                .iter()
                .any(|r| r.domain_suffix == rule.domain_suffix)
            {
                merged.deny.push(rule.clone());
                provenance.insert(
                    format!("workloads.{name}.network.deny.{}", rule.domain_suffix),
                    layer_ctx.name.clone(),
                );
            }
        }
    }

    if raw_network.contains_key("ingress") {
        merged.ingress = layer.ingress.clone();
        provenance.insert(
            format!("workloads.{name}.network.ingress"),
            layer_ctx.name.clone(),
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(path: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("layering")
            .join(path)
            .join("workestrate.toml")
    }

    fn load_fixture(name: &str, path: &str) -> Layer {
        Layer::load(name, &fixture(path)).unwrap()
    }

    #[test]
    fn happy_path_merge_base_team_personal() -> Result<()> {
        let base = load_fixture("base", "base");
        let team = load_fixture("team", "team");
        let personal = load_fixture("personal", "personal");

        let (merged, provenance) = merge_layers(&[base, team, personal])?;
        let pi = merged.workloads.get("pi").unwrap();

        assert_eq!(pi.cpus, Some(2), "team override should set cpus=2");
        assert_eq!(
            pi.memory_mib,
            Some(2048),
            "personal override should set memory_mib=2048"
        );
        assert_eq!(pi.network.default_deny, Some(true));

        let egress_names: Vec<String> = pi
            .network
            .egress
            .iter()
            .map(|r| match r {
                EgressRecipeRef::Dns => "dns".to_string(),
                EgressRecipeRef::LitellmProxy => "litellm_proxy".to_string(),
                EgressRecipeRef::Github => "github".to_string(),
                EgressRecipeRef::AgentBase => "agent_base".to_string(),
                EgressRecipeRef::Https { hosts } => format!("https:{}", hosts.join(",")),
            })
            .collect();
        assert!(egress_names.contains(&"dns".to_string()));
        assert!(egress_names.contains(&"github".to_string()));

        let deny_suffixes: Vec<&str> = pi
            .network
            .deny
            .iter()
            .map(|r| r.domain_suffix.as_str())
            .collect();
        assert!(deny_suffixes.contains(&".evil.com"));
        assert!(deny_suffixes.contains(&".tracker.io"));

        assert_eq!(
            provenance.get("workloads.pi.cpus"),
            Some(&"personal".to_string())
        );
        assert_eq!(
            provenance.get("workloads.pi.memory_mib"),
            Some(&"personal".to_string())
        );

        Ok(())
    }

    #[test]
    fn scalar_override_team_changes_cpus() -> Result<()> {
        let base = load_fixture("base", "base");
        let team = load_fixture("team", "team");

        let (merged, _) = merge_layers(&[base, team])?;
        let pi = merged.workloads.get("pi").unwrap();
        assert_eq!(pi.cpus, Some(2));
        Ok(())
    }

    #[test]
    fn map_deep_merge_adds_secret() -> Result<()> {
        let base = load_fixture("base", "base");
        let team = load_fixture("team", "team");

        let (merged, _) = merge_layers(&[base, team])?;
        assert!(merged.secrets.contains_key("LITELLM_MASTER_KEY"));
        assert!(merged.secrets.contains_key("GITHUB_TOKEN"));
        Ok(())
    }

    #[test]
    fn list_replace_command() -> Result<()> {
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = [\"echo\"]\nlog_stop_errors = false\n\n[workloads.pi.network]\ndefault_deny = true",
        )?;
        let team = Layer::from_string(
            "team",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = [\"cat\"]\nlog_stop_errors = false\n\n[workloads.pi.network]\ndefault_deny = true",
        )?;

        let (merged, _) = merge_layers(&[base, team])?;
        let pi = merged.workloads.get("pi").unwrap();
        assert_eq!(pi.command, vec!["cat"]);
        Ok(())
    }

    #[test]
    fn deny_union_across_layers() -> Result<()> {
        let team = load_fixture("team", "team");
        let personal = load_fixture("personal", "personal");

        let (merged, _) = merge_layers(&[team, personal])?;
        let pi = merged.workloads.get("pi").unwrap();
        let suffixes: Vec<&str> = pi
            .network
            .deny
            .iter()
            .map(|r| r.domain_suffix.as_str())
            .collect();
        assert!(suffixes.contains(&".evil.com"));
        assert!(suffixes.contains(&".tracker.io"));
        Ok(())
    }

    #[test]
    fn egress_union_across_layers() -> Result<()> {
        let base = load_fixture("base", "base");
        let team = load_fixture("team", "team");

        let (merged, _) = merge_layers(&[base, team])?;
        let pi = merged.workloads.get("pi").unwrap();
        assert!(pi.network.egress.contains(&EgressRecipeRef::Dns));
        assert!(pi.network.egress.contains(&EgressRecipeRef::Github));
        Ok(())
    }

    #[test]
    fn default_deny_monotonic_true_blocks_false() {
        let base = load_fixture("base", "base");
        let hostile = load_fixture("hostile_default_deny", "hostile_default_deny");

        let err = merge_layers(&[base, hostile]).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("monotonic-true"),
            "error should mention monotonic-true: {msg}"
        );
    }

    #[test]
    fn default_deny_false_requires_entitlement() {
        let hostile = load_fixture("hostile_default_deny", "hostile_default_deny");

        let err = merge_layers(&[hostile]).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("entitlement"),
            "error should mention entitlement: {msg}"
        );
    }

    #[test]
    fn egress_ceiling_blocks_unknown_host() {
        let hostile = load_fixture("hostile_egress", "hostile_egress");

        let err = merge_layers(&[hostile]).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("allowlist"),
            "error should mention allowlist: {msg}"
        );
        assert!(msg.contains("evil.com"), "error should mention host: {msg}");
    }

    #[test]
    fn single_layer_parity() -> Result<()> {
        let base = load_fixture("base", "base");
        let direct: ConfigFile = toml::from_str(&std::fs::read_to_string(&fixture("base"))?)?;

        let (merged, _) = merge_layers(&[base])?;
        assert_eq!(merged, direct);
        Ok(())
    }

    #[test]
    fn tempest_entitlement_default_deny_false_ok() -> Result<()> {
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.tempest]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.tempest.network]\ndefault_deny = false",
        )?;

        let (merged, _) = merge_layers(&[base])?;
        let tempest = merged.workloads.get("tempest").unwrap();
        assert_eq!(tempest.network.default_deny, Some(false));
        Ok(())
    }
}
