//! Minimal workestrate library example: build, merge, and validate a
//! layered configuration entirely in-process — no CLI, no sandbox runtime.
//!
//! This demonstrates the three-step pipeline that `workestrate plan` runs
//! internally:
//!
//!   1. Parse each layer from a TOML string ([`Layer::from_string`]).
//!   2. Merge layers in precedence order ([`merge_layers`]) — earlier layers
//!      are lower precedence; the returned provenance map records which layer
//!      set each final field value.
//!   3. Validate the merged config ([`validate_config`]) — checks schema
//!      version, recipe vocabulary, and network entitlements.
//!
//! Run with: `cargo run --example microsandbox_minimal`

use workestrate::config::validate_config;
use workestrate::merge::{merge_layers, Layer};

/// A base layer: declares a single `agent` workload with default-deny
/// networking and DNS egress.
const BASE_TOML: &str = r#"
schema_version = 1

[workloads.pi]
kind = "agent"
image = { recipe = "registry", ref = "node:24" }
command = []

[workloads.pi.network.defaults]
egress = "deny"

[[workloads.pi.policy.egress.allow.host]]
ports = [53]
protocols = ["tcp", "udp"]
"#;

/// A team override layer: bumps `cpus` to 2 and adds a GitHub egress recipe.
/// Note it does NOT re-declare `kind`, `image`, or `command` — the merge
/// engine only touches keys each layer explicitly declares.
const TEAM_TOML: &str = r#"
schema_version = 1

[workloads.pi]
cpus = 2

[[workloads.pi.policy.egress.allow.domain]]
domains = ["github.com", "api.github.com"]
port = 443
protocol = "tcp"
"#;

fn main() -> anyhow::Result<()> {
    // 1. Parse layers.
    let base = Layer::from_string("base", BASE_TOML)?;
    let team = Layer::from_string("team", TEAM_TOML)?;

    // 2. Merge (earlier = lower precedence).
    let (merged, provenance) = merge_layers(&[base, team])?;

    let pi = merged
        .workloads
        .get("pi")
        .ok_or_else(|| anyhow::anyhow!("workload 'pi' must survive the merge"))?;

    println!("workload kind  : {}", pi.kind);
    println!("cpus           : {:?}", pi.cpus);
    println!(
        "egress default : {:?}",
        pi.network.defaults.and_then(|d| d.egress)
    );
    println!("egress policy  : {:?}", pi.policy.egress);

    // The team layer set `cpus`; the base layer set the egress default.
    assert_eq!(
        provenance.get("workloads.pi.cpus").map(|s| s.as_str()),
        Some("team"),
        "cpus provenance should be the team layer"
    );
    assert_eq!(
        provenance
            .get("workloads.pi.network.defaults.egress")
            .map(|s| s.as_str()),
        Some("base"),
        "egress-default provenance should be the base layer"
    );

    // 3. Validate the merged config.
    validate_config(&merged)?;
    println!(
        "\nConfig validated successfully (schema_version = {}).",
        merged.schema_version
    );

    Ok(())
}
