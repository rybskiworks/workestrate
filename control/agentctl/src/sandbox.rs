//! Sandbox driver. Deferred in the POC; the production form uses
//! `microsandbox` (`msb run`) to wrap the LiteLLM process in a microVM
//! with a deny-all network policy and an explicit allow for the
//! provider host.

pub fn plan() -> Result<u8, String> {
    println!("agentctl sandbox plan: deferred (POC uses a host-side egress proxy)");
    println!("  the sandbox plan is produced by `agentctl litellm print-plan`");
    Ok(0)
}
