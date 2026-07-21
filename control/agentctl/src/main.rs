use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use std::io::Write;
use std::path::PathBuf;

mod config;
mod merge;
mod microsandbox;
mod policy;
mod recipes;
mod scaffold;

use config::CheckEntry;
use microsandbox::workload::{ConfigWorkload, Workload};

#[derive(Parser)]
#[command(name = "workestrate")]
#[command(about = "Control plane CLI for the AI workbench")]
#[command(version)]
struct Cli {
    #[arg(long, help = "Disable project-layer config loading")]
    no_project_config: bool,

    #[arg(long, global = true, help = "Show source layer for each plan field")]
    show_source: bool,

    #[arg(long, global = true, help = "Active context name")]
    context: Option<String>,

    /// Emit machine-readable JSON to stdout and an error envelope to stderr
    /// on failure. Applies to: ps, plan, config list, down (per-instance
    /// results), and the up/exec/down error envelope.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

/// Actions available on service workloads (headless, detached by default).
#[derive(Subcommand)]
enum ServiceAction {
    /// Start the sandbox (detached by default; --foreground to block)
    Up {
        #[arg(short, long, help = "Run in foreground (block until Ctrl-C)")]
        foreground: bool,

        /// Tear down any existing instance at this slot before starting
        /// (destructive; the ADR 0021 explicit-replace escape hatch).
        #[arg(long)]
        replace: bool,

        /// Target a parallel instance `<slot>@<id>`. Refuses if that exact
        /// instance name is already running.
        #[arg(long, value_name = "ID")]
        instance: Option<String>,

        /// Auto-allocate the lowest free integer id >= 2 and target
        /// `<slot>@<id>`.
        #[arg(long)]
        new: bool,

        /// Add N to every HOST port (guest ports unchanged).
        #[arg(long, default_value_t = 0, value_name = "N")]
        port_offset: u16,
    },
    /// Stop and remove the sandbox
    Down {
        /// Stop the parallel instance `<slot>@<id>`.
        #[arg(long, value_name = "ID")]
        instance: Option<String>,

        /// Stop the singleton AND every parallel instance of this workload.
        #[arg(long)]
        all_instances: bool,
    },
    /// Tail the detached service's log file
    Logs,
    /// Print the planned sandbox workload
    Plan {
        /// Add N to every HOST port in the displayed plan (mirrors --port-offset on up).
        #[arg(long, default_value_t = 0, value_name = "N")]
        port_offset: u16,
    },
}

/// Actions available on agent workloads (interactive TUI attach).
#[derive(Subcommand)]
enum AgentAction {
    /// Attach to the sandbox interactively (TUI)
    Exec {
        /// Tear down any existing instance at this slot before starting.
        #[arg(long)]
        replace: bool,

        /// Target a parallel instance `<slot>@<id>`.
        #[arg(long, value_name = "ID")]
        instance: Option<String>,

        /// Auto-allocate the lowest free integer id >= 2.
        #[arg(long)]
        new: bool,

        /// Add N to every HOST port (guest ports unchanged).
        #[arg(long, default_value_t = 0, value_name = "N")]
        port_offset: u16,
    },
    /// Stop and remove the sandbox
    Down {
        #[arg(long, value_name = "ID")]
        instance: Option<String>,
        #[arg(long)]
        all_instances: bool,
    },
    /// Print the planned sandbox workload
    Plan {
        #[arg(long, default_value_t = 0, value_name = "N")]
        port_offset: u16,
    },
}

/// Actions for managing config repositories and trust.
#[derive(Subcommand)]
enum ConfigAction {
    /// Clone a config repo into the managed store and register it
    Add {
        url: String,
        name: String,
        #[arg(long, default_value = "main")]
        r#ref: String,
    },
    /// Pull latest for a config repo (or all) and update rev in registry
    Update { name: Option<String> },
    /// List registered config repos with rev + dirty status
    List,
    /// Trust a project directory for project-layer config loading
    Trust { dir: String },
    /// Remove trust from a project directory
    Untrust { dir: String },
    /// Scaffold a new config repo locally (minimal valid workestrate.toml
    /// + SOPS + README). Replaces the copier template for the
    /// minimal-personal subset; writes a `.copier-answers.yml` sidecar so
    /// `copier update` stays usable for richer features (team keys, flake).
    New {
        /// Name for the new config repo (e.g. "personal", "work").
        name: String,

        /// Destination directory (default: ./<name>).
        #[arg(long, value_name = "DIR")]
        path: Option<std::path::PathBuf>,

        /// Age public recipient (age1...). If omitted, derived via
        /// `age-keygen -y` from `--age-key-file` (default
        /// `~/.config/sops/age/ai-workbench-secrets.txt`). Falls back to
        /// `age1PLACEHOLDER` + warning if derivation fails.
        #[arg(long, value_name = "KEY")]
        age_recipient: Option<String>,

        /// Override the age key file to derive the recipient from.
        #[arg(long, value_name = "PATH")]
        age_key_file: Option<std::path::PathBuf>,

        /// Include a flake.nix for inverted-dependency image builds
        /// (Phase 2).
        #[arg(long)]
        with_flake: bool,

        /// URL of the workestrator core flake (only used with --with-flake).
        #[arg(
            long,
            value_name = "URL",
            default_value = "github:georgrybski/ai-workbench"
        )]
        core_flake_url: String,

        /// Skip registering the new repo in the workestrate registry.
        #[arg(long)]
        no_register: bool,

        /// Skip `git init` in the new directory.
        #[arg(long)]
        no_git_init: bool,

        /// Seed workestrate.toml from config.reference/workestrate.toml
        /// (full 5-workload fixture). Conflicts with --empty.
        #[arg(long, conflicts_with = "empty")]
        from_reference: bool,

        /// Write only a minimal workestrate.toml (no secrets/sops/readme).
        /// Conflicts with --from-reference.
        #[arg(long, conflicts_with = "from_reference")]
        empty: bool,
    },
}

/// Actions for managing agent source checkouts.
#[derive(Subcommand)]
enum SourceAction {
    /// Clone agent source into the managed store
    Clone {
        name: String,
        /// Optional path (defaults to sources/<name>/repo/)
        path: Option<String>,
    },
    /// Build agent from source using the workload's local_build recipe
    Build { name: String },
    /// List agent source checkouts with status
    List,
    /// Reset agent source to canonical (discard local edits)
    Reset { name: String },
}

#[derive(Subcommand)]
enum Commands {
    /// Runtime/config sanity check
    Check,
    /// Initialize workestrate configuration
    Init {
        /// Optional dotfiles repo URL to clone as the registry source
        url: Option<String>,
    },
    /// Scaffold a new agent project
    New {
        /// Name for the new agent (e.g., "my-agent")
        name: String,
    },
    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
        #[arg(
            long = "for",
            value_name = "NAME",
            default_value = "workestrate",
            help = "Command name to generate completions for"
        )]
        for_name: String,
    },
    /// Run an arbitrary command with decrypted secrets
    Run {
        /// Command and arguments (after --)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 1..)]
        command: Vec<String>,
    },
    /// Validate active config against schema and policy allowlists
    ValidateConfig,
    /// Print the env_var names of all secrets defined in config
    SecretsSchema,
    /// Generate a .env.example from the config secrets section
    GenerateEnvExample {
        /// Write output to a file instead of stdout
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
    /// List running workestrate sandbox instances (reads the port registry).
    /// Use --json for machine-readable output.
    Ps,
    /// Stop every running workestrate sandbox across all workloads/contexts.
    /// Destructive; confirms unless --yes.
    DownAll {
        #[arg(long, help = "Skip the interactive confirmation")]
        yes: bool,
    },
    /// Print the JSON Schema for workestrate.toml to stdout (or write to --out).
    /// The schema is generated from the same serde/schemars types the config
    /// loader uses (single source of truth; ADR 0021 §8).
    GenerateSchema {
        /// Write the schema to this path instead of stdout.
        #[arg(short, long, value_name = "PATH")]
        out: Option<std::path::PathBuf>,
    },
    /// Manage config repositories and trusted projects
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Manage agent source checkouts
    Source {
        #[command(subcommand)]
        action: SourceAction,
    },
    /// Typed subcommand for the LiteLLM proxy service
    Litellm {
        #[command(subcommand)]
        action: ServiceAction,
    },
    /// Typed subcommand for the Pi coding agent
    Pi {
        #[command(subcommand)]
        action: AgentAction,
    },
    /// Typed subcommand for the Odysseus service
    Odysseus {
        #[command(subcommand)]
        action: ServiceAction,
    },
    /// Typed subcommand for the OpenCode agent
    Opencode {
        #[command(subcommand)]
        action: AgentAction,
    },
    /// Typed subcommand for the T3MP3ST agent
    Tempest {
        #[command(subcommand)]
        action: AgentAction,
    },
    /// Catch-all for config-defined workloads
    #[command(external_subcommand)]
    Workload(Vec<String>),
}

