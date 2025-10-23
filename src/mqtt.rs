use std::collections::HashMap;

use anyhow::{Error, Result};
use chrono::NaiveDateTime;
use reqwest::blocking::Client;
use rumqttc::{Connection, Event, Packet};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::hem::{DeviceId, SensorIds};

/// DS18B20 temperature sensor data from Tasmota firmware.
/// Uses PascalCase field names as received from Tasmota MQTT messages.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct DS18B20 {
    /// Sensor ID string (not used in processing)
    #[serde(rename = "Id")]
    _id: String,
    /// Temperature reading in Celsius
    temperature: f32,
}

/// DHT11 sensor data from Tasmota firmware.
/// Uses PascalCase field names as received from Tasmota MQTT messages.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct DHT11 {
    /// Temperature reading in Celsius
    temperature: f32,
    /// Relative humidity percentage
    humidity: f32,
    /// Calculated dew point in Celsius
    dew_point: f32,
}

/// Complete sensor data entry from Tasmota MQTT message.
/// Contains timestamp and optional sensor readings.
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct SensorEntry {
    /// Timestamp of the reading (not used in processing)
    #[serde(rename = "Time")]
    _time: NaiveDateTime,
    /// Optional DS18B20 sensor data
    #[serde(rename = "DS18B20")]
    ds18b20: Option<DS18B20>,
    /// Optional DHT11 sensor data
    #[serde(rename = "DHT11")]
    dht11: Option<DHT11>,
    /// Temperature unit string (not used in processing)
    #[serde(rename = "TempUnit")]
    _temp_unit: String,
}

/// Measurement data structure for hemrs API.
/// Represents a single sensor reading for a specific device and sensor.
#[derive(Serialize, Debug)]
pub struct Measurement {
    /// Device ID in hemrs
    device: i32,
    /// Sensor ID in hemrs
    sensor: i32,
    /// Measurement value
    measurement: f32,
}

impl Measurement {
    /// Creates a new measurement instance.
    ///
    /// # Arguments
    /// * `device` - Device ID from hemrs
    /// * `sensor` - Sensor ID from hemrs
    /// * `measurement` - Measurement value
    ///
    /// # Returns
    /// * `Measurement` - New measurement instance
    pub fn new(device: i32, sensor: i32, measurement: f32) -> Self {
        Self {
            device,
            sensor,
            measurement,
        }
    }
}

/// Stores sensor measurements to hemrs API.
/// Processes both DHT11 and DS18B20 sensor data from a sensor entry.
///
/// # Arguments
/// * `client` - HTTP client for making requests
/// * `url` - Base URL for hemrs API (measurements endpoint will be appended)
/// * `entry` - Parsed sensor data from MQTT message
/// * `device_id` - Device ID in hemrs
/// * `sensor_ids` - Collection of sensor IDs in hemrs
///
/// # Returns
/// * `Ok(())` - All measurements stored successfully
/// * `Err(anyhow::Error)` - HTTP or API error
pub fn store_measurement(
    client: &reqwest::blocking::Client,
    url: &str,
    entry: SensorEntry,
    device_id: &DeviceId,
    sensor_ids: &SensorIds,
) -> Result<()> {
    match entry.dht11 {
        Some(dht11) => {
            info!("Logging DHT11");
            let dht11_temperature =
                Measurement::new(*device_id, sensor_ids.dht11_temperature, dht11.temperature);
            let dht11_humidity =
                Measurement::new(*device_id, sensor_ids.dht11_humidity, dht11.humidity);
            let dht11_dew_point =
                Measurement::new(*device_id, sensor_ids.dht11_dew_point, dht11.dew_point);
            client.post(url).json(&dht11_temperature).send()?;
            client.post(url).json(&dht11_humidity).send()?;
            client.post(url).json(&dht11_dew_point).send()?;
        }
        None => {
            warn!("Unable to process DHT11");
        }
    }

    match entry.ds18b20 {
        Some(ds18b20) => {
            info!("Logging DS18B20");
            let ds18b20_entry =
                Measurement::new(*device_id, sensor_ids.ds18b20, ds18b20.temperature);
            client.post(url).json(&ds18b20_entry).send()?;
        }
        None => {
            warn!("Unable to process DS18B20");
        }
    }

    Ok(())
}

