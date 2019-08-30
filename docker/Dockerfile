ARG  FROMTAG=latest
FROM nushell/nu-base:${FROMTAG} as base
FROM rust:1.37-slim
COPY --from=base /usr/local/bin/nu /usr/local/bin/nu
ENTRYPOINT ["nu"]
