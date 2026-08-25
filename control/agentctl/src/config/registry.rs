//! Registry persistence and active-context resolution.

use anyhow::Result;

use crate::config::paths::registry_path;
use crate::config::{ActiveContext, ConfigRepoEntry, Registry};

/// Load the tool-home registry (`registry.toml` at [`registry_path`]).
/// Returns `Ok(None)` when the file does not exist (normal first-run state);
/// read/parse failures are hard errors.
///
/// G4 (fail-closed): every `[contexts.<name>]` key is validated with
/// [`validate_context_name`] after deserialize — a registry carrying an
/// invalid context name fails to load rather than silently admitting a name
/// that would produce malformed sandbox instance prefixes. (The lenient
/// `load_registry_for_dir_resolution` in config/paths.rs deliberately
/// swallows this error for early path resolution.)
pub fn load_registry() -> Result<Option<Registry>> {
    let path = registry_path();
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| anyhow::anyhow!("failed to read registry {}: {}", path.display(), e))?;
    let registry: Registry = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("failed to parse registry {}: {}", path.display(), e))?;
    for name in registry.contexts.keys() {
        validate_context_name(name)?;
    }
    Ok(Some(registry))
}

/// Validate a context name (G4). Context names become sandbox instance name
/// prefixes (`<context>-<workload>`), so they must be safe instance-name
/// components: `^[a-z0-9][a-z0-9-]*$` — lowercase alphanumerics and hyphens,
/// starting alphanumeric — plus no trailing hyphen (a trailing `-` would
/// double the `<context>-<workload>` separator). Unlike
/// [`crate::microsandbox::slots::validate_instance_id`] there is NO length
/// cap and NO reserved-word/numeric rule.
///
/// Contexts are created by hand-editing the registry TOML (there is no
/// `context new` command), so the load-time gate in [`load_registry`] is the
/// single enforcement point; this function is its pure core.
pub fn validate_context_name(name: &str) -> Result<()> {
    let valid = !name.is_empty()
        && !name.ends_with('-')
        && name.chars().enumerate().all(|(i, c)| {
            if i == 0 {
                c.is_ascii_lowercase() || c.is_ascii_digit()
            } else {
                c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'
            }
        });
    if !valid {
        anyhow::bail!(
            "invalid context name '{}': must match ^[a-z0-9][a-z0-9-]*$ (lowercase alphanumerics \
             and hyphens, starting alphanumeric) — context names become sandbox instance name \
             prefixes",
            name
        );
    }
    Ok(())
}

/// Persist the registry to `registry.toml`, creating the parent directory as
/// needed. The write is atomic (serialize to a sibling `.tmp` file, then
/// rename) so readers never observe a truncated registry (FN-5).
pub fn save_registry(registry: &Registry) -> Result<()> {
    let path = registry_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(registry)
        .map_err(|e| anyhow::anyhow!("failed to serialize registry: {}", e))?;
    // Atomic write: serialize to `<path>.tmp` on the SAME filesystem, then
    // rename(2) over the target. A crash mid-write can only corrupt the tmp
    // file — the last good registry stays intact under the original name;
    // readers never observe a truncated file (FN-5).
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, content)?;
    std::fs::rename(&tmp, &path)?;
    Ok(())
}

/// Name of the advisory lock file placed next to `config.toml`.
const REGISTRY_LOCK_NAME: &str = "config.toml.lock";

/// RAII advisory lock for registry mutations (FN-5).
///
/// Acquires by O_EXCL-creating `config.toml.lock` next to the registry file
/// (the create fails while another holder's file exists); [`Drop`] removes
/// it. Holders serialize the full load → mutate → save critical section in
/// [`register_config`], [`crate::config::trust_project`], and
/// [`crate::config::untrust_project`].
///
/// This is a cooperative lock between workestrate processes (mirroring the
/// port-registry lock in `microsandbox::port_registry`), not a mandatory
/// `flock`: the registry is only ever mutated by workestrate itself. To keep
/// concurrent trust/register commands from failing spuriously, acquisition
/// retries briefly (~2s, 25ms backoff) before giving up.
struct RegistryLock {
    path: std::path::PathBuf,
}

impl RegistryLock {
    fn acquire() -> Result<Self> {
        let registry = registry_path();
        if let Some(parent) = registry.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let path = registry.with_file_name(REGISTRY_LOCK_NAME);
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(2000);
        let backoff = std::time::Duration::from_millis(25);
        loop {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(_f) => return Ok(Self { path }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if std::time::Instant::now() >= deadline {
                        anyhow::bail!(
                            "timed out acquiring registry lock {}; another workestrate                              process is mutating the registry. Retry, or remove the file                              if no workestrate process is running.",
                            path.display()
                        );
                    }
                    std::thread::sleep(backoff);
                }
                Err(e) => {
                    return Err(e).map_err(|e| {
                        anyhow::anyhow!("failed to acquire registry lock {}: {}", path.display(), e)
                    });
                }
            }
        }
    }
}

impl Drop for RegistryLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Run `f` with the advisory registry lock held across the whole
/// load → mutate → save critical section (FN-5). `f` receives the freshly
/// loaded registry (or the default when none exists yet).
pub(crate) fn with_registry_lock<R>(f: impl FnOnce(&mut Registry) -> Result<R>) -> Result<R> {
    let _lock = RegistryLock::acquire()?;
    let mut registry = load_registry()?.unwrap_or_default();
    let out = f(&mut registry)?;
    save_registry(&registry)?;
    Ok(out)
}

/// Whether a registry entry is a LOCAL-PATH repo (registered via
/// `config new` — url is a filesystem path, ref=None, rev=None) as opposed
/// to a GIT-URL repo (registered via `config add <url>`).
///
/// FS-18: `cmd_config_update` previously classified `entry.rev.is_none()` as
/// local, which mis-skipped git-URL entries whose rev was simply unrecorded
/// (hand-edited registry, or a clone whose rev was never written back).
/// The explicit classifier: an entry is local-path ONLY when its url is not
/// a git remote AND no ref/rev was ever recorded. A git-URL entry with a
/// missing rev is NOT local — it is pulled (which re-records the rev).
pub fn entry_is_local_path(entry: &ConfigRepoEntry) -> bool {
    !looks_like_git_url(&entry.url) && entry.r#ref.is_none() && entry.rev.is_none()
}

/// Resolve the checkout directory of a LOCAL-PATH registry entry.
///
/// Returns `None` for git-URL entries (those resolve to the managed store
/// clone via [`crate::config::paths::config_repo_dir`]). For local-path
/// entries: tilde-expand the url, then — if the result is RELATIVE —
/// resolve it against the tool home (`resolve_home_with_kind().0`).
///
/// Rationale: a shared home may be mounted at different roots (container
/// `/home/node` vs host `/home/rybski`). An absolute url breaks on the
/// other side; a relative url resolves against each side's own mount, so
/// one registry entry works in both worlds permanently.
pub fn local_entry_checkout_dir(entry: &ConfigRepoEntry) -> Option<std::path::PathBuf> {
    if !entry_is_local_path(entry) {
        return None;
    }
    let path = crate::config::paths::expand_tilde(&entry.url);
    if path.is_absolute() {
        Some(path)
    } else {
        Some(crate::config::paths::resolve_home_with_kind().0.join(path))
    }
}

