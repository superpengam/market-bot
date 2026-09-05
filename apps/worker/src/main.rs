mod telemetry;

use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    telemetry::init();
    tracing::info!("market bot worker started");

    tokio::signal::ctrl_c().await?;
    tracing::info!("market bot worker stopped");

    Ok(())
}