/// Construct an InstanceSpec from the parsed CLI flags + the active context.
///
/// `workload_name` is the bare workload name (e.g. "litellm"). The slot is
/// derived from the active context. `instance_id` (from --instance) is
/// validated. `new_id` (from --new, already-allocated) is used as-is.
///
/// Mutually-exclusive flag groups (replace/instance/new) are validated here.
fn build_instance_spec(
    workload_name: &str,
    replace: bool,
    instance_id: Option<&str>,
    new_id: Option<u32>,
    port_offset: u16,
) -> Result<crate::microsandbox::runtime::InstanceSpec> {
    use crate::microsandbox::runtime::InstanceSpec;
    use crate::microsandbox::slots::{instance_name, slot_for, validate_instance_id};

    let exclusives = [replace, instance_id.is_some(), new_id.is_some()]
        .iter()
        .filter(|&&b| b)
        .count();
    if exclusives > 1 {
        anyhow::bail!(
            "--replace, --instance <id>, and --new are mutually exclusive; \
             pass at most one"
        );
    }

    let context = crate::config::active_context_name();
    let slot = slot_for(workload_name, context.as_deref());

    let id: Option<String> = if let Some(id) = instance_id {
        validate_instance_id(id)?;
        Some(id.to_string())
    } else {
        new_id.map(|n| n.to_string())
    };

    let instance = instance_name(&slot, id.as_deref());

    Ok(InstanceSpec {
        slot,
        instance,
        workload: workload_name.to_string(),
        context,
        port_offset,
        replace,
    })
}

async fn dispatch_service<W: Workload>(
    workload: &W,
    action: ServiceAction,
    show_source: bool,
    json: bool,
) -> Result<()> {
    if let Some(name) = config::active_context_name() {
        if !json {
            eprintln!("context: {}", name);
        }
    }
    match action {
        ServiceAction::Up {
            foreground,
            replace,
            instance,
            new,
            port_offset,
        } => {
            let new_id = if new {
                let state_dir = crate::config::resolve_state_dir();
                Some(
                    crate::microsandbox::port_registry::auto_allocate_integer_id(
                        &state_dir,
                        &crate::microsandbox::slots::slot_for(
                            workload.name(),
                            crate::config::active_context_name().as_deref(),
                        ),
                    )?,
                )
            } else {
                None
            };
            let spec = build_instance_spec(
                workload.name(),
                replace,
                instance.as_deref(),
                new_id,
                port_offset,
            )?;
            crate::microsandbox::runtime::up_service_with_spec(workload, &spec, foreground).await
        }
        ServiceAction::Down {
            instance,
            all_instances,
        } => cmd_down(workload.name(), instance.as_deref(), all_instances, json).await,
        ServiceAction::Logs => crate::microsandbox::logs(&workload.sandbox_instance_name()).await,
        ServiceAction::Plan { port_offset } => cmd_plan(workload, show_source, json, port_offset),
    }
}

async fn dispatch_agent<W: Workload>(
    workload: &W,
    action: AgentAction,
    show_source: bool,
    json: bool,
) -> Result<()> {
    if let Some(name) = config::active_context_name() {
        if !json {
            eprintln!("context: {}", name);
        }
    }
    match action {
        AgentAction::Exec {
            replace,
            instance,
            new,
            port_offset,
        } => {
            let new_id = if new {
                let state_dir = crate::config::resolve_state_dir();
                Some(
                    crate::microsandbox::port_registry::auto_allocate_integer_id(
                        &state_dir,
                        &crate::microsandbox::slots::slot_for(
                            workload.name(),
                            crate::config::active_context_name().as_deref(),
                        ),
                    )?,
                )
            } else {
                None
            };
            let spec = build_instance_spec(
                workload.name(),
                replace,
                instance.as_deref(),
                new_id,
                port_offset,
            )?;
            crate::microsandbox::runtime::exec_agent_with_spec(workload, &spec).await
        }
        AgentAction::Down {
            instance,
            all_instances,
        } => cmd_down(workload.name(), instance.as_deref(), all_instances, json).await,
        AgentAction::Plan { port_offset } => cmd_plan(workload, show_source, json, port_offset),
    }
}

fn parse_service_action(action: &str, args: &[String]) -> Result<ServiceAction> {
    match action {
        "up" => {
            let foreground = args.iter().any(|a| a == "--foreground");
            let replace = args.iter().any(|a| a == "--replace");
            let new = args.iter().any(|a| a == "--new");
            let instance = parse_flag_value(args, "--instance");
            let port_offset = parse_port_offset(args)?;
            Ok(ServiceAction::Up {
                foreground,
                replace,
                instance,
                new,
                port_offset,
            })
        }
        "down" => {
            let instance = parse_flag_value(args, "--instance");
            let all_instances = args.iter().any(|a| a == "--all-instances");
            Ok(ServiceAction::Down {
                instance,
                all_instances,
            })
        }
        "logs" => Ok(ServiceAction::Logs),
        "plan" => {
            let port_offset = parse_port_offset(args)?;
            Ok(ServiceAction::Plan { port_offset })
        }
        other => anyhow::bail!("unknown service action: {}", other),
    }
}

fn parse_agent_action(action: &str, args: &[String]) -> Result<AgentAction> {
    match action {
        "exec" => {
            let replace = args.iter().any(|a| a == "--replace");
            let new = args.iter().any(|a| a == "--new");
            let instance = parse_flag_value(args, "--instance");
            let port_offset = parse_port_offset(args)?;
            Ok(AgentAction::Exec {
                replace,
                instance,
                new,
                port_offset,
            })
        }
        "down" => {
            let instance = parse_flag_value(args, "--instance");
            let all_instances = args.iter().any(|a| a == "--all-instances");
            Ok(AgentAction::Down {
                instance,
                all_instances,
            })
        }
        "plan" => {
            let port_offset = parse_port_offset(args)?;
            Ok(AgentAction::Plan { port_offset })
        }
        other => anyhow::bail!("unknown agent action: {}", other),
    }
}

/// Extract the value of `--flag <value>` or `--flag=value` from a Vec<String>
/// (the workload catch-all args). Returns None if the flag is absent.
fn parse_flag_value(args: &[String], flag: &str) -> Option<String> {
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == flag {
            if let Some(v) = iter.next() {
                return Some(v.clone());
            }
        } else if let Some(rest) = a.strip_prefix(&format!("{}=", flag)) {
            return Some(rest.to_string());
        }
    }
    None
}

/// Parse `--port-offset <N>` (or `--port-offset=N`) from the workload
/// catch-all args. Defaults to 0 when absent.
fn parse_port_offset(args: &[String]) -> Result<u16> {
    let Some(raw) = parse_flag_value(args, "--port-offset") else {
        return Ok(0);
    };
    raw.parse::<u16>().map_err(|_| {
        anyhow::anyhow!(
            "invalid --port-offset value '{}' (expected u16 0..=65535)",
            raw
        )
    })
}

// ---------------------------------------------------------------------------
// JSON-output helpers (ps / down / plan / config list / generate-schema)
// ---------------------------------------------------------------------------

#[derive(serde::Serialize)]
struct DownResultJson {
    instance: String,
    status: &'static str,
    message: Option<String>,
}

fn down_result_json(r: &crate::microsandbox::runtime::DownResult) -> DownResultJson {
    // Inline conversion (avoids `expect()` on a 1->1 invariant; clippy
    // `expect_used` is deny in this crate).
    use crate::microsandbox::runtime::DownStatus;
    DownResultJson {
        instance: r.instance.clone(),
        status: match r.status {
            DownStatus::Stopped => "stopped",
            DownStatus::NotFound => "not_found",
            DownStatus::Error => "error",
        },
        message: r.message.clone(),
    }
}

fn down_results_json(results: &[crate::microsandbox::runtime::DownResult]) -> Vec<DownResultJson> {
    use crate::microsandbox::runtime::DownStatus;
    results
        .iter()
        .map(|r| DownResultJson {
            instance: r.instance.clone(),
            status: match r.status {
                DownStatus::Stopped => "stopped",
                DownStatus::NotFound => "not_found",
                DownStatus::Error => "error",
            },
            message: r.message.clone(),
        })
        .collect()
}

#[derive(serde::Serialize)]
struct PsPortJson {
    host: u16,
    guest: u16,
}

#[derive(serde::Serialize)]
struct PsEntryJson {
    instance: String,
    workload: String,
    context: Option<String>,
    ports: Vec<PsPortJson>,
    created: String,
}

fn ps_entries_json(entries: &[crate::microsandbox::runtime::PsEntry]) -> Vec<PsEntryJson> {
    entries
        .iter()
        .map(|e| PsEntryJson {
            instance: e.instance.clone(),
            workload: e.workload.clone(),
            context: e.context.clone(),
            ports: e
                .ports
                .iter()
                .map(|p| PsPortJson {
                    host: p.host,
                    guest: p.guest,
                })
                .collect(),
            created: e.created.clone(),
        })
        .collect()
}

