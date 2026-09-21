//! Config resolution (ADR 0023 single-config layout), XDG path resolution,
//! and state/store directory derivation.

use std::path::PathBuf;

use crate::config::types::Registry;

// ---------------------------------------------------------------------------
// Config resolution (ADR 0023 single-config layout)
// ---------------------------------------------------------------------------

/// How the workestrate config was resolved (ADR 0023 single-config layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigDirKind {
    /// `WORKESTRATE_CONFIG` env var — new single-config layout.
    Env,
    /// Legacy XDG layout (`XDG_*_HOME` set) — compatibility, read/write as before.
    LegacyXdg,
    /// Default `~/.workestrate` — new single-config layout.
    Default,
}

/// One-time stderr migration note for the legacy XDG layout.
static LEGACY_NOTE: std::sync::Once = std::sync::Once::new();

fn emit_legacy_xdg_note() {
    LEGACY_NOTE.call_once(|| {
        eprintln!(
            "note: using legacy XDG workestrate layout; run 'workestrate migrate-config' to \
             consolidate into a single WORKESTRATE_CONFIG"
        );
    });
}

fn xdg_var_set(name: &str) -> bool {
    std::env::var(name).map(|v| !v.is_empty()).unwrap_or(false)
}

/// Base config resolution (Env/LegacyXdg/Default only).
///
/// Used by the base-registry trust check ([`is_dir_trusted_via_base_registry`])
/// so that loading the global trust registry cannot recurse back through config
/// resolution. (The discovery tier that originally motivated this split was
/// removed in spec 08 step (e); the base resolution is kept because the trust
/// registry check still uses it.)
fn resolve_config_dir_base_with_kind() -> (PathBuf, ConfigDirKind) {
    // (a) Env: WORKESTRATE_CONFIG
    if let Ok(value) = std::env::var("WORKESTRATE_CONFIG")
        && !value.is_empty()
    {
        return (expand_tilde(&value), ConfigDirKind::Env);
    }
    // (c) Legacy XDG
    if xdg_var_set("XDG_CONFIG_HOME")
        || xdg_var_set("XDG_DATA_HOME")
        || xdg_var_set("XDG_STATE_HOME")
    {
        return (xdg_config_dir(), ConfigDirKind::LegacyXdg);
    }
    // (d) Default
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    (
        PathBuf::from(home).join(".workestrate"),
        ConfigDirKind::Default,
    )
}

/// The registry path computed from the *base* resolution.
///
/// Location of the global trust list, resolved via the base config resolution
/// (Env/LegacyXdg/Default) so that loading the trust registry never recurses
/// through config resolution.
pub(crate) fn base_registry_path() -> PathBuf {
    let (config_dir, kind) = resolve_config_dir_base_with_kind();
    match kind {
        ConfigDirKind::LegacyXdg => xdg_config_dir().join("config.toml"),
        _ => config_dir.join("config.toml"),
    }
}

/// Resolve the workestrate config and how it was chosen (ADR 0023).
///
/// Precedence (first match wins):
/// 1. **Env** — `WORKESTRATE_CONFIG` (used verbatim, leading `~/` expanded).
/// 2. **LegacyXdg** — any of `XDG_CONFIG_HOME`/`XDG_DATA_HOME`/`XDG_STATE_HOME`
///    set and non-empty (compatibility; emits a one-time migration note).
/// 3. **Default** — `~/.workestrate`.
///
/// The trusted-ancestor auto-discovery tier was removed (spec 08 step (e);
/// see `docs/validation-and-improvements/06-improvements/08-no-repo-local-home.md`
/// and spec 10 `10-fleets-as-working-copies.md`): repo-local configs and
/// discovery caused split-brain/shadow-config ambiguity.
pub fn resolve_config_dir_with_kind() -> (PathBuf, ConfigDirKind) {
    // (a) Env: WORKESTRATE_CONFIG
    if let Ok(value) = std::env::var("WORKESTRATE_CONFIG")
        && !value.is_empty()
    {
        return (expand_tilde(&value), ConfigDirKind::Env);
    }

    // (b) Legacy XDG
    let xdg_explicit = xdg_var_set("XDG_CONFIG_HOME")
        || xdg_var_set("XDG_DATA_HOME")
        || xdg_var_set("XDG_STATE_HOME");
    if xdg_explicit {
        emit_legacy_xdg_note();
        return (xdg_config_dir(), ConfigDirKind::LegacyXdg);
    }

    // (c) Default
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    (
        PathBuf::from(home).join(".workestrate"),
        ConfigDirKind::Default,
    )
}

