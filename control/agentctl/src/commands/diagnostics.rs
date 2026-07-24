//! Inspection and diagnostics commands: `plan`, `ps`, `check`,
//! `validate-config`, `generate-schema`, `generate-env-example`, and the
//! generic `run` exec entry.
//!
//! NOTE (WP4-B): verbatim copies of the private items still live in
//! `main.rs`. Commit C cuts `main.rs` over to these and deletes its own.

use std::path::PathBuf;

use anyhow::Result;

use crate::commands::source::build_env_var_name;
use crate::config;
use crate::config::CheckEntry;
use crate::git::{git_is_dirty, short_rev};
use crate::json_out::ps_entries_json;

pub(crate) fn cmd_plan<W: crate::microsandbox::workload::Workload>(
    workload: &W,
    show_source: bool,
    json: bool,
    port_offset: u16,
) -> Result<()> {
    if json {
        let mut plan = workload.plan();
        if port_offset != 0 {
            for p in &mut plan.ports {
                p.host = p.host.checked_add(port_offset).ok_or_else(|| {
                    anyhow::anyhow!("port offset {} overflows host port {}", port_offset, p.host)
                })?;
            }
        }
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else if show_source {
        println!("{}", workload.show_source());
    } else if port_offset != 0 {
        // Reuse the Display impl but shift host ports first.
        let mut plan = workload.plan();
        for p in &mut plan.ports {
            p.host = p.host.checked_add(port_offset).ok_or_else(|| {
                anyhow::anyhow!("port offset {} overflows host port {}", port_offset, p.host)
            })?;
        }
        print!("{}", plan);
    } else {
        println!("{}", workload.plan());
    }
    Ok(())
}

pub(crate) async fn cmd_ps(json: bool) -> Result<()> {
    use crate::microsandbox::runtime::{probe_liveness, ps};
    let state_dir = crate::config::resolve_state_dir();
    let mut entries = ps(&state_dir)?;
    // Best-effort liveness probe (ADR 0021 §4). ps() stays pure; this is the
    // only place ps rows touch msb. When the msb DB is unreachable we cannot
    // distinguish running from stale, so emit one honest stderr note rather
    // than silently trusting the registry records.
    let unreachable = probe_liveness(&mut entries).await;
    if unreachable > 0 {
        eprintln!(
            "warning: could not verify liveness of {n} instance(s) via msb (db unreachable); stale flags may be inaccurate",
            n = unreachable,
        );
    }
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&ps_entries_json(&entries))?
        );
    } else {
        print_ps_text(&entries)?;
    }
    Ok(())
}

pub(crate) fn print_ps_text(entries: &[crate::microsandbox::runtime::PsEntry]) -> Result<()> {
    // Thin stdout wrapper over the writer-injectable renderer, so the footer
    // text and table layout are unit-testable without capturing global stdout.
    print_ps_text_to(entries, &mut std::io::stdout())?;
    Ok(())
}

/// Render `ps` rows (table + stale-remediation footer) to `out`. Pure I/O:
/// no msb, no env. `cmd_ps` passes `&mut std::io::stdout()`; tests pass a
/// `Vec<u8>`. Returns io::Error on write failure (propagated as anyhow).
pub(crate) fn print_ps_text_to<W: std::io::Write>(
    entries: &[crate::microsandbox::runtime::PsEntry],
    out: &mut W,
) -> std::io::Result<()> {
    if entries.is_empty() {
        writeln!(out, "(no running workestrate instances)")?;
        return Ok(());
    }
    // Stable column layout: INSTANCE | WORKLOAD | CONTEXT | PORTS | STARTED
    // (renamed from CREATED to match the ADR 0021 §7 `started_at` field).
    writeln!(
        out,
        "{:<32} {:<16} {:<12} {:<24} STARTED",
        "INSTANCE", "WORKLOAD", "CONTEXT", "PORTS"
    )?;
    let mut sorted: Vec<_> = entries.iter().collect();
    sorted.sort_by(|a, b| a.instance.cmp(&b.instance));
    for e in sorted {
        let ports_str = e
            .ports
            .iter()
            .map(|p| format!("{}:{}", p.host, p.guest))
            .collect::<Vec<_>>()
            .join(",");
        let started_display = if e.started_at.is_empty() {
            "-".to_string()
        } else {
            e.started_at.clone()
        };
        writeln!(
            out,
            "{:<32} {:<16} {:<12} {:<24} {}",
            e.instance,
            e.workload,
            e.context.clone().unwrap_or_else(|| "-".into()),
            ports_str,
            started_display,
        )?;
    }

    // ADR 0021 §4: stale state records (registry entry exists but the backing
    // sandbox is gone) get a remediation footer naming the precise teardown
    // command per instance, plus the catch-all `down --all`.
    let stale: Vec<&crate::microsandbox::runtime::PsEntry> =
        entries.iter().filter(|e| e.stale).collect();
    if !stale.is_empty() {
        writeln!(out)?;
        writeln!(
            out,
            "{} stale instance(s): registry record exists but the sandbox is not running.",
            stale.len()
        )?;
        for e in &stale {
            // Parallel instance (slot@id) → `down --instance <id>`;
            // singleton → bare `down`. Uses the id, not the full instance name.
            if let Some((_, id)) = e.instance.split_once('@') {
                writeln!(out, "  workestrate {} down --instance {}", e.workload, id)?;
            } else {
                writeln!(out, "  workestrate {} down", e.workload)?;
            }
        }
        writeln!(out, "Remove every instance with: workestrate down --all")?;
    }
    Ok(())
}

