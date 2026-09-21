//! Config command (`workestrate config …`): initialize the resolved config
//! as a dotfiles-style git repo (spec 10 Task 3, Decision B — the config
//! tracks `config.toml` + `overrides.toml`; store dirs and secret material
//! stay out of the index), and provision a NEW config from an existing one
//! (`config clone <src> [dest]`, ADR 0025).

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::cli_actions::ConfigAction;
use crate::config::{self, FleetEntry, Registry};

/// `.gitignore` written into the config. The store dirs and plaintext
/// secret material stay untracked; age ciphertext (`*.enc`) stays committable.
const CONFIG_GITIGNORE: &str = "\
# workestrate config — installed by `workestrate config init`.
/fleets/
/sources/
/state/
/cache/
*.agekey
age.txt
*.pem
id_rsa*
.env
";

/// tombi configuration written into the config (spec 15 §2.3). The
/// `[[schemas]]` section carries three mappings: the full schema
/// (`workestrate.schema.json`) lints the single-file `workestrate.toml` plus
/// the directory-mode full-file entries `default.toml`/`secrets.toml`; the
/// workload subschema (`workestrate-workload.schema.json`) lints capsule
/// files under `workestrate/workloads/`; the registry schema
/// (`registry.schema.json`) lints the config `config.toml`. `overrides.toml`
/// stays format-only. Store dirs and the lock stay excluded.
const CONFIG_TOMBI_TOML: &str = r#"# tombi configuration for the workestrate config.
# tombi 1.2.5+ — see https://tombi-toml.github.io/tombi/

toml-version = "v1.0.0"

[format.rules]
line-width = 100
indent-width = 2

[lint.rules]
tables-out-of-order = "warn"
dotted-keys-out-of-order = "warn"

[schema]
enabled = true
strict = true

[[schemas]]
path = "schemas/workestrate.schema.json"
include = ["fleets/*/workestrate.toml", "fleets/*/workestrate/default.toml", "fleets/*/workestrate/secrets.toml"]

[[schemas]]
path = "schemas/workestrate-workload.schema.json"
include = ["fleets/*/workestrate/workloads/**/*.toml"]

[[schemas]]
path = "schemas/registry.schema.json"
include = ["config.toml"]

[files]
include = [
  "fleets/*/workestrate.toml",
  "fleets/*/workestrate/**/*.toml",
  "config.toml",
  "overrides.toml",
]
exclude = [
  "workestrate.lock",
  "secrets/**",
  "sources/**",
  "state/**",
]
"#;

/// Vendored JSON Schema for `workestrate.toml`, embedded at compile time so
/// the config's tombi schema lint works offline. Same relative path as the
/// scaffold copy (`src/scaffold/mod.rs`) — config_cmd.rs sits at the same depth.
const CONFIG_SCHEMA_JSON: &str = include_str!("../../../../schemas/workestrate.schema.json");

/// Vendored JSON Schema for workload capsule files
/// (`workestrate/workloads/<name>/workload.toml`), embedded at compile time
/// so the config's tombi schema lint works offline. Same relative path as the
/// scaffold copy (`src/scaffold/mod.rs`) — config_cmd.rs sits at the same depth.
const CONFIG_SCHEMA_WORKLOAD_JSON: &str =
    include_str!("../../../../schemas/workestrate-workload.schema.json");

/// Vendored JSON Schema for the config registry (`config.toml`),
/// embedded at compile time so the config's tombi schema lint works offline.
/// Same relative path as the scaffold copy (`src/scaffold/mod.rs`).
const CONFIG_SCHEMA_REGISTRY_JSON: &str = include_str!("../../../../schemas/registry.schema.json");

/// Pre-commit hook installed into the config: rejects embedded git repos
/// (gitlinks, mode 160000), store-dir paths, and secret material. The tombi
/// TOML gates are best-effort: absent or version-mismatched tombi skips them
/// with an audible stderr echo (never a hard fail), matching the fleet shims;
/// exactness is enforced by `nix flake check`. Kept as a const so tests can
/// assert on the canonical content.
const CONFIG_PRE_COMMIT_HOOK: &str = r#"#!/bin/sh
# Installed by `workestrate config init` — guards the workestrate config git repo.
# Rejects embedded git repos (gitlinks, mode 160000), store-dir paths, and
# secret material. Encrypted secrets (*.enc) stay committable.
fail=0
gitlinks=$(git ls-files -s | awk '$1 == "160000" { print $4 }')
if [ -n "$gitlinks" ]; then
    echo "config pre-commit: refusing embedded git repos (gitlinks, mode 160000):" >&2
    echo "$gitlinks" >&2
    fail=1