/// Resolve the workestrate config (ADR 0023).
pub fn resolve_config_dir() -> PathBuf {
    resolve_config_dir_with_kind().0
}

// ---------------------------------------------------------------------------
// Invocation cwd capture (wrong-CWD `${CWD}` mount fix)
// ---------------------------------------------------------------------------

/// Env var holding the operator's invocation cwd, captured ONCE at CLI entry
/// (`main()`) and inherited by re-exec'd / detached children.
///
/// Bug class: `${CWD}` mount-host resolution (and every other "caller's PWD"
/// read — project-config discovery, content-root fallbacks) used to call
/// `std::env::current_dir()` LAZILY at plan-materialization time. Any process
/// in the chain whose cwd differed from the operator's invocation cwd (a
/// re-exec'd or detached child, a wrapper that chdirs) silently retargeted
/// `host = "${CWD}" → /work` at the WRONG directory — and because msb
/// persists sandbox mounts, the wrong create was then re-served by
/// `ChainStep::Reuse`/`StartExisting`.
pub const INVOKE_CWD_ENV: &str = "WORKESTRATE_INVOKE_CWD";

/// The operator's invocation working directory.
///
/// Returns the captured [`INVOKE_CWD_ENV`] value when it is set, non-empty,
/// and absolute (the common case: `main()` captured it at entry, or a parent
/// process passed it down). Otherwise falls back to
/// `std::env::current_dir()` — preserving the pre-capture behavior for
/// library and test callers that never ran `main()`.
pub fn invoke_cwd() -> Option<PathBuf> {
    if let Ok(value) = std::env::var(INVOKE_CWD_ENV) {
        let path = PathBuf::from(&value);
        if !value.is_empty() && path.is_absolute() {
            return Some(path);
        }
    }
    std::env::current_dir().ok()
}

/// Hard-error variant of [`invoke_cwd`] for call sites that previously used
/// `std::env::current_dir()?`: no resolvable cwd is an error, not a silent
/// fallback.
pub fn invoke_cwd_or_err() -> anyhow::Result<PathBuf> {
    invoke_cwd()
        .ok_or_else(|| anyhow::anyhow!("failed to resolve the invocation working directory"))
}

/// The CANONICALIZED invocation cwd as a string (ADR 0030 V-addendum §V1):
/// `fs::canonicalize` resolves symlinks, `..`, and trailing slashes so every
/// spelling of one directory keys the SAME `per-dir` instance id (and two
/// different directories never share one). Non-UTF-8 paths are rendered
/// lossy — deterministic, and such paths are pathological for this
/// mechanism.
pub fn canonical_invoke_cwd_string() -> anyhow::Result<String> {
    let canonical = std::fs::canonicalize(invoke_cwd_or_err()?)?;
    Ok(canonical.to_string_lossy().into_owned())
}

