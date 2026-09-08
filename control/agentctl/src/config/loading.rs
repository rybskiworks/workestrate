//! Config loading (`load_config` layering), user-global overrides, secrets
//! layer resolution, and the `agentctl check` scaffold checks.
//!
//! ## A5 Session 2: pinned consumption (ADR 0032 addendum §Config source model)
//!
//! WHERE a config layer's files physically live depends on the registry
//! entry's [`crate::config::ConfigSourceKind`] (see [`layer_content_root`]):
//!
//! - **PlainPath** entries are consumed CONTENT-AS-IS from
//!   [`crate::config::local_entry_checkout_dir`] (falling back to the
//!   historical store path) with branch `"local"` — the documented explicit
//!   exception: commit-before-consume does NOT apply, edits are visible on
//!   the next load, and no archive/lock entry is ever produced for them.
//! - **Remote / GitFile** entries are consumed REF-PINNED from the
//!   content-addressed archive store (`<state>/cache/gitv3/<sha>/`, see
//!   `config::archive`), NOT from the managed clone's working tree. The
//!   consumed rev resolves (in order): the lock entry for
//!   `(name, effective ref)` → the registry's recorded `rev` →
//!   FIRST-RESOLUTION-WITH-NOTICE (resolve the effective ref in the clone,
//!   write the lock entry, print a stderr notice). The first two are SILENT
//!   (already pinned); only the third writes, and never silently.
//!
//! ## A5 Session 3a: the `--config-ref` override (ADR 0032 addendum §Selection ladder)
//!
//! When `WORKESTRATE_CONFIG_REF` is set (the global `--config-ref
//! <branch|sha>` flag), every Remote/GitFile entry resolves at THAT ref —
//! overriding its own pinned/default ref — via
//! [`config_ref_layer_content_root`]: the refs-map lock entry for
//! `(name, <ref>)` (SILENT, write-free) wins; otherwise the ref is
//! rev-parsed in the managed clone (FAIL-CLOSED naming repo+ref when it
//! does not resolve), archived, and locked into `refs[<ref>]` with the
//! first-resolution stderr notice. The entry's PRIMARY pin is never moved
//! by an override resolution. PlainPath entries are UNAFFECTED by
//! `--config-ref` — content-as-is, no archive, no lock write.
//!
//! BEHAVIOR CHANGE (vs. pre-A5): edits committed in the managed clone are
//! INVISIBLE to consumption until `workestrate config update` moves the pin
//! (commit-before-consume). Everything path-shaped is UNCHANGED (spec 17
//! §3b): provenance strings stay `<repo>#<relpath>` and `layer_dirs` point
//! at the content root — only the physical location moves to the archive
//! dir for pinned entries.
//!
//! ## A5 Session 3b: the per-workload inline `name:ref` override
//! (ADR 0032 addendum §Selection ladder rung 3)
//!
//! When the process-global inline override is ARMED (see
//! [`crate::config::inline_ref`] for the two-phase pending/armed model),
//! [`load_config`] substitutes the named workload's declaration at the
//! override ref from its DECLARING repo's pinned archive —
//! [`apply_inline_override_substitution`], applied after the home-scoped
//! merge and before validation. Deps NEVER follow the override in v1: the
//! pending-but-not-armed load (dependency auto-start's view) is
//! byte-identical to no override.

use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::config::paths::{
    HomeKind, expand_tilde, overrides_path, reference_config_path, resolve_home_with_kind,
    resolve_store_dir, xdg_config_dir,
};
use crate::config::registry::{load_registry, resolve_active_context};
use crate::config::trust::is_trusted_project;
use crate::config::types::{CONFIG_FIELDS, WORKLOAD_FIELDS};
use crate::config::validation::validate_config;
use crate::config::{ConfigFile, ConfigRepoEntry, Registry, SecretsLayer, set_active_context};

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
            // A5 Session 3b: a single dev layer has no declaring config
            // repo, so an inline ref override can never substitute into it —
            // fail closed rather than silently ignore the override.
            if crate::config::inline_ref::armed_inline_override().is_some() {
                anyhow::bail!(
                    "inline ref overrides (name:ref) are not supported with \
                     WORKESTRATE_CONFIG_DIR (a single dev layer has no declaring \
                     config repo)"
                );
            }
            set_active_context(None);
            let layer = crate::merge::Layer::load("local", &path)?;
            let collected = collect_policy_scopes(None, std::slice::from_ref(&layer))?;
            let ladder = collect_secret_policy_ladder(None, std::slice::from_ref(&layer));
            let network_ladder = collect_network_policy_ladder(None, std::slice::from_ref(&layer));
            let virt_ladder = collect_virtualization_ladder(None, std::slice::from_ref(&layer));
            let ssh_ladder = collect_ssh_policy_ladder(None, std::slice::from_ref(&layer));
            let layer_dirs = crate::merge::layer_dirs_from(std::slice::from_ref(&layer));
            let (merged, provenance) = crate::merge::merge_layers(&[layer])?;
            crate::mount_policy::set_collected_policy(Some(collected));
            crate::merge::set_secret_policy_ladder(Some(ladder));
            crate::merge::set_network_policy_ladder(Some(network_ladder));
            crate::merge::set_virtualization_ladder(Some(virt_ladder));
            crate::merge::set_ssh_policy_ladder(Some(ssh_ladder));
            crate::merge::set_provenance(Some(provenance));
            crate::merge::set_layer_dirs(Some(layer_dirs));
            validate_config(&merged)?;
            return Ok(merged);
        }
    }

    let mut layers: Vec<crate::merge::Layer> = Vec::new();
    let registry = load_registry()?;

    // 2. Reference config as the base layer (opt-in: reference_config_path()
    //    returns None unless WORKESTRATE_REFERENCE_CONFIG=1).
    if let Some(path) = reference_config_path()
        && path.exists()
    {
        layers.push(crate::merge::Layer::load("reference", &path)?);
    }

    // 3. Resolve active context and load its layers. Each layer's CONTENT
    //    ROOT comes from `layer_content_root` (A5 Session 2: pinned archive
    //    consumption for Remote/GitFile entries, content-as-is for
    //    PlainPath; see the module doc).
    let active_context = resolve_active_context()?;
    set_active_context(Some(active_context.clone()));
    for name in &active_context.layers {
        let content_root = match registry.as_ref() {
            Some(reg) => layer_content_root(name, reg)?,
            // No registry → no configs map to classify against; keep the
            // historical store path (unreachable in practice: a registry-less
            // resolve_active_context yields no layers).
            None => resolve_store_dir().join("config-repos").join(name),
        };
        layers.extend(load_config_repo_layers(name, &content_root)?);
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
        let cwd = crate::config::invoke_cwd_or_err()?;
        let project_path = cwd.join("workestrate.toml");
        if project_path.exists() {
            match load_registry()? {
                Some(_) => {
                    if is_trusted_project(&cwd) {
                        layers.push(crate::merge::Layer::load("project", &project_path)?);
                    } else {
                        eprintln!(
                            "project config ./workestrate.toml found but not trusted; run 'workestrate config trust <dir>' to trust it"
                        );
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
        let local_cwd = crate::config::invoke_cwd_or_err()?;
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

    let mut layer_dirs = crate::merge::layer_dirs_from(&layers);
    let (mut merged, mut provenance) = crate::merge::merge_layers(&layers)?;
    // A5 Session 3b (ADR 0032 addendum §Selection ladder rung 3): the ARMED
    // inline `name:ref` override substitutes the named workload's
    // declaration at `<ref>` from its declaring repo's pinned archive —
    // AFTER the home-scoped merge (deps never follow the override), BEFORE
    // validation (the post-substitution config is what validate_config and
    // the dependent's ConfigWorkload see). PENDING-but-not-armed (dep
    // auto-start's view) leaves the merged config byte-identical to no
    // override.
    let substituted_layer =
        if let Some((workload, config_ref)) = crate::config::inline_ref::armed_inline_override() {
            apply_inline_override_substitution(
                &mut merged,
                &mut provenance,
                &mut layer_dirs,
                registry.as_ref(),
                &workload,
                &config_ref,
            )?
            .map(|layer| (workload, layer))
        } else {
            None
        };
    let mut collected = collect_policy_scopes(registry.as_ref(), &layers)?;
    let mut ladder = collect_secret_policy_ladder(registry.as_ref(), &layers);
    let mut network_ladder = collect_network_policy_ladder(registry.as_ref(), &layers);
    let mut virt_ladder = collect_virtualization_ladder(registry.as_ref(), &layers);
    let mut ssh_ladder = collect_ssh_policy_ladder(registry.as_ref(), &layers);
    if let Some((workload, layer)) = substituted_layer {
        // The policy collection must reflect the substitution: the home
        // collection above saw the PRE-substitution declaration, which
        // would silently drop the ref's `policy.mounts` fragment while
        // validate_config sees the substituted one. The substituted
        // declaration is the whole workload declaration at the ref, so its
        // policy is authoritative for this workload.
        replace_workload_scopes_from_layer(&mut collected, &workload, &layer);
        replace_workload_secret_rungs_from_layer(&mut ladder, &workload, &layer);
        replace_workload_network_rungs_from_layer(&mut network_ladder, &workload, &layer);
        replace_workload_virtualization_rungs_from_layer(&mut virt_ladder, &workload, &layer);
        replace_workload_ssh_rungs_from_layer(&mut ssh_ladder, &workload, &layer);
    }
    crate::mount_policy::set_collected_policy(Some(collected));
    crate::merge::set_secret_policy_ladder(Some(ladder));
    crate::merge::set_network_policy_ladder(Some(network_ladder));
    crate::merge::set_virtualization_ladder(Some(virt_ladder));
    crate::merge::set_ssh_policy_ladder(Some(ssh_ladder));
    crate::merge::set_provenance(Some(provenance));
    crate::merge::set_layer_dirs(Some(layer_dirs));
    validate_config(&merged)?;
    Ok(merged)
}

/// Apply the ARMED inline `name:ref` override (A5 Session 3b): replace
/// `merged.workloads[workload]` with the workload's declaration at
/// `config_ref` from its DECLARING repo's pinned archive, and repoint the
/// declaring layer's content root at the archive (so repo-relative mount /
/// seed_file resolution — the F1 machinery — resolves against the ref's
/// content). Everything else stays home-scoped (or `--config-ref`-scoped:
/// the inline ref wins over `--config-ref` for THIS workload's declaring
/// repo only; every other layer was already loaded by the caller).
///
/// Fail-closed rules:
///
/// - Workload ABSENT from the merged config: leave everything untouched —
///   the existing "workload not found in config" error fires downstream,
///   unmasked.
/// - Declaring repo not determinable from the merge provenance (e.g. the
///   workload is declared by the trusted-project layer, not a registry
///   config repo): hard error.
/// - Declaring repo is a PlainPath entry: hard error (content-as-is has no
///   refs — inline overrides require a git-backed repo).
/// - The ref does not resolve in the managed clone, or the workload does
///   not exist at that ref: hard errors naming repo+ref / workload+ref+repo.
///
/// Returns the substituted layer loaded from the archive (`Some`) so the
/// caller can re-collect the workload's policy scopes from the REF's
/// declaration; `None` when no substitution happened (workload absent at
/// home scope).
fn apply_inline_override_substitution(
    merged: &mut ConfigFile,
    provenance: &mut crate::merge::Provenance,
    layer_dirs: &mut std::collections::HashMap<String, PathBuf>,
    registry: Option<&Registry>,
    workload: &str,
    config_ref: &str,
) -> Result<Option<crate::merge::Layer>> {
    use crate::config::registry::ConfigSourceKind;

    // Unknown workload at HOME scope: do not mask today's existing error.
    if !merged.workloads.contains_key(workload) {
        return Ok(None);
    }

    // The declaring repo = the repo component of the provenance path that
    // declared the workload (`<repo>#<relpath>` for directory-mode
    // pseudo-layers, `<repo>` for single-file registry layers). Prefer the
    // `kind` field's provenance; fall back to the lexicographically-first
    // `workloads.<name>.*` key for determinism.
    let prefix = format!("workloads.{workload}.");
    let declaring_layer = provenance
        .get(&format!("workloads.{workload}.kind"))
        .or_else(|| provenance.keys().filter(|k| k.starts_with(&prefix)).min())
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "cannot determine the declaring config repo of workload '{workload}' \
                 (no merge provenance recorded); inline ref overrides require a \
                 registered config repo"
            )
        })?;
    let repo = declaring_layer
        .split('#')
        .next()
        .unwrap_or(&declaring_layer)
        .to_string();
    let registry = registry.ok_or_else(|| {
        anyhow::anyhow!(
            "cannot determine the declaring config repo of workload '{workload}': no home \
             registry; inline ref overrides require a git-backed registry config repo"
        )
    })?;
    let entry = registry.configs.get(&repo).ok_or_else(|| {
        anyhow::anyhow!(
            "cannot determine the declaring config repo of workload '{workload}': \
             provenance layer '{declaring_layer}' is not a registered config repo; \
             inline ref overrides require a git-backed registry config repo"
        )
    })?;
    if crate::config::source_kind(&entry.url) == ConfigSourceKind::PlainPath {
        anyhow::bail!(
            "inline ref overrides require a git-backed config repo; '{repo}' is a \
             local path source (url '{}'; content-as-is has no refs)",
            entry.url
        );
    }
    let clone = resolve_store_dir().join("config-repos").join(&repo);
    if !clone.join(".git").exists() {
        anyhow::bail!(
            "config repo '{}' (url '{}') has no managed clone at {}; \
             provision it with `workestrate config add` or `workestrate home clone`",
            repo,
            entry.url,
            clone.display()
        );
    }
    let archive = refs_map_locked_archive(
        &repo,
        entry,
        registry,
        &clone,
        config_ref,
        "inline override",
    )?;

    // Load ONLY the named workload at the ref (capsule-only substitution):
    // probe the two spec-17 §2.3 directory-mode forms first — the capsule
    // <archive>/workestrate/workloads/<name>/workload.toml, then the flat
    // file <archive>/workestrate/workloads/<name>.toml (both parse via the
    // existing entry loader, bare or full form) — else fall back to the
    // single-file mode <archive>/workestrate.toml's [workloads.<name>].
    // Fail-closed "does not exist at ref" fires only when NONE of the three
    // forms carries the workload (A5 review MEDIUM-2: the flat file is a
    // first-class directory-mode form and must substitute at a ref).
    let workloads_root = archive.join("workestrate").join("workloads");
    let capsule_dir = workloads_root.join(workload);
    let flat_name = format!("{workload}.toml");
    let flat_file = workloads_root.join(&flat_name);
    let layer = if capsule_dir.join("workload.toml").is_file() {
        load_workload_entry(
            &repo,
            workload,
            &capsule_dir,
            &mut std::collections::HashMap::new(),
        )?
    } else if flat_file.is_file() {
        load_workload_entry(
            &repo,
            &flat_name,
            &flat_file,
            &mut std::collections::HashMap::new(),
        )?
    } else {
        let file = archive.join("workestrate.toml");
        if !file.is_file() {
            anyhow::bail!(
                "workload '{workload}' does not exist at ref '{config_ref}' in repo '{repo}'"
            );
        }
        crate::merge::Layer::load(&repo, &file)?
    };
    let Some(substituted) = layer.config.workloads.get(workload).cloned() else {
        anyhow::bail!(
            "workload '{workload}' does not exist at ref '{config_ref}' in repo '{repo}'"
        );
    };

    // Per-field provenance for the substituted declaration (a one-layer
    // merge reproduces exactly the provenance keys the layer declares).
    let (_sub_cfg, sub_provenance) = crate::merge::merge_layers(std::slice::from_ref(&layer))?;

    merged.workloads.insert(workload.to_string(), substituted);
    let exact = format!("workloads.{workload}");
    provenance.retain(|k, _| *k != exact && !k.starts_with(&prefix));
    for (key, layer_name) in sub_provenance {
        if key == exact || key.starts_with(&prefix) {
            provenance.insert(key, layer_name);
        }
    }
    // The declaring layer's content root moves to the archive dir so F1
    // repo-relative resolution resolves against the ref's content. Only the
    // substituted layer's keys are replaced — other layers of the same repo
    // stay home-scoped.
    layer_dirs.extend(crate::merge::layer_dirs_from(std::slice::from_ref(&layer)));
    Ok(Some(layer))
}

/// Re-collect ONE workload's policy scopes from a substituted layer,
/// replacing whatever the home-scoped [`collect_policy_scopes`] pass
/// recorded for it (inline-override policy consistency, 2026-08-28). The
/// substituted declaration IS the whole workload declaration at the ref,
/// so its policy is authoritative: a ref declaration carrying NO policy
/// fragment REMOVES the home-collected scopes for that workload. Scope
/// construction mirrors collect_policy_scopes exactly (workload fragment →
/// [`crate::mount_policy::ScopeKind::Workload`], mount policies →
/// `ScopeKind::MountEntry`; the archive layer names a config repo, so a
/// layer-level `[policy.mounts]` would be `ScopeKind::ConfigRepoLayer` —
/// layer-global scopes are NOT re-collected here: the substitution is
/// capsule-only and global scopes stay home-scoped).
fn replace_workload_scopes_from_layer(
    collected: &mut crate::mount_policy::CollectedPolicy,
    workload: &str,
    layer: &crate::merge::Layer,
) {
    use crate::mount_policy::{PolicyScope, ScopeKind};
    let source = layer
        .source_path
        .clone()
        .unwrap_or_else(|| PathBuf::from(&layer.name));
    let mut scopes = Vec::new();
    if let Some(decl) = layer.config.workloads.get(workload) {
        if let Some(fragment) = decl.policy.mounts.clone() {
            scopes.push(PolicyScope::new(
                ScopeKind::Workload,
                layer.name.clone(),
                source.clone(),
                fragment,
            ));
        }
        // Mount rows are wholesale-replaced; only a layer that DECLARES
        // this workload's `mounts` contributes entry policies.
        let declares_mounts = layer
            .raw()
            .get("workloads")
            .and_then(|v| v.get(workload))
            .and_then(|v| v.as_table())
            .is_some_and(|t| t.contains_key("mounts"));
        if declares_mounts {
            for mount in &decl.mounts {
                if let Some(fragment) = mount.policy.clone() {
                    scopes.push(
                        PolicyScope::new(
                            ScopeKind::MountEntry,
                            layer.name.clone(),
                            source.clone(),
                            fragment,
                        )
                        .for_mount(mount.guest.clone()),
                    );
                }
            }
        }
    }
    if scopes.is_empty() {
        collected.workloads.remove(workload);
    } else {
        collected.workloads.insert(workload.to_string(), scopes);
    }
}

/// Re-collect ONE workload's secret-policy rungs from a substituted layer,
/// replacing whatever the home-scoped [`collect_secret_policy_ladder`] pass
/// recorded for it (the same inline-override consistency rule as
/// [`replace_workload_scopes_from_layer`]): the substituted declaration IS
/// the whole workload declaration at the ref, so its `[policy.secrets]` is
/// authoritative — a ref declaration carrying NO fragment REMOVES the
/// home-collected rungs for that workload. Capsule-only: layer-global
/// `[policy.secrets]` rungs stay home-scoped.
fn replace_workload_secret_rungs_from_layer(
    ladder: &mut crate::merge::SecretPolicyLadder,
    workload: &str,
    layer: &crate::merge::Layer,
) {
    let rung = layer
        .config
        .workloads
        .get(workload)
        .and_then(|decl| decl.policy.secrets.clone())
        .map(|fragment| (layer.name.clone(), fragment));
    match rung {
        Some(rung) => {
            ladder.workloads.insert(workload.to_string(), vec![rung]);
        }
        None => {
            ladder.workloads.remove(workload);
        }
    }
}

/// Collect policy fragments in the loader's actual order. This deliberately
/// reads each layer independently; no policy field is passed through
/// `merge_layers`.
fn collect_policy_scopes(
    registry: Option<&crate::config::Registry>,
    layers: &[crate::merge::Layer],
) -> Result<crate::mount_policy::CollectedPolicy> {
    use crate::mount_policy::{CollectedPolicy, PolicyScope, ScopeKind};
    let mut collected = CollectedPolicy::default();
    if let Some(registry) = registry
        && let Some(fragment) = registry.policy.mounts.clone()
    {
        collected.global.push(PolicyScope::new(
            ScopeKind::HomeRegistry,
            "home-registry",
            crate::config::registry_path(),
            fragment,
        ));
    }
    for layer in layers {
        let source = layer
            .source_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(&layer.name));
        let kind = if layer.name == "reference" {
            ScopeKind::ReferenceConfig
        } else if layer.name.ends_with("-override") {
            ScopeKind::UserGlobalOverrides
        } else {
            ScopeKind::ConfigRepoLayer
        };
        if let Some(fragment) = layer.config.policy.mounts.clone() {
            collected.global.push(PolicyScope::new(
                kind,
                layer.name.clone(),
                source.clone(),
                fragment,
            ));
        }
        for (name, workload) in &layer.config.workloads {
            if let Some(fragment) = workload.policy.mounts.clone() {
                collected
                    .workloads
                    .entry(name.clone())
                    .or_default()
                    .push(PolicyScope::new(
                        ScopeKind::Workload,
                        layer.name.clone(),
                        source.clone(),
                        fragment,
                    ));
            }
        }
        // Mount rows are wholesale-replaced. Only the final layer that
        // declares this workload's `mounts` contributes entry policies.
        for (name, workload) in &layer.config.workloads {
            let declares_mounts = layer
                .raw()
                .get("workloads")
                .and_then(|v| v.get(name))
                .and_then(|v| v.as_table())
                .is_some_and(|t| t.contains_key("mounts"));
            if declares_mounts {
                let entries = collected.workloads.entry(name.clone()).or_default();
                entries.retain(|scope| scope.scope_kind != ScopeKind::MountEntry);
                for mount in &workload.mounts {
                    if let Some(fragment) = mount.policy.clone() {
                        entries.push(
                            PolicyScope::new(
                                ScopeKind::MountEntry,
                                layer.name.clone(),
                                source.clone(),
                                fragment,
                            )
                            .for_mount(mount.guest.clone()),
                        );
                    }
                }
            }
        }
    }
    Ok(collected)
}

