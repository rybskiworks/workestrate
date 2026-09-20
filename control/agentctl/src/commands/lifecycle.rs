//! Workload lifecycle: instance-spec construction, service/agent dispatch,
//! raw-args action parsing, and teardown (`workload <name> down`, the
//! ladder-scoped `down <scope>`, `clean`).

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
/// Mutually-exclusive flag groups are validated here: `--new` conflicts with
/// both `--replace` and `--instance`; `--replace` + `--instance` is legal
/// (ADR 0030 V-addendum pin 5 — replace THAT instance).
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

/// The `InstanceSpec.source_dir` value for an up/exec dispatch (ADR 0030
/// V-addendum §V3): the CANONICAL invocation cwd for a `per-dir`-strategy
/// workload (recorded on the registry record at create; read back by the
/// source-gone reconcile state), `None` for every other strategy.
fn per_dir_source_dir(strategy: crate::config::InstanceStrategy) -> Result<Option<String>> {
    if strategy == crate::config::InstanceStrategy::PerDir {
        Ok(Some(crate::config::canonical_invoke_cwd_string()?))
    } else {
        Ok(None)
    }
}

/// Resolve the DEPENDENT workload's own parallel instance id BEFORE dep
/// auto-start (ADR 0030 P2.1). main.rs calls this for the `up`/`exec` verbs
/// so the dependent's id is known when `plan_dep_starts` composes scoped
/// dep instance ids (`<dep>@<dependent>-<id>`) and when fresh dep
/// selections are injected as `--use` overrides.
///
/// Semantics (mirrors [`parallel_strategy_defaults_new`]):
///
/// - explicit `--instance <id>` → `Some(id)` verbatim (passthrough);
/// - else `strategy = "per-dir"` WITHOUT `--new` →
///   `Some(per_dir_instance_id(canonical invocation cwd))` — deterministic,
///   no allocation (ADR 0030 V-addendum §V1). This applies INCLUDING with
///   `--replace` (pin 5: per-dir `--replace` replaces the cwd-keyed
///   instance);
/// - else `--new`, OR `strategy = "parallel"` without `--replace` →
///   `Some(auto_allocate_slug(state_dir, slot))` — the caller REWRITES the
///   action (`instance = Some(id)`, `new = false`) so
///   `dispatch_service`/`dispatch_agent` use the SAME id and never allocate
///   a second slug (with `instance` now `Some`, the `no_instance` guard in
///   the Up/Exec arms skips their allocation). `--new` on a per-dir
///   workload lands HERE (a FRESH, non-dir-keyed instance);
/// - else (`--replace` with the parallel strategy, singleton strategy, …) →
///   `None` (the singleton model; no allocation).
pub fn resolve_dependent_instance_id(
    name: &str,
    instance: Option<&str>,
    new: bool,
    replace: bool,
) -> Result<Option<String>> {
    if let Some(id) = instance {
        return Ok(Some(id.to_string()));
    }
    let config = crate::config::load_config()?;
    let strategy = config
        .workloads
        .get(name)
        .map(|w| w.instance.strategy)
        .unwrap_or(crate::config::InstanceStrategy::Singleton);
    if strategy == crate::config::InstanceStrategy::PerDir && !new {
        // ADR 0030 V-addendum §V1: the canonicalized invocation cwd keys the
        // instance — re-invoking from the same directory derives the SAME
        // id, a different/moved directory derives a new one.
        let canonical = crate::config::canonical_invoke_cwd_string()?;
        return Ok(Some(crate::microsandbox::slots::per_dir_instance_id(
            &canonical,
        )));
    }
    if parallel_strategy_defaults_new(strategy, new, replace, true) {
        let state_dir = crate::config::resolve_state_dir();
        let slot = crate::microsandbox::slots::slot_for(
            name,
            crate::config::active_context_name().as_deref(),
        );
        return Ok(Some(
            crate::microsandbox::port_registry::auto_allocate_slug(&state_dir, &slot)?,
        ));
    }
    Ok(None)
}

