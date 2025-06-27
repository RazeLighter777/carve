FROM ghcr.io/astral-sh/uv:debian

WORKDIR /app

RUN apt update && apt install -y \
    sshpass \
    && rm -rf /var/lib/apt/lists/*

ENV ANSIBLE_INVENTORY_PLUGINS=/app/inventory_plugin
ENV UV_COMPILE_BYTECODE=1


# Copy from the cache instead of linking since it's a mounted volume
ENV UV_LINK_MODE=copy

RUN --mount=type=bind,source=./carve-ansible/uv.lock,target=uv.lock \
    --mount=type=bind,source=./carve-ansible/pyproject.toml,target=pyproject.toml \
    uv sync --locked --no-install-project --no-dev


COPY ./carve-ansible/* .

COPY ./carve-ansible/inventory_plugin ./inventory_plugin

RUN --mount=type=cache,target=/root/.cache/uv \
    uv sync --locked --no-dev

ENTRYPOINT []

CMD [ "tail", "-f", "/dev/null" ]