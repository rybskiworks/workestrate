//! Registry persistence and active-fleet resolution.

use anyhow::Result;

use crate::config::paths::registry_path;
use crate::config::{ActiveFleet, FleetEntry, Registry};

/// Load the config registry (`registry.toml` at [`registry_path`]).
/// Returns `Ok(None)` when the file does not exist (normal first-run state);
/// read/parse failures are hard errors.
///
/// G4 (fail-closed): every `[fleets.<name>]` key is validated with
/// [`validate_fleet_prefix`] after deserialize — a registry carrying an
/// invalid fleet name fails to load rather than silently admitting a name
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
    for name in registry.fleets.keys() {
        validate_fleet_prefix(name)?;
    }
    Ok(Some(registry))
}

/// Validate a fleet name as an instance-name prefix (G4). The active fleet's
/// name becomes a sandbox instance name prefix (`<fleet>-<workload>`), so it
/// must be a safe instance-name component: `^[a-z0-9][a-z0-9-]*$` — lowercase
/// alphanumerics and hyphens, starting alphanumeric — plus no trailing hyphen
/// (a trailing `-` would double the `<fleet>-<workload>` separator). Unlike
/// [`crate::microsandbox::slots::validate_instance_id`] there is NO length
/// cap and NO reserved-word/numeric rule.
///
/// Fleets are registered by `fleet add`/`fleet new` (which apply the broader
/// [`crate::config::validate_fleet_name`]) or by hand-editing the registry
/// TOML, so the load-time gate in [`load_registry`] is the single
/// enforcement point for the prefix rule; this function is its pure core.
pub fn validate_fleet_prefix(name: &str) -> Result<()> {
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
            "invalid fleet name '{}': must match ^[a-z0-9][a-z0-9-]*$ (lowercase alphanumerics \
             and hyphens, starting alphanumeric, no trailing hyphen) — the active fleet's name \
             becomes a sandbox instance name prefix",
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
/// [`register_fleet`], [`crate::config::trust_project`], and
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
/// `fleet new` — url is a filesystem path, ref=None, rev=None) as opposed
/// to a GIT-URL repo (registered via `fleet add <url>`).
///
/// FS-18: `cmd_fleet_update` previously classified `entry.rev.is_none()` as
/// local, which mis-skipped git-URL entries whose rev was simply unrecorded
/// (hand-edited registry, or a clone whose rev was never written back).
/// The explicit classifier: an entry is local-path ONLY when its url is not
/// a git remote AND no ref/rev was ever recorded. A git-URL entry with a
/// missing rev is NOT local — it is pulled (which re-records the rev).
pub fn entry_is_local_path(entry: &FleetEntry) -> bool {
    !looks_like_git_url(&entry.url) && entry.r#ref.is_none() && entry.rev.is_none()
}

/// Resolve the checkout directory of a LOCAL-PATH registry entry.
///
/// Returns `None` for git-URL entries (those resolve to the managed store
/// clone via [`crate::config::paths::fleet_dir`]). For local-path
/// entries: tilde-expand the url, then — if the result is RELATIVE —
/// resolve it against the config (`resolve_config_dir_with_kind().0`).
///
/// Rationale: a shared config may be mounted at different roots (container
/// `/home/node` vs host `/home/dev`). An absolute url breaks on the
/// other side; a relative url resolves against each side's own mount, so
/// one registry entry works in both worlds permanently.
pub fn local_entry_checkout_dir(entry: &FleetEntry) -> Option<std::path::PathBuf> {
    if !entry_is_local_path(entry) {
        return None;
    }
    let path = crate::config::paths::expand_tilde(&entry.url);
    if path.is_absolute() {
        Some(path)
    } else {
        Some(
            crate::config::paths::resolve_config_dir_with_kind()
                .0
                .join(path),
        )
    }
}

/// Classifier: whether `url` names a GIT remote (http(s)/ssh/git protocol or
/// a `.git`-suffixed path) as opposed to a plain local filesystem path.
/// Extracted from [`entry_is_local_path`] (ADR 0025): `config clone`
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
                "cannot resolve a default ref for fleet '{}': no explicit `ref` in the \
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
    entry: &FleetEntry,
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
/// `fleet update` rev-parses. Unlike [`resolve_default_ref`] it does NOT
/// probe git when an explicit ref is set (the probe could not change the
/// answer — explicit wins — so skipping it is pure savings).
pub fn effective_ref(name: &str, entry: &FleetEntry, checkout: &std::path::Path) -> Result<String> {
    match entry.r#ref.as_deref() {
        Some(r) => Ok(r.to_string()),
        None => resolve_default_ref(name, entry, checkout),
    }
}

