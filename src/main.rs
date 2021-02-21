use anyhow::{Context, Result};
use clap::{App, Arg};
use dotenv::dotenv;
use polygon_data_relay::{stream_to_kafka, Polygon};
use sentry_anyhow::capture_anyhow;
use std::env;

fn main() -> Result<()> {
    let _guard = sentry::init(sentry::ClientOptions::new());
    dotenv()?;
    env_logger::builder().format_timestamp_micros().init();
    let matches = App::new("Market Data Relay")
        .version(option_env!("CARGO_PKG_VERSION").unwrap_or(""))
        .about("Data relay from various sources to Kafka")
        .arg(
            Arg::with_name("tickers")
                .short("t")
                .long("tickers")
                .required(true)
                .min_values(1),
        )
        .arg(Arg::with_name("quotes").short("Q"))
        .arg(Arg::with_name("trades").short("T"))
        .arg(Arg::with_name("second_aggregates").short("S"))
        .arg(Arg::with_name("minute_aggregates").short("M"))
        .get_matches();

    let tickers: Vec<String> = matches
        .values_of("tickers")
        .unwrap()
        .map(|x| x.to_string())
        .collect();
    let mut data: Vec<String> = vec![];
    if matches.is_present("quotes") {
        data.push("Q".to_string());
    }
    if matches.is_present("trades") {
        data.push("T".to_string());
    }
    if matches.is_present("second_aggregates") {
        data.push("A".to_string());
    }
    if matches.is_present("minute_aggregates") {
        data.push("AM".to_string());
    }
    let polygon = Polygon::new(
        env::var("POLYGON_KEY").context("Could not find POLYGON_KEY")?,
        data,
        tickers,
    );

    if let Err(e) = stream_to_kafka(polygon) {
        capture_anyhow(&e);
        return Err(e);
    }
    Ok(())
}