// too_many_arguments: the spec constructor mirrors the up/exec flag set
// positionally (one parameter per CLI flag); matches the pre-existing shape.
#[allow(clippy::too_many_arguments)]
pub fn build_instance_spec(
    workload_name: &str,
    replace: bool,
    instance_id: Option<&str>,
    new_id: Option<&str>,
    port_auto: bool,
    use_overrides: &[(String, String)],
    no_deps: bool,
    reseed: bool,
) -> Result<crate::microsandbox::runtime::InstanceSpec> {
    use crate::microsandbox::runtime::InstanceSpec;
    use crate::microsandbox::slots::{instance_name, slot_for, validate_instance_id};

    // ADR 0030 V-addendum (pin 5): `--replace` + `--instance <id>` is LEGAL
    // (replace THAT instance — the teardown machinery is already
    // instance-name-generic); per-dir `--replace` (replace the cwd-keyed
    // instance) and the pinned-instance smoke both rely on it. `--new` stays
    // mutually exclusive with BOTH (a fresh allocation cannot also name or
    // replace a specific instance).
    if new_id.is_some() && (replace || instance_id.is_some()) {
        anyhow::bail!(
            "--new is mutually exclusive with --replace and --instance <id>; \
             pass at most one of them"
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
        reseed,
        // Spec 21 §2.2: the ensure-images token defaults OFF here — callers
        // set it (a detached up's spec describes the ensured child-to-be;
        // the detached child reconstitutes it from its clap parse).
        images_ready: false,
        // ADR 0030 V-addendum §V3: the per-dir canonical source dir is set by
        // the dispatch arms (they know the workload's strategy); the spec
        // constructor defaults to None.
        source_dir: None,
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
    if let Some(name) = config::active_context_name()
        && !json
    {
        eprintln!("context: {}", name);
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
            reseed,
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
                reseed,
            )?;
            // Spec 21 §2.2: the spec of a DETACHED up describes the ensured
            // child-to-be (the parent ensures before spawning it), and the
            // detached child itself re-parsed --images-ready — both carry
            // the token. A plain foreground up is the parent (no token).
            spec.images_ready = images_ready || !foreground;
            // ADR 0030 V-addendum §V3: record the canonical invocation cwd
            // at create time for per-dir workloads (the registry record's
            // `source_dir`, read by the source-gone reconcile state).
            spec.source_dir = per_dir_source_dir(workload.instance_strategy())?;
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
        // `--use` was already applied by the caller's workload construction
        // (main.rs); cmd_plan renders THAT workload (no second resolution).
        ServiceAction::Plan { instance, use_: _ } => {
            cmd_plan(workload, show_source, json, instance.as_deref())
        }
    }
}

pub async fn dispatch_agent<W: Workload>(
    workload: &W,
    action: AgentAction,
    show_source: bool,
    json: bool,
) -> Result<()> {
    if let Some(name) = config::active_context_name()
        && !json
    {
        eprintln!("context: {}", name);
    }
    match action {
        AgentAction::Exec {
            replace,
            instance,
            new,
            port_auto,
            use_,
            no_deps,
            reseed,
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
                reseed,
            )?;
            // ADR 0030 V-addendum §V3: same source_dir recording as the
            // service Up arm above.
            let spec = crate::microsandbox::runtime::InstanceSpec {
                source_dir: per_dir_source_dir(workload.instance_strategy())?,
                ..spec
            };
            crate::microsandbox::runtime::exec_agent_with_spec(workload, &spec).await
        }
        AgentAction::Down {
            instance,
            all_instances,
        } => cmd_down(workload.name(), instance.as_deref(), all_instances, json).await,
        // Same as ServiceAction::Plan: the caller-constructed workload
        // already carries the --use resolution.
        AgentAction::Plan { instance, use_: _ } => {
            cmd_plan(workload, show_source, json, instance.as_deref())
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
            let reseed = args.iter().any(|a| a == "--reseed");
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
                reseed,
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
            let reseed = args.iter().any(|a| a == "--reseed");
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
                reseed,
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

/// The pinned interactive prompt for the EVERYTHING scope (ADR 0032
/// addendum §Down scope ladder double gate): names the widened blast
/// radius — msb sandboxes workestrate does NOT manage are included.
const EVERYTHING_PROMPT: &str = "This will stop EVERY msb sandbox INCLUDING ones \
     workestrate does not manage. Continue? [y/N] ";

/// Outcome of the ONE stdin-confirm read shared by every destructive verb's
/// yes-gate (`down` managed rungs, `down --everything`, `clean`).
#[derive(Debug, Clone, PartialEq, Eq)]
enum ConfirmAnswer {
    /// The answer token confirms (`y`/`yes`, case-insensitive, trimmed).
    Confirm,
    /// An explicit non-confirm token — decline.
    Decline,
    /// No answer given: EOF on stdin, or an empty/whitespace-only line (the
    /// `[y/N]` default-No). Aborts exactly like a decline.
    Eof,
    /// A genuine stdin READ FAILURE — surfaced to the caller, never
    /// swallowed into a silent abort. Carries the rendered error text (an
    /// `io::Error` is neither `Clone` nor `Eq`, which the pure-decision
    /// tests need).
    Error(String),
}

/// Pure decision core for [`read_confirm_answer`]: classify one line-read
/// result against the unified confirm tokens (`y`/`yes`, case-insensitive,
/// trimmed — matching every destructive verb). Split out so the confirm
/// contract is unit-testable without stdin plumbing.
fn classify_confirm_answer(read: std::io::Result<Option<String>>) -> ConfirmAnswer {
    match read {
        Ok(Some(line)) => match line.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => ConfirmAnswer::Confirm,
            "" => ConfirmAnswer::Eof,
            _ => ConfirmAnswer::Decline,
        },
        Ok(None) => ConfirmAnswer::Eof,
        Err(e) => ConfirmAnswer::Error(e.to_string()),
    }
}

/// THE ONE stdin-confirm reader for every destructive verb: the prompt shows
/// only on a tty, but ONE line is read in every mode so a piped "y"/"yes"
/// confirms and an empty/EOF stream declines. A genuine read ERROR comes
/// back as [`ConfirmAnswer::Error`] so the caller can surface it instead of
/// masquerading it as a decline.
fn read_confirm_answer(prompt: &str) -> ConfirmAnswer {
    use std::io::IsTerminal;
    if std::io::stdin().is_terminal() {
        eprint!("{prompt}");
        std::io::stderr().flush().ok();
    }
    use std::io::BufRead;
    let read = std::io::stdin().lock().lines().next().transpose();
    classify_confirm_answer(read)
}

/// The managed-rung yes-gate (`down <scope>`): prompt + read via
/// [`read_confirm_answer`]. Confirm proceeds (`Ok`). An explicit decline or
/// EOF-as-decline prints the JSON abort envelope (or plain "aborted") and
/// exits 1. A genuine stdin READ ERROR propagates as `Err` — distinct
/// message, standard error envelope under `--json` — instead of the old
/// silent-abort behavior.
fn confirm_or_abort(prompt: &str, abort_message: &str, json: bool) -> Result<()> {
    match read_confirm_answer(prompt) {
        ConfirmAnswer::Confirm => Ok(()),
        ConfirmAnswer::Decline | ConfirmAnswer::Eof => {
            abort_not_confirmed(abort_message, json);
        }
        ConfirmAnswer::Error(e) => {
            anyhow::bail!("could not read confirmation from stdin: {e}");
        }
    }
}

/// Print the aborted envelope (JSON mode) or plain "aborted" and exit 1 —
/// the shared refusal exit for every down-scope confirmation.
fn abort_not_confirmed(message: &str, json: bool) -> ! {
    if json {
        eprintln!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "error": { "kind": "aborted", "message": message }
            }))
            .unwrap_or_else(|_| "{\"error\":{\"kind\":\"aborted\"}}".to_string())
        );
    } else {
        eprintln!("aborted");
    }
    std::process::exit(1);
}

