# 2026-10-18 — Single-host data services and pgBackRest

## Changes

- Added optional Docker Compose PostgreSQL 18.6, Meilisearch 1.55.0, and a
  pgBackRest scheduler outside k3s, including on the same Linux machine.
- Named this operational group `data-services` in Compose/deployment folders,
  environment profiles, xtask commands, and operator recipes. It groups the
  database and search service with PostgreSQL backup execution.
- Added separate dev/prod data settings and operator commands for explicit Docker
  contexts, private host bindings, persistent volumes, startup/shutdown, backup
  history/checks, and new-volume recovery. Application deployment remains k3s.
- Added PostgreSQL bootstrap roles and optional migration-only role-grant
  configuration. Migration completion gates runtime rollout. Existing DDL or
  inherited/PUBLIC mutation authority causes failure. Audit/API usage stay
  append-only for runtime; only maintenance can delete those records.
- Added continuous WAL archiving and an encrypted S3-compatible repository
  configuration. Startup creates/checks the stanza and takes an initial full
  backup before starting the non-root scheduler.
- Scheduled weekly full and daily differential backups from successful repository
  history. Missed slots produce one current catch-up. Weekly full debt takes
  precedence. Manual and scheduled backups share locks and the database socket;
  the scheduler reads PGDATA read-only and receives no database passwords.
- Added latest/PITR recovery into new volumes. Validation uses an isolated server
  with no TCP listener and archiving disabled. It completes WAL replay and stops
  without changing the live database. A configurable wait and `restore-resume`
  support retained recovery volumes. Cutover remains an explicit operator action.
- Added actual physical recovery, role restriction, scheduler, and Meilisearch
  persistence acceptance to CI, plus data configuration/isolation contracts.
- Addressed the PR CodeQL plaintext-storage finding in the acceptance fixture.
  Generated database passwords, the repository cipher, and the Meilisearch key
  pass through the process environment. Fixture files contain variable names,
  not generated secrets. Captured output and failure exceptions redact them.

## Decisions and constraints

- PostgreSQL and its backup executor share the Docker lifecycle. A k3s CronJob
  would need host-path/node coupling or a secured remote pgBackRest service.
  No Docker socket or host cron is used.
- Production backups belong outside the machine. Local data volumes and replicas
  do not provide host failover. Keep application/repository keys and credentials
  separately recoverable. Serialize data operations.
- The initial profile has no PostgreSQL or Meilisearch TLS termination. Bind only
  to the private host network and verify actual k3s connectivity and Docker-aware
  firewall rules. Remote backup TLS verification remains enabled.
- Backups retain two full chains and required WAL by default. Archive failures
  remain failures and can grow pg_wal beyond max_wal_size; monitor archive health,
  last successful backup, and free disk space. The schedule uses explicit UTC.
- Physical recovery includes roles and password hashes. Meilisearch is derived
  state and needs reconstruction after a database rollback. Object-store files
  and application secrets have independent backup requirements.
- Read-only Astra review confirmed this topology. Findings addressed environment
  selector overrides, existing DDL/PUBLIC authority, column-level ACLs, weekly
  schedule precedence, and the recovery wait/resume boundary.

## Validation

- Local database image build and actual data-service acceptance passed full and
  differential encrypted backups, non-root/read-only scheduler access, latest
  recovery, PITR excluding a later transaction, live-volume restore rejection,
  retained grants, PostgreSQL restart, and authenticated Meilisearch persistence
  after container replacement. The fixture repository is disposable and local.
- The PostgreSQL fixture rejects existing schema CREATE and PUBLIC audit table
  and column mutation. All of these hardened cases passed.
- Local formatting, xtask/migration Clippy, deployment contracts, Python syntax,
  documentation links, and operator recipe parsing passed. Contracts also reject
  a data environment-selector override and a shared dev/prod database volume.
- The environment-only fixture passed actual backup/recovery and persistence
  acceptance. A focused failure check confirmed stdout, stderr, and exception
  redaction for database, repository, and Meilisearch secrets.
- Remote CI found that Compose Bake rejects the `/dev/null` empty build-secret
  default outside the build context. Replaced it with a tracked empty CA file
  inside the context and added a deployment contract for its location and size.
  The managed fixture also copies its public proxy CA inside its build context.
- Local Compose Bake passed the default empty-CA build without extra filesystem
  permissions and actual data acceptance with the managed public proxy CA.
- Remote data-services acceptance passed. The existing k3s
  fixture exposed a scheduling race: production preferred spreading allowed both
  replacement API Pods on one node, but acceptance requires distinct nodes.
  A fixture-only hard spread now selects each Pod-template revision independently
  of terminating Pods. Production scheduling policy stays preferred. Astra
  reviewed this correction; distinct-node, availability, and traffic checks remain.
- Remote [CI run 39108883562](https://github.com/HiruneWorks/hirune-axum-kit/actions/runs/39108883562)
  passed for `3e83972579c55c035b48f26897c2670de37de4ca`. General Rust, DB, API,
  process, file/content, deployment, load, and renamed-template checks passed.
  The canonical release image passed production acceptance and actual encrypted
  full/differential backup, latest/PITR recovery, grants, and search persistence.
- The corrected two-node k3s fixture passed distinct-node API replacement,
  719 authenticated API-rollout requests and 1,146 gateway-rollout requests with
  zero errors, shared sessions/quotas and Valkey, counter failure/recovery,
  dev/prod isolation, and private ingress denial.
- Python, JavaScript/TypeScript, and Rust CodeQL jobs succeeded on the verified
  head. Rust extracted 354 files with zero extraction errors and uploaded its
  analysis successfully. The plaintext-storage review was automatically resolved.
  GitHub's aggregate CodeQL check remained neutral with a missing baseline Rust
  configuration warning. Its managed-job rerun API returned 403; this is not a
  completed aggregate comparison and must not be reported as one.
- No S3/R2 provider, target host firewall, real Tunnel, or live production
  deployment was exercised.

## Remaining work

Rehearse off-host repository access, target-host connectivity/firewall, recovery
time, and operator cutover on the intended infrastructure. Measure memory, connection, and disk-I/O
budgets before production use.
