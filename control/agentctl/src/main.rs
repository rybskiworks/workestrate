use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use std::collections::HashSet;
use std::path::PathBuf;

use workestrate::cli_actions::{
    AgentAction, ConfigAction, ContextAction, HomeAction, ImagesAction, PolicyAction,
    SchemasAction, ServiceAction, SourceAction, WorkloadAction,
};
use workestrate::cli_error::{classify_exit_code, emit_error};
use workestrate::commands::config_cmd::{
    cmd_config, cmd_config_list_json, cmd_config_new, cmd_context,
};
use workestrate::commands::deps::{auto_start_dependencies, cmd_workload_up_all};
use workestrate::commands::diagnostics::{
    cmd_check, cmd_generate_env_example, cmd_generate_schema, cmd_instances, cmd_msb, cmd_ps,
    cmd_run, cmd_validate_config, cmd_workloads,
};
use workestrate::commands::doctor::cmd_doctor;
use workestrate::commands::home::cmd_home;
use workestrate::commands::init::{cmd_init, cmd_new};
use workestrate::commands::lifecycle::{
    WorkloadRoute, cmd_clean, cmd_down_ladder, dispatch_agent, dispatch_service,
    resolve_dependent_instance_id, workload_route,
};
use workestrate::commands::migrate::cmd_migrate_home;
use workestrate::commands::schemas::cmd_schemas;
use workestrate::commands::secrets_target::{cmd_secrets_schema, cmd_secrets_target};
use workestrate::commands::source::cmd_source;
use workestrate::commands::versions::cmd_versions;
use workestrate::microsandbox::workload::ConfigWorkload;

#[derive(Parser)]
#[command(name = "workestrate")]
#[command(about = "Control plane CLI for the AI workbench")]
#[command(version = env!("WORKESTRATE_VERSION"))]
struct Cli {
    #[arg(long, help = "Disable project-layer config loading")]
    no_project_config: bool,

    #[arg(long, global = true, help = "Show source layer for each plan field")]
    show_source: bool,

    #[arg(long, global = true, help = "Active context name")]
    context: Option<String>,

    #[arg(
        long,
        global = true,
        value_name = "REF",
        help = "Consume every git-backed config entry at this branch/sha and derive the context from it (ADR 0032 addendum: the --config-ref ladder rung)"
    )]
    config_ref: Option<String>,

    #[arg(
        long,
        global = true,
        value_name = "DIR",
        help = "Workestrate tool home"
    )]
    home: Option<PathBuf>,

    /// Emit machine-readable JSON to stdout and an error envelope to stderr
    /// on failure. Applies to: ps, plan, config list, down (per-instance
    /// results), and the up/exec/down error envelope.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Serve one private control endpoint for selected running instances
    Control {
        #[command(subcommand)]
        action: ControlAction,
    },
    /// Runtime/config sanity check
    Check,
    /// Initialize workestrate configuration (DEPRECATED: use `home init`;
    /// the [url] dotfiles positional errors — use `home clone <src>`)
    Init {
        /// DEPRECATED: passing a url errors; use `home clone <src>` instead
        url: Option<String>,
    },
    /// Scaffold a new agent project (DEPRECATED: use `workload new <name>`)
    #[command(hide = true)]
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
    // Locked-`--` spike decision (ADR 0036 D4): a per-subcommand
    // `external_subcommand` field (which would fire only after `msb` matches,
    // so no verb-first/legacy-shim conflict) was considered to allow
    // `workestrate msb ps` without `--`, but it is NOT adopted: help/JSON/
    // completion cleanliness is unverifiable without a toolchain, so the
    // plan D4 fallback locks form (b) — the explicit `--` passthrough below,
    // matching the `Run` precedent.
    /// Passthrough to the msb binary: `workestrate msb -- <args>` forwards
    /// <args> verbatim to msb. `workestrate completions` covers wrapper
    /// flags only, never inner msb args (static-only; full dynamic
    /// completion needs a fork-side `completion` port, then delegation).
    Msb {
        /// msb arguments (after --)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 1..)]
        args: Vec<String>,
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
    /// Stop sandboxes at an explicit scope (ADR 0032 addendum §Down scope
    /// ladder: instance < workload < context < config-ref < home <
    /// everything). Exactly ONE scope selector per invocation; bare `down`
    /// is a usage error naming the ladder. The instance/workload rungs stay
    /// on `workload <name> down [--instance|--all-instances]`.
    /// Destructive; confirms unless --yes (--everything is DOUBLE-gated).
    #[command(alias = "down-all")]
    Down {
        /// Home scope: every workestrate-managed target (back-compat with
        /// the former `down-all` behavior).
        #[arg(long, conflicts_with_all = ["context", "config_ref", "everything"])]
        all: bool,
        /// Context scope: every managed target whose record context
        /// (primary) or `<ctx>-` slot prefix (corroborating) matches.
        #[arg(long, value_name = "CTX", conflicts_with_all = ["all", "config_ref", "everything"])]
        context: Option<String>,
        /// Config-ref scope: a VALIDATED branch-shaped ref; resolves as the
        /// context the branch implies (a sha implies nothing and is refused).
        #[arg(long, value_name = "REF", conflicts_with_all = ["all", "context", "everything"])]
        config_ref: Option<String>,
        /// Everything scope (DOUBLE-GATED): give it TWICE plus the standard
        /// yes-gate; stops EVERY msb sandbox including unmanaged ones.
        #[arg(
            long,
            action = clap::ArgAction::Count,
            conflicts_with_all = ["all", "context", "config_ref"]
        )]
        everything: u8,
        /// Skip the interactive confirmation.
        #[arg(long, help = "Skip the interactive confirmation")]
        yes: bool,
    },
    /// Remove state-dir contents (workspaces, var, run). Does not touch config-repos/sources/config.
    Clean {
        /// Skip the interactive confirmation.
        #[arg(long)]
        yes: bool,
    },
    /// Manage nix-layered images in the msb store (ADR 0032 §Image tags).
    Images {
        #[command(subcommand)]
        action: ImagesAction,
    },
    /// Manage workestrate contexts
    Context {
        #[command(subcommand)]
        action: ContextAction,
    },
    /// Print the JSON Schema for workestrate.toml to stdout (or write to
    /// --output; `--out` is accepted as a hidden back-compat alias).
    /// The schema is generated from the same serde/schemars types the config
    /// loader uses (single source of truth; ADR 0021 §8).
    GenerateSchema {
        /// Write the schema to this path instead of stdout.
        /// (`--out` is accepted as a hidden back-compat alias.)
        #[arg(short, long, value_name = "PATH", alias = "out")]
        output: Option<std::path::PathBuf>,
        /// Also write the bare-workload subschema to this path (requires
        /// --output). The subschema is derived from WorkloadConfig with the
        /// same post-processing that matched the previous hand-derived file.
        #[arg(long, value_name = "PATH")]
        output_workload: Option<std::path::PathBuf>,
        /// Also write the tool-home registry schema to this path (requires
        /// --output). The registry schema is derived from the Registry type.
        #[arg(long, value_name = "PATH")]
        output_registry: Option<std::path::PathBuf>,
    },
    /// Manage config repositories and trusted projects
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Manage the workestrate tool home (init as a dotfiles-style git repo)
    Home {
        #[command(subcommand)]
        action: HomeAction,
    },
    /// Manage the generated JSON Schema artifacts (workestrate.schema.json +
    /// workestrate-workload.schema.json + registry.schema.json) at every consumer location.
    Schemas {
        #[command(subcommand)]
        action: SchemasAction,
    },
    /// Resolve a registered config repo's secrets target paths (for setup-secrets).
    SecretsTarget {
        /// Config repo name to resolve.
        name: String,
    },
    /// Diagnose environment and tool health (KVM, nix, sops, age, msb, config repos).
    /// Use the global --json flag for machine-readable output.
    Doctor,
    /// Print the Phase-0 observability quadruple (workestrate/msb/agentd/
    /// libkrunfw versions plus the db-schema marker and fork-rev pin).
    /// Use the global --json flag for machine-readable output.
    Versions,
    /// Manage agent source checkouts
    Source {
        #[command(subcommand)]
        action: SourceAction,
    },
    /// Migrate legacy XDG (or bundled .workestrate/{config,data,state}/workestrate/)
    /// layout into a single WORKESTRATE_HOME (ADR 0023).
    MigrateHome {
        /// Source layout to migrate from: "xdg" (XDG_*_HOME dirs) or "bundle"
        /// (the old .workestrate/{config,data,state}/workestrate/ triplication).
        /// When omitted, auto-detect: prefer bundle if a .workestrate/config.toml
        /// exists in the target, else xdg.
        #[arg(long, value_name = "LAYOUT")]
        from: Option<String>,
        /// Show what would happen; move nothing.
        #[arg(long)]
        dry_run: bool,
        /// Allow migrating into a destination that already exists / is non-empty.
        #[arg(long)]
        force: bool,
    },
    /// Run a configured workload: up/exec/plan/down/logs <name>, or scaffold
    /// one with `new <name>` (ADR 0027).
    /// Workload names are ARGUMENTS, never subcommands, so a config-defined
    /// workload can never be shadowed by a built-in verb.
    Workload {
        #[command(subcommand)]
        action: WorkloadAction,
    },
    /// List configured workloads (name, kind, image, running status).
    Workloads,
    /// List instances across the registry + msb with the reconciled 5-state
    /// status (ADR 0030 §4.4). Optional <workload> filter. Use --json for
    /// machine-readable output.
    Instances {
        /// Optional workload name to filter to.
        workload: Option<String>,
    },
    /// Diagnose mount masking policy (spec 22 §13).
    Policy {
        #[command(subcommand)]
        action: PolicyAction,
    },
}

#[derive(Subcommand)]
enum ControlAction {
    /// Adopt exact live launches without starting or stopping their VMs
    Serve {
        /// Existing canonical private directory for endpoint and desired state
        #[arg(long)]
        state_dir: PathBuf,
        /// Existing instance selector; repeat to share one owner across instances
        #[arg(long = "instance", required = true, action = clap::ArgAction::Append)]
        instances: Vec<String>,
        /// Explicitly initialize a new desired store; otherwise require one
        #[arg(long)]
        initialize: bool,
    },
}

/// Pre-scan argv for a global `--json` flag so ANY error (including clap
/// parse errors, which call process::exit before `cli.json` is available) can
/// be formatted as the JSON envelope. Scanning stops at the first `--`
/// separator or at the `run`/`msb` subcommand: both capture all trailing args
/// verbatim as the payload, so a payload `--json` (e.g.
/// `workestrate run -- somecmd --json`, `workestrate msb -- foo --json`)
/// must not enable JSON mode.
fn json_mode_from_args(args: &[String]) -> bool {
    for a in args.iter().skip(1) {
        if a == "--json" {
            return true;
        }
        if a == "--" || a == "run" || a == "msb" {
            break;
        }
    }
    false
}

/// One-cycle deprecation shim for the pre-ADR-0027 invocation shape
/// `workestrate <name> <verb> ...` → `workestrate workload <verb> <name> ...`.
///
/// Pure and total: it NEVER makes a valid invocation fail. A rewrite happens
/// only when ALL of these hold:
///   - `args[1]` exists, does not start with `-` (a leading global flag at
///     the subcommand position leaves argv untouched), and is NOT a known
///     built-in subcommand name (built-ins ALWAYS win — clap precedence is
///     preserved by never shimming a name clap would match),
///   - `args[2]` is one of the five workload verbs,
///   - `is_workload(args[1])` confirms the name is a configured workload.
///
/// Any doubt → the argv is returned unchanged with no warning.
///
/// Returns the (possibly rewritten) argv plus an optional deprecation
/// warning for the caller to emit on stderr.
fn rewrite_legacy_workload_argv(
    args: Vec<String>,
    known: &HashSet<String>,
    is_workload: impl Fn(&str) -> bool,
) -> (Vec<String>, Option<String>) {
    const VERBS: [&str; 5] = ["up", "exec", "plan", "down", "logs"];
    let Some(name) = args.get(1) else {
        return (args, None);
    };
    if name.starts_with('-') || known.contains(name.as_str()) {
        return (args, None);
    }
    let Some(verb) = args.get(2) else {
        return (args, None);
    };
    if !VERBS.contains(&verb.as_str()) {
        return (args, None);
    }
    if !is_workload(name) {
        return (args, None);
    }
    let warning = format!(
        "warning: `workestrate {name} {verb} ...` is deprecated; use `workestrate workload {verb} {name} ...`"
    );
    let mut rewritten = Vec::with_capacity(args.len() + 1);
    rewritten.push(args[0].clone());
    rewritten.push("workload".to_string());
    rewritten.push(verb.clone());
    rewritten.push(name.clone());
    rewritten.extend(args.iter().skip(3).cloned());
    (rewritten, Some(warning))
}

