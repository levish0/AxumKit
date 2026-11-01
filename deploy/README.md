# Deployment

`deploy/justfile` deploys application processes to Kubernetes. Separate `data-*`
commands manage optional self-hosted data services with Docker Compose. It does
not install k3s. Image build files live in `compose/`. Root `just infra-up` starts
only local development dependencies with Compose.

The production path is Cloudflare Tunnel → private APISIX → API Service → API
Pods. The baseline has two API, two APISIX, and two cloudflared replicas.
One environment-specific Cloudflare Tunnel accepts both cloudflared connectors.
API scaling does not create tunnels. APISIX uses the same generated routing
configuration on both Pods. PostgreSQL owns sessions, login limits, API usage,
and jobs. APISIX replicas share one Valkey instance per environment for a separate
broad IP limit. No forward-auth service is used.

PostgreSQL, SMTP, optional S3 storage, and optional search are external services.
The production manifests do not install PostgreSQL or allocate a database PVC.
Provision persistent storage, database grants, backups, and recovery separately.
A managed PostgreSQL service uses the same connection settings. For one Linux
machine, the [data profile](data-services/README.md) runs PostgreSQL, Meilisearch, and a
pgBackRest scheduler beside k3s. Its data volumes and off-host backups have a
separate lifecycle from application rollout.

## Environments and commands

Publish the application with a `v<workspace-version>` Git tag, such as `v0.1.0`.
The CI workflow checks that the tag matches `Cargo.toml`. It runs the normal
checks and production, backup/recovery, and two-node Kubernetes acceptance before
publishing the tested application image as `ghcr.io/<owner>/<repository>:0.1.0`.
The publication job loads the tested image and checks its identity; it does not
rebuild it. Branch, pull-request, and manual runs do not publish images.

