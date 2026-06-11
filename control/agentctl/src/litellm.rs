use anyhow::Result;

pub async fn plan() -> Result<()> {
    let plan = crate::microsandbox::build_litellm_plan();
    println!("{}", plan);
    Ok(())
}
