//! Tool home resolution (ADR 0023 single-home layout), XDG path resolution,
//! and state/store directory derivation.

use std::path::{Path, PathBuf};

use crate::config::types::Registry;

// ---------------------------------------------------------------------------
// Tool home resolution (ADR 0023 single-home layout)
// ---------------------------------------------------------------------------

/// How the workestrate tool home was resolved (ADR 0023 single-home layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeKind {
    /// `WORKESTRATE_HOME` env var — new single-home layout.
    Env,
    /// Auto-discovered `.workestrate/config.toml` inside a trusted project — new layout.
    Discovered,
    /// Legacy XDG layout (`XDG_*_HOME` set) — compatibility, read/write as before.
    LegacyXdg,
    /// Default `~/.workestrate` — new single-home layout.
    Default,
}

/// One-time stderr migration note for the legacy XDG layout.
static LEGACY_NOTE: std::sync::Once = std::sync::Once::new();

fn emit_legacy_xdg_note() {
    LEGACY_NOTE.call_once(|| {
        eprintln!(
            "note: using legacy XDG workestrate layout; run 'workestrate migrate-home' to \
             consolidate into a single WORKESTRATE_HOME"
        );
    });
}

/// One-time stderr warning when an untrusted `.workestrate/config.toml` is found
/// during discovery. `resolve_home_with_kind` is called many times per command
/// (cmd_check, registry_path, store/state resolution, ...); without this guard
/// the warning would print once per call.
static DISCOVERY_WARN: std::sync::Once = std::sync::Once::new();

fn emit_untrusted_discovery_warn(dir: &Path) {
    DISCOVERY_WARN.call_once(|| {
        eprintln!(
            ".workestrate/config.toml found in {} but it is not a trusted project; \
             ignoring (run 'workestrate config trust <dir>' to trust it)",
            dir.display()
        );
    });
}

fn xdg_var_set(name: &str) -> bool {
    std::env::var(name).map(|v| !v.is_empty()).unwrap_or(false)
}

/// Base home resolution WITHOUT discovery (Env/LegacyXdg/Default only).
///
/// Used by the discovery trust-check ([`is_dir_trusted_via_base_registry`]) so
/// that loading the global trust registry cannot recurse back into discovery.
fn resolve_home_base_with_kind() -> (PathBuf, HomeKind) {
    // (a) Env: WORKESTRATE_HOME
    if let Ok(value) = std::env::var("WORKESTRATE_HOME") {
        if !value.is_empty() {
            return (expand_tilde(&value), HomeKind::Env);
        }
    }
    // (c) Legacy XDG
    if xdg_var_set("XDG_CONFIG_HOME")
        || xdg_var_set("XDG_DATA_HOME")
        || xdg_var_set("XDG_STATE_HOME")
    {
        return (xdg_config_dir(), HomeKind::LegacyXdg);
    }
    // (d) Default
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    (PathBuf::from(home).join(".workestrate"), HomeKind::Default)
}

/// The registry path computed from the *base* resolution (no discovery).
///
/// Location of the global trust list, independent of any discovered project
/// home — so a hostile `.workestrate/` cannot self-trust.
pub(crate) fn base_registry_path() -> PathBuf {
    let (home, kind) = resolve_home_base_with_kind();
    match kind {
        HomeKind::LegacyXdg => xdg_config_dir().join("config.toml"),
        _ => home.join("config.toml"),
    }
}

