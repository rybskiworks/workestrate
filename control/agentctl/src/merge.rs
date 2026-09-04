use crate::config::{
    ConfigFile, EgressPolicyFragment, IdnaPolicyFragment, IngressPolicyFragment, SecretDefConfig,
    SecretsPolicyFragment, VirtualizationPolicyFragment, WorkloadConfig,
};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Provenance: which layer set each field.
///
/// Key is a dot-path like `workloads.pi.cpus` or `workloads.pi.network.defaults.egress`.
/// Value is the layer name that set the final value.
pub type Provenance = HashMap<String, String>;

/// A loaded layer with its name and config.
pub struct Layer {
    pub name: String,
    pub config: ConfigFile,
    raw: toml::Value,
    /// The file this layer was loaded from, when known. `None` for synthetic
    /// layers built from in-memory strings with no on-disk source. Carried
    /// so repo-relative mount/seed paths can resolve against the DECLARING
    /// layer's content root — the directory-mode root (`<repo>/workestrate/`)
    /// for directory-mode layers, or the file's parent dir for single-file
    /// mode — rather than the flake project root; see [`layer_dirs_from`].
    pub source_path: Option<PathBuf>,
}

impl Layer {
    /// Load a layer from a `workestrate.toml` file.
    pub fn load(name: &str, path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read layer config {}", path.display()))?;
        Self::from_string_with_path(name, &content, Some(path.to_path_buf()))
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
    /// [workloads.pi.network.defaults]
    /// egress = "deny"
    /// "#;
    ///
    /// let layer = Layer::from_string("base", TOML)?;
    /// assert_eq!(layer.name, "base");
    /// assert!(layer.config.workloads.contains_key("pi"));
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn from_string(name: &str, content: &str) -> Result<Self> {
        Self::from_string_with_path(name, content, None)
    }

    /// Load a layer from a TOML string, recording the on-disk file the
    /// content came from (when there is one). Directory-mode config repos
    /// (spec 17) load each pseudo-layer from a real file under
    /// `<repo>/workestrate/`, so they pass that path here; purely synthetic
    /// layers pass `None`.
    pub fn from_string_with_path(
        name: &str,
        content: &str,
        source_path: Option<PathBuf>,
    ) -> Result<Self> {
        let raw: toml::Value = toml::from_str(content)
            .with_context(|| format!("failed to parse raw TOML for layer '{}'", name))?;
        let config: ConfigFile = toml::from_str(content)
            .map_err(|e| anyhow::anyhow!("failed to parse typed config for layer '{name}': {e}"))?;
        Ok(Self {
            name: name.to_string(),
            config,
            raw,
            source_path,
        })
    }

    pub(crate) fn raw(&self) -> &toml::Value {
        &self.raw
    }
}

/// Build the layer-name → content-root map for a merged layer set.
///
/// A layer's content root is the directory repo-relative mount/seed paths
/// resolve against (spec 17). It is derived STRUCTURALLY from the layer's
/// `source_path` (no filesystem access, so non-existent test paths work):
///
/// - **Single-file mode** (`<repo>/workestrate.toml`): the content root is
///   the file's parent dir (`<repo>/`). The single-file marker
///   `workestrate.toml` is detected by file name and short-circuits the
///   walk-up.
/// - **Directory mode** (`<repo>/workestrate/...`): the content root is the
///   **directory-mode root** `<repo>/workestrate/` — the dir containing
///   `default.toml` / the parent of `workloads/` — NOT the immediate parent
///   of the layer file. For `default.toml`/`secrets.toml` the parent IS the
///   root; for `workloads/<name>.toml` (flat) and
///   `workloads/<name>/workload.toml` (capsule) the root is found by walking
///   up to the nearest ancestor named `workloads` and taking its parent.
///
/// This corrects the phase-0/phase-1 capsule-relative choice (which resolved
/// a `host = "workloads/litellm"` mount against the capsule dir, doubling it
/// to `workestrate/workloads/litellm/workloads/litellm` — host-boot failure
/// 1). The directory-mode root is what config authors write paths against.
///
/// Layers without a source path (synthetic `from_string` layers) are absent
/// — callers then apply the documented fallback (flake project root, else
/// cwd) explicitly.
///
/// DESIGN NOTE (phases 1-4): later approved phases move image builds into
/// config-repo flakes, making the config repo the flake root. This map is
/// the plumbing those phases should reuse to locate each layer's content
/// root — do not duplicate the layer-name → dir derivation.
pub fn layer_dirs_from(layers: &[Layer]) -> HashMap<String, PathBuf> {
    layers
        .iter()
        .filter_map(|layer| {
            layer
                .source_path
                .as_ref()
                .and_then(|p| content_root_for_layer_file(p).map(|dir| (layer.name.clone(), dir)))
        })
        .collect()
}

/// Derive the content root for one layer file, structurally (no FS access).
///
/// See [`layer_dirs_from`] for the rules. Single-file mode (`workestrate.toml`)
/// → the file's parent. Directory mode → the nearest `workloads` ancestor's
/// parent, falling back to the file's parent when no `workloads` ancestor
/// exists (e.g. `default.toml`/`secrets.toml` sitting directly in the
/// workestrate root).
fn content_root_for_layer_file(source_path: &Path) -> Option<PathBuf> {
    let parent = source_path.parent()?;
    // Single-file mode marker: `workestrate.toml` at a repo root.
    if source_path.file_name() == Some(std::ffi::OsStr::new("workestrate.toml")) {
        return Some(parent.to_path_buf());
    }
    // Directory mode: walk up to the nearest ancestor named `workloads`; its
    // parent is the directory-mode root (`<repo>/workestrate/`). If none is
    // found (default.toml/secrets.toml at the root — parent IS the root),
    // fall back to the file's parent.
    for ancestor in parent.ancestors() {
        if ancestor.file_name() == Some(std::ffi::OsStr::new("workloads")) {
            return ancestor.parent().map(|p| p.to_path_buf());
        }
    }
    Some(parent.to_path_buf())
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
/// [workloads.pi.network.defaults]
/// egress = "deny"
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

/// Clone the stored provenance without consuming it. Companion to
/// [`take_provenance`] (which is consumed by `ConfigWorkload::new`):
/// read-only callers — e.g. `source clone` resolving the declaring config
/// layer — must not drain the slot. Same accessor shape as
/// [`get_layer_dirs`] / [`get_secret_provenance`].
pub fn get_provenance() -> Option<Provenance> {
    MERGED_PROVENANCE
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

// ---------------------------------------------------------------------------
// Layer content-dir process-global storage
// ---------------------------------------------------------------------------
//
// Companion to [`MERGED_PROVENANCE`]: provenance records WHICH layer set a
// field; this map records WHERE that layer's content lives on disk (the
// directory-mode root for directory-mode layers, or the file's parent for
// single-file mode — see [`layer_dirs_from`]). Together they let repo-relative
// mount and seed_file paths resolve against the DECLARING layer's content
// root instead of the flake project root (spec 17 directory mode: config
// content lives in config repos, not in the tool checkout). Same `Mutex`
// rationale as the provenance stores above (tokio multi-thread task migration).

/// Layer-name → content dir for the most recent config load.
static LAYER_DIRS: std::sync::Mutex<Option<HashMap<String, PathBuf>>> = std::sync::Mutex::new(None);

/// Store the layer content dirs for the most recent config load.
pub fn set_layer_dirs(dirs: Option<HashMap<String, PathBuf>>) {
    *LAYER_DIRS.lock().unwrap_or_else(|e| e.into_inner()) = dirs;
}

/// Clone the stored layer content dirs without consuming them. (Unlike
/// provenance — which is TAKEN by `ConfigWorkload::new` — the dirs are read
/// by every workload constructed after a load, so the accessor is a clone.)
pub fn get_layer_dirs() -> Option<HashMap<String, PathBuf>> {
    LAYER_DIRS.lock().unwrap_or_else(|e| e.into_inner()).clone()
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
// Secret violation-policy ladder process-global storage
// ---------------------------------------------------------------------------
//
// The collected `[policy.secrets]` rungs for the most recent config load,
// mirroring the mount-policy COLLECTED store (crate::mount_policy):
// fragments are COLLECTED per scope, never merged (no policy field passes
// through merge_layers), and the resolution walks them authority-ascending.
// Rung 2 (home registry) + rung 3 (config-repo layers, stack order) are
// global; rung 4 (workload capsule) is keyed by workload name. Rung 1
// (built-in passthrough) and rung 5 (merged per-secret entries) are not
// collected — they are constants of the resolution. Same Mutex rationale
// as the provenance stores above (tokio multi-thread task migration).

/// The collected secret violation-policy ladder rungs (rungs 2-4). Each
/// entry carries the ORIGIN label used in resolution provenance (the
/// home-registry scope label, or the declaring layer's name).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SecretPolicyLadder {
    /// Rung 2: the home registry's `[policy.secrets]` (operator scope).
    pub home: Option<(String, SecretsPolicyFragment)>,
    /// Rung 3: each layer's `[policy.secrets]` in loader stack order.
    pub layers: Vec<(String, SecretsPolicyFragment)>,
    /// Rung 4: workload capsule `[policy.secrets]` rungs per workload name,
    /// in loader stack order.
    pub workloads: HashMap<String, Vec<(String, SecretsPolicyFragment)>>,
}

static SECRET_POLICY_LADDER: std::sync::Mutex<Option<SecretPolicyLadder>> =
    std::sync::Mutex::new(None);

/// Store the collected secret violation-policy ladder for the most recent config load.
pub fn set_secret_policy_ladder(ladder: Option<SecretPolicyLadder>) {
    *SECRET_POLICY_LADDER
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = ladder;
}

/// Clone the stored secret violation-policy ladder without consuming it.
pub fn get_secret_policy_ladder() -> Option<SecretPolicyLadder> {
    SECRET_POLICY_LADDER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

// ---------------------------------------------------------------------------
// Network policy ladder process-global storage (ADR 0035)
// ---------------------------------------------------------------------------
//
// The collected `[policy.egress]` / `[policy.ingress]` / `[policy.idna]`
// rungs for the most recent config load, mirroring the secret-policy ladder:
// fragments are COLLECTED per scope, never merged (no policy field passes
// through merge_layers), and the resolution walks them authority-ascending.
// Rung 1 is home-registry, rung 2 is config layers in stack order, rung 3 is
// workload capsule per workload name. Same Mutex rationale as above.

/// The collected network policy ladder rungs (ADR 0035). Each entry carries
/// the ORIGIN label used in resolution provenance (home-registry scope label,
/// or the declaring layer's name).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NetworkPolicyLadder {
    /// Egress ladder: home, layers, workloads.
    pub egress_home: Option<(String, EgressPolicyFragment)>,
    pub egress_layers: Vec<(String, EgressPolicyFragment)>,
    pub egress_workloads: HashMap<String, Vec<(String, EgressPolicyFragment)>>,
    /// Ingress ladder.
    pub ingress_home: Option<(String, IngressPolicyFragment)>,
    pub ingress_layers: Vec<(String, IngressPolicyFragment)>,
    pub ingress_workloads: HashMap<String, Vec<(String, IngressPolicyFragment)>>,
    /// IDNA ladder.
    pub idna_home: Option<(String, IdnaPolicyFragment)>,
    pub idna_layers: Vec<(String, IdnaPolicyFragment)>,
    pub idna_workloads: HashMap<String, Vec<(String, IdnaPolicyFragment)>>,
}

static NETWORK_POLICY_LADDER: std::sync::Mutex<Option<NetworkPolicyLadder>> =
    std::sync::Mutex::new(None);

/// Store the collected network policy ladder for the most recent config load.
pub fn set_network_policy_ladder(ladder: Option<NetworkPolicyLadder>) {
    *NETWORK_POLICY_LADDER
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = ladder;
}

/// Clone the stored network policy ladder without consuming it.
pub fn get_network_policy_ladder() -> Option<NetworkPolicyLadder> {
    NETWORK_POLICY_LADDER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
}

// ---------------------------------------------------------------------------
// Nested-virtualization policy ladder process-global storage (ADR 0036)
// ---------------------------------------------------------------------------
//
// The collected `[policy.virtualization]` seal fragments for the most recent
// config load, mirroring the secret-policy ladder: fragments are COLLECTED
// per scope, never merged (no policy field passes through merge_layers),
// and the resolution walks them authority-ascending. Rung 1 is
// home-registry, rung 2 is config layers in stack order, rung 3 is workload
// capsules (`[workloads.<name>.policy.virtualization]`) per workload name.
// The workload's `[workloads.<name>.virtualization]` ASK is not a policy
// fragment — it merges whole-unit in `merge_workload` and is resolved
// against this ladder by `crate::microsandbox::nested::resolve_for_workload`.
// Same Mutex rationale as above.

/// The collected nested-virtualization seal ladder rungs (ADR 0036). Each
/// entry carries the ORIGIN label used in resolution provenance
/// (home-registry scope label, or the declaring layer's name).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VirtualizationLadder {
    /// Rung 1: the home registry's `[policy.virtualization]` (operator scope).
    pub home: Option<(String, VirtualizationPolicyFragment)>,
    /// Rung 2: each layer's `[policy.virtualization]` in loader stack order.
    pub layers: Vec<(String, VirtualizationPolicyFragment)>,
    /// Rung 3: workload capsule `[policy.virtualization]` rungs per workload
    /// name, in loader stack order.
    pub workloads: HashMap<String, Vec<(String, VirtualizationPolicyFragment)>>,
}

static VIRTUALIZATION_LADDER: std::sync::Mutex<Option<VirtualizationLadder>> =
    std::sync::Mutex::new(None);

/// Store the collected virtualization seal ladder for the most recent config load.
pub fn set_virtualization_ladder(ladder: Option<VirtualizationLadder>) {
    *VIRTUALIZATION_LADDER
        .lock()
        .unwrap_or_else(|e| e.into_inner()) = ladder;
}

/// Clone the stored virtualization seal ladder without consuming it.
pub fn get_virtualization_ladder() -> Option<VirtualizationLadder> {
    VIRTUALIZATION_LADDER
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
    if table.contains_key("allowed_hosts") {
        merged.allowed_hosts = layer.allowed_hosts.clone();
        provenance.insert(
            format!("secrets.{name}.allowed_hosts"),
            layer_ctx.name.clone(),
        );
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
    if table.contains_key("on_violation") {
        merged.on_violation = layer.on_violation;
        provenance.insert(
            format!("secrets.{name}.on_violation"),
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
        // WP6(a): env bindings merge union-by-name, last layer wins per key
        // — atomic per key (a binding replaces a same-key binding wholesale;
        // it never field-merges, spec 16 §5). Base order is preserved for
        // existing keys; brand-new keys are appended in override order.
        // (Previously wholesale replace.)
        for (key, binding) in layer.env.iter() {
            merged.env.upsert(key, binding.clone());
            // Per-key provenance (WP11 renders env provenance end-to-end).
            provenance.insert(
                format!("workloads.{name}.env.{key}"),
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
    if table.contains_key("depends_on") {
        // ADR 0026(d): depends_on maps merge union-by-dependency-name, last
        // layer wins per dep — mirroring the env union semantics above (for
        // a map, insert/overwrite replaces an existing entry in place).
        // Provenance is recorded per dep AND for the whole key, exactly like
        // the env arm.
        for (dep, spec) in &layer.depends_on {
            merged.depends_on.insert(dep.clone(), spec.clone());
            provenance.insert(
                format!("workloads.{name}.depends_on.{dep}"),
                layer_ctx.name.clone(),
            );
        }
        provenance.insert(
            format!("workloads.{name}.depends_on"),
            layer_ctx.name.clone(),
        );
    }

    if table.contains_key("instance") {
        // ADR 0030 Phase 1: the instance policy block is ONE unit — a higher
        // layer re-declaring [workloads.<name>.instance] replaces the WHOLE
        // block (strategy/on_conflict/port/label), last layer wins — the
        // same whole-spec reset semantics depends_on applies per dep. A
        // higher layer declaring only `port` resets the other fields to
        // defaults. Within one layer's declaration the fields are
        // independent; across layers there is NO per-field merge.
        merged.instance = layer.instance.clone();
        provenance.insert(format!("workloads.{name}.instance"), layer_ctx.name.clone());
    }

    if table.contains_key("virtualization") {
        // ADR 0036 §3: the virtualization ask is ONE unit — a higher layer
        // re-declaring [workloads.<name>.virtualization] replaces the WHOLE
        // table (last layer wins), the same whole-spec reset semantics the
        // instance block applies. The home `[policy.virtualization]` seal is
        // NOT merged here — it is collected via the virtualization ladder.
        merged.virtualization = layer.virtualization.clone();
        provenance.insert(
            format!("workloads.{name}.virtualization"),
            layer_ctx.name.clone(),
        );
    }

    if table.contains_key("network") {
        let raw_network = table.get("network").and_then(|v| v.as_table());
        // Explicit `defaults.{egress,ingress} = "allow"` stands alone: the
        // winning (most-specific) layer wins; home `final` seals veto via
        // the policy ladder. Monotonic-deny-upward is gone by design
        // (2026-09-04, entitlements removal).
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

    let raw_defaults = raw_network.get("defaults").and_then(|v| v.as_table());
    if raw_defaults.is_some_and(|d| d.contains_key("egress")) {
        // No gate: an explicit `allow` at the winning layer wins
        // (last-wins per precedence); `deny` (tightening) is always
        // allowed. Monotonic-deny-upward is gone by design (2026-09-04,
        // entitlements removal — replaced by final seals + review + plan NOTE).
        // Field-wise merge: assigning only the `egress` field preserves any
        // `ingress` default set by another layer (and vice versa below).
        let mut defaults = merged.defaults.unwrap_or_default();
        defaults.egress = layer.defaults.and_then(|d| d.egress);
        merged.defaults =
            (defaults != crate::config::NetworkDefaultsConfig::default()).then_some(defaults);
        provenance.insert(
            format!("workloads.{name}.network.defaults.egress"),
            layer_ctx.name.clone(),
        );
    }

    if raw_defaults.is_some_and(|d| d.contains_key("ingress")) {
        // Mirror of the egress rule above: no gate, last-wins.
        let mut defaults = merged.defaults.unwrap_or_default();
        defaults.ingress = layer.defaults.and_then(|d| d.ingress);
        merged.defaults =
            (defaults != crate::config::NetworkDefaultsConfig::default()).then_some(defaults);
        provenance.insert(
            format!("workloads.{name}.network.defaults.ingress"),
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
    use crate::config::EnvBinding;
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
        assert_eq!(
            pi.network.defaults.and_then(|d| d.egress),
            Some(crate::config::DefaultAction::Deny)
        );



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
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = [\"echo\"]\nlog_stop_errors = false\n\n[workloads.pi.network.defaults]\negress = \"deny\"",
        )?;
        let team = Layer::from_string(
            "team",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = [\"cat\"]\nlog_stop_errors = false\n\n[workloads.pi.network.defaults]\negress = \"deny\"",
        )?;

        let (merged, _) = merge_layers(&[base, team])?;
        let pi = merged.workloads.get("pi").unwrap();
        assert_eq!(pi.command, vec!["cat"]);
        Ok(())
    }

    // FS-24 tests removed (egress recipes removed per ADR 0035)


    // deny/egress union tests removed (per ADR 0035)


    #[test]
    fn egress_default_allow_stands_alone() {
        // Entitlements removed 2026-09-04: an explicit `egress = "allow"`
        // stands alone — the hostile fixture (pi, no magic word) now merges
        // cleanly with last-wins precedence.
        let hostile = load_fixture("hostile_default_egress", "hostile_default_egress");

        let (merged, provenance) = merge_layers(&[hostile]).expect("allow stands alone");
        let pi = merged.workloads.get("pi").unwrap();
        assert_eq!(
            pi.network.defaults.and_then(|d| d.egress),
            Some(crate::config::DefaultAction::Allow)
        );
        assert_eq!(
            provenance.get("workloads.pi.network.defaults.egress"),
            Some(&"hostile_default_egress".to_string())
        );
    }

    #[test]
    fn egress_default_allow_stands_alone_over_deny_base() {
        // A higher layer relaxing `"deny" -> "allow"` wins outright — no
        // precondition, no monotonic-deny-upward (gone by design).
        let base = load_fixture("base", "base");
        let hostile = load_fixture("hostile_default_egress", "hostile_default_egress");

        let (merged, provenance) = merge_layers(&[base, hostile]).expect("allow wins");
        let pi = merged.workloads.get("pi").unwrap();
        assert_eq!(
            pi.network.defaults.and_then(|d| d.egress),
            Some(crate::config::DefaultAction::Allow),
            "relaxing deny->allow from the winning layer must stick"
        );
        assert_eq!(
            provenance.get("workloads.pi.network.defaults.egress"),
            Some(&"hostile_default_egress".to_string())
        );
    }

    #[test]
    fn workload_relaxes_egress_default_to_allow() -> Result<()> {
        // Headline precedence test: a three-layer stack where base sets
        // example-offensive egress="deny", mid keeps deny, top sets allow.
        // The top layer wins — no entitlement declaration needed.
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.example-offensive]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.example-offensive.network.defaults]\negress = \"deny\"",
        )?;
        let mid = Layer::from_string(
            "mid",
            "schema_version = 1\n\n[workloads.example-offensive]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.example-offensive.network.defaults]\negress = \"deny\"",
        )?;
        let top = Layer::from_string(
            "top",
            "schema_version = 1\n\n[workloads.example-offensive]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.example-offensive.network.defaults]\negress = \"allow\"",
        )?;

        let (merged, provenance) = merge_layers(&[base, mid, top])?;
        let offensive = merged.workloads.get("example-offensive").unwrap();
        assert_eq!(
            offensive.network.defaults.and_then(|d| d.egress),
            Some(crate::config::DefaultAction::Allow),
            "workload should relax deny->allow from the top layer"
        );
        assert_eq!(
            provenance.get("workloads.example-offensive.network.defaults.egress"),
            Some(&"top".to_string()),
            "provenance should attribute the relaxed value to the top layer"
        );
        Ok(())
    }

    #[test]
    fn workload_tightens_egress_default_to_deny() -> Result<()> {
        // Precedence holds both directions: base allow, top deny → deny wins.
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.pi.network.defaults]\negress = \"allow\"",
        )?;
        let top = Layer::from_string(
            "top",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.pi.network.defaults]\negress = \"deny\"",
        )?;

        let (merged, provenance) = merge_layers(&[base, top])?;
        let pi = merged.workloads.get("pi").unwrap();
        assert_eq!(
            pi.network.defaults.and_then(|d| d.egress),
            Some(crate::config::DefaultAction::Deny),
            "tightening allow->deny from the top layer must stick"
        );
        assert_eq!(
            provenance.get("workloads.pi.network.defaults.egress"),
            Some(&"top".to_string())
        );
        Ok(())
    }

    // egress_ceiling test removed (allowlist removed per ADR 0035)


    #[test]
    fn single_layer_parity() -> Result<()> {
        let base = load_fixture("base", "base");
        let direct: ConfigFile = toml::from_str(&std::fs::read_to_string(fixture("base"))?)?;

        let (merged, _) = merge_layers(&[base])?;
        // Policy is collected via ladder, not merged — so direct and merged differ on policy; compare everything else
        let mut direct_no_policy = direct.clone();
        let mut merged_no_policy = merged.clone();
        for wl in direct_no_policy.workloads.values_mut() {
            wl.policy = Default::default();
        }
        direct_no_policy.policy = Default::default();
        for wl in merged_no_policy.workloads.values_mut() {
            wl.policy = Default::default();
        }
        merged_no_policy.policy = Default::default();
        assert_eq!(merged_no_policy, direct_no_policy);
        Ok(())
    }

    /// Extract `(name, literal-value)` pairs from an EnvBindings for
    /// order-sensitive assertions (secret bindings yield `None` values).
    fn env_pairs(wl: &WorkloadConfig) -> Vec<(&str, Option<&str>)> {
        wl.env
            .iter()
            .map(|(k, v)| {
                let value = match v {
                    EnvBinding::Literal(s) => Some(s.as_str()),
                    EnvBinding::Secret(_) => None,
                };
                (k.as_str(), value)
            })
            .collect()
    }

    #[test]
    fn env_union_appends_new_keys() -> Result<()> {
        // WP6(a): base env [A=1,B=2] + override env [C=3] → A=1, B=2, C=3.
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[[workloads.pi.env]]\nname = \"A\"\nvalue = \"1\"\n\n[[workloads.pi.env]]\nname = \"B\"\nvalue = \"2\"\n\n[workloads.pi.network.defaults]\negress = \"deny\"",
        )?;
        let team = Layer::from_string(
            "team",
            "schema_version = 1\n\n[workloads.pi]\n\n[[workloads.pi.env]]\nname = \"C\"\nvalue = \"3\"",
        )?;

        let (merged, provenance) = merge_layers(&[base, team])?;
        let pi = merged.workloads.get("pi").unwrap();
        assert_eq!(
            env_pairs(pi),
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
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[[workloads.pi.env]]\nname = \"A\"\nvalue = \"1\"\n\n[[workloads.pi.env]]\nname = \"B\"\nvalue = \"2\"\n\n[workloads.pi.network.defaults]\negress = \"deny\"",
        )?;
        let team = Layer::from_string(
            "team",
            "schema_version = 1\n\n[workloads.pi]\n\n[[workloads.pi.env]]\nname = \"A\"\nvalue = \"9\"",
        )?;

        let (merged, provenance) = merge_layers(&[base, team])?;
        let pi = merged.workloads.get("pi").unwrap();
        assert_eq!(
            env_pairs(pi),
            vec![("A", Some("9")), ("B", Some("2"))],
            "A should be replaced in place (last layer wins), B preserved, no duplicate A"
        );
        assert_eq!(
            pi.env.iter().filter(|(k, _)| k == "A").count(),
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
    fn single_layer_allow_egress_ok() -> Result<()> {
        // Single layer with explicit `egress = "allow"` merges cleanly.
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.example-offensive]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.example-offensive.network.defaults]\negress = \"allow\"",
        )?;

        let (merged, _) = merge_layers(&[base])?;
        let offensive = merged.workloads.get("example-offensive").unwrap();
        assert_eq!(
            offensive.network.defaults.and_then(|d| d.egress),
            Some(crate::config::DefaultAction::Allow)
        );
        Ok(())
    }

    // ---- ingress default: symmetric mirror of the egress rule ----

    #[test]
    fn ingress_default_allow_stands_alone() {
        // Entitlements removed 2026-09-04: explicit `ingress = "allow"`
        // stands alone (mirrors the egress rule).
        let hostile = load_fixture("hostile_default_ingress", "hostile_default_ingress");

        let (merged, provenance) = merge_layers(&[hostile]).expect("allow stands alone");
        let pi = merged.workloads.get("pi").unwrap();
        assert_eq!(
            pi.network.defaults.and_then(|d| d.ingress),
            Some(crate::config::DefaultAction::Allow)
        );
        assert_eq!(
            provenance.get("workloads.pi.network.defaults.ingress"),
            Some(&"hostile_default_ingress".to_string())
        );
    }

    #[test]
    fn ingress_default_allow_stands_alone_over_deny_base() {
        // A higher layer relaxing `"deny" -> "allow"` on ingress wins
        // outright (mirrors the egress WP3/A4 shape, minus the gate).
        let base = load_fixture("base", "base");
        let hostile = load_fixture("hostile_default_ingress", "hostile_default_ingress");

        let (merged, _) = merge_layers(&[base, hostile]).expect("allow wins");
        let pi = merged.workloads.get("pi").unwrap();
        assert_eq!(
            pi.network.defaults.and_then(|d| d.ingress),
            Some(crate::config::DefaultAction::Allow)
        );
    }

    #[test]
    fn workload_relaxes_ingress_default_to_allow() -> Result<()> {
        // Mirror of the egress headline test: base sets ingress="deny",
        // mid keeps deny, top sets allow — the top layer wins.
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.example-offensive]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.example-offensive.network.defaults]\ningress = \"deny\"",
        )?;
        let mid = Layer::from_string(
            "mid",
            "schema_version = 1\n\n[workloads.example-offensive]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.example-offensive.network.defaults]\ningress = \"deny\"",
        )?;
        let top = Layer::from_string(
            "top",
            "schema_version = 1\n\n[workloads.example-offensive]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.example-offensive.network.defaults]\ningress = \"allow\"",
        )?;

