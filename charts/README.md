# AxumKit Helm Charts

## Structure

```
charts/
├── axumkit/            # Umbrella chart (full stack)
│   ├── Chart.yaml      # Dependencies definition
│   ├── values.yaml     # Production defaults
│   └── values-local.yaml # Local development
├── axumkit-server/     # API server chart
├── axumkit-worker/     # Background worker chart
└── README.md
```

## Local Development (Rancher Desktop / k3s)

### 1. Build Docker Images

```bash
# From project root
docker build -t ghcr.io/levish0/axumkit:0.4.0 --target server-runtime .
docker build -t ghcr.io/levish0/axumkit-worker:0.4.0 --target worker-runtime .
```

### 2. Configure values-local.yaml

```bash
cd charts/axumkit
cp values-local.yaml.example values-local.yaml
# Edit and fill in your credentials
```

### 3. Download Dependencies and Deploy

```bash
cd charts/axumkit
helm dependency update
helm install axumkit . -f values-local.yaml
```

### 4. Verify

```bash
kubectl get pods
kubectl logs -l app.kubernetes.io/name=axumkit-server -f
kubectl port-forward svc/axumkit-axumkit-server 8000:8000
```

## DB Migration & Seed

### Migration (Automatic)

Migration runs automatically on `helm install/upgrade` via Helm hook:

```yaml
# charts/axumkit-server/templates/migration-job.yaml
annotations:
  "helm.sh/hook": post-install,post-upgrade
  "helm.sh/hook-weight": "-5"
  "helm.sh/hook-delete-policy": hook-succeeded
```

An initContainer waits for PostgreSQL to be ready before running migration.

### Manual Migration

```bash
# Run from existing Pod
kubectl exec -it <pod-name> -- ./migration

# Or run as a one-off Job
kubectl apply -f - <<EOF
apiVersion: batch/v1
kind: Job
metadata:
  name: migration-manual
spec:
  template:
    spec:
      restartPolicy: Never
      containers:
        - name: migration
          image: ghcr.io/levish0/axumkit:0.4.0
          command: ["./migration"]
          envFrom:
            - configMapRef:
                name: axumkit-axumkit-server
            - secretRef:
                name: axumkit-axumkit-server
EOF

kubectl delete job migration-manual
```

## Production Deployment

```bash
helm install axumkit ./charts/axumkit \
  -f values-prod.yaml \
  --set axumkit-server.secret.POSTGRES_WRITE_PASSWORD=xxx \
  --set axumkit-worker.secret.POSTGRES_WRITE_PASSWORD=xxx
```

## Chart Version Management

| Chart          | Version | Description       |
|----------------|---------|-------------------|
| axumkit        | 0.4.0   | Umbrella chart    |
| axumkit-server | 0.4.0   | API server        |
| axumkit-worker | 0.4.0   | Background worker |

Keep versions in sync with Cargo.toml:
```bash
# Cargo.toml
[workspace.package]
version = "0.4.0"

# charts/*/Chart.yaml
version: 0.4.0
appVersion: "0.4.0"
```

## Dependency Chart Versions

| Chart       | Version | Source      |
|-------------|---------|-------------|
| postgresql  | 18.2.3  | bitnami     |
| redis       | 24.1.2  | bitnami     |
| nats        | 2.12.3  | nats-io     |
| meilisearch | 0.21.0  | meilisearch |
| seaweedfs   | 4.0.407 | seaweedfs   |

## Troubleshooting

### Pod in CrashLoopBackOff

This is normal right after deployment. The pod will recover automatically once migration completes.

```bash
# Watch status
kubectl get pods -w
```

### PostgreSQL Connection Failed

```bash
# Check PostgreSQL Pod status
kubectl get pods -l app.kubernetes.io/name=postgresql
kubectl logs -l app.kubernetes.io/name=postgresql
```

### Helm Dependency Error

```bash
rm Chart.lock
helm dependency update
```
