use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A running sandbox instance record stored in the port registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxInstanceRecord {
    pub instance: String,
    pub context: Option<String>,
    pub workload: String,
    pub ports: Vec<u16>,
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
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
}
