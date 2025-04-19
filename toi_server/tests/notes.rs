#[tokio::test]
async fn route() -> Result<(), Box<dyn std::error::Error>> {
    let (binding_addr, mut state) = toi_server::init().await?;
    Ok(())
}
