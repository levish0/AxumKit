# Background Worker

AxumKit runs a separate worker binary (`worker`) that processes background jobs via NATS JetStream and runs scheduled cron tasks.

## Architecture

```
server                                  worker
+--------------+                        +----------------------+
|   Handler    |                        |  NATS Consumers      |
|      |       |   NATS JetStream       |  |- Email             |
|   Bridge ----+----------------------->|  |- Index User        |
|              |                        |  `- Reindex Users    |
+--------------+                        |                      |
                                        |  Cron Scheduler      |
                                        |  |- Cleanup           |
                                        |  `- Sitemap           |
                                        +----------------------+
```

## NATS JetStream Consumers

Each consumer is a durable pull subscriber with:

- WorkQueue retention (messages deleted after ack)
- Exponential backoff: 1s, 2s, 4s, 8s, 16s (5 retries)
- Concurrency control via Tokio semaphore
- 30-second ack timeout

### Email Consumer

Sends transactional emails via SMTP (Lettre). Templates are rendered with MRML + MiniJinja.

Subject: `axumkit.jobs.email`

### User Index Consumer

Indexes user documents in MeiliSearch when users are created or updated.

Subject: `axumkit.jobs.index.user`

### User Reindex Consumer

Bulk reindexes all users in MeiliSearch.

Subject: `axumkit.jobs.reindex.users`

## Cron Jobs

The worker runs `tokio-cron-scheduler` with configurable timezone (`CRON_TIMEZONE`, default: `UTC`).

| Job | Schedule | Description |
|-----|----------|-------------|
| Cleanup | Saturday 4:00 AM | Clean up expired auth/session data |
| Sitemap | Sunday 3:00 AM | Generate `sitemap.xml` and upload to R2 assets bucket |

## Worker Context

All consumers share a `WorkerContext` with SMTP, database, Redis cache, R2 client, JetStream, and runtime config.

## Configuration

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `NATS_URL` | No | `nats://localhost:4222` | NATS server URL |
| `SMTP_HOST` | Yes | - | SMTP server host |
| `SMTP_PORT` | No | `587` | SMTP server port |
| `SMTP_USER` | Yes | - | SMTP username |
| `SMTP_PASSWORD` | Yes | - | SMTP password |
| `SMTP_TLS` | No | `true` | Enable TLS |
| `EMAILS_FROM_EMAIL` | Yes | - | Sender email address |
| `EMAILS_FROM_NAME` | No | `SevenWiki` | Sender display name |
| `FRONTEND_HOST` | Yes | - | Frontend URL for email links |
| `PROJECT_NAME` | Yes | - | Project name in emails |
| `R2_ENDPOINT` | Yes | - | R2-compatible S3 endpoint |
| `R2_REGION` | No | `auto` | R2 region |
| `R2_ACCESS_KEY_ID` | Yes | - | R2 access key |
| `R2_SECRET_ACCESS_KEY` | Yes | - | R2 secret key |
| `R2_ASSETS_BUCKET_NAME` | Yes | - | R2 assets bucket |
| `R2_ASSETS_PUBLIC_DOMAIN` | Yes | - | Public domain for assets |
| `CRON_TIMEZONE` | No | `UTC` | Timezone for cron schedules |
