//! Config validation: schema-version enforcement, recipe/feature vocabulary,
//! policy allowlists, trust-boundary validators, and identifier validators.

use anyhow::Result;

use crate::config::types::ConfigFile;
use crate::policy;
use crate::recipes::EgressRecipeRef;

// ---------------------------------------------------------------------------
// workestrate.toml schema
// ---------------------------------------------------------------------------

/// The schema_version this build supports (WP6(f)/E2).
///
/// `ConfigFile.schema_version` is `#[serde(default)]`, so a MISSING version
/// parses as 0 — `validate_config` treats 0 as "absent/legacy" and accepts it
/// as 1 with a stderr warning (backward compat). An explicit
/// `schema_version = 0` is a degenerate case that falls into the same
/// warn+accept bucket. Any other value (>= 2) is a hard error.
pub const EXPECTED_SCHEMA_VERSION: u32 = 1;

/// Validate a config repo name for `workestrate config new`. Same safe-set
/// as workload names: names flow into both filesystem paths (the registry
/// store dir) and registry TOML keys, so the intersection `[a-z0-9-]` is
/// the only safe charset.
///
/// Pattern: `^[a-z0-9][a-z0-9-]{0,62}$` — starts with an alphanumeric, allows
/// lowercase letters / digits / hyphens, max 63 characters (DNS-label length).
/// Rejects empty / overlong / uppercase / underscores / dots / slashes / shell
/// metacharacters / leading hyphen.
///
/// "Escape nothing — reject instead" is the policy. This is a thin wrapper
/// around [`validate_identifier`] shared with [`validate_workload_name`] in
/// main.rs (mirrored here to keep config-domain logic in config.rs without a
/// cross-module dependency from main.rs's validator).
pub fn validate_config_name(name: &str) -> Result<()> {
    validate_identifier(name, "config name")
}

/// Shared identifier validator. `label` is interpolated into error messages
/// ("workload name ...", "config name ..."). Pattern:
/// `^[a-z0-9][a-z0-9-]{0,62}$`.
fn validate_identifier(name: &str, label: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("{label} cannot be empty");
    }
    if name.len() > 63 {
        anyhow::bail!(
            "{label} cannot exceed 63 characters (got {}): '{}'",
            name.len(),
            name
        );
    }
    let mut chars = name.chars();
    let first_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    if !first_ok {
        anyhow::bail!(
            "{label} must start with [a-z0-9]; \
             pattern: ^[a-z0-9][a-z0-9-]{{0,62}}$; got: '{name}'"
        );
    }
    for c in chars {
        if !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-' {
            anyhow::bail!(
                "{label} contains invalid character '{}' (allowed: [a-z0-9-]); \
                 pattern: ^[a-z0-9][a-z0-9-]{{0,62}}$; got: '{}'",
                c,
                name
            );
        }
    }
    Ok(())
}

/// Allowed `workloads.{wl}.image.recipe` values (WP6(d)/C9). Mirrors the
/// image recipes in `nix/lib/recipes.nix`.
const ALLOWED_IMAGE_RECIPES: &[&str] = &["registry", "nix-layered"];

/// Allowed build recipe values for `workloads.{wl}.image.binary.recipe` and
/// `workloads.{wl}.local_build.recipe` (WP6(d)/C9). Mirrors the build recipes
/// in `nix/lib/recipes.nix`.
const ALLOWED_BUILD_RECIPES: &[&str] = &["npm-build", "bun-compile", "pip-install", "bun-install"];

/// Allowed `workloads.{wl}.image.features` entries (WP6(d)/C9). Mirrors the
/// features vocabulary in `nix/lib/vocabulary.nix` (same style as
/// `policy::ALLOWED_PACKAGES`, which lives in policy.rs outside this WP's
/// owned file set).
const ALLOWED_FEATURES: &[&str] = &["create_tmp"];

