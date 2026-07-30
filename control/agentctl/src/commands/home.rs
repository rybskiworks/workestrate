//! Tool-home command (`workestrate home …`): initialize the resolved tool
//! home as a dotfiles-style git repo (spec 10 Task 3, Decision B — the home
//! tracks `config.toml` + `overrides.toml`; store dirs and secret material
//! stay out of the index), and provision a NEW home from an existing one
//! (`home init --from <src> [dest]`, ADR 0025).

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::cli_actions::HomeAction;
use crate::config::{self, ConfigRepoEntry, Registry};

/// `.gitignore` written into the home repo. The store dirs and plaintext
/// secret material stay untracked; age ciphertext (`*.enc`) stays committable.
const HOME_GITIGNORE: &str = "\
# workestrate home — installed by `workestrate home init`.
/config-repos/
/sources/
/state/
/cache/
*.agekey
age.txt
*.pem
id_rsa*
.env
";

/// Pre-commit hook installed into the home repo: rejects embedded git repos
/// (gitlinks, mode 160000), store-dir paths, and secret material. Kept as a
/// const so tests can assert on the canonical content.
const HOME_PRE_COMMIT_HOOK: &str = r#"#!/bin/sh
# Installed by `workestrate home init` — guards the workestrate home repo.
# Rejects embedded git repos (gitlinks, mode 160000), store-dir paths, and
# secret material. Encrypted secrets (*.enc) stay committable.
fail=0
gitlinks=$(git ls-files -s | awk '$1 == "160000" { print $4 }')
if [ -n "$gitlinks" ]; then
    echo "home pre-commit: refusing embedded git repos (gitlinks, mode 160000):" >&2
    echo "$gitlinks" >&2
    fail=1
fi
staged=$(git diff --cached --name-only --diff-filter=ACM)
if printf '%s\n' "$staged" | grep -qE '^(config-repos|sources|state)/'; then
    echo "home pre-commit: refusing store-dir paths (config-repos/ sources/ state/):" >&2
    printf '%s\n' "$staged" | grep -E '^(config-repos|sources|state)/' >&2
    fail=1
fi
if printf '%s\n' "$staged" | grep -qE '(^|/)([^/]*\.agekey|age\.txt|[^/]*\.pem|id_rsa[^/]*|\.env)$'; then
    echo "home pre-commit: refusing secret material (*.agekey, age.txt, *.pem, id_rsa*, .env):" >&2
    printf '%s\n' "$staged" | grep -E '(^|/)([^/]*\.agekey|age\.txt|[^/]*\.pem|id_rsa[^/]*|\.env)$' >&2
    fail=1
fi
[ "$fail" -eq 0 ] || exit 1
exit 0
"#;

/// Highest `settings.home_version` this binary can provision FROM (ADR 0025
/// fail-before-write compat check). Absent/1/2 are accepted; anything newer
/// is a hard error ("home created by a newer workestrate").
const MAX_SUPPORTED_HOME_VERSION: u32 = 2;

pub fn cmd_home(action: HomeAction) -> Result<()> {
    match action {
        HomeAction::Init {
            config,
            name,
            from,
            dest,
        } => cmd_home_init(config.as_deref(), &name, from.as_deref(), dest.as_deref()),
    }
}

