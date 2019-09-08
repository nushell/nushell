ARG base
FROM ${base}

COPY target/release/nu* /bin/
ENTRYPOINT ["nu"]