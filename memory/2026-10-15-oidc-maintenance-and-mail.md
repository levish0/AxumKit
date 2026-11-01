# 2026-10-15 — Multiple providers, portable maintenance, and identity mail

## Changes

- `OIDC_PROVIDERS` accepts multiple explicitly named provider configurations.
  Individual legacy provider variables are rejected. Parsing validates unique
  names, credentials, URLs, and object fields without exposing secret values.
- `maintenance` is a portable scheduler/worker executable. Deployment starts it
  alongside API and worker using the restricted maintenance role. Host systemd
  timer templates were removed. One-off cleanup reuses the same coordinator.
- Jobs have an explicit queue and optional schedule key. Database-time buckets
  and a unique queue/key index deduplicate periodic registration; claims and
  expired-lease recovery select only their queue. Ordinary mail/file jobs always
  use the application queue, including those registered by maintenance.
- Identity mail has separate verification, reset-password, welcome, and
  password-changed MJML/plain-text templates. Welcome follows verified signup;
  password change/reset completion creates a security notification. Delivery
  registration is encrypted and atomic with the account change.
- New SeaORM migrations add job routing/scheduling metadata and permit tokenless
  notification deliveries. Applied migrations remain unchanged.

## Decisions and constraints

- This supersedes the optional systemd scheduling suggestion in the
  [earlier milestone](2026-10-15-security-and-verification.md). Periodic maintenance
  must run without an operating-system scheduler.
- Maintenance replicas share the same interval (default 300 seconds, range
  1–86400). Registration is once per database-time bucket; this does not prohibit
  overlapping executions from different buckets or retries. Operations remain
  bounded and repeatable. Missing buckets during an outage are not backfilled.
- The scheduler and consumer stop together on fatal errors. The shared job runtime
  owns leases, retries, and attempt history. History retention and explicit manual
  retry cover both queues. Queue routing does not replace database permissions.
- The single-host deployment drains the old API and workers before starting new
  producers. Old unfiltered workers must not consume the new maintenance queue or
  notification payloads. This upgrade does not claim zero downtime; rolling
  deployments require compatible versioned job and payload contracts.
- Welcome and password-change notifications contain no action token and expire
  after 24 hours. A pending password-change alert remains eligible after account
  deactivation. Welcome requires a verified active account; token mail retains
  live challenge/security-version checks.
- OIDC state, provider binding, sessions, security versions, and rate limits remain
  authoritative in PostgreSQL across replicas. Real provider credentials and
  consent/redirect registration remain deployment responsibilities.

## Validation

- Passed `cargo check --locked --workspace --all-targets` and `just check`, including
  formatting, architecture/document links, Clippy with warnings denied, 22 unit
  tests, and OpenAPI drift checking.
- Passed `just test-db`: 16 identity and 9 queue tests, including notifications,
  single-use verification, MFA, independent worker loops, retries, and scheduling.
- Passed `just test-api`: 7 HTTP and 4 signed OIDC-provider tests. Two providers
  and two application instances share callbacks/sessions while rejecting provider
  mismatch, replay, missing providers, and provider outages.
- Passed `just test-process`: two real maintenance processes plus API/worker
  replacement, all four mail purposes through Mailpit, and log secrecy checks.
- Passed `just test-files` (8 tests) and `just test-deployment`.
- Passed `just load-test` smoke: 2 virtual users completed 4 mixed workflows;
  234/234 checks passed, zero HTTP/workflow failures, HTTP p95 97.69 ms, search
  visibility p95 5.49 seconds. This is local regression evidence, not a capacity
  claim. The earlier milestone's extended run was not repeated.
- Passed `just template-check`: the copied/renamed service passed 22 unit tests
  and all 16 PostgreSQL identity tests; initialization preserved existing secrets.
- Passed `just test-production`: built the canonical Linux release image with
  the maintenance executable, verified HTTPS and STARTTLS trust/hostname rejection,
  cookies/origins, runtime and maintenance database permissions, scheduled cleanup
  under restricted credentials, non-root/read-only API and worker containers,
  PostgreSQL backup/restore, encrypted TOTP recovery, and session revocation.
- Source changes are split into OIDC (`a1ad234`), maintenance (`5ac95c5`), and mail
  (`6178d6a`) commits. All verification above ran locally; remote CI is not awaited.

## Remaining work

- Actual deployment, external-provider acceptance, and original-file backup/restore
  remain specific to the consuming project. Final remote CI is not awaited, as
  requested by the owner.
