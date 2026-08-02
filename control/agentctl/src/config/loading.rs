//! Config loading (`load_config` layering), user-global overrides, secrets
//! layer resolution, and the `agentctl check` scaffold checks.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::config::paths::{
    expand_tilde, overrides_path, reference_config_path, resolve_home_with_kind, resolve_store_dir,
    xdg_config_dir, HomeKind,
};
use crate::config::registry::{load_registry, resolve_active_context};
use crate::config::trust::is_trusted_project;
use crate::config::types::{CONFIG_FIELDS, WORKLOAD_FIELDS};
use crate::config::validation::validate_config;
use crate::config::{set_active_context, ConfigFile, SecretsLayer};

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
        // NOTE (cleanup phase 2): personal workflow content (profiles/*.md,
        // profiles/agents/*.md, infra/microsandbox/sdk-notes.md) is no longer
        // a hard runtime gate — the tool repo must not couple `agentctl
        // check` to personal files. The files themselves remain in the repo
        // until a later migration step moves them out.
        // Optional: agent repos are typically supplied via flake
        // inputs. A fresh clone may legitimately omit local
        // `agents/<name>/repo` checkouts. Cleanup phase 4: the tool repo's
        // own devshell builds the synthetic example-* workloads from
        // config.reference; real agent sources live in personal config repo
        // flakes and are not this command's business.
        optional(
            "agents/example-agent/repo",
            root.join("agents/example-agent/repo"),
        ),
        optional(
            "agents/example-offensive/repo",
            root.join("agents/example-offensive/repo"),
        ),
        optional(
            "agents/example-agent/build",
            root.join("agents/example-agent/build"),
        ),
        optional(
            "agents/example-offensive/build",
            root.join("agents/example-offensive/build"),
        ),
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

// NOTE (WP6(e)/C10): the MAIN config structs all carry
// #[serde(deny_unknown_fields)] so unknown fields in a config LAYER hard-error
// at parse time; they live in `config::types` (re-exported above). The
// user-global overrides path stays lenient: `process_override_section` warns
// about AND strips unknown ConfigFile-level / workload-level keys BEFORE the
// fragment is re-parsed via `merge::Layer::from_string`, so override typos
// remain warnings, not errors.

// ---------------------------------------------------------------------------
// User-global overrides
// ---------------------------------------------------------------------------

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
            layers.push(crate::merge::Layer::from_string_with_path(
                "global-override",
                &processed,
                Some(overrides_path.to_path_buf()),
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
                layers.push(crate::merge::Layer::from_string_with_path(
                    &format!("configs.{}-override", name),
                    &processed,
                    Some(overrides_path.to_path_buf()),
                )?);
            }
        }
    }

    Ok(layers)
}

