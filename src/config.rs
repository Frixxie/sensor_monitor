use std::{collections::HashMap, fmt::Display};

use anyhow::Result;
use serde::Deserialize;
use structopt::StructOpt;
use tracing::Level;

use crate::hem;

#[derive(Debug, Clone)]
pub enum LogLevel {
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

#[derive(Debug, Clone, Deserialize)]
pub struct TopicConfig {
    pub topic: String,
    pub device_name: String,
    pub device_location: String,
}

#[derive(Debug)]
pub struct DeviceContext {
    pub device_id: hem::DeviceId,
    pub sensor_ids: hem::SensorIds,
}

pub type TopicDeviceMap = HashMap<String, DeviceContext>;

#[derive(StructOpt, Debug)]
pub struct Opts {
    #[structopt(short, long, env, default_value = "thor")]
    pub mqtt_host: String,

    #[structopt(short, long, env, default_value = "config.toml")]
    pub config_file: String,

    #[structopt(short, long, env, default_value = "http://desktop:65534")]
    pub hemrs_base_url: String,

    #[structopt(short, long, default_value = "info")]
    pub log_level: LogLevel,
}

impl Display for Opts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "mqtt_host: {}, config_file: {}, hemrs_base_url: {}",
            self.mqtt_host, self.config_file, self.hemrs_base_url
        )
    }
}

