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
    /// Optional port name (P3 namespaced ports). Skipped when absent so
    /// legacy output stays byte-identical.
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
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
    /// Reconciled 5-state status (ADR 0030 §4.4). Skipped when `None` so
    /// legacy JSON stays byte-identical (the pure `ps()` leaves it `None`;
    /// `cmd_ps` populates it).
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<crate::microsandbox::runtime::InstanceStatus>,
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
                    name: p.name.clone(),
                })
                .collect(),
            stale: e.stale,
            status: e.status,
        })
        .collect()
}

/// One row of `workestrate workloads --json` (ADR 0027): the configured
/// workload name, kind, image summary, the instance names currently
/// registered in the port registry (empty = not running), and the declared
/// instance policy / namespace columns (ADR 0030 §4.4).
#[derive(serde::Serialize)]
pub struct WorkloadJson {
    name: String,
    kind: String,
    image: String,
    instances: Vec<String>,
    namespace: String,
    strategy: String,
    on_conflict: String,
    port: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
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
            namespace: e.namespace.clone(),
            strategy: e.strategy.clone(),
            on_conflict: e.on_conflict.clone(),
            port: e.port.clone(),
            label: e.label.clone(),
        })
        .collect()
}

/// One row of `workestrate instances --json` (ADR 0030 §4.4): the reconciled
/// instance view across the registry + msb, with the 5-state status and the
/// workload's declared instance policy.
#[derive(serde::Serialize)]
pub struct InstanceJson {
    instance: String,
    workload: String,
    namespace: String,
    context: Option<String>,
    slot: String,
    kind: crate::microsandbox::runtime::PsKind,
    status: crate::microsandbox::runtime::InstanceStatus,
    started_at: String,
    ports: Vec<PsPortJson>,
    strategy: String,
    on_conflict: String,
    port: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
}

pub fn instances_json(
    entries: &[crate::commands::diagnostics::InstanceEntry],
) -> Vec<InstanceJson> {
    entries
        .iter()
        .map(|e| InstanceJson {
            instance: e.instance.clone(),
            workload: e.workload.clone(),
            namespace: e.namespace.clone(),
            context: e.context.clone(),
            slot: e.slot.clone(),
            kind: e.kind,
            status: e.status,
            started_at: e.started_at.clone(),
            ports: e
                .ports
                .iter()
                .map(|p| PsPortJson {
                    host: p.host,
                    guest: p.guest,
                    bind_ip: p.bind_ip,
                    name: p.name.clone(),
                })
                .collect(),
            strategy: e.strategy.clone(),
            on_conflict: e.on_conflict.clone(),
            port: e.port.clone(),
            label: e.label.clone(),
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

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unwrap_in_result
)]
mod tests {
    use super::*;

    /// P3: `ps --json` carries the port `name` when present and omits it
    /// entirely when absent (additive; legacy JSON stays byte-identical).
    #[test]
    fn ps_entries_json_serializes_name_when_present_and_omits_when_absent() {
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
            },
        ];

        let json = serde_json::to_string_pretty(&ps_entries_json(&entries)).expect("serialize");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
        // Named port: the name key is present with the declared value.
        assert_eq!(
            value[0]["ports"][0]["name"], "api",
            "named port must serialize its name; got:\n{json}"
        );
        // Unnamed port: no name key at all (legacy shape preserved).
        assert!(
            value[1]["ports"][0].get("name").is_none(),
            "unnamed port must omit the name key; got:\n{json}"
        );
    }
}
