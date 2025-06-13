FROM debian:bookworm-slim
# Install required packages
RUN apt-get update && apt-get install -y \
    dnsmasq \
    yq \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY ./carve-dnsmasq/entry.sh /entry.sh
RUN chmod +x /entry.sh

ENTRYPOINT ["/entry.sh"]