pub(crate) async fn cmd_check() -> Result<()> {
    println!("=== workestrate check ===\n");
    let mut all_ok = true;

    let registry_path = config::registry_path();
    if registry_path.exists() {
        println!("Registry: {} [OK]", registry_path.display());
        match config::load_registry()? {
            None => {
                println!("  (registry exists but could not be loaded)");
                all_ok = false;
            }
            Some(registry) => {
                println!("  Config repos:");
                if registry.configs.is_empty() {
                    println!("    (none)");
                } else {
                    for (name, entry) in &registry.configs {
                        let dest = config::config_repo_dir(name);
                        let (dirty_label, ok) = if dest.exists() {
                            match git_is_dirty(&dest) {
                                Ok(false) => ("clean", true),
                                Ok(true) => ("dirty", false),
                                Err(_) => ("unknown", false),
                            }
                        } else {
                            ("missing", false)
                        };
                        let rev = entry.rev.as_deref().unwrap_or("unknown");
                        let short = short_rev(rev);
                        let git_ref = entry.r#ref.as_deref().unwrap_or("main");
                        let status = if ok { "[OK]" } else { "[MISSING]" };
                        println!(
                            "    {}: {} (ref {}, rev {}, {}) {}",
                            name, entry.url, git_ref, short, dirty_label, status
                        );
                    }
                }
                println!("  Layers: {:?}", registry.layers);
                println!("  Trusted projects:");
                if registry.trusted_projects.is_empty() {
                    println!("    (none)");
                } else {
                    for p in &registry.trusted_projects {
                        let path = PathBuf::from(&p.path);
                        let status = if path.exists() { "[OK]" } else { "[MISSING]" };
                        println!("    {} {}", p.path, status);
                    }
                }
            }
        }
    } else {
        println!("Registry: {} [MISSING]", registry_path.display());
        all_ok = false;
    }

    // Active context
    match config::resolve_active_context() {
        Ok(ctx) => {
            if let Some(ref name) = ctx.name {
                println!("Active context: {} [OK]", name);
            } else {
                println!("Active context: (none — using bare layers)");
            }
            println!("  Layers: {:?}", ctx.layers);
        }
        Err(e) => {
            println!("Active context: [ERROR] {}", e);
            all_ok = false;
        }
    }

    println!("\nTool home:");
    let (home, kind) = config::resolve_home_with_kind();
    println!("  home: {} ({:?})", home.display(), kind);
    let dirs: Vec<(&str, std::path::PathBuf)> = match kind {
        config::HomeKind::LegacyXdg => vec![
            ("config", config::xdg_config_dir()),
            ("data", config::xdg_data_dir()),
            ("state", config::xdg_state_dir()),
        ],
        _ => vec![
            ("registry", config::registry_path()),
            ("secrets", home.join("secrets")),
            ("repos", home.join("repos")),
            ("state", home.join("state")),
        ],
    };
    for (label, dir) in dirs {
        let ok = dir.exists();
        let status = if ok { "[OK]" } else { "[MISSING]" };
        println!("  {}: {} {}", label, dir.display(), status);
        if !ok {
            all_ok = false;
        }
    }

    println!("\nSource overrides:");
    match config::load_config() {
        Ok(cfg) => {
            let mut found = false;
            for (name, workload) in &cfg.workloads {
                if workload.local_build.is_none() {
                    continue;
                }
                found = true;
                let env_var = build_env_var_name(name);
                let repo = config::source_store_dir(name).join("repo");
                let build = config::source_store_dir(name).join("build");
                let repo_status = if repo.exists() {
                    "checked out"
                } else {
                    "not checked out"
                };
                let build_status = if build.exists() { "built" } else { "not built" };
                match std::env::var(&env_var) {
                    Ok(path) => println!(
                        "  {}: override {} (repo: {}, build: {})",
                        name, path, repo_status, build_status
                    ),
                    Err(_) => println!(
                        "  {}: no override ({} not set) (repo: {}, build: {})",
                        name, env_var, repo_status, build_status
                    ),
                }
            }
            if !found {
                println!("  (none)");
            }
        }
        Err(e) => {
            println!("  (could not load config: {})", e);
            all_ok = false;
        }
    }

    println!("\nReference config:");
    let reference = find_reference_config();
    match reference {
        Some(path) if path.exists() => println!("  {} [OK]", path.display()),
        Some(path) => {
            println!("  {} [MISSING]", path.display());
            all_ok = false;
        }
        None => {
            println!("  (could not resolve reference config)");
            all_ok = false;
        }
    }

    println!("\nRequired files:");
    // WP5 / E1: project_root_optional() returns None when workestrate is
    // invoked outside a workbench checkout (the documented standalone-tool
    // install path). Previously cmd_check hard-failed on this, breaking the
    // first command after `nix profile install`. We now skip the
    // workbench-layout checks with a note and let the registry / XDG /
    // reference-config checks above stand on their own.
    match config::project_root_optional() {
        Some(root) => {
            let checks = config::check_required_files(&root)?;
            let mut had_missing_required = false;
            for entry in &checks {
                print_entry(entry);
                if !entry.ok && !entry.optional {
                    had_missing_required = true;
                }
            }
            if had_missing_required {
                all_ok = false;
            }
        }
        None => {
            println!("  (not in a workbench checkout — skipped)");
        }
    }

    if all_ok {
        println!("\nAll checks passed.");
        Ok(())
    } else {
        Err(anyhow::anyhow!("Some checks failed."))
    }
}

