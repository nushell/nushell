FROM rust:1.37-slim

# docker build -t nu .
# docker run -it nu

ENV DEBIAN_FRONTEND noninteractive
RUN apt-get update && apt-get install -y libssl-dev \
    libxcb-composite0-dev \
    libx11-dev \
    pkg-config && \
    mkdir -p /code

ADD . /code
WORKDIR /code
RUN cargo install nu
ENTRYPOINT ["nu"]