/// ADR 0030 P2.1 + V-addendum §V1: the dependent-instance id a `workload
/// plan` previews.
///
/// - An explicit `--instance <id>` passes through verbatim.
/// - With no `--instance`, a `per-dir`-strategy workload previews the
///   DERIVED cwd-keyed id (`per_dir_instance_id` of the canonical invocation
///   cwd) — deterministic, NO allocation. Any other strategy is `None` (the
///   singleton view).
///
/// Plan is read-only and must NEVER allocate a slug, so
/// [`resolve_dependent_instance_id`] (which auto-allocates for the parallel
/// strategy) is deliberately NOT called on this path. The caller resolves
/// `strategy` from the loaded config (the only fallible/I/O step); the
/// derivation itself is pure.
fn plan_preview_instance_id(
    action: &WorkloadAction,
    strategy: workestrate::config::InstanceStrategy,
) -> Result<Option<String>> {
    match action {
        WorkloadAction::Plan { instance, .. } => {
            if let Some(id) = instance {
                return Ok(Some(id.clone()));
            }
            if strategy == workestrate::config::InstanceStrategy::PerDir {
                let canonical = workestrate::config::canonical_invoke_cwd_string()?;
                return Ok(Some(workestrate::microsandbox::slots::per_dir_instance_id(
                    &canonical,
                )));
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

/// ADR 0030 P2.1: after [`resolve_dependent_instance_id`] and
/// [`auto_start_dependencies`], rewrite the clap-parsed `up`/`exec` action
/// in place so the rest of the pipeline sees ONE consistent view:
///
/// - the resolved dependent id becomes an explicit `instance = Some(id)`
///   with `new = false`, so `dispatch_service`/`dispatch_agent` reuse the
///   SAME id (their `no_instance` auto-slug guard then skips — no second
///   allocation) and `detach_args` forwards `--instance <id>` to the child;
/// - every FRESH dep selection the auto-start executor started is appended
///   as `--use <dep>@<slug>`, so `build_instance_spec` records it on the
///   spec and `detach_args` forwards it — the detached child's own planner
///   marks the fresh dep Satisfied (no second allocation). Clap's flag
///   parsing already ran, so the rewrite cannot trip clap-level exclusion;
///   `build_instance_spec`'s exclusive-group check sees `new = false`.
///
/// Other verbs (plan/down/logs) carry no fresh selections and no id to
/// rewrite — left untouched.
fn rewrite_action_for_resolved_instance(
    action: &mut WorkloadAction,
    dependent_instance_id: Option<&str>,
    fresh_selections: &[(String, String)],
) {
    let injected: Vec<String> = fresh_selections
        .iter()
        .map(|(dep, slug)| format!("{dep}@{slug}"))
        .collect();
    match action {
        WorkloadAction::Up {
            instance,
            new,
            use_,
            ..
        }
        | WorkloadAction::Exec {
            instance,
            new,
            use_,
            ..
        } => {
            if let Some(id) = dependent_instance_id {
                *instance = Some(id.to_string());
                *new = false;
            }
            use_.extend(injected);
        }
        _ => {}
    }
}

/// A5 Session 3b (ADR 0032 addendum §Selection ladder rung 3): rewrite the
/// clap-parsed action in place from the parsed inline selector
/// (`name[:config-ref][@instance]`, see
/// [`workestrate::config::parse_workload_selector`]):
///
/// - the workload name becomes the BARE name (the part before `:`);
/// - the inline instance identity lands in the action's `instance` field per
///   the PINNED id precedence: explicit `--instance` > inline `@id` >
///   `:ref`-derived id ([`sanitize_instance_id`]) — then the existing
///   machinery (per-dir derivation > `--new` > strategy default) applies
///   downstream. `--new` + inline ref: `--new` wins the id (no injection),
///   the ref still drives the substitution. An injected id clears `new`
///   (mirrors [`rewrite_action_for_resolved_instance`]; keeps
///   `build_instance_spec`'s mutual-exclusion check untriggered);
/// - VERB SCOPE (pin 5): `down`/`logs` with a `:ref` are a hard error —
///   teardown targets the derived instance id via `--instance <id>`.
///
/// The `:ref`-derived id beats the per-dir derivation by construction:
/// [`resolve_dependent_instance_id`] passes an explicit `instance` through
/// BEFORE consulting the per-dir strategy, so the injected id wins.
///
/// [`sanitize_instance_id`]: workestrate::microsandbox::slots::sanitize_instance_id
/// [`resolve_dependent_instance_id`]: workestrate::commands::lifecycle::resolve_dependent_instance_id
fn rewrite_action_for_inline_selector(
    action: &mut WorkloadAction,
    selector: &workestrate::config::InlineOverride,
) -> Result<()> {
    let workestrate::config::InlineOverride {
        name,
        config_ref,
        instance: at_id,
    } = selector;

    // Verb scope: down/logs reject the ref form (their grammar-only `@id`
    // form already failed closed in parse_workload_selector).
    match action {
        WorkloadAction::Down { name: n, .. } | WorkloadAction::Logs { name: n, .. } => {
            if config_ref.is_some() {
                anyhow::bail!(
                    "inline ref overrides are not meaningful for down/logs; \
                     use --instance <id>"
                );
            }
            *n = name.clone();
            return Ok(());
        }
        _ => {}
    }

    let new_requested = match action {
        WorkloadAction::Up { new, .. } | WorkloadAction::Exec { new, .. } => *new,
        _ => false,
    };
    let derived_id: Option<String> = if let Some(id) = at_id {
        Some(id.clone())
    } else if new_requested {
        None // --new wins the id over the :ref-derived default
    } else {
        config_ref
            .as_deref()
            .map(workestrate::microsandbox::slots::sanitize_instance_id)
            .transpose()?
    };

    match action {
        WorkloadAction::Up {
            name: n,
            instance,
            new,
            ..
        } => {
            *n = Some(name.clone());
            if instance.is_none()
                && let Some(id) = derived_id
            {
                *instance = Some(id);
                *new = false;
            }
        }
        WorkloadAction::Exec {
            name: n,
            instance,
            new,
            ..
        } => {
            *n = name.clone();
            if instance.is_none()
                && let Some(id) = derived_id
            {
                *instance = Some(id);
                *new = false;
            }
        }
        WorkloadAction::Plan {
            name: n, instance, ..
        } => {
            *n = name.clone();
            if instance.is_none()
                && let Some(id) = derived_id
            {
                *instance = Some(id);
            }
        }
        _ => {}
    }
    Ok(())
}

/// A5 review MEDIUM-1: the BARE workload name the current invocation targets
/// (the positional before any `:`), used to NAME-GATE the
/// `WORKESTRATE_WORKLOAD_REF` env pickup
/// ([`workestrate::config::set_pending_inline_override_from_env`]).
///
/// Spawn env inheritance reaches EVERY detached child — including a
/// DEPENDENCY's (`auto_start_dependencies` → `start_service_detached_instance`
/// → `workload up <dep>`) — so only an invocation whose positional names the
/// SAME workload as the override may record it pending. Returns `None` for
/// the bare batch `workload up` (no name), the batch forms of `build`, and
/// every non-workload command: those never set pending.
///
/// Lenient by construction (`split(':')`, no selector validation): a
/// malformed positional fails closed later at the real
/// `parse_workload_selector` call in the verb flow.
fn invocation_workload_name(command: &Commands) -> Option<String> {
    let Commands::Workload { action } = command else {
        return None;
    };
    let positional: Option<&str> = match action {
        WorkloadAction::Up { name, .. } | WorkloadAction::Build { name, .. } => name.as_deref(),
        WorkloadAction::Exec { name, .. }
        | WorkloadAction::Plan { name, .. }
        | WorkloadAction::Down { name, .. }
        | WorkloadAction::Logs { name, .. }
        | WorkloadAction::New { name, .. } => Some(name.as_str()),
    };
    positional.map(|p| p.split(':').next().unwrap_or(p).to_string())
}

/// Translate a clap-parsed verb-first [`WorkloadAction`] into the legacy
/// [`ServiceAction`] shape (pure field mapping — the caller has already
/// kind-checked the route via [`workload_route`]).
fn workload_action_as_service(action: WorkloadAction) -> ServiceAction {
    match action {
        WorkloadAction::Up {
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
            ..
        } => ServiceAction::Up {
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
        },
        WorkloadAction::Plan { instance, use_, .. } => ServiceAction::Plan { instance, use_ },
        WorkloadAction::Down {
            instance,
            all_instances,
            ..
        } => ServiceAction::Down {
            instance,
            all_instances,
        },
        WorkloadAction::Logs { instance, .. } => ServiceAction::Logs { instance },
        WorkloadAction::Exec { .. } => {
            unreachable!("workload_route guarantees exec only routes to agents")
        }
        WorkloadAction::New { .. } => {
            unreachable!("workload new is dispatched before workload translation")
        }
        WorkloadAction::Build { .. } => {
            unreachable!("workload build is dispatched before workload translation")
        }
    }
}

/// The [`AgentAction`] counterpart of [`workload_action_as_service`]. The
/// parity `foreground` flag on `workload exec` is accepted but unused
/// downstream (agents always run in the foreground).
fn workload_action_as_agent(action: WorkloadAction) -> AgentAction {
    match action {
        WorkloadAction::Exec {
            replace,
            instance,
            new,
            port_auto,
            use_,
            no_deps,
            reseed,
            reload_images,
            ..
        } => AgentAction::Exec {
            replace,
            instance,
            new,
            port_auto,
            use_,
            no_deps,
            reseed,
            reload_images,
        },
        WorkloadAction::Plan { instance, use_, .. } => AgentAction::Plan { instance, use_ },
        WorkloadAction::Down {
            instance,
            all_instances,
            ..
        } => AgentAction::Down {
            instance,
            all_instances,
        },
        WorkloadAction::Up { .. } | WorkloadAction::Logs { .. } => {
            unreachable!("workload_route guarantees up/logs only route to services")
        }
        WorkloadAction::New { .. } => {
            unreachable!("workload new is dispatched before workload translation")
        }
        WorkloadAction::Build { .. } => {
            unreachable!("workload build is dispatched before workload translation")
        }
    }
}

/// The bare-up flag-reject table (ADR 0021 addendum 2026-08-01 + spec 21
/// phase E): name-scoped flags are meaningless for the batch form and are
/// hard errors naming the offending flag. `--reload-images` is deliberately
/// NOT in the table (spec 21 §5.2: batch-scoped force, threaded into
/// [`cmd_workload_up_all`]); `--images-ready` IS rejected (the detach token
/// is per-child — batch children carry their own).
fn bare_up_reject_table(action: &WorkloadAction) -> Vec<(bool, &'static str)> {
    let WorkloadAction::Up {
        foreground,
        replace,
        instance,
        new,
        port_auto,
        use_,
        no_deps,
        reseed,
        images_ready,
        ..
    } = action
    else {
        return Vec::new();
    };
    vec![
        (*foreground, "--foreground"),
        (*replace, "--replace"),
        (instance.is_some(), "--instance"),
        (*new, "--new"),
        (*port_auto, "--port-auto"),
        (!use_.is_empty(), "--use"),
        (*no_deps, "--no-deps"),
        (*reseed, "--reseed"),
        (*images_ready, "--images-ready"),
    ]
}

fn main() {
    // Capture the operator's invocation cwd ONCE, before anything else can
    // chdir or re-exec: every "caller's PWD" read downstream (`${CWD}` mount
    // hosts, project-config discovery, content-root fallbacks) resolves
    // against this value via workestrate::config::invoke_cwd(). An inherited
    // value wins, so re-exec'd / detached children keep the ORIGINAL
    // operator cwd (the wrong-CWD `${CWD}` → /work mount bug).
    workestrate::config::ensure_invoke_cwd_env();

    // Pre-scan argv for --json so we can format ANY error (incl. clap parse
    // errors via Cli::parse()) as the JSON envelope when requested. The
    // global --json on Cli does not help here because Cli::parse() calls
    // process::exit on usage errors before we'd see the parsed value.
    let args: Vec<String> = std::env::args().collect();
    let json_mode = json_mode_from_args(&args);

    // ADR 0027 one-cycle deprecation shim: rewrite the legacy
    // `workestrate <name> <verb> ...` shape to verb-first BEFORE clap parses.
    // Best-effort: the config probe failures (or any doubt) leave argv
    // untouched, and built-in subcommand names always win.
    let known: HashSet<String> = Cli::command()
        .get_subcommands()
        .map(|s| s.get_name().to_string())
        .collect();
    let (args, shim_warning) = rewrite_legacy_workload_argv(args, &known, |name| {
        workestrate::config::load_config()
            .map(|cfg| cfg.workloads.contains_key(name))
            .unwrap_or(false)
    });
    if let Some(warning) = shim_warning {
        eprintln!("{warning}");
    }

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
    let result = rt.block_on(async_main(args));
    match result {
        Ok(()) => {}
        Err(e) => {
            emit_error(&e, json_mode);
            std::process::exit(classify_exit_code(&e));
        }
    }
}

#[allow(unsafe_code)]
async fn async_main(args: Vec<String>) -> Result<()> {
    let cli = Cli::parse_from(args);
    if cli.no_project_config {
        // SAFETY: startup-phase write-once CLI override, set before any
        // command flow reads it and before any spawned tasks mutate env; no
        // concurrent mutation of this key.
        unsafe { std::env::set_var("WORKESTRATE_NO_PROJECT_CONFIG", "1") };
    }
    if let Some(ref ctx) = cli.context {
        // SAFETY: startup-phase write-once CLI override, set before any
        // command flow reads it and before any spawned tasks mutate env; no
        // concurrent mutation of this key.
        unsafe { std::env::set_var("WORKESTRATE_CONTEXT", ctx) };
    }
    // --config-ref <branch|sha> populates WORKESTRATE_CONFIG_REF (ADR 0032
    // addendum §Selection ladder, A5 Session 3a): the pinned-consumption
    // layer (config::loading) resolves every Remote/GitFile entry at that
    // ref, and the context-derivation ladder
    // (config::registry::resolve_active_context step b) reads a
    // branch-shaped ref as the context-name candidate. Setting it as an env
    // var (like --context/--home) propagates the override to detached
    // children via spawn env inheritance.
    if let Some(ref config_ref) = cli.config_ref {
        // SAFETY: startup-phase write-once CLI override, set before any
        // command flow reads it and before any spawned tasks mutate env; no
        // concurrent mutation of this key.
        unsafe { std::env::set_var("WORKESTRATE_CONFIG_REF", config_ref) };
    }
    // A5 Session 3b (ADR 0032 addendum §Selection ladder rung 3): a DETACHED
    // CHILD inherits WORKESTRATE_WORKLOAD_REF="name:ref" from its parent
    // (env inheritance through the detach spawn; its argv carries only the
    // bare name + --instance from detach_args). Record it as the PENDING
    // inline override; the verb flow below arms it at the pinned points.
    // No-op when the var is absent (the common case).
    //
    // A5 review MEDIUM-1 NAME GATE: the inheritance reaches EVERY detached
    // child — including a DEPENDENCY's (`workload up litellm` spawned by
    // auto_start_dependencies) — so the pickup fires ONLY when this
    // invocation's workload positional names the SAME workload as the
    // override. The dep's child (name mismatch) records nothing; bare batch
    // `workload up` and non-workload commands (no positional) never record.
    let invocation_name = invocation_workload_name(&cli.command);
    workestrate::config::set_pending_inline_override_from_env(invocation_name.as_deref())?;
    // --home <DIR> populates the WORKESTRATE_HOME precedence step
    // (paths.rs resolve_home_with_kind checks it first), so the flag becomes
    // the highest-precedence override with no path-resolution change.
    if let Some(ref h) = cli.home {
        // SAFETY: startup-phase write-once CLI override, set before any
        // command flow reads it and before any spawned tasks mutate env; no
        // concurrent mutation of this key.
        unsafe { std::env::set_var("WORKESTRATE_HOME", h) };
    }

    match cli.command {
        Commands::Control { action } => match action {
            ControlAction::Serve {
                state_dir,
                instances,
                initialize,
            } => {
                #[cfg(unix)]
                {
                    workestrate::commands::control::cmd_control_serve(
                        &state_dir, &instances, initialize,
                    )
                    .await
                }
                #[cfg(not(unix))]
                {
                    let _ = (state_dir, instances, initialize);
                    anyhow::bail!("host control requires a Unix local endpoint")
                }
            }
        },
        Commands::Check => cmd_check(),
        Commands::Init { url } => cmd_init(url.as_deref()),
        Commands::New { name } => {
            eprintln!(
                "warning: `workestrate new <name>` is deprecated; use `workestrate workload new <name>`"
            );
            cmd_new(&name, "agent")
        }
        Commands::Completions { shell, for_name } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, &for_name, &mut std::io::stdout());
            Ok(())
        }
        Commands::Run { command } => cmd_run(&command),
        Commands::Msb { args } => cmd_msb(&args),
        Commands::ValidateConfig => cmd_validate_config(),
        Commands::SecretsSchema => cmd_secrets_schema(),
        Commands::GenerateEnvExample { output } => cmd_generate_env_example(output.as_deref()),
        Commands::Ps => cmd_ps(cli.json).await,
        Commands::Down {
            all,
            context,
            config_ref,
            everything,
            yes,
        } => {
            let scope = workestrate::microsandbox::runtime::down_scope::resolve_cli_scope(
                all,
                context.as_deref(),
                config_ref.as_deref(),
                everything,
            )?;
            cmd_down_ladder(scope, everything, yes, cli.json).await
        }
        Commands::Clean { yes } => cmd_clean(yes, cli.json),
        Commands::Images { action } => match action {
            // Manual keep-last-N sweep (ADR 0032 §Image tags — RESOLVED
            // user decision 3); cleanup-family aggregate exit rule applies.
            ImagesAction::Gc {} => workestrate::images::gc::cmd_images_gc(cli.json).await,
        },
        Commands::Context { action } => cmd_context(action, cli.json).await,
        Commands::GenerateSchema {
            output,
            output_workload,
            output_registry,
        } => cmd_generate_schema(
            output.as_deref(),
            output_workload.as_deref(),
            output_registry.as_deref(),
        ),
        Commands::Config { action } => match action {
            ConfigAction::List => {
                if cli.json {
                    cmd_config_list_json()
                } else {
                    cmd_config(ConfigAction::List).await
                }
            }
            ConfigAction::New {
                name,
                dest,
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
                    dest.as_deref(),
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
        Commands::Home { action } => cmd_home(action),
        Commands::Schemas { action } => cmd_schemas(action),
        Commands::SecretsTarget { name } => cmd_secrets_target(&name, cli.json).await,
        Commands::Doctor => cmd_doctor(cli.json),
        Commands::Versions => cmd_versions(cli.json),
        Commands::Source { action } => cmd_source(action).await,
        Commands::MigrateHome {
            from,
            dry_run,
            force,
        } => cmd_migrate_home(from.as_deref(), dry_run, cli.json, force),
        Commands::Workload { mut action } => {
            // `workload new` is a scaffold verb, not a lifecycle verb: it has
            // no --use/--no-deps flags and must not reach the name-verb match
            // below.
            if let WorkloadAction::New { name, kind } = &action {
                return cmd_new(name, kind);
            }
            // Spec 21 phase C: `workload build` is NOT kind-routed — it is
            // selector-driven (name / bare / --repo / --all-repos) and
            // dispatches early like `workload new`, before the name-verb
            // match below. It is also deliberately NOT in the legacy shim's
            // VERBS list: build is a new verb, verb-first only.
            if let WorkloadAction::Build {
                name,
                repo,
                all_repos,
                check,
                force,
            } = &action
            {
                return workestrate::images::build_cmd::cmd_workload_build(
                    name.as_deref(),
                    repo.as_deref(),
                    *all_repos,
                    *check,
                    *force,
                    cli.json,
                )
                .await;
            }
            // ADR 0021 addendum 2026-08-01: bare `workload up` (no name) is
            // the batch form — a topo-ordered start of ALL service-kind
            // workloads in the active context. Per-slot/per-dependent flags
            // are not meaningful for batch up and are hard errors naming
            // the offending flag. `--reload-images` is the exception (spec
            // 21 §5.2, USER DECISION D3): batch-scoped, threaded into
            // cmd_workload_up_all.
            if let WorkloadAction::Up { name: None, .. } = &action {
                let WorkloadAction::Up { reload_images, .. } = &action else {
                    unreachable!("matched WorkloadAction::Up above");
                };
                for (present, flag) in bare_up_reject_table(&action) {
                    if present {
                        anyhow::bail!(
                            "{flag} is not meaningful for bare `workestrate workload up` (batch mode); pass a workload name to use it"
                        );
                    }
                }
                return cmd_workload_up_all(cli.json, *reload_images).await;
            }
            // ADR 0027 verb-first dispatch: the workload name is a clap
            // positional, so `--json` and every flag is parsed by clap
            // directly (no raw-args extraction). The `--use` values reach
            // the workload constructor BEFORE resolution runs (ADR 0026(d)).
            let (name, verb, use_values, no_deps, reload_images, images_ready): (
                String,
                &'static str,
                Vec<String>,
                bool,
                bool,
                bool,
            ) = match &action {
                WorkloadAction::Up {
                    name: Some(n),
                    use_,
                    no_deps,
                    reload_images,
                    images_ready,
                    ..
                } => (
                    n.clone(),
                    "up",
                    use_.clone(),
                    *no_deps,
                    *reload_images,
                    *images_ready,
                ),
                WorkloadAction::Up { name: None, .. } => {
                    unreachable!("bare up is handled above")
                }
                WorkloadAction::Exec {
                    name,
                    use_,
                    no_deps,
                    reload_images,
                    ..
                } => (
                    name.clone(),
                    "exec",
                    use_.clone(),
                    *no_deps,
                    *reload_images,
                    false,
                ),
                WorkloadAction::Plan { name, use_, .. } => {
                    (name.clone(), "plan", use_.clone(), false, false, false)
                }
                WorkloadAction::Down { name, .. } => {
                    (name.clone(), "down", Vec::new(), false, false, false)
                }
                WorkloadAction::Logs { name, .. } => {
                    (name.clone(), "logs", Vec::new(), false, false, false)
                }
                WorkloadAction::New { .. } => {
                    unreachable!("workload new is dispatched above")
                }
                WorkloadAction::Build { .. } => {
                    unreachable!("workload build is dispatched above")
                }
            };
            // A5 Session 3b (ADR 0032 addendum §Selection ladder rung 3):
            // parse the inline `name[:config-ref][@instance]` selector
            // (fail-closed grammar — bare `name@id` and empty refs are hard
            // errors; down/logs reject the ref form) and REWRITE the action
            // in place so the rest of the pipeline sees ONE consistent view:
            // the bare workload name plus the resolved inline instance id
            // (--instance > inline @id > :ref-derived; --new beats the
            // :ref-derived default). The ref itself rides the two-phase
            // pending/armed state: deps never follow the override, so
            // `plan` arms IMMEDIATELY (it never starts deps — unknown
            // refs/workloads then fail closed AT PLAN TIME) while up/exec
            // arm AFTER auto_start_dependencies returns, below.
            let selector = workestrate::config::parse_workload_selector(&name)?;
            rewrite_action_for_inline_selector(&mut action, &selector)?;
            let name = selector.name.clone();
            if let Some(ref inline_ref) = selector.config_ref {
                workestrate::config::set_pending_inline_override(&name, inline_ref);
                // SAFETY: startup-phase write-once CLI override, set before
                // the verb flow reads it and before any spawned tasks mutate
                // env; no concurrent mutation of this key.
                unsafe {
                    std::env::set_var(
                        workestrate::config::WORKLOAD_REF_ENV,
                        format!("{name}:{inline_ref}"),
                    )
                };
                if verb == "plan" {
                    workestrate::config::arm_inline_override();
                }
            }
            let mut overrides =
                workestrate::microsandbox::discovery::parse_use_overrides(&use_values)?;
            // ADR 0030 P2.1: resolve the DEPENDENT's OWN parallel instance id
            // BEFORE dependency auto-start — the planner composes scoped dep
            // instance ids (`<dep>@<dependent>-<id>`) from it and fresh dep
            // selections are injected as `--use` overrides keyed to it.
            // Explicit --instance passes through verbatim; --new / the
            // parallel-strategy default allocate the slug HERE (once). The
            // action is REWRITTEN below so dispatch_service/dispatch_agent
            // reuse the SAME id (their no_instance allocation guard skips).
            // `plan` gets an EXPLICIT-passthrough id ONLY (the scoped dep
            // preview of the running dependent) — it is read-only and must
            // never allocate a slug, so resolve_dependent_instance_id is not
            // called for it (see plan_preview_instance_id). down/logs carry
            // no dependent id.
            let dependent_instance_id: Option<String> = match verb {
                "up" | "exec" => {
                    let (instance, new, replace) = match &action {
                        WorkloadAction::Up {
                            instance,
                            new,
                            replace,
                            ..
                        }
                        | WorkloadAction::Exec {
                            instance,
                            new,
                            replace,
                            ..
                        } => (instance.as_deref(), *new, *replace),
                        _ => unreachable!("verb up/exec only destructures from Up/Exec above"),
                    };
                    resolve_dependent_instance_id(&name, instance, new, replace)?
                }
                "plan" => {
                    // The strategy lookup is the only I/O: per-dir previews
                    // the DERIVED id (deterministic, no allocation); every
                    // other strategy stays on the singleton view.
                    let strategy = workestrate::config::load_config()?
                        .workloads
                        .get(&name)
                        .map(|w| w.instance.strategy)
                        .unwrap_or(workestrate::config::InstanceStrategy::Singleton);
                    plan_preview_instance_id(&action, strategy)?
                }
                _ => None,
            };
            // Spec 21 §2/§2.4 (phase E): the ensure-images pre-flight runs
            // on the NAMED workload BEFORE dependency auto-start — fail fast
            // on the workload the operator actually asked for before
            // spending minutes starting its dep closure. The detached child
            // carries the --images-ready token and skips this entirely
            // (§2.2); a token-free foreground up/exec IS the parent and
            // ensures. `--reload-images` maps to force (§5.2).
            //
            // A2/A5 SEAM (ADR 0032 §Image tags — DECIDED 2026-08-24): when a
            // per-workload inline override is PENDING (`workload up
            // prime:feat-x`), this ordering is REORDERED — dep auto-start
            // (unarmed, home-scoped deps) runs first, then
            // arm_inline_override(), THEN the ensure — so the ensure sees
            // the substituted config and tags/pointers under the OVERRIDE's
            // tag context (image_tag_context: `workestrate-prime:feat-x.<sha>`,
            // moving only the (name, "feat-x") pointer). With NO pending
            // override today's order stands (fail-fast preserved). The
            // decision is the pure images::ensure::ensure_after_arming; the
            // reordered ensure call site is below the arming point.
            let pending_override = workestrate::config::pending_inline_override().is_some();
            let ensure_after_arming = workestrate::images::ensure::ensure_after_arming(
                verb,
                images_ready,
                pending_override,
            );
            if workestrate::images::ensure::ensure_should_run(verb, images_ready)
                && !ensure_after_arming
            {
                workestrate::images::ensure::ensure_images_for_workload(&name, reload_images)
                    .await?;
            }
            // Construction-order rule (ADR 0026 addendum): declared deps
            // start BEFORE the dependent's ConfigWorkload is constructed —
            // construction runs resolve_depends_on, which refuses a
            // required-not-running dep, so the dep must already be up. Dep
            // auto-start inherits the ensure pre-flight per dependency (spec
            // 21 §2.1) inside auto_start_dependencies.
            //
            // ADR 0030 P2.1: the returned FRESH `(dep, slug)` selections are
            // INJECTED as `--use <dep>@<slug>` overrides into BOTH the
            // construction overrides below and the action's forwarded `use_`
            // list — the detached child's own planner then marks the fresh
            // dep Satisfied (no second allocation). Scoped deps need no
            // injection: the child re-derives `<dependent>-<id>` from its
            // forwarded `--instance`.
            let fresh_selections = auto_start_dependencies(
                &name,
                verb,
                no_deps,
                &overrides,
                dependent_instance_id.as_deref(),
            )
            .await?;
            // A5 Session 3b arming point (up/exec): deps NEVER follow the
            // override — auto_start_dependencies above saw the home-scoped
            // config — but the dependent's ConfigWorkload constructed below
            // MUST see the substituted declaration. No-op without a pending
            // override. (The detached child reaches this SAME point with the
            // env-derived pending: its own auto-start above likewise ran
            // un-armed.)
            //
            // A5 review MEDIUM-1 VERB SCOPE: only up/exec arm here. down/logs
            // (which also flow through this match arm) NEVER arm — a leftover
            // exported WORKESTRATE_WORKLOAD_REF must not arm the substitution
            // on a teardown/log config load.
            if workestrate::config::verb_arms_after_dep_autostart(verb) {
                workestrate::config::arm_inline_override();
            }
            // A2/A5 SEAM (the reordered ensure call site — see the comment
            // at the pre-flight above): a pending inline override on a
            // parent up/exec ensures HERE, after arming, so the ensure's
            // load_config sees the substituted declaration and its
            // tags/pointers land under the override's tag context (ADR 0032
            // §Image tags). No-op in every other shape.
            if ensure_after_arming {
                workestrate::images::ensure::ensure_images_for_workload(&name, reload_images)
                    .await?;
            }
            overrides.extend(fresh_selections.iter().cloned());
            rewrite_action_for_resolved_instance(
                &mut action,
                dependent_instance_id.as_deref(),
                &fresh_selections,
            );
            let workload = ConfigWorkload::new_with_use_overrides_and_instance(
                &name,
                &overrides,
                dependent_instance_id.as_deref(),
            )?;
            // Kind-check at dispatch (ADR 0027): wrong-kind usage names the
            // correct invocation; plan/down are universal.
            match workload_route(workload.kind(), verb, &name)? {
                WorkloadRoute::Service => {
                    dispatch_service(
                        &workload,
                        workload_action_as_service(action),
                        cli.show_source,
                        cli.json,
                    )
                    .await
                }
                WorkloadRoute::Agent => {
                    dispatch_agent(
                        &workload,
                        workload_action_as_agent(action),
                        cli.show_source,
                        cli.json,
                    )
                    .await
                }
            }
        }
        Commands::Workloads => cmd_workloads(cli.json),
        Commands::Instances { workload } => cmd_instances(workload.as_deref(), cli.json).await,
        Commands::Policy { action } => {
            workestrate::commands::policy::cmd_policy(action, cli.json).await
        }
    }
}

#[cfg(test)]
#[allow(unsafe_code)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use workestrate::config::test_support::TestConfigGuard;

    #[test]
    fn control_serve_requires_explicit_state_and_instances() {
        assert!(Cli::try_parse_from(["workestrate", "control", "serve"]).is_err());
        assert!(
            Cli::try_parse_from([
                "workestrate",
                "control",
                "serve",
                "--state-dir",
                "/private/control"
            ])
            .is_err()
        );
        let parsed = Cli::try_parse_from([
            "workestrate",
            "control",
            "serve",
            "--state-dir",
            "/private/control",
            "--instance",
            "context-worker@one",
            "--instance",
            "context-worker@two",
            "--initialize",
        ]);
        let Ok(Cli {
            command:
                Commands::Control {
                    action:
                        ControlAction::Serve {
                            state_dir,
                            instances,
                            initialize,
                        },
                },
            ..
        }) = parsed
        else {
            panic!("explicit control serve did not parse");
        };
        assert_eq!(state_dir, PathBuf::from("/private/control"));
        assert_eq!(instances, ["context-worker@one", "context-worker@two"]);
        assert!(initialize);
        let reopened = Cli::try_parse_from([
            "workestrate",
            "control",
            "serve",
            "--state-dir",
            "/private/control",
            "--instance",
            "worker",
        ]);
        assert!(matches!(
            reopened,
            Ok(Cli {
                command: Commands::Control {
                    action: ControlAction::Serve {
                        initialize: false,
                        ..
                    }
                },
                ..
            })
        ));
    }

    #[test]
    fn cli_exposes_expected_subcommands() {
        let cmd = Cli::command();
        let names: Vec<_> = cmd.get_subcommands().map(|s| s.get_name()).collect();
        for expected in [
            "control",
            "check",
            "init",
            "new",
            "completions",
            "run",
            "msb",
            "validate-config",
            "secrets-schema",
            "generate-env-example",
            "config",
            "home",
            "schemas",
            "secrets-target",
            "doctor",
            "versions",
            "clean",
            "context",
            "source",
            "migrate-home",
            "workload",
            "workloads",
        ] {
            assert!(names.contains(&expected), "missing subcommand: {expected}");
        }
        // Cleanup phase 4: NO config-specific workload names are baked into
        // the CLI as typed subcommands — the generic `workload <verb> <name>`
        // group is the only lifecycle path.
        for removed in ["litellm", "pi", "odysseus", "opencode", "tempest"] {
            assert!(
                !names.contains(&removed),
                "typed subcommand must not exist: {removed}"
            );
        }
    }

    // ---- ADR 0030 P2.1: rewrite_action_for_resolved_instance ----

    fn up_action() -> WorkloadAction {
        WorkloadAction::Up {
            name: Some("prime".to_string()),
            foreground: false,
            replace: false,
            instance: None,
            new: true,
            port_auto: false,
            use_: vec!["redis@blue-2".to_string()],
            no_deps: false,
            reseed: false,
            reload_images: false,
            images_ready: false,
        }
    }

    /// ADR 0030 P2.1 + V-addendum §V1: `plan --instance <id>` passes the id
    /// through for the scoped dep preview; plan WITHOUT --instance on a
    /// non-per-dir workload is None (the singleton view; no slug
    /// allocation); a per-dir workload previews the DERIVED cwd-keyed id
    /// (deterministic, no allocation); non-Plan actions yield None.
    #[test]
    fn plan_preview_instance_id_passthrough_and_per_dir_derivation() {
        use workestrate::config::InstanceStrategy;
        let plan_with = WorkloadAction::Plan {
            name: "prime".to_string(),
            instance: Some("x7".to_string()),
            use_: Vec::new(),
        };
        assert_eq!(
            plan_preview_instance_id(&plan_with, InstanceStrategy::Singleton).unwrap(),
            Some("x7".to_string()),
            "explicit --instance must pass through verbatim"
        );
        // Explicit id passes through even for a per-dir workload.
        assert_eq!(
            plan_preview_instance_id(&plan_with, InstanceStrategy::PerDir).unwrap(),
            Some("x7".to_string())
        );
        let plan_without = WorkloadAction::Plan {
            name: "prime".to_string(),
            instance: None,
            use_: Vec::new(),
        };
        assert_eq!(
            plan_preview_instance_id(&plan_without, InstanceStrategy::Singleton).unwrap(),
            None,
            "no --instance → None (singleton view; no slug allocation)"
        );
        assert_eq!(
            plan_preview_instance_id(&plan_without, InstanceStrategy::Parallel).unwrap(),
            None,
            "parallel strategy still previews nothing (never allocates)"
        );
        assert_eq!(
            plan_preview_instance_id(&up_action(), InstanceStrategy::PerDir).unwrap(),
            None,
            "non-Plan actions are not handled by the plan seam"
        );
    }

    /// The per-dir plan preview derives the cwd-keyed id from the CANONICAL
    /// invocation cwd — deterministic, with no registry allocation.
    #[test]
    fn plan_preview_instance_id_per_dir_derives_cwd_keyed_id() {
        use workestrate::config::test_support::{ENV_TEST_LOCK, EnvGuard, uniq_dir};
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _env = EnvGuard::capture(&[workestrate::config::INVOKE_CWD_ENV]);
        let invoke = uniq_dir("plan-preview-perdir");
        std::fs::create_dir_all(&invoke).unwrap();
        let canonical = std::fs::canonicalize(&invoke).unwrap();
        // SAFETY: serialized by ENV_TEST_LOCK (held directly); var restored
        // by EnvGuard on drop.
        unsafe { std::env::set_var(workestrate::config::INVOKE_CWD_ENV, &canonical) };
        let plan_without = WorkloadAction::Plan {
            name: "prime".to_string(),
            instance: None,
            use_: Vec::new(),
        };
        let id =
            plan_preview_instance_id(&plan_without, workestrate::config::InstanceStrategy::PerDir)
                .unwrap()
                .expect("per-dir plan previews the derived id");
        assert_eq!(
            id,
            workestrate::microsandbox::slots::per_dir_instance_id(&canonical.to_string_lossy())
        );
        let _ = std::fs::remove_dir_all(&invoke);
    }

    /// The resolved dependent id becomes an explicit `--instance` (with
    /// `--new` cleared) so dispatch_service reuses the SAME id and the
    /// detached child inherits it; fresh dep selections join the forwarded
    /// --use list.
    #[test]
    fn rewrite_up_action_injects_instance_and_fresh_use() {
        let mut action = up_action();
        let fresh = vec![("litellm".to_string(), "ab2z".to_string())];
        rewrite_action_for_resolved_instance(&mut action, Some("xy7"), &fresh);
        let WorkloadAction::Up {
            instance,
            new,
            use_,
            ..
        } = &action
        else {
            panic!("expected Up");
        };
        assert_eq!(instance.as_deref(), Some("xy7"));
        assert!(!new, "--new must be cleared once the id is materialized");
        assert_eq!(
            use_,
            &vec!["redis@blue-2".to_string(), "litellm@ab2z".to_string()],
            "fresh selections must be appended AFTER user-supplied --use"
        );
        // The translated ServiceAction carries the same id/use list — this
        // is what dispatch_service and detach_args consume.
        match workload_action_as_service(action) {
            ServiceAction::Up {
                instance,
                new,
                use_,
                ..
            } => {
                assert_eq!(instance.as_deref(), Some("xy7"));
                assert!(!new);
                assert!(use_.contains(&"litellm@ab2z".to_string()));
            }
            _ => panic!("expected ServiceAction::Up"),
        }
    }

    /// No resolved id (singleton dependent): instance/new are untouched but
    /// fresh selections are STILL injected (a singleton dependent can have
    /// fresh-mode deps).
    #[test]
    fn rewrite_without_dependent_id_injects_only_fresh_use() {
        let mut action = up_action();
        let fresh = vec![("litellm".to_string(), "ab2z".to_string())];
        rewrite_action_for_resolved_instance(&mut action, None, &fresh);
        let WorkloadAction::Up {
            instance,
            new,
            use_,
            ..
        } = &action
        else {
            panic!("expected Up");
        };
        assert_eq!(instance, &None);
        assert!(*new, "--new stays when no id was resolved");
        assert!(use_.contains(&"litellm@ab2z".to_string()));
    }

    /// Exec gets the same rewrite; plan/down/logs are untouched.
    #[test]
    fn rewrite_exec_action_and_other_verbs_untouched() {
        let mut action = WorkloadAction::Exec {
            name: "prime".to_string(),
            foreground: false,
            replace: false,
            instance: None,
            new: true,
            port_auto: false,
            use_: Vec::new(),
            no_deps: false,
            reseed: false,
            reload_images: false,
        };
        rewrite_action_for_resolved_instance(&mut action, Some("k9"), &[]);
        let WorkloadAction::Exec { instance, new, .. } = &action else {
            panic!("expected Exec");
        };
        assert_eq!(instance.as_deref(), Some("k9"));
        assert!(!new);

        let mut plan = WorkloadAction::Plan {
            name: "prime".to_string(),
            instance: None,
            use_: Vec::new(),
        };
        rewrite_action_for_resolved_instance(&mut plan, Some("k9"), &[]);
        let WorkloadAction::Plan { instance, .. } = &plan else {
            panic!("expected Plan");
        };
        assert_eq!(instance, &None, "plan is not rewritten");
    }

    // ---- A5 Session 3b: rewrite_action_for_inline_selector ----

    fn inline_selector(positional: &str) -> workestrate::config::InlineOverride {
        workestrate::config::parse_workload_selector(positional).expect("test selectors must parse")
    }

    fn up_action_no_new() -> WorkloadAction {
        WorkloadAction::Up {
            name: Some("prime".to_string()),
            foreground: false,
            replace: false,
            instance: None,
            new: false,
            port_auto: false,
            use_: Vec::new(),
            no_deps: false,
            reseed: false,
            reload_images: false,
            images_ready: false,
        }
    }

    /// `up prime:feat-x` → bare name + the :ref-derived parallel instance id
    /// `feat-x` (sanitize_instance_id; the instance COEXISTS with the
    /// home-ref instance via the standard parallel machinery).
    #[test]
    fn inline_selector_up_ref_derives_instance_id() {
        let mut action = up_action_no_new();
        rewrite_action_for_inline_selector(&mut action, &inline_selector("prime:feat-x")).unwrap();
        let WorkloadAction::Up {
            name,
            instance,
            new,
            ..
        } = &action
        else {
            panic!("expected Up");
        };
        assert_eq!(name.as_deref(), Some("prime"), "the name is de-ref'd");
        assert_eq!(instance.as_deref(), Some("feat-x"));
        assert!(!new);
    }

    /// `up prime:feat-x@canary` → the inline @id wins over the :ref-derived
    /// id.
    #[test]
    fn inline_selector_up_ref_at_id_wins_over_derived() {
        let mut action = up_action_no_new();
        rewrite_action_for_inline_selector(&mut action, &inline_selector("prime:feat-x@canary"))
            .unwrap();
        let WorkloadAction::Up { instance, .. } = &action else {
            panic!("expected Up");
        };
        assert_eq!(instance.as_deref(), Some("canary"));
    }

    /// Id precedence: explicit --instance beats BOTH inline forms.
    #[test]
    fn inline_selector_explicit_instance_flag_wins() {
        let mut action = WorkloadAction::Up {
            name: Some("prime".to_string()),
            foreground: false,
            replace: false,
            instance: Some("custom".to_string()),
            new: false,
            port_auto: false,
            use_: Vec::new(),
            no_deps: false,
            reseed: false,
            reload_images: false,
            images_ready: false,
        };
        rewrite_action_for_inline_selector(&mut action, &inline_selector("prime:feat-x@canary"))
            .unwrap();
        let WorkloadAction::Up { instance, .. } = &action else {
            panic!("expected Up");
        };
        assert_eq!(instance.as_deref(), Some("custom"), "--instance beats both");
    }

    /// `--new` + inline ref: --new wins the id (no injection, `new` stays
    /// set); the ref still drives the substitution (the pending/armed state,
    /// not this rewrite).
    #[test]
    fn inline_selector_new_wins_the_id_over_the_derived_default() {
        let mut action = up_action(); // new: true
        rewrite_action_for_inline_selector(&mut action, &inline_selector("prime:feat-x")).unwrap();
        let WorkloadAction::Up {
            name,
            instance,
            new,
            ..
        } = &action
        else {
            panic!("expected Up");
        };
        assert_eq!(name.as_deref(), Some("prime"), "the name is still de-ref'd");
        assert_eq!(instance, &None, "--new wins: no derived id is injected");
        assert!(*new);
    }

    /// Exec gets the same rewrite; plan previews the derived id.
    #[test]
    fn inline_selector_exec_and_plan_rewrite() {
        let mut exec = WorkloadAction::Exec {
            name: "prime".to_string(),
            foreground: false,
            replace: false,
            instance: None,
            new: false,
            port_auto: false,
            use_: Vec::new(),
            no_deps: false,
            reseed: false,
            reload_images: false,
        };
        rewrite_action_for_inline_selector(&mut exec, &inline_selector("prime:feat-x")).unwrap();
        let WorkloadAction::Exec {
            name,
            instance,
            new,
            ..
        } = &exec
        else {
            panic!("expected Exec");
        };
        assert_eq!(name, "prime");
        assert_eq!(instance.as_deref(), Some("feat-x"));
        assert!(!new);

        let mut plan = WorkloadAction::Plan {
            name: "prime".to_string(),
            instance: None,
            use_: Vec::new(),
        };
        rewrite_action_for_inline_selector(&mut plan, &inline_selector("prime:feat-x@canary"))
            .unwrap();
        let WorkloadAction::Plan { name, instance, .. } = &plan else {
            panic!("expected Plan");
        };
        assert_eq!(name, "prime");
        assert_eq!(instance.as_deref(), Some("canary"), "plan previews the id");
    }

    /// Verb scope (pin 5): down/logs with a `:ref` are a hard error naming
    /// --instance; the name is still de-ref'd when no ref is present.
    #[test]
    fn inline_selector_down_and_logs_reject_the_ref_form() {
        for mut action in [
            WorkloadAction::Down {
                name: "prime".to_string(),
                instance: None,
                all_instances: false,
            },
            WorkloadAction::Logs {
                name: "prime".to_string(),
                instance: None,
            },
        ] {
            let err =
                rewrite_action_for_inline_selector(&mut action, &inline_selector("prime:feat-x"))
                    .unwrap_err()
                    .to_string();
            assert!(
                err.contains(
                    "inline ref overrides are not meaningful for down/logs; use --instance <id>"
                ),
                "down/logs rejection message: {err}"
            );
        }
        // No ref → the name rewrites cleanly (defensive; parse already
        // guarantees no '@' survives on this path).
        let mut down = WorkloadAction::Down {
            name: "prime".to_string(),
            instance: None,
            all_instances: false,
        };
        rewrite_action_for_inline_selector(&mut down, &inline_selector("prime")).unwrap();
        let WorkloadAction::Down { name, .. } = &down else {
            panic!("expected Down");
        };
        assert_eq!(name, "prime");
    }

    /// A bare name leaves up/exec/plan actions untouched.
    #[test]
    fn inline_selector_bare_name_is_a_no_op() {
        let mut action = up_action_no_new();
        rewrite_action_for_inline_selector(&mut action, &inline_selector("prime")).unwrap();
        let WorkloadAction::Up {
            name,
            instance,
            new,
            ..
        } = &action
        else {
            panic!("expected Up");
        };
        assert_eq!(name.as_deref(), Some("prime"));
        assert_eq!(instance, &None);
        assert!(!new);
    }

    #[test]
    fn cli_root_has_global_home_flag() {
        let cmd = Cli::command();
        let home = cmd
            .get_arguments()
            .find(|a| a.get_long() == Some("home"))
            .expect("root command must have a --home argument");
        assert!(
            home.is_global_set(),
            "--home must be a global argument (valid on every subcommand)"
        );
    }

    /// A5 Session 3a (ADR 0032 addendum §Selection ladder): `--config-ref`
    /// is a GLOBAL flag — accepted at the root and on every subcommand.
    #[test]
    fn cli_root_has_global_config_ref_flag() {
        let mut cmd = Cli::command();
        let arg = cmd
            .get_arguments()
            .find(|a| a.get_long() == Some("config-ref"))
            .expect("root command must have a --config-ref argument");
        assert!(
            arg.is_global_set(),
            "--config-ref must be a global argument (valid on every subcommand)"
        );
        // Parses at the root…
        let cli = Cli::try_parse_from(["workestrate", "--config-ref", "feat-x", "ps"])
            .expect("--config-ref must parse before the subcommand");
        assert_eq!(cli.config_ref.as_deref(), Some("feat-x"));
        // …and after a subcommand (global propagation), mirroring the
        // doctor/migrate-home --json test's build()-propagation check.
        cmd.build();
        for name in ["ps", "workload", "config", "home"] {
            let sub = cmd
                .find_subcommand(name)
                .unwrap_or_else(|| panic!("missing subcommand: {name}"));
            let propagated = sub
                .get_arguments()
                .find(|a| a.get_long() == Some("config-ref"))
                .unwrap_or_else(|| panic!("{name} must expose --config-ref"));
            assert!(
                propagated.is_global_set(),
                "{name} --config-ref must be the propagated GLOBAL flag"
            );
        }
        let cli = Cli::try_parse_from(["workestrate", "ps", "--config-ref", "0123abc"])
            .expect("--config-ref must parse after the subcommand");
        assert_eq!(cli.config_ref.as_deref(), Some("0123abc"));
    }

    /// ADR 0032 addendum (2026-08-24 same-day amendment): the `--from <ref>`
    /// form of the per-workload override is DROPPED — the inline
    /// `name:ref[@instance]` grammar is the ONLY shape. NO command anywhere
    /// in the tree may carry a `--from` flag, with ONE grandfathered
    /// exception: `migrate-home --from <xdg|bundle>` (the legacy layout
    /// selector whose collision history is exactly why no NEW --from may be
    /// added; `config new --from-reference` is a different flag and is
    /// UNAFFECTED; the dropped `home init --from` per ADR 0025 stays
    /// dropped, per the home_init_has_no_path_flag guard).
    #[test]
    fn no_from_flag_anywhere_in_the_command_tree() {
        fn assert_no_from(cmd: &clap::Command, path: &str) {
            for arg in cmd.get_arguments() {
                if path == "workestrate migrate-home" && arg.get_long() == Some("from") {
                    continue; // grandfathered legacy layout selector
                }
                assert_ne!(
                    arg.get_long(),
                    Some("from"),
                    "{path} must NOT have a --from flag (ADR 0032 addendum)"
                );
                let aliases: Vec<&str> = arg.get_all_aliases().unwrap_or_default().to_vec();
                assert!(
                    !aliases.contains(&"from"),
                    "{path} --{} must NOT alias `from`; got: {aliases:?}",
                    arg.get_id()
                );
            }
            for sub in cmd.get_subcommands() {
                assert_no_from(sub, &format!("{path} {}", sub.get_name()));
            }
        }
        let mut cmd = Cli::command();
        cmd.build();
        assert_no_from(&cmd, "workestrate");
    }

    /// W1: `doctor` and `migrate-home` must NOT shadow the global --json with
    /// a local flag — the --json they see is the propagated global, so both
    /// `workestrate --json doctor` and `workestrate doctor --json` work.
    #[test]
    fn doctor_and_migrate_home_use_global_json_flag() {
        // build() propagates global args into subcommands, mirroring what
        // happens at parse time.
        let mut cmd = Cli::command();
        cmd.build();
        for name in ["doctor", "migrate-home"] {
            let sub = cmd
                .find_subcommand(name)
                .unwrap_or_else(|| panic!("missing subcommand: {name}"));
            let json = sub
                .get_arguments()
                .find(|a| a.get_long() == Some("json"))
                .unwrap_or_else(|| panic!("{name} must expose a --json argument"));
            assert!(
                json.is_global_set(),
                "{name} --json must be the propagated GLOBAL flag, not a local shadow"
            );
        }
    }

    /// W1: `generate-schema` uses canonical `-o/--output`; `--out` remains a
    /// hidden back-compat alias.
    #[test]
    fn generate_schema_output_flag_is_canonical() {
        let cmd = Cli::command();
        let sub = cmd
            .find_subcommand("generate-schema")
            .expect("generate-schema must exist");
        let output = sub
            .get_arguments()
            .find(|a| a.get_long() == Some("output"))
            .expect("generate-schema must have a canonical --output argument");
        let aliases: Vec<&str> = output.get_all_aliases().unwrap_or_default().to_vec();
        assert!(
            aliases.contains(&"out"),
            "--output must carry a hidden back-compat alias `out`; got: {aliases:?}"
        );
    }

    #[test]
    fn home_init_has_no_path_flag() {
        let cmd = Cli::command();
        let init = cmd
            .find_subcommand("home")
            .and_then(|s| s.find_subcommand("init"))
            .expect("home init must exist");
        let long_names: Vec<String> = init
            .get_arguments()
            .filter_map(|a| a.get_long().map(|s| s.to_string()))
            .collect();
        assert!(
            long_names.contains(&"config".to_string()),
            "home init missing --config flag; got: {long_names:?}"
        );
        assert!(
            long_names.contains(&"name".to_string()),
            "home init missing --name flag; got: {long_names:?}"
        );
        assert!(
            !long_names.contains(&"path".to_string()),
            "home init must NOT have a --path flag (spec 10 §2: operates on the \
             resolved home only); got: {long_names:?}"
        );
        assert!(
            !long_names.contains(&"from".to_string()),
            "home init must NOT have a --from flag (ADR 0025: provisioning moved \
             to 'home clone'); got: {long_names:?}"
        );
        let positionals: Vec<String> = init
            .get_positionals()
            .map(|a| a.get_id().to_string())
            .collect();
        assert!(
            positionals.is_empty(),
            "home init must have NO positionals (ADR 0025: dest moved to \
             'home clone'); got: {positionals:?}"
        );
    }

    #[test]
    fn home_clone_cli_shape() {
        let cmd = Cli::command();
        let clone = cmd
            .find_subcommand("home")
            .and_then(|s| s.find_subcommand("clone"))
            .expect("home clone must exist (ADR 0025)");
        let positionals: Vec<(String, bool)> = clone
            .get_positionals()
            .map(|a| (a.get_id().to_string(), a.is_required_set()))
            .collect();
        assert_eq!(
            positionals,
            vec![("src".to_string(), true), ("dest".to_string(), false)],
            "home clone must have a REQUIRED positional <src> and an OPTIONAL \
             positional <dest>; got: {positionals:?}"
        );
        let long_names: Vec<String> = clone
            .get_arguments()
            .filter_map(|a| a.get_long().map(|s| s.to_string()))
            .collect();
        for flag in ["config", "name", "from"] {
            assert!(
                !long_names.contains(&flag.to_string()),
                "home clone must NOT have a --{flag} flag; got: {long_names:?}"
            );
        }
    }

    #[test]
    fn config_new_cli_shape() {
        let cmd = Cli::command();
        let new = cmd
            .find_subcommand("config")
            .and_then(|s| s.find_subcommand("new"))
            .expect("config new must exist");
        let positionals: Vec<(String, bool)> = new
            .get_positionals()
            .map(|a| (a.get_id().to_string(), a.is_required_set()))
            .collect();
        assert_eq!(
            positionals,
            vec![("name".to_string(), true), ("dest".to_string(), false)],
            "config new must have a REQUIRED positional <name> and an \
             OPTIONAL positional <dest>; got: {positionals:?}"
        );
        let long_names: Vec<String> = new
            .get_arguments()
            .filter_map(|a| a.get_long().map(|s| s.to_string()))
            .collect();
        assert!(
            !long_names.contains(&"path".to_string()),
            "config new must NOT have a --path flag; got: {long_names:?}"
        );
        for flag in ["age-recipient", "empty", "from-reference"] {
            assert!(
                long_names.contains(&flag.to_string()),
                "config new must keep the --{flag} flag; got: {long_names:?}"
            );
        }
    }

    /// Build an InstanceSpec for `example-litellm` (no context) with selected
    /// flags.
    fn spec_for_detach(
        instance: &str,
        replace: bool,
    ) -> workestrate::microsandbox::runtime::InstanceSpec {
        spec_for_detach_full(instance, replace, false)
    }

    /// spec_for_detach with an explicit --port-auto flag (ADR 0026(c)/C3).
    fn spec_for_detach_full(
        instance: &str,
        replace: bool,
        port_auto: bool,
    ) -> workestrate::microsandbox::runtime::InstanceSpec {
        use workestrate::microsandbox::runtime::InstanceSpec;
        InstanceSpec {
            instance: instance.to_string(),
            workload: "example-litellm".to_string(),
            context: None,
            replace,
            port_auto,
            use_overrides: Vec::new(),
            no_deps: false,
            reseed: false,
            images_ready: false,
            source_dir: None,
        }
    }

    #[test]
    fn detach_args_include_foreground() -> Result<()> {
        let _guard = TestConfigGuard::new();
        use workestrate::microsandbox::workload::Workload;
        let example_litellm = ConfigWorkload::new("example-litellm")?;

        // Singleton, no flags: verb-first `workload up <name> --foreground`
        // (ADR 0027) + the unconditional `--images-ready` token (spec 21
        // §2.2 — the parent ensured before spawning the child).
        let args = example_litellm.detach_args(&spec_for_detach("example-litellm", false));
        assert_eq!(
            args,
            vec![
                "workload",
                "up",
                "example-litellm",
                "--foreground",
                "--images-ready"
            ]
        );

        // Parallel instance: forward `--instance <id>` with the BARE id, never
        // `slot@id`. `--new` must NOT appear (already materialized by parent).
        let args = example_litellm.detach_args(&spec_for_detach("example-litellm@canary", false));
        assert_eq!(
            args,
            vec![
                "workload",
                "up",
                "example-litellm",
                "--foreground",
                "--images-ready",
                "--instance",
                "canary"
            ]
        );
        assert!(
            !args.contains(&"--new".to_string()),
            "--new must not be forwarded (already materialized into spec.instance)"
        );

        // `--replace` is forwarded when requested.
        let args = example_litellm.detach_args(&spec_for_detach("example-litellm", true));
        assert_eq!(
            args,
            vec![
                "workload",
                "up",
                "example-litellm",
                "--foreground",
                "--images-ready",
                "--replace"
            ]
        );

        // `--replace` and `--instance` compose.
        let args = example_litellm.detach_args(&spec_for_detach("example-litellm@ab2z", true));
        assert_eq!(
            args,
            vec![
                "workload",
                "up",
                "example-litellm",
                "--foreground",
                "--images-ready",
                "--replace",
                "--instance",
                "ab2z",
            ]
        );

        // A workload other than example-litellm uses its own name as the <name>
        // positional (the argv prefix stays `workload up`).
        let pi = ConfigWorkload::new("pi")?;
        let args = pi.detach_args(&spec_for_detach("pi@xy7", false));
        assert_eq!(args.first(), Some(&"workload".to_string()));
        assert_eq!(args.get(1), Some(&"up".to_string()));
        assert_eq!(args.get(2), Some(&"pi".to_string()));
        assert!(args.contains(&"--foreground".to_string()));
        assert!(args.contains(&"--instance".to_string()));
        assert_eq!(args[args.len() - 1], "xy7");

        // `--port-auto` (ADR 0026(c)) is forwarded alongside --instance; it
        // must NOT appear when unset.
        let args = example_litellm.detach_args(&spec_for_detach_full(
            "example-litellm@canary",
            false,
            true,
        ));
        assert!(
            args.contains(&"--port-auto".to_string()),
            "--port-auto must be forwarded to the detached child: {args:?}"
        );
        assert_eq!(
            args,
            vec![
                "workload",
                "up",
                "example-litellm",
                "--foreground",
                "--images-ready",
                "--port-auto",
                "--instance",
                "canary",
            ]
        );
        let args = example_litellm.detach_args(&spec_for_detach("example-litellm", false));
        assert!(
            !args.contains(&"--port-auto".to_string()),
            "--port-auto must not appear when unset: {args:?}"
        );
        Ok(())
    }

    /// ADR 0026(d)/C3-W2: `--use <dep>@<instance>` overrides ride the spec so
    /// the DETACHED child re-enters `workload up <name> --foreground` with the
    /// same instance-selection the parent resolved (the child re-parses via
    /// parse_service_action, which collects every --use).
    #[test]
    fn detach_args_forwards_use_overrides() -> Result<()> {
        let _guard = TestConfigGuard::new();
        use workestrate::microsandbox::workload::Workload;
        let pi = ConfigWorkload::new("pi")?;

        let mut spec = spec_for_detach("pi", false);
        spec.use_overrides = vec![
            ("litellm".to_string(), "canary".to_string()),
            ("redis".to_string(), "blue".to_string()),
        ];
        let args = pi.detach_args(&spec);
        assert_eq!(
            args,
            vec![
                "workload",
                "up",
                "pi",
                "--foreground",
                "--images-ready",
                "--use",
                "litellm@canary",
                "--use",
                "redis@blue",
            ]
        );

        // No overrides → no --use tokens.
        let args = pi.detach_args(&spec_for_detach("pi", false));
        assert!(
            !args.contains(&"--use".to_string()),
            "--use must not appear without overrides: {args:?}"
        );

        // The forwarded args round-trip through the raw-args parser the
        // detached-child path uses: the flags sit after
        // ["workload", "up", <name>] (i.e. &args[3..]).
        let parsed = workestrate::commands::lifecycle::parse_service_action("up", &args[3..])?;
        match parsed {
            ServiceAction::Up { use_, .. } => assert!(use_.is_empty()),
            _ => panic!("expected Up variant"),
        }
        let round_trip = workestrate::commands::lifecycle::parse_service_action(
            "up",
            &pi.detach_args(&spec)[3..],
        )?;
        match round_trip {
            ServiceAction::Up { use_, .. } => {
                assert_eq!(use_, vec!["litellm@canary", "redis@blue"])
            }
            _ => panic!("expected Up variant"),
        }
        Ok(())
    }

    /// ADR 0026 addendum: `--no-deps` rides the spec into the detached-child
    /// argv so the child does NOT re-run dependency auto-start the parent
    /// was told to skip (CRITICAL: the child re-enters `workload up`).
    #[test]
    fn detach_args_forwards_no_deps() -> Result<()> {
        let _guard = TestConfigGuard::new();
        use workestrate::microsandbox::workload::Workload;
        let example_litellm = ConfigWorkload::new("example-litellm")?;

        let mut spec = spec_for_detach("example-litellm", false);
        spec.no_deps = true;
        let args = example_litellm.detach_args(&spec);
        assert!(
            args.contains(&"--no-deps".to_string()),
            "--no-deps must be forwarded to the detached child: {args:?}"
        );

        // And it round-trips through the raw-args parser the detached-child
        // path uses.
        let parsed = workestrate::commands::lifecycle::parse_service_action("up", &args[3..])?;
        match parsed {
            ServiceAction::Up { no_deps, .. } => assert!(no_deps),
            _ => panic!("expected Up variant"),
        }

        // Unset → no --no-deps token.
        let args = example_litellm.detach_args(&spec_for_detach("example-litellm", false));
        assert!(
            !args.contains(&"--no-deps".to_string()),
            "--no-deps must not appear when unset: {args:?}"
        );
        Ok(())
    }

    /// `--reseed` rides the spec into the detached-child argv so the child
    /// re-renders template seeds exactly as the parent was asked to.
    #[test]
    fn detach_args_forwards_reseed() -> Result<()> {
        let _guard = TestConfigGuard::new();
        use workestrate::microsandbox::workload::Workload;
        let example_litellm = ConfigWorkload::new("example-litellm")?;

        let mut spec = spec_for_detach("example-litellm", false);
        spec.reseed = true;
        let args = example_litellm.detach_args(&spec);
        assert!(
            args.contains(&"--reseed".to_string()),
            "--reseed must be forwarded to the detached child: {args:?}"
        );

        // And it round-trips through the raw-args parser the detached-child
        // path uses.
        let parsed = workestrate::commands::lifecycle::parse_service_action("up", &args[3..])?;
        match parsed {
            ServiceAction::Up { reseed, .. } => assert!(reseed),
            _ => panic!("expected Up variant"),
        }

        // Unset → no --reseed token.
        let args = example_litellm.detach_args(&spec_for_detach("example-litellm", false));
        assert!(
            !args.contains(&"--reseed".to_string()),
            "--reseed must not appear when unset: {args:?}"
        );
        Ok(())
    }

    // --- spec 21 phase E: the images_ready token + --reload-images -------

    /// §2.2 make-or-break token contract: `--images-ready` is ALWAYS in the
    /// detached-child argv (exactly once, right after `--foreground`) for
    /// every flag shape, and `--reload-images` is NEVER in it (USER
    /// DECISION D3) — including when the PARENT was invoked with
    /// `--reload-images` (the flag is parent-side force only; the child
    /// skips ensure anyway via the token).
    #[test]
    fn detach_args_always_carries_images_ready_and_never_reload_images() -> Result<()> {
        let _guard = TestConfigGuard::new();
        use workestrate::microsandbox::workload::Workload;
        let pi = ConfigWorkload::new("pi")?;

        // Every spec shape: singleton, parallel, replace, port-auto,
        // no-deps, use-overrides.
        let mut full = spec_for_detach_full("pi@canary", true, true);
        full.no_deps = true;
        full.use_overrides = vec![("redis".to_string(), "blue".to_string())];
        for spec in [
            spec_for_detach("pi", false),
            spec_for_detach("pi@x1", false),
            spec_for_detach("pi", true),
            spec_for_detach_full("pi", false, true),
            full,
        ] {
            let args = pi.detach_args(&spec);
            let count = args.iter().filter(|a| *a == "--images-ready").count();
            assert_eq!(count, 1, "--images-ready exactly once: {args:?}");
            let fg = args.iter().position(|a| a == "--foreground").unwrap();
            assert_eq!(
                args.get(fg + 1).map(|s| s.as_str()),
                Some("--images-ready"),
                "the token rides right after --foreground: {args:?}"
            );
            assert!(
                !args.contains(&"--reload-images".to_string()),
                "--reload-images must NEVER be forwarded (D3): {args:?}"
            );
        }

        // The parent was invoked with --reload-images: the flag lands on the
        // clap action but there is NO reload state on InstanceSpec to
        // forward — the child's re-parse sees reload_images=false and
        // images_ready=true.
        let cli = Cli::try_parse_from(["workestrate", "workload", "up", "pi", "--reload-images"])
            .expect("parent argv with --reload-images must parse");
        match cli.command {
            Commands::Workload {
                action:
                    WorkloadAction::Up {
                        reload_images,
                        images_ready,
                        ..
                    },
            } => {
                assert!(reload_images);
                assert!(!images_ready, "the parent carries no token");
            }
            _ => panic!("expected workload up"),
        }
        let args = pi.detach_args(&spec_for_detach("pi", false));
        let round_trip = workestrate::commands::lifecycle::parse_service_action("up", &args[3..])?;
        match round_trip {
            ServiceAction::Up {
                reload_images,
                images_ready,
                ..
            } => {
                assert!(!reload_images, "child never re-forces (D3)");
                assert!(images_ready, "child carries the token (§2.2)");
            }
            _ => panic!("expected Up variant"),
        }
        Ok(())
    }

    /// `--images-ready` is a hidden flag on `workload up` (the detach token
    /// is not user surface) and does NOT exist on `workload exec` (agents
    /// are never detached — no token to carry); `--reload-images` is
    /// user-facing on both.
    #[test]
    fn images_ready_is_hidden_on_up_and_absent_on_exec() {
        let cmd = Cli::command();
        let workload = cmd.find_subcommand("workload").unwrap();
        let up = workload.find_subcommand("up").unwrap();
        let images_ready = up
            .get_arguments()
            .find(|a| a.get_long() == Some("images-ready"))
            .expect("workload up must have --images-ready");
        assert!(
            images_ready.is_hide_set(),
            "--images-ready must be hidden (detach token, not user surface)"
        );
        let reload = up
            .get_arguments()
            .find(|a| a.get_long() == Some("reload-images"))
            .expect("workload up must have --reload-images");
        assert!(
            !reload.is_hide_set(),
            "--reload-images is user-facing (spec §5.2)"
        );

        let exec = workload.find_subcommand("exec").unwrap();
        assert!(
            exec.get_arguments()
                .all(|a| a.get_long() != Some("images-ready")),
            "workload exec must NOT have --images-ready (agents are never detached)"
        );
        assert!(
            exec.get_arguments()
                .any(|a| a.get_long() == Some("reload-images")),
            "workload exec must have --reload-images"
        );
    }

    /// Bare-up flag-reject table (§5.2): `--reload-images` is ACCEPTED on
    /// bare `workload up` (batch-scoped force, threaded into
    /// cmd_workload_up_all) while the genuinely name-scoped flags AND the
    /// detach token stay rejected.
    #[test]
    fn bare_up_reject_table_accepts_reload_images_rejects_name_scoped_flags() {
        // Bare up with --reload-images: nothing rejected.
        let cli = Cli::try_parse_from(["workestrate", "workload", "up", "--reload-images"])
            .expect("bare up --reload-images must parse");
        let Commands::Workload { action } = &cli.command else {
            panic!("expected workload action");
        };
        let table = bare_up_reject_table(action);
        assert!(
            table.iter().all(|(present, _)| !present),
            "--reload-images must NOT be in the reject table (D3): {table:?}"
        );
        let flags: Vec<&str> = table.iter().map(|(_, f)| *f).collect();
        assert!(
            !flags.contains(&"--reload-images"),
            "the force flag is batch-scoped, not rejected: {flags:?}"
        );

        // Genuinely-unsupported flags AND the detach token are rejected.
        for (argv, expected) in [
            (
                vec!["workestrate", "workload", "up", "--foreground"],
                "--foreground",
            ),
            (
                vec!["workestrate", "workload", "up", "--replace"],
                "--replace",
            ),
            (vec!["workestrate", "workload", "up", "--new"], "--new"),
            (
                vec!["workestrate", "workload", "up", "--port-auto"],
                "--port-auto",
            ),
            (
                vec!["workestrate", "workload", "up", "--no-deps"],
                "--no-deps",
            ),
            (
                vec!["workestrate", "workload", "up", "--reseed"],
                "--reseed",
            ),
            (
                vec!["workestrate", "workload", "up", "--images-ready"],
                "--images-ready",
            ),
        ] {
            let cli = Cli::try_parse_from(argv.clone()).expect("argv parses");
            let Commands::Workload { action } = &cli.command else {
                panic!("expected workload action");
            };
            let table = bare_up_reject_table(action);
            assert!(
                table.iter().any(|(present, f)| *present && *f == expected),
                "{expected} must be rejected on bare up ({argv:?}): {table:?}"
            );
        }
        // --instance <id> and --use <dep>@<id> shapes.
        for (argv, expected) in [
            (
                vec!["workestrate", "workload", "up", "--instance", "x1"],
                "--instance",
            ),
            (
                vec!["workestrate", "workload", "up", "--use", "redis@blue"],
                "--use",
            ),
        ] {
            let cli = Cli::try_parse_from(argv.clone()).expect("argv parses");
            let Commands::Workload { action } = &cli.command else {
                panic!("expected workload action");
            };
            let table = bare_up_reject_table(action);
            assert!(
                table.iter().any(|(present, f)| *present && *f == expected),
                "{expected} must be rejected on bare up ({argv:?}): {table:?}"
            );
        }
    }

    /// Round-trip (ADR 0021 WP-B): `--new` allocates a base32 slug, build_instance_spec
    /// composes `<slot>@<slug>` and passes it through `validate_instance_id` uniformly,
    /// and the resulting instance name is exactly what `down --instance <slug>` would
    /// target (so a subsequent teardown resolves the same sandbox).
    #[test]
    fn new_slug_round_trips_through_build_instance_spec_and_down_target() -> Result<()> {
        use workestrate::commands::lifecycle::build_instance_spec;
        use workestrate::microsandbox::port_registry::auto_allocate_slug;
        use workestrate::microsandbox::slots::{instance_name, slot_for, validate_instance_id};

        let state_dir = std::env::temp_dir().join(format!(
            "workestrate-slug-roundtrip-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        ));
        std::fs::create_dir_all(&state_dir)?;

        workestrate::config::set_active_context(None);
        let slot = slot_for("litellm", None);
        assert_eq!(slot, "litellm");

        let slug = auto_allocate_slug(&state_dir, &slot)?;
        assert_eq!(slug.len(), 4);
        validate_instance_id(&slug).expect("allocated slug must satisfy the slug rule");

        let spec = build_instance_spec(
            "litellm",
            false,
            None,
            Some(&slug),
            false,
            &[],
            false,
            false,
        )?;
        assert_eq!(
            spec.instance,
            format!("litellm@{slug}"),
            "instance must be <slot>@<slug>, not the bare singleton slot"
        );

        let down_target = instance_name(&slot, Some(&slug));
        assert_eq!(down_target, spec.instance);

        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn cli_exposes_lifecycle_subcommands() {
        let cmd = Cli::command();
        let names: Vec<_> = cmd.get_subcommands().map(|s| s.get_name()).collect();
        for expected in ["ps", "down", "generate-schema"] {
            assert!(names.contains(&expected), "missing subcommand: {expected}");
        }
    }

    /// Root `down` carries the ADR 0032 addendum §Down scope ladder; the
    /// former `down-all` verb survives as a HIDDEN clap alias (back-compat):
    /// `workestrate down --all` and `workestrate down-all --all` both parse
    /// into `Commands::Down { all: true, .. }` (the home scope), and the
    /// other selectors parse to their own rungs.
    #[test]
    fn root_down_ladder_scopes_parse() {
        let home_cases: [&[&str]; 4] = [
            &["workestrate", "down", "--all"],
            &["workestrate", "down-all", "--all"],
            &["workestrate", "down", "--all", "--yes"],
            &["workestrate", "down-all", "--all", "--yes"],
        ];
        for argv in home_cases {
            let cli = Cli::try_parse_from(argv).expect("home-scope down must parse");
            match cli.command {
                Commands::Down {
                    all,
                    context,
                    config_ref,
                    everything,
                    yes,
                } => {
                    assert!(all, "--all must be set for: {argv:?}");
                    assert!(context.is_none() && config_ref.is_none());
                    assert_eq!(everything, 0);
                    assert_eq!(yes, argv.contains(&"--yes"), "yes flag for: {argv:?}");
                }
                _ => panic!("expected Commands::Down for: {argv:?}"),
            }
        }
        // Context / config-ref / everything rungs.
        let cli = Cli::try_parse_from(["workestrate", "down", "--context", "personal"])
            .expect("context scope must parse");
        match cli.command {
            Commands::Down { context, .. } => assert_eq!(context.as_deref(), Some("personal")),
            _ => panic!("expected Commands::Down"),
        }
        let cli = Cli::try_parse_from(["workestrate", "down-all", "--config-ref", "feat-x"])
            .expect("config-ref scope must parse (alias kept)");
        match cli.command {
            Commands::Down { config_ref, .. } => {
                assert_eq!(config_ref.as_deref(), Some("feat-x"))
            }
            _ => panic!("expected Commands::Down"),
        }
        // The DOUBLE GATE's first half is a Count: once parses (the gate
        // refuses later), twice parses to count 2.
        let cli = Cli::try_parse_from(["workestrate", "down", "--everything"])
            .expect("single --everything parses; the gate refuses it");
        match cli.command {
            Commands::Down { everything, .. } => assert_eq!(everything, 1),
            _ => panic!("expected Commands::Down"),
        }
        let cli = Cli::try_parse_from([
            "workestrate",
            "down",
            "--everything",
            "--everything",
            "--yes",
        ])
        .expect("doubled --everything must parse");
        match cli.command {
            Commands::Down {
                everything, yes, ..
            } => {
                assert_eq!(everything, 2);
                assert!(yes);
            }
            _ => panic!("expected Commands::Down"),
        }
    }

    /// Exactly ONE scope selector per invocation (ADR 0032 addendum §Down
    /// scope ladder): clap conflicts_with rejects any pair of selectors.
    #[test]
    fn root_down_scope_selectors_conflict() {
        let cases: [&[&str]; 6] = [
            &["workestrate", "down", "--all", "--context", "x"],
            &["workestrate", "down", "--all", "--config-ref", "y"],
            &["workestrate", "down", "--all", "--everything"],
            &["workestrate", "down", "--context", "x", "--config-ref", "y"],
            &["workestrate", "down", "--context", "x", "--everything"],
            &["workestrate", "down", "--config-ref", "y", "--everything"],
        ];
        for argv in cases {
            assert!(
                Cli::try_parse_from(argv).is_err(),
                "selectors must conflict: {argv:?}"
            );
        }
    }

    /// Bare `down` with NO selector parses at the clap layer but is a USAGE
    /// error naming the ladder (resolve_cli_scope) — it never guesses a
    /// scope. The per-workload `workload down <name>` surface is unchanged.
    #[test]
    fn root_down_bare_requires_a_selector_naming_the_ladder() {
        let cli = Cli::try_parse_from(["workestrate", "down"]).expect("bare down parses");
        match cli.command {
            Commands::Down {
                all,
                context,
                config_ref,
                everything,
                ..
            } => {
                assert!(!all && context.is_none() && config_ref.is_none() && everything == 0);
            }
            _ => panic!("expected Commands::Down"),
        }
        let err =
            workestrate::microsandbox::runtime::down_scope::resolve_cli_scope(false, None, None, 0)
                .unwrap_err()
                .to_string();
        for scope in ["--all", "--context", "--config-ref", "--everything"] {
            assert!(err.contains(scope), "usage error must list {scope}: {err}");
        }
    }

    /// The root `down` alias lives in the ROOT namespace only: the
    /// `workload down <name>` verb keeps its own parse path unchanged.
    #[test]
    fn root_down_alias_does_not_collide_with_workload_down() {
        let cli = Cli::try_parse_from(["workestrate", "workload", "down", "litellm"])
            .expect("workload down litellm must parse");
        match cli.command {
            Commands::Workload {
                action: WorkloadAction::Down { name, .. },
            } => assert_eq!(name, "litellm"),
            _ => panic!("expected workload down"),
        }
    }

    // --- FN-20: --json pre-scan stops at run / -- ---------------------------

    #[test]
    fn json_pre_scan_ignores_payload_json_after_run_separator() {
        // `workestrate run -- bash -c 'echo --json'` — a standalone --json in
        // the run payload (after `--`) must NOT enable JSON mode.
        let args: Vec<String> = ["workestrate", "run", "--", "bash", "-c", "echo", "--json"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(
            !json_mode_from_args(&args),
            "payload --json after `run --` must not enable JSON mode"
        );
    }

    #[test]
    fn json_pre_scan_ignores_payload_json_after_run_no_separator() {
        // `workestrate run bash --json` (no `--`): run captures --json as payload.
        let args: Vec<String> = ["workestrate", "run", "bash", "--json"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(!json_mode_from_args(&args));
    }

    #[test]
    fn json_pre_scan_ignores_payload_json_after_msb_separator() {
        // `workestrate msb -- foo --json` — a standalone --json in the msb
        // payload (after `--`) must NOT enable JSON mode (ADR 0036 D4: msb
        // is a stop-word like run).
        let args: Vec<String> = ["workestrate", "msb", "--", "foo", "--json"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(
            !json_mode_from_args(&args),
            "payload --json after `msb --` must not enable JSON mode"
        );
    }

    #[test]
    fn json_pre_scan_ignores_payload_json_after_msb_no_separator() {
        // `workestrate msb foo --json` (no `--`): msb captures --json as payload.
        let args: Vec<String> = ["workestrate", "msb", "foo", "--json"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(!json_mode_from_args(&args));
    }

    #[test]
    fn msb_passthrough_captures_hyphen_args_after_separator() {
        // Locked-`--` form (ADR 0036 D4): inner msb flags ride after `--`.
        let cli = Cli::try_parse_from(["workestrate", "msb", "--", "sandbox", "list", "--json"])
            .expect("msb passthrough must parse hyphen args after --");
        match cli.command {
            Commands::Msb { args } => assert_eq!(args, vec!["sandbox", "list", "--json"]),
            _ => panic!("expected Commands::Msb"),
        }
    }

    #[test]
    fn json_pre_scan_detects_global_json_before_subcommand() {
        let args: Vec<String> = ["workestrate", "--json", "ps"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(json_mode_from_args(&args));
    }

    #[test]
    fn json_pre_scan_detects_global_json_after_subcommand() {
        // `--json` is a global flag valid on any subcommand (e.g. `ps --json`).
        let args: Vec<String> = ["workestrate", "ps", "--json"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(json_mode_from_args(&args));
    }

    // --- ADR 0027: verb-first `workload` dispatch surface ------------------

    /// The `workload` group exposes the lifecycle verbs plus `new` and
    /// `build` (spec 21 phase C), each with the workload name as the FIRST
    /// positional argument (optional for the batch-capable verbs up/build).
    #[test]
    fn workload_group_exposes_seven_verbs_with_name_positional() {
        let cmd = Cli::command();
        let workload = cmd
            .find_subcommand("workload")
            .expect("workload subcommand must exist");
        let verbs: HashSet<_> = workload
            .get_subcommands()
            .map(|s| s.get_name().to_string())
            .collect();
        for v in ["up", "exec", "plan", "down", "logs", "new", "build"] {
            assert!(verbs.contains(v), "workload missing verb: {v}");
        }
        for v in ["up", "exec", "plan", "down", "logs", "new", "build"] {
            let sub = workload.find_subcommand(v).unwrap();
            let positionals: Vec<String> = sub
                .get_positionals()
                .map(|a| a.get_id().to_string())
                .collect();
            assert_eq!(
                positionals.first().map(|s| s.as_str()),
                Some("name"),
                "workload {v} must take the workload name as its first positional"
            );
        }
    }

    // --- spec 21 phase C: `workload build` grammar (§5.1) ------------------

    /// Bare `workload build` parses with name=None (the batch form: all
    /// nix-layered workloads in the active context) and default flags.
    #[test]
    fn workload_build_bare_parses_as_batch_form_with_defaults() {
        let cli = Cli::try_parse_from(["workestrate", "workload", "build"])
            .expect("bare workload build must parse");
        match cli.command {
            Commands::Workload {
                action:
                    WorkloadAction::Build {
                        name,
                        repo,
                        all_repos,
                        check,
                        force,
                    },
            } => {
                assert_eq!(name, None, "bare build → batch form");
                assert_eq!(repo, None);
                assert!(!all_repos && !check && !force, "defaults are all off");
            }
            _ => panic!("expected workload build"),
        }
    }

    /// Named build + flags parse; the GLOBAL --json propagates (not
    /// re-declared on the verb).
    #[test]
    fn workload_build_named_with_flags_parses() {
        let cli = Cli::try_parse_from([
            "workestrate",
            "workload",
            "build",
            "pi",
            "--check",
            "--force",
            "--json",
        ])
        .expect("workload build <name> --check --force --json must parse");
        assert!(cli.json, "global --json propagates into workload build");
        match cli.command {
            Commands::Workload {
                action:
                    WorkloadAction::Build {
                        name,
                        repo,
                        all_repos,
                        check,
                        force,
                    },
            } => {
                assert_eq!(name.as_deref(), Some("pi"));
                assert_eq!(repo, None);
                assert!(!all_repos);
                assert!(check && force);
            }
            _ => panic!("expected workload build"),
        }
        let cli = Cli::try_parse_from(["workestrate", "workload", "build", "--repo", "personal"])
            .expect("--repo form must parse");
        match cli.command {
            Commands::Workload {
                action: WorkloadAction::Build { name, repo, .. },
            } => {
                assert_eq!(name, None);
                assert_eq!(repo.as_deref(), Some("personal"));
            }
            _ => panic!("expected workload build"),
        }
        let cli = Cli::try_parse_from(["workestrate", "workload", "build", "--all-repos"])
            .expect("--all-repos form must parse");
        match cli.command {
            Commands::Workload {
                action: WorkloadAction::Build { all_repos, .. },
            } => assert!(all_repos),
            _ => panic!("expected workload build"),
        }
    }

    /// §5.1 selector conflicts are parse errors: name+--repo, name+--all-repos,
    /// --repo+--all-repos.
    #[test]
    fn workload_build_selector_conflicts_are_rejected() {
        for argv in [
            vec![
                "workestrate",
                "workload",
                "build",
                "pi",
                "--repo",
                "personal",
            ],
            vec!["workestrate", "workload", "build", "pi", "--all-repos"],
            vec![
                "workestrate",
                "workload",
                "build",
                "--repo",
                "a",
                "--all-repos",
            ],
        ] {
            assert!(
                Cli::try_parse_from(argv.clone()).is_err(),
                "conflicting selectors must fail to parse: {argv:?}"
            );
        }
    }

    /// `build` is a NEW verb — verb-first only: it must NOT join the legacy
    /// name-first shim's VERBS list (`workestrate pi build` stays unparsed).
    #[test]
    fn legacy_shim_does_not_rewrite_build() {
        let args = argv(&["workestrate", "redis", "build"]);
        let (rewritten, warning) =
            rewrite_legacy_workload_argv(args.clone(), &known_subcommand_names(), |n| n == "redis");
        assert_eq!(rewritten, args, "build is not a shimmed legacy verb");
        assert!(warning.is_none());
    }

    /// W6a: `workload new` takes `name` as its first positional and exposes
    /// a `--kind` flag (value-constrained to agent|service).
    #[test]
    fn workload_new_exposes_name_positional_and_kind_flag() {
        let cmd = Cli::command();
        let new = cmd
            .find_subcommand("workload")
            .and_then(|s| s.find_subcommand("new"))
            .expect("workload new must exist");
        let positionals: Vec<String> = new
            .get_positionals()
            .map(|a| a.get_id().to_string())
            .collect();
        assert_eq!(
            positionals.first().map(|s| s.as_str()),
            Some("name"),
            "workload new must take the workload name as its first positional"
        );
        let long_names: Vec<String> = new
            .get_arguments()
            .filter_map(|a| a.get_long().map(|s| s.to_string()))
            .collect();
        assert!(
            long_names.contains(&"kind".to_string()),
            "workload new missing --kind flag; got: {long_names:?}"
        );
        // Parse-level check: --kind service is accepted and lands in the
        // New action; an unconstrained kind value is rejected by clap.
        let cli = Cli::try_parse_from([
            "workestrate",
            "workload",
            "new",
            "my-agent",
            "--kind",
            "service",
        ])
        .expect("workload new <name> --kind service must parse");
        match cli.command {
            Commands::Workload {
                action: WorkloadAction::New { name, kind },
            } => {
                assert_eq!(name, "my-agent");
                assert_eq!(kind, "service");
            }
            _ => panic!("expected Commands::Workload/New"),
        }
        assert!(
            Cli::try_parse_from([
                "workestrate",
                "workload",
                "new",
                "my-agent",
                "--kind",
                "banana",
            ])
            .is_err(),
            "clap must reject an unconstrained --kind value"
        );
    }

    /// W6a: top-level `workestrate new` remains a parseable deprecated alias
    /// but is hidden from help output.
    #[test]
    fn top_level_new_is_hidden_deprecated_alias() {
        let cmd = Cli::command();
        let new = cmd
            .find_subcommand("new")
            .expect("top-level new must remain (deprecated alias)");
        assert!(
            new.is_hide_set(),
            "top-level new must be hidden (deprecated alias for `workload new`)"
        );
    }

    /// `workload up` and `workload exec` expose the full lifecycle flag set
    /// (parity per ADR 0027); plan/down/logs expose their subsets.
    #[test]
    fn workload_verbs_expose_lifecycle_flags() {
        let cmd = Cli::command();
        let workload = cmd.find_subcommand("workload").unwrap();
        let flag_names = |verb: &str| -> Vec<String> {
            workload
                .find_subcommand(verb)
                .unwrap_or_else(|| panic!("workload {verb} must exist"))
                .get_arguments()
                .filter_map(|a| a.get_long().map(|s| s.to_string()))
                .collect()
        };
        for verb in ["up", "exec"] {
            let flags = flag_names(verb);
            for f in [
                "replace",
                "instance",
                "new",
                "foreground",
                "port-auto",
                "use",
                "no-deps",
                "reload-images",
            ] {
                assert!(
                    flags.contains(&f.to_string()),
                    "workload {verb} missing flag: {f}; got: {flags:?}"
                );
            }
        }
        let plan_flags = flag_names("plan");
        for f in ["instance", "use"] {
            assert!(
                plan_flags.contains(&f.to_string()),
                "workload plan missing flag: {f}; got: {plan_flags:?}"
            );
        }
        let down_flags = flag_names("down");
        for f in ["instance", "all-instances"] {
            assert!(
                down_flags.contains(&f.to_string()),
                "workload down missing flag: {f}; got: {down_flags:?}"
            );
        }
        let logs_flags = flag_names("logs");
        assert!(
            logs_flags.contains(&"instance".to_string()),
            "workload logs missing flag: instance; got: {logs_flags:?}"
        );
    }

    /// `--json` is a global flag: `workestrate workload up <name> --json`
    /// must parse with cli.json set (the removed catch-all had to pull
    /// --json out of raw args by hand; clap now propagates it).
    #[test]
    fn workload_up_parses_trailing_global_json_flag() {
        let cli = Cli::try_parse_from(["workestrate", "workload", "up", "pi", "--json"])
            .expect("workload up <name> --json must parse");
        assert!(cli.json, "global --json must propagate into workload up");
        match cli.command {
            Commands::Workload {
                action: WorkloadAction::Up { name, .. },
            } => assert_eq!(name.as_deref(), Some("pi")),
            _ => panic!("expected workload up"),
        }
    }

    /// W4 (ADR 0021 addendum 2026-08-01): bare `workload up` (no name)
    /// parses with `name: None` (the batch form); a named up keeps
    /// `Some(name)`.
    #[test]
    fn workload_up_without_name_parses_as_batch_form() {
        let cli = Cli::try_parse_from(["workestrate", "workload", "up"])
            .expect("bare workload up must parse");
        match cli.command {
            Commands::Workload {
                action: WorkloadAction::Up { name, .. },
            } => assert_eq!(name, None),
            _ => panic!("expected workload up"),
        }
        let cli = Cli::try_parse_from(["workestrate", "workload", "up", "web"])
            .expect("named workload up must parse");
        match cli.command {
            Commands::Workload {
                action: WorkloadAction::Up { name, .. },
            } => assert_eq!(name.as_deref(), Some("web")),
            _ => panic!("expected workload up"),
        }
    }

    /// `workload exec` still REQUIRES a name — only `up` gained the batch
    /// form.
    #[test]
    fn workload_exec_without_name_still_fails_to_parse() {
        assert!(
            Cli::try_parse_from(["workestrate", "workload", "exec"]).is_err(),
            "workload exec without a name must fail to parse"
        );
    }

    /// The detached-child argv shape (`workload up <name> --foreground
    /// --images-ready ...`, ADR 0027 + spec 21 §2.2) parses through clap
    /// into the same action the legacy raw-args parser produced, with the
    /// ensure-images token landing on the Up action.
    #[test]
    fn detach_child_argv_parses_through_clap() {
        let cli = Cli::try_parse_from([
            "workestrate",
            "workload",
            "up",
            "litellm",
            "--foreground",
            "--images-ready",
            "--replace",
            "--port-auto",
            "--use",
            "redis@blue",
            "--instance",
            "canary",
        ])
        .expect("detach-child argv must parse");
        match cli.command {
            Commands::Workload {
                action:
                    WorkloadAction::Up {
                        name,
                        foreground,
                        replace,
                        instance,
                        port_auto,
                        use_,
                        images_ready,
                        reload_images,
                        ..
                    },
            } => {
                assert_eq!(name.as_deref(), Some("litellm"));
                assert!(foreground && replace && port_auto);
                assert!(images_ready, "the token rides the re-exec (spec 21 §2.2)");
                assert!(
                    !reload_images,
                    "--reload-images is never forwarded to the child (D3)"
                );
                assert_eq!(instance.as_deref(), Some("canary"));
                assert_eq!(use_, vec!["redis@blue"]);
            }
            _ => panic!("expected workload up"),
        }
        // A plain foreground up carries NO token: the foreground process IS
        // the parent and ensures.
        let cli = Cli::try_parse_from(["workestrate", "workload", "up", "litellm", "--foreground"])
            .expect("plain foreground up must parse");
        match cli.command {
            Commands::Workload {
                action: WorkloadAction::Up { images_ready, .. },
            } => assert!(!images_ready),
            _ => panic!("expected workload up"),
        }
    }

    // --- ADR 0027: legacy-shape deprecation shim ---------------------------

    fn known_subcommand_names() -> HashSet<String> {
        Cli::command()
            .get_subcommands()
            .map(|s| s.get_name().to_string())
            .collect()
    }

    fn argv(tokens: &[&str]) -> Vec<String> {
        tokens.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn shim_rewrites_legacy_name_first_shape() {
        // `redis` is a config-defined workload with NO built-in subcommand —
        // the exact case the shim exists for. (Cleanup phase 4: there are NO
        // typed workload subcommands left at all — every config-defined
        // workload name takes this path.)
        let args = argv(&["workestrate", "redis", "exec", "--instance", "x"]);
        let (rewritten, warning) =
            rewrite_legacy_workload_argv(args, &known_subcommand_names(), |n| n == "redis");
        assert_eq!(
            rewritten,
            argv(&[
                "workestrate",
                "workload",
                "exec",
                "redis",
                "--instance",
                "x"
            ])
        );
        let warning = warning.expect("a rewrite must carry a deprecation warning");
        assert_eq!(
            warning,
            "warning: `workestrate redis exec ...` is deprecated; use `workestrate workload exec redis ...`"
        );
    }

    #[test]
    fn shim_never_rewrites_builtin_subcommand_names() {
        // Built-ins ALWAYS win: even when is_workload claims `ps` is a
        // configured workload, the name-first shape is left for clap.
        let args = argv(&["workestrate", "ps", "up"]);
        let (rewritten, warning) =
            rewrite_legacy_workload_argv(args.clone(), &known_subcommand_names(), |_| true);
        assert_eq!(rewritten, args);
        assert!(warning.is_none());
    }

    #[test]
    fn shim_never_rewrites_the_workload_group_itself() {
        // The detached child argv already uses the verb-first shape; the
        // shim must never fire for it ("workload" is a known subcommand).
        let args = argv(&["workestrate", "workload", "up", "litellm", "--foreground"]);
        let (rewritten, warning) =
            rewrite_legacy_workload_argv(args.clone(), &known_subcommand_names(), |_| true);
        assert_eq!(rewritten, args);
        assert!(warning.is_none());
    }

    #[test]
    fn shim_ignores_unknown_verb() {
        let args = argv(&["workestrate", "redis", "bogus"]);
        let (rewritten, warning) =
            rewrite_legacy_workload_argv(args.clone(), &known_subcommand_names(), |n| n == "redis");
        assert_eq!(rewritten, args);
        assert!(warning.is_none());
    }

    #[test]
    fn shim_ignores_missing_verb() {
        let args = argv(&["workestrate", "redis"]);
        let (rewritten, warning) =
            rewrite_legacy_workload_argv(args.clone(), &known_subcommand_names(), |n| n == "redis");
        assert_eq!(rewritten, args);
        assert!(warning.is_none());
    }

    #[test]
    fn shim_ignores_leading_global_flag() {
        // A leading global flag at the subcommand position leaves argv
        // untouched (best-effort: never risk breaking a valid invocation).
        let args = argv(&["workestrate", "--json", "redis", "up"]);
        let (rewritten, warning) =
            rewrite_legacy_workload_argv(args.clone(), &known_subcommand_names(), |n| n == "redis");
        assert_eq!(rewritten, args);
        assert!(warning.is_none());
    }

    #[test]
    fn shim_ignores_names_that_are_not_configured_workloads() {
        let args = argv(&["workestrate", "nosuch", "up"]);
        let (rewritten, warning) =
            rewrite_legacy_workload_argv(args.clone(), &known_subcommand_names(), |_| false);
        assert_eq!(rewritten, args);
        assert!(warning.is_none());
    }

    // ---- A5 review MEDIUM-1: invocation_workload_name (the env-pickup name gate) ----

    /// Parse `argv` through the REAL clap parser and return the name gate
    /// input the async_main env pickup would see.
    fn gate_input(args: &[&str]) -> Option<String> {
        let cli = Cli::try_parse_from(args).expect("argv must parse");
        invocation_workload_name(&cli.command)
    }

    /// Every named workload verb yields the BARE positional name (the part
    /// before any `:ref`) — this is what a detached dependent child
    /// (`workload up prime --instance <id>`, env WORKESTRATE_WORKLOAD_REF=
    /// prime:feat-x) matches on.
    #[test]
    fn invocation_workload_name_extracts_the_bare_positional() {
        for (args, expected) in [
            (
                &["workestrate", "workload", "up", "prime"][..],
                Some("prime"),
            ),
            (
                &["workestrate", "workload", "up", "prime:feat-x"][..],
                Some("prime"),
            ),
            (
                &["workestrate", "workload", "up", "prime:feat-x@canary"][..],
                Some("prime"),
            ),
            (
                &["workestrate", "workload", "exec", "prime"][..],
                Some("prime"),
            ),
            (
                &["workestrate", "workload", "plan", "prime"][..],
                Some("prime"),
            ),
            (
                &["workestrate", "workload", "down", "prime"][..],
                Some("prime"),
            ),
            (
                &["workestrate", "workload", "logs", "prime"][..],
                Some("prime"),
            ),
            (
                &["workestrate", "workload", "build", "prime"][..],
                Some("prime"),
            ),
        ] {
            assert_eq!(
                gate_input(args),
                expected.map(str::to_string),
                "gate input for {args:?}"
            );
        }
    }

    /// The bare batch `workload up` (no name), batch `build`, and every
    /// non-workload command carry NO invocation name — the env pickup never
    /// sets pending for them, so an inherited/leftover
    /// WORKESTRATE_WORKLOAD_REF can never arm there.
    #[test]
    fn invocation_workload_name_is_none_for_batch_and_non_workload_commands() {
        for args in [
            &["workestrate", "workload", "up"][..],
            &["workestrate", "workload", "build"][..],
            &["workestrate", "ps"][..],
            &["workestrate", "workloads"][..],
        ] {
            assert_eq!(gate_input(args), None, "gate input for {args:?}");
        }
    }
}
