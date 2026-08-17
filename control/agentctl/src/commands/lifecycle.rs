//! Workload lifecycle: instance-spec construction, service/agent dispatch,
//! raw-args action parsing, and teardown (`down`, `down --all`, `clean`).

use anyhow::Result;
use std::io::Write;

use crate::cli_actions::{AgentAction, ServiceAction};
use crate::commands::diagnostics::cmd_plan;
use crate::config;
use crate::json_out::{down_result_json, down_results_json};
use crate::microsandbox::workload::Workload;

///
/// `workload_name` is the bare workload name (e.g. "litellm"). The slot is
/// derived from the active context. `instance_id` (from --instance) is
/// `instance_id` (from `--instance`) and `new_id` (from `--new`,
/// already-allocated slug) are BOTH validated through `validate_instance_id` —
/// uniform validation closes the bypass where the old `--new` integer id
/// skipped the slug rule.
///
/// Mutually-exclusive flag groups (replace/instance/new) are validated here.
/// Whether `up`/`exec` should default to a NEW parallel instance (auto-slug)
/// for a workload with the given strategy and flags (ADR 0030 §4.1): true
/// when `--new` was given, OR the strategy is `parallel` AND no explicit
/// `--instance <id>` / `--replace` overrides. Pure — unit-testable.
fn parallel_strategy_defaults_new(
    strategy: crate::config::InstanceStrategy,
    new: bool,
    replace: bool,
    no_instance: bool,
) -> bool {
    new || (strategy == crate::config::InstanceStrategy::Parallel && !replace && no_instance)
}

pub fn build_instance_spec(
    workload_name: &str,
    replace: bool,
    instance_id: Option<&str>,
    new_id: Option<&str>,
    port_auto: bool,
    use_overrides: &[(String, String)],
    no_deps: bool,
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
        instance,
        workload: workload_name.to_string(),
        context,
        replace,
        port_auto,
        use_overrides: use_overrides.to_vec(),
        no_deps,
        // Spec 21 §2.2: the ensure-images token defaults OFF here — callers
        // set it (a detached up's spec describes the ensured child-to-be;
        // the detached child reconstitutes it from its clap parse).
        images_ready: false,
    })
}

/// Where a verb-first `workestrate workload <verb> <name>` action routes
/// (ADR 0027): services take `up`/`down`/`logs`/`plan`, agents take
/// `exec`/`down`/`plan`; `plan` and `down` are universal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkloadRoute {
    /// Dispatch through [`dispatch_service`] ([`ServiceAction`]).
    Service,
    /// Dispatch through [`dispatch_agent`] ([`AgentAction`]).
    Agent,
}

/// Kind-check a verb-first workload action at dispatch (ADR 0027). Pure:
/// given the workload's configured `kind`, the verb, and the workload name,
/// decide the dispatch route or produce a clear wrong-kind error. Wrong-kind
/// usage names the correct invocation (e.g. "my-agent is an agent; use
/// `workestrate workload exec my-agent`").
pub fn workload_route(kind: &str, verb: &str, name: &str) -> Result<WorkloadRoute> {
    match (kind, verb) {
        ("service", "up" | "down" | "logs" | "plan") => Ok(WorkloadRoute::Service),
        ("service", "exec") => anyhow::bail!(
            "{} is a service; use `workestrate workload up {}` (services do not support exec)",
            name,
            name
        ),
        ("agent", "exec" | "down" | "plan") => Ok(WorkloadRoute::Agent),
        ("agent", "up" | "logs") => anyhow::bail!(
            "{} is an agent; use `workestrate workload exec {}`",
            name,
            name
        ),
        (other, _) => anyhow::bail!("unknown workload kind '{}' for '{}'", other, name),
    }
}