/// Process an override section: check unknown fields, filter unknown workloads,
/// return the processed TOML string.
///
/// WP6(e)/C10: unknown fields are not only warned about but STRIPPED before
/// the fragment is re-serialized into a merge layer. This keeps overrides
/// lenient (warn, not error) now that the main config structs carry
/// `#[serde(deny_unknown_fields)]` — without stripping, the downstream
/// `Layer::from_string` would turn the previously-lenient warning into a
/// hard parse error.
fn process_override_section(
    table: &toml::map::Map<String, toml::Value>,
    section_path: &str,
    existing_workloads: &std::collections::HashSet<String>,
) -> Result<String> {
    // Clone the table for filtering.
    let mut value = toml::Value::Table(table.clone());

    // Warn about AND strip unknown fields at ConfigFile level.
    if let Some(table_mut) = value.as_table_mut() {
        let unknown: Vec<String> = table_mut
            .keys()
            .filter(|k| !CONFIG_FIELDS.contains(&k.as_str()))
            .cloned()
            .collect();
        for key in &unknown {
            eprintln!(
                "WARNING: unknown field '{}' in override section [{}] (probable typo)",
                key, section_path
            );
            table_mut.remove(key);
        }
    }

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

        // Warn about AND strip unknown fields in each remaining workload
        // sub-table.
        for (wl_name, wl_value) in workloads.iter_mut() {
            if let Some(wl_table) = wl_value.as_table_mut() {
                let unknown_fields: Vec<String> = wl_table
                    .keys()
                    .filter(|k| !WORKLOAD_FIELDS.contains(&k.as_str()))
                    .cloned()
                    .collect();
                for key in &unknown_fields {
                    eprintln!(
                        "WARNING: unknown field '{}' in override section [{}.workloads.{}] (probable typo)",
                        key, section_path, wl_name
                    );
                    wl_table.remove(key);
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

/// Load the active `workestrate.toml`.
///
/// Resolution order (lowest to highest precedence):
/// 1. Reference config (`config.reference/workestrate.toml`) — OPT-IN since
///    cleanup phase 2: only included when `WORKESTRATE_REFERENCE_CONFIG=1`
///    (see `reference_config_path()`; the spec-05 cwd gate stays layered on
///    top for cwd-derived roots).
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
            let layer_dirs = crate::merge::layer_dirs_from(std::slice::from_ref(&layer));
            let (merged, provenance) = crate::merge::merge_layers(&[layer])?;
            crate::merge::set_provenance(Some(provenance));
            crate::merge::set_layer_dirs(Some(layer_dirs));
            validate_config(&merged)?;
            return Ok(merged);
        }
    }

    let mut layers: Vec<crate::merge::Layer> = Vec::new();

    // 2. Reference config as the base layer (opt-in: reference_config_path()
    //    returns None unless WORKESTRATE_REFERENCE_CONFIG=1).
    if let Some(path) = reference_config_path() {
        if path.exists() {
            layers.push(crate::merge::Layer::load("reference", &path)?);
        }
    }

    // 3. Resolve active context and load its layers.
    let active_context = resolve_active_context()?;
    set_active_context(Some(active_context.clone()));
    for name in &active_context.layers {
        let repo_dir = resolve_store_dir().join("config-repos").join(name);
        layers.extend(load_config_repo_layers(name, &repo_dir)?);
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

    let layer_dirs = crate::merge::layer_dirs_from(&layers);
    let (merged, provenance) = crate::merge::merge_layers(&layers)?;
    crate::merge::set_provenance(Some(provenance));
    crate::merge::set_layer_dirs(Some(layer_dirs));
    validate_config(&merged)?;
    Ok(merged)
}

// ---------------------------------------------------------------------------
// Config-repo directory mode (spec 17)
// ---------------------------------------------------------------------------

/// Load the layer(s) contributed by one registry config repo.
///
/// Two repo layouts are supported (spec 17):
///
/// - **File mode**: `<repo_dir>/workestrate.toml` — a single layer named
///   `<name>` (the historical layout, unchanged).
/// - **Directory mode**: `<repo_dir>/workestrate/` — a directory of
///   cross-cutting files plus a `workloads/` tree of flat `<name>.toml` files
///   and `<name>/` capsule directories. Each loaded file becomes one
///   pseudo-layer named `<name>#<relpath>` (repo-relative, forward slashes),
///   so per-field provenance and `--show-source` compose with the merge
///   engine unchanged. Load order: `default.toml` → `secrets.toml`
///   (optional) → `workloads/` entries sorted lexicographically by entry
///   file name (optional; flat files and capsule dirs interleaved).
///
/// If BOTH `workestrate.toml` and `workestrate/` exist → hard error naming
/// both paths. If neither exists → no layers (historical skip behavior).
fn load_config_repo_layers(name: &str, repo_dir: &Path) -> Result<Vec<crate::merge::Layer>> {
    let file_path = repo_dir.join("workestrate.toml");
    let dir_path = repo_dir.join("workestrate");
    let has_file = file_path.exists();
    let has_dir = dir_path.is_dir();

    if has_file && has_dir {
        anyhow::bail!(
            "config repo '{}' mixes layout modes: both {} and {} exist; \
             a config repo uses either file mode (workestrate.toml) or \
             directory mode (workestrate/), never both",
            name,
            file_path.display(),
            dir_path.display()
        );
    }
    if has_file {
        return Ok(vec![crate::merge::Layer::load(name, &file_path)?]);
    }
    if !has_dir {
        return Ok(vec![]);
    }

    // Directory mode. Track every workload name → provenance path of the
    // file that first defined it, for cross-file duplicate detection.
    let mut layers = Vec::new();
    let mut defined: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // 1. default.toml — required; the sole authority for schema_version.
    let default_path = dir_path.join("default.toml");
    if !default_path.is_file() {
        anyhow::bail!(
            "config repo '{}' uses directory mode but is missing the required \
             entry file {}",
            name,
            default_path.display()
        );
    }
    let default_prov = format!("{name}#workestrate/default.toml");
    let (default_content, default_raw) = read_toml_file(&default_path)?;
    if default_raw.get("schema_version").is_none() {
        anyhow::bail!(
            "directory-mode config repo '{}' must declare schema_version in {} \
             (the single version authority for directory mode)",
            name,
            default_path.display()
        );
    }
    record_workload_names(&mut defined, &default_raw, &default_prov)?;
    layers.push(crate::merge::Layer::from_string_with_path(
        &default_prov,
        &default_content,
        Some(default_path.clone()),
    )?);

    // 2. secrets.toml — optional; must NOT repeat schema_version.
    let secrets_path = dir_path.join("secrets.toml");
    if secrets_path.is_file() {
        let secrets_prov = format!("{name}#workestrate/secrets.toml");
        let (secrets_content, secrets_raw) = read_toml_file(&secrets_path)?;
        reject_schema_version(&secrets_raw, &secrets_prov)?;
        layers.push(crate::merge::Layer::from_string_with_path(
            &secrets_prov,
            &secrets_content,
            Some(secrets_path.clone()),
        )?);
    }

    // 3. workloads/ — optional; entries in lexicographic order by entry file
    //    name, flat files and capsule dirs interleaved.
    let workloads_dir = dir_path.join("workloads");
    if workloads_dir.is_dir() {
        let mut entries: Vec<(String, PathBuf)> = Vec::new();
        let read_dir = std::fs::read_dir(&workloads_dir)
            .map_err(|e| anyhow::anyhow!("failed to read {}: {}", workloads_dir.display(), e))?;
        for entry in read_dir {
            let entry = entry?;
            entries.push((
                entry.file_name().to_string_lossy().into_owned(),
                entry.path(),
            ));
        }
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        for (file_name, path) in entries {
            layers.push(load_workload_entry(name, &file_name, &path, &mut defined)?);
        }
    }

    Ok(layers)
}

/// Read a TOML file, returning the raw content and its parsed `toml::Value`.
fn read_toml_file(path: &Path) -> Result<(String, toml::Value)> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", path.display(), e))?;
    let raw: toml::Value = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("failed to parse {}: {}", path.display(), e))?;
    Ok((content, raw))
}