/// Capture the invocation cwd into [`INVOKE_CWD_ENV`] if not already set.
///
/// Called as the FIRST thing in `main()`, before argv handling, so every
/// downstream "caller's PWD" read observes the operator's directory. An
/// INHERITED value WINS: a re-exec'd or detached child keeps the ORIGINAL
/// operator cwd rather than re-capturing its own. If `current_dir()` fails
/// the var is left unset and readers fall back per [`invoke_cwd`].
#[allow(unsafe_code)]
pub fn ensure_invoke_cwd_env() {
    if std::env::var_os(INVOKE_CWD_ENV).is_some() {
        return;
    }
    if let Ok(cwd) = std::env::current_dir() {
        // SAFETY: called as the FIRST thing in main(), before the tokio
        // runtime and any threads exist; no concurrent env access.
        unsafe { std::env::set_var(INVOKE_CWD_ENV, cwd) };
    }
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

/// Registry path: the active config's `config.toml`.
///
/// In legacy XDG mode this is `$XDG_CONFIG_HOME/workestrate/config.toml`
/// (unchanged from pre-ADR-0023); in every other mode it is `<config>/config.toml`.
pub fn registry_path() -> PathBuf {
    let (config_dir, kind) = resolve_config_dir_with_kind();
    match kind {
        ConfigDirKind::LegacyXdg => xdg_config_dir().join("config.toml"),
        _ => config_dir.join("config.toml"),
    }
}

/// Overrides path: the active config's `overrides.toml`.
pub fn overrides_path() -> PathBuf {
    let (config_dir, kind) = resolve_config_dir_with_kind();
    match kind {
        ConfigDirKind::LegacyXdg => xdg_config_dir().join("overrides.toml"),
        _ => config_dir.join("overrides.toml"),
    }
}

/// Fleet store: resolve_store_dir()/fleets/<name>/
pub fn fleet_dir(name: &str) -> PathBuf {
    resolve_store_dir().join("fleets").join(name)
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
            eprintln!(
                "WARNING: corrupt registry ({e:#}); ignoring it and falling back to the default state/store directory. Fix or remove the registry file, or run 'workestrate fleet list' to diagnose."
            );
            None
        }
    }
}

/// Resolve the state directory (workspaces, var, run).
///
/// Precedence (first match wins):
/// 1. **`WORKESTRATE_STATE_DIR` env var** (highest; additive escape hatch —
///    used by hermetic tests, e.g. the parallel-slot golden plan, to isolate
///    the port registry from the real dev config without touching the
///    registry). Used verbatim (leading `~/` expanded).
/// 2. A registry `settings.state_dir`.
/// 3. Derived from the active config (`<config>/state`, or the legacy XDG
///    state dir in `ConfigDirKind::LegacyXdg` mode). A corrupt registry warns and
///    falls back to the default.
pub fn resolve_state_dir() -> PathBuf {
    if let Ok(value) = std::env::var("WORKESTRATE_STATE_DIR")
        && !value.is_empty()
    {
        return expand_tilde(&value);
    }
    if let Some(registry) = load_registry_for_dir_resolution()
        && let Some(ref state_dir) = registry.settings.state_dir
    {
        return expand_tilde(state_dir);
    }
    let (config_dir, kind) = resolve_config_dir_with_kind();
    match kind {
        ConfigDirKind::LegacyXdg => xdg_state_dir(),
        _ => config_dir.join("state"),
    }
}

