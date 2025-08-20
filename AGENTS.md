# AGENTS.md

## Build/Test Commands
- `just build` - Check and build the project (cargo check + cargo build)
- `just test` - Run all tests (includes build first)
- `cargo test` - Run tests directly
- `cargo test <test_name>` - Run a specific test
- `cargo check` - Quick compilation check

## Code Style Guidelines

### Project Structure
- Rust 2021 edition project with main.rs, hem.rs, mqtt.rs modules
- Use `mod` declarations in main.rs for module organization

### Imports
- Standard library imports first, then external crates, then local modules
- Group by: std, external crates (anyhow, serde, etc.), local crate modules
- Use full paths for clarity: `use crate::hem::{DeviceId, SensorIds}`

### Types & Naming
- Use PascalCase for structs/enums: `SensorEntry`, `LogLevel`
- snake_case for functions/variables: `setup_device`, `device_id`
- Type aliases for clarity: `pub type DeviceId = i32`
- Descriptive field names with units in comments when relevant

### Error Handling
- Use `anyhow::Result<T>` for all fallible functions
- Pattern match on `Option` and `Result` types explicitly
- Log warnings for non-fatal errors using `tracing::warn!`
- Use `?` operator for error propagation