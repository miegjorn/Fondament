FROM rust:latest AS builder
WORKDIR /build
COPY . .
RUN cargo build --release --bin fondament-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/fondament-server /usr/local/bin/fondament-server
COPY definitions /fondament/definitions
ENV FONDAMENT_DEFINITIONS_PATH=/fondament/definitions
ENV FONDAMENT_PORT=7800
EXPOSE 7800
CMD ["fondament-server"]