pub async fn dispatch_service<W: Workload>(
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
            port_auto,
            use_,
            no_deps,
            // --reload-images is consumed by the ensure-images pre-flight
            // in main.rs (before dispatch); it never reaches the spec.
            reload_images: _,
            images_ready,
        } => {
            let new_id: Option<String> = if parallel_strategy_defaults_new(
                workload.instance_strategy(),
                new,
                replace,
                instance.is_none(),
            ) {
                // ADR 0030 §4.1: `strategy = "parallel"` defaults up/exec to a
                // FRESH parallel instance (auto-slug) unless an explicit
                // `--instance <id>` / `--new` / `--replace` overrides (Q3:
                // auto-slug recommended).
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
            let mut spec = build_instance_spec(
                workload.name(),
                replace,
                instance.as_deref(),
                new_id.as_deref(),
                port_auto,
                &crate::microsandbox::discovery::parse_use_overrides(&use_)?,
                no_deps,
            )?;
            // Spec 21 §2.2: the spec of a DETACHED up describes the ensured
            // child-to-be (the parent ensures before spawning it), and the
            // detached child itself re-parsed --images-ready — both carry
            // the token. A plain foreground up is the parent (no token).
            spec.images_ready = images_ready || !foreground;
            crate::microsandbox::runtime::up_service_with_spec(workload, &spec, foreground).await
        }
        ServiceAction::Down {
            instance,
            all_instances,
        } => cmd_down(workload.name(), instance.as_deref(), all_instances, json).await,
        ServiceAction::Logs { instance } => {
            // Resolve the instance name like `up` does: slot from the active
            // context + optional parallel id (default = the singleton). The
            // id passes through the same validate_instance_id gate as
            // up/down (FS-10).
            use crate::microsandbox::slots::{instance_name, slot_for, validate_instance_id};
            let context = crate::config::active_context_name();
            let slot = slot_for(workload.name(), context.as_deref());
            if let Some(id) = instance.as_deref() {
                validate_instance_id(id)?;
            }
            let target = instance_name(&slot, instance.as_deref());
            crate::microsandbox::logs(&target).await
        }
        ServiceAction::Plan { instance, use_ } => {
            cmd_plan(workload, show_source, json, instance.as_deref(), &use_)
        }
    }
}

pub async fn dispatch_agent<W: Workload>(
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
            port_auto,
            use_,
            no_deps,
            // Consumed by the ensure-images pre-flight in main.rs.
            reload_images: _,
        } => {
            let new_id: Option<String> = if parallel_strategy_defaults_new(
                workload.instance_strategy(),
                new,
                replace,
                instance.is_none(),
            ) {
                // ADR 0030 §4.1: `strategy = "parallel"` defaults up/exec to a
                // FRESH parallel instance (auto-slug) unless an explicit
                // `--instance <id>` / `--new` / `--replace` overrides (Q3:
                // auto-slug recommended).
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
                port_auto,
                &crate::microsandbox::discovery::parse_use_overrides(&use_)?,
                no_deps,
            )?;
            crate::microsandbox::runtime::exec_agent_with_spec(workload, &spec).await
        }
        AgentAction::Down {
            instance,
            all_instances,
        } => cmd_down(workload.name(), instance.as_deref(), all_instances, json).await,
        AgentAction::Plan { instance, use_ } => {
            cmd_plan(workload, show_source, json, instance.as_deref(), &use_)
        }
    }
}

