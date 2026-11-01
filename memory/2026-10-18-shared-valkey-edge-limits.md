# 2026-10-18 — Shared Valkey edge limits

## Changes

- Added one private Valkey 9.1.2 counter store per k3s environment. Two APISIX
  replicas share one broad IP allowance through the Redis-compatible protocol.
- Added `gateway.env` with `EDGE_RATE_LIMIT_REQUESTS`. The example permits 60000
  requests per canonical client IP per 60-second fixed window. This is a starting
  abuse ceiling, not measured capacity.
- Matched backend Cloudflare/secret-authenticated SSR address selection in a
  gateway Lua module. Reject invalid or repeated selected IP headers. Ignore
  untrusted SSR overrides and Forwarded headers. Canonicalize IPv6 and mapped IPv4.
  A private request-context variable supplies the counter key.
- Added standard gateway errors with request IDs, exact-origin credentialed CORS,
  `Retry-After`, and no-store. Preserve backend quota errors and headers. Expose
  `Retry-After` for allowed-origin backend 429 responses.
- Excluded only exact GET/HEAD health routes from edge counting. Inject only the
  optional SSR secret key into gateways, with public origins and edge settings.
- Added singleton/Recreate validation, counter-first runtime rollout, coordinated
  drain preservation, shutdown handling, and gateway-only counter ingress.
- Extended real k3s acceptance with direct requests to both gateways, canonical
  identity and spoof tests, gateway restart continuity, counter timeout/recovery,
  retained PostgreSQL quota, counter replacement reset, and separate dev state.

## Decisions and constraints

- Valkey replaces the initially discussed Redis server. APISIX still uses
  `policy: redis` to select the compatible protocol. Application processes do
  not connect to Valkey. PostgreSQL owns authentication, revocation, strict API
  allocations, sessions, and jobs. No forward-auth is introduced.
- Edge counting fails open on Valkey errors. The configured socket timeout is
  200 ms per connect/send/read operation, not an end-to-end deadline. PostgreSQL
  enforcement still runs. Counter health does not gate API/gateway readiness.
- Valkey has one replica, `Recreate`, a 128 MiB data limit, `noeviction`, and no
  persistence or PVC. Replacement resets short edge windows. Independent replicas
  behind the Service would split authority; do not scale this Deployment or add
  an HPA. This baseline does not provide counter failover.
- Same-namespace gateway NetworkPolicy labels and namespace write permissions
  protect the authless counter. Keep all Services private and policy enforcement
  active. Namespace administrators and host-network traffic are outside this
  boundary. Dev and prod have separate stores.
- Counter configuration is separate from application Secrets and gateway settings.
  API/gateway changes and application drain retain the counter. `down` stops it.
- Read-only Astra review found no production correctness blocker. Its two
  findings were fixed: continuity checks use a fixture-only 600-second window,
  and timeout documentation does not promise a 200 ms end-to-end bound.

## Validation

- Local `cargo fmt --all -- --check`, xtask Clippy with warnings denied, and
  `cargo xtask test-deployment` passed. Deployment checks cover rendered dev/prod
  isolation, database roles, selective gateway Secret access, singleton Valkey,
  edge-setting validation, and replacement of only affected Pods.
- Python fixture syntax, documentation file links, and `git diff --check` passed.
- Two real APISIX 3.19 containers and Valkey 9.1.2 passed the full edge identity,
  combined allowance, spoof/duplicate, CORS, typed error, and health checks.
  Gateway replacement preserved counters. That fast smoke used a recording
  upstream; it did not establish backend authorization behavior.
  A second smoke run confirmed zero stored keys after Valkey replacement and
  resumed shared enforcement after stale gateway sockets reconnected.
- Local real two-node k3s verified two API Pods on separate nodes, signup/mail,
  CF/SSR forwarding, retained sessions/strict quota, and coordinated drain.
  API replacement handled 758 authenticated requests without errors; gateway
  configuration replacement handled 1890 without errors.
- That fixture passed combined gateway allowance, canonical/spoof/duplicate
  identity, CORS/errors/health, gateway restart continuity, Valkey timeout and
  recovery with PostgreSQL quota still enforced, intentional counter reset,
  and unchanged dev sentinel/counter state.
- The complete local command failed at private ingress denial. This managed
  kernel lacks `xt_physdev`; both node policy controllers reported aborted
  iptables synchronization. This is not a full local acceptance pass. Policies
  remain enabled. The remote run below completed policy enforcement and the
  final fixture-only timing/reconnection checks.
- Remote [CI run 39101200034](https://github.com/HiruneWorks/hirune-axum-kit/actions/runs/39101200034)
  passed `verify` and `container` for functional commit
  `2dccd6bcf1e8826267330d8c73799dd94ae33048`. Later changes are documentation only.
  It passed workspace checks, PostgreSQL/API/process/files/content suites,
  deployment contracts, k6 smoke, and copied-template acceptance.
- That CI built the canonical release image and passed production acceptance:
  HTTPS, STARTTLS, cookies/origins, restricted database roles, backup/restore,
  and encrypted MFA recovery.
- CI two-node k3s acceptance passed with two API Pods on different nodes and
  two real APISIX gateways sharing Valkey. API replacement handled 700
  authenticated requests without errors; gateway configuration replacement
  handled 1713 without errors. Sessions and strict quota survived rolling and
  drained replacement. Direct gateway tests passed IP/SSR/spoof/duplicate rules,
  CORS/errors/health, gateway replacement continuity, counter timeout/recovery
  with strict PostgreSQL quota intact, and counter reset/reconnection.
- CI retained independent dev counter state and its sentinel Pods. Tunnel-labelled
  Pods could reach the gateway but could not bypass it to the API or Valkey.
  Untrusted Pods could not reach any of those services. A dev tunnel-labelled Pod
  could not reach prod gateway, API, or Valkey.

## Remaining work

Target infrastructure still needs real Tunnel/provider checks, monitoring,
database recovery, and capacity measurements. No live production environment
was deployed. Valkey failover and production capacity were not tested.