async fn cmd_down(
    workload_name: &str,
    instance_id: Option<&str>,
    all_instances: bool,
    json: bool,
) -> Result<()> {
    use crate::microsandbox::runtime::{down_all_instances, down_instance};
    use crate::microsandbox::slots::{instance_name, slot_for, validate_instance_id};

    let context = crate::config::active_context_name();
    let slot = slot_for(workload_name, context.as_deref());

    if all_instances {
        let state_dir = crate::config::resolve_state_dir();
        let results = down_all_instances(&state_dir, workload_name).await?;
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&down_results_json(&results))?
            );
        } else {
            print_down_results_text(&results);
        }
        report_down_aggregate(&results)
    } else if let Some(id) = instance_id {
        validate_instance_id(id)?;
        let target = instance_name(&slot, Some(id));
        let state_dir = crate::config::resolve_state_dir();
        let result = down_instance(&state_dir, &target).await;
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&down_result_json(&result))?
            );
        } else {
            print_down_results_text(std::slice::from_ref(&result));
        }
        report_down_aggregate(std::slice::from_ref(&result))
    } else {
        // Singleton slot down (the legacy default).
        let state_dir = crate::config::resolve_state_dir();
        let result = down_instance(&state_dir, &slot).await;
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&down_result_json(&result))?
            );
        } else {
            print_down_results_text(std::slice::from_ref(&result));
        }
        report_down_aggregate(std::slice::from_ref(&result))
    }
}

fn print_down_results_text(results: &[crate::microsandbox::runtime::DownResult]) {
    use crate::microsandbox::runtime::DownStatus;
    for r in results {
        match r.status {
            DownStatus::Stopped => println!("{}: stopped", r.instance),
            DownStatus::NotFound => println!("{}: not found (state cleared)", r.instance),
            DownStatus::Error => println!(
                "{}: ERROR — {}",
                r.instance,
                r.message.as_deref().unwrap_or("(no detail)")
            ),
        }
    }
}

fn report_down_aggregate(results: &[crate::microsandbox::runtime::DownResult]) -> Result<()> {
    use crate::microsandbox::runtime::DownStatus;
    let had_error = results
        .iter()
        .any(|r| matches!(r.status, DownStatus::Error));
    if had_error {
        anyhow::bail!("one or more instances failed to stop");
    }
    Ok(())
}

fn cmd_plan<W: crate::microsandbox::workload::Workload>(
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

async fn cmd_ps(json: bool) -> Result<()> {
    use crate::microsandbox::runtime::ps;
    let state_dir = crate::config::resolve_state_dir();
    let entries = ps(&state_dir)?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&ps_entries_json(&entries))?
        );
    } else {
        print_ps_text(&entries);
    }
    Ok(())
}

fn print_ps_text(entries: &[crate::microsandbox::runtime::PsEntry]) {
    if entries.is_empty() {
        println!("(no running workestrate instances)");
        return;
    }
    // Stable column layout: INSTANCE | WORKLOAD | CONTEXT | PORTS | CREATED
    println!(
        "{:<32} {:<16} {:<12} {:<24} CREATED",
        "INSTANCE", "WORKLOAD", "CONTEXT", "PORTS"
    );
    let mut sorted: Vec<_> = entries.iter().collect();
    sorted.sort_by(|a, b| a.instance.cmp(&b.instance));
    for e in sorted {
        let ports_str = e
            .ports
            .iter()
            .map(|p| format!("{}:{}", p.host, p.guest))
            .collect::<Vec<_>>()
            .join(",");
        println!(
            "{:<32} {:<16} {:<12} {:<24} {}",
            e.instance,
            e.workload,
            e.context.clone().unwrap_or_else(|| "-".into()),
            ports_str,
            if e.created.is_empty() {
                "-"
            } else {
                &e.created
            },
        );
    }
}

async fn cmd_down_all(yes: bool, json: bool) -> Result<()> {
    use crate::microsandbox::runtime::{down_all, DownStatus};
    if !yes {
        // Best-effort interactive confirm: read a single y/Y from stdin.
        // Non-tty stdin → abort with a clear hint to pass --yes.
        use std::io::IsTerminal;
        if std::io::stdin().is_terminal() {
            eprint!("This will stop EVERY running workestrate sandbox. Continue? [y/N] ");
            let mut buf = String::new();
            use std::io::Read;
            std::io::stdin().read_to_string(&mut buf)?;
            if !buf.trim().eq_ignore_ascii_case("y") {
                if json {
                    eprintln!(
                        "{}",
                        serde_json::to_string(&serde_json::json!({
                            "error": {
                                "kind": "aborted",
                                "message": "down --all not confirmed"
                            }
                        }))?
                    );
                } else {
                    eprintln!("aborted");
                }
                std::process::exit(1);
            }
        } else {
            anyhow::bail!(
                "down --all requires an interactive tty for confirmation; \
                 pass --yes to skip"
            );
        }
    }
    let state_dir = crate::config::resolve_state_dir();
    let results = down_all(&state_dir).await?;
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&down_results_json(&results))?
        );
    } else {
        print_down_results_text(&results);
    }
    let had_error = results
        .iter()
        .any(|r| matches!(r.status, DownStatus::Error));
    if had_error {
        anyhow::bail!("one or more instances failed to stop");
    }
    Ok(())
}

fn cmd_generate_schema(out: Option<&std::path::Path>) -> Result<()> {
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

async fn cmd_config_list_json() -> Result<()> {
    // The committed JSON shape is a stable object mapping layer info; this is
    // a best-effort serialization of the in-memory Registry (or null when no
    // registry exists).
    match config::load_registry()? {
        None => {
            println!("null");
            Ok(())
        }
        Some(registry) => {
            println!("{}", serde_json::to_string_pretty(&registry)?);
            Ok(())
        }
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

/// Validate a workload name for `workestrate new`. Closes review finding A20.
///
/// Pattern: `^[a-z0-9][a-z0-9-]{0,62}$` — starts with an alphanumeric, allows
/// lowercase letters / digits / hyphens, max 63 characters (DNS-label length).
/// Rejects:
/// - empty / overlong names
/// - uppercase, underscores, dots, slashes, shell metacharacters
/// - anything starting with a hyphen (would create a hidden dir or flag-like
///   arg)
///
/// "Escape nothing — reject instead" is the policy: workload names flow into
/// both filesystem paths and TOML keys, so the safe set is the intersection.
fn validate_workload_name(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("workload name cannot be empty");
    }
    if name.len() > 63 {
        anyhow::bail!(
            "workload name cannot exceed 63 characters (got {}): '{}'",
            name.len(),
            name
        );
    }
    let mut chars = name.chars();
    let first_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    if !first_ok {
        anyhow::bail!(
            "workload name must start with [a-z0-9]; \
             pattern: ^[a-z0-9][a-z0-9-]{{0,62}}$; got: '{name}'"
        );
    }
    for c in chars {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' {
            anyhow::bail!(
                "workload name contains invalid character '{}' (allowed: [a-z0-9-]); \
                 pattern: ^[a-z0-9][a-z0-9-]{{0,62}}$; got: '{}'",
                c,
                name
            );
        }
    }
    Ok(())
}

async fn cmd_new(name: &str) -> Result<()> {
    // WP1 / A20: validate the workload name BEFORE using it as a directory
    // name or interpolating it into TOML. Reject everything that is not a
    // safe lowercase-hyphen identifier; this prevents both path escape
    // (`../pwned`) and TOML injection (`a]b` breaking out of the workload
    // table).
    validate_workload_name(name)?;

    // Resolve the active config directory (trusted project, registry, or env).
    let config_dir = config::resolve_active_config_dir()?;
    let agent_dir = config_dir.join("agents").join(name);

    if agent_dir.exists() {
        anyhow::bail!("agents/{} already exists in {}", name, config_dir.display());
    }

    // Create directory structure relative to the config repo.
    let config_subdir = agent_dir.join("config");
    std::fs::create_dir_all(&config_subdir)?;
    std::fs::write(config_subdir.join(".gitkeep"), "")?;

    // Append a default workload entry to the config repo's workestrate.toml.
    let config_path = config_dir.join("workestrate.toml");
    let toml_entry = format!(
        "\n[workloads.{}]\n\
        kind = \"agent\"\n\
        image = {{ recipe = \"registry\", ref = \"node:24-bookworm-slim\" }}\n\
        workdir = \"/work\"\n\
        cpus = 2\n\
        memory_mib = 2048\n\
        command = []\n\
        log_stop_errors = false\n\n\
        [[workloads.{}.mounts]]\n\
        host = \"${{CWD}}\"\n\
        guest = \"/work\"\n\
        read_only = false\n\n\
        [workloads.{}.network]\n\
        default_deny = true\n\n\
        [[workloads.{}.network.egress]]\n\
        recipe = \"agent_base\"\n",
        name, name, name, name
    );

    // Create parent dir if needed, then open with create+append.
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&config_path)?;
    file.write_all(toml_entry.as_bytes())?;

    println!(
        "Created agents/{}/config/ in {}",
        name,
        config_dir.display()
    );
    println!("Appended [workloads.{}] to {}", name, config_path.display());
    println!();
    println!("Edit {} to configure:", config_path.display());
    println!("  - Set image (recipe + ref, or recipe + contents for nix-layered)");
    println!("  - Set command");
    println!("  - Add env/secret_env/mounts as needed");
    println!();
    println!("Test: workestrate {} plan", name);

    Ok(())
}

