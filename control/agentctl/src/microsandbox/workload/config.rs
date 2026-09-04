use super::secrets::{
    apply_secret_policy_ladder, build_env_and_secret_env, build_secret_definitions,
};
use super::validate::resolve_mount_host_template;
use super::{SandboxCommand, Workload};
use crate::config::WorkloadConfig;
use crate::microsandbox::env::{render_seed_text, SeedEnvView};
use crate::microsandbox::plan::{
    EnvVar, HostBoundSecret, MountPlan, SandboxPlan,
};
use anyhow::Result;
use std::path::PathBuf;

/// Resolve the content root for one workload field (`mounts`,
/// `seed_files`): the content dir of the layer that DECLARED the field, per
/// the merge provenance (`workloads.<name>.<field>` → layer name) and the
/// layer-dirs map (layer name → parent dir of the layer file).
///
/// Returns `None` when provenance or the layer's dir is unavailable
/// (synthetic `from_string` layers without a source path); the caller then
/// applies the documented fallback explicitly.
fn field_content_root(
    provenance: Option<&crate::merge::Provenance>,
    layer_dirs: &std::collections::HashMap<String, PathBuf>,
    workload: &str,
    field: &str,
) -> Option<PathBuf> {
    let layer = provenance?.get(&format!("workloads.{workload}.{field}"))?;
    layer_dirs.get(layer).cloned()
}

/// Workload implementation driven by `workestrate.toml`. This replaces the
/// per-agent `workloads/*.rs` modules with a single generic implementation.
#[derive(Debug)]
pub struct ConfigWorkload {
    pub(super) name: String,
    pub(super) workload: WorkloadConfig,
    pub(super) env: Vec<EnvVar>,
    pub(super) secret_env: Vec<HostBoundSecret>,
    pub(super) provenance: Option<crate::merge::Provenance>,
    /// Content root for repo-relative mount hosts: the directory of the
    /// config layer that declared `workloads.<name>.mounts`. Resolved at
    /// construction from provenance + the load-time layer-dirs map.
    pub(super) mount_content_root: Option<PathBuf>,
    /// Content root for seed-file sources and non-state targets: the
    /// directory of the config layer that declared
    /// `workloads.<name>.seed_files`.
    pub(super) seed_content_root: Option<PathBuf>,
    /// ADR 0026(d) discovery-lite: depends_on resolutions computed at
    /// construction (declaration triggers resolution on every up/exec/plan
    /// path). `plan()` appends the injected env AFTER the declared env and
    /// the derived egress AFTER the expanded declared egress rules.
    pub(super) depends_resolved: Vec<crate::microsandbox::discovery::ResolvedDependency>,
    /// Compiled policies in mount declaration order. The runtime will select
    /// the program for the relevant guest mount when per-mount transmission is
    /// wired.
    pub(super) mount_policies: Vec<(String, crate::mount_policy::MountPolicyProgram)>,
    /// The declaring config-repo namespace (ADR 0030 Phase 2 T1): resolved
    /// from provenance at construction; the registry-record namespace this
    /// workload's instances register under and depends_on resolution filters
    /// by.
    pub(super) namespace: String,
}

impl ConfigWorkload {
    /// Plan-time host-overlap WARNING (ADR 0028 companion, E0): after
    /// `${CWD}` / `${WORKESTRATE_<NAME>_BUILD}` substitution, two mounts
    /// whose RESOLVED host directories overlap (identical, or one is an
    /// ancestor of the other) get a stderr warning. The classic trigger is
    /// a `${CWD}` mount run from inside a declared state mount's host
    /// (e.g. `exec` from `…/state/workspaces/prime-state` when another
    /// mount binds that dir). Warn only — overlapping HOST dirs are
    /// semantically harmless (separate virtiofs tags, coherent views)
    /// unless combined with per-mount masking (spec 22), where an
    /// unmasked tag would see what a masked tag hides. Nested GUEST paths
    /// are a separate, LEGAL spec 01 shadow pattern and are never warned.
    fn host_overlap_warnings(&self, mounts: &[MountPlan]) -> Vec<String> {
        use std::path::{Component, Path, PathBuf};

        // Resolve each host the way the runtime would bind it, so a
        // `${CWD}` absolute path is comparable with a `workspaces/...`
        // state-dir host or a declaring-content-root-relative host.
        let state_dir = crate::config::resolve_state_dir();
        let content_root = self.mount_content_root();
        let mut resolved: Vec<(String, PathBuf)> = Vec::new();
        for m in mounts {
            let host = if m.host.starts_with("workspaces/") || m.host.starts_with("var/") {
                state_dir.join(&m.host)
            } else if m.host.starts_with("${MSB_HOME}/") {
                // Sandbox-internal home dir; cannot overlap a workload host.
                continue;
            } else {
                let p = Path::new(&m.host);
                if p.is_absolute() {
                    p.to_path_buf()
                } else if let Some(root) = &content_root {
                    root.join(p)
                } else {
                    continue; // unresolvable relative host without a content root
                }
            };
            // Lexical normalization (collapse `.` / `..`) — rw mounts may not
            // exist yet, so fs::canonicalize would fail; component-wise is
            // deterministic and testable.
            let mut norm = PathBuf::new();
            for comp in host.components() {
                match comp {
                    Component::CurDir => {}
                    Component::ParentDir => {
                        norm.pop();
                    }
                    other => norm.push(other.as_os_str()),
                }
            }
            resolved.push((m.guest.clone(), norm));
        }

        let mut warnings = Vec::new();
        for i in 0..resolved.len() {
            for j in (i + 1)..resolved.len() {
                let (guest_a, host_a) = &resolved[i];
                let (guest_b, host_b) = &resolved[j];
                if guest_a == guest_b {
                    continue; // exact-duplicate guests are already a validate-time error
                }
                let overlaps =
                    host_a == host_b || host_a.starts_with(host_b) || host_b.starts_with(host_a);
                if overlaps {
                    warnings.push(format!(
                        "workload '{}': mount host '{}' (guest '{}') overlaps host '{}' (guest '{}'); \
                         both resolve under the same host directory — verify this is intended (spec 22 masking is per-mount)",
                        self.name,
                        host_a.display(),
                        guest_a,
                        host_b.display(),
                        guest_b
                    ));
                }
            }
        }
        warnings
    }

    /// Load the active config and construct a workload by name.
    pub fn new(name: &str) -> Result<Self> {
        Self::new_with_use_overrides(name, &[])
    }

    /// Load the active config and construct a workload by name, applying the
    /// typed `--use <dep>@<instance>` instance-selection overrides
    /// (`(dep, instance-id)` pairs; ADR 0026(d)) to depends_on resolution.
    ///
    /// The overrides are validated INSIDE
    /// [`crate::microsandbox::discovery::resolve_depends_on`] against this
    /// workload's DECLARED depends_on map: an override naming an undeclared
    /// dep (or any override with no depends_on at all) is a hard error here,
    /// so every up/exec/plan path refuses identically.
    ///
    /// Thin wrapper over [`Self::new_with_use_overrides_and_instance`] with
    /// NO dependent-instance threading (ADR 0030 P2.1: the singleton-
    /// dependent view — scoped DEFAULT selection does not fire).
    pub fn new_with_use_overrides(name: &str, use_overrides: &[(String, String)]) -> Result<Self> {
        Self::new_with_use_overrides_and_instance(name, use_overrides, None)
    }

    /// [`Self::new_with_use_overrides`] plus the DEPENDENT's own resolved
    /// parallel instance id (ADR 0030 P2.1), threaded into
    /// [`crate::microsandbox::discovery::resolve_depends_on_full`] so a
    /// SCOPED dep's no-override DEFAULT exports selection picks the
    /// `<dep>@<workload>-<id>` record instead of the singleton. main.rs
    /// resolves the id BEFORE dependency auto-start
    /// ([`crate::commands::lifecycle::resolve_dependent_instance_id`]) and
    /// passes it here; the detached child re-derives the same id from its
    /// forwarded `--instance`, so parent and child construct identical
    /// workload views. `None` (plan/down/logs and singleton dependents)
    /// keeps the singleton-selection behavior unchanged.
    pub fn new_with_use_overrides_and_instance(
        name: &str,
        use_overrides: &[(String, String)],
        dependent_instance_id: Option<&str>,
    ) -> Result<Self> {
        let config = crate::config::load_config()?;
        // take_provenance is always Some immediately after load_config in
        // production; unwrap_or_default covers degenerate paths (synthetic
        // constructions without a load) and lets the secret violation-policy
        // ladder resolution below always run against a real map.
        let mut provenance = crate::merge::take_provenance().unwrap_or_default();
        let layer_dirs = crate::merge::get_layer_dirs().unwrap_or_default();
        crate::config::validate_config(&config)?;
        let workload = config
            .workloads
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("workload '{}' not found in config", name))?
            .clone();

        let mount_policies = crate::mount_policy::get_collected_policy()
            .map(|collected| {
                workload
                    .mounts
                    .iter()
                    .filter_map(|mount| {
                        let mut scopes = collected.global.clone();
                        scopes.extend(
                            collected
                                .workloads
                                .get(name)
                                .into_iter()
                                .flat_map(|scopes| scopes.iter())
                                .filter(|scope| {
                                    scope.scope_kind != crate::mount_policy::ScopeKind::MountEntry
                                        || scope.mount_guest.as_deref() == Some(mount.guest.as_str())
                                })
                                .cloned(),
                        );
                        if scopes.is_empty() {
                            return None;
                        }
                        let guest = mount.guest.clone();
                        let program = crate::mount_policy::compile(scopes).map_err(|e| {
                            anyhow::anyhow!(
                                "mount policy for workload '{name}' mount '{guest}' failed to compile: {e}"
                            )
                        });
                        Some(program.map(|program| (guest, program)))
                    })
                    .collect::<Result<Vec<_>>>()
            })
            .transpose()?
            .unwrap_or_default();

        // Content roots (spec 17): repo-relative mount hosts and seed-file
        // paths resolve against the DECLARING layer's directory, not the
        // flake project root. Mounts and seed_files merge wholesale-replace,
        // so each field has exactly one declaring layer.
        let mount_content_root = field_content_root(Some(&provenance), &layer_dirs, name, "mounts");
        let seed_content_root =
            field_content_root(Some(&provenance), &layer_dirs, name, "seed_files");

        // ADR 0030 Phase 2 T1: the workload's declaring config-repo namespace
        // (from provenance) — the registry-record namespace its instances
        // register under and depends_on resolution filters by. The provenance
        // is passed explicitly (take_provenance drained it above; a second
        // read would return None).
        let layer_dirs = crate::merge::get_layer_dirs().unwrap_or_default();
        let namespace = crate::commands::deps::namespace_for(Some(&provenance), &layer_dirs, name);

        // ADR 0026(d): a declared depends_on map resolves EVERY declared dep
        // at plan time — no flag. A required-but-not-running dep refuses
        // here, on every up/exec/plan path (they all construct via `new`).
        // ADR 0030 P2.1: `dependent_instance_id` drives the SCOPED mode's
        // no-override DEFAULT selection (`<dep>@<name>-<id>`).
        let state_dir = crate::config::resolve_state_dir();
        let depends_resolved = crate::microsandbox::discovery::resolve_depends_on_full(
            &config,
            name,
            &state_dir,
            use_overrides,
            &namespace,
            dependent_instance_id,
        )?;

        let mut secrets = build_secret_definitions(&config)?;
        apply_secret_policy_ladder(&mut secrets, &config, name, &mut provenance);
        let (env, secret_env) = build_env_and_secret_env(&workload, &secrets)?;