/// Collect the secret violation-policy ladder rungs in the loader's actual
/// order — the secrets edition of [`collect_policy_scopes`]: fragments are
/// collected per scope, never merged (no policy field passes through
/// `merge_layers`), and the resolution walks them authority-ascending.
/// Rung 2 is the home registry's `[policy.secrets]`; rung 3 is each layer's
/// `[policy.secrets]` in stack order; rung 4 is each workload's
/// `[workloads.<name>.policy.secrets]` in stack order (a bare
/// directory-mode capsule's top-level `[policy.secrets]` lands there via
/// the workload wrapper). Origins are the home-registry scope label and the
/// declaring layer's name — the labels the resolution provenance records.
fn collect_secret_policy_ladder(
    registry: Option<&Registry>,
    layers: &[crate::merge::Layer],
) -> crate::merge::SecretPolicyLadder {
    let mut ladder = crate::merge::SecretPolicyLadder::default();
    if let Some(fragment) = registry.and_then(|r| r.policy.secrets.clone()) {
        ladder.home = Some(("home-registry".to_string(), fragment));
    }
    for layer in layers {
        if let Some(fragment) = layer.config.policy.secrets.clone() {
            ladder.layers.push((layer.name.clone(), fragment));
        }
        for (name, workload) in &layer.config.workloads {
            if let Some(fragment) = workload.policy.secrets.clone() {
                ladder
                    .workloads
                    .entry(name.clone())
                    .or_default()
                    .push((layer.name.clone(), fragment));
            }
        }
    }
    ladder
}

/// Collect the network policy ladder rungs in the loader's actual order —
/// the network edition of [`collect_secret_policy_ladder`]: fragments are
/// collected per scope, never merged, and the resolution walks them
/// authority-ascending. Rung 1 is home-registry, rung 2 is config layers in
/// stack order, rung 3 is workload capsules per workload name.
fn collect_network_policy_ladder(
    registry: Option<&Registry>,
    layers: &[crate::merge::Layer],
) -> crate::merge::NetworkPolicyLadder {
    let mut ladder = crate::merge::NetworkPolicyLadder::default();
    if let Some(registry) = registry {
        if let Some(fragment) = registry.policy.egress.clone() {
            ladder.egress_home = Some(("home-registry".to_string(), fragment));
        }
        if let Some(fragment) = registry.policy.ingress.clone() {
            ladder.ingress_home = Some(("home-registry".to_string(), fragment));
        }
        if let Some(fragment) = registry.policy.idna.clone() {
            ladder.idna_home = Some(("home-registry".to_string(), fragment));
        }
    }
    for layer in layers {
        if let Some(fragment) = layer.config.policy.egress.clone() {
            ladder.egress_layers.push((layer.name.clone(), fragment));
        }
        if let Some(fragment) = layer.config.policy.ingress.clone() {
            ladder.ingress_layers.push((layer.name.clone(), fragment));
        }
        if let Some(fragment) = layer.config.policy.idna.clone() {
            ladder.idna_layers.push((layer.name.clone(), fragment));
        }
        for (name, workload) in &layer.config.workloads {
            if let Some(fragment) = workload.policy.egress.clone() {
                ladder
                    .egress_workloads
                    .entry(name.clone())
                    .or_default()
                    .push((layer.name.clone(), fragment));
            }
            if let Some(fragment) = workload.policy.ingress.clone() {
                ladder
                    .ingress_workloads
                    .entry(name.clone())
                    .or_default()
                    .push((layer.name.clone(), fragment));
            }
            if let Some(fragment) = workload.policy.idna.clone() {
                ladder
                    .idna_workloads
                    .entry(name.clone())
                    .or_default()
                    .push((layer.name.clone(), fragment));
            }
        }
    }
    ladder
}

/// Re-collect ONE workload's network policy rungs from a substituted layer,
/// replacing whatever the home-scoped [`collect_network_policy_ladder`] pass
/// recorded for it. Capsule-only: layer-global rungs stay home-scoped.
fn replace_workload_network_rungs_from_layer(
    ladder: &mut crate::merge::NetworkPolicyLadder,
    workload: &str,
    layer: &crate::merge::Layer,
) {
    if let Some(fragment) = layer
        .config
        .workloads
        .get(workload)
        .and_then(|decl| decl.policy.egress.clone())
    {
        ladder
            .egress_workloads
            .insert(workload.to_string(), vec![(layer.name.clone(), fragment)]);
    } else {
        ladder.egress_workloads.remove(workload);
    }
    if let Some(fragment) = layer
        .config
        .workloads
        .get(workload)
        .and_then(|decl| decl.policy.ingress.clone())
    {
        ladder
            .ingress_workloads
            .insert(workload.to_string(), vec![(layer.name.clone(), fragment)]);
    } else {
        ladder.ingress_workloads.remove(workload);
    }
    if let Some(fragment) = layer
        .config
        .workloads
        .get(workload)
        .and_then(|decl| decl.policy.idna.clone())
    {
        ladder
            .idna_workloads
            .insert(workload.to_string(), vec![(layer.name.clone(), fragment)]);
    } else {
        ladder.idna_workloads.remove(workload);
    }
}

/// Collect the nested-virtualization seal ladder rungs in the loader's
/// actual order (ADR 0036 §5) — the virtualization edition of
/// [`collect_secret_policy_ladder`]: `[policy.virtualization]` fragments are
/// collected per scope, never merged, and the resolution walks them
/// authority-ascending. Rung 1 is the home registry's fragment (operator
/// seal); rung 2 is each layer's fragment in stack order; rung 3 is each
/// workload's `[workloads.<name>.policy.virtualization]` in stack order (a
/// bare directory-mode capsule's top-level `[policy.virtualization]` lands
/// there via the workload wrapper). The workload's
/// `[workloads.<name>.virtualization]` ASK is not a fragment — it merges
/// whole-unit in `merge_layers` and is resolved against this ladder by
/// `crate::microsandbox::nested::resolve_for_workload`.
fn collect_virtualization_ladder(
    registry: Option<&Registry>,
    layers: &[crate::merge::Layer],
) -> crate::merge::VirtualizationLadder {
    let mut ladder = crate::merge::VirtualizationLadder::default();
    if let Some(fragment) = registry.and_then(|r| r.policy.virtualization.clone()) {
        ladder.home = Some(("home-registry".to_string(), fragment));
    }
    for layer in layers {
        if let Some(fragment) = layer.config.policy.virtualization.clone() {
            ladder.layers.push((layer.name.clone(), fragment));
        }
        for (name, workload) in &layer.config.workloads {
            if let Some(fragment) = workload.policy.virtualization.clone() {
                ladder
                    .workloads
                    .entry(name.clone())
                    .or_default()
                    .push((layer.name.clone(), fragment));
            }
        }
    }
    ladder
}

