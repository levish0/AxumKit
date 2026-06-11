# Docker Deployment

## Multi-stage Dockerfile

AxumKit uses a multi-stage Dockerfile with [cargo-chef](https://github.com/LukeMathWalker/cargo-chef) for dependency caching.

### Build Stages

1. `chef` - installs cargo-chef on Rust base image
2. `planner` - generates `recipe.json`
3. `builder` - builds dependencies and binaries
4. `server-runtime` - minimal image for API server
5. `worker-runtime` - minimal image for worker

### Build Images

```bash
docker build --target server-runtime -t server .
docker build --target worker-runtime -t worker .
```

## Compose Environments

AxumKit has two root compose files:

| File | Purpose |
|------|---------|
| `docker-compose.dev.yml` | Local infrastructure and optional full app stack. Uses `.envs/.local/*`. |
| `docker-compose.test.yml` | Disposable e2e stack. Uses committed `.envs/.test/*`. |

Create local compose env files with:

```bash
cp -r .envs/.example .envs/.local
```

Common commands:

```bash
just up-infra
just up
just down
just e2e
```

### Security Notes

- Runs as non-root user
- Minimal runtime dependencies
- Separate runtime targets for server and worker

## Health Checks

| Service | Health Check |
|---------|-------------|
| PostgreSQL | `pg_isready -U axumkit` |
| PgDog | `pg_isready -h localhost -p 6432` |
| Redis | `redis-cli ping` |
| MeiliSearch | `curl http://localhost:7700/health` |
| NATS | `wget http://localhost:8222/healthz` |
| Object Storage (e2e) | SeaweedFS bucket init job |
| Server | `curl http://localhost:8000/health-check` |
