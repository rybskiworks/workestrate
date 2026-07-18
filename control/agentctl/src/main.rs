use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use std::io::Write;
use std::path::PathBuf;

mod config;
mod merge;
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
    #[arg(long, help = "Disable project-layer config loading")]
    no_project_config: bool,

    #[arg(long, global = true, help = "Show source layer for each plan field")]
    show_source: bool,

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

/// Actions for managing config repositories and trust.
#[derive(Subcommand)]
enum ConfigAction {
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
}

/// Actions for managing agent source checkouts.
#[derive(Subcommand)]
enum SourceAction {
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
    /// Manage config repositories and trusted projects
    Config {
        #[command(subcommand)]
        action: ConfigAction,
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
    /// Catch-all for config-defined workloads
    #[command(external_subcommand)]
    Workload(Vec<String>),
}

async fn dispatch_service<W: Workload>(
    workload: &W,
    action: ServiceAction,
    show_source: bool,
) -> Result<()> {
    match action {
        ServiceAction::Up { foreground } => microsandbox::up_service(workload, foreground).await,
        ServiceAction::Down => microsandbox::down(workload.name()).await,
        ServiceAction::Logs => microsandbox::logs(workload.name()).await,
        ServiceAction::Plan => {
            if show_source {
                println!("{}", workload.show_source());
            } else {
                println!("{}", workload.plan());
            }
            Ok(())
        }
    }
}

async fn dispatch_agent<W: Workload>(
    workload: &W,
    action: AgentAction,
    show_source: bool,
) -> Result<()> {
    match action {
        AgentAction::Exec => microsandbox::exec_agent(workload).await,
        AgentAction::Down => microsandbox::down(workload.name()).await,
        AgentAction::Plan => {
            if show_source {
                println!("{}", workload.show_source());
            } else {
                println!("{}", workload.plan());
            }
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
    let config_dir = agent_dir.join("config");
    std::fs::create_dir_all(&config_dir)?;
    std::fs::write(config_dir.join(".gitkeep"), "")?;

    // Append a default workload entry to workestrate.toml
    let config_path = root.join("workestrate.toml");
    let toml_entry = format!(
        "\n[workloads.{}]\n\
        kind = \"agent\"\n\
        image = {{ recipe = \"registry\", ref = \"node:24-bookworm-slim\" }}\n\
        workdir = \"/work\"\n\
        cpus = 2\n\
        memory_mib = 2048\n\
        command = []\n\
        log_stop_errors = false\n\n\
        [[workloads.{}.mounts]]\n\
        host = \"${{CWD}}\"\n\
        guest = \"/work\"\n\
        read_only = false\n\n\
        [workloads.{}.network]\n\
        default_deny = true\n\n\
        [[workloads.{}.network.egress]]\n\
        recipe = \"agent_base\"\n",
        name, name, name, name
    );

    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&config_path)?;
    file.write_all(toml_entry.as_bytes())?;

    println!("Created agents/{}/config/", name);
    println!(
        "Appended [workloads.{}] to workestrate.toml with defaults.",
        name
    );
    println!();
    println!("Edit workestrate.toml to configure:");
    println!("  - Set image (recipe + ref, or recipe + contents for nix-layered)");
    println!("  - Set command");
    println!("  - Add env/secret_env/mounts as needed");
    println!();
    println!("Test: workestrate {} plan", name);

    Ok(())
}

// ---------------------------------------------------------------------------
// Git helpers
// ---------------------------------------------------------------------------

fn git_clone(url: &str, dest: &std::path::Path, branch: Option<&str>) -> Result<()> {
    let mut cmd = std::process::Command::new("git");
    cmd.args(["clone", "--depth", "1"]);
    if let Some(branch) = branch {
        cmd.args(["--branch", branch]);
    }
    let status = cmd.arg(url).arg(dest).status()?;
    if !status.success() {
        anyhow::bail!("git clone failed for {}", url);
    }
    Ok(())
}

fn git_rev_parse(repo: &std::path::Path) -> Result<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !output.status.success() {
        anyhow::bail!("git rev-parse failed for {}", repo.display());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn git_is_dirty(repo: &std::path::Path) -> Result<bool> {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["diff", "--quiet", "HEAD"])
        .status()?;
    Ok(!status.success())
}

fn git_pull(repo: &std::path::Path, branch: &str) -> Result<()> {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["pull", "origin", branch])
        .status()?;
    if !status.success() {
        anyhow::bail!("git pull failed for {}", repo.display());
    }
    Ok(())
}

fn git_checkout_dot(repo: &std::path::Path) -> Result<()> {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["checkout", "."])
        .status()?;
    if !status.success() {
        anyhow::bail!("git checkout failed for {}", repo.display());
    }
    Ok(())
}

