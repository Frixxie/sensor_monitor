use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Collection of sensor IDs for all supported sensor types.
/// Contains the hemrs database IDs for each sensor type.
#[derive(Debug)]
pub struct SensorIds {
    /// DS18B20 temperature sensor ID
    pub ds18b20: i32,
    /// DHT11 temperature sensor ID
    pub dht11_temperature: i32,
    /// DHT11 humidity sensor ID
    pub dht11_humidity: i32,
    /// DHT11 dew point sensor ID
    pub dht11_dew_point: i32,
}

/// Sensor definition for hemrs API communication.
/// Used for both fetching existing sensors and creating new ones.
#[derive(Serialize, Deserialize, Debug)]
pub struct Sensor {
    /// Sensor ID (not serialized when creating new sensors)
    #[serde(skip_serializing)]
    id: i32,
    /// Human-readable sensor name
    name: String,
    /// Measurement unit (e.g., "°C", "%")
    unit: String,
}

/// Type alias for device ID returned from hemrs API
pub type DeviceId = i32;

/// Device definition for hemrs API communication.
/// Used for both fetching existing devices and creating new ones.
#[derive(Serialize, Deserialize, Debug)]
pub struct Device {
    /// Device ID (not serialized when creating new devices)
    #[serde(skip_serializing)]
    id: i32,
    /// Device name
    name: String,
    /// Device location
    location: String,
}

/// Fetches all devices from the hemrs API.
///
/// # Arguments
/// * `client` - HTTP client for making requests
/// * `url` - Full URL to the devices API endpoint
///
/// # Returns
/// * `Ok(Vec<Device>)` - List of devices from API
/// * `Err(anyhow::Error)` - HTTP or JSON parsing error
pub fn fetch_devices(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<Device>> {
    let devices = client.get(url).send()?.json::<Vec<Device>>()?;
    Ok(devices)
}

/// Fetches all sensors from the hemrs API.
///
/// # Arguments
/// * `client` - HTTP client for making requests
/// * `url` - Full URL to the sensors API endpoint
///
/// # Returns
/// * `Ok(Vec<Sensor>)` - List of sensors from API
/// * `Err(anyhow::Error)` - HTTP or JSON parsing error
pub fn fetch_sensors(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<Sensor>> {
    let devices = client.get(url).send()?.json::<Vec<Sensor>>()?;
    Ok(devices)
}

/// Sets up a single sensor in hemrs, creating it if it doesn't exist.
/// This function will recursively call itself after creating a new sensor.
///
/// # Arguments
/// * `client` - HTTP client for making requests
/// * `url` - Full URL to the sensors API endpoint
/// * `sensor_name` - Name of the sensor to find or create
/// * `sensor_unit` - Unit of measurement for the sensor
///
/// # Returns
/// * `Ok(i32)` - Sensor ID from hemrs
/// * `Err(anyhow::Error)` - HTTP, JSON, or API error
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

/// Sets up all required sensors in hemrs for the supported sensor types.
/// Creates sensors if they don't exist and returns their IDs.
///
/// # Arguments
/// * `client` - HTTP client for making requests
/// * `url` - Full URL to the sensors API endpoint
///
/// # Returns
/// * `Ok(SensorIds)` - Collection of all sensor IDs
/// * `Err(anyhow::Error)` - HTTP, JSON, or API error
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

/// Sets up a device in hemrs, creating it if it doesn't exist.
/// This function will recursively call itself after creating a new device.
///
/// # Arguments
/// * `client` - HTTP client for making requests
/// * `url` - Full URL to the devices API endpoint
/// * `device_name` - Name of the device to find or create
/// * `device_location` - Location of the device to find or create
///
/// # Returns
/// * `Ok(DeviceId)` - Device ID from hemrs
/// * `Err(anyhow::Error)` - HTTP, JSON, or API error
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
        assert!(!json.contains("\"id\""));
    }

    #[test]
    fn test_device_serialization() {
        let device = Device {
            id: 1,
            name: "Test Device".to_string(),
            location: "Test Location".to_string(),
        };
        let json = serde_json::to_string(&device).unwrap();
        assert!(json.contains("Test Device"));
        assert!(json.contains("Test Location"));
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
        let json = r#"{"id": 1, "name": "Test Device", "location": "Test Location"}"#;
        let device: Device = serde_json::from_str(json).unwrap();
        assert_eq!(device.id, 1);
        assert_eq!(device.name, "Test Device");
        assert_eq!(device.location, "Test Location");
    }
}
