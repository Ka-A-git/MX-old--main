use mx::CLI;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    CLI::run().await;
    Ok(())
}
