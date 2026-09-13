//! Config validation: schema-version enforcement, recipe/feature vocabulary,
//! policy allowlists, trust-boundary validators, and identifier validators.

use anyhow::Result;
use std::collections::HashMap;

use crate::config::types::ConfigFile;
use crate::config::types::{
    InstancePort, InstanceStrategy, ParameterizedIncrement, PortOccupiedStep,
};
use crate::policy;

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
/// around [`validate_identifier`] — the ONE shared identifier rule backing
/// every name gate: this create-time check, [`validate_workload_name`]
/// (`workestrate workload new`, commands/init.rs), and the load-time
/// workload-key gate in [`validate_config`]. One implementation means the
/// load-time and create-time rules cannot drift.
pub fn validate_config_name(name: &str) -> Result<()> {
    validate_identifier(name, "config name")
}

/// THE ONE shared identifier validator (crate-visible so commands/init.rs's
/// `validate_workload_name` delegates here instead of carrying a drifted
/// copy). `label` is interpolated into error messages ("workload name ...",
/// "config name ..."). Pattern: `^[a-z0-9][a-z0-9-]{0,62}$`.
pub(crate) fn validate_identifier(name: &str, label: &str) -> Result<()> {
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

/// Validate guest literals without interpreting paths on the host or expanding
/// shell/environment templates. The plan-to-SDK boundary repeats this check.
pub(crate) fn validate_init(init: &super::InitConfig) -> Result<()> {
    if let super::InitConfig::Handoff { cmd, args, env } = init {
        anyhow::ensure!(
            cmd.starts_with('/') && cmd != "/" && !cmd.contains(['\\', '\0']),
            "init.cmd must be an absolute Linux guest executable path without backslashes or NUL"
        );
        anyhow::ensure!(
            args.iter().all(|arg| !arg.contains('\0')),
            "init.args must not contain NUL"
        );
        for (name, value) in env {
            anyhow::ensure!(
                is_valid_env_var_name(name),
                "init.env has an invalid environment variable name"
            );
            anyhow::ensure!(
                !value.contains('\0'),
                "init.env values must not contain NUL"
            );
        }
    }
    Ok(())
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

/// Check the SDK's nonzero u32 MiB representation, not host free-space admission.
pub fn validate_root_disk_mib(size: u32) -> Result<()> {
    anyhow::ensure!(
        size != 0,
        "root_disk_mib must be a nonzero u32 MiB capacity"
    );
    Ok(())
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

    // Hardening (@ retrospective): workload keys are raw TOML map keys, so a
    // hand-edited layer bypasses the creation-time `validate_workload_name`
    // gate entirely. Re-run the shared identifier validator on every key at
    // load/validation time: an out-of-charset name must fail closed HERE,
    // naming the offending key and the rule, instead of leaking into
    // filesystem paths / registry keys / msb sandbox names downstream.
    for workload_name in config.workloads.keys() {
        validate_identifier(workload_name, "workload name")?;
    }

    // WP6(d)/C9: recipe/feature vocabulary hard errors. The string-valued
    // recipes are plain strings in the schema (unlike egress, whose tagged
    // enum already fails unknown variants at TOML parse), so they must be
    // validated here.
    for (workload_name, workload) in &config.workloads {
        if let Some(seconds) = workload.readiness_timeout_secs {
            anyhow::ensure!(
                (1..=3600).contains(&seconds),
                "workload '{workload_name}': readiness_timeout_secs must be in 1..=3600"
            );
            // Validate the effective workload, after partial layers inherit kind.
            anyhow::ensure!(
                workload.kind == "service",
                "workload '{workload_name}': readiness_timeout_secs requires kind = \"service\""
            );
        }
        if let Some(size) = workload.root_disk_mib {
            validate_root_disk_mib(size)
                .map_err(|e| anyhow::anyhow!("workload '{workload_name}': {e}"))?;
        }
        if let Some(init) = &workload.init {
            validate_init(init).map_err(|e| anyhow::anyhow!("workload '{workload_name}': {e}"))?;
        }
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
        if let Some(ref binary) = workload.image.binary
            && !ALLOWED_BUILD_RECIPES.contains(&binary.recipe.as_str())
        {
            anyhow::bail!(
                "workload '{}' image.binary.recipe '{}' is not a known recipe (expected one of: {})",
                workload_name,
                binary.recipe,
                ALLOWED_BUILD_RECIPES.join(", ")
            );
        }
        if let Some(ref build) = workload.local_build
            && !ALLOWED_BUILD_RECIPES.contains(&build.recipe.as_str())
        {
            anyhow::bail!(
                "workload '{}' local_build.recipe '{}' is not a known recipe (expected one of: {})",
                workload_name,
                build.recipe,
                ALLOWED_BUILD_RECIPES.join(", ")
            );
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
    }

    // ADR 0030 Phase 1: per-workload instance policy port bounds (the closed
    // vocabularies + on_conflict/on_occupied chain rules validate at parse).
    for (workload_name, workload) in &config.workloads {
        let Some(port) = &workload.instance.port else {
            continue;
        };
        match port {
            InstancePort::Strict(n) => {
                if *n == 0 {
                    anyhow::bail!(
                        "workload '{workload_name}' instance.port: strict port must be in 1..=65535 (got 0)"
                    );
                }
            }
            InstancePort::Auto => {}
            InstancePort::Preferred(p) => {
                if p.preferred == 0 {
                    anyhow::bail!(
                        "workload '{workload_name}' instance.port: preferred must be in 1..=65535 (got 0)"
                    );
                }
                let Some(chain) = &p.on_occupied else {
                    continue;
                };
                for step in &chain.0 {
                    let PortOccupiedStep::Increment(ParameterizedIncrement { increment }) = step
                    else {
                        continue;
                    };
                    match (&increment.limit, &increment.range) {
                        (Some(_), Some(_)) => anyhow::bail!(
                            "workload '{workload_name}' instance.port: increment must set exactly one of limit or range"
                        ),
                        (None, None) => anyhow::bail!(
                            "workload '{workload_name}' instance.port: increment must set exactly one of limit or range"
                        ),
                        (Some(limit), None) => {
                            if *limit == 0 {
                                anyhow::bail!(
                                    "workload '{workload_name}' instance.port: increment limit must be >= 1"
                                );
                            }
                            if u32::from(p.preferred) + u32::from(*limit) > 65535 {
                                anyhow::bail!(
                                    "workload '{workload_name}' instance.port: preferred {} + limit {} exceeds 65535",
                                    p.preferred,
                                    limit
                                );
                            }
                        }
                        (None, Some((start, end))) => {
                            if *start == 0 || *end == 0 {
                                anyhow::bail!(
                                    "workload '{workload_name}' instance.port: increment range bounds must be in 1..=65535"
                                );
                            }
                            if start > end {
                                anyhow::bail!(
                                    "workload '{workload_name}' instance.port: increment range START {} must be <= END {}",
                                    start,
                                    end
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    // ADR 0030 V-addendum §V1 validity gate: `strategy = "per-dir"` is valid
    // ONLY for workloads with a cwd-templated mount (a mount whose host is
    // `${CWD}` or `${CWD}/...` — commit ec1908e's invoke-cwd template). A
    // per-dir instance with no per-dir content is a contradiction, so this
    // fails closed at config validation rather than surprising at runtime.
    for (workload_name, workload) in &config.workloads {
        if workload.instance.strategy != InstanceStrategy::PerDir {
            continue;
        }
        let has_cwd_mount = workload
            .mounts
            .iter()
            .any(|m| m.host == "${CWD}" || m.host.starts_with("${CWD}/"));
        if !has_cwd_mount {
            anyhow::bail!(
                "workload '{}' declares instance.strategy = \"per-dir\" but has no \
                 cwd-templated mount: per-dir keys the instance on the invocation \
                 directory, so it requires a mount with host = \"${{CWD}}\" or \
                 \"${{CWD}}/...\". Add a cwd-templated mount or drop the per-dir \
                 strategy.",
                workload_name
            );
        }
    }

    // WP10/A11: `secrets.{name}.env_var` values become real environment
    // variables (read from the host and injected into the sandbox); reject
    // names no shell or `exec` could set.
    for (secret_name, secret) in &config.secrets {
        if let Some(ref env_var) = secret.env_var
            && !is_valid_env_var_name(env_var)
        {
            anyhow::bail!(
                "secret '{}' env_var '{}' is not a valid environment variable name (must match ^[A-Za-z_][A-Za-z0-9_]*$)",
                secret_name,
                env_var
            );
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

    // Network defaults are fail-closed by convention (absent = deny) but
    // need no validation gate: an explicit `egress = "allow"` /
    // `ingress = "allow"` stands alone (entitlements mechanism removed
    // 2026-09-04 — pre-release, final seals + review + plan NOTE suffice).
    // Home `final` seals still veto via the policy ladder (untouched).
    // E1 (2026-09-04): the effective `[network.defaults]` allow now rides the
    // ladder freeze walk as a synthetic lowest-rung allow-all, so a higher-rung
    // covering final deny (home `deny.all=true final=true`; coarse FIX1 parity)
    // seals the flip per `on_conflict` — sealed ⇒ deny stands, unsealed ⇒
    // unchanged; the workload's own final deny does not self-freeze.

    // ADR 0036 §4 (nested virtualization): validate-config is STATIC
    // COHERENCE ONLY — no host I/O. The closed `nested` vocabulary
    // ("off"|"prefer"|"require") is enforced at PARSE time (`NestedMode`
    // serde enum + `deny_unknown_fields` on `VirtualizationConfig` /
    // `VirtualizationPolicyFragment`, so typos and stray keys hard-error
    // before this function ever runs), exactly like the image/binary
    // recipe vocabularies above are enforced here only because they are
    // plain strings. Cross-rung freeze (home-final vs workload ask) is
    // PLAN-time, not a validate error: per-rung-legal configs validate
    // clean, `plan` reports `frozen_out` provenance, `up` refuses. Hence
    // there is deliberately NO virtualization refusal arm below.

    // Secret references must be defined in the secrets section.
    for (workload_name, workload) in &config.workloads {
        for (env_name, binding) in workload.env.iter() {
            if let crate::config::EnvBinding::Secret(ref_) = binding
                && !config.secrets.contains_key(&ref_.secret)
            {
                anyhow::bail!(
                    "workload '{}' env references undefined secret '{}'",
                    workload_name,
                    ref_.secret
                );
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
            if let Some(env) = &spec.env
                && !is_valid_env_var_name(env)
            {
                anyhow::bail!(
                    "workload '{}' depends_on '{}' env '{}' is not a valid environment variable name (must match ^[A-Za-z_][A-Za-z0-9_]*$)",
                    workload_name,
                    dep,
                    env
                );
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
        validate_seed_source, validate_seed_target, validate_seed_target_coverage,
    };
    for (workload_name, workload) in &config.workloads {
        let mut seen_guests = std::collections::HashSet::new();
        for m in &workload.mounts {
            if m.policy_file.is_some() {
                anyhow::bail!(
                    "workload '{workload_name}' mount '{}' sets policy_file; \
                     policy_file is a runtime-resolved field; it cannot be set in configuration",
                    m.guest
                );
            }
            validate_mount_host(&m.host).map_err(|e| {
                anyhow::anyhow!("workload '{workload_name}' mount host validation failed: {e}")
            })?;
            validate_mount_guest(&m.guest, m.is_read_only()).map_err(|e| {
                anyhow::anyhow!("workload '{workload_name}' mount guest validation failed: {e}")
            })?;
            if !seen_guests.insert(m.guest.clone()) {
                anyhow::bail!(
                    "workload '{workload_name}' has duplicate mount guest path '{}'; mount guest paths must be unique within a workload (per-mount policy files are keyed by guest slug)",
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
            // Host Bug C: a seed target that no declared mount host covers
            // silently renders on the HOST and never reaches the guest —
            // fail closed here (defense-in-depth also in prepare()).
            let mount_hosts: Vec<&str> = workload.mounts.iter().map(|m| m.host.as_str()).collect();
            validate_seed_target_coverage(&seed.target, &mount_hosts).map_err(|e| {
                anyhow::anyhow!(
                    "workload '{workload_name}' seed_files.target '{}' mount coverage validation failed: {e}",
                    seed.target
                )
            })?;
        }
        if let Some(build) = &workload.local_build
            && let Some(name) = &build.env_override
        {
            validate_env_override(name).map_err(|e| {
                anyhow::anyhow!(
                    "workload '{workload_name}' local_build.env_override validation failed: {e}"
                )
            })?;
        }
    }

    // credential broker: catalog integrity + grant references + SSH
    // confinement coherence. The ladders come from the process-global stores
    // populated by loading.rs BEFORE validate_config runs; synthetic/test
    // paths without a load degrade to empty ladders (strict defaults false).
    validate_credentials(
        config,
        &crate::merge::get_ssh_policy_ladder().unwrap_or_default(),
        &crate::merge::get_network_policy_ladder().unwrap_or_default(),
    )?;

    Ok(())
}

/// Whether one egress fragment carries an SSH allowance: an allow-all,
/// a host entry covering port 22, or a domain entry scoped to port 22.
/// Deny entries never count (they restrict). Protocol-agnostic by design —
/// port-22 presence is the approximation of "SSH allowance".
fn egress_fragment_covers_ssh(fragment: &crate::config::EgressPolicyFragment) -> bool {
    let Some(allow) = &fragment.allow else {
        return false;
    };
    if allow.all == Some(true) {
        return true;
    }
    if allow.host.iter().any(|h| h.ports.contains(&22)) {
        return true;
    }
    if allow.domain.iter().any(|d| d.port == Some(22)) {
        return true;
    }
    false
}

/// Whether the effective egress policy for `workload_name` carries an SSH
/// allowance: the home rung, every layer rung (global), and this workload's
/// capsule rungs.
fn effective_egress_covers_ssh(
    workload_name: &str,
    network_ladder: &crate::merge::NetworkPolicyLadder,
) -> bool {
    if network_ladder
        .egress_home
        .as_ref()
        .is_some_and(|(_, f)| egress_fragment_covers_ssh(f))
    {
        return true;
    }
    if network_ladder
        .egress_layers
        .iter()
        .any(|(_, f)| egress_fragment_covers_ssh(f))
    {
        return true;
    }
    if network_ladder
        .egress_workloads
        .get(workload_name)
        .is_some_and(|rungs| rungs.iter().any(|(_, f)| egress_fragment_covers_ssh(f)))
    {
        return true;
    }
    false
}

/// Effective SSH confinement for one workload: the `[policy.ssh]` ladder
/// rungs in authority-ascending order (home, layers, this workload's
/// capsule rungs), walked with the same final-freeze semantics as the
/// secrets ladder.
fn effective_ssh_strict(
    workload_name: &str,
    ssh_ladder: &crate::merge::SshPolicyLadder,
) -> (bool, String) {
    let mut rungs: Vec<(String, crate::config::SshPolicyFragment)> = Vec::new();
    if let Some((origin, fragment)) = &ssh_ladder.home {
        rungs.push((origin.clone(), fragment.clone()));
    }
    for (origin, fragment) in &ssh_ladder.layers {
        rungs.push((origin.clone(), fragment.clone()));
    }
    if let Some(workload_rungs) = ssh_ladder.workloads.get(workload_name) {
        for (origin, fragment) in workload_rungs {
            rungs.push((origin.clone(), fragment.clone()));
        }
    }
    let rung_refs: Vec<(&str, &crate::config::SshPolicyFragment)> =
        rungs.iter().map(|(o, f)| (o.as_str(), f)).collect();
    crate::microsandbox::workload::credentials::resolve_ssh_strict(&rung_refs)
}

/// Validate the credential-broker surface (fail-closed): catalog entries
/// must reference existing secrets and carry their required scope;
/// workload grant refs must name catalog entries; `strict` confinement
/// without any SSH grant or SSH egress allowance is a config error.
///
/// `bound = "guest"` needs NO new machinery here — it keeps the existing
/// secret-delivery semantics validated by the env/secret arms above.
fn validate_credentials(
    config: &ConfigFile,
    ssh_ladder: &crate::merge::SshPolicyLadder,
    network_ladder: &crate::merge::NetworkPolicyLadder,
) -> Result<()> {
    // Catalog: SSH entries.
    let mut ssh_names: Vec<&String> = config.credentials.ssh.keys().collect();
    ssh_names.sort();
    for name in ssh_names {
        let entry = &config.credentials.ssh[name];
        if entry.material.is_empty() {
            anyhow::bail!(
                "credential ssh '{name}' must declare material (a [secrets.<N>] entry name)"
            );
        }
        if !config.secrets.contains_key(&entry.material) {
            anyhow::bail!(
                "credential ssh '{name}' references undefined secret '{}'",
                entry.material
            );
        }
        if entry.hosts.is_empty() {
            anyhow::bail!("credential ssh '{name}' must declare hosts (at least one host)");
        }
        if entry.hosts.iter().any(|h| h.is_empty()) {
            anyhow::bail!("credential ssh '{name}' declares an empty host entry");
        }
        if entry.users.is_empty() {
            anyhow::bail!("credential ssh '{name}' must declare users (at least one user)");
        }
        if entry.users.iter().any(|u| u.is_empty()) {
            anyhow::bail!("credential ssh '{name}' declares an empty user entry");
        }
        if let Some(ports) = &entry.ports {
            if ports.is_empty() {
                anyhow::bail!(
                    "credential ssh '{name}' declares an empty ports list (omit ports for the default [22])"
                );
            }
            if ports.contains(&0) {
                anyhow::bail!(
                    "credential ssh '{name}' declares port 0 (ports must be in 1..=65535)"
                );
            }
        }
    }

    // Catalog: signing entries.
    let mut signing_names: Vec<&String> = config.credentials.signing.ssh.keys().collect();
    signing_names.sort();
    for name in signing_names {
        let entry = &config.credentials.signing.ssh[name];
        if entry.material.is_empty() {
            anyhow::bail!(
                "credential signing '{name}' must declare material (a [secrets.<N>] entry name)"
            );
        }
        if !config.secrets.contains_key(&entry.material) {
            anyhow::bail!(
                "credential signing '{name}' references undefined secret '{}'",
                entry.material
            );
        }
        if entry.namespace.is_empty() {
            anyhow::bail!("credential signing '{name}' must declare namespace");
        }
    }

    // Workload grant references must name catalog entries.
    let mut workload_names: Vec<&String> = config.workloads.keys().collect();
    workload_names.sort();
    for workload_name in workload_names {
        let workload = &config.workloads[workload_name];
        let mut ssh_refs = workload.credentials.ssh.clone();
        ssh_refs.sort();
        for grant in ssh_refs {
            if !config.credentials.ssh.contains_key(&grant) {
                anyhow::bail!(
                    "workload '{workload_name}' credentials.ssh references undefined credential '{grant}'"
                );
            }
        }
        let mut signing_refs = workload.credentials.signing.clone();
        signing_refs.sort();
        for grant in signing_refs {
            if !config.credentials.signing.ssh.contains_key(&grant) {
                anyhow::bail!(
                    "workload '{workload_name}' credentials.signing references undefined credential '{grant}'"
                );
            }
        }

        // Confinement coherence: strict=true requires at least one SSH
        // credential grant OR at least one SSH egress allowance in the same
        // effective policy — else every recognized SSH flow would fail
        // closed with no legal path, so the config itself is the error.
        let (strict, origin) = effective_ssh_strict(workload_name, ssh_ladder);
        if strict
            && workload.credentials.ssh.is_empty()
            && !effective_egress_covers_ssh(workload_name, network_ladder)
        {
            anyhow::bail!(
                "workload '{workload_name}' enables [policy.ssh] strict=true (from '{origin}') but declares no credentials.ssh grant and no SSH egress allowance (port 22): strict confinement with no legal SSH path is a config error"
            );
        }
    }

    Ok(())
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

    // ---- Hardening (@ retrospective): workload-key charset gate ----

    /// Hand-edited TOML layers bypass the creation-time
    /// `validate_workload_name` gate, so `validate_config` must re-run the
    /// shared identifier validator on every workload KEY and fail closed.
    /// Keys are always QUOTED in these fixtures (`[workloads."<key>"]`) — a
    /// literal dot in a bare key would parse as table nesting instead of
    /// part of the name.
    #[test]
    fn validate_rejects_invalid_workload_key_charset() {
        let too_long = "a".repeat(64);
        for key in ["Pi", "my_agent", "my.agent", "-pi", too_long.as_str()] {
            let toml = MINIMAL_VALID_TOML
                .replace("[workloads.pi]", &format!("[workloads.\"{key}\"]"))
                .replace(
                    "[workloads.pi.network]",
                    &format!("[workloads.\"{key}\".network]"),
                );
            let config: ConfigFile = toml::from_str(&toml).unwrap_or_else(|e| {
                panic!("key '{key}' must PARSE (the gate is validation, not deserialization): {e}")
            });
            let err = validate_config(&config).unwrap_err().to_string();
            assert!(
                err.contains(key),
                "error must name the offending key '{key}': {err}"
            );
            if key.len() > 63 {
                // The overlong branch of `validate_identifier` reports the
                // length violation without the regex; pin its own message.
                assert!(
                    err.contains("cannot exceed 63 characters"),
                    "overlong key must be rejected with the length message: {err}"
                );
            } else {
                assert!(
                    err.contains("^[a-z0-9][a-z0-9-]{0,62}$"),
                    "error must carry the rule text for key '{key}': {err}"
                );
            }
        }
    }

    /// Valid keys pass unchanged — including the 63-character DNS-label
    /// boundary — so the gate rejects only genuinely out-of-charset names.
    #[test]
    fn validate_accepts_valid_workload_keys() {
        let boundary = "a".repeat(63);
        let toml = format!(
            "schema_version = 1\n\n\
             [workloads.pi]\n\
             kind = \"agent\"\n\
             image = {{ recipe = \"registry\", ref = \"node:24\" }}\n\
             command = []\n\n\
             [workloads.pi.network.defaults]\n\
             egress = \"deny\"\n\n\
             [workloads.\"{boundary}\"]\n\
             kind = \"agent\"\n\
             image = {{ recipe = \"registry\", ref = \"node:24\" }}\n\
             command = []\n\n\
             [workloads.\"{boundary}\".network.defaults]\n\
             egress = \"deny\"\n\n\
             [workloads.example-agent-2]\n\
             kind = \"service\"\n\
             image = {{ recipe = \"registry\", ref = \"node:24\" }}\n\
             command = []\n\n\
             [workloads.example-agent-2.network.defaults]\n\
             egress = \"deny\"\n"
        );
        let config: ConfigFile = toml::from_str(&toml).expect("multi-workload config must parse");
        assert_eq!(config.workloads.len(), 3);
        validate_config(&config)
            .unwrap_or_else(|e| panic!("valid workload keys must pass the charset gate: {e}"));
    }

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
        let toml = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"nix-layered\", name = \"pi\", binary = { recipe = \"go-build\", src = \"flake://pi\" } }\ncommand = []\n\n[workloads.pi.network.defaults]\negress = \"deny\"";
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
        let toml = "schema_version = 1\n\n[workloads.prime]\nkind = \"agent\"\nimage = { recipe = \"nix-layered\", name = \"prime\", tag = \"latest\", binary = { recipe = \"bun-compile\", src = \"flake://prime\", entrypoint = \"packages/coding-agent/dist/bun/cli.js\" } }\ncommand = []\n\n[workloads.prime.network.defaults]\negress = \"deny\"";
        let config: ConfigFile = toml::from_str(toml).unwrap();
        validate_config(&config)
            .unwrap_or_else(|e| panic!("workerless bun-compile binary should validate clean: {e}"));
    }

    /// A bun-compile binary WITH a worker (pi passes its image-resize-worker.ts)
    /// must validate clean.
    #[test]
    fn validate_accepts_binary_with_worker() {
        let toml = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"nix-layered\", name = \"pi\", tag = \"latest\", binary = { recipe = \"bun-compile\", src = \"flake://pi\", entrypoint = \"packages/coding-agent/dist/bun/cli.js\", worker = \"packages/coding-agent/src/utils/image-resize-worker.ts\" } }\ncommand = []\n\n[workloads.pi.network.defaults]\negress = \"deny\"";
        let config: ConfigFile = toml::from_str(toml).unwrap();
        validate_config(&config).unwrap_or_else(|e| {
            panic!("bun-compile binary with worker should validate clean: {e}")
        });
    }

    #[test]
    fn validate_rejects_unknown_local_build_recipe() {
        let toml = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.network.defaults]\negress = \"deny\"\n\n[workloads.pi.local_build]\nrecipe = \"make\"\nsource = \"flake://pi\"";
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
        let toml = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"nix-layered\", name = \"pi\", features = [\"bogus\"] }\ncommand = []\n\n[workloads.pi.network.defaults]\negress = \"deny\"";
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
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG_DIR", &tmp) };

        let result = crate::config::load_config();

        match old {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("WORKESTRATE_CONFIG_DIR", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("WORKESTRATE_CONFIG_DIR") },
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
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_CONFIG_DIR", &tmp) };

        let result = crate::config::load_config();

        match old {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("WORKESTRATE_CONFIG_DIR", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("WORKESTRATE_CONFIG_DIR") },
        }
        let _ = std::fs::remove_dir_all(&tmp);

        result?;
        Ok(())
    }

    // ---- ADR 0030 V-addendum §V1: per-dir requires a cwd-templated mount ----

    const PER_DIR_NO_CWD_MOUNT_TOML: &str = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.instance]\nstrategy = \"per-dir\"\n\n[workloads.pi.network.defaults]\negress = \"deny\"";

    /// `strategy = "per-dir"` with NO cwd-templated mount is a hard
    /// validation error naming the workload and the remediation.
    #[test]
    fn validate_rejects_per_dir_without_cwd_mount() {
        let config: ConfigFile = toml::from_str(PER_DIR_NO_CWD_MOUNT_TOML).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("workload 'pi'") && err.contains("per-dir"),
            "error must name the workload and the strategy: {err}"
        );
        assert!(
            err.contains("${CWD}"),
            "error must name the cwd-templated mount remediation: {err}"
        );
    }

    /// A per-dir workload WITH a cwd-templated mount (`${CWD}` exact or
    /// `${CWD}/...`) validates clean; non-per-dir workloads need no such
    /// mount.
    #[test]
    fn validate_accepts_per_dir_with_cwd_mount() {
        for host in ["${CWD}", "${CWD}/sub/dir"] {
            let toml = format!(
                "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = {{ recipe = \"registry\", ref = \"node:24\" }}\ncommand = []\n\n[[workloads.pi.mounts]]\nhost = \"{host}\"\nguest = \"/work\"\n\n[workloads.pi.instance]\nstrategy = \"per-dir\"\n\n[workloads.pi.network.defaults]\negress = \"deny\""
            );
            let config: ConfigFile = toml::from_str(&toml).unwrap();
            validate_config(&config)
                .unwrap_or_else(|e| panic!("per-dir with host '{host}' must validate: {e}"));
        }
        // A non-cwd mount (state root) does NOT satisfy the gate.
        let toml = "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[[workloads.pi.mounts]]\nhost = \"workspaces/pi-state\"\nguest = \"/state\"\n\n[workloads.pi.instance]\nstrategy = \"per-dir\"\n\n[workloads.pi.network.defaults]\negress = \"deny\"";
        let config: ConfigFile = toml::from_str(toml).unwrap();
        assert!(
            validate_config(&config).is_err(),
            "a state-only mount must not satisfy the per-dir gate"
        );
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
    fn validate_config_rejects_duplicate_mount_guest_path() {
        let mut config = base_config_for_validation();
        let workload = config.workloads.get_mut("pi").unwrap();
        workload.mounts = vec![
            crate::microsandbox::plan::MountPlan {
                host: "first".to_string(),
                guest: "/data".to_string(),
                mode: crate::microsandbox::plan::MountMode::Rw,
                policy: None,
                owner: None,
                policy_file: None,
            },
            crate::microsandbox::plan::MountPlan {
                host: "second".to_string(),
                guest: "/data".to_string(),
                mode: crate::microsandbox::plan::MountMode::Rw,
                policy: None,
                owner: None,
                policy_file: None,
            },
        ];
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(err.contains("duplicate mount guest path"), "error: {err}");
        assert!(err.contains("/data"), "error: {err}");
    }

    #[test]
    fn validate_config_rejects_user_supplied_policy_file() {
        let mut config = base_config_for_validation();
        let workload = config.workloads.get_mut("pi").unwrap();
        workload.mounts = vec![crate::microsandbox::plan::MountPlan {
            host: "state".to_string(),
            guest: "/data".to_string(),
            mode: crate::microsandbox::plan::MountMode::Rw,
            policy: None,
            owner: None,
            policy_file: Some(std::path::PathBuf::from("/some/path")),
        }];
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("policy_file is a runtime-resolved field"),
            "error must explain policy_file is runtime-resolved: {err}"
        );
        assert!(err.contains("pi"), "error must name the workload: {err}");
        assert!(
            err.contains("/data"),
            "error must name the mount guest: {err}"
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

[workloads.pi.network.defaults]
egress = "deny"
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

[workloads.pi.network.defaults]
egress = "deny"
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

    // ---- Generic secret allowed-hosts (phase 4) ----

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

[workloads.example-agent.network.defaults]
egress = "deny"
"#;
        let config: ConfigFile = toml::from_str(toml).expect("config must parse");
        // ADR 0035: secret allowed_hosts no longer validated against hardcoded allowlist — any host is allowed
        validate_config(&config).expect(
            "secret with any allowed_hosts should now validate (allowlist removed per ADR 0035)",
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

[workloads.example-agent.network.defaults]
egress = "deny"
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

[workloads.example-agent.network.defaults]
egress = "deny"
"#;
        let config: ConfigFile = toml::from_str(toml).expect("config must parse");
        validate_config(&config).expect("secrets without env_var skip the allowed-hosts gate");
    }

    #[test]
    fn validate_accepts_egress_allow_standalone() {
        // Entitlements removed 2026-09-04: explicit `egress = "allow"` stands
        // alone, no magic word needed.
        let toml = r#"
schema_version = 1

[workloads.example-offensive]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.example-offensive.network.defaults]
egress = "allow"
"#;
        let config: ConfigFile = toml::from_str(toml).expect("config must parse");
        validate_config(&config).expect("standalone egress allow must validate");
    }

    #[test]
    fn validate_accepts_ingress_allow_standalone() {
        // Entitlements removed 2026-09-04: explicit `ingress = "allow"`
        // stands alone, no magic word needed.
        let toml = r#"
schema_version = 1

[workloads.example-offensive]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.example-offensive.network.defaults]
ingress = "allow"
"#;
        let config: ConfigFile = toml::from_str(toml).expect("config must parse");
        validate_config(&config).expect("standalone ingress allow must validate");
    }

    #[test]
    fn validate_rejects_unknown_entitlements_key() {
        // Free-enforcer property: the removed `entitlements` key is now an
        // unknown field and fails at parse time via `deny_unknown_fields`.
        let toml = r#"
schema_version = 1

[workloads.example-agent]
kind = "agent"
entitlements = ["default_egress_allow"]
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.example-agent.network.defaults]
egress = "deny"
"#;
        let err = toml::from_str::<ConfigFile>(toml).unwrap_err().to_string();
        assert!(
            err.contains("unknown field") && err.contains("entitlements"),
            "removed entitlements key must fail as unknown field, got: {err}"
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

[workloads.pi.network.defaults]
egress = "deny"

[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.litellm.network.defaults]
egress = "deny"
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
                on_conflict: None,
                instance: None,
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
                on_conflict: None,
                instance: None,
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

[workloads.pi.network.defaults]
egress = "deny"

[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[[workloads.litellm.ports]]
host = 4000
guest = 4000
name = "http"

[workloads.litellm.network.defaults]
egress = "deny"
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
                on_conflict: None,
                instance: None,
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
                "\n[workloads.{name}]\nkind = \"service\"\nimage = {{ recipe = \"registry\", ref = \"node:24-bookworm-slim\" }}\ncommand = []\n\n[workloads.{name}.network.defaults]\negress = \"deny\"\n"
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
    /// test. The state mount exists because seed targets must be covered by
    /// a declared mount host (host Bug C coverage rule).
    fn seed_config() -> ConfigFile {
        let toml = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = []

[[workloads.svc.mounts]]
host = "workspaces/svc-state"
guest = "/data"

[[workloads.svc.seed_files]]
source = "seed/a.json"
target = "workspaces/svc-state/a.json"

[workloads.svc.network.defaults]
egress = "deny"
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

[[workloads.svc.mounts]]
host = "workspaces/svc-state"
guest = "/data"

[[workloads.svc.seed_files]]
source = "seed/a.json"
target = "workspaces/svc-state/a.json"

[[workloads.svc.seed_files]]
glob = "seed/**/*.env"
target = "workspaces/svc-state/env"

[workloads.svc.network.defaults]
egress = "deny"
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

    // ---- Host Bug C: seed target must be covered by a declared mount host ----

    /// The EXACT regression test for the host bug: the litellm capsule
    /// seeded `app/config/config.yaml` with only a `${MSB_HOME}/...` mount —
    /// the seed silently rendered into the personal repo working tree and the
    /// guest never saw `/app/config/config.yaml`. Must now fail closed.
    #[test]
    fn validate_config_rejects_litellm_shape_uncovered_seed_target() {
        let toml = r#"
schema_version = 1

[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = []

[[workloads.litellm.mounts]]
host = "${MSB_HOME}/logs"
guest = "/logs"

[[workloads.litellm.seed_files]]
source = "workloads/litellm/config.yaml"
target = "app/config/config.yaml"
template = true

[workloads.litellm.network.defaults]
egress = "deny"
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("app/config/config.yaml"),
            "error must name the uncovered target: {err}"
        );
        assert!(
            err.contains("mount"),
            "error must mention mount coverage: {err}"
        );
    }

    /// Template hosts (`${CWD}`, `${MSB_HOME}`, ...) are runtime-resolved and
    /// can NEVER prove static coverage — they never cover a seed target.
    #[test]
    fn validate_config_rejects_seed_target_with_only_template_mount() {
        let mut config = seed_config();
        {
            let svc = config.workloads.get_mut("svc").unwrap();
            svc.mounts = vec![crate::microsandbox::plan::MountPlan {
                host: "${CWD}".to_string(),
                guest: "/work".to_string(),
                mode: crate::microsandbox::plan::MountMode::Rw,
                policy: None,
                owner: None,
                policy_file: None,
            }];
            svc.seed_files[0].target = "work/x.json".to_string();
        }
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("work/x.json") && err.contains("mount"),
            "template-only mounts must not cover a seed target: {err}"
        );
    }

    /// Content-class coverage: a content-root-relative mount host that is a
    /// component-wise prefix of a content-class target covers it.
    #[test]
    fn validate_config_accepts_content_class_seed_coverage() {
        let mut config = seed_config();
        {
            let svc = config.workloads.get_mut("svc").unwrap();
            svc.mounts = vec![crate::microsandbox::plan::MountPlan {
                host: "workloads/svc/config".to_string(),
                guest: "/app/config".to_string(),
                mode: crate::microsandbox::plan::MountMode::Ro,
                policy: None,
                owner: None,
                policy_file: None,
            }];
            svc.seed_files[0].target = "workloads/svc/config/app.json".to_string();
        }
        validate_config(&config).expect("content-class prefix mount must cover the target");
    }

    /// Classes must match: a state-class target (`workspaces/...`) is NOT
    /// covered by a content-class mount host.
    #[test]
    fn validate_config_rejects_state_target_with_only_content_mount() {
        let mut config = seed_config();
        {
            let svc = config.workloads.get_mut("svc").unwrap();
            svc.mounts = vec![crate::microsandbox::plan::MountPlan {
                host: "state/svc".to_string(),
                guest: "/data".to_string(),
                mode: crate::microsandbox::plan::MountMode::Rw,
                policy: None,
                owner: None,
                policy_file: None,
            }];
        }
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("workspaces/svc-state/a.json") && err.contains("mount"),
            "a content-class mount must not cover a state-class target: {err}"
        );
    }

    /// Component-wise exactness: `workspaces/svc-state-evil` must NOT cover
    /// `workspaces/svc-state/x.json` (string-prefix trap).
    #[test]
    fn validate_config_rejects_string_prefix_but_not_component_prefix_mount() {
        let mut config = seed_config();
        {
            let svc = config.workloads.get_mut("svc").unwrap();
            svc.mounts = vec![crate::microsandbox::plan::MountPlan {
                host: "workspaces/svc-state-evil".to_string(),
                guest: "/data".to_string(),
                mode: crate::microsandbox::plan::MountMode::Rw,
                policy: None,
                owner: None,
                policy_file: None,
            }];
            svc.seed_files[0].target = "workspaces/svc-state/x.json".to_string();
        }
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("workspaces/svc-state/x.json"),
            "a string-prefix-but-not-component-prefix mount must not cover: {err}"
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

[workloads.pi.network.defaults]
egress = "deny"
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("workload 'pi'") && err.contains("mount guest path '/work'"),
            "error names the workload + the duplicated guest: {err}"
        );
        assert!(
            err.contains("duplicate mount guest path"),
            "duplicate-guest wording (converged to policy-keying semantics): {err}"
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

[workloads.pi.network.defaults]
egress = "deny"
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        validate_config(&config)
            .unwrap_or_else(|e| panic!("nested guests (spec 01 shadow) must validate clean: {e}"));
    }

    // ---- Mount `mode` field (read_only deprecated alias) ----

    /// Minimal config wrapper for mount-mode parse tests.
    fn mount_mode_config(mount_lines: &str) -> String {
        format!(
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\n\
             image = {{ recipe = \"registry\", ref = \"node:24\" }}\ncommand = []\n\n\
             [[workloads.pi.mounts]]\nhost = \"state\"\nguest = \"/data\"\n{mount_lines}\n"
        )
    }

    fn parsed_mount_mode(toml: &str) -> crate::microsandbox::plan::MountMode {
        let config: ConfigFile = toml::from_str(toml).unwrap();
        config.workloads["pi"].mounts[0].mode
    }

    /// `mode = "ro"` and `mode = "rw"` are the accepted canonical values.
    #[test]
    fn mount_mode_ro_and_rw_parse() {
        use crate::microsandbox::plan::MountMode;
        assert_eq!(
            parsed_mount_mode(&mount_mode_config("mode = \"ro\"")),
            MountMode::Ro
        );
        assert_eq!(
            parsed_mount_mode(&mount_mode_config("mode = \"rw\"")),
            MountMode::Rw
        );
    }

    /// An omitted `mode` defaults to rw.
    #[test]
    fn mount_mode_omitted_defaults_to_rw() {
        assert_eq!(
            parsed_mount_mode(&mount_mode_config("")),
            crate::microsandbox::plan::MountMode::Rw
        );
    }

    /// The deprecated `read_only` alias normalizes into `mode`:
    /// `read_only = true` ≡ "ro", `read_only = false` ≡ "rw".
    #[test]
    fn mount_read_only_alias_maps_to_mode() {
        use crate::microsandbox::plan::MountMode;
        assert_eq!(
            parsed_mount_mode(&mount_mode_config("read_only = true")),
            MountMode::Ro,
            "read_only = true must normalize to mode ro"
        );
        assert_eq!(
            parsed_mount_mode(&mount_mode_config("read_only = false")),
            MountMode::Rw,
            "read_only = false must normalize to mode rw"
        );
    }

    /// Both fields set and AGREEING → accepted, resolving to that mode
    /// (a single deprecation warning for `read_only` goes to stderr).
    #[test]
    fn mount_mode_and_agreeing_read_only_alias_accepted() {
        use crate::microsandbox::plan::MountMode;
        assert_eq!(
            parsed_mount_mode(&mount_mode_config("mode = \"ro\"\nread_only = true")),
            MountMode::Ro
        );
        assert_eq!(
            parsed_mount_mode(&mount_mode_config("mode = \"rw\"\nread_only = false")),
            MountMode::Rw
        );
    }

    /// Both fields set and CONFLICTING → hard deserialization error naming
    /// BOTH fields and both values.
    #[test]
    fn mount_mode_conflicting_read_only_alias_is_a_hard_error() {
        for (toml, mode, read_only) in [
            ("mode = \"rw\"\nread_only = true", "rw", "true"),
            ("mode = \"ro\"\nread_only = false", "ro", "false"),
        ] {
            let err = toml::from_str::<ConfigFile>(&mount_mode_config(toml))
                .unwrap_err()
                .to_string();
            assert!(
                err.contains("read_only"),
                "conflict error must name `read_only`: {err}"
            );
            assert!(
                err.contains("mode"),
                "conflict error must name `mode`: {err}"
            );
            assert!(
                err.contains(&format!("read_only = {read_only}")),
                "conflict error must name the read_only value: {err}"
            );
            assert!(
                err.contains(&format!("mode = \"{mode}\"")),
                "conflict error must name the mode value: {err}"
            );
        }
    }

    /// An unknown `mode` value is rejected with a useful error (closed
    /// vocabulary: only "ro" / "rw").
    #[test]
    fn mount_mode_unknown_value_rejected() {
        let err = toml::from_str::<ConfigFile>(&mount_mode_config("mode = \"rx\""))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("rx") && err.contains("ro") && err.contains("rw"),
            "unknown mode must be rejected naming the expected variants: {err}"
        );
    }

    /// `mode` serializes as the canonical string form; the deprecated
    /// `read_only` alias is NEVER serialized.
    #[test]
    fn mount_plan_serializes_mode_and_never_read_only() {
        use crate::microsandbox::plan::{MountMode, MountPlan};
        let mount = MountPlan {
            host: "state".to_string(),
            guest: "/data".to_string(),
            mode: MountMode::Ro,
            policy: None,
            owner: None,
            policy_file: None,
        };
        let json = serde_json::to_value(&mount).unwrap();
        assert_eq!(json["mode"], serde_json::json!("ro"));
        assert!(
            json.get("read_only").is_none(),
            "read_only must never be serialized: {json}"
        );
        // And the canonical JSON round-trips without a deprecation warning.
        let reparsed: MountPlan = serde_json::from_value(json).unwrap();
        assert_eq!(reparsed, mount);
    }

    #[test]
    fn mount_owner_pair_round_trips_and_omission_is_unchanged() {
        use crate::microsandbox::plan::{MountOwner, MountPlan};
        let old = r#"{"host":"state","guest":"/data","mode":"rw"}"#;
        let absent: MountPlan = serde_json::from_str(old).unwrap();
        assert_eq!(absent.owner, None);
        assert_eq!(serde_json::to_string(&absent).unwrap(), old);
        for (uid, gid) in [(0, 0), (61040, 61040), (7, 42), (u32::MAX, u32::MAX)] {
            let config: ConfigFile = toml::from_str(&mount_mode_config(&format!(
                "owner = {{ uid = {uid}, gid = {gid} }}"
            )))
            .unwrap();
            let mount = &config.workloads["pi"].mounts[0];
            assert_eq!(mount.owner, Some(MountOwner { uid, gid }));
            let json = serde_json::to_value(mount).unwrap();
            assert_eq!(json["owner"], serde_json::json!({"uid": uid, "gid": gid}));
            assert_eq!(serde_json::from_value::<MountPlan>(json).unwrap(), *mount);
        }
    }

    #[test]
    fn mount_owner_rejects_incomplete_ambiguous_and_out_of_range_forms() {
        for owner in [
            "{}",
            "{ uid = 0 }",
            "{ gid = 0 }",
            "{ uid = -1, gid = 0 }",
            "{ uid = 0, gid = 4294967296 }",
            "{ uid = 4294967296, gid = 0 }",
            "{ uid = 0.5, gid = 0 }",
            "{ uid = false, gid = 0 }",
            "{ uid = \"root\", gid = 0 }",
            "{ uid = 0, gid = 0, user = \"root\" }",
            "\"0:0\"",
            "[0, 0]",
        ] {
            let text = mount_mode_config(&format!("owner = {owner}"));
            assert!(toml::from_str::<ConfigFile>(&text).is_err(), "{owner}");
        }
        let duplicate = mount_mode_config("owner = { uid = 0, uid = 1, gid = 0 }");
        assert!(toml::from_str::<ConfigFile>(&duplicate).is_err());
        for owner in [
            "{}",
            r#"{"uid":0}"#,
            r#"{"gid":0}"#,
            r#"{"uid":-1,"gid":0}"#,
            r#"{"uid":0,"gid":4294967296}"#,
            r#"{"uid":4294967296,"gid":0}"#,
            r#"{"uid":0.5,"gid":0}"#,
            r#"{"uid":false,"gid":0}"#,
            r#"{"uid":"root","gid":0}"#,
            r#"{"uid":0,"gid":0,"user":"root"}"#,
            r#"{"uid":0,"uid":1,"gid":0}"#,
            r#"{"uid":0,"gid":0,"gid":1}"#,
            r#""0:0""#,
            "[0,0]",
        ] {
            let text = format!(r#"{{"host":"state","guest":"/data","owner":{owner}}}"#);
            assert!(
                serde_json::from_str::<crate::microsandbox::plan::MountPlan>(&text).is_err(),
                "{owner}"
            );
        }
    }

    #[test]
    fn mount_owner_tracks_whole_array_layer_replacement() {
        use crate::merge::{Layer, merge_layers};
        for owner in ["owner = { uid = 0, gid = 0 }", ""] {
            let base = Layer::from_string(
                "base",
                &mount_mode_config("owner = { uid = 61040, gid = 61040 }"),
            )
            .unwrap();
            let overlay = Layer::from_string("overlay", &mount_mode_config(owner)).unwrap();
            let (merged, provenance) = merge_layers(&[base, overlay]).unwrap();
            let expected: ConfigFile = toml::from_str(&mount_mode_config(owner)).unwrap();
            assert_eq!(
                merged.workloads["pi"].mounts,
                expected.workloads["pi"].mounts
            );
            assert_eq!(provenance.get("workloads.pi.mounts").unwrap(), "overlay");
        }
    }

    // ---- Mount-policy sugar (read/write axis sub-tables on [[mounts]]) ----

    /// Parse a one-workload config and return the normalized `policy`
    /// fragment of its first mount (sugar + policy table combined).
    fn parsed_mount_policy(toml: &str) -> crate::mount_policy::MountsFragment {
        let config: ConfigFile = toml::from_str(toml).unwrap();
        config.workloads["pi"].mounts[0]
            .policy
            .clone()
            .expect("sugar/policy table must normalize into Some(policy)")
    }

    /// Sugar-only form: `read`/`write` axis sub-tables directly on the
    /// mount row normalize into the row's `policy` fragment; bare strings
    /// are relaxable (the compact-form default, `final = false`).
    #[test]
    fn mount_policy_sugar_only_parse_normalizes_into_policy() {
        let fragment = parsed_mount_policy(&mount_mode_config(
            "read.deny = [\".env\", { pattern = \".git/\", final = true }]\n\
             read.allow = [\".env.example\"]\n\
             write.deny = [\"*.key\"]",
        ));
        let read = fragment.read.expect("read sugar must build the read axis");
        assert_eq!(read.deny.len(), 2);
        assert_eq!(read.deny[0].value, ".env");
        assert!(!read.deny[0].terminal, "bare string = relaxable");
        assert_eq!(read.deny[1].value, ".git/");
        assert!(read.deny[1].terminal);
        assert_eq!(read.allow.len(), 1);
        assert_eq!(read.allow[0].value, ".env.example");
        let write = fragment
            .write
            .expect("write.deny sugar must build the write axis");
        assert!(write.allow.is_empty());
        assert_eq!(write.deny.len(), 1);
        assert_eq!(write.deny[0].value, "*.key");
        assert!(!write.deny[0].terminal);
    }

    /// The removed sugar words (`mask`/`unmask`/`protect`/`writes_deny`) are
    /// hard unknown-field errors on the mount row — no aliases, no shims.
    #[test]
    fn removed_sugar_words_are_unknown_field_errors() {
        for lines in [
            "mask = [\".env\"]",
            "unmask = [\".env\"]",
            "protect = [\".env\"]",
            "writes_deny = [\".env\"]",
        ] {
            let err = toml::from_str::<ConfigFile>(&mount_mode_config(lines))
                .unwrap_err()
                .to_string();
            assert!(
                err.contains("unknown field"),
                "{lines:?} must be a hard unknown-field error: {err}"
            );
        }
    }

    /// Policy-table-only form: no sugar → the fragment parses exactly as
    /// before (byte-compat posture: sugar absent = nothing changes).
    #[test]
    fn mount_policy_table_only_parse_unchanged() {
        let fragment = parsed_mount_policy(&mount_mode_config(
            "policy.read.deny = [\".env\"]\n\
             policy.read.allow = [{ pattern = \".env.example\", final = true }]",
        ));
        let read = fragment.read.expect("read axis");
        assert_eq!(read.deny.len(), 1);
        assert_eq!(read.deny[0].value, ".env");
        assert_eq!(read.allow.len(), 1);
        assert!(read.allow[0].terminal);
        assert!(fragment.write.is_none());
    }

    /// A mount with NEITHER sugar nor a policy table keeps `policy: None`.
    #[test]
    fn mount_without_policy_or_sugar_keeps_policy_none() {
        let config: ConfigFile = toml::from_str(&mount_mode_config("")).unwrap();
        assert!(config.workloads["pi"].mounts[0].policy.is_none());
    }

    /// Mixed form: sugar concatenates with an explicitly-declared `policy`
    /// table on the same mount — both forms land in the normalized fragment
    /// (policy-table entries first, sugar appended; per-scope compile groups
    /// read.deny-before-read.allow, so the order is semantics-free).
    #[test]
    fn mount_policy_sugar_concatenates_with_policy_table() {
        let fragment = parsed_mount_policy(&mount_mode_config(
            "policy.read.deny = [\"table-deny\"]\n\
             policy.write.deny = [\"table-write-deny\"]\n\
             read.deny = [\"sugar-deny\"]\n\
             write.deny = [\"sugar-write-deny\"]",
        ));
        let denies: Vec<&str> = fragment
            .read
            .as_ref()
            .expect("read axis")
            .deny
            .iter()
            .map(|e| e.value.as_str())
            .collect();
        assert_eq!(denies, ["table-deny", "sugar-deny"]);
        let write_denies: Vec<&str> = fragment
            .write
            .as_ref()
            .expect("write axis")
            .deny
            .iter()
            .map(|e| e.value.as_str())
            .collect();
        assert_eq!(write_denies, ["table-write-deny", "sugar-write-deny"]);
    }

    /// The final flag rides the sugar's expanded form
    /// (`{ pattern, final = true }`), same as the policy table.
    #[test]
    fn mount_policy_sugar_final_flag_via_expanded_form() {
        let fragment = parsed_mount_policy(&mount_mode_config(
            "read.deny = [{ pattern = \".env\", final = true }]",
        ));
        assert!(
            fragment.read.expect("read axis").deny[0].terminal,
            "expanded-form final = true must land as a terminal value"
        );
    }

    /// Unknown fields inside a sugar entry's inline table are rejected
    /// verbatim (the `deny_unknown_fields` posture of the policy-table form
    /// is preserved) — including the removed `overridable` key.
    #[test]
    fn mount_policy_sugar_unknown_field_rejected() {
        for entry in [
            "{ pattern = \".env\", bogus = 1 }",
            "{ pattern = \".env\", overridable = false }",
        ] {
            let err =
                toml::from_str::<ConfigFile>(&mount_mode_config(&format!("read.deny = [{entry}]")))
                    .unwrap_err()
                    .to_string();
            assert!(
                err.contains("unknown field"),
                "unknown-field error must surface verbatim for {entry}: {err}"
            );
        }
    }

    /// Compile the normalized mount-entry policy of a parsed mount row,
    /// matching how the loader collects MountEntry scopes.
    fn compile_mount_entry_policy(
        toml: &str,
    ) -> Result<crate::mount_policy::MountPolicyProgram, crate::mount_policy::CompileError> {
        let fragment = parsed_mount_policy(toml);
        let scope = crate::mount_policy::PolicyScope::new(
            crate::mount_policy::ScopeKind::MountEntry,
            "test-layer",
            std::path::PathBuf::from("test/workestrate.toml"),
            fragment,
        )
        .for_mount("/data");
        crate::mount_policy::compile(vec![scope])
    }

    /// Invalid patterns are rejected with the offending value named, at the
    /// same point the policy-table form validates (compile time — fragments
    /// hold raw strings so the compiler can name the declaring origin).
    #[test]
    fn mount_policy_sugar_invalid_pattern_rejected_at_compile() {
        let err = compile_mount_entry_policy(&mount_mode_config("read.deny = [\"/etc/passwd\"]"))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("/etc/passwd") && err.contains("absolute"),
            "pattern rejection must name the value and reason: {err}"
        );
        assert!(
            err.contains("test-layer"),
            "pattern rejection must name the declaring origin: {err}"
        );
    }

    /// Trust rules are unchanged for sugar: a final read.allow from the
    /// (non-operator) mount-entry scope is rejected at compile.
    #[test]
    fn mount_policy_sugar_terminal_read_allow_rejected_for_non_operator() {
        let err = compile_mount_entry_policy(&mount_mode_config(
            "read.allow = [{ pattern = \".env\", final = true }]",
        ))
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("final read.allow") && err.contains(".env"),
            "final read.allow rejection must name the axis and pattern: {err}"
        );
    }

    /// A final read.deny from the (non-operator) mount-entry scope is
    /// ACCEPTED: denying visibility is the fail-closed direction (spec 22
    /// §5), so it compiles to a terminal Mask rule in the rules array — NOT
    /// the protect bucket (protect routing is operator-only).
    #[test]
    fn mount_policy_sugar_terminal_read_deny_accepted_for_non_operator() {
        let program = compile_mount_entry_policy(&mount_mode_config(
            "read.deny = [{ pattern = \".secret\", final = true }]",
        ))
        .unwrap_or_else(|err| {
            panic!("final read.deny from a non-operator scope must compile: {err}")
        });
        assert!(
            program.protect.is_empty(),
            "protect routing is operator-only"
        );
        assert_eq!(program.rules.len(), 1);
        assert_eq!(
            program.rules[0].effect,
            crate::mount_policy::RuleEffect::Mask
        );
        assert!(program.rules[0].is_terminal());
    }

    // ---- ADR 0030 Phase 1: instance policy port bounds ----

    /// A strict port of 0 is rejected (ports are 1..=65535).
    #[test]
    fn instance_port_strict_zero_rejected() {
        let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = 0
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("strict port must be in 1..=65535"),
            "strict 0 must be rejected with the bound message: {err}"
        );
    }

    /// A preferred port of 0 is rejected.
    #[test]
    fn instance_port_preferred_zero_rejected() {
        let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 0 }
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("preferred must be in 1..=65535"),
            "preferred 0 must be rejected: {err}"
        );
    }

    /// A relative increment with `limit = 0` is rejected (>= 1).
    #[test]
    fn instance_port_increment_limit_zero_rejected() {
        let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = { increment = { limit = 0 } } }
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("increment limit must be >= 1"),
            "limit 0 must be rejected: {err}"
        );
    }

    /// `preferred + limit` overflowing 65535 is rejected (u32 math).
    #[test]
    fn instance_port_increment_preferred_plus_limit_exceeds_rejected() {
        let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 65530, on_occupied = { increment = { limit = 100 } } }
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("exceeds 65535"),
            "preferred + limit overflow must be rejected: {err}"
        );
    }

    /// An absolute range starting at 0 is rejected.
    #[test]
    fn instance_port_increment_range_start_zero_rejected() {
        let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = { increment = { range = [0, 100] } } }
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("range bounds must be in 1..=65535"),
            "range start 0 must be rejected: {err}"
        );
    }

    /// A range with START > END is rejected.
    #[test]
    fn instance_port_increment_range_start_gt_end_rejected() {
        let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = { increment = { range = [5000, 4999] } } }
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("START 5000 must be <= END 4999"),
            "START > END must be rejected with the bound message: {err}"
        );
    }

    /// Setting BOTH limit and range is rejected (exactly one).
    #[test]
    fn instance_port_increment_both_limit_and_range_rejected() {
        let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = { increment = { limit = 1, range = [1, 2] } } }
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("exactly one of limit or range"),
            "both-fields increment must be rejected: {err}"
        );
    }

    /// Setting NEITHER limit nor range is rejected (exactly one).
    #[test]
    fn instance_port_increment_neither_rejected() {
        let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = { increment = {} } }
"#;
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("exactly one of limit or range"),
            "neither-fields increment must be rejected: {err}"
        );
    }

    /// Every legal port form validates clean: strict, auto, preferred with a
    /// chain, and a range whose START == END is legal.
    #[test]
    fn instance_port_valid_cases_pass() {
        for toml in [
            r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = 4000
"#,
            r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = "auto"
"#,
            r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = ["increment", "auto"] }
"#,
            r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = { increment = { range = [5000, 5000] } } }
"#,
        ] {
            let config: ConfigFile = toml::from_str(toml).unwrap();
            validate_config(&config)
                .unwrap_or_else(|e| panic!("valid instance port form must pass: {e}"));
        }
    }

    /// Values outside u16 fail at PARSE (the range fields are typed u16) —
    /// the addendum's `[65536, 70000]` case never reaches validation.
    #[test]
    fn instance_port_range_65536_parse_rejected() {
        let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.instance]
