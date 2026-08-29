pub mod depgraph;
pub mod discovery;
mod env;
pub(crate) mod mounts;
pub mod plan;
pub mod policy_file;
pub mod port_registry;
pub mod provenance;
pub mod runtime;
pub mod secrets;
pub mod secrets_loader;
pub mod slots;
pub mod workload;

// Re-export the single lifecycle function that callers consume via the
// module root (`crate::microsandbox::logs`). All other `runtime` items are
// reached through the `pub(crate)` module path directly.
pub use runtime::logs;

// WP1 trust-boundary validators — called from `config::validate_config`.
pub use mounts::{validate_mount_guest, validate_mount_host};
pub use policy_file::{
    approved_policy_root, mount_slug, policy_file_path, policy_file_rel, remove_policy_dir,
    write_policy_file, MOUNT_POLICY_DIR_NAME,
};
pub use workload::{
    validate_env_override, validate_seed_glob, validate_seed_source, validate_seed_target,
    validate_seed_target_coverage,
};
