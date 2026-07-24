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
    ///
    /// The string is parsed twice: once as raw `toml::Value` (used by the
    /// merge engine to detect which keys a layer actually declares) and once
    /// as the typed [`ConfigFile`]. Both parses must succeed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use workestrate::merge::Layer;
    ///
    /// const TOML: &str = r#"
    /// schema_version = 1
    ///
    /// [workloads.pi]
    /// kind = "agent"
    /// image = { recipe = "registry", ref = "node:24" }
    /// command = []
    ///
    /// [workloads.pi.network]
    /// default_deny = true
    /// "#;
    ///
    /// let layer = Layer::from_string("base", TOML)?;
    /// assert_eq!(layer.name, "base");
    /// assert!(layer.config.workloads.contains_key("pi"));
    /// # Ok::<(), anyhow::Error>(())
    /// ```
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
///
/// # Example
///
/// ```rust
/// use workestrate::merge::{merge_layers, Layer};
///
/// const TOML: &str = r#"
/// schema_version = 1
///
/// [workloads.pi]
/// kind = "agent"
/// image = { recipe = "registry", ref = "node:24" }
/// command = []
///
/// [workloads.pi.network]
/// default_deny = true
/// "#;
///
/// let base = Layer::from_string("base", TOML)?;
/// let (merged, provenance) = merge_layers(&[base])?;
/// assert!(merged.workloads.contains_key("pi"));
/// assert_eq!(provenance.get("workloads.pi.kind").map(|s| s.as_str()), Some("base"));
/// # Ok::<(), anyhow::Error>(())
/// ```
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
// Provenance process-global storage (WP10/A9)
// ---------------------------------------------------------------------------
//
// A9: these used to be `thread_local!` `RefCell`s. On a tokio MULTI-THREAD
// runtime (main.rs builds `tokio::runtime::Builder::new_multi_thread()`) a
// task can be migrated across OS threads between `.await` points, so a value
// stored in a plain `thread_local` on one thread could be read on a DIFFERENT
// thread after an await — returning `None` or a stale value. Both stores are
// process-level "most recent load in this process" state, so a
// `std::sync::Mutex` is the correct primitive: it has no thread affinity and
// needs no scope establishment (unlike `tokio::task_local!`, which would
// require a `TaskLocal::scope` at task spawn — i.e. a main.rs change that is
// out of scope — and panics when accessed outside its scope). `Mutex` (not
// `RwLock`) is chosen because writes are as common as reads here
// (`take_provenance` writes) and the critical sections are trivially short.
//
// Lock-poisoning policy: `lock().unwrap_or_else(|e| e.into_inner())` recovers
// the inner value instead of panicking, so a panic in one command path cannot
// wedge provenance for every later path (never panics, per WP10).

/// Store the provenance for the most recent merge.
static MERGED_PROVENANCE: std::sync::Mutex<Option<Provenance>> = std::sync::Mutex::new(None);

/// Store the provenance for the most recent merge.
pub fn set_provenance(provenance: Option<Provenance>) {
    *MERGED_PROVENANCE.lock().unwrap_or_else(|e| e.into_inner()) = provenance;
}

/// Take ownership of the stored provenance, leaving the global slot empty.
pub fn take_provenance() -> Option<Provenance> {
    MERGED_PROVENANCE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
}

// ---------------------------------------------------------------------------
// Secret provenance process-global storage (WP10/A9)
// ---------------------------------------------------------------------------

/// Provenance of the most recent secret load ("which layer set each env var").
/// Process-global: overwritten on each `load_secrets` call in secrets_loader.rs
/// (not owned — its `set_secret_provenance` signature must stay), read by
/// `ConfigWorkload::show_source` in workload.rs. See the A9 note on
/// [`MERGED_PROVENANCE`] for why this is a `Mutex` rather than a thread-local.
static SECRET_PROVENANCE: std::sync::Mutex<Option<Provenance>> = std::sync::Mutex::new(None);