port = { preferred = 4000, on_occupied = { increment = { range = [65536, 70000] } } }
"#;
        assert!(
            toml::from_str::<ConfigFile>(toml).is_err(),
            "out-of-u16 range bounds must be a parse error"
        );
    }

    // ---- ADR 0036: virtualization is static-coherent by construction ----

    /// Every per-rung-legal virtualization shape validates clean — including
    /// the cross-rung freeze (home-final ban + workload require): freeze is
    /// plan-time (`frozen_out`), NOT a validate error (ADR 0036 §4).
    #[test]
    fn validate_accepts_virtualization_shapes() {
        for toml in [
            // require ask, no home seal.
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.virtualization]\nnested = \"require\"\n",
            // prefer ask.
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.virtualization]\nnested = \"prefer\"\n",
            // explicit off.
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.virtualization]\nnested = \"off\"\n",
            // home-final ban + workload require: per-rung legal, validates
            // clean (plan reports frozen_out, up refuses).
            "schema_version = 1\n\n[policy.virtualization]\nallow_nested = false\nfinal = true\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.virtualization]\nnested = \"require\"\n",
            // home grant + workload ask.
            "schema_version = 1\n\n[policy.virtualization]\nallow_nested = true\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.virtualization]\nnested = \"prefer\"\n",
        ] {
            let config: ConfigFile = toml::from_str(toml).unwrap();
            validate_config(&config)
                .unwrap_or_else(|e| panic!("legal virtualization shape must pass: {e}"));
        }
    }

    /// Typo/unknown-variant fixtures fail at PARSE with the closed-vocab
    /// error (ADR 0036 §4) — validate-config never sees them.
    #[test]
    fn validate_virtualization_typos_are_parse_errors() {
        for toml in [
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.virtualization]\nnested = \"on\"\n",
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.virtualization]\nnested = \"require\"\nrequire_device = true\n",
        ] {
            let err = toml::from_str::<ConfigFile>(toml).unwrap_err().to_string();
            assert!(
                err.contains("unknown variant") || err.contains("unknown field"),
                "virtualization typo must fail closed-vocab at parse (ADR 0036): {err}"
            );
        }
    }

    // ---- credential broker: deny_unknown_fields ----

    /// `users`/`ports` on a SIGNING entry are a parse error (signing
    /// entries carry namespace, never users/ports) — as is any unknown
    /// field on any credentials surface (`via`/`recipe`/`bound` included:
    /// there is no parallel config language).
    #[test]
    fn credentials_deny_unknown_fields_everywhere() {
        let base = "schema_version = 1\n\n[secrets.DEPLOY_KEY]\n";
        for (label, fragment) in [
            (
                "signing users",
                "[credentials.signing.ssh.rel]\nmaterial = \"DEPLOY_KEY\"\nnamespace = \"release\"\nusers = [\"root\"]\n",
            ),
            (
                "signing ports",
                "[credentials.signing.ssh.rel]\nmaterial = \"DEPLOY_KEY\"\nnamespace = \"release\"\nports = [22]\n",
            ),
            (
                "signing via",
                "[credentials.signing.ssh.rel]\nmaterial = \"DEPLOY_KEY\"\nnamespace = \"release\"\nvia = \"proxy\"\n",
            ),
            (
                "ssh via",
                "[credentials.ssh.deploy]\nmaterial = \"DEPLOY_KEY\"\nhosts = [\"github.com\"]\nusers = [\"git\"]\nvia = \"proxy\"\n",
            ),
            (
                "ssh recipe",
                "[credentials.ssh.deploy]\nmaterial = \"DEPLOY_KEY\"\nhosts = [\"github.com\"]\nusers = [\"git\"]\nrecipe = \"https\"\n",
            ),
            (
                "ssh bound",
                "[credentials.ssh.deploy]\nmaterial = \"DEPLOY_KEY\"\nhosts = [\"github.com\"]\nusers = [\"git\"]\nbound = \"guest\"\n",
            ),
            (
                "workload credentials unknown",
                "[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.credentials]\nssh = [\"deploy\"]\nvia = [\"x\"]\n",
            ),
            (
                "policy.ssh unknown",
                "[policy.ssh]\nstrict = true\nrecipe = \"https\"\n",
            ),
        ] {
            let toml = format!("{base}{fragment}");
            let err = toml::from_str::<ConfigFile>(&toml).unwrap_err().to_string();
            assert!(
                err.contains("unknown field"),
                "{label} must fail deny_unknown_fields at parse: {err}"
            );
        }
    }

    /// A fully-populated, legal credentials surface parses AND validates
    /// clean (the new deny rules must not reject any known field).
    #[test]
    fn credentials_accepts_all_known_fields() {
        let toml = "schema_version = 1\n\n[secrets.DEPLOY_KEY]\n[secrets.SIGN_KEY]\n\n[credentials.ssh.deploy]\nmaterial = \"DEPLOY_KEY\"\nhosts = [\"github.com\"]\nusers = [\"git\"]\nports = [22, 2222]\n\n[credentials.signing.ssh.rel]\nmaterial = \"SIGN_KEY\"\nnamespace = \"release\"\non_violation = \"block\"\n\n[policy.ssh]\nstrict = true\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.credentials]\nssh = [\"deploy\"]\nsigning = [\"rel\"]\n\n[[workloads.pi.policy.egress.allow.host]]\nports = [22]\nprotocols = [\"tcp\"]\n\n[workloads.pi.network.defaults]\negress = \"deny\"\n";
        let config: ConfigFile = toml::from_str(toml).unwrap();
        // The [policy.ssh] rung is collected by loading.rs (never merged),
        // so ladder-less validate_config sees strict=false here — the grant
        // and catalog arms are what this exercises (the strict arm is
        // covered with explicit ladders below).
        validate_config(&config)
            .unwrap_or_else(|e| panic!("legal credentials surface must pass: {e}"));
    }

    // ---- credential broker: material + grant validation ----

    /// A material reference to a missing secret fails closed naming BOTH
    /// the credential and the secret (ssh and signing alike).
    #[test]
    fn credentials_material_missing_secret_names_both() {
        for (credential, kind) in [
            (
                "[credentials.ssh.deploy]\nmaterial = \"MISSING_KEY\"\nhosts = [\"github.com\"]\nusers = [\"git\"]\n",
                "ssh",
            ),
            (
                "[credentials.signing.ssh.rel]\nmaterial = \"MISSING_KEY\"\nnamespace = \"release\"\n",
                "signing",
            ),
        ] {
            let toml = format!(
                "schema_version = 1\n\n[secrets.DEPLOY_KEY]\n{credential}\n[workloads.pi]\nkind = \"agent\"\nimage = {{ recipe = \"registry\", ref = \"node:24\" }}\ncommand = []\n\n[workloads.pi.network.defaults]\negress = \"deny\"\n"
            );
            let config: ConfigFile = toml::from_str(&toml).unwrap();
            let err = validate_config(&config).unwrap_err().to_string();
            assert!(
                err.contains("MISSING_KEY"),
                "{kind} material error must name the secret: {err}"
            );
            assert!(
                err.contains("deploy") || err.contains("rel"),
                "{kind} material error must name the credential: {err}"
            );
        }
    }

    /// SSH entries without material/hosts/users (or with empty/zero ports)
    /// fail closed at validation naming the credential and the constraint;
    /// signing entries without material/namespace fail the same way.
    #[test]
    fn credentials_scope_requirements_fail_closed() {
        let base = "schema_version = 1\n\n[secrets.DEPLOY_KEY]\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.network.defaults]\negress = \"deny\"\n";
        for (label, fragment, keyword) in [
            (
                "missing material",
                "[credentials.ssh.deploy]\nhosts = [\"github.com\"]\nusers = [\"git\"]\n",
                "material",
            ),
            (
                "missing hosts",
                "[credentials.ssh.deploy]\nmaterial = \"DEPLOY_KEY\"\nusers = [\"git\"]\n",
                "hosts",
            ),
            (
                "missing users",
                "[credentials.ssh.deploy]\nmaterial = \"DEPLOY_KEY\"\nhosts = [\"github.com\"]\n",
                "users",
            ),
            (
                "port zero",
                "[credentials.ssh.deploy]\nmaterial = \"DEPLOY_KEY\"\nhosts = [\"github.com\"]\nusers = [\"git\"]\nports = [0]\n",
                "port 0",
            ),
            (
                "empty ports",
                "[credentials.ssh.deploy]\nmaterial = \"DEPLOY_KEY\"\nhosts = [\"github.com\"]\nusers = [\"git\"]\nports = []\n",
                "ports",
            ),
            (
                "missing namespace",
                "[credentials.signing.ssh.rel]\nmaterial = \"DEPLOY_KEY\"\n",
                "namespace",
            ),
            (
                "signing missing material",
                "[credentials.signing.ssh.rel]\nnamespace = \"release\"\n",
                "material",
            ),
        ] {
            let toml = format!("{base}{fragment}");
            let config: ConfigFile = toml::from_str(&toml).unwrap_or_else(|e| {
                panic!("{label} must parse (required-ness is validation-level): {e}")
            });
            let err = validate_config(&config).unwrap_err().to_string();
            assert!(
                err.contains(keyword),
                "{label} must fail closed at validation naming the constraint: {err}"
            );
        }
    }

    /// Workload grant refs to unknown catalog entries fail closed naming
    /// the workload and the credential.
    #[test]
    fn workload_grant_refs_must_exist() {
        let toml = "schema_version = 1\n\n[secrets.DEPLOY_KEY]\n\n[credentials.ssh.deploy]\nmaterial = \"DEPLOY_KEY\"\nhosts = [\"github.com\"]\nusers = [\"git\"]\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.credentials]\nssh = [\"ghost\"]\n\n[workloads.pi.network.defaults]\negress = \"deny\"\n";
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("'pi'") && err.contains("ghost"),
            "unknown grant must name workload and credential: {err}"
        );

        let toml = "schema_version = 1\n\n[secrets.DEPLOY_KEY]\n\n[credentials.ssh.deploy]\nmaterial = \"DEPLOY_KEY\"\nhosts = [\"github.com\"]\nusers = [\"git\"]\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.credentials]\nsigning = [\"ghost-sign\"]\n\n[workloads.pi.network.defaults]\negress = \"deny\"\n";
        let config: ConfigFile = toml::from_str(toml).unwrap();
        let err = validate_config(&config).unwrap_err().to_string();
        assert!(
            err.contains("'pi'") && err.contains("ghost-sign"),
            "unknown signing grant must name workload and credential: {err}"
        );
    }

    // ---- credential broker: strict confinement coherence ----

    fn ssh_ladder_with(strict: Option<bool>, origin: &str) -> crate::merge::SshPolicyLadder {
        crate::merge::SshPolicyLadder {
            home: None,
            layers: vec![(
                origin.to_string(),
                crate::config::SshPolicyFragment {
                    strict,
                    r#final: false,
                },
            )],
            workloads: Default::default(),
        }
    }

    fn strict_test_config() -> ConfigFile {
        toml::from_str(
            "schema_version = 1\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.network.defaults]\negress = \"deny\"\n",
        )
        .unwrap()
    }

    /// strict=true with NEITHER an ssh grant NOR an ssh egress allowance is
    /// a config error naming the workload.
    #[test]
    fn strict_without_grants_or_egress_is_rejected() {
        let config = strict_test_config();
        let ssh_ladder = ssh_ladder_with(Some(true), "team");
        let err = validate_credentials(
            &config,
            &ssh_ladder,
            &crate::merge::NetworkPolicyLadder::default(),
        )
        .unwrap_err()
        .to_string();
        assert!(
            err.contains("'pi'") && err.contains("strict=true"),
            "strict-without-grants must name workload and confinement: {err}"
        );
    }

    /// strict=true with an ssh grant passes; strict=false (or absent) never
    /// fires — even with no grants at all.
    #[test]
    fn strict_with_grant_or_relaxed_passes() {
        // Grant path.
        let config: ConfigFile = toml::from_str(
            "schema_version = 1\n\n[secrets.DEPLOY_KEY]\n\n[credentials.ssh.deploy]\nmaterial = \"DEPLOY_KEY\"\nhosts = [\"github.com\"]\nusers = [\"git\"]\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.credentials]\nssh = [\"deploy\"]\n\n[workloads.pi.network.defaults]\negress = \"deny\"\n",
        )
        .unwrap();
        validate_credentials(
            &config,
            &ssh_ladder_with(Some(true), "team"),
            &crate::merge::NetworkPolicyLadder::default(),
        )
        .unwrap_or_else(|e| panic!("strict with a grant must pass: {e}"));

        // Relaxed path: no grants, no strict.
        let config = strict_test_config();
        validate_credentials(
            &config,
            &crate::merge::SshPolicyLadder::default(),
            &crate::merge::NetworkPolicyLadder::default(),
        )
        .unwrap_or_else(|e| panic!("relaxed confinement must pass: {e}"));
    }

    /// strict=true with a port-22 egress allowance (host entry, domain
    /// entry, or allow-all) passes without any grant.
    #[test]
    fn strict_with_ssh_egress_allowance_passes() {
        let config = strict_test_config();
        let ssh_ladder = ssh_ladder_with(Some(true), "team");
        for (label, fragment) in [
            (
                "host port 22",
                crate::config::EgressPolicyFragment {
                    allow: Some(crate::config::EgressAllowTable {
                        host: vec![crate::config::HostEntry {
                            ports: vec![22],
                            protocols: vec!["tcp".to_string()],
                            r#final: false,
                        }],
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ),
            (
                "domain port 22",
                crate::config::EgressPolicyFragment {
                    allow: Some(crate::config::EgressAllowTable {
                        domain: vec![crate::config::DomainEntry {
                            domains: vec!["github.com".to_string()],
                            port: Some(22),
                            ..Default::default()
                        }],
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ),
            (
                "allow-all",
                crate::config::EgressPolicyFragment {
                    allow: Some(crate::config::EgressAllowTable {
                        all: Some(true),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ),
        ] {
            let network_ladder = crate::merge::NetworkPolicyLadder {
                egress_layers: vec![("team".to_string(), fragment)],
                ..Default::default()
            };
            validate_credentials(&config, &ssh_ladder, &network_ladder)
                .unwrap_or_else(|e| panic!("strict with {label} allowance must pass: {e}"));
        }
    }

    // ---- credential broker: bound="guest" backward compat ----

    /// A `bound = "guest"` env consumption keeps the EXISTING secret
    /// semantics untouched when credentials are present: the guest-bound
    /// real-value plan entry still resolves, and the catalog entry it
    /// shares material with does not disturb it.
    #[test]
    fn guest_binding_keeps_existing_secret_semantics() {
        let toml = "schema_version = 1\n\n[secrets.DEPLOY_KEY]\nallowed_hosts = [\"github.com\"]\n\n[secrets.PLAIN]\n\n[credentials.ssh.deploy]\nmaterial = \"DEPLOY_KEY\"\nhosts = [\"github.com\"]\nusers = [\"git\"]\n\n[workloads.pi]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24\" }\ncommand = []\n\n[workloads.pi.credentials]\nssh = [\"deploy\"]\n\n[workloads.pi.env]\nSSH_KEY = { secret = \"DEPLOY_KEY\", bound = \"guest\" }\nPLAIN = true\n\n[workloads.pi.network.defaults]\negress = \"deny\"\n";
        let config: ConfigFile = toml::from_str(toml).unwrap();
        validate_config(&config)
            .unwrap_or_else(|e| panic!("guest-bound consumption must validate: {e}"));
        // The pre-existing env/secret plan path resolves identically with
        // the catalog present: one real-value is-secret env entry (guest)
        // plus the same-name host-bound placeholder entry.
        let secrets =
            crate::microsandbox::workload::secrets::build_secret_definitions(&config).unwrap();
        let workload = config.workloads.get("pi").unwrap();
        let (env, secret_env) =
            crate::microsandbox::workload::secrets::build_env_and_secret_env(workload, &secrets)
                .unwrap();
        assert_eq!(env.len(), 1);
        assert_eq!(env[0].name, "SSH_KEY");
        assert!(env[0].is_secret);
        assert_eq!(secret_env.len(), 1);
        assert_eq!(secret_env[0].name, "PLAIN");
    }
}