fn short_rev(rev: &str) -> String {
    rev.chars().take(7).collect()
}

// ---------------------------------------------------------------------------
// Config commands
// ---------------------------------------------------------------------------

async fn cmd_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Add { url, name, r#ref } => cmd_config_add(&url, &name, &r#ref).await,
        ConfigAction::Update { name } => cmd_config_update(name.as_deref()).await,
        ConfigAction::List => cmd_config_list().await,
        ConfigAction::Trust { dir } => cmd_config_trust(&dir).await,
        ConfigAction::Untrust { dir } => cmd_config_untrust(&dir).await,
    }
}

async fn cmd_config_add(url: &str, name: &str, git_ref: &str) -> Result<()> {
    let dest = config::config_repo_dir(name);
    let (rev, short) = if dest.exists() {
        let git_dir = dest.join(".git");
        if !git_dir.exists() {
            anyhow::bail!(
                "config repo destination '{}' already exists and is not a git repo",
                dest.display()
            );
        }
        let rev = git_rev_parse(&dest)?;
        let short = short_rev(&rev);
        (rev, short)
    } else {
        let parent = dest
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid repo path: {}", dest.display()))?;
        std::fs::create_dir_all(parent)?;
        git_clone(url, &dest, Some(git_ref))?;
        let rev = git_rev_parse(&dest)?;
        let short = short_rev(&rev);
        (rev, short)
    };

    let mut registry = config::load_registry()?.unwrap_or_default();
    registry.configs.insert(
        name.to_string(),
        config::ConfigRepoEntry {
            url: url.to_string(),
            r#ref: Some(git_ref.to_string()),
            rev: Some(rev),
        },
    );
    if registry.layers.is_empty() {
        registry.layers.push(name.to_string());
    }
    config::save_registry(&registry)?;
    println!(
        "Registered config repo {} from {} at {} (rev {})",
        name,
        url,
        dest.display(),
        short
    );
    Ok(())
}

