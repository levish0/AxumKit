# Postgres + pgBackRest. The stock postgres:18-alpine image has no pgbackrest, but
# both WAL archiving (archive_command) and scheduled backups must run co-located with
# the database (pgBackRest needs the data dir AND a local DB connection), so the binary
# has to live inside the postgres container. Tiny layer on top of the official image.
ARG POSTGRES_VERSION=18-alpine
FROM postgres:${POSTGRES_VERSION}

RUN apk add --no-cache pgbackrest \
    && mkdir -p /var/log/pgbackrest /etc/pgbackrest \
    && chown -R postgres:postgres /var/log/pgbackrest /etc/pgbackrest
