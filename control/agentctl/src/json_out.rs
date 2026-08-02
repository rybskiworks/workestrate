//! JSON-output helpers (ps / down).
//!
//! Serialisable view structs for the `--json` output of `workestrate ps` and
//! `workestrate down`, plus the conversion functions from runtime types.

#[derive(serde::Serialize)]
pub struct DownResultJson {
    instance: String,
    status: &'static str,
    message: Option<String>,
}

pub fn down_result_json(r: &crate::microsandbox::runtime::DownResult) -> DownResultJson {
    // Inline conversion (avoids `expect()` on a 1->1 invariant; clippy
    // `expect_used` is deny in this crate).
    use crate::microsandbox::runtime::DownStatus;
    DownResultJson {
        instance: r.instance.clone(),
        status: match r.status {
            DownStatus::Stopped => "stopped",
            DownStatus::NotFound => "not_found",
            DownStatus::Error => "error",
        },
        message: r.message.clone(),
    }
}

pub fn down_results_json(
    results: &[crate::microsandbox::runtime::DownResult],
) -> Vec<DownResultJson> {
    results.iter().map(down_result_json).collect()
}

#[derive(serde::Serialize)]
pub struct PsPortJson {
    host: u16,
    guest: u16,
    /// Host bind address (ADR 0026). Always serialized (uniform shape;
    /// additive field) — serde renders it as a string, e.g. "127.0.0.1".
    pub bind_ip: std::net::IpAddr,
}

#[derive(serde::Serialize)]
pub struct PsEntryJson {
    instance: String,
    workload: String,
    context: Option<String>,
    slot: String,
    kind: crate::microsandbox::runtime::PsKind,
    started_at: String,
    ports: Vec<PsPortJson>,
    stale: bool,
}

pub fn ps_entries_json(entries: &[crate::microsandbox::runtime::PsEntry]) -> Vec<PsEntryJson> {
    entries
        .iter()
        .map(|e| PsEntryJson {
            instance: e.instance.clone(),
            workload: e.workload.clone(),
            context: e.context.clone(),
            slot: e.slot.clone(),
            kind: e.kind,
            started_at: e.started_at.clone(),
            ports: e
                .ports
                .iter()
                .map(|p| PsPortJson {
                    host: p.host,
                    guest: p.guest,
                    bind_ip: p.bind_ip,
                })
                .collect(),
            stale: e.stale,
        })
        .collect()
}

/// One row of `workestrate workloads --json` (ADR 0027): the configured
/// workload name, kind, image summary, and the instance names currently
/// registered in the port registry (empty = not running).
#[derive(serde::Serialize)]
pub struct WorkloadJson {
    name: String,
    kind: String,
    image: String,
    instances: Vec<String>,
}

pub fn workloads_json(
    entries: &[crate::commands::diagnostics::WorkloadListEntry],
) -> Vec<WorkloadJson> {
    entries
        .iter()
        .map(|e| WorkloadJson {
            name: e.name.clone(),
            kind: e.kind.clone(),
            image: e.image.clone(),
            instances: e.instances.clone(),
        })
        .collect()
}

/// One row of `workestrate workload build --json` (spec 21 §5.1, phase C):
/// the per-workload change-detection result. The envelope is a bare ARRAY of
/// these objects — for BOTH the single-name and the batch scopes (the
/// single-name shape is the same array of one), matching the bare-array
/// convention of `ps_entries_json`/`workloads_json` above.
#[derive(serde::Serialize)]
pub struct BuildResultJson {
    name: String,
    /// repo_key (registered repo name or canonical path).
    repo: String,
    attr: String,
    tag: String,
    /// "absent" | "fresh" | "stale" | "unknown" (nix-absent degrade).
    record_state: String,
    /// "present" | "gone".
    store_state: String,
    /// The current drvPath eval; `null` when nix is absent (unverifiable).
    drv_path: Option<String>,
    decision: String,
    action_taken: String,
}

pub fn build_results_json(
    reports: &[crate::images::build_cmd::TargetReport],
) -> Vec<BuildResultJson> {
    reports
        .iter()
        .map(|r| BuildResultJson {
            name: r.name.clone(),
            repo: r.repo.clone(),
            attr: r.attr.clone(),
            tag: r.tag.clone(),
            record_state: r.record_state.clone(),
            store_state: r.store_state.clone(),
            drv_path: r.drv_path.clone(),
            decision: r.decision.clone(),
            action_taken: r.action_taken.clone(),
        })
        .collect()
}