/// Classifier: whether `url` names a GIT remote (http(s)/ssh/git protocol or
/// a `.git`-suffixed path) as opposed to a plain local filesystem path.
/// Extracted from [`entry_is_local_path`] (ADR 0025): `home clone`
/// reuses it to classify the provisioning source (the `<src>` positional) and each
/// registry entry's reproducibility. NOTE: a local path ending in `.git` is
/// classified remote — git itself treats such paths as cloneable URLs, and
/// offline tests lean on exactly that behavior.
pub(crate) fn looks_like_git_url(url: &str) -> bool {
    url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("git@")
        || url.starts_with("ssh://")
        || url.starts_with("git://")
        || url.ends_with(".git")
}

/// Scheme-discriminated source-kind of a registry entry's `url`
/// (ADR 0032 addendum 2026-08-24 §Config source model). This is the A5
/// classification; it does NOT change [`looks_like_git_url`] /
/// [`entry_is_local_path`] semantics (their existing call sites are
/// unchanged).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSourceKind {
    /// `github:` / `https://` / `http://` / `ssh://` / `git@` / `git://` /
    /// `.git`-suffixed — cloned into the managed store; network fetch.
    /// (A strict superset of [`looks_like_git_url`]: that set PLUS
    /// `github:`.)
    Remote,
    /// `git+file://` — a local git repo with full ref support (branches,
    /// tags, shas resolve), NO network. Checked BEFORE the `.git`-suffix
    /// rule so `git+file:///path/repo.git` is GitFile, not Remote.
    GitFile,
    /// Everything else — a local working repo (git) or a plain dir. A git
    /// working repo resolves refs against its own object store; a plain dir
    /// is consumed content-as-is with branch `"local"`.
    PlainPath,
}

/// Classify `url` into its [`ConfigSourceKind`]. Pure; the ONLY ordering
/// subtlety is that `git+file://` is checked before the `.git`-suffix
/// remote rule (a `git+file://` url may itself end in `.git`).
pub fn source_kind(url: &str) -> ConfigSourceKind {
    if url.starts_with("git+file://") {
        return ConfigSourceKind::GitFile;
    }
    if url.starts_with("github:") || looks_like_git_url(url) {
        return ConfigSourceKind::Remote;
    }
    ConfigSourceKind::PlainPath
}

/// Pure decision core of [`resolve_default_ref`]: the ADR 0032 addendum
/// default-ref order — explicit `ref` > `origin/HEAD` (Remote/GitFile
/// checkouts) > checkout HEAD (PlainPath working git repos) > HARD ERROR
/// naming the repo.
fn pick_default_ref(
    name: &str,
    explicit: Option<&str>,
    kind: ConfigSourceKind,
    remote_head: Option<&str>,
    checkout_head: Option<&str>,
) -> Result<String> {
    if let Some(r) = explicit {
        return Ok(r.to_string());
    }
    let resolved = match kind {
        ConfigSourceKind::Remote | ConfigSourceKind::GitFile => remote_head,
        ConfigSourceKind::PlainPath => checkout_head,
    };
    match resolved {
        Some(r) => Ok(r.to_string()),
        None => {
            let tried = match kind {
                ConfigSourceKind::Remote | ConfigSourceKind::GitFile => {
                    "no origin/HEAD in the managed clone"
                }
                ConfigSourceKind::PlainPath => "the checkout is not a git working repo on a branch",
            };
            anyhow::bail!(
                "cannot resolve a default ref for config repo '{}': no explicit `ref` in the \
                 registry entry and {}; set `ref` in the registry entry",
                name,
                tried
            )
        }
    }
}

/// Resolve the default ref of one registry entry against its checkout
/// (ADR 0032 addendum §Config source model). Order: the entry's explicit
/// `ref` > `origin/HEAD` of `checkout` for Remote/GitFile entries >
/// `checkout`'s own HEAD branch when it is a working git repo (PlainPath
/// with `.git`) > HARD ERROR naming the repo.
///
/// Pure classification + decision; the only side effects are the two
/// read-only git probes. Session 1 ships the helper unwired — consumption
/// lands with the A5 session that resolves refs through the archive store.
pub fn resolve_default_ref(
    name: &str,
    entry: &ConfigRepoEntry,
    checkout: &std::path::Path,
) -> Result<String> {
    let kind = source_kind(&entry.url);
    let (remote_head, checkout_head) = match kind {
        ConfigSourceKind::Remote | ConfigSourceKind::GitFile => {
            (crate::git::git_remote_default_branch(checkout)?, None)
        }
        ConfigSourceKind::PlainPath => {
            if checkout.join(".git").exists() {
                (None, crate::git::git_checkout_branch(checkout)?)
            } else {
                (None, None)
            }
        }
    };
    pick_default_ref(
        name,
        entry.r#ref.as_deref(),
        kind,
        remote_head.as_deref(),
        checkout_head.as_deref(),
    )
}

/// The EFFECTIVE ref of one registry entry (A5 Session 2): the entry's
/// explicit `ref` when set, else [`resolve_default_ref`] against `checkout`.
/// This is the ref the lock pins under `(name, effective ref)` and the ref
/// `config update` rev-parses. Unlike [`resolve_default_ref`] it does NOT
/// probe git when an explicit ref is set (the probe could not change the
/// answer — explicit wins — so skipping it is pure savings).
pub fn effective_ref(
    name: &str,
    entry: &ConfigRepoEntry,
    checkout: &std::path::Path,
) -> Result<String> {
    match entry.r#ref.as_deref() {
        Some(r) => Ok(r.to_string()),
        None => resolve_default_ref(name, entry, checkout),
    }
}

/// Insert/replace a config repo entry in the registry. If `layers` is empty,
/// push `name` as the default layer (mirrors cmd_config_add's behavior).
/// Shared by `cmd_config_add` (clone + register) and `cmd_config_new`
/// (local path + register). For local-path scaffolds, pass `git_ref = None`
/// and `rev = None` — `cmd_config_update` recognizes this as a local-path
/// repo and skips the pull step.
pub fn register_config(
    name: &str,
    url: &str,
    git_ref: Option<&str>,
    rev: Option<&str>,
) -> Result<()> {
    with_registry_lock(|registry| {
        registry.configs.insert(
            name.to_string(),
            ConfigRepoEntry {
                url: url.to_string(),
                r#ref: git_ref.map(|s| s.to_string()),
                rev: rev.map(|s| s.to_string()),
                secrets: None,
                secrets_file: None,
                age_key_file: None,
                image_keep_last: None,
            },
        );
        if registry.layers.is_empty() {
            registry.layers.push(name.to_string());
        }
        Ok(())
    })
}