        Ok(Self {
            name: name.to_string(),
            workload,
            env,
            secret_env,
            provenance: Some(provenance),
            mount_content_root,
            seed_content_root,
            depends_resolved,
            mount_policies,
            namespace,
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
                let declared = self.workload.image.tag.as_deref().unwrap_or("latest");
                // A2 (ADR 0032 §Image tags — DECIDED 2026-08-24): resolve
                // through the state-dir current-pointer for
                // (name, image_tag_context()). READ-ONLY (a cheap
                // images.json load at plan time; never a write). MISS → the
                // legacy `name:<declared-tag>` fallback (byte-identical
                // pre-migration behavior — golden plans and
                // not-yet-rebuilt homes keep working until their first
                // post-upgrade ensure writes a pointer).
                //
                // Repo matching: `self.namespace` is the declaring
                // config-repo identity (ADR 0030 P2 T1) — the registered
                // repo NAME when the declaring layer is under a registered
                // checkout (matching the pointer key's repo segment), or the
                // "default" sentinel when no repo identity is resolvable
                // (synthetic/single-file/unregistered layers). Pass it as
                // the preference hint; resolve_image_tag falls back to a
                // name+ctx scan across repos when the hint is absent or
                // misses (unregistered repos key their pointers by
                // canonical path, which the namespace never carries).
                let state_dir = crate::config::resolve_state_dir();
                let repo = (self.namespace
                    != crate::microsandbox::port_registry::default_namespace())
                .then_some(self.namespace.as_str());
                Some(crate::images::state::resolve_image_tag(
                    &state_dir, repo, name, declared,
                ))
            }
            _ => None,
        }
    }

    /// Resolve the build path, reporting whether the result is the
    /// UNDECLARED reserved default (`.workestrate-build/<name>`, spec 21
    /// §6.1): no configured `env_override` in effect, no declared `fallback`,
    /// and no conventional `WORKESTRATE_<NAME>_BUILD` env var set. The
    /// reserved default resolves declaring-layer-relative and must not
    /// trigger the flake project-root machinery; declared fallbacks and env
    /// overrides keep the pre-reservation behavior unchanged.
    fn resolve_build_path(&self) -> (String, bool) {
        if let Some(ref build) = self.workload.local_build {
            if let Some(ref env_override) = build.env_override {
                if let Ok(p) = std::env::var(env_override) {
                    return (p, false);
                }
            }
            if let Some(ref fallback) = build.fallback {
                return (fallback.clone(), false);
            }
        }
        // Fallback to the workload-trait default env-var convention.
        let key = format!(
            "WORKESTRATE_{}_BUILD",
            self.name.to_ascii_uppercase().replace('-', "_")
        );
        if let Ok(p) = std::env::var(key) {
            return (p, false);
        }
        (format!(".workestrate-build/{}", self.name), true)
    }

    /// Seed a single file from a `seed_files` entry: skip when
    /// `only_if_missing` and the target already exists, create the target's
    /// parent, then either template-render (`template = true`, against the
    /// guest-visible env view) or byte-copy the source. `source_label` is the
    /// raw relative source string for error messages (the resolved `source`
    /// path may be content-root-joined). Shared by the per-entry loop in
    /// `prepare()` and reused for glob-expanded entries.
    ///
    /// `reseed` (the `--reseed` flag) bypasses the `only_if_missing` skip
    /// for TEMPLATE seeds only: an existing target is re-rendered and
    /// overwritten, and the refresh is logged. Static (non-template) seeds
    /// keep the existing semantics under `--reseed` (never overwritten when
    /// `only_if_missing`).
    // too_many_arguments: `target_label` preserves the raw config `target`
    // string in error labels (the resolved `target` path is state/content-root
    // joined); the positional shape matches the pre-refactor `prepare()` loop.
    #[allow(clippy::too_many_arguments)]
    fn seed_one_file(
        &self,
        source: &std::path::Path,
        target: &std::path::Path,
        target_label: &str,
        template: bool,
        only_if_missing: bool,
        source_label: &str,
        env_view: &SeedEnvView,
        reseed: bool,
    ) -> Result<()> {
        let target_exists = target.exists();
        if only_if_missing && target_exists && !(reseed && template) {
            return Ok(());
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if template {
            let text = std::fs::read_to_string(source).map_err(|e| {
                anyhow::anyhow!(
                    "failed to read seed source {} for template rendering: {}",
                    source.display(),
                    e
                )
            })?;
            let label = format!("seed source '{}' target '{}'", source_label, target_label);
            let rendered = render_seed_text(&text, env_view, &label)?;
            std::fs::write(target, &rendered).map_err(|e| {
                anyhow::anyhow!(
                    "failed to seed {} to {}: {}",
                    source.display(),
                    target.display(),
                    e
                )
            })?;
            // --reseed visibility: name each template seed target refreshed
            // over an existing file so the user sees what changed.
            if reseed && target_exists {
                eprintln!(
                    "reseed: re-rendered template seed '{}' -> {}",
                    source_label,
                    target.display()
                );
            }
        } else {
            std::fs::copy(source, target).map_err(|e| {
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

    #[allow(clippy::panic)]
    fn plan(&self) -> SandboxPlan {
        // ADR 0035: hierarchical policy compilation replaces recipe expansion.
        // The ladder is collected at load time (home > layers > workload) and
        // stored process-global. We compile it here per-workload.
        let ladder = crate::merge::get_network_policy_ladder().unwrap_or_default();
        let mut network_plan = match crate::policy::network_policy::compile_network_plan(
            &ladder,
            &self.name,
            &self.workload,
        ) {
            Ok(plan) => plan,
            Err(e) => {
                // For on_conflict=fail and validation errors, plan must fail.
                // The Workload trait's plan() is infallible, so we panic with the
                // structured error — callers that need to handle the error
                // should call the compiler directly.
                panic!("network policy compilation failed for workload '{}': {e}", self.name);
            }
        };
        let egress_rules = network_plan.egress_rules.clone();

        // ADR 0026(d): apply the depends_on resolution — injected env AFTER
        // declared env (declared wins on a name conflict; skipped inside),
        // derived egress AFTER the compiled declared rules (identical rules
        // deduped inside). Derivation only ADDS: the egress default is untouched
        // (precedence-resolved; explicit allow stands alone), and the derived
        // rules land in `egress_rules` so `network_plan_to_policy` consumes
        // them identically to declared egress.
        let (injected_env, derived_egress) = crate::microsandbox::discovery::apply_resolution(
            &self.depends_resolved,
            &self.env,
            &egress_rules,
        );
        let mut env = self.env.clone();
        env.extend(injected_env);
        let mut egress_rules = egress_rules;
        egress_rules.extend(derived_egress);
        // Update the network plan with derived egress
        network_plan.egress_rules = egress_rules;

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
        // ADR 0028 companion (E0): warn-only on overlapping resolved HOST
        // dirs (the `${CWD}`-vs-state-mount case); never an error.
        for w in self.host_overlap_warnings(&mounts) {
            eprintln!("warning: {w}");
        }

        SandboxPlan {
            name: self.sandbox_instance_name(),
            image: self.resolve_image(),
            workdir: self.workload.workdir.clone(),
            command: self.workload.command.clone(),
            cpus: self.workload.cpus,
            memory_mib: self.workload.memory_mib,
            env,
            secret_env: self.secret_env.clone(),
            ports: self.workload.ports.clone(),
            mounts,
            network: network_plan,
            instance_policy: (self.workload.instance != Default::default())
                .then(|| self.workload.instance.clone()),
        }
    }

    fn show_source(&self) -> String {
        self.show_source_render()
    }

    fn mount_policy_for(&self, guest: &str) -> Option<&crate::mount_policy::MountPolicyProgram> {
        self.mount_policies
            .iter()
            .find(|(mount_guest, _)| mount_guest == guest)
            .map(|(_, policy)| policy)
    }

    fn instance_conflict_chain(&self) -> Vec<crate::config::ConflictStep> {
        self.workload
            .instance
            .on_conflict
            .clone()
            .map(|c| c.0)
            .unwrap_or_else(|| crate::config::DepConflict::default_chain().0)
    }

    fn namespace(&self) -> String {
        self.namespace.clone()
    }

    fn instance_strategy(&self) -> crate::config::InstanceStrategy {
        self.workload.instance.strategy
    }

    fn instance_port(&self) -> Option<crate::config::InstancePort> {
        self.workload.instance.port.clone()
    }

    fn instance_on_skew(&self) -> Option<crate::config::OnSkew> {
        self.workload.instance.on_skew
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

    #[allow(private_interfaces)] // SeedEnvView is crate-internal by design
    fn prepare(
        &self,
        env_view: &SeedEnvView,
        reseed: bool,
        instance_state_key: Option<&str>,
    ) -> Result<()> {
        if self.workload.seed_files.is_empty() {
            return Ok(());
        }
        // Host Bug C, defense-in-depth: seeds are mount-backed BY DESIGN.
        // Before ANY filesystem write, require every seed target to be
        // covered by a declared mount host — otherwise prepare() would
        // silently render into the host working tree (or state dir) where
        // the guest never sees it. Config validation enforces the same rule
        // at load time; this guard catches any future caller that bypasses
        // it. The check uses the RAW target (pre-instance-scoping): the
        // scoped path stays under the same state root, so coverage of the
        // raw target implies coverage of the scoped one.
        let mount_hosts: Vec<&str> = self
            .workload
            .mounts
            .iter()
            .map(|m| m.host.as_str())
            .collect();
        for seed in &self.workload.seed_files {
            super::validate::validate_seed_target_coverage(&seed.target, &mount_hosts).map_err(
                |e| {
                    anyhow::anyhow!(
                        "workload '{}' seed_files.target '{}' mount coverage check failed: {e}",
                        self.name,
                        seed.target
                    )
                },
            )?;
        }
        // Content root for seed sources and non-state targets: the directory
        // of the config layer that declared this workload's seed_files
        // (spec 17). EXPLICIT FALLBACK: when no declaring layer dir is
        // knowable (synthetic layers), fall back to the flake project root
        // when one resolves. There is NO silent cwd fallback (F3 — removed
        // the old `unwrap_or_else(current_dir)`): if neither is available,
        // hard-error and name the workload.
        let root = match self
            .seed_content_root
            .clone()
            .or_else(crate::config::project_root_optional)
        {
            Some(root) => root,
            None => anyhow::bail!(
                "workload '{}' declares seed_files but no content root can be resolved: \
                 the declaring config layer has no source directory and no flake project \
                 root was found. Set AGENTCTL_ROOT, run from the workbench root, or \
                 declare seed_files from a file-backed config layer.",
                self.name
            ),
        };
        let state_dir = crate::config::resolve_state_dir();
        for seed in &self.workload.seed_files {
            // ADR 0030 V-addendum §V2: per-instance state addressing — under
            // a per-dir instance key, targets at/below the workload's state
            // root (`workspaces/<name>-state[/...]`) gain the key segment so
            // each instance seeds its OWN state subdir. `None` (singleton /
            // non-per-dir) leaves every target byte-identical to before.
            let scoped_target = crate::microsandbox::mounts::instance_scoped_state_path(
                &seed.target,
                self.name(),
                instance_state_key,
            );
            let target_dir =
                if scoped_target.starts_with("workspaces/") || scoped_target.starts_with("var/") {
                    state_dir.join(&scoped_target)
                } else {
                    root.join(&scoped_target)
                };
            match &seed.source {
                Some(src) => {
                    let source = root.join(src);
                    self.seed_one_file(
                        &source,
                        &target_dir,
                        &scoped_target,
                        seed.template,
                        seed.only_if_missing.unwrap_or(true),
                        src,
                        env_view,
                        reseed,
                    )?;
                }
                None => {
                    // Validation enforces exactly one of source|glob; if a
                    // config ever bypassed it, refuse rather than panic.
                    let Some(glob) = seed.glob.as_deref() else {
                        anyhow::bail!(
                            "workload '{}' seed_files entry has neither source nor glob \
                             (config validation should have rejected it)",
                            self.name
                        );
                    };
                    let expansion = crate::microsandbox::mounts::expand_seed_glob(&root, glob)?;
                    if expansion.files.is_empty() {
                        anyhow::bail!(
                            "workload '{}' seed_files glob '{}' matched no files under {}",
                            self.name,
                            glob,
                            root.display()
                        );
                    }
                    for file in &expansion.files {
                        let rel = file.strip_prefix(&expansion.root).map_err(|_| {
                            anyhow::anyhow!(
                                "seed glob match {} escaped the glob root {}",
                                file.display(),
                                expansion.root.display()
                            )
                        })?;
                        let target = target_dir.join(rel);
                        self.seed_one_file(
                            file,
                            &target,
                            &scoped_target,
                            seed.template,
                            seed.only_if_missing.unwrap_or(true),
                            &rel.to_string_lossy(),
                            env_view,
                            reseed,
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    fn build_path(&self) -> String {
        self.resolve_build_path().0
    }

    fn build_path_is_reserved_default(&self) -> bool {
        self.resolve_build_path().1
    }

    fn log_stop_errors(&self) -> bool {
        self.workload.log_stop_errors.unwrap_or(true)
    }

    fn mount_content_root(&self) -> Option<PathBuf> {
        self.mount_content_root.clone()
    }

    /// F2 lazy gate: `build_sandbox` calls `project_root()` ONLY when the
    /// workload genuinely needs the flake checkout. The triggers:
    ///
    /// (a) a `nix-layered` image recipe (image build artifacts live in the
    ///     tool flake);
    /// (b) a `local_build` config (recipes — including `flake://` sources —
    ///     execute against the flake checkout);
    /// (c) a mount whose host, after `${WORKESTRATE_<NAME>_BUILD}` template
    ///     substitution in `plan()`, IS the workload's relative build path
    ///     AND that path is a flake-checkout artifact (a declared fallback
    ///     or a relative env override, e.g. `agents/<name>/build`). The
    ///     UNDECLARED reserved default (`.workestrate-build/<name>`, spec 21
    ///     §6.1) resolves declaring-layer-relative and is excluded — it
    ///     needs no flake root.
    ///
    /// Registry-image workloads with none of these return `None`, so
    /// `build_sandbox` never touches the flake-root gate for them.
    fn flake_root_requirement(&self, plan: &SandboxPlan) -> Option<String> {
        if self.workload.image.recipe == "nix-layered" {
            return Some("nix-layered image".to_string());
        }
        if self.workload.local_build.is_some() {
            return Some("local_build config".to_string());
        }
        let (build_path, reserved_default) = self.resolve_build_path();
        if !reserved_default
            && !std::path::Path::new(&build_path).is_absolute()
            && plan.mounts.iter().any(|m| m.host == build_path)
        {
            return Some(format!("relative build-path mount '{build_path}'"));
        }
        None
    }

    /// ConfigWorkload override: check mounts (via the shared core) PLUS seed
    /// sources (resolved against the declaring layer's seed content root) and
    /// the `local_build.fallback` build output. See the trait method doc for
    /// the hard/warn semantics.
    fn preflight_existence(&self, plan: &SandboxPlan, hard: bool) -> Result<Vec<String>> {
        let owned = crate::microsandbox::mounts::resolve_mount_roots_owned(self, plan)?;
        let roots = owned.as_roots();
        let fallback = self
            .workload
            .local_build
            .as_ref()
            .and_then(|b| b.fallback.as_deref());
        crate::microsandbox::mounts::preflight_existence(
            &roots,
            plan,
            &self.workload.seed_files,
            self.seed_content_root.as_deref(),
            fallback,
            &self.name,
            hard,
        )
    }
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
    use crate::config::test_support::TestConfigGuard;
    use crate::microsandbox::plan::EgressTarget;

    #[test]
    fn build_path_reads_per_agent_env_override() -> Result<()> {
        let _guard = TestConfigGuard::new();
        let pi = ConfigWorkload::new("pi")?;
        // Override set → returns the env value (and is NOT the reserved default).
        std::env::set_var("WORKESTRATE_PI_BUILD", "/tmp/test-pi-build");
        assert_eq!(pi.build_path(), "/tmp/test-pi-build");
        assert!(!pi.build_path_is_reserved_default());
        // Override removed → falls back to the reserved default
        // `.workestrate-build/<name>` (spec 21 §6.1).
        std::env::remove_var("WORKESTRATE_PI_BUILD");
        assert_eq!(pi.build_path(), ".workestrate-build/pi");
        assert!(pi.build_path_is_reserved_default());
        Ok(())
    }

    /// Spec 21 §6.1: the reservation changes the DEFAULT, never a
    /// declaration — a declared `local_build.fallback` resolves exactly as
    /// before, and the env override still beats it.
    #[test]
    fn build_path_declared_fallback_unchanged_and_env_beats_it() -> Result<()> {
        let _guard = TestConfigGuard::new();
        // Fixture odysseus declares `fallback = "agents/odysseus/build"`.
        std::env::remove_var("WORKESTRATE_ODYSSEUS_BUILD");
        let odysseus = ConfigWorkload::new("odysseus")?;
        assert_eq!(odysseus.build_path(), "agents/odysseus/build");
        assert!(!odysseus.build_path_is_reserved_default());
        // Env override precedence unchanged: it beats the declared fallback.
        std::env::set_var("WORKESTRATE_ODYSSEUS_BUILD", "/tmp/test-ody-build");
        assert_eq!(odysseus.build_path(), "/tmp/test-ody-build");
        std::env::remove_var("WORKESTRATE_ODYSSEUS_BUILD");
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

    // ---- ADR 0026(d): depends_on resolution fires inside ConfigWorkload ----

    /// Minimal two-workload config: `pi` depends on `litellm` (4000:4000).
    /// The caller adjusts `required` per test.
    const DEPENDS_CONFIG_TOML: &str = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[workloads.pi.depends_on.litellm]
env = "LITELLM_URL"

[workloads.pi.network.defaults]
egress = "deny"

[workloads.litellm]
kind = "service"
image = { recipe = "registry", ref = "node:24-bookworm-slim" }
command = []

[[workloads.litellm.ports]]
host = 4000
guest = 4000

[workloads.litellm.network.defaults]
egress = "deny"
"#;

    /// RAII guard: point `WORKESTRATE_CONFIG_DIR` at a temp dir holding a
    /// `workestrate.toml` with `content`, and `WORKESTRATE_STATE_DIR` at a
    /// temp dir acting as the (initially empty) port-registry state home.
    /// Holds the global env lock; both vars are removed on drop.
    struct DependsEnvGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
        config_dir: std::path::PathBuf,
        state_dir: std::path::PathBuf,
    }

    impl DependsEnvGuard {
        fn new(label: &str, content: &str) -> Self {
            let lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
            let config_dir = crate::config::test_support::unique_state_dir(label);
            let state_dir = crate::config::test_support::unique_state_dir(label);
            std::fs::create_dir_all(&config_dir).expect("create temp config dir");
            std::fs::create_dir_all(&state_dir).expect("create temp state dir");
            std::fs::write(config_dir.join("workestrate.toml"), content)
                .expect("write temp workestrate.toml");
            std::env::set_var("WORKESTRATE_CONFIG_DIR", &config_dir);
            std::env::set_var("WORKESTRATE_STATE_DIR", &state_dir);
            Self {
                _lock: lock,
                config_dir,
                state_dir,
            }
        }

        fn state_dir(&self) -> &std::path::Path {
            &self.state_dir
        }

        fn config_dir(&self) -> &std::path::Path {
            &self.config_dir
        }
    }

    impl Drop for DependsEnvGuard {
        fn drop(&mut self) {
            std::env::remove_var("WORKESTRATE_CONFIG_DIR");
            std::env::remove_var("WORKESTRATE_STATE_DIR");
            let _ = std::fs::remove_dir_all(&self.config_dir);
            let _ = std::fs::remove_dir_all(&self.state_dir);
        }
    }

    fn register_litellm_singleton(state_dir: &std::path::Path) -> Result<()> {
        crate::microsandbox::port_registry::check_and_register_sandbox_lifecycle(
            state_dir,
            "litellm",
            None,
            "litellm",
            crate::microsandbox::plan::default_bind_ip(),
            &[4000],
            &[crate::microsandbox::plan::PortMapping::new(4000, 4000)],
            "2026-07-30T00:00:00Z",
            "default",
            None,
            // A2 (ADR 0032 §Image tags): no running tag known at this site.
            None,
            None,
            None,
        )
    }

    /// ConfigWorkload::new resolves declared deps and plan() injects the
    /// guest-form address env + derives the egress rule, marked.
    #[test]
    fn new_resolves_depends_on_and_plan_injects_env_and_egress() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-inject", DEPENDS_CONFIG_TOML);
        register_litellm_singleton(guard.state_dir())?;

        let pi = ConfigWorkload::new("pi")?;
        let plan = pi.plan();

        let injected = plan
            .env
            .iter()
            .find(|e| e.name == "LITELLM_URL")
            .expect("plan env must carry the injected LITELLM_URL");
        assert_eq!(injected.value, "host.microsandbox.internal:4000");
        assert!(!injected.is_secret, "the injected URL is not a secret");
        assert_eq!(injected.injected_by.as_deref(), Some("litellm"));
        // The plan Display marks the injection.
        assert!(
            format!("{plan}").contains(
                "env: LITELLM_URL=host.microsandbox.internal:4000 (injected: depends_on 'litellm')"
            ),
            "plan render must mark the injected env; got:\n{plan}"
        );

        let derived: Vec<_> = plan
            .network
            .egress_rules
            .iter()
            .filter(|r| r.derived_from.as_deref() == Some("litellm"))
            .collect();
        assert_eq!(derived.len(), 1, "exactly one derived egress rule");
        assert_eq!(derived[0].port, 4000);
        assert_eq!(derived[0].target, EgressTarget::Host);
        assert!(
            format!("{plan}")
                .contains("  egress: tcp:4000 -> host (derived: depends_on 'litellm')"),
            "plan render must mark the derived egress; got:\n{plan}"
        );
        // Monotonic: derivation only ADDED; the egress default is untouched.
        assert!(plan.network.egress_default_deny);
        Ok(())
    }

    /// Optional dep with NO running record: resolution falls back to the
    /// declared port; the injection still lands in the plan.
    #[test]
    fn new_falls_back_to_declared_port_when_dep_not_running() -> Result<()> {
        let _guard = DependsEnvGuard::new("cw-fallback", DEPENDS_CONFIG_TOML);
        let pi = ConfigWorkload::new("pi")?;
        let plan = pi.plan();
        let injected = plan
            .env
            .iter()
            .find(|e| e.name == "LITELLM_URL")
            .expect("fallback injection still lands in the plan");
        assert_eq!(injected.value, "host.microsandbox.internal:4000");
        Ok(())
    }

    /// Required dep with NO running record: ConfigWorkload::new REFUSES —
    /// every up/exec/plan path constructs through `new`, so the refusal
    /// propagates on all of them.
    #[test]
    fn new_refuses_when_required_dep_not_running() -> Result<()> {
        let required_toml = DEPENDS_CONFIG_TOML.replace(
            "env = \"LITELLM_URL\"",
            "env = \"LITELLM_URL\"\nrequired = true",
        );
        let _guard = DependsEnvGuard::new("cw-refuse", &required_toml);
        let err = ConfigWorkload::new("pi").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("is required but not running; start it with"),
            "refusal must carry the remediation lead: {msg}"
        );
        assert!(
            msg.contains("workestrate workload up litellm"),
            "refusal must name the start command: {msg}"
        );
        Ok(())
    }

    // ---- ADR 0030 Phase 2 T1 regression: namespace-matched resolution ----

    /// RAII guard: a temp `WORKESTRATE_HOME` holding a REGISTERED
    /// directory-mode config repo "personal" (declaring `litellm` + `prime`,
    /// the required-dep pair). Unlike `DependsEnvGuard` (a single synthetic
    /// "local" layer whose namespace is always "default"), this exercises the
    /// provenance → layer → repo-key namespace resolution end to end.
    // Field order matters: fields drop in declaration order, so `_env`
    // restores WORKESTRATE_HOME (EnvGuard::drop) BEFORE `_lock` releases
    // ENV_TEST_LOCK — no window for a racing lock holder to observe the
    // mutated env.
    struct RegisteredHomeGuard {
        _env: crate::config::test_support::EnvGuard,
        _lock: std::sync::MutexGuard<'static, ()>,
        home: std::path::PathBuf,
    }

    impl RegisteredHomeGuard {
        fn new(label: &str) -> Result<Self> {
            use crate::config::test_support::{uniq_dir, EnvGuard, ENV_TEST_LOCK, HOME_ENV_KEYS};
            let lock = ENV_TEST_LOCK.lock().unwrap();
            let env = EnvGuard::capture(HOME_ENV_KEYS);
            let home = uniq_dir(label);
            std::fs::create_dir_all(&home).expect("create temp home");
            std::env::set_var("WORKESTRATE_HOME", &home);
            std::env::remove_var("WORKESTRATE_CONFIG_DIR");
            let repo = home.join("config-repos").join("personal");
            let workloads = repo.join("workestrate").join("workloads");
            std::fs::create_dir_all(&workloads)?;
            std::fs::write(
                repo.join("workestrate").join("default.toml"),
                "schema_version = 1\n",
            )?;
            std::fs::write(workloads.join("litellm.toml"), DIR_MODE_LITELLM_TOML)?;
            std::fs::write(workloads.join("prime.toml"), DIR_MODE_PRIME_TOML)?;
            let canonical = repo.canonicalize()?;
            crate::config::register_config("personal", &canonical.to_string_lossy(), None, None)?;
            Ok(Self {
                _env: env,
                _lock: lock,
                home,
            })
        }

        /// The state dir the runtime resolves under this home
        /// (`<home>/state`, HomeKind::Env).
        fn state_dir(&self) -> std::path::PathBuf {
            self.home.join("state")
        }
    }

    impl Drop for RegisteredHomeGuard {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.home);
        }
    }

    /// Directory-mode `workloads/litellm.toml` (full form; schema_version is
    /// default.toml-only in directory mode).
    const DIR_MODE_LITELLM_TOML: &str = "[workloads.litellm]\nkind = \"service\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\n\n[[workloads.litellm.ports]]\nhost = 4000\nguest = 4000\n\n[workloads.litellm.network.defaults]\negress = \"deny\"\n";

    /// Directory-mode `workloads/prime.toml`: an agent with a REQUIRED dep on
    /// litellm.
    const DIR_MODE_PRIME_TOML: &str = "[workloads.prime]\nkind = \"agent\"\nimage = { recipe = \"registry\", ref = \"node:24-bookworm-slim\" }\ncommand = []\n\n[workloads.prime.depends_on.litellm]\nenv = \"LITELLM_URL\"\nrequired = true\n\n[workloads.prime.network.defaults]\negress = \"deny\"\n";

    /// Write the standalone-started `litellm` record the way `build_sandbox`
    /// does (slot instance name, the given namespace).
    fn register_litellm_record(state_dir: &std::path::Path, namespace: &str) -> Result<()> {
        crate::microsandbox::port_registry::check_and_register_sandbox_lifecycle(
            state_dir,
            "litellm",
            None,
            "litellm",
            crate::microsandbox::plan::default_bind_ip(),
            &[4000],
            &[crate::microsandbox::plan::PortMapping::new(4000, 4000)],
            "2026-07-30T00:00:00Z",
            namespace,
            None,
            // A2 (ADR 0032 §Image tags): no running tag known at this site.
            None,
            None,
            None,
        )
    }

    /// A standalone-started service's record (namespace = its declaring repo
    /// key) is FOUND by a same-repo dependent's namespace-filtered resolution:
    /// construction succeeds and the plan carries the injected env.
    #[test]
    fn dep_resolution_matches_standalone_record_in_declaring_namespace() -> Result<()> {
        let guard = RegisteredHomeGuard::new("cw-ns-match")?;
        register_litellm_record(&guard.state_dir(), "personal")?;

        let prime = ConfigWorkload::new("prime")?;
        assert_eq!(prime.namespace(), "personal");
        let plan = prime.plan();
        let injected = plan
            .env
            .iter()
            .find(|e| e.name == "LITELLM_URL")
            .expect("the running record's injection lands in the plan");
        assert_eq!(injected.value, "host.microsandbox.internal:4000");
        Ok(())
    }

    /// The negative twin: a record in a FOREIGN namespace ("default" — e.g. a
    /// legacy pre-namespace record) is NOT usable by the dependent; resolution
    /// refuses with the collision message naming the required namespace and
    /// the namespace(s) where records exist.
    #[test]
    fn new_refuses_foreign_namespace_record_naming_namespaces() -> Result<()> {
        let guard = RegisteredHomeGuard::new("cw-ns-refuse")?;
        register_litellm_record(&guard.state_dir(), "default")?;

        let err = ConfigWorkload::new("prime").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("namespace 'personal'"),
            "refusal must name the dependent's required namespace: {msg}"
        );
        assert!(
            msg.contains("[default]"),
            "refusal must name the namespace(s) holding the record: {msg}"
        );
        Ok(())
    }

    /// The reference/test fixtures declare NO depends_on → construction is
    /// unaffected and the plan carries no injected/derived markers.
    #[test]
    fn plan_without_depends_on_is_unaffected() -> Result<()> {
        let _guard = TestConfigGuard::new();
        let pi = ConfigWorkload::new("pi")?;
        let plan = pi.plan();
        assert!(
            plan.env.iter().all(|e| e.injected_by.is_none()),
            "no env may be marked injected without depends_on"
        );
        assert!(
            plan.network
                .egress_rules
                .iter()
                .all(|r| r.derived_from.is_none()),
            "no egress may be marked derived without depends_on"
        );
        Ok(())
    }

    // ---- ADR 0026(d)/C3-W2: new_with_use_overrides ----

    /// A singleton AND a parallel record exist; `new` selects the singleton
    /// while `new_with_use_overrides([("litellm","canary")])` injects the
    /// PARALLEL record's port (guest form) into the plan env.
    #[test]
    fn new_with_use_overrides_selects_the_parallel_record() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-use", DEPENDS_CONFIG_TOML);
        register_litellm_singleton(guard.state_dir())?;
        crate::microsandbox::port_registry::check_and_register_sandbox_lifecycle(
            guard.state_dir(),
            "litellm@canary",
            None,
            "litellm",
            std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 2)),
            &[14000],
            &[crate::microsandbox::plan::PortMapping::new(14000, 4000)],
            "2026-07-30T00:00:00Z",
            "default",
            None,
            // A2 (ADR 0032 §Image tags): no running tag known at this site.
            None,
            None,
            None,
        )?;

        let pi = ConfigWorkload::new("pi")?;
        let injected = pi
            .plan()
            .env
            .iter()
            .find(|e| e.name == "LITELLM_URL")
            .expect("default injection present")
            .value
            .clone();
        assert_eq!(injected, "host.microsandbox.internal:4000");

        let pi = ConfigWorkload::new_with_use_overrides(
            "pi",
            &[("litellm".to_string(), "canary".to_string())],
        )?;
        let plan = pi.plan();
        let injected = plan
            .env
            .iter()
            .find(|e| e.name == "LITELLM_URL")
            .expect("override injection present");
        assert_eq!(injected.value, "host.microsandbox.internal:14000");
        let derived: Vec<_> = plan
            .network
            .egress_rules
            .iter()
            .filter(|r| r.derived_from.as_deref() == Some("litellm"))
            .collect();
        assert_eq!(derived.len(), 1);
        assert_eq!(derived[0].port, 14000, "egress follows the selected record");
        Ok(())
    }

    /// An override naming a dep the workload does NOT declare refuses at
    /// construction — every up/exec/plan path surfaces the same hard error.
    #[test]
    fn new_with_use_overrides_refuses_undeclared_dep() -> Result<()> {
        let _guard = DependsEnvGuard::new("cw-use-undeclared", DEPENDS_CONFIG_TOML);
        let err = ConfigWorkload::new_with_use_overrides(
            "pi",
            &[("redis".to_string(), "canary".to_string())],
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("redis"), "error must name the dep: {msg}");
        assert!(
            msg.contains("not a declared depends_on entry"),
            "error must explain --use only overrides DECLARED deps: {msg}"
        );
        Ok(())
    }

    /// An unknown selected instance refuses at construction, naming dep +
    /// instance.
    #[test]
    fn new_with_use_overrides_refuses_unknown_instance() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-use-ghost", DEPENDS_CONFIG_TOML);
        register_litellm_singleton(guard.state_dir())?;
        let err = ConfigWorkload::new_with_use_overrides(
            "pi",
            &[("litellm".to_string(), "ghost".to_string())],
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("litellm") && msg.contains("ghost"));
        Ok(())
    }

    // ---- F1: content roots resolve against the DECLARING layer's dir ----

    /// Pure resolution: provenance key → layer name → layer dir.
    #[test]
    fn field_content_root_follows_provenance_to_layer_dir() {
        let mut provenance = crate::merge::Provenance::new();
        provenance.insert(
            "workloads.litellm.mounts".to_string(),
            "personal#workestrate/workloads/litellm.toml".to_string(),
        );
        let mut dirs = std::collections::HashMap::new();
        dirs.insert(
            "personal#workestrate/workloads/litellm.toml".to_string(),
            PathBuf::from("/store/config-repos/personal/workestrate/workloads"),
        );
        assert_eq!(
            field_content_root(Some(&provenance), &dirs, "litellm", "mounts"),
            Some(PathBuf::from(
                "/store/config-repos/personal/workestrate/workloads"
            ))
        );
        // No provenance → None (caller applies the documented fallback).
        assert_eq!(field_content_root(None, &dirs, "litellm", "mounts"), None);
        // Provenance names a synthetic layer with no dir → None.
        let mut provenance = crate::merge::Provenance::new();
        provenance.insert(
            "workloads.litellm.mounts".to_string(),
            "synthetic".to_string(),
        );
        assert_eq!(
            field_content_root(Some(&provenance), &dirs, "litellm", "mounts"),
            None
        );
    }

    /// Multi-layer pipeline: a directory-mode-style layer re-declaring
    /// `mounts` moves the mount content root to ITS directory-mode root;
    /// `seed_files` declared only by the base layer keep the BASE layer's
    /// directory-mode root. The content root is the `<repo>/workestrate/`
    /// dir (the parent of `workloads/`), NOT the capsule dir or the
    /// `workloads/` dir — so a `host = "workloads/svc"` mount resolves
    /// without doubling. Mirrors the `load_config` merge + `layer_dirs_from`
    /// wiring end to end.
    #[test]
    fn content_roots_track_the_declaring_layer_across_a_merge() -> Result<()> {
        let base_root = std::path::Path::new("/tmp/f1-base-repo/workestrate");
        let capsule_root = std::path::Path::new("/tmp/f1-personal-repo/workestrate");
        let base_dir = base_root.join("workloads");
        let capsule_dir = capsule_root.join("workloads").join("svc");
        let base = crate::merge::Layer::from_string_with_path(
            "base#workestrate/workloads/svc.toml",
            "schema_version = 1\n\n[workloads.svc]\nkind = \"service\"\nimage = { recipe = \"registry\", ref = \"python:3.12-slim\" }\ncommand = []\n\n[[workloads.svc.mounts]]\nhost = \"base-config.yaml\"\nguest = \"/app/cfg\"\nread_only = true\n\n[[workloads.svc.seed_files]]\nsource = \"seed/s.json\"\ntarget = \"workspaces/svc-state/s.json\"\n\n[workloads.svc.network.defaults]\negress = \"deny\"",
            Some(base_dir.join("svc.toml")),
        )?;
        let capsule = crate::merge::Layer::from_string_with_path(
            "personal#workestrate/workloads/svc/workload.toml",
            "schema_version = 1\n\n[workloads.svc]\n\n[[workloads.svc.mounts]]\nhost = \"config.yaml\"\nguest = \"/app/cfg\"\nread_only = true",
            Some(capsule_dir.join("workload.toml")),
        )?;

        let dirs = crate::merge::layer_dirs_from(std::slice::from_ref(&base));
        let mut dirs = dirs;
        dirs.extend(crate::merge::layer_dirs_from(std::slice::from_ref(
            &capsule,
        )));
        let (_merged, provenance) = crate::merge::merge_layers(&[base, capsule])?;

        // Mounts: wholesale-replaced by the capsule layer → the capsule
        // layer's directory-mode root (`<repo>/workestrate/`), NOT the
        // capsule dir.
        assert_eq!(
            field_content_root(Some(&provenance), &dirs, "svc", "mounts"),
            Some(capsule_root.to_path_buf())
        );
        // Seed files: declared only by the base layer → the base layer's
        // directory-mode root, NOT the `workloads/` dir.
        assert_eq!(
            field_content_root(Some(&provenance), &dirs, "svc", "seed_files"),
            Some(base_root.to_path_buf())
        );
        Ok(())
    }

    /// End-to-end through `ConfigWorkload::new` (WORKESTRATE_CONFIG_DIR
    /// single layer named "local"): the content root is the config dir
    /// holding the declaring `workestrate.toml`, and `prepare()` seeds from
    /// it — never from project_root or cwd.
    const SEED_CONFIG_TOML: &str = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = ["true"]

[[workloads.svc.mounts]]
host = "config.yaml"
guest = "/app/config.yaml"
read_only = true

[[workloads.svc.mounts]]
host = "workspaces/svc-state"
guest = "/data"

[[workloads.svc.seed_files]]
source = "seed/settings.json"
target = "workspaces/svc-state/settings.json"
only_if_missing = true

[workloads.svc.network.defaults]
egress = "deny"
"#;

    /// Same shape as `SEED_CONFIG_TOML` but the seed entry is a
    /// `template = true` file: `prepare()` renders it against the
    /// guest-visible env view instead of copying byte-identical.
    const TEMPLATE_CONFIG_TOML: &str = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = ["true"]

[[workloads.svc.mounts]]
host = "workspaces/svc-state"
guest = "/data"

[[workloads.svc.seed_files]]
source = "seed/settings.json.tpl"
target = "workspaces/svc-state/settings.json"
template = true

[workloads.svc.network.defaults]
egress = "deny"
"#;

    /// P2.2: a glob seed entry (no `source`; validation enforces exactly one
    /// of source|glob). Each regular-file match of `seed/**/*.json` seeds to
    /// `target/<rel-path>` (rel = match minus the literal glob root `seed/`).
    const GLOB_CONFIG_TOML: &str = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = ["true"]

[[workloads.svc.mounts]]
host = "workspaces/svc-state"
guest = "/data"

[[workloads.svc.seed_files]]
glob = "seed/**/*.json"
target = "workspaces/svc-state/globbed"

[workloads.svc.network.defaults]
egress = "deny"
"#;

    /// P2.2 template+glob variant: every matched `.tpl` file is rendered
    /// against the guest-visible env view before being seeded.
    const GLOB_TEMPLATE_CONFIG_TOML: &str = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = ["true"]

[[workloads.svc.mounts]]
host = "workspaces/svc-state"
guest = "/data"

[[workloads.svc.seed_files]]
glob = "seed/**/*.tpl"
target = "workspaces/svc-state/globbed"
template = true

[workloads.svc.network.defaults]
egress = "deny"
"#;

    #[test]
    fn new_resolves_content_roots_to_the_declaring_config_dir() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-content-root", SEED_CONFIG_TOML);
        let svc = ConfigWorkload::new("svc")?;
        assert_eq!(
            svc.mount_content_root.as_deref(),
            Some(guard.config_dir()),
            "mount content root is the declaring layer's dir"
        );
        assert_eq!(
            svc.seed_content_root.as_deref(),
            Some(guard.config_dir()),
            "seed content root is the declaring layer's dir"
        );
        // And via the trait surface used by build_sandbox.
        assert_eq!(
            Workload::mount_content_root(&svc).as_deref(),
            Some(guard.config_dir())
        );
        Ok(())
    }

    #[test]
    fn mount_policies_do_not_cross_contaminate_mounts() -> Result<()> {
        let _guard = DependsEnvGuard::new("cw-per-mount-policy", PER_MOUNT_POLICY_TOML);
        let wl = ConfigWorkload::new("svc")?;
        let workspace = wl
            .mount_policy_for("/workspace")
            .expect("workspace policy must compile");
        let data = wl
            .mount_policy_for("/data")
            .expect("data policy must compile");

        assert_eq!(
            workspace
                .decide(&crate::mount_policy::LexicalPath::new(
                    "data/node_modules/foo",
                )?)
                .decision,
            crate::mount_policy::Decision::Visible
        );
        assert_eq!(
            data.decide(&crate::mount_policy::LexicalPath::new(
                "workspace/secrets/foo",
            )?)
            .decision,
            crate::mount_policy::Decision::Visible
        );
        assert_eq!(
            workspace
                .decide(&crate::mount_policy::LexicalPath::new("node_modules/foo")?)
                .decision,
            crate::mount_policy::Decision::Masked
        );
        assert_eq!(
            data.decide(&crate::mount_policy::LexicalPath::new("secrets/foo")?)
                .decision,
            crate::mount_policy::Decision::Masked
        );
        Ok(())
    }

    const PER_MOUNT_POLICY_TOML: &str = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "alpine:latest" }
command = ["true"]

[[workloads.svc.mounts]]
host = "."
guest = "/workspace"
read_only = false
policy = { read = { deny = ["node_modules/"] } }

[[workloads.svc.mounts]]
host = "."
guest = "/data"
read_only = false
policy = { read = { deny = ["secrets/"] } }

[workloads.svc.network.defaults]
egress = "deny"
"#;

    /// F3: prepare() copies seed files from the DECLARING layer's dir
    /// (colocated seed payload), never silently from cwd.
    #[test]
    fn prepare_seeds_from_the_declaring_layer_dir() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-prepare-seed", SEED_CONFIG_TOML);
        std::fs::create_dir_all(guard.config_dir().join("seed"))?;
        std::fs::write(
            guard.config_dir().join("seed").join("settings.json"),
            "{\"seeded\":true}",
        )?;

        let svc = ConfigWorkload::new("svc")?;
        svc.prepare(&empty_seed_env_view(), false, None)?;

        let target = guard.state_dir().join("workspaces/svc-state/settings.json");
        assert_eq!(
            std::fs::read_to_string(&target)?,
            "{\"seeded\":true}",
            "seed payload must come from the declaring layer's dir"
        );
        Ok(())
    }

    /// Host Bug C shape: a seed target (`app/config/config.yaml`) with only
    /// a template mount (`${MSB_HOME}/logs`) — NOTHING covers the target.
    const UNCOVERED_SEED_CONFIG_TOML: &str = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = ["true"]

