# syntax=docker/dockerfile:1.7-labs

# Leverage pre-built Docker images with cargo-chef and the Rust toolchain
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY --parents **/Cargo.toml **/Cargo.lock /app/
RUN cargo chef prepare --recipe-path recipe.json --bin vtep

# --- Build stage ---
FROM chef AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
COPY --parents **/Cargo.toml **/Cargo.lock /app/
RUN cargo chef cook --release --recipe-path recipe.json
COPY --parents **/carve/src **/vtep/src/ /app/
RUN cargo build --release --bin vtep

# --- Runtime stage ---
FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y libssl3 ca-certificates
# install the tools for iptables and iproute2
RUN apt-get install -y iptables iproute2 && rm -rf /var/lib/apt/lists/*
# use update-alternatives to set iptables to use the legacy version
RUN update-alternatives --set iptables /usr/sbin/iptables-legacy
RUN update-alternatives --set ip6tables /usr/sbin/ip6tables-legacy
COPY --from=builder /app/target/release/vtep /app/vtep
CMD ["/app/vtep"]