/// `workestrate home init [--from <src>] [<dest>]`.
///
/// Bare path (no `--from`, no positional dest): scaffold the RESOLVED tool
/// home (no `--path`; `WORKESTRATE_HOME` / legacy XDG / default resolution
/// decides which home) as a dotfiles-style git repo. Idempotent: re-running
/// on an initialized home is a pure no-op.
///
/// Positional dest: scaffold a NEW home at dest — a thin wrapper that sets
/// `WORKESTRATE_HOME=<dest>` and runs the same bare flow.
///
/// `--from <src>`: provision the dest home from an existing home (ADR 0025;
/// see [`provision_home_from`]).
pub fn cmd_home_init(
    config_url: Option<&str>,
    name: &str,
    from: Option<&str>,
    dest: Option<&str>,
) -> Result<()> {
    let bare = from.is_none() && dest.is_none();

    // Resolve the dest home: positional dest (`~/` expanded; relative paths
    // resolve against cwd at use time) or the standard resolution.
    let home = match dest {
        Some(d) => config::expand_tilde(d),
        None => config::resolve_home_with_kind().0,
    };

    // DEST NON-EMPTY GUARD (spec §1: "dest must not exist or be empty — bail
    // if non-empty"). Only on the non-bare paths: the bare path keeps its
    // idempotent behavior toward an existing (populated) home.
    if !bare && dir_exists_and_is_non_empty(&home) {
        anyhow::bail!(
            "destination '{}' already exists and is not empty; \
             'home init --from' / a positional dest requires a missing or empty directory",
            home.display()
        );
    }

    // When a positional dest is given, pin the process home to it EARLY so
    // every env-based helper (config_repo_dir, save_registry, cmd_config_add,
    // the post-flight validate) operates on dest.
    if dest.is_some() {
        std::env::set_var("WORKESTRATE_HOME", home.to_string_lossy().as_ref());
    }

    if let Some(src) = from {
        return provision_home_from(src, &home);
    }

    // ---- Bare path (unchanged behavior) ----

    // 1. Resolve the home via the standard precedence (env → legacy XDG →
    //    default). No path flag; resolution is entirely what
    //    resolve_home_with_kind already does.
    std::fs::create_dir_all(&home)?;

    // 2. Idempotency: an existing .git means the home is already a repo.
    //    Pure no-op — do NOT rewrite .gitignore/hook, do NOT run the
    //    --config flow.
    if home.join(".git").exists() {
        println!(
            "Tool home at {} is already initialized as a git repo (nothing to do).",
            home.display()
        );
        if config_url.is_some() {
            println!(
                "note: --config was ignored on the no-op path; run \
                 'workestrate config add <url> {name}' instead."
            );
        }
        return Ok(());
    }

    // 3. Ensure the home layout dirs exist (adds only what is missing;
    //    idempotent over a populated home per spec §2 "Home init" item 3).
    let ensured = ensure_layout_dirs(&home);

    // 4. git init — the git layer is the entire point of this command, so
    //    propagate failures (unlike cmd_config_new's warn-and-continue).
    crate::git::git_init(&home)?;

    // 5. .gitignore: store dirs + secret material untracked; age ciphertext
    //    (*.enc) stays committable.
    std::fs::write(home.join(".gitignore"), HOME_GITIGNORE)?;

    // 6. Pre-commit hook (executable on unix).
    install_pre_commit_hook(&home)?;

    // 7. Optional --config: clone + register via the existing machinery.
    //    Validate the name first so a bad name fails before the clone.
    if let Some(url) = config_url {
        config::validate_config_name(name)?;
        crate::commands::config_cmd::cmd_config_add(url, name, "main")?;
    }

    // 8. ADR 0025(e): write the generated lock. This runs AFTER a successful
    //    init only — never on the idempotent no-op path above (which leaves
    //    the home, including its lock, untouched). The registry may not exist
    //    yet when --config wasn't passed (cmd_config_add already wrote the
    //    lock when it was); a bare init with no config repos writes a lock
    //    with an empty `repos` map.
    let registry = config::load_registry()?.unwrap_or_default();
    let lock = config::lock_from_registry(&registry, &home);
    config::save_home_lock_to(&home, &lock)?;

    // 9. Summary + next steps.
    print_summary(&home, config_url.is_some(), name, &ensured);
    Ok(())
}

/// True when `dir` exists and contains at least one entry.
fn dir_exists_and_is_non_empty(dir: &Path) -> bool {
    dir.is_dir()
        && std::fs::read_dir(dir)
            .map(|mut it| it.next().is_some())
            .unwrap_or(false)
}

/// Create the home layout dirs (`config-repos/`, `sources/`, `state/`,
/// `secrets/`), returning the names that were newly created.
fn ensure_layout_dirs(home: &Path) -> Vec<&'static str> {
    let mut ensured: Vec<&str> = Vec::new();
    for dir in ["config-repos", "sources", "state", "secrets"] {
        let path = home.join(dir);
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

/// How one `[configs.<name>]` entry can be reproduced on this machine
/// (ADR 0025 §2 step 2 reproducibility report).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReproKind {
    /// The registered url is a git remote — re-clone it.
    ReproducibleViaUrl,
    /// The registered url is a local path and the SOURCE home is local —
    /// copy the working copy out of the source tree.
    LocalOnlyCopy,
    /// The registered url is a local path but the SOURCE came from a git
    /// URL — the working copy is unreachable from here; skip (loudly).
    UnreproducibleOnRemoteSource,
}

