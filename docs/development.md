# Development

Use Rust 1.98.1, just 1.52 or newer, Docker, and Docker Compose. Commit Cargo.lock.
On Windows, Rust MSVC builds require the Visual Studio C++ build tools. All
development commands use Rust xtask or call executables directly. They do not
require a Bash-only test wrapper. Start by reading [conventions](conventions.md).

GitHub Actions use version tags or maintained channels. The
[Dependabot configuration](../.github/dependabot.yml) checks Cargo, Actions,
Dockerfile images, and local Compose images weekly. Review toolchain changes
across `rust-toolchain.toml`, CI, the Dockerfile, and these instructions together.
See the [GitHub configuration reference](https://docs.github.com/en/code-security/reference/supply-chain-security/dependabot-options-reference).

| Command | Purpose |
| --- | --- |
| `just init` | Create `.envs/.local/app.env` with independent random local secrets; fail if it already exists. |
| `just infra-up` / `just infra-down` | Start/stop development PostgreSQL and Mailpit; preserve development data. |
| `just infra-up-files` | Start PostgreSQL, Mailpit, SeaweedFS, and Meilisearch with local-only credentials. |
| `just migrate` | Apply SeaORM migrations under a PostgreSQL advisory transaction lock. |
| `just api` / `just worker` | Start the actual API/worker executable. |
| `just maintenance` | Start the portable periodic scheduler/worker; production supplies separate maintenance-role credentials. |
| `just manage cleanup` | Remove bounded expired authentication/audit/finished-job batches and reconcile files; production uses the maintenance role through `deploy/justfile cleanup`. |
| `just manage set-administrator <uuid> <true\|false>` | Operator-only role change; promotion requires enrolled MFA and changes revoke existing credentials. |
| `just manage set-api-client-quota <uuid> <per-minute> <per-month>` | Operator-only request allocation shared by all keys and API replicas; changing limits preserves usage. |
| `just manage retry-job <uuid>` | Retry a retained failed job without erasing its attempt history. |
| `just manage initialize-integrations` | Explicitly create the configured private bucket and filename/post/profile indexes/settings; safe to repeat. |
| `just manage repair-search` | Queue up to 100 current records per configured filename/post/profile index for reconciliation. |
| `just fmt` | Format the workspace. |
| `just check` | Formatting, architecture boundaries, local documentation links, Clippy with warnings denied, unit/doc tests, Turnstile verifier tests against a local fake server, OpenAPI drift. |
| `just test-db` | Real PostgreSQL identity, queue, and schema-consistency tests, including transaction/concurrency failures, immediate slot refill across 1/2/4 worker pools, empty-queue polling bounds, and entity/enum drift detection. |
| `just test-job-load` | PostgreSQL queue load with 1/2/4 worker pools, transactional database effects, mixed bulk/notification work, retained history, and foreground database latency. Writes `target/job-load-report.json`. |
| `just test-api` | Production-router API and local signed OIDC tests; external API scope, shared quota concurrency, window rollover, authority races, transaction rollback, and retention; Turnstile-protected identity routes against a local fake siteverify server. |
| `just test-process` | Separate API/worker/maintenance processes, four mail purposes through Mailpit, worker replacement, shared revocation, periodic cleanup deduplication, and API-key allocation/restart behavior. |
| `just test-files` | Real SeaweedFS/Meilisearch, original-byte preservation, direct range downloads, tenant access, quota races, conditional PUT races, and late-upload deletion. |
| `just test-content` | Real PostgreSQL/SeaweedFS/Meilisearch content, profiles, Cedar, revision conflicts, scoped pagination/search, transactional inbox, session-bound push devices, and provider-independent push gates. |
| `just test-deployment` | Validate local Compose, isolated dev/prod Kubernetes overlays, database-role separation, and Secret-triggered rollouts using temporary examples. |
| `just test-production` | Build and run the release container with local TLS/STARTTLS fixtures, restricted PostgreSQL roles, and backup/restore checks. |
| `just test-data-services` | Actual PostgreSQL/pgBackRest and Meilisearch Compose services: restricted migration grants, encrypted full/differential backup, isolated latest/PITR recovery, live-volume rejection, and persistent state across replacement. Uses a disposable local repository. |
| `just test-k3s` | Canonical deployed E2E: two k3d nodes, release image, APISIX, restricted DB roles, worker/mail, direct presigned transfers, attached content, search, retained state across rollouts, k6, HPA, shared counters, and ingress policies. Requires `K3S_TEST_IMAGE`. |
| `cd deploy; just up dev` / `just up prod` | Deploy to an explicit Kubernetes target. Defaults to auto: pending migrations use maintenance and drain; other running releases roll. Use `drain` to force an interruption. |
| `just load-test` | Same deployed Kubernetes acceptance as `just test-k3s`, including the selected k6 profile. Requires `K3S_TEST_IMAGE`. |
| `just load-test-local` | Optional diagnostic: shared k6 workflows against two host API processes and local Compose providers. |
| `just openapi` | Regenerate the committed OpenAPI document. |
| `just template-check` | Copy/rename the source into a temporary directory and run its workspace and identity/database tests. |

## Database isolation

Integration commands create a unique Compose project with dynamic local ports,
then remove only that project's containers and volumes. Each test creates its own
UUID-named database and applies migrations. Test cleanup drops only those generated
databases. Plain `cargo test --workspace` leaves infrastructure tests ignored.
The explicit integration commands select them with `--ignored`.

The database suite runs independent cases one at a time so queue timing checks
do not compete with other cases. Each concurrency scenario still creates its own
simultaneous actors and independent worker pools. Other integration commands
retain their existing case parallelism.

For an existing local disposable PostgreSQL server, set `TEST_DATABASE_URL`.
The role must be allowed to create and drop test databases. Non-loopback database
hosts are rejected. Process/load tests additionally require `TEST_SMTP_PORT` and
`TEST_MAILPIT_URL`. Do not point test administration at a production database.
File tests additionally require loopback `TEST_S3_ENDPOINT` and
`TEST_MEILISEARCH_URL` using the local Compose credentials. `just test-files`
starts these services automatically in its disposable Compose project.
Direct storage clients recheck each generated URL against the configured loopback
origin. They disable ambient proxies, redirects, and Referer generation. Localhost
is pinned to loopback. Only disposable credentials and fixture bytes use this
local HTTP boundary. Public deployments must use HTTPS for presigned URLs;
`S3_ALLOW_HTTP=true` is an explicit transport-policy override, not a production default.
Test databases from a panic may remain in external infrastructure. Inspect their
generated `kit_test_` names before explicit cleanup.

See [schema consistency](schema-verification.md) for the database comparison,
enum-column bindings, supported checks, and verification limits.

## Load evidence

The k6 entry point is `tests/load/mixed.js`. Shared HTTP assertions live in
`helpers.js`. Organization, session, file, external API, and content workflows
each have their own module. The Kubernetes fixture mounts these scripts read-only
into a k6 Job. It seeds accounts through signup, worker mail verification, and
login, then allocates client quotas through the real operator command. A temporary
Secret supplies client credentials. The Job receives no storage signing keys.
A credential-free init container first checks both ready gateway Pod addresses
and the storage proxy from the workload Pod. NetworkPolicy allow rules can reach
different nodes after that Pod starts; see [Pod lifecycle](https://kubernetes.io/docs/concepts/services-networking/network-policies/#pod-lifecycle).
Only then does k6 start its measured workflows. Initialization has a bounded retry
window and reports safe status/type counters. A failed gate fails the Job.

The default k6 profile uses two virtual users for ten seconds, each with its own
account and organizations. It exercises scoped reads and pagination, optimistic
updates, cross-organization denial, cross-instance session rotation/revocation,
and original-file upload/download/search/deletion. Each VU also has a separate
commenter account in its primary organization. It creates and updates published
posts plus a private draft, traverses user/post/comment/inbox cursors, reads
notifications, searches current post/profile revisions with numbered pages,
and deletes the content. The first published post attaches the uploaded original
and verifies attachment hydration. Two API keys per client also exercise external
organization and published-post routes through both gateway Pods and verify their
shared monthly counter. Gateway routing uses the real API Service.
These fixture clients receive explicit operator allocation, independent of the
ordinary session and file limits.

Search visibility waits are bounded. Expected rejection probes have explicit
allowed statuses. Unexpected HTTP failures and failed business checks fail the
run. The check also rejects p95 latency at or above two seconds. Thresholds require
`http_req_failed: rate==0`, `checks: rate==1`, and `workflow_failures: count==0`.
Every virtual user must complete a full workflow and revoke its separate session
once. Search visibility must remain below ten seconds for each workflow, with a
matching p95 threshold.
The Kubernetes fixture does not rewrite database deadlines. A separate retained
file/post fixture checks access after rolling replacement and maintenance, then
checks immediate API denial and worker removal of object bytes after deletion.
Focused provider tests cover late uploads, reservation horizons, and quota release.
The optional native diagnostic additionally inspects admission/job rows and
advances fixture writer deadlines to verify cleanup completion.

`K6_PROFILE=extended` uses twenty users for two minutes. Set this and
`K3S_TEST_IMAGE` before `just load-test` or `just test-k3s`. CI runs this fixture once;
the workflow dispatch input selects its
profile in CI. Thresholds are regression gates, not a production capacity promise.
Benchmark production builds, realistic data, distinct user populations, deployed
network paths, and intended concurrency before a capacity decision.

## PostgreSQL job load

Run `just test-job-load` to measure the existing PostgreSQL queue and worker
runtime. The command creates disposable local infrastructure unless
`TEST_DATABASE_URL` selects an existing local disposable server. Each scenario
uses a new migrated database, four producers, four execution slots per worker,
and one foreground database client. Scenarios use one, two, and four independent
worker pools. They do not start a new broker or change the application runtime.

| Profile | Jobs per scenario | Retained job rows per scenario |
| --- | --- | --- |
| `JOB_LOAD_PROFILE=smoke` (default) | 2,000 | 10,000 |
| `JOB_LOAD_PROFILE=extended` | 20,000 | 100,000 |

Every producer commits a source record and its job together. Each handler updates
that source record. One in sixteen registrations models a notification; the other
jobs model a bulk projection with a two-millisecond asynchronous delay. Both use
the same application queue. The test verifies every job, attempt, and database
effect. Missing effects, repeated effects, unexpected retries, worker failures,
and failure to drain within 180 seconds fail the scenario.

The JSON report records scenario completion throughput, registration latency,
separate bulk and notification registration-to-handler latency, and foreground
transaction latency before and during load. Latencies include p50, p95, p99, and
maximum values.
Registration-to-handler uses a shared monotonic clock and includes registration,
commit, claim, and dispatch. The foreground probe updates and reads its own row
in a transaction. It measures shared database pressure without intentional row
contention.

CI runs the selected profile and saves `job-load-report` as an artifact. Set
`JOB_LOAD_RELEASE=true` for an optimized test build. Compare measurements only
when the build mode and host resources match. The synthetic effects exclude HTTP,
SMTP, FCM, storage, and search-provider latency. These measurements do not replace
the deployed k6 workflow or establish maximum throughput or production capacity.
Retained history models row count, not aged-table dead tuples, vacuum work, or
historical attempt volume. Each foreground transaction has a five-second bound.
The test does not promise notification priority over bulk work. Use its separate
latency results to assess interference before changing queue policy.

## Production fixture scope

`just test-production` builds the canonical release image and runs it with
`APP_ENV=production` inside a uniquely named local Compose project. Set
`PRODUCTION_TEST_IMAGE` only when deliberately verifying an existing local image.
Cleanup preserves that image. The fixture uses an internal Docker network, an
ephemeral private CA, a Caddy HTTPS proxy, and Mailpit requiring
STARTTLS. Caddy uses the fixture's certificate and does not request external
certificates. It simulates the Cloudflare client-IP header at the private ingress.
Production uses Cloudflare Tunnel and APISIX. The fixture checks secure cookies,
origins, proxy header replacement, and actual
Rust-worker mail delivery, including rejection of an untrusted SMTP certificate
and a certificate whose hostname does not match. Runtime containers use a
non-root user and read-only filesystem.

The fixture provisions separate migration, runtime, and maintenance PostgreSQL
roles and exercises allowed and denied operations, including append-only API usage,
operator API allocation, external admission/exhaustion, and scheduled maintenance
with restricted credentials. It stops API, worker, and maintenance processes,
backs up PostgreSQL, recreates the database, restores the dump, and signs in with
the restored password and encrypted TOTP state using retained encryption keys.
It also checks that API credentials, exhausted allocation, and usage survive the
restore, then revokes the restored key through an authenticated owner session.
Only fixture containers, volumes, and generated images are removed. These checks
cover local release-image behavior. They do not publish or deploy the image,
verify a real SMTP/OIDC provider, or test S3 original-file backup and restoration.
Record actual execution results in [milestones](../memory/README.md). The presence
of this command or its CI step does not establish that it passed.

## Kubernetes fixture scope

`K3S_TEST_IMAGE=hirune-kit:local just test-k3s` needs Docker API 1.48 or newer,
k3d v5.9.0, and
kubectl v1.37.1. Build the canonical release image first. The fixture creates an
isolated two-node k3s cluster and removes it afterward. It applies the production
profile with local HTTP/mail substitutions and no real Tunnel connection.
It checks two API Pods on separate nodes, real APISIX routing, migration Job
sequencing, automatic maintenance/failure/retry and cold-start recovery,
failed preflight preserving running APIs, live API and config-only gateway rollouts, session/quota continuity,
two gateways sharing Valkey, counter timeout/recovery and reset, separate dev
counter state, and private ingress NetworkPolicies. Separate schema-owner,
runtime, and maintenance database roles enforce deployment grants. Real SeaweedFS
and Meilisearch serve the workflow fixtures. A separate Caddy storage ingress
preserves the signed Host and limits total request-body time to 30 seconds.
Presigned traffic goes directly to this ingress, outside APISIX. CORS, conditional
PUT, empty originals, GET/HEAD/range, authorization, attachments, deletion, and
retained reads are exercised. The k6 Job runs before edge-limit fault injection.
See the [k3s runbook](../deploy/k3s/README.md).
Run this in addition to production acceptance; each fixture verifies a different
deployment boundary. CI uses the same release image for both commands.

## Editing and diagnostics

Read a feature's behavior contract before changing service rules. Add tests at the
boundary affected: pure policy/input logic, database atomicity, HTTP extraction,
or actual process/external effects. Avoid tests that merely repeat a field setter.
Use axum-test on `build_router`. Do not create alternate test-only routes. Tests
may create persistence fixtures, but runtime requests use real authentication.

Do not suppress internal errors or broadly disable warnings. An infrastructure
failure is not a passing test. Record actual commands and limitations in a
milestone, separately from CI, release, deployment, and live provider acceptance.
OpenAPI consumers regenerate through their own pipelines. Never hand-edit a
generated client. Keep release metadata consistent when adding a release pipeline.
