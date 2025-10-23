# AGENTS.md

- Build: `just build` (cargo check + cargo build)
- Test all: `just test` or `cargo test` (verbose via just)
- Test one: `cargo test <test_name>` (module::test supported)
- Quick check: `cargo check`
- Run: `cargo run [-- <args>]`
- Lint/format: `cargo fmt -- --check` and `cargo clippy -- -D warnings` (run locally; not enforced in CI)
- Docker: `just container` (latest) or `just container_tagged DOCKERTAG=<tag>`

Code style (Rust 2021):
- Imports: std first, then external crates, then local modules; group and alphabetize within groups; prefer explicit paths (e.g., `use crate::hem::{DeviceId, SensorIds}`)
- Modules: `main.rs` declares `mod hem; mod mqtt; mod config;`
- Types/naming: PascalCase for types/enums; snake_case for functions/vars; constants in SCREAMING_SNAKE_CASE; newtypes/type aliases for clarity (e.g., `pub type DeviceId = i32`)
- Errors: use `anyhow::Result<T>`; propagate with `?`; match `Option`/`Result` explicitly; log non-fatal issues with `tracing::warn!`; prefer context via `anyhow::Context` when adding details
- Logging: initialize tracing subscriber in main; use structured fields where helpful; avoid println!
- Serialization: derive via `serde` where applicable; keep config in TOML (see tests/*.toml)
- Testing: keep unit tests near code; integration tests use `tests/`; use `tokio-test` as needed
- CI/CD: Docker images published via `.github/workflows/*.yml`; multi-arch buildx used; ensure `GITHUB_TOKEN`/`PROJECT_NAME` are set for just targets
- Cursor/Copilot rules: none present (`.cursor/`, `.cursorrules`, `.github/copilot-instructions.md` not found)