/// Re-collect ONE workload's virtualization seal rungs from a substituted
/// layer, replacing whatever the home-scoped
/// [`collect_virtualization_ladder`] pass recorded for it (the same
/// inline-override consistency rule as
/// [`replace_workload_secret_rungs_from_layer`]): the substituted
/// declaration IS the whole workload declaration at the ref, so its
/// `[policy.virtualization]` is authoritative — a ref declaration carrying
/// NO fragment REMOVES the home-collected rungs for that workload.
/// Capsule-only: layer-global `[policy.virtualization]` rungs stay
/// home-scoped.
fn replace_workload_virtualization_rungs_from_layer(
    ladder: &mut crate::merge::VirtualizationLadder,
    workload: &str,
    layer: &crate::merge::Layer,
) {
    let rung = layer
        .config
        .workloads
        .get(workload)
        .and_then(|decl| decl.policy.virtualization.clone())
        .map(|fragment| (layer.name.clone(), fragment));
    match rung {
        Some(rung) => {
            ladder.workloads.insert(workload.to_string(), vec![rung]);
        }
        None => {
            ladder.workloads.remove(workload);
        }
    }
}

/// Collect the SSH confinement-policy ladder rungs in the loader's actual
/// order — the SSH edition of
/// [`collect_secret_policy_ladder`]: `[policy.ssh]` fragments are collected
/// per scope, never merged, and the resolution walks them
/// authority-ascending. Rung 1 is the home registry's fragment (operator
/// scope); rung 2 is each layer's fragment in stack order; rung 3 is each
/// workload's `[workloads.<name>.policy.ssh]` in stack order (a bare
/// directory-mode capsule's top-level `[policy.ssh]` lands there via the
/// workload wrapper).
fn collect_ssh_policy_ladder(
    registry: Option<&Registry>,
    layers: &[crate::merge::Layer],
) -> crate::merge::SshPolicyLadder {
    let mut ladder = crate::merge::SshPolicyLadder::default();
    if let Some(fragment) = registry.and_then(|r| r.policy.ssh.clone()) {
        ladder.home = Some(("home-registry".to_string(), fragment));
    }
    for layer in layers {
        if let Some(fragment) = layer.config.policy.ssh.clone() {
            ladder.layers.push((layer.name.clone(), fragment));
        }
        for (name, workload) in &layer.config.workloads {
            if let Some(fragment) = workload.policy.ssh.clone() {
                ladder
                    .workloads
                    .entry(name.clone())
                    .or_default()
                    .push((layer.name.clone(), fragment));
            }
        }
    }
    ladder
}

/// Re-collect ONE workload's SSH confinement rungs from a substituted layer,
/// replacing whatever the home-scoped [`collect_ssh_policy_ladder`] pass
/// recorded for it (the same inline-override consistency rule as
/// [`replace_workload_secret_rungs_from_layer`]): the substituted
/// declaration IS the whole workload declaration at the ref, so its
/// `[policy.ssh]` is authoritative — a ref declaration carrying NO fragment
/// REMOVES the home-collected rungs for that workload. Capsule-only:
/// layer-global `[policy.ssh]` rungs stay home-scoped.
fn replace_workload_ssh_rungs_from_layer(
    ladder: &mut crate::merge::SshPolicyLadder,
    workload: &str,
    layer: &crate::merge::Layer,
) {
    let rung = layer
        .config
        .workloads
        .get(workload)
        .and_then(|decl| decl.policy.ssh.clone())
        .map(|fragment| (layer.name.clone(), fragment));
    match rung {
        Some(rung) => {
            ladder.workloads.insert(workload.to_string(), vec![rung]);
        }
        None => {
            ladder.workloads.remove(workload);
        }
    }
}

// ---------------------------------------------------------------------------
// Config-repo directory mode (spec 17)
// ---------------------------------------------------------------------------

/// Resolve the CONTENT ROOT of one config layer (A5 Session 2; ADR 0032
/// addendum §Config source model, spec 17 §3b). This is THE one resolver
/// shared by `load_config` (layer files) and `resolve_secrets_layers`
/// (`.env.enc` rides the same root — it is committed encrypted content).
///
/// - Entry absent from the registry (a context naming an unregistered
///   layer): the historical store path, unchanged (a missing dir yields no
///   layers, as before).
/// - [`ConfigSourceKind::PlainPath`]: [`local_entry_checkout_dir`] when the
///   entry has the local-path shape, else the historical store path —
///   content-as-is, branch `"local"`, commit-before-consume does NOT apply.
///   PlainPath entries are UNAFFECTED by `--config-ref` (A5 Session 3a):
///   a local path is consumed as-is regardless of the override.
/// - [`ConfigSourceKind::Remote`] / [`ConfigSourceKind::GitFile`]:
///   REF-PINNED archive consumption — see [`pinned_layer_content_root`]
///   (and, under `--config-ref`, [`config_ref_layer_content_root`]).
pub(crate) fn layer_content_root(name: &str, registry: &Registry) -> Result<PathBuf> {
    use crate::config::registry::ConfigSourceKind;
    let store_path = || resolve_store_dir().join("config-repos").join(name);
    let Some(entry) = registry.configs.get(name) else {
        return Ok(store_path());
    };
    match crate::config::source_kind(&entry.url) {
        ConfigSourceKind::PlainPath => {
            Ok(crate::config::local_entry_checkout_dir(entry).unwrap_or_else(store_path))
        }
        ConfigSourceKind::Remote | ConfigSourceKind::GitFile => {
            pinned_layer_content_root(name, entry, registry)
        }
    }
}

