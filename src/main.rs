use anyhow::{Context, Result};
use dotenv::dotenv;
use polygon::ws::Connection;
use polygon_data_relay::run;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let _guard = sentry::init(sentry::ClientOptions::new());
    dotenv()?;
    env_logger::builder().format_timestamp_micros().init();

    let tickers: Vec<String> = env::var("TICKERS")?
        .split(',')
        .map(|x| x.to_string())
        .collect();
    let mut data: Vec<String> = vec![];

    if env::var("QUOTES").is_ok() {
        data.push("Q".to_string());
    }
    if env::var("TRADES").is_ok() {
        data.push("T".to_string());
    }
    if env::var("SECOND_AGGREGATES").is_ok() {
        data.push("A".to_string());
    }
    if env::var("MINUTE_AGGREGATES").is_ok() {
        data.push("AM".to_string());
    }
    let ws = Connection::new(
        env::var("POLYGON_KEY").context("Could not find POLYGON_KEY")?,
        data,
        tickers,
    )
    .connect()
    .await?;

    run(ws).await;
    Ok(())
}
