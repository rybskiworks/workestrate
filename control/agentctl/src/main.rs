use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};

mod config;
mod microsandbox;
mod policy;
mod recipes;

use config::CheckEntry;
use microsandbox::workload::{ConfigWorkload, Workload};

#[derive(Parser)]
#[command(name = "workestrate")]
#[command(about = "Control plane CLI for the AI workbench")]
#[command(version)]
struct Cli {
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
    },
    /// Stop and remove the sandbox
    Down,
    /// Tail the detached service's log file
    Logs,
    /// Print the planned sandbox workload
    Plan,
}

/// Actions available on agent workloads (interactive TUI attach).
#[derive(Subcommand)]
enum AgentAction {
    /// Attach to the sandbox interactively (TUI)
    Exec,
    /// Stop and remove the sandbox
    Down,
    /// Print the planned sandbox workload
    Plan,
}

#[derive(Subcommand)]
enum Commands {
    /// Runtime/config sanity check
    Check,
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

async fn dispatch_service<W: Workload>(workload: &W, action: ServiceAction) -> Result<()> {
    match action {
        ServiceAction::Up { foreground } => microsandbox::up_service(workload, foreground).await,
        ServiceAction::Down => microsandbox::down(workload.name()).await,
        ServiceAction::Logs => microsandbox::logs(workload.name()).await,
        ServiceAction::Plan => {
            println!("{}", workload.plan());
            Ok(())
        }
    }
}

async fn dispatch_agent<W: Workload>(workload: &W, action: AgentAction) -> Result<()> {
    match action {
        AgentAction::Exec => microsandbox::exec_agent(workload).await,
        AgentAction::Down => microsandbox::down(workload.name()).await,
        AgentAction::Plan => {
            println!("{}", workload.plan());
            Ok(())
        }
    }
}

fn parse_service_action(action: &str, args: &[String]) -> Result<ServiceAction> {
    match action {
        "up" => {
            let foreground = args.iter().any(|a| a == "--foreground");
            Ok(ServiceAction::Up { foreground })
        }
        "down" => Ok(ServiceAction::Down),
        "logs" => Ok(ServiceAction::Logs),
        "plan" => Ok(ServiceAction::Plan),
        other => anyhow::bail!("unknown service action: {}", other),
    }
}

fn parse_agent_action(action: &str) -> Result<AgentAction> {
    match action {
        "exec" => Ok(AgentAction::Exec),
        "down" => Ok(AgentAction::Down),
        "plan" => Ok(AgentAction::Plan),
        other => anyhow::bail!("unknown agent action: {}", other),
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

async fn cmd_new(name: &str) -> Result<()> {
    let root = config::project_root()?;
    let agent_dir = root.join("agents").join(name);

    if agent_dir.exists() {
        anyhow::bail!("agents/{} already exists", name);
    }

    // Create directory structure
    std::fs::create_dir_all(agent_dir.join("repo"))?;
    std::fs::create_dir_all(agent_dir.join("config"))?;
    std::fs::write(agent_dir.join(".gitkeep"), "")?;

    println!("Created agents/{}/", name);
    println!(
        "  agents/{}/repo/    — clone or create your agent source here",
        name
    );
    println!(
        "  agents/{}/config/  — agent-specific config files here",
        name
    );
    println!();
    println!("Next steps:");
    println!(
        "  1. Clone your agent: git clone <url> agents/{}/repo",
        name
    );
    println!(
        "  2. Build it:        cd agents/{}/repo && npm install && npm run build",
        name
    );
    println!(
        "     (or: cp -r agents/{}/repo agents/{}/build && cd agents/{}/build && <build-cmd>)",
        name, name, name
    );
    println!("  3. Add a workload entry to workestrate.toml");
    println!("  4. Test: cargo run -- {} plan", name);
    println!();
    println!("Or use a pre-built image (like LiteLLM):");
    println!("  Set image = {{ recipe = 'registry', ref = 'your-image:tag' }} and skip the source mount.");

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check => cmd_check().await,
        Commands::New { name } => cmd_new(&name).await,
        Commands::Completions { shell, for_name } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, &for_name, &mut std::io::stdout());
            Ok(())
        }
        Commands::Run { command } => cmd_run(&command).await,
        Commands::ValidateConfig => cmd_validate_config().await,
        Commands::Litellm { action } => {
            let workload = ConfigWorkload::new("litellm")?;
            dispatch_service(&workload, action).await
        }
        Commands::Pi { action } => {
            let workload = ConfigWorkload::new("pi")?;
            dispatch_agent(&workload, action).await
        }
        Commands::Odysseus { action } => {
            let workload = ConfigWorkload::new("odysseus")?;
            dispatch_service(&workload, action).await
        }
        Commands::Opencode { action } => {
            let workload = ConfigWorkload::new("opencode")?;
            dispatch_agent(&workload, action).await
        }
        Commands::Tempest { action } => {
            let workload = ConfigWorkload::new("tempest")?;
            dispatch_agent(&workload, action).await
        }
        Commands::Workload(mut args) => {
            if args.is_empty() {
                anyhow::bail!("no workload name given");
            }
            let name = args.remove(0);
            let action = args.first().cloned().unwrap_or_else(|| "plan".to_string());
            let workload = ConfigWorkload::new(&name)?;
            match workload.kind() {
                "service" => {
                    let service_action = parse_service_action(&action, &args)?;
                    dispatch_service(&workload, service_action).await
                }
                "agent" => {
                    let agent_action = parse_agent_action(&action)?;
                    dispatch_agent(&workload, agent_action).await
                }
                other => anyhow::bail!("unknown workload kind '{}' for '{}'", other, name),
            }
        }
    }
}

async fn cmd_check() -> Result<()> {
    let root = config::project_root()?;
    let checks = config::check_required_files(&root)?;

    let mut all_required_ok = true;
    for entry in &checks {
        print_entry(entry);
        if !entry.ok && !entry.optional {
            all_required_ok = false;
        }
    }

    if all_required_ok {
        println!("\nAll required checks passed.");
        Ok(())
    } else {
        Err(anyhow::anyhow!("Some required checks failed."))
    }
}

async fn cmd_validate_config() -> Result<()> {
    let config = config::load_config()?;
    config::validate_config(&config)?;
    println!("workestrate.toml is valid.");
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
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn cli_exposes_expected_subcommands() {
        let cmd = Cli::command();
        let names: Vec<_> = cmd.get_subcommands().map(|s| s.get_name()).collect();
        for expected in [
            "check",
            "new",
            "completions",
            "run",
            "validate-config",
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
        match cmd.find_subcommand(name) {
            Some(sub) => {
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
                for unexpected in ["exec"] {
                    assert!(
                        !action_names.contains(unexpected),
                        "{} should not have action: {}",
                        name,
                        unexpected
                    );
                }
            }
            None => assert!(false, "missing subcommand: {name}"),
        }
    }

    fn check_agent_subcommands(cmd: &clap::Command, name: &str) {
        match cmd.find_subcommand(name) {
            Some(sub) => {
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
                for unexpected in ["up", "logs"] {
                    assert!(
                        !action_names.contains(unexpected),
                        "{} should not have action: {}",
                        name,
                        unexpected
                    );
                }
            }
            None => assert!(false, "missing subcommand: {name}"),
        }
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
}
