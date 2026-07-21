//! Scaffold templates for `workestrate config new`.
//!
//! Embedded at compile time via `include_str!` so the binary stays
//! self-contained (no runtime file lookup, no CWD dependence). The
//! `workestrate config new` command renders these templates into a
//! destination directory to bootstrap a new workestrator-config repo.
//!
//! Design constraints:
//! - Templates use `{{ var }}` placeholders only (no jinja conditionals,
//!   no `{% %}`, no `{# #}`). The [`render`] function is a bare
//!   `str::replace` pass plus an unsubstituted-token guard.
//! - The unsubstituted-token guard rejects any output that still contains
//!   `{{`, so adding a new placeholder without also adding it to the vars
//!   list fails loudly instead of leaking template syntax into the world.
//! - The minimal-personal render is byte-compatible with the copier
//!   template at `templates/workestrator-config/` for the overlap files
//!   (workestrate.toml, .sops.yaml, .env.example, .gitignore). The CI
//!   guard at `tests/scaffold_template.rs` enforces this parity when
//!   `copier` is available (HOST-NIX gate; SKIPs otherwise).

use anyhow::Result;

/// Canonical age key file path. Matches
/// `microsandbox::secrets_loader::default_age_key_file`
/// (`microsandbox/secrets_loader.rs:118`); single source of truth shared
/// with the scaffold templates so the README never drifts back to the
/// legacy `workestrate.txt` filename.
pub const AGE_KEY_DEFAULT_PATH: &str = "~/.config/sops/age/ai-workbench-secrets.txt";

/// Default copier source path written to `.copier-answers.yml` to enable
/// `copier update` interop. Pinned to the public GitHub repo.
pub const DEFAULT_COPIER_SRC_PATH: &str =
    "github:georgrybski/ai-workbench/templates/workestrator-config";

/// Default copier VCS ref written to `.copier-answers.yml`.
pub const DEFAULT_COPIER_VCS_REF: &str = "main";

/// Default core flake URL used by the optional `--with-flake` render.
pub const DEFAULT_CORE_FLAKE_URL: &str = "github:georgrybski/ai-workbench";

// Embedded templates. Paths are relative to this file.
const T_WORKESTRATE_TOML: &str = include_str!("template/workestrate.toml.tpl");
const T_SOPS_YAML: &str = include_str!("template/.sops.yaml.tpl");
const T_ENV_EXAMPLE: &str = include_str!("template/.env.example");
const T_GITIGNORE: &str = include_str!("template/.gitignore");
const T_README: &str = include_str!("template/README.md.tpl");
const T_COPIER_ANSWERS: &str = include_str!("template/.copier-answers.yml.tpl");
const T_FLAKE_NIX: &str = include_str!("template/flake.nix.tpl");

/// Variables needed to render the scaffold. Each field maps 1:1 to a
/// `{{ var }}` token in one or more templates.
#[derive(Debug, Clone)]
pub struct ScaffoldVars {
    /// `[a-z0-9-]` config repo name (validated upstream).
    pub config_name: String,
    /// Age recipient (`age1...`). Falls back to `age1PLACEHOLDER` when
    /// derivation fails.
    pub age_recipient: String,
    /// Copier `_src_path` for `.copier-answers.yml`. Defaults to
    /// [`DEFAULT_COPIER_SRC_PATH`].
    pub copier_src_path: String,
    /// Copier `_vcs_ref` for `.copier-answers.yml`. Defaults to
    /// [`DEFAULT_COPIER_VCS_REF`].
    pub copier_vcs_ref: String,
    /// When `Some`, render `flake.nix` with this core flake URL.
    /// When `None`, omit `flake.nix` from the output set.
    pub core_flake_url: Option<String>,
}

/// Render a single template by replacing each `("{{ key }}", value)` pair
/// in order, then asserting no `{{` remains in the output. The
/// unsubstituted-token guard turns "forgot to pass a var" into a loud
/// error instead of silent template-syntax leakage.
pub fn render(template: &str, vars: &[(&str, &str)]) -> Result<String> {
    let mut out = template.to_string();
    for (token, value) in vars {
        out = out.replace(token, value);
    }
    if let Some(idx) = out.find("{{") {
        // Show a short window around the first offending token to make
        // the error message actionable.
        let start = idx.saturating_sub(20);
        let end = (idx + 40).min(out.len());
        let window = &out[start..end];
        anyhow::bail!(
            "scaffold template has unsubstituted token near: ...{}...",
            window
        );
    }
    Ok(out)
}

