# Runtime and deployment

`api`, `worker`, `maintenance`, `migration`, and `manage` are separate executables
from one source workspace. The API owns the HTTP lifecycle. The worker runs jobs
with bounded concurrency. Run migration and management commands explicitly.
`bootstrap(settings)` assembles shared dependencies. `build_router` constructs
the production router. `serve` accepts an existing listener and shutdown token.
Tests can create multiple independent applications without modifying global
environment or initializing global tracing.

## Shared authority

PostgreSQL owns sessions, security versions, challenges, OAuth flows, rate limits,
organizations, external API clients/keys/usage budgets, job leases, and delivery
state. API instances require no sticky routing. Add worker processes to increase
concurrency. Database locks and fencing remain authoritative across them.
In-process semaphores only bound local CPU/request capacity. Valkey, a broker,
and provider credentials are not application startup dependencies. The k3s
gateways use Valkey for a separate broad IP limit before requests reach the API.

Workers fill available execution slots without an inter-job delay. The
`WORKER_POLL_MILLISECONDS` setting (default 250) controls only the delay after
an empty queue lookup. Worker concurrency and database/provider limits bound
active work. This polling setting does not limit throughput.

Database connections have bounded connection/acquisition times and configured
pool size. Budget the sum across API and worker replicas against PostgreSQL's
connection limit. Run migrations first. Bootstrap checks migration history using
a read-only query. It rejects an outdated schema and does not apply DDL.
Use a schema-owner migration role and a runtime role restricted to the required
tables, sequences if introduced by dependencies, and migration-history SELECT.
Runtime audit and API-usage access is append-only: it must not update or delete
audit_events or api_usage. A separate maintenance role adds deletion on those
tables for retention. It retains the other permissions needed for cleanup but
must not have UPDATE or TRUNCATE privileges on those tables. It must not own the
schema or have DDL privileges. Kubernetes injects this role only into maintenance Pods. Provision and verify these database grants in the deployment.
Separate environment files alone do not enforce permissions. Development Compose
uses a privileged local role solely for convenience/testing.
The optional migration-only `MIGRATION_RUNTIME_ROLE` and
`MIGRATION_MAINTENANCE_ROLE` settings apply restricted grants after migration.
Set both or neither. The migration Job fails on existing DDL authority or unsafe
effective table privileges. See [self-hosted data](../deploy/data-services/README.md).

## Configuration

`.envs/.example/local/app.env` lists local settings. `just init` generates keys.
Never reuse public test fixtures. `Settings::from_values` validates explicit
configuration. Only executables read the injected environment. Root just recipes
load `.envs/.local/app.env`. Hosted Kubernetes overlays generate and inject explicit ConfigMaps and Secrets.
Production requires HTTPS application, origin, and provider URLs plus STARTTLS
SMTP. Cookie security follows APP_ENV.
Secrets use redacted Debug representations. Do not log complete settings.

Configure any number of named providers with the `OIDC_PROVIDERS` JSON array.
Each entry requires `name`, `issuer`, `client_id`, `client_secret`, and `redirect_url`.
The optional `native_client_ids` array enables native ID token login:

```json
[
  {
    "name": "google",
    "issuer": "https://accounts.google.com",
    "client_id": "replace-with-google-client-id",
    "client_secret": "replace-with-google-client-secret",
    "redirect_url": "https://app.example.com/api/v1/auth/oidc/google/callback",
    "native_client_ids": [
      "replace-with-google-ios-client-id",
      "replace-with-google-android-client-id"
    ]
  },
  {
    "name": "company",
    "issuer": "https://identity.example.com",
    "client_id": "replace-with-company-client-id",
    "client_secret": "replace-with-company-client-secret",
    "redirect_url": "https://app.example.com/api/v1/auth/oidc/company/callback"
  }
]
```

In `app.env`, encode this as one line inside single quotes:
`OIDC_PROVIDERS='[{...},{...}]'`, replacing the placeholders with complete objects.
Keep the JSON and its client secrets out of logs. Omitted, blank, or `[]` disables
OIDC. Provider names are unique, contain ASCII letters, digits, or hyphens, and
are at most 64 bytes. Empty credentials, unknown fields, duplicate names, and
invalid URLs fail configuration. Production issuer and callback URLs require
HTTPS. Configuration rejects the old individual `OIDC_*` provider settings.
Keep provider definitions and compatible secret keys consistent across API
replicas. Routes select the provider by its `name`. Identity ownership uses the
verified issuer and subject. This configuration supports OIDC providers.
OAuth-only integrations need their own protocol adapter.

