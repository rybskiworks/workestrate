//! Fleet commands (`workestrate fleet …`).

use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::cli_actions::FleetAction;
use crate::commands::init::find_reference_workestrate;
use crate::commands::secrets_target::derive_age_recipient;
use crate::config;
use crate::git::{git_clone, git_init, git_is_dirty, git_pull, git_rev_parse, short_rev};
use crate::scaffold;

/// Pre-commit hook installed into new fleets: TOML format + lint +
/// schema validation via tombi. Warns-and-skips (exit 0) when tombi is not
/// on PATH — hooks run on arbitrary machines; fails (exit 1) on tombi
/// check failures. Kept as a const so tests can assert on canonical content.
const FLEET_PRE_COMMIT_HOOK: &str = r#"#!/bin/sh
# workestrate fleet pre-commit: TOML format + lint + schema validation.
TOMBI_REQUIRED="1.2.5"
if ! command -v tombi >/dev/null 2>&1; then
    echo "pre-commit: tombi not found (need $TOMBI_REQUIRED); skipping tombi checks" >&2
    exit 0
fi
tombi_version="$(tombi --version | awk '{print $2}')"
if [ "$tombi_version" != "$TOMBI_REQUIRED" ]; then echo "pre-commit: tombi version mismatch (found '${tombi_version:-unknown}', want $TOMBI_REQUIRED); skipping tombi checks" >&2; else tombi format --check || exit 1; tombi lint --error-on-warnings || exit 1; fi
"#;

/// Write the pre-commit hook into `<dest>/.git/hooks/pre-commit` and mark it
/// executable (unix: mode 0o755). Only called when git init succeeded.
fn install_fleet_pre_commit_hook(dest: &Path) -> Result<()> {
    let hook = dest.join(".git").join("hooks").join("pre-commit");
    std::fs::write(&hook, FLEET_PRE_COMMIT_HOOK)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(())
}

