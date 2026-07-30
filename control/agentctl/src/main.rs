use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use std::path::PathBuf;

use workestrate::cli_actions::{
    AgentAction, ConfigAction, ContextAction, HomeAction, ServiceAction, SourceAction,
};
use workestrate::cli_error::{classify_exit_code, emit_error};
use workestrate::commands::config_cmd::{
    cmd_config, cmd_config_list_json, cmd_config_new, cmd_context,
};
use workestrate::commands::diagnostics::{
    cmd_check, cmd_generate_env_example, cmd_generate_schema, cmd_ps, cmd_run, cmd_validate_config,
};
use workestrate::commands::doctor::cmd_doctor;
use workestrate::commands::home::cmd_home;
use workestrate::commands::init::{cmd_init, cmd_new};
use workestrate::commands::lifecycle::{
    cmd_clean, cmd_down_all, dispatch_agent, dispatch_service, parse_agent_action,
    parse_service_action,
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
    Doctor {
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
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
        /// Emit a machine-readable JSON summary instead of human text.
        #[arg(long)]
        json: bool,
        /// Allow migrating into a destination that already exists / is non-empty.
        #[arg(long)]
        force: bool,
    },
    /// Catch-all for config-defined workloads
    #[command(external_subcommand)]
    Workload(Vec<String>),
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

fn main() {
    // Pre-scan argv for --json so we can format ANY error (incl. clap parse
    // errors via Cli::parse()) as the JSON envelope when requested. The
    // global --json on Cli does not help here because Cli::parse() calls
    // process::exit on usage errors before we'd see the parsed value.
    let json_mode = json_mode_from_args(&std::env::args().collect::<Vec<_>>());

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
        Commands::GenerateSchema { out } => cmd_generate_schema(out.as_deref()),
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
        Commands::Home { action } => cmd_home(action),
        Commands::SecretsTarget { name } => cmd_secrets_target(&name, cli.json).await,
        Commands::Doctor { json } => cmd_doctor(json),
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
        Commands::MigrateHome {
            from,
            dry_run,
            json,
            force,
        } => cmd_migrate_home(from.as_deref(), dry_run, json, force),
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
        // ADR 0025: provisioning flags.
        assert!(
            long_names.contains(&"from".to_string()),
            "home init missing --from flag (ADR 0025); got: {long_names:?}"
        );
        let positionals: Vec<String> = init
            .get_positionals()
            .map(|a| a.get_id().to_string())
            .collect();
        assert!(
            positionals.contains(&"dest".to_string()),
            "home init missing positional <dest> (ADR 0025); got: {positionals:?}"
        );
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
        port_offset: u16,
    ) -> workestrate::microsandbox::runtime::InstanceSpec {
        use workestrate::microsandbox::runtime::InstanceSpec;
        InstanceSpec {
            instance: instance.to_string(),
            workload: "litellm".to_string(),
            context: None,
            port_offset,
            replace,
        }
    }

    #[test]
    fn detach_args_include_foreground() -> Result<()> {
        let _guard = TestConfigGuard::new();
        use workestrate::microsandbox::workload::Workload;
        let litellm = ConfigWorkload::new("litellm")?;

        // Singleton, no flags: just `<name> up --foreground`.
        let args = litellm.detach_args(&spec_for_detach("litellm", false, 0));
        assert_eq!(args, vec!["litellm", "up", "--foreground"]);

        // Parallel instance: forward `--instance <id>` with the BARE id, never
        // `slot@id`. `--new` must NOT appear (already materialized by parent).
        let args = litellm.detach_args(&spec_for_detach("litellm@canary", false, 0));
        assert_eq!(
            args,
            vec!["litellm", "up", "--foreground", "--instance", "canary"]
        );
        assert!(
            !args.contains(&"--new".to_string()),
            "--new must not be forwarded (already materialized into spec.instance)"
        );

        // `--replace` is forwarded when requested.
        let args = litellm.detach_args(&spec_for_detach("litellm", true, 0));
        assert_eq!(args, vec!["litellm", "up", "--foreground", "--replace"]);

        // `--port-offset` is forwarded only when nonzero.
        let args = litellm.detach_args(&spec_for_detach("litellm@ab2z", true, 10000));
        assert_eq!(
            args,
            vec![
                "litellm",
                "up",
                "--foreground",
                "--replace",
                "--instance",
                "ab2z",
                "--port-offset",
                "10000",
            ]
        );

        // A workload other than litellm uses its own name as argv[0].
        let pi = ConfigWorkload::new("pi")?;
        let args = pi.detach_args(&spec_for_detach("pi@xy7", false, 5000));
        assert_eq!(args.first(), Some(&"pi".to_string()));
        assert!(args.contains(&"--foreground".to_string()));
        assert!(args.contains(&"--instance".to_string()));
        assert!(args.contains(&"--port-offset".to_string()));
        assert_eq!(args[args.len() - 1], "5000");
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

        let spec = build_instance_spec("litellm", false, None, Some(&slug), 10000)?;
        assert_eq!(
            spec.instance,
            format!("litellm@{slug}"),
            "instance must be <slot>@<slug>, not the bare singleton slot"
        );
        assert_eq!(spec.port_offset, 10000);

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
}
