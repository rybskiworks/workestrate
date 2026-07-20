mod env;
mod mounts;
pub mod plan;
pub(crate) mod port_registry;
mod runtime;
pub mod secrets;
pub(crate) mod secrets_loader;
pub mod slots;
pub mod workload;

// Re-export lifecycle functions that callers need.
pub use runtime::{down, exec_agent, logs, up_service};

// WP1 trust-boundary validators — called from `config::validate_config`.
pub(crate) use mounts::{validate_mount_guest, validate_mount_host};
pub(crate) use workload::{validate_env_override, validate_seed_source};