/// Resolve the workestrate tool home and how it was chosen (ADR 0023).
///
/// Precedence (first match wins):
/// 1. **Env** — `WORKESTRATE_HOME` (used verbatim, leading `~/` expanded).
/// 2. **Discovered** — a `.workestrate/config.toml` in a *trusted* ancestor of
///    the cwd, but only when no `XDG_*_HOME` var is set; an explicit XDG var is
///    a deliberate legacy-layout signal that discovery must not override. An
///    untrusted discovery prints a one-time warning and *stops* walking (does
///    not keep looking higher), then falls through.
/// 3. **LegacyXdg** — any of `XDG_CONFIG_HOME`/`XDG_DATA_HOME`/`XDG_STATE_HOME`
///    set and non-empty (compatibility; emits a one-time migration note).
/// 4. **Default** — `~/.workestrate`.
pub fn resolve_home_with_kind() -> (PathBuf, HomeKind) {
    // (a) Env: WORKESTRATE_HOME
    if let Ok(value) = std::env::var("WORKESTRATE_HOME") {
        if !value.is_empty() {
            return (expand_tilde(&value), HomeKind::Env);
        }
    }

    let xdg_explicit = xdg_var_set("XDG_CONFIG_HOME")
        || xdg_var_set("XDG_DATA_HOME")
        || xdg_var_set("XDG_STATE_HOME");

    // (b) Discovery — only when XDG is NOT explicitly set. An explicit XDG
    // var is a deliberate legacy-layout choice that discovery must not
    // override (keeps XDG-pinned environments and tests working even when
    // a trusted .workestrate/config.toml exists in an ancestor).
    if !xdg_explicit {
        if let Ok(cwd) = std::env::current_dir() {
            let mut dir: &Path = &cwd;
            loop {
                let candidate = dir.join(".workestrate").join("config.toml");
                if candidate.exists() {
                    if crate::config::is_dir_trusted_via_base_registry(dir) {
                        return (dir.join(".workestrate"), HomeKind::Discovered);
                    }
                    // Untrusted: warn (once per process), STOP walking, fall through.
                    emit_untrusted_discovery_warn(dir);
                    break;
                }
                match dir.parent() {
                    Some(parent) => dir = parent,
                    None => break,
                }
            }
        }
    }

    // (c) Legacy XDG
    if xdg_explicit {
        emit_legacy_xdg_note();
        return (xdg_config_dir(), HomeKind::LegacyXdg);
    }

    // (d) Default
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    (PathBuf::from(home).join(".workestrate"), HomeKind::Default)
}

/// Resolve the workestrate tool home (ADR 0023).
pub fn resolve_home() -> PathBuf {
    resolve_home_with_kind().0
}

// ---------------------------------------------------------------------------
// XDG path resolution
// ---------------------------------------------------------------------------

/// XDG config dir for workestrate: $XDG_CONFIG_HOME/workestrate/ or ~/.config/workestrate/
pub fn xdg_config_dir() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(".config")
        });
    base.join("workestrate")
}

/// XDG data dir for workestrate: $XDG_DATA_HOME/workestrate/ or ~/.local/share/workestrate/
pub fn xdg_data_dir() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(".local").join("share")
        });
    base.join("workestrate")
}

/// XDG state dir for workestrate: $XDG_STATE_HOME/workestrate/ or ~/.local/state/workestrate/
pub fn xdg_state_dir() -> PathBuf {
    let base = std::env::var("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
            PathBuf::from(home).join(".local").join("state")
        });
    base.join("workestrate")
}

/// Registry path: the active tool home's `config.toml`.
///
/// In legacy XDG mode this is `$XDG_CONFIG_HOME/workestrate/config.toml`
/// (unchanged from pre-ADR-0023); in every other mode it is `<home>/config.toml`.
pub fn registry_path() -> PathBuf {
    let (home, kind) = resolve_home_with_kind();
    match kind {
        HomeKind::LegacyXdg => xdg_config_dir().join("config.toml"),
        _ => home.join("config.toml"),
    }
}

/// Overrides path: the active tool home's `overrides.toml`.
pub fn overrides_path() -> PathBuf {
    let (home, kind) = resolve_home_with_kind();
    match kind {
        HomeKind::LegacyXdg => xdg_config_dir().join("overrides.toml"),
        _ => home.join("overrides.toml"),
    }
}

/// Config repo store: resolve_store_dir()/config-repos/<name>/
pub fn config_repo_dir(name: &str) -> PathBuf {
    resolve_store_dir().join("config-repos").join(name)
}

/// Source override store: resolve_store_dir()/sources/<name>/
pub fn source_store_dir(name: &str) -> PathBuf {
    resolve_store_dir().join("sources").join(name)
}