/// Insert/replace a fleet entry in the registry. If `layers` is empty,
/// push `name` as the default layer (mirrors cmd_fleet_add's behavior).
/// When no `default_fleet` is set, the newly registered fleet becomes the
/// default so a freshly provisioned fleet is active immediately.
/// Shared by `cmd_fleet_add` (clone + register) and `cmd_fleet_new`
/// (local path + register). For local-path scaffolds, pass `git_ref = None`
/// and `rev = None` — `cmd_fleet_update` recognizes this as a local-path
/// repo and skips the pull step.
pub fn register_fleet(
    name: &str,
    url: &str,
    git_ref: Option<&str>,
    rev: Option<&str>,
) -> Result<()> {
    with_registry_lock(|registry| {
        registry.fleets.insert(
            name.to_string(),
            FleetEntry {
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
        if registry.settings.default_fleet.is_none() {
            registry.settings.default_fleet = Some(name.to_string());
        }
        Ok(())
    })
}

/// Slugify a ladder-derived fleet-name candidate (rungs b/c of the A5
/// derivation order) into the fleet-name charset `^[a-z0-9][a-z0-9-]*$`
/// ([`validate_fleet_prefix`]): lowercase ASCII, every maximal run of
/// characters outside `[a-z0-9]` mapped to a single `-`, leading/trailing
/// `-` trimmed. Returns None when nothing usable remains (e.g. `"###"`) —
/// the derivation ladder simply falls through as if no candidate existed.
///
/// Slugifying AT DERIVATION (rather than validate-and-skip) keeps the
/// common `feature/x` case working: `migration/tool-model` rides as
/// `migration-tool-model`, `feat/Foo#1.2` as `feat-foo-1-2`. Fleet names
/// become sandbox instance-name prefixes and image-tag segments, so a raw
/// branch name is never a legal fleet name as-is. Collisions between
/// distinct branches slugging equal are accepted: the fleet name is
/// identity-only in lenient mode; records are the identity authority (ADR
/// 0032 addendum §Selection ladder, note 2026-08-28).
///
/// Also reused by `crate::images::state::image_tag_context` to slugify the
/// armed inline-override ref into the image-tag ctx segment — the SAME slug
/// as the fleet candidate for the same branch, so tag ctx and fleet
/// name stay consistent for a given ref.
pub(crate) fn slugify_fleet_candidate(raw: &str) -> Option<String> {
    let mut out = String::with_capacity(raw.len());
    let mut pending_dash = false;
    for c in raw.chars() {
        if c.is_ascii_lowercase() || c.is_ascii_digit() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c);
        } else if c.is_ascii_uppercase() {
            if pending_dash && !out.is_empty() {
                out.push('-');
            }
            pending_dash = false;
            out.push(c.to_ascii_lowercase());
        } else {
            // Illegal run (incl. leading): collapse to at most one dash,
            // never leading.
            pending_dash = true;
        }
    }
    if out.is_empty() {
        return None;
    }
    Some(out)
}

/// Step (b) of the A5 derivation order (see [`resolve_active_fleet`]):
/// when `WORKESTRATE_CONFIG_REF` (`--config-ref`) names a BRANCH —
/// `refs/heads/<ref>` or `refs/remotes/origin/<ref>` present in ANY
/// Remote/GitFile entry's managed clone (first match wins) — the ref is
/// the fleet-name candidate, SLUGIFIED by
/// [`slugify_fleet_candidate`] (a candidate with no usable characters
/// yields no candidate, and the ladder falls through). A purely-sha ref
/// yields None (shas are not branches). Probe failures (missing clone, git
/// error) read as non-matches: consumption is the fail-closed layer
/// (`config::loading`'s pinned resolver errors name repo+ref).
fn config_ref_branch_candidate(registry: &Registry) -> Option<String> {
    let config_ref = std::env::var("WORKESTRATE_CONFIG_REF").ok()?;
    if config_ref.is_empty() {
        return None;
    }
    for (name, entry) in &registry.fleets {
        if source_kind(&entry.url) == ConfigSourceKind::PlainPath {
            continue;
        }
        let clone = crate::config::paths::resolve_store_dir()
            .join("fleets")
            .join(name);
        if !clone.join(".git").exists() {
            continue;
        }
        if crate::git::git_branch_ref_exists(&clone, &config_ref).unwrap_or(false) {
            return slugify_fleet_candidate(&config_ref);
        }
    }
    None
}

/// Step (c) of the A5 derivation order (see [`resolve_active_fleet`]):
/// the FIRST layer's checkout branch — [`local_entry_checkout_dir`] for
/// PlainPath entries, else the managed clone — when that dir is a git repo
/// on a branch ([`crate::git::git_checkout_branch`]; detached HEAD and
/// non-repos yield None). The branch name is the fleet-name candidate,
/// SLUGIFIED by [`slugify_fleet_candidate`] (a candidate with no usable
/// characters yields no candidate, and the ladder falls through).
fn checkout_branch_candidate(registry: &Registry) -> Option<String> {
    let first = registry.layers.first()?;
    let dir = registry
        .fleets
        .get(first)
        .and_then(local_entry_checkout_dir)
        .unwrap_or_else(|| {
            crate::config::paths::resolve_store_dir()
                .join("fleets")
                .join(first)
        });
    if !dir.join(".git").exists() {
        return None;
    }
    crate::git::git_checkout_branch(&dir)
        .ok()
        .flatten()
        .and_then(|branch| slugify_fleet_candidate(&branch))
}

