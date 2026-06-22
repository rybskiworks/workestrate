use anyhow::Result;

pub async fn plan() -> Result<()> {
    let plan = crate::microsandbox::build_litellm_plan();
    println!("{}", plan);
    Ok(())
}

pub async fn up(background: bool) -> Result<()> {
    crate::microsandbox::up_litellm(background).await
}

pub async fn down() -> Result<()> {
    crate::microsandbox::down_litellm().await
}