// ---------------------------------------------------------------------------
// Git helpers
// ---------------------------------------------------------------------------

fn git_clone(url: &str, dest: &std::path::Path, branch: Option<&str>) -> Result<()> {
    let mut cmd = std::process::Command::new("git");
    cmd.args(["clone", "--depth", "1"]);
    if let Some(branch) = branch {
        cmd.args(["--branch", branch]);
    }
    let status = cmd.arg(url).arg(dest).status()?;
    if !status.success() {
        anyhow::bail!("git clone failed for {}", url);
    }
    Ok(())
}

/// Initialize a new git repo at `dir` (no commit, mirrors `cargo new`).
/// Used by `workestrate config new` to make the scaffold immediately
/// committable. Returns a distinct error kind when the `git` binary is
/// absent so the caller can warn-and-continue rather than fail the whole
/// scaffold (the files are already written and valid).
fn git_init(dir: &std::path::Path) -> Result<()> {
    let status = std::process::Command::new("git")
        .arg("init")
        .arg(dir)
        .status()
        .map_err(|e| anyhow::anyhow!("git binary not found: {}", e))?;
    if !status.success() {
        anyhow::bail!("git init failed in '{}'", dir.display());
    }
    Ok(())
}

fn git_rev_parse(repo: &std::path::Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("git rev-parse failed for {}", repo.display());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_is_dirty(repo: &std::path::Path) -> Result<bool> {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["diff", "--quiet", "HEAD"])
        .status()?;
    Ok(!status.success())
}

fn git_pull(repo: &std::path::Path, branch: &str) -> Result<()> {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["pull", "origin", branch])
        .status()?;
    if !status.success() {
        anyhow::bail!("git pull failed for {}", repo.display());
    }
    Ok(())
}

fn git_checkout_dot(repo: &std::path::Path) -> Result<()> {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["checkout", "."])
        .status()?;
    if !status.success() {
        anyhow::bail!("git checkout failed for {}", repo.display());
    }
    Ok(())
}

fn short_rev(rev: &str) -> String {
    rev.chars().take(7).collect()
}

// ---------------------------------------------------------------------------
// Config commands
// ---------------------------------------------------------------------------

async fn cmd_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Add { url, name, r#ref } => cmd_config_add(&url, &name, &r#ref).await,
        ConfigAction::Update { name } => cmd_config_update(name.as_deref()).await,
        ConfigAction::List => cmd_config_list().await,
        ConfigAction::Trust { dir } => cmd_config_trust(&dir).await,
        ConfigAction::Untrust { dir } => cmd_config_untrust(&dir).await,
        // `New` is dispatched directly in `async_main` so it can route the
        // global `--json` flag. Reaching this arm would be a regression.
        ConfigAction::New { .. } => {
            anyhow::bail!("config new must be dispatched from async_main (--json routing)")
        }
    }
}

async fn cmd_config_add(url: &str, name: &str, git_ref: &str) -> Result<()> {
    let dest = config::config_repo_dir(name);
    let (rev, short) = if dest.exists() {
        let git_dir = dest.join(".git");
        if !git_dir.exists() {
            anyhow::bail!(
                "config repo destination '{}' already exists and is not a git repo",
                dest.display()
            );
        }
        let rev = git_rev_parse(&dest)?;
        let short = short_rev(&rev);
        (rev, short)
    } else {
        let parent = dest
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid repo path: {}", dest.display()))?;
        std::fs::create_dir_all(parent)?;
        git_clone(url, &dest, Some(git_ref))?;
        let rev = git_rev_parse(&dest)?;
        let short = short_rev(&rev);
        (rev, short)
    };

    // Insert/replace the registry entry. Delegates to config::register_config
    // (shared with cmd_config_new's local-path registration).
    config::register_config(name, url, Some(git_ref), Some(rev.as_str()))?;
    println!(
        "Registered config repo {} from {} at {} (rev {})",
        name,
        url,
        dest.display(),
        short
    );
    Ok(())
}

/// `--json` output envelope for `workestrate config new`. Mirrors the
/// shape of other JSON-emitting commands (registry, ps): a single
/// pretty-printed object on stdout, errors on stderr.
#[derive(serde::Serialize)]
struct ConfigNewResult<'a> {
    name: &'a str,
    path: std::path::PathBuf,
    files_written: Vec<String>,
    git_initialized: bool,
    registered: bool,
    /// "flag" | "derived" | "placeholder"
    age_recipient_source: &'a str,
    age_recipient: &'a str,
    next_steps: Vec<&'a str>,
}

/// Resolve `~` in a path string via `$HOME` (no `dirs` crate dep). Falls
/// back to the literal path if `~/` prefix is absent or `$HOME` is unset.
fn expand_tilde(p: &std::path::Path) -> std::path::PathBuf {
    let s = p.to_string_lossy();
    if let Some(rest) = s.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return std::path::PathBuf::from(home).join(rest);
        }
    }
    p.to_path_buf()
}