/// Load the registry for state/store-dir resolution, distinguishing the
/// three cases (WP10/A16):
///   - **missing** → `Ok(None)`: fall back to the default dir silently
///     (normal first-run/bootstrap state, not corruption);
///   - **valid** → `Ok(Some)`;
///   - **corrupt** (exists but fails to parse) → loud `eprintln!` WARNING and
///     `None` (fall back), so a broken registry no longer silently routes
///     state/store to the default location while the operator believes the
///     configured `settings.state_dir`/`store_dir` is in effect.
///
/// Returning `Result` (hard error) would be strictly louder, but
/// [`resolve_state_dir`]/[`resolve_store_dir`] return `PathBuf` (not
/// `Result`) and are called from main.rs/runtime.rs — changing the signature
/// is out of scope for WP10, so warn-and-fall-back is the maximal in-scope
/// surfacing. Full error propagation needs a signature change (follow-up).
fn load_registry_for_dir_resolution() -> Option<Registry> {
    match crate::config::load_registry() {
        Ok(registry) => registry,
        Err(e) => {
            eprintln!("WARNING: corrupt registry ({e:#}); ignoring it and falling back to the default state/store directory. Fix or remove the registry file, or run 'workestrate config list' to diagnose.");
            None
        }
    }
}

/// Resolve the state directory (workspaces, var, run). A registry
/// `settings.state_dir` wins; otherwise derived from the active tool home
/// (`<home>/state`, or the legacy XDG state dir in `HomeKind::LegacyXdg`
/// mode). A corrupt registry warns and falls back to the default.
pub fn resolve_state_dir() -> PathBuf {
    if let Some(registry) = load_registry_for_dir_resolution() {
        if let Some(ref state_dir) = registry.settings.state_dir {
            return expand_tilde(state_dir);
        }
    }
    let (home, kind) = resolve_home_with_kind();
    match kind {
        HomeKind::LegacyXdg => xdg_state_dir(),
        _ => home.join("state"),
    }
}

/// Resolve the store directory (managed config-repo clones under
/// `config-repos/` and source checkouts under `sources/`). A registry
/// `settings.store_dir` wins; otherwise derived from the active tool home
/// (the home dir itself, or the legacy XDG data dir in `HomeKind::LegacyXdg`
/// mode). A corrupt registry warns and falls back to the default.
pub fn resolve_store_dir() -> PathBuf {
    if let Some(registry) = load_registry_for_dir_resolution() {
        if let Some(ref store_dir) = registry.settings.store_dir {
            return expand_tilde(store_dir);
        }
    }
    let (home, kind) = resolve_home_with_kind();
    match kind {
        HomeKind::LegacyXdg => xdg_data_dir(),
        _ => home,
    }
}

/// The ONE tilde-expansion helper (FS-16; previously three divergent copies:
/// here, `commands::secrets_target::expand_tilde`, and
/// `commands::migrate::expand_user_home`).
///
/// Behavior:
/// - `~/rest` with `HOME` set      → `$HOME/rest`.
/// - `~/rest` with `HOME` UNSET    → the path is returned UNEXPANDED and a
///   one-time stderr warning is emitted. Substituting "." (the old paths.rs /
///   migrate.rs behavior) silently redirected `$HOME`-anchored paths into the
///   caller's cwd — a state-directory corruption risk in minimal containers.
/// - anything else (including a bare `~` with no trailing slash) → unchanged.
///
/// Call-site note: `secrets_target::expand_tilde` took `&Path`; the migration
/// losslessly stringifies via `to_string_lossy` at the two affected call
/// sites (paths are operator-supplied config values, always valid UTF-8 in
/// practice). The `HOME`-unset behavior is the ONLY intentional change, and
/// it changes for every former call site.
pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        match std::env::var("HOME") {
            Ok(home) if !home.is_empty() => PathBuf::from(home).join(rest),
            _ => {
                emit_tilde_home_unset_warn();
                PathBuf::from(path)
            }
        }
    } else {
        PathBuf::from(path)
    }
}

/// One-time stderr warning for the `HOME`-unset edge of [`expand_tilde`].
/// expand_tilde is called from many resolution paths per command; without the
/// guard the note would repeat on every expansion.
static TILDE_HOME_UNSET_WARN: std::sync::Once = std::sync::Once::new();

fn emit_tilde_home_unset_warn() {
    TILDE_HOME_UNSET_WARN.call_once(|| {
        eprintln!("warning: HOME is unset; leaving '~/…' paths unexpanded (not substituting '.')");
    });
}

