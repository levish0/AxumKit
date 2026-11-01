# Notifications and optional FCM

## Inbox and preferences

When someone comments on a published organization post, the post author receives
one `post_comment` notification if still an active, verified member and comment
notifications are enabled. Self-comments do not notify. The comment, inbox row,
eligible per-device delivery records, and their jobs commit together. A unique
comment/recipient constraint absorbs duplicate event delivery.

The inbox stores actor, recipient, organization, post, and comment UUIDs rather
than copied content. Inbox list/detail, unread count, and read operations share a
current-visibility query: recipient ownership, current membership, published
parent, and nondeleted comment. Unpublishing/deletion or membership removal hides
the notification and removes it from the unread count immediately. The inbox
preserves old read state if the target becomes visible again.

`post_comments` defaults true. `push_enabled` defaults false. Disabling comment
notifications suppresses new inbox and push records. Existing inbox rows remain.
Push preferences are checked again before sending. Reading an item is idempotent.
`POST /api/v1/notifications/read-batch` marks at most 1,000 currently visible
unread items and returns `updated`. Repeat until zero to clear a large inbox.

## Optional provider and devices

FCM is disabled by default. Set `FCM_ENABLED=true` and `FCM_PROJECT_ID` to enable
device registration and durable delivery. Authentication uses lazy Google
Application Default Credentials through `gcp_auth`. Use a workload's metadata
identity where available, or mount a service-account credential file and set
`GOOGLE_APPLICATION_CREDENTIALS`. No provider token is fetched during application
bootstrap. Disabled FCM requires no Google credentials and does not enqueue push
jobs. An enabled API and disabled worker is a deployment mismatch. Pending push
jobs fail with `fcm_disabled`. Correct the configuration, then retry those jobs.

Authenticated clients register an FCM token and platform (`android`, `ios`,
`web`). Registration represents that exact account and login session. Tokens are
encrypted at rest with the existing key ring and indexed by domain-separated
BLAKE3 hashes. Device/list DTOs and logs never expose tokens. Each account has at
most 20 live registrations. Registration expires 30 days after renewal.

Renewing the same live token/account/session/platform preserves its generation.
Ownership/session/platform changes or revival after expiry increment it.
Pending deliveries capture the generation and are never retargeted to a new
binding. Deleting a session cascades its devices and their pending delivery
records. Security-version changes, session expiry, recipient suspension, current
MFA gates, preferences, read state, and content access are rechecked before send.

## Delivery and retention

Workers lock recipient account, organization, then device as needed, recheck
authority, commit, and perform provider I/O outside locks. Comment creation never
locks another recipient account after its organization lock.

FCM HTTP v1 receives only generic notification text plus notification/delivery
UUIDs. The client must authenticate and fetch the current notification/target
before showing private content. Authorization or logout can race network delivery.
The generic payload avoids sending copied private text in that window.

Per-device success is durable. FCM acceptance and the database update cannot be
atomic. A crash between them can produce a duplicate. Clients deduplicate stable
delivery IDs. Do not interpret job fencing as provider exactly-once delivery.

`UNREGISTERED` removes only the still-matching device generation. Generic
`INVALID_ARGUMENT` does not delete a token. Provider 429 and server errors are
retryable. Push delivery preserves Retry-After across ordinary retries and
crashed-worker recovery, bounded to one day. A 429 without a header waits at least 60 seconds.
Other provider rejections remain visible as failed jobs for operator correction.
The existing five-attempt budget and manual retry command apply.

Maintenance removes up to 1,000 expired device registrations and 1,000 old inbox
rows per run. `NOTIFICATION_RETENTION_DAYS` defaults to 90. Deleting inbox rows
cascades to their delivery rows. These limits are separate from job/audit retention.

## References

- [FCM HTTP v1](https://firebase.google.com/docs/cloud-messaging/send/v1-api)
- [FCM registration management](https://firebase.google.com/docs/cloud-messaging/manage-tokens)
- [FCM error codes](https://firebase.google.com/docs/cloud-messaging/error-codes)
- [gcp_auth ADC support](https://docs.rs/gcp_auth/latest/gcp_auth/)
