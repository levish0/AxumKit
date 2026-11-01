# 2026-10-13 — Template foundation

## Changes

- Established a Rust 2024 source template with Axum, SeaORM, Tokio, PostgreSQL,
  structured tracing, optional Sentry, Cedar policies, MJML mail, and just commands.
- Added password signup/verification/reset, opaque revocable sessions, optional
  TOTP/recovery codes, optional OIDC with explicit linking and the common MFA gate,
  and a small organization domain demonstrating scoped access and cursor pagination.
- Added thirteen focused SeaORM migrations, a transactional PostgreSQL job queue,
  fenced leases, retry history, bounded shutdown, encrypted mail deliveries, and
  explicit migration/management executables.
- Separated `.envs/` examples and ignored secrets, `compose/` container/local
  configuration, and `deploy/` operator commands. Runtime and migration credentials
  are separated in deployment. Startup checks schema history without applying DDL.
- Added AGENTS.md, conventions, confirmed requirements, behavior and transaction
  contracts, generated OpenAPI, a reference regression inventory, CI, and isolated
  database, API, OIDC, process/SMTP, k6, deployment, and copied-template checks.

## Decisions and constraints

- This is a modular monolith with independently replaceable API and worker
  processes. PostgreSQL owns mutable authority; Redis is not a baseline dependency.
- Keep ownership local to each feature and enforce dependency direction. Crates
  represent real boundaries; avoid generic repositories and unnecessary traits.
- Prefer SeaORM/SeaQuery, UUIDv7 entity identities, one table per baseline migration,
  explicit descriptive names, and concise English comments only where needed.
- New-device email verification is excluded. Once enabled, MFA applies to password
  and OIDC login alike. Matching provider email never merges accounts automatically.
- OIDC refresh cooldown starts at completion, including slow failures and cancelled
  requests. Public provider keys may be cached; account/session authority may not.
- Author is Levi Laine, copyright 2026, Apache-2.0. Reference projects do not supply
  branding or a mandatory crate diagram. See the requirements and conventions.
- The owner requested this initial checkpoint on `main`. Branch names must not use
  an agent/tool prefix. Preserve unrelated local edits.
- Actions use version tags or maintained channels, without commit-hash pinning.
  Dependabot checks Cargo, Actions, Docker, and local Compose images weekly.

## Validation

- `just check`: passed formatting, dependency/HTTP boundaries, documentation links,
  Clippy with warnings denied, 18 unit tests, doc tests, and OpenAPI drift checking.
  Infrastructure tests are deliberately ignored by this command and run separately.
- `just test-db`: passed 9 identity and 6 queue tests against isolated PostgreSQL
  databases, including shared revocation, replay prevention, atomic rollback,
  heartbeat fencing, retained failures, retry delay, and bounded worker drain.
- `just test-api`: passed 5 production-router tests and 3 signed local OIDC tests,
  including slow discovery failure and cancellation cooldown.
- `just test-process`: passed the real HTTP/API/worker/Mailpit flow, worker
  replacement, cross-process revocation, ciphertext clearing, and log secrecy.
- `just load-test`: passed the two-user, ten-second k6 smoke profile: 384 requests,
  zero failed requests/checks, p95 21.51 ms. This local debug-build result is not
  deployed capacity evidence. The extended profile was not run.
- `just template-check`: passed fresh-key initialization, overwrite refusal,
  copying/renaming, 18 unit tests, and 9 identity/database tests. The subsequent
  Compose filename change is checked separately by the deployment verifier.
- `just test-deployment`: passed after the final filename change, including local
  Compose syntax and runtime/migration environment separation. PostgreSQL role
  privileges are operator-provisioned and were not proven by this configuration check.
- `actionlint .github/workflows/ci.yml`: passed. Dependabot YAML and configured
  directories were parsed/checked locally; no Dependabot update run is claimed.
- The Linux release image built successfully as `hirune-kit:verification`, but its
  build context preceded the final OIDC cooldown and worker error-propagation
  changes. The committed revision's container build remains a CI check; the local
  image is not a deployment artifact for this final revision.
- A short three-second heartbeat test lost its lease during concurrent release
  compilation. The test now keeps its handler open explicitly beyond a nine-second
  lease and checks successful completion, leaving production lease fencing intact.
- OpenAPI was regenerated after adding author metadata. A missing maintenance
  profile in the deployment verifier was corrected so manage/migration are checked.

## Remaining work

- Remote CI and production deployment/provider acceptance are separate from local
  checks. Live SMTP receipt, external OIDC configuration, reverse proxy trust,
  restricted PostgreSQL roles, backups, and production capacity need deployment
  validation.
- Choose product-specific breached/common-password screening, privileged-user MFA,
  account recovery, audit/failed-job retention, and edge abuse controls before
  exposing a copied application publicly. These are documented extension decisions,
  not implemented or certified controls.
- Public profiles, content/social features, storage, search, campaign email,
  recovery-code regeneration, and automatic key re-encryption are outside this
  checkpoint. No frontend is included for consuming email action fragments.
