use anyhow::Result;
use dotenv::dotenv;
use polygon::ws::Connection;
use polygon_data_relay::relay::run;
use polygon_data_relay::settings::Settings;
use sentry_anyhow::capture_anyhow;
use tracing::{debug, info, subscriber::set_global_default};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    let _ = dotenv();
    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();
    set_global_default(subscriber).expect("Failed to set subscriber");
    let settings = Settings::new()?;
    let _guard = sentry::init((
        settings.sentry.address,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));
    info!("Starting");

    let mut data: Vec<&str> = vec![];

    if settings.polygon.quotes {
        data.push("Q");
        debug!(subscription = "Quotes", "Subscribing");
    }
    if settings.polygon.trades {
        data.push("T");
        debug!(subscription = "Trades", "Subscribing");
    }
    if settings.polygon.second_aggregates {
        data.push("A");
        debug!(subscription = "SecondAggregates", "Subscribing");
    }
    if settings.polygon.minute_aggregates {
        data.push("AM");
        debug!(subscription = "MinuteAggregates", "Subscribing");
    }

    let kafka_settings = settings.kafka;
    let polygon_settings = settings.polygon;

    let half_owned: Vec<_> = polygon_settings
        .tickers
        .iter()
        .map(|x| x.as_ref())
        .collect();
    let connection = Connection::new(
        &polygon_settings.base_url,
        &polygon_settings.key_id,
        &data,
        &half_owned,
    );
    let res = run(&kafka_settings, connection).await;
    if let Err(e) = res {
        capture_anyhow(&e);
    }

    Ok(())
}
