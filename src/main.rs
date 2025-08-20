use std::time::Duration;

use anyhow::Result;

use metrics_exporter_prometheus::PrometheusBuilder;
use rumqttc::{Client, MqttOptions, QoS};
use structopt::StructOpt;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use sensor_monitor::{
    parse_topic_configs, Opts, DeviceContext, TopicDeviceMap,
    hem::{setup_device, setup_sensors},
    mqtt::handle_connection,
};

fn main() -> Result<()> {
    let opts = Opts::from_args();
    let level: Level = opts.log_level.clone().into();
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .json()
        .finish();

    tracing::subscriber::set_global_default(subscriber).unwrap();
    let _metrics_handler = PrometheusBuilder::new()
        .install()
        .expect("failed to install recorder/exporter");

    let http_client = reqwest::blocking::Client::new();

    // Parse topic configurations
    let topic_configs = parse_topic_configs(&opts)?;
    info!("Loaded {} topic configurations", topic_configs.len());

    // Setup devices for each topic
    let mut topic_device_map = TopicDeviceMap::new();
    for config in topic_configs {
        info!("Setting up device for topic: {}", config.topic);
        
        let device_id = setup_device(
            &http_client,
            &format!("{}/api/devices", opts.hemrs_base_url),
            &config.device_name,
            &config.device_location,
        )?;

        info!("Device ID for {}: {:?}", config.topic, device_id);

        let sensor_ids = setup_sensors(
            &http_client,
            &format!("{}/api/sensors", opts.hemrs_base_url),
        )?;

        info!("Sensor IDs for {}: {:?}", config.topic, sensor_ids);

        topic_device_map.insert(
            config.topic.clone(),
            DeviceContext {
                device_id,
                sensor_ids,
                topic: config.topic,
            }
        );
    }

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
    
    // Subscribe to all topics
    for topic in topic_device_map.keys() {
        info!("Subscribing to topic: {}", topic);
        client.subscribe(topic, QoS::AtMostOnce)?;
    }

    handle_connection(
        connection,
        &http_client,
        topic_device_map,
        &opts.hemrs_base_url,
    )?;
    Ok(())
}
