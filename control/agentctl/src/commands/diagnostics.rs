//! Inspection and diagnostics commands: `plan`, `ps`, `check`,
//! `validate-config`, `generate-schema`, `generate-env-example`, and the
//! generic `run` exec entry.

use std::path::PathBuf;

use anyhow::Result;

use crate::commands::source::build_env_var_name;
use crate::config;
use crate::config::CheckEntry;
use crate::json_out::ps_entries_json;

pub fn cmd_plan<W: crate::microsandbox::workload::Workload>(
    workload: &W,
    show_source: bool,
    json: bool,
    instance: Option<&str>,
) -> Result<()> {
    use crate::microsandbox::slots::{instance_name, slot_for, validate_instance_id};

    // ADR 0026(d)/ADR 0030 P2.1: depends_on resolution happens INSIDE the
    // caller's workload construction — main.rs builds the plan workload via
    // `ConfigWorkload::new_with_use_overrides_and_instance` with the parsed
    // `--use` overrides and the explicit `--instance <id>` passthrough, so
    // `workload.plan()` ALREADY reflects them. cmd_plan deliberately does
    // NOT re-construct: an earlier `plan_holder` re-construction resolved
    // depends_on a SECOND time per invocation, double-printing every
    // resolution warning (e.g. the per-IP bind warning on `plan --use`).
    let effective: &dyn crate::microsandbox::workload::Workload = workload;

    // ADR 0026/C2: `--instance <id>` renders the plan as the parallel slot
    // `<slot>@<id>` would see it — the instance name plus the prospective
    // per-instance bind IP on every published port. The prospective bind is
    // a READ-ONLY registry snapshot (no lock, no reservation): a preview of
    // what `up --instance <id>` would draw right now.
    let mut plan = effective.plan();
    if let Some(id) = instance {
        validate_instance_id(id)?;
        let slot = slot_for(workload.name(), config::active_context_name().as_deref());
        plan.name = instance_name(&slot, Some(id));
        let state_dir = config::resolve_state_dir();
        let bind = crate::microsandbox::port_registry::prospective_loopback_ip(&state_dir)?;
        for p in &mut plan.ports {
            p.bind_ip = bind;
        }
    }

    // Plan-time existence preflight (security-model enforcement point; see
    // docs/migration/30-security-model.md §enforcement-points): fail fast on
    // a missing read-only mount source or missing seed source BEFORE rendering
    // — the exact failure-1 signal (a doubled/wrong-root path that points
    // nowhere). Read-write mounts and local_build fallbacks warn only.
    for w in effective.preflight_existence(&plan, true)? {
        eprintln!("warning: {w}");
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else if show_source {
        if instance.is_some() {
            // The prospective parallel view has no per-field source
            // annotation; render the plain plan (source view stays the
            // singleton's).
            println!("{}", plan);
        } else {
            println!("{}", workload.show_source());
        }
    } else {
        println!("{}", plan);
    }
    Ok(())
}

/// One row of the `workestrate workloads` listing (ADR 0027): the configured
/// workload name, its kind, a short image summary, the instance names
/// currently registered in the port registry (empty = not running), and the
/// declared instance policy / namespace columns (ADR 0030 §4.4).
#[derive(Debug, Clone)]
pub struct WorkloadListEntry {
    pub name: String,
    pub kind: String,
    pub image: String,
    pub instances: Vec<String>,
    /// The workload's declaring config-repo namespace (ADR 0030 Phase 2 T1).
    pub namespace: String,
    /// The active context at listing time (G5): the same for every row —
    /// the listing is the active context's config view. None under
    /// bare-layers backward-compat.
    pub context: Option<String>,
    /// The workload's declared instance strategy (from config; "singleton"
    /// default).
    pub strategy: String,
    /// The workload's declared on_conflict chain (from config; default
    /// ["reuse","start","replace"]).
    pub on_conflict: String,
    /// The workload's declared instance.port policy (from config; "fixed"
    /// default).
    pub port: String,
    /// The workload's declared instance.label (from config; None default).
    pub label: Option<String>,
}

/// Short image summary for the listing: `<recipe>:<ref>`, falling back to
/// `name[:tag]`, then the bare recipe when no ref/name/tag is set.
fn image_summary(image: &crate::config::ImageSpec) -> String {
    let detail = image
        .reference
        .clone()
        .or_else(|| match (&image.name, &image.tag) {
            (Some(n), Some(t)) => Some(format!("{n}:{t}")),
            (Some(n), None) => Some(n.clone()),
            (None, Some(t)) => Some(t.clone()),
            (None, None) => None,
        });
    match detail {
        Some(d) => format!("{}:{}", image.recipe, d),
        None => image.recipe.clone(),
    }
}

/// Derive the display strings for a workload's instance policy (ADR 0030
/// §4.4): `(strategy, on_conflict, port, label)`. Defaults mirror the
/// reconcile chain / plan display: strategy "singleton", on_conflict
/// "reuse,start,replace", port "fixed", label None.
fn policy_strings(
    policy: &crate::config::InstancePolicy,
) -> (String, String, String, Option<String>) {
    let strategy = policy.strategy.to_string();
    let on_conflict = policy
        .on_conflict
        .as_ref()
        .map(|c| {
            c.0.iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_else(|| "reuse,start,replace".to_string());
    let port = policy
        .port
        .as_ref()
        .map(crate::microsandbox::plan::render_instance_port)
        .unwrap_or_else(|| "fixed".to_string());
    let label = policy.label.clone();
    (strategy, on_conflict, port, label)
}

/// `workestrate workloads` (ADR 0027): list every configured workload with
/// its kind, image summary, and running status (instances registered in the
/// port registry). Deterministic order: sorted by workload name (the config
/// workloads map is unordered). Running status is registry-based (no msb
/// liveness probe) — this is a discovery view, not a health check.
pub fn cmd_workloads(json: bool) -> Result<()> {
    let cfg = config::load_config()?;
    let state_dir = config::resolve_state_dir();
    let records = crate::microsandbox::port_registry::list_records(&state_dir)?;

    let mut names: Vec<&String> = cfg.workloads.keys().collect();
    names.sort();
    let provenance = crate::merge::get_provenance();
    let layer_dirs = crate::merge::get_layer_dirs().unwrap_or_default();
    // G5: the listing is the active context's config view — one value for
    // every row.
    let active_context = crate::config::active_context_name();
    let entries: Vec<WorkloadListEntry> = names
        .into_iter()
        .map(|name| {
            let wl = &cfg.workloads[name];
            let mut instances: Vec<String> = records
                .iter()
                .filter(|r| &r.workload == name)
                .map(|r| r.instance.clone())
                .collect();
            instances.sort();
            let (strategy, on_conflict, port, label) = policy_strings(&wl.instance);
            WorkloadListEntry {
                name: name.clone(),
                kind: wl.kind.clone(),
                image: image_summary(&wl.image),
                instances,
                namespace: crate::commands::deps::namespace_for(
                    provenance.as_ref(),
                    &layer_dirs,
                    name,
                ),
                context: active_context.clone(),
                strategy,
                on_conflict,
                port,
                label,
            }
        })
        .collect();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&crate::json_out::workloads_json(&entries))?
        );
    } else {
        print_workloads_text_to(&entries, &mut std::io::stdout())?;
    }
    Ok(())
}

/// Render `workloads` rows to `out`. Pure I/O: no env, no registry.
/// `cmd_workloads` passes `&mut std::io::stdout()`; tests pass a `Vec<u8>`.
pub fn print_workloads_text_to<W: std::io::Write>(
    entries: &[WorkloadListEntry],
    out: &mut W,
) -> std::io::Result<()> {
    if entries.is_empty() {
        writeln!(out, "(no configured workloads)")?;
        return Ok(());
    }
    for e in entries {
        let running = if e.instances.is_empty() {
            "(none running)".to_string()
        } else {
            format!("running: {}", e.instances.join(", "))
        };
        let label = e
            .label
            .as_deref()
            .map(|l| format!(" label={l}"))
            .unwrap_or_default();
        let context = e.context.as_deref().unwrap_or("-");
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}{}",
            e.name,
            e.kind,
            e.image,
            running,
            e.namespace,
            context,
            e.strategy,
            e.on_conflict,
            e.port,
            label,
        )?;
    }
    Ok(())
}

pub async fn cmd_ps(json: bool) -> Result<()> {
    use crate::microsandbox::runtime::{classify_status, gather_facts, probe_liveness, ps};
    let state_dir = crate::config::resolve_state_dir();
    let mut entries = ps(&state_dir)?;
    // Best-effort liveness probe (ADR 0021 §4). ps() stays pure; this is the
    // only place ps rows touch msb. When the msb DB is unreachable we cannot
    // distinguish running from stale, so emit one honest stderr note rather
    // than silently trusting the registry records.
    let (unreachable, unknown) = probe_liveness(&mut entries).await;
    if unreachable > 0 {
        eprintln!(
            "warning: could not verify liveness of {n} instance(s) via msb (db unreachable); stale flags may be inaccurate",
            n = unreachable,
        );
    }
    // FS-7: unexpected (non-reachability) probe errors were already noted
    // per-instance on stderr by probe_liveness; one aggregate line here makes
    // the count visible without repeating the per-instance detail.
    if unknown > 0 {
        eprintln!(
            "warning: liveness probe returned an unexpected error for {n} instance(s) (see notes above); stale flags left unchanged",
            n = unknown,
        );
    }
    // ADR 0030 §4.4: populate the reconciled 5-state status per entry from the
    // shared reconcile facts (registry record + msb status + host-port
    // liveness). Best-effort: a fact-gathering error leaves status None.
    for e in entries.iter_mut() {
        let declared_ports: Vec<u16> = e.ports.iter().map(|p| p.host).collect();
        match gather_facts(&state_dir, &e.instance, &declared_ports).await {
            Ok(facts) => e.status = Some(classify_status(&facts)),
            Err(err) => {
                eprintln!(
                    "note: could not gather reconcile facts for '{}' ({}); status omitted",
                    e.instance, err
                );
            }
        }
    }
    // ADR 0032 §Provenance stamps: config-hash staleness per entry, computed
    // only where CHEAPLY DERIVABLE from the active config view. Pre-stamp
    // records and unresolvable rows display nothing (honest unknown).
    apply_config_staleness(&mut entries);
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&ps_entries_json(&entries))?
        );
    } else {
        print_ps_text(&entries)?;
    }
    Ok(())
}