async fn cmd_config_update(name: Option<&str>) -> Result<()> {
    let mut registry =
        config::load_registry()?.ok_or_else(|| anyhow::anyhow!("no config repos registered"))?;
    let names: Vec<String> = match name {
        Some(n) => {
            if !registry.configs.contains_key(n) {
                anyhow::bail!("config repo '{}' not found", n);
            }
            vec![n.to_string()]
        }
        None => registry.configs.keys().cloned().collect(),
    };

    for n in names {
        let dest = config::config_repo_dir(&n);
        if git_is_dirty(&dest)? {
            anyhow::bail!(
                "config repo '{}' has uncommitted changes; commit or stash first",
                n
            );
        }
        let git_ref = registry
            .configs
            .get(&n)
            .and_then(|e| e.r#ref.as_deref())
            .unwrap_or("main")
            .to_string();
        git_pull(&dest, &git_ref)?;
        let rev = git_rev_parse(&dest)?;
        let short = short_rev(&rev);
        registry
            .configs
            .get_mut(&n)
            .ok_or_else(|| anyhow::anyhow!("config repo '{}' disappeared", n))?
            .rev = Some(rev);
        println!("{}: updated to {}", n, short);
    }
    config::save_registry(&registry)?;
    Ok(())
}

async fn cmd_config_list() -> Result<()> {
    match config::load_registry()? {
        None => {
            println!("(no registry found; run 'workestrate init' or 'workestrate config add <url> <name>')");
        }
        Some(registry) => {
            println!("Config repos:");
            if registry.configs.is_empty() {
                println!("  (none)");
            } else {
                for (name, entry) in &registry.configs {
                    let dest = config::config_repo_dir(name);
                    let (dirty_label, ok) = if dest.exists() {
                        match git_is_dirty(&dest) {
                            Ok(false) => ("clean", true),
                            Ok(true) => ("dirty", false),
                            Err(_) => ("unknown", false),
                        }
                    } else {
                        ("missing", false)
                    };
                    let rev = entry.rev.as_deref().unwrap_or("unknown");
                    let short = short_rev(rev);
                    let git_ref = entry.r#ref.as_deref().unwrap_or("main");
                    let status = if ok { "[OK]" } else { "[MISSING]" };
                    println!(
                        "  {}: {} (ref {}, rev {}, {}) {}",
                        name, entry.url, git_ref, short, dirty_label, status
                    );
                }
            }
            println!("Layers: {:?}", registry.layers);
            println!("Trusted projects:");
            if registry.trusted_projects.is_empty() {
                println!("  (none)");
            } else {
                for p in &registry.trusted_projects {
                    let path = PathBuf::from(&p.path);
                    let status = if path.exists() { "[OK]" } else { "[MISSING]" };
                    println!("  {} {}", p.path, status);
                }
            }
        }
    }
    Ok(())
}

async fn cmd_config_trust(dir: &str) -> Result<()> {
    let path = PathBuf::from(dir);
    config::trust_project(&path)?;
    println!("Trusted: {}", path.display());
    Ok(())
}

async fn cmd_config_untrust(dir: &str) -> Result<()> {
    let path = PathBuf::from(dir);
    config::untrust_project(&path)?;
    println!("Untrusted: {}", path.display());
    Ok(())
}

// ---------------------------------------------------------------------------
// Init command
// ---------------------------------------------------------------------------

async fn cmd_init(url: Option<&str>) -> Result<()> {
    let registry_path = config::registry_path();
    if registry_path.exists() {
        println!(
            "Registry already exists at {}; use 'workestrate config add' to add repos",
            registry_path.display()
        );
        return Ok(());
    }

    let mut registry = config::Registry::default();
    registry.settings.default_context = Some("personal".to_string());

    std::fs::create_dir_all(config::xdg_data_dir())?;
    std::fs::create_dir_all(config::xdg_state_dir())?;

    if let Some(url) = url {
        let temp_dir =
            std::env::temp_dir().join(format!("workestrate-init-{}", std::process::id()));
        git_clone(url, &temp_dir, None)?;

        let found = if temp_dir.join("workestrate").join("config.toml").exists() {
            Some(temp_dir.join("workestrate").join("config.toml"))
        } else if temp_dir
            .join(".config")
            .join("workestrate")
            .join("config.toml")
            .exists()
        {
            Some(
                temp_dir
                    .join(".config")
                    .join("workestrate")
                    .join("config.toml"),
            )
        } else {
            None
        };

        let registry_parent = registry_path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid registry path: {}", registry_path.display()))?;
        std::fs::create_dir_all(registry_parent)?;

        match found {
            Some(src) => {
                std::fs::copy(&src, &registry_path)?;
                println!(
                    "Cloned {} and copied workestrate config to {}",
                    url,
                    registry_path.display()
                );
            }
            None => {
                println!(
                    "Cloned {} but no workestrate config found; created empty registry",
                    url
                );
                config::save_registry(&registry)?;
            }
        }
        let _ = std::fs::remove_dir_all(&temp_dir);
    } else {
        config::save_registry(&registry)?;
    }

    println!(
        "Initialized workestrate registry at {}",
        registry_path.display()
    );
    println!("Run 'workestrate config add <url> personal' to add your config repo");
    Ok(())
}

// ---------------------------------------------------------------------------
// Source commands
// ---------------------------------------------------------------------------

async fn cmd_source(action: SourceAction) -> Result<()> {
    match action {
        SourceAction::Clone { name, path } => cmd_source_clone(&name, path.as_deref()).await,
        SourceAction::Build { name } => cmd_source_build(&name).await,
        SourceAction::List => cmd_source_list().await,
        SourceAction::Reset { name } => cmd_source_reset(&name).await,
    }
}

async fn cmd_source_clone(name: &str, path: Option<&str>) -> Result<()> {
    let cfg = config::load_config()?;
    let workload = cfg
        .workloads
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("workload '{}' not found", name))?;
    let local_build = workload
        .local_build
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("workload '{}' has no local_build recipe", name))?;

    let dest = match path {
        Some(p) => PathBuf::from(p),
        None => config::source_store_dir(name).join("repo"),
    };

    if local_build.source.starts_with("flake://") {
        let input = local_build
            .source
            .strip_prefix("flake://")
            .unwrap_or(&local_build.source);
        println!(
            "To clone the canonical source, run: nix develop (materializes flake inputs). Or clone manually to: {}",
            dest.display()
        );
        println!("Flake input name: {}", input);
    } else {
        if dest.exists() {
            anyhow::bail!("source path already exists: {}", dest.display());
        }
        let parent = dest
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid source path: {}", dest.display()))?;
        std::fs::create_dir_all(parent)?;
        git_clone(&local_build.source, &dest, None)?;
        println!("Cloned {} source to {}", name, dest.display());
    }

    let env_var = format!("WORKESTRATE_{}_BUILD", name.to_uppercase());
    println!("Set {}={} for this session", env_var, dest.display());
    Ok(())
}

