---
title: Boards & notifications
description: The demo domain — posts, comments, pins, and the notification inbox.
order: 7
---

The board domain exists to show every template pattern working end to end: RBAC-gated
writes, owner checks, moderation, counters, background fan-out, and notifications.
Three boards are seeded: `notice`, `general`, and `qna`.

## Boards, posts, comments

```
GET  /v0/board/list, /v0/board/by-slug?slug=…
POST /v0/board, /board/update, /board/delete        (board:manage)

POST /v0/board/post                                 create (authed, unbanned)
POST /v0/board/post/update                          owner-only
POST /v0/board/post/delete                          owner or board:moderate
POST /v0/board/post/pin | unpin | reorder-pins      board:pin_post
POST /v0/board/post/lock | unlock                   board:lock_post
GET  /v0/board/post/list, /v0/board/post            reads (public)

POST /v0/board/comment, /comment/update, /comment/delete, GET /comment/list
```

Details that carry over to real projects:

- **Content is stored raw** — no server-side markup pipeline. Render clientside, or
  plug your own renderer in where the response mappers live.
- **Reply depth is capped at 2** (YouTube-style): a reply to a reply attaches to the
  same thread root, so pagination and reply counts stay simple.
- **Pinned posts** hold an explicit position; `reorder-pins` takes the full desired
  order and rejects a stale set (`board:pin_set_mismatch`) so two moderators cannot
  silently clobber each other.
- **View counts are buffered**: each viewer counts once per 6-hour window (deduped in
  Redis by user id or IP), increments land in a Redis hash, and a worker cron drains
  the hash atomically every minute into `board_posts.view_count`. A Redis hiccup can
  never fail a read.
- Posts and comments are attributed to **actors** (user/anonymous/system), which is
  what lets a deleted user's content survive with a masked identity.

## Mentions

`@handle` tokens in post/comment content are resolved to users (capped at 10 per
body); each resolved mention produces a `user_mentioned` notification, excluding the
author. Edits only notify **newly added** mentions — re-saving a post never re-pings
everyone already mentioned.

## Notification inbox

Notifications are two tables: one `notification_events` row per event, one
`notification_deliveries` row per recipient (read state lives on the delivery). The
event's target columns (`board_id`/`post_id`/`comment_id`) are shape-checked by the
database per `target_kind`, so a malformed event cannot be inserted at all.

Current producers: `board_comment_created` (someone commented on your post) and
`user_mentioned`. Everything goes through one chokepoint (`service_notify_user` /
`notify_mentions`), which drops self-notifications and respects per-action opt-outs.

```
GET  /v0/notifications/list                cursor-paginated, filterable
GET  /v0/notifications/unread/count
POST /v0/notifications/mark-as-read, /mark-all-as-read, /delete
GET|PUT /v0/notifications/preferences      channel flags
GET|PUT /v0/notifications/preferences/actions   per-action opt-out
```

Old notifications are reclaimed by the weekly cleanup cron (90-day retention).

## Search

User search is served from Meilisearch (`GET /v0/search/users`). The index schema is
the shared `search_index` crate: the worker writes `SearchUser` documents on signup
and profile changes, the server reads the same struct back, and a full reindex
(admin-triggered) rebuilds into a temp index and atomically swaps it in.
