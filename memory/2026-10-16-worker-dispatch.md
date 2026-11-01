# 2026-10-16 — Worker dispatch

## Changes

- Replaced the fixed claim interval with immediate filling/refilling of available
  execution slots. Successful claims do not incur an inter-job polling delay.
- `WORKER_POLL_MILLISECONDS` retains its name and 250 ms default. It now applies
  only after a lookup finds no eligible job. Its absolute retry deadline is not
  postponed by task completions, and a full worker does not prefetch extra work.
- Expired or exhausted claim recovery commits each row's state/history change
  and continues looking for ready work. Recovering a row no longer returns the
  same empty result used to schedule idle waiting.
- Added real PostgreSQL regressions for initial slot filling and repeated refill
  across 1, 2, and 4 independent worker pools, per-worker concurrency bounds,
  duplicate execution, idle lookup spacing, later arrivals, and lease recovery
  ahead of ready work. Updated behavior, requirements, runtime, regression index,
  development commands, and environment examples.

## Decisions and constraints

- Keep the existing PostgreSQL queue, short claim transactions, SKIP LOCKED,
  claim-token fencing, heartbeat, execution deadlines, retries, and attempt history.
  Execution slots bound active work; idle polling must not throttle ready work.
- Each worker retains independent connections and lifecycle. No broker, new
  dependency, schema change, or HTTP contract change was introduced.
- The capacity regressions deliberately set idle polling to 60 seconds and
  require ready work to fill/refill and finish within 10 seconds. Semaphore gates
  demonstrate concurrency/refill without assuming a machine-specific speedup.
- Keep timing-sensitive database and load evidence separate from cold builds.
  A passing scheduling regression is not a production capacity measurement.

## Validation

- Before the fix, the new refill and recovery regressions failed against the old
  implementation: refill exceeded its deadline, and recovery reported an empty
  queue ahead of ready work. The other 11 queue tests and 16 identity tests passed.
- Passed `just check`: formatting, architecture boundaries, documentation links,
  Clippy with warnings denied, 29 workspace unit tests, doc tests, and OpenAPI drift.
- The first post-fix database run overlapped cold workspace compilation and had
  a refill timeout plus lease loss in the preexisting slow-handler heartbeat test.
  The exact environmental cause was not established. With identical Rust source
  and unchanged timing thresholds, standalone `just test-db` passed all 16 identity
  and 14 queue tests, including all four dispatch regressions and heartbeat.
- Passed `just test-process`: all three separate-process API/worker/maintenance,
  identity-mail/restart, revocation, and external-key allocation tests.
- Passed `just load-test` smoke against two API processes and one worker with
  PostgreSQL, Mailpit, SeaweedFS, and Meilisearch: 2 complete workflows,
  251/251 checks, 155 HTTP requests, no unexpected HTTP failures, HTTP p95
  343.98 ms, filename visibility p95 4.65 s, and content/profile visibility p95
  2.31 s. Admission history, background drain, attempt outcomes, and log-secret
  checks passed. The extended profile was not run.
- The API, file, content, deployment, release-image, and copied-template suites
  were not repeated for this localized queue/runtime change. No remote CI run,
  production deployment, external-provider receipt, or production capacity was
  verified.

## Remaining work

- The 1/2/4-pool regressions verify scheduling, ownership, and concurrency rather
  than throughput scaling curves. Production queue throughput, wait distributions,
  API latency under a sustained backlog, database CPU/I/O/vacuum cost, and connection
  budgets still need measurements for the intended deployment and workload.
- Idle discovery remains polling with a configurable delay. Database notifications,
  batched claims, or a broker should follow a measured need and a separate design
  decision preserving durable registration and recovery.
