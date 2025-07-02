FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    qemu-utils \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY ./qemu-nbd/entry.sh /app/entry.sh
RUN chmod +x /app/entry.sh

EXPOSE 10809

ENTRYPOINT ["/app/entry.sh"]
