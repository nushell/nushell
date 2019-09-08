ARG artifact
ARG base
FROM ${base}

COPY ${artifact} /bin/
ENTRYPOINT ["nu"]