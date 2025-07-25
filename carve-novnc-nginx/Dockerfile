# syntax=docker/dockerfile:1.7-labs

# Leveraging the pre-built Docker images with 
# cargo-chef and the Rust toolchain
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY --parents **/Cargo.toml **/Cargo.lock /app/
RUN cargo chef prepare --recipe-path recipe.json --bin carve-novnc-nginx

# --- Build stage ---
FROM chef AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer!
COPY --parents **/Cargo.toml **/Cargo.lock /app/
RUN cargo chef cook --release --recipe-path recipe.json
COPY --parents **/carve/src/ **/carve-novnc-nginx/src/ /app/
RUN cargo build --release --bin carve-novnc-nginx

# --- Runtime stage ---
FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y libssl3 ca-certificates bind9-dnsutils inetutils-ping nginx curl && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/carve-novnc-nginx /app/carve-novnc-nginx
RUN mkdir -p /etc/nginx
RUN rm -rf /etc/nginx/sites-enabled && mkdir -p /etc/nginx/sites-enabled
CMD ["/app/carve-novnc-nginx"]
