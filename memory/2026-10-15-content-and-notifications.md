# 2026-10-15 — Connected content and typed HTTP structure

## Changes

- Added organization posts and single-level comments, explicit existing-account membership operations, profiles, scoped user/post search, and transactional comment notifications.
- Added eight SeaORM migrations, each with one owning table and UUID keys. Application writes generate UUIDv7 IDs.
- Added descending (created_at, id) cursor lists and bounded numbered search pages. Search returns current PostgreSQL-authorized revisions and honest total_in_window metadata.
- Shared the bounded Meilisearch transport across filename, post, and profile projections. Explicit initialization and periodic repair coordinate through src/integrations.rs.
- Added optional FCM HTTP v1 with lazy Google ADC, encrypted session-bound tokens, generation fencing, current visibility/read/preference gates, generic payloads, durable delivery IDs, retry deadlines, and bounded retention.
- Added the posts:read API scope and external published-post list using the existing shared key/quota admission contract.
- Organized all HTTP adapters under src/http/v1 by domain and named endpoint. Each operation owns exclusive DTOs, contracts.rs owns shared types, routes.rs owns URL/method composition, and application services remain unversioned.
- Split notification assembly/inbox/preferences/comment registration, post external reads, profile search-job registration, and owning comment/membership repository queries.
- Converted existing and new closed sets to enums: file/job/mail/push/post states, job attempt outcomes, challenge purposes, membership roles, API scopes, device platforms, cursor ordering, authentication-limit operations, Meilisearch task states, and application environment. SeaORM ActiveEnum preserves existing string columns and values.
- Added real infrastructure content tests, provider-retry crash regression, and modular k6 organization/session/file/external/content workflows against two API processes and a worker.
- Updated confirmed requirements, concise agent rules, architecture, behavior/regression contracts, examples, and generated OpenAPI.

## Decisions and constraints

- This remains a modular monolith with replaceable API/worker/maintenance processes and authoritative PostgreSQL state; no sticky routing or Redis dependency.
- Current organization membership gates content, user discovery, search hydration, and inbox visibility. Global administrators have no implicit organization access.
- File originals remain unchanged in private SeaweedFS/S3. Attachments retain their independent organization ACL, including when linked to a draft.
- PostPublication permits only draft/published input; stored PostState additionally represents a deleted tombstone. Unknown closed values fail instead of silently defaulting.
- Job kinds, queue names, OIDC provider names, and opaque OIDC state are open extension/protocol identifiers and remain validated strings.
- Search indexes are expendable projections. Version matching and current SQL scope prevent stale content or removed memberships from leaking through results or counts.
- FCM is disabled by default. In-app notifications work without Google credentials. Provider acceptance is at least once; clients deduplicate stable delivery UUIDs.
- Re-registering the same live token/account/session/platform preserves device generation. Ownership/session/platform changes and expiry revival advance it.
- Physical file purge cascades attachment links only. Provider retry deadlines survive both ordinary completion and crashed-worker recovery.
- Work is on main as requested. No remote CI waiting.

## Validation

- Passed cargo check --locked --workspace --all-targets.
- Passed just check: formatting, crate/HTTP persistence boundaries, documentation links, Clippy with warnings denied, 29 workspace unit tests, doc tests, and generated OpenAPI drift.
- Passed just test-db: 16 identity and 10 jobs tests, including shared authority races and provider retry deadlines across normal finish, crash recovery, and stale claims.
- Passed just test-api: 7 production-router, 9 API-access, and 4 signed local OIDC tests.
- Passed just test-content: 14 real PostgreSQL/SeaweedFS/Meilisearch tests covering Cedar roles, draft/member isolation, optimistic and concurrent writes, timestamp-tie/scope-bound cursors, current-revision search counts, atomic inbox/push registration rollback, read/preferences/visibility, device renewal/transfer/logout, and physical attachment purge.
- Passed just test-process: 3 separate-process API/worker/maintenance, SMTP/Mailpit, credential revocation, restart, and external allocation tests.
- Passed just test-files: 8 real storage/search tests, including original bytes, ranges, tenant isolation, quota/transfer/cancellation, and cleanup races.
- Passed just load-test smoke: 2 virtual users, 4 full workflows, 454/454 checks, 274 HTTP requests with zero unexpected failures, HTTP p95 46.61 ms, filename visibility p95 5.66 s, and content/profile visibility p95 350.1 ms. Workflow admission history, background drain, and process-log secrecy also passed. The extended profile was not repeated; these are local regression gates, not production capacity evidence.
- Passed just test-deployment.
- Passed just test-production with a fresh canonical Linux release build: local HTTPS/STARTTLS, cookie/origin/proxy controls, restricted runtime/maintenance database permissions, API-key allocation/exhaustion, and PostgreSQL backup/restore with encrypted MFA and key/usage recovery. This did not publish or deploy the image.
- Passed just template-check: the copied/renamed service passed 29 workspace unit tests and all 16 real PostgreSQL identity tests using independently generated settings.
- Earlier checks found a route-literal replacement in signup/login, one redundant enum conversion, and one needless borrow; all were corrected before the successful runs above. Existing route tables match the pre-split URL snapshot, and generated OpenAPI removes no preexisting paths.
- The first expanded k6 run failed because its user-cursor assertion read id instead of the documented account_id. Correcting the workload preserved its strict error, latency, and authorization assertions; the rerun above passed.
- Astra read-only architecture/correctness reviews confirmed the attachment-cascade, crashed-retry-deadline, device-generation, and route fixes and found no remaining confirmed P1/P2 issue in the final enum review. Runtime/build evidence above was executed by the primary agent.
- No real FCM provider/device receipt, live external OIDC/SMTP provider acceptance, cloud deployment, or production capacity was verified. Remote CI was not awaited.

## Remaining work

- Consuming deployments must supply provider credentials/workload identity, edge protection, infrastructure, and capacity measurements.
- Real FCM receipt and client-side deduplication require a configured provider and client device. Optional integration configuration must agree across API and worker replicas.
- Full product-specific moderation, nested replies, invitations, billing, and cloud-cluster provisioning remain outside this example.
