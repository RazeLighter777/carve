FROM debian:bookworm-slim

# Install required packages
RUN apt-get update && apt-get install -y \
    qemu-system-x86 \
    qemu-utils \
    yq \
    python3 \
    curl \
    iptables \
    && rm -rf /var/lib/apt/lists/*

# set iptables alternative to legacy
RUN update-alternatives --set iptables /usr/sbin/iptables-legacy

# Create necessary directories
RUN mkdir -p /disk /config /cloud-init

# Copy entry script
COPY ./qemu-box/entry.sh /entry.sh
RUN chmod +x /entry.sh

# Expose VNC port and HTTP server port
EXPOSE 5900 8000

# Set the entrypoint
ENTRYPOINT ["./entry.sh"]