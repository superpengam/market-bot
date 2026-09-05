mod telemetry;

use std::error::Error;

use axum::serve;
use market_bot_api::{app::build_app, config::AppConfig};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    telemetry::init();

    let config = AppConfig::from_env()?;
    let listener = TcpListener::bind(config.bind_address).await?;

    tracing::info!(address = %config.bind_address, "market bot API listening");
    serve(listener, build_app()).await?;

    Ok(())
}