pub async fn cmd_fleet(action: FleetAction) -> Result<()> {
    match action {
        FleetAction::Add { url, name, r#ref } => cmd_fleet_add(&url, &name, &r#ref),
        FleetAction::Update { name } => cmd_fleet_update(name.as_deref()).await,
        FleetAction::List => cmd_fleet_list().await,
        FleetAction::Trust { dir } => cmd_fleet_trust(&dir),
        FleetAction::Untrust { dir } => cmd_fleet_untrust(&dir),
        FleetAction::Remove {
            name,
            delete,
            force,
        } => cmd_fleet_remove(&name, delete, force),
        // `New` is dispatched directly in `async_main` so it can route the
        // global `--json` flag. Reaching this arm would be a regression.
        FleetAction::New { .. } => {
            anyhow::bail!("fleet new must be dispatched from async_main (--json routing)")
        }
    }
}

/// `workestrate fleet remove <name>` — unregister a fleet. With
/// `--delete`, also remove the store clone at `<store>/fleets/<name>`;
/// refuses
/// to delete a dirty clone unless `--force` is passed. The name is also
/// scrubbed from the bare `layers` list. A dangling `settings.default_fleet`
/// pointing at the removed name is cleared (with a warning).
pub fn cmd_fleet_remove(name: &str, delete: bool, force: bool) -> Result<()> {
    config::validate_fleet_name(name)?;
    let mut registry = config::load_registry()?
        .ok_or_else(|| anyhow::anyhow!("no registry found; run 'workestrate config init' first"))?;
    if !registry.fleets.contains_key(name) {
        anyhow::bail!("fleet '{}' is not registered", name);
    }

    if delete {
        let dest = config::fleet_dir(name);
        if dest.exists() {
            if git_is_dirty(&dest)? && !force {
                anyhow::bail!(
                    "fleet '{}' has uncommitted changes; use --force to delete anyway",
                    name
                );
            }
            std::fs::remove_dir_all(&dest)?;
            println!("Deleted store clone: {}", dest.display());
        } else {
            println!("(store clone already absent)");
        }
    }

    registry.fleets.remove(name);
    registry.layers.retain(|l| l != name);
    if registry.settings.default_fleet.as_deref() == Some(name) {
        registry.settings.default_fleet = None;
        eprintln!(
            "warning: cleared default_fleet '{}' (it pointed at the removed fleet)",
            name
        );
    }
    config::save_registry(&registry)?;

    // ADR 0025(e): drop the lock entry for this fleet. Only mutate an
    // EXISTING lock — never create one in a config that never had one (legacy
    // configs stay lock-free until a writer that pins fleets runs).
    if let Some(mut lock) = config::load_config_lock()? {
        lock.fleets.remove(name);
        lock.tool_version = env!("CARGO_PKG_VERSION").to_string();
        config::save_config_lock(&lock)?;
    }

    println!("Unregistered fleet: {}", name);
    Ok(())
}

pub fn cmd_fleet_add(url: &str, name: &str, git_ref: &str) -> Result<()> {
    let dest = config::fleet_dir(name);
    let (rev, short) = if dest.exists() {
        let git_dir = dest.join(".git");
        if !git_dir.exists() {
            anyhow::bail!(
                "fleet destination '{}' already exists and is not a git repo",
                dest.display()
            );
        }
        let rev = git_rev_parse(&dest)?;
        let short = short_rev(&rev);
        (rev, short)
    } else {
        let parent = dest
            .parent()
            .ok_or_else(|| anyhow::anyhow!("invalid repo path: {}", dest.display()))?;
        std::fs::create_dir_all(parent)?;
        git_clone(url, &dest, Some(git_ref))?;
        let rev = git_rev_parse(&dest)?;
        let short = short_rev(&rev);
        (rev, short)
    };

    // Insert/replace the registry entry. Delegates to config::register_fleet
    // (shared with cmd_fleet_new's local-path registration).
    config::register_fleet(name, url, Some(git_ref), Some(rev.as_str()))?;

    // A5 Session 2: `fleet add` is an explicit management verb — produce
    // the initial content archive AND the initial lock pin (same write path
    // as `fleet update`, minus the pull). Consumption reads the archive of
    // the locked rev from here on.
    config::ensure_archive(&dest, &rev)?;

    // ADR 0025(e): upsert the lock entry for this fleet. Load-or-default so a
    // config that predates the lock gains one on the first add; config_version
    // comes from the registry (default 2 — the ADR 0023 single-config layout).
    let config_version = config::load_registry()?
        .and_then(|r| r.settings.config_version)
        .unwrap_or(2);
    let mut lock = config::load_config_lock()?.unwrap_or_else(|| config::ConfigLock {
        version: config::LOCK_VERSION,
        config_version,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        fleets: std::collections::BTreeMap::new(),
    });
    lock.version = config::LOCK_VERSION;
    lock.config_version = config_version;
    lock.tool_version = env!("CARGO_PKG_VERSION").to_string();
    config::upsert_locked_pin(&mut lock, name, url, Some(git_ref), &rev);
    config::save_config_lock(&lock)?;

    println!(
        "Registered fleet {} from {} at {} (rev {})",
        name,
        url,
        dest.display(),
        short
    );
    Ok(())
}

/// `--json` output envelope for `workestrate fleet new`. Mirrors the
/// shape of other JSON-emitting commands (registry, ps): a single
/// pretty-printed object on stdout, errors on stderr.
#[derive(serde::Serialize)]
pub struct FleetNewResult<'a> {
    name: &'a str,
    path: std::path::PathBuf,
    files_written: Vec<String>,
    git_initialized: bool,
    registered: bool,
    /// "flag" | "derived" | "placeholder"
    age_recipient_source: &'a str,
    age_recipient: &'a str,
    next_steps: Vec<&'a str>,
}