[[workloads.svc.mounts]]
host = "${MSB_HOME}/logs"
guest = "/logs"

[[workloads.svc.seed_files]]
source = "seed/config.yaml"
target = "app/config/config.yaml"
template = true

[workloads.svc.network.defaults]
egress = "deny"
"#;

    /// Host Bug C regression: prepare() must hard-error on a seed target
    /// that no declared mount host covers — BEFORE writing anything to the
    /// host (the litellm bug silently rendered into the repo working tree).
    /// Built via `synthetic_workload` (config-load validation — which fails
    /// closed on the same shape — is bypassed, exercising the prepare guard).
    #[test]
    fn prepare_hard_errors_on_uncovered_seed_target_before_writing() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-prepare-uncovered", UNCOVERED_SEED_CONFIG_TOML);
        std::fs::create_dir_all(guard.config_dir().join("seed"))?;
        std::fs::write(
            guard.config_dir().join("seed").join("config.yaml"),
            "key: value\n",
        )?;

        let mut svc = synthetic_workload(UNCOVERED_SEED_CONFIG_TOML, "svc");
        svc.seed_content_root = Some(guard.config_dir().to_path_buf());
        let err = svc
            .prepare(&empty_seed_env_view(), false, None)
            .expect_err("an uncovered seed target must hard-error in prepare()");
        let msg = err.to_string();
        assert!(
            msg.contains("workload 'svc'") && msg.contains("app/config/config.yaml"),
            "error must name the workload and the uncovered target: {msg}"
        );
        assert!(
            !guard.config_dir().join("app").exists(),
            "no stray host write may happen for a non-mount-backed target"
        );
        Ok(())
    }

    /// ADR 0030 V-addendum §V2: with a per-instance state key, prepare()
    /// seeds under `workspaces/<name>-state/<key>/...`; with `None` the
    /// target is byte-identical to the legacy path (no layout churn).
    #[test]
    fn prepare_scopes_state_seed_targets_to_the_instance_key() -> Result<()> {
        // Keyed (per-dir instance): the state target gains the key segment.
        // (Scoped block: DependsEnvGuard holds ENV_TEST_LOCK for its
        // lifetime, so the unkeyed guard below is constructed only after
        // this one drops.)
        {
            let guard = DependsEnvGuard::new("cw-prepare-seed-keyed", SEED_CONFIG_TOML);
            std::fs::create_dir_all(guard.config_dir().join("seed"))?;
            std::fs::write(
                guard.config_dir().join("seed").join("settings.json"),
                "{\"seeded\":true}",
            )?;
            let svc = ConfigWorkload::new("svc")?;
            svc.prepare(&empty_seed_env_view(), false, Some("work-1234abcd"))?;
            let keyed = guard
                .state_dir()
                .join("workspaces/svc-state/work-1234abcd/settings.json");
            assert_eq!(
                std::fs::read_to_string(&keyed)?,
                "{\"seeded\":true}",
                "keyed prepare must seed under the instance-scoped state subdir"
            );
            assert!(
                !guard
                    .state_dir()
                    .join("workspaces/svc-state/settings.json")
                    .exists(),
                "the keyed prepare must NOT write the legacy unscoped path"
            );
        }

        // Unkeyed (singleton/non-per-dir): byte-identical legacy path.
        let guard2 = DependsEnvGuard::new("cw-prepare-seed-unkeyed", SEED_CONFIG_TOML);
        std::fs::create_dir_all(guard2.config_dir().join("seed"))?;
        std::fs::write(
            guard2.config_dir().join("seed").join("settings.json"),
            "{\"seeded\":true}",
        )?;
        let svc2 = ConfigWorkload::new("svc")?;
        svc2.prepare(&empty_seed_env_view(), false, None)?;
        assert!(
            guard2
                .state_dir()
                .join("workspaces/svc-state/settings.json")
                .exists(),
            "key=None must seed the legacy path unchanged (no churn)"
        );
        Ok(())
    }

    /// F3: seed files present but NO content root resolvable (synthetic
    /// workload, no declaring layer dir, no flake project root reachable)
    /// → explicit hard error naming the workload, NEVER a silent cwd
    /// fallback.
    #[test]
    fn prepare_hard_errors_when_seed_content_root_unresolvable() -> Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(
            crate::config::test_support::HOME_ENV_KEYS,
        );
        let old_root = std::env::var("AGENTCTL_ROOT").ok();
        let old_manifest = std::env::var("CARGO_MANIFEST_DIR").ok();

        // Scrub every project_root tier: no AGENTCTL_ROOT, no
        // CARGO_MANIFEST_DIR, cwd = a flake-less temp dir.
        let cwd = crate::config::test_support::uniq_dir("cw-prepare-noroot");
        std::fs::create_dir_all(&cwd)?;
        std::env::remove_var("AGENTCTL_ROOT");
        std::env::remove_var("CARGO_MANIFEST_DIR");
        std::env::set_current_dir(&cwd)?;

        let result = synthetic_workload(SEED_CONFIG_TOML, "svc").prepare(
            &empty_seed_env_view(),
            false,
            None,
        );

        match old_root {
            Some(v) => std::env::set_var("AGENTCTL_ROOT", v),
            None => std::env::remove_var("AGENTCTL_ROOT"),
        }
        match old_manifest {
            Some(v) => std::env::set_var("CARGO_MANIFEST_DIR", v),
            None => std::env::remove_var("CARGO_MANIFEST_DIR"),
        }
        let _ = std::fs::remove_dir_all(&cwd);

        let err = result.expect_err("seed files without a content root must hard-error");
        let msg = err.to_string();
        assert!(
            msg.contains("workload 'svc'") && msg.contains("no content root"),
            "error must name the workload and the missing content root: {msg}"
        );
        Ok(())
    }

    /// F3: no seed files → prepare() is a no-op even with no content root.
    #[test]
    fn prepare_without_seed_files_is_a_noop() -> Result<()> {
        let svc = synthetic_workload(MINIMAL_SEEDLESS_TOML, "svc");
        svc.prepare(&empty_seed_env_view(), false, None)
    }

    // ---- P1.2: template = true seed files render against the env view ----

    /// A view with no vars and no defined secrets; the tests that need
    /// defined secrets / vars build the struct inline.
    fn empty_seed_env_view() -> crate::microsandbox::env::SeedEnvView {
        crate::microsandbox::env::SeedEnvView {
            vars: std::collections::HashMap::new(),
            defined_secrets: std::collections::HashSet::new(),
        }
    }

    /// P1.2: `template = true` seeds are rendered against the guest-visible
    /// env view; the target carries the fully substituted text.
    #[test]
    fn prepare_renders_template_seed_file() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-render", TEMPLATE_CONFIG_TOML);
        std::fs::create_dir_all(guard.config_dir().join("seed"))?;
        std::fs::write(
            guard.config_dir().join("seed").join("settings.json.tpl"),
            r#"{"baseUrl":"http://${LITELLM_ADDR}/v1","apiKey":"${LITELLM_MASTER_KEY}"}"#,
        )?;

        let view = crate::microsandbox::env::SeedEnvView {
            vars: std::collections::HashMap::from([
                (
                    "LITELLM_ADDR".to_string(),
                    "host.microsandbox.internal:4000".to_string(),
                ),
                (
                    "LITELLM_MASTER_KEY".to_string(),
                    "$MSB_LITELLM_MASTER_KEY".to_string(),
                ),
            ]),
            defined_secrets: std::collections::HashSet::new(),
        };
        let svc = ConfigWorkload::new("svc")?;
        svc.prepare(&view, false, None)?;

        let target = guard.state_dir().join("workspaces/svc-state/settings.json");
        assert_eq!(
            std::fs::read_to_string(&target)?,
            r#"{"baseUrl":"http://host.microsandbox.internal:4000/v1","apiKey":"$MSB_LITELLM_MASTER_KEY"}"#,
            "rendered seed must substitute every template var against the view"
        );
        Ok(())
    }

    /// P1.2: a template referencing a var NOT in the view is a hard error
    /// naming the var and the seed file.
    #[test]
    fn prepare_template_missing_var_hard_errors() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-missing", TEMPLATE_CONFIG_TOML);
        std::fs::create_dir_all(guard.config_dir().join("seed"))?;
        std::fs::write(
            guard.config_dir().join("seed").join("settings.json.tpl"),
            r#"{"baseUrl":"http://${NOPE}/v1"}"#,
        )?;

        let view = empty_seed_env_view();
        let svc = ConfigWorkload::new("svc")?;
        let err = svc
            .prepare(&view, false, None)
            .expect_err("a template referencing a missing var must hard-error");
        let msg = format!("{err}");
        assert!(msg.contains("NOPE"), "must name the missing var: {msg}");
        assert!(
            msg.contains("missing env var"),
            "must say 'missing env var': {msg}"
        );
        assert!(
            msg.contains("seed source"),
            "must name the seed file: {msg}"
        );
        Ok(())
    }

    /// P1.2: a template referencing a DEFINED-but-unbound secret is a hard
    /// error naming the secret, the "not bound" remediation, and the seed
    /// file — the real secret value never appears.
    #[test]
    fn prepare_template_unbound_secret_hard_errors() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-unbound", TEMPLATE_CONFIG_TOML);
        std::fs::create_dir_all(guard.config_dir().join("seed"))?;
        std::fs::write(
            guard.config_dir().join("seed").join("settings.json.tpl"),
            r#"{"apiKey":"${UNBOUND}"}"#,
        )?;

        let view = crate::microsandbox::env::SeedEnvView {
            vars: std::collections::HashMap::new(),
            defined_secrets: std::collections::HashSet::from(["UNBOUND".to_string()]),
        };
        let svc = ConfigWorkload::new("svc")?;
        let err = svc
            .prepare(&view, false, None)
            .expect_err("a template referencing an unbound secret must hard-error");
        let msg = format!("{err}");
        assert!(msg.contains("UNBOUND"), "must name the secret: {msg}");
        assert!(msg.contains("not bound"), "must say 'not bound': {msg}");
        assert!(
            msg.contains("seed source"),
            "must name the seed file: {msg}"
        );
        assert!(
            !msg.contains("super-secret"),
            "real secret value must never appear in the error: {msg}"
        );
        Ok(())
    }

    /// P1.2: `only_if_missing` (default true) wins over re-rendering — an
    /// existing target is left untouched even for template seeds.
    #[test]
    fn prepare_template_respects_only_if_missing() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-oim-true", TEMPLATE_CONFIG_TOML);
        std::fs::create_dir_all(guard.config_dir().join("seed"))?;
        std::fs::write(
            guard.config_dir().join("seed").join("settings.json.tpl"),
            r#"{"v":"${LITELLM_ADDR}"}"#,
        )?;
        let target = guard.state_dir().join("workspaces/svc-state/settings.json");
        std::fs::create_dir_all(target.parent().expect("target parent"))?;
        std::fs::write(&target, "stale")?;

        let view = crate::microsandbox::env::SeedEnvView {
            vars: std::collections::HashMap::from([(
                "LITELLM_ADDR".to_string(),
                "host.microsandbox.internal:4000".to_string(),
            )]),
            defined_secrets: std::collections::HashSet::new(),
        };
        let svc = ConfigWorkload::new("svc")?;
        svc.prepare(&view, false, None)?;

        assert_eq!(
            std::fs::read_to_string(&target)?,
            "stale",
            "existing target must NOT be re-rendered when only_if_missing is true"
        );
        Ok(())
    }

    /// P1.2: `only_if_missing = false` re-renders over an existing target.
    #[test]
    fn prepare_template_with_only_if_missing_false_rerenders() -> Result<()> {
        let guard = DependsEnvGuard::new(
            "cw-oim-false",
            &TEMPLATE_CONFIG_TOML.replace(
                "template = true",
                "template = true\nonly_if_missing = false",
            ),
        );
        std::fs::create_dir_all(guard.config_dir().join("seed"))?;
        std::fs::write(
            guard.config_dir().join("seed").join("settings.json.tpl"),
            r#"{"v":"${LITELLM_ADDR}"}"#,
        )?;
        let target = guard.state_dir().join("workspaces/svc-state/settings.json");
        std::fs::create_dir_all(target.parent().expect("target parent"))?;
        std::fs::write(&target, "stale")?;

        let view = crate::microsandbox::env::SeedEnvView {
            vars: std::collections::HashMap::from([(
                "LITELLM_ADDR".to_string(),
                "host.microsandbox.internal:4000".to_string(),
            )]),
            defined_secrets: std::collections::HashSet::new(),
        };
        let svc = ConfigWorkload::new("svc")?;
        svc.prepare(&view, false, None)?;

        let content = std::fs::read_to_string(&target)?;
        assert!(
            content.contains("host.microsandbox.internal:4000"),
            "stale target must be re-rendered when only_if_missing is false; got: {content}"
        );
        Ok(())
    }

    // ---- --reseed: template seeds re-render over existing targets ----

    /// `--reseed` bypasses `only_if_missing` for `template = true` seeds
    /// ONLY: the first prepare renders, a source edit without reseed keeps
    /// the stale copy, and a reseed run re-renders the edited source over
    /// the existing target.
    #[test]
    fn prepare_reseed_rerenders_existing_template_seed() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-reseed", TEMPLATE_CONFIG_TOML);
        std::fs::create_dir_all(guard.config_dir().join("seed"))?;
        let source = guard.config_dir().join("seed").join("settings.json.tpl");
        std::fs::write(&source, r#"{"v":"${LITELLM_ADDR}"}"#)?;

        let view = crate::microsandbox::env::SeedEnvView {
            vars: std::collections::HashMap::from([(
                "LITELLM_ADDR".to_string(),
                "host.microsandbox.internal:4000".to_string(),
            )]),
            defined_secrets: std::collections::HashSet::new(),
        };
        let target = guard.state_dir().join("workspaces/svc-state/settings.json");

        // First boot: the seed renders into the (missing) target.
        let svc = ConfigWorkload::new("svc")?;
        svc.prepare(&view, false, None)?;
        assert_eq!(
            std::fs::read_to_string(&target)?,
            r#"{"v":"host.microsandbox.internal:4000"}"#
        );

        // Source edited; a run WITHOUT --reseed keeps the stale copy
        // (only_if_missing semantics unchanged).
        std::fs::write(&source, r#"{"v2":"${LITELLM_ADDR}"}"#)?;
        let view2 = crate::microsandbox::env::SeedEnvView {
            vars: std::collections::HashMap::from([(
                "LITELLM_ADDR".to_string(),
                "host.microsandbox.internal:5000".to_string(),
            )]),
            defined_secrets: std::collections::HashSet::new(),
        };
        svc.prepare(&view2, false, None)?;
        assert_eq!(
            std::fs::read_to_string(&target)?,
            r#"{"v":"host.microsandbox.internal:4000"}"#,
            "without --reseed the existing target must be left untouched"
        );

        // With --reseed the template seed re-renders over the existing
        // target (edited source + CURRENT env view).
        svc.prepare(&view2, true, None)?;
        assert_eq!(
            std::fs::read_to_string(&target)?,
            r#"{"v2":"host.microsandbox.internal:5000"}"#,
            "--reseed must re-render the template seed over the existing target"
        );
        Ok(())
    }

    /// `--reseed` does NOT touch static (non-template) seeds: an existing
    /// target stays byte-identical even when the source changed.
    #[test]
    fn prepare_reseed_leaves_untemplated_seed_untouched() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-reseed-static", SEED_CONFIG_TOML);
        std::fs::create_dir_all(guard.config_dir().join("seed"))?;
        std::fs::write(
            guard.config_dir().join("seed").join("settings.json"),
            "{\"seeded\":true}",
        )?;
        let target = guard.state_dir().join("workspaces/svc-state/settings.json");
        std::fs::create_dir_all(target.parent().expect("target parent"))?;
        std::fs::write(&target, "stale")?;

        let svc = ConfigWorkload::new("svc")?;
        svc.prepare(&empty_seed_env_view(), true, None)?;

        assert_eq!(
            std::fs::read_to_string(&target)?,
            "stale",
            "--reseed must never overwrite a static (non-template) seed target"
        );
        Ok(())
    }

    /// `--reseed` leaves `only_if_missing = false` behavior unchanged: the
    /// seed already re-renders every boot, with or without the flag.
    #[test]
    fn prepare_reseed_only_if_missing_false_unchanged() -> Result<()> {
        let guard = DependsEnvGuard::new(
            "cw-reseed-oim-false",
            &TEMPLATE_CONFIG_TOML.replace(
                "template = true",
                "template = true\nonly_if_missing = false",
            ),
        );
        std::fs::create_dir_all(guard.config_dir().join("seed"))?;
        std::fs::write(
            guard.config_dir().join("seed").join("settings.json.tpl"),
            r#"{"v":"${LITELLM_ADDR}"}"#,
        )?;
        let target = guard.state_dir().join("workspaces/svc-state/settings.json");
        std::fs::create_dir_all(target.parent().expect("target parent"))?;
        std::fs::write(&target, "stale")?;

        let view = crate::microsandbox::env::SeedEnvView {
            vars: std::collections::HashMap::from([(
                "LITELLM_ADDR".to_string(),
                "host.microsandbox.internal:4000".to_string(),
            )]),
            defined_secrets: std::collections::HashSet::new(),
        };
        let svc = ConfigWorkload::new("svc")?;
        // Both with and without --reseed the target is re-rendered.
        for reseed in [false, true] {
            std::fs::write(&target, "stale")?;
            svc.prepare(&view, reseed, None)?;
            assert_eq!(
                std::fs::read_to_string(&target)?,
                r#"{"v":"host.microsandbox.internal:4000"}"#,
                "only_if_missing = false must re-render regardless of --reseed (reseed={reseed})"
            );
        }
        Ok(())
    }

    /// A `--reseed` run on a template seed whose target does NOT exist yet
    /// still seeds it (the flag only widens the overwrite case).
    #[test]
    fn prepare_reseed_seeds_missing_template_target() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-reseed-fresh", TEMPLATE_CONFIG_TOML);
        std::fs::create_dir_all(guard.config_dir().join("seed"))?;
        std::fs::write(
            guard.config_dir().join("seed").join("settings.json.tpl"),
            r#"{"v":"${LITELLM_ADDR}"}"#,
        )?;

        let view = crate::microsandbox::env::SeedEnvView {
            vars: std::collections::HashMap::from([(
                "LITELLM_ADDR".to_string(),
                "host.microsandbox.internal:4000".to_string(),
            )]),
            defined_secrets: std::collections::HashSet::new(),
        };
        let svc = ConfigWorkload::new("svc")?;
        svc.prepare(&view, true, None)?;

        let target = guard.state_dir().join("workspaces/svc-state/settings.json");
        assert_eq!(
            std::fs::read_to_string(&target)?,
            r#"{"v":"host.microsandbox.internal:4000"}"#,
            "--reseed on a missing target must seed it like a normal first boot"
        );
        Ok(())
    }

    /// P1.2 regression: an untemplated seed (template default false) is
    /// copied BYTE-IDENTICAL — a literal `${VAR}` in the source must never
    /// be rendered.
    #[test]
    fn prepare_untemplated_seed_byte_identical() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-copy", SEED_CONFIG_TOML);
        std::fs::create_dir_all(guard.config_dir().join("seed"))?;
        let source_path = guard.config_dir().join("seed").join("settings.json");
        let text = r#"{"baseUrl":"http://${LITELLM_ADDR}/v1"}"#;
        std::fs::write(&source_path, text)?;

        let view = crate::microsandbox::env::SeedEnvView {
            vars: std::collections::HashMap::from([(
                "LITELLM_ADDR".to_string(),
                "resolved.example".to_string(),
            )]),
            defined_secrets: std::collections::HashSet::new(),
        };
        let svc = ConfigWorkload::new("svc")?;
        svc.prepare(&view, false, None)?;

        let target = guard.state_dir().join("workspaces/svc-state/settings.json");
        assert_eq!(
            std::fs::read(&target)?,
            std::fs::read(&source_path)?,
            "untemplated seeds are copied byte-identical, never rendered"
        );
        Ok(())
    }

    // ---- P2.2: glob seed_files expand end-to-end in prepare() ----

    /// P2.2: a glob seed entry expands every regular-file match to
    /// `target/<rel-path>` (rel = match minus the literal glob root), sorted
    /// and preserving subdirectories; directories are never seeded.
    #[test]
    fn prepare_glob_expands_sorted_preserving_subdirs() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-glob", GLOB_CONFIG_TOML);
        // Files intentionally written out of sorted order (b before a) to
        // prove prepare() seeds deterministically; `expand_seed_glob` sorts.
        std::fs::create_dir_all(guard.config_dir().join("seed").join("a"))?;
        std::fs::write(guard.config_dir().join("seed").join("b.json"), "b")?;
        std::fs::write(
            guard.config_dir().join("seed").join("a").join("x.json"),
            "x",
        )?;
        std::fs::write(
            guard.config_dir().join("seed").join("a").join("sub.json"),
            "s",
        )?;
        // A real directory inside the match tree must never be seeded.
        std::fs::create_dir_all(guard.config_dir().join("seed").join("emptydir"))?;

        let svc = ConfigWorkload::new("svc")?;
        svc.prepare(&empty_seed_env_view(), false, None)?;

        let globbed = guard.state_dir().join("workspaces/svc-state/globbed");
        let expect = [("a/x.json", "x"), ("a/sub.json", "s"), ("b.json", "b")];
        for (rel, content) in expect {
            let target = globbed.join(rel);
            assert!(
                target.is_file(),
                "globbed target missing: {}",
                target.display()
            );
            assert_eq!(
                std::fs::read_to_string(&target)?,
                content,
                "globbed target content: {}",
                target.display()
            );
        }
        // The directory produced no file under the target.
        assert!(
            !globbed.join("emptydir").exists(),
            "a directory match must never be seeded under the target"
        );
        Ok(())
    }

    /// P2.2: a glob with no matches is a hard error at prepare time.
    #[test]
    fn prepare_glob_no_match_hard_errors() -> Result<()> {
        let _guard = DependsEnvGuard::new("cw-glob-nomatch", GLOB_CONFIG_TOML);
        // No `seed/` tree at all → the pattern matches nothing.
        let svc = ConfigWorkload::new("svc")?;
        let err = svc
            .prepare(&empty_seed_env_view(), false, None)
            .expect_err("a no-match seed glob must hard-error in prepare()");
        let msg = err.to_string();
        assert!(
            msg.contains("matched no files"),
            "error must say 'matched no files': {msg}"
        );
        assert!(msg.contains("svc"), "error must name the workload: {msg}");
        Ok(())
    }

    /// P2.2: `only_if_missing` (default true) applies PER FILE — a glob match
    /// whose target already exists is skipped while the others are seeded.
    #[test]
    fn prepare_glob_only_if_missing_applies_per_file() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-glob-oim", GLOB_CONFIG_TOML);
        std::fs::create_dir_all(guard.config_dir().join("seed"))?;
        std::fs::write(guard.config_dir().join("seed").join("a.json"), "fresh-a")?;
        std::fs::write(guard.config_dir().join("seed").join("b.json"), "fresh-b")?;
        // Pre-create the target for a.json → only b.json may be seeded.
        let globbed = guard.state_dir().join("workspaces/svc-state/globbed");
        std::fs::create_dir_all(&globbed)?;
        std::fs::write(globbed.join("a.json"), "stale")?;

        let svc = ConfigWorkload::new("svc")?;
        svc.prepare(&empty_seed_env_view(), false, None)?;

        assert_eq!(
            std::fs::read_to_string(globbed.join("a.json"))?,
            "stale",
            "existing target must NOT be overwritten when only_if_missing is true"
        );
        assert_eq!(
            std::fs::read_to_string(globbed.join("b.json"))?,
            "fresh-b",
            "the non-existing target must be seeded"
        );
        Ok(())
    }

    /// P2.2: `template = true` composes with globs — every matched file is
    /// rendered against the guest-visible env view.
    #[test]
    fn prepare_glob_and_template_compose() -> Result<()> {
        let guard = DependsEnvGuard::new("cw-glob-tpl", GLOB_TEMPLATE_CONFIG_TOML);
        std::fs::create_dir_all(guard.config_dir().join("seed").join("sub"))?;
        std::fs::write(
            guard.config_dir().join("seed").join("a.tpl"),
            "a:${LITELLM_ADDR}",
        )?;
        std::fs::write(
            guard.config_dir().join("seed").join("sub").join("b.tpl"),
            "b:${LITELLM_ADDR}",
        )?;

        let view = crate::microsandbox::env::SeedEnvView {
            vars: std::collections::HashMap::from([(
                "LITELLM_ADDR".to_string(),
                "host.microsandbox.internal:4000".to_string(),
            )]),
            defined_secrets: std::collections::HashSet::new(),
        };
        let svc = ConfigWorkload::new("svc")?;
        svc.prepare(&view, false, None)?;

        let globbed = guard.state_dir().join("workspaces/svc-state/globbed");
        assert_eq!(
            std::fs::read_to_string(globbed.join("a.tpl"))?,
            "a:host.microsandbox.internal:4000",
            "each glob match must be template-rendered"
        );
        assert_eq!(
            std::fs::read_to_string(globbed.join("sub").join("b.tpl"))?,
            "b:host.microsandbox.internal:4000",
            "nested glob matches keep their rel-path and are rendered too"
        );
        Ok(())
    }

    const MINIMAL_SEEDLESS_TOML: &str = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = ["true"]