/// Derive the age public recipient from a private key file via
/// `age-keygen -y <keyfile>`. Returns `Ok(recipient)` on success.
/// Returns `Err(message)` when either the `age-keygen` binary is missing
/// or the key file does not exist (so the caller can fall back to the
/// placeholder and surface a single, specific reason).
fn derive_age_recipient(key_file: &std::path::Path) -> Result<String> {
    if !key_file.exists() {
        anyhow::bail!("age key file not found at {}", key_file.display());
    }
    let output = std::process::Command::new("age-keygen")
        .arg("-y")
        .arg(key_file)
        .output()
        .map_err(|e| anyhow::anyhow!("age-keygen binary not found: {}", e))?;
    if !output.status.success() {
        anyhow::bail!(
            "age-keygen -y failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Walk up from `start` looking for `<root>/config.reference/workestrate.toml`.
/// Used by `--from-reference` to seed the scaffold with the canonical
/// 5-workload fixture. Returns the file path on success; bails with an
/// actionable message if not found within 10 levels.
fn find_reference_workestrate(start: &std::path::Path) -> Result<std::path::PathBuf> {
    let mut cursor = start.to_path_buf();
    for _ in 0..10 {
        let candidate = cursor.join("config.reference").join("workestrate.toml");
        if candidate.exists() {
            return Ok(candidate);
        }
        if !cursor.pop() {
            break;
        }
    }
    anyhow::bail!(
        "could not locate config.reference/workestrate.toml by walking up from '{}'; \
         run `workestrate config new --from-reference` from inside the ai-workbench checkout",
        start.display()
    );
}

#[allow(clippy::too_many_arguments)]
async fn cmd_config_new(
    name: &str,
    path: Option<&std::path::Path>,
    age_recipient: Option<&str>,
    age_key_file: Option<&std::path::Path>,
    with_flake: bool,
    core_flake_url: &str,
    no_register: bool,
    no_git_init: bool,
    from_reference: bool,
    empty: bool,
    json_mode: bool,
) -> Result<()> {
    // Validate name first so an invalid name fails before touching the
    // filesystem.
    config::validate_config_name(name)?;

    // Fail-fast: if we'd auto-register, check the registry now so we don't
    // write files + git init only to bail at the registration step.
    if !no_register {
        if let Some(reg) = config::load_registry()? {
            if reg.configs.contains_key(name) {
                anyhow::bail!(
                    "config repo '{}' already registered; use a different name or \
                     'workestrate config update {}'",
                    name,
                    name
                );
            }
        }
    }

    // Resolve destination directory.
    let dest: std::path::PathBuf = match path {
        Some(p) => p.to_path_buf(),
        None => std::path::PathBuf::from(name),
    };
    if dest.exists() {
        // Refuse if non-empty. An empty existing directory is OK (init in
        // a pre-created dir); a populated one likely means we'd clobber.
        let is_non_empty = std::fs::read_dir(&dest)
            .map(|mut it| it.next().is_some())
            .unwrap_or(false);
        if is_non_empty {
            anyhow::bail!(
                "destination '{}' exists and is non-empty; refusing to overwrite \
                 (remove it or pass a different --path)",
                dest.display()
            );
        }
    }

    // Resolve age recipient: explicit flag → derived → placeholder.
    let (recipient, recipient_source) = match age_recipient {
        Some(r) => (r.to_string(), "flag"),
        None => {
            let key_file = age_key_file.map(|p| expand_tilde(p)).unwrap_or_else(|| {
                expand_tilde(std::path::Path::new(scaffold::AGE_KEY_DEFAULT_PATH))
            });
            match derive_age_recipient(&key_file) {
                Ok(r) => (r, "derived"),
                Err(e) => {
                    eprintln!(
                        "warning: could not derive age recipient ({}); \
                         wrote 'age1PLACEHOLDER'.\n\
                         Set the real key with: edit .sops.yaml, OR re-run with \
                         --age-recipient <key>.\n\
                         Generate a key with: age-keygen -o ~/.config/sops/age/ai-workbench-secrets.txt",
                        e
                    );
                    ("age1PLACEHOLDER".to_string(), "placeholder")
                }
            }
        }
    };

    // Build the file set from the scaffold templates.
    let vars = scaffold::ScaffoldVars {
        config_name: name.to_string(),
        age_recipient: recipient.clone(),
        copier_src_path: scaffold::DEFAULT_COPIER_SRC_PATH.to_string(),
        copier_vcs_ref: scaffold::DEFAULT_COPIER_VCS_REF.to_string(),
        core_flake_url: if with_flake {
            Some(core_flake_url.to_string())
        } else {
            None
        },
    };

    let mut files: Vec<(String, String)> = if empty {
        // Minimal: just workestrate.toml (hand-written, no secrets) + .gitignore.
        vec![
            (
                "workestrate.toml".to_string(),
                format!(
                    "#:schema https://raw.githubusercontent.com/georgrybski/ai-workbench/main/schemas/workestrate.schema.json\n\n\
                     schema_version = 1\n\n\
                     # workestrator config: {name}\\
                     # Generated by `workestrate config new --empty`.\n\
                     # Add [secrets.*] and [workloads.*] tables here.\n"
                ),
            ),
            (
                ".gitignore".to_string(),
                scaffold::render(
                    include_str!("scaffold/template/.gitignore"),
                    &[],
                )?,
            ),
        ]
    } else {
        scaffold::render_all(&vars)?
            .into_iter()
            .map(|(n, c)| (n.to_string(), c))
            .collect()
    };

    // --from-reference overrides the workestrate.toml entry with the full
    // 5-workload reference fixture.
    if from_reference {
        let cwd = std::env::current_dir()?;
        let ref_path = find_reference_workestrate(&cwd)?;
        let ref_content = std::fs::read_to_string(&ref_path)?;
        let entry = files
            .iter_mut()
            .find(|(n, _)| n == "workestrate.toml")
            .ok_or_else(|| {
                anyhow::anyhow!("internal: workestrate.toml missing from scaffold file set")
            })?;
        entry.1 = ref_content;
    }

    // Create destination + write each file.
    std::fs::create_dir_all(&dest)?;
    for (rel, content) in &files {
        let full = dest.join(rel);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full, content)?;
    }

    // git init (hard-error on non-zero status; warn-and-continue if binary missing).
    let mut git_initialized = false;
    if !no_git_init {
        match git_init(&dest) {
            Ok(()) => git_initialized = true,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("git binary not found") {
                    eprintln!(
                        "warning: git not installed; skipping `git init` in '{}'. \
                         Re-run with --no-git-init to silence this.",
                        dest.display()
                    );
                } else {
                    return Err(e);
                }
            }
        }
    }

    // Register in workestrate registry.
    let mut registered = false;
    if !no_register {
        // The already-registered check ran above (fail-fast, before any
        // filesystem writes).
        let url = dest.canonicalize()?.to_string_lossy().to_string();
        // Local-path repos get ref=None, rev=None. cmd_config_update
        // recognizes this and skips the pull step.
        config::register_config(name, &url, None, None)?;
        registered = true;
    }

    // Build next-steps.
    let mut next_steps: Vec<&str> = Vec::with_capacity(4);
    if recipient_source == "placeholder" {
        next_steps.push("edit .sops.yaml and replace age1PLACEHOLDER with your real age1... key");
    }
    next_steps.push("cd into the new directory and edit workestrate.toml");
    if !no_register {
        next_steps.push(
            "to enable `workestrate config update`, push to a remote and edit the registry's url + ref",
        );
    }
    next_steps.push("run `workestrate validate-config` from inside the new repo");

    if json_mode {
        let result = ConfigNewResult {
            name,
            path: dest.clone(),
            files_written: files.iter().map(|(n, _)| n.clone()).collect(),
            git_initialized,
            registered,
            age_recipient_source: recipient_source,
            age_recipient: &recipient,
            next_steps,
        };
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // Human-readable summary.
    println!(
        "Created workestrator-config '{}' in {}",
        name,
        dest.display()
    );
    println!();
    println!("Files written:");
    for (rel, _) in &files {
        println!("  {}", rel);
    }
    if git_initialized {
        println!();
        println!(
            "git initialized (no initial commit; `git add . && git commit -m init` when ready)"
        );
    }
    if registered {
        println!();
        println!(
            "Registered as a local-path config repo (no remote yet; `workestrate config update` will skip)."
        );
    }
    println!();
    println!(
        "Age recipient: {} (source: {})",
        recipient, recipient_source
    );
    println!();
    println!("Next steps:");
    for step in next_steps {
        println!("  - {}", step);
    }
    Ok(())
}

async fn cmd_config_update(name: Option<&str>) -> Result<()> {
    let mut registry =
        config::load_registry()?.ok_or_else(|| anyhow::anyhow!("no config repos registered"))?;
    let names: Vec<String> = match name {
        Some(n) => {
            if !registry.configs.contains_key(n) {
                anyhow::bail!("config repo '{}' not found", n);
            }
            vec![n.to_string()]
        }
        None => registry.configs.keys().cloned().collect(),
    };

    for n in names {
        let dest = config::config_repo_dir(&n);
        // Local-path repos (created via `config new`) have no pinned rev and
        // typically no `origin` remote — `git pull` would fail. Skip them
        // with a forward-looking hint instead.
        let entry_ref = registry.configs.get(&n);
        let is_local_path = entry_ref.is_some_and(|e| e.rev.is_none());
        if is_local_path {
            println!(
                "config repo '{}' is a local path (no pinned rev); skipping update.\n\
                 To enable updates, push to a remote and edit the registry entry's url + ref.",
                n
            );
            continue;
        }
        if git_is_dirty(&dest)? {
            anyhow::bail!(
                "config repo '{}' has uncommitted changes; commit or stash first",
                n
            );
        }
        let git_ref = entry_ref
            .and_then(|e| e.r#ref.as_deref())
            .unwrap_or("main")
            .to_string();
        git_pull(&dest, &git_ref)?;
        let rev = git_rev_parse(&dest)?;
        let short = short_rev(&rev);
        registry
            .configs
            .get_mut(&n)
            .ok_or_else(|| anyhow::anyhow!("config repo '{}' disappeared", n))?
            .rev = Some(rev);
        println!("{}: updated to {}", n, short);
    }
    config::save_registry(&registry)?;
    Ok(())
}

async fn cmd_config_list() -> Result<()> {
    match config::load_registry()? {
        None => {
            println!("(no registry found; run 'workestrate init' or 'workestrate config add <url> <name>')");
        }
        Some(registry) => {
            println!("Config repos:");
            if registry.configs.is_empty() {
                println!("  (none)");
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
                        "  {}: {} (ref {}, rev {}, {}) {}",
                        name, entry.url, git_ref, short, dirty_label, status
                    );
                }
            }
            println!("Layers: {:?}", registry.layers);
            println!("Trusted projects:");
            if registry.trusted_projects.is_empty() {
                println!("  (none)");
            } else {
                for p in &registry.trusted_projects {
                    let path = PathBuf::from(&p.path);
                    let status = if path.exists() { "[OK]" } else { "[MISSING]" };
                    println!("  {} {}", p.path, status);
                }
            }
        }
    }
    Ok(())
}

async fn cmd_config_trust(dir: &str) -> Result<()> {
    let path = PathBuf::from(dir);
    config::trust_project(&path)?;
    println!("Trusted: {}", path.display());
    Ok(())
}

async fn cmd_config_untrust(dir: &str) -> Result<()> {
    let path = PathBuf::from(dir);
    config::untrust_project(&path)?;
    println!("Untrusted: {}", path.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// Init command
// ---------------------------------------------------------------------------

async fn cmd_init(url: Option<&str>) -> Result<()> {
    let registry_path = config::registry_path();
    if registry_path.exists() {
        println!(
            "Registry already exists at {}; use 'workestrate config add' to add repos",
            registry_path.display()
        );
        return Ok(());
    }

    let mut registry = config::Registry::default();
    registry.settings.default_context = Some("personal".to_string());

    std::fs::create_dir_all(config::xdg_data_dir())?;
    std::fs::create_dir_all(config::xdg_state_dir())?;

    if let Some(url) = url {
        let temp_dir =
            std::env::temp_dir().join(format!("workestrate-init-{}", std::process::id()));
        git_clone(url, &temp_dir, None)?;

        let found = if temp_dir.join("workestrate").join("config.toml").exists() {
            Some(temp_dir.join("workestrate").join("config.toml"))
        } else if temp_dir
            .join(".config")
            .join("workestrate")
            .join("config.toml")
            .exists()
        {
            Some(
                temp_dir
                    .join(".config")
                    .join("workestrate")
                    .join("config.toml"),
            )
        } else {
            None
        };

        let registry_parent = registry_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid registry path: {}", registry_path.display()))?;
        std::fs::create_dir_all(registry_parent)?;

        match found {
            Some(src) => {
                std::fs::copy(&src, &registry_path)?;
                println!(
                    "Cloned {} and copied workestrate config to {}",
                    url,
                    registry_path.display()
                );
            }
            None => {
                println!(
                    "Cloned {} but no workestrate config found; created empty registry",
                    url
                );
                config::save_registry(&registry)?;
            }
        }
        let _ = std::fs::remove_dir_all(&temp_dir);
    } else {
        config::save_registry(&registry)?;
    }

    println!(
        "Initialized workestrate registry at {}",
        registry_path.display()
    );
    println!("Run 'workestrate config add <url> personal' to add your config repo");
    Ok(())
}

// ---------------------------------------------------------------------------
// Source commands
// ---------------------------------------------------------------------------

async fn cmd_source(action: SourceAction) -> Result<()> {
    match action {
        SourceAction::Clone { name, path } => cmd_source_clone(&name, path.as_deref()).await,
        SourceAction::Build { name } => cmd_source_build(&name).await,
        SourceAction::List => cmd_source_list().await,
        SourceAction::Reset { name } => cmd_source_reset(&name).await,
    }
}

async fn cmd_source_clone(name: &str, path: Option<&str>) -> Result<()> {
    let cfg = config::load_config()?;
    let workload = cfg
        .workloads
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("workload '{}' not found", name))?;
    let local_build = workload
        .local_build
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("workload '{}' has no local_build recipe", name))?;

    let dest = match path {
        Some(p) => PathBuf::from(p),
        None => config::source_store_dir(name).join("repo"),
    };

    if local_build.source.starts_with("flake://") {
        let input = local_build
            .source
            .strip_prefix("flake://")
            .unwrap_or(&local_build.source);
        println!(
            "To clone the canonical source, run: nix develop (materializes flake inputs). Or clone manually to: {}",
            dest.display()
        );
        println!("Flake input name: {}", input);
    } else {
        if dest.exists() {
            anyhow::bail!("source path already exists: {}", dest.display());
        }
        let parent = dest
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid source path: {}", dest.display()))?;
        std::fs::create_dir_all(parent)?;
        git_clone(&local_build.source, &dest, None)?;
        println!("Cloned {} source to {}", name, dest.display());
    }

    let env_var = format!("WORKESTRATE_{}_BUILD", name.to_uppercase());
    println!("Set {}={} for this session", env_var, dest.display());
    Ok(())
}

fn build_command_string(name: &str, local_build: &config::LocalBuildConfig) -> String {
    match local_build.recipe.as_str() {
        "pip-install" | "pip" => {
            let target = local_build.target.as_deref().unwrap_or(".deps");
            let req = local_build
                .requirements_file
                .as_deref()
                .unwrap_or("requirements.txt");
            format!(
                "REQ=$([ -f requirements.lock ] && echo requirements.lock || echo {}) && python3.12 -m pip install --only-binary=:all: --break-system-packages --target ./{} -r \"$REQ\"",
                req, target
            )
        }
        "npm-build" | "npm" => "npm install && npm run build".to_string(),
        "bun-install" | "bun" => "HUSKY=0 bun install && bun run build".to_string(),
        other => format!("{} build recipe for {}", other, name),
    }
}

fn nix_available() -> bool {
    std::process::Command::new("nix")
        .arg("--version")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

async fn cmd_source_build(name: &str) -> Result<()> {
    let cfg = config::load_config()?;
    let workload = cfg
        .workloads
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("workload '{}' not found", name))?;
    let local_build = workload
        .local_build
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("workload '{}' has no local_build recipe", name))?;

    let command = build_command_string(name, local_build);
    println!("Build command for {}: {}", name, command);

    let env_var = format!("WORKESTRATE_{}_BUILD", name.to_uppercase());
    let build_dir = match std::env::var(&env_var) {
        Ok(path) => PathBuf::from(path),
        Err(_) => config::source_store_dir(name).join("build"),
    };

    let repo_dir = config::source_store_dir(name).join("repo");
    if repo_dir.exists() {
        if build_dir.exists() {
            std::fs::remove_dir_all(&build_dir)?;
        }
        if let Some(parent) = build_dir.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let status = std::process::Command::new("cp")
            .args([
                "-r",
                repo_dir.to_str().ok_or_else(|| {
                    anyhow::anyhow!(
                        "source repo path is not valid UTF-8: {}",
                        repo_dir.display()
                    )
                })?,
                build_dir.to_str().ok_or_else(|| {
                    anyhow::anyhow!(
                        "build directory path is not valid UTF-8: {}",
                        build_dir.display()
                    )
                })?,
            ])
            .status()?;
        if !status.success() {
            anyhow::bail!(
                "failed to copy source to build directory {}",
                build_dir.display()
            );
        }
    }

    if !build_dir.exists() {
        println!("Build directory {} does not exist.", build_dir.display());
        println!("Run: workestrate source clone {}", name);
        return Ok(());
    }

    if nix_available() {
        println!("Running build in {} via nix shell...", build_dir.display());
        let status = std::process::Command::new("nix")
            .args(["shell", ".", "--command", "bash", "-c", &command])
            .current_dir(&build_dir)
            .status()?;
        if !status.success() {
            anyhow::bail!("nix shell build failed for {}", name);
        }
        println!("Build succeeded: {}", build_dir.display());
    } else {
        println!("Nix not available. To build manually, run:");
        println!(
            "  cd {} && nix develop --command bash -c '{}'",
            build_dir.display(),
            command
        );
    }
    Ok(())
}

