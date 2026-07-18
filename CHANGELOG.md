# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.19.0] - 2026-07-18

Breaking: the RBAC surface drops its legacy `acl` naming (fresh migration set — reset
the database; API paths and error codes change).

### Changed

- Tables `acl_groups` / `acl_group_members` / `acl_group_permissions` are now
  `groups` / `group_members` / `group_permissions`.
- Admin endpoints move from `/v0/acl/*` to `/v0/groups`, `/v0/groups/delete`,
  `/v0/groups/members` (+`/remove`), `/v0/permissions`, and
  `/v0/groups/permissions` (+`/replace`).
- Wire codes: `acl:denied` → `permission:denied`, `acl:invalid_rule` →
  `permission:invalid`, `acl:group_*` → `group:*`; moderation log actions follow
  (`group:create`, `group:permissions_replace`, …).
- Matching type and function renames throughout (`Errors::PermissionDenied`,
  `GroupListResponse`, `service_create_group`, …).

### Removed

- The GitHub Pages docs workflow — the docs site deploys on Cloudflare.

## [0.18.2] - 2026-07-18

Tooling, tests, and documentation.

### Added

- **Black-box e2e harness + suites** — cookie-jar `TestClient` with per-actor IPs, Mailpit
  token polling, and direct-DB role bootstrap. Ten suites (59 tests) cover auth, TOTP
  (incl. concurrent single-use backup codes), account lifecycle, moderation, RBAC grants
  taking real effect, boards, notifications (incl. IDOR probes), search indexing, and
  public profiles — all green against the disposable docker stack.
- **CI split** — unit tests run in a parallel `test` workflow; `check` gates fmt, clippy
  `-D warnings`, OpenAPI schema drift, and fresh-database migrations; `e2e` keeps the
  docker stack. `just e2e` preserves the suite's exit status and caps parallelism.
- **Docs site** — lily-pad guide (EN/KO): getting started, architecture, configuration,
  authentication, authorization, boards & notifications, background jobs, testing,
  deployment.

### Fixed