List every client ID that a native SDK token can contain in `aud` or `azp`. The
provider `client_id` is always accepted and need not be listed. For example, the
Google Android SDK uses the web client as `aud` and the Android client as `azp`.
Sign in with Apple uses the app bundle ID as `aud`. Omit the field or use `[]` to
disable native login for a provider. Empty or duplicate native client IDs fail
configuration.

Configured providers must be reachable for initial discovery. SENTRY_DSN is optional.
SENTRY_SAMPLE_RATE controls performance sampling when enabled. The SMTP server
need not be reachable at API startup. Delivery failures follow the queue contract.

Session lifetimes are `SESSION_IDLE_SECONDS` (default 1209600, 14 days) and
`SESSION_ABSOLUTE_SECONDS` (default 7776000, 90 days). Idle expiry must be at
least 60 seconds. Absolute expiry must be at least the idle expiry. The same
values apply to cookie and bearer sessions. Lower both values for deployments
that need shorter unattended sessions.

Authentication budgets are `AUTH_IDENTITY_LIMIT_PER_MINUTE` (default 10 per
operation/identity) and `AUTH_CLIENT_LIMIT_PER_MINUTE` (default 60 shared across
authentication routes per client). Both accept 1 through 100000. Retention settings
are `SUCCEEDED_JOB_RETENTION_DAYS` (30), `FAILED_JOB_RETENTION_DAYS` (90), and
`AUDIT_RETENTION_DAYS` (180), `API_USAGE_RETENTION_DAYS` (90), and
`NOTIFICATION_RETENTION_DAYS` (90), each from 1
through 36500 days. Use the same API, worker, and management settings across
replicas. Changing a limit does not reset shared counters.

External API request allocation is stored per client in PostgreSQL rather than
replica environment variables. New clients receive zero allocation. A trusted
operator runs `manage set-api-client-quota <uuid> <per-minute> <per-month>`.
The quotas use fixed UTC minutes and UTC calendar months. See
[API access](behavior/api-access.md) for accounting and extension boundaries.

`MAINTENANCE_INTERVAL_SECONDS` defaults to 300 and supports 1 through 86400.
Run `maintenance` with the restricted maintenance database role. It schedules one
durable job per database-time interval bucket in the `maintenance` queue and
consumes that queue with the common job runtime. Normal workers consume only the
`application` queue. Multiple maintenance replicas use the same interval and
deduplicate registration in PostgreSQL. Each process runs one maintenance job at
a time. Retries or adjacent buckets may overlap across replicas. Cleanup and
search repair tolerate repeated execution. After a full outage, maintenance does
not backfill missing buckets. On restart, it resumes queued jobs and registers a
job for the current bucket. Retention and explicit manual retry cover both queues.

