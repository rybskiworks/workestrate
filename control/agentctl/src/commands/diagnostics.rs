//! Inspection and diagnostics commands: `plan`, `ps`, `check`,
//! `validate-config`, `generate-schema`, `generate-env-example`, and the
//! generic `run` exec entry.

use std::path::PathBuf;

use anyhow::Result;

use crate::commands::source::build_env_var_name;
use crate::config;
use crate::config::CheckEntry;
use crate::json_out::ps_entries_json;

pub fn cmd_plan<W: crate::microsandbox::workload::Workload>(
    workload: &W,
    show_source: bool,
    json: bool,
    instance: Option<&str>,
    use_values: &[String],
) -> Result<()> {
    use crate::microsandbox::slots::{instance_name, slot_for, validate_instance_id};

    // ADR 0026(d): depends_on resolution happens INSIDE the workload's
    // construction (every up/exec/plan path), so `workload.plan()` already
    // reflects the overrides the caller constructed it with. This parser
    // entry point (the workload catch-all) takes the RAW `--use` values and
    // applies the selection here instead — the plan render itself is
    // unchanged: the overrides only change WHICH record resolution selected.
    let plan_holder = if use_values.is_empty() {
        None
    } else {
        let overrides = crate::microsandbox::discovery::parse_use_overrides(use_values)?;
        Some(
            crate::microsandbox::workload::ConfigWorkload::new_with_use_overrides(
                workload.name(),
                &overrides,
            )?,
        )
    };
    let effective: &dyn crate::microsandbox::workload::Workload = match plan_holder.as_ref() {
        Some(w) => w,
        None => workload,
    };

    // ADR 0026/C2: `--instance <id>` renders the plan as the parallel slot
    // `<slot>@<id>` would see it — the instance name plus the prospective
    // per-instance bind IP on every published port. The prospective bind is
    // a READ-ONLY registry snapshot (no lock, no reservation): a preview of
    // what `up --instance <id>` would draw right now.
    let mut plan = effective.plan();
    if let Some(id) = instance {
        validate_instance_id(id)?;
        let slot = slot_for(workload.name(), config::active_context_name().as_deref());
        plan.name = instance_name(&slot, Some(id));
        let state_dir = config::resolve_state_dir();
        let bind = crate::microsandbox::port_registry::prospective_loopback_ip(&state_dir)?;
        for p in &mut plan.ports {
            p.bind_ip = bind;
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else if show_source {
        if instance.is_some() {
            // The prospective parallel view has no per-field source
            // annotation; render the plain plan (source view stays the
            // singleton's).
            println!("{}", plan);
        } else {
            println!("{}", workload.show_source());
        }
    } else {
        println!("{}", plan);
    }
    Ok(())
}

/// One row of the `workestrate workloads` listing (ADR 0027): the configured
/// workload name, its kind, a short image summary, and the instance names
/// currently registered in the port registry (empty = not running).
#[derive(Debug, Clone)]
pub struct WorkloadListEntry {
    pub name: String,
    pub kind: String,
    pub image: String,
    pub instances: Vec<String>,
}

/// Short image summary for the listing: `<recipe>:<ref>`, falling back to
/// `name[:tag]`, then the bare recipe when no ref/name/tag is set.
fn image_summary(image: &crate::config::ImageSpec) -> String {
    let detail = image
        .reference
        .clone()
        .or_else(|| match (&image.name, &image.tag) {
            (Some(n), Some(t)) => Some(format!("{n}:{t}")),
            (Some(n), None) => Some(n.clone()),
            (None, Some(t)) => Some(t.clone()),
            (None, None) => None,
        });
    match detail {
        Some(d) => format!("{}:{}", image.recipe, d),
        None => image.recipe.clone(),
    }
}

/// `workestrate workloads` (ADR 0027): list every configured workload with
/// its kind, image summary, and running status (instances registered in the
/// port registry). Deterministic order: sorted by workload name (the config
/// workloads map is unordered). Running status is registry-based (no msb
/// liveness probe) — this is a discovery view, not a health check.
pub fn cmd_workloads(json: bool) -> Result<()> {
    let cfg = config::load_config()?;
    let state_dir = config::resolve_state_dir();
    let records = crate::microsandbox::port_registry::list_records(&state_dir)?;

    let mut names: Vec<&String> = cfg.workloads.keys().collect();
    names.sort();
    let entries: Vec<WorkloadListEntry> = names
        .into_iter()
        .map(|name| {
            let wl = &cfg.workloads[name];
            let mut instances: Vec<String> = records
                .iter()
                .filter(|r| &r.workload == name)
                .map(|r| r.instance.clone())
                .collect();
            instances.sort();
            WorkloadListEntry {
                name: name.clone(),
                kind: wl.kind.clone(),
                image: image_summary(&wl.image),
                instances,
            }
        })
        .collect();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&crate::json_out::workloads_json(&entries))?
        );
    } else {
        print_workloads_text_to(&entries, &mut std::io::stdout())?;
    }
    Ok(())
}