pub fn parse_service_action(action: &str, args: &[String]) -> Result<ServiceAction> {
    match action {
        "up" => {
            let foreground = args.iter().any(|a| a == "--foreground");
            let replace = args.iter().any(|a| a == "--replace");
            let new = args.iter().any(|a| a == "--new");
            let port_auto = args.iter().any(|a| a == "--port-auto");
            let no_deps = args.iter().any(|a| a == "--no-deps");
            let reload_images = args.iter().any(|a| a == "--reload-images");
            let images_ready = args.iter().any(|a| a == "--images-ready");
            let instance = parse_flag_value(args, "--instance");
            let use_ = parse_flag_values(args, "--use");
            Ok(ServiceAction::Up {
                foreground,
                replace,
                instance,
                new,
                port_auto,
                use_,
                no_deps,
                reload_images,
                images_ready,
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
        "logs" => {
            let instance = parse_flag_value(args, "--instance");
            Ok(ServiceAction::Logs { instance })
        }
        "plan" => {
            let instance = parse_flag_value(args, "--instance");
            let use_ = parse_flag_values(args, "--use");
            Ok(ServiceAction::Plan { instance, use_ })
        }
        other => anyhow::bail!("unknown service action: {}", other),
    }
}

pub fn parse_agent_action(action: &str, args: &[String]) -> Result<AgentAction> {
    match action {
        "exec" => {
            let replace = args.iter().any(|a| a == "--replace");
            let new = args.iter().any(|a| a == "--new");
            let port_auto = args.iter().any(|a| a == "--port-auto");
            let no_deps = args.iter().any(|a| a == "--no-deps");
            let reload_images = args.iter().any(|a| a == "--reload-images");
            let instance = parse_flag_value(args, "--instance");
            let use_ = parse_flag_values(args, "--use");
            Ok(AgentAction::Exec {
                replace,
                instance,
                new,
                port_auto,
                use_,
                no_deps,
                reload_images,
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
            let instance = parse_flag_value(args, "--instance");
            let use_ = parse_flag_values(args, "--use");
            Ok(AgentAction::Plan { instance, use_ })
        }
        other => anyhow::bail!("unknown agent action: {}", other),
    }
}

/// Extract the value of `--flag <value>` or `--flag=value` from a Vec<String>
/// (the workload catch-all args). Returns None if the flag is absent.
pub fn parse_flag_value(args: &[String], flag: &str) -> Option<String> {
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

/// Extract EVERY occurrence of `--flag <value>` or `--flag=value` from a
/// Vec<String> (the workload catch-all args), in argv order. Used for
/// repeatable flags like `--use <dep>@<instance>` (ADR 0026(d)) where each
/// occurrence overrides selection for one dep. A trailing bare `--flag` (no
/// following value) is silently skipped, matching `parse_flag_value`.
pub fn parse_flag_values(args: &[String], flag: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut iter = args.iter();
    while let Some(a) = iter.next() {
        if a == flag {
            if let Some(v) = iter.next() {
                values.push(v.clone());
            }
        } else if let Some(rest) = a.strip_prefix(&format!("{}=", flag)) {
            values.push(rest.to_string());
        }
    }
    values
}

pub async fn cmd_down(
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

pub fn print_down_results_text(results: &[crate::microsandbox::runtime::DownResult]) {
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

pub fn report_down_aggregate(results: &[crate::microsandbox::runtime::DownResult]) -> Result<()> {
    use crate::microsandbox::runtime::DownStatus;
    let had_error = results
        .iter()
        .any(|r| matches!(r.status, DownStatus::Error));
    if had_error {
        anyhow::bail!("one or more instances failed to stop");
    }
    Ok(())
}

pub async fn cmd_down_all(yes: bool, json: bool) -> Result<()> {
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
/// themselves in place. Never touches the store (`config-repos/`, `sources/`)
/// or any
/// config file. Interactive confirmation unless `--yes`; non-interactive
/// stdin without `--yes` is a hard refusal (same policy as `down --all`).
pub fn cmd_clean(yes: bool, json: bool) -> Result<()> {
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;

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

    // ---- ADR 0026(c)/C3: --port-auto raw-args parsing ----

    #[test]
    fn parse_service_action_up_reads_port_auto() {
        let args: Vec<String> = vec!["--port-auto".into()];
        match parse_service_action("up", &args).unwrap() {
            ServiceAction::Up { port_auto, .. } => assert!(port_auto),
            _ => panic!("expected Up variant"),
        }
        let args: Vec<String> = vec!["--foreground".into()];
        match parse_service_action("up", &args).unwrap() {
            ServiceAction::Up { port_auto, .. } => assert!(!port_auto),
            _ => panic!("expected Up variant"),
        }
    }

    #[test]
    fn parse_agent_action_exec_reads_port_auto() {
        let args: Vec<String> = vec!["--port-auto".into()];
        match parse_agent_action("exec", &args).unwrap() {
            AgentAction::Exec { port_auto, .. } => assert!(port_auto),
            _ => panic!("expected Exec variant"),
        }
        let args: Vec<String> = vec!["--replace".into()];
        match parse_agent_action("exec", &args).unwrap() {
            AgentAction::Exec { port_auto, .. } => assert!(!port_auto),
            _ => panic!("expected Exec variant"),
        }
    }

    // ---- ADR 0026(d)/C3-W2: --use <dep>@<instance> raw-args parsing ----

    #[test]
    fn parse_flag_values_collects_space_and_equals_forms_in_order() {
        let args: Vec<String> = vec![
            "--use".into(),
            "litellm@canary".into(),
            "--replace".into(),
            "--use=redis@blue".into(),
            "--use".into(),
            "odysseus@x1".into(),
        ];
        assert_eq!(
            parse_flag_values(&args, "--use"),
            vec!["litellm@canary", "redis@blue", "odysseus@x1"]
        );
    }

    #[test]
    fn parse_flag_values_returns_empty_when_absent() {
        let args: Vec<String> = vec!["--instance".into(), "canary".into()];
        assert!(parse_flag_values(&args, "--use").is_empty());
    }

    #[test]
    fn parse_service_action_up_collects_all_use_values() {
        let args: Vec<String> = vec![
            "--use".into(),
            "litellm@canary".into(),
            "--use=redis@blue".into(),
        ];
        match parse_service_action("up", &args).unwrap() {
            ServiceAction::Up { use_, .. } => {
                assert_eq!(use_, vec!["litellm@canary", "redis@blue"])
            }
            _ => panic!("expected Up variant"),
        }
    }

    #[test]
    fn parse_service_action_plan_collects_use_values() {
        let args: Vec<String> = vec!["--use".into(), "litellm@canary".into()];
        match parse_service_action("plan", &args).unwrap() {
            ServiceAction::Plan { use_, .. } => assert_eq!(use_, vec!["litellm@canary"]),
            _ => panic!("expected Plan variant"),
        }
    }

    #[test]
    fn parse_agent_action_exec_collects_use_values() {
        let args: Vec<String> = vec!["--use".into(), "litellm@canary".into()];
        match parse_agent_action("exec", &args).unwrap() {
            AgentAction::Exec { use_, .. } => assert_eq!(use_, vec!["litellm@canary"]),
            _ => panic!("expected Exec variant"),
        }
    }

    #[test]
    fn parse_agent_action_plan_collects_use_values() {
        let args: Vec<String> = vec!["--use=litellm@canary".into()];
        match parse_agent_action("plan", &args).unwrap() {
            AgentAction::Plan { use_, .. } => assert_eq!(use_, vec!["litellm@canary"]),
            _ => panic!("expected Plan variant"),
        }
    }

    // ---- ADR 0026 addendum: --no-deps raw-args parsing ----

    #[test]
    fn parse_service_action_up_reads_no_deps() {
        let args: Vec<String> = vec!["--no-deps".into()];
        match parse_service_action("up", &args).unwrap() {
            ServiceAction::Up { no_deps, .. } => assert!(no_deps),
            _ => panic!("expected Up variant"),
        }
        let args: Vec<String> = vec!["--foreground".into()];
        match parse_service_action("up", &args).unwrap() {
            ServiceAction::Up { no_deps, .. } => assert!(!no_deps),
            _ => panic!("expected Up variant"),
        }
    }

    #[test]
    fn parse_agent_action_exec_reads_no_deps() {
        let args: Vec<String> = vec!["--no-deps".into()];
        match parse_agent_action("exec", &args).unwrap() {
            AgentAction::Exec { no_deps, .. } => assert!(no_deps),
            _ => panic!("expected Exec variant"),
        }
        let args: Vec<String> = vec!["--replace".into()];
        match parse_agent_action("exec", &args).unwrap() {
            AgentAction::Exec { no_deps, .. } => assert!(!no_deps),
            _ => panic!("expected Exec variant"),
        }
    }

    /// build_instance_spec stores the typed overrides on the spec so
    /// detach_args can forward them to the detached child (ADR 0021/0026(d)).
    #[test]
    fn build_instance_spec_stores_use_overrides() {
        let overrides = vec![
            ("litellm".to_string(), "canary".to_string()),
            ("redis".to_string(), "blue".to_string()),
        ];
        let spec = build_instance_spec("pi", false, None, None, false, &overrides, false).unwrap();
        assert_eq!(spec.use_overrides, overrides);
        assert!(!spec.no_deps);
        let spec = build_instance_spec("pi", false, None, None, false, &[], false).unwrap();
        assert!(spec.use_overrides.is_empty());
    }

    /// build_instance_spec stores --no-deps on the spec so detach_args can
    /// forward it to the detached child (ADR 0026 addendum).
    #[test]
    fn build_instance_spec_stores_no_deps() {
        let spec = build_instance_spec("pi", false, None, None, false, &[], true).unwrap();
        assert!(spec.no_deps);
    }

    // ---- ADR 0030 Phase 2: parallel strategy defaults to a NEW instance ----

    /// `strategy = "parallel"` + no explicit flags → up/exec defaults to a
    /// NEW parallel instance (auto-slug).
    #[test]
    fn parallel_strategy_allocates_new_instance_by_default() {
        assert!(parallel_strategy_defaults_new(
            crate::config::InstanceStrategy::Parallel,
            false,
            false,
            true,
        ));
    }

    /// `strategy = "singleton"` + no explicit flags keeps the singleton slot.
    #[test]
    fn singleton_strategy_keeps_singleton() {
        assert!(!parallel_strategy_defaults_new(
            crate::config::InstanceStrategy::Singleton,
            false,
            false,
            true,
        ));
    }

    /// An explicit `--instance <id>` beats the parallel strategy (the
    /// targeted instance wins).
    #[test]
    fn explicit_instance_beats_parallel_strategy() {
        assert!(!parallel_strategy_defaults_new(
            crate::config::InstanceStrategy::Parallel,
            false,
            false,
            false, // --instance given
        ));
    }

    /// `--replace` beats the parallel strategy (replace the singleton).
    #[test]
    fn replace_beats_parallel_strategy() {
        assert!(!parallel_strategy_defaults_new(
            crate::config::InstanceStrategy::Parallel,
            false,
            true,
            true,
        ));
    }

    /// `--new` always allocates, regardless of strategy.
    #[test]
    fn explicit_new_allocates_even_for_singleton_strategy() {
        assert!(parallel_strategy_defaults_new(
            crate::config::InstanceStrategy::Singleton,
            true,
            false,
            true,
        ));
    }

    // ---- ADR 0027: verb-first kind-check routing ----

    #[test]
    fn workload_route_allows_all_service_verbs_on_services() {
        for verb in ["up", "down", "logs", "plan"] {
            assert_eq!(
                workload_route("service", verb, "litellm").unwrap(),
                WorkloadRoute::Service,
                "service + {verb} must route to the service dispatch"
            );
        }
    }

    #[test]
    fn workload_route_rejects_exec_on_services() {
        let err = workload_route("service", "exec", "litellm").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("litellm is a service") && msg.contains("workestrate workload up litellm"),
            "service+exec error must point at `workload up`; got: {msg}"
        );
    }

    #[test]
    fn workload_route_allows_exec_down_plan_on_agents() {
        for verb in ["exec", "down", "plan"] {
            assert_eq!(
                workload_route("agent", verb, "pi").unwrap(),
                WorkloadRoute::Agent,
                "agent + {verb} must route to the agent dispatch"
            );
        }
    }

    #[test]
    fn workload_route_rejects_up_and_logs_on_agents() {
        for verb in ["up", "logs"] {
            let err = workload_route("agent", verb, "pi").unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains("pi is an agent") && msg.contains("workestrate workload exec pi"),
                "agent+{verb} error must point at `workload exec`; got: {msg}"
            );
        }
    }

    #[test]
    fn workload_route_rejects_unknown_kind() {
        let err = workload_route("worker", "up", "foo").unwrap_err();
        assert_eq!(err.to_string(), "unknown workload kind 'worker' for 'foo'");
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod fs10_tests {
    use super::*;
    use crate::microsandbox::slots::{instance_name, slot_for, validate_instance_id};

    // ---- FS-10: `logs --instance <id>` resolves the per-instance log path ----

    /// The raw-args parser must pick up `--instance <id>` on the logs action
    /// (both space and equals forms), mirroring up/down.
    #[test]
    fn parse_service_action_logs_reads_instance_flag() {
        let args: Vec<String> = vec!["--instance".into(), "canary".into()];
        match parse_service_action("logs", &args).unwrap() {
            ServiceAction::Logs { instance } => {
                assert_eq!(instance.as_deref(), Some("canary"))
            }
            _ => panic!("expected Logs variant"),
        }

        let args: Vec<String> = vec!["--instance=canary".into()];
        match parse_service_action("logs", &args).unwrap() {
            ServiceAction::Logs { instance } => {
                assert_eq!(instance.as_deref(), Some("canary"))
            }
            _ => panic!("expected Logs variant"),
        }
    }

    /// Without `--instance`, logs defaults to the singleton (instance None),
    /// and the resolved target is the bare slot; with an id it is
    /// `<slot>@<id>` — the same composition `up` uses.
    #[test]
    fn logs_instance_resolution_matches_up_composition() {
        // No context: slot == workload name.
        let slot = slot_for("litellm", None);
        assert_eq!(slot, "litellm");
        assert_eq!(instance_name(&slot, None), "litellm");
        assert_eq!(instance_name(&slot, Some("canary")), "litellm@canary");

        // With a context the slot is namespaced, exactly like up's target.
        let slot = slot_for("litellm", Some("personal"));
        assert_eq!(
            instance_name(&slot, Some("canary")),
            "personal-litellm@canary"
        );
    }

    /// An invalid `--instance` id is rejected by the same validator the
    /// dispatch path runs before resolving the log path.
    #[test]
    fn logs_instance_id_is_validated() {
        for bad in ["all", "1234", "-leading", "UPPER", "has_underscore"] {
            assert!(
                validate_instance_id(bad).is_err(),
                "invalid id '{bad}' must be rejected"
            );
        }
        validate_instance_id("canary").expect("canary is a valid id");
    }
}