[workloads.svc.network.defaults]
egress = "deny"
"#;

    /// Build a ConfigWorkload directly (no config load): provenance and
    /// content roots are None, simulating a synthetic layer stack.
    fn synthetic_workload(toml: &str, name: &str) -> ConfigWorkload {
        let cf: crate::config::ConfigFile =
            toml::from_str(toml).expect("synthetic config must parse");
        let workload = cf.workloads.get(name).expect("workload present").clone();
        ConfigWorkload {
            name: name.to_string(),
            workload,
            env: Vec::new(),
            secret_env: Vec::new(),
            provenance: None,
            mount_content_root: None,
            seed_content_root: None,
            depends_resolved: Vec::new(),
            mount_policies: Vec::new(),
            namespace: crate::microsandbox::port_registry::default_namespace(),
        }
    }

    // ---- F2: the lazy flake-root gate predicate ----

    /// Registry image, no local_build, no build-path mounts → NO
    /// requirement: build_sandbox must never call project_root for
    /// example-litellm.
    #[test]
    fn flake_root_requirement_none_for_plain_registry_workload() -> Result<()> {
        let _guard = TestConfigGuard::new();
        let example_litellm = ConfigWorkload::new("example-litellm")?;
        assert_eq!(
            example_litellm.flake_root_requirement(&example_litellm.plan()),
            None
        );
        Ok(())
    }

    /// nix-layered image recipe → requirement naming the feature.
    #[test]
    fn flake_root_requirement_names_nix_layered_image() -> Result<()> {
        let _guard = TestConfigGuard::new();
        let pi = ConfigWorkload::new("pi")?;
        let req = pi
            .flake_root_requirement(&pi.plan())
            .expect("nix-layered pi must require the flake root");
        assert!(req.contains("nix-layered"), "got: {req}");
        Ok(())
    }

    /// Registry image WITH local_build → requirement naming local_build.
    #[test]
    fn flake_root_requirement_names_local_build() -> Result<()> {
        let _guard = TestConfigGuard::new();
        let odysseus = ConfigWorkload::new("odysseus")?;
        let req = odysseus
            .flake_root_requirement(&odysseus.plan())
            .expect("local_build odysseus must require the flake root");
        assert!(req.contains("local_build"), "got: {req}");
        Ok(())
    }

    /// Case (c): a `${WORKESTRATE_<NAME>_BUILD}` mount requires the flake
    /// root only when the build path is a RELATIVE flake-checkout artifact
    /// (an env override to a relative path). The UNDECLARED reserved default
    /// (`.workestrate-build/<name>`, spec 21 §6.1) resolves
    /// declaring-layer-relative and needs NO flake root.
    #[test]
    fn flake_root_requirement_names_relative_build_path_mount() -> Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let toml = r#"
