---
title: Background jobs
description: The NATS JetStream queue, the worker, and cron.
order: 8
---

Anything slow, flaky, or retryable runs in the **worker**, not the request path.

## The contract crate

`job_queue` is the entire server↔worker boundary: payload structs, stream/subject/
consumer names, and idempotent stream creation. The server publishes through
`bridge/worker_client/` (serialize → publish → await the JetStream ack); the worker
re-exports the same payload types from its handlers. Both binaries run
`initialize_all_streams` at startup, so a fresh NATS works no matter which process
boots first.

Current jobs: transactional **email** (MJML templates rendered once and cached,
minijinja variables HTML-escaped), **user indexing** and batched **user reindex**
(self-enqueueing batches building a temp index, atomically swapped in), and **OAuth
avatar processing** (SSRF-guarded fetch → media processor → content-addressed R2
upload).

## The consumer engine

Every consumer is a durable JetStream pull consumer driven by one generic engine with
the failure semantics already worked out:

- **Retries** — explicit backoff (1/2/4/8/16s); a failed message is `Nak`'d and
  redelivered up to 6 total attempts.
- **Dead-letter queue** — permanently failed or unparsable messages are republished to
  the DLQ stream (14-day retention) with origin headers; the original is only dropped
  once the DLQ copy is server-acknowledged, so a job is never lost with no copy
  anywhere.
- **Heartbeats + timeout backstop** — long handlers send in-progress acks so they
  aren't redelivered mid-run, and the whole handler is bounded (default 300 s, 900 s
  for reindex batches) so a hung external call cannot pin a concurrency slot forever.
- **Panic isolation** — a panicking handler flows through the normal failure path
  instead of wedging the queue.
- **Per-message dedup** — non-idempotent handlers (email, avatar processing, reindex)
  mark each processed stream sequence in the noeviction Redis before acking, so a
  redelivery after a lost ack is skipped rather than repeated.
- **Config propagation** — consumers are created with create-or-update semantics, so
  tuning changes actually reach existing durables on redeploy.

A supervisor restarts any consumer that exits or panics; on SIGTERM the worker stops
pulling, drains in-flight handlers (bounded 30 s), and shuts the cron scheduler down
so no job strands its distributed lock.

## Cron

`tokio-cron-scheduler`, with multi-instance safety via Redis locks (`SET NX` + lua
heartbeat extension; TTL expiry handles release, absorbing clock skew):

| Job | Schedule | What it does |
| --- | --- | --- |
| cleanup | Sat 04:00 | Batched deletes: expired group memberships, roles, and bans; notifications older than 90 days |
| sitemap | Sun 03:00 | Generates and uploads a sitemap to R2 |
| view-count flush | every minute | Atomic drain (HGETALL + DEL) of buffered board view counts into Postgres — needs no lock, the drain itself is the mutual exclusion |

## Adding a job

1. Define the payload + stream/subject/consumer names in `job_queue`.
2. Write the handler + `run_consumer` in `worker/src/jobs/…`, register the consumer
   kind in the worker's `main.rs`.
3. Add a publish function in `server/src/bridge/worker_client/`; call it inline when
   the user must see failures (signup email), or via a post-commit `tokio::spawn`
   wrapper when it's best-effort (indexing, notifications).
