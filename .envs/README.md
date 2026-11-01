# Environment files

Commit only `.example/` templates. Git ignores actual `.local/`, `.dev/`, and
`.prod/` settings. Image build contexts exclude these settings. Never commit
credentials or print rendered production Kubernetes resources.

- `just init` creates `.local/app.env` with fresh local keys.
- Root development commands load `.local/app.env`. Override with
  `just --dotenv-path <path>`.
- Hosted deployments copy `.example/dev/` to `.dev/` and `.example/prod/` to
  `.prod/`. Each has a `k3s/` overlay. `deploy/justfile` disables local dotenv
  loading and selects the named deployment environment explicitly.
- Tests receive explicit settings and never read developer secrets.

Each hosted overlay owns its target context/namespace, image, public runtime
settings, `gateway.env` for the edge IP limit, generated application/provider Secrets, three database-role env files,
and a Tunnel token. Keep databases, keys, origins, and tunnels separate between
dev and prod. Both hosted environments use `APP_ENV=production` for HTTPS rules.
See [deployment](../deploy/README.md) and the [k3s runbook](../deploy/k3s/README.md).
Each hosted environment also has an optional `data-services/` profile for self-hosted
PostgreSQL/Meilisearch and pgBackRest. Data and Kubernetes credentials must match,
but only PostgreSQL/backup containers receive bootstrap and backup secrets. See
the [data runbook](../deploy/data-services/README.md).

Kubernetes generator env files use raw values. Write `OIDC_PROVIDERS=[]` or raw
JSON with double quotes and no outer single quotes. These files do not expand
shell variables. Local `.local/app.env` uses dotenv syntax and may quote JSON.
Do not transfer dotenv quotes into Kubernetes values.

`runtime.env` supplies only the restricted runtime `DATABASE_URL`.
`maintenance.env` supplies its restricted cleanup `DATABASE_URL`.
`migration.env` supplies the schema-owner `MIGRATION_DATABASE_URL` and optional
`MIGRATION_RUNTIME_ROLE` / `MIGRATION_MAINTENANCE_ROLE` identifiers. Set both to
enable post-migration role grants, or leave both empty for external management.
Kustomize creates separate hashed Secrets and resolves their workload references.
API and worker Pods receive neither maintenance nor schema-owner credentials.
Provision and verify actual database grants separately. An env-file name does
not enforce PostgreSQL permissions.

Executables read injected environment variables only. Direct `cargo run` requires
explicit exports or `just`. Kubernetes injects the selected ConfigMaps and Secrets.
No executable searches for environment files during startup.

Production ingress requires Cloudflare Tunnel and private gateway/API listeners.
Set `INTERNAL_PROXY_SECRET` only when an SSR server forwards the original client
IP. Share the same random value (at least 32 visible ASCII characters) with that
server, gateways, and all API replicas. Gateways receive only this Secret key.
Empty disables SSR overrides. The API still reads
`CF-Connecting-IP`. Remove the old `TRUSTED_PROXY_IPS` setting. See
[proxy trust](../docs/security.md#proxy-and-cache-trust).

`MAINTENANCE_INTERVAL_SECONDS` controls periodic registration (300 seconds by
default). Keep it consistent across maintenance replicas. The same process
schedules and consumes the dedicated queue; no operating-system scheduler is needed.

Set authentication limits and retention periods in `app.env`. Use the same values
on all replicas. Defaults are 10 identity attempts and 60 client requests per
minute. Retain succeeded jobs for 30 days, failed jobs for 90 days, and audit
events for 180 days. Retain API admission history for 90 days through
`API_USAGE_RETENTION_DAYS`. PostgreSQL stores external API allocations per client.
Replica environment variables do not set these allocations.
See [runtime settings](../docs/runtime.md) for names and
supported ranges. There is no password-screening provider configuration.

`OIDC_PROVIDERS` is a JSON array of provider objects, with `[]` as the disabled
default. Use dotenv quotes in local `app.env`; use raw JSON in hosted `secrets.env`. Each configured provider has its own client secret. The
[runtime example](../docs/runtime.md) shows two providers; no separate per-provider
environment variable scheme is needed. Add the optional `native_client_ids` array
to a provider only when native applications sign in with its SDK.

`TURNSTILE_SECRET_KEY` enables optional Cloudflare Turnstile human verification.
Leave it empty to disable the check. Store the real secret only in `.<environment>/`
or secret management. Cloudflare's public test secrets belong only in local files;
production configuration rejects them. See [runtime settings](../docs/runtime.md).
