ARG base
FROM debian:stable-slim AS patch
FROM ${base}

ARG artifact
COPY ${artifact} /bin/

COPY --from=patch                   \
    /lib/x86_64-linux-gnu/libz.so.1 \
    /lib/x86_64-linux-gnu/

ENTRYPOINT ["/bin/nu"]