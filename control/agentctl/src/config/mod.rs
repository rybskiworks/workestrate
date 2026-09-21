pub mod archive;
pub mod inline_ref;
pub mod loading;
pub mod lockfile;
pub mod migration;
pub mod paths;
pub mod registry;
pub mod trust;
pub mod types;
pub mod validation;

// Re-exports: every previously-`pub` item of the old flat `config.rs` keeps
// resolving as `crate::config::X` via these (a binary crate has no external
// `use` of the re-exports, hence the allow).
#[allow(unused_imports)]
pub use archive::{archive_dir, archive_store_root, ensure_archive};
#[allow(unused_imports)]
pub use inline_ref::{
    InlineOverride, WORKLOAD_REF_ENV, arm_inline_override, clear_inline_override,
    parse_workload_selector, pending_inline_override, set_pending_inline_override,
    set_pending_inline_override_from_env, verb_arms_after_dep_autostart,
};
#[allow(unused_imports)]
pub use loading::{
    CheckEntry, check_required_files, load_config, load_overrides, resolve_secrets_layers,
};
#[allow(unused_imports)]
pub use lockfile::{
    ConfigLock, LOCK_FILE_NAME, LOCK_VERSION, LockedFleet, LockedRef, config_lock_path,
    config_lock_path_for, load_config_lock, load_config_lock_from, lock_from_registry,
    save_config_lock, save_config_lock_to, upsert_locked_pin, upsert_locked_ref,
};
#[allow(unused_imports)]
pub use migration::{MigrateSummary, MovedEntry, run_migrate_config};
#[allow(unused_imports)]
pub use paths::expand_tilde;
#[allow(unused_imports)]
pub use paths::{
    ConfigDirKind, fleet_dir, overrides_path, registry_path, resolve_active_fleet_dir,
    resolve_config_dir, resolve_config_dir_with_kind, resolve_state_dir, resolve_store_dir,
    source_store_dir, xdg_config_dir, xdg_data_dir, xdg_state_dir,
};
#[allow(unused_imports)]
pub use paths::{
    INVOKE_CWD_ENV, canonical_invoke_cwd_string, ensure_invoke_cwd_env, invoke_cwd,
    invoke_cwd_or_err,
};
#[allow(unused_imports)]
pub(crate) use registry::looks_like_git_url;
#[allow(unused_imports)]
pub use registry::{
    ConfigSourceKind, effective_ref, entry_is_local_path, load_registry, local_entry_checkout_dir,
    register_fleet, resolve_active_fleet, resolve_default_ref, save_registry, source_kind,
};
#[allow(unused_imports)]
pub use trust::is_dir_trusted_via_base_registry;
#[allow(unused_imports)]
pub use trust::{is_trusted_project, trust_project, untrust_project};
#[allow(unused_imports)]
pub use types::{
    BakedFileSpec, BinarySpec, Bound, ConfigFile, ConflictStep, CredentialsConfig, DefaultAction,
    DepConflict, DepInstanceMode, DependsOnSpec, DomainEntry, EgressAllowTable, EgressDenyTable,
    EgressPolicyFragment, EnvBinding, EnvBindings, EnvSecretRef, EnvVarConfig, FleetEntry,
    HostEntry, IdnaMode, IdnaPolicyFragment, ImageSpec, IngressAllowTable, IngressDenyTable,
    IngressPolicyFragment, InitConfig, InstancePolicy, InstancePort, InstanceStrategy,
    LocalBuildConfig, NestedMode, NetworkConfig, NetworkDefaultsConfig, OnConflict, OnSkew,
    PolicyConfig, PortEntry, PortOccupiedBare, PortOccupiedChain, PortOccupiedStep, Registry,
    RegistrySettings, SecretDefConfig, SecretViolationPolicy, SecretsLayer, SecretsPolicyFragment,
    SeedFileConfig, SigningCredentialsConfig, SigningSshCredentialDef, SshCredentialDef,
    SshPolicyFragment, TrustedProject, VirtualizationConfig, VirtualizationPolicyFragment,
    WorkloadConfig, WorkloadCredentials,
};
#[allow(unused_imports)]
pub use validation::{EXPECTED_SCHEMA_VERSION, validate_config, validate_fleet_name};