fi
staged=$(git diff --cached --name-only --diff-filter=ACM)
if printf '%s\n' "$staged" | grep -qE '^(fleets|sources|state)/'; then
    echo "config pre-commit: refusing store-dir paths (fleets/ sources/ state/):" >&2
    printf '%s\n' "$staged" | grep -E '^(fleets|sources|state)/' >&2
    fail=1
fi
if printf '%s\n' "$staged" | grep -qE '(^|/)([^/]*\.agekey|age\.txt|[^/]*\.pem|id_rsa[^/]*|\.env)$'; then
    echo "config pre-commit: refusing secret material (*.agekey, age.txt, *.pem, id_rsa*, .env):" >&2
    printf '%s\n' "$staged" | grep -E '(^|/)([^/]*\.agekey|age\.txt|[^/]*\.pem|id_rsa[^/]*|\.env)$' >&2
    fail=1
fi
[ "$fail" -eq 0 ] || exit 1
# tombi TOML gates (best-effort — skipped with an audible echo when tombi is
# absent or version-mismatched; exactness is enforced by `nix flake check`).
# Canonical hooks note: workestrate docs/nix/store-hygiene-and-gc.md §"Git hooks vs GC"
TOMBI_REQUIRED="1.2.5"
if command -v tombi >/dev/null 2>&1; then
    tombi_version="$(tombi --version | awk '{print $2}')"
    if [ "$tombi_version" != "$TOMBI_REQUIRED" ]; then
        echo "config pre-commit: tombi version mismatch (found '${tombi_version:-unknown}', want $TOMBI_REQUIRED); skipping tombi gates" >&2
    else
        tombi format --check || exit 1
        tombi lint --error-on-warnings || exit 1
    fi
else
    echo "config pre-commit: tombi not found; skipping tombi gates" >&2
fi
exit 0
"#;

/// Highest `settings.config_version` this binary can provision FROM (ADR 0025
/// fail-before-write compat check). Absent/1/2 are accepted; anything newer
/// is a hard error ("config created by a newer workestrate").
const MAX_SUPPORTED_CONFIG_VERSION: u32 = 2;

pub fn cmd_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Init {} => cmd_config_init(),
        ConfigAction::Clone { src, dest } => cmd_config_clone(&src, dest.as_deref()),
    }
}

