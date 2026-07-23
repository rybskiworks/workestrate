use crate::config::{ConfigFile, WorkloadConfig};
use crate::microsandbox::plan::{EgressRule, EnvVar, HostBoundSecret, NetworkPlan, SandboxPlan};
use crate::microsandbox::secrets::{RemappedSecret, SecretDefinition};
use anyhow::Result;
use std::collections::HashMap;

/// Program and args to exec inside the sandbox via exec_stream.
#[derive(Debug, Clone)]
pub struct SandboxCommand {
    pub binary: String,
    pub arguments: Vec<String>,
}

impl SandboxCommand {
    #[allow(dead_code)]
    pub fn new(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
            arguments: vec![],
        }
    }
    pub fn with_args(binary: impl Into<String>, args: &[&str]) -> Self {
        Self {
            binary: binary.into(),
            arguments: args.iter().map(|s| s.to_string()).collect(),
        }
    }
}

/// How to set the sandbox entrypoint.
#[derive(Debug, Clone)]
pub enum EntrypointSpec {
    /// Blocking keep-alive foreground (`/bin/sh -c "tail -f /dev/null"`) that
    /// keeps the sandbox alive while `exec_stream` runs the real service. A bare
    /// `/bin/sh` would run `/bin/sh <image-cmd>` (Docker ENTRYPOINT+CMD) and exit.
    Shell,
}

/// A sandbox workload. The generic `ConfigWorkload` implementation reads from
/// `workestrate.toml`; the lifecycle in `runtime.rs` operates on `&W where W:
/// Workload`.
pub trait Workload: Send + Sync + std::fmt::Debug {
    /// Sandbox name (used for Sandbox::get, logging, user messages).
    fn name(&self) -> &str;

    /// Sandbox instance name (used for Sandbox::builder, log dirs, down).
    /// Default: bare workload name. ConfigWorkload overrides to
    /// `<context>-<workload>` when contexts are active.
    fn sandbox_instance_name(&self) -> String {
        self.name().to_string()
    }

    /// Build the declarative sandbox plan.
    fn plan(&self) -> SandboxPlan;

    /// Format the plan with a `[source]` annotation for each field.
    fn show_source(&self) -> String {
        self.plan().to_string()
    }

    /// Program and args to exec inside the sandbox.
    fn exec(&self) -> SandboxCommand;

    /// Args to pass when re-exec'ing in detached (background) mode.
    ///
    /// Reconstructs the CLI from `spec` so the detached child re-enters the
    /// `up --foreground` path with the SAME identity and flags the parent
    /// resolved:
    ///   - `--replace` (when `spec.replace`),
    ///   - `--instance <id>` for a parallel instance — the **bare id**, not
    ///     `slot@id` (the child re-derives the slot from its own context),
    ///   - `--port-offset <N>` when the offset is nonzero.
    ///
    /// `--new` is intentionally NOT forwarded: the parent has already
    /// materialized the slug into `spec.instance`, so the child must target
    /// that concrete instance rather than allocate a fresh one. The child
    /// re-parses these args via the existing `parse_service_action` /
    /// `parse_agent_action` path; no child-side change is required.
    fn detach_args(&self, spec: &crate::microsandbox::runtime::InstanceSpec) -> Vec<String> {
        let mut args: Vec<String> = vec![
            self.name().to_string(),
            "up".to_string(),
            "--foreground".to_string(),
        ];
        if spec.replace {
            args.push("--replace".to_string());
        }
        // Forward the parallel-instance id only when this is NOT the singleton
        // (instance == slot, no `@`). instance_id_of splits on the first `@`;
        // slots never contain `@`, so this is unambiguous.
        if let Some(id) = crate::microsandbox::slots::instance_id_of(&spec.instance) {
            args.push("--instance".to_string());
            args.push(id.to_string());
        }
        if spec.port_offset != 0 {
            args.push("--port-offset".to_string());
            args.push(spec.port_offset.to_string());
        }
        args
    }

    /// Optional pre-start hook (e.g., writing config files to persistent data dir).
    fn prepare(&self) -> Result<()> {
        Ok(())
    }

    /// Whether to log errors when stopping the sandbox (default: true).
    fn log_stop_errors(&self) -> bool {
        true
    }

    /// Sandbox entrypoint (default: Shell).
    fn entrypoint(&self) -> EntrypointSpec {
        EntrypointSpec::Shell
    }

    /// Where to find the built agent code. Defaults to agents/<name>/build;
    /// override per-agent with WORKESTRATE_<NAME>_BUILD (NAME uppercased,
    /// '-' → '_') — used by the nix wrapper to point at a store path.
    #[allow(dead_code)]
    fn build_path(&self) -> String {
        let key = format!(
            "WORKESTRATE_{}_BUILD",
            self.name().to_ascii_uppercase().replace('-', "_")
        );
        if let Ok(p) = std::env::var(key) {
            p
        } else {
            format!("agents/{}/build", self.name())
        }
    }

