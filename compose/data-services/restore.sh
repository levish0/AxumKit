#!/bin/sh
set -eu
# This service receives only a newly created recovery volume.
if [ -n "$(ls -A /var/lib/postgresql)" ]; then
    echo 'Recovery requires an empty volume.' >&2
    exit 1
fi
install -d -o postgres -g postgres "$PGDATA"
exec gosu postgres pgbackrest --stanza=db --lock-path=/tmp/pgbackrest-restore "$@" restore