async fn cmd_source_list() -> Result<()> {
    let cfg = config::load_config()?;
    println!("Source overrides:");
    let mut found = false;
    for (name, workload) in &cfg.workloads {
        if workload.local_build.is_none() {
            continue;
        }
        found = true;
        let env_var = format!("WORKESTRATE_{}_BUILD", name.to_uppercase());
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
    Ok(())
}

async fn cmd_source_reset(name: &str) -> Result<()> {
    let repo = config::source_store_dir(name).join("repo");
    if !repo.exists() {
        anyhow::bail!(
            "source for '{}' not checked out at {}",
            name,
            repo.display()
        );
    }
    git_checkout_dot(&repo)?;
    println!("Reset {} source to canonical", name);
    Ok(())
}

fn main() {
    // Pre-scan argv for --json so we can format ANY error (incl. clap parse
    // errors via Cli::parse()) as the JSON envelope when requested. The
    // global --json on Cli does not help here because Cli::parse() calls
    // process::exit on usage errors before we'd see the parsed value.
    let json_mode = std::env::args().any(|a| a == "--json");

    let rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            emit_error(
                &anyhow::anyhow!("failed to build tokio runtime: {e}"),
                json_mode,
            );
            std::process::exit(1);
        }
    };
    let result = rt.block_on(async_main());
    match result {
        Ok(()) => {}
        Err(e) => {
            emit_error(&e, json_mode);
            std::process::exit(classify_exit_code(&e));
        }
    }
}

async fn async_main() -> Result<()> {
    let cli = Cli::parse();
    if cli.no_project_config {
        std::env::set_var("WORKESTRATE_NO_PROJECT_CONFIG", "1");
    }
    if let Some(ref ctx) = cli.context {
        std::env::set_var("WORKESTRATE_CONTEXT", ctx);
    }

    match cli.command {
        Commands::Check => cmd_check().await,
        Commands::Init { url } => cmd_init(url.as_deref()).await,
        Commands::New { name } => cmd_new(&name).await,
        Commands::Completions { shell, for_name } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, &for_name, &mut std::io::stdout());
            Ok(())
        }
        Commands::Run { command } => cmd_run(&command).await,
        Commands::ValidateConfig => cmd_validate_config().await,
        Commands::SecretsSchema => cmd_secrets_schema().await,
        Commands::GenerateEnvExample { output } => {
            cmd_generate_env_example(output.as_deref()).await
        }
        Commands::Ps => cmd_ps(cli.json).await,
        Commands::DownAll { yes } => cmd_down_all(yes, cli.json).await,
        Commands::GenerateSchema { out } => cmd_generate_schema(out.as_deref()),
        Commands::Config { action } => match action {
            ConfigAction::List => {
                if cli.json {
                    cmd_config_list_json().await
                } else {
                    cmd_config(ConfigAction::List).await
                }
            }
            ConfigAction::New {
                name,
                path,
                age_recipient,
                age_key_file,
                with_flake,
                core_flake_url,
                no_register,
                no_git_init,
                from_reference,
                empty,
            } => {
                cmd_config_new(
                    &name,
                    path.as_deref(),
                    age_recipient.as_deref(),
                    age_key_file.as_deref(),
                    with_flake,
                    &core_flake_url,
                    no_register,
                    no_git_init,
                    from_reference,
                    empty,
                    cli.json,
                )
                .await
            }
            other => cmd_config(other).await,
        },
        Commands::Source { action } => cmd_source(action).await,
        Commands::Litellm { action } => {
            let workload = ConfigWorkload::new("litellm")?;
            dispatch_service(&workload, action, cli.show_source, cli.json).await
        }
        Commands::Pi { action } => {
            let workload = ConfigWorkload::new("pi")?;
            dispatch_agent(&workload, action, cli.show_source, cli.json).await
        }
        Commands::Odysseus { action } => {
            let workload = ConfigWorkload::new("odysseus")?;
            dispatch_service(&workload, action, cli.show_source, cli.json).await
        }
        Commands::Opencode { action } => {
            let workload = ConfigWorkload::new("opencode")?;
            dispatch_agent(&workload, action, cli.show_source, cli.json).await
        }
        Commands::Tempest { action } => {
            let workload = ConfigWorkload::new("tempest")?;
            dispatch_agent(&workload, action, cli.show_source, cli.json).await
        }
        Commands::Workload(mut args) => {
            if args.is_empty() {
                anyhow::bail!("no workload name given");
            }
            // Pull --json out of the raw args so it works in any position. The
            // global --json on Cli does not see inside external_subcommand args.
            let json = cli.json || args.iter().any(|a| a == "--json");
            args.retain(|a| a != "--json");

            let name = args.remove(0);
            let action = args.first().cloned().unwrap_or_else(|| "plan".to_string());
            let workload = ConfigWorkload::new(&name)?;
            match workload.kind() {
                "service" => {
                    let service_action = parse_service_action(&action, &args)?;
                    dispatch_service(&workload, service_action, cli.show_source, json).await
                }
                "agent" => {
                    let agent_action = parse_agent_action(&action, &args)?;
                    dispatch_agent(&workload, agent_action, cli.show_source, json).await
                }
                other => anyhow::bail!("unknown workload kind '{}' for '{}'", other, name),
            }
        }
    }
}

