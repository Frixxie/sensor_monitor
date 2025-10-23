use std::{fmt::Display, time::Duration, collections::HashMap};

use anyhow::Result;

use metrics_exporter_prometheus::PrometheusBuilder;
use rumqttc::{Client, MqttOptions, QoS};
use structopt::StructOpt;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use crate::{
    hem::{setup_device, setup_sensors},
    mqtt::handle_connection,
};

mod hem;
mod mqtt;
mod config;

/// Log level enumeration for tracing configuration.
/// Maps string values to tracing::Level for structured logging.
#[derive(Debug, Clone)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl std::str::FromStr for LogLevel {
    type Err = String;

    /// Parses a string into a LogLevel variant.
    ///
    /// # Arguments
    /// * `s` - String to parse (case sensitive)
    ///
    /// # Returns
    /// * `Ok(LogLevel)` - Successfully parsed log level
    /// * `Err(String)` - Invalid log level string
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err("unknown log level".to_string()),
        }
    }
}

impl From<LogLevel> for Level {
    /// Converts LogLevel to tracing::Level for use with tracing subscriber.
    ///
    /// # Arguments
    /// * `log_level` - LogLevel to convert
    ///
    /// # Returns
    /// * `Level` - Corresponding tracing::Level
    fn from(log_level: LogLevel) -> Self {
        match log_level {
            LogLevel::Trace => Level::TRACE,
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
        }
    }
}

/// Command-line options for the sensor monitor application.
/// Uses structopt for automatic CLI parsing with environment variable support.
#[derive(StructOpt, Debug)]
pub struct Opts {
    /// MQTT broker hostname or IP address
    #[structopt(short, long, env, default_value = "thor.lan")]
    pub mqtt_host: String,

    /// Base URL for hemrs API
    #[structopt(short, long, env, default_value = "http://desktop:65534")]
    pub hemrs_base_url: String,

    /// Path to TOML configuration file containing device definitions
    #[structopt(long = "config", env = "CONFIG", default_value = "config.toml")]
    pub config_path: String,

    /// Log level for tracing output
    #[structopt(short, long, default_value = "info")]
    log_level: LogLevel,
}

impl Display for Opts {
    /// Formats the options for display/logging purposes.
    ///
    /// # Arguments
    /// * `f` - Formatter
    ///
    /// # Returns
    /// * `std::fmt::Result` - Formatting result
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "mqtt_host: {}, hemrs_base_url: {}, config_path: {}",
            self.mqtt_host, self.hemrs_base_url, self.config_path
        )
    }
}

/// Main application entry point.
/// Sets up logging, loads configuration, initializes sensors and devices,
/// subscribes to MQTT topics, and runs the main event loop.
///
/// # Returns
/// * `Ok(())` - Application completed successfully
/// * `Err(anyhow::Error)` - Configuration, network, or runtime error
fn main() -> Result<()> {
    let opts = Opts::from_args();
    let level: Level = opts.log_level.into();
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .json()
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();
    PrometheusBuilder::new()
        .install()
        .expect("failed to install recorder/exporter");

    let http_client = reqwest::blocking::Client::new();

    // Load config
    let cfg_str = std::fs::read_to_string(&opts.config_path)?;
    let cfg = crate::config::Config::from_str(&cfg_str)?;

    // Setup sensors once
    let sensor_ids = setup_sensors(
        &http_client,
        &format!("{}/api/sensors", opts.hemrs_base_url),
    )?;
    info!("{:?}", sensor_ids);

    // Prepare MQTT
    let mut mqttoptions = MqttOptions::new(
        format!(
            "sensor_monitor_{}",
            gethostname::gethostname().to_str().unwrap()
        ),
        opts.mqtt_host,
        1883,
    );
    mqttoptions.set_keep_alive(Duration::from_secs(60));

    let (client, connection) = Client::new(mqttoptions, 10);

    // For each device in config: ensure device exists and subscribe to its topic
    let mut topic_to_device = HashMap::new();
    for dev in &cfg.devices {
        let device_id = setup_device(
            &http_client,
            &format!("{}/api/devices", opts.hemrs_base_url),
            &dev.name,
            &dev.location,
        )?;
        info!("device_id for {}@{}: {:?}", dev.name, dev.location, device_id);
        topic_to_device.insert(dev.topic.clone(), device_id);
        info!("subscribing topic {}", dev.topic);
        client.subscribe(dev.topic.clone(), QoS::AtMostOnce)?;
    }

    if topic_to_device.is_empty() {
        return Err(anyhow::anyhow!("No devices configured in config file"));
    }

    handle_connection(
        connection,
        &http_client,
        &topic_to_device,
        &sensor_ids,
        &opts.hemrs_base_url,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_log_level_from_str() {
        assert!(matches!(LogLevel::from_str("trace"), Ok(LogLevel::Trace)));
        assert!(matches!(LogLevel::from_str("debug"), Ok(LogLevel::Debug)));
        assert!(matches!(LogLevel::from_str("info"), Ok(LogLevel::Info)));
        assert!(matches!(LogLevel::from_str("warn"), Ok(LogLevel::Warn)));
        assert!(matches!(LogLevel::from_str("error"), Ok(LogLevel::Error)));
        assert!(LogLevel::from_str("invalid").is_err());
    }

    #[test]
    fn test_log_level_to_tracing_level() {
        assert_eq!(Level::from(LogLevel::Trace), Level::TRACE);
        assert_eq!(Level::from(LogLevel::Debug), Level::DEBUG);
        assert_eq!(Level::from(LogLevel::Info), Level::INFO);
        assert_eq!(Level::from(LogLevel::Warn), Level::WARN);
        assert_eq!(Level::from(LogLevel::Error), Level::ERROR);
    }

    #[test]
    fn test_opts_display() {
        let opts = Opts {
            mqtt_host: "test.host".to_string(),
            hemrs_base_url: "http://test.url".to_string(),
            config_path: "test.toml".to_string(),
            log_level: LogLevel::Info,
        };
        let display_str = format!("{}", opts);
        assert!(display_str.contains("test.host"));
        assert!(display_str.contains("http://test.url"));
        assert!(display_str.contains("test.toml"));
    }

    #[test]
    fn test_log_level_clone() {
        let level = LogLevel::Info;
        let cloned = level.clone();
        assert!(matches!(cloned, LogLevel::Info));
    }
}
