FROM rust:latest as build-stage
WORKDIR /usr/src/app
COPY . .
RUN cargo install --path .

FROM rust:slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build-stage /usr/local/cargo/bin/sensor_monitor /usr/local/bin/sensor_monitor

# Create a working directory for the application
WORKDIR /app

# Copy example config file (can be overridden by volume mount)
COPY config.example.toml /app/config.toml

CMD ["sensor_monitor"]
