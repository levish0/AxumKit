# 2026-10-18 — Schema consistency

## Changes

- Added PostgreSQL schema comparison to `just test-db` and its existing CI step.
- Enabled SeaORM `entity-registry` through a test-only dependency feature.
- Added explicit entity string lengths, text types, and JSONB metadata to match
  the existing schema. Preserved migrations and enum wire values.
- Added typed bindings for constrained string-enum columns. The checker reads
  values from `ActiveEnum::values()`.
- Added deliberate database changes to verify failure detection. Diagnostics and
  assertions use English.

## Decisions and constraints

- Migrations remain independent historical snapshots. They do not import current
  domain enums.
- Materialize entities from `application::*` and `jobs::*` in an
  `entity_reference` schema inside the disposable test database. SeaORM's registry
  metadata is private, so compare PostgreSQL catalog descriptions of both schemas.
- Compare tables, columns, exact types and lengths, nullability, and primary key
  positions. Check both directions and reject an empty registry.
- Compare complete supported single-column string membership CHECK expressions.
  Reject missing, unregistered, unsupported, duplicate, or unvalidated enum checks.
- Do not infer arbitrary SQL semantics. Defaults, indexes, foreign keys, unique
  keys, cross-column checks, data migrations, rollback, and deployed upgrades are
  outside this comparison. See `docs/schema-verification.md`.
- The enum-to-column mapping remains explicit. Review new string enums for both
  their CHECK constraint and binding.
- Run database-suite cases serially to reduce interference with queue timing
  checks. Preserve internal actor/worker concurrency and existing deadlines.
  Other integration commands retain their case parallelism.
- Work on main in an isolated checkout. Preserve the existing Turnstile branch
  and its uncommitted changes in the original checkout.

## Validation

- Focused schema tests passed against PostgreSQL: the current schema matches,
  all 16 deliberate drift cases are detected, and reordered enum values do not
  produce drift. The empty-registry and missing-binding guards also passed.
- Both CHECK-expression parser unit tests passed.
- The first `just test-db` run passed all 16 identity tests and 13 of 14 queue
  tests. The multi-pool dispatch regression exceeded its existing 10-second
  deadline. Other Rust compilations were active; the exact cause was not
  established. Cargo stopped before the schema test target. Do not weaken the
  deadline to hide this failure.
- Formatting, architecture boundaries, document links, and Clippy passed. The
  first full workspace test build then failed with compiler memory-allocation
  errors. Retried with `CARGO_BUILD_JOBS=1` for the local process only. Project
  build settings and test deadlines remain unchanged.
- With `CARGO_BUILD_JOBS=1`, `just check` passed: formatting, architecture boundaries,
  document links, Clippy with warnings denied, 31 workspace unit tests, doc tests,
  and OpenAPI comparison.
- An external-fixture rerun was stopped before tests started because its temporary
  container had disappeared.
- The next standard Compose run passed the previously failing multi-pool case,
  but the single-pool dispatch case exceeded the same 10-second deadline. Both
  runs passed all 16 identity tests and 13 of 14 queue tests. Changed only the
  database-suite case harness to serial execution; kept test bodies and deadlines.
- After the harness change, `just check` passed again with the same checks and
  31 unit tests. The final standard Compose `just test-db` run passed all
  16 identity, 14 queue, and 2 schema tests, including the 16 deliberate drift
  cases. Both previously failing dispatch cases passed. Earlier timeout causes
  were not conclusively established.
- `git diff --check` passed. No migration files changed.
- API, content, file, process, deployment, release-image, copied-template, and k6
  suites were not repeated for this metadata and test-tooling change. Remote CI,
  deployment, and production acceptance were not verified.

## Remaining work

- Keep entity metadata and enum bindings current when schemas change.
