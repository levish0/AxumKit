# 2026-10-30 — PostgreSQL job load coverage

## Changes

- Added PostgreSQL regressions for shared business/job commit visibility and
  rollback, database row-lock access while workers are saturated, bounded claims,
  and finished-history cleanup across multiple batches.
- Added `just test-job-load` through the existing xtask infrastructure lifecycle.
  Each scenario uses four producers, one foreground database client, and one,
  two, or four independent worker pools with four execution slots each.
- The smoke profile registers 2,000 jobs against 10,000 retained rows per
  scenario. The extended profile registers 20,000 jobs against 100,000 retained
  rows. Every job has a source row committed in the same transaction. The handler
  changes that row. Verification checks jobs, attempts, source rows, and effects.
- Mixed bulk and notification jobs share the existing application queue.
  Reports include scenario completion throughput, registration latency, separate
  registration-to-handler latency, and baseline/loaded foreground transaction
  latency. CI retains `target/job-load-report.json` as the `job-load-report`
  artifact. `JOB_LOAD_RELEASE=true` selects an optimized test build.

## Decisions and constraints

- Keep PostgreSQL jobs and the existing worker runtime. Retain the shared
  transaction path without a separate broker or outbox relay. This change adds no runtime transport,
  queue policy, schema migration, or deployment service.
- Use a shared monotonic clock for diagnostic latency. The persisted job and
  attempt timestamp defaults use transaction start time and cannot measure this
  interval safely. Each foreground transaction has a five-second bound.
- Scenario completion has a 180-second bound. Timing distributions describe
  this synthetic workload; there is no machine-specific throughput threshold or
  notification priority guarantee.
- Retained rows model table cardinality, not aged-table dead tuples, vacuum work,
  or historical attempt volume. Synthetic database effects exclude HTTP, SMTP,
  FCM, storage, and search providers. Keep deployed k6 acceptance separate.

## Validation

- Passed `cargo xtask test-db`: 16 identity, 17 jobs, and 3 schema-consistency
  tests. The job count includes the three new pressure regressions.
- Passed `cargo xtask check`: formatting, architecture boundaries, documentation
  links, Clippy with warnings denied, workspace unit/doc tests, and OpenAPI drift.
- Passed `cargo xtask test-job-load`: all three smoke scenarios, 6,000 jobs in
  total, with every job, attempt, and database effect verified.
- Astra reviewed the new tests. The review found unsafe transaction-timestamp
  subtraction and an unbounded foreground probe wait. Both were corrected and
  re-reviewed before the final smoke run.
- Passed `JOB_LOAD_PROFILE=extended cargo xtask test-job-load`: all three
  scenarios, 60,000 jobs in total. Each scenario retained 100,000 history rows
  and verified 20,000 successful jobs, attempts, source rows, and effects.
- Local runs used the unoptimized test build and disposable PostgreSQL 18
  containers on this five-CPU cloud executor. The extended scenarios completed
  at approximately 141, 243, and 377 jobs/second for one, two, and four pools.
  Loaded foreground p95 transaction latency was approximately 3.6, 5.3, and 7.4
  milliseconds. These are scenario measurements, not maximum production capacity.
  Notification jobs shared the bulk backlog; the report records that delay.
- Remote CI is separate and was still running at handover. Local validation did
  not run deployed k6, live providers, or the optimized job-load option.

## Remaining work

- Run release-mode and deployed provider/API capacity measurements on the
  intended production resources before setting capacity limits.