fn build_command_string(name: &str, local_build: &config::LocalBuildConfig) -> String {
    match local_build.recipe.as_str() {
        "pip-install" | "pip" => {
            let target = local_build.target.as_deref().unwrap_or(".deps");
            let req = local_build
                .requirements_file
                .as_deref()
                .unwrap_or("requirements.txt");
            format!("pip install --target {} -r {}", target, req)
        }
        "npm-build" | "npm" => "npm ci && npm run build".to_string(),
        "bun-install" | "bun" => "bun install && bun run build".to_string(),
        other => format!("{} build recipe for {}", other, name),
    }
}

async fn cmd_source_build(name: &str) -> Result<()> {
    let cfg = config::load_config()?;
    let workload = cfg
        .workloads
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("workload '{}' not found", name))?;
    let local_build = workload
        .local_build
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("workload '{}' has no local_build recipe", name))?;

    let command = build_command_string(name, local_build);
    println!("Build command for {}: {}", name, command);
    println!("Run inside nix develop: {}", command);
    Ok(())
}

async fn cmd_source_list() -> Result<()> {
    let cfg = config::load_config()?;
    println!("Source overrides:");
    let mut found = false;
    for (name, workload) in &cfg.workloads {
        if workload.local_build.is_none() {
            continue;
        }
        found = true;
        let env_var = format!("WORKESTRATE_{}_BUILD", name.to_uppercase());
        let repo = config::source_store_dir(name).join("repo");
        let build = config::source_store_dir(name).join("build");
        let repo_status = if repo.exists() {
            "checked out"
        } else {
            "not checked out"
        };
        let build_status = if build.exists() { "built" } else { "not built" };
        match std::env::var(&env_var) {
            Ok(path) => println!(
                "  {}: override {} (repo: {}, build: {})",
                name, path, repo_status, build_status
            ),
            Err(_) => println!(
                "  {}: no override ({} not set) (repo: {}, build: {})",
                name, env_var, repo_status, build_status
            ),
        }
    }
    if !found {
        println!("  (none)");
    }
    Ok(())
}

