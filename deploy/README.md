# AxumKit Deploy Bundle

Server deploys do not need the full source tree when using GHCR images. Keep these
paths together under one deploy root, for example `/opt/axumkit`.

**Parametrized for two environments** (production + dev) from one set of compose files.
A per-env compose env-file (`deploy/<env>.env`, committed — no secrets) selects what
differs: `DEPLOY_ENV` (which `.envs/.<env>/` to load), `COMPOSE_PROJECT_NAME` (isolates
containers/volumes/networks — prod and dev can run on the **same host**), `APP_VERSION`
(image tag), `APISIX_HOST_PORT` (host port).

```text
/opt/axumkit/
  .envs/
    .production/   # prod secrets — gitignored
    .dev/          # dev secrets  — gitignored
      postgres.env pgbackrest.env r2.env server.env worker.env
      media-processor.env meilisearch.env

  compose/
    production/    # apisix/ pgdog/ pgbackrest/ configs — SHARED by both envs
      apisix/{config.yaml,apisix.yaml}
      pgdog/{pgdog.toml,users.toml}
      pgbackrest/pgbackrest.conf

  deploy/
    production.env  dev.env          # compose vars (committed)
    docker-compose.infra.yml
    docker-compose.app.yml
    postgres/Dockerfile              # postgres:18-alpine + pgbackrest
    justfile  README.md
```

The `compose/production/` configs are env-agnostic (Docker service names + container
ports only) and **shared** by both envs — both use the same DB credentials but separate
Postgres instances (isolated volumes). Don't move them to a per-env folder.

Required paths on the host: `deploy/`, `compose/production/`, and the env's
`.envs/.<env>/`. `.envs/.<env>/` is gitignored — copy it out-of-band (start from
`.envs/.example/`).

## Start

The short way — `deploy/justfile` (run from inside `deploy/`):

```bash
cd /opt/axumkit/deploy
just up dev                  # infra + app  (or: just up production)
just ps dev
just logs dev server
just pull production && just up production    # update
just down dev                # stop (keep volumes);  `just down dev -v` to wipe
```

The long way (what those wrap):

```bash
cd /opt/axumkit/deploy
docker compose --env-file dev.env \
  -f docker-compose.infra.yml -f docker-compose.app.yml up -d --build
```

## Backups (pgBackRest → R2)

PostgreSQL backs up with **pgBackRest**. Two things run together:

1. **WAL archiving** — the postgres container pushes every changed WAL segment to the R2
   repo via `archive_command` (always on). This is what makes point-in-time recovery (PITR)
   possible.
2. **Scheduled backups** — `full`/`diff`/`incr` snapshots, run periodically from host crontab.

The repo is a **dedicated R2 backup bucket** (separate from the asset bucket in `r2.env`).
Credentials/bucket/path live in `.envs/.<env>/pgbackrest.env`; static config (retention,
compression, stanza) is in `compose/production/pgbackrest/pgbackrest.conf`. Retention is
`repo1-retention-full=2` (keep the 2 newest full backups + their WAL) — tune it in the conf.

The postgres service is a custom image (`postgres:18-alpine` + pgbackrest, built from
`deploy/postgres/Dockerfile`); `just up <env>` builds it automatically (`--build`).

```bash
cd deploy   # <env> = dev | production

# 0) (once) fill R2 bucket/endpoint/keys in pgbackrest.env, then create the stanza.
#    Until this runs, WAL archiving fails and WAL accumulates in pg_wal.
just pgbackrest-init dev
just pgbackrest-check dev        # verify stanza/repo/archiving are healthy

# 1) Run a backup (manual)
just pgbackrest-backup dev full  # full backup
just pgbackrest-backup dev diff  # differential (since last full)
just pgbackrest-backup dev incr  # incremental (default)
just pgbackrest-info dev         # list backup sets / sizes / retention in the repo
```

**Scheduling (host crontab).** With the containers up, host crontab calls `just` (it must
`cd` into `deploy/`). Example: weekly full on Sunday 03:00, daily diff otherwise.

```cron
# /etc/crontab or `crontab -e` (adjust the deploy root path)
0 3 * * 0   cd /opt/axumkit/deploy && just pgbackrest-backup production full >> /var/log/pgbackrest-cron.log 2>&1
0 3 * * 1-6 cd /opt/axumkit/deploy && just pgbackrest-backup production diff >> /var/log/pgbackrest-cron.log 2>&1
```

**Restore (PITR).** Restore overwrites the data directory, so stop postgres first. The repo
lives on R2, so recovery works even if the data volume is wiped/recreated.

```bash
cd deploy   # <env> = dev | production
just compose dev stop postgres
# After emptying the data dir (or recreating the volume), restore via a one-off container.
# Restore the latest backup:
just compose dev run --rm --no-deps postgres pgbackrest --stanza=db restore
# Or to a specific point in time (PITR):
just compose dev run --rm --no-deps postgres \
  pgbackrest --stanza=db --type=time "--target=2026-06-28 12:00:00+00" restore
just compose dev start postgres
just pgbackrest-check dev
```

> Container names derive from `COMPOSE_PROJECT_NAME` (e.g. `axumkit_dev-postgres-1`). To
> reach a service by name, use `just compose <env> exec <service> ...`.

## Ports

The tunnel/ingress (host-side) points at APISIX. One route per env:

| Env | APISIX (tunnel → ) |
|-----|--------------------|
| production | `127.0.0.1:9090` |
| dev        | `127.0.0.1:9080` |

`server:8000` and `pgdog:6432` are not host-published — they're reached over the
compose network; use `docker compose ... exec` for debugging.

> **Security:** the tunnel/ingress must target APISIX, never `server:8000` directly, or
> the gateway's rate limiting is bypassed. Set a strong `INTERNAL_PROXY_SECRET` in
> `server.env` (shared with the backend) so APISIX may forward the true client IP via
> `X-Real-Client-IP`.
