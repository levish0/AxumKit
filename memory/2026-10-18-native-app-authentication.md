# 2026-10-18 — Native app authentication

## Changes

- Raised the default session lifetimes from 30 minutes idle and 12 hours absolute
  to 14 days idle (`SESSION_IDLE_SECONDS=1209600`) and 90 days absolute
  (`SESSION_ABSOLUTE_SECONDS=7776000`). Cookie and bearer sessions share them.
  The five-minute recent-authentication window is unchanged.
- Added native OIDC ID token login:
  `POST /api/v1/auth/oidc/{provider}/native` returns a single-use `flow` secret
  and `nonce`; `POST /api/v1/auth/oidc/{provider}/native/complete` accepts
  `flow` and the provider SDK `id_token` and always returns a bearer session or
  an MFA challenge.
- `OIDC_PROVIDERS` entries accept an optional `native_client_ids` array. An empty
  or missing list disables the native routes for that provider (`404`).
- Added the `oauth_native_flows` table (migration 31) with hashed flow and nonce,
  provider, and expiry. Authentication cleanup deletes expired rows.
- Extracted shared ID token verification (with the existing unknown-key refresh)
  and the external-identity login branch, so browser callback and native login
  apply the same account, welcome-mail, and MFA rules.

## Decisions and constraints

- The owner chose native SDK ID tokens over a browser redirect with an app
  handoff code, and chose global 14/90-day defaults over bearer-only lifetimes.
- Native flows are a separate table, so neither completion route can consume the
  other flow type. Completion deletes the flow before parsing or verifying the
  token. A flow sent to another provider is not consumed.
- The service disables the library audience match for native tokens and then
  requires every `aud` and any `azp` to be the provider `client_id` or a
  configured native client ID. Multiple audiences require `azp`. The library
  still verifies signature, issuer, expiry, and nonce.
- Native routes are login only. Identity linking remains a browser flow.
- OIDC login, browser or native, does not use Turnstile. The provider performs
  the interactive sign-in, and provider SDKs have no Turnstile widget.
- The branch merged `main` (schema consistency) and the Turnstile branch. The
  merge kept both sides of each conflict. The native flow entity declares its
  64-character string columns for the schema check.

## Validation

- Passed `cargo xtask check`: formatting, architecture boundaries, documentation
  links, Clippy with warnings denied, workspace unit and doc tests (including new
  `native_client_ids` configuration tests), and OpenAPI drift after
  `just openapi`. After the last test addition, `cargo fmt --check` and workspace
  Clippy with warnings denied passed again.
- Passed `cargo xtask test-api`: api 7/7, api_access 9/9, and oidc 8/8. That run
  included the four first native tests and the unchanged browser OIDC tests after
  the shared-verification refactor.
- The added expired-flow cleanup test and the full oidc binary passed 9/9 against
  a disposable local Compose PostgreSQL through `TEST_DATABASE_URL`.
- The first `cargo xtask test-db` run passed 14/16 identity tests. The idle
  extension cleanup test hit its five-second lock-wait deadline, and the session
  ownership test hit a connection pool timeout. Other worktrees were running
  `test-db`, API tests, and Clippy on the same machine at that time. Both tests
  passed alone. The full identity (16/16) and jobs (14/14) binaries then passed
  with `--test-threads=2` against a disposable local PostgreSQL. The exact cause
  of the first failures was not established beyond the observed load.
- After merging `main` and the Turnstile branch, `cargo xtask check` passed and
  regenerated OpenAPI had no content drift. `cargo xtask test-db` passed identity
  16/16, jobs 14/14, and schema_consistency 2/2. `cargo xtask test-api` passed
  api 7/7, api_access 9/9, human_verification 2/2, and oidc 9/9.
- Process, file, content, deployment, release-image, k6, and copied-template
  suites were not run. No remote CI, deployment, or live provider acceptance was
  verified.

## Remaining work

- Live Google and Apple SDK acceptance against real client registrations has
  not been performed.
