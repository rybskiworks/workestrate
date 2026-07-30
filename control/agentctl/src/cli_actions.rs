//! Clap subcommand action enums shared between the `workestrate` binary and
//! the library command handlers.
//!
//! These enums describe the per-workload / per-domain action surfaces
//! (`ServiceAction`, `AgentAction`, `ConfigAction`, `ContextAction`,
//! `SourceAction`). They live in the library (not `main.rs`) because the
//! `commands::*` handlers pattern-match on them; `main.rs` re-exports them so
//! the clap parser tree continues to expose the same CLI surface.

use clap::Subcommand;

/// Actions available on service workloads (headless, detached by default).
#[derive(Subcommand)]
pub enum ServiceAction {
    /// Start the sandbox (detached by default; --foreground to block)
    Up {
        #[arg(short, long, help = "Run in foreground (block until Ctrl-C)")]
        foreground: bool,

        /// Tear down any existing instance at this slot before starting
        /// (destructive; the ADR 0021 explicit-replace escape hatch).
        #[arg(long)]
        replace: bool,

        /// Target a parallel instance `<slot>@<id>`. Refuses if that exact
        /// instance name is already running.
        #[arg(long, value_name = "ID")]
        instance: Option<String>,

        /// Auto-allocate the lowest free integer id >= 2 and target
        /// `<slot>@<id>`.
        #[arg(long)]
        new: bool,
    },
    /// Stop and remove the sandbox
    Down {
        /// Stop the parallel instance `<slot>@<id>`.
        #[arg(long, value_name = "ID")]
        instance: Option<String>,

        /// Stop the singleton AND every parallel instance of this workload.
        #[arg(long)]
        all_instances: bool,
    },
    /// Tail the detached service's log file
    Logs {
        /// Tail the parallel instance `<slot>@<id>` (default: the singleton).
        #[arg(long, value_name = "ID")]
        instance: Option<String>,
    },
    /// Print the planned sandbox workload
    Plan,
}

/// Actions available on agent workloads (interactive TUI attach).
#[derive(Subcommand)]
pub enum AgentAction {
    /// Attach to the sandbox interactively (TUI)
    Exec {
        /// Tear down any existing instance at this slot before starting.
        #[arg(long)]
        replace: bool,

        /// Target a parallel instance `<slot>@<id>`.
        #[arg(long, value_name = "ID")]
        instance: Option<String>,

        /// Auto-allocate the lowest free integer id >= 2.
        #[arg(long)]
        new: bool,
    },
    /// Stop and remove the sandbox
    Down {
        #[arg(long, value_name = "ID")]
        instance: Option<String>,
        #[arg(long)]
        all_instances: bool,
    },
    /// Print the planned sandbox workload
    Plan,
}

/// Actions for managing config repositories and trust.
#[derive(Subcommand)]
pub enum ConfigAction {
    /// Clone a config repo into the managed store and register it
    Add {
        url: String,
        name: String,
        #[arg(long, default_value = "main")]
        r#ref: String,
    },
    /// Pull latest for a config repo (or all) and update rev in registry
    Update { name: Option<String> },
    /// List registered config repos with rev + dirty status
    List,
    /// Trust a project directory for project-layer config loading
    Trust { dir: String },
    /// Remove trust from a project directory
    Untrust { dir: String },
    /// Scaffold a new config repo locally (minimal valid workestrate.toml,
    /// SOPS, README). Replaces the copier template for the minimal-personal
    /// subset; writes a `.copier-answers.yml` sidecar so `copier update` stays
    /// usable for richer features (team keys, flake).
    New {
        /// Name for the new config repo (e.g. "personal", "work").
        name: String,

        /// Destination directory (default: <store>/config-repos/<name>).
        #[arg(long, value_name = "DIR")]
        path: Option<std::path::PathBuf>,

        /// Age public recipient (age1...). If omitted, derived via
        /// `age-keygen -y` from `--age-key-file` (default
        /// `~/.config/sops/age/ai-workbench-secrets.txt`). Falls back to
        /// `age1PLACEHOLDER` + warning if derivation fails.
        #[arg(long, value_name = "KEY")]
        age_recipient: Option<String>,

        /// Override the age key file to derive the recipient from.
        #[arg(long, value_name = "PATH")]
        age_key_file: Option<std::path::PathBuf>,

        /// Include a flake.nix for inverted-dependency image builds
        /// (Phase 2).
        #[arg(long)]
        with_flake: bool,

        /// URL of the workestrate core flake (only used with --with-flake).
        #[arg(
            long,
            value_name = "URL",
            default_value = "github:georgrybski/ai-workbench"
        )]
        core_flake_url: String,

        /// Skip registering the new repo in the workestrate registry.
        #[arg(long)]
        no_register: bool,

        /// Skip `git init` in the new directory.
        #[arg(long)]
        no_git_init: bool,

        /// Seed workestrate.toml from config.reference/workestrate.toml
        /// (full 5-workload fixture). Conflicts with --empty.
        #[arg(long, conflicts_with = "empty")]
        from_reference: bool,

        /// Write only a minimal workestrate.toml (no secrets/sops/readme).
        /// Conflicts with --from-reference.
        #[arg(long, conflicts_with = "from_reference")]
        empty: bool,
    },
    /// Unregister a config repo from the registry
    Remove {
        /// Config repo name to remove.
        name: String,
        /// Also delete the store clone directory.
        #[arg(long)]
        delete: bool,
        /// Force deletion even if the clone is dirty (has uncommitted changes).
        #[arg(long)]
        force: bool,
    },
}

/// Actions for managing the workestrate tool home itself.
#[derive(Subcommand)]
pub enum HomeAction {
    /// Initialize the resolved tool home as a dotfiles-style git repo
    /// (git init + .gitignore + pre-commit hook). Idempotent.
    ///
    /// With `--from <src>` the home is PROVISIONED from an existing home
    /// (ADR 0025): the registry layer is cloned/copied, config repos are
    /// re-cloned or copied, and registry urls pointing into the source home
    /// are rewritten to the dest-local `config-repos/<name>` paths. A
    /// positional `<dest>` selects where the new home is created (default:
    /// the resolved tool home). `--config`/`--name` remain valid on the bare
    /// path only — combining them with `--from` or a positional dest is a
    /// usage error.
    Init {
        /// Clone and register this config-repo URL as config-repos/<name>
        /// after scaffolding the home (bare path only).
        #[arg(long, conflicts_with = "from", conflicts_with = "dest")]
        config: Option<String>,

        /// Name for the cloned config repo (only used with --config).
        #[arg(
            long,
            default_value = "personal",
            conflicts_with = "from",
            conflicts_with = "dest"
        )]
        name: String,

        /// Provision the home from this source (absolute path, relative path
        /// resolved against cwd, or git URL).
        #[arg(long, value_name = "SRC")]
        from: Option<String>,

        /// Destination directory for the new home (default: the resolved
        /// tool home).
        dest: Option<String>,
    },
}

/// Actions for managing workestrate contexts.
#[derive(Subcommand)]
pub enum ContextAction {
    /// List all defined contexts.
    List,
    /// Show the currently-resolved context and why it was selected.
    Current,
}

/// Actions for managing agent source checkouts.
#[derive(Subcommand)]
pub enum SourceAction {
    /// Clone agent source into the managed store
    Clone {
        name: String,
        /// Optional path (defaults to sources/<name>/repo/)
        path: Option<String>,
    },
    /// Build agent from source using the workload's local_build recipe
    Build { name: String },
    /// List agent source checkouts with status
    List,
    /// Reset agent source to canonical (discard local edits)
    Reset { name: String },
}