/// Interactive confirmation for the EVERYTHING scope's yes-gate half:
/// `Ok(Some(true/false))` from a tty prompt, `Ok(None)` when stdin is not a
/// tty (no confirmation available → [`everything_gate`] hard-refuses, the
/// cmd_clean posture). A genuine stdin READ ERROR propagates as `Err`
/// instead of masquerading as a decline.
fn everything_interactive_confirm() -> Result<Option<bool>> {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() {
        return Ok(None);
    }
    match read_confirm_answer(EVERYTHING_PROMPT) {
        ConfirmAnswer::Confirm => Ok(Some(true)),
        ConfirmAnswer::Decline | ConfirmAnswer::Eof => Ok(Some(false)),
        ConfirmAnswer::Error(e) => {
            anyhow::bail!("could not read confirmation from stdin: {e}");
        }
    }
}

/// The per-scope confirmation prompt for the MANAGED rungs (context /
/// config-ref / config). Config keeps the exact legacy down-all wording.
fn managed_scope_prompt(scope: &crate::microsandbox::runtime::down_scope::DownScope) -> String {
    use crate::microsandbox::runtime::down_scope::DownScope;
    match scope {
        DownScope::Config => {
            "This will stop EVERY running workestrate sandbox. Continue? [y/N] ".to_string()
        }
        DownScope::Context(ctx) => format!(
            "This will stop every workestrate-managed sandbox in context \
             '{ctx}'. Continue? [y/N] "
        ),
        DownScope::ConfigRef(r) => format!(
            "This will stop every workestrate-managed sandbox derived from \
             config ref '{r}' (context '{r}'). Continue? [y/N] "
        ),
        DownScope::Everything => EVERYTHING_PROMPT.to_string(),
    }
}