/// Resolve the active fleet.
///
/// A5 derivation order (ADR 0032 addendum 2026-08-24 §Selection ladder;
/// order PINNED 2026-08-24):
///
/// a. `WORKESTRATE_FLEET` env (set by `--fleet` or by hand) —
///    UNCHANGED strict semantics: when fleets are registered the name MUST
///    be one of them (hard error otherwise); a fleets-less config ignores
///    the env and keeps the bare-layers shape (`name = None`), as before.
/// b. `WORKESTRATE_CONFIG_REF` (`--config-ref`) when it names a BRANCH —
///    see [`config_ref_branch_candidate`]. A purely-sha ref yields NO
///    fleet here.
/// c. Checkout branch: the FIRST layer's checkout — see
///    [`checkout_branch_candidate`].
/// d. Otherwise today's behavior: `[settings] default_fleet` / bare
///    `layers` when no fleets are registered / the existing hard error. The
///    ADR's "> main" final step IS this default resolution (main = the
///    stable line) — there is NO literal "main" fleet name.
///
/// Candidate-name resolution for steps b/c (the name did NOT come from an
/// explicit `--fleet`): the raw candidate (a branch name) is SLUGIFIED
/// at derivation by [`slugify_fleet_candidate`] into the fleet-name
/// charset `^[a-z0-9][a-z0-9-]*$` (2026-08-28: fleet names become
/// sandbox instance-name prefixes and image-tag segments, so raw branch
/// names like `migration/tool-model` are never legal as-is); a candidate
/// that slugifies to nothing yields NO candidate (the ladder falls
/// through to step d). The registered-fleet lookup below uses the SLUG —
/// a fleet named `migration-tool-model` matches a checkout of
/// `migration/tool-model`. When `registry.fleets` CONTAINS the
/// candidate, that fleet's single layer is used; ELSE the candidate rides
/// LENIENTLY as the fleet
/// NAME (slot prefixing, instance identity) while the LAYER LIST falls
/// back to `default_fleet`'s single layer, else the bare `layers`. When
/// fleets ARE registered AND the candidate is unregistered AND no
/// `default_fleet` exists, the existing hard error stands. Bare-layers
/// homes (no `[fleets]`) keep `name = None` UNLESS a candidate arose
/// from step b/c — then `name = Some(candidate)` over the bare layers
/// (the dev-model namespacing).
///
/// A resolved named fleet maps to a SINGLE-ELEMENT layer set (the fleet IS
/// its own layer); the merge machinery downstream is unchanged.
pub fn resolve_active_fleet() -> Result<ActiveFleet> {
    let registry = match load_registry()? {
        Some(r) => r,
        None => {
            return Ok(ActiveFleet {
                name: None,
                layers: vec![],
            });
        }
    };
    let available = || {
        let mut names: Vec<String> = registry.fleets.keys().cloned().collect();
        names.sort();
        names.join(", ")
    };

    // (a) WORKESTRATE_FLEET env (set by --fleet flag or by user) —
    //     unchanged strict semantics; a fleets-less config ignores it.
    if let Ok(name) = std::env::var("WORKESTRATE_FLEET") {
        if registry.fleets.is_empty() {
            return Ok(ActiveFleet {
                name: None,
                layers: registry.layers,
            });
        }
        if registry.fleets.contains_key(&name) {
            return Ok(ActiveFleet {
                name: Some(name.clone()),
                layers: vec![name],
            });
        }
        anyhow::bail!(
            "fleet '{}' not found in registry; available fleets: {}",
            name,
            available()
        );
    }

    // (b) / (c): a derived candidate name — the --config-ref branch, else
    //     the first layer's checkout branch.
    if let Some(candidate) =
        config_ref_branch_candidate(&registry).or_else(|| checkout_branch_candidate(&registry))
    {
        // A REGISTERED fleet of that name: its single layer.
        if registry.fleets.contains_key(&candidate) {
            return Ok(ActiveFleet {
                name: Some(candidate.clone()),
                layers: vec![candidate],
            });
        }
        // Lenient identity-only mode: the candidate rides as the fleet
        // NAME over default_fleet's layer, else the bare layers.
        if let Some(ref default) = registry.settings.default_fleet {
            if registry.fleets.contains_key(default) {
                return Ok(ActiveFleet {
                    name: Some(candidate),
                    layers: vec![default.clone()],
                });
            }
            anyhow::bail!(
                "default_fleet '{}' not found in registry fleets; available: {}",
                default,
                available()
            );
        }
        if registry.fleets.is_empty() {
            return Ok(ActiveFleet {
                name: Some(candidate),
                layers: registry.layers,
            });
        }
        // Fleets registered, candidate unregistered, no default: the
        // existing hard error stands.
        anyhow::bail!(
            "fleets are registered but no default_fleet is set; use --fleet <name> or set WORKESTRATE_FLEET env. Available fleets: {}",
            available()
        );
    }

    // (d) today's behavior, unchanged: bare layers when no fleets are
    //     registered (backward compat), else default_fleet, else the hard
    //     error.
    if registry.fleets.is_empty() {
        return Ok(ActiveFleet {
            name: None,
            layers: registry.layers,
        });
    }
    if let Some(ref default) = registry.settings.default_fleet {
        if registry.fleets.contains_key(default) {
            return Ok(ActiveFleet {
                name: Some(default.clone()),
                layers: vec![default.clone()],
            });
        }
        anyhow::bail!(
            "default_fleet '{}' not found in registry fleets; available: {}",
            default,
            available()
        );
    }

    // Fleets registered but no env/candidate/default → error
    anyhow::bail!(
        "fleets are registered but no default_fleet is set; use --fleet <name> or set WORKESTRATE_FLEET env. Available fleets: {}",
        available()
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

    // ---- Active-fleet resolution: the registry-driven ladder ----

    /// Seed a registry with `personal` + `work` fleets (default_fleet:
    /// `personal`) in the pinned config and return the config dir. Caller must
    /// hold ENV_TEST_LOCK + an EnvGuard for the ladder env keys.
    fn seed_two_fleet_home(label: &str) -> std::path::PathBuf {
        let config_dir = a5_derive_config(label);
        let mut registry = Registry::default();
        registry.settings.default_fleet = Some("personal".to_string());
        for name in ["personal", "work"] {
            registry.fleets.insert(
                name.to_string(),
                FleetEntry {
                    url: format!("https://example.invalid/{name}.git"),
                    r#ref: Some("main".to_string()),
                    rev: None,
                    secrets: None,
                    secrets_file: None,
                    age_key_file: None,
                    image_keep_last: None,
                },
            );
        }
        save_registry(&registry).expect("seed registry");
        config_dir
    }

    #[test]
    fn resolve_active_fleet_no_registry_uses_empty_layers() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let config_dir = a5_derive_config("fleet-no-reg");
        // No registry written at all.

        let fleet = resolve_active_fleet()?;

        assert_eq!(fleet.name, None);
        assert!(fleet.layers.is_empty());
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    #[test]
    fn resolve_active_fleet_no_fleets_uses_bare_layers() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let config_dir = a5_derive_config("fleet-bare");
        // Registry with bare layers, no fleets.
        std::fs::write(
            config_dir.join("config.toml"),
            "layers = [\"work\", \"personal\"]\n",
        )?;

        let fleet = resolve_active_fleet()?;

        // No fleets registered → bare layers, name is None (backward compat).
        assert_eq!(fleet.name, None);
        assert_eq!(fleet.layers, vec!["work", "personal"]);
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    #[test]
    fn resolve_active_fleet_env_ignored_when_no_fleets_registered() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let config_dir = a5_derive_config("fleet-env-ignored");
        std::fs::write(
            config_dir.join("config.toml"),
            "layers = [\"work\", \"personal\"]\n",
        )?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_FLEET", "work") };

        let fleet = resolve_active_fleet()?;

        // A fleets-less config ignores the env (unchanged strict semantics).
        assert_eq!(fleet.name, None);
        assert_eq!(fleet.layers, vec!["work", "personal"]);
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    #[test]
    fn resolve_active_fleet_env_overrides_default() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let config_dir = seed_two_fleet_home("fleet-env");
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_FLEET", "work") };

        let fleet = resolve_active_fleet()?;

        assert_eq!(fleet.name.as_deref(), Some("work"));
        assert_eq!(fleet.layers, vec!["work"]);
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    #[test]
    fn resolve_active_fleet_default_when_no_env() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let config_dir = seed_two_fleet_home("fleet-default");

        let fleet = resolve_active_fleet()?;

        assert_eq!(fleet.name.as_deref(), Some("personal"));
        assert_eq!(fleet.layers, vec!["personal"]);
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    #[test]
    fn resolve_active_fleet_unknown_env_errors() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let config_dir = seed_two_fleet_home("fleet-unknown-env");
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_FLEET", "nonexistent") };

        let err = resolve_active_fleet().expect_err("an unknown fleet must error");

        let msg = err.to_string();
        assert!(
            msg.contains("not found"),
            "error should mention 'not found': {msg}"
        );
        assert!(
            msg.contains("personal") && msg.contains("work"),
            "error should list the available fleets: {msg}"
        );
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    #[test]
    fn resolve_active_fleet_fleets_but_no_default_no_env_errors() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let config_dir = a5_derive_config("fleet-nodflt");
        // Fleets registered but no default_fleet and no env.
        std::fs::write(
            config_dir.join("config.toml"),
            "[fleets.personal]\nurl = \"https://example.invalid/personal.git\"\n\n[fleets.work]\nurl = \"https://example.invalid/work.git\"\n",
        )?;

        let err = resolve_active_fleet().expect_err("no default_fleet must error");

        let msg = err.to_string();
        assert!(
            msg.contains("default_fleet") || msg.contains("--fleet"),
            "error should mention default_fleet or --fleet: {msg}"
        );
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    #[test]
    fn resolve_active_fleet_unregistered_default_errors() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let config_dir = a5_derive_config("fleet-ghost-default");
        std::fs::write(
            config_dir.join("config.toml"),
            "[settings]\ndefault_fleet = \"ghost\"\n\n[fleets.personal]\nurl = \"https://example.invalid/personal.git\"\n",
        )?;

        let err = resolve_active_fleet().expect_err("an unregistered default_fleet must error");

        let msg = err.to_string();
        assert!(
            msg.contains("default_fleet 'ghost' not found"),
            "error must name the unregistered default_fleet: {msg}"
        );
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    // ---- FS-18: local-path vs git-URL registry entry classification ----

    fn entry(url: &str, git_ref: Option<&str>, rev: Option<&str>) -> FleetEntry {
        FleetEntry {
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
    /// path — `cmd_fleet_update` must not skip it (it gets pulled, which
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

    /// Genuine local-path entries (what `fleet new` registers: filesystem
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
        // the `fleet new` shape, so not local.
        assert!(!entry_is_local_path(&entry(
            "/home/user/my-config",
            Some("main"),
            None
        )));
    }

    // ---- FN-5: atomic save + advisory lock ----

    /// Set WORKESTRATE_CONFIG to a fresh temp dir (ConfigDirKind::Env → registry at
    /// `<tmp>/config.toml`) and return the dir. Caller must hold
    /// ENV_TEST_LOCK and an EnvGuard for CONFIG_ENV_KEYS.
    fn pin_config(label: &str) -> std::path::PathBuf {
        let config_dir = uniq_dir(label);
        std::fs::create_dir_all(&config_dir).expect("create pinned config dir");
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG", &config_dir) };
        config_dir
    }

    #[test]
    fn save_registry_writes_atomically_and_leaves_no_tmp() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("fn5-atomic");

        let mut registry = Registry::default();
        registry.fleets.insert(
            "personal".to_string(),
            FleetEntry {
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
        assert_eq!(loaded.fleets.len(), 1);
        assert_eq!(
            loaded.fleets["personal"].url,
            "https://example.invalid/personal.git"
        );
        assert_eq!(loaded.fleets["personal"].rev.as_deref(), Some("abc123"));

        // No tmp file left behind next to config.toml.
        let tmp = registry_path().with_extension("toml.tmp");
        assert!(!tmp.exists(), "tmp file must not survive the rename");
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    // ---- local_entry_checkout_dir (shared-config duality) ----

    #[test]
    fn local_entry_checkout_dir_resolves_relative_against_home() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("local-entry-rel");
        let e = entry("fleets/personal", None, None);
        assert_eq!(
            local_entry_checkout_dir(&e),
            Some(config_dir.join("fleets/personal"))
        );
        let _ = std::fs::remove_dir_all(&config_dir);
    }

    #[test]
    fn local_entry_checkout_dir_passes_absolute_and_tilde_through() {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
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
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("fn5-interrupt");

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
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    #[test]
    fn register_fleet_removes_lock_file_and_persists_entry() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("fn5-lock");

        register_fleet(
            "personal",
            "https://example.invalid/personal.git",
            Some("main"),
            Some("abc123"),
        )?;

        // Lock file is removed after the critical section.
        let lock_path = registry_path().with_file_name(REGISTRY_LOCK_NAME);
        assert!(
            !lock_path.exists(),
            "registry lock must be released after register_fleet"
        );

        // The entry persisted (load → mutate → save ran under the lock).
        let loaded = load_registry()?.expect("registry should exist");
        assert!(loaded.fleets.contains_key("personal"));
        assert_eq!(loaded.layers, vec!["personal".to_string()]);
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    #[test]
    fn trust_round_trip_releases_lock() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("fn5-trust");

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
        let _ = std::fs::remove_dir_all(&config_dir);
        let _ = std::fs::remove_dir_all(&project);
        Ok(())
    }

    // ---- G4: fleet-name validation (fail-closed at registry load) ----

    #[test]
    fn validate_fleet_prefix_accepts_valid_names() {
        for ok in ["personal", "work-2", "a", "0", "a-b-c", "team2"] {
            validate_fleet_prefix(ok)
                .unwrap_or_else(|e| panic!("legitimate fleet '{ok}' rejected: {e}"));
        }
    }

    #[test]
    fn validate_fleet_prefix_rejects_invalid_names_and_names_them() {
        for bad in ["has space", "has@at", "Upper", "-leading", "trailing-", ""] {
            let err = validate_fleet_prefix(bad).unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains(&format!("'{bad}'")),
                "error must name the offending fleet '{bad}': {msg}"
            );
            assert!(
                msg.contains("^[a-z0-9][a-z0-9-]*$"),
                "error must state the pattern: {msg}"
            );
        }
    }

    /// A registry TOML carrying a bad `[fleets.<name>]` key must FAIL
    /// `load_registry` (fail-closed; G4), naming the offending fleet.
    #[test]
    fn load_registry_rejects_invalid_fleet_key() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(CONFIG_ENV_KEYS);
        let config_dir = pin_config("g4-bad-fleet");
        std::fs::write(
            registry_path(),
            "[settings]\ndefault_fleet = \"personal\"\n\n[fleets.personal]\nurl = \"https://example.invalid/personal.git\"\n\n[fleets.\"has space\"]\nurl = \"https://example.invalid/team.git\"\n",
        )?;

        let err = load_registry().unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("has space"),
            "error must name the offending fleet key: {msg}"
        );
        assert!(
            msg.contains("invalid fleet name"),
            "error must be the G4 validation error: {msg}"
        );
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    /// A registry whose fleets are all valid loads unchanged (the G4 gate
    /// admits the existing well-formed registries).
    #[test]
    fn load_registry_accepts_valid_fleet_keys() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let config_dir = seed_two_fleet_home("g4-ok-fleet");

        let registry = load_registry()?.expect("valid registry must load");
        assert!(registry.fleets.contains_key("personal"));
        assert!(registry.fleets.contains_key("work"));
        let _ = std::fs::remove_dir_all(&config_dir);
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

    // ---- A5 Session 3a: the fleet derivation order (ADR 0032 addendum) ----
    //
    // Fixture conventions: pinned WORKESTRATE_CONFIG (ConfigDirKind::Env: registry
    // at <config>/config.toml, managed clones at <config>/fleets/<name>),
    // REAL temp git repos (the git.rs precedent: git on the pinned PATH),
    // ENV_TEST_LOCK + EnvGuard over the discovery vars PLUS the two ladder
    // env vars (WORKESTRATE_FLEET is removed by default and set per test;
    // WORKESTRATE_CONFIG_REF likewise).

    const A5_DERIVE_ENV_KEYS: &[&str] = &[
        "WORKESTRATE_CONFIG",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_STATE_HOME",
        "WORKESTRATE_FLEET_DIR",
        "WORKESTRATE_NO_PROJECT_CONFIG",
        "WORKESTRATE_INVOKE_CWD",
        "HOME",
        "WORKESTRATE_FLEET",
        "WORKESTRATE_CONFIG_REF",
    ];

    /// Pin the config and neutralize the ladder env vars. Caller holds
    /// ENV_TEST_LOCK; the EnvGuard is captured by the caller BEFORE this.
    fn a5_derive_config(label: &str) -> std::path::PathBuf {
        let config_dir = pin_config(label);
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("WORKESTRATE_FLEET") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("WORKESTRATE_CONFIG_REF") };
        config_dir
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
    /// (`<config>/fleets/<name>`) on branch `checkout_branch`, as the
    /// config's default_fleet (`layers = [<name>]`).
    fn a5_derive_remote_config(
        label: &str,
        name: &str,
        checkout_branch: &str,
    ) -> std::path::PathBuf {
        let config_dir = a5_derive_config(label);
        let clone = config_dir.join("fleets").join(name);
        a5_derive_repo(&clone, checkout_branch);
        std::fs::write(
            config_dir.join("config.toml"),
            format!(
                "layers = [\"{name}\"]\n\n[settings]\ndefault_fleet = \"{name}\"\n\n[fleets.{name}]\nurl = \"https://example.invalid/{name}.git\"\nref = \"main\"\n"
            ),
        )
        .expect("write registry");
        config_dir
    }

    /// Step b: a branch-shaped --config-ref namespaces the session —
    /// name = Some(<branch>) over the default fleet's single layer (the
    /// dev-model namespacing).
    #[test]
    fn config_ref_branch_namespaces_the_default_fleet() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let config_dir = a5_derive_remote_config("a5-derive-cfgref", "team", "main");
        let clone = config_dir.join("fleets").join("team");
        // The branch exists in the clone but is NOT checked out (isolating
        // step b from step c, which would report the checkout branch).
        a5_derive_git(&clone, &["branch", "feat-x"]);
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG_REF", "feat-x") };

        let ctx = resolve_active_fleet()?;

        assert_eq!(ctx.name.as_deref(), Some("feat-x"));
        assert_eq!(ctx.layers, vec!["team".to_string()]);
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    /// Step b with a purely-sha --config-ref yields NO derived fleet name
    /// (shas are not branches); with no checkout-branch candidate either
    /// (detached HEAD), the default_fleet resolves.
    #[test]
    fn sha_config_ref_yields_no_derived_fleet_name() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let config_dir = a5_derive_remote_config("a5-derive-sha", "team", "main");
        let clone = config_dir.join("fleets").join("team");
        let sha = crate::git::git_rev_parse(&clone)?;
        // Detach HEAD so step c cannot name a fleet either — isolating
        // the sha-ref behavior at step b.
        crate::git::git_checkout_rev(&clone, &sha)?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG_REF", &sha) };

        let ctx = resolve_active_fleet()?;

        assert_eq!(
            ctx.name.as_deref(),
            Some("team"),
            "a purely-sha --config-ref yields no candidate; the default_fleet resolves"
        );
        assert_eq!(ctx.layers, vec!["team".to_string()]);
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    /// Step c: with no --config-ref, the FIRST layer's checkout branch is
    /// the candidate — a PlainPath working repo (local_entry_checkout_dir)
    /// and a managed clone alike.
    #[test]
    fn checkout_branch_namespaces_the_default_fleet() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);

        // (i) PlainPath first layer on branch wip.
        let config_dir = a5_derive_config("a5-derive-plain");
        let work = config_dir.join("my-work-config");
        a5_derive_repo(&work, "wip");
        std::fs::write(
            config_dir.join("config.toml"),
            format!(
                "layers = [\"local\"]\n\n[settings]\ndefault_fleet = \"local\"\n\n[fleets.local]\nurl = \"{}\"\n",
                work.display()
            ),
        )?;
        let ctx = resolve_active_fleet()?;
        assert_eq!(ctx.name.as_deref(), Some("wip"));
        assert_eq!(ctx.layers, vec!["local".to_string()]);
        let _ = std::fs::remove_dir_all(&config_dir);

        // (ii) Remote entry: the managed clone's checkout branch.
        let config_dir = a5_derive_remote_config("a5-derive-clonebr", "team", "on-call");
        let ctx = resolve_active_fleet()?;
        assert_eq!(ctx.name.as_deref(), Some("on-call"));
        assert_eq!(ctx.layers, vec!["team".to_string()]);
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    /// Ladder precedence (a > b): an explicit --fleet (WORKESTRATE_FLEET)
    /// beats a branch-shaped --config-ref, with UNCHANGED strict semantics.
    #[test]
    fn explicit_fleet_beats_config_ref_branch() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let config_dir = a5_derive_remote_config("a5-derive-explicit", "team", "main");
        let clone = config_dir.join("fleets").join("team");
        a5_derive_git(&clone, &["branch", "feat-x"]);
        // Register a second fleet `work` alongside the default `team`.
        std::fs::write(
            config_dir.join("config.toml"),
            "layers = [\"team\"]\n\n[settings]\ndefault_fleet = \"team\"\n\n[fleets.team]\nurl = \"https://example.invalid/team.git\"\nref = \"main\"\n\n[fleets.work]\nurl = \"https://example.invalid/work.git\"\n",
        )?;
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_FLEET", "work") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG_REF", "feat-x") };

        let ctx = resolve_active_fleet()?;

        assert_eq!(ctx.name.as_deref(), Some("work"));
        assert_eq!(ctx.layers, vec!["work".to_string()]);
        let _ = std::fs::remove_dir_all(&config_dir);
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
        let config_dir = a5_derive_remote_config("a5-derive-b-over-c", "team", "main");
        let clone = config_dir.join("fleets").join("team");
        a5_derive_git(&clone, &["branch", "feat-x"]);
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG_REF", "feat-x") };

        let ctx = resolve_active_fleet()?;

        assert_eq!(
            ctx.name.as_deref(),
            Some("feat-x"),
            "the --config-ref branch must beat the checkout branch (main)"
        );
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    /// Ladder precedence (c > d): the checkout branch beats the default —
    /// an UNREGISTERED candidate rides leniently over default_fleet's
    /// layer; a REGISTERED candidate resolves to its own single layer.
    #[test]
    fn checkout_branch_beats_default_fleet() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let config_dir = a5_derive_remote_config("a5-derive-c-over-d", "team", "feat-wip");
        let clone = config_dir.join("fleets").join("team");
        std::fs::write(
            config_dir.join("config.toml"),
            "layers = [\"team\"]\n\n[fleets.team]\nurl = \"https://example.invalid/team.git\"\nref = \"main\"\n\n[settings]\ndefault_fleet = \"stable\"\n\n[fleets.stable]\nurl = \"https://example.invalid/stable.git\"\n\n[fleets.dev]\nurl = \"https://example.invalid/dev.git\"\n",
        )?;

        // Unregistered candidate: the name rides over the DEFAULT's layer.
        let ctx = resolve_active_fleet()?;
        assert_eq!(ctx.name.as_deref(), Some("feat-wip"));
        assert_eq!(
            ctx.layers,
            vec!["stable".to_string()],
            "an unregistered candidate falls back to default_fleet's layer"
        );

        // Registered candidate: its OWN single layer.
        a5_derive_git(&clone, &["checkout", "--quiet", "-b", "dev"]);
        let ctx = resolve_active_fleet()?;
        assert_eq!(ctx.name.as_deref(), Some("dev"));
        assert_eq!(ctx.layers, vec!["dev".to_string()]);
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    /// Ladder discipline: fleets registered + candidate unregistered + NO
    /// default_fleet → the hard error stands.
    #[test]
    fn undefined_candidate_with_fleets_and_no_default_errors() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let config_dir = a5_derive_remote_config("a5-derive-no-default", "team", "wip");
        std::fs::write(
            config_dir.join("config.toml"),
            "layers = [\"team\"]\n\n[fleets.team]\nurl = \"https://example.invalid/team.git\"\nref = \"main\"\n\n[fleets.work]\nurl = \"https://example.invalid/work.git\"\n",
        )?;

        let err = resolve_active_fleet().expect_err("undefined candidate must hard-error");
        let msg = err.to_string();
        assert!(
            msg.contains("fleets are registered but no default_fleet is set"),
            "the hard error must stand: {msg}"
        );
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    // ---- Candidate slugification (2026-08-28): ladder-derived fleet-name
    // candidates ride in the fleet-name charset ----

    /// The pure slug core: lowercase ASCII, every maximal illegal run → one
    /// `-`, leading/trailing `-` trimmed, None when nothing usable remains.
    #[test]
    fn slugify_fleet_candidate_maps_illegal_runs_to_dashes() {
        // Slash (the common feature/x case), uppercase, `#`, dots.
        assert_eq!(
            slugify_fleet_candidate("feat/Foo#1.2").as_deref(),
            Some("feat-foo-1-2")
        );
        assert_eq!(
            slugify_fleet_candidate("migration/tool-model").as_deref(),
            Some("migration-tool-model")
        );
        assert_eq!(
            slugify_fleet_candidate("Fix__Big--Thing").as_deref(),
            Some("fix-big-thing")
        );
        // Maximal illegal runs collapse to a single dash; leading/trailing
        // runs are trimmed (never a leading/trailing dash).
        assert_eq!(slugify_fleet_candidate("--wip--").as_deref(), Some("wip"));
        assert_eq!(slugify_fleet_candidate("//a//b//").as_deref(), Some("a-b"));
        // All-illegal input yields NO candidate (the ladder falls through).
        assert_eq!(slugify_fleet_candidate("###"), None);
        assert_eq!(slugify_fleet_candidate(""), None);
        // A legal branch name passes through UNCHANGED.
        assert_eq!(
            slugify_fleet_candidate("feat-wip-2").as_deref(),
            Some("feat-wip-2")
        );
        assert_eq!(slugify_fleet_candidate("main").as_deref(), Some("main"));
        // Every Some result satisfies the fleet-name gate.
        for raw in ["feat/Foo#1.2", "migration/tool-model", "feat-wip-2"] {
            let slug = slugify_fleet_candidate(raw).expect("slug");
            validate_fleet_prefix(&slug).expect("slug must satisfy validate_fleet_prefix");
        }
    }

    /// Step c slugifies: a checkout branch outside the fleet-name charset
    /// namespaces the session under the SLUG, not the raw name.
    #[test]
    fn checkout_branch_candidate_is_slugified() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let config_dir =
            a5_derive_remote_config("a5-derive-slug-c", "team", "migration/tool-model");

        let ctx = resolve_active_fleet()?;
        assert_eq!(ctx.name.as_deref(), Some("migration-tool-model"));
        assert_eq!(ctx.layers, vec!["team".to_string()]);
        let _ = std::fs::remove_dir_all(&config_dir);

        // A branch with `#`, uppercase, and dots.
        let config_dir = a5_derive_remote_config("a5-derive-slug-c2", "team", "feat/Foo#1.2");
        let ctx = resolve_active_fleet()?;
        assert_eq!(ctx.name.as_deref(), Some("feat-foo-1-2"));
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    /// Step b slugifies: a branch-shaped --config-ref outside the charset
    /// rides under its slug.
    #[test]
    fn config_ref_branch_candidate_is_slugified() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        let config_dir = a5_derive_remote_config("a5-derive-slug-b", "team", "main");
        let clone = config_dir.join("fleets").join("team");
        a5_derive_git(&clone, &["branch", "feat/Foo#1.2"]);
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG_REF", "feat/Foo#1.2") };

        let ctx = resolve_active_fleet()?;
        assert_eq!(ctx.name.as_deref(), Some("feat-foo-1-2"));
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }

    /// An all-illegal branch slugifies to NOTHING: the ladder falls through
    /// to step d (the default_fleet resolves) rather than riding an illegal
    /// fleet name.
    #[test]
    fn all_illegal_branch_yields_no_candidate() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let _g = EnvGuard::capture(A5_DERIVE_ENV_KEYS);
        // `###` is a valid git branch name (check-ref-format) but has no
        // usable fleet-name characters.
        let config_dir = a5_derive_remote_config("a5-derive-slug-none", "team", "###");

        let ctx = resolve_active_fleet()?;
        assert_eq!(
            ctx.name.as_deref(),
            Some("team"),
            "an all-illegal checkout branch yields NO candidate; the default_fleet resolves"
        );
        assert_eq!(ctx.layers, vec!["team".to_string()]);
        let _ = std::fs::remove_dir_all(&config_dir);
        Ok(())
    }
}
