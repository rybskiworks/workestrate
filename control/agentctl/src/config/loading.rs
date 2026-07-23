//! Config loading (`load_config` layering), user-global overrides, secrets
//! layer resolution, and the `agentctl check` scaffold checks.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::config::paths::{
    expand_tilde, overrides_path, reference_config_path, resolve_home_with_kind,
    resolve_store_dir, xdg_config_dir, HomeKind,
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
    matches!(
        value.get("secrets").and_then(|v| v.as_str()),
        Some("none")
    )
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
        // Optional local overrides: 4 agent repos + 4 agent builds + workspaces/ + var/ = 10.
        assert_eq!(
            optional_entries.len(),
            10,
            "expected 10 optional checks, got {}",
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
}
