FROM gitpod/workspace-full

# Gitpod will not rebuild Nushell's dev image unless *some* change is made to this Dockerfile.
# To force a rebuild, simply increase this counter:
ENV TRIGGER_REBUILD 1

USER gitpod

RUN sudo apt-get update && \
    sudo apt-get install -y \
        libssl-dev \
        libxcb-composite0-dev \
        pkg-config \
        libpython3.6 \
        rust-lldb \
    && sudo rm -rf /var/lib/apt/lists/*

ENV RUST_LLDB=/usr/bin/lldb-11