schema_version = 1

[workloads.svc]
kind = "service"
image = { recipe = "registry", ref = "python:3.12-slim" }
command = ["true"]

[[workloads.svc.mounts]]
host = "${WORKESTRATE_SVC_BUILD}"
guest = "/app"
read_only = true

[workloads.svc.network.defaults]
egress = "deny"
"#;
        // UNDECLARED reserved default: declaring-layer-relative → no gate.
        std::env::remove_var("WORKESTRATE_SVC_BUILD");
        let svc = synthetic_workload(toml, "svc");
        assert_eq!(svc.build_path(), ".workestrate-build/svc");
        assert_eq!(
            svc.flake_root_requirement(&svc.plan()),
            None,
            "the reserved default resolves declaring-layer-relative; no flake root needed"
        );

        // Env override to a RELATIVE path (flake-checkout artifact, e.g.
        // `agents/svc/build`) → the gate still fires.
        std::env::set_var("WORKESTRATE_SVC_BUILD", "agents/svc/build");
        let svc = synthetic_workload(toml, "svc");
        let req = svc
            .flake_root_requirement(&svc.plan())
            .expect("a relative flake-checkout build-path mount must require the flake root");
        assert!(req.contains("agents/svc/build"), "got: {req}");

        // An ABSOLUTE build path (env override to a store path) needs no root.
        std::env::set_var("WORKESTRATE_SVC_BUILD", "/nix/store/abc-build");
        let svc = synthetic_workload(toml, "svc");
        assert_eq!(svc.flake_root_requirement(&svc.plan()), None);
        std::env::remove_var("WORKESTRATE_SVC_BUILD");
        Ok(())
    }

    // ---- E0 / ADR 0028: F2 gate resolves the flake root location-independently ----

    /// A nix-layered workload declared by a directory-mode config repo: the
    /// F2 gate resolves the DECLARING repo's flake root even when the cwd is
    /// a flake-less foreign directory (ADR 0028 acceptance 1). Builds a
    /// ConfigWorkload with `mount_content_root` = the repo root (the same
    /// provenance-derived value `ConfigWorkload::new` sets for directory
    /// mode) and a `flake.nix` at the repo root.
    fn declaring_repo_fixture(label: &str) -> (PathBuf, ConfigWorkload) {
        use crate::config::test_support::uniq_dir;
        let repo = uniq_dir(label);
        std::fs::create_dir_all(&repo).unwrap();
        std::fs::write(repo.join("flake.nix"), "{}\n").unwrap();
        let toml = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "nix-layered", name = "img-pi" }