/// `workestrate config init`.
///
/// Scaffolds the RESOLVED config (no path flag; `WORKESTRATE_CONFIG` /
/// legacy XDG / default resolution decides which config) as a dotfiles-style
/// git repo. Idempotent: re-running on an initialized config is a pure no-op.
/// For an empty scaffold at a custom path, use the global flag:
/// `workestrate --config <path> config init`. To provision from an existing
/// config, use `workestrate config clone <src> [dest]`. To register a fleet
/// afterwards, use `workestrate fleet add <url> <name>`.
pub fn cmd_config_init() -> Result<()> {
    // 1. Resolve the config via the standard precedence (env → legacy XDG →
    //    default). No path flag; resolution is entirely what
    //    resolve_config_dir_with_kind already does.
    let config_dir = config::resolve_config_dir_with_kind().0;
    std::fs::create_dir_all(&config_dir)?;

    // 2. Idempotency: an existing .git means the config is already a repo.
    //    Pure no-op — do NOT rewrite .gitignore/hook.
    if config_dir.join(".git").exists() {
        println!(
            "Config at {} is already initialized as a git repo (nothing to do).",
            config_dir.display()
        );
        return Ok(());
    }

    // 3. Ensure the config layout dirs exist (adds only what is missing;
    //    idempotent over a populated config per spec §2 "Config init" item 3).
    let ensured = ensure_layout_dirs(&config_dir);

    // 4. git init — the git layer is the entire point of this command, so
    //    propagate failures (unlike cmd_fleet_new's warn-and-continue).
    crate::git::git_init(&config_dir)?;

    // 5. .gitignore: store dirs + secret material untracked; age ciphertext
    //    (*.enc) stays committable.
    std::fs::write(config_dir.join(".gitignore"), CONFIG_GITIGNORE)?;

    // 5b. tombi toolchain files (spec 15): config tombi.toml + the vendored
    //     schema the config's [[schemas]] entry resolves against. The
    //     pre-commit hook below runs the tombi gates when tombi is present.
    std::fs::write(config_dir.join("tombi.toml"), CONFIG_TOMBI_TOML)?;
    std::fs::create_dir_all(config_dir.join("schemas"))?;
    std::fs::write(
        config_dir.join("schemas").join("workestrate.schema.json"),
        CONFIG_SCHEMA_JSON,
    )?;
    std::fs::write(
        config_dir
            .join("schemas")
            .join("workestrate-workload.schema.json"),
        CONFIG_SCHEMA_WORKLOAD_JSON,
    )?;
    std::fs::write(
        config_dir.join("schemas").join("registry.schema.json"),
        CONFIG_SCHEMA_REGISTRY_JSON,
    )?;

    // 6. Pre-commit hook (executable on unix).
    install_pre_commit_hook(&config_dir)?;

    // 7. ADR 0025(e): write the generated lock. This runs AFTER a successful
    //    init only — never on the idempotent no-op path above (which leaves
    //    the config, including its lock, untouched). The registry may not exist
    //    yet; a bare init writes a lock with an empty `fleets` map.
    let registry = config::load_registry()?.unwrap_or_default();
    let lock = config::lock_from_registry(&registry, &config_dir);
    config::save_config_lock_to(&config_dir, &lock)?;

    // 8. Summary + next steps.
    print_summary(&config_dir, &ensured);
    Ok(())
}

/// `workestrate config clone <src> [<dest>]`.
///
/// Provision the dest config from an existing config (ADR 0025; see
/// [`provision_config_from`]). The positional `<dest>` selects where the new
/// config is created (default: the resolved config); it must not exist or
/// be empty.
#[allow(unsafe_code)]
pub fn cmd_config_clone(src: &str, dest: Option<&str>) -> Result<()> {
    // Resolve the dest config: positional dest (`~/` expanded; relative paths
    // resolve against cwd at use time) or the standard resolution.
    let config_dir = match dest {
        Some(d) => config::expand_tilde(d),
        None => config::resolve_config_dir_with_kind().0,
    };

    // DEST NON-EMPTY GUARD (spec §1: "dest must not exist or be empty — bail
    // if non-empty"). Cloning is NOT idempotent toward an existing populated
    // config (unlike `config init`).
    if dir_exists_and_is_non_empty(&config_dir) {
        anyhow::bail!(
            "destination '{}' already exists and is not empty; \
             'config clone' requires a missing or empty directory",
            config_dir.display()
        );
    }

    // When a positional dest is given, pin the process config to it EARLY so
    // every env-based helper (fleet_dir, save_registry, cmd_fleet_add,
    // the post-flight validate) operates on dest.
    if dest.is_some() {
        // SAFETY: pins WORKESTRATE_CONFIG early in the command handler so
        // env-based helpers operate on dest; sequential command flow, no
        // concurrent env mutation.
        unsafe { std::env::set_var("WORKESTRATE_CONFIG", config_dir.to_string_lossy().as_ref()) };
    }

    provision_config_from(src, &config_dir)
}

/// True when `dir` exists and contains at least one entry.
fn dir_exists_and_is_non_empty(dir: &Path) -> bool {
    dir.is_dir()
        && std::fs::read_dir(dir)
            .map(|mut it| it.next().is_some())
            .unwrap_or(false)
}

/// Create the config layout dirs (`fleets/`, `sources/`, `state/`,
/// `secrets/`), returning the names that were newly created.
fn ensure_layout_dirs(config_dir: &Path) -> Vec<&'static str> {
    let mut ensured: Vec<&str> = Vec::new();
    for dir in ["fleets", "sources", "state", "secrets"] {
        let path = config_dir.join(dir);
        if !path.exists() {
            if let Err(e) = std::fs::create_dir_all(&path) {
                eprintln!("warning: could not create {}: {}", path.display(), e);
                continue;
            }
            ensured.push(dir);
        }
    }
    ensured
}

