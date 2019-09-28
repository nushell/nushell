ARG FROMTAG=latest
FROM quay.io/nushell/nu-base:${FROMTAG} as base
FROM ubuntu:18.04
COPY --from=base /usr/local/bin/nu /usr/local/bin/nu
ENV DEBIAN_FRONTEND noninteractive
RUN apt-get update && apt-get install -y libssl-dev \
    pkg-config
ENTRYPOINT ["nu"]
CMD ["-l", "info"]
