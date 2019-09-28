FROM gitpod/workspace-full
USER root
RUN apt-get update && apt-get install -y libssl-dev \
    libxcb-composite0-dev \
    pkg-config \
    curl \
    rustc
