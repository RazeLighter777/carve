# syntax=docker/dockerfile:1

# --- Build stage ---
FROM rust:slim-bullseye AS builder
WORKDIR /app
COPY . .
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
RUN cargo build --release

# --- Runtime stage ---
FROM debian:bullseye-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y libssl1.1 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/canary /app/canary
CMD ["/app/canary"]