/// `workestrate down <scope>` — the ADR 0032 addendum §Down scope ladder:
/// enumerate the candidates for the resolved scope, classify them, resolve,
/// and tear EVERY selected target down through the hardened path
/// ([`down_hardened`]: stop → wait-exit → remove → unregister → policy-dir
/// → sandbox-dir). Per-target outcomes are reported; ANY failure exits
/// nonzero (`report_down_aggregate`). An EMPTY selection is Ok and reported
/// as `0 target(s)` — never an error.
///
/// Gates: the managed rungs (context/config-ref/config) take the standard
/// single yes-gate (interactive prompt; piped y confirms; `--yes` skips).
/// EVERYTHING is DOUBLE-GATED: the flag must appear twice AND pass the
/// yes-gate, whose non-interactive form hard-refuses without `--yes`
/// (cmd_clean posture).
pub async fn cmd_down_ladder(
    scope: crate::microsandbox::runtime::down_scope::DownScope,
    everything_count: u8,
    yes: bool,
    json: bool,
) -> Result<()> {
    use crate::microsandbox::runtime::down_hardened;
    use crate::microsandbox::runtime::down_scope::{
        DownScope, enumerate_all_candidates, enumerate_managed, everything_gate, known_config_refs,
        resolve_scope, validate_config_ref,
    };

    // ---- gates ----
    if let DownScope::Everything = scope {
        // Yes-gate half (interactive decline aborts exactly like the managed
        // rungs) — but ONLY once the doubled-flag half would even pass, so an
        // under-counted invocation gets its usage error without a prompt.
        let confirmed = if yes || everything_count < 2 {
            None
        } else {
            everything_interactive_confirm()?
        };
        if confirmed == Some(false) {
            abort_not_confirmed("down everything not confirmed", json);
        }
        everything_gate(everything_count, yes, confirmed)?;
    } else if !yes {
        confirm_or_abort(
            &managed_scope_prompt(&scope),
            &format!("down {} not confirmed", scope.description()),
            json,
        )?;
    }

    // ---- config-ref fail-closed validation (BEFORE any teardown) ----
    if let DownScope::ConfigRef(r) = &scope {
        let known = known_config_refs()?;
        validate_config_ref(r, &known)?;
    }

    // ---- enumeration + resolution ----
    let state_dir = crate::config::resolve_state_dir();
    let targets = match &scope {
        DownScope::Everything => enumerate_all_candidates(&state_dir).await?,
        _ => enumerate_managed(&state_dir).await?,
    };
    let selected = resolve_scope(&scope, &targets);

    // ---- hardened teardown at every scope ----
    let mut results = Vec::with_capacity(selected.len());
    for t in &selected {
        results.push(down_hardened(&state_dir, &t.instance).await);
    }

    // ---- retained-generation sweeps (BROAD rungs only) ----
    // Config/Everything sweep EVERY retained generation home, not just the
    // resolved one (msb state generations): the record-driven targets were
    // torn down above, ONCE, against the resolved home (the workestrate
    // port registry is generation-agnostic); each EXTRA generation
    // contributes its own dir-driven candidates, torn down with MSB_HOME
    // pinned to that generation dir. Targeted rungs stay current-only.
    let generation_sweeps = if matches!(scope, DownScope::Config | DownScope::Everything) {
        down_retained_generations(&state_dir, &scope).await
    } else {
        Vec::new()
    };

    // ---- outcomes: ONE scope header line + per-target lines / JSON ----
    let description = scope.description();
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&crate::json_out::down_scope_results_json(
                &description,
                &results,
                &generation_sweeps
            ))?
        );
    } else {
        println!("down {description}: {} target(s)", results.len());
        print_down_results_text(&results);
        for (generation, gen_results) in &generation_sweeps {
            println!(
                "down {description} (generation {generation}): {} target(s)",
                gen_results.len()
            );
            print_down_results_text(gen_results);
        }
    }
    // ANY per-target failure in ANY swept home exits nonzero — the same
    // aggregate posture as the single-config sweep.
    let all_results: Vec<crate::microsandbox::runtime::DownResult> = results
        .iter()
        .chain(
            generation_sweeps
                .iter()
                .flat_map(|(_, gen_results)| gen_results.iter()),
        )
        .cloned()
        .collect();
    report_down_aggregate(&all_results)
}

