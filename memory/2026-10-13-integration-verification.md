# 2026-10-13 — Integrated template verification

## Changes

- Completed verification of the original-file/search implementation, concurrent
  session-cleanup fix, and upstream dependency updates together on main.
- The verified implementation is commit
  `db4cb165170ca3c22b270da40a08955f98327248`. This note changes documentation only.

## Decisions and constraints

- Original bytes remain private S3 data; current PostgreSQL permissions and state
  govern access and search results. No content processing was introduced.
- ApiJson, ApiQuery, and ApiPath deserialize typed inputs and map extraction errors.
  Services validate values and current authority. OpenAPI schema derivation does
  not perform validation; no automatic DTO validation trait is present.
- Load results are regression evidence from a local debug build. The current
  script uses one account fixture across virtual users and read operations;
  realistic distinct-user/tenant and read/write capacity remains a separate test.

## Validation

- Local `just check`: passed formatting, boundaries, documentation links, Clippy,
  20 unit tests, doc tests, and generated OpenAPI checking.
- Local `just test-db`: ten identity and six queue tests passed, including the
  reproduced-and-fixed session cleanup race.
- Local `just test-api`: five production-router and three signed local OIDC tests
  passed. Local `just test-files`: seven real PostgreSQL/SeaweedFS/Meilisearch tests
  passed on the combined revision in 46.20 seconds.
- Local `just test-process`: actual API/worker/Mailpit v1.31.4 flow, replacement,
  cross-process revocation, ciphertext clearing, and log checks passed.
- Local `just test-deployment` and Actions syntax lint passed.
- Local `K6_PROFILE=extended just load-test`: twenty virtual users for two minutes;
  19,620 requests/checks, zero failures, and HTTP p95 145.15 ms. The earlier smoke
  profile also passed; this is not production capacity evidence.
- [Implementation CI](https://github.com/HiruneWorks/hirune-axum-kit/actions/runs/38407100181)
  passed on the implementation commit. The verify job completed every canonical
  command, including copied-template tests and smoke k6, in 8m26s. The production
  Linux release image built successfully in the container job in 9m19s.
- [CodeQL execution](https://github.com/HiruneWorks/hirune-axum-kit/actions/runs/38407098256)
  completed successfully for Rust and JavaScript. This records execution status,
  not a claim that all security risks are absent.
- Removed only owned disposable test projects and volumes. No running user
  infrastructure was stopped. The image was built in CI; no registry publication
  or production deployment was performed.

## Remaining work

- Actual production-image runtime with restricted PostgreSQL roles, proxy/TLS and
  SMTP STARTTLS acceptance, backup/restore, external provider acceptance, and
  realistic mixed load remain deployment/operator verification.
- Common/breached-password screening, administrator MFA requirements,
  recovery-code regeneration, and configurable audit/failed-job retention remain
  product/security extensions. They are not part of this completed file milestone.
- This documentation checkpoint may trigger another CI run. The implementation
  evidence above refers to its exact source commit and does not claim a later
  documentation commit has completed remote CI.
