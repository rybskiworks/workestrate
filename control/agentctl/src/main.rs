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
#[command(version)]
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
    /// Run a configured workload: up/exec/plan/down/logs <name> (ADR 0027).
    /// Workload names are ARGUMENTS, never subcommands, so a config-defined
    /// workload can never be shadowed by a built-in verb.
    Workload {
        #[command(subcommand)]
        action: WorkloadAction,
    },
    /// List configured workloads (name, kind, image, running status).
    Workloads,
}

/// Extract the typed `--use <dep>@<instance>` overrides from a clap-parsed
/// ServiceAction (ADR 0026(d)). The hardcoded subcommands parse the action
/// via clap BEFORE constructing the workload, so the overrides reach
/// `ConfigWorkload::new_with_use_overrides` and resolution selects the same
/// records the dispatch will forward to a detached child.
fn service_action_use_overrides(action: &ServiceAction) -> Result<Vec<(String, String)>> {
    let raw: &[String] = match action {
        ServiceAction::Up { use_, .. } | ServiceAction::Plan { use_, .. } => use_,
        _ => &[],
    };
    workestrate::microsandbox::discovery::parse_use_overrides(raw)
}

/// The AgentAction counterpart of [`service_action_use_overrides`].
fn agent_action_use_overrides(action: &AgentAction) -> Result<Vec<(String, String)>> {
    let raw: &[String] = match action {
        AgentAction::Exec { use_, .. } | AgentAction::Plan { use_, .. } => use_,
        _ => &[],
    };
    workestrate::microsandbox::discovery::parse_use_overrides(raw)
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
            ..
        } => ServiceAction::Up {
            foreground,
            replace,
            instance,
            new,
            port_auto,
            use_,
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
            ..
        } => AgentAction::Exec {
            replace,
            instance,
            new,
            port_auto,
            use_,
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
    }
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
        Commands::New { name } => cmd_new(&name),
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
        Commands::GenerateSchema { output } => cmd_generate_schema(output.as_deref()),
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
        Commands::Litellm { action } => {
            let overrides = service_action_use_overrides(&action)?;
            let workload = ConfigWorkload::new_with_use_overrides("litellm", &overrides)?;
            dispatch_service(&workload, action, cli.show_source, cli.json).await
        }
        Commands::Pi { action } => {
            let overrides = agent_action_use_overrides(&action)?;
            let workload = ConfigWorkload::new_with_use_overrides("pi", &overrides)?;
            dispatch_agent(&workload, action, cli.show_source, cli.json).await
        }
        Commands::Odysseus { action } => {
            let overrides = service_action_use_overrides(&action)?;
            let workload = ConfigWorkload::new_with_use_overrides("odysseus", &overrides)?;
            dispatch_service(&workload, action, cli.show_source, cli.json).await
        }
        Commands::Opencode { action } => {
            let overrides = agent_action_use_overrides(&action)?;
            let workload = ConfigWorkload::new_with_use_overrides("opencode", &overrides)?;
            dispatch_agent(&workload, action, cli.show_source, cli.json).await
        }
        Commands::Tempest { action } => {
            let overrides = agent_action_use_overrides(&action)?;
            let workload = ConfigWorkload::new_with_use_overrides("tempest", &overrides)?;
            dispatch_agent(&workload, action, cli.show_source, cli.json).await
        }
        Commands::MigrateHome {
            from,
            dry_run,
            force,
        } => cmd_migrate_home(from.as_deref(), dry_run, cli.json, force),
        Commands::Workload { action } => {
            // ADR 0027 verb-first dispatch: the workload name is a clap
            // positional, so `--json` and every flag is parsed by clap
            // directly (no raw-args extraction). The `--use` values reach
            // the workload constructor BEFORE resolution runs (ADR 0026(d)).
            let (name, verb, use_values): (String, &'static str, Vec<String>) = match &action {
                WorkloadAction::Up { name, use_, .. } => (name.clone(), "up", use_.clone()),
                WorkloadAction::Exec { name, use_, .. } => (name.clone(), "exec", use_.clone()),
                WorkloadAction::Plan { name, use_, .. } => (name.clone(), "plan", use_.clone()),
                WorkloadAction::Down { name, .. } => (name.clone(), "down", Vec::new()),
                WorkloadAction::Logs { name, .. } => (name.clone(), "logs", Vec::new()),
            };
            let overrides = workestrate::microsandbox::discovery::parse_use_overrides(&use_values)?;
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
            "litellm",
            "pi",
            "odysseus",
            "opencode",
            "tempest",
            "migrate-home",
            "workload",
            "workloads",
        ] {
            assert!(names.contains(&expected), "missing subcommand: {expected}");
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

    /// Build an InstanceSpec for `litellm` (no context) with selected flags.
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
            workload: "litellm".to_string(),
            context: None,
            replace,
            port_auto,
            use_overrides: Vec::new(),
        }
    }

    #[test]
    fn detach_args_include_foreground() -> Result<()> {
        let _guard = TestConfigGuard::new();
        use workestrate::microsandbox::workload::Workload;
        let litellm = ConfigWorkload::new("litellm")?;

        // Singleton, no flags: verb-first `workload up <name> --foreground`
        // (ADR 0027).
        let args = litellm.detach_args(&spec_for_detach("litellm", false));
        assert_eq!(args, vec!["workload", "up", "litellm", "--foreground"]);

        // Parallel instance: forward `--instance <id>` with the BARE id, never
        // `slot@id`. `--new` must NOT appear (already materialized by parent).
        let args = litellm.detach_args(&spec_for_detach("litellm@canary", false));
        assert_eq!(
            args,
            vec![
                "workload",
                "up",
                "litellm",
                "--foreground",
                "--instance",
                "canary"
            ]
        );
        assert!(
            !args.contains(&"--new".to_string()),
            "--new must not be forwarded (already materialized into spec.instance)"
        );

        // `--replace` is forwarded when requested.
        let args = litellm.detach_args(&spec_for_detach("litellm", true));
        assert_eq!(
            args,
            vec!["workload", "up", "litellm", "--foreground", "--replace"]
        );

        // `--replace` and `--instance` compose.
        let args = litellm.detach_args(&spec_for_detach("litellm@ab2z", true));
        assert_eq!(
            args,
            vec![
                "workload",
                "up",
                "litellm",
                "--foreground",
                "--replace",
                "--instance",
                "ab2z",
            ]
        );

        // A workload other than litellm uses its own name as the <name>
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
        let args = litellm.detach_args(&spec_for_detach_full("litellm@canary", false, true));
        assert!(
            args.contains(&"--port-auto".to_string()),
            "--port-auto must be forwarded to the detached child: {args:?}"
        );
        assert_eq!(
            args,
            vec![
                "workload",
                "up",
                "litellm",
                "--foreground",
                "--port-auto",
                "--instance",
                "canary",
            ]
        );
        let args = litellm.detach_args(&spec_for_detach("litellm", false));
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

        let spec = build_instance_spec("litellm", false, None, Some(&slug), false, &[])?;
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

    #[test]
    fn service_up_exposes_lifecycle_flags() {
        let cmd = Cli::command();
        let up = cmd
            .find_subcommand("litellm")
            .and_then(|s| s.find_subcommand("up"))
            .expect("litellm up must exist");
        // get_long() returns the user-facing long flag name (clap hyphenates
        // underscores: all_instances → all-instances). get_id() preserves the
        // raw field identifier; the CLI surface is what we care about here.
        let flag_names: Vec<_> = up
            .get_arguments()
            .filter_map(|a| a.get_long().map(|s| s.to_string()))
            .collect();
        for f in ["replace", "instance", "new", "foreground", "port-auto"] {
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

    /// The `workload` group exposes exactly the five verbs, each with the
    /// workload name as the FIRST positional argument.
    #[test]
    fn workload_group_exposes_five_verbs_with_name_positional() {
        let cmd = Cli::command();
        let workload = cmd
            .find_subcommand("workload")
            .expect("workload subcommand must exist");
        let verbs: HashSet<_> = workload
            .get_subcommands()
            .map(|s| s.get_name().to_string())
            .collect();
        for v in ["up", "exec", "plan", "down", "logs"] {
            assert!(verbs.contains(v), "workload missing verb: {v}");
        }
        for v in ["up", "exec", "plan", "down", "logs"] {
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
            } => assert_eq!(name, "pi"),
            _ => panic!("expected workload up"),
        }
    }

    /// The detached-child argv shape (`workload up <name> --foreground ...`,
    /// ADR 0027) parses through clap into the same action the legacy raw-args
    /// parser produced.
    #[test]
    fn detach_child_argv_parses_through_clap() {
        let cli = Cli::try_parse_from([
            "workestrate",
            "workload",
            "up",
            "litellm",
            "--foreground",
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
                        ..
                    },
            } => {
                assert_eq!(name, "litellm");
                assert!(foreground && replace && port_auto);
                assert_eq!(instance.as_deref(), Some("canary"));
                assert_eq!(use_, vec!["redis@blue"]);
            }
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
        // the exact case the shim exists for. (`pi`/`litellm` are built-in
        // typed subcommands, so built-ins win and the shim never fires.)
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