/// Handles incoming MQTT packets, specifically publish packets with sensor data.
/// Parses JSON payload and stores measurements to hemrs API.
///
/// # Arguments
/// * `inc` - Incoming MQTT packet
/// * `http_client` - HTTP client for API requests
/// * `topic_to_device` - HashMap mapping topic strings to device IDs
/// * `sensor_ids` - Collection of sensor IDs in hemrs
/// * `url` - Base URL for hemrs API
///
/// # Returns
/// * `Ok(())` - Packet processed successfully
/// * `Err(anyhow::Error)` - Parse error, HTTP error, or API error
pub fn handle_incomming(
    inc: Packet,
    http_client: &Client,
    topic_to_device: &HashMap<String, DeviceId>,
    sensor_ids: &SensorIds,
    url: &str,
) -> Result<()> {
    if let Packet::Publish(p) = inc {
        let topic = p.topic.clone();
        let payload = String::from_utf8(p.payload.to_vec())?;
        info!("Got payload from topic {}: {}", topic, payload);
        
        let device_id = topic_to_device.get(&topic)
            .ok_or_else(|| anyhow::anyhow!("No device found for topic: {}", topic))?;
        
        match serde_json::from_str::<SensorEntry>(&payload) {
            Ok(sensor) => {
                store_measurement(
                    http_client,
                    &format!("{}/api/measurements", url),
                    sensor,
                    device_id,
                    sensor_ids,
                )?;
                Ok(())
            }
            Err(e) => {
                warn!("Error = {:?}", e);
                Err(Error::new(e))
            }
        }
    } else {
        info!("Got packet {:?}", inc);
        Ok(())
    }
}

/// Main MQTT connection handler loop.
/// Processes incoming and outgoing MQTT events until connection closes or errors.
///
/// # Arguments
/// * `connection` - MQTT connection from rumqttc
/// * `http_client` - HTTP client for API requests
/// * `topic_to_device` - HashMap mapping topic strings to device IDs
/// * `sensor_ids` - Collection of sensor IDs in hemrs
/// * `url` - Base URL for hemrs API
///
/// # Returns
/// * `Ok(())` - Connection closed normally
/// * `Err(anyhow::Error)` - Fatal connection or processing error
pub fn handle_connection(
    mut connection: Connection,
    http_client: &Client,
    topic_to_device: &HashMap<String, DeviceId>,
    sensor_ids: &SensorIds,
    url: &str,
) -> Result<()> {
    for item in connection.iter() {
        match item {
            Ok(event) => match event {
                Event::Incoming(inc) => {
                    handle_incomming(inc, http_client, topic_to_device, sensor_ids, url)?
                }
                Event::Outgoing(out) => {
                    info!("Sending {:?}", out)
                }
            },
            Err(e) => {
                warn!("Error = {:?}", e);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_measurement_new() {
        let measurement = Measurement::new(1, 2, 25.5);
        assert_eq!(measurement.device, 1);
        assert_eq!(measurement.sensor, 2);
        assert_eq!(measurement.measurement, 25.5);
    }

    #[test]
    fn test_measurement_serialization() {
        let measurement = Measurement::new(1, 2, 25.5);
        let json = serde_json::to_string(&measurement).unwrap();
        assert!(json.contains("\"device\":1"));
        assert!(json.contains("\"sensor\":2"));
        assert!(json.contains("\"measurement\":25.5"));
    }

    #[test]
    fn test_ds18b20_deserialization() {
        let json = r#"{"Id": "test_id", "Temperature": 22.5}"#;
        let ds18b20: DS18B20 = serde_json::from_str(json).unwrap();
        assert_eq!(ds18b20._id, "test_id");
        assert_eq!(ds18b20.temperature, 22.5);
    }

    #[test]
    fn test_dht11_deserialization() {
        let json = r#"{"Temperature": 23.0, "Humidity": 45.0, "DewPoint": 10.5}"#;
        let dht11: DHT11 = serde_json::from_str(json).unwrap();
        assert_eq!(dht11.temperature, 23.0);
        assert_eq!(dht11.humidity, 45.0);
        assert_eq!(dht11.dew_point, 10.5);
    }

    #[test]
    fn test_sensor_entry_deserialization() {
        let json = r#"{
            "Time": "2023-01-01T12:00:00",
            "DS18B20": {"Id": "test", "Temperature": 22.0},
            "DHT11": {"Temperature": 23.0, "Humidity": 45.0, "DewPoint": 10.5},
            "TempUnit": "C"
        }"#;
        let entry: SensorEntry = serde_json::from_str(json).unwrap();
        assert!(entry.ds18b20.is_some());
        assert!(entry.dht11.is_some());
        assert_eq!(entry._temp_unit, "C");
    }

    #[test]
    fn test_sensor_entry_partial_deserialization() {
        let json = r#"{
            "Time": "2023-01-01T12:00:00",
            "DS18B20": {"Id": "test", "Temperature": 22.0},
            "TempUnit": "C"
        }"#;
        let entry: SensorEntry = serde_json::from_str(json).unwrap();
        assert!(entry.ds18b20.is_some());
        assert!(entry.dht11.is_none());
    }

    #[test]
    fn test_sensor_entry_invalid_json() {
        let json = r#"{"invalid": "json"}"#;
        assert!(serde_json::from_str::<SensorEntry>(json).is_err());
    }
}
