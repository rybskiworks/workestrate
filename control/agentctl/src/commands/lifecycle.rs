//! Workload lifecycle: instance-spec construction, service/agent dispatch,
//! raw-args action parsing, and teardown (`down`, `down --all`, `clean`).
//!
//! NOTE (WP4-B): verbatim copies of the private items still live in
//! `main.rs`. Commit C cuts `main.rs` over to these and deletes its own.

use anyhow::Result;
use std::io::Write;

use crate::commands::diagnostics::cmd_plan;
use crate::config;
use crate::json_out::{down_result_json, down_results_json};
use crate::microsandbox::workload::Workload;
use crate::{AgentAction, ServiceAction};

///
/// `workload_name` is the bare workload name (e.g. "litellm"). The slot is
/// derived from the active context. `instance_id` (from --instance) is
/// `instance_id` (from `--instance`) and `new_id` (from `--new`,
/// already-allocated slug) are BOTH validated through `validate_instance_id` —
/// uniform validation closes the bypass where the old `--new` integer id
/// skipped the slug rule.
///
/// Mutually-exclusive flag groups (replace/instance/new) are validated here.
pub(crate) fn build_instance_spec(
    workload_name: &str,
    replace: bool,
    instance_id: Option<&str>,
    new_id: Option<&str>,
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

    // UNIFORM validation: whichever of --instance / --new was supplied,
    // the id passes through the same validate_instance_id gate. The slug
    // allocator (auto_allocate_slug) produces ids guaranteed to pass this.
    let id: Option<String> = if let Some(id) = instance_id {
        validate_instance_id(id)?;
        Some(id.to_string())
    } else if let Some(slug) = new_id {
        validate_instance_id(slug)?;
        Some(slug.to_string())
    } else {
        None
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

pub(crate) async fn dispatch_service<W: Workload>(
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
            let new_id: Option<String> = if new {
                let state_dir = crate::config::resolve_state_dir();
                Some(crate::microsandbox::port_registry::auto_allocate_slug(
                    &state_dir,
                    &crate::microsandbox::slots::slot_for(
                        workload.name(),
                        crate::config::active_context_name().as_deref(),
                    ),
                )?)
            } else {
                None
            };
            let spec = build_instance_spec(
                workload.name(),
                replace,
                instance.as_deref(),
                new_id.as_deref(),
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

pub(crate) async fn dispatch_agent<W: Workload>(
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
            let new_id: Option<String> = if new {
                let state_dir = crate::config::resolve_state_dir();
                Some(crate::microsandbox::port_registry::auto_allocate_slug(
                    &state_dir,
                    &crate::microsandbox::slots::slot_for(
                        workload.name(),
                        crate::config::active_context_name().as_deref(),
                    ),
                )?)
            } else {
                None
            };
            let spec = build_instance_spec(
                workload.name(),
                replace,
                instance.as_deref(),
                new_id.as_deref(),
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

pub(crate) fn parse_service_action(action: &str, args: &[String]) -> Result<ServiceAction> {
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

pub(crate) fn parse_agent_action(action: &str, args: &[String]) -> Result<AgentAction> {
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
pub(crate) fn parse_flag_value(args: &[String], flag: &str) -> Option<String> {
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
pub(crate) fn parse_port_offset(args: &[String]) -> Result<u16> {
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

pub(crate) async fn cmd_down(
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

pub(crate) fn print_down_results_text(results: &[crate::microsandbox::runtime::DownResult]) {
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

pub(crate) fn report_down_aggregate(
    results: &[crate::microsandbox::runtime::DownResult],
) -> Result<()> {
    use crate::microsandbox::runtime::DownStatus;
    let had_error = results
        .iter()
        .any(|r| matches!(r.status, DownStatus::Error));
    if had_error {
        anyhow::bail!("one or more instances failed to stop");
    }
    Ok(())
}

pub(crate) async fn cmd_down_all(yes: bool, json: bool) -> Result<()> {
    use crate::microsandbox::runtime::{down_all, DownStatus};
    if !yes {
        // Read a single line of confirmation so piped input ("y\n") does not
        // block waiting for EOF — the old read_to_string hung interactive and
        // scripted use. Mirrors cmd_clean's single-line confirm; the prompt is
        // shown only on a tty, but a line is read in every mode so a piped
        // "y"/"yes" confirms and an empty/non-tty stdin aborts. Accepted tokens
        // are unified to `y`/`yes` (case-insensitive), matching cmd_clean.
        use std::io::IsTerminal;
        if std::io::stdin().is_terminal() {
            eprint!("This will stop EVERY running workestrate sandbox. Continue? [y/N] ");
            std::io::stderr().flush()?;
        }
        use std::io::BufRead;
        let answer = std::io::stdin()
            .lock()
            .lines()
            .next()
            .transpose()?
            .unwrap_or_default();
        let confirmed = matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes");
        if !confirmed {
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

/// `workestrate clean` — remove the CONTENTS of the volatile state-dir
/// subdirectories (`workspaces/`, `var/`, `run/`), leaving the directories
/// themselves in place. Never touches the store (`repos/`, `sources/`) or any
/// config file. Interactive confirmation unless `--yes`; non-interactive
/// stdin without `--yes` is a hard refusal (same policy as `down --all`).
pub(crate) async fn cmd_clean(yes: bool, json: bool) -> Result<()> {
    use std::io::IsTerminal;
    let state_dir = config::resolve_state_dir();
    const SUBDIRS: [&str; 3] = ["workspaces", "var", "run"];

    if !yes {
        if std::io::stdin().is_terminal() {
            eprint!(
                "This will remove contents of {}/{{workspaces,var,run}}. Continue? [y/N] ",
                state_dir.display()
            );
            std::io::stderr().flush()?;
            use std::io::BufRead;
            let answer = std::io::stdin()
                .lock()
                .lines()
                .next()
                .transpose()?
                .unwrap_or_default();
            let confirmed = matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes");
            if !confirmed {
                eprintln!("aborted");
                std::process::exit(1);
            }
        } else {
            anyhow::bail!("refusing to clean in non-interactive mode without --yes");
        }
    }

    struct CleanEntry {
        dir: &'static str,
        entries_removed: usize,
        status: &'static str,
    }
    let mut entries: Vec<CleanEntry> = Vec::new();
    for sub in SUBDIRS {
        let dir = state_dir.join(sub);
        if !dir.exists() {
            entries.push(CleanEntry {
                dir: sub,
                entries_removed: 0,
                status: "absent",
            });
            continue;
        }
        let removed = std::fs::read_dir(&dir)?.filter_map(|e| e.ok()).count();
        std::fs::remove_dir_all(&dir)?;
        std::fs::create_dir_all(&dir)?;
        entries.push(CleanEntry {
            dir: sub,
            entries_removed: removed,
            status: "cleaned",
        });
    }

    if json {
        let cleaned: Vec<serde_json::Value> = entries
            .iter()
            .map(|e| {
                serde_json::json!({
                    "dir": e.dir,
                    "entries_removed": e.entries_removed,
                    "status": e.status,
                })
            })
            .collect();
        let body = serde_json::json!({
            "state_dir": state_dir.display().to_string(),
            "cleaned": cleaned,
        });
        println!("{}", serde_json::to_string_pretty(&body)?);
    } else {
        let mut cleaned_count = 0usize;
        for e in &entries {
            if e.status == "cleaned" {
                cleaned_count += 1;
                println!("  {}: removed {} entries [OK]", e.dir, e.entries_removed);
            } else {
                println!("  {}: (absent)", e.dir);
            }
        }
        println!(
            "Cleaned {} directories in {}",
            cleaned_count,
            state_dir.display()
        );
    }
    Ok(())
}