/// Directory mode: `schema_version` may appear ONLY in `default.toml`.
fn reject_schema_version(raw: &toml::Value, provenance: &str) -> Result<()> {
    if raw.get("schema_version").is_some() {
        anyhow::bail!(
            "schema_version must be declared only in workestrate/default.toml \
             of a directory-mode config repo; found it in {provenance}"
        );
    }
    Ok(())
}

/// Record the workload names a full-form file defines, rejecting cross-file
/// duplicates (spec 17 §2.4). Within-file duplicates stay the existing TOML
/// parse error, so each name here is unique per file.
fn record_workload_names(
    defined: &mut std::collections::HashMap<String, String>,
    raw: &toml::Value,
    provenance: &str,
) -> Result<()> {
    if let Some(workloads) = raw.get("workloads").and_then(|v| v.as_table()) {
        for wl_name in workloads.keys() {
            if let Some(first) = defined.get(wl_name) {
                anyhow::bail!(
                    "duplicate workload '{wl_name}' defined in both {first} and \
                     {provenance}; each workload name may be defined in only one file"
                );
            }
            defined.insert(wl_name.clone(), provenance.to_string());
        }
    }
    Ok(())
}

/// Load one `workestrate/workloads/` entry as a pseudo-layer.
///
/// Entry forms (spec 17 §2.3):
///
/// - Flat `<name>.toml`: either a bare workload table (workload fields at
///   top level; the workload name is implied from the filename stem) or a
///   full `[workloads.<name>]` table (multi-workload files allowed).
/// - Capsule `<name>/`: entry file `workload.toml` (bare table; name implied
///   from the dirname). All other files in the capsule are opaque artifacts
///   and are ignored entirely.
///
/// Bare tables are wrapped as `{ workloads = { <name> = <table> } }` and
/// re-serialized so the merge engine sees the canonical full form.
fn load_workload_entry(
    repo: &str,
    file_name: &str,
    path: &Path,
    defined: &mut std::collections::HashMap<String, String>,
) -> Result<crate::merge::Layer> {
    let (implied_name, entry_file, rel_path) = if path.is_dir() {
        let entry_file = path.join("workload.toml");
        if !entry_file.is_file() {
            anyhow::bail!(
                "workload capsule {} is missing its required entry file {}",
                path.display(),
                entry_file.display()
            );
        }
        (
            file_name.to_string(),
            entry_file,
            format!("workestrate/workloads/{file_name}/workload.toml"),
        )
    } else if path.is_file() && file_name.ends_with(".toml") {
        let stem = file_name.strip_suffix(".toml").unwrap_or(file_name);
        (
            stem.to_string(),
            path.to_path_buf(),
            format!("workestrate/workloads/{file_name}"),
        )
    } else {
        anyhow::bail!(
            "unsupported entry {} in workestrate/workloads/: entries must be \
             flat <name>.toml files or <name>/ capsule directories",
            path.display()
        );
    };

    let provenance = format!("{repo}#{rel_path}");
    let (content, raw) = read_toml_file(&entry_file)?;
    reject_schema_version(&raw, &provenance)?;

    if raw.get("workloads").is_some() {
        // Full form: [workloads.<name>] table(s) loaded as-is.
        record_workload_names(defined, &raw, &provenance)?;
        crate::merge::Layer::from_string_with_path(&provenance, &content, Some(entry_file))
    } else {
        // Bare form: synthesize the [workloads.<implied>] wrapper.
        let wrapper = bare_wrapper_value(&implied_name, &raw);
        record_workload_names(defined, &wrapper, &provenance)?;
        let wrapped = toml::to_string(&wrapper).map_err(|e| {
            anyhow::anyhow!(
                "failed to synthesize workload wrapper for {}: {}",
                entry_file.display(),
                e
            )
        })?;
        crate::merge::Layer::from_string_with_path(&provenance, &wrapped, Some(entry_file))
    }
}

/// Wrap a bare workload table as `{ workloads = { <name> = <table> } }`.
fn bare_wrapper_value(name: &str, table: &toml::Value) -> toml::Value {
    let mut inner = toml::map::Map::new();
    inner.insert(name.to_string(), table.clone());
    let mut outer = toml::map::Map::new();
    outer.insert("workloads".to_string(), toml::Value::Table(inner));
    toml::Value::Table(outer)
}