    /// Convention: config files live at agents/<name>/config/<filename>
    #[allow(dead_code)]
    fn config_path(&self, filename: &str) -> String {
        format!("agents/{}/config/{}", self.name(), filename)
    }
}

/// Resolved secret definition: direct (has its own env_var) or remapped
/// (exposes another secret under a different name).
#[derive(Debug, Clone)]
enum ResolvedSecret {
    Direct(SecretDefinition),
    Remapped(RemappedSecret),
}

/// Workload implementation driven by `workestrate.toml`. This replaces the
/// per-agent `workloads/*.rs` modules with a single generic implementation.
#[derive(Debug)]
pub struct ConfigWorkload {
    name: String,
    workload: WorkloadConfig,
    env: Vec<EnvVar>,
    secret_env: Vec<HostBoundSecret>,
    /// Maps the rendered env/secret_env name (for a remapped secret, the
    /// `exposed_as` name) back to the originating `[secrets.NAME]` definition
    /// name. Merge provenance is recorded under the SECRET DEF NAME
    /// (`workloads.{wl}.secret_env.{SECRET_DEF_NAME}`), while the rendered
    /// `HostBoundSecret.name` is the EXPOSED name — without this map a
    /// remapped secret like LITELLM_AUTH (exposed as OPENAI_API_KEY) looked up
    /// the wrong provenance key and fell back to "core" (WP6(b)/A5).
    secret_def_names: HashMap<String, String>,
    provenance: Option<crate::merge::Provenance>,
}

impl ConfigWorkload {
    /// Load the active config and construct a workload by name.
    pub fn new(name: &str) -> Result<Self> {
        let config = crate::config::load_config()?;
        let provenance = crate::merge::take_provenance();
        crate::config::validate_config(&config)?;
        let workload = config
            .workloads
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("workload '{}' not found in config", name))?
            .clone();

        let secrets = build_secret_definitions(&config)?;
        let env = build_env(&workload, &secrets)?;
        let secret_env = build_secret_env(&workload, &secrets)?;
        let secret_def_names = build_secret_def_name_map(&workload, &secrets);

        Ok(Self {
            name: name.to_string(),
            workload,
            env,
            secret_env,
            secret_def_names,
            provenance,
        })
    }

    /// Return the configured kind ("service" or "agent").
    pub fn kind(&self) -> &str {
        &self.workload.kind
    }

    fn resolve_image(&self) -> Option<String> {
        match self.workload.image.recipe.as_str() {
            "registry" => self.workload.image.reference.clone(),
            "nix-layered" => {
                let name = self.workload.image.name.as_deref().unwrap_or("");
                let tag = self.workload.image.tag.as_deref().unwrap_or("latest");
                Some(format!("{}:{}", name, tag))
            }
            _ => None,
        }
    }
}

impl Workload for ConfigWorkload {
    fn name(&self) -> &str {
        &self.name
    }

    fn sandbox_instance_name(&self) -> String {
        match crate::config::active_context_name() {
            Some(ctx) => format!("{}-{}", ctx, self.name),
            None => self.name.clone(),
        }
    }

    fn plan(&self) -> SandboxPlan {
        let egress_rules: Vec<EgressRule> = self
            .workload
            .network
            .egress
            .iter()
            .flat_map(crate::recipes::EgressRecipeRef::expand)
            .collect();

        let mut mounts = self.workload.mounts.clone();
        // WP6(c)/A6: a configured local_build.env_override is ALSO honored as
        // a mount-host template token (checked before the default convention).
        let env_override = self
            .workload
            .local_build
            .as_ref()
            .and_then(|b| b.env_override.as_deref());
        for m in &mut mounts {
            m.host =
                resolve_mount_host_template(&m.host, self.name(), &self.build_path(), env_override);
        }

        SandboxPlan {
            name: self.sandbox_instance_name(),
            image: self.resolve_image(),
            workdir: self.workload.workdir.clone(),
            command: self.workload.command.clone(),
            cpus: self.workload.cpus,
            memory_mib: self.workload.memory_mib,
            env: self.env.clone(),
            secret_env: self.secret_env.clone(),
            ports: self.workload.ports.clone(),
            mounts,
            network: NetworkPlan {
                default_deny: self.workload.network.default_deny.unwrap_or(true),
                egress_rules,
                deny_rules: self.workload.network.deny.clone(),
                ingress_rules: self.workload.network.ingress.clone(),
            },
        }
    }

