//! Tool-home command (`workestrate home …`): initialize the resolved tool
//! home as a dotfiles-style git repo (spec 10 Task 3, Decision B — the home
//! tracks `config.toml` + `overrides.toml`; store dirs and secret material
//! stay out of the index).

use std::path::Path;

use anyhow::Result;

use crate::cli_actions::HomeAction;
use crate::config;

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

pub fn cmd_home(action: HomeAction) -> Result<()> {
    match action {
        HomeAction::Init { config, name } => cmd_home_init(config.as_deref(), &name),
    }
}

/// `workestrate home init` — scaffold the RESOLVED tool home (no `--path`;
/// `WORKESTRATE_HOME` / legacy XDG / default resolution decides which home)
/// as a dotfiles-style git repo: `git init` + `.gitignore` + pre-commit hook.
/// Idempotent: re-running on an initialized home is a pure no-op.
pub fn cmd_home_init(config_url: Option<&str>, name: &str) -> Result<()> {
    // 1. Resolve the home via the standard precedence (env → legacy XDG →
    //    default). No path flag; resolution is entirely what
    //    resolve_home_with_kind already does.
    let (home, _kind) = config::resolve_home_with_kind();
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
    let mut ensured: Vec<&str> = Vec::new();
    for dir in ["config-repos", "sources", "state", "secrets"] {
        let path = home.join(dir);
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
            ensured.push(dir);
        }
    }

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

    // 8. Summary + next steps.
    print_summary(&home, config_url.is_some(), name, &ensured);
    Ok(())
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