/// FN-22 (ADR 0018): detect the `secrets = "none"` opt-out marker in a
/// `WORKESTRATE_CONFIG_DIR` override directory's `workestrate.toml`.
///
/// The env-override branch bypasses registry discovery entirely, so there is
/// no `ConfigRepoEntry.secrets` field to read (unlike branch 3). The only
/// place the "none" signal can live is the override dir's own
/// `workestrate.toml`. The `ConfigFile` schema has NO top-level secrets-mode
/// field — its `[secrets]` table is `HashMap<String, SecretDefConfig>`
/// (secret DEFINITIONS) — so we parse LENIENTLY as `toml::Value` (NOT via
/// `ConfigFile`, which would reject the type mismatch) and check for a
/// top-level `secrets` STRING equal to `"none"`.
///
/// A `[secrets]` TABLE (secret definitions, as in `config.reference`) is a
/// `toml::Value::Table`, not a string, and is correctly NOT treated as the
/// none-marker. Unparseable/missing files, or any other `secrets` shape,
/// default to `false` (secrets active).
fn env_dir_secrets_none(dir: &Path) -> bool {
    let toml_path = dir.join("workestrate.toml");
    let content = match std::fs::read_to_string(&toml_path) {
        Ok(c) => c,
        Err(_) => return false,
    };
    let value: toml::Value = match toml::from_str(&content) {
        Ok(v) => v,
        Err(_) => return false,
    };
    matches!(value.get("secrets").and_then(|v| v.as_str()), Some("none"))
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
                dir: path.clone(),
                secrets_file: ".env.enc".to_string(),
                age_key_file: None,
                // FN-22 (ADR 0018): honor the `secrets = "none"` opt-out in
                // the env-override branch instead of hardcoding `skip: false`.
                skip: env_dir_secrets_none(&path),
            });
            return Ok(layers);
        }
    }

    // 2. Reference config dir (shipped with the tool — no .env.enc expected,
    //    but included so the layer list mirrors load_config()). Opt-in since
    //    cleanup phase 2: reference_config_path() returns None unless
    //    WORKESTRATE_REFERENCE_CONFIG=1.
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
            let dir = resolve_store_dir().join("config-repos").join(name);
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