    fn show_source(&self) -> String {
        let plan = self.plan();
        let mut out = String::new();
        let default_source = "core";
        let secret_prov = crate::merge::get_secret_provenance();

        let source_of = |key: &str| -> &str {
            self.provenance
                .as_ref()
                .and_then(|p| p.get(key))
                .map(|s| s.as_str())
                .unwrap_or(default_source)
        };

        let write_line = |out: &mut String, prefix: &str, content: &str, source: &str| {
            let full = format!("{}{}", prefix, content);
            let pad = if full.len() < 48 {
                " ".repeat(48 - full.len())
            } else {
                "  ".to_string()
            };
            out.push_str(&full);
            out.push_str(&pad);
            out.push('[');
            out.push_str(source);
            out.push(']');
            out.push('\n');
        };

        write_line(
            &mut out,
            "",
            &format!("name: {}", plan.name),
            default_source,
        );
        if let Some(img) = &plan.image {
            write_line(
                &mut out,
                "",
                &format!("image: {}", img),
                source_of(&format!("workloads.{}.image", self.name)),
            );
        }
        if let Some(wd) = &plan.workdir {
            write_line(
                &mut out,
                "",
                &format!("workdir: {}", wd),
                source_of(&format!("workloads.{}.workdir", self.name)),
            );
        }
        if !plan.command.is_empty() {
            write_line(
                &mut out,
                "",
                &format!("command: {}", plan.command.join(" ")),
                source_of(&format!("workloads.{}.command", self.name)),
            );
        }
        if let Some(cpus) = plan.cpus {
            write_line(
                &mut out,
                "",
                &format!("cpus: {}", cpus),
                source_of(&format!("workloads.{}.cpus", self.name)),
            );
        }
        if let Some(mem) = plan.memory_mib {
            write_line(
                &mut out,
                "",
                &format!("memory: {} MiB", mem),
                source_of(&format!("workloads.{}.memory_mib", self.name)),
            );
        }
        let env_source = source_of(&format!("workloads.{}.env", self.name));
        for e in &plan.env {
            let source = if e.is_secret {
                // WP6(b)/A5: resolve via the SECRET DEF NAME so remapped
                // secrets attribute to their true layer, not "core".
                secret_line_source(
                    &e.name,
                    &self.secret_def_names,
                    &self.name,
                    self.provenance.as_ref(),
                    secret_prov.as_ref(),
                    default_source,
                )
            } else {
                env_source
            };
            write_line(&mut out, "", &format!("env: {}", e), source);
        }
        for se in &plan.secret_env {
            // WP6(b)/A5: se.name is the EXPOSED name (e.g. OPENAI_API_KEY);
            // merge provenance is keyed by the SECRET DEF NAME (e.g.
            // LITELLM_AUTH). Resolve via the def-name map.
            let source = secret_line_source(
                &se.name,
                &self.secret_def_names,
                &self.name,
                self.provenance.as_ref(),
                secret_prov.as_ref(),
                default_source,
            );
            write_line(
                &mut out,
                "",
                &format!(
                    "secret_env: {} (redacted, allowed: {})",
                    se.name,
                    se.allowed_hosts.join(", ")
                ),
                source,
            );
        }
        let ports_source = source_of(&format!("workloads.{}.ports", self.name));
        for p in &plan.ports {
            write_line(
                &mut out,
                "",
                &format!("port: {}:{}", p.host, p.guest),
                ports_source,
            );
        }
        let mounts_source = source_of(&format!("workloads.{}.mounts", self.name));
        for m in &plan.mounts {
            let ro = if m.read_only { " (ro)" } else { "" };
            write_line(
                &mut out,
                "",
                &format!("mount: {}:{}{}", m.host, m.guest, ro),
                mounts_source,
            );
        }
        write_line(
            &mut out,
            "",
            &format!("network: default_deny={}", plan.network.default_deny),
            source_of(&format!("workloads.{}.network.default_deny", self.name)),
        );
        let ingress_source = source_of(&format!("workloads.{}.network.ingress", self.name));
        for rule in &plan.network.ingress_rules {
            write_line(
                &mut out,
                "  ",
                &format!("ingress: {}:{} {}", rule.protocol, rule.port, rule.scope),
                ingress_source,
            );
        }
        for rule in &plan.network.egress_rules {
            write_line(
                &mut out,
                "  ",
                &format!("egress: {}:{} -> {}", rule.protocol, rule.port, rule.target),
                "core",
            );
        }
        for rule in &plan.network.deny_rules {
            write_line(
                &mut out,
                "  ",
                &format!("egress: deny domain suffix {}", rule.domain_suffix),
                source_of(&format!(
                    "workloads.{}.network.deny.{}",
                    self.name, rule.domain_suffix
                )),
            );
        }

        out
    }

    fn exec(&self) -> SandboxCommand {
        match self.workload.command.split_first() {
            Some((binary, args)) => {
                let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                SandboxCommand::with_args(binary, &args)
            }
            None => SandboxCommand::with_args("", &[]),
        }
    }