pub fn parse_topic_configs(opts: &Opts) -> Result<Vec<TopicConfig>> {
    let content = std::fs::read_to_string(&opts.config_file)
        .map_err(|e| anyhow::anyhow!("Failed to read config file {}: {}", opts.config_file, e))?;
    
    #[derive(Deserialize)]
    struct TomlConfig {
        topics: Vec<TopicConfig>,
    }
    
    let config: TomlConfig = toml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse TOML file {}: {}", opts.config_file, e))?;
    
    Ok(config.topics)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::hem::{SensorIds, DeviceId};
    use tempfile::NamedTempFile;
    use std::fs;

    fn create_test_sensor_ids() -> SensorIds {
        SensorIds {
            ds18b20: 1,
            dht11_temperature: 2,
            dht11_humidity: 3,
            dht11_dew_point: 4,
        }
    }

    fn create_test_device_context(_topic: &str, device_id: DeviceId) -> DeviceContext {
        DeviceContext {
            device_id,
            sensor_ids: create_test_sensor_ids(),
        }
    }

    #[test]
    fn test_multi_topic_routing() {
        let mut topic_device_map: TopicDeviceMap = HashMap::new();
        
        // Setup multiple topics with different device contexts
        topic_device_map.insert(
            "kitchen/sensors".to_string(),
            create_test_device_context("kitchen/sensors", 1)
        );
        topic_device_map.insert(
            "bedroom/sensors".to_string(),
            create_test_device_context("bedroom/sensors", 2)
        );
        topic_device_map.insert(
            "garage/sensors".to_string(),
            create_test_device_context("garage/sensors", 3)
        );

        // Test that each topic maps to correct device
        assert_eq!(topic_device_map.get("kitchen/sensors").unwrap().device_id, 1);
        assert_eq!(topic_device_map.get("bedroom/sensors").unwrap().device_id, 2);
        assert_eq!(topic_device_map.get("garage/sensors").unwrap().device_id, 3);
        
        // Test that unknown topic returns None
        assert!(!topic_device_map.contains_key("unknown/topic"));
    }

    #[test]
    fn test_topic_config_integration() {
        let toml_content = r#"
[[topics]]
topic = "integration/topic1"
device_name = "integration_device1"
device_location = "Integration Location 1"

[[topics]]
topic = "integration/topic2"
device_name = "integration_device2"
device_location = "Integration Location 2"

[[topics]]
topic = "integration/topic3"
device_name = "integration_device3"
device_location = "Integration Location 3"
"#;
        
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), toml_content).unwrap();
        
        let opts = Opts {
            mqtt_host: "localhost".to_string(),
            config_file: temp_file.path().to_string_lossy().to_string(),
            hemrs_base_url: "http://localhost".to_string(),
            log_level: LogLevel::Info,
        };
        
        let configs = parse_topic_configs(&opts).unwrap();
        
        // Test that all topics are parsed correctly
        assert_eq!(configs.len(), 3);
        
        // Test first topic
        assert_eq!(configs[0].topic, "integration/topic1");
        assert_eq!(configs[0].device_name, "integration_device1");
        assert_eq!(configs[0].device_location, "Integration Location 1");
        
        // Test second topic
        assert_eq!(configs[1].topic, "integration/topic2");
        assert_eq!(configs[1].device_name, "integration_device2");
        assert_eq!(configs[1].device_location, "Integration Location 2");
        
        // Test third topic
        assert_eq!(configs[2].topic, "integration/topic3");
        assert_eq!(configs[2].device_name, "integration_device3");
        assert_eq!(configs[2].device_location, "Integration Location 3");
    }

    #[test]
    fn test_device_context_properties() {
        let context = create_test_device_context("test/topic", 42);
        
        assert_eq!(context.device_id, 42);
        assert_eq!(context.sensor_ids.ds18b20, 1);
        assert_eq!(context.sensor_ids.dht11_temperature, 2);
        assert_eq!(context.sensor_ids.dht11_humidity, 3);
        assert_eq!(context.sensor_ids.dht11_dew_point, 4);
    }

    #[test]
    fn test_large_topic_configuration() {
        // Test with a larger number of topics to ensure scalability
        let mut toml_content = String::new();
        
        for i in 1..=10 {
            toml_content.push_str(&format!(r#"
[[topics]]
topic = "house/room{}/SENSOR"
device_name = "esp32_room{}"
device_location = "Room {}"
"#, i, i, i));
        }
        
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), toml_content).unwrap();
        
        let opts = Opts {
            mqtt_host: "localhost".to_string(),
            config_file: temp_file.path().to_string_lossy().to_string(),
            hemrs_base_url: "http://localhost".to_string(),
            log_level: LogLevel::Info,
        };
        
        let configs = parse_topic_configs(&opts).unwrap();
        assert_eq!(configs.len(), 10);
        
        // Test first and last entries
        assert_eq!(configs[0].topic, "house/room1/SENSOR");
        assert_eq!(configs[0].device_name, "esp32_room1");
        assert_eq!(configs[0].device_location, "Room 1");
        
        assert_eq!(configs[9].topic, "house/room10/SENSOR");
        assert_eq!(configs[9].device_name, "esp32_room10");
        assert_eq!(configs[9].device_location, "Room 10");
    }

    #[test]
    fn test_error_handling_malformed_toml() {
        let malformed_toml = r#"
[topics  # Missing closing bracket
topic = "test/topic"
device_name = "test_device"
device_location = "Test Location"
"#;
        
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), malformed_toml).unwrap();
        
        let opts = Opts {
            mqtt_host: "localhost".to_string(),
            config_file: temp_file.path().to_string_lossy().to_string(),
            hemrs_base_url: "http://localhost".to_string(),
            log_level: LogLevel::Info,
        };
        
        let result = parse_topic_configs(&opts);
        assert!(result.is_err());
        let error_message = result.unwrap_err().to_string();
        assert!(error_message.contains("Failed to parse TOML"));
    }

    #[test]
    fn test_error_handling_missing_required_fields() {
        let incomplete_toml = r#"
[[topics]]
topic = "test/topic"
# Missing device_name and device_location
"#;
        
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), incomplete_toml).unwrap();
        
        let opts = Opts {
            mqtt_host: "localhost".to_string(),
            config_file: temp_file.path().to_string_lossy().to_string(),
            hemrs_base_url: "http://localhost".to_string(),
            log_level: LogLevel::Info,
        };
        
        let result = parse_topic_configs(&opts);
        assert!(result.is_err());
    }

    #[test]
    fn test_error_handling_empty_topic_name() {
        let empty_topic_toml = r#"
[[topics]]
topic = ""
device_name = "test_device"
device_location = "Test Location"
"#;
        
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), empty_topic_toml).unwrap();
        
        let opts = Opts {
            mqtt_host: "localhost".to_string(),
            config_file: temp_file.path().to_string_lossy().to_string(),
            hemrs_base_url: "http://localhost".to_string(),
            log_level: LogLevel::Info,
        };
        
        let result = parse_topic_configs(&opts);
        // This should parse successfully but result in empty topic string
        assert!(result.is_ok());
        let configs = result.unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].topic, "");
    }

    #[test]
    fn test_error_handling_duplicate_topics() {
        let duplicate_topics_toml = r#"
[[topics]]
topic = "test/duplicate"
device_name = "device1"
device_location = "Location 1"

[[topics]]
topic = "test/duplicate"
device_name = "device2"
device_location = "Location 2"
"#;
        
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), duplicate_topics_toml).unwrap();
        
        let opts = Opts {
            mqtt_host: "localhost".to_string(),
            config_file: temp_file.path().to_string_lossy().to_string(),
            hemrs_base_url: "http://localhost".to_string(),
            log_level: LogLevel::Info,
        };
        
        let result = parse_topic_configs(&opts);
        // This should parse successfully - the application should handle duplicate topics
        assert!(result.is_ok());
        let configs = result.unwrap();
        assert_eq!(configs.len(), 2);
        // Both should have same topic but different device info
        assert_eq!(configs[0].topic, "test/duplicate");
        assert_eq!(configs[1].topic, "test/duplicate");
        assert_ne!(configs[0].device_name, configs[1].device_name);
    }

    #[test]
    fn test_error_handling_unicode_in_config() {
        let unicode_toml = r#"
[[topics]]
topic = "test/émojis/🏠"
device_name = "esp32_café"
device_location = "Café ☕"
"#;
        
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), unicode_toml).unwrap();
        
        let opts = Opts {
            mqtt_host: "localhost".to_string(),
            config_file: temp_file.path().to_string_lossy().to_string(),
            hemrs_base_url: "http://localhost".to_string(),
            log_level: LogLevel::Info,
        };
        
        let result = parse_topic_configs(&opts);
        assert!(result.is_ok());
        let configs = result.unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].topic, "test/émojis/🏠");
        assert_eq!(configs[0].device_name, "esp32_café");
        assert_eq!(configs[0].device_location, "Café ☕");
    }

    #[test]
    fn test_error_handling_very_long_strings() {
        let long_string = "a".repeat(1000);
        let long_topic_toml = format!(r#"
[[topics]]
topic = "test/{}"
device_name = "device_{}"
device_location = "location_{}"
"#, long_string, long_string, long_string);
        
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), long_topic_toml).unwrap();
        
        let opts = Opts {
            mqtt_host: "localhost".to_string(),
            config_file: temp_file.path().to_string_lossy().to_string(),
            hemrs_base_url: "http://localhost".to_string(),
            log_level: LogLevel::Info,
        };
        
        let result = parse_topic_configs(&opts);
        assert!(result.is_ok());
        let configs = result.unwrap();
        assert_eq!(configs.len(), 1);
        assert!(configs[0].topic.len() > 1000);
    }

    #[test]
    fn test_error_handling_special_characters_in_paths() {
        let special_chars_toml = r#"
[[topics]]
topic = "test/path with spaces/and-dashes/and_underscores"
device_name = "device-with-dashes_and_underscores"
device_location = "Location with spaces & symbols!"
"#;
        
        let temp_file = NamedTempFile::new().unwrap();
        fs::write(temp_file.path(), special_chars_toml).unwrap();
        
        let opts = Opts {
            mqtt_host: "localhost".to_string(),
            config_file: temp_file.path().to_string_lossy().to_string(),
            hemrs_base_url: "http://localhost".to_string(),
            log_level: LogLevel::Info,
        };
        
        let result = parse_topic_configs(&opts);
        assert!(result.is_ok());
        let configs = result.unwrap();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].topic, "test/path with spaces/and-dashes/and_underscores");
        assert_eq!(configs[0].device_name, "device-with-dashes_and_underscores");
        assert_eq!(configs[0].device_location, "Location with spaces & symbols!");
    }
}