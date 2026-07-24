//! Config-repo and context commands (`workestrate config …`,
//! `workestrate context …`).

use std::path::PathBuf;

use anyhow::Result;

use crate::cli_actions::{ConfigAction, ContextAction};
use crate::commands::init::find_reference_workestrate;
use crate::commands::secrets_target::derive_age_recipient;
use crate::config;
use crate::git::{git_clone, git_init, git_is_dirty, git_pull, git_rev_parse, short_rev};
use crate::scaffold;

pub async fn cmd_config(action: ConfigAction) -> Result<()> {
    match action {
        ConfigAction::Add { url, name, r#ref } => cmd_config_add(&url, &name, &r#ref),
        ConfigAction::Update { name } => cmd_config_update(name.as_deref()).await,
        ConfigAction::List => cmd_config_list().await,
        ConfigAction::Trust { dir } => cmd_config_trust(&dir),
        ConfigAction::Untrust { dir } => cmd_config_untrust(&dir),
        ConfigAction::Remove {
            name,
            delete,
            force,
        } => cmd_config_remove(&name, delete, force),
        // `New` is dispatched directly in `async_main` so it can route the
        // global `--json` flag. Reaching this arm would be a regression.
        ConfigAction::New { .. } => {
            anyhow::bail!("config new must be dispatched from async_main (--json routing)")
        }
    }
}

/// `workestrate config remove <name>` — unregister a config repo. With
/// `--delete`, also remove the store clone at `<store>/repos/<name>`; refuses
/// to delete a dirty clone unless `--force` is passed. The name is also
/// scrubbed from the bare `layers` list and every context's `layers`. A
/// dangling `settings.default_context` pointing at the removed name is
/// cleared (with a warning).
pub fn cmd_config_remove(name: &str, delete: bool, force: bool) -> Result<()> {
    config::validate_config_name(name)?;
    let mut registry = config::load_registry()?
        .ok_or_else(|| anyhow::anyhow!("no registry found; run 'workestrate init' first"))?;
    if !registry.configs.contains_key(name) {
        anyhow::bail!("config repo '{}' is not registered", name);
    }

    if delete {
        let dest = config::config_repo_dir(name);
        if dest.exists() {
            if git_is_dirty(&dest)? && !force {
                anyhow::bail!(
                    "config repo '{}' has uncommitted changes; use --force to delete anyway",
                    name
                );
            }
            std::fs::remove_dir_all(&dest)?;
            println!("Deleted store clone: {}", dest.display());
        } else {
            println!("(store clone already absent)");
        }
    }

    registry.configs.remove(name);
    registry.layers.retain(|l| l != name);
    for ctx in registry.contexts.values_mut() {
        ctx.layers.retain(|l| l != name);
    }
    if registry.settings.default_context.as_deref() == Some(name) {
        registry.settings.default_context = None;
        eprintln!(
            "warning: cleared default_context '{}' (it pointed at the removed repo)",
            name
        );
    }
    config::save_registry(&registry)?;
    println!("Unregistered config repo: {}", name);
    Ok(())
}

/// `workestrate context list|current` — inspect defined contexts and the
/// currently-resolved active context.
pub async fn cmd_context(action: ContextAction, json: bool) -> Result<()> {
    match action {
        ContextAction::List => cmd_context_list(json),
        ContextAction::Current => cmd_context_current(json),
    }
}

pub fn cmd_context_list(json: bool) -> Result<()> {
    let registry = match config::load_registry()? {
        Some(r) => r,
        None => {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "contexts": [],
                    }))?
                );
            } else {
                println!("(no registry found)");
            }
            return Ok(());
        }
    };

    let mut names: Vec<&String> = registry.contexts.keys().collect();
    names.sort();

    if json {
        let contexts: Vec<serde_json::Value> = names
            .iter()
            .map(|name| {
                let ctx = &registry.contexts[*name];
                let is_default = registry.settings.default_context.as_ref() == Some(*name);
                serde_json::json!({
                    "name": name,
                    "layers": ctx.layers,
                    "is_default": is_default,
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({ "contexts": contexts }))?
        );
    } else {
        println!("Contexts:");
        for name in names {
            let ctx = &registry.contexts[name];
            let is_default = registry.settings.default_context.as_ref() == Some(name);
            if is_default {
                println!("  {} (default)", name);
            } else {
                println!("  {}", name);
            }
            println!("    layers: {}", ctx.layers.join(", "));
        }
    }
    Ok(())
}

pub fn cmd_context_current(json: bool) -> Result<()> {
    let active = config::resolve_active_context()?;

    // Resolution-source classification: env (--context flag / WORKESTRATE_CONTEXT)
    // wins; otherwise the registry default_context; otherwise bare-layers
    // backward-compat when no contexts are defined at all.
    let source = if std::env::var("WORKESTRATE_CONTEXT").is_ok() {
        "env"
    } else if let Ok(Some(ref reg)) = config::load_registry() {
        if reg.settings.default_context.is_some() {
            "default"
        } else if reg.contexts.is_empty() {
            "bare"
        } else {
            "default"
        }
    } else {
        "bare"
    };

    if json {
        let body = serde_json::json!({
            "name": active.name,
            "source": source,
            "layers": active.layers,
        });
        println!("{}", serde_json::to_string_pretty(&body)?);
    } else {
        match active.name {
            Some(ref name) => println!("Active context: {}", name),
            None => println!("Active context: (none — using bare layers)"),
        }
        let source_label = match source {
            "env" => "env (--context flag or WORKESTRATE_CONTEXT)",
            other => other,
        };
        println!("  source: {}", source_label);
        println!("  layers: [{}]", active.layers.join(", "));
    }
    Ok(())
}

pub fn cmd_config_add(url: &str, name: &str, git_ref: &str) -> Result<()> {
    let dest = config::config_repo_dir(name);
    let (rev, short) = if dest.exists() {
        let git_dir = dest.join(".git");
        if !git_dir.exists() {
            anyhow::bail!(
                "config repo destination '{}' already exists and is not a git repo",
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

    // Insert/replace the registry entry. Delegates to config::register_config
    // (shared with cmd_config_new's local-path registration).
    config::register_config(name, url, Some(git_ref), Some(rev.as_str()))?;
    println!(
        "Registered config repo {} from {} at {} (rev {})",
        name,
        url,
        dest.display(),
        short
    );
    Ok(())
}

/// `--json` output envelope for `workestrate config new`. Mirrors the
/// shape of other JSON-emitting commands (registry, ps): a single
/// pretty-printed object on stdout, errors on stderr.
#[derive(serde::Serialize)]
pub struct ConfigNewResult<'a> {
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
pub async fn cmd_config_new(
    name: &str,
    path: Option<&std::path::Path>,
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
    config::validate_config_name(name)?;

    // Fail-fast: if we'd auto-register, check the registry now so we don't
    // write files + git init only to bail at the registration step.
    if !no_register {
        if let Some(reg) = config::load_registry()? {
            if reg.configs.contains_key(name) {
                anyhow::bail!(
                    "config repo '{}' already registered; use a different name or \
                     'workestrate config update {}'",
                    name,
                    name
                );
            }
        }
    }

    // Resolve destination directory. Default is the managed store
    // (<store>/repos/<name>) so the repo is immediately active for layer
    // resolution once registered. An explicit --path overrides this.
    let store_path = config::config_repo_dir(name);
    let dest: std::path::PathBuf = match path {
        Some(p) => p.to_path_buf(),
        None => store_path.clone(),
    };
    if dest.exists() {
        // Refuse if non-empty. An empty existing directory is OK (init in
        // a pre-created dir); a populated one likely means we'd clobber.
        let is_non_empty = std::fs::read_dir(&dest)
            .map(|mut it| it.next().is_some())
            .unwrap_or(false);
        if is_non_empty {
            anyhow::bail!(
                "destination '{}' exists and is non-empty; refusing to overwrite \
                 (remove it or pass a different --path)",
                dest.display()
            );
        }
    }

    // Warn when an explicit --path places the repo outside the managed
    // store. The repo won't be active for layer resolution until moved into
    // the store or re-added via `config add` after pushing to a remote.
    if !no_register && dest != store_path {
        eprintln!(
            "warning: '{}' is outside the config store ('{}'); \
             the repo won't be active for layer resolution until moved into \
             the store or re-added via `workestrate config add` after pushing \
             to a remote.",
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
        // Minimal: just workestrate.toml (hand-written, no secrets) + .gitignore.
        vec![
            (
                "workestrate.toml".to_string(),
                format!(
                    "#:schema https://raw.githubusercontent.com/georgrybski/ai-workbench/main/schemas/workestrate.schema.json\n\
                     \n\
                     schema_version = 1\n\
                     \n\
                     # workestrator config: {name}\n\
                     # Generated by `workestrate config new --empty`.\n\
                     # Add [secrets.*] and [workloads.*] tables here.\n"
                ),
            ),
            (
                ".gitignore".to_string(),
                scaffold::render(include_str!("../scaffold/template/.gitignore"), &[])?,
            ),
        ]
    } else {
        scaffold::render_all(&vars)?
            .into_iter()
            .map(|(n, c)| (n.to_string(), c))
            .collect()
    };

    // --from-reference overrides the workestrate.toml entry with the full
    // 5-workload reference fixture.
    if from_reference {
        let cwd = std::env::current_dir()?;
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

    // Register in workestrate registry.
    let mut registered = false;
    if !no_register {
        // The already-registered check ran above (fail-fast, before any
        // filesystem writes).
        let canonical = dest.canonicalize()?;
        let url = canonical.to_string_lossy().to_string();
        // FS-25: canonicalize() silently rewrites a user-supplied --path
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
        // Local-path repos get ref=None, rev=None. cmd_config_update
        // recognizes this and skips the pull step.
        config::register_config(name, &url, None, None)?;
        registered = true;
    }

    // Build next-steps.
    let mut next_steps: Vec<&str> = Vec::with_capacity(4);
    if recipient_source == "placeholder" {
        next_steps.push("edit .sops.yaml and replace age1PLACEHOLDER with your real age1... key");
    }
    next_steps.push("cd into the new directory and edit workestrate.toml");
    if !no_register {
        next_steps.push(
            "to enable `workestrate config update`, push to a remote and edit the registry's url + ref",
        );
    }
    next_steps.push("run `workestrate validate-config` from inside the new repo");

    if json_mode {
        let result = ConfigNewResult {
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
    println!(
        "Created workestrator-config '{}' in {}",
        name,
        dest.display()
    );
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
    }
    if registered {
        println!();
        println!(
            "Registered as a local-path config repo (no remote yet; `workestrate config update` will skip)."
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

pub async fn cmd_config_update(name: Option<&str>) -> Result<()> {
    let mut registry =
        config::load_registry()?.ok_or_else(|| anyhow::anyhow!("no config repos registered"))?;
    let names: Vec<String> = match name {
        Some(n) => {
            if !registry.configs.contains_key(n) {
                anyhow::bail!("config repo '{}' not found", n);
            }
            vec![n.to_string()]
        }
        None => registry.configs.keys().cloned().collect(),
    };

    for n in names {
        let dest = config::config_repo_dir(&n);
        // FS-18: distinguish LOCAL-PATH entries (registered via
        // `config new` — url is a filesystem path, ref=None/rev=None) from
        // GIT-URL entries. Only local-path entries skip the pull. A git-URL
        // entry whose rev is unrecorded (e.g. hand-edited registry, or a
        // clone whose rev was never written back) is NOT "local" — it falls
        // through and is pulled, which also re-records its rev.
        let entry_ref = registry.configs.get(&n);
        let is_local_path = entry_ref.is_some_and(config::entry_is_local_path);
        if is_local_path {
            println!(
                "config repo '{}' is a local path (no pinned rev); skipping update.\n\
                 To enable updates, push to a remote and edit the registry entry's url + ref.",
                n
            );
            continue;
        }
        if git_is_dirty(&dest)? {
            anyhow::bail!(
                "config repo '{}' has uncommitted changes; commit or stash first",
                n
            );
        }
        let git_ref = entry_ref
            .and_then(|e| e.r#ref.as_deref())
            .unwrap_or("main")
            .to_string();
        git_pull(&dest, &git_ref)?;
        let rev = git_rev_parse(&dest)?;
        let short = short_rev(&rev);
        registry
            .configs
            .get_mut(&n)
            .ok_or_else(|| anyhow::anyhow!("config repo '{}' disappeared", n))?
            .rev = Some(rev);
        println!("{}: updated to {}", n, short);
    }
    config::save_registry(&registry)?;
    Ok(())
}

pub fn cmd_config_list_json() -> Result<()> {
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

pub async fn cmd_config_list() -> Result<()> {
    match config::load_registry()? {
        None => {
            println!(
                "(no registry found; run 'workestrate init' or 'workestrate config add <url> <name>')"
            );
        }
        Some(registry) => {
            println!("Config repos:");
            if registry.configs.is_empty() {
                println!("  (none)");
            } else {
                for (name, entry) in &registry.configs {
                    let dest = config::config_repo_dir(name);
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
            println!("Layers: {:?}", registry.layers);
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

pub fn cmd_config_trust(dir: &str) -> Result<()> {
    let path = PathBuf::from(dir);
    config::trust_project(&path)?;
    println!("Trusted: {}", path.display());
    Ok(())
}

pub fn cmd_config_untrust(dir: &str) -> Result<()> {
    let path = PathBuf::from(dir);
    config::untrust_project(&path)?;
    println!("Untrusted: {}", path.display());
    Ok(())
}