/// The retained-generation sweep for the BROAD down rungs (msb state
/// generations): for each extra generation home from
/// [`retained_generation_homes`], pin `MSB_HOME` to it (save/set/restore
/// via the shared EnvGuard; restored on drop even on error), enumerate
/// that generation's DIR-driven candidates, resolve the same scope against
/// them, and tear each selected target down through the hardened path.
/// Record-driven targets are NOT re-swept here — the workestrate port
/// registry is generation-agnostic and was handled once against the
/// resolved home. A generation dir that vanished mid-sweep is logged and
/// skipped, never a hard error. Sequential (the ladder is sequential), so
/// the env pinning cannot race.
///
/// [`retained_generation_homes`]: crate::microsandbox::runtime::down_scope::retained_generation_homes
#[allow(unsafe_code)]
async fn down_retained_generations(
    state_dir: &std::path::Path,
    scope: &crate::microsandbox::runtime::down_scope::DownScope,
) -> Vec<(String, Vec<crate::microsandbox::runtime::DownResult>)> {
    use crate::microsandbox::runtime::down_hardened;
    use crate::microsandbox::runtime::down_scope::{
        DownScope, enumerate_generation_dir_candidates, resolve_scope, retained_generation_homes,
    };
    let mut sweeps = Vec::new();
    for gen_home in retained_generation_homes() {
        let key = gen_home
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        // A generation dir that vanished mid-sweep: log + skip, not a hard
        // error.
        if !gen_home.is_dir() {
            eprintln!(
                "warning: retained generation home {} vanished mid-sweep; skipping",
                gen_home.display()
            );
            continue;
        }
        // Pin MSB_HOME to THIS generation for the enumeration + teardown
        // section; the guard restores the prior value on drop even on
        // error (it also snapshots cwd, which is untouched here).
        let _pin = crate::config::test_support::EnvGuard::capture(&["MSB_HOME"]);
        // SAFETY: MSB_HOME is pinned per-generation and restored by the
        // EnvGuard on drop (even on error); the sweep is sequential, so no
        // concurrent env mutation of this key.
        unsafe { std::env::set_var("MSB_HOME", gen_home) };
        let candidates =
            enumerate_generation_dir_candidates(matches!(scope, DownScope::Everything));
        let selected = resolve_scope(scope, &candidates);
        let mut results = Vec::with_capacity(selected.len());
        for t in &selected {
            results.push(down_hardened(state_dir, &t.instance).await);
        }
        sweeps.push((key, results));
    }
    sweeps
}

