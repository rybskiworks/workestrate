use anyhow::Result;
use clap::ValueEnum;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum AgentName {
    Pi,
    Odysseus,
}

pub async fn plan(name: AgentName) -> Result<()> {
    let plan = match name {
        AgentName::Pi => crate::microsandbox::build_pi_plan(),
        AgentName::Odysseus => crate::microsandbox::build_odysseus_plan(),
    };
    println!("{}", plan);
    Ok(())
}