/// Persist `settings.default_context = name` in the registry. Errors if no
/// registry exists or if `name` is not a defined context (the message lists
/// the available contexts). The load → mutate → save critical section runs
/// under the advisory registry lock (FN-5), same as [`register_config`].
pub fn set_default_context(name: &str) -> Result<()> {
    let _lock = RegistryLock::acquire()?;
    let mut registry = load_registry()?
        .ok_or_else(|| anyhow::anyhow!("no registry found; run 'workestrate init' first"))?;
    if !registry.contexts.contains_key(name) {
        let mut available: Vec<String> = registry.contexts.keys().cloned().collect();
        available.sort();
        let available = if available.is_empty() {
            "(none defined)".to_string()
        } else {
            available.join(", ")
        };
        anyhow::bail!(
            "context '{}' is not defined; available contexts: {}",
            name,
            available
        );
    }
    registry.settings.default_context = Some(name.to_string());
    save_registry(&registry)?;
    Ok(())
}

/// Step (b) of the A5 derivation order (see [`resolve_active_context`]):
/// when `WORKESTRATE_CONFIG_REF` (`--config-ref`) names a BRANCH —
/// `refs/heads/<ref>` or `refs/remotes/origin/<ref>` present in ANY
/// Remote/GitFile entry's managed clone (first match wins) — the ref IS the
/// context-name candidate. A purely-sha ref yields None (shas are not
/// branches). Probe failures (missing clone, git error) read as
/// non-matches: consumption is the fail-closed layer
/// (`config::loading`'s pinned resolver errors name repo+ref).
fn config_ref_branch_candidate(registry: &Registry) -> Option<String> {
    let config_ref = std::env::var("WORKESTRATE_CONFIG_REF").ok()?;
    if config_ref.is_empty() {
        return None;
    }
    for (name, entry) in &registry.configs {
        if source_kind(&entry.url) == ConfigSourceKind::PlainPath {
            continue;
        }
        let clone = crate::config::paths::resolve_store_dir()
            .join("config-repos")
            .join(name);
        if !clone.join(".git").exists() {
            continue;
        }
        if crate::git::git_branch_ref_exists(&clone, &config_ref).unwrap_or(false) {
            return Some(config_ref);
        }
    }
    None
}

/// Step (c) of the A5 derivation order (see [`resolve_active_context`]):
/// the FIRST layer's checkout branch — [`local_entry_checkout_dir`] for
/// PlainPath entries, else the managed clone — when that dir is a git repo
/// on a branch ([`crate::git::git_checkout_branch`]; detached HEAD and
/// non-repos yield None).
fn checkout_branch_candidate(registry: &Registry) -> Option<String> {
    let first = registry.layers.first()?;
    let dir = registry
        .configs
        .get(first)
        .and_then(local_entry_checkout_dir)
        .unwrap_or_else(|| {
            crate::config::paths::resolve_store_dir()
                .join("config-repos")
                .join(first)
        });
    if !dir.join(".git").exists() {
        return None;
    }
    crate::git::git_checkout_branch(&dir).ok().flatten()
}

