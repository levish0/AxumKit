# syntax=docker/dockerfile:1
FROM postgres:18.6-alpine
RUN --mount=type=secret,id=proxy_ca \
    if [ -s /run/secrets/proxy_ca ]; then \
      cat /etc/ssl/cert.pem /run/secrets/proxy_ca > /tmp/build-ca.pem; \
      export SSL_CERT_FILE=/tmp/build-ca.pem; \
    fi; \
    apk add --no-cache pgbackrest python3 && rm -f /tmp/build-ca.pem
COPY compose/data-services/pgbackrest.conf /etc/pgbackrest/pgbackrest.conf
COPY compose/data-services/bootstrap.sh /docker-entrypoint-initdb.d/10-hirune-roles.sh
COPY compose/data-services/backup_scheduler.py /usr/local/bin/backup_scheduler.py
COPY compose/data-services/restore.sh /usr/local/bin/hirune-restore
RUN chmod 755 /docker-entrypoint-initdb.d/10-hirune-roles.sh /usr/local/bin/hirune-restore \
    && chmod 644 /etc/pgbackrest/pgbackrest.conf /usr/local/bin/backup_scheduler.py
