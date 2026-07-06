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

    /// Args to pass when re-exec'ing in background mode.
    /// The child invokes `<name> up --foreground` so it blocks instead of
    /// re-detaching forever.
    fn detach_args(&self) -> Vec<String> {
        vec![self.name().into(), "up".into(), "--foreground".into()]
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
    fn config_path(&self, filename: &str) -> String {
        format!("agents/{}/config/{}", self.name(), filename)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use crate::microsandbox::workload::Workload;
    use crate::workloads::Pi;

    #[test]
    fn build_path_reads_per_agent_env_override() {
        let pi = Pi;
        // Override set → returns the env value.
        std::env::set_var("WORKESTRATE_PI_BUILD", "/tmp/test-pi-build");
        assert_eq!(pi.build_path(), "/tmp/test-pi-build");
        // Override removed → falls back to agents/<name>/build.
        std::env::remove_var("WORKESTRATE_PI_BUILD");
        assert_eq!(pi.build_path(), "agents/pi/build");
    }
}
