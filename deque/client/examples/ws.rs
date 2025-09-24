use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    deque_client::ws::subscribe_program_and_send().await?;

    Ok(())
}