/// How one `[fleets.<name>]` entry can be reproduced on this machine
/// (ADR 0025 §2 step 2 reproducibility report).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReproKind {
    /// The registered url is a git remote — re-clone it.
    ReproducibleViaUrl,
    /// The registered url is a local path and the SOURCE config is local —
    /// copy the working copy out of the source tree.
    LocalOnlyCopy,
    /// The registered url is a local path but the SOURCE came from a git
    /// URL — the working copy is unreachable from here; skip (loudly).
    UnreproducibleOnRemoteSource,
}

/// The `config clone <src>` provisioning path (ADR 0025 §2 steps 1–10).
fn provision_config_from(from: &str, dest: &Path) -> Result<()> {
    // -- Step 1: RESOLVE SRC ---------------------------------------------
    // A git URL (or a local path ending in .git) is cloned to a temp dir and
    // the clone is treated as src. Anything else is a filesystem path:
    // absolute as-is, relative resolved against cwd.
    let mut _temp_clone: Option<RemoteSrcClone> = None;
    let (src, src_was_remote) = if config::looks_like_git_url(from) {
        let clone = RemoteSrcClone::clone_from(from)?;
        let path = clone.path().to_path_buf();
        _temp_clone = Some(clone);
        (path, true)
    } else {
        let expanded = config::expand_tilde(from);
        let path = if expanded.is_absolute() {
            expanded
        } else {
            config::invoke_cwd_or_err()?.join(expanded)
        };
        (path, false)
    };
    if !src.exists() {
        anyhow::bail!(
            "provisioning source '{}' does not exist (resolved to {})",
            from,
            src.display()
        );
    }

    // -- Step 2: PRE-FLIGHT (fail BEFORE writing anything — dest must not
    //    even be created on failure) --------------------------------------
    let src_registry_path = src.join("config.toml");
    if !src_registry_path.exists() {
        anyhow::bail!(
            "provisioning source '{}' has no config.toml (not a workestrate config)",
            src.display()
        );
    }
    let content = std::fs::read_to_string(&src_registry_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", src_registry_path.display(), e))?;
    let src_registry: Registry = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("failed to parse {}: {}", src_registry_path.display(), e))?;
    if let Some(v) = src_registry.settings.config_version
        && v > MAX_SUPPORTED_CONFIG_VERSION
    {
        anyhow::bail!(
            "config created by a newer workestrate (settings.config_version = {} > {}); \
                 upgrade this workestrate before provisioning from {}",
            v,
            MAX_SUPPORTED_CONFIG_VERSION,
            src.display()
        );
    }
    // Source lock (spec 11 §2 step 2: read src workestrate.lock if present,
    // same version rules). Absent file → legacy path (Ok(None)); absent
    // `version` field → oldest; newer-than-supported → the hard "config created
    // by a newer workestrate" error. All BEFORE any writes (zero residue).
    let src_lock = config::load_config_lock_from(&src)?;

    // Reproducibility report (per fleet entry).
    let mut repro: Vec<(String, FleetEntry, ReproKind)> = Vec::new();
    for (name, entry) in &src_registry.fleets {
        let kind = if config::looks_like_git_url(&entry.url) {
            ReproKind::ReproducibleViaUrl
        } else if src_was_remote {
            ReproKind::UnreproducibleOnRemoteSource
        } else {
            ReproKind::LocalOnlyCopy
        };
        repro.push((name.clone(), entry.clone(), kind));
    }
    // Deterministic output regardless of HashMap iteration order.
    repro.sort_by(|a, b| a.0.cmp(&b.0));

    // -- Step 3+4: MATERIALIZE dest + REGISTRY LAYER ----------------------
    // A git source config is full-cloned into dest (origin comes for free; the
    // clone carries config.toml, overrides.toml, .gitignore, the hook, and
    // any committed *.enc). A non-git source gets config.toml
    // (+ overrides.toml) copied and the standard scaffolding. The layout
    // dirs are (re-)ensured afterwards: state/ is NEVER copied from src,
    // sources/ is excluded (created empty), secrets/ is created EMPTY
    // (machine-local by design, ADR 0018).
    // A commitless source (git init'd but no commits) must fall back to the
    // file-copy path: cloning it yields an empty tree because config.toml is
    // uncommitted/untracked.
    let src_is_git_repo = src.join(".git").exists() && crate::git::git_has_head(&src);
    if src_is_git_repo {
        let url = src.to_string_lossy().to_string();
        // An existing dest passed the non-empty guard, so it is EMPTY here;
        // `git clone <src> <empty-dir>` succeeds (git only rejects a non-empty
        // target dir).
        crate::git::git_clone_full(&url, dest)?;
        // A clone of a repo that predates the hook still gets guarded.
        if !dest.join(".git").join("hooks").join("pre-commit").exists() {
            install_pre_commit_hook(dest)?;
        }
    } else {
        std::fs::create_dir_all(dest)?;
        std::fs::copy(&src_registry_path, dest.join("config.toml"))?;
        let src_overrides = src.join("overrides.toml");
        if src_overrides.exists() {
            std::fs::copy(&src_overrides, dest.join("overrides.toml"))?;
        }
        crate::git::git_init(dest)?;
        std::fs::write(dest.join(".gitignore"), CONFIG_GITIGNORE)?;
        install_pre_commit_hook(dest)?;
    }
    let ensured = ensure_layout_dirs(dest);

    // -- Step 5: FLEETS --------------------------------------------
    let mut cloned: Vec<String> = Vec::new();
    let mut copied: Vec<String> = Vec::new();
    let mut skipped_unreproducible: Vec<String> = Vec::new();
    let mut pin_warnings: Vec<String> = Vec::new();
    for (name, entry, kind) in &repro {
        let repo_dest = dest.join("fleets").join(name);
        let locked = src_lock.as_ref().and_then(|l| l.fleets.get(name));
        match kind {
            ReproKind::ReproducibleViaUrl => {
                // Pin selection order (spec 11 §2 step 5):
                //   a. LOCK rev wins — clone from the lock's recorded url when
                //      the lock entry exists and its url is usable (the lock
                //      pins what the source config actually ran; the registry
                //      url may have drifted), else the registry url.
                //   b. else the registry rev (current behavior).
                //   c. else leave at the registry ref tip (default main); warn
                //      when no pin exists (current behavior).
                let (pin_rev, pin_from_lock) = match locked.and_then(|lr| lr.rev.as_deref()) {
                    Some(r) => (Some(r), true),
                    None => (entry.rev.as_deref(), false),
                };
                let lock_url = locked
                    .map(|lr| lr.url.trim())
                    .filter(|u| !u.is_empty() && config::looks_like_git_url(u));
                let clone_url = match lock_url {
                    Some(u) => u.to_string(),
                    None => entry.url.clone(),
                };
                crate::git::git_clone_full(&clone_url, &repo_dest)?;
                match (pin_rev, &entry.r#ref) {
                    (Some(rev), _) => {
                        crate::git::git_checkout_rev(&repo_dest, rev)?;
                        let origin = if pin_from_lock { "locked" } else { "pinned" };
                        cloned.push(format!(
                            "{name} ({origin} rev {})",
                            crate::git::short_rev(rev)
                        ));
                    }
                    (None, None) => {
                        pin_warnings.push(format!(
                            "fleet '{name}': no rev or ref recorded in the registry; \
                             leaving the clone at the default-branch tip (NOT pinned)"
                        ));
                        cloned.push(name.clone());
                    }
                    (None, Some(_)) => cloned.push(name.clone()),
                }
            }
            ReproKind::LocalOnlyCopy => {
                // The registered url points into the src config's tree (e.g.
                // <src>/fleets/<name>); copy the working copy including
                // its .git, then wire origin to the src-side path.
                // NOTE: relative urls resolve against the SRC config here, not
                // the current config — `local_entry_checkout_dir`
                // (config-relative) does NOT apply to a cross-config clone.
                let url_path = config::expand_tilde(&entry.url);
                let url_path = if url_path.is_absolute() {
                    url_path
                } else {
                    src.join(&url_path)
                };
                if !url_path.exists() {
                    skipped_unreproducible.push(format!(
                        "{name} (registered url {} does not exist on disk)",
                        entry.url
                    ));
                    continue;
                }
                copy_dir_recursive(&url_path, &repo_dest)?;
                if repo_dest.join(".git").exists() {
                    let src_repo = url_path.to_string_lossy().to_string();
                    crate::git::git_remote_add_or_set_url(&repo_dest, "origin", &src_repo)?;
                }
                // A lock pin applies to copies too (spec 11 §2 step 5: the
                // copy carries full history when the source repo is
                // unshallowed). A lock that cannot be honored must NOT
                // silently produce a different rev → hard error naming the
                // repo and rev.
                if let Some(rev) = locked.and_then(|lr| lr.rev.as_deref()) {
                    crate::git::git_checkout_rev(&repo_dest, rev).map_err(|e| {
                        anyhow::anyhow!(
                            "fleet '{name}': cannot honor the locked rev {rev} \
                             in the copied working copy (the source repo may be shallow \
                             or the rev missing): {e:#}"
                        )
                    })?;
                    copied.push(format!(
                        "{name} (locked rev {})",
                        crate::git::short_rev(rev)
                    ));
                } else {
                    copied.push(name.clone());
                }
            }
            ReproKind::UnreproducibleOnRemoteSource => {
                skipped_unreproducible.push(format!("{name} (local-path url {})", entry.url));
            }
        }
    }
    for name in &skipped_unreproducible {
        eprintln!("WARNING: fleet '{name}' is unreproducible, clone manually");
    }
    for warning in &pin_warnings {
        eprintln!("WARNING: {warning}");
    }

    // -- Step 6: REWRITE REGISTRY URLS + stamp config_version ---------------
    // Load/mutate/save the DEST config.toml directly by path so both the
    // positional-dest and resolved-config forms behave identically.
    let mut urls_rewritten: Vec<String> = Vec::new();
    let dest_registry_path = dest.join("config.toml");
    let dest_content = std::fs::read_to_string(&dest_registry_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", dest_registry_path.display(), e))?;
    let mut dest_registry: Registry = toml::from_str(&dest_content)
        .map_err(|e| anyhow::anyhow!("failed to parse {}: {}", dest_registry_path.display(), e))?;
    let src_root = src.canonicalize().unwrap_or_else(|_| src.to_path_buf());
    for (name, entry) in dest_registry.fleets.iter_mut() {
        if config::looks_like_git_url(&entry.url) {
            continue;
        }
        let url_path = config::expand_tilde(&entry.url);
        let url_path = if url_path.is_absolute() {
            url_path
        } else {
            src.join(&url_path)
        };
        let url_canon = url_path.canonicalize().unwrap_or(url_path);
        if url_canon == src_root || url_canon.starts_with(&src_root) {
            entry.url = dest.join("fleets").join(name).to_string_lossy().to_string();
            urls_rewritten.push(name.clone());
        }
    }
    urls_rewritten.sort();
    dest_registry.settings.config_version = Some(MAX_SUPPORTED_CONFIG_VERSION);
    let serialized = toml::to_string_pretty(&dest_registry)
        .map_err(|e| anyhow::anyhow!("failed to serialize dest registry: {}", e))?;
    std::fs::write(&dest_registry_path, serialized)?;

    // -- Step 7: DEST CONFIG ORIGIN -----------------------------------------
    // The clone already wired origin (to the temp path for a remote src) —
    // repoint it at the ORIGINAL <src> value so pull-based sync tracks the
    // real source, not a soon-to-be-deleted temp dir. For a local src the
    // clone's origin IS the src path already; the set is idempotent. For a
    // non-git src the fresh repo has no origin; add one pointing at src.
    let origin_target = if src_was_remote {
        from.to_string()
    } else {
        src.to_string_lossy().to_string()
    };
    crate::git::git_remote_add_or_set_url(dest, "origin", &origin_target)?;
    println!(
        "hint: to allow push-back into the source config, run: \
         git -C {} config receive.denyCurrentBranch updateInstead",
        origin_target
    );

    // -- Step 8: WRITE workestrate.lock ------------------------------------
    // Fresh pins from the ACTUAL checked-out revs under dest (the dest
    // registry value parsed/mutated in step 6 is reused). The lock lands in
    // the dest config root and is committed to the config git repo (NOT gitignored).
    let lock = config::lock_from_registry(&dest_registry, dest);
    config::save_config_lock_to(dest, &lock)?;

    // -- Step 9: TRUSTED_PROJECTS (carried with the registry; warn LOUDLY) --
    for tp in &dest_registry.trusted_projects {
        eprintln!(
            "WARNING: trusted_projects entry '{}' is a machine-specific absolute path, \
             likely wrong on this machine — review with 'workestrate fleet untrust <dir>'",
            tp.path
        );
    }

    // -- Step 10: POST-FLIGHT (warn-only) + SUMMARY ------------------------
    // A config with skipped-unreproducible fleets may not validate; the warning
    // must not fail the provisioning.
    if let Err(e) = run_post_flight_validation() {
        eprintln!(
            "WARNING: post-flight validate-config failed (the provisioned config may need \
             manual fixes; skipped-unreproducible fleets are a common cause): {e:#}"
        );
    }

    print_provision_summary(ProvisionSummary {
        src: &src,
        src_was_remote,
        dest,
        cloned: &cloned,
        copied: &copied,
        skipped: &skipped_unreproducible,
        urls_rewritten: &urls_rewritten,
        origin: &origin_target,
        trusted_projects: dest_registry.trusted_projects.len(),
        ensured: &ensured,
    });
    Ok(())
}

/// RAII temp dir holding a full clone of a remote clone source. Removed
/// on drop (best-effort).
struct RemoteSrcClone {
    dir: PathBuf,
}

impl RemoteSrcClone {
    fn clone_from(url: &str) -> Result<Self> {
        let dir = std::env::temp_dir().join(format!(
            "workestrate-from-clone-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        crate::git::git_clone_full(url, &dir)?;
        Ok(Self { dir })
    }

    fn path(&self) -> &Path {
        &self.dir
    }
}

impl Drop for RemoteSrcClone {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Recursively copy a directory tree (`cp -r` semantics; dest must not
/// exist yet — it is created).
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    let meta = std::fs::symlink_metadata(src)?;
    if meta.is_dir() {
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            copy_dir_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        }
    } else if meta.file_type().is_symlink() {
        let target = std::fs::read_link(src)?;
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, dst)?;
        #[cfg(not(unix))]
        std::fs::copy(src, dst).map(|_| ())?;
    } else {
        std::fs::copy(src, dst)?;
    }
    Ok(())
}

/// Post-flight `validate-config` against the dest config. The dest config is the
/// process config at this point whenever a positional dest was given (env was
/// set early); when `config clone` has no positional dest, dest IS the resolved
/// config — so `WORKESTRATE_CONFIG` is pinned explicitly only when it is not
/// already pointing at dest. Returns the validation result for warn-only
/// handling by the caller.
fn run_post_flight_validation() -> Result<()> {
    let config = config::load_config()?;
    config::validate_config(&config)?;
    Ok(())
}

/// Inputs for the provisioning summary printer (keeps the arity of
/// `print_provision_summary` manageable).
struct ProvisionSummary<'a> {
    src: &'a Path,
    src_was_remote: bool,
    dest: &'a Path,
    cloned: &'a [String],
    copied: &'a [String],
    skipped: &'a [String],
    urls_rewritten: &'a [String],
    origin: &'a str,
    trusted_projects: usize,
    ensured: &'a [&'a str],
}

