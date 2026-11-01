# 2026-10-17 — Initial password for OIDC-only accounts

## Changes

- Added `POST /api/v1/account/password/initial`. An authenticated account
  without a password can add its first password. `GET /api/v1/account` now
  returns `has_password`.
- Added `IdentityService::set_initial_password` in its own module, the
  `set_initial_password` identity rate-limit operation, and the
  `initial_password_set` audit event.
- Added PostgreSQL service and production-router tests in
  `tests/oidc/initial_password.rs`. Updated the identity, security, mail, and
  regression documentation and the generated OpenAPI document.

## Decisions and constraints

- The operation follows `change_password` exactly, except that no current
  password exists to verify. `change_password` and `reset_password` handle
  credentials in the same way, so no choice between them was needed: hash
  before the lock, `replace_password` advances the security version,
  `revoke_account_credentials` revokes all sessions and challenges, the
  `PasswordChanged` notification and the audit event commit in the same
  transaction, and the HTTP response clears the session cookie.
- Recent authentication (five minutes) is the proof of account ownership,
  because an OIDC-only account has no current password. A fresh OIDC login,
  including its MFA gate, satisfies it.
- An existing password returns `409 conflict` before hashing and again under the
  account lock. A concurrent request loses with `409` or `401`, because the
  winner revokes its session. Only one password is stored.
- The notification reuses the `password-changed` template. A separate template
  was not added; its wording says that the password "was changed".
- After the change, password reset becomes available and the account can
  unlink its last external identity. Both already depend on `password_hash`.

## Validation

All checks ran locally on Windows with Docker, on 2026-10-17.

- `just fmt` and `just openapi` ran. The OpenAPI document gained the new path,
  `SetInitialPasswordInput`, and `AccountOutput.has_password`.
- Passed `just check`: formatting, architecture boundaries, documentation links,
  Clippy with warnings denied, 29 workspace unit tests, doc tests, and OpenAPI
  drift. A later edit only rewrapped two documentation paragraphs.
- Passed `just test-api`: 7 API, 9 external API, and 7 OIDC tests, including the
  three new initial-password tests.
- Passed `just test-db`: 16 identity and 14 queue tests.
- `just test-process`, `just test-files`, `just test-content`, k6, deployment,
  release-image, and copied-template checks were not run for this change. No
  remote CI run or live OIDC provider was verified.
- GitHub CodeQL flagged the literal test passwords (alerts 22-25). The tests
  now generate passwords at runtime. A new test path to the shared
  `TEST_PASSWORD` fixture constant (existing alert 5) was replaced by a stored
  hash comparison. `just check` and `just test-api` passed again after each
  change, with the same counts.
- On 2026-10-18, `main` (Turnstile, schema consistency, native OIDC login, and
  14/90-day sessions) was merged into this branch. The conflicts in
  `docs/security.md`, `memory/README.md`, and `tests/oidc.rs` kept both sides.
  Regenerated OpenAPI had no content drift. `cargo xtask check` passed.
  `cargo xtask test-api` passed api 7/7, api_access 9/9, human_verification 2/2,
  and oidc 12/12. `cargo xtask test-db` passed identity 16/16, but two
  timing-sensitive queue dispatch tests failed while another project compiled
  and ran its own test containers on the same machine. The queue binary (14/14)
  and schema_consistency (2/2) then passed against a disposable local
  PostgreSQL with `--test-threads=2`.

## Remaining work

- A dedicated "password added" mail template is optional product copy work.
- Live OIDC provider acceptance and remote CI are not covered by this record.
