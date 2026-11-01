# 2026-10-15 — External API keys and shared allocation

## Changes

- Added the api_access application domain with organization-owned clients,
  expiring hashed API keys, strict PostgreSQL request counters, and admission
  history. SeaORM migrations separately create clients, keys, usage, and
  resource identifiers on existing audit events. Applied migrations are unchanged.
- Added owner-session management routes and an explicitly scoped external
  organization read. Typed HTTP contracts, common errors, and generated OpenAPI
  describe the public behavior.
- Added the trusted set-api-client-quota management command. New clients have
  zero allocation; owners cannot grant themselves request budgets.
- Periodic cleanup now removes bounded old API-usage batches. Runtime usage
  access is append-only; maintenance additionally permits deletion, with no
  update or truncate privilege. API_USAGE_RETENTION_DAYS defaults to 90.
- Added PostgreSQL/HTTP regressions, separate-process replacement and operator
  command coverage, two-key k6 workflows, and restricted-role release-container
  checks including restored API credentials, allocation, history, and revocation.
- Documented the credential format and ownership/accounting contracts in
  docs/behavior/api-access.md, with supporting conventions and operator guidance.

## Decisions and constraints

- The owner selected API-key-based external access as the baseline. OAuth2 client
  credentials, an authorization server, subscription billing, and a developer
  portal are outside this implementation.
- A client is an organization principal, independent of its creator's account.
  Keys do not authenticate to account, administrator, or ordinary organization
  routes. The initial scope is organizations:read and its projection contains
  only the owning organization's UUID, name, and version.
- The project-defined key format is hak_v1_<simple UUIDv7>.<43-character secret>.
  The secret has 256 random bits; only a domain-separated BLAKE3 hash is stored.
  Complete keys are returned once. This string format is a convention, not a
  universal API-key standard. Both authentication transports share Bearer
  parsing; HTTP scheme names are case-insensitive.
- All keys and replicas share fixed UTC-minute and UTC-calendar-month counters
  on the client row. Rotation, scope changes, and allocation changes preserve
  usage. Admission locks client then key, reads database time after locks, repeats
  mutable checks, and atomically commits both counters and a UUIDv7 usage row.
  Incorrect secrets fail before waiting for authority locks.
- Each admitted HTTP attempt consumes one unit, even if response delivery later
  fails. Rejected authentication/scope/allocation requests consume none.
  Retries are new attempts; request IDs are correlation, not billing deduplication.
  HEAD on the external example is explicitly rejected without admission.
- Owner writes require recent full authentication and the existing Cedar owner
  policy. Human management locks account, organization, client, then key.
  External reads do not take a later organization lock. New suspension/deletion
  policy or external writes require another authority/lock-order review.
- Metadata caps count all registered clients/keys, including revoked/expired
  records: 100 clients per organization and 100 keys per client. Archival is
  not implemented. Retention removes at most 1000 old admission rows per run
  without changing active counters.
- PostgreSQL is the allocation authority. Edge protection remains a deployment
  concern; no APISIX, Redis, or cloud-provider dependency is introduced.
  Per-client serialization is deliberate; local tests do not establish capacity.

## Validation

- Passed just test-db: 16 identity and 9 job tests.
- Passed just test-api: 7 HTTP, 9 external API-access, and 4 signed OIDC tests.
  Coverage includes two application instances, 24 contenders for 5 remaining
  units, expiry/revocation/scope/allocation changes while blocked, independent
  persisted window rollover, recent owner authentication, creator independence,
  final metadata-cap races, rollback, and retention.
- Passed just test-files (8 tests) and just test-deployment.
- Passed just load-test smoke: 2 virtual users, 4 complete workflows, 248/248
  checks, 136 HTTP requests with zero failures, HTTP p95 61.2 ms, and search
  visibility p95 6.47 seconds. Admission records and process-log secrecy were
  checked after k6. This is local regression evidence, not production capacity.
  The extended profile was not repeated for this milestone.
- The initial k6 extension failed because its header lookup treated HTTP header
  names as case-sensitive. A case-insensitive lookup fixed the test; the request,
  allocation, accounting, and performance assertions remain unchanged.
- Passed just test-production using a fresh canonical Linux release build:
  HTTPS/STARTTLS trust and hostname checks, production cookies/origins/proxy
  behavior, operator allocation, lowercase Bearer-scheme interoperability,
  external admission and exhaustion under restricted runtime credentials,
  append-only audit/usage grants, periodic maintenance, and PostgreSQL
  backup/restore followed by restored key/usage checks and owner revocation.
  API and worker containers were non-root with read-only filesystems.
- An earlier release-fixture run used the source snapshot taken before the final
  shared Bearer-parser change and rejected the new lowercase-scheme probe.
  Rebuilding from the final source resolved that failure; the run above passed.
- Passed just check: formatting, architecture and documentation-link checks,
  Clippy with warnings denied, 24 unit tests, doc tests, and OpenAPI drift.
- Passed just template-check: the copied/renamed service passed 24 unit tests
  and all 16 PostgreSQL identity tests, using independently generated settings.
- Passed just test-process: 3 tests covering real API/worker/maintenance processes,
  operator CLI allocation, shared keys and quotas after API replacement, key
  revocation across processes, Mailpit delivery, and secret-free process logs.
- Astra's final read-only source/contract review reported no remaining actionable
  findings. Builds and runtime evidence above were executed locally by the main
  agent; remote CI was not awaited. The feature commit is bb965f3.

## Remaining work

- Deployment, edge protection, production capacity measurements, and consuming
  projects' additional external scopes/policies remain environment-specific.
- Remote CI is not awaited, as requested by the owner.