command = []

[workloads.pi.network.defaults]
egress = "deny"
"#;
        let cf: crate::config::ConfigFile = toml::from_str(toml).unwrap();
        let workload = cf.workloads.get("pi").unwrap().clone();
        let wl = ConfigWorkload {
            name: "pi".to_string(),
            workload,
            env: Vec::new(),
            secret_env: Vec::new(),
            provenance: None,
            mount_content_root: Some(repo.clone()),
            seed_content_root: None,
            depends_resolved: Vec::new(),
            mount_policies: Vec::new(),
            namespace: crate::microsandbox::port_registry::default_namespace(),
        };
        (repo, wl)
    }

    /// Test (a): the F2 gate resolves the DECLARING repo's flake root from a
    /// flake-less CWD — the ADR 0028 core fix.
    #[test]
    fn f2_gate_resolves_declaring_repo_flake_root_from_flakeless_cwd() -> Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(
            crate::config::test_support::HOME_ENV_KEYS,
        );
        let (repo, wl) = declaring_repo_fixture("f2-declaring");
        // Foreign, flake-less cwd (not the tool checkout, not the repo).
        let foreign = crate::config::test_support::uniq_dir("f2-flakeless-cwd");
        std::fs::create_dir_all(&foreign).unwrap();
        std::env::set_current_dir(&foreign)?;
        std::env::remove_var("AGENTCTL_ROOT");

        let plan = wl.plan();
        let roots = crate::microsandbox::mounts::resolve_mount_roots_owned(&wl, &plan)?;
        assert_eq!(
            roots.project_root.expect("nix-layered requires a root"),
            repo.canonicalize()?,
            "declaring repo's flake root wins over the flake-less cwd"
        );
        assert_eq!(
            roots.content_root, repo,
            "F1 content root = declaring layer dir (spec 17)"
        );

        let _ = std::fs::remove_dir_all(&repo);
        let _ = std::fs::remove_dir_all(&foreign);
        Ok(())
    }

    /// Test (b): AGENTCTL_ROOT remains the EXPLICIT override — it beats the
    /// declaring repo when set (and contains flake.nix).
    #[test]
    fn f2_gate_agentctl_root_override_wins_over_declaring_repo() -> Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(
            crate::config::test_support::HOME_ENV_KEYS,
        );
        let (repo, wl) = declaring_repo_fixture("f2-override");
        let override_root = crate::config::test_support::uniq_dir("f2-override-root");
        std::fs::create_dir_all(&override_root).unwrap();
        std::fs::write(override_root.join("flake.nix"), "{}\n").unwrap();
        std::env::set_var("AGENTCTL_ROOT", &override_root);
        // Foreign cwd so tier 3 cannot accidentally win.
        let foreign = crate::config::test_support::uniq_dir("f2-override-cwd");
        std::fs::create_dir_all(&foreign).unwrap();
        std::env::set_current_dir(&foreign)?;

        let plan = wl.plan();
        let roots = crate::microsandbox::mounts::resolve_mount_roots_owned(&wl, &plan)?;
        assert_eq!(
            roots.project_root.expect("nix-layered requires a root"),
            override_root,
            "AGENTCTL_ROOT explicit override beats the declaring repo (ADR 0028 §Decision 3)"
        );

        let _ = std::fs::remove_dir_all(&repo);
        let _ = std::fs::remove_dir_all(&override_root);
        let _ = std::fs::remove_dir_all(&foreign);
        Ok(())
    }

    /// Test (c): a SYNTHETIC layer (no declaring dir) preserves the legacy
    /// hard gate + exact error wording — byte-pinned per the repo's wording
    /// contract (the F2 error wrapper + the project_root() message).
    #[test]
    fn f2_gate_synthetic_layer_legacy_error_is_preserved() -> Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(
            crate::config::test_support::HOME_ENV_KEYS,
        );
        let old_root = std::env::var("AGENTCTL_ROOT").ok();
        let old_manifest = std::env::var("CARGO_MANIFEST_DIR").ok();
        let cwd = crate::config::test_support::uniq_dir("f2-synthetic-cwd");
        std::fs::create_dir_all(&cwd).unwrap();
        std::env::remove_var("AGENTCTL_ROOT");
        std::env::remove_var("CARGO_MANIFEST_DIR");
        std::env::set_current_dir(&cwd)?;

        // Synthetic workload: mount_content_root = None → tier 2 skipped.
        let toml = r#"
