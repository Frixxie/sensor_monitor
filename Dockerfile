FROM rust:latest as build-stage
WORKDIR /usr/src/app
COPY . .
RUN cargo install --path .

FROM rust:slim
COPY --from=build-stage /usr/local/cargo/bin/sensor_monitor /usr/local/bin/sensor_monitor
CMD ["sensor_monitor"]
