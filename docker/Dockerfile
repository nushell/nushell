# Git: git version 2.30.2
# /etc/os-release: Alpine Linux v3.16
# Kernel: Linux ca3abedc4fb1 5.17.15-76051715-generic #202206141358~1655919116~22.04~1db9e34 SMP PREEMPT Wed Jun 22 19 x86_64 Linux
# Build cmd: docker build --no-cache . -t nushell-latest
# Other tags: nushell/alpine-nu:latest, nushell
FROM alpine

LABEL maintainer=nushell

RUN echo '/usr/bin/nu' >> /etc/shells \
    && adduser -D -s /usr/bin/nu nushell \
    && mkdir -p /home/nushell/.config/nushell/ \
    && wget -q https://raw.githubusercontent.com/nushell/nushell/main/crates/nu-utils/src/sample_config/default_config.nu -O /home/nushell/.config/nushell/config.nu \
    && wget -q https://raw.githubusercontent.com/nushell/nushell/main/crates/nu-utils/src/sample_config/default_env.nu -O /home/nushell/.config/nushell/env.nu \
    && cd /tmp \
    && wget -qO - https://api.github.com/repos/nushell/nushell/releases/latest \
    |grep browser_download_url \
    |grep musl \
    |cut -f4 -d '"' \
    |xargs -I{} wget {} \
    && tar -xzf nu* \
    && chmod +x nu \
    && mv nu /usr/bin/nu \
    && chown -R nushell:nushell /home/nushell/.config/nushell \
    && rm -rf /tmp/*

USER nushell

WORKDIR /home/nushell

ENTRYPOINT ["nu"]
