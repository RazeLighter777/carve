# syntax=docker/dockerfile:1.7-labs

# --- Build stage ---
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY --parents **/Cargo.toml **/Cargo.lock /app/
RUN cargo chef prepare --recipe-path recipe.json --bin qemu-box

FROM chef AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json
COPY --parents **/Cargo.toml **/Cargo.lock /app/
RUN cargo chef cook --release --recipe-path recipe.json
COPY --parents **/qemu-box/src/ **/carve/src/ /app/
RUN cargo build --release --bin qemu-box

# --- Runtime stage ---
FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y \
    qemu-system-x86 \
    qemu-utils \
    whois \
    yq \
    python3 \
    curl \
    iptables \
    cloud-utils \
    libssl3 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*
RUN update-alternatives --set iptables /usr/sbin/iptables-legacy
RUN mkdir -p /disk /cloud-init
COPY --from=builder /app/target/release/qemu-box /app/qemu-box
EXPOSE 5900 8000
ENTRYPOINT ["/app/qemu-box"]