/// The `--from <src>` provisioning path (ADR 0025 §2 steps 1–10).
fn provision_home_from(from: &str, dest: &Path) -> Result<()> {
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
            std::env::current_dir()?.join(expanded)
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
            "provisioning source '{}' has no config.toml (not a workestrate home)",
            src.display()
        );
    }
    let content = std::fs::read_to_string(&src_registry_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", src_registry_path.display(), e))?;
    let src_registry: Registry = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("failed to parse {}: {}", src_registry_path.display(), e))?;
    if let Some(v) = src_registry.settings.home_version {
        if v > MAX_SUPPORTED_HOME_VERSION {
            anyhow::bail!(
                "home created by a newer workestrate (settings.home_version = {} > {}); \
                 upgrade this workestrate before provisioning from {}",
                v,
                MAX_SUPPORTED_HOME_VERSION,
                src.display()
            );
        }
    }
    // Source lock (spec 11 §2 step 2: read src workestrate.lock if present,
    // same version rules). Absent file → legacy path (Ok(None)); absent
    // `version` field → oldest; newer-than-supported → the hard "home created
    // by a newer workestrate" error. All BEFORE any writes (zero residue).
    let src_lock = config::load_home_lock_from(&src)?;

    // Reproducibility report (per config repo entry).
    let mut repro: Vec<(String, ConfigRepoEntry, ReproKind)> = Vec::new();
    for (name, entry) in &src_registry.configs {
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
    // A git source home is full-cloned into dest (origin comes for free; the
    // clone carries config.toml, overrides.toml, .gitignore, the hook, and
    // any committed *.enc). A non-git source gets config.toml
    // (+ overrides.toml) copied and the standard scaffolding. The layout
    // dirs are (re-)ensured afterwards: state/ is NEVER copied from src,
    // sources/ is excluded (created empty), secrets/ is created EMPTY
    // (machine-local by design, ADR 0018).
    let src_is_git_repo = src.join(".git").exists();
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
        std::fs::write(dest.join(".gitignore"), HOME_GITIGNORE)?;
        install_pre_commit_hook(dest)?;
    }
    let ensured = ensure_layout_dirs(dest);

    // -- Step 5: CONFIG REPOS --------------------------------------------
    let mut cloned: Vec<String> = Vec::new();
    let mut copied: Vec<String> = Vec::new();
    let mut skipped_unreproducible: Vec<String> = Vec::new();
    let mut pin_warnings: Vec<String> = Vec::new();
    for (name, entry, kind) in &repro {
        let repo_dest = dest.join("config-repos").join(name);
        let locked = src_lock.as_ref().and_then(|l| l.repos.get(name));
        match kind {
            ReproKind::ReproducibleViaUrl => {
                // Pin selection order (spec 11 §2 step 5):
                //   a. LOCK rev wins — clone from the lock's recorded url when
                //      the lock entry exists and its url is usable (the lock
                //      pins what the source home actually ran; the registry
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
                            "config repo '{name}': no rev or ref recorded in the registry; \
                             leaving the clone at the default-branch tip (NOT pinned)"
                        ));
                        cloned.push(name.clone());
                    }
                    (None, Some(_)) => cloned.push(name.clone()),
                }
            }
            ReproKind::LocalOnlyCopy => {
                // The registered url points into the src home's tree (e.g.
                // <src>/config-repos/<name>); copy the working copy including
                // its .git, then wire origin to the src-side path.
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
                            "config repo '{name}': cannot honor the locked rev {rev} \
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
        eprintln!("WARNING: config repo '{name}' is unreproducible, clone manually");
    }
    for warning in &pin_warnings {
        eprintln!("WARNING: {warning}");
    }

    // -- Step 6: REWRITE REGISTRY URLS + stamp home_version ---------------
    // Load/mutate/save the DEST config.toml directly by path so both the
    // positional-dest and resolved-home forms behave identically.
    let mut urls_rewritten: Vec<String> = Vec::new();
    let dest_registry_path = dest.join("config.toml");
    let dest_content = std::fs::read_to_string(&dest_registry_path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {}", dest_registry_path.display(), e))?;
    let mut dest_registry: Registry = toml::from_str(&dest_content)
        .map_err(|e| anyhow::anyhow!("failed to parse {}: {}", dest_registry_path.display(), e))?;
    let src_root = src.canonicalize().unwrap_or_else(|_| src.to_path_buf());
    for (name, entry) in dest_registry.configs.iter_mut() {
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
            entry.url = dest
                .join("config-repos")
                .join(name)
                .to_string_lossy()
                .to_string();
            urls_rewritten.push(name.clone());
        }
    }
    urls_rewritten.sort();
    dest_registry.settings.home_version = Some(MAX_SUPPORTED_HOME_VERSION);
    let serialized = toml::to_string_pretty(&dest_registry)
        .map_err(|e| anyhow::anyhow!("failed to serialize dest registry: {}", e))?;
    std::fs::write(&dest_registry_path, serialized)?;

    // -- Step 7: DEST HOME ORIGIN -----------------------------------------
    // The clone already wired origin (to the temp path for a remote src) —
    // repoint it at the ORIGINAL --from value so pull-based sync tracks the
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
        "hint: to allow push-back into the source home, run: \
         git -C {} config receive.denyCurrentBranch updateInstead",
        origin_target
    );

    // -- Step 8: WRITE workestrate.lock ------------------------------------
    // Fresh pins from the ACTUAL checked-out revs under dest (the dest
    // registry value parsed/mutated in step 6 is reused). The lock lands in
    // the dest home root and is committed to the home repo (NOT gitignored).
    let lock = config::lock_from_registry(&dest_registry, dest);
    config::save_home_lock_to(dest, &lock)?;

    // -- Step 9: TRUSTED_PROJECTS (carried with the registry; warn LOUDLY) --
    for tp in &dest_registry.trusted_projects {
        eprintln!(
            "WARNING: trusted_projects entry '{}' is a machine-specific absolute path, \
             likely wrong on this machine — review with 'workestrate config untrust <dir>'",
            tp.path
        );
    }

    // -- Step 10: POST-FLIGHT (warn-only) + SUMMARY ------------------------
    // A home with skipped-unreproducible repos may not validate; the warning
    // must not fail the provisioning.
    if let Err(e) = run_post_flight_validation() {
        eprintln!(
            "WARNING: post-flight validate-config failed (the provisioned home may need \
             manual fixes; skipped-unreproducible config repos are a common cause): {e:#}"
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

/// RAII temp dir holding a full clone of a remote `--from` source. Removed
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

/// Post-flight `validate-config` against the dest home. The dest home is the
/// process home at this point whenever a positional dest was given (env was
/// set early); when `--from` has no positional dest, dest IS the resolved
/// home — so `WORKESTRATE_HOME` is pinned explicitly only when it is not
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
        "Provisioned workestrate home at {} from {}{}.",
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
    println!("  state/    empty (never copied from the source home)");
    println!("  sources/  empty (re-derivable)");
    println!("  secrets/  empty (machine-local by design)");
    if !s.ensured.is_empty() {
        println!("  ensured:  {}", s.ensured.join(", "));
    }
    println!();
    println!("Config repos:");
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
    println!("Origin: dest home 'origin' -> {}", s.origin);
    if s.trusted_projects > 0 {
        println!(
            "trusted_projects: {} carried over (see the per-entry warnings above)",
            s.trusted_projects
        );
    }
    println!();
    println!("Next steps:");
    println!("  - run 'workestrate config list' to see the registered repos");
    if !s.skipped.is_empty() {
        println!("  - clone each SKIPPED config repo manually, then re-register it");
    }
    println!("  - review trusted_projects entries (warnings above)");
    println!("  - run 'workestrate validate-config' to check the active config");
}

/// Write the pre-commit hook into `<home>/.git/hooks/pre-commit` and mark it
/// executable (unix: mode 0o755).
fn install_pre_commit_hook(home: &Path) -> Result<()> {
    let hook = home.join(".git").join("hooks").join("pre-commit");
    std::fs::write(&hook, HOME_PRE_COMMIT_HOOK)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

/// Human-readable summary in the cmd_init/cmd_new println style. Two forms:
/// with/without --config; both end with the dotfiles-remote step.
fn print_summary(home: &Path, with_config: bool, name: &str, ensured: &[&str]) {
    println!(
        "Initialized workestrate tool home at {} as a git repo.",
        home.display()
    );
    println!();
    println!("Installed:");
    println!("  .gitignore (store dirs + secret material untracked; *.enc committable)");
    println!("  .git/hooks/pre-commit (rejects gitlinks, store-dir paths, secret material)");
    if !ensured.is_empty() {
        println!();
        println!("Ensured dirs: {}", ensured.join(", "));
    }
    if with_config {
        println!();
        println!("Cloned and registered config repo '{name}' in config-repos/{name}.");
    }
    println!();
    println!("Next steps:");
    if with_config {
        println!("  - run 'workestrate config list' to see the registered repo");
        println!("  - run 'workestrate validate-config' to check the active config");
    } else {
        println!(
            "  - add a config repo: 'workestrate config new <name>' \
             (or 'workestrate home init --config <url>')"
        );
    }
    println!(
        "  - add a remote for the dotfiles repo: \
         'git -C {} remote add origin <your-dotfiles-remote>' then push",
        home.display()
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
        let dir = uniq_dir("home-nonempty-guard");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("sentinel"), "x").unwrap();
        assert!(dir_exists_and_is_non_empty(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn non_empty_guard_allows_missing_or_empty_dir() {
        let missing = uniq_dir("home-nonempty-guard-missing");
        assert!(!dir_exists_and_is_non_empty(&missing));
        let empty = uniq_dir("home-nonempty-guard-empty");
        std::fs::create_dir_all(&empty).unwrap();
        assert!(!dir_exists_and_is_non_empty(&empty));
        let _ = std::fs::remove_dir_all(&empty);
    }

    #[test]
    fn copy_dir_recursive_copies_git_repo_trees() {
        let src = uniq_dir("home-copy-src");
        let dst = uniq_dir("home-copy-dst");
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
