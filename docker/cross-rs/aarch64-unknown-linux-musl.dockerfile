FROM ghcr.io/cross-rs/aarch64-unknown-linux-musl:latest

RUN dpkg --add-architecture arm64 && \
    apt-get update && \
    apt-get install --assume-yes libssl-dev:arm64 clang
