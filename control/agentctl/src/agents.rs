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

pub async fn up(name: AgentName, background: bool) -> Result<()> {
    match name {
        AgentName::Pi => crate::microsandbox::up_pi(background).await,
        AgentName::Odysseus => crate::microsandbox::up_odysseus(background).await,
    }
}

pub async fn down(name: AgentName) -> Result<()> {
    match name {
        AgentName::Pi => crate::microsandbox::down_pi().await,
        AgentName::Odysseus => crate::microsandbox::down_odysseus().await,
    }
}
