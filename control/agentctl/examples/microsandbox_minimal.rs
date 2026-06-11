use microsandbox::{NetworkPolicy, Sandbox};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define a default-deny policy that allows egress TCP/443 to public IPs.
    let policy = NetworkPolicy::builder()
        .default_deny()
        .egress(|e| e.tcp().port(443).allow_public())
        .build()?;

    // Configure the sandbox (planning only).
    let _plan = Sandbox::builder("example-sandbox")
        .image("alpine:3.20")
        .cpus(1)
        .memory(512u32)
        .network(|n| n.policy(policy))
        .env("EXAMPLE", "1");

    // Runtime is blocked in this milestone: no .create().await
    println!("Sandbox plan constructed successfully (compile-check only).");
    Ok(())
}
