use super::lock::PortRegistryLock;
use super::SandboxInstanceRecord;
use crate::microsandbox::plan::PortMapping;
use anyhow::Result;
use std::path::Path;

/// Read and parse a registry record file, warning loudly (WP10/A12) instead
/// of silently skipping when the file exists but is unreadable or corrupt.
///
/// `Ok(Some)` = valid record; `Ok(None)` = unreadable/corrupt (a WARNING was
/// emitted) — callers keep their pre-A12 lenient behavior (skip the record,
/// still succeed), but the corruption is no longer invisible. A MISSING file
/// is not corruption: returns `Ok(None)` silently (for [`find_record`]).
fn read_record_loud(path: &Path) -> Result<Option<SandboxInstanceRecord>> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            eprintln!(
                "WARNING: unreadable port-registry record {}: {}",
                path.display(),
                e
            );
            return Ok(None);
        }
    };
    match serde_json::from_str::<SandboxInstanceRecord>(&content) {
        Ok(record) => Ok(Some(record)),
        Err(e) => {
            eprintln!(
                "WARNING: corrupt port-registry record {}: {}",
                path.display(),
                e
            );
            Ok(None)
        }
    }
}

/// Check for host-port collisions against already-running workestrate sandboxes.
///
/// Scans `${state_dir}/var/run/*.json` for port mappings. Excludes the file
/// for `instance_name` (in case it's a restart). If any port in `ports`
/// matches a port in another instance's record, returns a hard error naming
/// both sandboxes, the port, and remediation.
///
/// Acquires the registry lock around its own critical section (WP10/A17):
/// this makes each individual call atomic against concurrent registrations,
/// but does NOT close the check-then-register race across two separate calls
/// — see [`check_and_register_sandbox_lifecycle`] for the atomic path.
pub fn check_port_collisions(state_dir: &Path, instance_name: &str, ports: &[u16]) -> Result<()> {
    let _lock = PortRegistryLock::acquire(state_dir)?;
    check_port_collisions_locked(state_dir, instance_name, ports)
}

/// Lock-free core of [`check_port_collisions`]; caller must hold the
/// registry lock (or otherwise guarantee exclusive registry access).
fn check_port_collisions_locked(
    state_dir: &Path,
    instance_name: &str,
    ports: &[u16],
) -> Result<()> {
    if ports.is_empty() {
        return Ok(());
    }
    let run_dir = state_dir.join("var").join("run");
    if !run_dir.exists() {
        return Ok(());
    }

    for entry in std::fs::read_dir(&run_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        if stem == instance_name {
            continue; // self (restart case)
        }
        let Some(record) = read_record_loud(&path)? else {
            continue;
        };
        for port in ports {
            if record.ports.contains(port) {
                anyhow::bail!(
                    "port collision: port {} is already in use by sandbox '{}' \
                     (workload '{}', context {}).\n\
                     Sandbox '{}' cannot use this port.\n\
                     Remediation: change the port in one of the config repos, \
                     or stop the other sandbox with 'workestrate {} down'.",
                    port,
                    record.instance,
                    record.workload,
                    record.context.as_deref().unwrap_or("(none)"),
                    instance_name,
                    record.workload
                );
            }
        }
    }
    Ok(())
}

/// Atomically check for port collisions AND register the instance
/// (WP10/A17).
///
/// This is the atomic path: the registry lock is held across BOTH the
/// collision check and the registration, so no other process can slip a
/// registration for the same port between the two (the classic
/// time-of-check-time-of-use race).
///
/// **NOTE:** `runtime.rs` currently calls [`check_port_collisions`] and
/// [`register_sandbox_lifecycle`] SEPARATELY (with the sandbox-create `.await`
/// in between), so the live `up` path is NOT yet atomic — the two standalone
/// calls each lock only their own critical section. Closing A17 fully requires
/// migrating runtime.rs to this combined function (out of scope for WP10).
//
// `dead_code`: no in-crate caller yet (runtime.rs is outside WP10's owned
// set); the tests exercise it. Retained as the atomic entry point of the
// registry API for the follow-up runtime.rs migration.
#[allow(dead_code)]
#[allow(clippy::too_many_arguments)]
pub fn check_and_register_sandbox_lifecycle(
    state_dir: &Path,
    instance_name: &str,
    context: Option<&str>,
    workload: &str,
    host_ports: &[u16],
    port_pairs: &[PortMapping],
    port_offset: u16,
    created_at: &str,
) -> Result<()> {
    let _lock = PortRegistryLock::acquire(state_dir)?;
    check_port_collisions_locked(state_dir, instance_name, host_ports)?;
    register_sandbox_lifecycle_locked(
        state_dir,
        instance_name,
        context,
        workload,
        host_ports,
        port_pairs,
        port_offset,
        created_at,
    )
}