fn print_provision_summary(s: ProvisionSummary) {
    println!(
        "Provisioned workestrate config at {} from {}{}.",
        s.dest.display(),
        s.src.display(),
        if s.src_was_remote {
            " (via a full clone of the remote source)"
        } else {
            ""
        }
    );
    println!();
    println!("Layout:");
    println!("  state/    empty (never copied from the source config)");
    println!("  sources/  empty (re-derivable)");
    println!("  secrets/  empty (machine-local by design)");
    if !s.ensured.is_empty() {
        println!("  ensured:  {}", s.ensured.join(", "));
    }
    println!();
    println!("Fleets:");
    for name in s.cloned {
        println!("  cloned:   {name}");
    }
    for name in s.copied {
        println!("  copied:   {name} (local-only working copy, origin wired to the source repo)");
    }
    for name in s.skipped {
        println!("  SKIPPED:  {name} — unreproducible, clone manually");
    }
    if s.cloned.is_empty() && s.copied.is_empty() && s.skipped.is_empty() {
        println!("  (none registered in the source registry)");
    }
    if !s.urls_rewritten.is_empty() {
        println!();
        println!(
            "Registry urls rewritten to dest-local paths: {}",
            s.urls_rewritten.join(", ")
        );
    }
    println!();
    println!("Origin: dest config 'origin' -> {}", s.origin);
    if s.trusted_projects > 0 {
        println!(
            "trusted_projects: {} carried over (see the per-entry warnings above)",
            s.trusted_projects
        );
    }
    println!();
    println!("Next steps:");
    println!("  - run 'workestrate fleet list' to see the registered fleets");
    if !s.skipped.is_empty() {
        println!("  - clone each SKIPPED fleet manually, then re-register it");
    }
    println!("  - review trusted_projects entries (warnings above)");
    println!("  - run 'workestrate validate-config' to check the active config");
}