/// ADR 0032 §Provenance stamps: compute per-entry config staleness against
/// the ACTIVE config view. For each entry whose workload resolves in the
/// merged config under the record's OWN namespace, build the current plan
/// view (the same deterministic mutations the up path applies — instance
/// state scoping + name override, via the shared runtime helper) and compare
/// its config hash with the record's stamp. Honest-unknown posture:
/// - pre-stamp records (`config_hash: None`) are NEVER compared (never
///   auto-stale) — nothing displayed;
/// - a workload gone from the active config, a FOREIGN-NAMESPACE record,
///   a config-load failure, or a workload-construction failure (e.g. a
///   required dep not running) all leave `staleness` None — no display.
///   Equal hashes also leave None (only a real divergence is displayed).
fn apply_config_staleness(entries: &mut [crate::microsandbox::runtime::PsEntry]) {
    let cfg = match config::load_config() {
        Ok(cfg) => cfg,
        Err(_) => return, // no active view → honest unknown for every row
    };
    // get_provenance (non-draining) + layer dirs resolve each workload's
    // declaring-repo namespace exactly as ConfigWorkload::new would.
    let provenance = crate::merge::get_provenance();
    let layer_dirs = crate::merge::get_layer_dirs().unwrap_or_default();
    for e in entries.iter_mut() {
        let Some(recorded) = e.config_hash.clone() else {
            continue; // pre-stamp record: never auto-stale, nothing displayed
        };
        if !cfg.workloads.contains_key(&e.workload) {
            continue; // workload gone from the active config
        }
        let namespace =
            crate::commands::deps::namespace_for(provenance.as_ref(), &layer_dirs, &e.workload);
        if namespace != e.namespace {
            continue; // foreign-namespace record: not this config's row
        }
        // Full construction (dep resolution included); failures → unknown.
        let Ok(wl) = crate::microsandbox::workload::ConfigWorkload::new(&e.workload) else {
            continue;
        };
        let current =
            crate::microsandbox::runtime::current_config_hash_for_workload(&wl, &e.instance);
        if current != recorded {
            e.staleness = Some(crate::microsandbox::runtime::ConfigStaleness { recorded, current });
        }
    }
}

pub fn print_ps_text(entries: &[crate::microsandbox::runtime::PsEntry]) -> Result<()> {
    // Thin stdout wrapper over the writer-injectable renderer, so the footer
    // text and table layout are unit-testable without capturing global stdout.
    print_ps_text_to(entries, &mut std::io::stdout())?;
    Ok(())
}

/// Render `ps` rows (table + stale-remediation footer) to `out`. Pure I/O:
/// no msb, no env. `cmd_ps` passes `&mut std::io::stdout()`; tests pass a
/// `Vec<u8>`. Returns io::Error on write failure (propagated as anyhow).
pub fn print_ps_text_to<W: std::io::Write>(
    entries: &[crate::microsandbox::runtime::PsEntry],
    out: &mut W,
) -> std::io::Result<()> {
    if entries.is_empty() {
        writeln!(out, "(no running workestrate instances)")?;
        return Ok(());
    }
    // Stable column layout: INSTANCE | WORKLOAD | CONTEXT | PORTS | STATUS |
    // STARTED (CREATED was renamed to STARTED to match the ADR 0021 §7
    // `started_at` field; STATUS is the ADR 0030 §4.4 reconciled 5-state
    // status — a zombie shows `running-unhealthy`, not `Running`).
    writeln!(
        out,
        "{:<32} {:<16} {:<12} {:<24} {:<18} STARTED",
        "INSTANCE", "WORKLOAD", "CONTEXT", "PORTS", "STATUS"
    )?;
    let mut sorted: Vec<_> = entries.iter().collect();
    sorted.sort_by(|a, b| a.instance.cmp(&b.instance));
    for e in sorted {
        // ADR 0026/C3: a pair renders `<bind_ip>:<host>:<guest>` when bound
        // on a non-default bind (parallel slot), else the legacy
        // `<host>:<guest>` (singleton rows stay clean). Matches the plan
        // Display rule (C2).
        let ports_str = e
            .ports
            .iter()
            .map(|p| {
                // P3: a port name (when present) prefixes the pair
                // (`api:4000:4000`, or `api:127.0.0.2:14000:4000` on a
                // non-default bind); unnamed pairs keep the legacy form
                // byte-identical.
                let name_prefix = p
                    .name
                    .as_deref()
                    .map(|n| format!("{n}:"))
                    .unwrap_or_default();
                if p.bind_ip == crate::microsandbox::plan::default_bind_ip() {
                    format!("{}{}:{}", name_prefix, p.host, p.guest)
                } else {
                    format!("{}{}:{}:{}", name_prefix, p.bind_ip, p.host, p.guest)
                }
            })
            .collect::<Vec<_>>()
            .join(",");
        let started_display = if e.started_at.is_empty() {
            "-".to_string()
        } else {
            e.started_at.clone()
        };
        // ADR 0030 §4.4: reconciled 5-state status; `-` when the async caller
        // could not gather facts (or from the pure `ps()` path).
        let status_display = e.status.map(|s| s.as_str()).unwrap_or("-");
        // ADR 0032 §Provenance stamps: config-hash drift suffix, matching
        // the ADR's `stale (config a1b2 → current d4e5)` shape (4-char
        // prefixes of the FULL hashes). Absent for pre-stamp/unresolvable
        // rows.
        let staleness_display = e
            .staleness
            .as_ref()
            .map(|s| {
                use crate::microsandbox::provenance::short_hash;
                format!(
                    " stale (config {} → current {})",
                    short_hash(&s.recorded),
                    short_hash(&s.current)
                )
            })
            .unwrap_or_default();
        writeln!(
            out,
            "{:<32} {:<16} {:<12} {:<24} {:<18} {}{}",
            e.instance,
            e.workload,
            e.context.clone().unwrap_or_else(|| "-".into()),
            ports_str,
            status_display,
            started_display,
            staleness_display,
        )?;
    }

    // ADR 0021 §4: stale state records (registry entry exists but the backing
    // sandbox is gone) get a remediation footer naming the precise teardown
    // command per instance, plus the catch-all `down --all`.
    let stale: Vec<&crate::microsandbox::runtime::PsEntry> =
        entries.iter().filter(|e| e.stale).collect();
    if !stale.is_empty() {
        writeln!(out)?;
        writeln!(
            out,
            "{} stale instance(s): registry record exists but the sandbox is not running.",
            stale.len()
        )?;
        for e in &stale {
            // Parallel instance (slot@id) → `down --instance <id>`;
            // singleton → bare `down`. Uses the id, not the full instance
            // name. Canonical verb-first form (ADR 0027; cleanup phase 4
            // removed the typed `<name>` subcommands).
            if let Some((_, id)) = e.instance.split_once('@') {
                writeln!(
                    out,
                    "  workestrate workload down {} --instance {}",
                    e.workload, id
                )?;
            } else {
                writeln!(out, "  workestrate workload down {}", e.workload)?;
            }
        }
        writeln!(out, "Remove every instance with: workestrate down-all")?;
    }
    Ok(())
}

/// One row of `workestrate instances` (ADR 0030 §4.4): the reconciled
/// instance view across the registry + msb.
#[derive(Debug, Clone)]
pub struct InstanceEntry {
    pub instance: String,
    pub workload: String,
    pub namespace: String,
    pub context: Option<String>,
    pub slot: String,
    pub kind: crate::microsandbox::runtime::PsKind,
    pub status: crate::microsandbox::runtime::InstanceStatus,
    pub ports: Vec<crate::microsandbox::plan::PortMapping>,
    pub started_at: String,
    /// The workload's declared instance strategy (from config; "singleton"
    /// default).
    pub strategy: String,
    /// The workload's declared on_conflict chain (from config; default
    /// ["reuse","start","replace"]).
    pub on_conflict: String,
    /// The workload's declared instance.port policy (from config; "fixed"
    /// default).
    pub port: String,
    /// The workload's declared instance.label (from config; None default).
    pub label: Option<String>,
}

impl InstanceEntry {
    /// The human-readable status string for the text renderer (ADR 0030 §4.4).
    pub fn status_str(&self) -> &'static str {
        self.status.as_str()
    }
}

/// `workestrate instances` (ADR 0030 §4.4): list every registry record with
/// its reconciled 5-state status (registry record + msb status + host-port
/// liveness), grouped by workload in text mode or as an extended record array
/// in JSON mode. Optional `<workload>` filter. Deterministic order: sorted by
/// workload, then instance.
pub async fn cmd_instances(workload_filter: Option<&str>, json: bool) -> Result<()> {
    let cfg = config::load_config()?;
    let state_dir = config::resolve_state_dir();
    let records = crate::microsandbox::port_registry::list_records(&state_dir)?;
    let mut entries = Vec::new();
    for r in records {
        if let Some(filter) = workload_filter
            && r.workload != filter
        {
            continue;
        }
        let declared_ports: Vec<u16> = r.ports.clone();
        let facts = crate::microsandbox::runtime::reconcile::gather_facts(
            &state_dir,
            &r.instance,
            &declared_ports,
        )
        .await?;
        let status = crate::microsandbox::runtime::classify_status(&facts);
        let wl = cfg.workloads.get(&r.workload);
        let policy = wl.map(|w| &w.instance);
        let strategy = policy
            .map(|p| p.strategy.to_string())
            .unwrap_or_else(|| "singleton".to_string());
        let on_conflict = policy
            .and_then(|p| p.on_conflict.as_ref())
            .map(|c| {
                c.0.iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .unwrap_or_else(|| "reuse,start,replace".to_string());
        let port = policy
            .and_then(|p| p.port.as_ref())
            .map(crate::microsandbox::plan::render_instance_port)
            .unwrap_or_else(|| "fixed".to_string());
        let label = policy.and_then(|p| p.label.clone());
        let kind = if crate::microsandbox::slots::instance_id_of(&r.instance).is_some() {
            crate::microsandbox::runtime::PsKind::Parallel
        } else {
            crate::microsandbox::runtime::PsKind::Singleton
        };
        let slot = crate::microsandbox::slots::slot_of_instance(&r.instance).to_string();
        let ports = if r.port_pairs.is_empty() {
            r.ports
                .iter()
                .map(|&h| crate::microsandbox::plan::PortMapping::new(h, h))
                .collect()
        } else {
            r.port_pairs.clone()
        };
        entries.push(InstanceEntry {
            instance: r.instance.clone(),
            workload: r.workload.clone(),
            namespace: r.namespace.clone(),
            context: r.context.clone(),
            slot,
            kind,
            status,
            ports,
            started_at: r.created_at.clone(),
            strategy,
            on_conflict,
            port,
            label,
        });
    }
    entries.sort_by(|a, b| {
        a.workload
            .cmp(&b.workload)
            .then(a.instance.cmp(&b.instance))
    });
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&crate::json_out::instances_json(&entries))?
        );
    } else {
        print_instances_text_to(&entries, &mut std::io::stdout())?;
    }
    Ok(())
}

