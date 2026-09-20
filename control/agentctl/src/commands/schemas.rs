//! `workestrate schemas` — distribute the tool's OWN generated JSON Schema
//! artifacts to every known consumer location.
//!
//! Authority model (single source of truth, ADR 0021 §8):
//!   - The Rust serde/schemars types compiled into this binary ARE the
//!     checker (`validate-config`); the JSON Schema files are tool-derived
//!     PROJECTIONS of those types.
//!   - This command distributes the projections to consumers: the tool repo's
//!     copier template (`<root>/templates/workestrate-config/schemas/`), the
//!     config (`<config>/schemas/`), and every registered fleet.
//!   - Fleets are written ONLY when they carry a `schemas/` dir — a
//!     workestrate-managed fleet is scaffolded with one, so hand-made fleets
//!     are intentionally skipped (the tool never invents a `schemas/` dir in
//!     a fleet that does not have one).
//!
//! Target rules:
//!   - Target order: tool template, config, then registered fleets.
//!   - The tool template is used only when the project root resolves AND its
//!     `templates/workestrate-config/` exists (standalone installs / non-tool
//!     checkouts print a skip note and continue).
//!   - The config is used only when it exists (a first-run / uninitialized
//!     config prints a skip note and continues).
//!   - Repos: local-path entries resolve to `entry.url` directly; git-URL
//!     entries resolve to the store clone `<store>/fleets/<name>`. A
//!     missing clone, or a fleet without a `schemas/` dir, is skipped with a
//!     note.
//!   - `--fleet <name>` scopes the run to ONE registered fleet and
//!     SKIPS the tool-template + config targets.
//!
//! Idempotency: each artifact is written only when the target file is missing
//! or its bytes differ; identical files are skipped. `--check` never writes —
//! it reports `ok`/`stale` per file and exits 1 when any consumer copy is
//! stale or missing (the CI drift gate).

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::cli_actions::SchemasAction;
use crate::config;

/// Dispatch entry for the `schemas` command group.
pub fn cmd_schemas(action: SchemasAction) -> Result<()> {
    match action {
        SchemasAction::Update { fleet, check } => cmd_schemas_update(fleet.as_deref(), check),
    }
}

/// `workestrate schemas update [--fleet <name>] [--check]`: distribute the
/// generated schema triple to every known consumer location (idempotently).
pub fn cmd_schemas_update(fleet: Option<&str>, check: bool) -> Result<()> {
    // Generate the artifacts from the single source of truth (the same
    // `generate_schema_triple` the schema drift guard uses). The returned
    // Strings must stay alive for the whole function; the artifacts array
    // borrows them as &str.
    let (full, workload, registry) = crate::commands::diagnostics::generate_schema_triple()?;
    let artifacts: [(&str, &str); 3] = [
        ("workestrate.schema.json", full.as_str()),
        ("workestrate-workload.schema.json", workload.as_str()),
        ("registry.schema.json", registry.as_str()),
    ];

    // Collect the consumer target dirs (in target order) via the shared
    // enumerator, plus the human skip notes. Only the schemas/ dirs that
    // actually qualify are targets; everything else prints a skip note and
    // continues.
    let (targets, notes) = schema_targets_with_notes(fleet)?;
    for note in &notes {
        eprintln!("{note}");
    }

    // Distribute each artifact to each collected target, idempotently.
    let mut written = 0usize;
    let mut skipped = 0usize;
    let mut stale = 0usize;
    for target in &targets {
        for (file_name, content) in artifacts {
            let path = target.dir.join(file_name);
            // The canonical on-disk form carries the trailing newline that
            // `cmd_generate_schema` writes to --output (the committed consumer
            // copies carry it; writing it keeps `schemas update` idempotent
            // against them).
            let canonical = format!("{}\n", content);
            let unchanged = file_bytes_equal(&path, canonical.as_bytes());
            if check {
                if unchanged {
                    println!("ok      {}", path.display());
                } else {
                    println!("stale   {}", path.display());
                    stale += 1;
                }
            } else if unchanged {
                println!("skipped {} (unchanged)", path.display());
                skipped += 1;
            } else {
                std::fs::create_dir_all(&target.dir)?;
                std::fs::write(&path, canonical)?;
                println!("wrote   {}", path.display());
                written += 1;
            }
        }
    }

    if check {
        if stale > 0 {
            println!(
                "schemas update --check: {stale} stale consumer file(s); run 'workestrate schemas update'"
            );
            std::process::exit(1);
        }
        println!("all consumer schema copies are fresh");
    } else {
        println!("schemas update: {written} written, {skipped} skipped");
    }
    Ok(())
}

/// A consumer location for the generated schema artifacts.
pub(crate) struct SchemaTarget {
    /// Human label for reporting ("tool template", "config", "fleet 'x'").
    pub label: String,
    /// Directory that carries schemas/workestrate.schema.json (+ workload subschema).
    pub dir: PathBuf,
}

