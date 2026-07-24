//! JSON-output helpers (ps / down).
//!
//! Serialisable view structs for the `--json` output of `workestrate ps` and
//! `workestrate down`, plus the conversion functions from runtime types.

#[derive(serde::Serialize)]
pub(crate) struct DownResultJson {
    instance: String,
    status: &'static str,
    message: Option<String>,
}

pub(crate) fn down_result_json(r: &crate::microsandbox::runtime::DownResult) -> DownResultJson {
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

pub(crate) fn down_results_json(
    results: &[crate::microsandbox::runtime::DownResult],
) -> Vec<DownResultJson> {
    use crate::microsandbox::runtime::DownStatus;
    results
        .iter()
        .map(|r| DownResultJson {
            instance: r.instance.clone(),
            status: match r.status {
                DownStatus::Stopped => "stopped",
                DownStatus::NotFound => "not_found",
                DownStatus::Error => "error",
            },
            message: r.message.clone(),
        })
        .collect()
}

#[derive(serde::Serialize)]
pub(crate) struct PsPortJson {
    host: u16,
    guest: u16,
}

#[derive(serde::Serialize)]
pub(crate) struct PsEntryJson {
    instance: String,
    workload: String,
    context: Option<String>,
    slot: String,
    kind: crate::microsandbox::runtime::PsKind,
    started_at: String,
    ports: Vec<PsPortJson>,
    /// Omitted entirely when None (singleton / --port-offset 0), present only
    /// on parallel instances started with a non-zero offset (ADR 0021 §7).
    #[serde(skip_serializing_if = "Option::is_none")]
    port_offset: Option<u16>,
    stale: bool,
}

pub(crate) fn ps_entries_json(
    entries: &[crate::microsandbox::runtime::PsEntry],
) -> Vec<PsEntryJson> {
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
                })
                .collect(),
            port_offset: e.port_offset,
            stale: e.stale,
        })
        .collect()
}