/// Resolve the store directory (managed fleet clones under
/// `fleets/` and source checkouts under `sources/`). A registry
/// `settings.store_dir` wins; otherwise derived from the active config
/// (the config dir itself, or the legacy XDG data dir in `ConfigDirKind::LegacyXdg`
/// mode). A corrupt registry warns and falls back to the default.
pub fn resolve_store_dir() -> PathBuf {
    if let Some(registry) = load_registry_for_dir_resolution()
        && let Some(ref store_dir) = registry.settings.store_dir
    {
        return expand_tilde(store_dir);
    }
    let (config_dir, kind) = resolve_config_dir_with_kind();
    match kind {
        ConfigDirKind::LegacyXdg => xdg_data_dir(),
        _ => config_dir,
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

/// One-time stderr note for the cleanup-phase-2 opt-in gate: the reference
/// config base layer is skipped unless `WORKESTRATE_REFERENCE_CONFIG=1`.
static REFERENCE_CONFIG_IGNORED_NOTE: std::sync::Once = std::sync::Once::new();

/// Resolve the reference config (`<root>/config.reference/workestrate.toml`),
/// the OPT-IN base layer of non-bypassed `load_config()` calls.
///
/// Cleanup-phase-2 gate (decoupling runtime gates from personal workflow
/// content): the reference base layer is only resolved when the explicit
/// opt-in `WORKESTRATE_REFERENCE_CONFIG=1` is set. Without the opt-in this
/// function returns `None`; when a reference config actually exists at the
/// resolved root, a one-time stderr note names the skipped path and the
/// opt-in escape hatch (standalone installs with no reference config
/// anywhere stay silent).
///
/// Spec-05 security gate (fix options (d) + (c)) — layered ON TOP of the
/// phase-2 opt-in: when the project root was resolved from the **current
/// working directory** (tier 3 of `project_root_with_source()` — i.e.
/// neither `AGENTCTL_ROOT` nor `CARGO_MANIFEST_DIR` was set), the cwd-derived
/// reference is only loaded with the additional explicit opt-in
/// `WORKESTRATE_ALLOW_CWD_REFERENCE=1`. Without it the path is ignored (a
/// one-time stderr note is emitted) and the function falls through to the
/// `CARGO_MANIFEST_DIR` probe. With it the path is loaded and a one-time
/// stderr warning names the cwd source. Roots pinned via `AGENTCTL_ROOT` or
/// `CARGO_MANIFEST_DIR` never require the cwd opt-in — only the phase-2
/// `WORKESTRATE_REFERENCE_CONFIG=1` flag.
pub(crate) fn reference_config_path() -> Option<PathBuf> {
    if std::env::var("WORKESTRATE_REFERENCE_CONFIG").as_deref() != Ok("1") {
        // Only note when a reference config genuinely exists at the resolved
        // root — a standalone-installed tool with no reference anywhere must
        // not spam this note on every command.
        if let Ok((root, _)) = crate::config::project_root_with_source() {
            let path = root.join("config.reference").join("workestrate.toml");
            if path.exists() {
                REFERENCE_CONFIG_IGNORED_NOTE.call_once(|| {
                    eprintln!(
                        "note: ignoring reference config {} (set WORKESTRATE_REFERENCE_CONFIG=1 to include it as the base layer)",
                        path.display()
                    );
                });
            }
        }
        return None;
    }
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
/// 1. WORKESTRATE_FLEET_DIR env var
/// 2. Trusted project ./workestrate.toml (cwd)
/// 3. Registry single layer (default fleet)
/// 4. Error: no active fleet
pub fn resolve_active_fleet_dir() -> anyhow::Result<PathBuf> {
    // 1. WORKESTRATE_FLEET_DIR (must exist)
    if let Ok(dir) = std::env::var("WORKESTRATE_FLEET_DIR") {
        let path = PathBuf::from(dir);
        if path.exists() {
            return Ok(path);
        }
    }

    // 2. Trusted project (the operator's invocation cwd)
    let skip_project = std::env::var("WORKESTRATE_NO_PROJECT_CONFIG").is_ok();
    if !skip_project {
        let cwd = invoke_cwd_or_err()?;
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

    // 3. Active fleet's first layer
    if let Ok(Some(_registry)) = crate::config::load_registry() {
        let active_fleet = crate::config::resolve_active_fleet()?;
        if let Some(name) = active_fleet.layers.first() {
            return Ok(resolve_store_dir().join("fleets").join(name));
        }
    }

    anyhow::bail!(
        "no active fleet; run 'workestrate config init' or 'workestrate fleet add <url> <name>' first"
    );
}

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
    use anyhow::Result;

    #[test]
    fn resolve_config_env_wins() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);

        let env_home = uniq_dir("rh-env");
        let xdg = uniq_dir("rh-env-xdg");
        std::fs::create_dir_all(&env_home)?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", &env_home) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &xdg) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG", &env_home) };

        let (config_dir, kind) = resolve_config_dir_with_kind();
        assert_eq!(
            kind,
            ConfigDirKind::Env,
            "WORKESTRATE_CONFIG must win over XDG"
        );
        assert_eq!(config_dir, env_home);

        let _ = std::fs::remove_dir_all(&env_home);
        let _ = std::fs::remove_dir_all(&xdg);
        Ok(())
    }

    #[test]
    fn resolve_config_legacy_xdg_when_xdg_set() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);

        let xdg = uniq_dir("rh-xdg");
        let home = uniq_dir("rh-xdg-home");
        std::fs::create_dir_all(&xdg)?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", &home) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &xdg) };

        let (resolved, kind) = resolve_config_dir_with_kind();
        assert_eq!(kind, ConfigDirKind::LegacyXdg);
        assert_eq!(resolved, xdg.join("workestrate"));

        let _ = std::fs::remove_dir_all(&xdg);
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn resolve_config_default_when_nothing_set() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);

        let home = uniq_dir("rh-default-home");
        std::fs::create_dir_all(&home)?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", &home) };

        let (resolved, kind) = resolve_config_dir_with_kind();
        assert_eq!(kind, ConfigDirKind::Default);
        assert_eq!(resolved, home.join(".workestrate"));

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn legacy_xdg_registry_path_compat() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);

        let xdg = uniq_dir("rh-compat");
        std::fs::create_dir_all(&xdg)?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &xdg) };

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
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);

        let home = uniq_dir("a16-corrupt-home");
        std::fs::create_dir_all(home.join(".workestrate"))?;
        // Corrupt registry: invalid TOML.
        std::fs::write(
            home.join(".workestrate").join("config.toml"),
            "this is = not = valid toml [[[",
        )?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", &home) };

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
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);

        let home = uniq_dir("a16-valid-home");
        let custom_state = uniq_dir("a16-custom-state");
        std::fs::create_dir_all(home.join(".workestrate"))?;
        std::fs::write(
            home.join(".workestrate").join("config.toml"),
            format!("[settings]\nstate_dir = \"{}\"\n", custom_state.display()),
        )?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", &home) };

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
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);

        let home = uniq_dir("tilde-home");
        std::fs::create_dir_all(&home)?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", &home) };

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
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);

        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", "/definitely/not/used") };
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
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);

        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("HOME") };
        let expanded = expand_tilde("~/some/state");
        assert_eq!(
            expanded,
            PathBuf::from("~/some/state"),
            "HOME-unset must return the path unexpanded, never '.'"
        );
        Ok(())
    }

    // ---- Invocation cwd capture (wrong-CWD `${CWD}` mount fix) ----

    /// The captured env value WINS over the live process cwd — the
    /// re-exec'd / detached child case that motivated the fix.
    #[test]
    fn invoke_cwd_prefers_env_over_process_cwd() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(&[INVOKE_CWD_ENV]);

        let invoke = uniq_dir("invoke-cwd-env");
        let elsewhere = uniq_dir("invoke-cwd-elsewhere");
        std::fs::create_dir_all(&invoke)?;
        std::fs::create_dir_all(&elsewhere)?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var(INVOKE_CWD_ENV, &invoke) };
        std::env::set_current_dir(&elsewhere)?;

        assert_eq!(invoke_cwd(), Some(invoke.clone()));
        assert_eq!(invoke_cwd_or_err()?, invoke);

        let _ = std::fs::remove_dir_all(&invoke);
        let _ = std::fs::remove_dir_all(&elsewhere);
        Ok(())
    }

    /// Fallback preserved: var unset → the process cwd wins (pre-capture
    /// behavior for library/test callers that never ran `main()`).
    #[test]
    fn invoke_cwd_falls_back_to_process_cwd_when_unset() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(&[INVOKE_CWD_ENV]);

        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var(INVOKE_CWD_ENV) };
        assert_eq!(invoke_cwd(), Some(std::env::current_dir()?));
        Ok(())
    }

    /// A non-absolute (or empty) captured value is ignored — it can never
    /// pin a mount host to a relative path.
    #[test]
    fn invoke_cwd_ignores_non_absolute_env_value() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(&[INVOKE_CWD_ENV]);

        for bad in ["", "relative/dir", "./dot"] {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            unsafe { std::env::set_var(INVOKE_CWD_ENV, bad) };
            assert_eq!(
                invoke_cwd(),
                Some(std::env::current_dir()?),
                "non-absolute value '{bad}' must fall back to the process cwd"
            );
        }
        Ok(())
    }

    #[test]
    fn ensure_invoke_cwd_env_captures_cwd_when_unset() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(&[INVOKE_CWD_ENV]);

        let dir = uniq_dir("invoke-cwd-ensure");
        std::fs::create_dir_all(&dir)?;
        std::env::set_current_dir(&dir)?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var(INVOKE_CWD_ENV) };

        ensure_invoke_cwd_env();

        assert_eq!(
            std::env::var(INVOKE_CWD_ENV).unwrap(),
            std::env::current_dir()?.to_string_lossy(),
            "unset var must be captured from the current process cwd"
        );

        let _ = std::fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn ensure_invoke_cwd_env_keeps_inherited_value() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(&[INVOKE_CWD_ENV]);

        // Simulates a re-exec'd / detached child: the ORIGINAL operator cwd
        // inherited from the parent must NOT be overwritten by the child's
        // own cwd.
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var(INVOKE_CWD_ENV, "/inherited/operator-cwd") };
        ensure_invoke_cwd_env();
        assert_eq!(
            std::env::var(INVOKE_CWD_ENV).unwrap(),
            "/inherited/operator-cwd",
            "an inherited value wins over the child's own cwd"
        );
        Ok(())
    }

    // ---- WORKESTRATE_STATE_DIR override (C2; hermetic registry isolation) ----

    /// The env override is the HIGHEST-precedence step: it wins over a
    /// registry `settings.state_dir` AND over the config-derived default.
    #[test]
    fn state_dir_env_override_wins_over_registry_and_default() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        // WORKESTRATE_STATE_DIR is not in CONFIG_ENV_KEYS; guard it manually.
        let old_state = std::env::var("WORKESTRATE_STATE_DIR").ok();

        let home = uniq_dir("sd-override-home");
        let registry_state = uniq_dir("sd-override-registry");
        let env_state = uniq_dir("sd-override-env");
        std::fs::create_dir_all(home.join(".workestrate"))?;
        std::fs::write(
            home.join(".workestrate").join("config.toml"),
            format!("[settings]\nstate_dir = \"{}\"\n", registry_state.display()),
        )?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", &home) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_STATE_DIR", &env_state) };

        assert_eq!(
            resolve_state_dir(),
            env_state,
            "WORKESTRATE_STATE_DIR must win over registry settings.state_dir"
        );

        // Unset → the registry setting takes over again.
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("WORKESTRATE_STATE_DIR") };
        assert_eq!(resolve_state_dir(), registry_state);

        match old_state {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("WORKESTRATE_STATE_DIR", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("WORKESTRATE_STATE_DIR") },
        }
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn missing_registry_falls_back_silently() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);

        let home = uniq_dir("a16-missing-home");
        std::fs::create_dir_all(&home)?; // no .workestrate/config.toml at all
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", &home) };

        assert!(
            load_registry_for_dir_resolution().is_none(),
            "missing registry → None (normal bootstrap, silent)"
        );
        assert_eq!(resolve_store_dir(), home.join(".workestrate"));

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    // ---- Spec 05: cwd-derived reference config is opt-in gated ----
    // ---- Cleanup phase 2: the whole reference base layer is opt-in ----

    /// Env keys every spec-05/phase-2 test captures/clears.
    /// CARGO_MANIFEST_DIR is removed in tests 1–3 so tier 2 cannot win over
    /// the cwd tier; WORKESTRATE_FLEET_DIR is removed so load-layer bypass
    /// cannot mask the reference resolution; WORKESTRATE_REFERENCE_CONFIG is
    /// the cleanup-phase-2 opt-in for the reference base layer itself.
    const SPEC05_ENV_KEYS: &[&str] = &[
        "AGENTCTL_ROOT",
        "CARGO_MANIFEST_DIR",
        "WORKESTRATE_ALLOW_CWD_REFERENCE",
        "WORKESTRATE_REFERENCE_CONFIG",
        "WORKESTRATE_FLEET_DIR",
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
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            unsafe { std::env::remove_var(k) };
        }
        Ok(tmp)
    }

    #[test]
    fn reference_ignored_without_phase2_opt_in() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(SPEC05_ENV_KEYS);

        let tmp = spec05_cwd_fixture("phase2-no-opt-in")?;
        // Even a PINNED root (tier 1) must not yield the reference layer
        // without WORKESTRATE_REFERENCE_CONFIG=1 (cleanup phase 2).
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("AGENTCTL_ROOT", &tmp) };

        assert_eq!(
            reference_config_path(),
            None,
            "reference base layer must be ignored without WORKESTRATE_REFERENCE_CONFIG=1"
        );

        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn cwd_reference_ignored_without_opt_in() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(SPEC05_ENV_KEYS);

        let tmp = spec05_cwd_fixture("spec05-no-opt-in")?;
        // Phase-2 opt-in present, so only the spec-05 cwd gate is under test.
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_REFERENCE_CONFIG", "1") };

        // Root resolves from cwd (tier 3); without the cwd opt-in env the
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
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_REFERENCE_CONFIG", "1") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_ALLOW_CWD_REFERENCE", "1") };

        assert_eq!(
            reference_config_path(),
            Some(tmp.join("config.reference").join("workestrate.toml")),
            "cwd-derived reference must load with both opt-ins set"
        );

        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn agentctl_root_tier_needs_only_phase2_opt_in() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(SPEC05_ENV_KEYS);

        let tmp = spec05_cwd_fixture("spec05-pinned-root")?;
        // A pinned root never requires the cwd opt-in — only the phase-2
        // WORKESTRATE_REFERENCE_CONFIG=1 flag.
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("AGENTCTL_ROOT", &tmp) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_REFERENCE_CONFIG", "1") };

        assert_eq!(
            reference_config_path(),
            Some(tmp.join("config.reference").join("workestrate.toml")),
            "AGENTCTL_ROOT-pinned reference must load with only WORKESTRATE_REFERENCE_CONFIG=1"
        );

        let _ = std::fs::remove_dir_all(&tmp);
        Ok(())
    }

    #[test]
    fn manifest_dir_tier_needs_only_phase2_opt_in() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(SPEC05_ENV_KEYS);

        // Empty tempdir as cwd (NO flake.nix, NO config.reference) so tier 3
        // cannot resolve; CARGO_MANIFEST_DIR (tier 2 / manifest fallback)
        // resolves the real repo fixture with only the phase-2 opt-in.
        let tmp = uniq_dir("spec05-manifest-tier");
        std::fs::create_dir_all(&tmp)?;
        std::env::set_current_dir(&tmp)?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("AGENTCTL_ROOT") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("WORKESTRATE_ALLOW_CWD_REFERENCE") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("WORKESTRATE_FLEET_DIR") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("CARGO_MANIFEST_DIR", env!("CARGO_MANIFEST_DIR")) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_REFERENCE_CONFIG", "1") };

        let path = reference_config_path()
            .expect("manifest-tier reference must resolve with only the phase-2 opt-in");
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
