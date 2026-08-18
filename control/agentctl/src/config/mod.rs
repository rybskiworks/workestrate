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
pub use loading::{
    check_required_files, load_config, load_overrides, resolve_secrets_layers, CheckEntry,
};
#[allow(unused_imports)]
pub use lockfile::{
    home_lock_path, home_lock_path_for, load_home_lock, load_home_lock_from, lock_from_registry,
    save_home_lock, save_home_lock_to, HomeLock, LockedRepo, LOCK_FILE_NAME, LOCK_VERSION,
};
#[allow(unused_imports)]
pub use migration::{run_migrate_home, MigrateSummary, MovedEntry};
#[allow(unused_imports)]
pub use paths::expand_tilde;
#[allow(unused_imports)]
pub use paths::{
    config_repo_dir, overrides_path, registry_path, resolve_active_config_dir, resolve_home,
    resolve_home_with_kind, resolve_state_dir, resolve_store_dir, source_store_dir, xdg_config_dir,
    xdg_data_dir, xdg_state_dir, HomeKind,
};
#[allow(unused_imports)]
pub(crate) use registry::looks_like_git_url;
#[allow(unused_imports)]
pub use registry::{
    entry_is_local_path, load_registry, local_entry_checkout_dir, register_config,
    resolve_active_context, save_registry, set_default_context,
};
#[allow(unused_imports)]
pub use trust::is_dir_trusted_via_base_registry;
#[allow(unused_imports)]
pub use trust::{is_trusted_project, trust_project, untrust_project};
#[allow(unused_imports)]
pub use types::{
    BakedFileSpec, BinarySpec, Bound, ConfigFile, ConfigRepoEntry, ConflictStep, Context,
    DepConflict, DepInstanceMode, DependsOnSpec, EnvBinding, EnvBindings, EnvSecretRef,
    EnvVarConfig, ImageSpec, InstancePolicy, InstancePort, InstanceStrategy, LocalBuildConfig,
    NetworkConfig, PolicyConfig, PortOccupiedBare, PortOccupiedChain, PortOccupiedStep, Registry,
    RegistrySettings, SecretDefConfig, SecretsLayer, SeedFileConfig, TrustedProject,
    WorkloadConfig,
};
#[allow(unused_imports)]
pub use validation::{validate_config, validate_config_name, EXPECTED_SCHEMA_VERSION};

use anyhow::Result;
use std::path::PathBuf;

/// The resolved active context.
/// `name` is None when no contexts are defined (bare-layers backward-compat).
#[derive(Debug, Clone)]
pub struct ActiveContext {
    pub name: Option<String>,
    pub layers: Vec<String>,
}

/// The active context for this process (WP10/A9).
///
/// This used to be a `thread_local!` `RefCell`. On a tokio MULTI-THREAD
/// runtime (main.rs builds `tokio::runtime::Builder::new_multi_thread()`) a
/// task can be migrated across OS threads between `.await` points, so a value
/// stored in a plain `thread_local` on one thread could be read on a
/// DIFFERENT thread after an await — returning `None` or a stale value. The
/// active context is set once per command invocation (inside `load_config`,
/// which also resolves it) and is inherently process-level state, so a
/// `std::sync::Mutex` is the correct primitive: no thread affinity, no
/// scope-establishment requirement (unlike `tokio::task_local!`, which would
/// need `TaskLocal::scope` at task spawn — a main.rs change that is out of
/// scope — and panics outside its scope). Lock poisoning is recovered with
/// `into_inner()` so a panic elsewhere can never wedge context resolution.
static ACTIVE_CONTEXT: std::sync::Mutex<Option<ActiveContext>> = std::sync::Mutex::new(None);

/// Store the active context for this process.
pub fn set_active_context(ctx: Option<ActiveContext>) {
    *ACTIVE_CONTEXT.lock().unwrap_or_else(|e| e.into_inner()) = ctx;
}

/// Get the active context name (None = bare-layers backward-compat).
pub fn active_context_name() -> Option<String> {
    ACTIVE_CONTEXT
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
            (std::env::current_dir()?, RootSource::ManifestDir)
        }
    }
    // 3. Current working directory
    else {
        (std::env::current_dir()?, RootSource::Cwd)
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
    // 3. Current working directory + flake.nix check.
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join("flake.nix").exists() {
            return Some(cwd);
        }
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
        std::env::set_var("AGENTCTL_ROOT", "/tmp");
        let result = project_root();
        match old {
            Some(v) => std::env::set_var("AGENTCTL_ROOT", v),
            None => std::env::remove_var("AGENTCTL_ROOT"),
        }
        // /tmp doesn't have flake.nix, so this should error
        assert!(result.is_err(), "expected error when flake.nix missing");
    }

    #[test]
    fn project_root_rejects_missing_flake_nix() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let old = std::env::var("AGENTCTL_ROOT").ok();
        std::env::set_var("AGENTCTL_ROOT", "/tmp/nonexistent-ai-workbench-test");
        let result = project_root();
        match old {
            Some(v) => std::env::set_var("AGENTCTL_ROOT", v),
            None => std::env::remove_var("AGENTCTL_ROOT"),
        }
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("flake.nix"),
            "error should mention flake.nix: {err}"
        );
    }

    // ---- WP10/A9: active-context storage is process-global ----

    /// A9 regression: a context set on one OS thread must be readable on a
    /// DIFFERENT thread (a plain thread_local would return None there — the
    /// exact failure mode on a tokio multi-thread runtime after task
    /// migration). Uses ENV_TEST_LOCK because set_active_context is process
    /// global and other tests mutate it.
    #[test]
    fn active_context_set_from_another_thread_is_visible() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        set_active_context(None);
        std::thread::spawn(|| {
            set_active_context(Some(ActiveContext {
                name: Some("personal".to_string()),
                layers: vec!["personal".to_string()],
            }));
        })
        .join()
        .expect("setter thread panicked");
        assert_eq!(active_context_name(), Some("personal".to_string()));
        set_active_context(None); // clean up for other tests
    }

    #[test]
    fn active_context_survives_tokio_multi_thread_migration() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .build()
            .expect("failed to build multi-thread runtime");
        rt.block_on(async {
            set_active_context(Some(ActiveContext {
                name: Some("work".to_string()),
                layers: vec!["work".to_string()],
            }));
            for _ in 0..100 {
                tokio::task::yield_now().await;
            }
            assert_eq!(active_context_name(), Some("work".to_string()));
        });
        set_active_context(None);
    }
}
