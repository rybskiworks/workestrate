mod env;
mod mounts;
pub mod plan;
pub(crate) mod port_registry;
pub(crate) mod runtime;
pub mod secrets;
pub(crate) mod secrets_loader;
pub mod slots;
pub mod workload;

// Re-export the single lifecycle function that callers consume via the
// module root (`crate::microsandbox::logs`). All other `runtime` items are
// reached through the `pub(crate)` module path directly.
pub use runtime::logs;

// WP1 trust-boundary validators — called from `config::validate_config`.
pub(crate) use mounts::{validate_mount_guest, validate_mount_host};
pub(crate) use workload::{validate_env_override, validate_seed_source};
