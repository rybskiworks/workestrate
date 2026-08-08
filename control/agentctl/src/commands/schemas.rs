//! `workestrate schemas` — distribute the tool's OWN generated JSON Schema
//! artifacts to every known consumer location.
//!
//! Authority model (single source of truth, ADR 0021 §8):
//!   - The Rust serde/schemars types compiled into this binary ARE the
//!     checker (`validate-config`); the JSON Schema files are tool-derived
//!     PROJECTIONS of those types.
//!   - This command distributes the projections to consumers: the tool repo's
//!     copier template (`<root>/templates/workestrate-config/schemas/`), the
//!     tool home (`<home>/schemas/`), and every registered config repo.
//!   - Config repos are written ONLY when they carry a `schemas/` dir — a
//!     workestrate-managed repo is scaffolded with one, so hand-made repos
//!     are intentionally skipped (the tool never invents a `schemas/` dir in
//!     a repo that does not have one).
//!
//! Target rules:
//!   - Target order: tool template, tool home, then registered config repos.
//!   - The tool template is used only when the project root resolves AND its
//!     `templates/workestrate-config/` exists (standalone installs / non-tool
//!     checkouts print a skip note and continue).
//!   - The tool home is used only when it exists (a first-run / uninitialized
//!     home prints a skip note and continues).
//!   - Repos: local-path entries resolve to `entry.url` directly; git-URL
//!     entries resolve to the store clone `<store>/config-repos/<name>`. A
//!     missing clone, or a repo without a `schemas/` dir, is skipped with a
//!     note.
//!   - `--repo <name>` scopes the run to ONE registered config repo and
//!     SKIPS the tool-template + tool-home targets.
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
        SchemasAction::Update { repo, check } => cmd_schemas_update(repo.as_deref(), check),
    }
}

/// `workestrate schemas update [--repo <name>] [--check]`: distribute the
/// generated schema pair to every known consumer location (idempotently).
pub fn cmd_schemas_update(repo: Option<&str>, check: bool) -> Result<()> {
    // Generate the artifacts from the single source of truth (the same
    // `generate_schema_pair` the schema drift guard uses). The returned
    // Strings must stay alive for the whole function; the artifacts array
    // borrows them as &str.
    let (full, workload) = crate::commands::diagnostics::generate_schema_pair()?;
    let artifacts: [(&str, &str); 2] = [
        ("workestrate.schema.json", full.as_str()),
        ("workestrate-workload.schema.json", workload.as_str()),
    ];

    // Collect the consumer target dirs (in target order). Only the schemas/
    // dirs that actually qualify are collected; everything else prints a
    // skip note and continues.
    let mut targets: Vec<PathBuf> = Vec::new();

    if repo.is_none() {
        // Target 1: the tool repo's copier template. The template is
        // workestrate-managed, so its schemas/ subdir is created when missing.
        if let Some(root) = config::project_root_optional() {
            let template = root.join("templates").join("workestrate-config");
            if template.is_dir() {
                targets.push(template.join("schemas"));
            } else {
                eprintln!(
                    "note: tool-repo copier template {} is absent (standalone binary or non-tool checkout); skipping it",
                    template.display()
                );
            }
        }

        // Target 2: the tool home. The home is workestrate-managed, so its
        // schemas/ subdir is created when missing.
        let (home, _kind) = config::resolve_home_with_kind();
        if home.exists() {
            targets.push(home.join("schemas"));
        } else {
            eprintln!(
                "note: tool home {} does not exist; skipping it (run 'workestrate home init')",
                home.display()
            );
        }
    }

    // Target 3: registered config repos. `--repo <name>` scopes to ONE entry
    // (targets 1 and 2 were already skipped above); a missing name errors.
    let registry = config::load_registry()?;
    match repo {
        Some(name) => {
            let registry = registry
                .ok_or_else(|| anyhow::anyhow!("no config repo named '{name}' in the registry"))?;
            let entry = registry
                .configs
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("no config repo named '{name}' in the registry"))?;
            push_config_repo_target(name, entry, &mut targets);
        }
        None => {
            if let Some(registry) = registry {
                let mut names: Vec<&String> = registry.configs.keys().collect();
                names.sort();
                for name in names {
                    push_config_repo_target(name, &registry.configs[name], &mut targets);
                }
            }
        }
    }

    // Distribute each artifact to each collected target, idempotently.
    let mut written = 0usize;
    let mut skipped = 0usize;
    let mut stale = 0usize;
    for dir in &targets {
        for (file_name, content) in artifacts {
            let path = dir.join(file_name);
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
                std::fs::create_dir_all(dir)?;
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

/// Resolve one registered config repo's checkout and collect its `schemas/`
/// dir when it qualifies (exists AND carries `schemas/`). Prints a skip note
/// otherwise — the tool never creates a `schemas/` dir in a repo that does
/// not already have one.
fn push_config_repo_target(
    name: &str,
    entry: &crate::config::ConfigRepoEntry,
    targets: &mut Vec<PathBuf>,
) {
    let dir = if config::entry_is_local_path(entry) {
        // Local-path entries (registered via `config new`): url IS the dir.
        PathBuf::from(&entry.url)
    } else {
        // Git-URL entries: the store clone path.
        config::config_repo_dir(name)
    };
    if !dir.is_dir() {
        eprintln!(
            "note: config repo '{name}' clone not present at {}; skipping it",
            dir.display()
        );
        return;
    }
    let schemas = dir.join("schemas");
    if !schemas.is_dir() {
        eprintln!(
            "note: config repo '{name}' is not a workestrate-managed repo (no schemas/ dir at {}); skipping it",
            schemas.display()
        );
        return;
    }
    targets.push(schemas);
}

/// Whether `path` already holds exactly `content` bytes (missing file → false).
fn file_bytes_equal(path: &Path, content: &[u8]) -> bool {
    std::fs::read(path)
        .map(|existing| existing == content)
        .unwrap_or(false)
}