/// Render all scaffold files for the given vars. Returns
/// `(relative_path, content)` pairs in deterministic order. When
/// `vars.core_flake_url` is `None`, the `flake.nix` entry is omitted.
pub fn render_all(vars: &ScaffoldVars) -> Result<Vec<(&'static str, String)>> {
    let config_name = vars.config_name.as_str();
    let age_recipient = vars.age_recipient.as_str();
    let copier_src_path = vars.copier_src_path.as_str();
    let copier_vcs_ref = vars.copier_vcs_ref.as_str();

    let toml_vars: &[(&str, &str)] = &[("{{ config_name }}", config_name)];
    let sops_vars: &[(&str, &str)] = &[
        ("{{ config_name }}", config_name),
        ("{{ age_recipient }}", age_recipient),
    ];
    let readme_vars: &[(&str, &str)] = &[("{{ config_name }}", config_name)];
    let copier_vars: &[(&str, &str)] = &[
        ("{{ copier_src_path }}", copier_src_path),
        ("{{ copier_vcs_ref }}", copier_vcs_ref),
        ("{{ config_name }}", config_name),
        ("{{ age_recipient }}", age_recipient),
    ];

    let mut out: Vec<(&'static str, String)> = Vec::with_capacity(8);
    out.push(("workestrate.toml", render(T_WORKESTRATE_TOML, toml_vars)?));
    out.push((".sops.yaml", render(T_SOPS_YAML, sops_vars)?));
    out.push((".env.example", render(T_ENV_EXAMPLE, &[])?));
    out.push((".gitignore", render(T_GITIGNORE, &[])?));
    out.push(("README.md", render(T_README, readme_vars)?));
    out.push((
        ".copier-answers.yml",
        render(T_COPIER_ANSWERS, copier_vars)?,
    ));
    if let Some(core_flake_url) = vars.core_flake_url.as_deref() {
        let flake_vars: &[(&str, &str)] = &[("{{ core_flake_url }}", core_flake_url)];
        out.push(("flake.nix", render(T_FLAKE_NIX, flake_vars)?));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use super::*;

    fn vars() -> ScaffoldVars {
        ScaffoldVars {
            config_name: "personal".into(),
            age_recipient: "age1TESTPLACEHOLDER".into(),
            copier_src_path: DEFAULT_COPIER_SRC_PATH.into(),
            copier_vcs_ref: DEFAULT_COPIER_VCS_REF.into(),
            core_flake_url: None,
        }
    }

    #[test]
    fn render_rejects_unsubstituted_tokens() {
        let res = render("hello {{ name }}!", &[]);
        assert!(res.is_err());
        let err = res.unwrap_err().to_string();
        assert!(err.contains("unsubstituted"), "got: {err}");
    }

    #[test]
    fn render_passes_through_when_no_tokens() {
        let res = render("no tokens here", &[]).expect("render");
        assert_eq!(res, "no tokens here");
    }

    #[test]
    fn render_all_includes_required_files_no_flake() {
        let out = render_all(&vars()).expect("render_all");
        let names: Vec<_> = out.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"workestrate.toml"));
        assert!(names.contains(&".sops.yaml"));
        assert!(names.contains(&".env.example"));
        assert!(names.contains(&".gitignore"));
        assert!(names.contains(&"README.md"));
        assert!(names.contains(&".copier-answers.yml"));
        assert!(
            !names.contains(&"flake.nix"),
            "flake.nix should be omitted when core_flake_url is None"
        );
    }

    #[test]
    fn render_all_includes_flake_when_url_provided() {
        let mut v = vars();
        v.core_flake_url = Some(DEFAULT_CORE_FLAKE_URL.into());
        let out = render_all(&v).expect("render_all");
        let names: Vec<_> = out.iter().map(|(n, _)| *n).collect();
        assert!(names.contains(&"flake.nix"));
    }

    #[test]
    fn render_all_substitutes_config_name() {
        let out = render_all(&vars()).expect("render_all");
        let toml_entry = out
            .iter()
            .find(|(n, _)| *n == "workestrate.toml")
            .map(|(_, c)| c.as_str())
            .expect("workestrate.toml");
        assert!(
            toml_entry.contains("workestrator config: personal"),
            "config_name not substituted"
        );
        assert!(
            !toml_entry.contains("{{"),
            "unsubstituted token in workestrate.toml"
        );
    }

    #[test]
    fn render_all_substitutes_age_recipient_in_sops() {
        let out = render_all(&vars()).expect("render_all");
        let sops = out
            .iter()
            .find(|(n, _)| *n == ".sops.yaml")
            .map(|(_, c)| c.as_str())
            .expect(".sops.yaml");
        assert!(
            sops.contains("age1TESTPLACEHOLDER"),
            "age_recipient not substituted"
        );
        assert!(!sops.contains("{{"), "unsubstituted token in .sops.yaml");
    }

    #[test]
    fn render_all_copier_answers_has_personal_only_team_key() {
        let out = render_all(&vars()).expect("render_all");
        let answers = out
            .iter()
            .find(|(n, _)| *n == ".copier-answers.yml")
            .map(|(_, c)| c.as_str())
            .expect(".copier-answers.yml");
        assert!(
            answers.contains("team_age_recipient: \"\""),
            "team_age_recipient must be empty for personal-only render"
        );
        assert!(
            !answers.contains("{{"),
            "unsubstituted token in copier answers"
        );
    }

    #[test]
    fn age_key_default_path_matches_secrets_loader_canonical() {
        // Drift guard: the README embeds AGE_KEY_DEFAULT_PATH; if the
        // secrets_loader canonical path ever changes, this test fails and
        // forces a coordinated update. See microsandbox/secrets_loader.rs:118.
        assert_eq!(
            AGE_KEY_DEFAULT_PATH,
            "~/.config/sops/age/ai-workbench-secrets.txt"
        );
    }

    #[test]
    fn readme_uses_canonical_age_key_path_not_legacy_workestrate_txt() {
        let mut v = vars();
        v.config_name = "personal".into();
        let out = render_all(&v).expect("render_all");
        let readme = out
            .iter()
            .find(|(n, _)| *n == "README.md")
            .map(|(_, c)| c.as_str())
            .expect("README.md");
        assert!(
            readme.contains("ai-workbench-secrets.txt"),
            "README must reference canonical age key path"
        );
        assert!(
            !readme.contains("workestrate.txt"),
            "README must NOT reference the legacy workestrate.txt path (live drift bug)"
        );
    }
}
