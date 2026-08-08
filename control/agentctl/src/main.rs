use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use std::collections::HashSet;
use std::path::PathBuf;

use workestrate::cli_actions::{
    AgentAction, ConfigAction, ContextAction, HomeAction, ServiceAction, SourceAction,
    WorkloadAction,
};
use workestrate::cli_error::{classify_exit_code, emit_error};
use workestrate::commands::config_cmd::{
    cmd_config, cmd_config_list_json, cmd_config_new, cmd_context,
};
use workestrate::commands::deps::{auto_start_dependencies, cmd_workload_up_all};
use workestrate::commands::diagnostics::{
    cmd_check, cmd_generate_env_example, cmd_generate_schema, cmd_ps, cmd_run, cmd_validate_config,
    cmd_workloads,
};
use workestrate::commands::doctor::cmd_doctor;
use workestrate::commands::home::cmd_home;
use workestrate::commands::init::{cmd_init, cmd_new};
use workestrate::commands::lifecycle::{
    cmd_clean, cmd_down_all, dispatch_agent, dispatch_service, workload_route, WorkloadRoute,
};
use workestrate::commands::migrate::cmd_migrate_home;
use workestrate::commands::secrets_target::{cmd_secrets_schema, cmd_secrets_target};
use workestrate::commands::source::cmd_source;
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
    /// Remove state-dir contents (workspaces, var, run). Does not touch config-repos/sources/config.
    Clean {
        /// Skip the interactive confirmation.
        #[arg(long)]
        yes: bool,
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
    /// Resolve a registered config repo's secrets target paths (for setup-secrets).
    SecretsTarget {
        /// Config repo name to resolve.
        name: String,
    },
    /// Diagnose environment and tool health (KVM, nix, sops, age, msb, config repos).
    /// Use the global --json flag for machine-readable output.
    Doctor,
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
}

/// Pre-scan argv for a global `--json` flag so ANY error (including clap
/// parse errors, which call process::exit before `cli.json` is available) can
/// be formatted as the JSON envelope. Scanning stops at the first `--`
/// separator or at the `run` subcommand: `run` captures all trailing args
/// verbatim as the command payload, so a payload `--json` (e.g.
/// `workestrate run -- somecmd --json`) must not enable JSON mode.
fn json_mode_from_args(args: &[String]) -> bool {
    for a in args.iter().skip(1) {
        if a == "--json" {
            return true;
        }
        if a == "--" || a == "run" {
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
            reload_images,
            ..
        } => AgentAction::Exec {
            replace,
            instance,
            new,
            port_auto,
            use_,
            no_deps,
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
        (*images_ready, "--images-ready"),
    ]
}

fn main() {
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

async fn async_main(args: Vec<String>) -> Result<()> {
    let cli = Cli::parse_from(args);
    if cli.no_project_config {
        std::env::set_var("WORKESTRATE_NO_PROJECT_CONFIG", "1");
    }
    if let Some(ref ctx) = cli.context {
        std::env::set_var("WORKESTRATE_CONTEXT", ctx);
    }
    // --home <DIR> populates the WORKESTRATE_HOME precedence step
    // (paths.rs resolve_home_with_kind checks it first), so the flag becomes
    // the highest-precedence override with no path-resolution change.
    if let Some(ref h) = cli.home {
        std::env::set_var("WORKESTRATE_HOME", h);
    }

    match cli.command {
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
        Commands::ValidateConfig => cmd_validate_config(),
        Commands::SecretsSchema => cmd_secrets_schema(),
        Commands::GenerateEnvExample { output } => cmd_generate_env_example(output.as_deref()),
        Commands::Ps => cmd_ps(cli.json).await,
        Commands::DownAll { yes } => cmd_down_all(yes, cli.json).await,
        Commands::Clean { yes } => cmd_clean(yes, cli.json),
        Commands::Context { action } => cmd_context(action, cli.json).await,
        Commands::GenerateSchema {
            output,
            output_workload,
        } => cmd_generate_schema(output.as_deref(), output_workload.as_deref()),
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
        Commands::SecretsTarget { name } => cmd_secrets_target(&name, cli.json).await,
        Commands::Doctor => cmd_doctor(cli.json),
        Commands::Source { action } => cmd_source(action).await,
        Commands::MigrateHome {
            from,
            dry_run,
            force,
        } => cmd_migrate_home(from.as_deref(), dry_run, cli.json, force),
        Commands::Workload { action } => {
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
            let overrides = workestrate::microsandbox::discovery::parse_use_overrides(&use_values)?;
            // Spec 21 §2/§2.4 (phase E): the ensure-images pre-flight runs
            // on the NAMED workload BEFORE dependency auto-start — fail fast
            // on the workload the operator actually asked for before
            // spending minutes starting its dep closure. The detached child
            // carries the --images-ready token and skips this entirely
            // (§2.2); a token-free foreground up/exec IS the parent and
            // ensures. `--reload-images` maps to force (§5.2).
            if workestrate::images::ensure::ensure_should_run(verb, images_ready) {
                workestrate::images::ensure::ensure_images_for_workload(&name, reload_images)
                    .await?;
            }
            // Construction-order rule (ADR 0026 addendum): declared deps
            // start BEFORE the dependent's ConfigWorkload is constructed —
            // construction runs resolve_depends_on, which refuses a
            // required-not-running dep, so the dep must already be up. Dep
            // auto-start inherits the ensure pre-flight per dependency (spec
            // 21 §2.1) inside auto_start_dependencies.
            auto_start_dependencies(&name, verb, no_deps, &overrides).await?;
            let workload = ConfigWorkload::new_with_use_overrides(&name, &overrides)?;
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
    use workestrate::config::test_support::TestConfigGuard;

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
            "home",
            "secrets-target",
            "doctor",
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
            images_ready: false,
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

        let spec = build_instance_spec("litellm", false, None, Some(&slug), false, &[], false)?;
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
        for expected in ["ps", "down-all", "generate-schema"] {
            assert!(names.contains(&expected), "missing subcommand: {expected}");
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
}