fn emit_error(e: &anyhow::Error, json_mode: bool) {
    let classified = classify_error(e);
    if json_mode {
        let body = serde_json::json!({
            "error": {
                "kind": classified.kind,
                "message": classified.message,
            }
        });
        eprintln!(
            "{}",
            serde_json::to_string(&body).unwrap_or_else(|_| {
                r#"{"error":{"kind":"serialize_failed","message":"see logs"}}"#.to_string()
            })
        );
    } else {
        eprintln!("error: {}", classified.message);
    }
}

struct Classified {
    kind: &'static str,
    message: String,
    exit_code: i32,
}

fn classify_error(err: &anyhow::Error) -> Classified {
    let msg = err.to_string();
    if msg.contains("is already running") {
        Classified {
            kind: "refuse_occupied",
            message: msg,
            exit_code: 3,
        }
    } else if msg.contains("port collision") {
        Classified {
            kind: "port_collision",
            message: msg,
            exit_code: 4,
        }
    } else {
        Classified {
            kind: "error",
            message: msg,
            exit_code: 1,
        }
    }
}

fn classify_exit_code(err: &anyhow::Error) -> i32 {
    classify_error(err).exit_code
}

async fn cmd_check() -> Result<()> {
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

    println!("\nXDG dirs:");
    for (label, dir) in [
        ("config", config::xdg_config_dir()),
        ("data", config::xdg_data_dir()),
        ("state", config::xdg_state_dir()),
    ] {
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
                let env_var = format!("WORKESTRATE_{}_BUILD", name.to_uppercase());
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

fn find_reference_config() -> Option<PathBuf> {
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

async fn cmd_validate_config() -> Result<()> {
    let config = config::load_config()?;
    config::validate_config(&config)?;
    println!("workestrate.toml is valid.");
    Ok(())
}

async fn cmd_secrets_schema() -> Result<()> {
    let config = config::load_config()?;
    let mut names: Vec<&str> = config
        .secrets
        .values()
        .filter_map(|s| s.env_var.as_deref())
        .collect();
    names.sort();
    for name in names {
        println!("{}", name);
    }
    Ok(())
}

async fn cmd_generate_env_example(output: Option<&std::path::Path>) -> Result<()> {
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

async fn cmd_run(command: &[String]) -> Result<()> {
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::PathBuf;

    /// RAII guard that points `WORKESTRATE_CONFIG_DIR` at the committed test
    /// fixture (a copy of the pre-strip-down 5-workload config) and restores the
    /// previous state on drop. Holds a global lock so env-var tests do not race
    /// when Cargo runs them in parallel.
    struct TestConfigGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl TestConfigGuard {
        fn new() -> Self {
            let lock = crate::config::tests::ENV_TEST_LOCK.lock().unwrap();
            let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join("config");
            std::env::set_var("WORKESTRATE_CONFIG_DIR", fixture);
            Self { _lock: lock }
        }
    }

    impl Drop for TestConfigGuard {
        fn drop(&mut self) {
            std::env::remove_var("WORKESTRATE_CONFIG_DIR");
        }
    }

    #[test]
    fn cli_exposes_expected_subcommands() {
        let cmd = Cli::command();
        let names: Vec<_> = cmd.get_subcommands().map(|s| s.get_name()).collect();
        for expected in [
            "check",
            "init",
            "new",
            "completions",
            "run",
            "validate-config",
            "secrets-schema",
            "generate-env-example",
            "config",
            "source",
            "litellm",
            "pi",
            "odysseus",
            "opencode",
            "tempest",
        ] {
            assert!(names.contains(&expected), "missing subcommand: {expected}");
        }
    }

    fn check_service_subcommands(cmd: &clap::Command, name: &str) {
        let sub = cmd.find_subcommand(name);
        assert!(sub.is_some(), "missing subcommand: {name}");
        let sub = sub.unwrap();
        let action_names: HashSet<_> = sub
            .get_subcommands()
            .map(|s| s.get_name().to_string())
            .collect();
        for expected in ["up", "down", "logs", "plan"] {
            assert!(
                action_names.contains(expected),
                "{} missing action: {}",
                name,
                expected
            );
        }
        assert!(
            !action_names.contains("exec"),
            "{} should not have action: exec",
            name
        );
    }

    fn check_agent_subcommands(cmd: &clap::Command, name: &str) {
        let sub = cmd.find_subcommand(name);
        assert!(sub.is_some(), "missing subcommand: {name}");
        let sub = sub.unwrap();
        let action_names: HashSet<_> = sub
            .get_subcommands()
            .map(|s| s.get_name().to_string())
            .collect();
        for expected in ["exec", "down", "plan"] {
            assert!(
                action_names.contains(expected),
                "{} missing action: {}",
                name,
                expected
            );
        }
        assert!(
            !action_names.contains("up"),
            "{} should not have action: up",
            name
        );
        assert!(
            !action_names.contains("logs"),
            "{} should not have action: logs",
            name
        );
    }

    #[test]
    fn cli_workload_subcommands_match_registry_kinds() {
        let cmd = Cli::command();
        check_service_subcommands(&cmd, "litellm");
        check_service_subcommands(&cmd, "odysseus");
        check_agent_subcommands(&cmd, "pi");
        check_agent_subcommands(&cmd, "opencode");
        check_agent_subcommands(&cmd, "tempest");
    }

    #[test]
    fn detach_args_include_foreground() -> Result<()> {
        let _guard = TestConfigGuard::new();
        let litellm = ConfigWorkload::new("litellm")?;
        assert!(
            litellm.detach_args().contains(&"--foreground".to_string()),
            "litellm detach_args must contain --foreground"
        );
        let pi = ConfigWorkload::new("pi")?;
        assert!(
            pi.detach_args().contains(&"--foreground".to_string()),
            "pi detach_args must contain --foreground"
        );
        Ok(())
    }

    #[test]
    fn cmd_new_resolves_active_config_dir() -> Result<()> {
        // Create a temp config repo with a minimal workestrate.toml
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-cmd-new-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        std::fs::write(tmp.join("workestrate.toml"), "schema_version = 1\n")?;

        // Point WORKESTRATE_CONFIG_DIR at the temp repo
        let old = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        std::env::set_var("WORKESTRATE_CONFIG_DIR", &tmp);

        // Verify resolve_active_config_dir returns the temp dir
        let config_dir = config::resolve_active_config_dir()?;
        assert_eq!(
            config_dir, tmp,
            "resolve_active_config_dir should return the WORKESTRATE_CONFIG_DIR path"
        );

        // Simulate what cmd_new does: create agent config dir + append to workestrate.toml
        let agent_config = tmp.join("agents").join("test-agent").join("config");
        std::fs::create_dir_all(&agent_config)?;
        std::fs::write(agent_config.join(".gitkeep"), "")?;

        let config_path = tmp.join("workestrate.toml");
        let toml_entry = "\n[workloads.test-agent]\nkind = \"agent\"\n";
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&config_path)?;
        use std::io::Write;
        file.write_all(toml_entry.as_bytes())?;

        // Restore env
        match old {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }

        // Verify the agent config dir was created in the config repo
        assert!(
            agent_config.exists(),
            "agent config dir should exist in config repo"
        );

        // Verify workestrate.toml was appended
        let toml_content = std::fs::read_to_string(tmp.join("workestrate.toml"))?;
        assert!(
            toml_content.contains("[workloads.test-agent]"),
            "workestrate.toml should contain the new workload entry"
        );

        // Cleanup
        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn cmd_new_no_config_repo_produces_clear_error() {
        // Use a temp HOME so no registry exists and no project config is found.
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-no-config-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp_home).ok();

        let old_home = std::env::var("HOME").ok();
        let old_config = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        let old_no_project = std::env::var("WORKESTRATE_NO_PROJECT_CONFIG").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::set_var("XDG_DATA_HOME", tmp_home.join(".local").join("share"));
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");
        std::env::set_var("WORKESTRATE_NO_PROJECT_CONFIG", "1");

        let result = config::resolve_active_config_dir();

        // Restore env
        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_config {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        match old_no_project {
            Some(v) => std::env::set_var("WORKESTRATE_NO_PROJECT_CONFIG", v),
            None => std::env::remove_var("WORKESTRATE_NO_PROJECT_CONFIG"),
        }
        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::remove_var("XDG_DATA_HOME");

        let _ = std::fs::remove_dir_all(&tmp_home);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("no active config repo") || err.contains("workestrate init"),
            "error should mention 'no active config repo' or 'workestrate init'; got: {err}"
        );
    }

    // ---- A20 regression: cmd_new rejects invalid workload names ----

    #[test]
    fn validate_workload_name_accepts_legitimate_names() {
        for ok in [
            "pi",
            "opencode",
            "example-agent",
            "my-cool-workload",
            "abc",
            "a1b",
            "a",
            "0",
            "1agent",
            &"a".repeat(63),
        ] {
            validate_workload_name(ok)
                .unwrap_or_else(|e| panic!("legitimate name '{ok}' rejected: {e}"));
        }
    }

    #[test]
    fn validate_workload_name_rejects_hostile_inputs() {
        // Each must fail. Categories: path escape, TOML injection,
        // shell-meta, uppercase, underscore, leading-hyphen, overlong, empty.
        let hostile = [
            "../pwned",      // path escape
            "/etc/pwned",    // absolute path
            "a]b",           // TOML table close-bracket injection
            "a.b",           // dot (TOML nested-key separator)
            "a b",           // whitespace
            "a$b",           // shell meta
            "a;b",           // shell meta
            "Agent",         // uppercase
            "my_agent",      // underscore (DNS-label style disallows)
            "-leading",      // leading hyphen
            "",              // empty
            &"x".repeat(64), // overlong (64 > 63)
        ];
        for h in hostile {
            let result = validate_workload_name(h);
            assert!(
                result.is_err(),
                "hostile name '{h}' should be rejected, but was accepted"
            );
        }
    }

    #[test]
    fn validate_workload_name_trailing_hyphen_is_allowed_by_design() {
        // The regex ^[a-z0-9][a-z0-9-]{0,62}$ permits trailing hyphens.
        // DNS labels disallow them, but workestrate workload names are not
        // DNS labels — they're filesystem path components and TOML keys.
        // If a future decision tightens this, update both the regex and this
        // test together.
        validate_workload_name("foo-").expect("trailing hyphen is allowed");
    }

    // ---- config::validate_config_name (used by `workestrate config new`) ----

    #[test]
    fn validate_config_name_accepts_legitimate_names() {
        for ok in [
            "personal",
            "work",
            "team",
            "prod",
            "a",
            "0",
            "p1",
            "my-config-2",
        ] {
            config::validate_config_name(ok)
                .unwrap_or_else(|e| panic!("legitimate config name '{ok}' rejected: {e}"));
        }
    }

    #[test]
    fn validate_config_name_rejects_hostile_inputs() {
        // Same safe-set as validate_workload_name: config names flow into
        // both filesystem paths and registry TOML keys, so the
        // intersection [a-z0-9-] is the only safe charset.
        let hostile = [
            "../pwned",
            "/etc/pwned",
            "a]b",
            "a.b",
            "a b",
            "Personal", // uppercase
            "my_config",
            "-leading",
            "",
            &"x".repeat(64),
        ];
        for h in hostile {
            let result = config::validate_config_name(h);
            assert!(
                result.is_err(),
                "hostile config name '{h}' should be rejected, but was accepted"
            );
        }
    }

    #[test]
    fn validate_config_name_error_mentions_config_name() {
        // Error label is interpolated from the validator, not hardcoded —
        // this catches a copy-paste regression where the workload-name
        // message would leak through.
        let err = config::validate_config_name("BAD").unwrap_err().to_string();
        assert!(
            err.contains("config name"),
            "error should mention 'config name'; got: {err}"
        );
        assert!(
            !err.contains("workload name"),
            "error should NOT mention 'workload name'; got: {err}"
        );
    }

    // ---- WP5 / E1 regression: graceful check outside a workbench checkout ----

    #[test]
    fn project_root_optional_returns_none_outside_workbench() {
        let _lock = crate::config::tests::ENV_TEST_LOCK.lock().unwrap();
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
        let _lock = crate::config::tests::ENV_TEST_LOCK.lock().unwrap();

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

        // cmd_check is async; run it on a fresh tokio runtime.
        let rt = tokio::runtime::Runtime::new().expect("failed to build tokio runtime for E1 test");
        let result = rt.block_on(async { cmd_check().await });

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

    // ---- ADR 0021 CLI flag-parsing tests ----

    #[test]
    fn parse_flag_value_supports_space_form() {
        let args: Vec<String> = vec!["--instance".into(), "canary".into()];
        assert_eq!(
            parse_flag_value(&args, "--instance").as_deref(),
            Some("canary")
        );
    }

    #[test]
    fn parse_flag_value_supports_equals_form() {
        let args: Vec<String> = vec!["--instance=blue-green".into()];
        assert_eq!(
            parse_flag_value(&args, "--instance").as_deref(),
            Some("blue-green")
        );
    }

    #[test]
    fn parse_flag_value_returns_none_when_absent() {
        let args: Vec<String> = vec!["--replace".into()];
        assert!(parse_flag_value(&args, "--instance").is_none());
    }

    #[test]
    fn parse_port_offset_defaults_to_zero() -> Result<()> {
        let args: Vec<String> = vec!["--replace".into()];
        assert_eq!(parse_port_offset(&args)?, 0);
        Ok(())
    }

    #[test]
    fn parse_port_offset_reads_value() -> Result<()> {
        let args: Vec<String> = vec!["--port-offset".into(), "10000".into()];
        assert_eq!(parse_port_offset(&args)?, 10000);
        Ok(())
    }

    #[test]
    fn parse_port_offset_rejects_non_numeric() {
        let args: Vec<String> = vec!["--port-offset".into(), "huge".into()];
        assert!(parse_port_offset(&args).is_err());
    }

    #[test]
    fn parse_port_offset_rejects_overflow() {
        let args: Vec<String> = vec!["--port-offset".into(), "70000".into()];
        assert!(parse_port_offset(&args).is_err());
    }

    #[test]
    fn classify_error_refuse_occupied() {
        let e =
            anyhow::anyhow!("instance 'personal-litellm' is already running. Use --replace ...");
        let c = classify_error(&e);
        assert_eq!(c.kind, "refuse_occupied");
        assert_eq!(c.exit_code, 3);
    }

    #[test]
    fn classify_error_port_collision() {
        let e = anyhow::anyhow!("port collision: port 4000 is already in use by ...");
        let c = classify_error(&e);
        assert_eq!(c.kind, "port_collision");
        assert_eq!(c.exit_code, 4);
    }

    #[test]
    fn classify_error_generic() {
        let e = anyhow::anyhow!("something went wrong");
        let c = classify_error(&e);
        assert_eq!(c.kind, "error");
        assert_eq!(c.exit_code, 1);
    }

    #[test]
    fn cli_exposes_lifecycle_subcommands() {
        let cmd = Cli::command();
        let names: Vec<_> = cmd.get_subcommands().map(|s| s.get_name()).collect();
        for expected in ["ps", "down-all", "generate-schema"] {
            assert!(names.contains(&expected), "missing subcommand: {expected}");
        }
    }

    #[test]
    fn service_up_exposes_lifecycle_flags() {
        let cmd = Cli::command();
        let up = cmd
            .find_subcommand("litellm")
            .and_then(|s| s.find_subcommand("up"))
            .expect("litellm up must exist");
        // get_long() returns the user-facing long flag name (clap hyphenates
        // underscores: port_offset → port-offset). get_id() preserves the raw
        // field identifier; the CLI surface is what we care about here.
        let flag_names: Vec<_> = up
            .get_arguments()
            .filter_map(|a| a.get_long().map(|s| s.to_string()))
            .collect();
        for f in ["replace", "instance", "new", "port-offset", "foreground"] {
            assert!(
                flag_names.contains(&f.to_string()),
                "litellm up missing flag: {f}"
            );
        }
    }

    #[test]
    fn service_down_exposes_lifecycle_flags() {
        let cmd = Cli::command();
        let down = cmd
            .find_subcommand("litellm")
            .and_then(|s| s.find_subcommand("down"))
            .expect("litellm down must exist");
        let flag_names: Vec<_> = down
            .get_arguments()
            .filter_map(|a| a.get_long().map(|s| s.to_string()))
            .collect();
        for f in ["instance", "all-instances"] {
            assert!(
                flag_names.contains(&f.to_string()),
                "litellm down missing flag: {f}"
            );
        }
    }
}