use anyhow::Result;
use std::path::PathBuf;

/// The resolved active fleet.
/// `name` is None when no fleets are registered (bare-layers mode).
#[derive(Debug, Clone)]
pub struct ActiveFleet {
    pub name: Option<String>,
    pub layers: Vec<String>,
}

/// The active fleet for this process (WP10/A9).
///
/// This used to be a `thread_local!` `RefCell`. On a tokio MULTI-THREAD
/// runtime (main.rs builds `tokio::runtime::Builder::new_multi_thread()`) a
/// task can be migrated across OS threads between `.await` points, so a value
/// stored in a plain `thread_local` on one thread could be read on a
/// DIFFERENT thread after an await — returning `None` or a stale value. The
/// active fleet is set once per command invocation (inside `load_config`,
/// which also resolves it) and is inherently process-level state, so a
/// `std::sync::Mutex` is the correct primitive: no thread affinity, no
/// scope-establishment requirement (unlike `tokio::task_local!`, which would
/// need `TaskLocal::scope` at task spawn — a main.rs change that is out of
/// scope — and panics outside its scope). Lock poisoning is recovered with
/// `into_inner()` so a panic elsewhere can never wedge fleet resolution.
static ACTIVE_FLEET: std::sync::Mutex<Option<ActiveFleet>> = std::sync::Mutex::new(None);

/// Store the active fleet for this process.
pub fn set_active_fleet(ctx: Option<ActiveFleet>) {
    *ACTIVE_FLEET.lock().unwrap_or_else(|e| e.into_inner()) = ctx;
}

/// Get the active fleet name (None = bare-layers mode).
pub fn active_fleet_name() -> Option<String> {
    ACTIVE_FLEET
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
        .and_then(|ctx| ctx.name.clone())
}

/// Which tier resolved the project root in [`project_root_with_source`].
///
/// The `Cwd` variant is the security-relevant one (spec 05): a cwd-derived
/// root is operator-unpinned, so consumers like `reference_config_path()`
/// gate behavior behind an explicit opt-in when it is the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RootSource {
    /// Tier 1: `AGENTCTL_ROOT` env var.
    AgentctlRoot,
    /// Tier 2: two directories up from `CARGO_MANIFEST_DIR` (cargo run/test).
    /// Also covers the tier-2 edge case where `CARGO_MANIFEST_DIR` is set but
    /// the double-pop fails and the current directory is used instead.
    ManifestDir,
    /// Tier 3: current working directory (unpinned).
    Cwd,
}

/// Resolve the AI-workbench project root (the checkout containing
/// `flake.nix`) and report which tier resolved it. Precedence:
/// `AGENTCTL_ROOT` env var, then two directories up from `CARGO_MANIFEST_DIR`
/// (cargo run/test), then the current working directory. Hard-errors when the
/// resolved root does not contain `flake.nix`; callers that can degrade
/// gracefully should use [`project_root_optional`] instead.
pub(crate) fn project_root_with_source() -> Result<(PathBuf, RootSource)> {
    // 1. AGENTCTL_ROOT env var
    let (root, source) = if let Ok(root) = std::env::var("AGENTCTL_ROOT") {
        (PathBuf::from(root), RootSource::AgentctlRoot)
    }
    // 2. Walk up from CARGO_MANIFEST_DIR (compile-time, works in cargo run)
    else if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut path = PathBuf::from(manifest);
        if path.pop() && path.pop() {
            (path, RootSource::ManifestDir)
        } else {
            (paths::invoke_cwd_or_err()?, RootSource::ManifestDir)
        }
    }
    // 3. Current working directory (the operator's INVOCATION cwd — see
    //    paths::INVOKE_CWD_ENV; never re-read lazily).
    else {
        (paths::invoke_cwd_or_err()?, RootSource::Cwd)
    };

    // Validate: the root must contain flake.nix
    if !root.join("flake.nix").exists() {
        anyhow::bail!(
            "resolved project root '{}' does not contain flake.nix.\n\
             Set AGENTCTL_ROOT or run from the workbench root directory.",
            root.display()
        );
    }

    Ok((root, source))
}

