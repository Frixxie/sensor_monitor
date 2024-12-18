use std::{fmt::Display, time::Duration};

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

#[derive(StructOpt, Debug)]
pub struct Opts {
    #[structopt(short, long, env, default_value = "thor.lan")]
    pub mqtt_host: String,

    #[structopt(short, long, env, default_value = "tele/vinterhage/SENSOR")]
    pub topic: String,

    #[structopt(short, long, env, default_value = "http://desktop:65534")]
    pub hemrs_base_url: String,

    #[structopt(short, long, env, default_value = "esp32_stue")]
    pub device_name: String,

    #[structopt(short = "l", long, env, default_value = "Stue")]
    pub device_location: String,

    #[structopt(short, long, default_value = "info")]
    log_level: LogLevel,
}

impl Display for Opts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "mqtt_host: {}, topic: {}, hemrs_base_url: {}, device_name: {}, device_location: {}",
            self.mqtt_host, self.topic, self.hemrs_base_url, self.device_name, self.device_location
        )
    }
}

fn main() -> Result<()> {
    let opts = Opts::from_args();
    let level: Level = opts.log_level.into();
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .json()
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();
    let _metrics_handler = PrometheusBuilder::new()
        .install()
        .expect("failed to install recorder/exporter");

    let http_client = reqwest::blocking::Client::new();

    let device_id = setup_device(
        &http_client,
        &format!("{}/api/devices", opts.hemrs_base_url),
        &opts.device_name,
        &opts.device_location,
    )?;

    info!("{:?}", device_id);

    let sensor_ids = setup_sensors(
        &http_client,
        &format!("{}/api/sensors", opts.hemrs_base_url),
    )?;

    info!("{:?}", sensor_ids);

    let mut mqttoptions = MqttOptions::new(
        format!(
            "sensor_monitor_{}",
            gethostname::gethostname().to_str().unwrap()
        ),
        opts.mqtt_host,
        1883,
    );
    mqttoptions.set_keep_alive(Duration::from_secs(5));

    let (client, connection) = Client::new(mqttoptions, 10);
    client.subscribe(opts.topic, QoS::AtMostOnce)?;

    handle_connection(
        connection,
        &http_client,
        &device_id,
        &sensor_ids,
        &opts.hemrs_base_url,
    )?;
    Ok(())
}
