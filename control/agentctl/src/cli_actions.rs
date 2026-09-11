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

        /// Publish each port on a lock-probed free port on the slot's bind
        /// (the chosen ports are recorded in the instance record).
        #[arg(long)]
        port_auto: bool,

        /// Instance-selection override `<dep>@<instance>` for depends_on
        /// resolution (ADR 0026(d)). PURE selection: discovery is activated
        /// by the depends_on declaration alone — `--use` only picks WHICH
        /// running instance of a declared dependency supplies the injected
        /// address. Repeatable (one per dep).
        #[arg(long = "use", value_name = "DEP@INSTANCE")]
        use_: Vec<String>,

        /// Do not auto-start declared depends_on dependencies (ADR 0026
        /// addendum); required deps then refuse-with-remediation at plan
        /// time, optional deps fall back per convention + warn.
        #[arg(long)]
        no_deps: bool,

        /// Re-render `template = true` seed_files over their EXISTING
        /// targets (bypasses `only_if_missing` for those entries). Static
        /// (non-template) seeds and `only_if_missing = false` behavior are
        /// unchanged. Forwarded to the detached child via `detach_args`.
        #[arg(long)]
        reseed: bool,

        /// Force the ensure-images pre-flight to rebuild+load+record
        /// nix-layered images even when the skew matrix would skip or trust
        /// (spec 21 §5.2). Parent-side only — NEVER forwarded to the
        /// detached child (USER DECISION D3).
        #[arg(long)]
        reload_images: bool,

        /// Hidden detach token (spec 21 §2.2): the parent already ran the
        /// ensure-images pre-flight, so this process (the detached child)
        /// skips it. Appended unconditionally by `detach_args` — same shape
        /// as `--foreground`.
        #[arg(long, hide = true)]
        images_ready: bool,
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
    Plan {
        /// Render the plan as parallel slot <slot>@<id> would see it
        /// (prospective per-instance bind).
        #[arg(long, value_name = "ID")]
        instance: Option<String>,

        /// Instance-selection override `<dep>@<instance>` for depends_on
        /// resolution (ADR 0026(d)). PURE selection: discovery is activated
        /// by the depends_on declaration alone — `--use` only picks WHICH
        /// running instance of a declared dependency supplies the injected
        /// address. Repeatable (one per dep).
        #[arg(long = "use", value_name = "DEP@INSTANCE")]
        use_: Vec<String>,
    },
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

        /// Publish each port on a lock-probed free port on the slot's bind
        /// (the chosen ports are recorded in the instance record).
        #[arg(long)]
        port_auto: bool,

        /// Instance-selection override `<dep>@<instance>` for depends_on
        /// resolution (ADR 0026(d)). PURE selection: discovery is activated
        /// by the depends_on declaration alone — `--use` only picks WHICH
        /// running instance of a declared dependency supplies the injected
        /// address. Repeatable (one per dep).
        #[arg(long = "use", value_name = "DEP@INSTANCE")]
        use_: Vec<String>,

        /// Do not auto-start declared depends_on dependencies (ADR 0026
        /// addendum); required deps then refuse-with-remediation at plan
        /// time, optional deps fall back per convention + warn.
        #[arg(long)]
        no_deps: bool,

        /// Re-render `template = true` seed_files over their EXISTING
        /// targets (bypasses `only_if_missing` for those entries). Static
        /// (non-template) seeds and `only_if_missing = false` behavior are
        /// unchanged.
        #[arg(long)]
        reseed: bool,

        /// Force the ensure-images pre-flight to rebuild+load+record
        /// nix-layered images even when the skew matrix would skip or trust
        /// (spec 21 §5.2). Parent-side only (agents are always foreground).
        #[arg(long)]
        reload_images: bool,
    },
    /// Stop and remove the sandbox
    Down {
        #[arg(long, value_name = "ID")]
        instance: Option<String>,
        #[arg(long)]
        all_instances: bool,
    },
    /// Print the planned sandbox workload
    Plan {
        /// Render the plan as parallel slot <slot>@<id> would see it
        /// (prospective per-instance bind).
        #[arg(long, value_name = "ID")]
        instance: Option<String>,

        /// Instance-selection override `<dep>@<instance>` for depends_on
        /// resolution (ADR 0026(d)). PURE selection: discovery is activated
        /// by the depends_on declaration alone — `--use` only picks WHICH
        /// running instance of a declared dependency supplies the injected
        /// address. Repeatable (one per dep).
        #[arg(long = "use", value_name = "DEP@INSTANCE")]
        use_: Vec<String>,
    },
}