Copy the `APPLICATION_IMAGE=...@sha256:...` reference from the workflow summary
into the selected overlay. Use `newName` for the registry path and `digest` for
the `sha256:...` value; remove `newTag` when selecting a digest. The image package
must be readable by the cluster. New GHCR packages start private; configure their
access or visibility separately. Publishing an image does not deploy it.
PostgreSQL uses its independent [`postgres-v*` release](data-services/README.md#publish-and-select-the-postgresql-image).

Copy `.envs/.example/dev/` to ignored `.envs/.dev/` and
`.envs/.example/prod/` to ignored `.envs/.prod/`. Each `k3s/` directory contains:

- `target.env`: an explicit Kubernetes context and namespace.
- `kustomization.yaml`: the namespace, application image, and optional patches.
- `app.env`: public runtime settings.
- `gateway.env`: `EDGE_RATE_LIMIT_REQUESTS` per IP per 60 seconds, shared by gateways.
- `secrets.env`: encryption keys, authentication settings, and provider secrets.
- `runtime.env`, `maintenance.env`, `migration.env`: separate database roles.
- `tunnel.env`: that environment's Cloudflare Tunnel token.

Use separate databases, encryption/rate-limit keys, origins, and tunnels for dev
and prod. Each namespace also has its own disposable Valkey. Both hosted environments use `APP_ENV=production` for HTTPS security.
The deployment name `dev` does not disable production security rules.

Set `KUBE_CONTEXT` to an existing context from your kubeconfig. Set
`KUBE_NAMESPACE` to match the overlay's `namespace`. Dev defaults to `hirune-dev`;
prod defaults to `hirune-prod`. Contexts can select separate clusters or the same
cluster with separate namespaces. Every operator command validates the rendered
namespace and supplies both context and namespace explicitly. It does not follow
the kubeconfig's current context. These settings select targets; Kubernetes RBAC
must enforce operator permissions.

Set an immutable image tag or digest in each overlay. Replace credential
placeholders. Kubernetes generator env files use raw values: JSON has no outer
single quotes, and shell variables are not expanded. See the
[k3s runbook](k3s/README.md) for cluster and Tunnel preparation.

The just recipes use a POSIX shell for argument forwarding. On Windows, use
Git for Windows `sh`, or invoke the Rust CLI directly, for example
`cargo xtask deploy-k3s up prod`. Local development recipes retain their normal
platform shell.

Run from this directory:

```sh
just check dev
just up dev
just check prod
just up prod
just status prod
just logs prod hirune-api
just manage prod set-api-client-quota UUID 100 10000
just cleanup prod
```

`check` renders and validates locally without displaying Secret values or
contacting the cluster. `up` defaults to `auto`. A read-only Job checks pending
migrations with the selected release image before runtime changes. Pending
migrations, active gateway maintenance, or stopped applications select a planned
interruption: verify APISIX maintenance, drain applications, migrate, restore
runtime readiness, then disable maintenance. Other releases roll normally.
API startup does not run migrations. Use `just up prod drain` to force an offline
upgrade, including incompatible job or configuration changes without a migration.
Failures retain maintenance; repeat `just up prod` after correcting the cause.
`just down prod` stops runtime Pods and preserves
namespace, settings, Secrets, and external database data. HPAs remain installed
during down and drain. The helper saves positive replica counts, verifies native
HPA deactivation at zero, and restores counts after migration success. HPA
minimums must be positive. No manual HPA removal or restoration is required.

Root `just deploy-k3s prod` delegates to this same `up` entrypoint. All deployment
commands require `dev` or `prod`; none defaults to production. Do not run concurrent
releases. Serialize deployment in your release pipeline.

## SMTP trust

Production requires `SMTP_STARTTLS=true`. Publicly trusted SMTP certificates use
the transport's normal trust store. For a private SMTP CA, set the optional
`SMTP_CA_CERTIFICATE` to the PEM CA certificate path inside the container, such
as `/certificates/smtp-ca.crt`. Add a read-only certificate mount to each API,
worker, manage, and maintenance container that uses this setting. Every application
bootstrap constructs the mail transport. The base manifests do not add this mount automatically. Application user 10001 must be able to read the
file. Mount only the public CA certificate. The application does not need its private key.

An empty or omitted setting uses the normal trust store. Startup fails if the
certificate is unreadable or invalid. A custom certificate requires STARTTLS.
The transport still verifies the certificate chain and the `SMTP_HOST` hostname
when you add a CA. Keep production certificates and mounts separate from the
ephemeral certificates generated by `just test-production`.

## Database roles and maintenance

Provision three database roles:

- Migration owns schema changes.
- Runtime has only the required application privileges and migration-history
  SELECT. It may insert and read audit events and API usage, but may not update
  or delete them.
- Maintenance has the runtime permissions needed for authentication, job, and
  file cleanup. It may also delete audit events and API usage. It has no schema
  ownership or DDL permissions.

Verify the actual database grants during deployment acceptance. Separate
environment files do not enforce these permissions.
The optional `MIGRATION_RUNTIME_ROLE` and `MIGRATION_MAINTENANCE_ROLE` settings
enable post-migration grants for the self-hosted profile. Set both or neither.
The migration Job must finish this step before runtime rollout. Existing DDL or
inherited/PUBLIC authority that violates the role contract causes failure.

API and worker receive the generated runtime database Secret. Ordinary `manage`
commands execute in an API Pod with that runtime role. Maintenance receives its
own generated database Secret. `just cleanup prod` executes `manage cleanup` in
a maintenance Pod with that role. Runtime Pods receive neither migration nor
maintenance database credentials. Do not broaden runtime permissions to run
maintenance commands.

Defaults retain succeeded jobs for 30 days, failed jobs for 90 days, and audit
events for 180 days. Set `SUCCEEDED_JOB_RETENTION_DAYS`, `FAILED_JOB_RETENTION_DAYS`,
and `AUDIT_RETENTION_DAYS` in `app.env`; each supports 1 through 36500 days. Deleting
a failed job removes its attempt history and retry ability. Bounded cleanup may
need repeated runs to clear a backlog.

API admission history defaults to 90 days via `API_USAGE_RETENTION_DAYS` (range
1 through 36500); each cleanup removes at most 1000 old records. Runtime has
SELECT and INSERT on `api_usage`, while maintenance additionally has DELETE.
Neither role may UPDATE or TRUNCATE this table. Retention never resets request
counters or grants allocation.

New organization-owned API clients have zero allocation. An authorized operator
uses `just manage prod set-api-client-quota <uuid> <per-minute> <per-month>`
to assign strict fixed-minute and UTC-calendar-month budgets. The owning
organization can manage scopes and keys but cannot assign itself quota. All keys
and API instances share PostgreSQL authority; see [API access](../docs/behavior/api-access.md).
Deploy edge abuse protection separately when exposing a public developer API.

The portable `maintenance` executable schedules cleanup followed by search repair
using the shared job runtime. `just up prod` starts it with API and worker.
Set `MAINTENANCE_INTERVAL_SECONDS` in `app.env` (default 300, range 1–86400). Multiple
maintenance replicas must use the same interval. The database deduplicates each
interval's job. Jobs from different intervals or retries may overlap. Handlers
tolerate repetition. Kubernetes restarts the process if scheduling or claiming fails.
No host cron or systemd timer is required. Existing failed maintenance jobs can
be inspected and retried explicitly with `manage retry-job <uuid>`.

Grant global administrator status with
`just manage prod set-administrator <uuid> true` only after the account is
active, verified, and enrolled in TOTP. Use `false` to demote it. The command
invalidates existing credentials and audits a changed role. Restrict operator
access to management commands; there is no public administrator-management API.

Optional file storage uses an externally provisioned private S3-compatible
SeaweedFS gateway and bucket. The master, filer, volume servers, and gateway admin
interfaces must stay on private infrastructure. Configure production credentials
with bucket-limited read/write/list access. Use a separate operator credential for
provisioning when runtime credentials cannot create buckets or indexes.
API/worker startup does not create buckets or search indexes.

Meilisearch is an optional private derived index. Provision the file index using
`manage initialize-integrations` with appropriate operator permissions, then use
an index-limited server key. Never send the master/runtime key to browsers.
The maintenance service registers search repair as well as cleanup; file deletion
and indexing jobs use the application queue and require a normal worker. Back up
PostgreSQL and file originals. Rebuild the search index from database metadata when needed.

File bytes bypass the API through presigned S3 URLs. Set S3_PUBLIC_ENDPOINT when
clients cannot reach S3_ENDPOINT. Preserve the signed host, path, and query. Keep
the bucket private. Provision its CORS rules from ALLOWED_ORIGINS through
`manage initialize-integrations`. Set `X-Content-Type-Options: nosniff` on the
storage ingress and omit signed queries from logs and error reports. Enforce a
maximum total PUT duration no greater than FILE_UPLOAD_GRACE_SECONDS (one hour
by default). Deleting files retain their quota through that writer horizon.
See [the file contract](../docs/behavior/files-and-search.md).