    fn prepare(&self) -> Result<()> {
        let root = crate::config::project_root()
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
        let state_dir = crate::config::resolve_state_dir();
        for seed in &self.workload.seed_files {
            let source = root.join(&seed.source);
            let target =
                if seed.target.starts_with("workspaces/") || seed.target.starts_with("var/") {
                    state_dir.join(&seed.target)
                } else {
                    root.join(&seed.target)
                };
            if seed.only_if_missing.unwrap_or(true) && target.exists() {
                continue;
            }
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&source, &target).map_err(|e| {
                anyhow::anyhow!(
                    "failed to seed {} to {}: {}",
                    source.display(),
                    target.display(),
                    e
                )
            })?;
        }
        Ok(())
    }

    fn build_path(&self) -> String {
        if let Some(ref build) = self.workload.local_build {
            if let Some(ref env_override) = build.env_override {
                if let Ok(p) = std::env::var(env_override) {
                    return p;
                }
            }
            if let Some(ref fallback) = build.fallback {
                return fallback.clone();
            }
        }
        // Fallback to the workload-trait default env-var convention.
        let key = format!(
            "WORKESTRATE_{}_BUILD",
            self.name.to_ascii_uppercase().replace('-', "_")
        );
        if let Ok(p) = std::env::var(key) {
            return p;
        }
        format!("agents/{}/build", self.name)
    }

    fn log_stop_errors(&self) -> bool {
        self.workload.log_stop_errors.unwrap_or(true)
    }
}

/// Validate a `local_build.env_override` value as it appears in config.
/// Closes review finding C2: previously, a config layer could set
/// `env_override = "HOME"`, causing `build_path()` to read `$HOME` and (if a
/// mount host used the conventional `${WORKESTRATE_<NAME>_BUILD}` template)
/// bind-mount the operator's home directory into the sandbox.
///
/// Rules:
/// 1. Must match `^[A-Z_][A-Z0-9_]*$` (POSIX-ish env-var name).
/// 2. Must not be a process-global name that would hijack a meaningful
///    variable: HOME, USER, PATH, SHELL, PWD, SOPS_AGE_KEY_FILE,
///    WORKESTRATE_*, AGENTCTL_ROOT, XDG_*.
pub(crate) fn validate_env_override(name: &str) -> Result<()> {
    if name.is_empty() {
        anyhow::bail!("local_build.env_override cannot be empty");
    }
    let mut chars = name.chars();
    let first_ok = chars
        .next()
        .is_some_and(|c| c.is_ascii_uppercase() || c == '_');
    if !first_ok {
        anyhow::bail!(
            "local_build.env_override='{name}' must match ^[A-Z_][A-Z0-9_]*$              (must start with uppercase letter or underscore)"
        );
    }
    for c in chars {
        if !c.is_ascii_uppercase() && !c.is_ascii_digit() && c != '_' {
            anyhow::bail!(
                "local_build.env_override='{name}' must match ^[A-Z_][A-Z0-9_]*$                  (invalid character '{c}')"
            );
        }
    }
    const DENYLIST: &[&str] = &[
        "HOME",
        "USER",
        "PATH",
        "SHELL",
        "PWD",
        "SOPS_AGE_KEY_FILE",
        "WORKESTRATE_CONFIG_DIR",
        "WORKESTRATE_NO_PROJECT_CONFIG",
        "WORKESTRATE_CONTEXT",
        "AGENTCTL_ROOT",
    ];
    if DENYLIST.contains(&name) {
        anyhow::bail!(
            "local_build.env_override='{name}' is denied (process-global name);              choose a workload-specific name such as WORKESTRATE_MYWORKLOAD_BUILD"
        );
    }
    if name.starts_with("XDG_") {
        anyhow::bail!(
            "local_build.env_override='{name}' is denied (XDG_* process-global name);              choose a workload-specific name such as WORKESTRATE_MYWORKLOAD_BUILD"
        );
    }
    Ok(())
}

/// Validate a `seed_files.source` value. Closes review finding C3:
/// `root.join(&seed.source)` had no traversal check, so `source = "../../etc/passwd"`
/// would copy host files into sandbox state.
///
/// Rules:
/// 1. Reject empty.
/// 2. Reject absolute paths (leading `/`). Seed sources are project-root-relative.
/// 3. Reject any `..` component.
///
/// Note: seed sources do not participate in mount-template substitution, so
/// template prefixes like `${CWD}` are not expanded here — they would be
/// treated as literal directory names. Reject anything that is not a plain
/// relative path.
pub(crate) fn validate_seed_source(src: &str) -> Result<()> {
    use std::path::{Component, Path};
    if src.is_empty() {
        anyhow::bail!("seed_files.source cannot be empty");
    }
    if src.starts_with('/') {
        anyhow::bail!(
            "seed_files.source cannot be an absolute path (got '{src}');              seed sources must be project-root-relative"
        );
    }
    for component in Path::new(src).components() {
        if let Component::ParentDir = component {
            anyhow::bail!(
                "seed_files.source contains '..' component (got '{src}');                  path traversal is not allowed"
            );
        }
    }
    // Reject template-looking prefixes — they are not expanded for seed sources
    // and would silently create a literal directory named ${CWD}.
    if src.starts_with("${") {
        anyhow::bail!(
            "seed_files.source='{src}' looks like a template token, but seed sources              are not template-substituted; use a project-root-relative path"
        );
    }
    Ok(())
}

