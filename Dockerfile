# syntax=docker/dockerfile:1.7

# Convergio daemon image.
#
# Default bind stays localhost for safety. To expose from a container:
#   -e CONVERGIO_BIND=0.0.0.0:8420 -e CONVERGIO_ALLOW_NON_LOCAL_BIND=1

FROM rust:1.78-bookworm AS builder
WORKDIR /work

# Pre-copy manifests for better layer caching.
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates ./crates

RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/work/target \
    cargo build --release --locked -p convergio-server

FROM debian:bookworm-slim
RUN set -euo pipefail; \
    apt-get update; \
    apt-get install -y --no-install-recommends ca-certificates; \
    rm -rf /var/lib/apt/lists/*

# Run as non-root.
RUN useradd -r -u 65532 -g nogroup convergio
USER 65532:65532

COPY --from=builder /work/target/release/convergio /usr/local/bin/convergio

EXPOSE 8420
ENTRYPOINT ["/usr/local/bin/convergio"]
CMD ["start"]