schema_version = 1

[workloads.svc]
kind = "agent"
image = { recipe = "nix-layered", name = "img-svc" }
command = []

[workloads.svc.network.defaults]
egress = "deny"
"#;
        let wl = synthetic_workload(toml, "svc");
        let plan = wl.plan();
        let err = crate::microsandbox::mounts::resolve_mount_roots_owned(&wl, &plan)
            .expect_err("no declaring dir + no AGENTCTL_ROOT + flake-less cwd must hard-error");
        let msg = err.to_string();
        assert!(
            msg.contains("workload 'svc' uses nix-layered image"),
            "F2 wrapper names the workload + feature: {msg}"
        );
        assert!(
            msg.contains("requires a flake project root"),
            "F2 wrapper wording: {msg}"
        );
        assert!(
            msg.contains("does not contain flake.nix"),
            "legacy project_root() error preserved: {msg}"
        );

        match old_root {
            Some(v) => std::env::set_var("AGENTCTL_ROOT", v),
            None => std::env::remove_var("AGENTCTL_ROOT"),
        }
        match old_manifest {
            Some(v) => std::env::set_var("CARGO_MANIFEST_DIR", v),
            None => std::env::remove_var("CARGO_MANIFEST_DIR"),
        }
        let _ = std::fs::remove_dir_all(&cwd);
        Ok(())
    }

    /// Test (d) end-to-end: preflight_existence (the plan path —
    /// lifecycle.rs:184/239) is Ok from a foreign CWD for a directory-mode
    /// nix-layered workload (ADR 0028 acceptance 1).
    #[test]
    fn preflight_existence_ok_from_foreign_cwd_for_directory_mode() -> Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(
            crate::config::test_support::HOME_ENV_KEYS,
        );
        let (repo, wl) = declaring_repo_fixture("f2-preflight");
        let foreign = crate::config::test_support::uniq_dir("f2-preflight-cwd");
        std::fs::create_dir_all(&foreign).unwrap();
        std::env::set_current_dir(&foreign)?;
        std::env::remove_var("AGENTCTL_ROOT");

        let plan = wl.plan();
        // hard = true (the `plan` command semantics): must succeed.
        let warnings = wl.preflight_existence(&plan, true)?;
        assert!(
            warnings.is_empty(),
            "no mount/seed warnings expected in this fixture: {warnings:?}"
        );

        let _ = std::fs::remove_dir_all(&repo);
        let _ = std::fs::remove_dir_all(&foreign);
        Ok(())
    }

    /// Test (g): the plan-time host-overlap WARNING fires for a `${CWD}`
    /// mount overlapping a declared state-dir mount (both rw). Warn only —
    /// plan() succeeds.
    #[test]
    fn plan_warns_on_overlapping_cwd_and_state_mount_hosts() -> Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(
            crate::config::test_support::HOME_ENV_KEYS,
        );
        let home = crate::config::test_support::uniq_dir("host-overlap-home");
        std::fs::create_dir_all(&home).unwrap();
        std::env::set_var("WORKESTRATE_HOME", &home);
        std::env::remove_var("WORKESTRATE_STATE_DIR");

        // Make the cwd a SUBDIR of the state dir that `workspaces/prime-state`
        // resolves to, so the two resolved hosts nest.
        let state = crate::config::resolve_state_dir();
        let nested = state.join("workspaces").join("prime-state").join("cwd");
        std::fs::create_dir_all(&nested).unwrap();
        std::env::set_current_dir(&nested)?;

        let toml = r#"