/// Ref-pinned content root of a Remote/GitFile layer: the content-addressed
/// archive of the consumed rev, produced from the EXISTING managed clone
/// (NO worktrees, NO checkouts — the clone is the object database).
///
/// Rev resolution precedence (LOCK-NEVER-SILENT discipline):
///
/// 1. The lock entry for `(name, effective ref)`: a `[repos.<name>.refs
///    .<ref>]` pin, or the primary pin when the lock's recorded `ref`
///    matches the registry entry's current `ref` (a changed `ref` invalidates
///    the primary pin). SILENT — already pinned.
/// 2. The registry entry's recorded `rev` (the pin `config update` wrote
///    back). SILENT — already pinned.
/// 3. FIRST-RESOLUTION-WITH-NOTICE: resolve the effective ref in the clone
///    (`git_rev_parse_ref`), produce the archive, WRITE the lock entry
///    `{rev = sha, sha, fetched_at = now}` and print a stderr notice. This
///    is the ONLY lock write a consumption verb may perform.
fn pinned_layer_content_root(
    name: &str,
    entry: &ConfigRepoEntry,
    registry: &Registry,
) -> Result<PathBuf> {
    let clone = resolve_store_dir().join("config-repos").join(name);
    if !clone.join(".git").exists() {
        anyhow::bail!(
            "config repo '{}' (url '{}') has no managed clone at {}; \
             provision it with `workestrate config add` or `workestrate home clone`",
            name,
            entry.url,
            clone.display()
        );
    }

    // A5 Session 3a (ADR 0032 addendum §Selection ladder): the --config-ref
    // rung — when WORKESTRATE_CONFIG_REF is set, THIS ref overrides the
    // entry's own pinned/default ref entirely.
    if let Ok(config_ref) = std::env::var("WORKESTRATE_CONFIG_REF")
        && !config_ref.is_empty()
    {
        return config_ref_layer_content_root(name, entry, registry, &clone, &config_ref);
    }

    let effective_ref = crate::config::effective_ref(name, entry, &clone)?;
    let lock = crate::config::load_home_lock()?;

    // (1) Lock pin for (name, effective ref) — silent.
    if let Some(locked) = lock.as_ref().and_then(|l| l.repos.get(name)) {
        if let Some(pin) = locked.refs.get(&effective_ref) {
            return crate::config::ensure_archive(&clone, &pin.sha);
        }
        if locked.r#ref == entry.r#ref
            && let Some(sha) = locked.sha.as_deref().or(locked.rev.as_deref())
        {
            return crate::config::ensure_archive(&clone, sha);
        }
    }

    // (2) Registry-recorded rev (written back by `config update`) — silent.
    //     The rev is consumed as a CONTENT ADDRESS: validate before hitting
    //     the archive store so a symbolic value (a branch/tag name recorded
    //     where a sha belongs) errors CLEARLY instead of surfacing as a bare
    //     archive-path rejection.
    if let Some(rev) = entry.rev.as_deref() {
        if !crate::config::archive::is_hex_sha(rev) {
            anyhow::bail!(
                "config repo '{name}' records rev '{rev}', which is not a lowercase hex sha \
                 (7..=40 chars): a symbolic ref (branch/tag) is not a content address; \
                 re-pin with `workestrate config update {name}`",
            );
        }
        return crate::config::ensure_archive(&clone, rev);
    }

    // (3) First resolution WITH notice: resolve, archive, write the lock.
    let sha = crate::git::git_rev_parse_ref(&clone, &effective_ref)?;
    let archive = crate::config::ensure_archive(&clone, &sha)?;
    let mut lock = lock.unwrap_or_else(|| crate::config::HomeLock {
        version: crate::config::LOCK_VERSION,
        home_version: registry.settings.home_version.unwrap_or(2),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        repos: std::collections::BTreeMap::new(),
    });
    lock.version = crate::config::LOCK_VERSION;
    lock.tool_version = env!("CARGO_PKG_VERSION").to_string();
    crate::config::upsert_locked_pin(&mut lock, name, &entry.url, entry.r#ref.as_deref(), &sha);
    crate::config::save_home_lock(&lock)?;
    eprintln!(
        "locked {}@{} → {} (first resolution); `workestrate config update` moves pins explicitly",
        name,
        effective_ref,
        crate::git::short_rev(&sha)
    );
    Ok(archive)
}

/// Ref-pinned content root under `--config-ref` (A5 Session 3a; ADR 0032
/// addendum §Selection ladder rung 2): the content-addressed archive of
/// `config_ref` resolved in the managed clone — overriding the entry's own
/// pinned/default ref. Same LOCK-NEVER-SILENT discipline as
/// [`pinned_layer_content_root`], keyed into the entry's REFS MAP:
///
/// 1. The lock's `refs[config_ref]` pin for `(name, config_ref)`: SILENT
///    and write-free — already locked. (The PRIMARY pin and the registry's
///    recorded `rev` are deliberately NOT consulted: the override replaces
///    the entry's ref selection wholesale.)
/// 2. FIRST-RESOLUTION-WITH-NOTICE: rev-parse `config_ref` in the clone —
///    FAIL-CLOSED naming repo+ref when the ref does not resolve — produce
///    the archive, WRITE `refs[config_ref] = {rev = config_ref, sha,
///    fetched_at}` via [`crate::config::upsert_locked_ref`] (the primary
///    pin is NOT moved), and print the stderr notice. A sha-shaped
///    `config_ref` is legal and is keyed by itself in the refs map.
fn config_ref_layer_content_root(
    name: &str,
    entry: &ConfigRepoEntry,
    registry: &Registry,
    clone: &Path,
    config_ref: &str,
) -> Result<PathBuf> {
    refs_map_locked_archive(name, entry, registry, clone, config_ref, "--config-ref")
}

/// Shared refs-map resolution behind [`config_ref_layer_content_root`]
/// (the `--config-ref` rung) and the A5 Session 3b inline `name:ref`
/// override substitution: resolve `ref_` for repo `name` to the
/// content-addressed archive of its sha, keyed into the entry's REFS MAP.
///
/// 1. The lock's `refs[ref_]` pin for `(name, ref_)`: SILENT and write-free
///    — already locked. (The PRIMARY pin and the registry's recorded `rev`
///    are deliberately NOT consulted: an override replaces the entry's ref
///    selection wholesale.)
/// 2. FIRST-RESOLUTION-WITH-NOTICE: rev-parse `ref_` in the clone —
///    FAIL-CLOSED naming repo+ref when it does not resolve — produce the
///    archive, WRITE `refs[ref_] = {rev = ref_, sha, fetched_at}` via
///    [`crate::config::upsert_locked_ref`] (the primary pin is NOT moved),
///    and print the stderr notice. A sha-shaped ref is legal and is keyed
///    by itself in the refs map.
///
/// `via` labels the error/notice with the consuming rung (`"--config-ref"`
/// or `"inline override"`).
fn refs_map_locked_archive(
    name: &str,
    entry: &ConfigRepoEntry,
    registry: &Registry,
    clone: &Path,
    config_ref: &str,
    via: &str,
) -> Result<PathBuf> {
    let lock = crate::config::load_home_lock()?;

    // (1) Locked refs pin — silent.
    if let Some(pin) = lock
        .as_ref()
        .and_then(|l| l.repos.get(name))
        .and_then(|repo| repo.refs.get(config_ref))
    {
        return crate::config::ensure_archive(clone, &pin.sha);
    }

    // (2) First resolution WITH notice.
    let sha = crate::git::git_rev_parse_ref(clone, config_ref).map_err(|e| {
        anyhow::anyhow!(
            "config repo '{}' (url '{}') does not resolve ref '{}' ({}): {}",
            name,
            entry.url,
            config_ref,
            via,
            e
        )
    })?;
    let archive = crate::config::ensure_archive(clone, &sha)?;
    let mut lock = lock.unwrap_or_else(|| crate::config::HomeLock {
        version: crate::config::LOCK_VERSION,
        home_version: registry.settings.home_version.unwrap_or(2),
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        repos: std::collections::BTreeMap::new(),
    });
    lock.version = crate::config::LOCK_VERSION;
    lock.tool_version = env!("CARGO_PKG_VERSION").to_string();
    crate::config::upsert_locked_ref(
        &mut lock,
        name,
        &entry.url,
        entry.r#ref.as_deref(),
        config_ref,
        &sha,
    );
    crate::config::save_home_lock(&lock)?;
    eprintln!(
        "locked {}@{} → {} (first resolution); `workestrate config update` moves pins explicitly",
        name,
        config_ref,
        crate::git::short_rev(&sha)
    );
    Ok(archive)
}

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
///
/// `pub(crate)` for spec 21 phase C: the `workload build --repo/--all-repos`
/// selectors load one registered repo's OWN layers through this (the repo's
/// declarations, not the active-context merge).
pub(crate) fn load_config_repo_layers(
    name: &str,
    repo_dir: &Path,
) -> Result<Vec<crate::merge::Layer>> {
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
    if let Some(path) = reference_config_path()
        && let Some(parent) = path.parent()
    {
        layers.push(SecretsLayer {
            name: "reference".to_string(),
            dir: parent.to_path_buf(),
            secrets_file: ".env.enc".to_string(),
            age_key_file: None,
            skip: false,
        });
    }

    // 3. Context layers in declared order, each with its own .env.enc. The
    //    secrets dir is the SAME content root as config consumption (A5
    //    Session 2): the pinned archive dir for Remote/GitFile entries
    //    (.env.enc is committed encrypted content and rides the archive like
    //    any other file), the plain-path dir unchanged otherwise.
    let registry = load_registry()?;
    let active_context = resolve_active_context()?;
    if let Some(registry) = registry {
        for name in &active_context.layers {
            let dir = layer_content_root(name, &registry)?;
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
        let cwd = crate::config::invoke_cwd_or_err()?;
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
        unsafe_code,
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
        let toml = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\nbogus_wl = 1\n\n[workloads.pi.network.defaults]\negress = \"deny\"";
        let err = toml::from_str::<ConfigFile>(toml).unwrap_err().to_string();
        assert!(
            err.contains("unknown field `bogus_wl`"),
            "error must name the unknown field: {err}"
        );
    }

    #[test]
    fn main_layer_rejects_unknown_nested_image_field() {
        let toml = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\ncommand = []\n\n[workloads.pi.image]\nrecipe = \"registry\"\nref = \"node:24\"\nbogus_img = 1\n\n[workloads.pi.network.defaults]\negress = \"deny\"";
        let err = toml::from_str::<ConfigFile>(toml).unwrap_err().to_string();
        assert!(
            err.contains("unknown field `bogus_img`"),
            "error must name the unknown field: {err}"
        );
    }

    #[test]
    fn main_layer_rejects_unknown_env_entry_field() {
        let toml = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[[workloads.pi.env]]\nname = \"A\"\nvalue = \"1\"\nbogus_env = 1\n\n[workloads.pi.network.defaults]\negress = \"deny\"";
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
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.network.defaults]\negress = \"deny\"",
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
    fn load_overrides_egress_allow_stands_alone() -> Result<()> {
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-ov-policy-dd-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        // Override sets egress="allow" on pi. Entitlements removed
        // 2026-09-04: explicit allow stands alone, so the override wins by
        // precedence (home `final` seals still veto via the policy ladder).
        let path = write_overrides(
            &tmp,
            "[global.workloads.pi.network.defaults]\negress = \"allow\"\n",
        );
        let existing: std::collections::HashSet<String> = ["pi".to_string()].into_iter().collect();
        let layers = load_overrides(&path, &[], &existing)?;
        assert_eq!(layers.len(), 1);

        // Base layer has pi with egress="deny".
        let base = crate::merge::Layer::from_string(
            "base",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.network.defaults]\negress = \"deny\"",
        )?;
        let mut all = vec![base];
        all.extend(layers);
        let (merged, _) = crate::merge::merge_layers(&all).expect("override allow stands alone");
        let pi = merged.workloads.get("pi").unwrap();
        assert_eq!(
            pi.network.defaults.and_then(|d| d.egress),
            Some(crate::config::DefaultAction::Allow),
            "override egress=\"allow\" wins by precedence (relaxed default is plan-NOTE visible)"
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
        // Override adds an old network.egress recipe — now hard-errors with ADR citation (ADR 0035).
        let path = write_overrides(
            &tmp,
            "[[global.workloads.pi.network.egress]]\nrecipe = \"https\"\nhosts = [\"evil.com\"]\n",
        );
        let existing: std::collections::HashSet<String> = ["pi".to_string()].into_iter().collect();
        let result = load_overrides(&path, &[], &existing);
        assert!(
            result.is_err(),
            "override with old recipe syntax should hard-fail with ADR citation (removed syntax)"
        );
        let err = result.err().unwrap().to_string();
        assert!(
            err.contains("network.egress was removed"),
            "error should mention removed network.egress: {err}"
        );
        assert!(
            err.contains("see ADR 0035"),
            "error should cite ADR 0035: {err}"
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

        // A5 Session 2: a Remote entry is consumed REF-PINNED from the
        // archive store, so the store clone must be a real git repo and the
        // registry carries the pinned rev (case (ii): silent consumption).
        let repo_dir = data_dir.join("config-repos").join("personal");
        init_git_repo(&repo_dir, &[("workestrate.toml", "schema_version = 1\n")]);
        let pinned_rev = crate::git::git_rev_parse(&repo_dir)?;

        // Registry with a context and a config repo
        std::fs::write(
            config_dir.join("config.toml"),
            format!(
                "[settings]\ndefault_context = \"personal\"\n\n[contexts.personal]\nlayers = [\"personal\"]\n\n[configs.personal]\nurl = \"git@example.com:personal.git\"\nref = \"main\"\nrev = \"{pinned_rev}\"\n"
            ),
        )?;

        // Create a dummy .env.local.enc in the XDG config dir
        std::fs::write(config_dir.join(".env.local.enc"), "# dummy")?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg_config = std::env::var("XDG_CONFIG_HOME").ok();
        let old_xdg_data = std::env::var("XDG_DATA_HOME").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_ref = std::env::var("WORKESTRATE_REFERENCE_CONFIG").ok();

        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", &tmp_home) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("XDG_CONFIG_HOME", tmp_home.join(".config")) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("XDG_DATA_HOME", tmp_home.join(".local").join("share")) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("WORKESTRATE_CONFIG_DIR") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("WORKESTRATE_CONTEXT") };
        // Opt into the reference base layer (cleanup phase 2): under cargo
        // test CARGO_MANIFEST_DIR pins the repo root, so the reference layer
        // resolves deterministically.
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_REFERENCE_CONFIG", "1") };

        let layers = resolve_secrets_layers()?;

        // Restore env
        match old_home {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("HOME") },
        }
        match old_xdg_config {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("XDG_CONFIG_HOME", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
        }
        match old_xdg_data {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("XDG_DATA_HOME", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("XDG_DATA_HOME") },
        }
        match old_config_dir {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("WORKESTRATE_CONFIG_DIR", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("WORKESTRATE_CONFIG_DIR") },
        }
        match old_ctx {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("WORKESTRATE_CONTEXT", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("WORKESTRATE_CONTEXT") },
        }
        match old_ref {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("WORKESTRATE_REFERENCE_CONFIG", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("WORKESTRATE_REFERENCE_CONFIG") },
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

        // A5 Session 2: a Remote entry is consumed REF-PINNED from the
        // archive store (see the sibling test above) — real git clone dir +
        // registry-recorded rev pin.
        let repo_dir = data_dir.join("config-repos").join("personal");
        init_git_repo(&repo_dir, &[("workestrate.toml", "schema_version = 1\n")]);
        let pinned_rev = crate::git::git_rev_parse(&repo_dir)?;

        // Registry with a context
        std::fs::write(
            config_dir.join("config.toml"),
            format!(
                "[settings]\ndefault_context = \"personal\"\n\n[contexts.personal]\nlayers = [\"personal\"]\n\n[configs.personal]\nurl = \"git@example.com:personal.git\"\nref = \"main\"\nrev = \"{pinned_rev}\"\n"
            ),
        )?;

        // NO .env.local.enc — should still include the layer (decrypt handles missing file)
        let old_home = std::env::var("HOME").ok();
        let old_xdg_config = std::env::var("XDG_CONFIG_HOME").ok();
        let old_xdg_data = std::env::var("XDG_DATA_HOME").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();

        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", &tmp_home) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("XDG_CONFIG_HOME", tmp_home.join(".config")) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("XDG_DATA_HOME", tmp_home.join(".local").join("share")) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("WORKESTRATE_CONFIG_DIR") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("WORKESTRATE_CONTEXT") };

        let layers = resolve_secrets_layers()?;

        // Restore env
        match old_home {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("HOME", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("HOME") },
        }
        match old_xdg_config {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("XDG_CONFIG_HOME", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("XDG_CONFIG_HOME") },
        }
        match old_xdg_data {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("XDG_DATA_HOME", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("XDG_DATA_HOME") },
        }
        match old_config_dir {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("WORKESTRATE_CONFIG_DIR", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("WORKESTRATE_CONFIG_DIR") },
        }
        match old_ctx {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("WORKESTRATE_CONTEXT", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("WORKESTRATE_CONTEXT") },
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

        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG_DIR", &dir) };
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

        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG_DIR", &dir) };
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
            "kind = \"agent\"\nimage = {{ recipe = \"registry\", ref = \"node:24\" }}\ncommand = []\ncpus = {cpus}\n\n[network.defaults]\negress = \"deny\"\n"
        )
    }

    /// Full `[workloads.<name>]` table form of the same workload.
    fn full_workload(name: &str, cpus: u32) -> String {
        format!(
            "[workloads.{name}]\nkind = \"agent\"\nimage = {{ recipe = \"registry\", ref = \"node:24\" }}\ncommand = []\ncpus = {cpus}\n\n[workloads.{name}.network.defaults]\negress = \"deny\"\n"
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

    /// ADR 0030 Phase 1: a capsule `workload.toml` carrying an `[instance]`
    /// table loads via the directory-mode loader and the policy lands on the
    /// implied workload.
    #[test]
    fn capsule_workload_toml_instance_block_parses() -> Result<()> {
        let repo = uniq_dir("spec30-capsule-instance");
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 1\n");
        write_repo_file(
            &repo,
            "workestrate/workloads/pi/workload.toml",
            "kind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\ncpus = 4\n\n[network.defaults]\negress = \"deny\"\n\n[instance]\nstrategy = \"reuse\"\nlabel = \"capsule\"\n",
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
        assert_eq!(wl.instance.strategy, crate::config::InstanceStrategy::Reuse);
        assert_eq!(wl.instance.label.as_deref(), Some("capsule"));
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

    // --- ADR 0028: policy collection stays separate from config merging ---

    fn policy_layer(name: &str, source: &Path, body: &str) -> crate::merge::Layer {
        crate::merge::Layer::from_string_with_path(name, body, Some(source.to_path_buf())).unwrap()
    }

    #[test]
    fn policy_compact_and_expanded_forms_are_strict_at_config_boundary() -> Result<()> {
        let layer = crate::merge::Layer::from_string(
            "policy",
            r#"
schema_version = 1

[policy.mounts.read]
deny = ["compact", { pattern = "expanded", final = true }]
allow = [{ pattern = "carve-out" }]
"#,
        )?;
        let fragment = layer.config.policy.mounts.unwrap();
        let read = fragment.read.expect("read axis");
        assert_eq!(read.deny[0].value, "compact");
        assert!(!read.deny[0].terminal);
        assert_eq!(read.deny[1].value, "expanded");
        assert!(read.deny[1].terminal);
        assert_eq!(read.allow[0].value, "carve-out");
        assert!(!read.allow[0].terminal);

        let err = match crate::merge::Layer::from_string(
            "bad-policy",
            "schema_version = 1\n[policy.mounts.read]\ndeny = [{ pattern = \"x\", typo = true }]\n",
        ) {
            Ok(_) => panic!("unknown policy field must be rejected"),
            Err(err) => format!("{err:#}"),
        };
        assert!(
            err.contains("unknown field") && err.contains("typo"),
            "{err}"
        );
        Ok(())
    }

    #[test]
    fn policy_collection_preserves_home_config_workload_and_mount_precedence() -> Result<()> {
        let source = PathBuf::from("/tmp/policy-config.toml");
        let registry = crate::config::Registry {
            policy: crate::config::PolicyConfig {
                mounts: Some(crate::mount_policy::MountsFragment {
                    read: Some(crate::mount_policy::AxisFragment {
                        deny: vec![crate::mount_policy::PolicyValue::relaxable("home".into())],
                        allow: vec![],
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        let layer = policy_layer(
            "repo#workestrate/workloads/pi/workload.toml",
            &source,
            r#"
schema_version = 1

[policy.mounts.read]
deny = ["config"]

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.policy.mounts.read]
deny = ["workload"]

[[workloads.pi.mounts]]
host = "config"
guest = "/config"
read_only = true

[workloads.pi.mounts.policy.read]
deny = ["mount"]
"#,
        );
        fn read_deny(fragment: &crate::mount_policy::MountsFragment) -> &str {
            fragment.read.as_ref().expect("read axis").deny[0]
                .value
                .as_str()
        }
        let collected = collect_policy_scopes(Some(&registry), &[layer])?;
        let global = collected
            .global
            .iter()
            .map(|scope| (scope.scope_kind, read_deny(&scope.fragment)))
            .collect::<Vec<_>>();
        assert_eq!(global.len(), 2);
        assert_eq!(global[0].0, crate::mount_policy::ScopeKind::HomeRegistry);
        assert_eq!(global[0].1, "home");
        assert_eq!(global[1].0, crate::mount_policy::ScopeKind::ConfigRepoLayer);
        assert_eq!(global[1].1, "config");

        let workload = &collected.workloads["pi"];
        assert_eq!(workload.len(), 2);
        assert_eq!(
            workload[0].scope_kind,
            crate::mount_policy::ScopeKind::Workload
        );
        assert_eq!(read_deny(&workload[0].fragment), "workload");
        assert_eq!(
            workload[1].scope_kind,
            crate::mount_policy::ScopeKind::MountEntry
        );
        assert_eq!(read_deny(&workload[1].fragment), "mount");
        Ok(())
    }

    /// The secret-policy ladder collection (the secrets edition of
    /// `collect_policy_scopes`) picks up the home-registry, layer, and
    /// workload-capsule `[policy.secrets]` rungs in loader stack order with
    /// the origin labels the resolution provenance records: "home-registry"
    /// for rung 2, the declaring layer's name for rungs 3-4.
    #[test]
    fn secret_policy_ladder_collection_preserves_home_layer_workload_order() -> Result<()> {
        let registry = crate::config::Registry {
            policy: crate::config::PolicyConfig {
                secrets: Some(crate::config::SecretsPolicyFragment {
                    on_violation: Some(crate::config::SecretViolationPolicy::Block),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        };
        let lower = crate::merge::Layer::from_string(
            "lower",
            r#"
schema_version = 1

[policy.secrets]
on_violation = "block-and-log"

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.policy.secrets]
on_violation = "block-and-terminate"
"#,
        )?;
        let higher = crate::merge::Layer::from_string(
            "higher",
            r#"
schema_version = 1

[policy.secrets]
on_violation = "block"

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.policy.secrets]
on_violation = "block-and-log"
"#,
        )?;
        let ladder = collect_secret_policy_ladder(Some(&registry), &[lower, higher]);

        // Rung 2: home registry, fixed "home-registry" origin label.
        let (home_origin, home_fragment) = ladder.home.expect("home rung collected");
        assert_eq!(home_origin, "home-registry");
        assert_eq!(
            home_fragment.on_violation,
            Some(crate::config::SecretViolationPolicy::Block)
        );
        assert!(!home_fragment.r#final);

        // Rung 3: layer fragments in input stack order, layer-name origins.
        assert_eq!(
            ladder
                .layers
                .iter()
                .map(|(origin, _)| origin.as_str())
                .collect::<Vec<_>>(),
            vec!["lower", "higher"],
            "layer rungs keep loader stack order"
        );
        assert_eq!(
            ladder.layers[0].1.on_violation,
            Some(crate::config::SecretViolationPolicy::BlockAndLog)
        );
        assert_eq!(
            ladder.layers[1].1.on_violation,
            Some(crate::config::SecretViolationPolicy::Block)
        );

        // Rung 4: workload rungs keyed by workload name, stack order.
        let pi_rungs = ladder
            .workloads
            .get("pi")
            .expect("workload rungs collected");
        assert_eq!(
            pi_rungs
                .iter()
                .map(|(origin, _)| origin.as_str())
                .collect::<Vec<_>>(),
            vec!["lower", "higher"],
            "workload rungs keep loader stack order"
        );
        assert_eq!(
            pi_rungs[0].1.on_violation,
            Some(crate::config::SecretViolationPolicy::BlockAndTerminate)
        );
        assert_eq!(
            pi_rungs[1].1.on_violation,
            Some(crate::config::SecretViolationPolicy::BlockAndLog)
        );
        Ok(())
    }

    #[test]
    fn policy_collection_keeps_only_the_winning_mounts_array() -> Result<()> {
        let lower = policy_layer(
            "lower",
            Path::new("/tmp/lower.toml"),
            r#"
schema_version = 1
[workloads.pi]
[[workloads.pi.mounts]]
host = "old"
guest = "/old"
read_only = true
[workloads.pi.mounts.policy.read]
deny = ["old-policy"]
"#,
        );
        let higher = policy_layer(
            "higher",
            Path::new("/tmp/higher.toml"),
            r#"
schema_version = 1
[workloads.pi]
mounts = []
"#,
        );
        let collected = collect_policy_scopes(None, &[lower, higher])?;
        assert!(collected.workloads.get("pi").is_none_or(Vec::is_empty));
        Ok(())
    }

    /// Mounts merge WHOLE-ARRAY, last layer wins — and since the deprecated
    /// `read_only` alias normalizes into `mode` at parse time, merged layers
    /// only ever carry the canonical `mode` field.
    #[test]
    fn mounts_merge_last_layer_wins_with_mode_across_layers() -> Result<()> {
        use crate::microsandbox::plan::MountMode;
        let lower = crate::merge::Layer::from_string(
            "lower",
            r#"
schema_version = 1
[workloads.pi]
[[workloads.pi.mounts]]
host = "old"
guest = "/old"
read_only = true
"#,
        )?;
        let higher = crate::merge::Layer::from_string(
            "higher",
            r#"
schema_version = 1
[workloads.pi]
[[workloads.pi.mounts]]
host = "new"
guest = "/new"
mode = "rw"
"#,
        )?;
        let (merged, provenance) = crate::merge::merge_layers(&[lower, higher])?;
        let mounts = &merged.workloads["pi"].mounts;
        assert_eq!(
            mounts.len(),
            1,
            "the higher layer's mounts array wins wholesale: {mounts:?}"
        );
        assert_eq!(mounts[0].host, "new");
        assert_eq!(mounts[0].mode, MountMode::Rw);
        assert_eq!(
            provenance.get("workloads.pi.mounts"),
            Some(&"higher".to_string())
        );
        // And the alias form normalizes before merge: a lower layer using
        // `read_only` carries the same MountMode a `mode` layer would.
        let alias_only = crate::merge::Layer::from_string(
            "alias",
            r#"
schema_version = 1
[workloads.pi]
[[workloads.pi.mounts]]
host = "cfg"
guest = "/cfg"
read_only = true
"#,
        )?;
        let (merged, _) = crate::merge::merge_layers(&[alias_only])?;
        assert_eq!(merged.workloads["pi"].mounts[0].mode, MountMode::Ro);
        Ok(())
    }

    /// Mount-policy sugar normalizes BEFORE merge: a layer carrying sugar
    /// merges as the canonical `policy` fragment, and the mounts array
    /// still merges whole-array last-layer-wins (mode-work precedent).
    #[test]
    fn mounts_merge_last_layer_wins_with_sugar_normalized() -> Result<()> {
        let lower = crate::merge::Layer::from_string(
            "lower",
            r#"
schema_version = 1
[workloads.pi]
[[workloads.pi.mounts]]
host = "old"
guest = "/old"
read.deny = ["lower-deny"]
"#,
        )?;
        let higher = crate::merge::Layer::from_string(
            "higher",
            r#"
schema_version = 1
[workloads.pi]
[[workloads.pi.mounts]]
host = "new"
guest = "/new"
policy.read.deny = ["table-deny"]
read.deny = ["sugar-deny"]
write.deny = ["sugar-write-deny"]
"#,
        )?;
        let (merged, _) = crate::merge::merge_layers(&[lower, higher])?;
        let mounts = &merged.workloads["pi"].mounts;
        assert_eq!(
            mounts.len(),
            1,
            "the higher layer's mounts array wins wholesale: {mounts:?}"
        );
        let policy = mounts[0]
            .policy
            .as_ref()
            .expect("sugar must normalize into Some(policy) before merge");
        let denies: Vec<&str> = policy
            .read
            .as_ref()
            .expect("read axis")
            .deny
            .iter()
            .map(|e| e.value.as_str())
            .collect();
        assert_eq!(denies, ["table-deny", "sugar-deny"]);
        let write = policy.write.as_ref().expect("write axis");
        assert_eq!(write.deny.len(), 1);
        assert_eq!(write.deny[0].value, "sugar-write-deny");
        // The lower layer's sugar-normalized policy is gone with its array.
        assert!(!denies.contains(&"lower-deny"));
        Ok(())
    }

    #[test]
    fn policy_collection_preserves_directory_and_capsule_source_provenance() -> Result<()> {
        let repo = uniq_dir("policy-provenance");
        write_repo_file(&repo, "workestrate/default.toml", "schema_version = 1\n");
        write_repo_file(
            &repo,
            "workestrate/workloads/capsule/workload.toml",
            "kind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n[policy.mounts.read]\ndeny = [\"capsule\"]\n",
        );
        let layers = load_config_repo_layers("personal", &repo)?;
        let collected = collect_policy_scopes(None, &layers)?;
        let scope = &collected.workloads["capsule"][0];
        assert_eq!(
            scope.layer_name,
            "personal#workestrate/workloads/capsule/workload.toml"
        );
        assert_eq!(
            scope.source_path,
            repo.join("workestrate/workloads/capsule/workload.toml")
        );
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    /// A bare directory-mode capsule's top-level `[policy.ssh]` lands
    /// on the workload rung via the workload wrapper (the same path the
    /// `[policy.secrets]` capsule rung takes), and layer-level
    /// `[policy.ssh]` lands on the layer rung.
    #[test]
    fn ssh_policy_collection_covers_layers_and_capsules() -> Result<()> {
        let repo = uniq_dir("ssh-policy-provenance");
        write_repo_file(
            &repo,
            "workestrate/default.toml",
            "schema_version = 1\n[policy.ssh]\nstrict = false\n",
        );
        write_repo_file(
            &repo,
            "workestrate/workloads/capsule/workload.toml",
            "kind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n[policy.ssh]\nstrict = true\n",
        );
        let layers = load_config_repo_layers("personal", &repo)?;
        let ladder = collect_ssh_policy_ladder(None, &layers);
        assert_eq!(ladder.layers.len(), 1);
        assert_eq!(ladder.layers[0].0, "personal#workestrate/default.toml");
        assert_eq!(ladder.layers[0].1.strict, Some(false));
        let capsule = &ladder.workloads["capsule"];
        assert_eq!(capsule.len(), 1);
        assert_eq!(
            capsule[0].0,
            "personal#workestrate/workloads/capsule/workload.toml"
        );
        assert_eq!(capsule[0].1.strict, Some(true));
        let _ = std::fs::remove_dir_all(&repo);
        Ok(())
    }

    #[test]
    fn non_operator_final_read_allow_is_rejected_after_collection() -> Result<()> {
        let layer = policy_layer(
            "repo",
            Path::new("/tmp/repo.toml"),
            "schema_version = 1\n[policy.mounts.read]\nallow = [{ pattern = \".env\", final = true }]\n",
        );
        let collected = collect_policy_scopes(None, &[layer])?;
        let err = crate::mount_policy::compile(collected.global).unwrap_err();
        assert!(err.to_string().contains("final read.allow") && err.to_string().contains(".env"));
        Ok(())
    }

    #[test]
    fn no_policy_collection_is_empty_and_config_merge_is_unchanged() -> Result<()> {
        let layer = policy_layer(
            "plain",
            Path::new("/tmp/plain.toml"),
            "schema_version = 1\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n",
        );
        let expected = layer.config.clone();
        let collected = collect_policy_scopes(None, &[layer])?;
        assert!(collected.is_empty());
        assert_eq!(expected.workloads["pi"].kind, "agent");
        Ok(())
    }

    // ------------------------------------------------------------------
    // A5 Session 2: pinned consumption (layer_content_root) + the
    // LOCK-NEVER-SILENT discipline.
    //
    // These tests build REAL temp git repos (the git.rs/archive.rs test
    // precedent: git + tar are on the pinned PATH, ungated). Each pins
    // WORKESTRATE_HOME at a fresh temp dir (HomeKind::Env: store = the home
    // itself, state = <home>/state, lock = <home>/workestrate.lock) and
    // holds ENV_TEST_LOCK + an EnvGuard.
    // ------------------------------------------------------------------

    /// Env keys these tests mutate (superset coverage of HOME_ENV_KEYS
    /// members they touch, plus the reference/state discovery vars).
    const A5_ENV_KEYS: &[&str] = &[
        "WORKESTRATE_HOME",
        "WORKESTRATE_CONFIG_DIR",
        "WORKESTRATE_NO_PROJECT_CONFIG",
        "WORKESTRATE_CONTEXT",
        "WORKESTRATE_CONFIG_REF",
        "WORKESTRATE_REFERENCE_CONFIG",
        "WORKESTRATE_STATE_DIR",
    ];

    /// Git-runner for the A5 fixtures: repo-local identity, assert success.
    fn a5_git(dir: &Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .expect("git must be runnable");
        assert!(
            status.success(),
            "git {:?} failed in {}",
            args,
            dir.display()
        );
    }

    /// Init a git repo at `dir` (branch main) with `files` committed;
    /// returns the HEAD sha. Shared by the A5 fixtures and the two pinned
    /// secrets-layer tests above.
    fn init_git_repo(dir: &Path, files: &[(&str, &str)]) {
        std::fs::create_dir_all(dir).expect("create repo dir");
        a5_git(dir, &["init", "--quiet", "-b", "main"]);
        a5_git(dir, &["config", "user.email", "a5-loading@test.invalid"]);
        a5_git(dir, &["config", "user.name", "a5-loading-test"]);
        for (rel, content) in files {
            let path = dir.join(rel);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("create parent");
            }
            std::fs::write(&path, content).expect("write fixture file");
        }
        a5_git(dir, &["add", "."]);
        a5_git(dir, &["commit", "--quiet", "-m", "fixture"]);
    }

    /// A one-workload workestrate.toml whose workload name is the content
    /// marker (rev A vs rev B discrimination).
    fn a5_marker_toml(marker: &str) -> String {
        format!(
            "schema_version = 1\n\n[workloads.{marker}]\nkind = \"agent\"\nimage = {{ recipe = \"registry\", ref = \"node:24\" }}\ncommand = []\n"
        )
    }

    /// Pin WORKESTRATE_HOME at a fresh temp dir and neutralize the other
    /// discovery env vars; return the home. Caller holds ENV_TEST_LOCK; the
    /// EnvGuard is created by the caller BEFORE calling this.
    fn a5_pin_home(label: &str) -> PathBuf {
        let home = uniq_dir(label);
        std::fs::create_dir_all(&home).expect("create pinned home");
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_HOME", &home) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_NO_PROJECT_CONFIG", "1") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("WORKESTRATE_CONFIG_DIR") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("WORKESTRATE_CONTEXT") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("WORKESTRATE_CONFIG_REF") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("WORKESTRATE_REFERENCE_CONFIG") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("WORKESTRATE_STATE_DIR") };
        home
    }

    /// Registry TOML for a single-layer home. `extra_entry_lines` carries
    /// e.g. `rev = "..."`.
    fn a5_registry(
        name: &str,
        url: &str,
        git_ref: Option<&str>,
        extra_entry_lines: &str,
    ) -> String {
        let ref_line = git_ref
            .map(|r| format!("ref = \"{r}\"\n"))
            .unwrap_or_default();
        format!(
            "layers = [\"{name}\"]\n\n[configs.{name}]\nurl = \"{url}\"\n{ref_line}{extra_entry_lines}"
        )
    }

    /// Build a home with a MANAGED CLONE `<home>/config-repos/<name>`
    /// committed at `files`, register it as a Remote entry (ref main), and
    /// return (home, clone, sha). No lock is written — each test decides
    /// the lock state explicitly.
    fn a5_remote_home(
        label: &str,
        name: &str,
        files: &[(&str, &str)],
    ) -> (PathBuf, PathBuf, String) {
        let home = a5_pin_home(label);
        let clone = home.join("config-repos").join(name);
        init_git_repo(&clone, files);
        let sha = crate::git::git_rev_parse(&clone).expect("clone HEAD sha");
        std::fs::write(
            home.join("config.toml"),
            a5_registry(name, "https://example.invalid/a5.git", Some("main"), ""),
        )
        .expect("write registry");
        (home, clone, sha)
    }

    /// Commit a follow-up rev on top of `clone` (advancing the working tree
    /// WITHOUT `config update` — the commit-before-consume gap).
    fn a5_advance_clone(clone: &Path, files: &[(&str, &str)]) -> String {
        for (rel, content) in files {
            std::fs::write(clone.join(rel), content).expect("write advance file");
        }
        a5_git(clone, &["add", "."]);
        a5_git(clone, &["commit", "--quiet", "-m", "advance"]);
        crate::git::git_rev_parse(clone).expect("advanced HEAD sha")
    }

    /// Write a lock pinning `name` at `sha` (primary pin, ref = main) into
    /// `home`.
    fn a5_write_lock(home: &Path, name: &str, sha: &str) {
        let mut lock = crate::config::HomeLock {
            version: crate::config::LOCK_VERSION,
            home_version: 2,
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            repos: std::collections::BTreeMap::new(),
        };
        crate::config::upsert_locked_pin(
            &mut lock,
            name,
            "https://example.invalid/a5.git",
            Some("main"),
            sha,
        );
        crate::config::save_home_lock_to(home, &lock).expect("write pin lock");
    }

    /// Pinned consumption: with the lock pinning rev A, advancing the clone
    /// to B (WITHOUT `config update`) must NOT change what `load_config`
    /// reads — consumption comes from A's archive, and the lock is left
    /// byte-identical (LOCK-NEVER-SILENT: case (i) writes nothing).
    #[test]
    fn pinned_consumption_reads_locked_rev_not_the_advanced_checkout() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        let (home, clone, sha_a) = a5_remote_home(
            "a5-pinned",
            "team",
            &[("workestrate.toml", &a5_marker_toml("rev-a"))],
        );
        a5_write_lock(&home, "team", &sha_a);
        let sha_b = a5_advance_clone(&clone, &[("workestrate.toml", &a5_marker_toml("rev_b"))]);
        assert_ne!(sha_a, sha_b);

        let lock_before = std::fs::read_to_string(home.join("workestrate.lock"))?;
        let cfg = load_config()?;

        assert!(
            cfg.workloads.contains_key("rev-a"),
            "consumption must read the LOCKED rev A, not the checkout: {:?}",
            cfg.workloads.keys().collect::<Vec<_>>()
        );
        assert!(!cfg.workloads.contains_key("rev_b"));
        // The content root IS the archive of sha A.
        let archive = crate::config::archive_dir(&sha_a)?;
        assert!(
            archive.join("workestrate.toml").exists(),
            "the archive of the locked rev must exist"
        );
        let layer_dirs = crate::merge::get_layer_dirs().expect("layer dirs set by load_config");
        assert_eq!(
            layer_dirs.get("team").map(|p| p.as_path()),
            Some(archive.as_path()),
            "layer_dirs must point at the archive dir, not the checkout"
        );
        assert_eq!(
            std::fs::read_to_string(home.join("workestrate.lock"))?,
            lock_before,
            "a lock-covered load must perform NO lock write (byte-identical)"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// First-resolution-with-notice: no lock, no registry rev → the first
    /// load resolves the effective ref in the clone, WRITES the lock entry
    /// and prints the stderr notice; a second identical load is silent and
    /// write-free (byte-identical lock). (The notice itself goes to the
    /// process's stderr and cannot be captured in-process; the write branch
    /// is the only place it is printed, and byte-identity proves that branch
    /// did not run on the second load.)
    #[test]
    fn first_resolution_writes_lock_then_second_load_is_silent() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        let (home, _clone, sha_a) = a5_remote_home(
            "a5-first",
            "team",
            &[("workestrate.toml", &a5_marker_toml("rev-a"))],
        );
        let lock_path = home.join("workestrate.lock");
        assert!(!lock_path.exists(), "fixture starts lock-free");

        let cfg = load_config()?;
        assert!(cfg.workloads.contains_key("rev-a"));

        // First resolution WROTE the lock: primary pin {rev = sha, sha,
        // fetched_at} for (team, main).
        let written = crate::config::load_home_lock()?.expect("lock written by first resolution");
        let entry = &written.repos["team"];
        assert_eq!(entry.rev.as_deref(), Some(sha_a.as_str()));
        assert_eq!(entry.sha.as_deref(), Some(sha_a.as_str()));
        assert!(
            entry.fetched_at.is_some(),
            "first resolution stamps fetched_at"
        );
        assert_eq!(written.version, crate::config::LOCK_VERSION);

        // Second load: silent and write-free.
        let lock_bytes = std::fs::read_to_string(&lock_path)?;
        let cfg2 = load_config()?;
        assert!(cfg2.workloads.contains_key("rev-a"));
        assert_eq!(
            std::fs::read_to_string(&lock_path)?,
            lock_bytes,
            "the second load must NOT touch the lock"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Registry-rev fallback: no lock, `rev` recorded in the registry →
    /// consume that rev SILENTLY (no lock write — the lock file stays
    /// absent), even when the clone has since advanced.
    #[test]
    fn registry_rev_fallback_consumes_recorded_rev_without_lock_write() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        let home = a5_pin_home("a5-regrev");
        let clone = home.join("config-repos").join("team");
        init_git_repo(&clone, &[("workestrate.toml", &a5_marker_toml("rev-a"))]);
        let sha_a = crate::git::git_rev_parse(&clone)?;
        a5_advance_clone(&clone, &[("workestrate.toml", &a5_marker_toml("rev_b"))]);
        std::fs::write(
            home.join("config.toml"),
            a5_registry(
                "team",
                "https://example.invalid/a5.git",
                Some("main"),
                &format!("rev = \"{sha_a}\"\n"),
            ),
        )?;

        let cfg = load_config()?;
        assert!(
            cfg.workloads.contains_key("rev-a"),
            "the registry-recorded rev must be consumed, not the checkout tip"
        );
        assert!(
            !home.join("workestrate.lock").exists(),
            "registry-rev consumption is SILENT: no lock file may appear"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// A registry-recorded SYMBOLIC rev (a branch/tag name where a content
    /// address belongs) errors CLEARLY before the archive store is touched:
    /// the message names the repo, the offending rev, explains that a
    /// symbolic ref is not a content address, and names the remediation.
    #[test]
    fn registry_recorded_symbolic_rev_fails_closed_with_remediation() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        let home = a5_pin_home("a5-regrev-symbolic");
        let clone = home.join("config-repos").join("team");
        init_git_repo(&clone, &[("workestrate.toml", &a5_marker_toml("rev-a"))]);
        std::fs::write(
            home.join("config.toml"),
            a5_registry(
                "team",
                "https://example.invalid/a5.git",
                Some("main"),
                "rev = \"main\"\n",
            ),
        )?;

        let err = load_config().expect_err("a symbolic recorded rev must fail closed");
        let msg = format!("{err:#}");
        assert!(msg.contains("team"), "error must name the repo: {msg}");
        assert!(msg.contains("main"), "error must name the rev: {msg}");
        assert!(
            msg.contains("symbolic ref (branch/tag) is not a content address"),
            "error must explain the symbolic-ref case: {msg}"
        );
        assert!(
            msg.contains("workestrate config update team"),
            "error must name the remediation: {msg}"
        );
        assert!(
            !home.join("workestrate.lock").exists(),
            "the failed load must NOT write the lock"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// PlainPath entries are the documented exception: content-as-is from
    /// the working dir (an edit is visible on the very next load), NO
    /// archive and NO lock write.
    #[test]
    fn plain_path_entry_consumes_working_dir_as_is() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        let home = a5_pin_home("a5-plain");
        let plain = home.join("my-plain-config");
        std::fs::create_dir_all(&plain)?;
        std::fs::write(plain.join("workestrate.toml"), a5_marker_toml("v1"))?;
        std::fs::write(
            home.join("config.toml"),
            a5_registry("local", &plain.to_string_lossy(), None, ""),
        )?;

        let cfg = load_config()?;
        assert!(cfg.workloads.contains_key("v1"));

        // Edit the plain dir → the NEXT load sees the edit (no commit, no
        // update, no archive).
        std::fs::write(plain.join("workestrate.toml"), a5_marker_toml("v2"))?;
        let cfg = load_config()?;
        assert!(
            cfg.workloads.contains_key("v2"),
            "plain-path consumption is content-as-is: edits are immediately visible"
        );
        assert!(
            !home.join("workestrate.lock").exists(),
            "plain-path consumption never writes the lock"
        );
        assert!(
            !crate::config::archive_store_root().exists(),
            "plain-path consumption never produces an archive"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Secrets layers resolve against the SAME content root: for a pinned
    /// entry the `.env.enc` path lives in the ARCHIVE dir (rev A content),
    /// not the checkout (rev B content).
    #[test]
    fn secrets_layer_resolves_from_the_archive_for_pinned_entries() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        let (home, clone, sha_a) = a5_remote_home(
            "a5-secrets",
            "team",
            &[
                ("workestrate.toml", &a5_marker_toml("rev-a")),
                (".env.enc", "enc-content-A"),
            ],
        );
        a5_write_lock(&home, "team", &sha_a);
        a5_advance_clone(&clone, &[(".env.enc", "enc-content-B")]);

        let layers = resolve_secrets_layers()?;
        let layer = layers
            .iter()
            .find(|l| l.name == "team")
            .expect("team secrets layer");
        let archive = crate::config::archive_dir(&sha_a)?;
        assert_eq!(
            layer.dir, archive,
            "the pinned secrets dir IS the archive of the locked rev"
        );
        assert_eq!(
            std::fs::read_to_string(layer.dir.join(&layer.secrets_file))?,
            "enc-content-A",
            ".env.enc rides the archive: rev A content, not the checkout's rev B"
        );
        assert_eq!(
            std::fs::read_to_string(clone.join(".env.enc"))?,
            "enc-content-B",
            "the checkout DID advance (the archive is what holds A)"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Provenance under pinned consumption (spec 17 §3b): pseudo-layer
    /// provenance strings still carry `<repo>#workestrate/...` relpaths —
    /// computed against the archive dir exactly as against a working copy.
    #[test]
    fn pinned_consumption_provenance_keeps_repo_relpath_strings() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        let (home, _clone, sha_a) = a5_remote_home(
            "a5-prov",
            "team",
            &[
                ("workestrate/default.toml", "schema_version = 1\n"),
                (
                    "workestrate/workloads/pinned.toml",
                    &full_workload("pinned", 2),
                ),
            ],
        );
        a5_write_lock(&home, "team", &sha_a);

        let cfg = load_config()?;
        assert!(cfg.workloads.contains_key("pinned"));
        let provenance = crate::merge::get_provenance().expect("provenance set by load_config");
        assert_eq!(
            provenance.get("schema_version").map(String::as_str),
            Some("team#workestrate/default.toml"),
            "provenance relpaths are computed against the archive dir (spec 17 §3b: F-rules unchanged)"
        );
        assert_eq!(
            provenance.get("workloads.pinned.kind").map(String::as_str),
            Some("team#workestrate/workloads/pinned.toml")
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    // ------------------------------------------------------------------
    // A5 Session 3a: the --config-ref override (WORKESTRATE_CONFIG_REF) —
    // every Remote/GitFile entry resolves at THAT ref, locked into the
    // entry's refs map; PlainPath entries are unaffected.
    // ------------------------------------------------------------------

    /// Commit `files` on a NEW branch `branch` of `clone`, then return the
    /// checkout to `main`; returns the branch tip sha.
    fn a5_commit_branch(clone: &Path, branch: &str, files: &[(&str, &str)]) -> String {
        a5_git(clone, &["checkout", "--quiet", "-b", branch]);
        for (rel, content) in files {
            std::fs::write(clone.join(rel), content).expect("write branch file");
        }
        a5_git(clone, &["add", "."]);
        a5_git(clone, &["commit", "--quiet", "-m", branch]);
        let sha = crate::git::git_rev_parse(clone).expect("branch tip sha");
        a5_git(clone, &["checkout", "--quiet", "main"]);
        sha
    }

    /// --config-ref resolves EVERY git-backed entry at the ref: two Remote
    /// entries with primary pins on main both consume their feat-x content;
    /// the PRIMARY pins are NOT moved, and the resolutions land in each
    /// entry's refs map.
    #[test]
    fn config_ref_resolves_every_git_entry_at_the_ref() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        let home = a5_pin_home("a5-cfgref-two");
        let alpha = home.join("config-repos").join("alpha");
        let beta = home.join("config-repos").join("beta");
        init_git_repo(
            &alpha,
            &[("workestrate.toml", &a5_marker_toml("alpha_main"))],
        );
        init_git_repo(&beta, &[("workestrate.toml", &a5_marker_toml("beta_main"))]);
        let alpha_main = crate::git::git_rev_parse(&alpha)?;
        let beta_main = crate::git::git_rev_parse(&beta)?;
        let alpha_feat = a5_commit_branch(
            &alpha,
            "feat-x",
            &[("workestrate.toml", &a5_marker_toml("alpha-feat"))],
        );
        let beta_feat = a5_commit_branch(
            &beta,
            "feat-x",
            &[("workestrate.toml", &a5_marker_toml("beta-feat"))],
        );
        std::fs::write(
            home.join("config.toml"),
            "layers = [\"alpha\", \"beta\"]\n\n[configs.alpha]\nurl = \"https://example.invalid/alpha.git\"\nref = \"main\"\n\n[configs.beta]\nurl = \"https://example.invalid/beta.git\"\nref = \"main\"\n",
        )?;
        // Primary pins on main for BOTH entries (S2 shape).
        let mut lock = crate::config::HomeLock {
            version: crate::config::LOCK_VERSION,
            home_version: 2,
            tool_version: env!("CARGO_PKG_VERSION").to_string(),
            repos: std::collections::BTreeMap::new(),
        };
        for (name, sha) in [("alpha", &alpha_main), ("beta", &beta_main)] {
            crate::config::upsert_locked_pin(
                &mut lock,
                name,
                &format!("https://example.invalid/{name}.git"),
                Some("main"),
                sha,
            );
        }
        crate::config::save_home_lock_to(&home, &lock)?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG_REF", "feat-x") };

        let cfg = load_config()?;

        for feat in ["alpha-feat", "beta-feat"] {
            assert!(
                cfg.workloads.contains_key(feat),
                "both entries must be consumed at feat-x: {:?}",
                cfg.workloads.keys().collect::<Vec<_>>()
            );
        }
        for main_rev in ["alpha_main", "beta_main"] {
            assert!(
                !cfg.workloads.contains_key(main_rev),
                "the primary (main) content must NOT leak into a --config-ref load"
            );
        }
        // Both archives come from feat-x's shas.
        assert!(
            crate::config::archive_dir(&alpha_feat)?
                .join("workestrate.toml")
                .exists()
        );
        assert!(
            crate::config::archive_dir(&beta_feat)?
                .join("workestrate.toml")
                .exists()
        );
        // Lock: refs map carries feat-x; PRIMARY pins unmoved.
        let lock = crate::config::load_home_lock()?.expect("lock");
        assert_eq!(
            lock.repos["alpha"].rev.as_deref(),
            Some(alpha_main.as_str())
        );
        assert_eq!(lock.repos["beta"].rev.as_deref(), Some(beta_main.as_str()));
        assert_eq!(lock.repos["alpha"].refs["feat-x"].sha, alpha_feat);
        assert_eq!(lock.repos["beta"].refs["feat-x"].sha, beta_feat);

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Fail-closed: an unknown --config-ref errors naming the repo AND the
    /// ref (per entry; the first failing entry surfaces).
    #[test]
    fn config_ref_unknown_ref_errors_naming_repo_and_ref() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        let (home, _clone, _sha) = a5_remote_home(
            "a5-cfgref-unknown",
            "team",
            &[("workestrate.toml", &a5_marker_toml("rev-a"))],
        );
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG_REF", "no-such-ref") };

        let err = load_config().expect_err("an unknown --config-ref must fail closed");
        let msg = format!("{err:#}");
        assert!(msg.contains("team"), "error must name the repo: {msg}");
        assert!(
            msg.contains("no-such-ref"),
            "error must name the ref: {msg}"
        );
        // Fail-closed means NO lock write and NO archive side effect.
        assert!(!home.join("workestrate.lock").exists());

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Refs-map lock discipline: the first --config-ref load writes
    /// refs[<ref>] (primary pin UNMOVED — it stays on main); the second
    /// load is SILENT and write-free (byte-identical lock).
    #[test]
    fn config_ref_first_resolution_locks_refs_then_second_is_write_free() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        let (home, clone, sha_main) = a5_remote_home(
            "a5-cfgref-discipline",
            "team",
            &[("workestrate.toml", &a5_marker_toml("rev-a"))],
        );
        a5_write_lock(&home, "team", &sha_main);
        let sha_feat = a5_commit_branch(
            &clone,
            "feat-x",
            &[("workestrate.toml", &a5_marker_toml("rev-feat"))],
        );
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG_REF", "feat-x") };

        let cfg = load_config()?;
        assert!(cfg.workloads.contains_key("rev-feat"));

        let lock_path = home.join("workestrate.lock");
        let lock = crate::config::load_home_lock()?.expect("lock after first resolution");
        let entry = &lock.repos["team"];
        assert_eq!(
            entry.rev.as_deref(),
            Some(sha_main.as_str()),
            "the PRIMARY pin must NOT be moved by a --config-ref resolution"
        );
        assert_eq!(entry.sha.as_deref(), Some(sha_main.as_str()));
        let pin = &entry.refs["feat-x"];
        assert_eq!(pin.rev, "feat-x", "the refs pin is keyed by the ref itself");
        assert_eq!(pin.sha, sha_feat);
        assert!(pin.fetched_at.ends_with('Z'), "fetched_at stamped");

        // Second load: silent + write-free (byte-identical lock).
        let bytes = std::fs::read_to_string(&lock_path)?;
        let cfg2 = load_config()?;
        assert!(cfg2.workloads.contains_key("rev-feat"));
        assert_eq!(
            std::fs::read_to_string(&lock_path)?,
            bytes,
            "the second --config-ref load must NOT touch the lock"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// A sha-shaped --config-ref is legal and keyed BY ITSELF in the refs
    /// map; with no prior lock, the primary pin stays ABSENT (an
    /// override-only history never fabricates one).
    #[test]
    fn config_ref_sha_pins_keyed_by_itself_without_a_primary_pin() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        let (home, _clone, sha_a) = a5_remote_home(
            "a5-cfgref-sha",
            "team",
            &[("workestrate.toml", &a5_marker_toml("rev-a"))],
        );
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG_REF", &sha_a) };

        let cfg = load_config()?;
        assert!(cfg.workloads.contains_key("rev-a"));

        let lock = crate::config::load_home_lock()?.expect("lock written");
        let entry = &lock.repos["team"];
        assert_eq!(entry.rev, None, "no primary pin is fabricated");
        assert_eq!(entry.sha, None);
        let pin = &entry.refs[&sha_a];
        assert_eq!(pin.rev, sha_a, "a sha-shaped ref is keyed by itself");
        assert_eq!(pin.sha, sha_a);

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// PlainPath entries are UNAFFECTED by --config-ref: content-as-is
    /// (an edit is visible on the very next load), no archive, no lock
    /// entry — while the git-backed sibling entry IS pinned at the ref.
    #[test]
    fn config_ref_leaves_plain_path_entries_unaffected() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        let home = a5_pin_home("a5-cfgref-plain");
        let plain = home.join("my-plain-config");
        std::fs::create_dir_all(&plain)?;
        std::fs::write(plain.join("workestrate.toml"), a5_marker_toml("plain-v1"))?;
        let clone = home.join("config-repos").join("team");
        init_git_repo(
            &clone,
            &[("workestrate.toml", &a5_marker_toml("team_main"))],
        );
        a5_commit_branch(
            &clone,
            "feat-x",
            &[("workestrate.toml", &a5_marker_toml("team-feat"))],
        );
        std::fs::write(
            home.join("config.toml"),
            format!(
                "layers = [\"plain\", \"team\"]\n\n[configs.plain]\nurl = \"{}\"\n\n[configs.team]\nurl = \"https://example.invalid/team.git\"\nref = \"main\"\n",
                plain.display()
            ),
        )?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG_REF", "feat-x") };

        let cfg = load_config()?;
        assert!(cfg.workloads.contains_key("plain-v1"));
        assert!(cfg.workloads.contains_key("team-feat"));
        assert!(!cfg.workloads.contains_key("team_main"));

        // Content-as-is: an edit to the plain dir is visible on the NEXT
        // load even under --config-ref.
        std::fs::write(plain.join("workestrate.toml"), a5_marker_toml("plain-v2"))?;
        let cfg = load_config()?;
        assert!(
            cfg.workloads.contains_key("plain-v2"),
            "plain-path consumption stays content-as-is under --config-ref"
        );
        assert!(cfg.workloads.contains_key("team-feat"));

        // Lock: the git-backed entry has its refs pin; the plain entry has
        // NO lock entry at all.
        let lock = crate::config::load_home_lock()?.expect("lock");
        assert!(lock.repos["team"].refs.contains_key("feat-x"));
        assert!(
            !lock.repos.contains_key("plain"),
            "plain-path entries never enter the lock"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    // ------------------------------------------------------------------
    // A5 Session 3b: the per-workload inline `name:ref` override
    // (two-phase pending/armed state → capsule-only substitution in
    // load_config). Same temp-git-repo fixture discipline as the S2/S3a
    // tests above; every test also clears the process-global inline
    // override state before and after itself.
    // ------------------------------------------------------------------

    /// Directory-mode fixture content: the prime capsule at cpus `n`.
    fn a5b_dir_mode_files(cpus: u32) -> Vec<(String, String)> {
        vec![
            (
                "workestrate/default.toml".to_string(),
                "schema_version = 1\n".to_string(),
            ),
            (
                "workestrate/workloads/prime/workload.toml".to_string(),
                bare_workload(cpus),
            ),
        ]
    }

    /// Build a directory-mode Remote home (repo `team`) with the prime
    /// capsule at cpus=1 on main and cpus=2 on feat-x; primary-lock main.
    /// Returns (home, clone, main sha, feat-x sha).
    fn a5b_dir_mode_home(label: &str) -> (PathBuf, PathBuf, String, String) {
        let owned = a5b_dir_mode_files(1);
        let files: Vec<(&str, &str)> = owned
            .iter()
            .map(|(r, c)| (r.as_str(), c.as_str()))
            .collect();
        let (home, clone, sha_main) = a5_remote_home(label, "team", &files);
        a5_write_lock(&home, "team", &sha_main);
        let owned_feat = a5b_dir_mode_files(2);
        let feat_files: Vec<(&str, &str)> = owned_feat
            .iter()
            .map(|(r, c)| (r.as_str(), c.as_str()))
            .collect();
        let sha_feat = a5_commit_branch(&clone, "feat-x", &feat_files);
        (home, clone, sha_main, sha_feat)
    }

    /// Armed substitution: the prime capsule is read at feat-x while the
    /// rest of the merged config stays home-scoped (pinned main).
    #[test]
    fn inline_override_armed_substitutes_the_capsule_at_the_ref() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        crate::config::clear_inline_override();
        let (home, _clone, _sha_main, sha_feat) = a5b_dir_mode_home("a5b-dir-armed");

        crate::config::set_pending_inline_override("prime", "feat-x");
        crate::config::arm_inline_override();
        let cfg = load_config()?;
        crate::config::clear_inline_override();

        assert_eq!(
            cfg.workloads["prime"].cpus,
            Some(2),
            "the armed load must read the capsule at feat-x"
        );
        // The declaring layer's content root is the feat-x archive's
        // directory-mode root, and the workload's field provenance points
        // at the substituted pseudo-layer.
        let layer_key = "team#workestrate/workloads/prime/workload.toml";
        let layer_dirs = crate::merge::get_layer_dirs().expect("layer dirs set");
        assert_eq!(
            layer_dirs.get(layer_key).map(|p| p.as_path()),
            Some(
                crate::config::archive_dir(&sha_feat)?
                    .join("workestrate")
                    .as_path()
            ),
            "the declaring layer's content root must be the feat-x archive"
        );
        let provenance = crate::merge::get_provenance().expect("provenance set");
        assert_eq!(
            provenance.get("workloads.prime.cpus").map(|s| s.as_str()),
            Some(layer_key),
            "the substituted field's provenance names the capsule layer"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Two-phase state (deps NEVER follow the override): a PENDING-but-not-
    /// armed load is byte-identical to no override at all — this is the
    /// auto_start_dependencies config view. The fixture's feat-x branch
    /// changes prime's depends_on to prove the home-scope depends_on is
    /// what the un-armed view sees.
    #[test]
    fn inline_override_pending_not_armed_is_byte_identical_to_no_override() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        crate::config::clear_inline_override();
        let mut main_files = a5b_dir_mode_files(1);
        main_files[1].1 = format!(
            "{}\n[depends_on.litellm]\nenv = \"LITELLM_ADDR\"\n",
            bare_workload(1)
        );
        main_files.push((
            "workestrate/workloads/litellm/workload.toml".to_string(),
            bare_workload(3),
        ));
        let main_refs: Vec<(&str, &str)> = main_files
            .iter()
            .map(|(r, c)| (r.as_str(), c.as_str()))
            .collect();
        let (home, clone, sha_main) = a5_remote_home("a5b-deps", "team", &main_refs);
        a5_write_lock(&home, "team", &sha_main);
        // feat-x: prime's depends_on is REMOVED (deps would differ if the
        // dep view ever followed the override).
        let mut feat_files = a5b_dir_mode_files(2);
        feat_files.push((
            "workestrate/workloads/litellm/workload.toml".to_string(),
            bare_workload(3),
        ));
        let feat_refs: Vec<(&str, &str)> = feat_files
            .iter()
            .map(|(r, c)| (r.as_str(), c.as_str()))
            .collect();
        a5_commit_branch(&clone, "feat-x", &feat_refs);

        let clean = load_config()?;
        assert!(clean.workloads["prime"].depends_on.contains_key("litellm"));

        crate::config::set_pending_inline_override("prime", "feat-x");
        let pending = load_config()?;
        crate::config::clear_inline_override();

        assert_eq!(
            pending, clean,
            "pending-but-not-armed load_config must be byte-identical to no override"
        );
        assert!(
            pending.workloads["prime"]
                .depends_on
                .contains_key("litellm"),
            "the un-armed (dep auto-start) view sees the HOME depends_on"
        );

        // ...while the ARMED view sees the substituted (dep-free) capsule.
        crate::config::set_pending_inline_override("prime", "feat-x");
        crate::config::arm_inline_override();
        let armed = load_config()?;
        crate::config::clear_inline_override();
        assert!(
            armed.workloads["prime"].depends_on.is_empty(),
            "the armed view sees the ref's capsule (depends_on removed at feat-x)"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Fail-closed: an unknown inline ref errors naming the repo AND the
    /// ref; NO lock write and NO substitution side effect.
    #[test]
    fn inline_override_unknown_ref_fails_closed_naming_repo_and_ref() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        crate::config::clear_inline_override();
        let (home, _clone, _sha_main, _sha_feat) = a5b_dir_mode_home("a5b-unknown-ref");

        crate::config::set_pending_inline_override("prime", "no-such-ref");
        crate::config::arm_inline_override();
        let err = load_config().expect_err("an unknown inline ref must fail closed");
        crate::config::clear_inline_override();
        let msg = format!("{err:#}");
        assert!(msg.contains("team"), "error must name the repo: {msg}");
        assert!(
            msg.contains("no-such-ref"),
            "error must name the ref: {msg}"
        );
        // Fail-closed: the lock gained no refs entry for the bad ref.
        let lock = crate::config::load_home_lock()?.expect("lock");
        assert!(
            !lock.repos["team"].refs.contains_key("no-such-ref"),
            "a failed resolution must not write the lock"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Fail-closed: the workload does not exist AT THE REF (the feat-x
    /// branch drops the capsule) — the error names workload + ref + repo.
    #[test]
    fn inline_override_unknown_workload_at_ref_fails_closed() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        crate::config::clear_inline_override();
        let owned = a5b_dir_mode_files(1);
        let files: Vec<(&str, &str)> = owned
            .iter()
            .map(|(r, c)| (r.as_str(), c.as_str()))
            .collect();
        let (home, clone, sha_main) = a5_remote_home("a5b-unknown-wl", "team", &files);
        a5_write_lock(&home, "team", &sha_main);
        // feat-x: the prime capsule is REMOVED (default.toml remains).
        a5_git(&clone, &["checkout", "--quiet", "-b", "feat-x"]);
        a5_git(
            &clone,
            &["rm", "-r", "--quiet", "workestrate/workloads/prime"],
        );
        a5_git(&clone, &["commit", "--quiet", "-m", "drop prime"]);
        a5_git(&clone, &["checkout", "--quiet", "main"]);

        crate::config::set_pending_inline_override("prime", "feat-x");
        crate::config::arm_inline_override();
        let err = load_config().expect_err("a workload missing at the ref must fail closed");
        crate::config::clear_inline_override();
        let msg = format!("{err:#}");
        assert!(msg.contains("prime"), "error must name the workload: {msg}");
        assert!(msg.contains("feat-x"), "error must name the ref: {msg}");
        assert!(msg.contains("team"), "error must name the repo: {msg}");

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Single-file mode variant: the substitution reads
    /// `<archive>/workestrate.toml`'s [workloads.prime] table at the ref.
    #[test]
    fn inline_override_single_file_mode_substitution() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        crate::config::clear_inline_override();
        let main_toml = "schema_version = 1\n\n[workloads.prime]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\ncpus = 1\n"
            .to_string();
        let feat_toml = main_toml.replace("cpus = 1", "cpus = 2");
        let (home, clone, sha_main) =
            a5_remote_home("a5b-file", "team", &[("workestrate.toml", &main_toml)]);
        a5_write_lock(&home, "team", &sha_main);
        a5_commit_branch(&clone, "feat-x", &[("workestrate.toml", &feat_toml)]);

        crate::config::set_pending_inline_override("prime", "feat-x");
        crate::config::arm_inline_override();
        let cfg = load_config()?;
        crate::config::clear_inline_override();

        assert_eq!(
            cfg.workloads["prime"].cpus,
            Some(2),
            "single-file mode: prime is read at feat-x"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Flat-file directory-mode fixture content: prime declared as the flat
    /// file `workestrate/workloads/prime.toml` (bare form, spec 17 §2.3) at
    /// cpus `n`.
    fn a5b_flat_mode_files(cpus: u32) -> Vec<(String, String)> {
        vec![
            (
                "workestrate/default.toml".to_string(),
                "schema_version = 1\n".to_string(),
            ),
            (
                "workestrate/workloads/prime.toml".to_string(),
                bare_workload(cpus),
            ),
        ]
    }

    /// Flat-file variant of [`a5b_dir_mode_home`]: prime at cpus=1 on main,
    /// cpus=2 on feat-x; primary-lock main. Returns (home, clone, main sha,
    /// feat-x sha).
    fn a5b_flat_mode_home(label: &str) -> (PathBuf, PathBuf, String, String) {
        let owned = a5b_flat_mode_files(1);
        let files: Vec<(&str, &str)> = owned
            .iter()
            .map(|(r, c)| (r.as_str(), c.as_str()))
            .collect();
        let (home, clone, sha_main) = a5_remote_home(label, "team", &files);
        a5_write_lock(&home, "team", &sha_main);
        let owned_feat = a5b_flat_mode_files(2);
        let feat_files: Vec<(&str, &str)> = owned_feat
            .iter()
            .map(|(r, c)| (r.as_str(), c.as_str()))
            .collect();
        let sha_feat = a5_commit_branch(&clone, "feat-x", &feat_files);
        (home, clone, sha_main, sha_feat)
    }

    /// A5 review MEDIUM-2: the flat-file directory-mode form
    /// (`workestrate/workloads/<name>.toml`) is substitutable at a ref —
    /// same capsule-only semantics as the capsule dir: the flat file is read
    /// at feat-x, provenance names the flat-file pseudo-layer, and the
    /// declaring layer's content root is the feat-x archive.
    #[test]
    fn inline_override_flat_file_substitutes_at_the_ref() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        crate::config::clear_inline_override();
        let (home, _clone, _sha_main, sha_feat) = a5b_flat_mode_home("a5b-flat-armed");

        crate::config::set_pending_inline_override("prime", "feat-x");
        crate::config::arm_inline_override();
        let cfg = load_config()?;
        crate::config::clear_inline_override();

        assert_eq!(
            cfg.workloads["prime"].cpus,
            Some(2),
            "the armed load must read the flat file at feat-x"
        );
        let layer_key = "team#workestrate/workloads/prime.toml";
        let layer_dirs = crate::merge::get_layer_dirs().expect("layer dirs set");
        assert_eq!(
            layer_dirs.get(layer_key).map(|p| p.as_path()),
            Some(
                crate::config::archive_dir(&sha_feat)?
                    .join("workestrate")
                    .as_path()
            ),
            "the flat-file layer's content root must be the feat-x archive"
        );
        let provenance = crate::merge::get_provenance().expect("provenance set");
        assert_eq!(
            provenance.get("workloads.prime.cpus").map(|s| s.as_str()),
            Some(layer_key),
            "the substituted field's provenance names the flat-file layer"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Fail-closed (A5 review MEDIUM-2): a workload declared ONLY as a flat
    /// file at home scope, dropped at the ref (no capsule, no flat file, no
    /// single-file table there), errors naming workload + ref + repo.
    #[test]
    fn inline_override_flat_file_absent_at_ref_fails_closed() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        crate::config::clear_inline_override();
        let owned = a5b_flat_mode_files(1);
        let files: Vec<(&str, &str)> = owned
            .iter()
            .map(|(r, c)| (r.as_str(), c.as_str()))
            .collect();
        let (home, clone, sha_main) = a5_remote_home("a5b-flat-absent", "team", &files);
        a5_write_lock(&home, "team", &sha_main);
        // feat-x: the prime flat file is REMOVED (default.toml remains).
        a5_git(&clone, &["checkout", "--quiet", "-b", "feat-x"]);
        a5_git(
            &clone,
            &["rm", "--quiet", "workestrate/workloads/prime.toml"],
        );
        a5_git(&clone, &["commit", "--quiet", "-m", "drop prime"]);
        a5_git(&clone, &["checkout", "--quiet", "main"]);

        crate::config::set_pending_inline_override("prime", "feat-x");
        crate::config::arm_inline_override();
        let err = load_config()
            .expect_err("a workload absent from ALL three forms at the ref must fail closed");
        crate::config::clear_inline_override();
        let msg = format!("{err:#}");
        assert!(msg.contains("prime"), "error must name the workload: {msg}");
        assert!(msg.contains("feat-x"), "error must name the ref: {msg}");
        assert!(msg.contains("team"), "error must name the repo: {msg}");

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Lock discipline: the first substitution resolution writes
    /// refs[feat-x] (primary pin UNMOVED); the repeat is silent and
    /// write-free (byte-identical lock).
    #[test]
    fn inline_override_first_resolution_locks_refs_then_repeat_is_write_free() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        crate::config::clear_inline_override();
        let (home, _clone, sha_main, sha_feat) = a5b_dir_mode_home("a5b-lock");

        crate::config::set_pending_inline_override("prime", "feat-x");
        crate::config::arm_inline_override();
        let cfg = load_config()?;
        assert_eq!(cfg.workloads["prime"].cpus, Some(2));

        let lock_path = home.join("workestrate.lock");
        let lock = crate::config::load_home_lock()?.expect("lock after first resolution");
        let entry = &lock.repos["team"];
        assert_eq!(
            entry.rev.as_deref(),
            Some(sha_main.as_str()),
            "the PRIMARY pin must NOT be moved by an inline resolution"
        );
        let pin = &entry.refs["feat-x"];
        assert_eq!(pin.rev, "feat-x", "the refs pin is keyed by the ref itself");
        assert_eq!(pin.sha, sha_feat);
        assert!(pin.fetched_at.ends_with('Z'), "fetched_at stamped");

        // Repeat: silent + write-free (byte-identical lock).
        let bytes = std::fs::read_to_string(&lock_path)?;
        let cfg2 = load_config()?;
        crate::config::clear_inline_override();
        assert_eq!(cfg2.workloads["prime"].cpus, Some(2));
        assert_eq!(
            std::fs::read_to_string(&lock_path)?,
            bytes,
            "the repeat inline-override load must NOT touch the lock"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// The home-scope unknown-workload error is NOT masked: an armed
    /// override naming a workload absent from the merged config leaves the
    /// config untouched (today's "not found in config" fires downstream).
    #[test]
    fn inline_override_unknown_workload_at_home_scope_is_not_masked() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        crate::config::clear_inline_override();
        let (home, _clone, _sha_main, _sha_feat) = a5b_dir_mode_home("a5b-home-unknown");

        crate::config::set_pending_inline_override("ghost", "feat-x");
        crate::config::arm_inline_override();
        let cfg = load_config()?;
        crate::config::clear_inline_override();
        assert!(
            !cfg.workloads.contains_key("ghost"),
            "the substitution must not fabricate the workload"
        );
        assert_eq!(
            cfg.workloads["prime"].cpus,
            Some(1),
            "home-scoped content is untouched"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Single-file-mode `workestrate.toml` for the prime workload, with an
    /// optional `[workloads.prime.policy.mounts.read]` fragment whose deny
    /// entry is the given marker (home vs ref discrimination).
    fn a5b_policy_toml(deny_marker: Option<&str>) -> String {
        let policy = match deny_marker {
            Some(marker) => {
                format!("\n[workloads.prime.policy.mounts.read]\ndeny = [\"{marker}\"]\n")
            }
            None => String::new(),
        };
        format!(
            "schema_version = 1\n\n[workloads.prime]\nkind = \"agent\"\nimage = {{ recipe = \"registry\", ref = \"node:24\" }}\ncommand = []\ncpus = 1\n{policy}"
        )
    }

    /// Policy re-collection reflects the substitution (2026-08-28): the
    /// collected policy for the overridden workload must come from the
    /// REF's declaration, not the home one — the pre-fix collection read
    /// the pre-substitution layers, silently dropping the ref's fragment
    /// while validate_config saw the substituted declaration.
    #[test]
    fn inline_override_policy_collection_uses_the_ref_fragment() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        crate::config::clear_inline_override();
        let main_toml = a5b_policy_toml(Some("home-scope"));
        let feat_toml = a5b_policy_toml(Some("ref-scope"));
        let (home, clone, sha_main) = a5_remote_home(
            "a5b-policy-ref",
            "team",
            &[("workestrate.toml", &main_toml)],
        );
        a5_write_lock(&home, "team", &sha_main);
        let sha_feat = a5_commit_branch(&clone, "feat-x", &[("workestrate.toml", &feat_toml)]);

        crate::config::set_pending_inline_override("prime", "feat-x");
        crate::config::arm_inline_override();
        let _cfg = load_config()?;
        crate::config::clear_inline_override();

        let collected = crate::mount_policy::get_collected_policy()
            .expect("collected policy set by load_config");
        let scopes = collected
            .workloads
            .get("prime")
            .expect("prime policy scopes collected");
        assert_eq!(scopes.len(), 1, "exactly the ref's workload fragment");
        assert_eq!(
            scopes[0].scope_kind,
            crate::mount_policy::ScopeKind::Workload
        );
        let deny = &scopes[0].fragment.read.as_ref().expect("read axis").deny;
        assert_eq!(
            deny[0].value, "ref-scope",
            "the collected policy must be the REF's fragment, not the home one"
        );
        assert_eq!(
            scopes[0].layer_name, "team",
            "the scope names the substituted layer"
        );
        let archive = crate::config::archive_dir(&sha_feat)?;
        assert!(
            scopes[0].source_path.starts_with(&archive),
            "the scope's source path is the feat-x archive's layer file: {}",
            scopes[0].source_path.display()
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// The substituted declaration is the WHOLE workload declaration: when
    /// the ref carries NO policy fragment, the home-collected scopes for
    /// that workload are REMOVED rather than left to mask the substitution.
    #[test]
    fn inline_override_no_policy_fragment_at_ref_removes_home_scopes() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_ENV_KEYS);
        crate::config::clear_inline_override();
        let main_toml = a5b_policy_toml(Some("home-scope"));
        let feat_toml = a5b_policy_toml(None);
        let (home, clone, sha_main) = a5_remote_home(
            "a5b-policy-none",
            "team",
            &[("workestrate.toml", &main_toml)],
        );
        a5_write_lock(&home, "team", &sha_main);
        a5_commit_branch(&clone, "feat-x", &[("workestrate.toml", &feat_toml)]);

        // Baseline: un-armed, the home fragment IS collected.
        let _cfg = load_config()?;
        let collected = crate::mount_policy::get_collected_policy().expect("collected policy");
        assert!(
            collected.workloads.contains_key("prime"),
            "the home declaration carries a policy fragment"
        );

        crate::config::set_pending_inline_override("prime", "feat-x");
        crate::config::arm_inline_override();
        let _cfg = load_config()?;
        crate::config::clear_inline_override();

        let collected = crate::mount_policy::get_collected_policy()
            .expect("collected policy set by load_config");
        assert!(
            !collected.workloads.contains_key("prime"),
            "no fragment at the ref must REMOVE the home-collected scopes for prime"
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    // ---- ADR 0036: virtualization seal collection (collect, never merge) ----

    /// The seal ladder collects home-registry → layers → workload capsules
    /// in loader order, carrying origin labels; fragments never pass through
    /// `merge_layers` (no policy field does).
    #[test]
    fn collect_virtualization_ladder_orders_rungs() {
        let mut registry = Registry::default();
        registry.policy.virtualization = Some(crate::config::VirtualizationPolicyFragment {
            allow_nested: Some(false),
            r#final: false,
        });
        let layer = crate::merge::Layer::from_string(
            "personal",
            "schema_version = 1\n\n[policy.virtualization]\nallow_nested = true\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.virtualization]\nnested = \"require\"\n\n[workloads.pi.policy.virtualization]\nallow_nested = true\n",
        )
        .unwrap();
        let ladder = collect_virtualization_ladder(Some(&registry), &[layer]);
        assert_eq!(
            ladder.home.as_ref().map(|(o, _)| o.as_str()),
            Some("home-registry"),
            "rung 1 is the home registry"
        );
        assert_eq!(ladder.layers.len(), 1, "rung 2 is the layer fragment");
        assert_eq!(ladder.layers[0].0, "personal");
        assert_eq!(
            ladder.workloads.get("pi").map(|v| v.len()),
            Some(1),
            "rung 3 is the workload capsule fragment"
        );
        // The ASK is not a fragment: it merged into the layer config, not
        // the ladder.
        assert_eq!(ladder.workloads["pi"][0].1.allow_nested, Some(true));
    }

    /// Overrides stay lenient AND keep the new keys: `virtualization` (new)
    /// and `instance` (bundled WORKLOAD_FIELDS fix) survive stripping
    /// instead of warning as typos.
    #[test]
    fn overrides_keep_virtualization_and_instance_keys() -> Result<()> {
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-ov-virt-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        let path = write_overrides(
            &tmp,
            "[global.workloads.pi]\ncpus = 4\n\n[global.workloads.pi.virtualization]\nnested = \"prefer\"\n\n[global.workloads.pi.instance]\nstrategy = \"parallel\"\n",
        );
        let existing: std::collections::HashSet<String> = ["pi".to_string()].into_iter().collect();
        let layers = load_overrides(&path, &[], &existing)?;
        assert_eq!(layers.len(), 1);
        let pi = layers[0].config.workloads.get("pi").unwrap();
        assert_eq!(
            pi.virtualization.as_ref().and_then(|v| v.nested),
            Some(crate::config::NestedMode::Prefer),
            "virtualization ask survives override stripping"
        );
        assert_eq!(
            pi.instance.strategy,
            crate::config::InstanceStrategy::Parallel,
            "instance block survives override stripping (bundled fix)"
        );
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }
}
