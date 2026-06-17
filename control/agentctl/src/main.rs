use anyhow::Result;
use clap::{Parser, Subcommand};

mod agents;
mod config;
mod litellm;
mod microsandbox;

use agents::AgentName;
use config::CheckEntry;

#[derive(Parser)]
#[command(name = "agentctl")]
#[command(about = "Control plane CLI for the AI workbench")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Runtime/config sanity check
    Check,
    /// LiteLLM sandbox commands
    Litellm {
        #[command(subcommand)]
        action: LitellmAction,
    },
    /// Agent sandbox commands
    Agent {
        #[command(subcommand)]
        action: AgentAction,
    },
}

#[derive(Subcommand)]
enum LitellmAction {
    /// Print the planned LiteLLM sandbox workload
    Plan,
    /// Start the LiteLLM sandbox
    Up,
    /// Stop and remove the LiteLLM sandbox
    Down,
}

#[derive(Subcommand)]
enum AgentAction {
    /// Print the planned agent sandbox workload
    Plan { name: AgentName },
    /// Start the agent sandbox
    Up { name: AgentName },
    /// Stop and remove the agent sandbox
    Down { name: AgentName },
}

fn print_entry(entry: &CheckEntry) {
    if entry.ok {
        println!("[OK] {}", entry.label);
        return;
    }
    if entry.optional {
        // Optional checks (e.g. local `agents/pi` and `agents/odysseus`
        // checkouts) are reported as warnings and never cause a non-zero
        // exit. These directories are documented as optional local
        // overrides; the agents are normally supplied via flake inputs.
        println!("[MISSING] (optional) {}", entry.label);
    } else {
        println!("[MISSING] {}", entry.label);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check => cmd_check().await,
        Commands::Litellm { action } => match action {
            LitellmAction::Plan => litellm::plan().await,
            LitellmAction::Up => litellm::up().await,
            LitellmAction::Down => litellm::down().await,
        },
        Commands::Agent { action } => match action {
            AgentAction::Plan { name } => agents::plan(name).await,
            AgentAction::Up { name } => agents::up(name).await,
            AgentAction::Down { name } => agents::down(name).await,
        },
    }
}

async fn cmd_check() -> Result<()> {
    let root = config::project_root()?;
    let checks = config::check_required_files(&root)?;

    // Only non-optional missing artifacts cause a non-zero exit. Optional
    // agent checkouts (agents/pi, agents/odysseus) are reported as warnings.
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
