use anyhow::{anyhow, Context, Result};
use futures::{SinkExt, StreamExt};
use kafka_settings::{producer, KafkaSettings};
use polygon::ws::{Aggregate, Connection, PolygonAction, PolygonMessage, Quote, Trade};
use rdkafka::producer::FutureRecord;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use tracing::{debug, error, info};

pub async fn run(
    settings: &KafkaSettings,
    connection: Connection<'_>,
    rx: Receiver<PolygonAction>,
) -> Result<()> {
    let producer = producer(settings)?;
    let ws = connection.connect().await.context("Failed to connect")?;
    let (mut sink, stream) = ws.split::<String>();
    tokio::spawn(async move {
        loop {
            let msg = rx.recv().expect("Failed to receive message");
            let msg_str = serde_json::to_string(&msg).expect("Failed to serialize command");
            info!("Control message received: {}", &msg_str);
            sink.send(msg_str)
                .await
                .map_err(|_| anyhow!("Failed to send message to Sink"))
                .unwrap();
        }
    });

    stream
        .for_each_concurrent(
            100_000, // Equal to the max buffer size in rdkafka
            |message| async {
                match message {
                    Ok(message) => {
                        let topic = get_topic(&message);
                        let key = get_key(&message);
                        let payload = serde_json::to_string(&message);
                        match payload {
                            Ok(payload) => {
                                debug!(
                                    "Message received: {}. Assigning key: {}, sending to topic: {}",
                                    &payload, &key, &topic
                                );
                                let res = producer
                                    .send(
                                        FutureRecord::to(topic).key(key).payload(&payload),
                                        Duration::from_secs(0),
                                    )
                                    .await;
                                if let Err((e, msg)) = res {
                                    let e = e.into();
                                    sentry_anyhow::capture_anyhow(&e);
                                    error!(
                                        "Failed to send message to kafka. Message: {:?}\nError: {}",
                                        msg, e
                                    )
                                }
                            }
                            Err(e) => {
                                sentry_anyhow::capture_anyhow(&e.into());
                                error!("Failed to serialize payload: {:?}", &message)
                            }
                        }
                    }
                    Err(e) => {
                        let e = e.into();
                        sentry_anyhow::capture_anyhow(&e);
                        error!("Failed to receive message from the WebSocket: {}", e)
                    }
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
