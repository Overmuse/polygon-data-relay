use config::{Config, ConfigError, Environment, File};
use kafka_settings::KafkaSettings;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct WebServerSettings {
    pub address: String,
    pub port: usize,
}

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub server: WebServerSettings,
    pub kafka: KafkaSettings,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut s = Config::new();
        s.merge(File::with_name("config/local").required(false))?;
        s.merge(Environment::with_prefix("app"))?;
        s.try_into()
    }
}
