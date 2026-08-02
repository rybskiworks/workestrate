use super::lock::PortRegistryLock;
use super::SandboxInstanceRecord;
use crate::microsandbox::plan::PortMapping;
use anyhow::Result;
use std::net::{IpAddr, Ipv4Addr};
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
/// for `instance_name` (in case it's a restart). Collisions are keyed on
/// `(bind_ip, port)` (ADR 0026(b)): if any pair in `pairs` matches both the
/// bind IP and a port of another instance's record, returns a hard error
/// naming both sandboxes, the bind:port, and remediation.
///
/// Acquires the registry lock around its own critical section (WP10/A17):
/// this makes each individual call atomic against concurrent registrations,
/// but does NOT close the check-then-register race across two separate calls
/// — see [`check_and_register_sandbox_lifecycle`] for the atomic path.
///
/// No production caller since FN-6 (build_sandbox routes through the atomic
/// combined entry point); retained as a supported registry API and exercised
/// across the store/lock test modules.
#[allow(dead_code)]
pub fn check_port_collisions(
    state_dir: &Path,
    instance_name: &str,
    pairs: &[(IpAddr, u16)],
) -> Result<()> {
    let _lock = PortRegistryLock::acquire(state_dir)?;
    check_port_collisions_locked(state_dir, instance_name, pairs)
}

