# k3s deployment

The production path is Cloudflare Tunnel → private APISIX → API Service → API
Pods. APISIX uses standalone YAML mode without etcd, an Admin API, or an ingress
controller. The gateway and API use only private ClusterIP Services. New backend
endpoints use the catch-all route. Add a controller when independently deployed
services need dynamic route ownership.

`base/` contains namespace-neutral runtime manifests and gateway configuration.
Ignored `.envs/.dev/k3s/` and `.envs/.prod/k3s/` overlays select images, namespaces,
settings, and generated Secrets. Follow [deployment](../README.md) to copy the
examples and use the canonical `deploy/justfile` commands.

## Prepare the cluster

Install k3s on your nodes. The acceptance fixture pins `v1.37.1+k3s1`.
On every server, merge these settings into `/etc/rancher/k3s/config.yaml`, then
restart k3s. Use the same settings when installing a new server.

```yaml
disable:
  - traefik
  - servicelb
```

Keep CoreDNS and the embedded network-policy controller. Keep metrics-server if
you enable HPA. Use at least two schedulable nodes with room for extra API,
worker, maintenance, and gateway Pods during rollout. Topology spread rules
prefer different nodes. PodDisruptionBudgets preserve one ready replica during
voluntary eviction. Provision control-plane availability separately.

Restrict namespace write access. The ingress policies allow only same-namespace
Pods with the expected labels: tunnel → gateway, gateway → API, and gateway →
Valkey. Valkey has no public listener or password; these namespace policies and
namespace write permissions establish its access boundary. A Pod in dev
cannot enter prod through these rules, even with the same labels. Anyone who can
create Pods inside the target namespace can impersonate these labels. Host-network
traffic and cluster administrators are outside this boundary. Do not add public
Services or ports for the gateway or API.

## Configure ingress and credentials

1. Publish the canonical image built from `compose/production/app/Dockerfile`.
   Set each overlay's image to the intended immutable tag or digest. Add an image
   pull secret if the registry requires one.
2. Set public HTTPS origins, SMTP, and runtime settings in the environment's
   `app.env`. Supply independent encryption and rate-limit keys in `secrets.env`.
   Set `INTERNAL_PROXY_SECRET` only when a trusted SSR server needs client-IP
   overrides. JSON values in Kubernetes env files have no outer dotenv quotes.
