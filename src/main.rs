use anyhow::Result;
use dotenv::dotenv;
use polygon::ws::Connection;
use polygon_data_relay::relay::run;
use polygon_data_relay::server::launch_server;
use polygon_data_relay::settings::Settings;
use sentry_anyhow::capture_anyhow;
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
    let settings = Settings::new()?;
    let _guard = sentry::init((
        settings.sentry.address,
        sentry::ClientOptions {
            release: sentry::release_name!(),
            ..Default::default()
        },
    ));
    info!("Starting polygon-data-relay");

    let mut data: Vec<&str> = vec![];

    if settings.polygon.quotes {
        data.push("Q");
        debug!("Will subscribe to Quotes");
    }
    if settings.polygon.trades {
        data.push("T");
        debug!("Will subscribe to Trades");
    }
    if settings.polygon.second_aggregates {
        data.push("A");
        debug!("Will subscribe to SecondAggregates");
    }
    if settings.polygon.minute_aggregates {
        data.push("AM");
        debug!("Will subscribe to MinuteAggregates");
    }

    let (tx, rx) = channel();

    let kafka_settings = settings.kafka;
    let server_settings = settings.server;
    let polygon_settings = settings.polygon;

    thread::spawn(move || {
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
        let tokio_runtime = tokio::runtime::Runtime::new().unwrap();
        tokio_runtime.block_on(async {
            let res = run(&kafka_settings, connection, rx).await;
            if let Err(e) = res {
                capture_anyhow(&e);
            }
        });
    });
    let sys = actix_web::rt::System::new();
    sys.block_on(async move {
        let res = launch_server(&server_settings, tx)
            .unwrap()
            .await
            .map_err(From::from);
        if let Err(e) = res {
            capture_anyhow(&e);
        }
    });

    Ok(())
}