/// Render `instances` rows to `out`, grouped by workload (ADR 0030 §4.4).
/// Pure I/O: no env, no registry. `cmd_instances` passes
/// `&mut std::io::stdout()`; tests pass a `Vec<u8>`.
pub fn print_instances_text_to<W: std::io::Write>(
    entries: &[InstanceEntry],
    out: &mut W,
) -> std::io::Result<()> {
    if entries.is_empty() {
        writeln!(out, "(no instances)")?;
        return Ok(());
    }
    let mut current_workload: Option<&str> = None;
    for e in entries {
        if current_workload != Some(e.workload.as_str()) {
            if current_workload.is_some() {
                writeln!(out)?;
            }
            writeln!(out, "{}:", e.workload)?;
            current_workload = Some(e.workload.as_str());
        }
        let ports = e
            .ports
            .iter()
            .map(|p| p.host.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let label = e
            .label
            .as_deref()
            .map(|l| format!(" label={l}"))
            .unwrap_or_default();
        // G5: the record's context column, `-` when None (legacy
        // unknown-context records).
        let context = e.context.as_deref().unwrap_or("-");
        writeln!(
            out,
            "  {}\t{}\t{}\t{}\tports=[{}]\tstrategy={}\ton_conflict=[{}]\tport={}{}",
            e.instance,
            e.status_str(),
            e.namespace,
            context,
            ports,
            e.strategy,
            e.on_conflict,
            e.port,
            label,
        )?;
    }
    Ok(())
}

pub fn cmd_check() -> Result<()> {
    println!("=== workestrate check ===\n");
    let mut all_ok = true;

    let registry_path = config::registry_path();
    if registry_path.exists() {
        println!("Registry: {} [OK]", registry_path.display());
        match config::load_registry()? {
            None => {
                println!("  (registry exists but could not be loaded)");
                all_ok = false;
            }
            Some(registry) => {
                println!("  Config repos:");
                if registry.configs.is_empty() {
                    println!("    (none)");
                } else {
                    for status in crate::git::collect_repo_statuses(&registry) {
                        let entry = &registry.configs[&status.name];
                        let (dirty_label, ok) = if status.exists {
                            if status.dirty {
                                ("dirty", false)
                            } else {
                                ("clean", true)
                            }
                        } else {
                            ("missing", false)
                        };
                        let git_ref = entry.r#ref.as_deref().unwrap_or("main");
                        let mark = if ok { "[OK]" } else { "[MISSING]" };
                        println!(
                            "    {}: {} (ref {}, rev {}, {}) {}",
                            status.name, entry.url, git_ref, status.short, dirty_label, mark
                        );
                    }
                }
                println!("  Layers: {:?}", registry.layers);
                println!("  Trusted projects:");
                if registry.trusted_projects.is_empty() {
                    println!("    (none)");
                } else {
                    for p in &registry.trusted_projects {
                        let path = PathBuf::from(&p.path);
                        let status = if path.exists() { "[OK]" } else { "[MISSING]" };
                        println!("    {} {}", p.path, status);
                    }
                }
            }
        }
    } else {
        println!("Registry: {} [MISSING]", registry_path.display());
        all_ok = false;
    }

    // Active context
    match config::resolve_active_context() {
        Ok(ctx) => {
            if let Some(ref name) = ctx.name {
                println!("Active context: {} [OK]", name);
            } else {
                println!("Active context: (none — using bare layers)");
            }
            println!("  Layers: {:?}", ctx.layers);
        }
        Err(e) => {
            println!("Active context: [ERROR] {}", e);
            all_ok = false;
        }
    }

    println!("\nTool home:");
    let (home, kind) = config::resolve_home_with_kind();
    println!("  home: {} ({:?})", home.display(), kind);
    let dirs: Vec<(&str, std::path::PathBuf)> = match kind {
        config::HomeKind::LegacyXdg => vec![
            ("config", config::xdg_config_dir()),
            ("data", config::xdg_data_dir()),
            ("state", config::xdg_state_dir()),
        ],
        _ => vec![
            ("registry", config::registry_path()),
            ("secrets", home.join("secrets")),
            ("config-repos", home.join("config-repos")),
            ("state", home.join("state")),
        ],
    };
    for (label, dir) in dirs {
        let ok = dir.exists();
        let status = if ok { "[OK]" } else { "[MISSING]" };
        println!("  {}: {} {}", label, dir.display(), status);
        if !ok {
            all_ok = false;
        }
    }

    println!("\nSource overrides:");
    match config::load_config() {
        Ok(cfg) => {
            let mut found = false;
            for (name, workload) in &cfg.workloads {
                if workload.local_build.is_none() {
                    continue;
                }
                found = true;
                let env_var = build_env_var_name(name);
                let repo = config::source_store_dir(name).join("repo");
                let build = config::source_store_dir(name).join("build");
                let repo_status = if repo.exists() {
                    "checked out"
                } else {
                    "not checked out"
                };
                let build_status = if build.exists() { "built" } else { "not built" };
                match std::env::var(&env_var) {
                    Ok(path) => println!(
                        "  {}: override {} (repo: {}, build: {})",
                        name, path, repo_status, build_status
                    ),
                    Err(_) => println!(
                        "  {}: no override ({} not set) (repo: {}, build: {})",
                        name, env_var, repo_status, build_status
                    ),
                }
            }
            if !found {
                println!("  (none)");
            }
        }
        Err(e) => {
            println!("  (could not load config: {})", e);
            all_ok = false;
        }
    }

    println!("\nReference config:");
    let reference = find_reference_config();
    match reference {
        Some(path) if path.exists() => println!("  {} [OK]", path.display()),
        Some(path) => {
            println!("  {} [MISSING]", path.display());
            all_ok = false;
        }
        None => {
            println!("  (could not resolve reference config)");
            all_ok = false;
        }
    }

    println!("\nRequired files:");
    // WP5 / E1: project_root_optional() returns None when workestrate is
    // invoked outside a workbench checkout (the documented standalone-tool
    // install path). Previously cmd_check hard-failed on this, breaking the
    // first command after `nix profile install`. We now skip the
    // workbench-layout checks with a note and let the registry / XDG /
    // reference-config checks above stand on their own.
    match config::project_root_optional() {
        Some(root) => {
            let checks = config::check_required_files(&root)?;
            let mut had_missing_required = false;
            for entry in &checks {
                print_entry(entry);
                if !entry.ok && !entry.optional {
                    had_missing_required = true;
                }
            }
            if had_missing_required {
                all_ok = false;
            }
        }
        None => {
            println!("  (not in a workbench checkout — skipped)");
        }
    }

    if all_ok {
        println!("\nAll checks passed.");
        Ok(())
    } else {
        Err(anyhow::anyhow!("Some checks failed."))
    }
}

fn print_entry(entry: &CheckEntry) {
    if entry.ok {
        println!("[OK] {}", entry.label);
        return;
    }
    if entry.optional {
        println!("[MISSING] (optional) {}", entry.label);
    } else {
        println!("[MISSING] {}", entry.label);
    }
}

pub fn find_reference_config() -> Option<PathBuf> {
    // WP5 / E1: project_root_optional() returns None for standalone installs;
    // we then fall through to the CARGO_MANIFEST_DIR probe (cargo run / test)
    // and finally return None so cmd_check can print "(could not resolve
    // reference config)" instead of bailing.
    if let Some(root) = config::project_root_optional() {
        let path = root.join("config.reference").join("workestrate.toml");
        if path.exists() {
            return Some(path);
        }
    }
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let mut path = PathBuf::from(manifest);
        if path.pop() && path.pop() {
            let reference = path.join("config.reference").join("workestrate.toml");
            if reference.exists() {
                return Some(reference);
            }
        }
    }
    None
}

pub fn cmd_validate_config() -> Result<()> {
    let config = config::load_config()?;
    config::validate_config(&config)?;
    println!("workestrate.toml is valid.");
    // Warn-only existence preflight across all workloads (security-model
    // enforcement point). Synthetic/reference configs may legitimately lack
    // the referenced files, so this NEVER bails — it surfaces missing mount
    // sources / seed sources / local_build fallbacks as warnings. Derives
    // content roots from the process-global provenance + layer_dirs captured
    // by load_config() (no ConfigWorkload construction, so depends_on
    // resolution does not refuse synthetic configs).
    for w in preflight_config_warnings(&config) {
        eprintln!("warning: {w}");
    }
    Ok(())
}

