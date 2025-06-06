# syntax=docker/dockerfile:1

# Leveraging the pre-built Docker images with 
# cargo-chef and the Rust toolchain
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . /app/
RUN cargo chef prepare --recipe-path recipe.json

# --- Build stage ---
FROM chef AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release

# --- Runtime stage ---
FROM debian:bullseye-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/canary/target/release/canary /app/canary
CMD ["/app/canary"]
