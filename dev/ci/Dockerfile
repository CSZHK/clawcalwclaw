# syntax=docker/dockerfile:1.7

FROM rust:1.92-slim@sha256:bf3368a992915f128293ac76917ab6e561e4dda883273c8f5c9f6f8ea37a378e
ARG CLAWCLAWCLAW_CARGO_REGISTRY_PROTOCOL=""
ARG CLAWCLAWCLAW_CARGO_REGISTRY_INDEX=""
ARG CLAWCLAWCLAW_RUSTUP_DIST_SERVER=""
ARG CLAWCLAWCLAW_RUSTUP_UPDATE_ROOT=""
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL="$CLAWCLAWCLAW_CARGO_REGISTRY_PROTOCOL"
ENV CARGO_REGISTRIES_CRATES_IO_INDEX="$CLAWCLAWCLAW_CARGO_REGISTRY_INDEX"
ENV RUSTUP_DIST_SERVER="$CLAWCLAWCLAW_RUSTUP_DIST_SERVER"
ENV RUSTUP_UPDATE_ROOT="$CLAWCLAWCLAW_RUSTUP_UPDATE_ROOT"

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    git \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN rustup toolchain install 1.92.0 --profile minimal --component rustfmt --component clippy

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo install --locked cargo-audit --version 0.22.1 && \
    cargo install --locked cargo-deny --version 0.18.5

WORKDIR /workspace

CMD ["bash"]
