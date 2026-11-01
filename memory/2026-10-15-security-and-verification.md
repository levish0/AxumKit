# 2026-10-15 — Authentication, retention, and runtime verification

## Changes

- `ApiJson` and `ApiQuery` require `validator::Validate`. HTTP DTOs reuse pure
  identity, organization, and file value rules; services repeat them for non-HTTP
  callers. Validation errors never expose submitted values. OpenAPI is regenerated.
- Global administrators require confirmed TOTP at primary login, session issuance,
  session authentication, and final service authorization. The operator-only
  `manage set-administrator <uuid> <true|false>` changes roles under account lock,
  revokes credentials, and records an audit event. Administrators cannot remove MFA.
- `POST /api/v1/account/recovery-codes` requires recent authentication and a fresh
  factor proof, replaces all ten codes, and revokes every session and challenge.
  Version advancement preserves the consumed TOTP step.
- Authentication budgets are configurable: identity 10 and client 60 per minute
  by default, each supporting 1 through 100000. Shared counters saturate above
  every supported budget, independently of each instance's configured limit.
- Retention defaults are succeeded jobs 30 days, failed jobs 90 days, and audit
  events 180 days. Cleanup deletes bounded batches and rechecks mutable job state
  and completion time in the final DELETE. Two new SeaORM migrations add indexes.
- Deployment has a separate restricted maintenance environment/service and optional
  five-minute systemd cleanup/search-repair templates. No host timer is installed.
- Mixed k6 uses separate accounts and organizations against two API processes,
  a worker, SeaweedFS, and Meilisearch. It covers mutations, version conflicts,
  tenant isolation, session rotation/revocation, original transfers, and search.
  Any unexpected HTTP error, failed business check, or failed background attempt
  fails verification; successful worker retries cannot conceal a defect.
- `just test-production` verifies the release image using local HTTPS/STARTTLS,
  separate database credentials, actual privilege denials, PostgreSQL restoration,
  and restored encrypted MFA. Optional `SMTP_CA_CERTIFICATE` adds a PEM root while
  retaining hostname verification. CI runs the fixture against its built image.

## Decisions and constraints

- External password screening is excluded. A local common/breached-password list
  is deferred; length checks are not breach screening. Other confirmed requirements
  remain in [requirements](../docs/requirements.md).
- Mixed load exposed a real PostgreSQL deadlock: file cleanup's second update
  acquired parent foreign-key locks in conflict with HTTP account/organization
  `FOR UPDATE` locks. Canonical parent locks now use `FOR NO KEY UPDATE`, preserving
  business serialization while permitting foreign-key existence checks. A
  synchronized regression holds the worker's file lock, observes the real service
  waiting, then completes cleanup. No retries or relaxed thresholds mask this fix.
- Current parent writes do not change unique keys or delete parents. Future parent
  deletion or updates to foreign-key-eligible unique columns, including account
  email, require a new lock-order review. See
  [transaction boundaries](../docs/transaction-boundaries.md).
- The idle-session cleanup regression directly constructs the expired snapshot
  and uncommitted extension. It no longer requires fixture setup within two seconds.
- Runtime audit access stays append-only; maintenance adds audit deletion without
  schema ownership. Migration credentials never enter API/worker environments.
- File originals remain unchanged in private S3 storage. No content processing or
  scanning was introduced. Load settings retain four transfer slots per API.

## Validation

- Passed `just check`: formatting, architecture/local-link checks, Clippy with
  warnings denied, 20 unit tests, doc tests, and generated OpenAPI comparison.
- Passed `just test-files`: eight tests including the synchronized FK-lock
  regression, original/range handling, scope/quota races, cancellation, and cleanup.
- Passed `just test-db` after the lock and fixture changes: 15 identity and seven
  queue tests. An earlier rerun failed only the old two-second fixture precondition;
  that timing dependency was removed and the full suite rerun successfully.
- Deployment Compose validation and `actionlint` passed.
- Passed `just test-api`: seven HTTP and three local signed-OIDC tests. Passed
  `just test-process`: actual API/worker SMTP delivery, worker replacement,
  cross-process revocation, and captured-log secrecy.
- Passed `just template-check`: the copied and renamed project passed workspace
  unit/doc tests and all 15 identity/database tests.
- Built the canonical release container and passed the final production fixture:
  HTTPS/private-CA verification, Rust SMTP rejection of untrusted certificates and
  wrong hostnames, valid STARTTLS delivery, secure cookies, origin/proxy handling,
  restricted database roles, maintenance cleanup, and offline PostgreSQL restore
  followed by password/TOTP login and session revocation. Runtime processes had
  no privileged database credentials, used UID 10001, and had read-only filesystems.
- The first production fixture failed its readiness check because its HTTP helper
  sent the application Origin to Mailpit. Mailpit requests now omit that header.
  Cleanup explicitly enables all Compose profiles; the final successful run left
  no owned containers, networks, or volumes. Existing development services remained
  untouched.
- Before the lock fix, extended k6 reproduced PostgreSQL deadlocks, including an
  HTTP 503 and retryable worker failures. An intervening HTTP-green run did not
  establish worker correctness; the new attempt-history assertion closes that gap.
- Passed final post-fix k6 smoke: two users, 128 HTTP requests, 228/228 checks,
  four complete workflows, and two session revocations. HTTP p95 was 54.68 ms;
  search visibility p95 was 5.18 seconds.
- Passed final post-fix k6 extended: twenty users for two minutes, 13,971 HTTP
  requests, 25,022/25,022 checks, 480 complete workflows, and twenty revocations.
  HTTP p95 was 65.43 ms; search visibility p95 was 3.21 seconds. Both profiles had
  zero unexpected HTTP errors, workflow failures, or failed background attempts;
  file/job drain and captured-log secrecy checks passed. All owned load-test
  containers, volumes, networks, and application processes were removed.
- These are local verification results. Remote CI had not run for this milestone
  when this record was written; its result must be checked on the published commit.

## Remaining work

- Actual deployment, real SMTP/OIDC provider acceptance, and backup/restore of S3
  originals require the consuming project's environment. Local fixtures do not
  establish production capacity or provider delivery.
