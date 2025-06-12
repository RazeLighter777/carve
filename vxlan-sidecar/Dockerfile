# syntax=docker/dockerfile:1.7-labs

# Leverage pre-built Docker images with cargo-chef and the Rust toolchain
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY --parents **/Cargo.toml **/Cargo.lock /app/
RUN cargo chef prepare --recipe-path recipe.json --bin vxlan-sidecar

# --- Build stage ---
FROM chef AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
COPY --parents **/Cargo.toml **/Cargo.lock /app/
RUN cargo chef cook --release --recipe-path recipe.json
COPY --parents **/carve/src/ **/vxlan-sidecar/src/ /app/
RUN cargo build --release --bin vxlan-sidecar

# --- Runtime stage ---
FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y libssl3 ca-certificates
RUN apt-get install -y iproute2 iputils-ping && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/vxlan-sidecar /app/vxlan-sidecar
CMD ["/app/vxlan-sidecar"]
