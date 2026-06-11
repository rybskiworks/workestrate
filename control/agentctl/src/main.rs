use anyhow::Result;
use clap::{Parser, Subcommand};

mod agents;
mod config;
mod litellm;
mod microsandbox;

use agents::AgentName;

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
}

#[derive(Subcommand)]
enum AgentAction {
    /// Print the planned agent sandbox workload
    Plan { name: AgentName },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check => cmd_check().await,
        Commands::Litellm { action } => match action {
            LitellmAction::Plan => litellm::plan().await,
        },
        Commands::Agent { action } => match action {
            AgentAction::Plan { name } => agents::plan(name).await,
        },
    }
}

async fn cmd_check() -> Result<()> {
    let root = config::project_root()?;
    let checks = config::check_required_files(&root)?;
    let mut all_ok = true;

    for (label, ok) in checks {
        let status = if ok { "[OK]" } else { "[MISSING]" };
        println!("{} {}", status, label);
        if !ok {
            all_ok = false;
        }
    }

    if all_ok {
        println!("\nAll checks passed.");
    } else {
        println!("\nSome checks failed.");
    }

    Ok(())
}
