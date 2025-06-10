# syntax=docker/dockerfile:latest

# Git: git version 2.30.2
# /etc/os-release: Alpine Linux v3.20
# Kernel: Linux ca3abedc4fb1 5.17.15-76051715-generic #202206141358~1655919116~22.04~1db9e34 SMP PREEMPT Wed Jun 22 19 x86_64 Linux
# Build cmd: docker build --no-cache . -t nushell:latest
# Other tags: nushell:latest-alpine, nushell
FROM alpine

ARG TARGETARCH
ARG ARCH=${TARGETARCH/arm64/aarch64}
ARG ARCH=${ARCH/arm/armv7}
ARG ARCH=${ARCH/amd64/x86_64}

ARG BUILD_REF
ARG BUILD_DATE
ARG RELEASE_QUERY_API="https://api.github.com/repos/nushell/nushell/releases/latest"

LABEL maintainer="The Nushell Project Developers" \
    org.opencontainers.image.licenses="MIT" \
    org.opencontainers.image.title="Nushell" \
    org.opencontainers.image.created=$BUILD_DATE \
    org.opencontainers.image.revision=$BUILD_REF \
    org.opencontainers.image.authors="The Nushell Project Developers" \
    org.opencontainers.image.vendor="Nushell Project" \
    org.opencontainers.image.description="A new type of shell" \
    org.opencontainers.image.source="https://github.com/nushell/nushell" \
    org.opencontainers.image.documentation="https://www.nushell.sh/book/"

RUN echo '/usr/bin/nu' >> /etc/shells \
    && adduser -D -s /usr/bin/nu nushell \
    && mkdir -p /home/nushell/.config/nushell/ \
    && cd /tmp \
    && wget -qO - ${RELEASE_QUERY_API} \
    | grep browser_download_url \
    | cut -d '"' -f 4 \
    | grep ${ARCH}-unknown-linux-musl \
    | xargs -I{} wget {} \
    && mkdir nu-latest && tar xvf nu-*.tar.gz --directory=nu-latest \
    && cp -aR nu-latest/**/* /usr/bin/ \
    # Setup default config file for nushell
    && cd /home/nushell/.config/nushell \
    && chmod +x /usr/bin/nu \
    && chown -R nushell:nushell /home/nushell/.config/nushell \
    # Reset Nushell config to default
    && su -c 'config reset -w' nushell \
    && ls /usr/bin/nu_plugin* \
    | xargs -I{} su -c 'plugin add {}' nushell \
    && rm -rf /tmp/*

USER nushell

WORKDIR /home/nushell

ENTRYPOINT ["nu"]
