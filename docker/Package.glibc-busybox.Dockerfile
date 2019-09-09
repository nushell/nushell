ARG base
FROM debian:stable-slim AS patch
FROM ${base}

ARG artifact
COPY ${artifact} /bin/

COPY --from=patch /lib/x86_64-linux-gnu/libz* /lib/x86_64-linux-gnu/
COPY --from=patch /lib/x86_64-linux-gnu/libdl* /lib/x86_64-linux-gnu/
COPY --from=patch /lib/x86_64-linux-gnu/librt* /lib/x86_64-linux-gnu/
ENTRYPOINT ["/bin/nu"]