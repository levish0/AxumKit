# Background Worker

AxumKit runs a separate worker binary (`axumkit-worker`) that processes background jobs via NATS JetStream and runs scheduled cron tasks.

## Architecture

```
axumkit-server                          axumkit-worker
┌──────────────┐                       ┌──────────────────────┐
│   Handler    │                       │  NATS Consumers      │
│      │       │   NATS JetStream      │  ├─ Email            │
│   Bridge ────┼──────────────────────▶│  ├─ Index Post       │
│              │                       │  ├─ Index User       │
└──────────────┘                       │  ├─ Reindex Posts    │
                                       │  ├─ Reindex Users    │
                                       │  └─ Delete Content   │
                                       │                      │
                                       │  Cron Scheduler      │
                                       │  ├─ Cleanup          │
                                       │  ├─ Sitemap          │
                                       │  └─ Orphan Cleanup   │
                                       └──────────────────────┘
```

## NATS JetStream Consumers

Each consumer is a durable pull subscriber with:
- **WorkQueue retention** — messages deleted after ack
- **Exponential backoff** — 1s, 2s, 4s, 8s, 16s (5 retries total)
- **Concurrency control** — via Tokio semaphore
- **30-second ack timeout** — messages redelivered if not ack'd

### Email Consumer

Sends transactional emails via SMTP (Lettre). Templates are rendered with MRML + MiniJinja.

**Subject:** `axumkit.jobs.email`

Email types:
- Verification email (account creation)
- Password reset email
- Email change confirmation

### Index Consumers

Index individual documents in MeiliSearch when created or updated.

**Subjects:**
- `axumkit.jobs.index.post` — Index a single post
- `axumkit.jobs.index.user` — Index a single user

### Reindex Consumers

Bulk reindex all documents. Used for recovery or after schema changes.

**Subjects:**
- `axumkit.jobs.reindex.posts` — Reindex all posts
- `axumkit.jobs.reindex.users` — Reindex all users

### Delete Content Consumer

Removes blobs from SeaweedFS/R2 storage when posts are deleted.

**Subject:** `axumkit.jobs.storage.delete_content`

## Cron Jobs

The worker runs a cron scheduler (`tokio-cron-scheduler`) with configurable timezone (`CRON_TIMEZONE`, default: UTC).

| Job | Schedule | Description |
|-----|----------|-------------|
| Cleanup | Saturday 4:00 AM | Clean up expired tokens and sessions |
| Sitemap | Sunday 3:00 AM | Generate `sitemap.xml` and upload to R2 |
| Orphaned Blob Cleanup | Friday 5:00 AM | Remove SeaweedFS blobs not referenced by any post |

## Generic Consumer Pattern

All consumers use a shared `NatsConsumer` implementation:

```rust
NatsConsumer::new(jetstream, stream_name, consumer_name, concurrency)
    .run(|job: EmailJob| async move {
        // Process the job
        send_email(job).await?;
        Ok(())
    })
    .await?;
```

Message lifecycle:
1. Deserialize JSON payload into job struct
2. Call handler function
3. On success → `ack()` — message removed from stream
4. On failure → `nak()` — message redelivered after backoff delay
5. Bad messages (deserialization failure) → `ack()` to prevent infinite retry

## Worker Context

All consumers share a `WorkerContext` with access to:

```rust
pub struct WorkerContext {
    pub mailer: Mailer,               // SMTP transport
    pub meili_client: SearchClient,   // MeiliSearch SDK
    pub db_pool: DbPool,              // PostgreSQL (write only)
    pub cache_client: CacheClient,    // Redis (cache)
    pub storage_client: StorageClient,// SeaweedFS
    pub r2_client: R2Client,          // Cloudflare R2
    pub jetstream: JetStreamContext,  // NATS JetStream
    pub config: &'static WorkerConfig,
}
```

## Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `NATS_URL` | No | `nats://localhost:4222` | NATS server URL |
| `SMTP_HOST` | Yes | — | SMTP server host |
| `SMTP_PORT` | No | `587` | SMTP server port |
| `SMTP_USER` | Yes | — | SMTP username |
| `SMTP_PASSWORD` | Yes | — | SMTP password |
| `SMTP_TLS` | No | `true` | Enable TLS |
| `EMAILS_FROM_EMAIL` | Yes | — | Sender email address |
| `EMAILS_FROM_NAME` | No | `SevenWiki` | Sender display name |
| `FRONTEND_HOST` | Yes | — | Frontend URL for email links |
| `PROJECT_NAME` | Yes | — | Project name in emails |
| `CRON_TIMEZONE` | No | `UTC` | Timezone for cron schedules |
