use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

#[derive(Debug)]
pub struct SensorIds {
    pub ds18b20: i32,
    pub dht11_temperature: i32,
    pub dht11_humidity: i32,
    pub dht11_dew_point: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Sensor {
    #[serde(skip_serializing)]
    id: i32,
    name: String,
    unit: String,
}

pub type DeviceId = i32;

#[derive(Serialize, Deserialize, Debug)]
pub struct Device {
    #[serde(skip_serializing)]
    id: i32,
    name: String,
    location: String,
}

pub fn fetch_devices(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<Device>> {
    let devices = client.get(url).send()?.json::<Vec<Device>>()?;
    Ok(devices)
}

pub fn fetch_sensors(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<Sensor>> {
    let devices = client.get(url).send()?.json::<Vec<Sensor>>()?;
    Ok(devices)
}

fn setup_sensor(
    client: &reqwest::blocking::Client,
    url: &str,
    sensor_name: &str,
    sensor_unit: &str,
) -> Result<i32> {
    let sensors = fetch_sensors(client, url)?;
    let device = sensors.iter().find(|d| d.name == sensor_name);
    match device {
        Some(d) => {
            info!("{:?}", d);
            Ok(d.id)
        }
        None => {
            let new_device = Sensor {
                id: 0,
                name: sensor_name.to_string(),
                unit: sensor_unit.to_string(),
            };
            let response = client.post(url).json(&new_device).send()?;
            info!("{:?}", response);
            setup_sensor(client, url, sensor_name, sensor_unit)
        }
    }
}

pub fn setup_sensors(client: &reqwest::blocking::Client, url: &str) -> Result<SensorIds> {
    let ds18b20 = setup_sensor(client, url, "DS18B20", "°C")?;
    let dht11_temperature = setup_sensor(client, url, "DHT11 Temperature", "°C")?;
    let dht11_humidity = setup_sensor(client, url, "DHT11 Humidity", "%")?;
    let dht11_dew_point = setup_sensor(client, url, "DHT11 Dew Point", "°C")?;

    Ok(SensorIds {
        ds18b20,
        dht11_temperature,
        dht11_humidity,
        dht11_dew_point,
    })
}

pub fn setup_device(
    client: &reqwest::blocking::Client,
    url: &str,
    device_name: &str,
    device_location: &str,
) -> Result<DeviceId> {
    let devices = fetch_devices(client, url)?;
    let device = devices
        .iter()
        .find(|d| d.name == device_name && d.location == device_location);
    match device {
        Some(d) => {
            info!("{:?}", d);
            Ok(d.id)
        }
        None => {
            let new_device = Device {
                id: 0,
                name: device_name.to_string(),
                location: device_location.to_string(),
            };
            let response = client.post(url).json(&new_device).send()?;
            info!("{:?}", response);
            setup_device(client, url, device_name, device_location)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensor_creation() {
        let sensor = Sensor {
            id: 1,
            name: "Test Sensor".to_string(),
            unit: "°C".to_string(),
        };
        
        assert_eq!(sensor.id, 1);
        assert_eq!(sensor.name, "Test Sensor");
        assert_eq!(sensor.unit, "°C");
    }

    #[test]
    fn test_device_creation() {
        let device = Device {
            id: 42,
            name: "ESP32".to_string(),
            location: "Kitchen".to_string(),
        };
        
        assert_eq!(device.id, 42);
        assert_eq!(device.name, "ESP32");
        assert_eq!(device.location, "Kitchen");
    }

    #[test]
    fn test_sensor_ids_creation() {
        let sensor_ids = SensorIds {
            ds18b20: 1,
            dht11_temperature: 2,
            dht11_humidity: 3,
            dht11_dew_point: 4,
        };
        
        assert_eq!(sensor_ids.ds18b20, 1);
        assert_eq!(sensor_ids.dht11_temperature, 2);
        assert_eq!(sensor_ids.dht11_humidity, 3);
        assert_eq!(sensor_ids.dht11_dew_point, 4);
    }

    #[test]
    fn test_sensor_serialization() {
        let sensor = Sensor {
            id: 1,
            name: "Test Sensor".to_string(),
            unit: "°C".to_string(),
        };
        
        let json = serde_json::to_string(&sensor).unwrap();
        assert!(json.contains("Test Sensor"));
        assert!(json.contains("°C"));
        // id should be skipped during serialization
        assert!(!json.contains("\"id\""));
    }

    #[test]
    fn test_device_serialization() {
        let device = Device {
            id: 42,
            name: "ESP32".to_string(),
            location: "Kitchen".to_string(),
        };
        
        let json = serde_json::to_string(&device).unwrap();
        assert!(json.contains("ESP32"));
        assert!(json.contains("Kitchen"));
        // id should be skipped during serialization
        assert!(!json.contains("\"id\""));
    }

    #[test]
    fn test_sensor_deserialization() {
        let json = r#"{"id": 1, "name": "Test Sensor", "unit": "°C"}"#;
        let sensor: Sensor = serde_json::from_str(json).unwrap();
        
        assert_eq!(sensor.id, 1);
        assert_eq!(sensor.name, "Test Sensor");
        assert_eq!(sensor.unit, "°C");
    }

    #[test]
    fn test_device_deserialization() {
        let json = r#"{"id": 42, "name": "ESP32", "location": "Kitchen"}"#;
        let device: Device = serde_json::from_str(json).unwrap();
        
        assert_eq!(device.id, 42);
        assert_eq!(device.name, "ESP32");
        assert_eq!(device.location, "Kitchen");
    }

    // Note: The fetch_devices, fetch_sensors, setup_sensor, setup_sensors, and setup_device
    // functions require HTTP mocking to test properly. In a full test suite, we would use
    // mockito or similar to mock HTTP responses.
}