/// Resolve mount-host template tokens (WP6(c)/A6).
///
/// Matches, in precedence order:
/// 1. `${<env_override>}` — the workload's configured
///    `local_build.env_override` var name (checked FIRST so a custom override
///    that happens to equal the default convention still wins).
/// 2. `${WORKESTRATE_<NAME>_BUILD}` — the default convention (NAME
///    uppercased, '-' → '_'), kept for backward compatibility.
/// 3. `${CWD}` / `${CWD}/...` — current working directory (unchanged).
///
/// Anything else is returned unchanged (literal).
fn resolve_mount_host_template(
    host: &str,
    name: &str,
    build_path: &str,
    env_override: Option<&str>,
) -> String {
    if let Some(custom) = env_override {
        let custom_template = format!("${{{custom}}}");
        if host == custom_template {
            return build_path.to_string();
        }
    }
    let default_build_var = format!(
        "${{WORKESTRATE_{}_BUILD}}",
        name.to_ascii_uppercase().replace('-', "_")
    );
    if host == default_build_var {
        return build_path.to_string();
    }
    if host == "${CWD}" {
        if let Ok(cwd) = std::env::current_dir() {
            return cwd.to_string_lossy().into_owned();
        }
    }
    if let Some(rest) = host.strip_prefix("${CWD}/") {
        if let Ok(cwd) = std::env::current_dir() {
            return format!("{}/{}", cwd.to_string_lossy(), rest);
        }
    }
    host.to_string()
}

fn build_secret_definitions(config: &ConfigFile) -> Result<HashMap<String, ResolvedSecret>> {
    let mut direct: HashMap<String, SecretDefinition> = HashMap::new();
    for (name, secret) in &config.secrets {
        if let Some(ref env_var) = secret.env_var {
            direct.insert(
                name.clone(),
                SecretDefinition {
                    env_var: env_var.clone(),
                    hosts: secret.hosts.clone().unwrap_or_default(),
                    required: secret.required.unwrap_or(true),
                    placeholder: secret.placeholder.clone(),
                    description: String::new(),
                },
            );
        }
    }

    let mut resolved: HashMap<String, ResolvedSecret> = HashMap::new();
    for (name, secret) in &config.secrets {
        if let Some(ref env_var) = secret.env_var {
            let def = direct
                .get(name)
                .ok_or_else(|| anyhow::anyhow!("secret '{}' env_var '{}' missing", name, env_var))?
                .clone();
            resolved.insert(name.clone(), ResolvedSecret::Direct(def));
        } else if let Some(ref source_name) = secret.source {
            let source = direct
                .get(source_name)
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "secret '{}' source '{}' not found or not a direct secret",
                        name,
                        source_name
                    )
                })
                .cloned()?;
            let exposed_as = secret
                .exposed_as
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("remapped secret '{}' has no exposed_as", name))?;
            resolved.insert(
                name.clone(),
                ResolvedSecret::Remapped(RemappedSecret {
                    source,
                    exposed_as: exposed_as.to_string(),
                }),
            );
        } else {
            anyhow::bail!("secret '{}' has neither env_var nor source", name);
        }
    }
    Ok(resolved)
}

fn build_env(
    workload: &WorkloadConfig,
    secrets: &HashMap<String, ResolvedSecret>,
) -> Result<Vec<EnvVar>> {
    let mut env = Vec::new();
    for e in &workload.env {
        match (&e.value, &e.secret) {
            (Some(value), None) => env.push(EnvVar::literal(&e.name, value)),
            (None, Some(secret_name)) => {
                let resolved = secrets.get(secret_name).ok_or_else(|| {
                    anyhow::anyhow!(
                        "env '{}' references undefined secret '{}'",
                        e.name,
                        secret_name
                    )
                })?;
                match resolved {
                    ResolvedSecret::Direct(def) => env.push(EnvVar {
                        name: e.name.clone(),
                        value: format!("${{{}}}", def.env_var),
                        is_secret: true,
                        reject_placeholder: def.placeholder.clone(),
                    }),
                    ResolvedSecret::Remapped(_) => {
                        anyhow::bail!(
                            "env '{}' cannot reference remapped secret '{}'",
                            e.name,
                            secret_name
                        )
                    }
                }
            }
            (None, None) => env.push(EnvVar::literal(&e.name, "")),
            (Some(_), Some(_)) => {
                anyhow::bail!("env '{}' cannot have both value and secret", e.name)
            }
        }
    }
    Ok(env)
}