- e2e defaults use `127.0.0.1` instead of `localhost` (which can resolve to `::1`,
  unanswered by Docker's port proxy on some hosts).
- Committed test/example env trees gained the missing `TOTP_ENCRYPTION_KEY` and worker
  frontend-path variables.

## [0.18.1] - 2026-07-18

### Changed

- **`search_index` crate** — the Meilisearch index uid and document schema are defined
  once and shared by the worker (writer) and server (reader), so schema drift becomes a
  compile error.
- Gateway auth-check identity carries `user_id` as a real `Uuid` (string only at the
  header boundary).

## [0.18.0] - 2026-07-18

### Added

- **In-app notification inbox** — one `notification_events` row per occurrence, one
  `notification_deliveries` row per recipient; target shape and action/type combinations
  are CHECK-constrained in the database. Producers (comment alerts, mentions) flow
  through a single chokepoint that drops self-notifications and honors per-action
  opt-outs. Cursor-paginated inbox with unread counts, read state, and preference
  endpoints; the shared `notification_repository` crate keeps server and worker writes
  identical. Old notifications are reclaimed by the weekly cleanup (90-day retention).

## [0.17.0] - 2026-07-18

### Added

- **Board domain (template demo feature)** — boards, posts, and comments (reply depth
  capped at 2) with owner-only edits, RBAC-gated pinning/locking/moderation, stale-set-
  rejecting pin reorder, `@handle` mentions, and per-viewer-deduped view counts buffered
  in Redis and flushed to Postgres by a per-minute worker cron. Content is stored raw.
  Three boards are seeded (`notice`, `general`, `qna`).
- Actor model (`actors`) for content attribution that survives account deactivation.

### Changed

- The weekly cleanup cron now performs real batched deletes: expired ACL group
  memberships, expired roles, expired bans, and old notifications.

## [0.16.0] - 2026-07-18

### Added

- **Django-style RBAC** — grantable permission codenames (`board:pin_post`,
  `board:lock_post`, `board:moderate`, `board:manage`) resolved by
  `UserContext::has_perm` in fixed order: ban hard gate → admin bypass → the Mod default
  set → the union of group-granted permissions. Denials return `acl:denied` with the
  missing codename. `acl_groups` / `acl_group_members` / `acl_group_permissions` tables
  with member reason/expiry metadata, plus an admin API (list/create/delete groups,
  member add/remove, whole-list permission replacement with codename validation).
  Stored codenames that no longer parse fail closed.

## [0.15.0] - 2026-07-18

Breaking: accounts now soft-delete; the users table drops `verified_at` and gains
`deleted_at` (fresh migration set — reset the database).

### Added

- `POST /v0/auth/set-initial-password` for OAuth-only accounts, Google One Tap nonce
  issuance + login, native-app mirrors for device verification and provider-token OAuth.
- User-image blob cleanup that only deletes unreferenced content-addressed assets.

### Changed

- **Account deletion is a soft delete with a PII scrub** — email is freed, credentials
  and images nulled, OAuth connections and roles removed; handle/display name stay
  reserved for attribution. Session resolution rejects deactivated users; public
  profiles mask them.
- `UserResponse.id` is a proper `Uuid`.
- Login timing is uniform across unknown/deactivated/OAuth-only/wrong-password paths;
  email lookups are case-insensitive at the database level.

### Fixed

- The media-processor client reads the current response headers, restoring profile- and
  banner-image uploads.

## [0.14.0] - 2026-07-18

### Changed

- **Error protocol** — per-domain wire codes (`"domain:snake_case"`) in a dedicated
  module; 4xx responses always include details, 5xx details are hidden outside
  development; structured `tracing` fields throughout.
- **Storage** — S3 operation/attempt timeouts so a stalled connection cannot hang a
  handler; auto-paginating prefix listing; private-bucket client with zstd content
  helpers.
- **Edge hardening** — production CORS panics when origins are unset and mirrors request
  headers (credential-safe); the anonymous cookie uses env-aware `__Host-`/`__Secure-`
  prefixes; internal-secret comparison is constant-time; Turnstile verification forwards
  the client IP.

## [0.13.0] - 2026-07-18

### Changed

- **`job_queue` crate** — job payloads, stream/subject/consumer names, and idempotent
  stream creation now live in one contract crate consumed by both binaries. The server
  no longer links the worker crate, and both declare the JetStream streams at startup so
  a fresh NATS works regardless of boot order.

## [0.12.1] - 2026-07-18

### Changed

- **Worker consumer engine hardening** — per-handler timeout backstop (nak on expiry,
  900s for reindex batches), panic isolation into the normal failure path, consumer
  create-or-update so tuning reaches existing durables, DLQ publishes confirmed before
  the original is dropped, and process-wide graceful shutdown that drains in-flight
  handlers. Dedup markers moved to the noeviction lock Redis.

## [0.12.0] - 2026-07-18

Breaking: database configuration is now a single URL.

### Changed

- **`DATABASE_URL`** replaces the assembled `POSTGRES_HOST/PORT/NAME/USER/PASSWORD`
  variables for the server, worker, and migration binaries alike — the deployment
  controls the full URL including the query string (`?sslmode=require`,
  `channel_binding` for managed providers). Logs redact credentials via
  `redact_database_url`, and config loading reports every missing variable in one
  message.

## [0.11.0] - 2026-07-05

Security-hardening release. A batch of auth/security features ported from the downstream
`V7` app, aligned with OWASP Top 10:2025 and ASVS 5.0.

### Added

- **`auth-core` crate** — project-agnostic cryptographic primitives (`aead`, `constant_time`,
  `keyed_hash`, `token`). Callers supply the key material and domain-separation `context`
  strings, so the primitives can be audited independently of the app. The app-layer adapters in
  `server/utils/crypto` own the `"axumkit …"` context strings.
- **New-device login verification (OWASP ASVS 6.3.5)** — a login from an unrecognized browser
  device is held and challenged by email; only a confirmed device is trusted and remembered via a
  long-lived, name-prefixed (`__Host-`/`__Secure-`) device cookie. New `known_devices` and
  `auth_events` tables, a device service (`resolve`/`confirm`), `POST /v0/auth/device/verify`, and
  `device_verification` / `security_alert` email templates. Integrated into both the password and
  the TOTP-verify login flows (native-app clients skip the browser-cookie check).
- **Authentication audit log (OWASP ASVS V16 / A09)** — private-tier `auth_events` table recording
  login success/failure, password change, and TOTP-disable, with actor IP + user-agent. Recording
  is best-effort so an audit-write failure never breaks the auth flow. No FK to `users`, so rows
  survive account deletion for forensics.
- **Security-alert emails** — the account owner is notified on sensitive changes (password changed,
  two-factor disabled).
- **Router-boundary RBAC gates** — `require_admin` / `require_mod` middleware make "which routes
  need which privilege" a single greppable property of the route table, so a handler can no longer
  expose a privileged endpoint by forgetting an in-service check.
- **Self-service account deletion (OWASP ASVS 7.5.1)** — `DELETE /v0/user/me` requires
  re-authentication (password, or a TOTP/backup code for OAuth-only 2FA accounts). OAuth-only
  accounts with no inline factor confirm via a single-use emailed token
  (`POST /v0/user/me/deletion/confirm`), backed by an `account_deletion` email template.

### Changed

- **TOTP secrets are encrypted at rest (OWASP A04).** The TOTP seed is now stored AES-256-GCM
  encrypted (keyed from the new `TOTP_ENCRYPTION_KEY`, which lives outside the database) instead of
  plaintext base32, so a DB-only leak no longer exposes 2FA seeds.
- **Backup-code verification is constant-time.** Stored keyed-hash digests are compared with a
  constant-time equality to remove a timing oracle on the code's hash.
- **Redis auth tokens are hashed at rest.** The TOTP temp-token Redis key now stores
  `blake3(token)` (matching the reset / email-change discipline), so a Redis snapshot yields only
  non-replayable hashes. `constants::cache_keys` was restructured into a `cache_keys/` module.
- **`/v0/moderation/logs` now requires the `Mod` role.** It was previously served with no
  authorization check.

### Security

- New required env vars: `TOTP_ENCRYPTION_KEY`, `AUTH_ACCOUNT_DELETION_TOKEN_EXPIRE_TIME`,
  `AUTH_DEVICE_VERIFY_TOKEN_EXPIRE_TIME`, `FRONTEND_PATH_CONFIRM_ACCOUNT_DELETION`,
  `FRONTEND_PATH_VERIFY_DEVICE` (see `.env.example`). Run the two new migrations
  (`create_auth_events`, `create_known_devices`) before deploying.

## [0.10.3] - 2026-06-28

### Changed

- **`image-processor` renamed to `media-processor`** throughout (service, image, config,
  and code). **Breaking:** the env vars `IMAGE_PROCESSOR_URL` / `IMAGE_PROCESSOR_TIMEOUT_SECS`
  are now `MEDIA_PROCESSOR_URL` / `MEDIA_PROCESSOR_TIMEOUT_SECS`; the compose service /
  per-env file are now `media-processor` / `media-processor.env`; and the image is
  `ghcr.io/levish0/smol-media-processor:0.2.0`. Existing `.env` files and deployments
  must be updated. Internally the bridge/worker clients, config fields, and the
  `process_media` function are renamed to match.

## [0.10.2] - 2026-06-28

### Added

- **Deployment bundle (`deploy/`)** — parametrized two-environment (dev + production)
  deploy stack mirroring the production posture: layered compose
  (`docker-compose.infra.yml` for stateful services + `docker-compose.app.yml` for
  stateless app services) selected by a committed per-env compose env-file
  (`deploy/{dev,production}.env`). `DEPLOY_ENV` picks `.envs/.<env>/`,
  `COMPOSE_PROJECT_NAME` isolates containers/volumes/networks so both envs run on one
  host, and `APP_VERSION` pins the GHCR image tag. Migrations run automatically via a
  one-shot `migration` service; only APISIX is host-published
  (`127.0.0.1:${APISIX_HOST_PORT}`), with infra and `server:8000` reached over the
  compose network. A `deploy/justfile` wraps the common flows
  (`just up <env>`, `down`, `ps`, `pull`, `logs`, `compose`).

### Fixed

- **Email link tokens are URL-encoded** — verification, password-reset, and
  email-change links now wrap the token in `urlencoding::encode` so the link stays
  valid regardless of token alphabet (defense-in-depth; current tokens are already
  URL-safe base64).

## [0.10.1] - 2026-06-28

### Fixed

Worker job delivery reliability — the server now confirms jobs are persisted, and the
worker no longer silently drops or duplicates failed/redelivered jobs.

- **Publish acknowledgement** — `bridge::worker_client::publish_job` now awaits the
  JetStream server ack instead of dropping the `PublishAckFuture`. A missing stream or
  a failed store is surfaced as `WorkerServiceConnectionFailed` rather than being
  mistaken for success (jobs are no longer silently lost). Serialize/publish/ack
  failures are now logged with the subject.

### Added

- **Dead-letter queue** — permanently failed messages (unparseable payloads or
  max-deliveries exhausted) are republished to a new `axumkit_jobs_dlq` stream
  (subject `axumkit.dlq.{origin_stream}`, `Limits` retention, 14-day max age) with
  `X-DLQ-Origin-Stream`/`X-DLQ-Consumer`/`X-DLQ-Reason` headers before being terminated,
  so they can be inspected and replayed instead of dropped.
- **Per-message dedup** — `NatsConsumer::with_dedup` skips redeliveries of an
  already-processed message (keyed by stream sequence in Redis, 24h TTL, fail-open).
  Applied to the non-idempotent `email` and `oauth/profile_image` consumers so a lost
  ack no longer resends a verification email or reprocesses a profile image.

## [0.10.0] - 2026-06-26

### Added

Native-app (non-browser) authentication and an APISIX edge gateway for tiered rate
limiting. Browser flows are unchanged; the same opaque session token now also works
for app clients over `Authorization: Bearer`.

- **Native-app session tokens (`Authorization: Bearer`)**
  - the session extractor now accepts the session token from either the `HttpOnly`
    cookie (browser) or an `Authorization: Bearer` header (app); the cookie wins when
    both are present, and validation is identical (same opaque token, hashed at rest)
  - new `SessionTokenResponse` returns the token in the response body (`Cache-Control: no-store`)
    for app clients, which have no cookie jar
  - session resolution is centralized in `SessionService::resolve_session` (Redis +
    DB user-existence check), shared by the extractors and the gateway `/auth/check`

- **`/v0/app/auth/*` route family** — app variants of the cookie-minting flows that
  return the token in the body: `login`, `totp/verify`, `verify-email`,
  `complete-signup`, and OAuth `oauth/google/token` + `oauth/github/token`

- **Native-app OAuth (provider-token flow, allauth `provider/token` pattern)**
  - Google: app submits a Google ID token; verified directly (JWKS signature, `iss`,
    `aud` pinned to our client id, `exp`, verified email) — extracted into a reusable
    `verify_google_id_token` shared with One Tap
  - GitHub: app submits an access token, verified via GitHub's token-introspection
    endpoint (`POST /applications/{client_id}/token`) so a token minted for another
    app is rejected — restoring the audience binding GitHub access tokens otherwise lack
  - all provider flows (redirect, One Tap, provider-token) now share
    `resolve_oauth_sign_in`; the pending-signup binding is `Some(anonymous)` for browser
    flows and `None` for app flows (bound by the pending token's secrecy alone)

- **APISIX edge gateway (`compose/production/apisix/`, `docker-compose.apisix.yml`)**
  - `/v0/auth/check` forward-auth endpoint: always 200 with `X-Auth-*` identity headers
    used only for rate-limit keying (per-user vs. per-IP); the backend never trusts
    these for authorization and re-validates the session credential on every route
  - global rule strips client-forged identity headers and normalizes the real client IP;
    tiered `limit-count` rules for auth-credential endpoints and a default safety net
  - `extract_ip_address` upgraded to trust `X-Real-Client-IP` only behind a matching
    `X-Internal-Secret` (new optional `INTERNAL_PROXY_SECRET`), mirrored by the gateway

## [0.9.0] - 2026-06-22

### Security

OWASP-aligned authentication hardening. **Breaking:** the initial users migration
was edited in place (email column-unique replaced by a `lower(email)` functional
unique index), so existing databases must be re-migrated with `cargo run -- fresh`.

- **Session tokens are now high-entropy and stored hashed**
  - session identifiers are 256-bit CSPRNG bearer tokens instead of predictable `UUIDv7`
  - only the blake3 hash of the token is stored in Redis; the raw token lives only in the cookie, so a store leak yields no usable sessions
  - `SessionService::create_session` returns `(raw_token, session)` and the session extractor resolves the cookie token to its hash before every lookup

- **One-time email tokens stored hashed at rest**
  - email-verification, password-reset, and email-change tokens are keyed by their blake3 hash in Redis (raw token only ever sent in the email link)
  - pending-signup email/handle indices store the hashed id; verification resend now mints a fresh token within the remaining window and invalidates the previous link

- **Constant-time login (account-enumeration defense)**
  - every login path performs exactly one Argon2 verification (against a fixed dummy hash when the account is missing or password-less), so failures take uniform time
  - a `MAX_PASSWORD_BYTES` (1 KiB) ceiling bounds Argon2 input as a DoS guard

- **Enumeration-safe signup**
  - an already-registered email returns the same "verification sent" response as a fresh signup, no longer revealing which emails are registered

- **Case-insensitive email identity**
  - emails are normalized (trim + lowercase) at the repository boundary on every create/update/find, with OAuth provisioning included
  - a `lower(email)` functional unique index enforces case-insensitive uniqueness as defense-in-depth

- **TOTP replay protection (RFC 6238 §5.2)**
  - a verified TOTP code is claimed single-use in Redis for its validity window, blocking replay through a fresh temp token

- **Cookie name prefixes**
  - the production session cookie uses `__Host-` (or `__Secure-` with a configured domain) to block sibling-origin cookie injection/fixation

- **OAuth hardening**
  - GitHub sign-in always derives identity from `/user/emails` and accepts only a primary **and** verified address, never the unverified public-profile email
  - OAuth sign-in / complete-signup now issue a non-persistent session cookie (no implicit 30-day remember-me)
  - a unique-constraint violation during OAuth user creation maps to `409` instead of a generic `500`

- **Stronger password policy**
  - signup / reset / change now require 12–128 characters (OWASP ASVS), up from 6–20
  - login and re-authentication password fields are no longer length-validated (verification candidates only)

## [0.8.0] - 2026-06-12

### Added

- **Concern-grouped environment files**
  - added `.envs/.example`, `.envs/.local`, and `.envs/.test` trees split by concern (`postgres.env`, `r2.env`, `server.env`, `worker.env`, `image-processor.env`, and `meilisearch.env`)
  - updated compose files to load only the env files each service needs
  - documented the new environment layout and production PgDog credential alignment

- **Disposable e2e stack and harness**
  - added the `e2e` crate for black-box HTTP tests against a running Docker stack
  - added Mailpit for email verification tests and SeaweedFS as the S3-compatible R2 substitute
  - added e2e coverage for health checks, signup/verification/login, duplicate handle rejection, and profile image WebP sanitization
  - added the GitHub Actions e2e workflow with container log dumping on failure

- **PgDog configuration templates**
  - added local PgDog config under `compose/local/pgdog.toml`
  - added production PgDog backend and client credential templates under `compose/production/pgdog/`

### Changed

- **Image processing moved to smol-image-processor**
  - server profile/banner uploads now call the external image processor and validate the returned WebP payload
  - worker OAuth profile image jobs use the same image processor path
  - removed the internal `image_utils` crate from server/worker upload paths

- **Database and compose setup aligned around one PostgreSQL endpoint**
  - standardized runtime config and migrations on `POSTGRES_HOST`, `POSTGRES_PORT`, `POSTGRES_NAME`, `POSTGRES_USER`, and `POSTGRES_PASSWORD`
  - local/test env templates default to direct `postgres:5432`
  - production guidance uses `pgdog:6432` when the app should connect through PgDog

- **Email verification now signs users in**
  - `POST /v0/auth/verify-email` now creates a session and returns the login cookie after the account is created
  - verified signup clients can continue with authenticated requests without making a separate login call

- **Worker runtime hardening**
  - NATS pull consumers now bound `max_ack_pending` to configured concurrency
  - long-running jobs send in-progress acks during processing
  - permanently failed messages are terminated after the configured max deliveries

- **Development workflow cleanup**
  - `just` now owns compose, migration, OpenAPI, test, e2e, and publish commands
  - root compose files use the concern-grouped env layout

### Removed

- **Legacy split database environment**
  - removed `POSTGRES_WRITE_*` / `POSTGRES_READ_*` compatibility from Rust config loaders and docs
  - removed the internal image processing crate from the workspace

## [0.7.5] - 2026-05-14

### Added

- **Active session listing and revocation endpoints**
  - added `GET /v0/auth/sessions` to list the authenticated user's active sessions with `is_current`
  - added `DELETE /v0/auth/sessions/{management_id}` to revoke an owned session using a public management identifier instead of the bearer cookie value
  - revoking the current session also clears the `session_id` cookie

### Changed

- **Session storage no longer exposes bearer session IDs**
  - session payloads now include a separate `management_id` for JS-visible session management flows
  - session listing responses return `management_id` and no longer expose the HttpOnly cookie `session_id`

- **Per-user session lookup now uses a Redis ZSET**
  - replaced `user_session_idx:{user_id}:{session_id}` scan-based lookup with `user_sessions:{user_id}` plus `session_mgmt:{management_id}`
  - listing and revocation no longer require scanning the Redis keyspace
  - stale management entries are pruned during list and delete flows

## [0.7.4] - 2026-05-08

### Changed

- **Dedicated Redis lock store for worker cron jobs**
  - worker cron locking now uses a separate `REDIS_LOCK_*` Redis connection instead of reusing the cache Redis client
  - added `redis-lock` to the local development stack with a `noeviction` policy for distributed lock safety
  - updated worker configuration, environment examples, and docs to reflect the new lock Redis role

### Fixed

- **OAuth pending-signup rejection consistency**
  - Google OAuth login and GitHub OAuth login now reject emails already reserved by pending email/password signups, matching Google One Tap and `complete_signup`
  - avoids issuing OAuth pending-signup tokens that would later fail at signup completion because the email was already reserved

## [0.7.3] - 2026-05-08

### Added

- **Worker-based OAuth profile image processing**
  - added `OAuthProfileImageJob` and a dedicated worker consumer/JetStream subject for OAuth profile image downloads
  - moved provider image fetch, validation, processing, and R2 upload out of synchronous signup completion
  - added server-side background job helpers and worker bridge publishing for OAuth profile image jobs

- **OAuth pending signup token state**
  - added `PendingSignupTokenState` with `pending` and `completed` variants for Redis-backed OAuth signup state
  - completed OAuth signup tokens are now retained briefly so clients can safely retry after a lost response

### Changed

- **OAuth signup completion flow**
  - `complete_signup` now creates the user first, stores completed token state, then schedules async side effects such as user indexing and profile image processing
  - OAuth sign-in flows now store pending signup state in the new token envelope instead of raw pending signup payloads

- **Worker storage naming cleanup**
  - renamed worker R2 connection/module usage to `r2_assets` / `R2AssetsClient` for consistency with the storage crate and asset bucket purpose

### Fixed

- **OAuth signup retry/idempotency**
  - repeated `complete_signup` requests can now recover from the already-created user path and issue a session instead of failing or duplicating work

- **Google One Tap JWKS refresh behavior**
  - added a refresh mutex and double-check flow to avoid parallel JWKS fetch stampedes
  - retries once with a forced JWKS refresh when Google rotates signing keys before cache expiry

## [0.7.2] - 2026-05-07

### Added

- **Shared `storage` crate**
  - extracted Cloudflare R2 client setup and asset operations into a reusable workspace crate
  - removed duplicated R2 client implementation between server and worker

- **Shared `image_utils` crate**
  - moved image validation, hashing, resize, and WebP conversion logic out of the server crate
  - kept upload flows in server while making image processing reusable across crates

### Changed

- **Internal crate naming simplified**
  - renamed internal workspace crates from `axumkit_*` / `axumkit-*` to plain names such as `server`, `worker`, `config`, `constants`, `dto`, `entity`, and `errors`
  - updated workspace dependencies, imports, and crate paths to match the new names

- **Build and docs alignment**
  - updated Docker build/runtime targets to use `server` and `worker` package/binary names
  - refreshed README and docs references to the renamed internal crates

## [0.7.1] - 2026-04-24

### Fixed

- **SET_COOKIE header overwriting** — switched from `insert` to `append` in login response to support multiple cookies.
- **JWKS write lock contention** — reworked cache handling to avoid holding the write lock during network I/O.

## [0.7.0] - 2026-04-05

### Added

- **Two-stage email signup flow**
  - `POST /v0/auth/signup` stores pending signup in Redis and queues a verification email (returns 202 Accepted)
  - `POST /v0/auth/verify-email` now creates the user account after token verification, not before
  - Atomic Redis reservation via Lua script (`reserve_pending_signup.lua`) — prevents concurrent email/handle collision
  - `PendingEmailSignupData` stored with email, handle, display name, and pre-hashed password
  - `POST /v0/auth/resend-verification-email` is now a **public** endpoint (email-based, no session required) with email enumeration prevention
  - `ResendVerificationEmailRequest` DTO with email validation
  - `repository_create_user_with_password_hash` for deferred user creation
  - `get_ttl_seconds` Redis utility for remaining token TTL lookup

- **Google One Tap OAuth sign-in**
  - `POST /v0/auth/oauth/google/one-tap/login` — validates Google ID token server-side via JWKS
  - JWKS caching with `Cache-Control` max-age parsing and forced refresh on key rotation
  - `GoogleOneTapLoginRequest` DTO
  - `GoogleInvalidIdToken`, `GoogleJwksFetchFailed`, `GoogleJwksParseFailed` error variants with protocol codes
  - `jsonwebtoken` v9 workspace dependency

- **User management system**
  - `POST /v0/users/ban` — ban a user with optional expiration
  - `POST /v0/users/unban` — remove an active ban
  - `POST /v0/users/roles/grant` — grant Mod/Admin role with optional expiration
  - `POST /v0/users/roles/revoke` — revoke a granted role
  - All management actions create moderation logs and require admin permission
  - `CannotManageSelf` error variant prevents self-management
  - 9 permission unit tests

- **Worker supervisor pattern**
  - JoinSet-based supervisor loop with `catch_unwind` auto-restart for consumer panics
  - `ConsumerKind` enum with `ConsumerExitOutcome` for structured exit tracking

- **Handle and display name validation**
  - `validate_handle`: ASCII alphanumeric + underscore, 4–15 chars, no leading/trailing `_`, no `__`
  - `validate_display_name`: blocks control characters, emoji, and Zalgo text (consecutive `NonspacingMark` limit)
  - `RESERVED_HANDLES`: 26 reserved words (e.g. `admin`, `support`, `system`)
  - `unicode-general-category` crate dependency

- **Pending email signup constants**
  - `EMAIL_SIGNUP_EMAIL_PREFIX`, `EMAIL_SIGNUP_HANDLE_PREFIX` with key generators in `axumkit-constants`

### Changed

- **Database connection simplified**
  - Merged separate `write_db` + `read_db` into single `db` field in `AppState`
  - Environment variables changed from `POSTGRES_WRITE_*` / `POSTGRES_READ_*` to `POSTGRES_*`
  - Single `establish_connection()` replaces `establish_write_connection()` + `establish_read_connection()`

- **Signup endpoint moved from User to Auth module**
  - Removed `POST /v0/users` (immediate user creation)
  - Replaced with `POST /v0/auth/signup` (deferred creation after email verification)
  - `CreateUserResponse` now returns 202 Accepted instead of 201 Created

- **OAuth pending signup collision checks**
  - `find_or_create_oauth_user` now checks pending email/password signups before creating OAuth users
  - `complete_signup` pre-checks pending signups for email and handle collisions
  - Google, GitHub, and Google One Tap sign-in flows reject emails held by pending signups

- **All Korean comments translated to English** across 85+ source files

### Fixed

- **`CannotManageSelf` permission bug** — was incorrectly returning `CannotManageHigherOrEqualRole`

---

## [0.5.0] - 2026-03-02

### Added

- **OAuth authorize flow query support**
  - Added `OAuthAuthorizeFlow` (`login`/`link`) and `OAuthAuthorizeQuery` DTOs
  - Added query validation and OpenAPI params for `/v0/auth/oauth/{provider}/authorize`
- **OAuth pending signup lock primitives**
  - Added `oauth_pending_lock_key` and `OAUTH_PENDING_LOCK_PREFIX` in `axumkit-constants`
  - Added Lua script `release_pending_lock.lua` for safe lock-token-based unlock
- **Session refresh tests**
  - Added unit tests for `Session::needs_refresh` sliding TTL threshold behavior

### Changed

- **OAuth state hardening**
  - OAuth state payload now stores `flow`, `provider`, and `anonymous_user_id`
  - Google/GitHub `login` and `link` now validate provider/flow/browser binding before token exchange
- **OAuth signup completion flow hardening**
  - `complete_signup` now uses a per-pending-token Redis lock to serialize completion attempts
  - Pending signup payload is now bound to `anonymous_user_id`
  - Pending token is consumed only after successful DB commit
- **OAuth unlink safety**
  - Switched to row-level lock (`SELECT ... FOR UPDATE`) on user for unlink flow serialization
  - Explicitly checks target provider exists before deletion
- **Session index model**
  - Replaced `user_sessions:{user_id}` set with TTL-synced `user_session_idx:{user_id}:{session_id}` keys
  - Updated create/refresh/delete/delete-all/delete-other logic to manage index TTL with session TTL
- **Session sliding window threshold calculation**
  - `Session::needs_refresh` now uses configured `auth_session_sliding_ttl_hours`-based threshold instead of session-age ratio

### Fixed

- **OAuth race conditions**
  - Reduced duplicate completion/race windows in pending OAuth signup completion
- **Last authentication factor unlink race**
  - Prevented concurrent unlink edge case that could remove the final login factor

---

## [0.4.5] - 2026-02-11

### Fixed

- **OAuth profile image upload to R2**
  - OAuth sign-up previously stored the provider's external profile image URL (e.g. `https://lh3.googleusercontent.com/...`) directly in the database, which broke when passed through `build_r2_public_url()`
  - Profile images are now downloaded from the OAuth provider, validated (4 MB limit), processed (WebP conversion, max 2000x2000), and uploaded to R2 during `complete_signup`
  - Database stores the R2 storage key (`user-images/{hash}.webp`) instead of the external URL
  - Gracefully falls back to no profile image if download/processing fails (sign-up still succeeds)

---

## [0.4.4] - 2026-02-08

### Refactored

- **OAuth provider trait extraction (`OAuthProviderConfig`)**
  - Unified duplicate Google/GitHub OAuth logic (URL generation, token exchange, service layer) into `OAuthProviderConfig` trait with generic functions
  - Added `generate_auth_url<P>()`, `exchange_code<P>()`, `service_generate_oauth_url<P>()` generic functions
  - Reorganized provider-specific code into `google/` and `github/` folders (config, client, generate_url, sign_in, link)
  - `provider/` now contains only shared infrastructure (trait + generic functions)
  - Added `OAUTH_STATE_TTL_SECONDS` constant to `axumkit-constants` (replaced hardcoded value)
  - Deleted 6 legacy flat service files and old provider subdirectories

---

## [0.4.3] - 2026-02-06

### Improved

- **Whitespace-only string validation (`validate_not_blank`)**
  - Added `string_validator::validate_not_blank` custom validator to reject whitespace-only strings
  - Applied to 13 fields across 9 request DTOs (handles, display names, titles, content, search queries, bio)
  - Excluded: passwords, tokens, TOTP codes, OAuth codes/states, email addresses
  - Addresses `validator` crate's `length(min=1)` not catching whitespace-only input (uses `chars().count()` without trim)

---

## [0.4.2] - 2026-02-06

### Improved

- **Environment variable validation**
  - `ServerConfig`: Replaced individual `.expect()` calls with `require!`/`require_parse!` macros that collect all missing/invalid variables and report them at once (30 required vars: 27 string + 3 parsed)
  - `WorkerConfig`: Same pattern applied with `require!` macro (20 required string vars)
  - Startup now shows all configuration errors in a single panic message instead of failing on the first missing variable

---

## [0.4.1] - 2026-02-02

### Added

- **Google OAuth verified_email check**
  - Added `verified_email` field validation for Google sign-in/link
  - Only verified emails are now allowed, consistent with GitHub behavior
  - Added `OauthEmailNotVerified` error type (`oauth:email_not_verified`)

---

## [0.4.0] - 2026-02-01

### Added

- **Helm Charts**: Kubernetes deployment support
  - `axumkit-server` chart with migration job (post-install hook + wait-for-postgres)
  - `axumkit-worker` chart for background workers
  - `axumkit` umbrella chart with all infrastructure dependencies
  - Dependencies: PostgreSQL, Redis (session/cache), NATS, MeiliSearch, SeaweedFS
  - Environment variables based on `server_config.rs` and `worker_config.rs`
  - HPA, PDB, Ingress, ServiceAccount templates included

---

## [0.3.2] - 2026-01-31

### Added

- **Stability Layer**: Tower middleware stack to protect server from overload
  - `ConcurrencyLimitLayer` - Max concurrent requests (default: 500)
  - `BufferLayer` - Request queue when at limit (default: 1024)
  - `TimeoutLayer` - Request timeout (default: 30s)
  - Configurable via `STABILITY_CONCURRENCY_LIMIT`, `STABILITY_BUFFER_SIZE`, `STABILITY_TIMEOUT_SECS`

- **xtask**: Development environment management tool
  - `cargo xtask dev` - Full setup (docker + migrate)
  - `cargo xtask docker-up/down/status` - Docker service management
  - `cargo xtask migrate/migrate-fresh` - Database migrations
  - Manages Redis Session, Redis Cache, NATS, MeiliSearch, SeaweedFS

- **load-tests**: k6 load testing suite
  - `health-check.js` - Health check endpoint load test
  - 4 scenarios: normal (100 VUs), at_limit (500), buffer (1000), spike (10000)

## [0.3.1] - 2026-01-27

### Changed

- **Read/Write DB Separation in Routes**: Read-only routes now use `read_db` (replica) instead of `write_db` (primary)
  - User: `get_user_profile`, `get_user_profile_by_id`, `get_my_profile`, `check_handle_available`
  - Posts: `list_posts`, `get_post`
  - Action Logs: `get_action_logs`
  - Auth: `list_oauth_connections`, `totp_status`

## [0.3.0] - 2026-01-26

### Changed

- **Database Connection Split**: Separate Write (Primary) and Read (Replica) database connections
  - `AppState.conn` → `AppState.write_db` and `AppState.read_db`
  - Environment variables changed from `POSTGRES_*` to `POSTGRES_WRITE_*` and `POSTGRES_READ_*`
  - Worker uses `POSTGRES_WRITE_*` only (background jobs don't need read replica)
  - Enables PgBouncer connection pooling and read replica support for better scalability

### Added

- Worker environment variables added to `.env.example` (SMTP, FRONTEND_HOST, etc.)

## [0.2.3] - 2026-01-26

### Changed

- Upgrade Rust version from 1.92.0 to 1.93.0
- Update `sea-orm` from 2.0.0-rc.28 to 2.0.0-rc.29
- Add v4 feature to `uuid` crate

### Improved

- Parallelize E2E tests
  - Each test run gets a unique project name for container isolation
  - File-based locking for coordinating image builds across test binaries
  - Use `docker compose` service names instead of container names for port lookup

## [0.2.2] - 2025-01-20

### Removed

- S3 checksum calculation/validation for SeaweedFS and R2 connections
  - Removed `RequestChecksumCalculation::WhenRequired` from SeaweedFS client
  - Removed `RequestChecksumCalculation::WhenRequired` and `ResponseChecksumValidation::WhenRequired` from R2 client
  - Applies to both `axumkit-server` and `axumkit-worker`
