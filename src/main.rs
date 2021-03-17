use anyhow::{Context, Result};
use dotenv::dotenv;
use polygon::ws::Connection;
use polygon_data_relay::relay::run;
use polygon_data_relay::server::launch_server;
use std::env;
use std::sync::mpsc::channel;
use std::thread;
use tracing::{debug, info, subscriber::set_global_default};
use tracing_log::LogTracer;
use tracing_subscriber::{filter::EnvFilter, FmtSubscriber};

fn main() -> Result<()> {
    let _ = dotenv();
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    set_global_default(subscriber).expect("Failed to set subscriber");
    LogTracer::init().expect("Failed to set logger");
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
    let base_url = env::var("POLYGON_BASE_URL").context("Cound not find POLYGON_BASE_URL")?;
    let token = env::var("POLYGON_KEY").context("Could not find POLYGON_KEY")?;
    let connection = Connection::new(base_url, token, data, tickers);

    let (tx, rx) = channel();

    thread::spawn(|| {
        let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
        tokio_runtime.block_on(async {
            run(connection, rx).await.unwrap();
        });
    });
    let sys = actix_web::rt::System::new();
    sys.block_on(async move {
        launch_server(tx).unwrap().await.unwrap();
    });

    Ok(())
}