        let (merged, provenance) = merge_layers(&[base, mid, top])?;
        let offensive = merged.workloads.get("example-offensive").unwrap();
        assert_eq!(
            offensive.network.defaults.and_then(|d| d.ingress),
            Some(crate::config::DefaultAction::Allow),
            "workload should relax deny->allow from the top layer"
        );
        assert_eq!(
            provenance.get("workloads.example-offensive.network.defaults.ingress"),
            Some(&"top".to_string()),
            "provenance should attribute the relaxed value to the top layer"
        );
        Ok(())
    }

    #[test]
    fn workload_tightens_ingress_default_to_deny() -> Result<()> {
        // Precedence holds both directions: base allow, top deny → deny wins.
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.pi.network.defaults]\ningress = \"allow\"",
        )?;
        let top = Layer::from_string(
            "top",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.pi.network.defaults]\ningress = \"deny\"",
        )?;

        let (merged, provenance) = merge_layers(&[base, top])?;
        let pi = merged.workloads.get("pi").unwrap();
        assert_eq!(
            pi.network.defaults.and_then(|d| d.ingress),
            Some(crate::config::DefaultAction::Deny),
            "tightening allow->deny from the top layer must stick"
        );
        assert_eq!(
            provenance.get("workloads.pi.network.defaults.ingress"),
            Some(&"top".to_string())
        );
        Ok(())
    }

    #[test]
    fn single_layer_allow_ingress_ok() -> Result<()> {
        // Single layer with explicit `ingress = "allow"` merges cleanly.
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.example-offensive]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.example-offensive.network.defaults]\ningress = \"allow\"",
        )?;

        let (merged, _) = merge_layers(&[base])?;
        let offensive = merged.workloads.get("example-offensive").unwrap();
        assert_eq!(
            offensive.network.defaults.and_then(|d| d.ingress),
            Some(crate::config::DefaultAction::Allow)
        );
        Ok(())
    }

    #[test]
    fn network_defaults_merge_is_field_wise() -> Result<()> {
        // A layer setting ONLY egress must not clobber an existing ingress
        // value (and vice versa): the defaults table merges per field.
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.pi.network.defaults]\ningress = \"deny\"",
        )?;
        let team = Layer::from_string(
            "team",
            "schema_version = 1\n\n[workloads.pi]\n\n[workloads.pi.network.defaults]\negress = \"deny\"",
        )?;

        let (merged, provenance) = merge_layers(&[base, team])?;
        let pi = merged.workloads.get("pi").unwrap();
        let defaults = pi.network.defaults.expect("defaults must survive");
        assert_eq!(defaults.egress, Some(crate::config::DefaultAction::Deny));
        assert_eq!(
            defaults.ingress,
            Some(crate::config::DefaultAction::Deny),
            "team layer setting only egress must preserve base's ingress"
        );
        assert_eq!(
            provenance.get("workloads.pi.network.defaults.egress"),
            Some(&"team".to_string())
        );
        assert_eq!(
            provenance.get("workloads.pi.network.defaults.ingress"),
            Some(&"base".to_string())
        );
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
    /// migration). Serial execution is guaranteed by a shared test-only lock
    /// (crate::config::test_support) so these tests cannot interleave with
    /// OTHER modules' tests that mutate the same process-global stores.
    use crate::config::test_support::PROVENANCE_STORAGE_TEST_LOCK;

    #[test]
    fn provenance_set_from_another_thread_is_visible() {
        let _guard = PROVENANCE_STORAGE_TEST_LOCK.lock().unwrap();
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
        let _guard = PROVENANCE_STORAGE_TEST_LOCK.lock().unwrap();
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

    /// A9 regression: provenance must survive await-points on a tokio
    /// MULTI-THREAD runtime (task migration across worker threads).
    ///
    /// Isolation note: this test holds [`PROVENANCE_STORAGE_TEST_LOCK`],
    /// which serializes the DIRECT mutators of the process-global stores
    /// (the other storage tests here and deps.rs's drain-pin test). Tests
    /// that mutate the slots only INDIRECTLY (any concurrent test calling
    /// `load_config` or `ConfigWorkload::new`) are NOT serialized by it, so
    /// a theoretical interference window remains between
    /// `set_provenance(Some(..))` and `take_provenance()` below; that window
    /// is µs-scale (100 yield_now awaits), matching this test's rare,
    /// load-dependent flake history.
    #[test]
    fn provenance_survives_tokio_multi_thread_migration() {
        let _guard = PROVENANCE_STORAGE_TEST_LOCK.lock().unwrap();
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
        let _guard = PROVENANCE_STORAGE_TEST_LOCK.lock().unwrap();
        let _: fn(Option<Provenance>) = set_provenance;
        let _: fn() -> Option<Provenance> = take_provenance;
        let _: fn(Option<Provenance>) = set_secret_provenance;
        let _: fn() -> Option<Provenance> = get_secret_provenance;
        let _: fn() -> Option<Provenance> = take_secret_provenance;
    }

    // ---- Layer content dirs (spec 17 path resolution plumbing) ----

    /// `Layer::load` captures the source file; `from_string` leaves it None.
    #[test]
    fn layer_source_path_captured_by_load_not_from_string() -> Result<()> {
        let loaded = load_fixture("base", "base");
        assert_eq!(
            loaded.source_path.as_deref(),
            Some(fixture("base").as_path()),
            "Layer::load must record the file it was loaded from"
        );
        let synthetic = Layer::from_string("synthetic", "schema_version = 1\n")?;
        assert!(
            synthetic.source_path.is_none(),
            "from_string layers have no on-disk source"
        );
        Ok(())
    }

    /// The layer-dirs map records each sourced layer's PARENT dir (its
    /// content root); synthetic layers are absent.
    #[test]
    fn layer_dirs_from_maps_layer_names_to_parent_dirs() -> Result<()> {
        let loaded = load_fixture("base", "base");
        let synthetic = Layer::from_string("synthetic", "schema_version = 1\n")?;
        let dirs = layer_dirs_from(&[loaded, synthetic]);
        assert_eq!(
            dirs.get("base").map(|p| p.as_path()),
            fixture("base").parent(),
            "content dir is the layer file's parent directory"
        );
        assert!(
            !dirs.contains_key("synthetic"),
            "synthetic layers contribute no content dir"
        );
        Ok(())
    }

    /// The process-global store round-trips and survives cross-thread sets
    /// (same tokio multi-thread migration hazard as provenance, WP10/A9).
    #[test]
    fn layer_dirs_set_from_another_thread_is_visible() {
        let _guard = PROVENANCE_STORAGE_TEST_LOCK.lock().unwrap();
        set_layer_dirs(None);
        let mut sample = HashMap::new();
        sample.insert(
            "personal#workestrate/workloads/litellm.toml".to_string(),
            PathBuf::from("/tmp/example/workestrate/workloads"),
        );
        std::thread::spawn(move || set_layer_dirs(Some(sample)))
            .join()
            .expect("setter thread panicked");
        let got = get_layer_dirs().expect("layer dirs set on another thread must be visible");
        assert_eq!(
            got.get("personal#workestrate/workloads/litellm.toml"),
            Some(&PathBuf::from("/tmp/example/workestrate/workloads"))
        );
        set_layer_dirs(None); // clean up for other tests
    }
    // ---- Spec 16: final unified secret/env model — merge rules ----

    /// `allowed_hosts` merges presence-gated like the other def scalars: an
    /// upper layer that does not declare it INHERITS the lower layer's list;
    /// declaring it REPLACES wholesale (arrays never partial-merge).
    #[test]
    fn secret_def_allowed_hosts_inherit_and_replace() -> Result<()> {
        let base = || {
            Layer::from_string(
                "base",
                "schema_version = 1\n\n[secrets.A]\nenv_var = \"A\"\nallowed_hosts = [\"example.com\"]\n",
            )
            .expect("base layer must parse")
        };
        let inherit = Layer::from_string(
            "team",
            "schema_version = 1\n\n[secrets.A]\nrequired = false\n",
        )?;
        let replace = Layer::from_string(
            "team",
            "schema_version = 1\n\n[secrets.A]\nallowed_hosts = [\"other.com\", \"third.com\"]\n",
        )?;

        let (merged, provenance) = merge_layers(&[base(), inherit])?;
        assert_eq!(
            merged.secrets["A"].allowed_hosts.as_deref(),
            Some(&["example.com".to_string()][..]),
            "undeclared allowed_hosts is inherited"
        );
        assert_eq!(
            provenance.get("secrets.A.allowed_hosts"),
            Some(&"base".to_string())
        );

        let (merged, provenance) = merge_layers(&[base(), replace])?;
        assert_eq!(
            merged.secrets["A"].allowed_hosts.as_deref(),
            Some(&["other.com".to_string(), "third.com".to_string()][..]),
            "declared allowed_hosts replaces wholesale"
        );
        assert_eq!(
            provenance.get("secrets.A.allowed_hosts"),
            Some(&"team".to_string())
        );
        Ok(())
    }

    /// Explicit `allowed_hosts = []` is NOT a no-op: it CLEARS the inherited
    /// list (the only way an upper layer revokes a lower layer's
    /// substitution grants) and resolves to deny-all — the resolved
    /// SecretDefinition has an EMPTY allowed_hosts.
    #[test]
    fn secret_def_allowed_hosts_explicit_empty_clears_to_deny_all() -> Result<()> {
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[secrets.A]\nenv_var = \"A\"\nallowed_hosts = [\"example.com\"]\n",
        )?;
        let clear = Layer::from_string(
            "team",
            "schema_version = 1\n\n[secrets.A]\nallowed_hosts = []\n",
        )?;

        let (merged, provenance) = merge_layers(&[base, clear])?;
        assert_eq!(
            merged.secrets["A"].allowed_hosts.as_deref(),
            Some(&[][..]),
            "explicit [] clears the inherited list"
        );
        assert_eq!(
            provenance.get("secrets.A.allowed_hosts"),
            Some(&"team".to_string())
        );
        // Post-merge resolution: the cleared list resolves to deny-all
        // (empty allowed_hosts on the SecretDefinition).
        let defs = crate::microsandbox::workload::secrets::build_secret_definitions(&merged)?;
        assert!(
            defs["A"].allowed_hosts.is_empty(),
            "explicit [] resolves to deny-all"
        );
        Ok(())
    }

    /// Env bindings are ATOMIC: a same-key binding replaces the lower
    /// layer's binding wholesale — `X = { secret = "A" }` followed by
    /// `X = "lit"` yields the literal, never a merged hybrid.
    #[test]
    fn env_binding_merge_is_atomic_same_key_replace() -> Result<()> {
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.env]\nX = { secret = \"A\" }\n\n[workloads.pi.network.defaults]\negress = \"deny\"",
        )?;
        let team = Layer::from_string(
            "team",
            "schema_version = 1\n\n[workloads.pi.env]\nX = \"lit\"",
        )?;

        let (merged, _) = merge_layers(&[base, team])?;
        let pi = merged.workloads.get("pi").unwrap();
        assert_eq!(
            pi.env.get("X"),
            Some(&EnvBinding::Literal("lit".to_string())),
            "the upper literal replaces the secret binding wholesale"
        );
        assert_eq!(
            pi.env.iter().filter(|(k, _)| k == "X").count(),
            1,
            "X must not be duplicated"
        );
        Ok(())
    }

    /// Atomicity across the desugar boundary: the upper layer's sugar
    /// (`X = { bound = "guest" }`) resolves against ITS OWN key — the lower
    /// layer's `secret = "A"` is NOT inherited (defaults apply post-merge,
    /// to the final binding).
    #[test]
    fn env_binding_merge_upper_sugar_resolves_against_key_not_lower_secret() -> Result<()> {
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.env]\nX = { secret = \"A\" }\n\n[workloads.pi.network.defaults]\negress = \"deny\"",
        )?;
        let team = Layer::from_string(
            "team",
            "schema_version = 1\n\n[workloads.pi.env]\nX = { bound = \"guest\" }",
        )?;

        let (merged, _) = merge_layers(&[base, team])?;
        let pi = merged.workloads.get("pi").unwrap();
        assert_eq!(
            pi.env.get("X"),
            Some(&EnvBinding::Secret(crate::config::EnvSecretRef {
                secret: "X".to_string(),
                bound: Some(crate::config::Bound::Guest),
            })),
            "the upper sugar binds secret X (key-name default); the lower secret A is gone"
        );
        Ok(())
    }

    /// Defaults are applied ONLY after the full merge: the lower layer's
    /// `required = false` survives an upper layer that sets only
    /// `placeholder` (presence-gated scalar merge), and the resolution
    /// default (`required → true`) applies only when the merged value is
    /// None.
    #[test]
    fn defaults_apply_after_full_merge_not_per_layer() -> Result<()> {
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[secrets.A]\nenv_var = \"A\"\nrequired = false\n",
        )?;
        let team = Layer::from_string(
            "team",
            "schema_version = 1\n\n[secrets.A]\nplaceholder = \"CHANGEME\"\n",
        )?;

        let (merged, _) = merge_layers(&[base, team])?;
        let a = &merged.secrets["A"];
        assert_eq!(
            a.required,
            Some(false),
            "the merged def keeps the lower layer's required = false"
        );
        assert_eq!(a.placeholder.as_deref(), Some("CHANGEME"));

        // Post-merge resolution honors the merged value; the default fires
        // only for a def that never declared required.
        let defs = crate::microsandbox::workload::secrets::build_secret_definitions(&merged)?;
        assert!(!defs["A"].required);

        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[secrets.B]\nenv_var = \"B\"\n",
        )?;
        let (merged, _) = merge_layers(&[base])?;
        let defs = crate::microsandbox::workload::secrets::build_secret_definitions(&merged)?;
        assert!(
            defs["B"].required,
            "required defaults to true only when the merged value is None"
        );
        Ok(())
    }

    // ---- ADR 0026(d): depends_on is union-by-dep-name, last layer wins ----

    #[test]
    fn depends_on_union_last_layer_wins_per_dep() -> Result<()> {
        // Base declares dep A; the override layer re-declares A with a
        // different env and adds B → A is overridden in place, B is appended,
        // and provenance names the override layer for both deps (plus the
        // whole-key entry).
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\nlog_stop_errors = false\n\n[workloads.pi.depends_on.litellm]\nenv = \"LITELLM_URL\"\n\n[workloads.pi.network.defaults]\negress = \"deny\"",
        )?;
        let team = Layer::from_string(
            "team",
            "schema_version = 1\n\n[workloads.pi]\n\n[workloads.pi.depends_on.litellm]\nenv = \"LITELLM_BASE_URL\"\nrequired = true\n\n[workloads.pi.depends_on.db]\nenv = \"DB_URL\"",
        )?;

        let (merged, provenance) = merge_layers(&[base, team])?;
        let pi = merged.workloads.get("pi").unwrap();

        assert_eq!(pi.depends_on.len(), 2, "A overridden + B appended");
        let a = &pi.depends_on["litellm"];
        assert_eq!(
            a.env.as_deref(),
            Some("LITELLM_BASE_URL"),
            "override wins per dep key"
        );
        assert!(a.required);
        let b = &pi.depends_on["db"];
        assert_eq!(b.env.as_deref(), Some("DB_URL"));
        assert!(!b.required, "required defaults to false");

        assert_eq!(
            provenance.get("workloads.pi.depends_on"),
            Some(&"team".to_string())
        );
        assert_eq!(
            provenance.get("workloads.pi.depends_on.litellm"),
            Some(&"team".to_string()),
            "re-declared dep must be re-attributed to the override layer"
        );
        assert_eq!(
            provenance.get("workloads.pi.depends_on.db"),
            Some(&"team".to_string()),
            "new dep provenance names the override layer"
        );
        Ok(())
    }

    /// depends_on maps merge union-by-dependency-name with the whole spec
    /// (INCLUDING on_conflict) replaced per dep — a higher layer declaring the
    /// dep replaces the lower layer's on_conflict; a higher layer that does
    /// not re-declare the dep leaves the lower layer's value untouched.
    #[test]
    fn depends_on_on_conflict_merge_last_layer_wins_per_dep() -> Result<()> {
        // Closure so the same base layer can be merged twice (the array
        // expression `&[base, top]` moves its elements, so a reused layer
        // must be re-created per call — same pattern as the
        // secret_def_allowed_hosts_inherit_and_replace test above).
        let base = || {
            Layer::from_string(
                "base",
                "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\n\n[workloads.pi.depends_on.litellm]\nenv = \"LITELLM_URL\"\non_conflict = \"replace\"",
            )
            .expect("base layer must parse")
        };
        let top = Layer::from_string(
            "top",
            "schema_version = 1\n\n[workloads.pi]\n\n[workloads.pi.depends_on.litellm]\nenv = \"LITELLM_URL\"",
        )?;
        let (merged, _) = merge_layers(&[base(), top])?;
        assert_eq!(
            merged.workloads["pi"].depends_on["litellm"].on_conflict, None,
            "a higher layer re-declaring the dep replaces the whole spec (on_conflict back to default)"
        );

        let top2 = Layer::from_string(
            "top2",
            "schema_version = 1\n\n[workloads.pi]\n\n[workloads.pi.depends_on.litellm]\nenv = \"LITELLM_URL\"\non_conflict = \"fail\"",
        )?;
        let (merged2, _) = merge_layers(&[base(), top2])?;
        assert_eq!(
            merged2.workloads["pi"].depends_on["litellm"].on_conflict,
            Some(crate::config::DepConflict::fail()),
            "a higher layer explicitly setting on_conflict wins"
        );
        Ok(())
    }

    /// ADR 0030 addendum 2 U4 #14: a higher layer re-declaring `on_conflict`
    /// as a SCALAR RESETS a lower layer's LIST — the merge is whole-value
    /// replacement per dep, never append. The scalar normalizes to a
    /// singleton chain at parse time, so the merged chain is exactly
    /// `["reuse"]`.
    #[test]
    fn depends_on_on_conflict_scalar_resets_list_last_layer_wins() -> Result<()> {
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\n\n[workloads.pi.depends_on.litellm]\nenv = \"LITELLM_URL\"\non_conflict = [\"reuse\", \"start\", \"replace\"]",
        )
        .expect("base layer must parse");
        let top = Layer::from_string(
            "top",
            "schema_version = 1\n\n[workloads.pi]\n\n[workloads.pi.depends_on.litellm]\nenv = \"LITELLM_URL\"\non_conflict = \"reuse\"",
        )?;
        let (merged, _) = merge_layers(&[base, top])?;
        assert_eq!(
            merged.workloads["pi"].depends_on["litellm"].on_conflict,
            Some(crate::config::DepConflict::reuse()),
            "a scalar on_conflict in the top layer must RESET the base list to a singleton chain"
        );
        Ok(())
    }

    // ---- ADR 0030 Phase 1: the instance policy block merges as ONE unit ----

    /// A higher layer re-declaring `[workloads.pi.instance]` replaces the
    /// WHOLE block (strategy/on_conflict/port/label reset to defaults),
    /// last layer wins — a higher layer declaring only `port` resets the
    /// base layer's `strategy`.
    #[test]
    fn instance_block_whole_replaces_last_layer_wins() -> Result<()> {
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\n\n[workloads.pi.instance]\nstrategy = \"parallel\"\non_conflict = \"fail\"\nlabel = \"dev\"",
        )?;
        let top = Layer::from_string(
            "top",
            "schema_version = 1\n\n[workloads.pi]\n\n[workloads.pi.instance]\nport = 4000",
        )?;

        let (merged, provenance) = merge_layers(&[base, top])?;
        let instance = &merged.workloads["pi"].instance;
        assert_eq!(
            instance.port,
            Some(crate::config::InstancePort::Strict(4000)),
            "the top layer's port wins"
        );
        assert_eq!(
            instance.strategy,
            crate::config::InstanceStrategy::Singleton,
            "a higher layer declaring only port RESETS the other fields to defaults"
        );
        assert_eq!(instance.on_conflict, None, "on_conflict resets to default");
        assert_eq!(instance.label, None, "label resets to default");
        assert_eq!(
            provenance.get("workloads.pi.instance"),
            Some(&"top".to_string()),
            "whole-block provenance names the last declaring layer"
        );
        Ok(())
    }

    /// A higher layer that does NOT re-declare the instance block leaves the
    /// lower layer's block untouched.
    #[test]
    fn instance_block_absent_preserves_lower_layer() -> Result<()> {
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\n\n[workloads.pi.instance]\nstrategy = \"parallel\"\nlabel = \"dev\"",
        )?;
        let top = Layer::from_string("top", "schema_version = 1\n\n[workloads.pi]\ncommand = []")?;

        let (merged, _) = merge_layers(&[base, top])?;
        let instance = &merged.workloads["pi"].instance;
        assert_eq!(
            instance.strategy,
            crate::config::InstanceStrategy::Parallel,
            "an absent instance block must preserve the lower layer's policy"
        );
        assert_eq!(instance.label.as_deref(), Some("dev"));
        Ok(())
    }

    // ---- ADR 0036: the virtualization ask merges whole-unit ----

    /// A higher layer re-declaring `[workloads.<name>.virtualization]`
    /// replaces the WHOLE table (last layer wins) and the merge provenance
    /// names it — the same whole-unit semantics as the instance block.
    #[test]
    fn virtualization_block_merges_whole_unit_last_layer_wins() -> Result<()> {
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\n\n[workloads.pi.virtualization]\nnested = \"prefer\"",
        )?;
        let top = Layer::from_string(
            "top",
            "schema_version = 1\n\n[workloads.pi]\n\n[workloads.pi.virtualization]\nnested = \"require\"",
        )?;

        let (merged, provenance) = merge_layers(&[base, top])?;
        assert_eq!(
            merged.workloads["pi"].virtualization,
            Some(crate::config::VirtualizationConfig {
                nested: Some(crate::config::NestedMode::Require)
            }),
            "the top layer's ask wins wholesale"
        );
        assert_eq!(
            provenance.get("workloads.pi.virtualization"),
            Some(&"top".to_string()),
            "whole-block provenance names the last declaring layer"
        );
        Ok(())
    }

    /// A higher layer that does NOT re-declare the virtualization table
    /// leaves the lower layer's ask untouched; a config with no ask
    /// anywhere merges to `None` (omitted → Off).
    #[test]
    fn virtualization_block_absent_preserves_lower_layer() -> Result<()> {
        let base = Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\n\n[workloads.pi.virtualization]\nnested = \"require\"",
        )?;
        let top = Layer::from_string("top", "schema_version = 1\n\n[workloads.pi]\ncommand = []")?;

        let (merged, _) = merge_layers(&[base, top])?;
        assert_eq!(
            merged.workloads["pi"].virtualization,
            Some(crate::config::VirtualizationConfig {
                nested: Some(crate::config::NestedMode::Require)
            }),
            "an absent virtualization table must preserve the lower layer's ask"
        );

        let bare = Layer::from_string(
            "bare",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []",
        )?;
        let (merged, _) = merge_layers(&[bare])?;
        assert_eq!(
            merged.workloads["pi"].virtualization, None,
            "no ask anywhere merges to None (omitted → Off)"
        );
        Ok(())
    }
}
