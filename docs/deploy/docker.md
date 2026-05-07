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

### Security Notes

- Runs as non-root user
- Minimal runtime dependencies
- Separate runtime targets for server and worker

## docker-compose (E2E)

`docker-compose.e2e.yml` provides infrastructure for end-to-end tests:

```yaml
services:
  postgres
  redis-session
  redis-cache
  meilisearch
  nats
  object-storage   # R2-compatible S3 API (MinIO)
  object-storage-init
  server
  worker
```

### Run

```bash
# Build and start all services
docker compose -f docker-compose.e2e.yml up --build

# Start infrastructure only
docker compose -f docker-compose.e2e.yml up postgres redis-session redis-cache meilisearch nats object-storage object-storage-init
```

### Health Checks

| Service | Health Check |
|---------|-------------|
| PostgreSQL | `pg_isready -U postgres -d axumkit` |
| Redis | `redis-cli ping` |
| MeiliSearch | `curl http://localhost:7700/health` |
| NATS | `wget http://localhost:8222/healthz` |
| Object Storage | `curl http://localhost:9000/minio/health/live` |
| Server | `curl http://localhost:8000/health-check` |

## Helm Charts

Helm charts live in `charts/`:

- `charts/axumkit` (umbrella)
- `charts/axumkit-server`
- `charts/axumkit-worker`

Key chart features:

- HPA support
- Pod disruption budgets
- Non-root security defaults
- Migration hook job
- ConfigMap/Secret separation
