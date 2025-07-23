# syntax=docker/dockerfile:1.7-labs

# Leveraging the pre-built Docker images with 
# cargo-chef and the Rust toolchain
FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY --parents **/Cargo.toml **/Cargo.lock /app/
RUN cargo chef prepare --recipe-path recipe.json --bin canary

# --- Build stage ---
FROM chef AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY --from=planner /app/recipe.json recipe.json

# Build dependencies - this is the caching Docker layer!
COPY --parents **/Cargo.toml **/Cargo.lock /app/
RUN cargo chef cook --release --recipe-path recipe.json
COPY --parents **/carve/src/ **/canary/src/ /app/
RUN cargo build --release --bin canary

# --- Runtime stage ---
FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y libssl3 ca-certificates bind9-dnsutils inetutils-ping wget xz-utils sudo && rm -rf /var/lib/apt/lists/*
RUN adduser --system --group canary --home /app --shell /bin/bash
# add the user to the sudoers file
RUN echo "canary ALL=(ALL) NOPASSWD: ALL" > /etc/sudoers.d/canary
# set the user to be able to run sudo without a password
RUN chmod 0440 /etc/sudoers.d/canary
RUN sudo chown canary:canary /app
USER canary
RUN wget --output-document=/dev/stdout  https://nixos.org/nix/install | sh 
# add nix to the PATH
ENV PATH="/app/.nix-profile/bin:${PATH}"
COPY --from=builder /app/target/release/canary /app/canary
CMD ["/app/canary"]
