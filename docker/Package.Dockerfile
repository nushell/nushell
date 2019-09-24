ARG base
FROM ${base}

ARG artifact
COPY ${artifact} /bin/

ENTRYPOINT ["/bin/nu"]