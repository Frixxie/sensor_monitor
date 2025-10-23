use serde::Deserialize;

/// Configuration for the sensor monitor application.
/// Contains a list of devices, each with their own topic, name, and location.
#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    /// List of devices to monitor
    pub devices: Vec<DeviceCfg>,
}

/// Configuration for a single device.
/// Each device has a unique name/location pair and subscribes to exactly one MQTT topic.
#[derive(Debug, Deserialize, Clone)]
pub struct DeviceCfg {
    /// Device name (used for device registration in hemrs)
    pub name: String,
    /// Device location (used for device registration in hemrs)
    pub location: String,
    /// MQTT topic to subscribe to for this device's sensor data
    pub topic: String,
}

impl Config {
    /// Parses a TOML configuration string into a Config struct.
    ///
    /// # Arguments
    /// * `toml_str` - TOML configuration string
    ///
    /// # Returns
    /// * `Ok(Config)` - Successfully parsed configuration
    /// * `Err(anyhow::Error)` - Parse error with details
    ///
    /// # Example
    /// ```
    /// let toml_str = r#"
    /// [[devices]]
    /// name = "esp32_stue"
    /// location = "Stue"
    /// topic = "tele/stue/SENSOR"
    /// "#;
    /// let config = Config::from_str(toml_str).unwrap();
    /// ```
    pub fn from_str(toml_str: &str) -> anyhow::Result<Self> {
        let cfg: Self = toml::from_str(toml_str)?;
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_parsing_single_device() {
        let toml_str = r#"
[[devices]]
name = "esp32_stue"
location = "Stue"
topic = "tele/stue/SENSOR"
"#;
        let config = Config::from_str(toml_str).unwrap();
        assert_eq!(config.devices.len(), 1);
        assert_eq!(config.devices[0].name, "esp32_stue");
        assert_eq!(config.devices[0].location, "Stue");
        assert_eq!(config.devices[0].topic, "tele/stue/SENSOR");
    }

    #[test]
    fn test_config_parsing_multiple_devices() {
        let toml_str = r#"
[[devices]]
name = "esp32_stue"
location = "Stue"
topic = "tele/stue/SENSOR"

[[devices]]
name = "esp32_vinterhage"
location = "Vinterhage"
topic = "tele/vinterhage/SENSOR"
"#;
        let config = Config::from_str(toml_str).unwrap();
        assert_eq!(config.devices.len(), 2);
        assert_eq!(config.devices[1].name, "esp32_vinterhage");
        assert_eq!(config.devices[1].location, "Vinterhage");
        assert_eq!(config.devices[1].topic, "tele/vinterhage/SENSOR");
    }

    #[test]
    fn test_config_parsing_empty_devices() {
        let toml_str = r#"
devices = []
"#;
        let config = Config::from_str(toml_str).unwrap();
        assert_eq!(config.devices.len(), 0);
    }

    #[test]
    fn test_config_parsing_invalid_toml() {
        let toml_str = "invalid toml content";
        assert!(Config::from_str(toml_str).is_err());
    }

    #[test]
    fn test_config_parsing_missing_field() {
        let toml_str = r#"
[[devices]]
name = "esp32_stue"
location = "Stue"
"#;
        assert!(Config::from_str(toml_str).is_err());
    }
}