/// `workestrate clean` — remove the CONTENTS of the volatile state-dir
/// subdirectories (`workspaces/`, `var/`, `run/`), leaving the directories
/// themselves in place. Never touches the store (`fleets/`, `sources/`)
/// or any
/// config file. Interactive confirmation unless `--yes`; non-interactive
/// stdin without `--yes` is a hard refusal (same policy as `down --all`).
/// Declines route through [`abort_not_confirmed`] so every destructive verb
/// emits the same JSON abort envelope; stdin read errors propagate instead
/// of aborting silently.
pub fn cmd_clean(yes: bool, json: bool) -> Result<()> {
    use std::io::IsTerminal;
    let state_dir = config::resolve_state_dir();
    const SUBDIRS: [&str; 3] = ["workspaces", "var", "run"];

    if !yes {
        if std::io::stdin().is_terminal() {
            let prompt = format!(
                "This will remove contents of {}/{{workspaces,var,run}}. Continue? [y/N] ",
                state_dir.display()
            );
            match read_confirm_answer(&prompt) {
                ConfirmAnswer::Confirm => {}
                ConfirmAnswer::Decline | ConfirmAnswer::Eof => {
                    abort_not_confirmed("clean not confirmed", json);
                }
                ConfirmAnswer::Error(e) => {
                    anyhow::bail!("could not read confirmation from stdin: {e}");
                }
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
#[allow(unsafe_code)]
mod tests {
    use super::*;

    // ---- destructive-verb confirm contract (classify_confirm_answer) ----

    /// The ONE confirm-token rule shared by every destructive verb:
    /// `y`/`yes` (case-insensitive, trimmed) confirms; any other non-empty
    /// token declines; an empty line or EOF is EOF-as-decline (the `[y/N]`
    /// default); a read ERROR is its own variant so it can surface instead
    /// of masquerading as a decline.
    #[test]
    fn classify_confirm_answer_matrix() {
        use super::{ConfirmAnswer, classify_confirm_answer};
        let ok = |line: &str| Ok(Some(line.to_string()));
        // Confirm tokens.
        for line in ["y", "Y", "yes", "YES", "Yes", " y ", "\tyes\t"] {
            assert_eq!(
                classify_confirm_answer(ok(line)),
                ConfirmAnswer::Confirm,
                "'{line}' must confirm"
            );
        }
        // Explicit declines.
        for line in ["n", "N", "no", "NO", "abort", "y es", "yes!", "0"] {
            assert_eq!(
                classify_confirm_answer(ok(line)),
                ConfirmAnswer::Decline,
                "'{line}' must decline"
            );
        }
        // EOF-as-decline: no answer at all, or the bare [y/N] default.
        assert_eq!(classify_confirm_answer(Ok(None)), ConfirmAnswer::Eof);
        assert_eq!(classify_confirm_answer(ok("")), ConfirmAnswer::Eof);
        assert_eq!(classify_confirm_answer(ok("   ")), ConfirmAnswer::Eof);
        // A genuine read error is distinguishable from every decline shape —
        // the anomaly fix: it must never be flattened into a silent abort.
        let err = classify_confirm_answer(Err(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "pipe closed",
        )));
        assert!(
            matches!(err, ConfirmAnswer::Error(ref m) if m.contains("pipe closed")),
            "a read error must surface as Error carrying the io message; got {err:?}"
        );
    }

    // ---- ADR 0032 addendum §Down scope ladder: outcome aggregation ----

    /// Partial failure (one Error among Stopped) → aggregate error (nonzero
    /// exit) — while every per-target outcome is still printed first
    /// (`print_down_results_text` renders the WHOLE slice unfiltered; the
    /// aggregate check runs only after). Pure pin of the exit rule.
    #[test]
    fn report_down_aggregate_fails_on_any_error_among_stopped() {
        use crate::microsandbox::runtime::{DownResult, DownStatus};
        let results = vec![
            DownResult {
                instance: "personal-litellm".to_string(),
                status: DownStatus::Stopped,
                message: None,
            },
            DownResult {
                instance: "work-pi".to_string(),
                status: DownStatus::Error,
                message: Some("msb unreachable".to_string()),
            },
            DownResult {
                instance: "team-worker".to_string(),
                status: DownStatus::NotFound,
                message: None,
            },
        ];
        let err = report_down_aggregate(&results).unwrap_err().to_string();
        assert!(
            err.contains("failed to stop"),
            "any Error must fail the aggregate: {err}"
        );
        // All-stopped and empty selections are Ok (empty = 0 target(s), exit 0).
        assert!(report_down_aggregate(&results[..1]).is_ok());
        assert!(report_down_aggregate(&[]).is_ok());
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

    // ---- --reseed raw-args parsing ----

    #[test]
    fn parse_service_action_up_reads_reseed() {
        let args: Vec<String> = vec!["--reseed".into()];
        match parse_service_action("up", &args).unwrap() {
            ServiceAction::Up { reseed, .. } => assert!(reseed),
            _ => panic!("expected Up variant"),
        }
        let args: Vec<String> = vec!["--foreground".into()];
        match parse_service_action("up", &args).unwrap() {
            ServiceAction::Up { reseed, .. } => assert!(!reseed),
            _ => panic!("expected Up variant"),
        }
    }

    #[test]
    fn parse_agent_action_exec_reads_reseed() {
        let args: Vec<String> = vec!["--reseed".into()];
        match parse_agent_action("exec", &args).unwrap() {
            AgentAction::Exec { reseed, .. } => assert!(reseed),
            _ => panic!("expected Exec variant"),
        }
        let args: Vec<String> = vec!["--replace".into()];
        match parse_agent_action("exec", &args).unwrap() {
            AgentAction::Exec { reseed, .. } => assert!(!reseed),
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
        let spec =
            build_instance_spec("pi", false, None, None, false, &overrides, false, false).unwrap();
        assert_eq!(spec.use_overrides, overrides);
        assert!(!spec.no_deps);
        assert!(!spec.reseed);
        let spec = build_instance_spec("pi", false, None, None, false, &[], false, false).unwrap();
        assert!(spec.use_overrides.is_empty());
    }

    /// build_instance_spec stores --no-deps on the spec so detach_args can
    /// forward it to the detached child (ADR 0026 addendum).
    #[test]
    fn build_instance_spec_stores_no_deps() {
        let spec = build_instance_spec("pi", false, None, None, false, &[], true, false).unwrap();
        assert!(spec.no_deps);
    }

    /// build_instance_spec stores --reseed on the spec so detach_args can
    /// forward it to the detached child.
    #[test]
    fn build_instance_spec_stores_reseed() {
        let spec = build_instance_spec("pi", false, None, None, false, &[], false, true).unwrap();
        assert!(spec.reseed);
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

    // ---- ADR 0030 P2.1: resolve_dependent_instance_id ----

    /// An explicit `--instance <id>` passes through verbatim — no config
    /// load, no allocation.
    #[test]
    fn resolve_dependent_instance_id_explicit_passthrough() {
        assert_eq!(
            resolve_dependent_instance_id("pi", Some("canary"), false, false).unwrap(),
            Some("canary".to_string())
        );
        // Explicit id wins even alongside --new/--replace (clap validates
        // the mutual exclusion downstream; the passthrough never allocates).
        assert_eq!(
            resolve_dependent_instance_id("pi", Some("canary"), true, false).unwrap(),
            Some("canary".to_string())
        );
    }

    /// A parallel-strategy workload with no flags allocates a fresh slug
    /// from the registry view of a REAL temp state dir.
    #[test]
    fn resolve_dependent_instance_id_parallel_strategy_allocates() {
        use crate::config::test_support::{ENV_TEST_LOCK, EnvGuard, uniq_dir};
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _env = EnvGuard::capture(&["WORKESTRATE_FLEET_DIR", "WORKESTRATE_STATE_DIR"]);
        let cfg_dir = uniq_dir("depid-cfg");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        std::fs::write(
            cfg_dir.join("workestrate.toml"),
            r#"schema_version = 1

[workloads.par]
kind = "service"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.par.instance]
strategy = "parallel"
"#,
        )
        .unwrap();
        let state_dir = uniq_dir("depid-state");
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_FLEET_DIR", &cfg_dir) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_STATE_DIR", &state_dir) };

        let id = resolve_dependent_instance_id("par", None, false, false).unwrap();
        let slug = id.expect("parallel strategy with no flags must allocate a slug");
        crate::microsandbox::slots::validate_instance_id(&slug)
            .expect("the allocated slug must satisfy the instance-id rule");

        // --replace with the parallel strategy does NOT allocate (replace
        // targets the singleton — existing semantics).
        assert_eq!(
            resolve_dependent_instance_id("par", None, false, true).unwrap(),
            None,
            "replace + parallel must not allocate"
        );
        let _ = std::fs::remove_dir_all(&cfg_dir);
        let _ = std::fs::remove_dir_all(&state_dir);
    }

    // ---- ADR 0030 V-addendum pin 5: --replace + --instance is legal ----

    /// `--replace` + `--instance <id>` builds a spec targeting THAT instance
    /// with replace set (teardown machinery is instance-name-generic).
    #[test]
    fn build_instance_spec_allows_replace_with_instance() {
        let spec = build_instance_spec("pi", true, Some("canary"), None, false, &[], false, false)
            .expect("--replace --instance canary must be legal (ADR 0030 V-addendum pin 5)");
        assert!(spec.replace);
        assert!(
            spec.instance.ends_with("@canary"),
            "the spec targets the named instance: {}",
            spec.instance
        );
    }

    /// `--new` stays mutually exclusive with BOTH --replace and --instance.
    #[test]
    fn build_instance_spec_new_still_exclusive() {
        let err = match build_instance_spec("pi", true, None, Some("x7"), false, &[], false, false)
        {
            Ok(_) => panic!("new+replace must fail"),
            Err(e) => e.to_string(),
        };
        assert!(
            err.contains("--new is mutually exclusive"),
            "new+replace must fail: {err}"
        );
        let err = match build_instance_spec(
            "pi",
            false,
            Some("canary"),
            Some("x7"),
            false,
            &[],
            false,
            false,
        ) {
            Ok(_) => panic!("new+instance must fail"),
            Err(e) => e.to_string(),
        };
        assert!(
            err.contains("--new is mutually exclusive"),
            "new+instance must fail: {err}"
        );
    }

    /// A per-dir-strategy workload with no explicit flags derives the
    /// deterministic cwd-keyed id — INCLUDING with `--replace` (pin 5:
    /// replace the cwd-keyed instance). `--new` takes the fresh-allocation
    /// path instead.
    #[test]
    fn resolve_dependent_instance_id_per_dir_derives_cwd_keyed_id() {
        use crate::config::test_support::{ENV_TEST_LOCK, EnvGuard, uniq_dir};
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _env = EnvGuard::capture(&[
            "WORKESTRATE_FLEET_DIR",
            "WORKESTRATE_STATE_DIR",
            crate::config::INVOKE_CWD_ENV,
        ]);
        let cfg_dir = uniq_dir("depid-cfg-perdir");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        std::fs::write(
            cfg_dir.join("workestrate.toml"),
            r#"schema_version = 1

[workloads.pd]
kind = "service"
image = { recipe = "registry", ref = "node:24" }
command = []

[[workloads.pd.mounts]]
host = "${CWD}"
guest = "/work"

[workloads.pd.instance]
strategy = "per-dir"
"#,
        )
        .unwrap();
        let state_dir = uniq_dir("depid-state-perdir");
        let invoke = uniq_dir("depid-invoke-perdir");
        std::fs::create_dir_all(&invoke).unwrap();
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_FLEET_DIR", &cfg_dir) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_STATE_DIR", &state_dir) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe {
            std::env::set_var(
                crate::config::INVOKE_CWD_ENV,
                std::fs::canonicalize(&invoke).unwrap(),
            )
        };

        let expected = Some(crate::microsandbox::slots::per_dir_instance_id(
            &std::fs::canonicalize(&invoke).unwrap().to_string_lossy(),
        ));
        // No flags → derived id.
        assert_eq!(
            resolve_dependent_instance_id("pd", None, false, false).unwrap(),
            expected
        );
        // --replace → the SAME derived id (replace the cwd-keyed instance).
        assert_eq!(
            resolve_dependent_instance_id("pd", None, false, true).unwrap(),
            expected
        );
        // --new → a FRESH auto-allocated slug, NOT the dir-keyed id.
        let fresh = resolve_dependent_instance_id("pd", None, true, false)
            .unwrap()
            .expect("--new must allocate");
        assert_ne!(Some(fresh), expected, "--new must not be dir-keyed");
        let _ = std::fs::remove_dir_all(&cfg_dir);
        let _ = std::fs::remove_dir_all(&state_dir);
        let _ = std::fs::remove_dir_all(&invoke);
    }

    /// A singleton-strategy workload with no flags stays on the singleton
    /// (None); `--new` allocates even then.
    #[test]
    fn resolve_dependent_instance_id_singleton_strategy_none_unless_new() {
        use crate::config::test_support::{ENV_TEST_LOCK, EnvGuard, uniq_dir};
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _env = EnvGuard::capture(&["WORKESTRATE_FLEET_DIR", "WORKESTRATE_STATE_DIR"]);
        let cfg_dir = uniq_dir("depid-cfg-single");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        std::fs::write(
            cfg_dir.join("workestrate.toml"),
            crate::config::test_support::one_workload_toml("solo"),
        )
        .unwrap();
        let state_dir = uniq_dir("depid-state-single");
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_FLEET_DIR", &cfg_dir) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_STATE_DIR", &state_dir) };

        assert_eq!(
            resolve_dependent_instance_id("solo", None, false, false).unwrap(),
            None,
            "singleton strategy with no flags must not allocate"
        );
        let id = resolve_dependent_instance_id("solo", None, true, false).unwrap();
        assert!(id.is_some(), "--new must allocate even for singleton");
        let _ = std::fs::remove_dir_all(&cfg_dir);
        let _ = std::fs::remove_dir_all(&state_dir);
    }

    /// A5 Session 3b (PINNED id precedence): the inline `:ref`-derived
    /// instance id beats the per-dir derivation. main.rs injects the
    /// derived id as the explicit `instance` BEFORE this resolver runs
    /// (rewrite_action_for_inline_selector), so the passthrough arm returns
    /// it verbatim WITHOUT consulting the per-dir strategy — this test pins
    /// that composition on a per-dir-strategy workload.
    #[test]
    fn resolve_dependent_instance_id_inline_ref_instance_beats_per_dir() {
        use crate::config::test_support::{ENV_TEST_LOCK, EnvGuard, uniq_dir};
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _env = EnvGuard::capture(&[
            "WORKESTRATE_FLEET_DIR",
            "WORKESTRATE_STATE_DIR",
            crate::config::INVOKE_CWD_ENV,
        ]);
        let cfg_dir = uniq_dir("depid-cfg-inline-perdir");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        std::fs::write(
            cfg_dir.join("workestrate.toml"),
            r#"schema_version = 1

[workloads.pd]
kind = "service"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pd.instance]
strategy = "per-dir"
"#,
        )
        .unwrap();
        let state_dir = uniq_dir("depid-state-inline-perdir");
        let invoke = uniq_dir("depid-invoke-inline-perdir");
        std::fs::create_dir_all(&invoke).unwrap();
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_FLEET_DIR", &cfg_dir) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_STATE_DIR", &state_dir) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe {
            std::env::set_var(
                crate::config::INVOKE_CWD_ENV,
                std::fs::canonicalize(&invoke).unwrap(),
            )
        };

        let per_dir = crate::microsandbox::slots::per_dir_instance_id(
            &std::fs::canonicalize(&invoke).unwrap().to_string_lossy(),
        );
        assert_eq!(
            resolve_dependent_instance_id("pd", Some("feat-x"), false, false).unwrap(),
            Some("feat-x".to_string()),
            "the inline-ref-derived id must beat the per-dir derivation ({per_dir})"
        );
        let _ = std::fs::remove_dir_all(&cfg_dir);
        let _ = std::fs::remove_dir_all(&state_dir);
        let _ = std::fs::remove_dir_all(&invoke);
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
