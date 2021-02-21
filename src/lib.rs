use anyhow::{Context, Result};
use log::{error, info, warn};
use polygon::ws::{Connection, PolygonMessage};
use rdkafka::{
    producer::{FutureProducer, FutureRecord},
    ClientConfig,
};
use serde::{Deserialize, Serialize};
use std::env;
use websocket::client::sync::Client;
use websocket::stream::sync::{TcpStream, TlsStream};
use websocket::url::Url;
use websocket::{ClientBuilder, OwnedMessage};

#[derive(Debug)]
pub struct SubscribeableMessage<T> {
    pub topic: String,
    pub key: String,
    pub data: T,
}

pub fn stream_to_kafka(mut service: Polygon) -> Result<()> {
    let producer = kafka_producer()?;

    service
        .connect()
        .context("Failed to connect to service Polygon")?;
    service
        .subscribe()
        .context("Failed to subscribe to service Polygon")?;
    loop {
        futures::executor::block_on(async {
            let messages = service.run();
            match messages {
                Ok(x) => {
                    for msg in x {
                        let payload = serde_json::to_string(&msg.data)?;
                        info!("Message received: {}", &payload);
                        producer.send(
                            FutureRecord::to(&msg.topic).key(&msg.key).payload(&payload),
                            0,
                        );
                    }
                    Ok(())
                }
                Err(e) => return Err(e),
            }
        })?;
    }
}

pub fn kafka_producer() -> Result<FutureProducer> {
    ClientConfig::new()
        .set("bootstrap.servers", &env::var("BOOTSTRAP_SERVERS")?)
        .set("security.protocol", &env::var("SECURITY_PROTOCOL")?)
        .set("sasl.mechanisms", &env::var("SASL_MECHANISMS")?)
        .set("sasl.username", &env::var("SASL_USERNAME")?)
        .set("sasl.password", &env::var("SASL_PASSWORD")?)
        .set("enable.ssl.certificate.verification", "false")
        .set("message.timeout.ms", "5000")
        .create()
        .context("Failed to create Kafka producer")
}

// TODO: Make PolygonAction public in Polygon and import here
#[derive(Serialize, Deserialize, Debug)]
struct PolygonAction {
    action: String,
    params: String,
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

pub struct Polygon {
    ws: Option<Client<TlsStream<TcpStream>>>,
    url: Url,
    auth_token: String,
    events: Vec<String>,
    assets: Vec<String>,
}

impl Polygon {
    pub fn new(auth_token: String, events: Vec<String>, assets: Vec<String>) -> Self {
        Self {
            ws: None,
            url: Url::parse("wss://alpaca.socket.polygon.io/stocks").unwrap(),
            auth_token,
            events,
            assets,
        }
    }

    pub fn bind_websocket(&mut self, ws: Client<TlsStream<TcpStream>>) {
        self.ws = Some(ws);
    }

    fn connect(&mut self) -> Result<()> {
        let client = ClientBuilder::from_url(&self.url)
            .connect_secure(None)
            .context("Failed to create Polygon client")?;
        self.bind_websocket(client);
        let auth_message = PolygonAction {
            action: "auth".to_string(),
            params: self.auth_token.clone(),
        };
        self.send_message(
            &serde_json::to_string(&auth_message).context("Failed to serialize auth messsage")?,
        )
        .context("Failed to send auth message")?;
        Ok(())
    }

    fn send_message(&mut self, msg: &str) -> Result<()> {
        if let Some(ws) = &mut self.ws {
            ws.send_message(&OwnedMessage::Text(msg.to_string()))
                .context("Failed to send websocket message")?;
        } else {
            error!("Tried to send a message before connecting to websocket!")
        }
        Ok(())
    }

    fn subscribe(&mut self) -> Result<()> {
        let subscriptions: Vec<_> = self
            .events
            .iter()
            .flat_map(|x| std::iter::repeat(x).zip(self.assets.iter()))
            .map(|(x, y)| format!("{}.{}", x, y))
            .collect();
        let subscription_message = PolygonAction {
            action: "subscribe".to_string(),
            params: subscriptions.join(","),
        };
        self.send_message(
            &serde_json::to_string(&subscription_message)
                .context("Failed to serialize subscription message")?,
        )
        .context("Failed to send subscription message")?;
        Ok(())
    }

    fn run(&mut self) -> Result<Vec<SubscribeableMessage<PolygonMessage>>> {
        loop {
            match self.ws.as_mut().unwrap().recv_message() {
                Ok(OwnedMessage::Text(s)) => match serde_json::from_str(&s) {
                    Ok(messages) => {
                        let messages: Vec<PolygonMessage> = messages;
                        let x: Vec<_> = messages
                            .into_iter()
                            .map(|m| {
                                let topic = get_topic(&m).to_string();
                                let key = get_key(&m).to_string();
                                SubscribeableMessage {
                                    topic,
                                    key,
                                    data: m,
                                }
                            })
                            .collect();
                        return Ok(x);
                    }
                    Err(e) => error!("Failed to deserialize message: {:?}. Error: {:?}", s, e),
                },
                Ok(_) => continue,
                Err(websocket::result::WebSocketError::NoDataAvailable) => {
                    warn!("Websocket was disconnected. Attempting to reconnect");
                    self.connect()?;
                    self.subscribe()?;
                    continue;
                }
                Err(e) => return Result::Err(anyhow::Error::new(e)),
            }
        }
    }
}
