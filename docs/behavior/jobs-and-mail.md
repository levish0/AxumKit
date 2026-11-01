# Jobs and mail

Application services register a job using their database transaction. A rollback
removes the domain change and job together. Job payloads are typed at the owning
handler and bounded at 16 KiB. Authentication mail jobs carry only a delivery UUID.
Do not put credentials or bodies in queue metadata or tracing fields.

Workers claim ready rows with PostgreSQL row locks and SKIP LOCKED. Every attempt
has a new UUIDv7 claim token and a history row. Heartbeat and completion require
the current token, running state, and a live lease. A stale worker cannot overwrite
a replacement's result. Database time governs lease authority. A corresponding
Tokio monotonic deadline bounds the complete handler/heartbeat operation.

Workers claim ready work immediately while execution slots are available and
refill capacity as running jobs finish. `WORKER_POLL_MILLISECONDS` defaults to
250 and delays another lookup only after no eligible job is found. It does not
pace ready work. An absolute retry deadline prevents task completions from
continually postponing that lookup. A full worker does not claim extra jobs.
Expired-lease recovery continues looking for ready work instead of reporting an
empty queue after recovering one row. Claim transactions remain short and never
hold database locks while a handler runs.

Ordinary registrations use the `application` queue. The maintenance executable
uses the separate `maintenance` queue. Claim and expired-lease recovery both
filter the selected queue. Periodic registration uses database time and a unique
`(queue, schedule_key)` constraint to insert at most one job per interval bucket.
Completed jobs retain that key until normal retention. Scheduling intervals are
at most one day, no longer than the minimum finished-history retention.

Successful work becomes succeeded. Retryable failure becomes pending with
exponential delay, bounded by the attempt budget. Permanent failure or exhausted
attempts become retained failed jobs. An expired claim closes its attempt history
and either schedules a delayed retry or retains failure. The default budget is
five attempts. `just manage retry-job <uuid>` explicitly grants another five
attempts while preserving history, up to the schema's 1000-attempt bound. A failed
job remains retryable only while its retained row exists.

Shutdown stops claiming, drains current work for a bounded period, then cancels
remaining futures. Interrupted claims recover after expiry. A heartbeat cannot
extend a job past its execution deadline. Handlers must respect cancellation.
Blocking operations need their own bound. This is at-least-once execution, not an
exactly-once guarantee for external side effects.

Multiple API instances enqueue into the same database and multiple worker
processes consume that shared queue. Workers need the same supported job kinds.
no API-to-worker affinity or manual job partition is required. Each worker's
concurrency limit applies locally, so budget the total database connections and
external-service capacity across all replicas.

Handlers return a stable error code and whether failure is retryable. The shared
runtime records the attempt and chooses delayed retry or retained failure. A
normal handler failure does not stop unrelated work. A lost lease cannot finalize
the old attempt. The runtime logs finalization/storage errors. Lease recovery
remains authoritative. A database error while claiming exits the worker with an
error. The deployment supervisor restarts it. This worker policy is separate from the
HTTP adapter's common status/code/request-ID error response.

## Queue verification

The PostgreSQL queue remains the background-work transport. Business changes and
job registration continue to share one transaction. The baseline does not require
a separate broker or publication relay.

`tests/jobs/pressure.rs` verifies shared commit visibility, rollback, row-lock
release while handlers are saturated, bounded claims, and multi-batch retention.
`just test-job-load` records queue throughput, bulk and notification wait times,
and foreground database latency with retained job history. See
[job load](../development.md#postgresql-job-load) for its profiles and evidence
limits.

## Authentication mail

Purpose-specific MJML and plain-text templates live under `crates/mail/templates/`:
`verify-email`, `reset-password`, `welcome`, and `password-changed`. Signup sends
verification instructions. Successful verification or new verified OIDC signup
registers the welcome message. A completed password change, initial password, or
reset registers a security notification, never the password itself. These notifications have no
action token and expire after 24 hours. All registrations commit with the owning
account change and use the same encrypted delivery/job path.

The identity domain prepares an encrypted, short-lived delivery in the business
transaction. The mail crate renders MJML HTML and plain text through strict
Minijinja templates and sends through lettre SMTP. Pending delivery secrets use
key IDs and delivery-specific encryption context. Expired or invalidated actions
are not prepared for sending. Acceptance clears stored ciphertext.

SMTP runs outside database transactions. Acceptance means the SMTP server accepted
the message, not that it reached an inbox. A crash after acceptance but before
recording it can resend. Reuse the delivery UUID in Message-ID. Do not assume this
forces downstream deduplication. Action-token consumption remains single use.
Account/challenge authority is checked again when the recipient uses the action.

## Retention and maintenance

`manage cleanup` removes expired authentication rows, finished jobs and their
attempt history, expired audit events, and due file lifecycle records in bounded
batches. Retention defaults are `SUCCEEDED_JOB_RETENTION_DAYS=30`,
`FAILED_JOB_RETENTION_DAYS=90`, and `AUDIT_RETENTION_DAYS=180`. Each accepts 1 through
36500 days. Measure job age from `finished_at` and audit age from `created_at`.

Each invocation deletes at most 100 finished jobs, with at most 1000 attempt rows
per job removed through the foreign key cascade, and at most 1000 audit events.
Pending and running jobs are never retention candidates. Final job deletion repeats
the terminal-state and age predicates after lock contention, preserving a job that
was retried or failed again recently while cleanup waited. Deleting a retained
failed job also removes its inspection history and ability to retry it. Export or
investigate it before the configured deadline.

The portable `maintenance` executable registers periodic cleanup and search repair
in its dedicated queue and consumes it through the normal fenced runtime. It runs
with the restricted maintenance role. Multiple replicas deduplicate each bucket's
registration. Repeat execution and overlap across different buckets remain
possible. Both cleanup and repair are bounded and repeatable. Keep the interval
consistent across replicas. A fatal scheduler or consumer error stops both so the
deployment supervisor can restart the process.

Use `just cleanup <environment>` from `deploy/` for a one-off cleanup with the
same restricted role. API, worker, and ordinary management commands keep
append-only audit privileges. The [deployment templates](../../deploy/README.md)
start maintenance as a normal service. A busy backlog may require shorter intervals
or explicit extra runs. Review retention against the consuming application's
requirements before deployment.
