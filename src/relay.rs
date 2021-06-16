use anyhow::{anyhow, Context, Result};
use futures::{SinkExt, StreamExt};
use kafka_settings::{producer, KafkaSettings};
use polygon::ws::{
    Aggregate, Connection, PolygonAction, PolygonMessage, PolygonStatus, Quote, Trade,
};
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
            info!(%msg_str);
            sink.send(msg_str)
                .await
                .map_err(|_| anyhow!("Failed to send message to Sink"))
                .unwrap();
        }
    });

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