/// Warn-only plan-time existence preflight for `validate-config`. Derives
/// each workload's mount/seed/local_build content root from the process-global
/// provenance + layer_dirs (set by `load_config`), resolves the referenced
/// paths via [`crate::microsandbox::mounts::resolve_mount_host`], and collects
/// missing ones as warnings. Never bails (synthetic/reference configs may
/// legitimately lack the files).
fn preflight_config_warnings(config: &crate::config::ConfigFile) -> Vec<String> {
    use crate::microsandbox::mounts::{MountRootsOwned, resolve_mount_host};
    let provenance = crate::merge::get_provenance();
    let layer_dirs = crate::merge::get_layer_dirs().unwrap_or_default();
    let project_root = crate::config::project_root_optional();
    let mut warnings = Vec::new();
    for (name, wl) in &config.workloads {
        let mount_root = provenance
            .as_ref()
            .and_then(|p| p.get(&format!("workloads.{name}.mounts")))
            .and_then(|l| layer_dirs.get(l));
        let seed_root = provenance
            .as_ref()
            .and_then(|p| p.get(&format!("workloads.{name}.seed_files")))
            .and_then(|l| layer_dirs.get(l));
        let build_root = provenance
            .as_ref()
            .and_then(|p| p.get(&format!("workloads.{name}.local_build")))
            .and_then(|l| layer_dirs.get(l));
        let content_root = mount_root
            .or(build_root)
            .cloned()
            .unwrap_or_else(|| crate::config::invoke_cwd().unwrap_or_default());
        let owned = MountRootsOwned {
            content_root,
            project_root: project_root.clone(),
            flake_build_path: None,
        };
        let roots = owned.as_roots();
        // Mounts: resolve each host and check existence (warn-only).
        for m in &wl.mounts {
            if let Ok(path) = resolve_mount_host(&roots, &m.host)
                && !path.exists()
            {
                if m.is_read_only() {
                    warnings.push(format!(
                            "workload '{name}': read-only mount source does not exist: {} (host = {:?})",
                            path.display(),
                            m.host
                        ));
                } else {
                    warnings.push(format!(
                            "workload '{name}': read-write mount source does not exist (will be created at runtime): {}",
                            path.display()
                        ));
                }
            }
        }
        // Seeds: resolve against the seed content root (or content_root /
        // project_root fallback, mirroring ConfigWorkload::prepare).
        let seed_base = seed_root
            .or(mount_root)
            .or(build_root)
            .or(project_root.as_ref());
        for seed in &wl.seed_files {
            let Some(source) = &seed.source else {
                // Glob entry: no single source path — expand the pattern and
                // warn when it matches nothing or fails to expand. Mirrors
                // preflight_existence's glob branch (warn-only here).
                if let Some(glob) = &seed.glob {
                    match seed_base {
                        Some(root) => {
                            match crate::microsandbox::mounts::expand_seed_glob(root, glob) {
                                Ok(exp) if exp.files.is_empty() => warnings.push(format!(
                                    "workload '{name}': seed_files glob matched no files: {} (root = {})",
                                    glob,
                                    root.display()
                                )),
                                Ok(_) => {}
                                Err(e) => warnings.push(format!(
                                    "workload '{name}': seed_files glob expansion failed: {} (root = {})",
                                    e,
                                    root.display()
                                )),
                            }
                        }
                        None => warnings.push(format!(
                            "workload '{name}': seed_files glob {} cannot be resolved (no content root)",
                            glob
                        )),
                    }
                }
                continue;
            };
            if let Some(root) = seed_base {
                let src = root.join(source);
                if !src.exists() {
                    warnings.push(format!(
                        "workload '{name}': seed source does not exist: {} (source = {:?})",
                        src.display(),
                        source
                    ));
                }
            } else {
                warnings.push(format!(
                    "workload '{name}': seed source {:?} cannot be resolved (no content root)",
                    source
                ));
            }
        }
        // local_build fallback: warn if missing (a build output).
        if let Some(lb) = &wl.local_build
            && let Some(fallback) = &lb.fallback
        {
            let resolved = if std::path::Path::new(fallback).is_absolute() {
                Some(std::path::PathBuf::from(fallback))
            } else {
                project_root
                    .as_deref()
                    .or(Some(owned.content_root.as_path()))
                    .map(|root| root.join(fallback))
            };
            if let Some(p) = resolved
                && !p.exists()
            {
                warnings.push(format!(
                    "workload '{name}': local_build fallback does not exist (will be built): {}",
                    p.display()
                ));
            }
        }
    }
    warnings
}

/// Title applied to the generated workload subschema — matches the
/// established capsule-file convention (the previous hand-derived
/// `schemas/workestrate-workload.schema.json` carried this exact title).
pub(crate) const WORKLOAD_SCHEMA_TITLE: &str = "workestrate workload capsule entry file (workestrate/workloads/<name>/workload.toml, bare table form)";

/// Generate the canonical full schema (`workestrate.toml`), the
/// bare-workload subschema (a workload capsule file), and the tool-home
/// registry schema (`config.toml`). All three derive from the same
/// schemars-annotated config types — the single source of truth
/// (ADR 0021 §8); the subschema replaces the previous hand-derived jq rule.
pub(crate) fn generate_schema_pair() -> Result<(String, String)> {
    // Full schema: schemars-derived ConfigFile (identical to the historical
    // `cmd_generate_schema` behavior).
    let schema = schemars::schema_for!(crate::config::ConfigFile);
    let full = serde_json::to_string_pretty(&schema)?;

    // Workload subschema: schemars-derived WorkloadConfig plus the
    // post-processing that matched the previous hand-derived file exactly.
    let mut wl = schemars::schema_for!(crate::config::WorkloadConfig);
    // custom title (matches the established capsule-file convention)
    if let Some(m) = wl.schema.metadata.as_mut() {
        m.title = Some(WORKLOAD_SCHEMA_TITLE.to_string());
    }
    // defensive filter: WorkloadConfig is the root (never in definitions);
    // SecretDefConfig is only reachable from ConfigFile (never in the
    // WorkloadConfig closure). Filter so the subschema can never carry them.
    wl.definitions.remove("WorkloadConfig");
    wl.definitions.remove("SecretDefConfig");
    // enforce the "additionalProperties false" rule even if schemars behavior changes
    if let Some(obj) = wl.schema.object.as_mut()
        && obj.additional_properties.is_none()
    {
        obj.additional_properties = Some(Box::new(schemars::schema::Schema::Bool(false)));
    }
    let workload = serde_json::to_string_pretty(&wl)?;
    Ok((full, workload))
}

/// Generate the tool-home registry schema (`config.toml`) from the
/// schemars-annotated `Registry` type (single source of truth, ADR 0021 §8).
pub(crate) fn generate_registry_schema() -> Result<String> {
    let schema = schemars::schema_for!(crate::config::Registry);
    Ok(serde_json::to_string_pretty(&schema)?)
}

/// Generate all three schema artifacts (full + workload + registry).
pub(crate) fn generate_schema_triple() -> Result<(String, String, String)> {
    let (full, workload) = generate_schema_pair()?;
    let registry = generate_registry_schema()?;
    Ok((full, workload, registry))
}

pub fn cmd_generate_schema(
    out: Option<&std::path::Path>,
    out_workload: Option<&std::path::Path>,
    out_registry: Option<&std::path::Path>,
) -> Result<()> {
    if out_workload.is_some() && out.is_none() {
        anyhow::bail!("--output-workload requires --output");
    }
    if out_registry.is_some() && out.is_none() {
        anyhow::bail!("--output-registry requires --output");
    }
    let (full, workload, registry) = generate_schema_triple()?;
    match out {
        Some(p) => {
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(p, format!("{}\n", full))?;
            println!("wrote schema to {}", p.display());
        }
        None => println!("{}", full),
    }
    if let Some(p) = out_workload {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(p, format!("{}\n", workload))?;
        println!("wrote workload schema to {}", p.display());
    }
    if let Some(p) = out_registry {
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(p, format!("{}\n", registry))?;
        println!("wrote registry schema to {}", p.display());
    }
    Ok(())
}

/// Build the `.env.example` content from the loaded config's secrets
/// section. Shared between `cmd_generate_env_example` and the
/// `workestrate secrets` provisioning buffers (which pre-fill the editor
/// from the same source).
pub(crate) fn generate_env_example_content() -> Result<String> {
    let config = config::load_config()?;
    // Bare sorted key list: the resolved source env var of every secret def
    // (raw env_var, defaulting to the secret ID). No description comments —
    // the legacy `description` field is gone in the final model.
    let mut keys: Vec<String> = config
        .secrets
        .iter()
        .map(|(id, s)| s.env_var.clone().unwrap_or_else(|| id.clone()))
        .collect();
    keys.sort();

    let mut buf = String::new();
    buf.push_str("# workestrate environment schema.\n");
    buf.push_str("# This file is committed and safe to share.\n");
    buf.push_str(
        "# Real secrets live in .env.enc (encrypted) and are loaded by workestrate at runtime.\n",
    );
    buf.push_str("#\n");
    buf.push_str("# How these values reach a workload (spec 16):\n");
    buf.push_str(
        "#   - env bindings default to the PLACEHOLDER (least exposure); the real value\n",
    );
    buf.push_str("#     is substituted by the egress rewrite only for hosts in `allowed_hosts`.\n");
    buf.push_str("#   - the real value in-sandbox is an explicit `bound = \"guest\"` opt-in,\n");
    buf.push_str(
        "#     reserved for workloads that verify the credential (e.g. a proxy service verifying its callers).\n",
    );
    buf.push_str(
        "#   - `secret` appears at a binding only when RENAMING (env name != secret ID);\n",
    );
    buf.push_str("#     same-name bindings are just `KEY = true`.\n");
    for key in &keys {
        buf.push_str(&format!("{}=\n", key));
    }
    Ok(buf)
}

pub fn cmd_generate_env_example(output: Option<&std::path::Path>) -> Result<()> {
    let buf = generate_env_example_content()?;
    match output {
        Some(path) => {
            std::fs::write(path, &buf)?;
        }
        None => {
            print!("{}", buf);
        }
    }
    Ok(())
}

