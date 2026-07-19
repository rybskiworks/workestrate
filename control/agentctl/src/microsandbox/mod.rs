mod env;
mod mounts;
pub mod plan;
pub(crate) mod port_registry;
mod runtime;
pub mod secrets;
pub(crate) mod secrets_loader;
pub mod workload;

// Re-export lifecycle functions that callers need.
pub use runtime::{down, exec_agent, logs, up_service};
