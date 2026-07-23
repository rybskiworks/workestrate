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
mod secrets;
mod show_source;
mod validate;

pub use config::ConfigWorkload;
pub(crate) use validate::{validate_env_override, validate_seed_source};

/// Program and args to exec inside the sandbox via exec_stream.
#[derive(Debug, Clone)]
pub struct SandboxCommand {
    pub binary: String,
    pub arguments: Vec<String>,
}

impl SandboxCommand {
    #[allow(dead_code)]
    pub fn new(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
            arguments: vec![],
        }
    }
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
    /// Reconstructs the CLI from `spec` so the detached child re-enters the
    /// `up --foreground` path with the SAME identity and flags the parent
    /// resolved:
    ///   - `--replace` (when `spec.replace`),
    ///   - `--instance <id>` for a parallel instance — the **bare id**, not
    ///     `slot@id` (the child re-derives the slot from its own context),
    ///   - `--port-offset <N>` when the offset is nonzero.
    ///
    /// `--new` is intentionally NOT forwarded: the parent has already
    /// materialized the slug into `spec.instance`, so the child must target
    /// that concrete instance rather than allocate a fresh one. The child
    /// re-parses these args via the existing `parse_service_action` /
    /// `parse_agent_action` path; no child-side change is required.
    fn detach_args(&self, spec: &crate::microsandbox::runtime::InstanceSpec) -> Vec<String> {
        let mut args: Vec<String> = vec![
            self.name().to_string(),
            "up".to_string(),
            "--foreground".to_string(),
        ];
        if spec.replace {
            args.push("--replace".to_string());
        }
        // Forward the parallel-instance id only when this is NOT the singleton
        // (instance == slot, no `@`). instance_id_of splits on the first `@`;
        // slots never contain `@`, so this is unambiguous.
        if let Some(id) = crate::microsandbox::slots::instance_id_of(&spec.instance) {
            args.push("--instance".to_string());
            args.push(id.to_string());
        }
        if spec.port_offset != 0 {
            args.push("--port-offset".to_string());
            args.push(spec.port_offset.to_string());
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

    /// Where to find the built agent code. Defaults to agents/<name>/build;
    /// override per-agent with WORKESTRATE_<NAME>_BUILD (NAME uppercased,
    /// '-' → '_') — used by the nix wrapper to point at a store path.
    #[allow(dead_code)]
    fn build_path(&self) -> String {
        let key = format!(
            "WORKESTRATE_{}_BUILD",
            self.name().to_ascii_uppercase().replace('-', "_")
        );
        if let Ok(p) = std::env::var(key) {
            p
        } else {
            format!("agents/{}/build", self.name())
        }
    }

    /// Convention: config files live at agents/<name>/config/<filename>
    #[allow(dead_code)]
    fn config_path(&self, filename: &str) -> String {
        format!("agents/{}/config/{}", self.name(), filename)
    }
}
