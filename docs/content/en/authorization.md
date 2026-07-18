---
title: Authorization (RBAC)
description: Roles, permissions, and ACL groups — Django semantics in Rust.
order: 6
---

Authorization has three layers, all resolved into a per-request `UserContext`.

## Roles

`user_roles` holds coarse grants from the `role` enum — `mod` and `admin` — with an
optional `expires_at` (filtered at read time). Route-level gates
(`require_admin` / `require_mod`) live at the router boundary so they are greppable;
`Admin` passes every role check.

Role management has built-in meta-rules: you cannot manage yourself, admins cannot ban
or demote other admins, and the `Admin` role itself can never be granted or revoked
through the API — blocking both privilege escalation and last-admin lockout.

## Permissions

Fine-grained capabilities are **permission codenames**, defined once as the
`Permission` enum in `constants` and stored as TEXT wherever they are granted:

- `board:pin_post` — pin/unpin/reorder pinned posts
- `board:lock_post` — lock/unlock a post's comment thread
- `board:moderate` — delete/hide other users' posts and comments
- `board:manage` — create/update/delete boards themselves

`UserContext::has_perm` resolves, in order: the **ban hard gate** (a banned user holds
no permissions), the **Admin bypass** (admins pass everything — anti-lockout), the
**Mod default set** (`pin`, `lock`, `moderate`), and finally the union of permissions
granted through group membership. Denials return `403` with code `permission:denied` and the
missing codename, so clients know exactly which capability was absent.

Adding a permission is a code change only — a new enum variant, no migration. A stored
codename that no longer parses never matches any check (fail closed, logged).

## Groups

Groups are Django's group model: named permission bundles with membership metadata.

| Table | Meaning |
| --- | --- |
| `groups` | Name, description, `is_system` (system groups are immutable via the API) |
| `group_members` | One user per row, with `reason`, `expires_at`, `created_by` |
| `group_permissions` | One codename per row, unique per group |

Membership expiry is read-time filtered, so a temporary grant (e.g. "trusted-uploaders
for 30 days") needs no revocation job — a weekly cron just reclaims dead rows.

## Admin API

All mutations are admin-only, moderation-logged with a required reason, and reads
require at least `Mod`:

```
GET  /v0/groups                       list groups
POST /v0/groups                       create (name, description, reason)
POST /v0/groups/delete                delete (system groups refused)
GET  /v0/groups/members?group_id=…    list members (active only, paginated)
POST /v0/groups/members               add member (user or ip, reason, expires_at)
POST /v0/groups/members/remove        remove member
GET  /v0/permissions                  every codename the app defines
GET  /v0/groups/permissions?group_id=…
POST /v0/groups/permissions/replace   whole-list replacement; unknown codenames rejected
```

Permission replacement is whole-list ("submit the desired end state") so admin UIs can
render checkboxes from `GET /v0/permissions` and PUT the result back without
diffing.

## Domain policy objects

Services never sprinkle role checks inline. Each domain defines a policy enum
implementing the `Rule` trait — for boards, `BoardPermission`:

```rust
BoardPermission::Write(facts).check(&ctx)?;          // authed + unbanned + board enabled
BoardPermission::EditContent { is_owner, facts }     // owner-only, held to the write bar
BoardPermission::DeleteContent { is_owner }          // owner, or board:moderate
BoardPermission::PinPost                             // board:pin_post
BoardPermission::ManageBoard                         // board:manage
```

The rules encode policy decisions worth keeping: reads are public and ban-exempt
(bans gate participation, not reading), disabled boards 404 for everyone but admins,
and moderators sanction content but never rewrite someone else's words.
