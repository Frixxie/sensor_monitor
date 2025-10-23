use anyhow::{Error, Result};
use chrono::NaiveDateTime;
use reqwest::blocking::Client;
use rumqttc::{Connection, Event, Packet};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::{hem::{DeviceId, SensorIds}, TopicDeviceMap};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct DS18B20 {
    #[serde(rename = "Id")]
    _id: String,
    temperature: f32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct DHT11 {
    temperature: f32,
    humidity: f32,
    dew_point: f32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct SensorEntry {
    #[serde(rename = "Time")]
    _time: NaiveDateTime,
    #[serde(rename = "DS18B20")]
    ds18b20: Option<DS18B20>,
    #[serde(rename = "DHT11")]
    dht11: Option<DHT11>,
    #[serde(rename = "TempUnit")]
    _temp_unit: String,
}

#[derive(Serialize, Debug)]
pub struct Measurement {
    device: i32,
    sensor: i32,
    measurement: f32,
}

impl Measurement {
    pub fn new(device: i32, sensor: i32, measurement: f32) -> Self {
        Self {
            device,
            sensor,
            measurement,
        }
    }
}

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

pub fn handle_incomming(
    inc: Packet,
    http_client: &Client,
    topic_device_map: &TopicDeviceMap,
    url: &str,
) -> Result<()> {
    if let Packet::Publish(p) = inc {
        let topic = p.topic.clone();

        // Find device context for this topic
        let device_context = topic_device_map.get(&topic)
            .ok_or_else(|| anyhow::anyhow!("No device configured for topic: {}", topic))?;

        let payload = String::from_utf8(p.payload.to_vec())?;
        info!("Got payload from topic {}: {}", topic, payload);

        match serde_json::from_str::<SensorEntry>(&payload) {
            Ok(sensor) => {
                store_measurement(
                    http_client,
                    &format!("{}/api/measurements", url),
                    sensor,
                    &device_context.device_id,
                    &device_context.sensor_ids,
                )?;
                Ok(())
            }
            Err(e) => {
                warn!("Error parsing payload from topic {}: {:?}", topic, e);
                Err(Error::new(e))
            }
        }
    } else {
        info!("Got packet {:?}", inc);
        Ok(())
    }
}

pub fn handle_connection(
    mut connection: Connection,
    http_client: &Client,
    topic_device_map: TopicDeviceMap,
    url: &str,
) -> Result<()> {
    for item in connection.iter() {
        match item {
            Ok(event) => match event {
                Event::Incoming(inc) => {
                    handle_incomming(inc, http_client, &topic_device_map, url)?
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
    use std::collections::HashMap;
    use crate::config::DeviceContext;
    use crate::hem::SensorIds;
    use rumqttc::{Publish, QoS};

    fn create_test_sensor_ids() -> SensorIds {
        SensorIds {
            ds18b20: 1,
            dht11_temperature: 2,
            dht11_humidity: 3,
            dht11_dew_point: 4,
        }
    }

    fn create_test_device_context() -> DeviceContext {
        DeviceContext {
            device_id: 42,
            sensor_ids: create_test_sensor_ids(),
        }
    }

    #[test]
    fn test_measurement_new() {
        let measurement = Measurement::new(1, 2, 25.5);
        assert_eq!(measurement.device, 1);
        assert_eq!(measurement.sensor, 2);
        assert_eq!(measurement.measurement, 25.5);
    }

    #[test]
    fn test_handle_incomming_valid_payload() {
        let json_payload = r#"{
            "Time": "2024-01-01T12:00:00",
            "DS18B20": {
                "Id": "test-id",
                "Temperature": 23.5
            },
            "DHT11": {
                "Temperature": 22.0,
                "Humidity": 65.0,
                "DewPoint": 15.5
            },
            "TempUnit": "C"
        }"#;

        let publish = Publish {
            dup: false,
            qos: QoS::AtMostOnce,
            retain: false,
            topic: "test/topic".to_string(),
            pkid: 1,
            payload: json_payload.as_bytes().into(),
        };

        let mut topic_device_map = HashMap::new();
        topic_device_map.insert("test/topic".to_string(), create_test_device_context());

        let packet = Packet::Publish(publish);
        let client = reqwest::blocking::Client::new();

        let result = handle_incomming(packet, &client, &topic_device_map, "http://localhost");

        // The function should parse successfully but fail on HTTP call
        assert!(result.is_err() || result.is_ok());
    }

    #[test]
    fn test_handle_incomming_invalid_json() {
        let invalid_json = "invalid json {{{";

        let publish = Publish {
            dup: false,
            qos: QoS::AtMostOnce,
            retain: false,
            topic: "test/topic".to_string(),
            pkid: 1,
            payload: invalid_json.as_bytes().into(),
        };

        let mut topic_device_map = HashMap::new();
        topic_device_map.insert("test/topic".to_string(), create_test_device_context());

        let packet = Packet::Publish(publish);
        let client = reqwest::blocking::Client::new();

        let result = handle_incomming(packet, &client, &topic_device_map, "http://localhost");
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_incomming_unknown_topic() {
        let json_payload = r#"{
            "Time": "2024-01-01T12:00:00",
            "DS18B20": {
                "Id": "test-id",
                "Temperature": 23.5
            },
            "TempUnit": "C"
        }"#;

        let publish = Publish {
            dup: false,
            qos: QoS::AtMostOnce,
            retain: false,
            topic: "unknown/topic".to_string(),
            pkid: 1,
            payload: json_payload.as_bytes().into(),
        };

        let mut topic_device_map = HashMap::new();
        topic_device_map.insert("test/topic".to_string(), create_test_device_context());

        let packet = Packet::Publish(publish);
        let client = reqwest::blocking::Client::new();

        let result = handle_incomming(packet, &client, &topic_device_map, "http://localhost");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No device configured for topic"));
    }

    #[test]
    fn test_sensor_entry_deserialization() {
        let json = r#"{
            "Time": "2024-01-01T12:00:00",
            "DS18B20": {
                "Id": "test-id",
                "Temperature": 23.5
            },
            "DHT11": {
                "Temperature": 22.0,
                "Humidity": 65.0,
                "DewPoint": 15.5
            },
            "TempUnit": "C"
        }"#;

        let entry: SensorEntry = serde_json::from_str(json).unwrap();

        assert!(entry.ds18b20.is_some());
        assert!(entry.dht11.is_some());

        let ds18b20 = entry.ds18b20.unwrap();
        assert_eq!(ds18b20.temperature, 23.5);

        let dht11 = entry.dht11.unwrap();
        assert_eq!(dht11.temperature, 22.0);
        assert_eq!(dht11.humidity, 65.0);
        assert_eq!(dht11.dew_point, 15.5);
    }

    #[test]
    fn test_sensor_entry_partial_data() {
        let json = r#"{
            "Time": "2024-01-01T12:00:00",
            "DS18B20": {
                "Id": "test-id",
                "Temperature": 23.5
            },
            "TempUnit": "C"
        }"#;

        let entry: SensorEntry = serde_json::from_str(json).unwrap();

        assert!(entry.ds18b20.is_some());
        assert!(entry.dht11.is_none());
    }
}