fn print_entry(entry: &CheckEntry) {
    if entry.ok {
        println!("[OK] {}", entry.label);
        return;
    }
    if entry.optional {
        println!("[MISSING] (optional) {}", entry.label);
    } else {
        println!("[MISSING] {}", entry.label);
    }
}

pub(crate) fn find_reference_config() -> Option<PathBuf> {
    // WP5 / E1: project_root_optional() returns None for standalone installs;
    // we then fall through to the CARGO_MANIFEST_DIR probe (cargo run / test)
    // and finally return None so cmd_check can print "(could not resolve
    // reference config)" instead of bailing.
    if let Some(root) = config::project_root_optional() {
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

pub(crate) async fn cmd_validate_config() -> Result<()> {
    let config = config::load_config()?;
    config::validate_config(&config)?;
    println!("workestrate.toml is valid.");
    Ok(())
}

pub(crate) fn cmd_generate_schema(out: Option<&std::path::Path>) -> Result<()> {
    let schema = schemars::schema_for!(crate::config::ConfigFile);
    let json = serde_json::to_string_pretty(&schema)?;
    match out {
        Some(p) => {
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(p, format!("{}\n", json))?;
            println!("wrote schema to {}", p.display());
        }
        None => println!("{}", json),
    }
    Ok(())
}

pub(crate) async fn cmd_generate_env_example(output: Option<&std::path::Path>) -> Result<()> {
    let config = config::load_config()?;
    let mut entries: Vec<(&str, &str)> = config
        .secrets
        .values()
        .filter_map(|s| {
            let env_var = s.env_var.as_deref()?;
            let description = s.description.as_deref().unwrap_or("");
            Some((env_var, description))
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));

    let mut buf = String::new();
    buf.push_str("# ai-workbench environment schema.\n");
    buf.push_str("# This file is committed and safe to share.\n");
    buf.push_str(
        "# Real secrets live in .env.enc (encrypted) and are loaded by workestrate at runtime.\n",
    );
    for (env_var, description) in entries {
        if !description.is_empty() {
            buf.push_str(&format!("\n# {}\n", description));
        } else {
            buf.push('\n');
        }
        buf.push_str(&format!("{}=\n", env_var));
    }
    buf.push_str("\n# Optional local paths\n");
    buf.push_str("AI_WORKBENCH_WORKSPACES_DIR=workspaces\n");
    buf.push_str("AI_WORKBENCH_VAR_DIR=var\n");

    match output {
        Some(path) => {
            std::fs::write(path, &buf)?;
        }
        None => {
            print!("{}", buf);
        }
    }
    Ok(())
}

pub(crate) async fn cmd_run(command: &[String]) -> Result<()> {
    if command.is_empty() {
        anyhow::bail!("no command specified. Usage: workestrate run -- <command> [args...]");
    }

    // Load secrets from .env.enc (generic — all keys, no filtering).
    crate::microsandbox::secrets_loader::load_secrets()?;

    // exec the command (replaces the workestrate process).
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new(&command[0])
            .args(&command[1..])
            .exec();
        // exec() only returns on failure.
        anyhow::bail!("failed to exec '{}': {}", command[0], err);
    }

    #[cfg(not(unix))]
    {
        let status = std::process::Command::new(&command[0])
            .args(&command[1..])
            .status()?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        Ok(())
    }
}