pub fn cmd_run(command: &[String]) -> Result<()> {
    if command.is_empty() {
        anyhow::bail!("no command specified. Usage: workestrate run -- <command> [args...]");
    }

    // Load secrets from .env.enc (generic — all keys, no filtering).
    let secrets = crate::microsandbox::secrets_loader::load_secrets()?;
    // FN-9 scoped set_var: this command `exec(2)` REPLACES the workestrate
    // process with the user command below, so the secrets can only reach it
    // via inherited process env. This is the one consumer that fundamentally
    // requires process env; everywhere else resolves from the returned map.
    crate::microsandbox::secrets_loader::apply_secrets_to_process_env(&secrets);

    // exec the command (replaces the workestrate process).
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new(&command[0])
            .args(&command[1..])
            .exec();
        // exec() only returns on failure.
        anyhow::bail!("failed to exec '{}': {}", command[0], err);
    }

    #[cfg(not(unix))]
    {
        let status = std::process::Command::new(&command[0])
            .args(&command[1..])
            .status()?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        Ok(())
    }
}

/// Passthrough to the msb binary (ADR 0036 D4): `workestrate msb -- <args>`
/// forwards <args> verbatim to [`crate::commands::doctor::msb_binary`]
/// (`MSB_PATH` when set, else `msb` on PATH), mirroring [`cmd_run`]'s exec
/// precedent but WITHOUT secrets (no env loading — msb needs none). Like
/// `cmd_run`, this `exec(2)` REPLACES the workestrate process on unix, so
/// interactive/pty use behaves as if msb were invoked directly.
pub fn cmd_msb(args: &[String]) -> Result<()> {
    if args.is_empty() {
        anyhow::bail!("no msb args specified. Usage: workestrate msb -- <args...>");
    }
    let bin = crate::commands::doctor::msb_binary();
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let err = std::process::Command::new(&bin).args(args).exec();
        // exec() only returns on failure.
        anyhow::bail!("failed to exec '{bin}': {err}");
    }
    #[cfg(not(unix))]
    {
        let status = std::process::Command::new(&bin).args(args).status()?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    /// `ps --json` must emit the exact shape pinned by ADR 0021 §7, in the
    /// documented field order: instance, workload, context, slot, kind,
    /// started_at, ports, stale.
    /// Pure: no env, no msb — constructs PsEntry rows directly and round-trips
    /// them through `ps_entries_json` + `serde_json::to_string_pretty`.
    #[test]
    fn ps_json_matches_adr_0021_section_7() {
        use crate::microsandbox::plan::PortMapping;
        use crate::microsandbox::runtime::{PsEntry, PsKind};

        let singleton = PsEntry {
            instance: "personal-litellm".to_string(),
            workload: "litellm".to_string(),
            context: Some("personal".to_string()),
            slot: "personal-litellm".to_string(),
            kind: PsKind::Singleton,
            ports: vec![PortMapping::new(4000, 4000)],
            started_at: "2026-07-20T14:03:11Z".to_string(),
            stale: false,
            status: None,
            config_hash: None,
            namespace: crate::microsandbox::port_registry::default_namespace(),
            staleness: None,
        };
        let parallel = PsEntry {
            instance: "personal-litellm@canary".to_string(),
            workload: "litellm".to_string(),
            context: Some("personal".to_string()),
            slot: "personal-litellm".to_string(),
            kind: PsKind::Parallel,
            ports: vec![PortMapping {
                host: 14000,
                guest: 4000,
                bind_ip: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 2)),
                name: None,
            }],
            started_at: "2026-07-20T14:05:42Z".to_string(),
            stale: false,
            status: None,
            config_hash: None,
            namespace: crate::microsandbox::port_registry::default_namespace(),
            staleness: None,
        };

        let json = serde_json::to_string_pretty(&ps_entries_json(&[singleton, parallel]))
            .expect("serialize ps entries");

        // Field order, names, and casing (kind lowercase) are all pinned
        // here. Any drift from ADR 0021 §7 fails this snapshot. ADR 0026/C3:
        // `bind_ip` is an additive per-port field, always serialized (uniform
        // shape); the parallel row carries its per-instance 127.0.0.2 bind.
        let expected = r#"[
  {
    "instance": "personal-litellm",
    "workload": "litellm",
    "context": "personal",
    "slot": "personal-litellm",
    "kind": "singleton",
    "started_at": "2026-07-20T14:03:11Z",
    "ports": [
      {
        "host": 4000,
        "guest": 4000,
        "bind_ip": "127.0.0.1"
      }
    ],
    "stale": false
  },
  {
    "instance": "personal-litellm@canary",
    "workload": "litellm",
    "context": "personal",
    "slot": "personal-litellm",
    "kind": "parallel",
    "started_at": "2026-07-20T14:05:42Z",
    "ports": [
      {
        "host": 14000,
        "guest": 4000,
        "bind_ip": "127.0.0.2"
      }
    ],
    "stale": false
  }
]"#;
        assert_eq!(json, expected, "ps --json shape drifted from ADR 0021 §7");
    }

    /// Stale-footer remediation text (ADR 0021 §4). When any entry is stale,
    /// `print_ps_text` emits per-instance `down` commands and a catch-all
    /// `down --all`. Exercises the writer-injectable renderer directly so the
    /// footer text is pinned verbatim (not via global-stdout capture).
    #[test]
    fn print_ps_text_emits_remediation_footer_for_stale_entries() {
        use crate::microsandbox::plan::PortMapping;
        use crate::microsandbox::runtime::{PsEntry, PsKind};

        // One stale parallel instance + one live singleton. Sorted output
        // orders litellm (l) before pi (p).
        let entries = vec![
            PsEntry {
                instance: "personal-litellm@canary".to_string(),
                workload: "litellm".to_string(),
                context: Some("personal".to_string()),
                slot: "personal-litellm".to_string(),
                kind: PsKind::Parallel,
                ports: vec![PortMapping::new(14000, 4000)],
                started_at: "2026-07-20T14:05:42Z".to_string(),
                stale: true,
                status: None,
                config_hash: None,
                namespace: crate::microsandbox::port_registry::default_namespace(),
                staleness: None,
            },
            PsEntry {
                instance: "personal-pi".to_string(),
                workload: "pi".to_string(),
                context: Some("personal".to_string()),
                slot: "personal-pi".to_string(),
                kind: PsKind::Singleton,
                ports: vec![PortMapping::new(3000, 3000)],
                started_at: "2026-07-20T14:06:00Z".to_string(),
                stale: false,
                status: None,
                config_hash: None,
                namespace: crate::microsandbox::port_registry::default_namespace(),
                staleness: None,
            },
        ];

        let mut buf: Vec<u8> = Vec::new();
        print_ps_text_to(&entries, &mut buf).expect("render ps text");
        let out = String::from_utf8(buf).expect("utf8");

        // Footer header (verbatim).
        assert!(
            out.contains(
                "1 stale instance(s): registry record exists but the sandbox is not running."
            ),
            "stale footer header missing; got:
{out}"
        );
        // Parallel stale entry → `down --instance <id>` (id, not full instance).
        assert!(
            out.contains("  workestrate workload down litellm --instance canary"),
            "parallel stale remediation line missing; got:
{out}"
        );
        // Catch-all.
        assert!(
            out.contains("Remove every instance with: workestrate down-all"),
            "catch-all remediation line missing; got:
{out}"
        );
        // The live singleton must NOT appear in any teardown line.
        assert!(
            !out.contains("down pi"),
            "live singleton leaked into the footer; got:
{out}"
        );
        // Column header renamed CREATED → STARTED (ADR §7 started_at).
        assert!(
            out.contains("STARTED"),
            "column header should be STARTED (renamed from CREATED); got:
{out}"
        );
    }

    /// ADR 0026/C3: the text PORTS column renders `<bind_ip>:<host>:<guest>`
    /// for a pair bound on a non-default bind (parallel slot), and keeps the
    /// legacy `<host>:<guest>` for default-bind pairs (singleton rows stay
    /// clean). Both JSON (`bind_ip` field) and text surfaces are pinned.
    #[test]
    fn ps_text_and_json_render_bind_ip_for_non_default_binds() {
        use crate::microsandbox::plan::PortMapping;
        use crate::microsandbox::runtime::{PsEntry, PsKind};

        let entries = vec![
            PsEntry {
                instance: "personal-litellm".to_string(),
                workload: "litellm".to_string(),
                context: Some("personal".to_string()),
                slot: "personal-litellm".to_string(),
                kind: PsKind::Singleton,
                ports: vec![PortMapping::new(4000, 4000)],
                started_at: "2026-07-20T14:03:11Z".to_string(),
                stale: false,
                status: None,
                config_hash: None,
                namespace: crate::microsandbox::port_registry::default_namespace(),
                staleness: None,
            },
            PsEntry {
                instance: "personal-litellm@canary".to_string(),
                workload: "litellm".to_string(),
                context: Some("personal".to_string()),
                slot: "personal-litellm".to_string(),
                kind: PsKind::Parallel,
                ports: vec![PortMapping {
                    host: 4000,
                    guest: 4000,
                    bind_ip: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 2)),
                    name: None,
                }],
                started_at: "2026-07-20T14:05:42Z".to_string(),
                stale: false,
                status: None,
                config_hash: None,
                namespace: crate::microsandbox::port_registry::default_namespace(),
                staleness: None,
            },
        ];

        // Text: non-default bind renders the three-field form; the default
        // bind keeps the two-field form and never prints 127.0.0.1.
        let mut buf: Vec<u8> = Vec::new();
        print_ps_text_to(&entries, &mut buf).expect("render ps text");
        let out = String::from_utf8(buf).expect("utf8");
        assert!(
            out.contains("127.0.0.2:4000:4000"),
            "non-default bind must render <bind_ip>:<host>:<guest>; got:\n{out}"
        );
        assert!(
            !out.contains("127.0.0.1:4000:4000"),
            "default bind must NOT print the bind IP; got:\n{out}"
        );

        // JSON: both rows carry the additive bind_ip field.
        let json =
            serde_json::to_string_pretty(&ps_entries_json(&entries)).expect("serialize ps entries");
        assert!(
            json.contains(r#""bind_ip": "127.0.0.1""#),
            "singleton port must serialize bind_ip 127.0.0.1; got:\n{json}"
        );
        assert!(
            json.contains(r#""bind_ip": "127.0.0.2""#),
            "parallel port must serialize bind_ip 127.0.0.2; got:\n{json}"
        );
    }

    /// P3: a named port prefixes the text PORTS column pair (`api:4000:4000`
    /// on the default bind; `api:127.0.0.2:14000:4000` on a non-default
    /// bind). Unnamed pairs keep the legacy form byte-identical — no
    /// colon-prefixed name may leak into an unnamed row.
    #[test]
    fn ps_text_renders_named_ports_with_name_prefix() {
        use crate::microsandbox::plan::PortMapping;
        use crate::microsandbox::runtime::{PsEntry, PsKind};

        let entries = vec![
            PsEntry {
                instance: "personal-litellm".to_string(),
                workload: "litellm".to_string(),
                context: Some("personal".to_string()),
                slot: "personal-litellm".to_string(),
                kind: PsKind::Singleton,
                ports: vec![PortMapping {
                    host: 4000,
                    guest: 4000,
                    bind_ip: crate::microsandbox::plan::default_bind_ip(),
                    name: Some("api".to_string()),
                }],
                started_at: "2026-07-20T14:03:11Z".to_string(),
                stale: false,
                status: None,
                config_hash: None,
                namespace: crate::microsandbox::port_registry::default_namespace(),
                staleness: None,
            },
            PsEntry {
                instance: "personal-litellm@canary".to_string(),
                workload: "litellm".to_string(),
                context: Some("personal".to_string()),
                slot: "personal-litellm".to_string(),
                kind: PsKind::Parallel,
                ports: vec![PortMapping {
                    host: 14000,
                    guest: 4000,
                    bind_ip: std::net::IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 2)),
                    name: Some("api".to_string()),
                }],
                started_at: "2026-07-20T14:05:42Z".to_string(),
                stale: false,
                status: None,
                config_hash: None,
                namespace: crate::microsandbox::port_registry::default_namespace(),
                staleness: None,
            },
            PsEntry {
                instance: "personal-pi".to_string(),
                workload: "pi".to_string(),
                context: Some("personal".to_string()),
                slot: "personal-pi".to_string(),
                kind: PsKind::Singleton,
                ports: vec![PortMapping::new(3000, 3000)],
                started_at: "2026-07-20T14:06:00Z".to_string(),
                stale: false,
                status: None,
                config_hash: None,
                namespace: crate::microsandbox::port_registry::default_namespace(),
                staleness: None,
            },
        ];

        let mut buf: Vec<u8> = Vec::new();
        print_ps_text_to(&entries, &mut buf).expect("render ps text");
        let out = String::from_utf8(buf).expect("utf8");

        // Named default-bind pair: `<name>:<host>:<guest>` (no bind IP).
        assert!(
            out.contains("api:4000:4000"),
            "named default-bind pair must render <name>:<host>:<guest>; got:\n{out}"
        );
        // Named non-default-bind pair: `<name>:<bind_ip>:<host>:<guest>`.
        assert!(
            out.contains("api:127.0.0.2:14000:4000"),
            "named non-default-bind pair must render <name>:<bind_ip>:<host>:<guest>; got:\n{out}"
        );
        // Unnamed pair: exact legacy form — no colon-prefixed name or bind
        // IP leaks past the pair.
        assert!(
            out.contains("3000:3000"),
            "unnamed pair must keep the legacy <host>:<guest> form; got:\n{out}"
        );
        assert!(
            !out.contains("3000:3000:"),
            "unnamed pair must not render a colon-prefixed name; got:\n{out}"
        );
    }

    /// No-stale case: the footer is entirely absent. Pins the negative branch.
    #[test]
    fn print_ps_text_omits_footer_when_nothing_stale() {
        use crate::microsandbox::plan::PortMapping;
        use crate::microsandbox::runtime::{PsEntry, PsKind};

        let entries = vec![PsEntry {
            instance: "personal-litellm".to_string(),
            workload: "litellm".to_string(),
            context: Some("personal".to_string()),
            slot: "personal-litellm".to_string(),
            kind: PsKind::Singleton,
            ports: vec![PortMapping::new(4000, 4000)],
            started_at: "2026-07-20T14:03:11Z".to_string(),
            stale: false,
            status: None,
            config_hash: None,
            namespace: crate::microsandbox::port_registry::default_namespace(),
            staleness: None,
        }];
        let mut buf: Vec<u8> = Vec::new();
        print_ps_text_to(&entries, &mut buf).expect("render ps text");
        let out = String::from_utf8(buf).expect("utf8");
        assert!(
            !out.contains("stale"),
            "no footer expected when nothing stale; got:
{out}"
        );
    }

    // ---- WP5 / E1 regression: graceful check outside a workbench checkout ----

    #[test]
    fn project_root_optional_returns_none_outside_workbench() {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();
        let old_root = std::env::var("AGENTCTL_ROOT").ok();
        let old_manifest = std::env::var("CARGO_MANIFEST_DIR").ok();
        let tmp = std::env::temp_dir().join(format!(
            "no-flake-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp).ok();
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("AGENTCTL_ROOT", &tmp) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("CARGO_MANIFEST_DIR") };

        let result = config::project_root_optional();

        match old_root {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("AGENTCTL_ROOT", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("AGENTCTL_ROOT") },
        }
        match old_manifest {
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            Some(v) => unsafe { std::env::set_var("CARGO_MANIFEST_DIR", v) },
            // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
            None => unsafe { std::env::remove_var("CARGO_MANIFEST_DIR") },
        }

        // The contract: project_root_optional NEVER returns a path that
        // lacks flake.nix. It returns Some verified-path or None.
        match result {
            None => { /* expected when not in a workbench */ }
            Some(p) => {
                assert!(
                    p.join("flake.nix").exists(),
                    "project_root_optional returned '{:?}' which lacks flake.nix",
                    p
                );
            }
        }
    }

    /// E1 integration: simulate a fresh-install `workestrate check` from /tmp
    /// with a tmp HOME and no workbench checkout. The check must NOT error
    /// just because no workbench checkout is reachable; required-files prints
    /// "(not in a workbench checkout — skipped)".
    #[test]
    fn cmd_check_degrades_gracefully_outside_workbench() -> Result<()> {
        let _lock = crate::config::test_support::ENV_TEST_LOCK.lock().unwrap();

        let tmp_home = std::env::temp_dir().join(format!(
            "workestrate-e1-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&tmp_home)?;

        let old_home = std::env::var("HOME").ok();
        let old_xdg_config = std::env::var("XDG_CONFIG_HOME").ok();
        let old_xdg_data = std::env::var("XDG_DATA_HOME").ok();
        let old_root = std::env::var("AGENTCTL_ROOT").ok();
        let old_manifest = std::env::var("CARGO_MANIFEST_DIR").ok();
        let old_no_project = std::env::var("WORKESTRATE_NO_PROJECT_CONFIG").ok();
        let old_config_dir = std::env::var("WORKESTRATE_CONFIG_DIR").ok();

        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("HOME", &tmp_home) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe {
            std::env::set_var(
                "XDG_CONFIG_HOME",
                tmp_home.join(".config").to_string_lossy().as_ref(),
            )
        };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe {
            std::env::set_var(
                "XDG_DATA_HOME",
                tmp_home
                    .join(".local")
                    .join("share")
                    .to_string_lossy()
                    .as_ref(),
            )
        };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("AGENTCTL_ROOT", &tmp_home) };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("CARGO_MANIFEST_DIR") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::set_var("WORKESTRATE_NO_PROJECT_CONFIG", "1") };
        // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
        unsafe { std::env::remove_var("WORKESTRATE_CONFIG_DIR") };

        let result = cmd_check();

        for (k, v) in [
            ("HOME", old_home),
            ("XDG_CONFIG_HOME", old_xdg_config),
            ("XDG_DATA_HOME", old_xdg_data),
            ("AGENTCTL_ROOT", old_root),
            ("CARGO_MANIFEST_DIR", old_manifest),
            ("WORKESTRATE_NO_PROJECT_CONFIG", old_no_project),
            ("WORKESTRATE_CONFIG_DIR", old_config_dir),
        ] {
            match v {
                // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
                Some(val) => unsafe { std::env::set_var(k, val) },
                // SAFETY: serialized by ENV_TEST_LOCK (held by this test / guard / caller).
                None => unsafe { std::env::remove_var(k) },
            }
        }
        let _ = std::fs::remove_dir_all(&tmp_home);

        // E1 contract: cmd_check must NOT error just because no workbench
        // checkout is reachable. It may error for OTHER reasons (e.g. a
        // missing required artifact) but not for project_root being absent.
        if let Err(e) = &result {
            let msg = e.to_string();
            assert!(
                !msg.contains("flake.nix") && !msg.contains("project root"),
                "E1 regression: cmd_check errored on project_root: {msg}"
            );
        }
        Ok(())
    }

    // ---- ADR 0030 §4.4 instances verb (cmd_instances rendering + JSON) ----

    use crate::microsandbox::runtime::{InstanceStatus, PsKind};

    /// Build a minimal `InstanceEntry` for rendering tests.
    fn instance_entry(workload: &str, instance: &str, status: InstanceStatus) -> InstanceEntry {
        InstanceEntry {
            instance: instance.to_string(),
            workload: workload.to_string(),
            namespace: "default".to_string(),
            context: Some("personal".to_string()),
            slot: instance.split('@').next().unwrap_or(instance).to_string(),
            kind: if instance.contains('@') {
                PsKind::Parallel
            } else {
                PsKind::Singleton
            },
            status,
            ports: vec![crate::microsandbox::plan::PortMapping::new(4000, 4000)],
            started_at: "2026-07-20T14:03:11Z".to_string(),
            strategy: "singleton".to_string(),
            on_conflict: "reuse,start,replace".to_string(),
            port: "fixed".to_string(),
            label: None,
        }
    }

    /// `instances --json` must emit the reconciled 5-state status in
    /// snake_case plus the policy/namespace columns (ADR 0030 §4.4).
    #[test]
    fn instances_json_shape() {
        let entries = vec![
            instance_entry(
                "litellm",
                "personal-litellm",
                InstanceStatus::RunningHealthy,
            ),
            instance_entry(
                "litellm",
                "personal-litellm@canary",
                InstanceStatus::RunningUnhealthy,
            ),
        ];
        let json = serde_json::to_string_pretty(&crate::json_out::instances_json(&entries))
            .expect("serialize instances");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(
            value[0]["status"], "running_healthy",
            "status snake_case; got:\n{json}"
        );
        assert_eq!(
            value[1]["status"], "running_unhealthy",
            "status snake_case; got:\n{json}"
        );
        assert_eq!(value[0]["workload"], "litellm");
        assert_eq!(value[0]["namespace"], "default");
        assert_eq!(value[0]["strategy"], "singleton");
        assert_eq!(value[0]["on_conflict"], "reuse,start,replace");
        assert_eq!(value[0]["port"], "fixed");
        assert_eq!(value[0]["kind"], "singleton");
        assert_eq!(value[1]["kind"], "parallel");
        // label is None → omitted.
        assert!(
            value[0].get("label").is_none(),
            "label must be omitted; got:\n{json}"
        );
    }

    /// `instances` text output groups rows by workload with a blank line
    /// between groups (ADR 0030 §4.4).
    #[test]
    fn print_instances_text_groups_by_workload() {
        let entries = vec![
            instance_entry(
                "litellm",
                "personal-litellm",
                InstanceStatus::RunningHealthy,
            ),
            instance_entry("pi", "personal-pi", InstanceStatus::Stopped),
        ];
        let mut buf: Vec<u8> = Vec::new();
        print_instances_text_to(&entries, &mut buf).expect("render instances text");
        let out = String::from_utf8(buf).expect("utf8");
        assert!(
            out.contains("litellm:"),
            "workload group header missing; got:\n{out}"
        );
        assert!(
            out.contains("pi:"),
            "workload group header missing; got:\n{out}"
        );
        assert!(
            out.contains("running-healthy"),
            "status string missing; got:\n{out}"
        );
        assert!(
            out.contains("stopped"),
            "status string missing; got:\n{out}"
        );
        assert!(
            out.contains("ports=[4000]"),
            "ports column missing; got:\n{out}"
        );
        assert!(
            out.contains("strategy=singleton"),
            "strategy column missing; got:\n{out}"
        );
        assert!(
            out.contains("on_conflict=[reuse,start,replace]"),
            "on_conflict column missing; got:\n{out}"
        );
        assert!(
            out.contains("port=fixed"),
            "port column missing; got:\n{out}"
        );
        // G5: the context column sits after the namespace column.
        let litellm_row = out
            .lines()
            .find(|l| l.contains("personal-litellm"))
            .expect("litellm row must exist");
        assert!(
            litellm_row.contains("\tdefault\tpersonal\t"),
            "context column (after namespace) missing; got row: {litellm_row}"
        );
    }

    /// G5: an instance entry with `context: None` (legacy unknown-context
    /// record) renders `-` in the context column.
    #[test]
    fn print_instances_text_renders_dash_for_none_context() {
        let entries = vec![InstanceEntry {
            context: None,
            ..instance_entry(
                "litellm",
                "personal-litellm",
                InstanceStatus::RunningHealthy,
            )
        }];
        let mut buf: Vec<u8> = Vec::new();
        print_instances_text_to(&entries, &mut buf).expect("render instances text");
        let out = String::from_utf8(buf).expect("utf8");
        let row = out
            .lines()
            .find(|l| l.contains("personal-litellm"))
            .expect("litellm row must exist");
        assert!(
            row.contains("\tdefault\t-\t"),
            "None context must render `-`; got row: {row}"
        );
    }

    /// `instances` text output with no entries prints "(no instances)".
    #[test]
    fn print_instances_text_empty() {
        let mut buf: Vec<u8> = Vec::new();
        print_instances_text_to(&[], &mut buf).expect("render instances text");
        let out = String::from_utf8(buf).expect("utf8");
        assert_eq!(out, "(no instances)\n", "empty instances text; got:\n{out}");
    }

    /// A `PsEntry` with `status: None` serializes WITHOUT the status field
    /// (back-compat: legacy `ps --json` stays byte-identical).
    #[test]
    fn ps_entry_status_omitted_when_none() {
        use crate::microsandbox::plan::PortMapping;
        use crate::microsandbox::runtime::PsEntry;
        let entry = PsEntry {
            instance: "personal-litellm".to_string(),
            workload: "litellm".to_string(),
            context: Some("personal".to_string()),
            slot: "personal-litellm".to_string(),
            kind: PsKind::Singleton,
            ports: vec![PortMapping::new(4000, 4000)],
            started_at: "2026-07-20T14:03:11Z".to_string(),
            stale: false,
            status: None,
            config_hash: None,
            namespace: crate::microsandbox::port_registry::default_namespace(),
            staleness: None,
        };
        let json = serde_json::to_string_pretty(&crate::json_out::ps_entries_json(&[entry]))
            .expect("serialize ps entry");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert!(
            value[0].get("status").is_none(),
            "status must be omitted when None; got:\n{json}"
        );
    }

    /// `ps` text renders the reconciled 5-state STATUS column (ADR 0030
    /// §4.4): a keep-alive zombie shows `running-unhealthy`, NOT `Running`;
    /// an entry whose facts could not be gathered (`status: None`) renders
    /// `-`. The stale footer (ADR 0021 §4) is unchanged.
    #[test]
    fn print_ps_text_renders_status_column() {
        use crate::microsandbox::plan::PortMapping;
        use crate::microsandbox::runtime::PsEntry;
        let entries = vec![
            PsEntry {
                instance: "personal-litellm".to_string(),
                workload: "litellm".to_string(),
                context: Some("personal".to_string()),
                slot: "personal-litellm".to_string(),
                kind: PsKind::Singleton,
                ports: vec![PortMapping::new(4000, 4000)],
                started_at: "2026-07-20T14:03:11Z".to_string(),
                stale: false,
                status: Some(InstanceStatus::RunningUnhealthy),
                config_hash: None,
                namespace: crate::microsandbox::port_registry::default_namespace(),
                staleness: None,
            },
            PsEntry {
                instance: "personal-pi".to_string(),
                workload: "pi".to_string(),
                context: Some("personal".to_string()),
                slot: "personal-pi".to_string(),
                kind: PsKind::Singleton,
                ports: vec![PortMapping::new(3000, 3000)],
                started_at: "2026-07-20T14:06:00Z".to_string(),
                stale: false,
                status: None,
                config_hash: None,
                namespace: crate::microsandbox::port_registry::default_namespace(),
                staleness: None,
            },
        ];
        let mut buf: Vec<u8> = Vec::new();
        print_ps_text_to(&entries, &mut buf).expect("render ps text");
        let out = String::from_utf8(buf).expect("utf8");
        // Header carries the STATUS column.
        assert!(
            out.contains("STATUS"),
            "STATUS column header missing; got:\n{out}"
        );
        // The zombie row shows running-unhealthy, never the bare `Running`.
        let litellm_line = out
            .lines()
            .find(|l| l.contains("personal-litellm"))
            .expect("litellm row must exist");
        assert!(
            litellm_line.contains("running-unhealthy"),
            "zombie must show running-unhealthy; got line: {litellm_line}"
        );
        assert!(
            !litellm_line.contains("Running"),
            "zombie must NOT render as Running; got line: {litellm_line}"
        );
        // A None status renders `-`.
        let pi_line = out
            .lines()
            .find(|l| l.contains("personal-pi"))
            .expect("pi row must exist");
        assert!(
            pi_line.contains(" - "),
            "None status must render `-`; got line: {pi_line}"
        );
    }

    // ---- ADR 0032 §Provenance stamps: ps staleness display + computation ----

    /// A stamped+drifted entry renders the PINNED drift suffix
    /// ` stale (config xxxx → current yyyy)` — 4-char prefixes
    /// ([`crate::microsandbox::provenance::PROVENANCE_DISPLAY_LEN`]) of the
    /// FULL stored hashes — appended after STARTED; a pre-stamp entry
    /// (staleness None) renders NOTHING extra.
    #[test]
    fn print_ps_text_renders_staleness_suffix_only_when_populated() {
        use crate::microsandbox::plan::PortMapping;
        use crate::microsandbox::runtime::{ConfigStaleness, PsEntry};

        let drifted = PsEntry {
            instance: "personal-litellm".to_string(),
            workload: "litellm".to_string(),
            context: Some("personal".to_string()),
            slot: "personal-litellm".to_string(),
            kind: PsKind::Singleton,
            ports: vec![PortMapping::new(4000, 4000)],
            started_at: "2026-07-20T14:03:11Z".to_string(),
            stale: false,
            status: None,
            config_hash: Some("a1b2c3d4e5f60718".to_string()),
            namespace: crate::microsandbox::port_registry::default_namespace(),
            staleness: Some(ConfigStaleness {
                recorded: "a1b2c3d4e5f60718".to_string(),
                current: "d4e5f60718273a4b".to_string(),
            }),
        };
        let pre_stamp = PsEntry {
            instance: "personal-pi".to_string(),
            workload: "pi".to_string(),
            context: Some("personal".to_string()),
            slot: "personal-pi".to_string(),
            kind: PsKind::Singleton,
            ports: vec![PortMapping::new(3000, 3000)],
            started_at: "2026-07-20T14:06:00Z".to_string(),
            stale: false,
            status: None,
            config_hash: None,
            namespace: crate::microsandbox::port_registry::default_namespace(),
            staleness: None,
        };

        let mut buf: Vec<u8> = Vec::new();
        print_ps_text_to(&[drifted, pre_stamp], &mut buf).expect("render ps text");
        let out = String::from_utf8(buf).expect("utf8");

        // The drifted row carries the pinned suffix with 4-char prefixes of
        // the stored FULL hashes.
        let litellm_line = out
            .lines()
            .find(|l| l.contains("personal-litellm"))
            .expect("litellm row must exist");
        assert!(
            litellm_line.contains(" stale (config a1b2 → current d4e5)"),
            "drifted row must render the pinned staleness suffix; got line: {litellm_line}"
        );
        // The pre-stamp row renders nothing extra (no drift suffix at all).
        let pi_line = out
            .lines()
            .find(|l| l.contains("personal-pi"))
            .expect("pi row must exist");
        assert!(
            !pi_line.contains("stale"),
            "pre-stamp row must render no staleness suffix; got line: {pi_line}"
        );
    }

    /// `ps --json`: the `staleness` field is ADDITIVE — present (with the
    /// full recorded/current hashes) only on stamped+drifted rows; absent
    /// entirely otherwise, so pre-stamp/unresolvable rows keep the legacy
    /// shape byte-identical (the established skip_serializing_if convention).
    #[test]
    fn ps_json_serializes_staleness_additively() {
        use crate::microsandbox::plan::PortMapping;
        use crate::microsandbox::runtime::{ConfigStaleness, PsEntry};

        let drifted = PsEntry {
            instance: "personal-litellm".to_string(),
            workload: "litellm".to_string(),
            context: Some("personal".to_string()),
            slot: "personal-litellm".to_string(),
            kind: PsKind::Singleton,
            ports: vec![PortMapping::new(4000, 4000)],
            started_at: "2026-07-20T14:03:11Z".to_string(),
            stale: false,
            status: None,
            config_hash: Some("a1b2c3d4e5f60718".to_string()),
            namespace: crate::microsandbox::port_registry::default_namespace(),
            staleness: Some(ConfigStaleness {
                recorded: "a1b2c3d4e5f60718".to_string(),
                current: "d4e5f60718273a4b".to_string(),
            }),
        };
        let clean = PsEntry {
            instance: "personal-pi".to_string(),
            workload: "pi".to_string(),
            context: Some("personal".to_string()),
            slot: "personal-pi".to_string(),
            kind: PsKind::Singleton,
            ports: vec![PortMapping::new(3000, 3000)],
            started_at: "2026-07-20T14:06:00Z".to_string(),
            stale: false,
            status: None,
            config_hash: None,
            namespace: crate::microsandbox::port_registry::default_namespace(),
            staleness: None,
        };

        let json = serde_json::to_string_pretty(&ps_entries_json(&[drifted, clean]))
            .expect("serialize ps entries");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
        // Drifted row: the additive object with FULL hashes.
        assert_eq!(
            value[0]["staleness"]["recorded"], "a1b2c3d4e5f60718",
            "staleness.recorded must carry the FULL recorded hash; got:\n{json}"
        );
        assert_eq!(
            value[0]["staleness"]["current"], "d4e5f60718273a4b",
            "staleness.current must carry the FULL current hash; got:\n{json}"
        );
        // Clean/pre-stamp row: the key is absent entirely (not null).
        assert!(
            value[1].get("staleness").is_none(),
            "staleness must be omitted when None; got:\n{json}"
        );
    }

    /// `apply_config_staleness` integration against the committed fixture
    /// config (TestConfigGuard): a record whose stamp matches the CURRENT
    /// runtime-relevant view shows no staleness; a diverged stamp populates
    /// it; a FOREIGN-NAMESPACE record and an UNKNOWN-WORKLOAD record are
    /// honest-unknown (nothing displayed).
    #[test]
    fn apply_config_staleness_matches_current_and_skips_unresolvable_rows() -> Result<()> {
        use crate::config::test_support::{TestConfigGuard, unique_state_dir};
        use crate::microsandbox::plan::PortMapping;

        let _guard = TestConfigGuard::new();
        let state_dir = unique_state_dir("staleness-apply");

        // The fixture's example-litellm is a registry-image service with no
        // mounts / depends_on: plan() is pure, and its declaring layer (the
        // fixture dir) is not a registered config repo → namespace "default".
        let wl = crate::microsandbox::workload::ConfigWorkload::new("example-litellm")?;
        let current =
            crate::microsandbox::runtime::current_config_hash_for_workload(&wl, "example-litellm");
        let stale_hash = "0000000000000000";

        let register = |state_dir: &_,
                        instance: &str,
                        context: Option<&str>,
                        workload: &str,
                        namespace: &str,
                        config_hash: &str|
         -> Result<()> {
            // Distinct host ports per record: the registry refuses a
            // (bind, port) collision. Ports never reach the hash (host ports
            // are allocation-dependent and pinned out), so the choice is
            // display-neutral.
            static NEXT_PORT: std::sync::atomic::AtomicU16 =
                std::sync::atomic::AtomicU16::new(4000);
            let port = NEXT_PORT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            crate::microsandbox::port_registry::check_and_register_sandbox_lifecycle(
                state_dir,
                instance,
                context,
                workload,
                crate::microsandbox::plan::default_bind_ip(),
                &[port],
                &[PortMapping::new(port, 4000)],
                "2026-08-24T00:00:00Z",
                namespace,
                None,
                None,
                None,
                Some(config_hash),
            )
        };

        // 1. Matching stamp → equal hashes → no staleness displayed.
        register(
            &state_dir,
            "example-litellm",
            None,
            "example-litellm",
            "default",
            &current,
        )?;
        // 2. Diverged stamp → staleness populated (recorded vs current).
        register(
            &state_dir,
            "personal-example-litellm",
            Some("personal"),
            "example-litellm",
            "default",
            stale_hash,
        )?;
        // 3. Foreign-namespace record → not this config's row → unknown.
        register(
            &state_dir,
            "work-example-litellm",
            Some("work"),
            "example-litellm",
            "foreign-repo",
            stale_hash,
        )?;
        // 4. Workload gone from the active config → unknown.
        register(
            &state_dir,
            "ghost-workload",
            None,
            "ghost-workload",
            "default",
            stale_hash,
        )?;

        let mut entries = crate::microsandbox::runtime::ps(&state_dir)?;
        assert_eq!(entries.len(), 4, "all four records listed");
        apply_config_staleness(&mut entries);

        let by_instance = |name: &str| {
            entries
                .iter()
                .find(|e| e.instance == name)
                .unwrap_or_else(|| panic!("entry '{name}' must be listed"))
        };

        let matching = by_instance("example-litellm");
        assert!(
            matching.staleness.is_none(),
            "an up-to-date stamp must display nothing; got {:?}",
            matching.staleness
        );

        let drifted = by_instance("personal-example-litellm");
        let s = drifted
            .staleness
            .as_ref()
            .expect("a diverged stamp must populate staleness");
        assert_eq!(s.recorded, stale_hash);
        assert_eq!(s.current, current, "current side is the live view's hash");

        assert!(
            by_instance("work-example-litellm").staleness.is_none(),
            "a foreign-namespace record is honest-unknown (nothing displayed)"
        );
        assert!(
            by_instance("ghost-workload").staleness.is_none(),
            "an unknown workload is honest-unknown (nothing displayed)"
        );

        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    /// `workloads --json` includes the policy + namespace columns (ADR 0030
    /// §4.4) and omits the label when None. G5: the context field is always
    /// serialized (null when None), matching the PsEntryJson.context
    /// convention.
    #[test]
    fn workloads_json_includes_policy_and_namespace() {
        let entries = vec![WorkloadListEntry {
            name: "litellm".to_string(),
            kind: "service".to_string(),
            image: "litellm:main".to_string(),
            instances: vec!["personal-litellm".to_string()],
            namespace: "default".to_string(),
            context: Some("personal".to_string()),
            strategy: "singleton".to_string(),
            on_conflict: "reuse,start,replace".to_string(),
            port: "fixed".to_string(),
            label: None,
        }];
        let json = serde_json::to_string_pretty(&crate::json_out::workloads_json(&entries))
            .expect("serialize workloads");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_eq!(value[0]["namespace"], "default");
        assert_eq!(value[0]["context"], "personal");
        assert_eq!(value[0]["strategy"], "singleton");
        assert_eq!(value[0]["on_conflict"], "reuse,start,replace");
        assert_eq!(value[0]["port"], "fixed");
        assert!(
            value[0].get("label").is_none(),
            "label must be omitted; got:\n{json}"
        );
        // context is ALWAYS serialized (null when None) — never omitted.
        let none_context = vec![WorkloadListEntry {
            context: None,
            ..entries[0].clone()
        }];
        let json = serde_json::to_string_pretty(&crate::json_out::workloads_json(&none_context))
            .expect("serialize workloads");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert!(
            value[0].get("context").is_some(),
            "context must be present even when None; got:\n{json}"
        );
        assert_eq!(value[0]["context"], serde_json::Value::Null);
    }

    /// `workloads` text output includes the policy + namespace columns (ADR
    /// 0030 §4.4) plus the G5 context column after namespace (`-` when None).
    #[test]
    fn print_workloads_text_includes_policy_columns() {
        let entries = vec![WorkloadListEntry {
            name: "litellm".to_string(),
            kind: "service".to_string(),
            image: "litellm:main".to_string(),
            instances: vec!["personal-litellm".to_string()],
            namespace: "default".to_string(),
            context: Some("personal".to_string()),
            strategy: "singleton".to_string(),
            on_conflict: "reuse,start,replace".to_string(),
            port: "fixed".to_string(),
            label: Some("v1".to_string()),
        }];
        let mut buf: Vec<u8> = Vec::new();
        print_workloads_text_to(&entries, &mut buf).expect("render workloads text");
        let out = String::from_utf8(buf).expect("utf8");
        // Byte-exact row: name, kind, image, running, namespace, context,
        // strategy, on_conflict, port, label.
        assert_eq!(
            out,
            "litellm\tservice\tlitellm:main\trunning: personal-litellm\tdefault\tpersonal\tsingleton\treuse,start,replace\tfixed label=v1\n",
            "workloads text row drifted; got:\n{out}"
        );
        // None context renders `-`.
        let none_context = vec![WorkloadListEntry {
            name: "pi".to_string(),
            kind: "service".to_string(),
            image: "pi:main".to_string(),
            instances: vec![],
            namespace: "default".to_string(),
            context: None,
            strategy: "singleton".to_string(),
            on_conflict: "reuse,start,replace".to_string(),
            port: "fixed".to_string(),
            label: None,
        }];
        let mut buf: Vec<u8> = Vec::new();
        print_workloads_text_to(&none_context, &mut buf).expect("render workloads text");
        let out = String::from_utf8(buf).expect("utf8");
        assert_eq!(
            out,
            "pi\tservice\tpi:main\t(none running)\tdefault\t-\tsingleton\treuse,start,replace\tfixed\n",
            "None context must render `-`; got:\n{out}"
        );
    }
}
