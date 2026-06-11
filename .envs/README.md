# Environment configuration (`.envs/`)

Concern-grouped dotenv files, one tree per environment.

```
.envs/
  .example/     committed template. Copy to .local or .production.
  .local/       gitignored real local values for docker-compose.dev.yml.
  .test/        committed disposable values for docker-compose.test.yml.
  .production/  gitignored real production values.
```

Compose services load only the concern files they need. For example, `server`
loads `postgres.env`, `r2.env`, and `server.env`; `worker` loads
`postgres.env`, `r2.env`, and `worker.env`.

## Setup

```sh
cp -r .envs/.example .envs/.local
```

Then edit real secrets in `.envs/.local`. The committed `.test` tree is for the
disposable e2e stack and should not contain real secrets.

## Notes

- Docker service hostnames use compose names: `pgdog`, `postgres`,
  `redis-session`, `redis-cache`, `redis-lock`, `nats`, and `meilisearch`.
- The app standardizes on `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_NAME`,
  `POSTGRES_USER`, and `POSTGRES_PASSWORD`. Legacy `POSTGRES_WRITE_*` keys are
  still accepted by the Rust config loader for existing local `.env` files.
- For production with PgDog, set `POSTGRES_HOST=pgdog` and
  `POSTGRES_PORT=6432`, and keep `compose/production/pgdog/users.toml` aligned.