#[allow(clippy::too_many_arguments)]
pub async fn cmd_fleet_new(
    name: &str,
    dest: Option<&str>,
    age_recipient: Option<&str>,
    age_key_file: Option<&std::path::Path>,
    with_flake: bool,
    core_flake_url: &str,
    no_register: bool,
    no_git_init: bool,
    from_reference: bool,
    empty: bool,
    json_mode: bool,
) -> Result<()> {
    // Validate name first so an invalid name fails before touching the
    // filesystem.
    config::validate_fleet_name(name)?;

    // Fail-fast: if we'd auto-register, check the registry now so we don't
    // write files + git init only to bail at the registration step.
    if !no_register
        && let Some(reg) = config::load_registry()?
        && reg.fleets.contains_key(name)
    {
        anyhow::bail!(
            "fleet '{}' already registered; use a different name or \
                     'workestrate fleet update {}'",
            name,
            name
        );
    }

    // Resolve destination directory. Default is the managed store
    // (<store>/fleets/<name>) so the repo is immediately active for
    // layer resolution once registered. An explicit positional dest
    // overrides this.
    let store_path = config::fleet_dir(name);
    let dest: std::path::PathBuf = dest
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| store_path.clone());
    if dest.exists() {
        // Refuse if non-empty. An empty existing directory is OK (init in
        // a pre-created dir); a populated one likely means we'd clobber.
        let is_non_empty = std::fs::read_dir(&dest)
            .map(|mut it| it.next().is_some())
            .unwrap_or(false);
        if is_non_empty {
            anyhow::bail!(
                "destination '{}' exists and is non-empty; refusing to overwrite \
                 (remove it or pass a different dest)",
                dest.display()
            );
        }
    }

    // Outside the managed store the repo is scaffolded but NOT registered:
    // it can't be active for layer resolution anyway. Print guidance instead.
    if !no_register && dest != store_path {
        eprintln!(
            "note: '{}' is outside the config store ('{}'); scaffolded but \
             NOT registered. To activate: push to a remote and run \
             `workestrate fleet add <url> <name>`, or re-run without a dest \
             to create it in the store.",
            dest.display(),
            store_path.display()
        );
    }

    // Resolve age recipient: explicit flag → derived → placeholder.
    let (recipient, recipient_source) = match age_recipient {
        Some(r) => (r.to_string(), "flag"),
        None => {
            let key_file = age_key_file
                .map(|p| config::expand_tilde(&p.to_string_lossy()))
                .unwrap_or_else(|| config::expand_tilde(scaffold::AGE_KEY_DEFAULT_PATH));
            match derive_age_recipient(&key_file) {
                Ok(r) => (r, "derived"),
                Err(e) => {
                    eprintln!(
                        "warning: could not derive age recipient ({}); \
                         wrote 'age1PLACEHOLDER'.\n\
                         Set the real key with: edit .sops.yaml, OR re-run with \
                         --age-recipient <key>.\n\
                         Generate a key with: age-keygen -o {}",
                        e,
                        scaffold::AGE_KEY_DEFAULT_PATH
                    );
                    ("age1PLACEHOLDER".to_string(), "placeholder")
                }
            }
        }
    };

    // Build the file set from the scaffold templates.
    let vars = scaffold::ScaffoldVars {
        config_name: name.to_string(),
        age_recipient: recipient.clone(),
        copier_src_path: scaffold::DEFAULT_COPIER_SRC_PATH.to_string(),
        copier_vcs_ref: scaffold::DEFAULT_COPIER_VCS_REF.to_string(),
        core_flake_url: if with_flake {
            Some(core_flake_url.to_string())
        } else {
            None
        },
    };

    let mut files: Vec<(String, String)> = if empty {
        // Minimal: just workestrate.toml (hand-written, no secrets) +
        // .gitignore + the tombi toolchain files. The workestrate.toml
        // header references `#:schema ./schemas/workestrate.schema.json`
        // and the pre-commit hook runs tombi, so the schema and tombi.toml
        // must be emitted too — same consts as the full render, via the
        // shared helper (no template duplication).
        let mut files: Vec<(String, String)> = vec![
            (
                "workestrate.toml".to_string(),
                format!(
                    "#:schema ./schemas/workestrate.schema.json\n\
                     \n\
                     schema_version = 1\n\
                     \n\
                     # workestrate fleet: {name}\n\
                     # Generated by `workestrate fleet new --empty`.\n\
                     # Add [secrets.*] and [workloads.*] tables here.\n"
                ),
            ),
            (
                ".gitignore".to_string(),
                scaffold::render(include_str!("../scaffold/template/.gitignore"), &[])?,
            ),
        ];
        files.extend(
            scaffold::tombi_files()?
                .into_iter()
                .map(|(n, c)| (n.to_string(), c)),
        );
        files
    } else {
        scaffold::render_all(&vars)?
            .into_iter()
            .map(|(n, c)| (n.to_string(), c))
            .collect()
    };

    // --from-reference overrides the workestrate.toml entry with the full
    // 5-workload reference fixture.
    if from_reference {
        let cwd = crate::config::invoke_cwd_or_err()?;
        let ref_path = find_reference_workestrate(&cwd)?;
        let ref_content = std::fs::read_to_string(&ref_path)?;
        let entry = files
            .iter_mut()
            .find(|(n, _)| n == "workestrate.toml")
            .ok_or_else(|| {
                anyhow::anyhow!("internal: workestrate.toml missing from scaffold file set")
            })?;
        entry.1 = ref_content;
    }

    // Create destination + write each file.
    std::fs::create_dir_all(&dest)?;
    for (rel, content) in &files {
        let full = dest.join(rel);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&full, content)?;
    }

    // git init (hard-error on non-zero status; warn-and-continue if binary missing).
    let mut git_initialized = false;
    if !no_git_init {
        match git_init(&dest) {
            Ok(()) => git_initialized = true,
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("git binary not found") {
                    eprintln!(
                        "warning: git not installed; skipping `git init` in '{}'. \
                         Re-run with --no-git-init to silence this.",
                        dest.display()
                    );
                } else {
                    return Err(e);
                }
            }
        }
    }

    // Install the tombi pre-commit hook when git init succeeded (skipped on
    // --no-git-init or when the git binary is missing).
    if git_initialized {
        install_fleet_pre_commit_hook(&dest)?;
    }

    // Register in workestrate registry — only when the repo lives at the
    // in-store default (out-of-store dests are scaffolded but NOT registered;
    // the guidance note was printed above).
    let mut registered = false;
    if !no_register && dest == store_path {
        // The already-registered check ran above (fail-fast, before any
        // filesystem writes).
        let canonical = dest.canonicalize()?;
        let url = canonical.to_string_lossy().to_string();
        // FS-25: canonicalize() silently rewrites a user-supplied dest
        // (symlink resolution, `.`/`..` collapse, case) into a DIFFERENT
        // registered url. Surface that rewrite instead of registering the
        // silent rewrite — the operator should know the registry records
        // the canonical form, not their literal input.
        if canonical != dest {
            eprintln!(
                "note: registered url uses the canonicalized path '{}' (resolved from literal '{}')",
                canonical.display(),
                dest.display()
            );
        }
        // Local-path fleets get ref=None, rev=None. cmd_fleet_update
        // recognizes this and skips the pull step.
        config::register_fleet(name, &url, None, None)?;
        registered = true;
    }

    // Build next-steps.
    let mut next_steps: Vec<&str> = Vec::with_capacity(4);
    if recipient_source == "placeholder" {
        next_steps.push("edit .sops.yaml and replace age1PLACEHOLDER with your real age1... key");
    }
    next_steps.push("cd into the new directory and edit workestrate.toml");
    if registered {
        next_steps.push(
            "to enable `workestrate fleet update`, push to a remote and edit the registry's url + ref",
        );
    }
    next_steps.push("run `workestrate validate-config` from inside the new repo");

    if json_mode {
        let result = FleetNewResult {
            name,
            path: dest.clone(),
            files_written: files.iter().map(|(n, _)| n.clone()).collect(),
            git_initialized,
            registered,
            age_recipient_source: recipient_source,
            age_recipient: &recipient,
            next_steps,
        };
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    // Human-readable summary.
    println!("Created fleet '{}' in {}", name, dest.display());
    println!();
    println!("Files written:");
    for (rel, _) in &files {
        println!("  {}", rel);
    }
    if git_initialized {
        println!();
        println!(
            "git initialized (no initial commit; `git add . && git commit -m init` when ready)"
        );
        println!(
            "  .git/hooks/pre-commit installed (tombi format + lint checks; skips when tombi absent)"
        );
    }
    if registered {
        println!();
        println!(
            "Registered as a local-path fleet (no remote yet; `workestrate fleet update` will skip)."
        );
    }
    println!();
    println!(
        "Age recipient: {} (source: {})",
        recipient, recipient_source
    );
    println!();
    println!("Next steps:");
    for step in next_steps {
        println!("  - {}", step);
    }
    Ok(())
}