/// Resolve the active context.
///
/// A5 derivation order (ADR 0032 addendum 2026-08-24 §Selection ladder;
/// order PINNED 2026-08-24):
///
/// a. `WORKESTRATE_CONTEXT` env (set by `--context` or by hand) —
///    UNCHANGED strict semantics: when contexts are defined the name MUST
///    be one of them (hard error otherwise); a contexts-less home ignores
///    the env and keeps the bare-layers shape (`name = None`), as before.
/// b. `WORKESTRATE_CONFIG_REF` (`--config-ref`) when it names a BRANCH —
///    see [`config_ref_branch_candidate`]. A purely-sha ref yields NO
///    context here.
/// c. Checkout branch: the FIRST layer's checkout — see
///    [`checkout_branch_candidate`].
/// d. Otherwise today's behavior: `[settings] default_context` / bare
///    `layers` when no contexts are defined / the existing hard error. The
///    ADR's "> main" final step IS this default resolution (main = the
///    stable line) — there is NO literal "main" context name.
///
/// Candidate-name resolution for steps b/c (the name did NOT come from an
/// explicit `--context`): when `registry.contexts` CONTAINS the candidate,
/// its layers are used; ELSE the candidate rides LENIENTLY as the context
/// NAME (slot prefixing, instance identity) while the LAYER LIST falls
/// back to `default_context`'s layers, else the bare `layers`. When
/// contexts ARE defined AND the candidate is undefined AND no
/// `default_context` exists, the existing hard error stands. Bare-layers
/// homes (no `[contexts]`) keep `name = None` UNLESS a candidate arose
/// from step b/c — then `name = Some(candidate)` over the bare layers
/// (the dev-model namespacing).
pub fn resolve_active_context() -> Result<ActiveContext> {
    let registry = match load_registry()? {
        Some(r) => r,
        None => {
            return Ok(ActiveContext {
                name: None,
                layers: vec![],
            })
        }
    };
    let available = || {
        registry
            .contexts
            .keys()
            .cloned()
            .collect::<Vec<_>>()
            .join(", ")
    };

    // (a) WORKESTRATE_CONTEXT env (set by --context flag or by user) —
    //     unchanged strict semantics; a contexts-less home ignores it.
    if let Ok(name) = std::env::var("WORKESTRATE_CONTEXT") {
        if registry.contexts.is_empty() {
            return Ok(ActiveContext {
                name: None,
                layers: registry.layers,
            });
        }
        if let Some(ctx) = registry.contexts.get(&name) {
            return Ok(ActiveContext {
                name: Some(name),
                layers: ctx.layers.clone(),
            });
        }
        anyhow::bail!(
            "context '{}' not found in registry; available contexts: {}",
            name,
            available()
        );
    }

    // (b) / (c): a derived candidate name — the --config-ref branch, else
    //     the first layer's checkout branch.
    if let Some(candidate) =
        config_ref_branch_candidate(&registry).or_else(|| checkout_branch_candidate(&registry))
    {
        // A DEFINED context of that name: its layers.
        if let Some(ctx) = registry.contexts.get(&candidate) {
            return Ok(ActiveContext {
                name: Some(candidate),
                layers: ctx.layers.clone(),
            });
        }
        // Lenient identity-only mode: the candidate rides as the context
        // NAME over default_context's layers, else the bare layers.
        if let Some(ref default) = registry.settings.default_context {
            match registry.contexts.get(default) {
                Some(ctx) => {
                    return Ok(ActiveContext {
                        name: Some(candidate),
                        layers: ctx.layers.clone(),
                    });
                }
                None => {
                    anyhow::bail!(
                        "default_context '{}' not found in registry contexts; available: {}",
                        default,
                        available()
                    );
                }
            }
        }
        if registry.contexts.is_empty() {
            return Ok(ActiveContext {
                name: Some(candidate),
                layers: registry.layers,
            });
        }
        // Contexts defined, candidate undefined, no default: the existing
        // hard error stands.
        anyhow::bail!(
            "contexts are defined but no default_context is set; use --context <name> or set WORKESTRATE_CONTEXT env. Available contexts: {}",
            available()
        );
    }

    // (d) today's behavior, unchanged: bare layers when no contexts are
    //     defined (backward compat), else default_context, else the hard
    //     error.
    if registry.contexts.is_empty() {
        return Ok(ActiveContext {
            name: None,
            layers: registry.layers,
        });
    }
    if let Some(ref default) = registry.settings.default_context {
        if let Some(ctx) = registry.contexts.get(default) {
            return Ok(ActiveContext {
                name: Some(default.clone()),
                layers: ctx.layers.clone(),
            });
        }
        anyhow::bail!(
            "default_context '{}' not found in registry contexts; available: {}",
            default,
            available()
        );
    }

    // Contexts exist but no env/candidate/default → error
    anyhow::bail!(
        "contexts are defined but no default_context is set; use --context <name> or set WORKESTRATE_CONTEXT env. Available contexts: {}",
        available()
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

    #[test]
    fn resolve_active_context_no_registry_uses_empty_layers() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-no-reg-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp_home)?;
        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let ctx = resolve_active_context()?;

        // Restore
        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        assert_eq!(ctx.name, None);
        assert!(ctx.layers.is_empty());
        Ok(())
    }

    #[test]
    fn resolve_active_context_no_contexts_uses_bare_layers() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-bare-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        // Registry with bare layers, no contexts
        std::fs::write(
            config_dir.join("config.toml"),
            "layers = [\"work\", \"personal\"]\n\n[settings]\ndefault_context = \"personal\"\n",
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let ctx = resolve_active_context()?;

        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        // No contexts defined → bare layers, name is None (backward compat)
        assert_eq!(ctx.name, None);
        assert_eq!(ctx.layers, vec!["work", "personal"]);
        Ok(())
    }

    #[test]
    fn resolve_active_context_env_overrides_default() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-env-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        std::fs::write(
            config_dir.join("config.toml"),
            "[settings]\ndefault_context = \"personal\"\n\n[contexts.personal]\nlayers = [\"personal\"]\n\n[contexts.work]\nlayers = [\"team\", \"personal\"]\n",
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::set_var("WORKESTRATE_CONTEXT", "work");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let ctx = resolve_active_context()?;

        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        assert_eq!(ctx.name.as_deref(), Some("work"));
        assert_eq!(ctx.layers, vec!["team", "personal"]);
        Ok(())
    }

    #[test]
    fn resolve_active_context_default_when_no_env() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-default-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        std::fs::write(
            config_dir.join("config.toml"),
            "[settings]\ndefault_context = \"personal\"\n\n[contexts.personal]\nlayers = [\"personal\"]\n\n[contexts.work]\nlayers = [\"team\", \"personal\"]\n",
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let ctx = resolve_active_context()?;

        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        assert_eq!(ctx.name.as_deref(), Some("personal"));
        assert_eq!(ctx.layers, vec!["personal"]);
        Ok(())
    }

    #[test]
    fn resolve_active_context_unknown_env_errors() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-unknown-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        std::fs::write(
            config_dir.join("config.toml"),
            "[settings]\ndefault_context = \"personal\"\n\n[contexts.personal]\nlayers = [\"personal\"]\n",
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::set_var("WORKESTRATE_CONTEXT", "nonexistent");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let result = resolve_active_context();

        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not found"),
            "error should mention 'not found': {err}"
        );
        Ok(())
    }

    #[test]
    fn resolve_active_context_contexts_but_no_default_no_env_errors() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-ctx-nodflt-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let config_dir = tmp_home.join(".config").join("workestrate");
        std::fs::create_dir_all(&config_dir)?;
        // Contexts defined but no default_context and no env
        std::fs::write(
            config_dir.join("config.toml"),
            "[contexts.personal]\nlayers = [\"personal\"]\n\n[contexts.work]\nlayers = [\"team\"]\n",
        )?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        std::env::set_var("HOME", &tmp_home);
        std::env::set_var(
            "XDG_CONFIG_HOME",
            tmp_home.join(".config").to_string_lossy().as_ref(),
        );
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_DIR");

        let result = resolve_active_context();

        match old_home {
            Some(v) => std::env::set_var("HOME", v),
            None => std::env::remove_var("HOME"),
        }
        match old_xdg {
            Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        match old_config_dir {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("default_context") || err.contains("--context"),
            "error should mention default_context or --context: {err}"
        );
        Ok(())
    }
    // ---- FS-18: local-path vs git-URL registry entry classification ----

    fn entry(url: &str, git_ref: Option<&str>, rev: Option<&str>) -> ConfigRepoEntry {
        ConfigRepoEntry {
            url: url.to_string(),
            r#ref: git_ref.map(|s| s.to_string()),
            rev: rev.map(|s| s.to_string()),
            secrets: None,
            secrets_file: None,
            age_key_file: None,
            image_keep_last: None,
        }
    }

    /// The FS-18 regression: a GIT-URL entry with rev=None is NOT a local
    /// path — `cmd_config_update` must not skip it (it gets pulled, which
    /// re-records the rev).
    #[test]
    fn git_url_entry_with_missing_rev_is_not_local_path() {
        for e in [
            entry("https://example.invalid/repo.git", Some("main"), None),
            entry("https://example.invalid/repo.git", None, None),
            entry("git@example.invalid:org/repo.git", Some("main"), None),
            entry("ssh://git@example.invalid/org/repo", None, None),
            // Rev recorded but ref missing: still git (rev was once known).
            entry("https://example.invalid/repo.git", None, Some("abc123")),
        ] {
            assert!(
                !entry_is_local_path(&e),
                "git-URL entry must NOT be classified local: {:?}",
                e.url
            );
        }
    }

    /// Genuine local-path entries (what `config new` registers: filesystem
    /// path url, ref=None, rev=None) ARE classified local and skip the pull.
    #[test]
    fn local_path_entry_is_classified_local() {
        for e in [
            entry("/home/user/my-config", None, None),
            entry("relative/path", None, None),
            entry("~/my-config", None, None),
        ] {
            assert!(
                entry_is_local_path(&e),
                "local-path entry must be classified local: {:?}",
                e.url
            );
        }
        // A recorded ref or rev upgrades a path-like url to "tracked" — not
        // the `config new` shape, so not local.
        assert!(!entry_is_local_path(&entry(
            "/home/user/my-config",
            Some("main"),
            None
        )));
    }

    // ---- FN-5: atomic save + advisory lock ----

    /// Set WORKESTRATE_HOME to a fresh temp dir (HomeKind::Env → registry at
    /// `<tmp>/config.toml`) and return the dir. Caller must hold
    /// ENV_TEST_LOCK and an EnvGuard for HOME_ENV_KEYS.
    fn pin_home(label: &str) -> std::path::PathBuf {
        let home = uniq_dir(label);
        std::fs::create_dir_all(&home).expect("create pinned home");
        std::env::set_var("WORKESTRATE_HOME", &home);
        home
    }

    #[test]
    fn save_registry_writes_atomically_and_leaves_no_tmp() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("fn5-atomic");

        let mut registry = Registry::default();
        registry.configs.insert(
            "personal".to_string(),
            ConfigRepoEntry {
                url: "https://example.invalid/personal.git".to_string(),
                r#ref: Some("main".to_string()),
                rev: Some("abc123".to_string()),
                secrets: None,
                secrets_file: None,
                age_key_file: None,
                image_keep_last: None,
            },
        );
        save_registry(&registry)?;

        // Round-trip: output identical to the saved registry.
        let loaded = load_registry()?.expect("registry should exist after save");
        assert_eq!(loaded.configs.len(), 1);
        assert_eq!(
            loaded.configs["personal"].url,
            "https://example.invalid/personal.git"
        );
        assert_eq!(loaded.configs["personal"].rev.as_deref(), Some("abc123"));

        // No tmp file left behind next to config.toml.
        let tmp = registry_path().with_extension("toml.tmp");
        assert!(!tmp.exists(), "tmp file must not survive the rename");
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    // ---- local_entry_checkout_dir (shared-home duality) ----

    #[test]
    fn local_entry_checkout_dir_resolves_relative_against_home() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("local-entry-rel");
        let e = entry("config-repos/personal", None, None);
        assert_eq!(
            local_entry_checkout_dir(&e),
            Some(home.join("config-repos/personal"))
        );
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn local_entry_checkout_dir_passes_absolute_and_tilde_through() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        // Absolute entries are returned unchanged.
        let abs = entry("/home/user/my-config", None, None);
        assert_eq!(
            local_entry_checkout_dir(&abs),
            Some(std::path::PathBuf::from("/home/user/my-config"))
        );
        // Tilde entries are expanded (against HOME) and unchanged beyond that.
        let home_dir = std::env::var("HOME").expect("HOME set in test env");
        let tilde = entry("~/my-config", None, None);
        assert_eq!(
            local_entry_checkout_dir(&tilde),
            Some(std::path::PathBuf::from(home_dir).join("my-config"))
        );
    }

    #[test]
    fn local_entry_checkout_dir_returns_none_for_git_urls() {
        let e = entry("https://example.invalid/repo.git", Some("main"), None);
        assert_eq!(local_entry_checkout_dir(&e), None);
    }

    #[test]
    fn interrupted_write_leaves_original_registry_intact() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("fn5-interrupt");

        // Commit one good registry first.
        let mut good = Registry::default();
        good.layers.push("good".to_string());
        save_registry(&good)?;
        let before = std::fs::read_to_string(registry_path())?;

        // Simulate a crashed concurrent writer: a stale tmp file must not
        // clobber the committed registry — the next save replaces it
        // atomically, and readers in between keep seeing the original.
        let tmp = registry_path().with_extension("toml.tmp");
        std::fs::write(&tmp, "garbage-partial-write")?;
        assert_eq!(
            std::fs::read_to_string(registry_path())?,
            before,
            "original registry must survive a stale tmp file"
        );

        let mut next = Registry::default();
        next.layers.push("next".to_string());
        save_registry(&next)?;
        let loaded = load_registry()?.expect("registry parses after save");
        assert_eq!(loaded.layers, vec!["next".to_string()]);
        assert!(!tmp.exists(), "save must consume (rename) the tmp file");
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn register_config_removes_lock_file_and_persists_entry() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("fn5-lock");

        register_config(
            "personal",
            "https://example.invalid/personal.git",
            Some("main"),
            Some("abc123"),
        )?;

        // Lock file is removed after the critical section.
        let lock_path = registry_path().with_file_name(REGISTRY_LOCK_NAME);
        assert!(
            !lock_path.exists(),
            "registry lock must be released after register_config"
        );

        // The entry persisted (load → mutate → save ran under the lock).
        let loaded = load_registry()?.expect("registry should exist");
        assert!(loaded.configs.contains_key("personal"));
        assert_eq!(loaded.layers, vec!["personal".to_string()]);
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn trust_round_trip_releases_lock() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("fn5-trust");

        let project = uniq_dir("fn5-trust-proj");
        std::fs::create_dir_all(&project)?;
        crate::config::trust_project(&project)?;
        assert!(crate::config::is_trusted_project(&project));
        crate::config::untrust_project(&project)?;
        assert!(!crate::config::is_trusted_project(&project));

        let lock_path = registry_path().with_file_name(REGISTRY_LOCK_NAME);
        assert!(
            !lock_path.exists(),
            "registry lock must be released after trust/untrust"
        );
        let _ = std::fs::remove_dir_all(&home);
        let _ = std::fs::remove_dir_all(&project);
        Ok(())
    }

    // ---- W6a: set_default_context (`workestrate context use`) ----

    /// Seed a registry with `personal` + `work` contexts (default:
    /// `personal`) in the pinned home and return the home dir. Caller must
    /// hold ENV_TEST_LOCK + an EnvGuard for HOME_ENV_KEYS.
    fn seed_two_context_home(label: &str) -> std::path::PathBuf {
        let home = pin_home(label);
        let mut registry = Registry::default();
        registry.settings.default_context = Some("personal".to_string());
        registry.contexts.insert(
            "personal".to_string(),
            crate::config::Context {
                layers: vec!["personal".to_string()],
            },
        );
        registry.contexts.insert(
            "work".to_string(),
            crate::config::Context {
                layers: vec!["team".to_string(), "personal".to_string()],
            },
        );
        save_registry(&registry).expect("seed registry");
        home
    }

    #[test]
    fn set_default_context_persists() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = seed_two_context_home("w6a-use-persist");

        set_default_context("work")?;

        let registry = load_registry()?.expect("registry must exist");
        assert_eq!(
            registry.settings.default_context.as_deref(),
            Some("work"),
            "settings.default_context must persist as \"work\""
        );
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    #[test]
    fn set_default_context_then_resolves_as_active_default() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = seed_two_context_home("w6a-use-resolve");
        // No env override may shadow the persisted default.
        let old_ctx = std::env::var("WORKESTRATE_CONTEXT").ok();
        std::env::remove_var("WORKESTRATE_CONTEXT");

        set_default_context("work")?;
        let active = resolve_active_context()?;

        match old_ctx {
            Some(v) => std::env::set_var("WORKESTRATE_CONTEXT", v),
            None => std::env::remove_var("WORKESTRATE_CONTEXT"),
        }
        let _ = std::fs::remove_dir_all(&home);

        assert_eq!(active.name.as_deref(), Some("work"));
        assert_eq!(active.layers, vec!["team", "personal"]);
        Ok(())
    }

    #[test]
    fn set_default_context_unknown_name_errors_and_lists_available() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = seed_two_context_home("w6a-use-unknown");

        let result = set_default_context("nonexistent");

        let _ = std::fs::remove_dir_all(&home);
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("not defined"),
            "error must state the context is not defined: {err}"
        );
        assert!(
            err.contains("personal") && err.contains("work"),
            "error must list the available contexts: {err}"
        );
        Ok(())
    }

    #[test]
    fn set_default_context_without_registry_errors() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("w6a-use-noreg");

        let result = set_default_context("work");

        let _ = std::fs::remove_dir_all(&home);
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("no registry found"),
            "error must state no registry exists: {err}"
        );
        Ok(())
    }

    // ---- G4: context-name validation (fail-closed at registry load) ----

    #[test]
    fn validate_context_name_accepts_valid_names() {
        for ok in ["personal", "work-2", "a", "0", "a-b-c", "team2"] {
            validate_context_name(ok)
                .unwrap_or_else(|e| panic!("legitimate context '{ok}' rejected: {e}"));
        }
    }

    #[test]
    fn validate_context_name_rejects_invalid_names_and_names_them() {
        for bad in ["has space", "has@at", "Upper", "-leading", "trailing-", ""] {
            let err = validate_context_name(bad).unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains(&format!("'{bad}'")),
                "error must name the offending context '{bad}': {msg}"
            );
            assert!(
                msg.contains("^[a-z0-9][a-z0-9-]*$"),
                "error must state the pattern: {msg}"
            );
        }
    }

    /// A registry TOML carrying a bad `[contexts.<name>]` key must FAIL
    /// `load_registry` (fail-closed; G4), naming the offending context.
    #[test]
    fn load_registry_rejects_invalid_context_key() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = pin_home("g4-bad-context");
        std::fs::write(
            registry_path(),
            "[settings]\ndefault_context = \"personal\"\n\n[contexts.personal]\nlayers = [\"personal\"]\n\n[contexts.\"has space\"]\nlayers = [\"team\"]\n",
        )?;

        let err = load_registry().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("has space"),
            "error must name the offending context key: {msg}"
        );
        assert!(
            msg.contains("invalid context name"),
            "error must be the G4 validation error: {msg}"
        );
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// A registry whose contexts are all valid loads unchanged (the G4 gate
    /// admits the existing well-formed registries).
    #[test]
    fn load_registry_accepts_valid_context_keys() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(HOME_ENV_KEYS);
        let home = seed_two_context_home("g4-ok-context");

        let registry = load_registry()?.expect("valid registry must load");
        assert!(registry.contexts.contains_key("personal"));
        assert!(registry.contexts.contains_key("work"));
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    // ---- A5: scheme-discriminated source-kind classification ----

    #[test]
    fn source_kind_classifies_each_scheme() {
        for remote in [
            "github:org/repo",
            "https://example.invalid/repo.git",
            "http://example.invalid/repo",
            "ssh://git@example.invalid/org/repo",
            "git@example.invalid:org/repo.git",
            "git://example.invalid/repo",
            // The `.git`-suffix rule (superset of looks_like_git_url).
            "/local/path/repo.git",
            "relative/repo.git",
        ] {
            assert_eq!(
                source_kind(remote),
                ConfigSourceKind::Remote,
                "{remote} must classify Remote"
            );
        }
        assert_eq!(
            source_kind("git+file:///home/user/repo"),
            ConfigSourceKind::GitFile
        );
        // `git+file://` is checked BEFORE the `.git`-suffix rule.
        assert_eq!(
            source_kind("git+file:///home/user/repo.git"),
            ConfigSourceKind::GitFile,
            "a git+file url ending in .git is GitFile, not Remote"
        );
        for plain in [
            "/home/user/my-config",
            "relative/path",
            "~/my-config",
            ".",
            "..",
            "",
        ] {
            assert_eq!(
                source_kind(plain),
                ConfigSourceKind::PlainPath,
                "{plain:?} must classify PlainPath"
            );
        }
    }

    // ---- A5: default-ref resolution (pure core) ----

    #[test]
    fn pick_default_ref_prefers_explicit_then_kind_source_then_errors() {
        // Explicit ref wins for every kind.
        for kind in [
            ConfigSourceKind::Remote,
            ConfigSourceKind::GitFile,
            ConfigSourceKind::PlainPath,
        ] {
            assert_eq!(
                pick_default_ref(
                    "r",
                    Some("pinned"),
                    kind,
                    Some("origin-main"),
                    Some("co-main")
                )
                .unwrap(),
                "pinned"
            );
        }
        // Remote/GitFile fall back to origin/HEAD.
        for kind in [ConfigSourceKind::Remote, ConfigSourceKind::GitFile] {
            assert_eq!(
                pick_default_ref("r", None, kind, Some("main"), None).unwrap(),
                "main"
            );
            let err = pick_default_ref("myrepo", None, kind, None, None).unwrap_err();
            assert!(
                err.to_string().contains("myrepo"),
                "hard error must name the repo: {err}"
            );
        }
        // PlainPath falls back to the checkout branch.
        assert_eq!(
            pick_default_ref("r", None, ConfigSourceKind::PlainPath, None, Some("feat-x")).unwrap(),
            "feat-x"
        );
        let err = pick_default_ref("localrepo", None, ConfigSourceKind::PlainPath, None, None)
            .unwrap_err();
        assert!(
            err.to_string().contains("localrepo"),
            "hard error must name the repo: {err}"
        );
    }

    // ---- A5: default-ref resolution (git-backed, real temp repos) ----

    /// Init a git repo in `dir` with one committed file (repo-scoped
    /// identity, mirrors the git.rs test helper).
    fn init_repo(dir: &std::path::Path) {
        let run = |args: &[&str]| {
            let status = std::process::Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(args)
                .status()
                .expect("git must be runnable");
            assert!(status.success(), "git {:?} failed", args);
        };
        std::fs::create_dir_all(dir).expect("create repo dir");
        run(&["init", "--quiet"]);
        run(&["config", "user.email", "a5@test.invalid"]);
        run(&["config", "user.name", "a5-test"]);
        std::fs::write(dir.join("f.txt"), "x").expect("write file");
        run(&["add", "f.txt"]);
        run(&["commit", "--quiet", "-m", "init"]);
    }

    #[test]
    fn resolve_default_ref_remote_uses_origin_head_of_clone() {
        // Holds ENV_TEST_LOCK: the clone probes HOME for git config, and
        // parallel env-mutating tests can point HOME at a removed dir.
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let src = uniq_dir("a5-defref-src");
        init_repo(&src);
        let clone = uniq_dir("a5-defref-clone");
        let out = std::process::Command::new("git")
            .args(["clone", "--quiet"])
            .arg(&src)
            .arg(&clone)
            .output()
            .expect("git must be runnable");
        assert!(
            out.status.success(),
            "git clone failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let want = crate::git::git_checkout_branch(&src)
            .unwrap()
            .expect("src on a branch");

        let e = entry("https://example.invalid/repo.git", None, None);
        assert_eq!(
            resolve_default_ref("repo", &e, &clone).unwrap(),
            want,
            "a Remote entry with no explicit ref resolves to the clone's origin/HEAD"
        );
        // Explicit ref wins even when origin/HEAD exists.
        let e = entry("https://example.invalid/repo.git", Some("pinned"), None);
        assert_eq!(resolve_default_ref("repo", &e, &clone).unwrap(), "pinned");

        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&clone);
    }

    #[test]
    fn resolve_default_ref_plain_path_uses_checkout_branch_or_errors() {
        let repo = uniq_dir("a5-defref-workrepo");
        init_repo(&repo);
        let want = crate::git::git_checkout_branch(&repo)
            .unwrap()
            .expect("repo on a branch");

        let e = entry(repo.to_str().unwrap(), None, None);
        assert_eq!(
            resolve_default_ref("work", &e, &repo).unwrap(),
            want,
            "a PlainPath entry on a git working repo resolves to its checkout branch"
        );

        // A plain dir (no .git) is a hard error naming the repo.
        let plain = uniq_dir("a5-defref-plain");
        std::fs::create_dir_all(&plain).unwrap();
        let e = entry(plain.to_str().unwrap(), None, None);
        let err = resolve_default_ref("myplain", &e, &plain).unwrap_err();
        assert!(
            err.to_string().contains("myplain"),
            "hard error must name the repo: {err}"
        );

        let _ = std::fs::remove_dir_all(&repo);
        let _ = std::fs::remove_dir_all(&plain);
    }

    // ---- A5 Session 2: effective_ref ----

    #[test]
    fn effective_ref_prefers_explicit_and_delegates_otherwise() {
        // Explicit ref: returned verbatim, NO git probe (a nonexistent
        // checkout would fail the probe — passing it proves the short-circuit).
        let missing = std::path::Path::new("/nonexistent/a5-effective-ref-checkout");
        let e = entry("https://example.invalid/repo.git", Some("pinned"), None);
        assert_eq!(effective_ref("r", &e, missing).unwrap(), "pinned");

        // No explicit ref: delegates to resolve_default_ref (a Remote entry
        // against a checkout with no origin/HEAD is its hard error).
        let repo = uniq_dir("a5-effref-repo");
        init_repo(&repo);
        let e = entry("https://example.invalid/repo.git", None, None);
        let err = effective_ref("effrepo", &e, &repo).unwrap_err();
        assert!(
            err.to_string().contains("effrepo"),
            "the delegated hard error must name the repo: {err}"
        );
        let _ = std::fs::remove_dir_all(&repo);
    }

    // ---- A5 Session 3a: the context derivation order (ADR 0032 addendum) ----
    //
    // Fixture conventions: pinned WORKESTRATE_HOME (HomeKind::Env: registry
    // at <home>/config.toml, managed clones at <home>/config-repos/<name>),
    // REAL temp git repos (the git.rs precedent: git on the pinned PATH),
    // ENV_TEST_LOCK + EnvGuard over the discovery vars PLUS the two ladder
    // env vars (WORKESTRATE_CONTEXT is removed by default and set per test;
    // WORKESTRATE_CONFIG_REF likewise).

    const A5_DERIVE_ENV_KEYS: &[&str] = &[
        "WORKESTRATE_HOME",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_STATE_HOME",
        "WORKESTRATE_CONFIG_DIR",
        "WORKESTRATE_NO_PROJECT_CONFIG",
        "WORKESTRATE_INVOKE_CWD",
        "HOME",
        "WORKESTRATE_CONTEXT",
        "WORKESTRATE_CONFIG_REF",
    ];

    /// Pin the home and neutralize the ladder env vars. Caller holds
    /// ENV_TEST_LOCK; the EnvGuard is captured by the caller BEFORE this.
    fn a5_derive_home(label: &str) -> std::path::PathBuf {
        let home = pin_home(label);
        std::env::remove_var("WORKESTRATE_CONTEXT");
        std::env::remove_var("WORKESTRATE_CONFIG_REF");
        home
    }

    fn a5_derive_git(dir: &std::path::Path, args: &[&str]) {
        let status = std::process::Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .status()
            .expect("git must be runnable");
        assert!(
            status.success(),
            "git {:?} failed in {}",
            args,
            dir.display()
        );
    }

    /// Init a git working repo at `dir` on branch `branch` with one commit;
    /// returns the HEAD sha.
    fn a5_derive_repo(dir: &std::path::Path, branch: &str) -> String {
        std::fs::create_dir_all(dir).expect("create repo dir");
        a5_derive_git(dir, &["init", "--quiet", "-b", branch]);
        a5_derive_git(dir, &["config", "user.email", "a5-derive@test.invalid"]);
        a5_derive_git(dir, &["config", "user.name", "a5-derive-test"]);
        std::fs::write(dir.join("f.txt"), "x").expect("write file");
        a5_derive_git(dir, &["add", "f.txt"]);
        a5_derive_git(dir, &["commit", "--quiet", "-m", "init"]);
        crate::git::git_rev_parse(dir).expect("HEAD sha")
    }

    /// Register a Remote entry `name` with a managed clone
    /// (`<home>/config-repos/<name>`) on branch `checkout_branch`, in a
    /// bare-layers home (`layers = [<name>]`, no [contexts]).
    fn a5_derive_remote_bare_home(
        label: &str,
        name: &str,
        checkout_branch: &str,
    ) -> std::path::PathBuf {
        let home = a5_derive_home(label);
        let clone = home.join("config-repos").join(name);
        a5_derive_repo(&clone, checkout_branch);
        std::fs::write(
            home.join("config.toml"),
            format!(
                "layers = [\"{name}\"]\n\n[configs.{name}]\nurl = \"https://example.invalid/{name}.git\"\nref = \"main\"\n"
            ),
        )
        .expect("write registry");
        home
    }

    /// Step b: a branch-shaped --config-ref namespaces a bare-layers home —
    /// name = Some(<branch>) over the bare layer list (the dev-model
    /// namespacing).
    #[test]
    fn config_ref_branch_namespaces_a_bare_layers_home() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let home = a5_derive_remote_bare_home("a5-derive-cfgref", "team", "main");
        let clone = home.join("config-repos").join("team");
        // The branch exists in the clone but is NOT checked out (isolating
        // step b from step c, which would report the checkout branch).
        a5_derive_git(&clone, &["branch", "feat-x"]);
        std::env::set_var("WORKESTRATE_CONFIG_REF", "feat-x");

        let ctx = resolve_active_context()?;

        assert_eq!(ctx.name.as_deref(), Some("feat-x"));
        assert_eq!(ctx.layers, vec!["team".to_string()]);
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Step b with a purely-sha --config-ref yields NO context (shas are
    /// not branches); with no checkout-branch candidate either (detached
    /// HEAD), the bare-layers home keeps name = None.
    #[test]
    fn sha_config_ref_yields_no_context_name() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let home = a5_derive_remote_bare_home("a5-derive-sha", "team", "main");
        let clone = home.join("config-repos").join("team");
        let sha = crate::git::git_rev_parse(&clone)?;
        // Detach HEAD so step c cannot name a context either — isolating
        // the sha-ref behavior at step b.
        crate::git::git_checkout_rev(&clone, &sha)?;
        std::env::set_var("WORKESTRATE_CONFIG_REF", &sha);

        let ctx = resolve_active_context()?;

        assert_eq!(
            ctx.name, None,
            "a purely-sha --config-ref must not set a context name"
        );
        assert_eq!(ctx.layers, vec!["team".to_string()]);
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Step c: with no --config-ref, the FIRST layer's checkout branch is
    /// the candidate — a PlainPath working repo (local_entry_checkout_dir)
    /// and a managed clone alike.
    #[test]
    fn checkout_branch_namespaces_a_bare_layers_home() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);

        // (i) PlainPath first layer on branch wip.
        let home = a5_derive_home("a5-derive-plain");
        let work = home.join("my-work-config");
        a5_derive_repo(&work, "wip");
        std::fs::write(
            home.join("config.toml"),
            format!(
                "layers = [\"local\"]\n\n[configs.local]\nurl = \"{}\"\n",
                work.display()
            ),
        )?;
        let ctx = resolve_active_context()?;
        assert_eq!(ctx.name.as_deref(), Some("wip"));
        assert_eq!(ctx.layers, vec!["local".to_string()]);
        let _ = std::fs::remove_dir_all(&home);

        // (ii) Remote entry: the managed clone's checkout branch.
        let home = a5_derive_remote_bare_home("a5-derive-clonebr", "team", "on-call");
        let ctx = resolve_active_context()?;
        assert_eq!(ctx.name.as_deref(), Some("on-call"));
        assert_eq!(ctx.layers, vec!["team".to_string()]);
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Ladder precedence (a > b): an explicit --context (WORKESTRATE_CONTEXT)
    /// beats a branch-shaped --config-ref, with UNCHANGED strict semantics.
    #[test]
    fn explicit_context_beats_config_ref_branch() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let home = a5_derive_remote_bare_home("a5-derive-explicit", "team", "main");
        let clone = home.join("config-repos").join("team");
        a5_derive_git(&clone, &["branch", "feat-x"]);
        // Add a [contexts.work] section (layers = [team]).
        std::fs::write(
            home.join("config.toml"),
            "layers = [\"team\"]\n\n[configs.team]\nurl = \"https://example.invalid/team.git\"\nref = \"main\"\n\n[contexts.work]\nlayers = [\"team\"]\n",
        )?;
        std::env::set_var("WORKESTRATE_CONTEXT", "work");
        std::env::set_var("WORKESTRATE_CONFIG_REF", "feat-x");

        let ctx = resolve_active_context()?;

        assert_eq!(ctx.name.as_deref(), Some("work"));
        assert_eq!(ctx.layers, vec!["team".to_string()]);
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Ladder precedence (b > c): a branch-shaped --config-ref beats the
    /// first layer's checkout branch.
    #[test]
    fn config_ref_branch_beats_checkout_branch() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        // Clone is CHECKED OUT on main (the step-c candidate) while feat-x
        // exists as a branch (the step-b candidate via --config-ref).
        let home = a5_derive_remote_bare_home("a5-derive-b-over-c", "team", "main");
        let clone = home.join("config-repos").join("team");
        a5_derive_git(&clone, &["branch", "feat-x"]);
        std::env::set_var("WORKESTRATE_CONFIG_REF", "feat-x");

        let ctx = resolve_active_context()?;

        assert_eq!(
            ctx.name.as_deref(),
            Some("feat-x"),
            "the --config-ref branch must beat the checkout branch (main)"
        );
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Ladder precedence (c > d): the checkout branch beats the default —
    /// an UNDEFINED candidate rides leniently over default_context's
    /// layers; a DEFINED candidate resolves to its own context's layers.
    #[test]
    fn checkout_branch_beats_default_context() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let home = a5_derive_remote_bare_home("a5-derive-c-over-d", "team", "feat-wip");
        let clone = home.join("config-repos").join("team");
        std::fs::write(
            home.join("config.toml"),
            "layers = [\"team\"]\n\n[configs.team]\nurl = \"https://example.invalid/team.git\"\nref = \"main\"\n\n[settings]\ndefault_context = \"stable\"\n\n[contexts.stable]\nlayers = [\"team\"]\n\n[contexts.dev]\nlayers = [\"other\"]\n",
        )?;

        // Undefined candidate: name rides over the DEFAULT's layers.
        let ctx = resolve_active_context()?;
        assert_eq!(ctx.name.as_deref(), Some("feat-wip"));
        assert_eq!(
            ctx.layers,
            vec!["team".to_string()],
            "an undefined candidate falls back to default_context's layers"
        );

        // Defined candidate: its OWN layers.
        a5_derive_git(&clone, &["checkout", "--quiet", "-b", "dev"]);
        let ctx = resolve_active_context()?;
        assert_eq!(ctx.name.as_deref(), Some("dev"));
        assert_eq!(ctx.layers, vec!["other".to_string()]);
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Ladder discipline: contexts defined + candidate undefined + NO
    /// default_context → the existing hard error stands.
    #[test]
    fn undefined_candidate_with_contexts_and_no_default_errors() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let home = a5_derive_remote_bare_home("a5-derive-no-default", "team", "wip");
        std::fs::write(
            home.join("config.toml"),
            "layers = [\"team\"]\n\n[configs.team]\nurl = \"https://example.invalid/team.git\"\nref = \"main\"\n\n[contexts.work]\nlayers = [\"team\"]\n",
        )?;

        let err = resolve_active_context().expect_err("undefined candidate must hard-error");
        let msg = err.to_string();
        assert!(
            msg.contains("contexts are defined but no default_context is set"),
            "the existing hard error must stand: {msg}"
        );
        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }
}