schema_version = 1

[workloads.svc]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[[workloads.svc.mounts]]
host = "workspaces/prime-state"
guest = "/data"
read_only = false

[[workloads.svc.mounts]]
host = "${CWD}"
guest = "/work"
read_only = false

[workloads.svc.network.defaults]
egress = "deny"
"#;
        let cf: crate::config::ConfigFile = toml::from_str(toml).unwrap();
        let workload = cf.workloads.get("svc").unwrap().clone();
        let wl = ConfigWorkload {
            name: "svc".to_string(),
            workload,
            env: Vec::new(),
            secret_env: Vec::new(),
            provenance: None,
            mount_content_root: None,
            seed_content_root: None,
            depends_resolved: Vec::new(),
            mount_policies: Vec::new(),
            namespace: crate::microsandbox::port_registry::default_namespace(),
        };
        let plan = wl.plan(); // emits the warning to stderr; assert via the helper
                              // The substituted mounts: ${CWD} = nested (absolute), workspaces/... = state join.
        let warnings = wl.host_overlap_warnings(&plan.mounts);
        assert_eq!(
            warnings.len(),
            1,
            "exactly one overlapping pair: {warnings:?}"
        );
        assert!(
            warnings[0].contains("overlaps host"),
            "warning names the overlap: {}",
            warnings[0]
        );

        let _ = std::fs::remove_dir_all(&home);
        Ok(())
    }

    /// Wrong-CWD `${CWD}` mount fix (plan level): with
    /// WORKESTRATE_INVOKE_CWD captured at CLI entry (dir A) and the process
    /// cwd elsewhere (dir B — the re-exec'd / detached child whose cwd
    /// differs from the operator's), `ConfigWorkload::plan()` resolves a
    /// `${CWD}` mount host to the captured A, so `host = "${CWD}" → /work`
    /// pins the ORIGINAL operator invocation dir.
    #[test]
    fn plan_resolves_cwd_mount_host_to_captured_invoke_cwd() -> Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let _g = crate::config::test_support::EnvGuard::capture(
            crate::config::test_support::HOME_ENV_KEYS,
        );
        let invoke = crate::config::test_support::uniq_dir("plan-invoke-cwd");
        let foreign = crate::config::test_support::uniq_dir("plan-foreign-cwd");
        std::fs::create_dir_all(&invoke).unwrap();
        std::fs::create_dir_all(&foreign).unwrap();
        std::env::set_var(crate::config::INVOKE_CWD_ENV, &invoke);
        std::env::set_current_dir(&foreign)?;

        let toml = r#"
schema_version = 1

[workloads.svc]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[[workloads.svc.mounts]]
host = "${CWD}"
guest = "/work"
read_only = false

[workloads.svc.network.defaults]
egress = "deny"
"#;
        let wl = synthetic_workload(toml, "svc");
        let plan = wl.plan();
        assert_eq!(
            plan.mounts[0].host,
            invoke.to_string_lossy(),
            "${{CWD}} mount host must resolve to the captured invocation cwd, \
             not the foreign process cwd"
        );

        let _ = std::fs::remove_dir_all(&invoke);
        let _ = std::fs::remove_dir_all(&foreign);
        Ok(())
    }
}
