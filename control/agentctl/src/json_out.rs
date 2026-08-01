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
