//! Config validation: schema-version enforcement, recipe/feature vocabulary,
//! policy allowlists, trust-boundary validators, and identifier validators.

use anyhow::Result;
use std::collections::HashMap;

use crate::config::types::ConfigFile;
use crate::policy;
use crate::recipes::EgressRecipeRef;

// ---------------------------------------------------------------------------
// workestrate.toml schema
// ---------------------------------------------------------------------------

/// The schema_version this build supports (spec 16).
///
/// `ConfigFile.schema_version` is `#[serde(default)]`, so a MISSING version
/// parses as 0 — `validate_config` treats 0 as "absent/legacy" and accepts it
/// with a stderr warning (backward compat). An explicit `schema_version = 0`
/// is a degenerate case that falls into the same warn+accept bucket.
/// `schema_version = 1` is the native and ONLY supported version (the
/// intermediate v2 event was retracted pre-release — there was no production
/// deployment and no migration to preserve). `schema_version = 2` and
/// anything above is a hard error.
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

/// Allowed `workloads.{wl}.entitlements` entries. Entitlements are
/// config-declared (no core hardcoded workload names); this is the closed
/// vocabulary core understands. `"default_deny_false"` permits
/// `network.default_deny = false`.
const ALLOWED_ENTITLEMENTS: &[&str] = &["default_deny_false"];

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