3. Provision separate runtime, maintenance, and schema-owner database roles.
   Set their URLs in the corresponding env files. The production profile uses
   external PostgreSQL. The [role contract](../README.md#database-roles-and-maintenance)
   defines the required permissions.
4. Create a remotely managed Cloudflare Tunnel for each environment. For namespace
   `hirune-prod`, route its public hostname to
   `http://hirune-apisix.hirune-prod.svc.cluster.local:9080`. Set that Tunnel's
   token in prod's `tunnel.env`. Dev uses its own hostname, namespace, and Tunnel.
   Both cloudflared replicas in one environment connect to the same Tunnel.
5. Verify that Cloudflare supplies `CF-Connecting-IP` and cannot reach a separate
   public origin. APISIX preserves it and the optional SSR override headers.
   PostgreSQL owns authentication and business API counters. APISIX uses Valkey
   for an additional broad IP limit. It does not run forward-auth.

APISIX obtains its namespace through the Kubernetes Downward API. Its route uses
`${{POD_NAMESPACE}}` in the API and Valkey Service DNS names. No Pod IP or fixed namespace is
stored in the gateway route. Increasing API replicas does not change Tunnel or
gateway configuration.

## Shared edge limits

Each environment runs two APISIX replicas and one Valkey 9.1.2 Pod. Both gateways
use the same counter store. The baseline has nine runtime Pods: two API, two
APISIX, two cloudflared, one worker, one maintenance, and one Valkey. The migration
Job runs separately. PostgreSQL and optional search/object storage stay external.

Set `EDGE_RATE_LIMIT_REQUESTS` in `gateway.env`. The example allows 60000 requests
per client IP per 60-second fixed window across both gateways. This is an initial
abuse ceiling, not measured service capacity. Tune it for real traffic and shared
NAT users. It covers all routes except exact GET/HEAD `/health/live` and
`/health/ready`. Authentication routes still enforce their lower PostgreSQL limits.
External API allocations stay strict and independent of this edge counter.

APISIX uses `limit-count` with `policy: redis` because Valkey speaks that protocol.
No Redis server or application Valkey client is installed. The private counter key
uses the same Cloudflare/secret-authenticated SSR IP selection as the backend.
Equivalent IPv6 forms and IPv4-mapped IPv6 share a budget. Forwarded headers do
not establish identity. Gateway-generated 429 responses use `edge_rate_limited`,
the common JSON error envelope, a generated request ID, `Retry-After`, and allowed
origin CORS. Backend 429 responses retain their own body and quota headers.

The gateway sets a 200 ms socket timeout for Valkey connect/send/read operations.
This is not an end-to-end request deadline. Counter failure or memory exhaustion
lets the request continue to the backend. Monitor Valkey health,
memory, APISIX counter errors, and rejection volume. PostgreSQL authentication
limits and API allocations still apply during a Valkey outage. Counter failure
does not make API or gateway readiness depend on Valkey.

Valkey has a 128 MiB data limit, `noeviction`, no disk persistence, and no PVC.
Its single-replica `Recreate` strategy prevents independent stores from splitting
one budget. Do not scale it or attach an HPA. Counter replacement intentionally
resets only the short edge windows. API/gateway rollouts and application drain
keep it running. This baseline does not provide Valkey failover. If the service
later needs highly available edge enforcement, use one replicated Valkey service
with coordinated primary failover and adapt the gateway connection configuration.

## Migration and rollout

From `deploy/`, run `just check prod`, then `just up prod`.
All resources are rendered and validated once before any cluster mutation.
`auto` is the default. A preflight Job uses the new image's
`migration --check-pending` command in a read-only PostgreSQL transaction. It
checks migration history without creating the history table. Missing history
means all compiled migrations are pending. Unknown versions and gaps fail the
check. A failed or invalid check stops deployment before runtime changes.

| Condition | Automatic deployment |
| --- | --- |
| Pending migrations, active maintenance, or stopped application Deployments | Maintenance and coordinated drain |
| Current schema, maintenance off, and running application Deployments | Rolling update |

Every pending migration selects an interruption, including additive changes.
This policy does not classify SQL compatibility. A preflight success is not a
migration dry run or proof that a migration will succeed. The migration Job
changes the real database after applications have stopped.

Completed check and migration Jobs are replaced on the next release. An
unfinished Job blocks another deployment, including pending or suspended Jobs.
A failed migration blocks runtime rollout. Inspect it with the selected target:

```sh
kubectl --context k3s-prod --namespace hirune-prod logs job/hirune-migration
```

For a planned interruption, the helper enables APISIX maintenance, upgrades old
gateways if needed, and verifies actual maintenance responses from every live
gateway Pod. It then stops API, worker, and periodic-maintenance Pods and waits
for exit. After migration success, it restores runtime Deployments and verifies
backend readiness through every gateway before disabling maintenance. It checks
that all gateways observed the change. An unsuccessful off operation requests
maintenance again. Failures before off leave maintenance enabled.

APISIX returns `503` with `error.code: maintenance`, `Retry-After: 30`, a generated
request ID, `Cache-Control: no-store`, and credentialed CORS for allowed origins.
Valid browser preflights return local `204` with the backend's allowed methods
and headers. The subsequent application request receives the maintenance `503`.
The stable `hirune-gateway-maintenance` ConfigMap is operator state. Ordinary
manifest application does not reset it. Gateways read its projected directory
dynamically; missing or invalid state keeps application admission closed.
`/health/gateway` checks the gateway independently of API or database health, so
the APISIX Service can serve maintenance while APIs are absent. `/health/live`
and `/health/ready` retain their backend health behavior.

After a failure, correct the cause and repeat `just up prod`. Active maintenance
selects drain even if the migration already completed. New deployments and
`down` followed by `up` also keep admission closed until recovery completes.
`just down <environment>` stops Valkey and the other Deployments; it retains the
namespace, settings, Secrets, maintenance state, and external data.

An explicit `just up prod rolling` rejects pending migrations, active
maintenance, and stopped applications. During a rolling update, schema, settings,
HTTP, and job payloads must work with old and new processes at the same time.

The API uses `maxUnavailable: 0` and `maxSurge: 1`. HTTP readiness gates Service
traffic. TCP liveness avoids restarting healthy processes during database failure
or HTTP admission pressure. A five-second pre-stop delay gives Service endpoints
time to converge. The API then drains for at most 30 seconds. The 45-second
termination grace also covers the default 35-second worker drain. Increase grace
when increasing the worker drain; pre-stop consumes that budget.

For an incompatible job or configuration change without a migration, force an
interruption:

```sh
just up prod drain
```

APISIX, Tunnel connectors, and Valkey keep running during application drain.
HPAs remain installed. The helper records each autoscaled Deployment's positive
replica count in its `hirune.axum-kit/resume-replicas` annotation before stopping
it. With `minReplicas > 0`, Kubernetes deactivates HPA adjustment when the target
replica count is zero. The helper verifies `ScalingActive=False` with reason
`ScalingDisabled` before migration. After migration success, it restores a
positive count within the current HPA minimum and maximum. The HPA then resumes
normal control. Failed migrations keep applications stopped and maintenance on;
retry and repeated `down` retain the saved count. No HPA deletion or manual
restoration is required. A per-Deployment
`Recreate` strategy does not coordinate all application roles.

The helper inspects both live and rendered HPAs for its Deployments. Their
minimum must be positive; scale-to-zero HPAs cannot use this native maintenance
mechanism. Unrelated HPAs are left alone. See the
[Kubernetes maintenance behavior](https://kubernetes.io/docs/tasks/run-application/horizontal-pod-autoscale/#implicit-maintenance-mode-deactivation).

Recovery overrides are explicit:

```sh
just maintenance prod status
just maintenance prod on
just maintenance prod off
```

`off` requires backend readiness through every live gateway. Use normal `up`
for release recovery so worker and scheduler rollout also complete before off.

Serialize release commands. This helper is not a distributed deployment lock.
Do not deploy with a plain `kubectl apply -k`: it does not wait for migration and
cannot replace a completed Job's immutable template. A plain `rollout undo` does
not restore database schema or configuration. Recover with an explicitly
compatible release and the required retained encryption keys.

## Configuration, scaling, and storage

Kustomize generates hashed ConfigMaps and Secrets. A value change updates the
Pod template reference and replaces affected Pods. Schema-owner credential changes
replace only the migration reference. Application-secret changes replace API,
worker, maintenance, and gateway Pods because gateways reference the optional SSR
secret key. Gateways receive no database, SMTP, or encryption credentials.
Edge-limit changes replace only gateways. Valkey config changes replace only the
counter Pod. Tunnel-token changes replace cloudflared Pods.
Old generated objects remain for existing Pods and recovery. Remove unused objects
only after checking live Pods and retained ReplicaSets. Do not prune them blindly.

APISIX mounts configuration files read-only. Paths for its generated configuration,
PID, logs, and temporary data remain writable as required by the standard image.
It does not retry upstream requests automatically, because retries can duplicate
admission. Its 128 MiB body limit and 330-second timeouts cover the default
100 MiB/300-second file transfers. Keep these limits and the Cloudflare plan
aligned with application upload settings.

API, worker, APISIX, and cloudflared replica counts are independent. The optional
`autoscaling.yaml` uses CPU to scale API replicas from two to four. Copy it into an
environment overlay and add it to that overlay's `resources`. Keep metrics-server
enabled. The deployment helper omits `spec.replicas` for HPA-controlled
Deployments. When enabling HPA, it first removes that field from the existing
client-side last-applied annotation without changing the live count. This follows
the [Kubernetes field-ownership migration steps](https://kubernetes.io/docs/tasks/run-application/horizontal-pod-autoscale/#migrating-deployments-and-statefulsets-to-horizontal-autoscaling).
Rolling updates preserve the current scale. Recovery from zero uses the saved
count, or the HPA minimum for a new Deployment. An overlay may also omit replicas.
Removing a resource from an overlay does not delete it from Kubernetes;
delete the HPA explicitly only when permanently disabling automatic scaling.

With pools of 10, two API Pods plus one API surge, one worker, and one maintenance
process need 50 database connections. At the HPA maximum, budget at least 70,
plus migration, management, worker/maintenance surges, and reserve. Check complete
concurrent rollout capacity before increasing replicas or pool sizes. Manual
scaling for Deployments without HPA must also be reflected in the overlay,
because deployment reapplies their desired replica counts.

API Pods contain no authoritative local data. PostgreSQL, SMTP, optional object
storage, and optional search are external. Kubernetes does not provision their
persistence or backups through these manifests. The optional [single-host data
profile](../data-services/README.md) supplies Docker Compose PostgreSQL/Meilisearch and
pgBackRest backups outside k3s on the same machine. Use read-only mounts for private
SMTP CAs or provider credential files when needed. Enable Kubernetes Secret
encryption at rest and restrict Secret read access. Coordinate key rotation across
all processes and retain keys required by stored encrypted data.

## Acceptance

On a Linux Docker host with API 1.48 or newer, install `k3d v5.9.0` and
`kubectl v1.37.1`, build the
canonical release image, and reserve at least 20 GiB for native snapshots:

```sh
K3S_TEST_IMAGE=hirune-kit:local just test-k3s
```

The Rust fixture creates and removes its own two-node k3d cluster. It uses a native
container snapshotter, Flannel host-gw, and a null kernel-message device for nested
hosts. Image export selects the daemon's native Linux platform. Cached image
indexes must not import references to absent architecture variants.
Network-policy enforcement stays enabled. It never changes the user's
kubeconfig. The isolated kubeconfig has an intentionally unusable current context;
operations must use the explicitly selected context.

The fixture deploys a disposable prod overlay with two API Pods on distinct nodes.
A fixture-only topology patch requires each new API revision to span these two
nodes. Terminating old Pods cannot skew placement of the new revision.
Production uses preferred spreading. The fixture also deploys two APISIX Pods,
Valkey, PostgreSQL, Mailpit, SeaweedFS, Meilisearch, a separate storage ingress,
probes, and a k6 Job. Real cloudflared replicas are zero. Local HTTP/mail are
fixture-only substitutions. Separate schema-owner, runtime, and maintenance
database roles use the real migration grants. The separate production fixture
verifies HTTPS, STARTTLS, and backup/restore.

The deployed workflow creates users through signup and worker mail, then verifies
organizations, sessions, external API access, presigned transfers, attached posts,
comments, profiles, notifications, and search. The client Job uses APISIX for API
requests and a separate Caddy endpoint for S3 bytes. It has no storage credentials.
Caddy preserves the signed Host and enforces a 30-second total body-read timeout.
Private provider ports reject direct client access. Retained originals and post
attachments remain readable after API replacement and coordinated maintenance.
Deletion denies new URLs and removes bytes through the worker. Quota-release and
late-upload horizons remain focused provider tests without database time changes
in the deployed fixture. `just load-test` runs this same acceptance command;
`K6_PROFILE=extended` selects twenty users for two minutes. Native process load is
available only as the optional `just load-test-local` diagnostic.

Acceptance covers an unfinished migration guard, automatic maintenance for a
fresh schema, failed deployment after migration with automatic recovery, failed
preflight preserving running APIs, and cold-start recovery with current schema.
It enables metrics-server and a real HPA controller. It verifies replica ownership
transfer from two configured replicas to three autoscaled replicas, preservation
during rolling traffic, native HPA deactivation across controller reconciliations,
failed migration and retry, repeated down/up, and resumed HPA scaling. The fixture
uses equal minimum and maximum bounds of three for deterministic capacity checks;
the example's production bounds remain two to four.
It checks typed maintenance through the Service while APIs are absent, signup
and mail, retained
sessions and API quota, continuous authenticated traffic during API/gateway
replacement, config-only gateway rollout, coordinated application drain, and
private ingress. Direct requests to both gateways verify one combined IP budget,
header spoofing, SSR trust, duplicate rejection, IP normalization, CORS, health
exceptions, gateway restart continuity, counter timeout/recovery, retained strict
PostgreSQL quota, and intentional reset on Valkey replacement. A same-name dev
sentinel and a separate dev Valkey retain their Pods and state across prod
operations. A dev Pod with tunnel labels must fail to reach prod gateway, API,
and Valkey. This does not establish real Tunnel connectivity,
control-plane failover, long-upload drain, or production capacity.
The continuity checks use a fixture-only 600-second window so slow rollouts do
not expire their counters. Production keeps its 60-second window.

The host kernel needs bridge filtering, IP sets, and `xt_physdev`. CI loads these
modules on its disposable Ubuntu runner. Missing modules can leave running Pods
with failed policy installation. The acceptance command must fail in that case;
its failure output includes policy-controller errors.

Review pinned k3s, k3d, kubectl, APISIX, Valkey, and cloudflared versions together. Verify
real-hostname identity headers, mail, providers, backups, recovery, and node drain
on the target cluster. See [development](../../docs/development.md) and actual
[milestones](../../memory/README.md) for validation evidence.
