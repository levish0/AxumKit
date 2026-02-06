# Docker Deployment

## Multi-stage Dockerfile

AxumKit uses a multi-stage Dockerfile with [cargo-chef](https://github.com/LukeMathWalker/cargo-chef) for dependency caching.

### Build Stages

```
┌─────────┐     ┌──────────┐     ┌──────────┐
│  chef   │────▶│ planner  │────▶│ builder  │
│ (rust)  │     │(recipe)  │     │(compile) │
└─────────┘     └──────────┘     └────┬─────┘
                                      │
                            ┌─────────┴─────────┐
                            ▼                   ▼
                    ┌───────────────┐   ┌───────────────┐
                    │server-runtime │   │worker-runtime │
                    │(debian slim)  │   │(debian slim)  │
                    └───────────────┘   └───────────────┘
```

1. **chef** — Installs `cargo-chef` on the Rust base image
2. **planner** — Generates `recipe.json` (dependency manifest)
3. **builder** — Builds dependencies (cached), then compiles the application
4. **server-runtime** — Minimal Debian image with the server binary
5. **worker-runtime** — Minimal Debian image with the worker binary

### Building

```bash
# Build server image
docker build --target server-runtime -t axumkit-server .

# Build worker image
docker build --target worker-runtime -t axumkit-worker .
```

### Security

- Non-root user (`app:app`)
- Minimal runtime image (debian:stable-slim)
- Only essential runtime dependencies (ca-certificates, libssl3, libpq5)

## docker-compose (E2E)

The `docker-compose.e2e.yml` file provides all infrastructure services for end-to-end testing:

```yaml
services:
  postgres:        # PostgreSQL 18
  redis-session:   # Redis 8 (AOF, persistent)
  redis-cache:     # Redis 8 (LRU, volatile)
  meilisearch:     # MeiliSearch v1.30.1
  nats:            # NATS 2.12.3 (JetStream)
  seaweedfs:       # SeaweedFS 4.04 (Filer + S3)
  server:          # AxumKit server
  worker:          # AxumKit worker
```

### Running

```bash
# Build and start all services
docker compose -f docker-compose.e2e.yml up --build

# Start infrastructure only (for local development)
docker compose -f docker-compose.e2e.yml up postgres redis-session redis-cache meilisearch nats seaweedfs
```

### Redis Configuration

Two Redis instances with different eviction policies:

| Instance | Policy | Purpose |
|----------|--------|---------|
| `redis-session` | `appendonly yes`, `volatile-ttl` | Sessions, tokens, rate limits (persistent) |
| `redis-cache` | `allkeys-lru` | Document cache, view counts (volatile) |

### Health Checks

All services have health checks configured:

| Service | Health Check |
|---------|-------------|
| PostgreSQL | `pg_isready -U postgres -d axumkit` |
| Redis | `redis-cli ping` |
| MeiliSearch | `curl http://localhost:7700/health` |
| NATS | `wget http://localhost:8222/healthz` |
| SeaweedFS | `curl http://localhost:8333/` |
| Server | `curl http://localhost:8000/health-check` |

## Helm Charts

AxumKit includes Helm charts for Kubernetes deployment in the `charts/` directory:

```
charts/
├── axumkit/           # Umbrella chart
├── axumkit-server/    # Server deployment
│   ├── templates/
│   │   ├── configmap.yaml
│   │   ├── secret.yaml
│   │   ├── deployment.yaml
│   │   ├── service.yaml
│   │   ├── ingress.yaml
│   │   ├── hpa.yaml            # Horizontal Pod Autoscaler
│   │   ├── pdb.yaml            # Pod Disruption Budget
│   │   ├── serviceaccount.yaml
│   │   └── migration-job.yaml  # Pre-deploy migration
│   └── values.yaml
└── axumkit-worker/    # Worker deployment
    ├── templates/
    │   ├── configmap.yaml
    │   ├── secret.yaml
    │   ├── deployment.yaml
    │   ├── hpa.yaml
    │   ├── pdb.yaml
    │   └── serviceaccount.yaml
    └── values.yaml
```

### Key Features

- **HPA:** Auto-scaling from 1 to 5 replicas at 70% CPU
- **PDB:** Minimum 1 pod always available
- **Security:** Non-root user (UID 1000), read-only root filesystem, no privilege escalation
- **Migration Job:** Runs database migrations before deployment
- **ConfigMap/Secret separation:** Non-sensitive config in ConfigMap, credentials in Secret

### Deploying

```bash
# Install with Helm
helm install axumkit charts/axumkit \
  --set server.config.ENVIRONMENT=prod \
  --set server.secrets.POSTGRES_WRITE_PASSWORD=mypassword \
  # ... other values

# Or use a values file
helm install axumkit charts/axumkit -f my-values.yaml
```