/// Store the provenance for the most recent secret load.
pub fn set_secret_provenance(provenance: Option<Provenance>) {
    *SECRET_PROVENANCE.lock().unwrap_or_else(|e| e.into_inner()) = provenance;
}

/// Take ownership of the stored secret provenance, leaving the global slot empty.
///
/// Test-only: consumed by the provenance round-trip tests; production reads
/// provenance via [`get_secret_provenance`].
#[cfg(test)]
pub fn take_secret_provenance() -> Option<Provenance> {
    SECRET_PROVENANCE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .take()
}

/// Clone the stored secret provenance without consuming it.
pub fn get_secret_provenance() -> Option<Provenance> {
    SECRET_PROVENANCE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
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
        // WP6(a): env lists merge union-by-name (deduped by EnvVarConfig.name),
        // last layer wins per key — mirroring the secret_env union semantics
        // below. Base order is preserved for existing keys; brand-new keys are
        // appended in override order. (Previously wholesale replace.)
        for e in &layer.env {
            if let Some(existing) = merged.env.iter_mut().find(|m| m.name == e.name) {
                *existing = e.clone();
            } else {
                merged.env.push(e.clone());
            }
            // Per-key provenance (WP11 renders env provenance end-to-end).
            provenance.insert(
                format!("workloads.{name}.env.{}", e.name),
                layer_ctx.name.clone(),
            );
        }
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
        // FN-3: secret_env lists merge union-by-secret-name with LAST layer
        // wins per secret — mirroring the env union semantics above (an
        // existing entry is replaced IN PLACE; brand-new secrets are
        // appended in override order). Provenance is updated on EVERY
        // (re)declaration, so it always names the layer that set the final
        // value. (Previously first-layer-wins with first-declaration-only
        // provenance — the opposite of env.)
        for se in &layer.secret_env {
            if let Some(existing) = merged.secret_env.iter_mut().find(|m| m.secret == se.secret) {
                *existing = se.clone();
            } else {
                merged.secret_env.push(se.clone());
            }
            provenance.insert(
                format!("workloads.{name}.secret_env.{}", se.secret),
                layer_ctx.name.clone(),
            );
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

/// Canonical dedup key for an egress recipe (FS-24). Two recipes are "the
/// same" when their canonical keys match: for `Https{hosts}` the key is the
/// SORTED + deduped host list (so `a,b` ≡ `b,a`); every other recipe variant
/// is its own key verbatim.
fn canonical_egress_key(recipe: &EgressRecipeRef) -> EgressRecipeRef {
    match recipe {
        EgressRecipeRef::Https { hosts } => {
            let mut hosts = hosts.clone();
            hosts.sort();
            hosts.dedup();
            EgressRecipeRef::Https { hosts }
        }
        other => other.clone(),
    }
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
                // WP3/A4: entitlement is checked BEFORE the monotonic-true
                // invariant. Only entitled workloads (see
                // `policy::DEFAULT_DENY_FALSE_ENTITLEMENT`) may hold or relax
                // to `default_deny = false`; for them the monotonic-true
                // invariant does NOT apply (entitlement is the explicit
                // opt-out from `default_deny = true`). Non-entitled workloads
                // can never reach `Some(false)` at all -- the bail below
                // upholds the monotonic-true invariant as defense-in-depth.
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
            // FS-24: dedup against a CANONICAL KEY (sorted+deduped hosts for
            // Https) so `a,b` and `b,a` collapse — previously order-sensitive
            // PartialEq let both through. The merged list keeps the FIRST
            // declaration's recipe verbatim (declared host order is the
            // rendered plan order, pinned by the golden plans).
            let key = canonical_egress_key(recipe);
            let idx = match merged
                .egress
                .iter()
                .position(|r| canonical_egress_key(r) == key)
            {
                Some(i) => i,
                None => {
                    merged.egress.push(recipe.clone());
                    merged.egress.len().saturating_sub(1)
                }
            };
            // FS-24: provenance updates on EVERY declaration (mirroring the
            // FN-3 last-layer-wins pattern), not only on first insert — the
            // index addresses the slot in the merged list, so a duplicate
            // declaration re-attributes that slot's provenance to the
            // later-declaring layer.
            provenance.insert(
                format!("workloads.{name}.network.egress.{idx}"),
                layer_ctx.name.clone(),
            );
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
#[allow(clippy::unwrap_used, clippy::expect_used)]
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

    // ---- FS-24: Https{hosts} egress dedup is order-insensitive; provenance updates on every declaration ----

    fn https_layer(name: &str, hosts: &str) -> Result<Layer> {
        let toml = format!(
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = {{ recipe = \"registry\", ref = \"node:24\" }}\ncommand = []\n\n[workloads.pi.network]\ndefault_deny = true\n\n[[workloads.pi.network.egress]]\nrecipe = \"https\"\nhosts = {hosts}\n"
        );
        Layer::from_string(name, &toml)
    }

    /// FS-24: `a,b` and `b,a` are the SAME Https recipe — the merged list
    /// must contain exactly one canonicalized (sorted) entry.
    #[test]
    fn https_egress_dedup_is_order_insensitive() -> Result<()> {
        let base = https_layer("base", r#"["openrouter.ai", "github.com"]"#)?;
        let later = https_layer("later", r#"["github.com", "openrouter.ai"]"#)?;

        let (merged, _provenance) = merge_layers(&[base, later])?;
        let pi = merged.workloads.get("pi").unwrap();
        let https_recipes: Vec<_> = pi
            .network
            .egress
            .iter()
            .filter(|r| matches!(r, EgressRecipeRef::Https { .. }))
            .collect();
        assert_eq!(
            https_recipes.len(),
            1,
            "order-permuted Https hosts must dedup to one entry: {:?}",
            pi.network.egress
        );
        match https_recipes[0] {
            EgressRecipeRef::Https { hosts } => {
                assert_eq!(
                    hosts,
                    &vec!["openrouter.ai".to_string(), "github.com".to_string()],
                    "the merged entry keeps the FIRST declaration's host order \
                     (dedup canonicalizes only the comparison key, not the output)"
                );
            }
            other => {
                return Err(anyhow::anyhow!("expected Https recipe, got {other:?}"));
            }
        }
        Ok(())
    }

    /// FS-24: provenance moves to the LATER declaring layer even when the
    /// recipe is a duplicate (mirrors FN-3 last-layer-wins), rather than
    /// sticking with the first declarer inside the dedup guard.
    #[test]
    fn https_egress_provenance_updates_on_duplicate_declaration() -> Result<()> {
        let base = https_layer("base", r#"["openrouter.ai", "github.com"]"#)?;
        let later = https_layer("later", r#"["github.com", "openrouter.ai"]"#)?;

        let (_merged, provenance) = merge_layers(&[base, later])?;
        assert_eq!(
            provenance
                .get("workloads.pi.network.egress.0")
                .map(|s| s.as_str()),
            Some("later"),
            "duplicate declaration must re-attribute provenance to the later layer"
        );
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
        // WP3/A4: after the entitlement-before-monotonic-true reorder, a
        // non-entitled workload (pi) attempting to relax `true -> false`
        // bails at the entitlement check. The monotonic-true invariant is
        // upheld for non-entitled workloads by that entitlement check (they
        // can never reach `Some(false)` at all).
        let base = load_fixture("base", "base");
        let hostile = load_fixture("hostile_default_deny", "hostile_default_deny");

        let err = merge_layers(&[base, hostile]).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("entitlement"),
            "non-entitled pi should bail at the entitlement check: {msg}"
        );
    }

    #[test]
    fn entitled_workload_relaxes_default_deny_true_to_false() -> Result<()> {
        // WP3/A4 regression (the headline test): a three-layer stack where
        // base sets tempest default_deny=true, mid keeps true, top sets
        // false. Because tempest is in DEFAULT_DENY_FALSE_ENTITLEMENT, the
        // top layer must win -- the monotonic-true check must NOT fire for
        // an entitled workload.
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.tempest]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.tempest.network]\ndefault_deny = true",
        )?;
        let mid = Layer::from_string(
            "mid",
            "schema_version = 1\n\n[workloads.tempest]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.tempest.network]\ndefault_deny = true",
        )?;
        let top = Layer::from_string(
            "top",
            "schema_version = 1\n\n[workloads.tempest]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.tempest.network]\ndefault_deny = false",
        )?;

        let (merged, provenance) = merge_layers(&[base, mid, top])?;
        let tempest = merged.workloads.get("tempest").unwrap();
        assert_eq!(
            tempest.network.default_deny,
            Some(false),
            "entitled workload should relax true->false from the top layer"
        );
        assert_eq!(
            provenance.get("workloads.tempest.network.default_deny"),
            Some(&"top".to_string()),
            "provenance should attribute the relaxed value to the top layer"
        );
        Ok(())
    }

    #[test]
    fn non_entitled_workload_cannot_relax_default_deny_true_to_false() {
        // WP3/A4 regression: a non-entitled workload (pi) can never reach
        // `default_deny = false`, even via a three-layer stack that first
        // sets true then attempts to relax to false. The entitlement check
        // (now first) bails at the top layer.
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.pi.network]\ndefault_deny = true",
        ).unwrap();
        let mid = Layer::from_string(
            "mid",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.pi.network]\ndefault_deny = true",
        ).unwrap();
        let top = Layer::from_string(
            "top",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.pi.network]\ndefault_deny = false",
        ).unwrap();

        let err = merge_layers(&[base, mid, top]).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("entitlement"),
            "non-entitled pi should bail at the entitlement check: {msg}"
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
        let direct: ConfigFile = toml::from_str(&std::fs::read_to_string(fixture("base"))?)?;

        let (merged, _) = merge_layers(&[base])?;
        assert_eq!(merged, direct);
        Ok(())
    }

    #[test]
    fn env_union_appends_new_keys() -> Result<()> {
        // WP6(a): base env [A=1,B=2] + override env [C=3] → A=1, B=2, C=3.
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[[workloads.pi.env]]\nname = \"A\"\nvalue = \"1\"\n\n[[workloads.pi.env]]\nname = \"B\"\nvalue = \"2\"\n\n[workloads.pi.network]\ndefault_deny = true",
        )?;
        let team = Layer::from_string(
            "team",
            "schema_version = 1\n\n[workloads.pi]\n\n[[workloads.pi.env]]\nname = \"C\"\nvalue = \"3\"",
        )?;

        let (merged, provenance) = merge_layers(&[base, team])?;
        let pi = merged.workloads.get("pi").unwrap();
        let env_pairs: Vec<(&str, Option<&str>)> = pi
            .env
            .iter()
            .map(|e| (e.name.as_str(), e.value.as_deref()))
            .collect();
        assert_eq!(
            env_pairs,
            vec![("A", Some("1")), ("B", Some("2")), ("C", Some("3"))],
            "override env entries should append after base entries in order"
        );
        assert_eq!(
            provenance.get("workloads.pi.env"),
            Some(&"team".to_string())
        );
        assert_eq!(
            provenance.get("workloads.pi.env.C"),
            Some(&"team".to_string()),
            "per-key provenance should attribute C to the override layer"
        );
        Ok(())
    }

    #[test]
    fn env_union_last_layer_wins_per_key() -> Result<()> {
        // WP6(a): base env [A=1,B=2] + override env [A=9] → A=9 (not
        // duplicated, replaced in place), B=2 still present.
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[[workloads.pi.env]]\nname = \"A\"\nvalue = \"1\"\n\n[[workloads.pi.env]]\nname = \"B\"\nvalue = \"2\"\n\n[workloads.pi.network]\ndefault_deny = true",
        )?;
        let team = Layer::from_string(
            "team",
            "schema_version = 1\n\n[workloads.pi]\n\n[[workloads.pi.env]]\nname = \"A\"\nvalue = \"9\"",
        )?;

        let (merged, provenance) = merge_layers(&[base, team])?;
        let pi = merged.workloads.get("pi").unwrap();
        let env_pairs: Vec<(&str, Option<&str>)> = pi
            .env
            .iter()
            .map(|e| (e.name.as_str(), e.value.as_deref()))
            .collect();
        assert_eq!(
            env_pairs,
            vec![("A", Some("9")), ("B", Some("2"))],
            "A should be replaced in place (last layer wins), B preserved, no duplicate A"
        );
        assert_eq!(
            pi.env.iter().filter(|e| e.name == "A").count(),
            1,
            "A must not be duplicated"
        );
        assert_eq!(
            provenance.get("workloads.pi.env.A"),
            Some(&"team".to_string()),
            "per-key provenance should attribute A to the override layer"
        );
        assert_eq!(
            provenance.get("workloads.pi.env.B"),
            Some(&"base".to_string()),
            "untouched key B keeps the base layer provenance"
        );
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

    // ---- WP10/A9: provenance storage is process-global (thread-safe) ----

    fn sample_provenance() -> Provenance {
        let mut p = Provenance::new();
        p.insert("workloads.pi.cpus".to_string(), "team".to_string());
        p
    }

    /// A9 regression: a value set on one OS thread must be readable on a
    /// DIFFERENT thread (a plain thread_local would return None there — the
    /// exact failure mode on a tokio multi-thread runtime after task
    /// migration). Serial execution is guaranteed by an env-mutex-style lock
    /// shared with the other storage tests below (these tests mutate global
    /// state, so they must not interleave).
    static STORAGE_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn provenance_set_from_another_thread_is_visible() {
        let _guard = STORAGE_TEST_LOCK.lock().unwrap();
        set_provenance(None); // isolate from any earlier test state
        std::thread::spawn(|| set_provenance(Some(sample_provenance())))
            .join()
            .expect("setter thread panicked");
        let taken = take_provenance().expect("provenance set on another thread must be visible");
        assert_eq!(taken.get("workloads.pi.cpus"), Some(&"team".to_string()));
        assert!(take_provenance().is_none(), "take must drain the slot");
    }

    #[test]
    fn secret_provenance_set_from_another_thread_is_visible() {
        let _guard = STORAGE_TEST_LOCK.lock().unwrap();
        set_secret_provenance(None);
        std::thread::spawn(|| set_secret_provenance(Some(sample_provenance())))
            .join()
            .expect("setter thread panicked");
        let got = get_secret_provenance()
            .expect("secret provenance set on another thread must be visible");
        assert_eq!(got.get("workloads.pi.cpus"), Some(&"team".to_string()));
        // get_ does not consume; take_ does.
        let taken = take_secret_provenance().expect("take must return the stored value");
        assert_eq!(taken.get("workloads.pi.cpus"), Some(&"team".to_string()));
        assert!(
            take_secret_provenance().is_none(),
            "take must drain the slot"
        );
    }

    #[test]
    fn provenance_survives_tokio_multi_thread_migration() {
        let _guard = STORAGE_TEST_LOCK.lock().unwrap();
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .build()
            .expect("failed to build multi-thread runtime");
        rt.block_on(async {
            set_provenance(None);
            set_provenance(Some(sample_provenance()));
            // Yield many times to give the scheduler every chance to migrate
            // this task across worker threads before the read.
            for _ in 0..100 {
                tokio::task::yield_now().await;
            }
            let taken =
                take_provenance().expect("provenance must survive await-points on multi-thread rt");
            assert_eq!(taken.get("workloads.pi.cpus"), Some(&"team".to_string()));
        });
    }

    /// Signatures are unchanged (main.rs/secrets_loader.rs are not editable):
    /// if any of these changed shape this would fail to compile.
    #[test]
    fn provenance_function_signatures_stable() {
        let _guard = STORAGE_TEST_LOCK.lock().unwrap();
        let _: fn(Option<Provenance>) = set_provenance;
        let _: fn() -> Option<Provenance> = take_provenance;
        let _: fn(Option<Provenance>) = set_secret_provenance;
        let _: fn() -> Option<Provenance> = get_secret_provenance;
        let _: fn() -> Option<Provenance> = take_secret_provenance;
    }
    // ---- FN-3: secret_env is last-layer-wins per secret, like env ----

    /// A layer declaring `secret_env` entries (by secret name) for workload
    /// `pi`. `SecretEnvConfig` carries only the `secret` key, so the win/lose
    /// signal is which declaration SURVIVES: on a redeclaration the later
    /// layer's entry replaces the earlier one in place (observable via
    /// provenance + single entry), while a first declaration appends.
    fn secret_env_layer(name: &str, secrets: &[&str]) -> Layer {
        let entries: String = secrets
            .iter()
            .map(|s| format!("[[workloads.pi.secret_env]]\nsecret = \"{s}\"\n\n"))
            .collect();
        Layer::from_string(
            name,
            &format!(
                "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = {{ recipe = \"registry\", ref = \"node:24-bookworm-slim\" }}\ncommand = []\nlog_stop_errors = false\n\n{entries}[workloads.pi.network]\ndefault_deny = true"
            ),
        )
        .expect("secret_env layer must parse")
    }

    #[test]
    fn secret_env_union_last_layer_wins_per_secret() -> Result<()> {
        // FN-3 regression: base secret_env [A] + later layer [A] → exactly
        // one entry survives (in-place replace, no duplicate) and provenance
        // attributes the (re)declared secret to the LATER layer. Under the
        // old first-layer-wins semantics the entry was kept and provenance
        // stayed at "base".
        let base = secret_env_layer("base", &["GH_TOKEN"]);
        let team = secret_env_layer("team", &["GH_TOKEN"]);

        let (merged, provenance) = merge_layers(&[base, team])?;
        let pi = merged.workloads.get("pi").unwrap();
        let names: Vec<&str> = pi.secret_env.iter().map(|se| se.secret.as_str()).collect();
        assert_eq!(
            names,
            vec!["GH_TOKEN"],
            "redeclared secret must not be duplicated"
        );
        assert_eq!(
            provenance.get("workloads.pi.secret_env.GH_TOKEN"),
            Some(&"team".to_string()),
            "provenance must attribute the redeclared secret to the later layer"
        );
        Ok(())
    }

    #[test]
    fn secret_env_union_appends_new_secrets_in_order() -> Result<()> {
        // Union still holds: a secret declared only in the base survives a
        // later layer declaring a different secret; the new secret appends
        // after the base entry, each with its own per-secret provenance.
        let base = secret_env_layer("base", &["GH_TOKEN"]);
        let team = secret_env_layer("team", &["NPM_TOKEN"]);

        let (merged, provenance) = merge_layers(&[base, team])?;
        let pi = merged.workloads.get("pi").unwrap();
        let names: Vec<&str> = pi.secret_env.iter().map(|se| se.secret.as_str()).collect();
        assert_eq!(
            names,
            vec!["GH_TOKEN", "NPM_TOKEN"],
            "base secrets preserved; new secrets appended in override order"
        );
        assert_eq!(
            provenance.get("workloads.pi.secret_env.GH_TOKEN"),
            Some(&"base".to_string())
        );
        assert_eq!(
            provenance.get("workloads.pi.secret_env.NPM_TOKEN"),
            Some(&"team".to_string())
        );
        Ok(())
    }

    #[test]
    fn secret_env_three_layer_stack_latest_declaration_wins() -> Result<()> {
        // Three layers declaring the same secret: one entry survives and
        // provenance tracks every (re)declaration, ending at the top layer.
        let base = secret_env_layer("base", &["KEY"]);
        let mid = secret_env_layer("mid", &["KEY"]);
        let top = secret_env_layer("top", &["KEY"]);

        let (merged, provenance) = merge_layers(&[base, mid, top])?;
        let pi = merged.workloads.get("pi").unwrap();
        assert_eq!(pi.secret_env.len(), 1, "no duplicate secret entries");
        assert_eq!(pi.secret_env[0].secret, "KEY");
        assert_eq!(
            provenance.get("workloads.pi.secret_env.KEY"),
            Some(&"top".to_string()),
            "provenance must reflect the final (top) declaration"
        );
        Ok(())
    }
}
