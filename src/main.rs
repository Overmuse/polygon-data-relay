use anyhow::{Context, Result};
use dotenv::dotenv;
use polygon::ws::Connection;
use polygon_data_relay::{
    run,
    telemetry::{get_subscriber, init_subscriber},
};
use std::env;
use tracing::{debug, info};

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv();
    let subscriber = get_subscriber("trader".into(), "info".into());
    init_subscriber(subscriber);
    info!("Starting polygon-data-relay");

    let tickers: Vec<String> = env::var("TICKERS")
        .context("Could not find TICKERS")?
        .split(',')
        .map(|x| x.to_string())
        .collect();
    let mut data: Vec<String> = vec![];

    if env::var("QUOTES").is_ok() {
        data.push("Q".to_string());
        debug!("Will subscribe to Quotes");
    }
    if env::var("TRADES").is_ok() {
        data.push("T".to_string());
        debug!("Will subscribe to Trades");
    }
    if env::var("SECOND_AGGREGATES").is_ok() {
        data.push("A".to_string());
        debug!("Will subscribe to SecondAggregates");
    }
    if env::var("MINUTE_AGGREGATES").is_ok() {
        data.push("AM".to_string());
        debug!("Will subscribe to MinuteAggregates");
    }
    let token = env::var("POLYGON_KEY").context("Could not find POLYGON_KEY")?;
    let ws = Connection::new(token, data, tickers)
        .connect()
        .await
        .context("Failed to conect to the WebSocket")?;

    run(ws).await;
    Ok(())
}