fn build_secret_env(
    workload: &WorkloadConfig,
    secrets: &HashMap<String, ResolvedSecret>,
) -> Result<Vec<HostBoundSecret>> {
    let mut secret_env = Vec::new();
    for se in &workload.secret_env {
        let resolved = secrets.get(&se.secret).ok_or_else(|| {
            anyhow::anyhow!("secret_env references undefined secret '{}'", se.secret)
        })?;
        match resolved {
            ResolvedSecret::Direct(def) => secret_env.push(HostBoundSecret::from(def)),
            ResolvedSecret::Remapped(remap) => secret_env.push(HostBoundSecret::remapped(remap)),
        }
    }
    Ok(secret_env)
}

/// Map each rendered secret-bearing env name to its `[secrets.NAME]`
/// definition name (WP6(b)/A5).
///
/// For `secret_env` entries the rendered name is the secret's exposed name
/// (its own `env_var` for a direct secret, `exposed_as` for a remapped one)
/// and the def name is `se.secret`. For secret-backed `env` entries
/// (`e.secret = "NAME"`) the rendered name is `e.name` and the def name is
/// `e.secret` (env entries can only reference direct secrets — remapped
/// references bail in `build_env`).
fn build_secret_def_name_map(
    workload: &WorkloadConfig,
    secrets: &HashMap<String, ResolvedSecret>,
) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for se in &workload.secret_env {
        if let Some(resolved) = secrets.get(&se.secret) {
            let rendered_name = match resolved {
                ResolvedSecret::Direct(def) => def.env_var.clone(),
                ResolvedSecret::Remapped(remap) => remap.exposed_as.clone(),
            };
            map.insert(rendered_name, se.secret.clone());
        }
    }
    for e in &workload.env {
        if let Some(ref secret_name) = e.secret {
            map.insert(e.name.clone(), secret_name.clone());
        }
    }
    map
}

