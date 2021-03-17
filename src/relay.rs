use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt};
use polygon::ws::{Connection, PolygonAction, PolygonMessage};
use rdkafka::{
    producer::{FutureProducer, FutureRecord},
    ClientConfig,
};
use std::env;
use std::sync::mpsc::Receiver;
use tracing::{debug, error, info};

pub async fn run(connection: Connection, rx: Receiver<PolygonAction>) -> Result<()> {
    let producer = kafka_producer()?;
    let ws = connection.connect().await.context("Failed to connect")?;
    let (mut sink, stream) = ws.split::<String>();
    tokio::spawn(async move {
        loop {
            let msg = rx.recv().expect("Failed to receive message");
            let msg_str = serde_json::to_string(&msg).expect("Failed to serialize command");
            info!("Control message received: {}", &msg_str);
            sink.send(msg_str)
                .await
                .map_err(|_| Err::<(), String>("Failed to send message to Sink".into()))
                .unwrap();
        }
    });

    stream
        .for_each(|message| async {
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
                            producer.send(FutureRecord::to(topic).key(key).payload(&payload), 0);
                        }
                        Err(_) => error!("Failed to serialize payload: {:?}", &message),
                    }
                }
                Err(e) => error!("Failed to receive message from the WebSocket: {}", e),
            }
        })
        .await;
    Ok(())
}

pub fn kafka_producer() -> Result<FutureProducer> {
    ClientConfig::new()
        .set("bootstrap.servers", &env::var("BOOTSTRAP_SERVERS")?)
        .set("security.protocol", "SASL_SSL")
        .set("sasl.mechanisms", "PLAIN")
        .set("sasl.username", &env::var("SASL_USERNAME")?)
        .set("sasl.password", &env::var("SASL_PASSWORD")?)
        // Don't resend any messages
        .set("message.send.max.retries", "0")
        // Send messages every 2ms as a group
        .set("queue.buffering.max.ms", "2")
        // Don't wait for acknowledgement from the server, just blast things
        .set("request.required.acks", "0")
        .set("enable.ssl.certificate.verification", "false")
        .create()
        .context("Failed to create Kafka producer")
}

fn get_topic(s: &PolygonMessage) -> &str {
    match s {
        PolygonMessage::Trade { .. } => "trades",
        PolygonMessage::Quote { .. } => "quotes",
        PolygonMessage::SecondAggregate { .. } => "second-aggregates",
        PolygonMessage::MinuteAggregate { .. } => "minute-aggregates",
        PolygonMessage::Status { .. } => "meta",
    }
}

fn get_key(s: &PolygonMessage) -> &str {
    match s {
        PolygonMessage::Trade { symbol, .. } => symbol,
        PolygonMessage::Quote { symbol, .. } => symbol,
        PolygonMessage::SecondAggregate { symbol, .. } => symbol,
        PolygonMessage::MinuteAggregate { symbol, .. } => symbol,
        PolygonMessage::Status { .. } => "status", // unkeyed on purpose to preserve ordering
    }
}
