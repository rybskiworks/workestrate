//! `agentctl` — control plane CLI for the AI workestrator.
//!
//! See ../../README.md for the project description and ../../CLAUDE.md
//! (if present) for the agent-development rules.
//!
//! The first three commands fully implemented are the ones called out
//! as the first milestone of the plan:
//!
//!   * `agentctl init`                   — create the on-disk layout
//!   * `agentctl providers check`        — validate .env contents
//!   * `agentctl litellm print-plan`     — describe the LiteLLM sandbox
//!
//! Other subcommands are present as stubs that print "deferred" and
//! exit 0, so the CLI surface matches the plan without claiming
//! behavior that isn't implemented.

use clap::{Parser, Subcommand};

mod config;
mod envfile;
mod init;
mod providers;
mod litellm;
mod postgres;
mod sandbox;
mod agents;

#[derive(Parser, Debug)]
#[command(
    name = "agentctl",
    version,
    about = "Control plane CLI for the AI workestrator",
    long_about = None,
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Create the on-disk layout (agents/, workspaces/, var/, tmp/).
    Init,

    /// Inspect and validate provider configuration.
    Providers {
        #[command(subcommand)]
        action: ProvidersAction,
    },

    /// LiteLLM subcommands.
    Litellm {
        #[command(subcommand)]
        action: LitellmAction,
    },

    /// Postgres-backed key/team storage (deferred).
    Postgres {
        #[command(subcommand)]
        action: PostgresAction,
    },

    /// Sandbox driver (deferred; see `agentctl litellm print-plan`).
    Sandbox {
        #[command(subcommand)]
        action: SandboxAction,
    },

    /// Agent definitions.
    Agents {
        #[command(subcommand)]
        action: AgentsAction,
    },
}

#[derive(Subcommand, Debug)]
enum ProvidersAction {
    /// Validate the .env contents against the expected provider shape.
    Check,
}

#[derive(Subcommand, Debug)]
enum LitellmAction {
    /// Print the planned LiteLLM sandbox (image, config, port, env in,
    /// env out, egress allowlist). Does not start LiteLLM.
    PrintPlan,
}

#[derive(Subcommand, Debug)]
enum PostgresAction {
    /// Print status of the postgres_16 instance.
    Status,
}

#[derive(Subcommand, Debug)]
enum SandboxAction {
    /// Print the planned sandbox driver configuration.
    Plan,
}

#[derive(Subcommand, Debug)]
enum AgentsAction {
    /// List the agent definitions in agents/.
    List,
}

fn main() {
    let cli = Cli::parse();
    let result: Result<u8, String> = match cli.command {
        Commands::Init => init::run(),
        Commands::Providers { action } => match action {
            ProvidersAction::Check => providers::check(),
        },
        Commands::Litellm { action } => match action {
            LitellmAction::PrintPlan => litellm::print_plan(),
        },
        Commands::Postgres { action } => match action {
            PostgresAction::Status => postgres::status(),
        },
        Commands::Sandbox { action } => match action {
            SandboxAction::Plan => sandbox::plan(),
        },
        Commands::Agents { action } => match action {
            AgentsAction::List => agents::list(),
        },
    };
    match result {
        Ok(0) => {}
        Ok(code) => std::process::exit(code as i32),
        Err(e) => {
            eprintln!("agentctl: error: {e}");
            std::process::exit(2);
        }
    }
}