/// Resolve the AI-workbench project root (the checkout containing
/// `flake.nix`). Precedence: `AGENTCTL_ROOT` env var, then two directories up
/// from `CARGO_MANIFEST_DIR` (cargo run/test), then the current working
/// directory. Hard-errors when the resolved root does not contain
/// `flake.nix`; callers that can degrade gracefully should use
/// [`project_root_optional`] instead.
///
/// This is the root-only projection of [`project_root_with_source`]; use that
/// variant when the caller needs to know which tier resolved the root (e.g.
/// the spec-05 cwd-reference gate in `reference_config_path()`).
pub fn project_root() -> Result<PathBuf> {
    project_root_with_source().map(|(root, _)| root)
}

/// Best-effort variant of [`project_root`] for the standalone-installed-tool
/// model (review finding E1, WP5). Returns `None` instead of bailing when no
/// workbench checkout can be located — letting callers like `cmd_check` and
/// `find_reference_config` degrade gracefully.
///
/// Callers that genuinely require a workbench root (e.g. workload source /
/// build resolution at sandbox-start time) should keep using [`project_root`]
/// so the hard failure surfaces at the operation that needs it.
pub fn project_root_optional() -> Option<PathBuf> {
    // 1. AGENTCTL_ROOT env var
    if let Ok(root) = std::env::var("AGENTCTL_ROOT") {
        let p = PathBuf::from(root);
        if p.join("flake.nix").exists() {
            return Some(p);
        }
    }
    // 2. Walk up from CARGO_MANIFEST_DIR (cargo run / cargo test).
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut path = PathBuf::from(manifest);
        if path.pop() && path.pop() && path.join("flake.nix").exists() {
            return Some(path);
        }
    }
    // 3. Invocation working directory + flake.nix check.
    if let Some(cwd) = paths::invoke_cwd()
        && cwd.join("flake.nix").exists()
    {
        return Some(cwd);
    }
    None
}

#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result,
    clippy::new_without_default
)]
pub mod test_support;

#[cfg(test)]
pub(crate) mod tests {
    #![allow(
        unsafe_code,
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::unwrap_in_result
    )]
    use super::*;
    use crate::config::test_support::*;

    #[test]
    fn project_root_with_agentctl_root_env() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let old = std::env::var("AGENTCTL_ROOT").ok();
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("AGENTCTL_ROOT", "/tmp") };
        let result = project_root();
        match old {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("AGENTCTL_ROOT", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("AGENTCTL_ROOT") },
        }
        // /tmp doesn't have flake.nix, so this should error
        assert!(result.is_err(), "expected error when flake.nix missing");
    }

    #[test]
    fn project_root_rejects_missing_flake_nix() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let old = std::env::var("AGENTCTL_ROOT").ok();
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("AGENTCTL_ROOT", "/tmp/nonexistent-ai-workbench-test") };
        let result = project_root();
        match old {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("AGENTCTL_ROOT", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("AGENTCTL_ROOT") },
        }
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("flake.nix"),
            "error should mention flake.nix: {err}"
        );
    }

    // ---- WP10/A9: active-fleet storage is process-global ----

    /// A9 regression: a fleet set on one OS thread must be readable on a
    /// DIFFERENT thread (a plain thread_local would return None there — the
    /// exact failure mode on a tokio multi-thread runtime after task
    /// migration). Uses ENV_TEST_LOCK because set_active_fleet is process
    /// global and other tests mutate it.
    #[test]
    fn active_fleet_set_from_another_thread_is_visible() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        set_active_fleet(None);
        std::thread::spawn(|| {
            set_active_fleet(Some(ActiveFleet {
                name: Some("personal".to_string()),
                layers: vec!["personal".to_string()],
            }));
        })
        .join()
        .expect("setter thread panicked");
        assert_eq!(active_fleet_name(), Some("personal".to_string()));
        set_active_fleet(None); // clean up for other tests
    }

    #[test]
    fn active_fleet_survives_tokio_multi_thread_migration() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .build()
            .expect("failed to build multi-thread runtime");
        rt.block_on(async {
            set_active_fleet(Some(ActiveFleet {
                name: Some("work".to_string()),
                layers: vec!["work".to_string()],
            }));
            for _ in 0..100 {
                tokio::task::yield_now().await;
            }
            assert_eq!(active_fleet_name(), Some("work".to_string()));
        });
        set_active_fleet(None);
    }
}