/// Whether `name` is a syntactically valid named-port slug:
/// `^[a-z0-9][a-z0-9-]*$` — first char `[a-z0-9]`, remaining chars
/// `[a-z0-9-]`, empty invalid. Named ports are referenced by
/// `depends_on.exports` keys and flow into plan/registry JSON, so the charset
/// is deliberately narrow (lowercase/digits/hyphens; no uppercase,
/// underscores, dots, or leading hyphen).
fn is_valid_port_name(name: &str) -> bool {
    let mut chars = name.chars();
    matches!(
        chars.next(),
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit()
    ) && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Validate a parsed [`ConfigFile`] against the invariants the TOML schema
/// alone cannot express: `schema_version` support (see
/// [`EXPECTED_SCHEMA_VERSION`]), recipe/feature vocabulary allowlists, trust,
/// and mount/env/secret rules. Returns the first violation found as a hard
/// error.
pub fn validate_config(config: &ConfigFile) -> Result<()> {
    // WP6(f)/E2 + spec 16: schema_version enforcement.
    // `ConfigFile.schema_version` is #[serde(default)], so a MISSING version
    // parses as 0 — treated as "absent/legacy": accepted with a stderr
    // warning (backward compat). An explicit `schema_version = 0` is a
    // degenerate case that falls into the same warn+accept bucket. Version 1
    // is native. Version 2 is a hard error (the intermediate v2 event was
    // retracted pre-release). Anything above is a hard error.
    match config.schema_version {
        0 => {
            eprintln!("WARNING: schema_version missing; assuming 1 (backward compat)");
        }
        1 => {}
        2 => {
            anyhow::bail!(
                "schema_version 2 is not supported; the unified model is schema_version 1"
            );
        }
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

    // Secret-declared egress hosts must come from the core egress allowlist
    // (fail-closed, generic: there is no core per-secret table — any
    // config-declared secret may bind hosts, but only allowlisted ones).
    // Secrets WITHOUT `env_var` are skipped: they are never read from the
    // host environment, so their `allowed_hosts` cannot route a host-env
    // value anywhere (the declaration may still be used by other binding
    // modes; gating preserved from the retired core binding table).
    for (secret_name, secret) in &config.secrets {
        if secret.env_var.is_some() {
            for host in secret.allowed_hosts.as_deref().unwrap_or(&[]) {
                if !policy::ALLOWED_EGRESS_HOSTS.contains(&host.as_str()) {
                    anyhow::bail!(
                        "secret '{}' allowed_hosts entry '{}' is not in the core egress allowlist",
                        secret_name,
                        host
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

    // Entitlements vocabulary + the default_deny gate (fail-closed):
    // `default_deny = false` requires the workload to DECLARE the
    // `default_deny_false` entitlement itself — no core-hardcoded names.
    for (workload_name, workload) in &config.workloads {
        for entitlement in &workload.entitlements {
            if !ALLOWED_ENTITLEMENTS.contains(&entitlement.as_str()) {
                anyhow::bail!(
                    "workload '{}' entitlement '{}' is not a known entitlement (expected one of: {})",
                    workload_name,
                    entitlement,
                    ALLOWED_ENTITLEMENTS.join(", ")
                );
            }
        }
        if workload.network.default_deny == Some(false)
            && !workload
                .entitlements
                .iter()
                .any(|e| e == "default_deny_false")
        {
            anyhow::bail!(
                "workload '{}' sets default_deny=false without declaring entitlements = [\"default_deny_false\"]",
                workload_name
            );
        }
    }

    // Secret references must be defined in the secrets section.
    for (workload_name, workload) in &config.workloads {
        for (env_name, binding) in workload.env.iter() {
            if let crate::config::EnvBinding::Secret(ref_) = binding {
                if !config.secrets.contains_key(&ref_.secret) {
                    anyhow::bail!(
                        "workload '{}' env references undefined secret '{}'",
                        workload_name,
                        ref_.secret
                    );
                }
            }
            // WP10/A11: env binding keys become real environment variables in
            // the sandbox; reject anything that is not a valid env-var name.
            if !is_valid_env_var_name(env_name) {
                anyhow::bail!(
                    "workload '{}' env name '{}' is not a valid environment variable name (must match ^[A-Za-z_][A-Za-z0-9_]*$)",
                    workload_name,
                    env_name
                );
            }
        }
    }

    // ADR 0026(d): depends_on entries must name an existing workload (a
    // workload may NOT depend on itself — self-dependency is nonsensical for
    // discovery), and the injected `env` target (when present) must be a
    // valid env-var name.
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
            if let Some(env) = &spec.env {
                if !is_valid_env_var_name(env) {
                    anyhow::bail!(
                        "workload '{}' depends_on '{}' env '{}' is not a valid environment variable name (must match ^[A-Za-z_][A-Za-z0-9_]*$)",
                        workload_name,
                        dep,
                        env
                    );
                }
            }
        }
    }

    // P0: namespaced ports + depends_on exports (ADR 0026(d) exports).
    // NOTE: `host = 0` is LEGAL (auto-allocation) — it means "allocate a free
    // port at boot" (allocator: port_registry probe_free_ports); no code
    // check rejects it here.
    //
    // (a) At-least-one rule: a depends_on entry must inject via `env` and/or
    // `exports` — an entry declaring neither resolves nothing.
    for (workload_name, workload) in &config.workloads {
        for (dep, spec) in &workload.depends_on {
            if spec.env.is_none() && spec.exports.is_empty() {
                anyhow::bail!(
                    "workload '{}' depends_on '{}' must declare `env` (primary/legacy form) or at least one `exports` entry",
                    workload_name,
                    dep
                );
            }
        }
    }

    // (b)/(c) Port names: unique within a workload and `^[a-z0-9][a-z0-9-]*$`
    // (first char ascii_lowercase/ascii_digit; then ascii_lowercase/ascii_digit/
    // '-'; empty invalid). Unnamed ports are the legacy primary port and need
    // no validation here.
    for (workload_name, workload) in &config.workloads {
        let mut seen: Vec<&str> = Vec::new();
        for p in &workload.ports {
            let Some(name) = p.name.as_deref() else {
                continue;
            };
            if !is_valid_port_name(name) {
                anyhow::bail!(
                    "workload '{}' port name '{}' is not a valid port name (must match ^[a-z0-9][a-z0-9-]*$)",
                    workload_name,
                    name
                );
            }
            if seen.contains(&name) {
                anyhow::bail!(
                    "workload '{}' declares duplicate port name '{}' (port names must be unique within a workload)",
                    workload_name,
                    name
                );
            }
            seen.push(name);
        }
    }

    // (d) Every `exports` key must name a port the dependency DECLARES
    // (`config.workloads[dep].ports` with `name == Some(key)`); a typo'd or
    // stale key would silently inject nothing, so it is a hard error.
    for (workload_name, workload) in &config.workloads {
        let mut deps: Vec<&String> = workload.depends_on.keys().collect();
        deps.sort();
        for dep in deps {
            let spec = &workload.depends_on[dep];
            if spec.exports.is_empty() {
                continue;
            }
            let declared: Vec<&str> = config
                .workloads
                .get(dep)
                .map(|w| w.ports.iter().filter_map(|p| p.name.as_deref()).collect())
                .unwrap_or_default();
            let mut keys: Vec<&String> = spec.exports.keys().collect();
            keys.sort();
            for key in keys {
                if !declared.contains(&key.as_str()) {
                    let declared_str = if declared.is_empty() {
                        "none".to_string()
                    } else {
                        declared.join(", ")
                    };
                    anyhow::bail!(
                        "workload '{}' depends_on '{}' exports key '{}' does not name a port declared by dependency '{}' (declared named ports: {})",
                        workload_name,
                        dep,
                        key,
                        dep,
                        declared_str
                    );
                }
            }
        }
    }

    // (e) Every injected env-var name — `spec.env` (validated in the ADR
    // 0026(d) loop above) and EVERY `exports` VALUE — must be a valid
    // env-var name (WP10/A11; a name no shell or `exec` could set).
    for (workload_name, workload) in &config.workloads {
        for (dep, spec) in &workload.depends_on {
            let mut ports: Vec<&String> = spec.exports.keys().collect();
            ports.sort();
            for port_name in ports {
                let env_name = &spec.exports[port_name];
                if !is_valid_env_var_name(env_name) {
                    anyhow::bail!(
                        "workload '{}' depends_on '{}' exports port '{}' env var '{}' is not a valid environment variable name (must match ^[A-Za-z_][A-Za-z0-9_]*$)",
                        workload_name,
                        dep,
                        port_name,
                        env_name
                    );
                }
            }
        }
    }

    // (f) Injected env names must be unique across a workload's deps: two
    // different deps (or the same dep twice, via env + an exports value)
    // injecting the SAME env name would clobber at plan time. Deterministic:
    // deps iterated SORTED by name; env names reported SORTED.
    for (workload_name, workload) in &config.workloads {
        let mut deps: Vec<&String> = workload.depends_on.keys().collect();
        deps.sort();
        let mut by_env: HashMap<&str, Vec<&str>> = HashMap::new();
        for dep in deps {
            let spec = &workload.depends_on[dep];
            if let Some(env) = &spec.env {
                by_env.entry(env.as_str()).or_default().push(dep.as_str());
            }
            let mut ports: Vec<&String> = spec.exports.keys().collect();
            ports.sort();
            for port_name in ports {
                by_env
                    .entry(spec.exports[port_name].as_str())
                    .or_default()
                    .push(dep.as_str());
            }
        }
        let mut env_names: Vec<&&str> = by_env.keys().collect();
        env_names.sort();
        for env_name in env_names {
            if by_env[env_name].len() > 1 {
                let mut sources = by_env[env_name].clone();
                sources.sort();
                sources.dedup();
                anyhow::bail!(
                    "workload '{}' depends_on injects env var '{}' from multiple sources (dependencies: {}); each injected env var must come from exactly one source",
                    workload_name,
                    env_name,
                    sources.join(", ")
                );
            }
        }
    }

    // ADR 0026 addendum (2026-08-01): dependency CYCLE detection is MANDATORY
    // in config validation — before this, A→B→A loaded cleanly (discovery-lite
    // has no topological need; the W4 compose-mirrored dependency lifecycle's
    // ORDERING does). Delegates to the three-color (white/gray/black) DFS in
    // `microsandbox::depgraph::topo_all` (deterministic: sorted roots, sorted
    // dep iteration) so validation and the runtime graph share exactly one
    // implementation and one error shape. PRECEDENCE: a self-dependency never
    // reaches here — the self-dep check above fires first and reports the
    // self-dep error (deliberate: it is the more specific diagnostic).
    crate::microsandbox::depgraph::topo_all(config)?;

    // WP1 trust-boundary validators (closes A2, C2, C3, C4). These run at
    // config-load time so hostile layers are rejected BEFORE merge / plan /
    // sandbox-start. See `80-remediation-plan.md` WP1.
    use crate::microsandbox::{
        validate_env_override, validate_mount_guest, validate_mount_host, validate_seed_glob,
        validate_seed_source, validate_seed_target,
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
        // Cross-mount pass: EXACT-duplicate guest paths are a hard error —
        // two binds at the same guest would emit the SAME virtiofs tag
        // (`guest_mount_tag` hashes the guest path) and silently last-wins
        // in-guest (spawn.rs dir-mount path). Mirrors the SDK's
        // canonicalized duplicate-disk rejection (spawn.rs:1407-1415).
        // NESTED guest paths stay LEGAL — that is the spec 01 shadow-mount
        // pattern (a read-only base plus a nested read-write shadow), so only
        // exact duplicates are rejected here.
        let mut seen_guests: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for m in &workload.mounts {
            if !seen_guests.insert(m.guest.as_str()) {
                anyhow::bail!(
                    "workload '{workload_name}' declares more than one mount with guest '{}':                      duplicate guest paths would emit the same virtiofs tag and mount                      last-wins in-guest; merge the mounts (nested guests remain legal —                      spec 01 shadow-mount pattern)",
                    m.guest
                );
            }
        }
        for seed in &workload.seed_files {
            match (&seed.source, &seed.glob) {
                (None, None) => anyhow::bail!(
                    "workload '{workload_name}' seed_files entry must declare exactly one of `source` or `glob`"
                ),
                (Some(_), Some(_)) => anyhow::bail!(
                    "workload '{workload_name}' seed_files entry cannot declare both `source` and `glob`"
                ),
                (Some(src), None) => validate_seed_source(src).map_err(|e| {
                    anyhow::anyhow!(
                        "workload '{workload_name}' seed_files.source validation failed: {e}"
                    )
                })?,
                (None, Some(glob)) => validate_seed_glob(glob).map_err(|e| {
                    anyhow::anyhow!(
                        "workload '{workload_name}' seed_files.glob validation failed: {e}"
                    )
                })?,
            }
            validate_seed_target(&seed.target).map_err(|e| {
                anyhow::anyhow!(
                    "workload '{workload_name}' seed_files.target validation failed: {e}"
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

    /// A bun-compile binary WITHOUT a worker (prime-agent compiles workerless)
    /// must validate clean.
    #[test]
    fn validate_accepts_binary_without_worker() {
        let toml = "schema_version = 1\n\n[workloads.prime]\nkind = \"agent\"\nimage = { recipe = \"nix-layered\", name = \"prime\", tag = \"latest\", binary = { recipe = \"bun-compile\", src = \"flake://prime\", entrypoint = \"packages/coding-agent/dist/bun/cli.js\" } }\ncommand = []\n\n[workloads.prime.network]\ndefault_deny = true";
        let config: ConfigFile = toml::from_str(toml).unwrap();
        validate_config(&config)
            .unwrap_or_else(|e| panic!("workerless bun-compile binary should validate clean: {e}"));
    }

    /// A bun-compile binary WITH a worker (pi passes its image-resize-worker.ts)
    /// must validate clean.
    #[test]
    fn validate_accepts_binary_with_worker() {
        let toml = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"nix-layered\", name = \"pi\", tag = \"latest\", binary = { recipe = \"bun-compile\", src = \"flake://pi\", entrypoint = \"packages/coding-agent/dist/bun/cli.js\", worker = \"packages/coding-agent/src/utils/image-resize-worker.ts\" } }\ncommand = []\n\n[workloads.pi.network]\ndefault_deny = true";
        let config: ConfigFile = toml::from_str(toml).unwrap();
        validate_config(&config).unwrap_or_else(|e| {
            panic!("bun-compile binary with worker should validate clean: {e}")
        });
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

    // ---- WP6(f)/E2 + spec 16: schema_version enforcement ----

    #[test]
    fn validate_rejects_unsupported_schema_version() {
        let toml = MINIMAL_VALID_TOML.replace("schema_version = 1", "schema_version = 3");
        let config: ConfigFile = toml::from_str(&toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("is not supported (expected 1)"),
            "error must contain 'is not supported (expected 1)': {err}"
        );
        assert_eq!(
            err,
            "schema_version 3 is not supported (expected 1). This workestrate build supports schema_version 1 only."
        );
    }

    #[test]
    fn validate_rejects_schema_version_2_with_unified_model_error() {
        // The intermediate v2 event was retracted pre-release: schema_version
        // 2 is a hard error with the exact unified-model message.
        let toml = MINIMAL_VALID_TOML.replace("schema_version = 1", "schema_version = 2");
        let config: ConfigFile = toml::from_str(&toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "schema_version 2 is not supported; the unified model is schema_version 1"
        );
    }

    #[test]
    fn validate_accepts_schema_version_1_native() {
        // v1 is the native and only schema version — accepted silently.
        let config: ConfigFile = toml::from_str(MINIMAL_VALID_TOML).unwrap();
        validate_config(&config).unwrap();
    }

    #[test]
    fn validate_accepts_missing_schema_version_as_legacy() {
        // schema_version is #[serde(default)] → missing parses as 0 → accepted
        // with a stderr warning (backward compat).
        let toml = MINIMAL_VALID_TOML.replace("schema_version = 1\n\n", "");
        let config: ConfigFile = toml::from_str(&toml).unwrap();
        assert_eq!(config.schema_version, 0);
        validate_config(&config).unwrap();
    }

    #[test]
    fn load_config_rejects_schema_version_3() -> Result<()> {
        let _lock = ENV_TEST_LOCK.lock().unwrap();
        let tmp = std::env::temp_dir().join(format!(
            "workestrate-sv3-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp)?;
        std::fs::write(
            tmp.join("workestrate.toml"),
            MINIMAL_VALID_TOML.replace("schema_version = 1", "schema_version = 3"),
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
            .upsert("1FOO", crate::config::EnvBinding::Literal("x".to_string()));
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
        config.workloads.get_mut("pi").unwrap().env.upsert(
            "VALID_NAME",
            crate::config::EnvBinding::Literal("x".to_string()),
        );
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
        // The secret allowed-hosts check runs before the env_var-name check;
        // BAD-NAME declares no allowed_hosts, so the env_var-name check is
        // the one that fires — it names both the secret and the value.
        assert!(
            msg.contains("env_var 'BAD-NAME'"),
            "error should name the secret env_var value; got: {msg}"
        );
        assert!(
            msg.contains("MY_SECRET"),
            "error should name the secret; got: {msg}"
        );
    }

    /// A map-form `env` entry with a typo'd secret name must be caught by
    /// the undefined-secret check, and the error must name the missing
    /// secret.
    #[test]
    fn validate_config_rejects_map_form_env_secret_typo() {
        let toml = r#"
schema_version = 1

[secrets.GITHUB_TOKEN]
env_var = "GITHUB_TOKEN"

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []
env = { WHATEVER = { secret = "GITHUB_TOKEN_TYPO" } }

[workloads.pi.network]
default_deny = true
"#;
        let config: ConfigFile = toml::from_str(toml).expect("config must parse");
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "workload 'pi' env references undefined secret 'GITHUB_TOKEN_TYPO'"
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

    // ---- Generic secret allowed-hosts + declared entitlements (phase 4) ----

    #[test]
    fn validate_rejects_secret_allowed_host_outside_core_egress_allowlist() {
        let toml = r#"
schema_version = 1

[secrets.MY_KEY]
env_var = "MY_KEY"
allowed_hosts = ["evil.example.com"]

[workloads.example-agent]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.example-agent.network]
default_deny = true
"#;
        let config: ConfigFile = toml::from_str(toml).expect("config must parse");
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "secret 'MY_KEY' allowed_hosts entry 'evil.example.com' is not in the core egress allowlist"
        );
    }

    #[test]
    fn validate_accepts_secret_allowed_host_inside_core_egress_allowlist() {
        let toml = r#"
schema_version = 1

[secrets.MY_KEY]
env_var = "MY_KEY"
allowed_hosts = ["github.com", "api.github.com"]

[workloads.example-agent]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.example-agent.network]
default_deny = true
"#;
        let config: ConfigFile = toml::from_str(toml).expect("config must parse");
        validate_config(&config).expect("allowlisted hosts must validate");
    }

    /// Secrets WITHOUT `env_var` are never read from the host environment,
    /// so their allowed_hosts are not gated against the egress allowlist
    /// (gating preserved from the retired core per-secret binding table).
    #[test]
    fn validate_skips_allowed_hosts_check_for_secret_without_env_var() {
        let toml = r#"
schema_version = 1

[secrets.EXTERNAL_ONLY]
allowed_hosts = ["not-in-the-core-allowlist.example.com"]
required = false

[workloads.example-agent]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.example-agent.network]
default_deny = true
"#;
        let config: ConfigFile = toml::from_str(toml).expect("config must parse");
        validate_config(&config).expect("secrets without env_var skip the allowed-hosts gate");
    }

    #[test]
    fn validate_rejects_default_deny_false_without_declared_entitlement() {
        let toml = r#"
schema_version = 1

[workloads.example-offensive]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.example-offensive.network]
default_deny = false
"#;
        let config: ConfigFile = toml::from_str(toml).expect("config must parse");
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "workload 'example-offensive' sets default_deny=false without declaring entitlements = [\"default_deny_false\"]"
        );
    }

    #[test]
    fn validate_accepts_default_deny_false_with_declared_entitlement() {
        let toml = r#"
schema_version = 1

[workloads.example-offensive]
kind = "agent"
entitlements = ["default_deny_false"]
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.example-offensive.network]
default_deny = false
"#;
        let config: ConfigFile = toml::from_str(toml).expect("config must parse");
        validate_config(&config).expect("declared entitlement must validate");
    }

    #[test]
    fn validate_rejects_unknown_entitlement() {
        let toml = r#"
schema_version = 1

[workloads.example-agent]
kind = "agent"
entitlements = ["root_access"]
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.example-agent.network]
default_deny = true
"#;
        let config: ConfigFile = toml::from_str(toml).expect("config must parse");
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "workload 'example-agent' entitlement 'root_access' is not a known entitlement (expected one of: default_deny_false)"
        );
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
                env: Some("MISSING_URL".to_string()),
                required: false,
                exports: Default::default(),
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
                env: Some("SELF_URL".to_string()),
                required: false,
                exports: Default::default(),
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
            .env = Some("BAD-NAME".to_string());
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "workload 'pi' depends_on 'litellm' env 'BAD-NAME' is not a valid environment variable name (must match ^[A-Za-z_][A-Za-z0-9_]*$)"
        );
    }

    // ---- P0: namespaced ports + depends_on exports validation ----

    /// Base config with a named port (`http`) on `litellm` that `pi` exports;
    /// the caller mutates it per test.
    fn named_ports_depends_config() -> ConfigFile {
        let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []
log_stop_errors = false

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"
exports = { http = "LITELLM_HTTP_URL" }

[workloads.pi.network]
default_deny = true

[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[[workloads.litellm.ports]]
host = 4000
guest = 4000
name = "http"

[workloads.litellm.network]
default_deny = true
"#;
        toml::from_str(toml).expect("named-ports depends_on config must parse")
    }

    #[test]
    fn validate_config_accepts_named_ports_with_exports() -> Result<()> {
        // Positive: env + a valid exports entry, and the dependency declares
        // the exported named port.
        let config = named_ports_depends_config();
        validate_config(&config)
    }

    #[test]
    fn validate_config_accepts_exports_only_depends_on() -> Result<()> {
        // At-least-one rule: exports-only (no `env`) is legal.
        let mut config = named_ports_depends_config();
        config
            .workloads
            .get_mut("pi")
            .unwrap()
            .depends_on
            .get_mut("litellm")
            .unwrap()
            .env = None;
        validate_config(&config)
    }

    #[test]
    fn validate_config_rejects_depends_on_without_env_or_exports() {
        let mut config = depends_on_config();
        config
            .workloads
            .get_mut("pi")
            .unwrap()
            .depends_on
            .get_mut("litellm")
            .unwrap()
            .env = None;
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "workload 'pi' depends_on 'litellm' must declare `env` (primary/legacy form) or at least one `exports` entry"
        );
    }

    #[test]
    fn validate_config_rejects_duplicate_port_name() {
        let mut config = depends_on_config();
        config.workloads.get_mut("litellm").unwrap().ports.extend([
            crate::microsandbox::plan::PortMapping::new(4000, 4000),
            crate::microsandbox::plan::PortMapping::new(4001, 4001),
        ]);
        for p in config
            .workloads
            .get_mut("litellm")
            .unwrap()
            .ports
            .iter_mut()
        {
            p.name = Some("http".to_string());
        }
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "workload 'litellm' declares duplicate port name 'http' (port names must be unique within a workload)"
        );
    }

    #[test]
    fn validate_config_rejects_invalid_port_name_slug() {
        let mut config = depends_on_config();
        config
            .workloads
            .get_mut("litellm")
            .unwrap()
            .ports
            .push(crate::microsandbox::plan::PortMapping::new(4000, 4000));
        config.workloads.get_mut("litellm").unwrap().ports[0].name = Some("HTTP_Port".to_string());
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("is not a valid port name") && err.contains("HTTP_Port"),
            "invalid slug must be rejected and name the value: {err}"
        );
        assert!(
            err.contains("^[a-z0-9][a-z0-9-]*$"),
            "error must carry the pattern: {err}"
        );
    }

    #[test]
    fn validate_config_rejects_exports_key_for_undeclared_port() {
        let mut config = named_ports_depends_config();
        config
            .workloads
            .get_mut("pi")
            .unwrap()
            .depends_on
            .get_mut("litellm")
            .unwrap()
            .exports
            .insert("admin".to_string(), "LITELLM_ADMIN_URL".to_string());
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "workload 'pi' depends_on 'litellm' exports key 'admin' does not name a port declared by dependency 'litellm' (declared named ports: http)"
        );
    }

    #[test]
    fn validate_config_rejects_exports_key_when_dependency_has_no_named_ports() {
        // The dependency declares ONLY unnamed ports → declared list is "none".
        let mut config = named_ports_depends_config();
        config
            .workloads
            .get_mut("litellm")
            .unwrap()
            .ports
            .iter_mut()
            .for_each(|p| p.name = None);
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "workload 'pi' depends_on 'litellm' exports key 'http' does not name a port declared by dependency 'litellm' (declared named ports: none)"
        );
    }

    #[test]
    fn validate_config_rejects_invalid_exports_env_var_name() {
        let mut config = named_ports_depends_config();
        config
            .workloads
            .get_mut("pi")
            .unwrap()
            .depends_on
            .get_mut("litellm")
            .unwrap()
            .exports
            .insert("http".to_string(), "BAD-NAME".to_string());
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "workload 'pi' depends_on 'litellm' exports port 'http' env var 'BAD-NAME' is not a valid environment variable name (must match ^[A-Za-z_][A-Za-z0-9_]*$)"
        );
    }

    #[test]
    fn validate_config_rejects_duplicate_injected_env_across_deps() {
        // Two deps (ahead < litellm, sorted deterministically) both inject
        // `SHARED_URL`; the error names the workload, the var, and both deps.
        let mut config = named_ports_depends_config();
        let litellm = config.workloads.get("litellm").unwrap().clone();
        config.workloads.insert("ahead".to_string(), litellm);
        config.workloads.get_mut("pi").unwrap().depends_on.insert(
            "ahead".to_string(),
            crate::config::DependsOnSpec {
                env: Some("SHARED_URL".to_string()),
                required: false,
                exports: Default::default(),
            },
        );
        config
            .workloads
            .get_mut("pi")
            .unwrap()
            .depends_on
            .get_mut("litellm")
            .unwrap()
            .env = Some("SHARED_URL".to_string());
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "workload 'pi' depends_on injects env var 'SHARED_URL' from multiple sources (dependencies: ahead, litellm); each injected env var must come from exactly one source"
        );
    }

    #[test]
    fn validate_config_rejects_same_dep_env_and_exports_collision() {
        // Same dep injecting the same env name via BOTH env and an exports
        // value is also a duplicate source.
        let mut config = named_ports_depends_config();
        config
            .workloads
            .get_mut("pi")
            .unwrap()
            .depends_on
            .get_mut("litellm")
            .unwrap()
            .env = Some("LITELLM_HTTP_URL".to_string());
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(
            err,
            "workload 'pi' depends_on injects env var 'LITELLM_HTTP_URL' from multiple sources (dependencies: litellm); each injected env var must come from exactly one source"
        );
    }

    #[test]
    fn port_name_helper_matches_slug_shape() {
        for ok in ["a", "http", "db-0", "a1", "0", "x-y-z"] {
            assert!(is_valid_port_name(ok), "'{ok}' should be a valid port name");
        }
        for bad in [
            "",
            "A",
            "HTTP",
            "http_port",
            "http.port",
            "-lead",
            "a b",
            "a/b",
        ] {
            assert!(!is_valid_port_name(bad), "'{bad}' should be invalid");
        }
    }

    // ---- ADR 0026 addendum (2026-08-01): dependency CYCLE detection ----
    //
    // Precedence note: a SELF-dependency still reports the self-dep error
    // ("... depends_on references itself ..."), NOT the cycle error — the
    // self-dep check runs first in `validate_config` and that precedence is
    // kept deliberately (it is the more specific diagnostic). The cycle
    // check below is what rejects MULTI-node cycles (A→B→A, A→B→C→A), which
    // loaded cleanly before this addendum.

    /// Cycle fixture builder: workloads a/b(/c) with the given edges.
    fn cycle_config(edges: &[(&str, &str)]) -> ConfigFile {
        let mut toml = String::from("schema_version = 1\n");
        for name in ["a", "b", "c"] {
            toml.push_str(&format!(
                "\n[workloads.{name}]\nkind = \"service\"\nimage = {{ recipe = \"registry\", ref = \"node:24-bookworm-slim\" }}\ncommand = []\n\n[workloads.{name}.network]\ndefault_deny = true\n"
            ));
        }
        for (from, to) in edges {
            toml.push_str(&format!(
                "\n[workloads.{from}.depends_on.{to}]\nenv = \"{}_URL\"\n",
                to.to_uppercase()
            ));
        }
        toml::from_str(&toml).expect("cycle fixture must parse")
    }

    #[test]
    fn validate_config_rejects_two_node_dependency_cycle() {
        let config = cycle_config(&[("a", "b"), ("b", "a")]);
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("dependency cycle detected") && err.contains('a') && err.contains('b'),
            "error must name the check and both workloads: {err}"
        );
        assert_eq!(err, "dependency cycle detected: a → b → a");
    }

    #[test]
    fn validate_config_rejects_three_node_dependency_cycle() {
        let config = cycle_config(&[("a", "b"), ("b", "c"), ("c", "a")]);
        let err = validate_config(&config).unwrap_err().to_string();
        assert_eq!(err, "dependency cycle detected: a → b → c → a");
    }

    #[test]
    fn validate_config_accepts_dependency_dag_diamond() -> Result<()> {
        // Diamond DAG: a → {b, c}, b → c — deps before dependents, no cycle.
        let config = cycle_config(&[("a", "b"), ("a", "c"), ("b", "c")]);
        validate_config(&config)
    }

    // ---- P0: seed_files source|glob exclusivity + target safety ----

    /// Base config with a source-only seed entry; the caller mutates it per
    /// test.
    fn seed_config() -> ConfigFile {
        let toml = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = []

[[workloads.svc.seed_files]]
source = "seed/a.json"
target = "workspaces/svc-state/a.json"

[workloads.svc.network]
default_deny = true
"#;
        toml::from_str(toml).expect("seed config must parse")
    }

    #[test]
    fn validate_config_rejects_seed_with_both_source_and_glob() {
        let mut config = seed_config();
        config.workloads.get_mut("svc").unwrap().seed_files[0].glob =
            Some("seed/**/*.json".to_string());
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("cannot declare both `source` and `glob`"),
            "source+glob together must be rejected: {err}"
        );
    }

    #[test]
    fn validate_config_rejects_seed_with_neither_source_nor_glob() {
        let mut config = seed_config();
        config.workloads.get_mut("svc").unwrap().seed_files[0].source = None;
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("must declare exactly one of `source` or `glob`"),
            "neither source nor glob must be rejected: {err}"
        );
    }

    #[test]
    fn validate_config_accepts_seed_with_either() -> Result<()> {
        // A source-only entry AND a glob-only entry in the SAME config both
        // pass validation.
        let toml = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = []

[[workloads.svc.seed_files]]
source = "seed/a.json"
target = "workspaces/svc-state/a.json"

[[workloads.svc.seed_files]]
glob = "seed/**/*.env"
target = "workspaces/svc-state/env"

[workloads.svc.network]
default_deny = true
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        validate_config(&config)
    }

    #[test]
    fn validate_config_rejects_absolute_seed_target() {
        let mut config = seed_config();
        config.workloads.get_mut("svc").unwrap().seed_files[0].target = "/etc/x".to_string();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("seed_files.target validation failed") && err.contains("absolute"),
            "absolute seed target must be rejected: {err}"
        );
    }

    #[test]
    fn validate_config_rejects_traversal_seed_target() {
        let mut config = seed_config();
        config.workloads.get_mut("svc").unwrap().seed_files[0].target = "a/../b".to_string();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("seed_files.target validation failed") && err.contains("'..'"),
            "traversal seed target must be rejected: {err}"
        );
    }
    // ---- E0 / ADR 0028 companion: duplicate-guest guard ----

    /// Exact-duplicate guest paths are a HARD error (two binds at the same
    /// guest would emit the same virtiofs tag and silently last-wins
    /// in-guest — mirrors the SDK disk-mount duplicate rejection).
    #[test]
    fn validate_rejects_exact_duplicate_mount_guests() {
        let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[[workloads.pi.mounts]]
host = "state/a"
guest = "/work"
read_only = false

[[workloads.pi.mounts]]
host = "state/b"
guest = "/work"
read_only = false

[workloads.pi.network]
default_deny = true
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("workload 'pi'") && err.contains("guest '/work'"),
            "error names the workload + the duplicated guest: {err}"
        );
        assert!(
            err.contains("duplicate guest paths"),
            "duplicate-guest wording: {err}"
        );
    }

    /// NESTED guest paths stay LEGAL — the spec 01 shadow-mount pattern
    /// (a read-only base plus a nested read-write shadow). Pinned so the
    /// duplicate-guest guard never over-reaches into nesting.
    #[test]
    fn validate_allows_nested_mount_guests_spec01_shadow() {
        let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[[workloads.pi.mounts]]
host = "config"
guest = "/config"
read_only = true

[[workloads.pi.mounts]]
host = "config-repos"
guest = "/config/config-repos"
read_only = false

[workloads.pi.network]
default_deny = true
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        validate_config(&config)
            .unwrap_or_else(|e| panic!("nested guests (spec 01 shadow) must validate clean: {e}"));
    }
}