Production ingress uses Cloudflare Tunnel and APISIX. Configure exact
ALLOWED_ORIGINS and keep both the gateway and API private. The API reads
`CF-Connecting-IP` without a proxy IP allowlist. SSR servers can preserve the
original client IP with `X-Real-Client-IP` and the optional shared
`INTERNAL_PROXY_SECRET` (at least 32 visible ASCII characters). A missing or empty
secret disables only the SSR override. Keep the secret consistent across API
replicas and the SSR server. The API ignores `X-Forwarded-For` and rejects the
removed `TRUSTED_PROXY_IPS` setting. See [proxy trust](security.md#proxy-and-cache-trust)
for header validation and the required ingress boundary.

## Optional human verification

Set `TURNSTILE_SECRET_KEY` to enable Cloudflare Turnstile on signup,
verification resend, password-reset requests, and password login. Leave it
empty or unset to disable the check. The API then requires no Turnstile
configuration and does not contact Cloudflare. Keep the secret in secret
management. It is a server credential; only the sitekey belongs in the client.

| Setting | Default | Purpose |
| --- | --- | --- |
| `TURNSTILE_SECRET_KEY` | unset | Widget secret key. Its presence enables verification. |
| `TURNSTILE_EXPECTED_HOSTNAMES` | unset | Comma-separated hostnames that may serve the widget. Unset skips the hostname check. |
| `TURNSTILE_VERIFY_ACTION` | `true` | Requires the widget action to match the operation. |

The other Turnstile settings require the secret. The server always calls
`https://challenges.cloudflare.com/turnstile/v0/siteverify` with a five-second
attempt deadline. Use the same settings on all API replicas. Workers and
maintenance do not use them.

For local development, Cloudflare publishes test keys. The secret
`1x0000000000000000000000000000000AA` always passes,
`2x0000000000000000000000000000000AA` always fails, and
`3x0000000000000000000000000000000AA` reports a spent token. Use them with a
test sitekey such as `1x00000000000000000000AA` and the dummy token
`XXXX.DUMMY.TOKEN.XXXX`. The test responses do not carry your operation's action
or hostname, so set `TURNSTILE_VERIFY_ACTION=false` and leave
`TURNSTILE_EXPECTED_HOSTNAMES` unset with test keys. `APP_ENV=production`
rejects these test secrets. See
[Cloudflare testing](https://developers.cloudflare.com/turnstile/troubleshooting/testing/)
and [human verification](security.md#human-verification).

## Optional search and push

The shared `MEILISEARCH_URL` and `MEILISEARCH_API_KEY` enable derived filename,
post, and profile search. Each has a distinct index: `MEILISEARCH_INDEX` defaults
to files, `MEILISEARCH_POSTS_INDEX` to posts, and `MEILISEARCH_PROFILES_INDEX`
to profiles. Duplicate names fail configuration. Run
`manage initialize-integrations` after migrations. Startup never provisions
integrations. `manage repair-search` and scheduled maintenance share
one explicit coordinator and enqueue at most 100 rows per configured domain per
run. They prioritize rows that have never been reconciled, then the least recently
reconciled rows. Repeated repair eventually visits the current dataset.
Startup does not perform an implicit full rebuild.

`FCM_ENABLED=false` is the default. The inbox and comment transactions work
without Google credentials. Enabling FCM requires `FCM_PROJECT_ID`. Workers use
Application Default Credentials lazily for the Firebase Messaging OAuth scope.
Use workload identity in hosted deployments, or supply
`GOOGLE_APPLICATION_CREDENTIALS` and mount its service-account file read-only.
That path is a Google runtime setting, not an application secret copied into
the repository or image. A valid project setting alone does not verify provider
permissions or device receipt. See [notifications and push](behavior/notifications-and-push.md).

## Telemetry and shutdown

Executables initialize structured JSON tracing once. HTTP spans include a generated
request UUID, matched route pattern, method, status, and duration. Services skip
arguments in instrumentation. Job spans add job/attempt IDs and the originating
request ID. SMTP acceptance includes delivery ID. The request ID connects traces
across the database queue. W3C traceparent propagation is not implemented.

The optional Sentry `ErrorLayer` records `ERROR` events from owned application
targets. Its `SpanLayer` records performance spans from the same targets with
request-local hubs. Both layers exclude provider and library targets. The client
removes request and user payloads. HTTP instrumentation does not capture raw URLs,
headers, bodies, or SQL parameters. Review future fields for sensitive data. Do not
enable verbose third-party logging for production credentials.

SIGTERM/Ctrl-C changes readiness and begins bounded shutdown. Liveness reports
process availability. Readiness also requires the database and an accepting API.
The API drains for at most thirty seconds. Workers stop claiming and drain for
WORKER_DRAIN_SECONDS, then cancel unfinished tasks. Leases recover them later.
Run the explicit `maintenance` process for periodic cleanup and search repair,
using the restricted maintenance role. Deployment starts it alongside API and
worker. No host cron or systemd timer is required. `manage cleanup` remains an
explicit one-off command using the same cleanup implementation.
See [jobs and mail](behavior/jobs-and-mail.md).

## Container

Build with `docker build -f compose/production/app/Dockerfile -t hirune-kit .`.
The default entrypoint is the API.
Select `--entrypoint worker`, `--entrypoint migration`, or `--entrypoint manage`
for other roles. The image runs as a non-root user and contains no source keys.
Supply environment/secrets at deployment, set BIND_ADDRESS=0.0.0.0:3000 for the
API container, run migrations once before rollout, and arrange health probes and
termination grace periods. The development Compose file does not deploy this image.
See [environment layout](../.envs/README.md), [container files](../compose/README.md),
and [deployment commands](../deploy/README.md) for the canonical paths.
The [k3s profile](../deploy/k3s/README.md) supplies two API replicas, a private
APISIX Service, shared Valkey edge counters, Tunnel replicas, readiness, graceful rollout, migration sequencing,
and an optional HPA. Deployment defaults to `auto`: pending migrations select
verified gateway maintenance and coordinated drain before migration. Current
schema releases roll unless maintenance is active or applications are stopped.
Incompatible job or configuration changes without migrations require explicit
drain. HPAs with positive minimums remain installed during drain. The helper
verifies native deactivation at zero, retains recovery counts, and restores
positive counts after migration success. Rolling preserves HPA-owned replicas.