#[cfg(test)]
pub(crate) mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_in_result
    )]
    use super::*;
    use crate::config::test_support::*;

    // ---- WP6(e)/C10: deny_unknown_fields on main config structs ----

    #[test]
    fn main_layer_rejects_unknown_top_level_field() {
        let toml = format!("{MINIMAL_VALID_TOML}\nbogus_key = 1\n");
        let err = toml::from_str::<ConfigFile>(&toml).unwrap_err().to_string();
        assert!(
            err.contains("unknown field `bogus_key`"),
            "error must name the unknown field: {err}"
        );
    }

    #[test]
    fn main_layer_rejects_unknown_workload_field() {
        let toml = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\nbogus_wl = 1\n\n[workloads.pi.network]\ndefault_deny = true";
        let err = toml::from_str::<ConfigFile>(toml).unwrap_err().to_string();
        assert!(
            err.contains("unknown field `bogus_wl`"),
            "error must name the unknown field: {err}"
        );
    }

    #[test]
    fn main_layer_rejects_unknown_nested_image_field() {
        let toml = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\ncommand = []\n\n[workloads.pi.image]\nrecipe = \"registry\"\nref = \"node:24\"\nbogus_img = 1\n\n[workloads.pi.network]\ndefault_deny = true";
        let err = toml::from_str::<ConfigFile>(toml).unwrap_err().to_string();
        assert!(
            err.contains("unknown field `bogus_img`"),
            "error must name the unknown field: {err}"
        );
    }

    #[test]
    fn main_layer_rejects_unknown_env_entry_field() {
        let toml = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[[workloads.pi.env]]\nname = \"A\"\nvalue = \"1\"\nbogus_env = 1\n\n[workloads.pi.network]\ndefault_deny = true";
        let err = toml::from_str::<ConfigFile>(toml).unwrap_err().to_string();
        assert!(
            err.contains("unknown field `bogus_env`"),
            "error must name the unknown field: {err}"
        );
    }

    #[test]
    fn overrides_with_unknown_fields_stay_lenient() -> Result<()> {
        // Unknown fields in an override section warn and are STRIPPED — the
        // resulting layer still parses (no deny_unknown_fields hard error).
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-ov-lenient-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        let path = write_overrides(
            &tmp,
            "[global]\nbogus_top = 1\n\n[global.workloads.pi]\ncpus = 4\nbogus_wl = 2\n",
        );
        let existing: std::collections::HashSet<String> = ["pi".to_string()].into_iter().collect();
        let layers = load_overrides(&path, &[], &existing)?;
        assert_eq!(
            layers.len(),
            1,
            "override with unknown fields should still produce a layer"
        );
        let pi = layers[0].config.workloads.get("pi").unwrap();
        assert_eq!(pi.cpus, Some(4), "known fields survive stripping");
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
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
        // Optional local overrides: 2 example-* agent repos + 2 builds = 4.
        assert_eq!(
            optional_entries.len(),
            4,
            "expected 4 optional checks, got {}",
            optional_entries.len()
        );
    }

    // --- User-global overrides tests ---

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
        let repo_dir = data_dir.join("config-repos").join("personal");
        std::fs::create_dir_all(&repo_dir)?;
        std::fs::write(repo_dir.join("workestrate.toml"), "schema_version = 1\n")?;

        // Create a dummy .env.local.enc in the XDG config dir
        std::fs::write(config_dir.join(".env.local.enc"), "# dummy")?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg_config = std::env::var("XDG_CONFIG_HOME").ok();
        let old_xdg_data = std::env::var("XDG_DATA_HOME").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_ref = std::env::var("WORKESTRATE_REFERENCE_CONFIG").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var("XDG_CONFIG_HOME", tmp_home.join(".config"));
        std::env::set_var("XDG_DATA_HOME", tmp_home.join(".local").join("share"));
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");
        std::env::remove_var("WORKESTRATE_CONTEXT");
        // Opt into the reference base layer (cleanup phase 2): under cargo
        // test CARGO_MANIFEST_DIR pins the repo root, so the reference layer
        // resolves deterministically.
        std::env::set_var("WORKESTRATE_REFERENCE_CONFIG", "1");

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
        match old_ref {
            Some(v) => std::env::set_var("WORKESTRATE_REFERENCE_CONFIG", v),
            None => std::env::remove_var("WORKESTRATE_REFERENCE_CONFIG"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        // Expected layers: reference (opted in above), personal (context
        // layer), user-global (trusted project is not present because cwd
        // has no workestrate.toml)
        let names: Vec<&str> = layers.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(
            names.first(),
            Some(&"reference"),
            "opted-in reference layer should lead the layer list: {:?}",
            names
        );
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

        let repo_dir = data_dir.join("config-repos").join("personal");
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

    // --- FN-22 (ADR 0018): WORKESTRATE_CONFIG_DIR honors `secrets = "none"` ---

    #[test]
    fn secrets_layers_env_dir_honors_secrets_none() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _guard = EnvGuard::capture(HOME_ENV_KEYS);
        let dir = uniq_dir("fn22-none");
        std::fs::create_dir_all(&dir)?;
        // Top-level `secrets = "none"` STRING — the opt-out marker. This dir
        // carries definitions but no decryptable secrets.
        std::fs::write(
            dir.join("workestrate.toml"),
            "schema_version = 1\nsecrets = \"none\"\n",
        )?;

        std::env::set_var("WORKESTRATE_CONFIG_DIR", &dir);
        let layers = resolve_secrets_layers()?;

        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(layers.len(), 1, "env override yields a single layer");
        assert_eq!(layers[0].name, "local");
        assert!(
            layers[0].skip,
            "secrets = \"none\" env dir must set skip == true (FN-22)"
        );
        Ok(())
    }

    #[test]
    fn secrets_layers_env_dir_defaults_to_file() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _guard = EnvGuard::capture(HOME_ENV_KEYS);
        let dir = uniq_dir("fn22-default");
        std::fs::create_dir_all(&dir)?;
        // Normal config with a [secrets] TABLE (secret definitions) — this
        // must NOT be misread as the none-marker. Mirrors config.reference.
        std::fs::write(
            dir.join("workestrate.toml"),
            "schema_version = 1\n\n[secrets.MY_KEY]\nenv_var = \"MY_KEY\"\nrequired = false\n",
        )?;

        std::env::set_var("WORKESTRATE_CONFIG_DIR", &dir);
        let layers = resolve_secrets_layers()?;

        let _ = std::fs::remove_dir_all(&dir);

        assert_eq!(layers.len(), 1, "env override yields a single layer");
        assert_eq!(layers[0].name, "local");
        assert!(
            !layers[0].skip,
            "a [secrets] TABLE (definitions) must NOT be treated as none-marker; skip must stay false"
        );
        Ok(())
    }

    // --- Spec 17: config-repo directory mode ---

    fn write_repo_file(repo: &Path, rel: &str, content: &str) {
        let path = repo.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&path, content).unwrap();
    }

    /// Bare workload table (no `[workloads.<name>]` wrapper); `cpus`
    /// discriminates workloads in assertions.
    fn bare_workload(cpus: u32) -> String {
        format!(
            "kind = \"agent\"\nimage = {{ recipe = \"registry\", ref = \"node:24\" }}\ncommand = []\ncpus = {cpus}\n\n[network]\ndefault_deny = true\n"
        )
    }

    /// Full `[workloads.<name>]` table form of the same workload.
    fn full_workload(name: &str, cpus: u32) -> String {
        format!(
            "[workloads.{name}]\nkind = \"agent\"\nimage = {{ recipe = \"registry\", ref = \"node:24\" }}\ncommand = []\ncpus = {cpus}\n\n[workloads.{name}.network]\ndefault_deny = true\n"
        )
    }

    const SECRETS_TOML: &str = "[secrets.MY_KEY]\nenv_var = \"MY_KEY\"\nrequired = false\n";

    fn merged_from(repo: &Path) -> Result<ConfigFile> {
        let layers = load_config_repo_layers("personal", repo)?;
        let (merged, _) = crate::merge::merge_layers(&layers)?;
        Ok(merged)
    }

    #[test]
    fn dir_mode_repo_loads_via_directory_mode() -> Result<()> {
        let repo = uniq_dir("spec17-detection");
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 1\n");
        let layers = load_config_repo_layers("personal", &repo)?;
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].name, "personal#workestrate/default.toml");
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn file_mode_repo_loads_unchanged() -> Result<()> {
        let repo = uniq_dir("spec17-filemode");
        write_repo_file(&repo, "workestrate.toml", MINIMAL_VALID_TOML);
        let layers = load_config_repo_layers("personal", &repo)?;
        assert_eq!(layers.len(), 1);
        assert_eq!(
            layers[0].name, "personal",
            "file mode keeps the repo-name layer"
        );
        assert!(layers[0].config.workloads.contains_key("pi"));
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn repo_with_neither_mode_yields_no_layers() -> Result<()> {
        let repo = uniq_dir("spec17-neither");
        std::fs::create_dir_all(&repo)?;
        let layers = load_config_repo_layers("personal", &repo)?;
        assert!(layers.is_empty(), "existing skip behavior is preserved");
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_both_file_and_dir_hard_errors() -> Result<()> {
        let repo = uniq_dir("spec17-both");
        write_repo_file(&repo, "workestrate.toml", MINIMAL_VALID_TOML);
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 1\n");
        let err = load_config_repo_layers("personal", &repo)
            .err()
            .expect("directory-mode load must fail")
            .to_string();
        assert!(
            err.contains(&repo.join("workestrate.toml").display().to_string()),
            "error names the workestrate.toml path: {err}"
        );
        assert!(
            err.contains(&repo.join("workestrate").display().to_string()),
            "error names the workestrate/ path: {err}"
        );
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_missing_default_toml_hard_errors() -> Result<()> {
        let repo = uniq_dir("spec17-no-default");
        write_repo_file(&repo, "workestrate/secrets.toml", SECRETS_TOML);
        let err = load_config_repo_layers("personal", &repo)
            .err()
            .expect("directory-mode load must fail")
            .to_string();
        assert!(
            err.contains(&repo.join("workestrate/default.toml").display().to_string()),
            "error names the missing default.toml path: {err}"
        );
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_bare_flat_file_implies_workload_name() -> Result<()> {
        let repo = uniq_dir("spec17-bare-flat");
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 1\n");
        write_repo_file(
            &repo,
            "workestrate/workloads/litellm.toml",
            &bare_workload(2),
        );
        let layers = load_config_repo_layers("personal", &repo)?;
        assert_eq!(layers.len(), 2);
        assert_eq!(
            layers[1].name,
            "personal#workestrate/workloads/litellm.toml"
        );
        let wl = layers[1]
            .config
            .workloads
            .get("litellm")
            .expect("bare table loads under the filename-implied name");
        assert_eq!(wl.cpus, Some(2));
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_bare_capsule_implies_workload_name() -> Result<()> {
        let repo = uniq_dir("spec17-bare-capsule");
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 1\n");
        write_repo_file(
            &repo,
            "workestrate/workloads/pi/workload.toml",
            &bare_workload(3),
        );
        let layers = load_config_repo_layers("personal", &repo)?;
        assert_eq!(layers.len(), 2);
        assert_eq!(
            layers[1].name,
            "personal#workestrate/workloads/pi/workload.toml"
        );
        let wl = layers[1]
            .config
            .workloads
            .get("pi")
            .expect("bare table loads under the dirname-implied name");
        assert_eq!(wl.cpus, Some(3));
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_full_table_form_loads() -> Result<()> {
        let repo = uniq_dir("spec17-full-form");
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 1\n");
        write_repo_file(
            &repo,
            "workestrate/workloads/alpha.toml",
            &full_workload("alpha", 4),
        );
        let layers = load_config_repo_layers("personal", &repo)?;
        assert_eq!(layers.len(), 2);
        let wl = layers[1]
            .config
            .workloads
            .get("alpha")
            .expect("full [workloads.<name>] form loads");
        assert_eq!(wl.cpus, Some(4));
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_multi_workload_full_form_loads() -> Result<()> {
        let repo = uniq_dir("spec17-multi-full");
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 1\n");
        let multi = format!(
            "{}\n{}",
            full_workload("alpha", 1),
            full_workload("beta", 2)
        );
        write_repo_file(&repo, "workestrate/workloads/multi.toml", &multi);
        let layers = load_config_repo_layers("personal", &repo)?;
        assert_eq!(layers.len(), 2);
        assert!(
            layers[1].config.workloads.contains_key("alpha"),
            "multi-workload full-form file loads the first workload"
        );
        assert!(
            layers[1].config.workloads.contains_key("beta"),
            "multi-workload full-form file loads the second workload"
        );
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_duplicate_flat_vs_capsule_hard_errors() -> Result<()> {
        let repo = uniq_dir("spec17-dup-flat-capsule");
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 1\n");
        write_repo_file(
            &repo,
            "workestrate/workloads/litellm.toml",
            &bare_workload(1),
        );
        write_repo_file(
            &repo,
            "workestrate/workloads/litellm/workload.toml",
            &bare_workload(2),
        );
        let err = load_config_repo_layers("personal", &repo)
            .err()
            .expect("directory-mode load must fail")
            .to_string();
        assert!(
            err.contains("personal#workestrate/workloads/litellm.toml"),
            "error names the flat-file provenance path: {err}"
        );
        assert!(
            err.contains("personal#workestrate/workloads/litellm/workload.toml"),
            "error names the capsule provenance path: {err}"
        );
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_duplicate_with_default_toml_hard_errors() -> Result<()> {
        let repo = uniq_dir("spec17-dup-default");
        let default = format!("schema_version = 1\n\n{}", full_workload("pi", 1));
        write_repo_file(&repo, "workestrate/default.toml", &default);
        write_repo_file(&repo, "workestrate/workloads/pi.toml", &bare_workload(2));
        let err = load_config_repo_layers("personal", &repo)
            .err()
            .expect("directory-mode load must fail")
            .to_string();
        assert!(
            err.contains("personal#workestrate/default.toml"),
            "error names the default.toml provenance path: {err}"
        );
        assert!(
            err.contains("personal#workestrate/workloads/pi.toml"),
            "error names the workload-file provenance path: {err}"
        );
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_layer_ordering_default_secrets_then_lexicographic() -> Result<()> {
        let repo = uniq_dir("spec17-ordering");
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 1\n");
        write_repo_file(&repo, "workestrate/secrets.toml", SECRETS_TOML);
        write_repo_file(&repo, "workestrate/workloads/zeta.toml", &bare_workload(1));
        write_repo_file(
            &repo,
            "workestrate/workloads/alpha/workload.toml",
            &bare_workload(2),
        );
        write_repo_file(
            &repo,
            "workestrate/workloads/mid.toml",
            &full_workload("mid", 3),
        );
        let layers = load_config_repo_layers("personal", &repo)?;
        let names: Vec<&str> = layers.iter().map(|l| l.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "personal#workestrate/default.toml",
                "personal#workestrate/secrets.toml",
                "personal#workestrate/workloads/alpha/workload.toml",
                "personal#workestrate/workloads/mid.toml",
                "personal#workestrate/workloads/zeta.toml",
            ],
            "default first, secrets second, then workload entries sorted lexicographically"
        );
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_schema_version_in_secrets_hard_errors() -> Result<()> {
        let repo = uniq_dir("spec17-sv-secrets");
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 1\n");
        write_repo_file(
            &repo,
            "workestrate/secrets.toml",
            &format!("schema_version = 1\n\n{SECRETS_TOML}"),
        );
        let err = load_config_repo_layers("personal", &repo)
            .err()
            .expect("directory-mode load must fail")
            .to_string();
        assert!(
            err.contains("personal#workestrate/secrets.toml"),
            "error names the offending secrets.toml provenance path: {err}"
        );
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_schema_version_in_bare_workload_hard_errors() -> Result<()> {
        let repo = uniq_dir("spec17-sv-bare");
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 1\n");
        write_repo_file(
            &repo,
            "workestrate/workloads/pi/workload.toml",
            &format!("schema_version = 1\n\n{}", bare_workload(1)),
        );
        let err = load_config_repo_layers("personal", &repo)
            .err()
            .expect("directory-mode load must fail")
            .to_string();
        assert!(
            err.contains("personal#workestrate/workloads/pi/workload.toml"),
            "error names the offending bare workload file provenance path: {err}"
        );
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_schema_version_in_full_workload_hard_errors() -> Result<()> {
        let repo = uniq_dir("spec17-sv-full");
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 1\n");
        write_repo_file(
            &repo,
            "workestrate/workloads/alpha.toml",
            &format!("schema_version = 1\n\n{}", full_workload("alpha", 1)),
        );
        let err = load_config_repo_layers("personal", &repo)
            .err()
            .expect("directory-mode load must fail")
            .to_string();
        assert!(
            err.contains("personal#workestrate/workloads/alpha.toml"),
            "error names the offending full-form workload file provenance path: {err}"
        );
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_schema_version_absent_from_default_hard_errors() -> Result<()> {
        let repo = uniq_dir("spec17-sv-absent");
        write_repo_file(
            &repo,
            "workestrate/default.toml",
            "# no schema_version here\n",
        );
        let err = load_config_repo_layers("personal", &repo)
            .err()
            .expect("directory-mode load must fail")
            .to_string();
        assert!(
            err.contains(&repo.join("workestrate/default.toml").display().to_string()),
            "error names the default.toml path: {err}"
        );
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_schema_version_2_surfaces_via_validate_config() -> Result<()> {
        // The loader does not duplicate the schema_version=2 rejection; it
        // must surface through the normal validate_config path.
        let repo = uniq_dir("spec17-sv2");
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 2\n");
        let merged = merged_from(&repo)?;
        let err = validate_config(&merged).unwrap_err().to_string();
        assert!(
            err.contains("schema_version 2 is not supported"),
            "schema_version = 2 surfaces through validate_config: {err}"
        );
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_provenance_strings_carry_relpath() -> Result<()> {
        let repo = uniq_dir("spec17-provenance");
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 1\n");
        write_repo_file(
            &repo,
            "workestrate/workloads/litellm/workload.toml",
            &bare_workload(2),
        );
        write_repo_file(
            &repo,
            "workestrate/workloads/tempest.toml",
            &full_workload("tempest", 4),
        );
        let layers = load_config_repo_layers("personal", &repo)?;
        let (_merged, provenance) = crate::merge::merge_layers(&layers)?;
        assert_eq!(
            provenance.get("schema_version").map(String::as_str),
            Some("personal#workestrate/default.toml")
        );
        assert_eq!(
            provenance.get("workloads.litellm.kind").map(String::as_str),
            Some("personal#workestrate/workloads/litellm/workload.toml")
        );
        assert_eq!(
            provenance.get("workloads.tempest.kind").map(String::as_str),
            Some("personal#workestrate/workloads/tempest.toml")
        );
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_capsule_artifacts_are_ignored() -> Result<()> {
        let repo_plain = uniq_dir("spec17-capsule-plain");
        let repo_extra = uniq_dir("spec17-capsule-extra");
        for repo in [&repo_plain, &repo_extra] {
            write_repo_file(repo, "workestrate/default.toml", "schema_version = 1\n");
            write_repo_file(
                repo,
                "workestrate/workloads/litellm/workload.toml",
                &bare_workload(2),
            );
        }
        // Opaque app-native artifacts colocated in the capsule: the loader
        // must ignore them entirely.
        write_repo_file(
            &repo_extra,
            "workestrate/workloads/litellm/config.yaml",
            "model_list: []\n",
        );
        write_repo_file(
            &repo_extra,
            "workestrate/workloads/litellm/models.yaml",
            "models: []\n",
        );
        write_repo_file(
            &repo_extra,
            "workestrate/workloads/litellm/flake.nix",
            "{}\n",
        );
        write_repo_file(
            &repo_extra,
            "workestrate/workloads/litellm/seed.sql",
            "SELECT 1;\n",
        );

        let merged_plain = merged_from(&repo_plain)?;
        let merged_extra = merged_from(&repo_extra)?;
        assert_eq!(
            merged_plain, merged_extra,
            "capsule artifacts do not change the merged config"
        );
        let _ = std::fs::remove_dir_all(&repo_plain);
        let _ = std::fs::remove_dir_all(&repo_extra);
        Ok(())
    }

    #[test]
    fn dir_mode_equivalent_to_single_file() -> Result<()> {
        let dir_repo = uniq_dir("spec17-equiv-dir");
        write_repo_file(
            &dir_repo,
            "workestrate/default.toml",
            "schema_version = 1\n",
        );
        write_repo_file(&dir_repo, "workestrate/secrets.toml", SECRETS_TOML);
        write_repo_file(
            &dir_repo,
            "workestrate/workloads/litellm/workload.toml",
            &bare_workload(2),
        );
        write_repo_file(
            &dir_repo,
            "workestrate/workloads/pi/workload.toml",
            &bare_workload(3),
        );
        write_repo_file(
            &dir_repo,
            "workestrate/workloads/tempest.toml",
            &full_workload("tempest", 4),
        );
        let dir_merged = merged_from(&dir_repo)?;

        let single = format!(
            "schema_version = 1\n\n{}\n{}\n{}\n{}",
            SECRETS_TOML,
            full_workload("litellm", 2),
            full_workload("pi", 3),
            full_workload("tempest", 4)
        );
        let file_repo = uniq_dir("spec17-equiv-file");
        write_repo_file(&file_repo, "workestrate.toml", &single);
        let file_merged = merged_from(&file_repo)?;

        assert_eq!(
            dir_merged, file_merged,
            "directory-mode repo and its single-file equivalent merge to the same ConfigFile"
        );
        let _ = std::fs::remove_dir_all(&dir_repo);
        let _ = std::fs::remove_dir_all(&file_repo);
        Ok(())
    }

    #[test]
    fn dir_mode_unsupported_workloads_entry_hard_errors() -> Result<()> {
        let repo = uniq_dir("spec17-bad-entry");
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 1\n");
        write_repo_file(&repo, "workestrate/workloads/README.md", "# notes\n");
        let err = load_config_repo_layers("personal", &repo)
            .err()
            .expect("directory-mode load must fail")
            .to_string();
        assert!(
            err.contains(
                &repo
                    .join("workestrate/workloads/README.md")
                    .display()
                    .to_string()
            ),
            "error names the unsupported entry: {err}"
        );
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn dir_mode_capsule_missing_workload_toml_hard_errors() -> Result<()> {
        let repo = uniq_dir("spec17-capsule-no-entry");
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 1\n");
        write_repo_file(
            &repo,
            "workestrate/workloads/litellm/config.yaml",
            "model_list: []\n",
        );
        let err = load_config_repo_layers("personal", &repo)
            .err()
            .expect("directory-mode load must fail")
            .to_string();
        assert!(
            err.contains(
                &repo
                    .join("workestrate/workloads/litellm")
                    .display()
                    .to_string()
            ),
            "error names the capsule dir: {err}"
        );
        assert!(
            err.contains("workload.toml"),
            "error names the missing entry file: {err}"
        );
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }
}
