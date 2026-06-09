//! Agent definitions. Deferred: the POC has no per-agent manifests; the
//! plan calls for an `agents/` directory populated with TOML/YAML
//! per-agent definitions (name, role, allowed models, key references).

pub fn list() -> Result<u8, String> {
    println!("agentctl agents list: deferred");
    println!("  the agents/ directory is created by `agentctl init` and currently empty");
    Ok(0)
}
