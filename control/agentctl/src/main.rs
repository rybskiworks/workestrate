use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};

mod config;
mod microsandbox;
mod workloads;

use config::CheckEntry;
use microsandbox::workload::Workload;

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

/// Single source of truth for sandbox workloads.
/// Adding a workload = one entry here + workloads/<name>.rs + mod.rs wiring.
macro_rules! workloads {
    ($macro:ident) => {
        $macro!(
            Litellm, workloads::Litellm, Service, "LiteLLM proxy sandbox";
            Odysseus, workloads::Odysseus, Service, "Odysseus agent sandbox";
            Pi, workloads::Pi, Agent, "Pi coding agent sandbox";
            Opencode, workloads::Opencode, Agent, "OpenCode agent sandbox";
            Tempest, workloads::Tempest, Agent, "T3MP3ST offensive-security agent sandbox";
        );
    };
    ($prefix:expr, $macro:ident) => {
        $macro!(
            $prefix,
            Litellm, workloads::Litellm, Service, "LiteLLM proxy sandbox";
            Odysseus, workloads::Odysseus, Service, "Odysseus agent sandbox";
            Pi, workloads::Pi, Agent, "Pi coding agent sandbox";
            Opencode, workloads::Opencode, Agent, "OpenCode agent sandbox";
            Tempest, workloads::Tempest, Agent, "T3MP3ST offensive-security agent sandbox";
        );
    };
}

macro_rules! kind_action {
    (Service) => {
        ServiceAction
    };
    (Agent) => {
        AgentAction
    };
}

macro_rules! kind_dispatch {
    (Service) => {
        dispatch_service
    };
    (Agent) => {
        dispatch_agent
    };
}

macro_rules! define_commands_enum {
    ($($name:ident, $ty:path, $kind:ident, $doc:literal);* $(;)?) => {
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
                #[arg(long = "for", value_name = "NAME", default_value = "workestrate", help = "Command name to generate completions for")]
                for_name: String,
            },
            /// Run an arbitrary command with decrypted secrets
            Run {
                /// Command and arguments (after --)
                #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 1..)]
                command: Vec<String>,
            },
            $(
                #[doc = $doc]
                $name {
                    #[command(subcommand)]
                    action: kind_action!($kind),
                },
            )*
        }
    };
}

workloads!(define_commands_enum);

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

macro_rules! define_dispatch {
    ($($name:ident, $ty:path, $kind:ident, $doc:literal);* $(;)?) => {
        async fn dispatch_workload(command: Commands) -> Result<()> {
            match command {
                $(
                    Commands::$name { action } => {
                        kind_dispatch!($kind)((&$ty), action).await
                    }
                )*
                _ => Err(anyhow::anyhow!("internal: non-workload command dispatched")),
            }
        }
    };
}

workloads!(define_dispatch);

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
    println!("  3. Register in Rust:");
    println!(
        "     a. Create control/agentctl/src/workloads/{}.rs",
        name.replace('-', "_")
    );
    println!("     b. Add to workloads/mod.rs registry");
    println!("     c. Add one entry to the `workloads!` macro in main.rs");
    println!("  4. Test: nix develop -c cargo run -- {} plan", name);
    println!();
    println!("Or use a pre-built image (like LiteLLM):");
    println!("  Set image: Some(\"your-image:tag\") and skip the source mount.");

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
        command => dispatch_workload(command).await,
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

async fn cmd_run(command: &[String]) -> Result<()> {
    if command.is_empty() {
        anyhow::bail!(
            "no command specified. Usage: workestrate run -- <command> [args...]"
        );
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
            "litellm",
            "pi",
            "odysseus",
            "opencode",
            "tempest",
        ] {
            assert!(names.contains(&expected), "missing subcommand: {expected}");
        }
    }

    macro_rules! check_kind {
        ($cmd:expr, $name:ident, $ty:path, $kind:ident, $doc:literal) => {{
            let name_lc = stringify!($name).to_lowercase();
            let sub = $cmd
                .find_subcommand(&name_lc)
                .unwrap_or_else(|| panic!("missing subcommand: {}", name_lc));
            let action_names: HashSet<_> = sub
                .get_subcommands()
                .map(|s| s.get_name().to_string())
                .collect();
            match stringify!($kind) {
                "Service" => {
                    for expected in ["up", "down", "logs", "plan"] {
                        assert!(
                            action_names.contains(expected),
                            "{} missing action: {}",
                            name_lc,
                            expected
                        );
                    }
                    for unexpected in ["exec"] {
                        assert!(
                            !action_names.contains(unexpected),
                            "{} should not have action: {}",
                            name_lc,
                            unexpected
                        );
                    }
                }
                "Agent" => {
                    for expected in ["exec", "down", "plan"] {
                        assert!(
                            action_names.contains(expected),
                            "{} missing action: {}",
                            name_lc,
                            expected
                        );
                    }
                    for unexpected in ["up", "logs"] {
                        assert!(
                            !action_names.contains(unexpected),
                            "{} should not have action: {}",
                            name_lc,
                            unexpected
                        );
                    }
                }
                other => panic!("unknown kind: {}", other),
            }
        }};
    }

    macro_rules! check_kinds_for_cmd {
        ($cmd:expr, $($name:ident, $ty:path, $kind:ident, $doc:literal);* $(;)?) => {
            $(
                check_kind!($cmd, $name, $ty, $kind, $doc);
            )*
        };
    }

    #[test]
    fn cli_workload_subcommands_match_registry_kinds() {
        let cmd = Cli::command();
        workloads!(cmd, check_kinds_for_cmd);
    }

    #[test]
    fn detach_args_include_foreground() {
        assert!(
            workloads::Litellm
                .detach_args()
                .contains(&"--foreground".to_string()),
            "litellm detach_args must contain --foreground"
        );
        assert!(
            workloads::Pi
                .detach_args()
                .contains(&"--foreground".to_string()),
            "pi detach_args must contain --foreground"
        );
    }
}
