FROM gitpod/workspace-full
WORKDIR /workspace/nushell
USER root
RUN apt-get update && apt-get install -y libssl-dev \
    libxcb-composite0-dev \
    pkg-config \
    curl \
    rustc
RUN echo "##vso[task.prependpath]/root/.cargo/bin" && \
    rustc -Vv && \
    if $RELEASE; then cargo build --release && cargo run --release; \
                   cp target/release/nu /usr/local/bin; \   
                 else cargo build; \
                   cp target/debug/nu /usr/local/bin; fi;
RUN cargo build
