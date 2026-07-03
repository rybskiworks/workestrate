use anyhow::Result;
use clap::{Parser, Subcommand};

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

#[derive(Subcommand)]
enum Commands {
    /// Runtime/config sanity check
    Check,
    /// Scaffold a new agent project
    New {
        /// Name for the new agent (e.g., "my-agent")
        name: String,
    },
    /// LiteLLM proxy sandbox
    Litellm {
        #[command(subcommand)]
        action: WorkloadAction,
    },
    /// Pi coding agent sandbox
    Pi {
        #[command(subcommand)]
        action: WorkloadAction,
    },
    /// Odysseus agent sandbox
    Odysseus {
        #[command(subcommand)]
        action: WorkloadAction,
    },
    /// OpenCode agent sandbox
    Opencode {
        #[command(subcommand)]
        action: WorkloadAction,
    },
}

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
    println!("     c. Add a Commands variant in main.rs");
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
        Commands::Litellm { action } => run(&workloads::Litellm, action).await,
        Commands::Pi { action } => run(&workloads::Pi, action).await,
        Commands::Odysseus { action } => run(&workloads::Odysseus, action).await,
        Commands::Opencode { action } => run(&workloads::Opencode, action).await,
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