/// Register a running sandbox instance with its port mappings.
///
/// Writes `${state_dir}/var/run/<instance>.json`. Called after the sandbox
/// is successfully created.
//
// Retained as the minimal-registration entry point of the registry API;
// the CLI currently routes through [`register_sandbox_lifecycle`], but this
// simpler form is part of the ADR 0021 registry surface kept for callers
// that don't need the full lifecycle metadata.
//
// `dead_code`: no in-crate PRODUCTION caller yet, but it is exercised by the
// registry test-suite across store/slug/ps/runtime test modules (~30 call
// sites), so it is kept as a supported API rather than deleted (WP5 audit).
#[allow(dead_code)]
pub fn register_sandbox(
    state_dir: &Path,
    instance_name: &str,
    context: Option<&str>,
    workload: &str,
    ports: &[u16],
) -> Result<()> {
    // WP10/A17: lock the registry around the write so concurrent registrations
    // serialize instead of interleaving (each call is self-contained atomic).
    let _lock = PortRegistryLock::acquire(state_dir)?;
    let run_dir = state_dir.join("var").join("run");
    std::fs::create_dir_all(&run_dir)?;
    let record = SandboxInstanceRecord {
        instance: instance_name.to_string(),
        context: context.map(|s| s.to_string()),
        workload: workload.to_string(),
        ports: ports.to_vec(),
        port_pairs: Vec::new(),
        port_offset: None,
        created_at: String::new(),
    };
    let path = run_dir.join(format!("{}.json", instance_name));
    let content = serde_json::to_string_pretty(&record)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Register a running sandbox instance with full lifecycle metadata.
///
/// Writes `${state_dir}/var/run/<instance>.json` with the port pairs, offset,
/// and RFC3339 created-at timestamp populated. Locks the registry around the
/// write (WP10/A17); see [`check_and_register_sandbox_lifecycle`] for the
/// atomic check+register path.
#[allow(clippy::too_many_arguments)]
pub fn register_sandbox_lifecycle(
    state_dir: &Path,
    instance_name: &str,
    context: Option<&str>,
    workload: &str,
    host_ports: &[u16],
    port_pairs: &[PortMapping],
    port_offset: u16,
    created_at: &str,
) -> Result<()> {
    let _lock = PortRegistryLock::acquire(state_dir)?;
    register_sandbox_lifecycle_locked(
        state_dir,
        instance_name,
        context,
        workload,
        host_ports,
        port_pairs,
        port_offset,
        created_at,
    )
}

/// Lock-free core of [`register_sandbox_lifecycle`]; caller must hold the
/// registry lock.
#[allow(clippy::too_many_arguments)]
fn register_sandbox_lifecycle_locked(
    state_dir: &Path,
    instance_name: &str,
    context: Option<&str>,
    workload: &str,
    host_ports: &[u16],
    port_pairs: &[PortMapping],
    port_offset: u16,
    created_at: &str,
) -> Result<()> {
    let run_dir = state_dir.join("var").join("run");
    std::fs::create_dir_all(&run_dir)?;
    let record = SandboxInstanceRecord {
        instance: instance_name.to_string(),
        context: context.map(|s| s.to_string()),
        workload: workload.to_string(),
        ports: host_ports.to_vec(),
        port_pairs: port_pairs.to_vec(),
        port_offset: if port_offset == 0 {
            None
        } else {
            Some(port_offset)
        },
        created_at: created_at.to_string(),
    };
    let path = run_dir.join(format!("{}.json", instance_name));
    let content = serde_json::to_string_pretty(&record)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Unregister a sandbox instance (remove its state file).
///
/// Called when the sandbox is stopped/removed. Missing file is not an error.
pub fn unregister_sandbox(state_dir: &Path, instance_name: &str) -> Result<()> {
    let path = state_dir
        .join("var")
        .join("run")
        .join(format!("{}.json", instance_name));
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

/// Read the record for an instance, if any.
///
/// Returns Ok(None) when no state file exists. A corrupt or unreadable file
/// also yields Ok(None) (lenient — callers keep working) but emits a loud
/// WARNING (WP10/A12): corruption is no longer invisible.
pub fn find_record(state_dir: &Path, instance_name: &str) -> Result<Option<SandboxInstanceRecord>> {
    let path = state_dir
        .join("var")
        .join("run")
        .join(format!("{}.json", instance_name));
    read_record_loud(&path)
}

/// List all registered instance records in `${state_dir}/var/run/*.json`.
///
/// Corrupt files are skipped (lenient) with a loud WARNING each (WP10/A12).
/// Returns records in filesystem order (callers sort as needed).
pub fn list_records(state_dir: &Path) -> Result<Vec<SandboxInstanceRecord>> {
    let run_dir = state_dir.join("var").join("run");
    if !run_dir.exists() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&run_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Some(record) = read_record_loud(&path)? {
            out.push(record);
        }
    }
    Ok(out)
}

/// List all records whose `workload` field equals `workload`.
pub fn list_records_for_workload(
    state_dir: &Path,
    workload: &str,
) -> Result<Vec<SandboxInstanceRecord>> {
    Ok(list_records(state_dir)?
        .into_iter()
        .filter(|r| r.workload == workload)
        .collect())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::super::lock::PORT_REGISTRY_LOCK_NAME;
    use super::*;
    use crate::config::test_support::unique_state_dir;

    /// Small helper: a lifecycle registration for `instance` on `port`.
    fn combined_register(
        state_dir: &Path,
        instance: &str,
        workload: &str,
        port: u16,
    ) -> Result<()> {
        check_and_register_sandbox_lifecycle(
            state_dir,
            instance,
            None,
            workload,
            &[port],
            &[crate::microsandbox::plan::PortMapping {
                host: port,
                guest: port,
            }],
            0,
            "2026-07-23T00:00:00Z",
        )
    }

    #[test]
    fn no_collision_when_no_existing_sandboxes() -> Result<()> {
        let state_dir = unique_state_dir("empty");
        let result = check_port_collisions(&state_dir, "personal-litellm", &[4000]);
        assert!(
            result.is_ok(),
            "no collision expected when no existing sandboxes"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn no_collision_when_different_ports() -> Result<()> {
        let state_dir = unique_state_dir("diff-ports");
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        // A different sandbox with a different port — should not collide.
        let result = check_port_collisions(&state_dir, "personal-pi", &[3000]);
        assert!(
            result.is_ok(),
            "no collision expected with different ports: {:?}",
            result.err()
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn collision_when_same_port() -> Result<()> {
        let state_dir = unique_state_dir("same-port");
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        // A different sandbox trying to use the same port — should collide.
        let result = check_port_collisions(&state_dir, "work-litellm", &[4000]);
        assert!(result.is_err(), "collision expected with same port");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("port collision"),
            "error should mention 'port collision': {err}"
        );
        assert!(
            err.contains("4000"),
            "error should mention port 4000: {err}"
        );
        assert!(
            err.contains("personal-litellm"),
            "error should mention existing instance: {err}"
        );
        assert!(
            err.contains("litellm"),
            "error should mention workload: {err}"
        );
        assert!(
            err.contains("Remediation"),
            "error should mention remediation: {err}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn no_collision_with_self_on_restart() -> Result<()> {
        let state_dir = unique_state_dir("restart");
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        // Same instance name restarting — should not collide with itself.
        let result = check_port_collisions(&state_dir, "personal-litellm", &[4000]);
        assert!(result.is_ok(), "no self-collision expected on restart");
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn collision_names_both_sandboxes() -> Result<()> {
        let state_dir = unique_state_dir("both-names");
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000, 7000],
        )?;
        // Try to start work-odysseus on port 7000 — should collide.
        let result = check_port_collisions(&state_dir, "work-odysseus", &[7000]);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("personal-litellm"),
            "error should name the existing sandbox: {err}"
        );
        assert!(
            err.contains("work-odysseus"),
            "error should name the new sandbox (in context): {err}"
        );
        assert!(
            err.contains("7000"),
            "error should mention port 7000: {err}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn unregister_removes_state_file() -> Result<()> {
        let state_dir = unique_state_dir("unregister");
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        let file_path = state_dir
            .join("var")
            .join("run")
            .join("personal-litellm.json");
        assert!(file_path.exists(), "state file should exist after register");
        unregister_sandbox(&state_dir, "personal-litellm")?;
        assert!(
            !file_path.exists(),
            "state file should be removed after unregister"
        );
        // Unregister again — should not error (idempotent).
        unregister_sandbox(&state_dir, "personal-litellm")?;
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn empty_ports_no_collision() -> Result<()> {
        let state_dir = unique_state_dir("empty-ports");
        register_sandbox(
            &state_dir,
            "personal-agent",
            Some("personal"),
            "pi",
            &[4000],
        )?;
        // A sandbox with no port mappings should never collide.
        let result = check_port_collisions(&state_dir, "personal-other", &[]);
        assert!(result.is_ok());
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn corrupt_state_file_skipped() -> Result<()> {
        let state_dir = unique_state_dir("corrupt");
        let run_dir = state_dir.join("var").join("run");
        std::fs::create_dir_all(&run_dir)?;
        // Write a corrupt JSON file.
        std::fs::write(run_dir.join("corrupt-sandbox.json"), "not valid json")?;
        // Should not error — corrupt files are skipped.
        let result = check_port_collisions(&state_dir, "personal-litellm", &[4000]);
        assert!(
            result.is_ok(),
            "corrupt state file should be skipped, not error"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn register_and_read_back_record() -> Result<()> {
        let state_dir = unique_state_dir("roundtrip");
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000, 4001],
        )?;
        let file_path = state_dir
            .join("var")
            .join("run")
            .join("personal-litellm.json");
        let content = std::fs::read_to_string(&file_path)?;
        let record: SandboxInstanceRecord = serde_json::from_str(&content)?;
        assert_eq!(record.instance, "personal-litellm");
        assert_eq!(record.context, Some("personal".to_string()));
        assert_eq!(record.workload, "litellm");
        assert_eq!(record.ports, vec![4000, 4001]);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- Instance lifecycle (ADR 0021) extensions ----

    #[test]
    fn register_lifecycle_round_trips_new_fields() -> Result<()> {
        let state_dir = unique_state_dir("lifecycle");
        let pairs = vec![
            crate::microsandbox::plan::PortMapping {
                host: 14000,
                guest: 4000,
            },
            crate::microsandbox::plan::PortMapping {
                host: 14001,
                guest: 4001,
            },
        ];
        register_sandbox_lifecycle(
            &state_dir,
            "personal-litellm@canary",
            Some("personal"),
            "litellm",
            &[14000, 14001],
            &pairs,
            10000,
            "2026-07-20T14:05:42Z",
        )?;
        let record = find_record(&state_dir, "personal-litellm@canary")?
            .expect("record should exist after register_lifecycle");
        assert_eq!(record.instance, "personal-litellm@canary");
        assert_eq!(record.workload, "litellm");
        assert_eq!(record.context.as_deref(), Some("personal"));
        assert_eq!(record.ports, vec![14000, 14001]);
        assert_eq!(record.port_pairs.len(), 2);
        assert_eq!(record.port_pairs[0].host, 14000);
        assert_eq!(record.port_pairs[0].guest, 4000);
        assert_eq!(record.port_offset, Some(10000));
        assert_eq!(record.created_at, "2026-07-20T14:05:42Z");
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn register_lifecycle_zero_offset_serializes_as_none() -> Result<()> {
        let state_dir = unique_state_dir("zero-offset");
        register_sandbox_lifecycle(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
            &[crate::microsandbox::plan::PortMapping {
                host: 4000,
                guest: 4000,
            }],
            0,
            "2026-07-20T14:03:11Z",
        )?;
        let record = find_record(&state_dir, "personal-litellm")?.unwrap();
        assert_eq!(record.port_offset, None, "offset 0 must serialize as None");
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn legacy_record_without_new_fields_parses() -> Result<()> {
        // A pre-lifecycle state file (only the original four fields) must
        // still parse with #[serde(default)] on the new fields.
        let state_dir = unique_state_dir("legacy");
        let run_dir = state_dir.join("var").join("run");
        std::fs::create_dir_all(&run_dir)?;
        std::fs::write(
            run_dir.join("legacy-litellm.json"),
            r#"{
  "instance": "legacy-litellm",
  "context": null,
  "workload": "litellm",
  "ports": [4000]
}"#,
        )?;
        let record = find_record(&state_dir, "legacy-litellm")?.expect("legacy record must parse");
        assert_eq!(record.instance, "legacy-litellm");
        assert_eq!(record.ports, vec![4000]);
        assert!(record.port_pairs.is_empty());
        assert_eq!(record.port_offset, None);
        assert!(record.created_at.is_empty());
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn find_record_returns_none_for_missing() -> Result<()> {
        let state_dir = unique_state_dir("missing");
        let record = find_record(&state_dir, "absent")?;
        assert!(record.is_none());
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn find_record_returns_none_for_corrupt() -> Result<()> {
        let state_dir = unique_state_dir("corrupt-find");
        let run_dir = state_dir.join("var").join("run");
        std::fs::create_dir_all(&run_dir)?;
        std::fs::write(run_dir.join("corrupt.json"), "not json")?;
        let record = find_record(&state_dir, "corrupt")?;
        assert!(record.is_none(), "corrupt record should yield None");
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn list_records_returns_all_valid() -> Result<()> {
        let state_dir = unique_state_dir("list-all");
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        register_sandbox(&state_dir, "personal-pi", Some("personal"), "pi", &[3000])?;
        register_sandbox_lifecycle(
            &state_dir,
            "personal-litellm@canary",
            Some("personal"),
            "litellm",
            &[14000],
            &[crate::microsandbox::plan::PortMapping {
                host: 14000,
                guest: 4000,
            }],
            10000,
            "2026-07-20T14:05:42Z",
        )?;
        let records = list_records(&state_dir)?;
        assert_eq!(
            records.len(),
            3,
            "expected 3 records, got {}",
            records.len()
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn list_records_for_workload_filters_correctly() -> Result<()> {
        let state_dir = unique_state_dir("list-workload");
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        register_sandbox(&state_dir, "personal-pi", Some("personal"), "pi", &[3000])?;
        register_sandbox(
            &state_dir,
            "personal-litellm@canary",
            Some("personal"),
            "litellm",
            &[14000],
        )?;
        let litellm_records = list_records_for_workload(&state_dir, "litellm")?;
        assert_eq!(
            litellm_records.len(),
            2,
            "expected 2 litellm records (singleton + parallel)"
        );
        for r in &litellm_records {
            assert_eq!(r.workload, "litellm");
        }
        let pi_records = list_records_for_workload(&state_dir, "pi")?;
        assert_eq!(pi_records.len(), 1);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- WP10/A17: atomic check+register ----

    #[test]
    fn combined_registers_when_no_collision() -> Result<()> {
        let state_dir = unique_state_dir("combined-ok");
        combined_register(&state_dir, "personal-litellm", "litellm", 4000)?;
        let record = find_record(&state_dir, "personal-litellm")?
            .expect("combined call should have registered the record");
        assert_eq!(record.ports, vec![4000]);
        // Lock file must be gone after the guard dropped.
        assert!(
            !state_dir
                .join("var")
                .join("run")
                .join(PORT_REGISTRY_LOCK_NAME)
                .exists(),
            "lock file should be released after the combined call"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn combined_errors_on_collision_and_leaves_no_record() -> Result<()> {
        let state_dir = unique_state_dir("combined-collide");
        combined_register(&state_dir, "personal-litellm", "litellm", 4000)?;
        // Second instance on the same port must fail the check …
        let err = combined_register(&state_dir, "work-litellm", "litellm", 4000).unwrap_err();
        assert!(
            err.to_string().contains("port collision"),
            "expected a collision error; got: {err}"
        );
        // … and must NOT have left a registered record behind.
        assert!(
            find_record(&state_dir, "work-litellm")?.is_none(),
            "failed combined call must not leave a registered record"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- WP10/A12: corrupt records are loud but non-fatal ----

    #[test]
    fn corrupt_record_still_skipped_but_valid_collision_still_fires() -> Result<()> {
        let state_dir = unique_state_dir("corrupt-loud");
        let run_dir = state_dir.join("var").join("run");
        std::fs::create_dir_all(&run_dir)?;
        std::fs::write(run_dir.join("corrupt-sandbox.json"), "{ not valid json")?;
        // A VALID record holding port 4000 sits next to the corrupt one.
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;

        // check_port_collisions: succeeds for a free port (corrupt file
        // skipped — loudly — not fatal) …
        assert!(check_port_collisions(&state_dir, "new-sandbox", &[3000]).is_ok());
        // … and still detects the collision from the VALID record.
        let err = check_port_collisions(&state_dir, "new-sandbox", &[4000]).unwrap_err();
        assert!(
            err.to_string().contains("port collision"),
            "valid record's collision must still fire despite the corrupt sibling: {err}"
        );

        // list_records: corrupt skipped, valid present.
        let records = list_records(&state_dir)?;
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].instance, "personal-litellm");

        // find_record: corrupt → None (with a warning), still Ok.
        assert!(find_record(&state_dir, "corrupt-sandbox")?.is_none());

        // The corrupt file itself is untouched (not silently deleted).
        assert!(run_dir.join("corrupt-sandbox.json").exists());
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }
}