/// Whether `name` is a syntactically valid environment-variable name:
/// `^[A-Za-z_][A-Za-z0-9_]*$` (WP10/A11).
///
/// Mirrors `is_valid_env_name` in `microsandbox/secrets_loader.rs` (deliberately
/// NOT imported — that module is outside this task's owned set, and the helper
/// is three lines). Anything that becomes a real env var (workload `env` entry
/// names, `secrets.{name}.env_var` values) must match, or the sandbox would be
/// handed a name no shell or `exec` can set.
fn is_valid_env_var_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(
        chars.next(),
        Some(c) if c.is_ascii_alphabetic() || c == '_'
    ) && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Validate a parsed [`ConfigFile`] against the invariants the TOML schema
/// alone cannot express: `schema_version` support (see
/// [`EXPECTED_SCHEMA_VERSION`]), recipe/feature vocabulary allowlists, trust,
/// and mount/env/secret rules. Returns the first violation found as a hard
/// error.
pub fn validate_config(config: &ConfigFile) -> Result<()> {
    // WP6(f)/E2: schema_version enforcement. `ConfigFile.schema_version` is
    // #[serde(default)], so a MISSING version parses as 0 — treated as
    // "absent/legacy": accepted as EXPECTED_SCHEMA_VERSION with a stderr
    // warning (backward compat). An explicit `schema_version = 0` is a
    // degenerate case that falls into the same warn+accept bucket. Any other
    // unsupported value (>= 2) is a hard error.
    match config.schema_version {
        0 => {
            eprintln!(
                "WARNING: schema_version missing; assuming {} (backward compat)",
                EXPECTED_SCHEMA_VERSION
            );
        }
        v if v == EXPECTED_SCHEMA_VERSION => {}
        other => {
            anyhow::bail!(
                "schema_version {} is not supported (expected {}). This workestrate build supports schema_version {} only.",
                other,
                EXPECTED_SCHEMA_VERSION,
                EXPECTED_SCHEMA_VERSION
            );
        }
    }

    // WP6(d)/C9: recipe/feature vocabulary hard errors. The string-valued
    // recipes are plain strings in the schema (unlike egress, whose tagged
    // enum already fails unknown variants at TOML parse), so they must be
    // validated here.
    for (workload_name, workload) in &config.workloads {
        if !workload.image.recipe.is_empty()
            && !ALLOWED_IMAGE_RECIPES.contains(&workload.image.recipe.as_str())
        {
            anyhow::bail!(
                "workload '{}' image.recipe '{}' is not a known recipe (expected one of: {})",
                workload_name,
                workload.image.recipe,
                ALLOWED_IMAGE_RECIPES.join(", ")
            );
        }
        if let Some(ref binary) = workload.image.binary {
            if !ALLOWED_BUILD_RECIPES.contains(&binary.recipe.as_str()) {
                anyhow::bail!(
                    "workload '{}' image.binary.recipe '{}' is not a known recipe (expected one of: {})",
                    workload_name,
                    binary.recipe,
                    ALLOWED_BUILD_RECIPES.join(", ")
                );
            }
        }
        if let Some(ref build) = workload.local_build {
            if !ALLOWED_BUILD_RECIPES.contains(&build.recipe.as_str()) {
                anyhow::bail!(
                    "workload '{}' local_build.recipe '{}' is not a known recipe (expected one of: {})",
                    workload_name,
                    build.recipe,
                    ALLOWED_BUILD_RECIPES.join(", ")
                );
            }
        }
        if let Some(ref features) = workload.image.features {
            for feature in features {
                if !ALLOWED_FEATURES.contains(&feature.as_str()) {
                    anyhow::bail!(
                        "workload '{}' image.features '{}' is not in the allowed feature set (expected one of: {})",
                        workload_name,
                        feature,
                        ALLOWED_FEATURES.join(", ")
                    );
                }
            }
        }
        // Defensive: egress recipe names are already constrained to the known
        // EgressRecipeRef variants by TOML deserialization (serde tagged
        // enum); this match documents the closed set.
        for egress in &workload.network.egress {
            match egress {
                EgressRecipeRef::Dns
                | EgressRecipeRef::LitellmProxy
                | EgressRecipeRef::Github
                | EgressRecipeRef::AgentBase
                | EgressRecipeRef::Https { .. } => {}
            }
        }
    }

    // Egress hosts in https recipes must be in ALLOWED_EGRESS_HOSTS.
    for (workload_name, workload) in &config.workloads {
        for egress in &workload.network.egress {
            if let EgressRecipeRef::Https { hosts } = egress {
                for host in hosts {
                    if !policy::ALLOWED_EGRESS_HOSTS.contains(&host.as_str()) {
                        anyhow::bail!(
                            "workload '{}' egress host '{}' is not in the core egress allowlist",
                            workload_name,
                            host
                        );
                    }
                }
            }
        }
    }

    // Secret bindings must match SECRET_HOST_BINDINGS.
    for (secret_name, secret) in &config.secrets {
        if let Some(ref env_var) = secret.env_var {
            let allowed_hosts = policy::SECRET_HOST_BINDINGS
                .iter()
                .find(|(key, _)| key == env_var)
                .map(|(_, hosts)| *hosts);

            match allowed_hosts {
                Some(allowed_hosts) => {
                    for host in secret.hosts.as_deref().unwrap_or(&[]) {
                        if !allowed_hosts.contains(&host.as_str()) {
                            anyhow::bail!(
                                "secret '{}' host '{}' is not in the core binding allowlist for '{}'",
                                secret_name,
                                host,
                                env_var
                            );
                        }
                    }
                }
                None => {
                    anyhow::bail!(
                        "secret '{}' env_var '{}' has no core secret binding allowlist entry",
                        secret_name,
                        env_var
                    );
                }
            }
        }
    }

    // WP10/A11: `secrets.{name}.env_var` values become real environment
    // variables (read from the host and injected into the sandbox); reject
    // names no shell or `exec` could set.
    for (secret_name, secret) in &config.secrets {
        if let Some(ref env_var) = secret.env_var {
            if !is_valid_env_var_name(env_var) {
                anyhow::bail!(
                    "secret '{}' env_var '{}' is not a valid environment variable name (must match ^[A-Za-z_][A-Za-z0-9_]*$)",
                    secret_name,
                    env_var
                );
            }
        }
    }

    // Package names in nix-layered images must be in ALLOWED_PACKAGES.
    for (workload_name, workload) in &config.workloads {
        if let Some(ref contents) = workload.image.contents {
            for pkg in contents {
                if !policy::ALLOWED_PACKAGES.contains(&pkg.as_str()) {
                    anyhow::bail!(
                        "workload '{}' package '{}' is not in the core package allowlist",
                        workload_name,
                        pkg
                    );
                }
            }
        }
    }

    // Only entitled workloads may use default_deny = false.
    for (workload_name, workload) in &config.workloads {
        if workload.network.default_deny == Some(false)
            && !policy::DEFAULT_DENY_FALSE_ENTITLEMENT.contains(&workload_name.as_str())
        {
            anyhow::bail!(
                "workload '{}' is not entitled to default_deny=false",
                workload_name
            );
        }
    }

    // Secret references must be defined in the secrets section.
    for (workload_name, workload) in &config.workloads {
        for env in &workload.env {
            if let Some(ref secret_name) = env.secret {
                if !config.secrets.contains_key(secret_name) {
                    anyhow::bail!(
                        "workload '{}' env references undefined secret '{}'",
                        workload_name,
                        secret_name
                    );
                }
            }
        }
        // WP10/A11: env entry names become real environment variables in the
        // sandbox; reject anything that is not a valid env-var name. (The
        // `secret_env` field holds only a `secret` reference — no name to
        // check here; the env var it produces is the secret's own `env_var` /
        // `exposed_as`, validated in the secrets section below.)
        for env in &workload.env {
            if !is_valid_env_var_name(&env.name) {
                anyhow::bail!(
                    "workload '{}' env name '{}' is not a valid environment variable name (must match ^[A-Za-z_][A-Za-z0-9_]*$)",
                    workload_name,
                    env.name
                );
            }
        }
        for se in &workload.secret_env {
            if !config.secrets.contains_key(&se.secret) {
                anyhow::bail!(
                    "workload '{}' secret_env references undefined secret '{}'",
                    workload_name,
                    se.secret
                );
            }
        }
    }

    // ADR 0026(d): depends_on entries must name an existing workload (a
    // workload may NOT depend on itself — self-dependency is nonsensical for
    // discovery), and the injected `env` target must be a valid env-var name.
    for (workload_name, workload) in &config.workloads {
        for (dep, spec) in &workload.depends_on {
            if dep == workload_name {
                anyhow::bail!(
                    "workload '{}' depends_on references itself (self-dependency is not allowed)",
                    workload_name
                );
            }
            if !config.workloads.contains_key(dep) {
                anyhow::bail!(
                    "workload '{}' depends_on references undefined workload '{}'",
                    workload_name,
                    dep
                );
            }
            if !is_valid_env_var_name(&spec.env) {
                anyhow::bail!(
                    "workload '{}' depends_on '{}' env '{}' is not a valid environment variable name (must match ^[A-Za-z_][A-Za-z0-9_]*$)",
                    workload_name,
                    dep,
                    spec.env
                );
            }
        }
    }

    // WP1 trust-boundary validators (closes A2, C2, C3, C4). These run at
    // config-load time so hostile layers are rejected BEFORE merge / plan /
    // sandbox-start. See `80-remediation-plan.md` WP1.
    use crate::microsandbox::{
        validate_env_override, validate_mount_guest, validate_mount_host, validate_seed_source,
    };
    for (workload_name, workload) in &config.workloads {
        for m in &workload.mounts {
            validate_mount_host(&m.host).map_err(|e| {
                anyhow::anyhow!("workload '{workload_name}' mount host validation failed: {e}")
            })?;
            validate_mount_guest(&m.guest, m.read_only).map_err(|e| {
                anyhow::anyhow!("workload '{workload_name}' mount guest validation failed: {e}")
            })?;
        }
        for seed in &workload.seed_files {
            validate_seed_source(&seed.source).map_err(|e| {
                anyhow::anyhow!(
                    "workload '{workload_name}' seed_files.source validation failed: {e}"
                )
            })?;
        }
        if let Some(build) = &workload.local_build {
            if let Some(name) = &build.env_override {
                validate_env_override(name).map_err(|e| {
                    anyhow::anyhow!(
                        "workload '{workload_name}' local_build.env_override validation failed: {e}"
                    )
                })?;
            }
        }
    }

    Ok(())
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

    // ---- WP6(d)/C9: recipe/feature vocabulary validation ----

    #[test]
    fn validate_rejects_unknown_image_recipe() {
        let toml = MINIMAL_VALID_TOML.replace("recipe = \"registry\"", "recipe = \"docker\"");
        let config: ConfigFile = toml::from_str(&toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("image.recipe") && err.contains("'docker'"),
            "error must name the field and the bad value: {err}"
        );
        assert_eq!(
            err,
            "workload 'pi' image.recipe 'docker' is not a known recipe (expected one of: registry, nix-layered)"
        );
    }

    #[test]
    fn validate_rejects_unknown_binary_recipe() {
        let toml = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"nix-layered\", name = \"pi\", binary = { recipe = \"go-build\", src = \"flake://pi\" } }\ncommand = []\n\n[workloads.pi.network]\ndefault_deny = true";
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("image.binary.recipe") && err.contains("'go-build'"),
            "error must name the field and the bad value: {err}"
        );
        assert_eq!(
            err,
            "workload 'pi' image.binary.recipe 'go-build' is not a known recipe (expected one of: npm-build, bun-compile, pip-install, bun-install)"
        );
    }

    #[test]
    fn validate_rejects_unknown_local_build_recipe() {
        let toml = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.network]\ndefault_deny = true\n\n[workloads.pi.local_build]\nrecipe = \"make\"\nsource = \"flake://pi\"";
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("local_build.recipe") && err.contains("'make'"),
            "error must name the field and the bad value: {err}"
        );
        assert_eq!(
            err,
            "workload 'pi' local_build.recipe 'make' is not a known recipe (expected one of: npm-build, bun-compile, pip-install, bun-install)"
        );
    }

    #[test]
    fn validate_rejects_unknown_image_feature() {
        let toml = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"nix-layered\", name = \"pi\", features = [\"bogus\"] }\ncommand = []\n\n[workloads.pi.network]\ndefault_deny = true";
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("image.features") && err.contains("'bogus'"),
            "error must name the field and the bad value: {err}"
        );
        assert_eq!(
            err,
            "workload 'pi' image.features 'bogus' is not in the allowed feature set (expected one of: create_tmp)"
        );
    }

    #[test]
    fn validate_accepts_committed_test_fixture() {
        // Positive test: the committed fixture exercises registry, nix-layered,
        // bun-compile, pip-install, bun-install, npm-build and feature
        // create_tmp — all in the allowed sets.
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("config")
            .join("workestrate.toml");
        let config: ConfigFile = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        validate_config(&config)
            .unwrap_or_else(|e| panic!("committed fixture should validate clean: {e}"));
    }

    #[test]
    fn validate_accepts_reference_config() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("config.reference")
            .join("workestrate.toml");
        let config: ConfigFile = toml::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        validate_config(&config)
            .unwrap_or_else(|e| panic!("reference config should validate clean: {e}"));
    }

    // ---- WP6(f)/E2: schema_version enforcement ----

    #[test]
    fn validate_rejects_unsupported_schema_version() {
        let toml = MINIMAL_VALID_TOML.replace("schema_version = 1", "schema_version = 2");
        let config: ConfigFile = toml::from_str(&toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("is not supported (expected 1)"),
            "error must contain 'is not supported (expected 1)': {err}"
        );
        assert_eq!(
            err,
            "schema_version 2 is not supported (expected 1). This workestrate build supports schema_version 1 only."
        );
    }

    #[test]
    fn validate_accepts_schema_version_1() {
        let config: ConfigFile = toml::from_str(MINIMAL_VALID_TOML).unwrap();
        validate_config(&config).unwrap();
    }

    #[test]
    fn validate_accepts_missing_schema_version_as_legacy() {
        // schema_version is #[serde(default)] → missing parses as 0 → accepted
        // as 1 with a stderr warning (backward compat).
        let toml = MINIMAL_VALID_TOML.replace("schema_version = 1\n\n", "");
        let config: ConfigFile = toml::from_str(&toml).unwrap();
        assert_eq!(config.schema_version, 0);
        validate_config(&config).unwrap();
    }

    #[test]
    fn load_config_rejects_schema_version_2() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-sv2-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        std::fs::write(
            tmp.join("workestrate.toml"),
            MINIMAL_VALID_TOML.replace("schema_version = 1", "schema_version = 2"),
        )?;
        let old = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        std::env::set_var("WORKESTRATE_CONFIG_DIR", &tmp);

        let result = crate::config::load_config();

        match old {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp);

        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("is not supported (expected 1)"),
            "load_config must propagate the schema_version error: {err}"
        );
        Ok(())
    }

    #[test]
    fn load_config_accepts_missing_schema_version() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-sv0-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        std::fs::write(
            tmp.join("workestrate.toml"),
            MINIMAL_VALID_TOML.replace("schema_version = 1\n\n", ""),
        )?;
        let old = std::env::var("WORKESTRATE_CONFIG_DIR").ok();
        std::env::set_var("WORKESTRATE_CONFIG_DIR", &tmp);

        let result = crate::config::load_config();

        match old {
            Some(v) => std::env::set_var("WORKESTRATE_CONFIG_DIR", v),
            None => std::env::remove_var("WORKESTRATE_CONFIG_DIR"),
        }
        let _ = std::fs::remove_dir_all(&tmp);

        result?;
        Ok(())
    }

    // ---- WP10/A11: env var name validation ----

    #[test]
    fn validate_config_rejects_invalid_env_name() {
        let mut config = base_config_for_validation();
        config
            .workloads
            .get_mut("pi")
            .unwrap()
            .env
            .push(crate::config::EnvVarConfig {
                name: "1FOO".to_string(),
                value: Some("x".to_string()),
                secret: None,
            });
        let err = validate_config(&config).unwrap_err();
        let msg = err.to_string();
        assert_eq!(
            msg,
            "workload 'pi' env name '1FOO' is not a valid environment variable name (must match ^[A-Za-z_][A-Za-z0-9_]*$)"
        );
    }

    #[test]
    fn validate_config_accepts_valid_env_name() -> Result<()> {
        let mut config = base_config_for_validation();
        config
            .workloads
            .get_mut("pi")
            .unwrap()
            .env
            .push(crate::config::EnvVarConfig {
                name: "VALID_NAME".to_string(),
                value: Some("x".to_string()),
                secret: None,
            });
        validate_config(&config)
    }

    #[test]
    fn validate_config_rejects_invalid_secret_env_var() {
        let toml = r#"
schema_version = 1

[secrets.MY_SECRET]
env_var = "BAD-NAME"

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []
log_stop_errors = false

[workloads.pi.network]
default_deny = true
"#;
        let config: ConfigFile = toml::from_str(toml).expect("config must parse");
        let err = validate_config(&config).unwrap_err();
        let msg = err.to_string();
        // The secret-bindings check runs before the env_var-name check, and
        // BAD-NAME has no allowlist entry — both orderings name the secret
        // and the offending value, so assert on the stable shared content.
        assert!(
            msg.contains("env_var 'BAD-NAME'"),
            "error should name the secret env_var value; got: {msg}"
        );
        assert!(
            msg.contains("MY_SECRET"),
            "error should name the secret; got: {msg}"
        );
    }

    /// Spec 13: a shorthand (bare-string) `secret_env` entry with a typo'd
    /// name must be caught by the same undefined-secret check as the table
    /// form, and the error must name the missing secret.
    #[test]
    fn validate_config_rejects_shorthand_secret_env_typo() {
        let toml = r#"
schema_version = 1

[secrets.GITHUB_TOKEN]
env_var = "GITHUB_TOKEN"

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []
secret_env = ["GITHUB_TOKEN_TYPO"]

[workloads.pi.network]
default_deny = true
"#;
        let config: ConfigFile = toml::from_str(toml).expect("config must parse");
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "workload 'pi' secret_env references undefined secret 'GITHUB_TOKEN_TYPO'"
        );
    }

    #[test]
    fn env_var_name_helper_matches_posix_shape() {
        for ok in ["A", "_FOO", "ABC_123", "a", "Z9_", "VALID_NAME"] {
            assert!(is_valid_env_var_name(ok), "'{ok}' should be valid");
        }
        for bad in ["", "1FOO", "FOO-BAR", "FOO BAR", "FOO.BAR", "-A"] {
            assert!(!is_valid_env_var_name(bad), "'{bad}' should be invalid");
        }
    }

    // ---- ADR 0026(d): depends_on validation ----

    /// Base config plus a second workload that `pi` can legitimately depend
    /// on; the caller mutates it per test.
    fn depends_on_config() -> ConfigFile {
        let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []
log_stop_errors = false

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"

[workloads.pi.network]
default_deny = true

[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.litellm.network]
default_deny = true
"#;
        toml::from_str(toml).expect("depends_on config must parse")
    }

    #[test]
    fn validate_config_accepts_valid_depends_on() -> Result<()> {
        let config = depends_on_config();
        validate_config(&config)
    }

    #[test]
    fn validate_config_rejects_undefined_depends_on_dep() {
        let mut config = depends_on_config();
        config.workloads.get_mut("pi").unwrap().depends_on.insert(
            "missing".to_string(),
            crate::config::DependsOnSpec {
                env: "MISSING_URL".to_string(),
                required: false,
            },
        );
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "workload 'pi' depends_on references undefined workload 'missing'"
        );
    }

    #[test]
    fn validate_config_rejects_depends_on_self_dependency() {
        let mut config = depends_on_config();
        config.workloads.get_mut("pi").unwrap().depends_on.insert(
            "pi".to_string(),
            crate::config::DependsOnSpec {
                env: "SELF_URL".to_string(),
                required: false,
            },
        );
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "workload 'pi' depends_on references itself (self-dependency is not allowed)"
        );
    }

    #[test]
    fn validate_config_rejects_invalid_depends_on_env_name() {
        let mut config = depends_on_config();
        config
            .workloads
            .get_mut("pi")
            .unwrap()
            .depends_on
            .get_mut("litellm")
            .unwrap()
            .env = "BAD-NAME".to_string();
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "workload 'pi' depends_on 'litellm' env 'BAD-NAME' is not a valid environment variable name (must match ^[A-Za-z_][A-Za-z0-9_]*$)"
        );
    }
}
