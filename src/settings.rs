use config::{Config, ConfigError, Environment};
use kafka_settings::KafkaSettings;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Deserialize)]
pub struct WebServerSettings {
    pub address: String,
    pub port: usize,
}

#[derive(Debug, Deserialize)]
pub struct PolygonSettings {
    pub base_url: String,
    pub key_id: String,
    #[serde(deserialize_with = "vec_from_str", default)]
    pub tickers: Vec<String>,
    #[serde(default)]
    pub trades: bool,
    #[serde(default)]
    pub quotes: bool,
    #[serde(default)]
    pub minute_aggregates: bool,
    #[serde(default)]
    pub second_aggregates: bool,
}

pub fn vec_from_str<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(s.split(',').map(From::from).collect())
}

#[derive(Debug, Deserialize)]
pub struct SentrySettings {
    pub address: String,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub server: WebServerSettings,
    pub kafka: KafkaSettings,
    pub kafka_paper: Option<KafkaSettings>,
    pub polygon: PolygonSettings,
    pub sentry: SentrySettings,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();
        s.merge(Environment::new().separator("__"))?;
        s.try_into()
    }
}
