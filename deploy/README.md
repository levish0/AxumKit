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

**First-time setup (once per `<env>`).** Fill in `pgbackrest.env`, then:

```bash
cd deploy   # <env> = dev | production

# 0) Fill R2 bucket/endpoint/keys in pgbackrest.env. Note:
#    - key var names are PGBACKREST_REPO1_S3_KEY / _KEY_SECRET (NOT R2_*)
#    - endpoint is host-only (no https://, no bucket path)
#    After editing the file, recreate the container so it reloads the env:
just up dev

# 1) Create the stanza + verify. Until init runs, WAL archiving fails and WAL
#    piles up in pg_wal.
just pgbackrest-init dev
just pgbackrest-check dev        # verify stanza/repo/archiving (WAL round-trip test)

# 2) First full backup (the PITR baseline — at least one is required)
just pgbackrest-backup dev full
just pgbackrest-info dev         # list backup sets / sizes / retention in the repo
```

**Manual backup.** type = `full` | `diff` (since last full) | `incr` (since last backup, default).

```bash
just pgbackrest-backup dev full
just pgbackrest-backup dev diff
just pgbackrest-info dev
just compose dev exec redis-session redis-cli BGSAVE   # Redis Session (AOF) — separate
```

**Scheduled backups (automatic).** Register the schedule in the host crontab. The just
recipe is the easy way (weekly full on Sunday 03:00 + daily diff Mon–Sat; idempotent —
per-env and safe to re-run, no duplicate lines):

```bash
just pgbackrest-cron-install dev    # install/refresh (no sudo, user crontab)
crontab -l                         # check
just pgbackrest-cron-uninstall dev # remove
```

cron runs with a minimal PATH — if logs show `docker: not found`, add a line
`PATH=/usr/local/bin:/usr/bin:/bin` at the top of the crontab (`crontab -e`). To add the
lines by hand instead:

```cron
0 3 * * 0   cd /opt/axumkit/deploy && just pgbackrest-backup production full >> ~/pgbackrest-cron.log 2>&1
0 3 * * 1-6 cd /opt/axumkit/deploy && just pgbackrest-backup production diff >> ~/pgbackrest-cron.log 2>&1
```

**DB rebuilt (`down -v`, etc.).** Wiping the data volume changes the cluster system id, so it
no longer matches the old stanza metadata in the repo and you get `ERROR: [028]`. Reset the
stanza (this **deletes all repo backups** for that stanza — fine on dev, careful on prod):

```bash
just pgbackrest-stanza-reset dev   # stop -> stanza-delete --force -> start -> stanza-create
just pgbackrest-check dev
```

**Restore (PITR).** The `pgbackrest-restore` recipe does it all: stop postgres → `--delta`
restore (as the postgres user, into postgres-owned PGDATA) → start. DESTRUCTIVE — overwrites
the current cluster. The repo lives on R2, so recovery works even if the data volume is wiped.

```bash
cd deploy   # <env> = dev | production

# First decide WHICH backup — list what's in the repo (labels, timestamps, WAL ranges):
just pgbackrest-info dev

# Restore the latest backup + all WAL:
just pgbackrest-restore dev
# A specific backup, stopped at its own end (no later WAL) — --set + --type=immediate:
just pgbackrest-restore dev --set=20260628-120000F --type=immediate
# Point in time (PITR; 'T' avoids spaces, promote to resume writes after the target).
# The reachable range is within info's 'wal archive min/max':
just pgbackrest-restore dev --type=time --target=2026-06-28T12:00:00+00 --target-action=promote

just pgbackrest-check dev   # verify after restore
```

Reading `pgbackrest-info`: `full backup: <label>` is the `--set` value (suffix `F`=full /
`D`=diff / `I`=incr), `timestamp start/stop` is when it was taken, and `wal archive min/max`
is the full range you can PITR into. On start postgres replays archived WAL to reach a
consistent state; without `--target-action=promote` a PITR pauses recovery at the target and
you must promote it manually.

**Common errors**

| Symptom | Cause / fix |
|---|---|
| `[037] requires option: repo1-s3-key` | Key var named `R2_*` → use `PGBACKREST_REPO1_S3_KEY` / `_KEY_SECRET`, then `just up <env>` to recreate |
| `[041] ... /tmp/pgbackrest/...: Permission denied` | exec ran as root and the lock dir ownership is stale. The just recipes run as `-u postgres` (already set); clean leftovers with `just compose <env> exec -T postgres rm -rf /tmp/pgbackrest` |
| `[028] info files ... do not match the database` | DB was rebuilt and the system id changed → `just pgbackrest-stanza-reset <env>` |
| `[055] stop file does not exist` | `stanza-delete` requires a prior `stop` → the `stanza-reset` recipe handles the order |
| S3 connection / endpoint error | Endpoint has `https://` or a bucket path → leave the host only |

> Container names derive from `COMPOSE_PROJECT_NAME` (e.g. `axumkit_dev-postgres-1`). To
> reach a service by name, use `just compose <env> exec <service> ...`.

## Ports

The tunnel/ingress (host-side) points at APISIX. One route per env:

| Env | APISIX (tunnel → ) | Postgres (loopback, SSH tunnel) |
|-----|--------------------|----------------------------------|
| production | `127.0.0.1:9090` | `127.0.0.1:15435` |
| dev        | `127.0.0.1:9080` | `127.0.0.1:15434` |

`server:8000` and `pgdog:6432` are not host-published — they're reached over the compose
network; use `docker compose ... exec` for debugging. Postgres IS published, but on
**127.0.0.1 only** (`POSTGRES_HOST_PORT` in `deploy/<env>.env`), so a DB client can reach it
through an SSH tunnel; never rebind it to `0.0.0.0` (that exposes the DB publicly, and Docker
bypasses ufw).

> **Security:** the tunnel/ingress must target APISIX, never `server:8000` directly, or
> the gateway's rate limiting is bypassed. Set a strong `INTERNAL_PROXY_SECRET` in
> `server.env` (shared with the backend) so APISIX may forward the true client IP via
> `X-Real-Client-IP`.