pub async fn cmd_fleet_update(name: Option<&str>) -> Result<()> {
    let mut registry =
        config::load_registry()?.ok_or_else(|| anyhow::anyhow!("no fleets registered"))?;
    let names: Vec<String> = match name {
        Some(n) => {
            if !registry.fleets.contains_key(n) {
                anyhow::bail!("fleet '{}' not found", n);
            }
            vec![n.to_string()]
        }
        None => registry.fleets.keys().cloned().collect(),
    };

    // A5 Session 2: track the entries actually updated this run — each gets
    // an archive refresh + an explicit per-entry lock pin below.
    let mut updated: Vec<(String, String)> = Vec::new();
    for n in names {
        let dest = config::fleet_dir(&n);
        // FS-18: distinguish LOCAL-PATH entries (registered via
        // `fleet new` — url is a filesystem path, ref=None/rev=None) from
        // GIT-URL entries. Only local-path entries skip the pull. A git-URL
        // entry whose rev is unrecorded (e.g. hand-edited registry, or a
        // clone whose rev was never written back) is NOT "local" — it falls
        // through and is pulled, which also re-records its rev.
        let entry_ref = registry.fleets.get(&n);
        let is_local_path = entry_ref.is_some_and(config::entry_is_local_path);
        if is_local_path {
            println!(
                "fleet '{}' is a local path (no pinned rev); skipping update.\n\
                 To enable updates, push to a remote and edit the registry entry's url + ref.",
                n
            );
            continue;
        }
        if git_is_dirty(&dest)? {
            anyhow::bail!(
                "fleet '{}' has uncommitted changes; commit or stash first",
                n
            );
        }
        // A5 review LOW-3: pull the SAME effective ref consumption resolves
        // (config::effective_ref — explicit `ref` > origin/HEAD of the
        // managed clone > checkout branch), not a hardcoded "main": a
        // ref-less entry on a non-main default branch must pull the ref its
        // consumers pinned. A ref-less entry whose default ref cannot be
        // resolved fails closed here, naming the fleet + remediation.
        let entry = entry_ref.ok_or_else(|| anyhow::anyhow!("fleet '{}' disappeared", n))?;
        let git_ref = config::effective_ref(&n, entry, &dest)?;
        git_pull(&dest, &git_ref)?;
        let rev = git_rev_parse(&dest)?;
        let short = short_rev(&rev);
        registry
            .fleets
            .get_mut(&n)
            .ok_or_else(|| anyhow::anyhow!("fleet '{}' disappeared", n))?
            .rev = Some(rev.clone());
        updated.push((n.clone(), rev));
        println!("{}: updated to {}", n, short);
    }
    config::save_registry(&registry)?;

    // ADR 0025(e) + A5 Session 2: `fleet update` is THE explicit
    // lock-writer and archive refresh. The lock is updated IN PLACE (a
    // `lock_from_registry` rebuild would clobber the v2 per-entry
    // sha/fetched_at/refs back to empty — see its fn doc); the rebuild only
    // bootstraps a lock for a config that never had one. For each pulled
    // entry: refresh the content archive of the new rev, then write the pin
    // {rev = sha, sha, fetched_at = now} (the registry.rev write-back above
    // stays for compat — the registry shape is unchanged). Entries not
    // covered by this run keep their existing pins untouched.
    let store = config::resolve_store_dir();
    let mut lock = config::load_config_lock()?
        .unwrap_or_else(|| config::lock_from_registry(&registry, &store));
    lock.version = config::LOCK_VERSION;
    lock.config_version = registry.settings.config_version.unwrap_or(2);
    lock.tool_version = env!("CARGO_PKG_VERSION").to_string();
    for (n, rev) in &updated {
        let dest = config::fleet_dir(n);
        config::ensure_archive(&dest, rev)?;
        let entry = registry
            .fleets
            .get(n)
            .ok_or_else(|| anyhow::anyhow!("fleet '{}' disappeared", n))?;
        config::upsert_locked_pin(&mut lock, n, &entry.url, entry.r#ref.as_deref(), rev);
    }
    config::save_config_lock(&lock)?;
    Ok(())
}

pub fn cmd_fleet_list_json() -> Result<()> {
    // The committed JSON shape is a stable object mapping layer info; this is
    // a best-effort serialization of the in-memory Registry (or null when no
    // registry exists).
    match config::load_registry()? {
        None => {
            println!("null");
            Ok(())
        }
        Some(registry) => {
            println!("{}", serde_json::to_string_pretty(&registry)?);
            Ok(())
        }
    }
}

pub async fn cmd_fleet_list() -> Result<()> {
    match config::load_registry()? {
        None => {
            println!(
                "(no registry found; run 'workestrate config init' or 'workestrate fleet add <url> <name>')"
            );
        }
        Some(registry) => {
            println!("Fleets:");
            if registry.fleets.is_empty() {
                println!("  (none)");
            } else {
                for (name, entry) in &registry.fleets {
                    let dest = config::fleet_dir(name);
                    let (dirty_label, ok) = if dest.exists() {
                        match git_is_dirty(&dest) {
                            Ok(false) => ("clean", true),
                            Ok(true) => ("dirty", false),
                            Err(_) => ("unknown", false),
                        }
                    } else {
                        ("missing", false)
                    };
                    let rev = entry.rev.as_deref().unwrap_or("unknown");
                    let short = short_rev(rev);
                    let git_ref = entry.r#ref.as_deref().unwrap_or("main");
                    let status = if ok { "[OK]" } else { "[MISSING]" };
                    println!(
                        "  {}: {} (ref {}, rev {}, {}) {}",
                        name, entry.url, git_ref, short, dirty_label, status
                    );
                }
            }
            match registry.settings.default_fleet.as_deref() {
                Some(name) => println!("Default fleet: {}", name),
                None => println!("Default fleet: (none)"),
            }
            match config::resolve_active_fleet() {
                Ok(active) => match active.name {
                    Some(name) => println!("Active fleet: {}", name),
                    None => println!(
                        "Active fleet: (none — using bare layers [{}])",
                        active.layers.join(", ")
                    ),
                },
                Err(e) => println!("Active fleet: [ERROR] {}", e),
            }
            println!("Trusted projects:");
            if registry.trusted_projects.is_empty() {
                println!("  (none)");
            } else {
                for p in &registry.trusted_projects {
                    let path = PathBuf::from(&p.path);
                    let status = if path.exists() { "[OK]" } else { "[MISSING]" };
                    println!("  {} {}", p.path, status);
                }
            }
        }
    }
    Ok(())
}

pub fn cmd_fleet_trust(dir: &str) -> Result<()> {
    let path = PathBuf::from(dir);
    config::trust_project(&path)?;
    println!("Trusted: {}", path.display());
    Ok(())
}

pub fn cmd_fleet_untrust(dir: &str) -> Result<()> {
    let path = PathBuf::from(dir);
    config::untrust_project(&path)?;
    println!("Untrusted: {}", path.display());
    Ok(())
}
