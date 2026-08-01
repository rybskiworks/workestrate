//! Tool home resolution (ADR 0023 single-home layout), XDG path resolution,
//! and state/store directory derivation.

use std::path::PathBuf;

use crate::config::types::Registry;

// ---------------------------------------------------------------------------
// Tool home resolution (ADR 0023 single-home layout)
// ---------------------------------------------------------------------------

/// How the workestrate tool home was resolved (ADR 0023 single-home layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeKind {
    /// `WORKESTRATE_HOME` env var — new single-home layout.
    Env,
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

fn xdg_var_set(name: &str) -> bool {
    std::env::var(name).map(|v| !v.is_empty()).unwrap_or(false)
}

/// Base home resolution (Env/LegacyXdg/Default only).
///
/// Used by the base-registry trust check ([`is_dir_trusted_via_base_registry`])
/// so that loading the global trust registry cannot recurse back through home
/// resolution. (The discovery tier that originally motivated this split was
/// removed in spec 08 step (e); the base resolution is kept because the trust
/// registry check still uses it.)
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

/// The registry path computed from the *base* resolution.
///
/// Location of the global trust list, resolved via the base home resolution
/// (Env/LegacyXdg/Default) so that loading the trust registry never recurses
/// through home resolution.
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
/// 2. **LegacyXdg** — any of `XDG_CONFIG_HOME`/`XDG_DATA_HOME`/`XDG_STATE_HOME`
///    set and non-empty (compatibility; emits a one-time migration note).
/// 3. **Default** — `~/.workestrate`.
///
/// The trusted-ancestor auto-discovery tier was removed (spec 08 step (e);
/// see `docs/validation-and-improvements/06-improvements/08-no-repo-local-home.md`
/// and spec 10 `10-config-repos-as-working-copies.md`): repo-local homes and
/// discovery caused split-brain/shadow-home ambiguity.
pub fn resolve_home_with_kind() -> (PathBuf, HomeKind) {
    // (a) Env: WORKESTRATE_HOME
    if let Ok(value) = std::env::var("WORKESTRATE_HOME") {
        if !value.is_empty() {
            return (expand_tilde(&value), HomeKind::Env);
        }
    }

    // (b) Legacy XDG
    let xdg_explicit = xdg_var_set("XDG_CONFIG_HOME")
        || xdg_var_set("XDG_DATA_HOME")
        || xdg_var_set("XDG_STATE_HOME");
    if xdg_explicit {
        emit_legacy_xdg_note();
        return (xdg_config_dir(), HomeKind::LegacyXdg);
    }

    // (c) Default
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

/// Resolve the state directory (workspaces, var, run).
///
/// Precedence (first match wins):
/// 1. **`WORKESTRATE_STATE_DIR` env var** (highest; additive escape hatch —
///    used by hermetic tests, e.g. the parallel-slot golden plan, to isolate
///    the port registry from the real dev home without touching the
///    registry). Used verbatim (leading `~/` expanded).
/// 2. A registry `settings.state_dir`.
/// 3. Derived from the active tool home (`<home>/state`, or the legacy XDG
///    state dir in `HomeKind::LegacyXdg` mode). A corrupt registry warns and
///    falls back to the default.
pub fn resolve_state_dir() -> PathBuf {
    if let Ok(value) = std::env::var("WORKESTRATE_STATE_DIR") {
        if !value.is_empty() {
            return expand_tilde(&value);
        }
    }
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

/// One-time stderr warning for the opt-in cwd-derived reference load
/// (spec 05 option (c), companion to the option (d) gate below).
static CWD_REFERENCE_LOADED_WARN: std::sync::Once = std::sync::Once::new();

/// One-time stderr note for the gated (not-loaded) cwd-derived reference
/// (spec 05 option (d)).
static CWD_REFERENCE_IGNORED_NOTE: std::sync::Once = std::sync::Once::new();

/// Resolve the reference config (`<root>/config.reference/workestrate.toml`),
/// the base layer of every non-bypassed `load_config()`.
///
/// Spec-05 security gate (fix options (d) + (c)): when the project root was
/// resolved from the **current working directory** (tier 3 of
/// `project_root_with_source()` — i.e. neither `AGENTCTL_ROOT` nor
/// `CARGO_MANIFEST_DIR` was set), the cwd-derived reference is only loaded
/// with the explicit opt-in `WORKESTRATE_ALLOW_CWD_REFERENCE=1`. Without the
/// opt-in the path is ignored (a one-time stderr note is emitted) and the
/// function falls through to the `CARGO_MANIFEST_DIR` probe. With the opt-in
/// the path is loaded and a one-time stderr warning names the cwd source.
/// Roots pinned via `AGENTCTL_ROOT` or `CARGO_MANIFEST_DIR` never require the
/// opt-in.
pub(crate) fn reference_config_path() -> Option<PathBuf> {
    if let Ok((root, source)) = crate::config::project_root_with_source() {
        let path = root.join("config.reference").join("workestrate.toml");
        if path.exists() {
            match source {
                crate::config::RootSource::AgentctlRoot
                | crate::config::RootSource::ManifestDir => {
                    return Some(path);
                }
                crate::config::RootSource::Cwd => {
                    if std::env::var("WORKESTRATE_ALLOW_CWD_REFERENCE").as_deref() == Ok("1") {
                        CWD_REFERENCE_LOADED_WARN.call_once(|| {
                            eprintln!(
                                "warning: reference config loaded from cwd '{}'; set AGENTCTL_ROOT to pin the root",
                                path.display()
                            );
                        });
                        return Some(path);
                    }
                    CWD_REFERENCE_IGNORED_NOTE.call_once(|| {
                        eprintln!(
                            "note: ignoring {} found in cwd (set WORKESTRATE_ALLOW_CWD_REFERENCE=1 to use it)",
                            path.display()
                        );
                    });
                    // Fall through to the CARGO_MANIFEST_DIR probe below.
                }
            }
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

    // ---- WORKESTRATE_STATE_DIR override (C2; hermetic registry isolation) ----

    /// The env override is the HIGHEST-precedence step: it wins over a
    /// registry `settings.state_dir` AND over the home-derived default.
    #[test]
    fn state_dir_env_override_wins_over_registry_and_default() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        // WORKESTRATE_STATE_DIR is not in HOME_ENV_KEYS; guard it manually.
        let old_state = std::env::var("WORKESTRATE_STATE_DIR").ok();

        let home = uniq_dir("sd-override-home");
        let registry_state = uniq_dir("sd-override-registry");
        let env_state = uniq_dir("sd-override-env");
        std::fs::create_dir_all(home.join(".workestrate"))?;
        std::fs::write(
            home.join(".workestrate").join("config.toml"),
            format!("[settings]\nstate_dir = \"{}\"\n", registry_state.display()),
        )?;
        std::env::set_var("HOME", &home);
        std::env::set_var("WORKESTRATE_STATE_DIR", &env_state);

        assert_eq!(
            resolve_state_dir(),
            env_state,
            "WORKESTRATE_STATE_DIR must win over registry settings.state_dir"
        );

        // Unset → the registry setting takes over again.
        std::env::remove_var("WORKESTRATE_STATE_DIR");
        assert_eq!(resolve_state_dir(), registry_state);

        match old_state {
            Some(v) => std::env::set_var("WORKESTRATE_STATE_DIR", v),
            None => std::env::remove_var("WORKESTRATE_STATE_DIR"),
        }
        let _ = std::fs::remove_dir_all(&home);
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

    // ---- Spec 05: cwd-derived reference config is opt-in gated ----

    /// Env keys every spec-05 test captures/clears. CARGO_MANIFEST_DIR is
    /// removed in tests 1–3 so tier 2 cannot win over the cwd tier;
    /// WORKESTRATE_CONFIG_DIR is removed so load-layer bypass cannot mask the
    /// reference resolution.
    const SPEC05_ENV_KEYS: &[&str] = &[
        "AGENTCTL_ROOT",
        "CARGO_MANIFEST_DIR",
        "WORKESTRATE_ALLOW_CWD_REFERENCE",
        "WORKESTRATE_CONFIG_DIR",
    ];

    /// Build a tempdir shaped like a workbench checkout (flake.nix +
    /// config.reference/workestrate.toml), chdir into it, and clear the
    /// spec-05 env keys. Returns the tempdir path (caller cleans up).
    fn spec05_cwd_fixture(label: &str) -> Result<PathBuf> {
        let tmp = uniq_dir(label);
        std::fs::create_dir_all(tmp.join("config.reference"))?;
        std::fs::write(tmp.join("flake.nix"), "")?;
        std::fs::write(
            tmp.join("config.reference").join("workestrate.toml"),
            "schema_version = 1\n",
        )?;
        std::env::set_current_dir(&tmp)?;
        for k in SPEC05_ENV_KEYS {
            std::env::remove_var(k);
        }
        Ok(tmp)
    }

    #[test]
    fn cwd_reference_ignored_without_opt_in() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(SPEC05_ENV_KEYS);

        let tmp = spec05_cwd_fixture("spec05-no-opt-in")?;

        // Root resolves from cwd (tier 3); without the opt-in env the
        // cwd-derived reference must NOT be returned. CARGO_MANIFEST_DIR is
        // removed, so the manifest fallback below cannot win either.
        assert_eq!(
            reference_config_path(),
            None,
            "cwd-derived reference must be ignored without WORKESTRATE_ALLOW_CWD_REFERENCE=1"
        );

        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn cwd_reference_loaded_with_opt_in() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(SPEC05_ENV_KEYS);

        let tmp = spec05_cwd_fixture("spec05-opt-in")?;
        std::env::set_var("WORKESTRATE_ALLOW_CWD_REFERENCE", "1");

        assert_eq!(
            reference_config_path(),
            Some(tmp.join("config.reference").join("workestrate.toml")),
            "cwd-derived reference must load with WORKESTRATE_ALLOW_CWD_REFERENCE=1"
        );

        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn agentctl_root_tier_unaffected() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(SPEC05_ENV_KEYS);

        let tmp = spec05_cwd_fixture("spec05-pinned-root")?;
        // A pinned root never requires the opt-in.
        std::env::set_var("AGENTCTL_ROOT", &tmp);

        assert_eq!(
            reference_config_path(),
            Some(tmp.join("config.reference").join("workestrate.toml")),
            "AGENTCTL_ROOT-pinned reference must load without the cwd opt-in"
        );

        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn manifest_dir_tier_unaffected() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(SPEC05_ENV_KEYS);

        // Empty tempdir as cwd (NO flake.nix, NO config.reference) so tier 3
        // cannot resolve; CARGO_MANIFEST_DIR (tier 2 / manifest fallback)
        // resolves the real repo fixture without any opt-in.
        let tmp = uniq_dir("spec05-manifest-tier");
        std::fs::create_dir_all(&tmp)?;
        std::env::set_current_dir(&tmp)?;
        std::env::remove_var("AGENTCTL_ROOT");
        std::env::remove_var("WORKESTRATE_ALLOW_CWD_REFERENCE");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");
        std::env::set_var("CARGO_MANIFEST_DIR", env!("CARGO_MANIFEST_DIR"));

        let path = reference_config_path()
            .expect("manifest-tier reference must resolve without the cwd opt-in");
        assert!(
            path.ends_with("config.reference/workestrate.toml"),
            "expected …/config.reference/workestrate.toml, got {}",
            path.display()
        );
        assert!(path.exists(), "{} must exist", path.display());

        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }
}