/// Resolve the provenance layer for a rendered secret line (WP6(b)/A5).
///
/// Lookup order:
/// 1. Secret-load provenance (`get_secret_provenance()`), keyed by the
///    rendered ENV VAR name (e.g. LITELLM_MASTER_KEY).
/// 2. Merge provenance `workloads.{wl}.secret_env.{SECRET_DEF_NAME}` — keyed
///    by the SECRET DEF NAME so remapped secrets (LITELLM_AUTH exposed as
///    OPENAI_API_KEY) resolve to the layer that actually bound them, instead
///    of falling through to "core" when keyed by the exposed name.
/// 3. `default` ("core").
fn secret_line_source<'a>(
    rendered_name: &str,
    secret_def_names: &HashMap<String, String>,
    workload_name: &str,
    provenance: Option<&'a crate::merge::Provenance>,
    secret_prov: Option<&'a crate::merge::Provenance>,
    default: &'a str,
) -> &'a str {
    if let Some(layer) = secret_prov.and_then(|p| p.get(rendered_name)) {
        return layer.as_str();
    }
    if let Some(def_name) = secret_def_names.get(rendered_name) {
        let key = format!("workloads.{workload_name}.secret_env.{def_name}");
        if let Some(layer) = provenance.and_then(|p| p.get(&key)) {
            return layer.as_str();
        }
    }
    default
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
    use std::path::PathBuf;

    /// RAII guard that points `WORKESTRATE_CONFIG_DIR` at the committed test
    /// fixture and restores the previous state on drop. Holds a global lock so
    /// env-var tests do not race when Cargo runs them in parallel.
    struct TestConfigGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl TestConfigGuard {
        fn new() -> Self {
            let lock = crate::config::tests::ENV_TEST_LOCK.lock().unwrap();
            let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("tests")
                .join("fixtures")
                .join("config");
            std::env::set_var("WORKESTRATE_CONFIG_DIR", fixture);
            Self { _lock: lock }
        }
    }

    impl Drop for TestConfigGuard {
        fn drop(&mut self) {
            std::env::remove_var("WORKESTRATE_CONFIG_DIR");
        }
    }

    #[test]
    fn build_path_reads_per_agent_env_override() -> Result<()> {
        let _guard = TestConfigGuard::new();
        let pi = ConfigWorkload::new("pi")?;
        // Override set → returns the env value.
        std::env::set_var("WORKESTRATE_PI_BUILD", "/tmp/test-pi-build");
        assert_eq!(pi.build_path(), "/tmp/test-pi-build");
        // Override removed → falls back to agents/<name>/build.
        std::env::remove_var("WORKESTRATE_PI_BUILD");
        assert_eq!(pi.build_path(), "agents/pi/build");
        Ok(())
    }

    #[test]
    fn sandbox_instance_name_bare_when_no_context() -> Result<()> {
        let _guard = TestConfigGuard::new();
        // TestConfigGuard sets WORKESTRATE_CONFIG_DIR which bypasses the registry,
        // so active_context_name() is None.
        crate::config::set_active_context(None);
        let pi = ConfigWorkload::new("pi")?;
        assert_eq!(pi.sandbox_instance_name(), "pi");
        assert_eq!(pi.name(), "pi");
        Ok(())
    }

    #[test]
    fn sandbox_instance_namespaced_when_context_active() -> Result<()> {
        let _guard = TestConfigGuard::new();
        let pi = ConfigWorkload::new("pi")?;
        // Simulate an active context after creating the workload; load_config()
        // resets the active context when WORKESTRATE_CONFIG_DIR is set.
        crate::config::set_active_context(Some(crate::config::ActiveContext {
            name: Some("personal".to_string()),
            layers: vec!["personal".to_string()],
        }));
        assert_eq!(pi.sandbox_instance_name(), "personal-pi");
        assert_eq!(pi.name(), "pi"); // bare name unchanged for CLI dispatch
                                     // Clean up
        crate::config::set_active_context(None);
        Ok(())
    }

    // ---- C2 regression: validate_env_override rejects dangerous names ----

    #[test]
    fn env_override_rejects_process_global_names() {
        for hostile in [
            "HOME",
            "USER",
            "PATH",
            "SHELL",
            "PWD",
            "SOPS_AGE_KEY_FILE",
            "WORKESTRATE_CONFIG_DIR",
            "WORKESTRATE_NO_PROJECT_CONFIG",
            "WORKESTRATE_CONTEXT",
            "AGENTCTL_ROOT",
            "XDG_CONFIG_HOME",
            "XDG_DATA_HOME",
        ] {
            let err = validate_env_override(hostile).unwrap_err();
            assert!(
                err.to_string().contains("denied"),
                "env_override='{hostile}' should be denied; got: {err}"
            );
        }
    }

    #[test]
    fn env_override_rejects_lowercase_and_invalid_chars() {
        // All entries must FAIL the regex `^[A-Z_][A-Z0-9_]*$` OR be empty.
        // (Syntactically valid names that are not in the denylist, e.g.
        // `BUILD_PATH`, are tested in the accept-list test.)
        for hostile in ["home", "Home", "FOO-BAR", "1FOO", "FOO BAR", ""] {
            assert!(
                validate_env_override(hostile).is_err(),
                "env_override='{hostile}' should be rejected"
            );
        }
    }

    #[test]
    fn env_override_accepts_legitimate_workload_specific_names() {
        for ok in [
            "WORKESTRATE_PI_BUILD",
            "PI_BUILD_DIR",
            "MY_WORKLOAD_PATH",
            "_FOO",
            "A",
            "ABC_123",
        ] {
            validate_env_override(ok)
                .unwrap_or_else(|e| panic!("legitimate env_override='{ok}' rejected: {e}"));
        }
    }

    // ---- C3 regression: validate_seed_source rejects traversal ----

    #[test]
    fn seed_source_rejects_traversal_and_absolute() {
        for hostile in [
            "../../etc/passwd",
            "foo/../bar",
            "..",
            "/etc/passwd",
            "${CWD}/secret",
            "${MSB_HOME}/secret",
        ] {
            let err = validate_seed_source(hostile).unwrap_err();
            assert!(
                err.to_string().contains("..")
                    || err.to_string().contains("absolute")
                    || err.to_string().contains("template"),
                "seed_source='{hostile}' should be rejected; got: {err}"
            );
        }
    }

    #[test]
    fn seed_source_accepts_relative_paths() {
        for ok in [
            "agents/pi/config.json",
            "config.reference/workestrate.toml",
            "seed/db.sql",
        ] {
            validate_seed_source(ok)
                .unwrap_or_else(|e| panic!("legitimate seed_source='{ok}' rejected: {e}"));
        }
    }

    // ---- WP6(b)/A5: secret provenance resolves by SECRET DEF NAME ----

    /// A5 before/after demonstration:
    /// - BEFORE (keyed by exposed name): provenance lookup
    ///   `workloads.odysseus.secret_env.OPENAI_API_KEY` misses → "core".
    /// - AFTER (keyed by secret-def name):
    ///   `workloads.odysseus.secret_env.LITELLM_AUTH` hits → the true layer.
    #[test]
    fn secret_provenance_resolves_remapped_secret_by_def_name() -> Result<()> {
        // Base layer defines the direct secret + binds LITELLM_AUTH on odysseus;
        // the team layer redefines LITELLM_AUTH (changes exposed_as).
        let base = crate::merge::Layer::from_string(
            "base",
            "schema_version = 1\n\n[secrets.LITELLM_MASTER_KEY]\nenv_var = \"LITELLM_MASTER_KEY\"\n\n[secrets.LITELLM_AUTH]\nsource = \"LITELLM_MASTER_KEY\"\nexposed_as = \"OPENAI_API_KEY\"\n\n[workloads.odysseus]\nkind = \"service\"\nimage = { recipe = \"registry\", ref = \"python:3.12-slim\" }\ncommand = []\n\n[[workloads.odysseus.secret_env]]\nsecret = \"LITELLM_AUTH\"\n\n[workloads.odysseus.network]\ndefault_deny = true",
        )?;
        let team = crate::merge::Layer::from_string(
            "team",
            "schema_version = 1\n\n[secrets.LITELLM_AUTH]\nsource = \"LITELLM_MASTER_KEY\"\nexposed_as = \"OPENAI_API_KEY\"",
        )?;

        let (config, provenance) = crate::merge::merge_layers(&[base, team])?;
        let secrets = build_secret_definitions(&config)?;
        let workload = config.workloads.get("odysseus").unwrap().clone();
        let secret_env = build_secret_env(&workload, &secrets)?;
        let def_names = build_secret_def_name_map(&workload, &secrets);

        // The rendered HostBoundSecret carries the EXPOSED name.
        let rendered = secret_env
            .iter()
            .find(|se| se.name == "OPENAI_API_KEY")
            .expect("remapped secret should render under its exposed name");

        // BEFORE: the buggy lookup keyed by the exposed name misses and falls
        // back to "core".
        let before = provenance
            .get("workloads.odysseus.secret_env.OPENAI_API_KEY")
            .map(|s| s.as_str())
            .unwrap_or("core");
        assert_eq!(
            before, "core",
            "BEFORE: keying by exposed name misses merge provenance → 'core'"
        );

        // AFTER: the helper resolves via the secret-def name → the layer that
        // bound the secret_env entry.
        let after = secret_line_source(
            &rendered.name,
            &def_names,
            "odysseus",
            Some(&provenance),
            None,
            "core",
        );
        assert_eq!(
            after, "base",
            "AFTER: keying by secret-def name (LITELLM_AUTH) yields the true layer"
        );
        Ok(())
    }

    #[test]
    fn secret_provenance_prefers_secret_load_provenance_by_env_name() {
        // secret-load provenance (keyed by env var name) wins over the merge
        // fallback when both are present.
        let mut secret_prov: crate::merge::Provenance = HashMap::new();
        secret_prov.insert("OPENAI_API_KEY".to_string(), "user-global".to_string());
        let mut prov: crate::merge::Provenance = HashMap::new();
        prov.insert(
            "workloads.odysseus.secret_env.LITELLM_AUTH".to_string(),
            "team".to_string(),
        );
        let mut def_names: HashMap<String, String> = HashMap::new();
        def_names.insert("OPENAI_API_KEY".to_string(), "LITELLM_AUTH".to_string());

        let src = secret_line_source(
            "OPENAI_API_KEY",
            &def_names,
            "odysseus",
            Some(&prov),
            Some(&secret_prov),
            "core",
        );
        assert_eq!(src, "user-global");
    }

    #[test]
    fn secret_provenance_falls_back_to_core_when_unrecorded() {
        let def_names: HashMap<String, String> = HashMap::new();
        let src = secret_line_source("SOME_KEY", &def_names, "pi", None, None, "core");
        assert_eq!(src, "core");
    }

    // ---- WP6(c)/A6: build_path template honors custom env_override ----

    #[test]
    fn mount_template_matches_custom_env_override() {
        // ${CUSTOM_BUILD} with env_override = CUSTOM_BUILD → build_path.
        let got = resolve_mount_host_template(
            "${CUSTOM_BUILD}",
            "pi",
            "/tmp/build-out",
            Some("CUSTOM_BUILD"),
        );
        assert_eq!(got, "/tmp/build-out");
    }

    #[test]
    fn mount_template_still_matches_default_convention_with_override_set() {
        // Backward compat: ${WORKESTRATE_PI_BUILD} still resolves even when a
        // custom env_override is configured.
        let got = resolve_mount_host_template(
            "${WORKESTRATE_PI_BUILD}",
            "pi",
            "/tmp/build-out",
            Some("CUSTOM_BUILD"),
        );
        assert_eq!(got, "/tmp/build-out");
    }

    #[test]
    fn mount_template_custom_token_literal_without_override() {
        // Without a configured override, ${CUSTOM_BUILD} is NOT substituted.
        let got = resolve_mount_host_template("${CUSTOM_BUILD}", "pi", "/tmp/build-out", None);
        assert_eq!(got, "${CUSTOM_BUILD}");
    }

    #[test]
    fn mount_template_cwd_unchanged() {
        let got = resolve_mount_host_template("${CWD}", "pi", "/tmp/build-out", None);
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(got, cwd.to_string_lossy());
        let got =
            resolve_mount_host_template("${CWD}/sub", "pi", "/tmp/build-out", Some("X_BUILD"));
        assert_eq!(got, format!("{}/sub", cwd.to_string_lossy()));
    }

    #[test]
    fn mount_template_default_convention_without_override() {
        let got = resolve_mount_host_template("${WORKESTRATE_PI_BUILD}", "pi", "/tmp/b", None);
        assert_eq!(got, "/tmp/b");
        // Hyphenated workload names map '-' → '_'.
        let got = resolve_mount_host_template(
            "${WORKESTRATE_EXAMPLE_AGENT_BUILD}",
            "example-agent",
            "/tmp/b",
            None,
        );
        assert_eq!(got, "/tmp/b");
    }
}