/// Render `workloads` rows to `out`. Pure I/O: no env, no registry.
/// `cmd_workloads` passes `&mut std::io::stdout()`; tests pass a `Vec<u8>`.
pub fn print_workloads_text_to<W: std::io::Write>(
    entries: &[WorkloadListEntry],
    out: &mut W,
) -> std::io::Result<()> {
    if entries.is_empty() {
        writeln!(out, "(no configured workloads)")?;
        return Ok(());
    }
    for e in entries {
        let running = if e.instances.is_empty() {
            "(none running)".to_string()
        } else {
            format!("running: {}", e.instances.join(", "))
        };
        writeln!(out, "{}\t{}\t{}\t{}", e.name, e.kind, e.image, running)?;
    }
    Ok(())
}

pub async fn cmd_ps(json: bool) -> Result<()> {
    use crate::microsandbox::runtime::{probe_liveness, ps};
    let state_dir = crate::config::resolve_state_dir();
    let mut entries = ps(&state_dir)?;
    // Best-effort liveness probe (ADR 0021 §4). ps() stays pure; this is the
    // only place ps rows touch msb. When the msb DB is unreachable we cannot
    // distinguish running from stale, so emit one honest stderr note rather
    // than silently trusting the registry records.
    let (unreachable, unknown) = probe_liveness(&mut entries).await;
    if unreachable > 0 {
        eprintln!(
            "warning: could not verify liveness of {n} instance(s) via msb (db unreachable); stale flags may be inaccurate",
            n = unreachable,
        );
    }
    // FS-7: unexpected (non-reachability) probe errors were already noted
    // per-instance on stderr by probe_liveness; one aggregate line here makes
    // the count visible without repeating the per-instance detail.
    if unknown > 0 {
        eprintln!(
            "warning: liveness probe returned an unexpected error for {n} instance(s) (see notes above); stale flags left unchanged",
            n = unknown,
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

pub fn print_ps_text(entries: &[crate::microsandbox::runtime::PsEntry]) -> Result<()> {
    // Thin stdout wrapper over the writer-injectable renderer, so the footer
    // text and table layout are unit-testable without capturing global stdout.
    print_ps_text_to(entries, &mut std::io::stdout())?;
    Ok(())
}

/// Render `ps` rows (table + stale-remediation footer) to `out`. Pure I/O:
/// no msb, no env. `cmd_ps` passes `&mut std::io::stdout()`; tests pass a
/// `Vec<u8>`. Returns io::Error on write failure (propagated as anyhow).
pub fn print_ps_text_to<W: std::io::Write>(
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
        // ADR 0026/C3: a pair renders `<bind_ip>:<host>:<guest>` when bound
        // on a non-default bind (parallel slot), else the legacy
        // `<host>:<guest>` (singleton rows stay clean). Matches the plan
        // Display rule (C2).
        let ports_str = e
            .ports
            .iter()
            .map(|p| {
                if p.bind_ip == crate::microsandbox::plan::default_bind_ip() {
                    format!("{}:{}", p.host, p.guest)
                } else {
                    format!("{}:{}:{}", p.bind_ip, p.host, p.guest)
                }
            })
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

pub fn cmd_check() -> Result<()> {
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
                    for status in crate::git::collect_repo_statuses(&registry) {
                        let entry = &registry.configs[&status.name];
                        let (dirty_label, ok) = if status.exists {
                            if status.dirty {
                                ("dirty", false)
                            } else {
                                ("clean", true)
                            }
                        } else {
                            ("missing", false)
                        };
                        let git_ref = entry.r#ref.as_deref().unwrap_or("main");
                        let mark = if ok { "[OK]" } else { "[MISSING]" };
                        println!(
                            "    {}: {} (ref {}, rev {}, {}) {}",
                            status.name, entry.url, git_ref, status.short, dirty_label, mark
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
            ("config-repos", home.join("config-repos")),
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

pub fn find_reference_config() -> Option<PathBuf> {
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

pub fn cmd_validate_config() -> Result<()> {
    let config = config::load_config()?;
    config::validate_config(&config)?;
    println!("workestrate.toml is valid.");
    Ok(())
}

pub fn cmd_generate_schema(out: Option<&std::path::Path>) -> Result<()> {
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

pub fn cmd_generate_env_example(output: Option<&std::path::Path>) -> Result<()> {
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

pub fn cmd_run(command: &[String]) -> Result<()> {
    if command.is_empty() {
        anyhow::bail!("no command specified. Usage: workestrate run -- <command> [args...]");
    }

    // Load secrets from .env.enc (generic — all keys, no filtering).
    let secrets = crate::microsandbox::secrets_loader::load_secrets()?;
    // FN-9 scoped set_var: this command `exec(2)` REPLACES the workestrate
    // process with the user command below, so the secrets can only reach it
    // via inherited process env. This is the one consumer that fundamentally
    // requires process env; everywhere else resolves from the returned map.
    crate::microsandbox::secrets_loader::apply_secrets_to_process_env(&secrets);

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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;

    /// `ps --json` must emit the exact shape pinned by ADR 0021 §7, in the
    /// documented field order: instance, workload, context, slot, kind,
    /// started_at, ports, stale.
    /// Pure: no env, no msb — constructs PsEntry rows directly and round-trips
    /// them through `ps_entries_json` + `serde_json::to_string_pretty`.
    #[test]
    fn ps_json_matches_adr_0021_section_7() {
        use crate::microsandbox::plan::PortMapping;
        use crate::microsandbox::runtime::{PsEntry, PsKind};

        let singleton = PsEntry {
            instance: "personal-litellm".to_string(),
            workload: "litellm".to_string(),
            context: Some("personal".to_string()),
            slot: "personal-litellm".to_string(),
            kind: PsKind::Singleton,
            ports: vec![PortMapping::new(4000, 4000)],
            started_at: "2026-07-20T14:03:11Z".to_string(),
            stale: false,
        };
        let parallel = PsEntry {
            instance: "personal-litellm@canary".to_string(),
            workload: "litellm".to_string(),
            context: Some("personal".to_string()),
            slot: "personal-litellm".to_string(),
            kind: PsKind::Parallel,
            ports: vec![PortMapping {
                host: 14000,
                guest: 4000,
                bind_ip: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 2)),
            }],
            started_at: "2026-07-20T14:05:42Z".to_string(),
            stale: false,
        };

        let json = serde_json::to_string_pretty(&ps_entries_json(&[singleton, parallel]))
            .expect("serialize ps entries");

        // Field order, names, and casing (kind lowercase) are all pinned
        // here. Any drift from ADR 0021 §7 fails this snapshot. ADR 0026/C3:
        // `bind_ip` is an additive per-port field, always serialized (uniform
        // shape); the parallel row carries its per-instance 127.0.0.2 bind.
        let expected = r#"[
  {
    "instance": "personal-litellm",
    "workload": "litellm",
    "context": "personal",
    "slot": "personal-litellm",
    "kind": "singleton",
    "started_at": "2026-07-20T14:03:11Z",
    "ports": [
      {
        "host": 4000,
        "guest": 4000,
        "bind_ip": "127.0.0.1"
      }
    ],
    "stale": false
  },
  {
    "instance": "personal-litellm@canary",
    "workload": "litellm",
    "context": "personal",
    "slot": "personal-litellm",
    "kind": "parallel",
    "started_at": "2026-07-20T14:05:42Z",
    "ports": [
      {
        "host": 14000,
        "guest": 4000,
        "bind_ip": "127.0.0.2"
      }
    ],
    "stale": false
  }
]"#;
        assert_eq!(json, expected, "ps --json shape drifted from ADR 0021 §7");
    }

    /// Stale-footer remediation text (ADR 0021 §4). When any entry is stale,
    /// `print_ps_text` emits per-instance `down` commands and a catch-all
    /// `down --all`. Exercises the writer-injectable renderer directly so the
    /// footer text is pinned verbatim (not via global-stdout capture).
    #[test]
    fn print_ps_text_emits_remediation_footer_for_stale_entries() {
        use crate::microsandbox::plan::PortMapping;
        use crate::microsandbox::runtime::{PsEntry, PsKind};

        // One stale parallel instance + one live singleton. Sorted output
        // orders litellm (l) before pi (p).
        let entries = vec![
            PsEntry {
                instance: "personal-litellm@canary".to_string(),
                workload: "litellm".to_string(),
                context: Some("personal".to_string()),
                slot: "personal-litellm".to_string(),
                kind: PsKind::Parallel,
                ports: vec![PortMapping::new(14000, 4000)],
                started_at: "2026-07-20T14:05:42Z".to_string(),
                stale: true,
            },
            PsEntry {
                instance: "personal-pi".to_string(),
                workload: "pi".to_string(),
                context: Some("personal".to_string()),
                slot: "personal-pi".to_string(),
                kind: PsKind::Singleton,
                ports: vec![PortMapping::new(3000, 3000)],
                started_at: "2026-07-20T14:06:00Z".to_string(),
                stale: false,
            },
        ];

        let mut buf: Vec<u8> = Vec::new();
        print_ps_text_to(&entries, &mut buf).expect("render ps text");
        let out = String::from_utf8(buf).expect("utf8");

        // Footer header (verbatim).
        assert!(
            out.contains(
                "1 stale instance(s): registry record exists but the sandbox is not running."
            ),
            "stale footer header missing; got:
{out}"
        );
        // Parallel stale entry → `down --instance <id>` (id, not full instance).
        assert!(
            out.contains("  workestrate litellm down --instance canary"),
            "parallel stale remediation line missing; got:
{out}"
        );
        // Catch-all.
        assert!(
            out.contains("Remove every instance with: workestrate down --all"),
            "catch-all remediation line missing; got:
{out}"
        );
        // The live singleton must NOT appear in any teardown line.
        assert!(
            !out.contains("workestrate pi down"),
            "live singleton leaked into the footer; got:
{out}"
        );
        // Column header renamed CREATED → STARTED (ADR §7 started_at).
        assert!(
            out.contains("STARTED"),
            "column header should be STARTED (renamed from CREATED); got:
{out}"
        );
    }

    /// ADR 0026/C3: the text PORTS column renders `<bind_ip>:<host>:<guest>`
    /// for a pair bound on a non-default bind (parallel slot), and keeps the
    /// legacy `<host>:<guest>` for default-bind pairs (singleton rows stay
    /// clean). Both JSON (`bind_ip` field) and text surfaces are pinned.
    #[test]
    fn ps_text_and_json_render_bind_ip_for_non_default_binds() {
        use crate::microsandbox::plan::PortMapping;
        use crate::microsandbox::runtime::{PsEntry, PsKind};

        let entries = vec![
            PsEntry {
                instance: "personal-litellm".to_string(),
                workload: "litellm".to_string(),
                context: Some("personal".to_string()),
                slot: "personal-litellm".to_string(),
                kind: PsKind::Singleton,
                ports: vec![PortMapping::new(4000, 4000)],
                started_at: "2026-07-20T14:03:11Z".to_string(),
                stale: false,
            },
            PsEntry {
                instance: "personal-litellm@canary".to_string(),
                workload: "litellm".to_string(),
                context: Some("personal".to_string()),
                slot: "personal-litellm".to_string(),
                kind: PsKind::Parallel,
                ports: vec![PortMapping {
                    host: 4000,
                    guest: 4000,
                    bind_ip: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 2)),
                }],
                started_at: "2026-07-20T14:05:42Z".to_string(),
                stale: false,
            },
        ];

        // Text: non-default bind renders the three-field form; the default
        // bind keeps the two-field form and never prints 127.0.0.1.
        let mut buf: Vec<u8> = Vec::new();
        print_ps_text_to(&entries, &mut buf).expect("render ps text");
        let out = String::from_utf8(buf).expect("utf8");
        assert!(
            out.contains("127.0.0.2:4000:4000"),
            "non-default bind must render <bind_ip>:<host>:<guest>; got:\n{out}"
        );
        assert!(
            !out.contains("127.0.0.1:4000:4000"),
            "default bind must NOT print the bind IP; got:\n{out}"
        );

        // JSON: both rows carry the additive bind_ip field.
        let json =
            serde_json::to_string_pretty(&ps_entries_json(&entries)).expect("serialize ps entries");
        assert!(
            json.contains(r#""bind_ip": "127.0.0.1""#),
            "singleton port must serialize bind_ip 127.0.0.1; got:\n{json}"
        );
        assert!(
            json.contains(r#""bind_ip": "127.0.0.2""#),
            "parallel port must serialize bind_ip 127.0.0.2; got:\n{json}"
        );
    }

    /// No-stale case: the footer is entirely absent. Pins the negative branch.
    #[test]
    fn print_ps_text_omits_footer_when_nothing_stale() {
        use crate::microsandbox::plan::PortMapping;
        use crate::microsandbox::runtime::{PsEntry, PsKind};

        let entries = vec![PsEntry {
            instance: "personal-litellm".to_string(),
            workload: "litellm".to_string(),
            context: Some("personal".to_string()),
            slot: "personal-litellm".to_string(),
            kind: PsKind::Singleton,
            ports: vec![PortMapping::new(4000, 4000)],
            started_at: "2026-07-20T14:03:11Z".to_string(),
            stale: false,
        }];
        let mut buf: Vec<u8> = Vec::new();
        print_ps_text_to(&entries, &mut buf).expect("render ps text");
        let out = String::from_utf8(buf).expect("utf8");
        assert!(
            !out.contains("stale"),
            "no footer expected when nothing stale; got:
{out}"
        );
    }

    // ---- WP5 / E1 regression: graceful check outside a workbench checkout ----

    #[test]
    fn project_root_optional_returns_none_outside_workbench() {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let old_root = std::env::var("AGENTCTL_ROOT").ok();
        let old_manifest = std::env::var("CARGO_MANIFEST_DIR").ok();
        let tmp = std::env::temp_dir().join(format!(
            "no-flake-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp).ok();
        std::env::set_var("AGENTCTL_ROOT", &tmp);
        std::env::remove_var("CARGO_MANIFEST_DIR");

        let result = config::project_root_optional();

        match old_root {
            Some(v) => std::env::set_var("AGENTCTL_ROOT", v),
            None => std::env::remove_var("AGENTCTL_ROOT"),
        }
        match old_manifest {
            Some(v) => std::env::set_var("CARGO_MANIFEST_DIR", v),
            None => std::env::remove_var("CARGO_MANIFEST_DIR"),
        }

        // The contract: project_root_optional NEVER returns a path that
        // lacks flake.nix. It returns Some verified-path or None.
        match result {
            None => { /* expected when not in a workbench */ }
            Some(p) => {
                assert!(
                    p.join("flake.nix").exists(),
                    "project_root_optional returned '{:?}' which lacks flake.nix",
                    p
                );
            }
        }
    }

    /// E1 integration: simulate a fresh-install `workestrate check` from /tmp
    /// with a tmp HOME and no workbench checkout. The check must NOT error
    /// just because no workbench checkout is reachable; required-files prints
    /// "(not in a workbench checkout — skipped)".
    #[test]
    fn cmd_check_degrades_gracefully_outside_workbench() -> Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();

        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-e1-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp_home)?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg_config = std::env::var("XDG_CONFIG_HOME").ok();
        let old_xdg_data = std::env::var("XDG_DATA_HOME").ok();
        let old_root = std::env::var("AGENTCTL_ROOT").ok();
        let old_manifest = std::env::var("CARGO_MANIFEST_DIR").ok();
        let old_no_project = std::env::var("WORKESTRATE_NO_PROJECT_CONFIG").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::set_var(
            "XDG_DATA_HOME",
            tmp_home
                .join(".local")
                .join("share")
                .to_string_lossy()
                .as_ref(),
        );
        std::env::set_var("AGENTCTL_ROOT", &tmp_home);
        std::env::remove_var("CARGO_MANIFEST_DIR");
        std::env::set_var("WORKESTRATE_NO_PROJECT_CONFIG", "1");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let result = cmd_check();

        for (k, v) in [
            ("HOME", old_home),
            ("XDG_CONFIG_HOME", old_xdg_config),
            ("XDG_DATA_HOME", old_xdg_data),
            ("AGENTCTL_ROOT", old_root),
            ("CARGO_MANIFEST_DIR", old_manifest),
            ("WORKESTRATE_NO_PROJECT_CONFIG", old_no_project),
            ("WORKESTRATE_CONFIG_DIR", old_config_dir),
        ] {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        // E1 contract: cmd_check must NOT error just because no workbench
        // checkout is reachable. It may error for OTHER reasons (e.g. a
        // missing required artifact) but not for project_root being absent.
        if let Err(e) = &result {
            let msg = e.to_string();
            assert!(
                !msg.contains("flake.nix") && !msg.contains("project root"),
                "E1 regression: cmd_check errored on project_root: {msg}"
            );
        }
        Ok(())
    }
}
