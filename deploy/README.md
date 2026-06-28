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
      postgres.env r2.env server.env worker.env
      media-processor.env meilisearch.env

  compose/
    production/    # apisix/ pgdog/ configs — SHARED by both envs
      apisix/{config.yaml,apisix.yaml}
      pgdog/{pgdog.toml,users.toml}

  deploy/
    production.env  dev.env          # compose vars (committed)
    docker-compose.infra.yml
    docker-compose.app.yml
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
  -f docker-compose.infra.yml -f docker-compose.app.yml up -d
```

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
