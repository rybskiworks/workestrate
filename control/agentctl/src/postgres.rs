//! Postgres-backed key/team storage. Deferred in the POC.

pub fn status() -> Result<u8, String> {
    println!("agentctl postgres status: deferred (the POC runs LiteLLM in in-memory mode)");
    println!("  see infra/litellm/config.yaml: database_url = \"\"");
    println!("  the production form uses a separate postgres_16 process and migration tooling");
    Ok(0)
}
