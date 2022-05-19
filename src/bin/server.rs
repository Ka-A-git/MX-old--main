use mx::Server;
use tracing::info;

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Starting server...");
    Server::run().await?;
    info!("Server shutdown");
    Ok(())
}
