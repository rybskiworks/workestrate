//! Sandbox workloads: the `Workload` trait, the config-driven
//! `ConfigWorkload`, and the plan/env/secret builders.
//!
//! WP2 split of the former `workload.rs` god-file into `config`
//! (`ConfigWorkload` + its `Workload` impl), `secrets` (secret resolution and
//! provenance helpers), `show_source` (the `show_source` formatter), and
//! `validate` (trust-boundary validators + mount-template resolution). Purely
//! mechanical — no behavior changes.

use crate::microsandbox::plan::SandboxPlan;
use anyhow::Result;

mod config;
pub(crate) mod secrets;
mod show_source;
mod validate;

pub use config::ConfigWorkload;
pub use validate::{validate_env_override, validate_seed_source};

/// Program and args to exec inside the sandbox via exec_stream.
#[derive(Debug, Clone)]
pub struct SandboxCommand {
    pub binary: String,
    pub arguments: Vec<String>,
}

impl SandboxCommand {
    pub fn with_args(binary: impl Into<String>, args: &[&str]) -> Self {
        Self {
            binary: binary.into(),
            arguments: args.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// How to set the sandbox entrypoint.
#[derive(Debug, Clone)]
pub enum EntrypointSpec {
    /// Blocking keep-alive foreground (`/bin/sh -c "tail -f /dev/null"`) that
    /// keeps the sandbox alive while `exec_stream` runs the real service. A bare
    /// `/bin/sh` would run `/bin/sh <image-cmd>` (Docker ENTRYPOINT+CMD) and exit.
    Shell,
}

/// A sandbox workload. The generic `ConfigWorkload` implementation reads from
/// `workestrate.toml`; the lifecycle in `runtime.rs` operates on `&W where W:
/// Workload`.
pub trait Workload: Send + Sync + std::fmt::Debug {
    /// Sandbox name (used for Sandbox::get, logging, user messages).
    fn name(&self) -> &str;

    /// Sandbox instance name (used for Sandbox::builder, log dirs, down).
    /// Default: bare workload name. ConfigWorkload overrides to
    /// `<context>-<workload>` when contexts are active.
    fn sandbox_instance_name(&self) -> String {
        self.name().to_string()
    }

    /// Build the declarative sandbox plan.
    fn plan(&self) -> SandboxPlan;

    /// Format the plan with a `[source]` annotation for each field.
    fn show_source(&self) -> String {
        self.plan().to_string()
    }

    /// Program and args to exec inside the sandbox.
    fn exec(&self) -> SandboxCommand;

    /// Args to pass when re-exec'ing in detached (background) mode.
    ///
    /// Reconstructs the CLI from `spec` in the verb-first shape (ADR 0027)
    /// — `workload up <name> --foreground ...` — so the detached child
    /// re-enters the `up --foreground` path with the SAME identity and flags
    /// the parent resolved:
    ///   - `--replace` (when `spec.replace`),
    ///   - `--port-auto` (when `spec.port_auto`, ADR 0026(c)),
    ///   - `--instance <id>` for a parallel instance — the **bare id**, not
    ///     `slot@id` (the child re-derives the slot from its own context),
    ///   - `--use <dep>@<instance>` for each depends_on instance-selection
    ///     override (`spec.use_overrides`, ADR 0026(d)) so the child resolves
    ///     the SAME records the parent did.
    ///
    /// Detach is service-only: only `spawn_detached_service` calls this
    /// (agents run in foreground), so `up` is the only verb emitted.
    ///
    /// `--new` is intentionally NOT forwarded: the parent has already
    /// materialized the slug into `spec.instance`, so the child must target
    /// that concrete instance rather than allocate a fresh one. The child
    /// parses these args via clap's `workload up` subcommand; the raw-args
    /// `parse_service_action` parser still round-trips the same flags for
    /// the detach tests.
    fn detach_args(&self, spec: &crate::microsandbox::runtime::InstanceSpec) -> Vec<String> {
        let mut args: Vec<String> = vec![
            "workload".to_string(),
            "up".to_string(),
            self.name().to_string(),
            "--foreground".to_string(),
        ];
        if spec.replace {
            args.push("--replace".to_string());
        }
        if spec.port_auto {
            args.push("--port-auto".to_string());
        }
        // ADR 0026 addendum: forward --no-deps so the detached child does
        // NOT re-run dependency auto-start the parent was told to skip.
        if spec.no_deps {
            args.push("--no-deps".to_string());
        }
        for (dep, id) in &spec.use_overrides {
            args.push("--use".to_string());
            args.push(format!("{}@{}", dep, id));
        }
        // Forward the parallel-instance id only when this is NOT the singleton
        // (instance == slot, no `@`). instance_id_of splits on the first `@`;
        // slots never contain `@`, so this is unambiguous.
        if let Some(id) = crate::microsandbox::slots::instance_id_of(&spec.instance) {
            args.push("--instance".to_string());
            args.push(id.to_string());
        }
        args
    }

    /// Optional pre-start hook (e.g., writing config files to persistent data dir).
    fn prepare(&self) -> Result<()> {
        Ok(())
    }

    /// Whether to log errors when stopping the sandbox (default: true).
    fn log_stop_errors(&self) -> bool {
        true
    }

    /// Sandbox entrypoint (default: Shell).
    fn entrypoint(&self) -> EntrypointSpec {
        EntrypointSpec::Shell
    }

    /// Where to find the built workload artifacts. Default: the reserved
    /// `.workestrate-build/<name>` dir (spec 21 §6.1), resolved
    /// declaring-layer-relative at mount time; override per workload with
    /// WORKESTRATE_<NAME>_BUILD (NAME uppercased, '-' → '_') — used by the
    /// nix wrapper to point at a store path.
    fn build_path(&self) -> String {
        let key = format!(
            "WORKESTRATE_{}_BUILD",
            self.name().to_ascii_uppercase().replace('-', "_")
        );
        if let Ok(p) = std::env::var(key) {
            p
        } else {
            format!(".workestrate-build/{}", self.name())
        }
    }

    /// Whether `build_path()` is the UNDECLARED reserved default
    /// (`.workestrate-build/<name>`, spec 21 §6.1) rather than a declared
    /// `local_build.fallback` or an env override. The reserved default is a
    /// config-repo artifact dir that resolves DECLARING-LAYER-relative — it
    /// must NOT trigger the flake project-root mount preference or the
    /// flake-root gate. Default impl matches the default `build_path()`
    /// above: true when the conventional env var is unset. Implementors that
    /// override `build_path()` with declared-fallback handling MUST override
    /// this too.
    fn build_path_is_reserved_default(&self) -> bool {
        let key = format!(
            "WORKESTRATE_{}_BUILD",
            self.name().to_ascii_uppercase().replace('-', "_")
        );
        std::env::var(key).is_err()
    }

    /// Content root for repo-relative mount hosts: the directory of the
    /// config layer that DECLARED this workload's mounts (spec 17 directory
    /// mode — config content lives in config repos, not in the tool
    /// checkout). `None` when no declaring layer dir is knowable (synthetic
    /// layers, hand-built workloads); the caller then applies the documented
    /// fallback (flake project root, else cwd) explicitly.
    fn mount_content_root(&self) -> Option<std::path::PathBuf> {
        None
    }

    /// The feature requiring a flake project root at sandbox-build time, if
    /// any — a human-readable label used in the gate's error message
    /// ("workload '<name>' uses <feature>, which requires a flake project
    /// root: ..."). `None` means `build_sandbox` must NOT call
    /// `project_root()` eagerly: registry-image workloads with no local
    /// build and no relative build-path mounts run from any cwd.
    fn flake_root_requirement(&self, plan: &SandboxPlan) -> Option<String> {
        let _ = plan;
        None
    }
}
