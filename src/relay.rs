use anyhow::{Context, Result};
use futures::StreamExt;
use kafka_settings::{producer, KafkaSettings};
use polygon::ws::{Aggregate, Connection, PolygonMessage, PolygonStatus, Quote, Trade};
use rdkafka::producer::FutureRecord;
use std::time::Duration;
use tracing::{debug, error};

pub async fn run(settings: &KafkaSettings, connection: Connection<'_>) -> Result<()> {
    let producer = producer(settings)?;
    let ws = connection.connect().await.context("Failed to connect")?;
    let (_, stream) = ws.split::<String>();
    stream
        .for_each_concurrent(
            10_000, // Equal to 1/10 the max buffer size in rdkafka
            |message| async {
                match message {
                    Ok(polygon_message) => {
                        if let PolygonMessage::Status { status, message } = &polygon_message {
                            if let PolygonStatus::MaxConnections | PolygonStatus::ForceDisconnect =
                                status
                            {
                                panic!("Disconnecting: {}", message)
                            }
                        }
                        let topic = get_topic(&polygon_message);
                        let key = get_key(&polygon_message);
                        let payload = serde_json::to_string(&polygon_message);
                        match payload {
                            Ok(payload) => {
                                debug!(%payload, %key, %topic);
                                let res = producer
                                    .send(
                                        FutureRecord::to(topic).key(key).payload(&payload),
                                        Duration::from_secs(0),
                                    )
                                    .await;
                                if let Err((e, _)) = res {
                                    let e = e.into();
                                    sentry_anyhow::capture_anyhow(&e);
                                    error!(%e, %payload)
                                }
                            }
                            Err(e) => {
                                let e = e.into();
                                sentry_anyhow::capture_anyhow(&e);
                                error!(%e)
                            }
                        }
                    }
                    Err(e) => match e {
                        polygon::errors::Error::Serde { .. } => {
                            let e = e.into();
                            sentry_anyhow::capture_anyhow(&e);
                            error!(%e)
                        }
                        _ => panic!("Failed to receive message from the WebSocket: {}", e),
                    },
                }
            },
        )
        .await;
    Ok(())
}

fn get_topic(s: &PolygonMessage) -> &str {
    match s {
        PolygonMessage::Trade { .. } => "trades",
        PolygonMessage::Quote { .. } => "quotes",
        PolygonMessage::Second { .. } => "second-aggregates",
        PolygonMessage::Minute { .. } => "minute-aggregates",
        PolygonMessage::Status { .. } => "meta",
    }
}

fn get_key(s: &PolygonMessage) -> &str {
    match s {
        PolygonMessage::Trade(Trade { symbol, .. }) => symbol,
        PolygonMessage::Quote(Quote { symbol, .. }) => symbol,
        PolygonMessage::Second(Aggregate { symbol, .. }) => symbol,
        PolygonMessage::Minute(Aggregate { symbol, .. }) => symbol,
        PolygonMessage::Status { .. } => "status", // unkeyed on purpose to preserve ordering
    }
}