/// Write the pre-commit hook into `<config>/.git/hooks/pre-commit` and mark it
/// executable (unix: mode 0o755).
fn install_pre_commit_hook(config_dir: &Path) -> Result<()> {
    let hook = config_dir.join(".git").join("hooks").join("pre-commit");
    std::fs::write(&hook, CONFIG_PRE_COMMIT_HOOK)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

/// Human-readable summary in the cmd_new println style. Ends with the
/// dotfiles-remote step.
fn print_summary(config_dir: &Path, ensured: &[&str]) {
    println!(
        "Initialized workestrate config at {} as a git repo.",
        config_dir.display()
    );
    println!();
    println!("Installed:");
    println!("  .gitignore (store dirs + secret material untracked; *.enc committable)");
    println!("  .git/hooks/pre-commit (rejects gitlinks, store-dir paths, secret material)");
    println!(
        "  tombi.toml + schemas/workestrate.schema.json + schemas/workestrate-workload.schema.json + schemas/registry.schema.json (tombi TOML gates; hook runs them when tombi is present)"
    );
    if !ensured.is_empty() {
        println!();
        println!("Ensured dirs: {}", ensured.join(", "));
    }
    println!();
    println!("Next steps:");
    println!(
        "  - add a fleet: 'workestrate fleet new <name>' \
         (or 'workestrate fleet add <url> <name>')"
    );
    println!("  - run 'workestrate validate-config' to check the active config");
    println!(
        "  - add a remote for the dotfiles repo: \
         'git -C {} remote add origin <your-dotfiles-remote>' then push",
        config_dir.display()
    );
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;
    use crate::config::test_support::uniq_dir;

    #[test]
    fn non_empty_guard_flags_populated_dir() {
        let dir = uniq_dir("config-nonempty-guard");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("sentinel"), "x").unwrap();
        assert!(dir_exists_and_is_non_empty(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn non_empty_guard_allows_missing_or_empty_dir() {
        let missing = uniq_dir("config-nonempty-guard-missing");
        assert!(!dir_exists_and_is_non_empty(&missing));
        let empty = uniq_dir("config-nonempty-guard-empty");
        std::fs::create_dir_all(&empty).unwrap();
        assert!(!dir_exists_and_is_non_empty(&empty));
        let _ = std::fs::remove_dir_all(&empty);
    }

    #[test]
    fn copy_dir_recursive_copies_git_repo_trees() {
        let src = uniq_dir("config-copy-src");
        let dst = uniq_dir("config-copy-dst");
        std::fs::create_dir_all(src.join(".git").join("objects")).unwrap();
        std::fs::create_dir_all(src.join("nested")).unwrap();
        std::fs::write(src.join(".git").join("HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::write(src.join("nested").join("file.txt"), "content").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();
        assert_eq!(
            std::fs::read_to_string(dst.join(".git").join("HEAD")).unwrap(),
            "ref: refs/heads/main\n"
        );
        assert_eq!(
            std::fs::read_to_string(dst.join("nested").join("file.txt")).unwrap(),
            "content"
        );
        let _ = std::fs::remove_dir_all(&src);
        let _ = std::fs::remove_dir_all(&dst);
    }
}