pub(crate) fn reference_config_path() -> Option<PathBuf> {
    if let Ok(root) = crate::config::project_root() {
        let path = root.join("config.reference").join("workestrate.toml");
        if path.exists() {
            return Some(path);
        }
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut path = PathBuf::from(manifest);
        if path.pop() && path.pop() {
            let reference = path.join("config.reference").join("workestrate.toml");
            if reference.exists() {
                return Some(reference);
            }
        }
    }
    None
}

/// Resolve the directory where new config entries should be written.
///
/// Resolution order (same spirit as load_config):
/// 1. WORKESTRATE_CONFIG_DIR env var
/// 2. Trusted project ./workestrate.toml (cwd)
/// 3. Registry single layer (default config repo)
/// 4. Error: no active config repo
pub fn resolve_active_config_dir() -> anyhow::Result<PathBuf> {
    // 1. WORKESTRATE_CONFIG_DIR (must exist)
    if let Ok(dir) = std::env::var("WORKESTRATE_CONFIG_DIR") {
        let path = PathBuf::from(dir);
        if path.exists() {
            return Ok(path);
        }
    }

    // 2. Trusted project (cwd)
    let skip_project = std::env::var("WORKESTRATE_NO_PROJECT_CONFIG").is_ok();
    if !skip_project {
        let cwd = std::env::current_dir()?;
        let project_path = cwd.join("workestrate.toml");
        if project_path.exists() {
            match crate::config::load_registry()? {
                Some(_) => {
                    if crate::config::is_trusted_project(&cwd) {
                        return Ok(cwd);
                    }
                }
                None => {
                    return Ok(cwd);
                }
            }
        }
    }

    // 3. Registry context's first layer
    if let Ok(Some(_registry)) = crate::config::load_registry() {
        let active_context = crate::config::resolve_active_context()?;
        if let Some(name) = active_context.layers.first() {
            return Ok(resolve_store_dir().join("config-repos").join(name));
        }
    }

    anyhow::bail!(
        "no active config repo; run 'workestrate init' or 'workestrate config add <url> <name>' first"
    );
}

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
    use anyhow::Result;

    #[test]
    fn resolve_home_env_wins() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let env_home = uniq_dir("rh-env");
        let xdg = uniq_dir("rh-env-xdg");
        std::fs::create_dir_all(&env_home)?;
        std::env::set_var("HOME", &env_home);
        std::env::set_var("XDG_CONFIG_HOME", &xdg);
        std::env::set_var("WORKESTRATE_HOME", &env_home);

        let (home, kind) = resolve_home_with_kind();
        assert_eq!(kind, HomeKind::Env, "WORKESTRATE_HOME must win over XDG");
        assert_eq!(home, env_home);

        let _ = std::fs::remove_dir_all(&env_home);
        let _ = std::fs::remove_dir_all(&xdg);
        Ok(())
    }

    #[test]
    fn resolve_home_discovery_trusted() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let base_home = uniq_dir("rh-disc-base");
        let project = uniq_dir("rh-disc-proj");
        std::fs::create_dir_all(base_home.join(".workestrate"))?;
        std::fs::create_dir_all(project.join(".workestrate"))?;

        // Seed a project-local config.toml so discovery notices it.
        std::fs::write(
            project.join(".workestrate").join("config.toml"),
            "layers = []\n",
        )?;

        // Trust list lives in the Default base registry (<HOME>/.workestrate).
        let canonical_project = std::fs::canonicalize(&project)?;
        let trust_toml = format!(
            "[[trusted_projects]]\npath = \"{}\"\n",
            canonical_project.display()
        );
        std::fs::write(
            base_home.join(".workestrate").join("config.toml"),
            trust_toml,
        )?;

        std::env::set_var("HOME", &base_home);
        std::env::set_current_dir(&project)?;

        let (home, kind) = resolve_home_with_kind();
        assert_eq!(kind, HomeKind::Discovered);
        assert_eq!(home, project.join(".workestrate"));

        let _ = std::fs::remove_dir_all(&base_home);
        let _ = std::fs::remove_dir_all(&project);
        Ok(())
    }

    /// Precedence regression: an explicit XDG var must win over discovery even
    /// when a *trusted* `.workestrate/config.toml` exists in the cwd. Without
    /// this guarantee, discovery overrides a deliberately-pinned legacy layout
    /// whenever a trusted project config is present in an ancestor.
    #[test]
    fn discovery_does_not_override_explicit_xdg() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let base_home = uniq_dir("rh-xdg-base");
        let project = uniq_dir("rh-xdg-proj");
        let xdg = uniq_dir("rh-xdg-explicit");
        std::fs::create_dir_all(base_home.join(".workestrate"))?;
        std::fs::create_dir_all(project.join(".workestrate"))?;
        std::fs::create_dir_all(&xdg)?;

        // Project-local config.toml so discovery WOULD notice it if it ran.
        std::fs::write(
            project.join(".workestrate").join("config.toml"),
            "layers = []\n",
        )?;

        // Base registry at <HOME>/.workestrate trusts the project dir, so
        // discovery would return Discovered if it were allowed to run.
        let canonical_project = std::fs::canonicalize(&project)?;
        let trust_toml = format!(
            "[[trusted_projects]]\npath = \"{}\"\n",
            canonical_project.display()
        );
        std::fs::write(
            base_home.join(".workestrate").join("config.toml"),
            trust_toml,
        )?;

        std::env::set_var("HOME", &base_home);
        std::env::set_var("XDG_CONFIG_HOME", &xdg);
        std::env::remove_var("WORKESTRATE_HOME");
        std::env::set_current_dir(&project)?;

        let (_home, kind) = resolve_home_with_kind();
        assert_ne!(
            kind,
            HomeKind::Discovered,
            "explicit XDG var must override discovery even for a trusted project"
        );
        assert_eq!(kind, HomeKind::LegacyXdg);

        let _ = std::fs::remove_dir_all(&base_home);
        let _ = std::fs::remove_dir_all(&project);
        let _ = std::fs::remove_dir_all(&xdg);
        Ok(())
    }

    #[test]
    fn resolve_home_discovery_untrusted_ignored() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let base_home = uniq_dir("rh-untr-base");
        let project = uniq_dir("rh-untr-proj");
        std::fs::create_dir_all(base_home.join(".workestrate"))?;
        std::fs::create_dir_all(project.join(".workestrate"))?;
        std::fs::write(
            project.join(".workestrate").join("config.toml"),
            "layers = []\n",
        )?;
        // Base registry exists but does NOT trust the project.
        std::fs::write(
            base_home.join(".workestrate").join("config.toml"),
            "[[trusted_projects]]\npath = \"/some/other/dir\"\n",
        )?;

        std::env::set_var("HOME", &base_home);
        std::env::set_current_dir(&project)?;

        let (home, kind) = resolve_home_with_kind();
        assert_ne!(
            kind,
            HomeKind::Discovered,
            "untrusted .workestrate must be ignored"
        );
        // Fell through to Default (<HOME>/.workestrate).
        assert_eq!(kind, HomeKind::Default);
        assert_eq!(home, base_home.join(".workestrate"));

        let _ = std::fs::remove_dir_all(&base_home);
        let _ = std::fs::remove_dir_all(&project);
        Ok(())
    }

    #[test]
    fn resolve_home_legacy_xdg_when_xdg_set() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let xdg = uniq_dir("rh-xdg");
        let home = uniq_dir("rh-xdg-home");
        std::fs::create_dir_all(&xdg)?;
        std::env::set_var("HOME", &home);
        std::env::set_var("XDG_CONFIG_HOME", &xdg);

        let (resolved, kind) = resolve_home_with_kind();
        assert_eq!(kind, HomeKind::LegacyXdg);
        assert_eq!(resolved, xdg.join("workestrate"));

        let _ = std::fs::remove_dir_all(&xdg);
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn resolve_home_default_when_nothing_set() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let home = uniq_dir("rh-default-home");
        std::fs::create_dir_all(&home)?;
        std::env::set_var("HOME", &home);

        let (resolved, kind) = resolve_home_with_kind();
        assert_eq!(kind, HomeKind::Default);
        assert_eq!(resolved, home.join(".workestrate"));

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn legacy_xdg_registry_path_compat() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let xdg = uniq_dir("rh-compat");
        std::fs::create_dir_all(&xdg)?;
        std::env::set_var("XDG_CONFIG_HOME", &xdg);

        // Backward-compat guarantee: existing XDG-pinned callers see the
        // same registry path as before ADR 0023.
        assert_eq!(registry_path(), xdg.join("workestrate").join("config.toml"));

        let _ = std::fs::remove_dir_all(&xdg);
        Ok(())
    }

    // ---- WP10/A16: corrupt registry is loud (warn) but falls back ----

    #[test]
    fn corrupt_registry_falls_back_to_default_dirs() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let home = uniq_dir("a16-corrupt-home");
        std::fs::create_dir_all(home.join(".workestrate"))?;
        // Corrupt registry: invalid TOML.
        std::fs::write(
            home.join(".workestrate").join("config.toml"),
            "this is = not = valid toml [[[",
        )?;
        std::env::set_var("HOME", &home);

        // load_registry_for_dir_resolution must yield None (fallback), not
        // panic — the corrupt file surfaces as a WARNING on stderr.
        assert!(
            load_registry_for_dir_resolution().is_none(),
            "corrupt registry must fall back to None"
        );
        // The dir resolvers still return sane default paths.
        assert_eq!(resolve_state_dir(), home.join(".workestrate").join("state"));
        assert_eq!(resolve_store_dir(), home.join(".workestrate"));

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn valid_registry_is_used_for_dir_resolution() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let home = uniq_dir("a16-valid-home");
        let custom_state = uniq_dir("a16-custom-state");
        std::fs::create_dir_all(home.join(".workestrate"))?;
        std::fs::write(
            home.join(".workestrate").join("config.toml"),
            format!("[settings]\nstate_dir = \"{}\"\n", custom_state.display()),
        )?;
        std::env::set_var("HOME", &home);

        assert!(
            load_registry_for_dir_resolution().is_some(),
            "valid registry must parse"
        );
        assert_eq!(resolve_state_dir(), custom_state);

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    // ---- FS-16: one canonical expand_tilde ----

    /// `~/rest` expands against `$HOME` (the common path, unchanged).
    #[test]
    fn expand_tilde_expands_against_home() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let home = uniq_dir("tilde-home");
        std::fs::create_dir_all(&home)?;
        std::env::set_var("HOME", &home);

        assert_eq!(expand_tilde("~/foo/bar"), home.join("foo").join("bar"));

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Non-tilde inputs pass through verbatim (relative, absolute, and a
    /// bare `~` with no trailing slash — none of the three legacy impls
    /// expanded a bare `~`, and the consolidated helper keeps that).
    #[test]
    fn expand_tilde_passes_through_non_tilde_prefixes() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        std::env::set_var("HOME", "/definitely/not/used");
        assert_eq!(expand_tilde("/abs/path"), PathBuf::from("/abs/path"));
        assert_eq!(expand_tilde("rel/path"), PathBuf::from("rel/path"));
        assert_eq!(expand_tilde("~"), PathBuf::from("~"));
        assert_eq!(expand_tilde("~other/x"), PathBuf::from("~other/x"));
        Ok(())
    }

    /// FS-16 behavior change: with HOME unset, the path is returned
    /// UNEXPANDED (never "."-substituted). The old paths.rs / migrate.rs
    /// impls substituted "." — silently redirecting `$HOME`-anchored state
    /// into the cwd; secrets_target's impl returned the literal path. The
    /// consolidated helper follows the secrets_target behavior for every
    /// caller.
    #[test]
    fn expand_tilde_home_unset_returns_unexpanded() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        std::env::remove_var("HOME");
        let expanded = expand_tilde("~/some/state");
        assert_eq!(
            expanded,
            PathBuf::from("~/some/state"),
            "HOME-unset must return the path unexpanded, never '.'"
        );
        Ok(())
    }

    #[test]
    fn missing_registry_falls_back_silently() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);

        let home = uniq_dir("a16-missing-home");
        std::fs::create_dir_all(&home)?; // no .workestrate/config.toml at all
        std::env::set_var("HOME", &home);

        assert!(
            load_registry_for_dir_resolution().is_none(),
            "missing registry → None (normal bootstrap, silent)"
        );
        assert_eq!(resolve_store_dir(), home.join(".workestrate"));

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }
}
