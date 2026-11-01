# 2026-10-17 — Optional Turnstile human verification

## Changes

- Added the optional `application::human_verification` module: a Cloudflare
  Turnstile siteverify client, `HumanVerificationService`, the closed
  `HumanVerificationAction` set, and `TurnstileSettings`.
- When `TURNSTILE_SECRET_KEY` is set, signup, verification resend,
  password-reset requests, and password login require
  `human_verification_token` in the JSON body. The handlers verify the token
  before they call the identity service.
- Added `TURNSTILE_SECRET_KEY`, `TURNSTILE_EXPECTED_HOSTNAMES`, and
  `TURNSTILE_VERIFY_ACTION`, the `human_verification_failed` (403) and
  `human_verification_unavailable` (503) error codes, and the `ClientAddress`
  extractor, which reuses the trusted-proxy client-address rule.
- Added fake-provider verifier tests that run in `just check`, production-router
  tests in `just test-api`, configuration unit tests, documentation, and
  environment examples.

## Decisions and constraints

- Turnstile is an HTTP-boundary abuse control, like the client rate limit. The
  identity service signatures do not change, so operator commands, workers, and
  tests do not need tokens. The verifier itself lives in the application crate
  with no HTTP-framework types.
- The token travels in the JSON body as `human_verification_token`. The field is
  optional in the schema and ignored when Turnstile is disabled.
- Each operation has a fixed widget action: `signup`, `email_verification`,
  `password_reset`, and `login`. The action check is on by default and can be
  disabled for Cloudflare test keys, whose responses do not carry real actions.
  The hostname check applies only when hostnames are configured.
- One retry follows a transport error, timeout, HTTP 429 or 5xx, or
  `internal-error`. It reuses the request's `idempotency_key`. Each attempt has a
  five-second deadline, redirects are disabled, and the body is limited to 16 KiB.
- Missing and rejected tokens return 403. Provider failures and a rejected
  secret fail closed with 503. Logs record only a fixed failure category.
- Production configuration rejects Cloudflare's published test secret keys.
  The siteverify URL and attempt deadline are settings fields so tests can use a
  local server; the environment loader always supplies Cloudflare's values.

## Validation

All checks ran locally on Windows with Docker, on 2026-10-17 and 2026-10-18.
Other sessions ran heavy builds and Docker suites on the same machine at the
same time.

- `just fmt` and `just openapi` ran. The OpenAPI document gained the optional
  `human_verification_token` field on the signup, login, and email DTOs.
- Passed `just check`: formatting, architecture boundaries, documentation links,
  Clippy with warnings denied, 36 workspace unit and fake-provider tests, doc
  tests, and OpenAPI drift.
- Passed `just test-api`: 7 API, 9 external API, 2 Turnstile router, and 4 OIDC
  tests. Two earlier runs failed with PostgreSQL pool acquisition timeouts in an
  external API test and then in both Turnstile router tests. The Turnstile
  router tests then passed alone on a disposable database, and the full suite
  passed on the next run with unchanged product code.
- Passed `just test-deployment`.
- `just test-process` passed on the third run (3 tests). The first two runs each
  failed one different separate-process test with a readiness deadline, an API
  process exit before readiness, or a mail delivery deadline. Unchanged code
  passed on the next run. The exact environmental cause was not established.
- The test credentials, including the fake siteverify secret, are generated at
  runtime so CodeQL does not report hard-coded credentials.
- `just test-db`, `just test-files`, `just test-content`, k6, release-image, and
  copied-template checks were not run. No live Cloudflare siteverify call,
  remote CI result, or deployment was verified by these local checks.

## Remaining work

- Live Cloudflare acceptance with a real sitekey and secret was not performed.
- Other abuse-prone operations, such as OIDC login start, are not protected.