/// Verb-first workload dispatch (ADR 0027): `workestrate workload
/// {up,exec,plan,down,logs,new} <name>`. The workload name is a positional
/// ARGUMENT (never a subcommand), so config-defined workloads can never be
/// shadowed by built-in verbs. The `name`-first flag set mirrors
/// [`ServiceAction`]/[`AgentAction`] exactly; `main.rs` kind-checks at
/// dispatch and translates into the legacy per-kind action enums.
#[derive(Subcommand)]
pub enum WorkloadAction {
    /// Start a service workload (detached by default; --foreground to block).
    /// Omit the name to start ALL service-kind workloads in the active
    /// context (ADR 0021 addendum 2026-08-01).
    Up {
        /// Workload name from the merged config. Omit for the batch form:
        /// topo-ordered start of every service-kind workload in the active
        /// context (agent-kind workloads are printed as SKIPPED).
        name: Option<String>,

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

        /// Publish each port on a lock-probed free port on the slot's bind
        /// (the chosen ports are recorded in the instance record).
        #[arg(long)]
        port_auto: bool,

        /// Instance-selection override `<dep>@<instance>` for depends_on
        /// resolution (ADR 0026(d)). PURE selection: discovery is activated
        /// by the depends_on declaration alone — `--use` only picks WHICH
        /// running instance of a declared dependency supplies the injected
        /// address. Repeatable (one per dep).
        #[arg(long = "use", value_name = "DEP@INSTANCE")]
        use_: Vec<String>,

        /// Do not auto-start declared depends_on dependencies (ADR 0026
        /// addendum); required deps then refuse-with-remediation at plan
        /// time, optional deps fall back per convention + warn.
        #[arg(long)]
        no_deps: bool,

        /// Re-render `template = true` seed_files over their EXISTING
        /// targets (bypasses `only_if_missing` for those entries). Static
        /// (non-template) seeds and `only_if_missing = false` behavior are
        /// unchanged. Name-scoped only — rejected on bare `up`.
        #[arg(long)]
        reseed: bool,

        /// Force the ensure-images pre-flight to rebuild+load+record
        /// nix-layered images even when the skew matrix would skip or trust
        /// (spec 21 §5.2). Batch scope (USER DECISION D3): on bare `up` the
        /// force applies to ALL service workloads in the batch. Parent-side
        /// only — NEVER forwarded to the detached child.
        #[arg(long)]
        reload_images: bool,

        /// Hidden detach token (spec 21 §2.2): the parent already ran the
        /// ensure-images pre-flight, so this process (the detached child)
        /// skips it. Appended unconditionally by `detach_args` — same shape
        /// as `--foreground`. Rejected on bare `up` (the batch form spawns
        /// per-workload children that carry their own token).
        #[arg(long, hide = true)]
        images_ready: bool,
    },
    /// Attach to an agent workload interactively (TUI)
    Exec {
        /// Workload name from the merged config.
        name: String,

        /// Accepted for flag parity with `up` (agents always run in
        /// foreground).
        #[arg(short, long, help = "Run in foreground (block until Ctrl-C)")]
        foreground: bool,

        /// Tear down any existing instance at this slot before starting.
        #[arg(long)]
        replace: bool,

        /// Target a parallel instance `<slot>@<id>`.
        #[arg(long, value_name = "ID")]
        instance: Option<String>,

        /// Auto-allocate the lowest free integer id >= 2.
        #[arg(long)]
        new: bool,

        /// Publish each port on a lock-probed free port on the slot's bind
        /// (the chosen ports are recorded in the instance record).
        #[arg(long)]
        port_auto: bool,

        /// Instance-selection override `<dep>@<instance>` for depends_on
        /// resolution (ADR 0026(d)). PURE selection: discovery is activated
        /// by the depends_on declaration alone — `--use` only picks WHICH
        /// running instance of a declared dependency supplies the injected
        /// address. Repeatable (one per dep).
        #[arg(long = "use", value_name = "DEP@INSTANCE")]
        use_: Vec<String>,

        /// Do not auto-start declared depends_on dependencies (ADR 0026
        /// addendum); required deps then refuse-with-remediation at plan
        /// time, optional deps fall back per convention + warn.
        #[arg(long)]
        no_deps: bool,

        /// Re-render `template = true` seed_files over their EXISTING
        /// targets (bypasses `only_if_missing` for those entries). Static
        /// (non-template) seeds and `only_if_missing = false` behavior are
        /// unchanged.
        #[arg(long)]
        reseed: bool,

        /// Force the ensure-images pre-flight to rebuild+load+record
        /// nix-layered images even when the skew matrix would skip or trust
        /// (spec 21 §5.2). Parent-side only (agents are always foreground).
        #[arg(long)]
        reload_images: bool,
    },
    /// Print the planned sandbox workload
    Plan {
        /// Workload name from the merged config.
        name: String,

        /// Render the plan as parallel slot <slot>@<id> would see it
        /// (prospective per-instance bind).
        #[arg(long, value_name = "ID")]
        instance: Option<String>,

        /// Instance-selection override `<dep>@<instance>` for depends_on
        /// resolution (ADR 0026(d)). PURE selection: discovery is activated
        /// by the depends_on declaration alone — `--use` only picks WHICH
        /// running instance of a declared dependency supplies the injected
        /// address. Repeatable (one per dep).
        #[arg(long = "use", value_name = "DEP@INSTANCE")]
        use_: Vec<String>,
    },
    /// Stop and remove the sandbox
    Down {
        /// Workload name from the merged config.
        name: String,

        /// Stop the parallel instance `<slot>@<id>`.
        #[arg(long, value_name = "ID")]
        instance: Option<String>,

        /// Stop the singleton AND every parallel instance of this workload.
        #[arg(long)]
        all_instances: bool,
    },
    /// Tail the detached service's log file
    Logs {
        /// Workload name from the merged config.
        name: String,

        /// Tail the parallel instance `<slot>@<id>` (default: the singleton).
        #[arg(long, value_name = "ID")]
        instance: Option<String>,
    },
    /// Scaffold a new workload (agent dir + [workloads.<name>] entry).
    New {
        /// Name for the new workload (e.g., "my-agent").
        name: String,
        /// Workload kind to scaffold.
        #[arg(long, value_parser = ["agent", "service"], default_value = "agent")]
        kind: String,
    },
    /// Build or check the nix-layered images of workloads (spec 21 phase C:
    /// drvPath change detection + the §3.4 skew matrix; the build/load
    /// pipeline itself is phase D). Omit the name for the batch form: all
    /// nix-layered workloads in the active context (mirrors the bare-up
    /// grammar of the ADR 0021 addendum). `--repo`/`--all-repos` widen the
    /// scope to registered config repos. NOT kind-routed — `main.rs`
    /// dispatches it early like `workload new`.
    Build {
        /// Workload name from the merged config. Omit for the batch form
        /// (all nix-layered workloads in the active context).
        #[arg(conflicts_with_all = ["repo", "all_repos"])]
        name: Option<String>,

        /// Build all nix-layered workloads declared by one registered config
        /// repo.
        #[arg(long, value_name = "CONFIG", conflicts_with = "all_repos")]
        repo: Option<String>,

        /// Build all nix-layered workloads across ALL registered config repos
        /// (`registry.configs`); repos without a flake.nix are skipped with a
        /// note.
        #[arg(long)]
        all_repos: bool,

        /// Print the staleness matrix (spec 21 §3.4) only: build nothing,
        /// load nothing, write no records.
        #[arg(long)]
        check: bool,

        /// Rebuild regardless of the change-detection outcome (the
        /// build-verb-local force; `--reload-images` on up/exec is phase E).
        #[arg(long)]
        force: bool,
    },
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
        /// Outside the store the repo is scaffolded but NOT registered.
        dest: Option<String>,

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
    /// Scaffolds at the RESOLVED tool home only — no positional dest. For an
    /// empty scaffold at a custom path, use `workestrate --home <path> home
    /// init` (the global --home flag). To provision a home from an existing
    /// one, use `workestrate home clone <src> [dest]`.
    Init {
        /// Clone and register this config-repo URL as config-repos/<name>
        /// after scaffolding the home.
        #[arg(long)]
        config: Option<String>,

        /// Name for the cloned config repo (only used with --config).
        #[arg(long, default_value = "personal")]
        name: String,
    },
    /// Provision a home from an existing one (git-clone semantics, ADR 0025):
    /// the registry layer is cloned/copied, config repos are re-cloned or
    /// copied, and registry urls pointing into the source home are rewritten
    /// to the dest-local `config-repos/<name>` paths.
    Clone {
        /// Source home to provision from (absolute path, relative path
        /// resolved against cwd, or git URL).
        src: String,

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
    /// Set the default context (persists settings.default_context in the registry).
    Use {
        /// Context name to make the default.
        name: String,
    },
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

/// Actions for managing the generated JSON Schema artifacts.
#[derive(Subcommand)]
pub enum SchemasAction {
    /// Write the generated JSON Schema artifacts to every known consumer
    /// location (tool-repo copier template, tool home, registered config
    /// repos). Idempotent: files whose content already matches are skipped.
    Update {
        /// Only update one registered config repo (by registry name).
        #[arg(long, value_name = "NAME")]
        repo: Option<String>,
        /// Report staleness without writing anything; exit 1 when any
        /// consumer file is stale or missing.
        #[arg(long)]
        check: bool,
    },
}
/// Actions for managing nix-layered images in the msb store
/// (`workestrate images ...`, ADR 0032 §Image tags).
#[derive(Subcommand)]
pub enum ImagesAction {
    /// Manual keep-last-N sweep of computed image tags across ALL state-dir
    /// groups (ADR 0032 §Image tags — RESOLVED user decision 3). The
    /// automatic prune-on-load runs on every build; this verb is the manual
    /// sweep. Running sandboxes are never affected.
    Gc {},
}

/// Mount-policy diagnostics (spec 22 §13).
#[derive(Subcommand)]
pub enum PolicyAction {
    /// Mount-masking diagnostics.
    Mounts {
        #[command(subcommand)]
        action: MountsDiagnosticsAction,
    },
}

#[derive(Subcommand)]
pub enum MountsDiagnosticsAction {
    /// Explain one mount-root-relative path.
    Explain {
        #[arg(long)]
        workload: String,
        #[arg(long)]
        mount: String,
        #[arg(long)]
        path: String,
    },
    /// Preview the annotated mount tree.
    Preview {
        #[arg(long)]
        workload: String,
        #[arg(long)]
        mount: String,
        #[arg(long)]
        root: Option<String>,
    },
}