/// Enumerate every known consumer location, applying the documented rules
/// (see module doc): tool-repo template when project_root_optional() resolves
/// AND templates/workestrate-config exists; config when it exists; each
/// registered fleet ONLY when its schemas/ dir exists. When
/// `repo_filter` is Some(name), ONLY that fleet is returned (and the
/// tool template + config targets are excluded, matching --fleet scoping).
///
/// Only PRESENT targets are returned — targets that were skipped (missing
/// template dir / config / clone / schemas/ dir) are silent here so read-only
/// callers like `doctor` do not emit `schemas update`'s skip notes.
pub(crate) fn schema_targets(fleet_filter: Option<&str>) -> Result<Vec<SchemaTarget>> {
    Ok(schema_targets_with_notes(fleet_filter)?.0)
}

/// The shared enumeration core: the present targets (in target order) plus
/// the human skip notes that `schemas update` prints on stderr. Collected in
/// one pass so side-effecting resolution (e.g. the legacy-XDG migration
/// note from `resolve_config_dir_with_kind`) happens exactly once per run.
fn schema_targets_with_notes(
    fleet_filter: Option<&str>,
) -> Result<(Vec<SchemaTarget>, Vec<String>)> {
    let mut targets: Vec<SchemaTarget> = Vec::new();
    let mut notes: Vec<String> = Vec::new();

    if fleet_filter.is_none() {
        // Target 1: the tool repo's copier template. The template is
        // workestrate-managed, so its schemas/ subdir is created when missing.
        if let Some(root) = config::project_root_optional() {
            let template = root.join("templates").join("workestrate-config");
            if template.is_dir() {
                targets.push(SchemaTarget {
                    label: "tool template".to_string(),
                    dir: template.join("schemas"),
                });
            } else {
                notes.push(format!(
                    "note: tool-repo copier template {} is absent (standalone binary or non-tool checkout); skipping it",
                    template.display()
                ));
            }
        }

        // Target 2: the config. The config dir is workestrate-managed, so its
        // schemas/ subdir is created when missing.
        let (config_dir, _kind) = config::resolve_config_dir_with_kind();
        if config_dir.exists() {
            targets.push(SchemaTarget {
                label: "config".to_string(),
                dir: config_dir.join("schemas"),
            });
        } else {
            notes.push(format!(
                "note: config {} does not exist; skipping it (run 'workestrate config init')",
                config_dir.display()
            ));
        }
    }

    // Target 3: registered fleets. `--fleet <name>` scopes to ONE entry
    // (targets 1 and 2 were already skipped above); a missing name errors.
    let registry = config::load_registry()?;
    match fleet_filter {
        Some(name) => {
            let registry = registry
                .ok_or_else(|| anyhow::anyhow!("no fleet named '{name}' in the registry"))?;
            let entry = registry
                .fleets
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("no fleet named '{name}' in the registry"))?;
            push_fleet_target(name, entry, &mut targets, &mut notes);
        }
        None => {
            if let Some(registry) = registry {
                let mut names: Vec<&String> = registry.fleets.keys().collect();
                names.sort();
                for name in names {
                    push_fleet_target(name, &registry.fleets[name], &mut targets, &mut notes);
                }
            }
        }
    }
    Ok((targets, notes))
}

/// Resolve one registered fleet's checkout and collect its `schemas/`
/// dir when it qualifies (exists AND carries `schemas/`). Records a skip note
/// otherwise — the tool never creates a `schemas/` dir in a fleet that does
/// not already have one.
fn push_fleet_target(
    name: &str,
    entry: &crate::config::FleetEntry,
    targets: &mut Vec<SchemaTarget>,
    notes: &mut Vec<String>,
) {
    let dir = if let Some(dir) = config::local_entry_checkout_dir(entry) {
        // Local-path entries (registered via `fleet new`): url IS the dir,
        // resolved config-relative when relative (shared-config duality).
        dir
    } else {
        // Git-URL entries: the store clone path.
        config::fleet_dir(name)
    };
    if !dir.is_dir() {
        notes.push(format!(
            "note: fleet '{name}' clone not present at {}; skipping it",
            dir.display()
        ));
        return;
    }
    let schemas = dir.join("schemas");
    if !schemas.is_dir() {
        notes.push(format!(
            "note: fleet '{name}' is not a workestrate-managed repo (no schemas/ dir at {}); skipping it",
            schemas.display()
        ));
        return;
    }
    targets.push(SchemaTarget {
        label: format!("fleet '{name}'"),
        dir: schemas,
    });
}

/// Whether `path` already holds exactly `content` bytes (missing file → false).
fn file_bytes_equal(path: &Path, content: &[u8]) -> bool {
    std::fs::read(path)
        .map(|existing| existing == content)
        .unwrap_or(false)
}