async fn cmd_source_reset(name: &str) -> Result<()> {
    let repo = config::source_store_dir(name).join("repo");
    if !repo.exists() {
        anyhow::bail!(
            "source for '{}' not checked out at {}",
            name,
            repo.display()
        );
    }
    git_checkout_dot(&repo)?;
    println!("Reset {} source to canonical", name);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.no_project_config {
        std::env::set_var("WORKESTRATE_NO_PROJECT_CONFIG", "1");
    }

    match cli.command {
        Commands::Check => cmd_check().await,
        Commands::Init { url } => cmd_init(url.as_deref()).await,
        Commands::New { name } => cmd_new(&name).await,
        Commands::Completions { shell, for_name } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, &for_name, &mut std::io::stdout());
            Ok(())
        }
        Commands::Run { command } => cmd_run(&command).await,
        Commands::ValidateConfig => cmd_validate_config().await,
        Commands::SecretsSchema => cmd_secrets_schema().await,
        Commands::GenerateEnvExample { output } => {
            cmd_generate_env_example(output.as_deref()).await
        }
        Commands::Config { action } => cmd_config(action).await,
        Commands::Source { action } => cmd_source(action).await,
        Commands::Litellm { action } => {
            let workload = ConfigWorkload::new("litellm")?;
            dispatch_service(&workload, action, cli.show_source).await
        }
        Commands::Pi { action } => {
            let workload = ConfigWorkload::new("pi")?;
            dispatch_agent(&workload, action, cli.show_source).await
        }
        Commands::Odysseus { action } => {
            let workload = ConfigWorkload::new("odysseus")?;
            dispatch_service(&workload, action, cli.show_source).await
        }
        Commands::Opencode { action } => {
            let workload = ConfigWorkload::new("opencode")?;
            dispatch_agent(&workload, action, cli.show_source).await
        }
        Commands::Tempest { action } => {
            let workload = ConfigWorkload::new("tempest")?;
            dispatch_agent(&workload, action, cli.show_source).await
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
                    dispatch_service(&workload, service_action, cli.show_source).await
                }
                "agent" => {
                    let agent_action = parse_agent_action(&action)?;
                    dispatch_agent(&workload, agent_action, cli.show_source).await
                }
                other => anyhow::bail!("unknown workload kind '{}' for '{}'", other, name),
            }
        }
    }
}

async fn cmd_check() -> Result<()> {
    println!("=== workestrate check ===\n");
    let mut all_ok = true;

    let registry_path = config::registry_path();
    if registry_path.exists() {
        println!("Registry: {} [OK]", registry_path.display());
        match config::load_registry()? {
            None => {
                println!("  (registry exists but could not be loaded)");
                all_ok = false;
            }
            Some(registry) => {
                println!("  Config repos:");
                if registry.configs.is_empty() {
                    println!("    (none)");
                } else {
                    for (name, entry) in &registry.configs {
                        let dest = config::config_repo_dir(name);
                        let (dirty_label, ok) = if dest.exists() {
                            match git_is_dirty(&dest) {
                                Ok(false) => ("clean", true),
                                Ok(true) => ("dirty", false),
                                Err(_) => ("unknown", false),
                            }
                        } else {
                            ("missing", false)
                        };
                        let rev = entry.rev.as_deref().unwrap_or("unknown");
                        let short = short_rev(rev);
                        let git_ref = entry.r#ref.as_deref().unwrap_or("main");
                        let status = if ok { "[OK]" } else { "[MISSING]" };
                        println!(
                            "    {}: {} (ref {}, rev {}, {}) {}",
                            name, entry.url, git_ref, short, dirty_label, status
                        );
                    }
                }
                println!("  Layers: {:?}", registry.layers);
                println!("  Trusted projects:");
                if registry.trusted_projects.is_empty() {
                    println!("    (none)");
                } else {
                    for p in &registry.trusted_projects {
                        let path = PathBuf::from(&p.path);
                        let status = if path.exists() { "[OK]" } else { "[MISSING]" };
                        println!("    {} {}", p.path, status);
                    }
                }
            }
        }
    } else {
        println!("Registry: {} [MISSING]", registry_path.display());
        all_ok = false;
    }

    println!("\nXDG dirs:");
    for (label, dir) in [
        ("config", config::xdg_config_dir()),
        ("data", config::xdg_data_dir()),
        ("state", config::xdg_state_dir()),
    ] {
        let ok = dir.exists();
        let status = if ok { "[OK]" } else { "[MISSING]" };
        println!("  {}: {} {}", label, dir.display(), status);
        if !ok {
            all_ok = false;
        }
    }

    println!("\nSource overrides:");
    match config::load_config() {
        Ok(cfg) => {
            let mut found = false;
            for (name, workload) in &cfg.workloads {
                if workload.local_build.is_none() {
                    continue;
                }
                found = true;
                let env_var = format!("WORKESTRATE_{}_BUILD", name.to_uppercase());
                let repo = config::source_store_dir(name).join("repo");
                let build = config::source_store_dir(name).join("build");
                let repo_status = if repo.exists() {
                    "checked out"
                } else {
                    "not checked out"
                };
                let build_status = if build.exists() { "built" } else { "not built" };
                match std::env::var(&env_var) {
                    Ok(path) => println!(
                        "  {}: override {} (repo: {}, build: {})",
                        name, path, repo_status, build_status
                    ),
                    Err(_) => println!(
                        "  {}: no override ({} not set) (repo: {}, build: {})",
                        name, env_var, repo_status, build_status
                    ),
                }
            }
            if !found {
                println!("  (none)");
            }
        }
        Err(e) => {
            println!("  (could not load config: {})", e);
            all_ok = false;
        }
    }

    println!("\nReference config:");
    let reference = find_reference_config();
    match reference {
        Some(path) if path.exists() => println!("  {} [OK]", path.display()),
        Some(path) => {
            println!("  {} [MISSING]", path.display());
            all_ok = false;
        }
        None => {
            println!("  (could not resolve reference config)");
            all_ok = false;
        }
    }

    println!("\nRequired files:");
    if let Ok(root) = config::project_root() {
        let checks = config::check_required_files(&root)?;
        let mut had_missing_required = false;
        for entry in &checks {
            print_entry(entry);
            if !entry.ok && !entry.optional {
                had_missing_required = true;
            }
        }
        if had_missing_required {
            all_ok = false;
        }
    } else {
        println!("  (could not resolve project root)");
        all_ok = false;
    }

    if all_ok {
        println!("\nAll checks passed.");
        Ok(())
    } else {
        Err(anyhow::anyhow!("Some checks failed."))
    }
}

