use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};

mod config;
mod microsandbox;
mod workloads;

use config::CheckEntry;
use microsandbox::workload::Workload;

#[derive(Parser)]
#[command(name = "agentctl")]
#[command(about = "Control plane CLI for the AI workbench")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Actions available on every workload.
#[derive(Subcommand)]
enum WorkloadAction {
    /// Start the sandbox
    Up {
        #[arg(short, long, help = "Start detached in background")]
        background: bool,
    },
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
            Litellm, workloads::Litellm, "LiteLLM proxy sandbox";
            Pi, workloads::Pi, "Pi coding agent sandbox";
            Odysseus, workloads::Odysseus, "Odysseus agent sandbox";
            Opencode, workloads::Opencode, "OpenCode agent sandbox";
        );
    };
}

macro_rules! define_commands_enum {
    ($($name:ident, $ty:path, $doc:literal);* $(;)?) => {
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
            },
            $(
                #[doc = $doc]
                $name {
                    #[command(subcommand)]
                    action: WorkloadAction,
                },
            )*
        }
    };
}

workloads!(define_commands_enum);

async fn run<W: Workload>(workload: &W, action: WorkloadAction) -> Result<()> {
    match action {
        WorkloadAction::Up { background } => microsandbox::up(workload, background).await,
        WorkloadAction::Down => microsandbox::down(workload.name()).await,
        WorkloadAction::Plan => {
            println!("{}", workload.plan());
            Ok(())
        }
    }
}

macro_rules! define_dispatch {
    ($($name:ident, $ty:path, $doc:literal);* $(;)?) => {
        async fn dispatch_workload(command: Commands) -> Result<()> {
            match command {
                $(
                    Commands::$name { action } => run(&$ty, action).await,
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
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "agentctl", &mut std::io::stdout());
            Ok(())
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_exposes_expected_subcommands() {
        let cmd = Cli::command();
        let names: Vec<_> = cmd.get_subcommands().map(|s| s.get_name()).collect();
        for expected in [
            "check",
            "new",
            "completions",
            "litellm",
            "pi",
            "odysseus",
            "opencode",
        ] {
            assert!(names.contains(&expected), "missing subcommand: {expected}");
        }
    }
}
