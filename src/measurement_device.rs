use serde::Deserialize;

use crate::hem::{DeviceId, SensorIds};

#[derive(Debug, Deserialize)]
pub struct Measurement {
    mqtt_topic: String,
    device_name: String,
    device_location: String,
}

#[derive(Debug, Deserialize)]
pub struct MeasurementDevice {
    mqtt_topic: String,
    device_id: DeviceId,
    sensor_ids: SensorIds,
}

#[cfg(test)]
mod tests {
    use crate::measurement_device::Measurement;


    #[test]
    fn should_read_config_from_toml() {
        let config = r#"
            [[Measurement]]
            mqtt_topic = "tele/vinterhage/SENSOR"
            device_name = "esp32_stue"
            device_location = "stue"
        "#;

        let measurement_config: Vec<Measurement> = toml::from_str(config).unwrap();
        assert_eq!(measurement_config.len(), 1);
    }
}
