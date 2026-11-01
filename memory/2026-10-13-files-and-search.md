# 2026-10-13 — Original files and derived search

## Changes

- Added organization-scoped original attachments backed by private SeaweedFS S3
  storage and a derived Meilisearch filename index. Services own permissions,
  quota reservations, state transitions, and durable deletion/index jobs.
- Added a focused SeaORM files migration with UUIDv7 identities, explicit states,
  byte/version constraints, foreign keys, and indexes for scoped lists,
  reservation rates, and reconciliation.
- Added typed HTTP contracts, raw-byte uploads, streaming GET/HEAD with single
  byte ranges, generated OpenAPI, optional integration settings, Compose
  profiles, management commands, and a real integration test command in CI.
- Kept the owner's Argon2, TOTP, and OpenAPI dependency upgrades and adapted the
  implementations to their current APIs without changing password parameters.
- Documented current input extraction: ApiJson/ApiQuery/ApiPath deserialize
  concrete DTOs and map errors; services validate values and current authority.
  No automatic DTO validation trait was introduced.

## Decisions and constraints

- Originals remain unchanged. No baseline content sanitization, antivirus,
  media processor, image reencoding, or video inspection. A ready file means
  completed storage, not a content-safety verdict. Never execute or unpack it.
- Generated S3 keys are independent of display names. Original retrieval uses
  attachment disposition and application/octet-stream. Current membership is
  required; upload ownership never bypasses parent access.
- Account -> organization -> file locks serialize quota and final authority.
  Streaming/storage/provider calls occur outside locks. Finalization rechecks
  state, deadline, session, and membership; deletion wins a late upload race.
- Bounded temporary files permit one streamed PutObject per unique key. Transfer
  permits, independent total/idle deadlines, and bounded forwarding prevent
  slow or unpolled responses from keeping transfer capacity indefinitely.
- Defaults: 100 MiB/file, 1 GiB and 10,000 live files/organization, 100 pending
  reservations, 30 reservations/uploader/minute, and 30-day tombstones. Zero-byte
  originals are supported and count toward metadata/rate quotas.
- Database hydration and a final membership check filter search candidates.
  Retained tombstones support repeated deletion; a persisted search-deletion
  obligation survives temporarily disabling the integration. Bounded maintenance
  orders due cleanup fairly and purges only confirmed eligible tombstones.
- Integrations are optional and use explicit provisioning, not startup DDL.
  The API and worker remain instances of the same modular monolith.

## Validation

- `just check`: passed formatting, boundaries, documentation links, Clippy with
  warnings denied, 20 unit tests, doc tests, and OpenAPI checking.
- `just test-files`: passed seven real PostgreSQL/SeaweedFS/Meilisearch tests:
  byte preservation and HTTP ranges/headers; membership isolation; size/zero-byte
  handling; quota races; cancellation; late-upload deletion; stale search;
  persisted deletion obligations; rate limits; tombstone purge and cleanup fairness.
- A repeat run during parallel compilation failed with `integration unavailable`
  in the original-file router test. That run lacked provisioning context/provider
  logs, so the cause was not established. Added safe test context and owned-service
  failure diagnostics. A fresh isolated run with the final indexes passed all seven
  tests in 58.51 seconds; no timeout increase or retry workaround was added.
- `just test-db`: passed nine identity and six queue tests.
- `just test-api`: passed five production-router and three signed local OIDC tests.
- `just test-process`: passed the actual API/worker/Mailpit flow, replacement,
  shared revocation, ciphertext clearing, and log-secrecy checks.
- `just test-deployment`: passed Compose and runtime/migration environment checks,
  including the quoted temporary-filesystem configuration.
- `just load-test`: passed the local two-user, ten-second smoke profile with 216
  requests, zero failed requests/checks, and p95 154.92 ms. Concurrent copied-template
  compilation was active; this debug-build result does not establish capacity.
- `just template-check`: passed copying/renaming, secret initialization and overwrite
  refusal, 20 unit tests, and nine identity/database tests.
- Remote main CI succeeded on 78f600c before these changes. This milestone's new
  source had not yet been pushed or verified in remote CI when this note was written.

## Remaining work

- Integrate the owner's merged Dependabot updates without rewriting history,
  verify the combined revision, and push the completed checkpoint on main.
- Actual production-image runtime, restricted PostgreSQL role privileges,
  proxy/TLS and SMTP STARTTLS acceptance, backup/restore, and realistic load remain
  distinct from local configuration and test evidence.
- Product-specific password screening, administrator MFA, recovery-code
  regeneration, and configurable audit/failed-job retention remain separate
  decisions/work. External SMTP receipt and OIDC registration need real targets.
- Consumers must define parent permissions for message attachments, inline media
  behavior, file abuse policy, backup coverage, and recurring maintenance.