/// Lock-free core of [`check_port_collisions`]; caller must hold the
/// registry lock (or otherwise guarantee exclusive registry access).
fn check_port_collisions_locked(
    state_dir: &Path,
    instance_name: &str,
    pairs: &[(IpAddr, u16)],
) -> Result<()> {
    if pairs.is_empty() {
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
        for (bind, port) in pairs {
            if record.bind_ip == *bind && record.ports.contains(port) {
                anyhow::bail!(
                    "port collision: {}:{} is already in use by sandbox '{}' \
                     (workload '{}', context {}).\n\
                     Sandbox '{}' cannot use this bind:port.\n\
                     Remediation: change the port in one of the config repos, \
                     or stop the other sandbox with 'workestrate {} down'.",
                    bind,
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
/// registration for the same `(bind_ip, port)` pair between the two (the
/// classic time-of-check-time-of-use race). Collisions are keyed on
/// `(bind_ip, port)` (ADR 0026(b)); the record stores `bind_ip`.
///
/// Since FN-6 this IS the live path: `build_sandbox` (runtime/run.rs)
/// registers through here after the sandbox create resolves. The async create
/// itself cannot sit inside the lock (the lock is a file; holding it across
/// `.await` would wedge concurrent processes), so a same-port race can still
/// collide mid-create — but the post-create registration window is closed:
/// the loser's combined call fails the collision check and leaves no record.
// too_many_arguments: the ADR 0021 registry surface is positional by design
// (identity, bind, ports, metadata); the bind_ip addition (ADR 0026) pushes
// the count to 8. A params struct is deferred to the C2 wiring commit.
#[allow(clippy::too_many_arguments)]
pub fn check_and_register_sandbox_lifecycle(
    state_dir: &Path,
    instance_name: &str,
    context: Option<&str>,
    workload: &str,
    bind_ip: IpAddr,
    host_ports: &[u16],
    port_pairs: &[PortMapping],
    created_at: &str,
) -> Result<()> {
    let _lock = PortRegistryLock::acquire(state_dir)?;
    let pairs: Vec<(IpAddr, u16)> = host_ports.iter().map(|p| (bind_ip, *p)).collect();
    check_port_collisions_locked(state_dir, instance_name, &pairs)?;
    register_sandbox_lifecycle_locked(
        state_dir,
        instance_name,
        context,
        workload,
        bind_ip,
        host_ports,
        port_pairs,
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
        created_at: String::new(),
        bind_ip: crate::microsandbox::plan::default_bind_ip(),
    };
    let path = run_dir.join(format!("{}.json", instance_name));
    let content = serde_json::to_string_pretty(&record)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Register a running sandbox instance with full lifecycle metadata.
///
/// Writes `${state_dir}/var/run/<instance>.json` with the port pairs and
/// RFC3339 created-at timestamp populated. Locks the registry around the
/// write (WP10/A17); see [`check_and_register_sandbox_lifecycle`] for the
/// atomic check+register path.
///
/// No production caller since FN-6 (build_sandbox uses the atomic combined
/// entry point); retained as the register-only half of the registry API and
/// exercised by the store/ps test modules.
// too_many_arguments: see check_and_register_sandbox_lifecycle.
#[allow(dead_code, clippy::too_many_arguments)]
pub fn register_sandbox_lifecycle(
    state_dir: &Path,
    instance_name: &str,
    context: Option<&str>,
    workload: &str,
    bind_ip: IpAddr,
    host_ports: &[u16],
    port_pairs: &[PortMapping],
    created_at: &str,
) -> Result<()> {
    let _lock = PortRegistryLock::acquire(state_dir)?;
    register_sandbox_lifecycle_locked(
        state_dir,
        instance_name,
        context,
        workload,
        bind_ip,
        host_ports,
        port_pairs,
        created_at,
    )
}

/// Lock-free core of [`register_sandbox_lifecycle`]; caller must hold the
/// registry lock.
// too_many_arguments: see check_and_register_sandbox_lifecycle.
#[allow(clippy::too_many_arguments)]
fn register_sandbox_lifecycle_locked(
    state_dir: &Path,
    instance_name: &str,
    context: Option<&str>,
    workload: &str,
    bind_ip: IpAddr,
    host_ports: &[u16],
    port_pairs: &[PortMapping],
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
        created_at: created_at.to_string(),
        bind_ip,
    };
    let path = run_dir.join(format!("{}.json", instance_name));
    let content = serde_json::to_string_pretty(&record)?;
    std::fs::write(&path, content)?;
    Ok(())
}

/// Unregister a sandbox instance (remove its state file).
///
/// Called when the sandbox is stopped/removed. Missing file is not an error.
///
/// The registry lock is held ONLY around the synchronous existence-check +
/// `remove_file` (FN-6). Callers (`down`, `down_one`,
/// `check_occupied_or_replace`) are async and stop the sandbox BEFORE calling
/// here: holding the lock across that `.await` would let a stalled stop wedge
/// every concurrent `up` behind the lock until its acquire timeout — the
/// guard is scoped to the synchronous remove so it can never cross an await.
pub fn unregister_sandbox(state_dir: &Path, instance_name: &str) -> Result<()> {
    let _lock = PortRegistryLock::acquire(state_dir)?;
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

// ---------------------------------------------------------------------------
// Loopback allocator (ADR 0026(a): parallel-slot bind IPs)
// ---------------------------------------------------------------------------

/// Allocate a per-instance loopback bind IP (`127.0.0.N`, `N >= 2`) for a
/// parallel slot (ADR 0026(a)).
///
/// Semantics (normative, ADR 0026(a)):
///
/// - `127.0.0.1` is the shared singleton bind and is NEVER allocated — the
///   allocator only draws from `127.0.0.2..=127.0.0.254`.
/// - Allocation is lowest-free across the CURRENT records: the returned IP is
///   the lowest `127.0.0.N` (`N` in `2..=254`) that is not any record's
///   `bind_ip`.
/// - An IP is freed automatically when its record is removed
///   (`unregister_sandbox` / `down`): the next allocation re-selects it.
/// - A STALE record (backing sandbox gone, record file still present) still
///   RESERVES its IP until the record is cleared — conservative by design:
///   it prevents handing an IP to a new instance while an untracked sandbox
///   might still hold it.
///
/// The registry lock is held across list + select (fail-closed): two
/// concurrent allocations cannot return the same IP. A second allocation
/// still colliding at publish time is caught by the `(bind_ip, port)`
/// collision check in [`check_and_register_sandbox_lifecycle`].
///
/// Hard error when the range is exhausted (all 253 addresses reserved).
pub fn allocate_loopback_ip(state_dir: &Path) -> Result<IpAddr> {
    let _lock = PortRegistryLock::acquire(state_dir)?;
    let records = list_records(state_dir)?;
    select_loopback(&records)
}

/// Shared selection core of [`allocate_loopback_ip`] and
/// [`prospective_loopback_ip`]: apply [`lowest_free_loopback`] to `records`
/// and map exhaustion to the hard error. Keeping BOTH entry points on this
/// one selection path guarantees the prospective (plan) view can never
/// diverge from the real (up) allocation.
fn select_loopback(records: &[SandboxInstanceRecord]) -> Result<IpAddr> {
    match lowest_free_loopback(records) {
        Some(ip) => Ok(IpAddr::V4(ip)),
        None => anyhow::bail!(
            "loopback bind address space exhausted: every 127.0.0.N (N in 2..=254) \
             is reserved by a registered instance. Stop unused instances \
             ('workestrate <workload> down', 'workestrate down --all') or clear \
             stale registry records before starting another parallel instance."
        ),
    }
}

/// The bind IP a parallel slot WOULD draw right now (ADR 0026(a)/C2) — the
/// prospective view used by `plan --instance <id>`.
///
/// READ-ONLY SNAPSHOT: this lists the current records WITHOUT acquiring the
/// registry lock and reserves nothing. It exists purely for plan rendering;
/// the authoritative allocation is [`allocate_loopback_ip`] (locked), which
/// shares the same selection core ([`select_loopback`]) so the two can never
/// diverge on the same registry view. A concurrent `up` between the plan
/// render and a later real `up` can of course change the answer — the plan
/// output is a point-in-time preview, not a reservation.
pub fn prospective_loopback_ip(state_dir: &Path) -> Result<IpAddr> {
    let records = list_records(state_dir)?;
    select_loopback(&records)
}

// ---------------------------------------------------------------------------
// --port-auto port probing (ADR 0026(c))
// ---------------------------------------------------------------------------

/// Probe `count` OS-free ports on `bind` for a `--port-auto` publish
/// (ADR 0026(c)).
///
/// NOT A RESERVATION (normative): the probed ports are released immediately
/// (each probe listener is dropped the moment its port is known). The
/// registry lock only serializes the PROBE against other workestrate
/// processes — it cannot stop a non-workestrate process (or the OS, or a
/// sandbox mid-create) from claiming a probed port the instant the lock
/// drops. Two mechanisms close the remaining window:
///
/// - the post-create atomic check+register (FN-6,
///   [`check_and_register_sandbox_lifecycle`]) fails the create if the
///   chosen `(bind, port)` was registered by another workestrate instance in
///   the meantime;
/// - the sandbox's own publish fails at create time if the port was claimed
///   outside the registry.
///
/// The chosen ports are recorded in the instance record, so `ps` / `down`
/// recover them afterwards.
///
/// A candidate is skipped (re-probed) when it (a) duplicates a port already
/// probed in THIS call, or (b) appears in any registry record's `ports` on
/// the SAME `bind` — defense-in-depth over records whose sandboxes may not
/// have bound the OS port yet. After `3 * count` attempts without enough
/// distinct candidates the probe bails (fail-closed) rather than loop.
pub fn probe_free_ports(state_dir: &Path, bind: IpAddr, count: usize) -> Result<Vec<u16>> {
    let _lock = PortRegistryLock::acquire(state_dir)?;
    let records = list_records(state_dir)?;
    let mut probed: Vec<u16> = Vec::with_capacity(count);
    let mut attempts = 0usize;
    let max_attempts = 3 * count;
    while probed.len() < count {
        attempts += 1;
        if attempts > max_attempts {
            anyhow::bail!(
                "--port-auto: failed to probe {} distinct free port(s) on {} after {} attempts. \
                 Retry, or publish explicit ports (drop --port-auto).",
                count,
                bind,
                max_attempts
            );
        }
        // Ephemeral bind probe: port 0 asks the OS for a free port on `bind`;
        // the listener is dropped immediately (see the NOT-A-RESERVATION note
        // in the doc comment above).
        let port = std::net::TcpListener::bind((bind, 0))?.local_addr()?.port();
        // (a) Skip duplicates within this call (two ephemeral probes can
        //     return the same port once the first listener is dropped).
        if probed.contains(&port) {
            continue;
        }
        // (b) Skip ports a registry record already holds on the SAME bind —
        //     the record may belong to a sandbox that has not bound the OS
        //     port yet, so an OS-free answer is not sufficient.
        let recorded = records
            .iter()
            .any(|r| r.bind_ip == bind && r.ports.contains(&port));
        if recorded {
            continue;
        }
        probed.push(port);
    }
    Ok(probed)
}

/// Pure core of [`allocate_loopback_ip`]: the lowest `127.0.0.N` (`N` in
/// `2..=254`) not present as any record's `bind_ip`. `None` when the range
/// is exhausted. Records bound on `127.0.0.1` (or any address outside the
/// allocatable range) do not consume allocator slots.
fn lowest_free_loopback(records: &[SandboxInstanceRecord]) -> Option<Ipv4Addr> {
    (2..=254u16)
        .map(|n| Ipv4Addr::new(127, 0, 0, n as u8))
        .find(|candidate| !records.iter().any(|r| r.bind_ip == IpAddr::V4(*candidate)))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::super::lock::PORT_REGISTRY_LOCK_NAME;
    use super::*;
    use crate::config::test_support::unique_state_dir;

    /// The shared singleton bind (127.0.0.1) as an owned `IpAddr`.
    fn singleton_bind() -> IpAddr {
        crate::microsandbox::plan::default_bind_ip()
    }

    /// Small helper: a lifecycle registration for `instance` on `port`,
    /// bound to the shared singleton 127.0.0.1.
    fn combined_register(
        state_dir: &Path,
        instance: &str,
        workload: &str,
        port: u16,
    ) -> Result<()> {
        register_instance_on(state_dir, instance, workload, singleton_bind(), port)
    }

    /// Small helper: a lifecycle registration for `instance` on `port` bound
    /// to an explicit `bind` (ADR 0026: (bind, port)-keyed tests).
    fn register_instance_on(
        state_dir: &Path,
        instance: &str,
        workload: &str,
        bind: IpAddr,
        port: u16,
    ) -> Result<()> {
        check_and_register_sandbox_lifecycle(
            state_dir,
            instance,
            None,
            workload,
            bind,
            &[port],
            &[crate::microsandbox::plan::PortMapping::new(port, port)],
            "2026-07-23T00:00:00Z",
        )
    }

    #[test]
    fn no_collision_when_no_existing_sandboxes() -> Result<()> {
        let state_dir = unique_state_dir("empty");
        let result =
            check_port_collisions(&state_dir, "personal-litellm", &[(singleton_bind(), 4000)]);
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
        let result = check_port_collisions(&state_dir, "personal-pi", &[(singleton_bind(), 3000)]);
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
        // A different sandbox trying to use the same port on the SAME bind —
        // should collide.
        let result = check_port_collisions(&state_dir, "work-litellm", &[(singleton_bind(), 4000)]);
        assert!(result.is_err(), "collision expected with same port");
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("port collision"),
            "error should mention 'port collision': {err}"
        );
        assert!(
            err.contains("127.0.0.1:4000"),
            "error should name the bind:port 127.0.0.1:4000: {err}"
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
        let result =
            check_port_collisions(&state_dir, "personal-litellm", &[(singleton_bind(), 4000)]);
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
        // Try to start work-odysseus on port 7000 (same bind) — should collide.
        let result =
            check_port_collisions(&state_dir, "work-odysseus", &[(singleton_bind(), 7000)]);
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
            err.contains("127.0.0.1:7000"),
            "error should name the bind:port 127.0.0.1:7000: {err}"
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
        let result =
            check_port_collisions(&state_dir, "personal-litellm", &[(singleton_bind(), 4000)]);
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
            crate::microsandbox::plan::PortMapping::new(14000, 4000),
            crate::microsandbox::plan::PortMapping::new(14001, 4001),
        ];
        register_sandbox_lifecycle(
            &state_dir,
            "personal-litellm@canary",
            Some("personal"),
            "litellm",
            singleton_bind(),
            &[14000, 14001],
            &pairs,
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
        assert_eq!(record.created_at, "2026-07-20T14:05:42Z");
        assert_eq!(
            record.bind_ip,
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            "lifecycle register stores the given bind_ip"
        );
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
        assert!(record.created_at.is_empty());
        assert_eq!(
            record.bind_ip,
            IpAddr::V4(Ipv4Addr::LOCALHOST),
            "legacy record without bind_ip parses as 127.0.0.1 (ADR 0026(b))"
        );
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
            singleton_bind(),
            &[14000],
            &[crate::microsandbox::plan::PortMapping::new(14000, 4000)],
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
        assert!(
            check_port_collisions(&state_dir, "new-sandbox", &[(singleton_bind(), 3000)]).is_ok()
        );
        // … and still detects the collision from the VALID record.
        let err = check_port_collisions(&state_dir, "new-sandbox", &[(singleton_bind(), 4000)])
            .unwrap_err();
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
    // ---- FN-6: concurrent same-port race (TOCTOU closed) ----

    /// Two threads racing to register the SAME port through the atomic
    /// combined entry point (the function build_sandbox now uses): exactly
    /// one may win; the loser must error cleanly (port collision or lock
    /// timeout) and must not leave a record behind.
    #[test]
    fn concurrent_same_port_combined_register_one_wins() -> Result<()> {
        let state_dir = unique_state_dir("fn6-race");
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let mut handles = Vec::new();
        for i in 0..2u32 {
            let dir = state_dir.clone();
            let b = std::sync::Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                b.wait(); // release both threads at once
                combined_register(&dir, &format!("race-{i}"), "litellm", 4000)
            }));
        }
        let results: Vec<Result<()>> = handles
            .into_iter()
            .map(|h| h.join().expect("worker thread panicked"))
            .collect();
        let wins = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(
            wins, 1,
            "exactly one racer may win the port; results: {results:?}"
        );
        for r in &results {
            if let Err(e) = r {
                let msg = e.to_string();
                assert!(
                    msg.contains("port collision") || msg.contains("timed out acquiring"),
                    "loser must fail cleanly (collision or lock timeout), got: {msg}"
                );
            }
        }
        // Exactly one record holds the port, and it is the winner's.
        let records = list_records(&state_dir)?;
        let holders: Vec<_> = records.iter().filter(|r| r.ports.contains(&4000)).collect();
        assert_eq!(holders.len(), 1, "exactly one record may hold port 4000");
        // The lock file is gone afterwards (no wedge).
        assert!(
            !state_dir
                .join("var")
                .join("run")
                .join(super::super::lock::PORT_REGISTRY_LOCK_NAME)
                .exists(),
            "registry lock must be released after the race"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- ADR 0026: loopback allocator ----

    /// Build a record bound on `ip` (no filesystem — pure selection tests).
    fn record_on(ip: IpAddr) -> SandboxInstanceRecord {
        SandboxInstanceRecord {
            instance: format!("inst-{ip}"),
            context: None,
            workload: "w".to_string(),
            ports: vec![],
            port_pairs: vec![],
            created_at: String::new(),
            bind_ip: ip,
        }
    }

    fn loopback(n: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(127, 0, 0, n))
    }

    #[test]
    fn lowest_free_loopback_empty_registry_gives_dot_2() {
        assert_eq!(lowest_free_loopback(&[]), Some(Ipv4Addr::new(127, 0, 0, 2)));
    }

    #[test]
    fn lowest_free_loopback_skips_used_ips() {
        let records = vec![record_on(loopback(2))];
        assert_eq!(
            lowest_free_loopback(&records),
            Some(Ipv4Addr::new(127, 0, 0, 3))
        );
        let records = vec![record_on(loopback(2)), record_on(loopback(3))];
        assert_eq!(
            lowest_free_loopback(&records),
            Some(Ipv4Addr::new(127, 0, 0, 4))
        );
    }

    #[test]
    fn lowest_free_loopback_reuses_gaps() {
        // .2 and .4 used → .3 is the lowest free.
        let records = vec![record_on(loopback(2)), record_on(loopback(4))];
        assert_eq!(
            lowest_free_loopback(&records),
            Some(Ipv4Addr::new(127, 0, 0, 3))
        );
    }

    #[test]
    fn lowest_free_loopback_ignores_singleton_bind() {
        // Records bound on 127.0.0.1 (the shared singleton bind) do NOT
        // consume allocator space (N >= 2).
        let records = vec![record_on(IpAddr::V4(Ipv4Addr::LOCALHOST))];
        assert_eq!(
            lowest_free_loopback(&records),
            Some(Ipv4Addr::new(127, 0, 0, 2))
        );
    }

    #[test]
    fn lowest_free_loopback_none_when_exhausted() {
        let records: Vec<_> = (2..=254u8).map(|n| record_on(loopback(n))).collect();
        assert_eq!(lowest_free_loopback(&records), None);
    }

    #[test]
    fn allocate_then_unregister_then_allocate_reuses_ip() -> Result<()> {
        let state_dir = unique_state_dir("alloc-reuse");
        // Register a record bound on .2 via the lifecycle path.
        check_and_register_sandbox_lifecycle(
            &state_dir,
            "personal-litellm@a",
            None,
            "litellm",
            loopback(2),
            &[4000],
            &[crate::microsandbox::plan::PortMapping::new(4000, 4000)],
            "2026-07-30T00:00:00Z",
        )?;
        // Next allocation skips .2 → .3.
        assert_eq!(allocate_loopback_ip(&state_dir)?, loopback(3));
        // Unregister frees .2; the next allocation re-selects it.
        unregister_sandbox(&state_dir, "personal-litellm@a")?;
        assert_eq!(allocate_loopback_ip(&state_dir)?, loopback(2));
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn allocate_loopback_ip_releases_lock_file() -> Result<()> {
        let state_dir = unique_state_dir("alloc-lock");
        let _ = allocate_loopback_ip(&state_dir)?;
        assert!(
            !state_dir
                .join("var")
                .join("run")
                .join(PORT_REGISTRY_LOCK_NAME)
                .exists(),
            "lock file should be released after allocate_loopback_ip"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn singleton_bind_records_do_not_consume_allocator_ips() -> Result<()> {
        let state_dir = unique_state_dir("alloc-singleton");
        // Legacy/simple register_sandbox records bind 127.0.0.1 …
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        // … and so do lifecycle registrations on the shared bind.
        combined_register(&state_dir, "personal-pi", "pi", 3000)?;
        // Neither consumes the 127.0.0.N (N >= 2) allocator space.
        assert_eq!(allocate_loopback_ip(&state_dir)?, loopback(2));
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- ADR 0026(c)/C3: probe_free_ports (--port-auto) ----

    /// Assert every probed port is bindable on `bind` RIGHT NOW (the probe
    /// dropped its listeners) and distinct.
    ///
    /// FLAKE-GUARD (TOCTOU): `probe_free_ports` is deliberately NOT a
    /// reservation — the probed ports are released the moment the probe
    /// returns. Under `cargo test`'s parallel harness, another test thread
    /// (or an unrelated process) can claim a probed port between the probe
    /// and this assertion's bind. Retrying the whole probe+assert cycle a
    /// few times distinguishes that benign race from a real regression (a
    /// genuinely broken probe fails EVERY cycle, not intermittently).
    fn assert_probed_ports_bindable(state_dir: &Path, bind: IpAddr, expected: usize) {
        const MAX_CYCLES: usize = 8;
        for cycle in 1..=MAX_CYCLES {
            let probed =
                probe_free_ports(state_dir, bind, expected).expect("probe_free_ports must succeed");
            assert_eq!(probed.len(), expected, "probe must return {expected} ports");
            let distinct: std::collections::HashSet<u16> = probed.iter().copied().collect();
            assert_eq!(
                distinct.len(),
                probed.len(),
                "probed ports must be distinct: {probed:?}"
            );
            // Hold every successfully bound listener while attempting the
            // rest: the assertion is "all probed ports are SIMULTANEOUSLY
            // bindable right now".
            let mut held = Vec::with_capacity(probed.len());
            let mut conflict = None;
            for p in &probed {
                match std::net::TcpListener::bind((bind, *p)) {
                    Ok(listener) => held.push(listener),
                    Err(e) => {
                        conflict = Some((*p, e));
                        break;
                    }
                }
            }
            drop(held);
            if let Some((port, err)) = conflict {
                assert!(
                    cycle < MAX_CYCLES,
                    "probed port {port} on {bind} must be bindable after the probe \
                     (failed all {MAX_CYCLES} probe/assert cycles): {err:?}"
                );
                eprintln!(
                    "probe cycle {cycle}/{MAX_CYCLES}: probed port {port} on {bind} was \
                     claimed before the assertion bind ({err}); re-probing"
                );
            } else {
                return;
            }
        }
    }

    #[test]
    fn probe_free_ports_returns_distinct_bindable_ports_on_singleton_bind() -> Result<()> {
        let state_dir = unique_state_dir("probe-singleton");
        let bind = singleton_bind();
        assert_probed_ports_bindable(&state_dir, bind, 3);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn probe_free_ports_returns_distinct_bindable_ports_on_parallel_bind() -> Result<()> {
        let state_dir = unique_state_dir("probe-parallel");
        // Linux 127/8 is bindable per-address: probing on 127.0.0.2 works.
        let bind = loopback(2);
        assert_probed_ports_bindable(&state_dir, bind, 3);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn probe_free_ports_skips_ports_recorded_on_same_bind() -> Result<()> {
        let state_dir = unique_state_dir("probe-skip-same-bind");
        let bind = singleton_bind();
        // A first probe yields P; a record claims (bind, P); a re-probe must
        // never hand P out again on the same bind (defense-in-depth: the
        // record's sandbox may not hold the OS port yet).
        let p = probe_free_ports(&state_dir, bind, 1)?[0];
        register_instance_on(&state_dir, "personal-litellm", "litellm", bind, p)?;
        for _ in 0..8 {
            let probed = probe_free_ports(&state_dir, bind, 2)?;
            assert!(
                !probed.contains(&p),
                "probe must skip port {p} recorded on {bind}: {probed:?}"
            );
        }
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn probe_free_ports_ignores_ports_recorded_on_other_binds() -> Result<()> {
        let state_dir = unique_state_dir("probe-other-bind");
        // A record on (127.0.0.2, P) does NOT reserve P on 127.0.0.1 — the
        // collision model (and the probe) key on (bind, port) (ADR 0026(b)).
        // (Only the record side is exercised here: whether P itself is
        // re-probed on 127.0.0.1 is left to the OS; asserting it would be
        // flaky since P is ephemeral-free on 127.0.0.1 too.)
        let p = probe_free_ports(&state_dir, loopback(2), 1)?[0];
        register_instance_on(&state_dir, "personal-litellm@a", "litellm", loopback(2), p)?;
        assert_probed_ports_bindable(&state_dir, singleton_bind(), 2);
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn probe_free_ports_releases_lock_file() -> Result<()> {
        let state_dir = unique_state_dir("probe-lock-release");
        let _ = probe_free_ports(&state_dir, singleton_bind(), 1)?;
        assert!(
            !state_dir
                .join("var")
                .join("run")
                .join(PORT_REGISTRY_LOCK_NAME)
                .exists(),
            "lock file should be released after probe_free_ports"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- ADR 0026/C2: prospective_loopback_ip (read-only plan view) ----

    #[test]
    fn prospective_matches_allocate_on_same_registry_view() -> Result<()> {
        let state_dir = unique_state_dir("prospective-parity");
        // Empty registry: both entry points agree on 127.0.0.2.
        assert_eq!(prospective_loopback_ip(&state_dir)?, loopback(2));
        assert_eq!(
            prospective_loopback_ip(&state_dir)?,
            allocate_loopback_ip(&state_dir)?,
            "prospective and locked allocation must agree on the same view"
        );
        // Register a record on .2: both move to .3.
        check_and_register_sandbox_lifecycle(
            &state_dir,
            "personal-litellm@canary",
            None,
            "litellm",
            loopback(2),
            &[4000],
            &[crate::microsandbox::plan::PortMapping::new(4000, 4000)],
            "2026-07-30T00:00:00Z",
        )?;
        assert_eq!(prospective_loopback_ip(&state_dir)?, loopback(3));
        assert_eq!(
            prospective_loopback_ip(&state_dir)?,
            allocate_loopback_ip(&state_dir)?,
            "prospective and locked allocation must agree after a registration"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn prospective_is_read_only_and_leaves_no_lock_file() -> Result<()> {
        let state_dir = unique_state_dir("prospective-readonly");
        let before = list_records(&state_dir)?.len();
        let _ = prospective_loopback_ip(&state_dir)?;
        let after = list_records(&state_dir)?.len();
        assert_eq!(before, after, "prospective view must not register anything");
        assert!(
            !state_dir
                .join("var")
                .join("run")
                .join(PORT_REGISTRY_LOCK_NAME)
                .exists(),
            "prospective view must not leave a lock file behind"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    // ---- ADR 0026(b): collisions keyed on (bind_ip, port) ----

    #[test]
    fn same_port_on_different_bind_ips_does_not_collide() -> Result<()> {
        let state_dir = unique_state_dir("diff-bind-ok");
        // 127.0.0.1:4000 is taken (simple register binds the singleton IP).
        register_sandbox(
            &state_dir,
            "personal-litellm",
            Some("personal"),
            "litellm",
            &[4000],
        )?;
        // The same port on a DIFFERENT bind IP must not collide.
        let result = check_port_collisions(&state_dir, "work-litellm", &[(loopback(2), 4000)]);
        assert!(
            result.is_ok(),
            "same port on a different bind IP must not collide: {:?}",
            result.err()
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn same_bind_and_port_refuses_with_bind_in_message() -> Result<()> {
        let state_dir = unique_state_dir("same-bind-collide");
        // Register 127.0.0.2:4000 via the lifecycle path with that bind.
        check_and_register_sandbox_lifecycle(
            &state_dir,
            "personal-litellm@canary",
            Some("personal"),
            "litellm",
            loopback(2),
            &[4000],
            &[crate::microsandbox::plan::PortMapping::new(4000, 4000)],
            "2026-07-30T00:00:00Z",
        )?;
        // A second registration for the same (127.0.0.2, 4000) must refuse,
        // naming the bind:port.
        let err = check_port_collisions(&state_dir, "work-litellm@x", &[(loopback(2), 4000)])
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("port collision"),
            "expected a collision error; got: {msg}"
        );
        assert!(
            msg.contains("127.0.0.2:4000"),
            "error must name the colliding bind:port 127.0.0.2:4000; got: {msg}"
        );
        assert!(
            msg.contains("Remediation"),
            "error should mention remediation: {msg}"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }

    #[test]
    fn legacy_record_collides_on_singleton_bind_only() -> Result<()> {
        // A pre-C1 record file (no bind_ip) is treated as 127.0.0.1
        // (ADR 0026(b)): it collides with (127.0.0.1, port) but NOT with
        // (127.0.0.2, port).
        let state_dir = unique_state_dir("legacy-bind");
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
        // Same (127.0.0.1, 4000) → collision.
        let err = check_port_collisions(&state_dir, "new-sandbox", &[(singleton_bind(), 4000)])
            .unwrap_err();
        assert!(
            err.to_string().contains("127.0.0.1:4000"),
            "legacy record must collide on the singleton bind: {err}"
        );
        // Same port on 127.0.0.2 → no collision.
        assert!(
            check_port_collisions(&state_dir, "new-sandbox", &[(loopback(2), 4000)]).is_ok(),
            "legacy record must NOT collide on a different bind IP"
        );
        let _ = std::fs::remove_dir_all(&state_dir);
        Ok(())
    }
}