fn find_reference_config() -> Option<PathBuf> {
    if let Ok(root) = config::project_root() {
        let path = root.join("config.reference").join("workestrate.toml");
        if path.exists() {
            return Some(path);
        }
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut path = PathBuf::from(manifest);
        if path.pop() && path.pop() {
            let reference = path.join("config.reference").join("workestrate.toml");
            if reference.exists() {
                return Some(reference);
            }
        }
    }
    None
}

async fn cmd_validate_config() -> Result<()> {
    let config = config::load_config()?;
    config::validate_config(&config)?;
    println!("workestrate.toml is valid.");
    Ok(())
}

async fn cmd_secrets_schema() -> Result<()> {
    let config = config::load_config()?;
    let mut names: Vec<&str> = config
        .secrets
        .values()
        .filter_map(|s| s.env_var.as_deref())
        .collect();
    names.sort();
    for name in names {
        println!("{}", name);
    }
    Ok(())
}

async fn cmd_generate_env_example(output: Option<&std::path::Path>) -> Result<()> {
    let config = config::load_config()?;
    let mut entries: Vec<(&str, &str)> = config
        .secrets
        .values()
        .filter_map(|s| {
            let env_var = s.env_var.as_deref()?;
            let description = s.description.as_deref().unwrap_or("");
            Some((env_var, description))
        })
        .collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));

    let mut buf = String::new();
    buf.push_str("# ai-workbench environment schema.\n");
    buf.push_str("# This file is committed and safe to share.\n");
    buf.push_str(
        "# Real secrets live in .env.enc (encrypted) and are loaded by workestrate at runtime.\n",
    );
    for (env_var, description) in entries {
        if !description.is_empty() {
            buf.push_str(&format!("\n# {}\n", description));
        } else {
            buf.push('\n');
        }
        buf.push_str(&format!("{}=\n", env_var));
    }
    buf.push_str("\n# Optional local paths\n");
    buf.push_str("AI_WORKBENCH_WORKSPACES_DIR=workspaces\n");
    buf.push_str("AI_WORKBENCH_VAR_DIR=var\n");

    match output {
        Some(path) => {
            std::fs::write(path, &buf)?;
        }
        None => {
            print!("{}", buf);
        }
    }
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
            "init",
            "new",
            "completions",
            "run",
            "validate-config",
            "secrets-schema",
            "generate-env-example",
            "config",
            "source",
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
