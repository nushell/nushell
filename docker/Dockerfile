ARG FROMTAG=latest
FROM quay.io/nushell/nu-base:${FROMTAG} as base
FROM ubuntu:18.04
COPY --from=base /usr/local/bin/nu /usr/local/bin/nu
ENV DEBIAN_FRONTEND noninteractive
RUN apt-get update \
    && apt-get install -y libssl-dev pkg-config \
    && apt-get clean \
    && rm -fr /var/lib/apt/lists/*
ENTRYPOINT ["nu"]
CMD ["-l", "info"]
