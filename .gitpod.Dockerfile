FROM gitpod/workspace-full

USER root
RUN apt-get update && apt-get install -y libssl-dev \
    libxcb-composite0-dev \
    pkg-config \
    curl
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y --no-modify-path --default-toolchain `cat rust-toolchain`
RUN echo "##vso[task.prependpath]/root/.cargo/bin" && \
    rustc -Vv && \
    if $RELEASE; then cargo build --release && cargo run --release; \
                   cp target/release/nu /usr/local/bin; \   
                 else cargo build; \
                   cp target/debug/nu /usr/local/bin; fi;
RUN cargo build
