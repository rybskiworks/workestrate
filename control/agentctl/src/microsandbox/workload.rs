use crate::microsandbox::plan::SandboxPlan;
use anyhow::Result;

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

/// How to run the workload's exec command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecMode {
    /// Stream stdout/stderr to the terminal; no stdin (headless service).
    Headless,
    /// Take over the terminal with a full TTY bridge (interactive TUI apps).
    Interactive,
}

/// How to set the sandbox entrypoint.
#[derive(Debug, Clone)]
pub enum EntrypointSpec {
    /// Blocking keep-alive foreground (`/bin/sh -c "tail -f /dev/null"`) that
    /// keeps the sandbox alive while `exec_stream` runs the real service. A bare
    /// `/bin/sh` would run `/bin/sh <image-cmd>` (Docker ENTRYPOINT+CMD) and exit.
    Shell,
}

/// A sandbox workload. Each application (litellm, pi, odysseus, opencode, ...)
/// implements this trait. The generic lifecycle in `runtime.rs` operates on
/// `&W where W: Workload`.
///
/// To add a new workload:
/// 1. Create `workloads/<name>.rs` (struct + `impl Workload`)
/// 2. Add `mod <name>;` + `pub use <name>::<Name>;` to `workloads/mod.rs`
/// 3. Add one entry to the `workloads!` macro in `main.rs`
///    (the `Commands` variant and dispatch logic are generated from that entry)
pub trait Workload: Send + Sync + std::fmt::Debug {
    /// Sandbox name (used for Sandbox::get, logging, user messages).
    fn name(&self) -> &str;

    /// Build the declarative sandbox plan.
    fn plan(&self) -> SandboxPlan;

    /// Program and args to exec inside the sandbox.
    fn exec(&self) -> SandboxCommand;

    /// Whether to run interactively (TUI) or headless. Default: Headless.
    fn exec_mode(&self) -> ExecMode {
        ExecMode::Headless
    }

    /// Args to pass when re-exec'ing in background mode.
    fn detach_args(&self) -> Vec<String>;

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

    /// Where to find the built agent code (agents/<name>/build).
    fn build_path(&self) -> String {
        format!("agents/{}/build", self.name())
    }

    /// Convention: config files live at agents/<name>/config/<filename>
    fn config_path(&self, filename: &str) -> String {
        format!("agents/{}/config/{}", self.name(), filename)
    }
}
