use crate::microsandbox::plan::PortMapping;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A running sandbox instance record stored in the port registry.
///
/// **Backward-compat:** `ports` (host-only u16 list) and the original four
/// fields are always present. Newer fields (`port_pairs`, `port_offset`,
/// `created_at`) are `#[serde(default)]` so older state files parse cleanly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInstanceRecord {
    pub instance: String,
    pub context: Option<String>,
    pub workload: String,
    pub ports: Vec<u16>,
    /// Full host:guest port pairs (post-offset host). Populated by the
    /// instance-lifecycle path; absent (empty) on legacy records.
    #[serde(default)]
    pub port_pairs: Vec<PortMapping>,
    /// Effective `--port-offset N` applied when this instance was started.
    /// `None` for legacy records and `--port-offset 0`.
    #[serde(default)]
    pub port_offset: Option<u16>,
    /// RFC3339 timestamp the instance was registered. Empty for legacy records.
    #[serde(default)]
    pub created_at: String,
}

/// Check for host-port collisions against already-running workestrate sandboxes.
///
/// Scans `${state_dir}/var/run/*.json` for port mappings. Excludes the file
/// for `instance_name` (in case it's a restart). If any port in `ports`
/// matches a port in another instance's record, returns a hard error naming
/// both sandboxes, the port, and remediation.
pub fn check_port_collisions(state_dir: &Path, instance_name: &str, ports: &[u16]) -> Result<()> {
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
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue, // unreadable file, skip
        };
        let record: SandboxInstanceRecord = match serde_json::from_str(&content) {
            Ok(r) => r,
            Err(_) => continue, // corrupt file, skip
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

/// Register a running sandbox instance with its port mappings.
///
/// Writes `${state_dir}/var/run/<instance>.json`. Called after the sandbox
/// is successfully created.
//
// Retained as the minimal-registration entry point of the registry API;
// the CLI currently routes through [`register_sandbox_lifecycle`], but this
// simpler form is part of the ADR 0021 registry surface kept for callers
// that don't need the full lifecycle metadata.
#[allow(dead_code)]
pub fn register_sandbox(
    state_dir: &Path,
    instance_name: &str,
    context: Option<&str>,
    workload: &str,
    ports: &[u16],
) -> Result<()> {
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
/// and RFC3339 created-at timestamp populated.
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
/// Returns Ok(None) when no state file exists or the file is corrupt
/// (corrupt files are silently skipped — same lenient policy as
/// [`check_port_collisions`]).
pub fn find_record(state_dir: &Path, instance_name: &str) -> Result<Option<SandboxInstanceRecord>> {
    let path = state_dir
        .join("var")
        .join("run")
        .join(format!("{}.json", instance_name));
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Ok(None);
    };
    Ok(serde_json::from_str(&content).ok())
}

/// List all registered instance records in `${state_dir}/var/run/*.json`.
///
/// Corrupt files are skipped (lenient). Returns records in filesystem order
/// (callers sort as needed).
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
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Ok(record) = serde_json::from_str::<SandboxInstanceRecord>(&content) {
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

/// Alphabet for `--new` instance slugs: lowercase base32 (RFC 4648), i.e.
/// `a-z` + `2-7` — the symbols `0`/`1`/`8`/`9` are excluded (ambiguous with
/// letters). 32 symbols means each byte of entropy maps cleanly (`256 % 32
/// == 0`), so `byte % 32` is a uniform, bias-free draw onto the alphabet.
///
/// Normative for ADR 0021 §2 (`--new` allocation): slug length is fixed at 4
/// (≈ 20 bits, namespace 32^4 = 1,048,576 per slot), drawn from `/dev/urandom`.
pub(crate) const SLUG_ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz234567";

/// Slug length produced by [`auto_allocate_slug`]. Normative (ADR 0021 §2).
pub(crate) const SLUG_LEN: usize = 4;

/// Maximum collision/validity retries before `--new` gives up (ADR 0021 §2).
pub(crate) const SLUG_MAX_RETRIES: u32 = 8;

/// Auto-allocate a fresh instance slug for `--new`.
///
/// Draws 4-char `[a-z2-7]` slugs from `/dev/urandom` over [`SLUG_ALPHABET`],
/// retrying (up to [`SLUG_MAX_RETRIES`] attempts) when a draw either:
///   - collides with an existing `<slot>@<slug>` record, OR
///   - is purely numeric (e.g. `2345`), which [`validate_instance_id`]
///     (the uniform id rule) rejects.
///
/// The returned slug is therefore **guaranteed** to satisfy the instance-id
/// slug rule and to be unique among the slot's parallel instances. Returns a
/// hard error only if every attempt is rejected — effectively unreachable
/// given the namespace size, but fail-closed by construction.
///
/// [`validate_instance_id`]: crate::microsandbox::slots::validate_instance_id
pub fn auto_allocate_slug(state_dir: &Path, slot: &str) -> Result<String> {
    auto_allocate_slug_with(state_dir, slot, SLUG_MAX_RETRIES, random_slug)
}

/// Testable core of [`auto_allocate_slug`]: takes an explicit retry budget and
/// a `draw` closure (so tests can inject deterministic candidate sequences)
/// but is otherwise identical to the public entry point.
fn auto_allocate_slug_with<F>(
    state_dir: &Path,
    slot: &str,
    retries: u32,
    mut draw: F,
) -> Result<String>
where
    F: FnMut() -> Result<String>,
{
    let used = used_slugs_for_slot(state_dir, slot)?;
    for _ in 0..retries {
        let candidate = draw()?;
        // Reject purely-numeric draws up front (validate_instance_id would)
        // and reject collisions with existing parallel instances.
        if candidate.chars().any(|c| c.is_ascii_alphabetic()) && !used.contains(&candidate) {
            return Ok(candidate);
        }
    }
    anyhow::bail!(
        "could not allocate a non-colliding instance slug for slot '{}' after {} attempts          (namespace unexpectedly saturated)",
        slot,
        retries,
    )
}

/// Collect the parallel-instance suffixes already in use for `slot` — the
/// `<id>` portion of every `<slot>@<id>` record (any shape; integer slugs,
/// named slugs, etc. all count). Singleton records (`<slot>` with no `@`) and
/// records for other slots are ignored.
fn used_slugs_for_slot(state_dir: &Path, slot: &str) -> Result<Vec<String>> {
    let prefix = format!("{}@", slot);
    let mut out = Vec::new();
    for r in list_records(state_dir)? {
        if let Some(suffix) = r.instance.strip_prefix(&prefix) {
            out.push(suffix.to_string());
        }
    }
    Ok(out)
}

/// Draw one [`SLUG_LEN`]-char `[a-z2-7]` slug from `/dev/urandom`.
///
/// No new dependency: `/dev/urandom` is read directly. Each byte maps
/// uniformly onto [`SLUG_ALPHABET`] (`256 % 32 == 0` → no modulo bias).
fn random_slug() -> Result<String> {
    use std::io::Read;
    let mut buf = [0u8; SLUG_LEN];
    let mut f = std::fs::File::open("/dev/urandom")?;
    f.read_exact(&mut buf)?;
    Ok(buf
        .iter()
        .map(|&b| SLUG_ALPHABET[(b as usize) % SLUG_ALPHABET.len()] as char)
        .collect())
}

/// Remove all state files whose record.workload matches `workload`.
///
/// Returns the list of instance names whose state files were removed.
/// Used by `down --all-instances` to clean up state for instances that no
/// longer have a backing sandbox.
//
// Part of the ADR 0021 registry surface; the live `down` paths currently
// tear down state inline, but this bulk helper is retained for parity with
// [`unregister_sandbox`] and future callers.
#[allow(dead_code)]
pub fn unregister_all_for_workload(state_dir: &Path, workload: &str) -> Result<Vec<String>> {
    let records = list_records_for_workload(state_dir, workload)?;
    let mut removed = Vec::with_capacity(records.len());
    for r in &records {
        if unregister_sandbox(state_dir, &r.instance).is_ok() {
            removed.push(r.instance.clone());
        }
    }
    Ok(removed)
}

/// Remove every state file in the registry. Returns the list of instance names
/// whose files were removed. Used by `down --all`.
//
// Part of the ADR 0021 registry surface; retained for parity with
// [`unregister_sandbox`] and the `down --all` cleanup path.
#[allow(dead_code)]
pub fn unregister_all(state_dir: &Path) -> Result<Vec<String>> {
    let records = list_records(state_dir)?;
    let mut removed = Vec::with_capacity(records.len());
    for r in &records {
        if unregister_sandbox(state_dir, &r.instance).is_ok() {
            removed.push(r.instance.clone());
        }
    }
    Ok(removed)
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn unique_state_dir(label: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "workestrate-port-{}-{}-{}",
            label,
            std::process::id(),
            nanos,
        ))
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

    // ---- auto_allocate_slug (ADR 0021 §2, WP-B) ----

    #[test]
    fn random_slug_is_4_chars_of_base32_alphabet() -> Result<()> {
        for _ in 0..256 {
            let s = random_slug()?;
            assert_eq!(
                s.len(),
                SLUG_LEN,
                "slug must be exactly {SLUG_LEN} chars: {s}"
            );
            assert!(
                s.bytes()
                    .all(|b| b.is_ascii_lowercase() || (b'2'..=b'7').contains(&b)),
                "slug '{s}' contains a char outside [a-z2-7]"
            );
        }
        Ok(())
    }

    #[test]
    fn auto_allocate_slug_shape_and_validates() -> Result<()> {
        let state_dir = unique_state_dir("slug-empty");
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        let slug = auto_allocate_slug(&state_dir, "personal-litellm")?;
        assert_eq!(slug.len(), SLUG_LEN);
        assert!(
            slug.bytes()
                .all(|b| b.is_ascii_lowercase() || (b'2'..=b'7').contains(&b)),
            "slug '{slug}' outside [a-z2-7]"
        );
        crate::microsandbox::slots::validate_instance_id(&slug)
            .expect("allocated slug must satisfy the instance-id slug rule");
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn auto_allocate_slug_avoids_existing_slugs_via_retry() -> Result<()> {
        let state_dir = unique_state_dir("slug-collide");
        register_sandbox(
            &state_dir,
            "personal-litellm@ab2z",
            Some("personal"),
            "litellm",
            &[14000],
        )?;
        let mut calls = 0u32;
        let draw = || -> Result<String> {
            calls += 1;
            Ok(if calls == 1 {
                "ab2z".to_string()
            } else {
                "mnxy".to_string()
            })
        };
        let slug = auto_allocate_slug_with(&state_dir, "personal-litellm", SLUG_MAX_RETRIES, draw)?;
        assert_eq!(
            slug, "mnxy",
            "must skip the colliding draw and return the fresh one"
        );
        assert_eq!(calls, 2);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn auto_allocate_slug_retries_past_purely_numeric_draws() -> Result<()> {
        let state_dir = unique_state_dir("slug-numeric");
        let mut calls = 0u32;
        let draw = || -> Result<String> {
            calls += 1;
            Ok(if calls == 1 {
                "2345".to_string()
            } else {
                "abcd".to_string()
            })
        };
        let slug = auto_allocate_slug_with(&state_dir, "personal-litellm", SLUG_MAX_RETRIES, draw)?;
        assert_eq!(slug, "abcd");
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn auto_allocate_slug_ignores_non_slug_and_other_slot_records() -> Result<()> {
        let state_dir = unique_state_dir("slug-ignores");
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        register_sandbox(
            &state_dir,
            "personal-litellm@2",
            Some("personal"),
            "litellm",
            &[14001],
        )?;
        register_sandbox(
            &state_dir,
            "personal-litellm@canary",
            Some("personal"),
            "litellm",
            &[14002],
        )?;
        register_sandbox(
            &state_dir,
            "work-litellm@ab2z",
            Some("work"),
            "litellm",
            &[24000],
        )?;
        register_sandbox(&state_dir, "personal-pi", Some("personal"), "pi", &[3000])?;
        let used = used_slugs_for_slot(&state_dir, "personal-litellm")?;
        assert_eq!(used.len(), 2, "only the two personal-litellm@* suffixes");
        assert!(used.contains(&"2".to_string()));
        assert!(used.contains(&"canary".to_string()));
        let slug = auto_allocate_slug(&state_dir, "personal-litellm")?;
        assert!(!used.contains(&slug), "allocated slug must not collide");
        crate::microsandbox::slots::validate_instance_id(&slug)?;
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn auto_allocate_slug_errors_when_retries_exhausted() -> Result<()> {
        let state_dir = unique_state_dir("slug-exhaust");
        register_sandbox(
            &state_dir,
            "personal-litellm@ab2z",
            Some("personal"),
            "litellm",
            &[14000],
        )?;
        let draw = || -> Result<String> { Ok("ab2z".to_string()) };
        let err = auto_allocate_slug_with(&state_dir, "personal-litellm", 3, draw).unwrap_err();
        assert!(
            err.to_string().contains("could not allocate"),
            "expected exhaustion error; got: {err}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn unregister_all_for_workload_removes_only_matching() -> Result<()> {
        let state_dir = unique_state_dir("unregister-workload");
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        register_sandbox(
            &state_dir,
            "personal-litellm@canary",
            Some("personal"),
            "litellm",
            &[14000],
        )?;
        register_sandbox(&state_dir, "personal-pi", Some("personal"), "pi", &[3000])?;
        let removed = unregister_all_for_workload(&state_dir, "litellm")?;
        assert_eq!(removed.len(), 2);
        // pi's state file must remain.
        assert!(find_record(&state_dir, "personal-pi")?.is_some());
        assert!(find_record(&state_dir, "personal-litellm")?.is_none());
        assert!(find_record(&state_dir, "personal-litellm@canary")?.is_none());
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn unregister_all_removes_every_record() -> Result<()> {
        let state_dir = unique_state_dir("unregister-all");
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        register_sandbox(&state_dir, "personal-pi", Some("personal"), "pi", &[3000])?;
        let removed = unregister_all(&state_dir)?;
        assert_eq!(removed.len(), 2);
        assert!(list_records(&state_dir)?.is_empty());
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }
}
