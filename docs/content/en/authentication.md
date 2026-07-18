---
title: Authentication
description: Sessions, TOTP, device verification, and OAuth.
order: 5
---

## Sessions

Signup is **deferred**: `POST /v0/auth/signup` stores a pending payload in Redis and
emails a verification token; the account row is only created when
`POST /v0/auth/verify-email` consumes it. This keeps unverified signups out of the
database entirely and makes signup naturally enumeration-safe.

A session is an opaque 256-bit bearer token. Only its BLAKE3 hash is stored
(`session:{hash}` in Redis), so a Redis snapshot never yields replayable credentials.
Sessions carry:

- a **sliding TTL** refreshed near expiry, capped by an absolute maximum lifetime;
- a **management id** — a separate identifier used by `GET /v0/auth/sessions` and
  `DELETE /v0/auth/sessions/{management_id}`, so listing/revoking sessions never
  exposes the bearer token hash.

Browsers carry the token in an `HttpOnly` cookie (`__Host-` prefixed in production);
native apps use the same endpoints under `/v0/app/auth/*` and replay the token as
`Authorization: Bearer`.

Credential changes invalidate sessions aggressively: password reset and email change
kill **all** sessions; password change keeps only the current one. Owners are notified
by email of every credential change, and an `auth_events` audit row is written
(login success/failure, password/email changes, TOTP toggles, new-device logins).

## TOTP 2FA

`POST /v0/auth/totp/setup → enable` provisions RFC 6238 TOTP. Secrets are AES-256-GCM
encrypted at rest with a key derived from `TOTP_ENCRYPTION_KEY`; verified codes are
claimed in Redis for their validity window so a code can never be replayed. Ten
single-use backup codes are stored as keyed hashes and consumed under row locks —
two concurrent uses of the same code cannot both succeed. When TOTP is enabled, login
returns `202` with a short-lived temp token that `POST /v0/auth/totp/verify` exchanges
for the real session.

## New-device verification

After full credential (and TOTP) verification, a login from an unrecognized device is
**withheld**: the server emails a single-use challenge and returns `202` instead of a
session. `POST /v0/auth/device/verify` consumes the emailed token, registers the
device (stored by token hash in `known_devices`), and mints the session plus a
long-lived device cookie. Native apps do the same dance with an `X-Device-Token`
header.

## OAuth (Google, GitHub, Google One Tap)

The authorization-code flow uses PKCE plus a single-use, hashed, TTL-bound `state`
that is also bound to the caller's anonymous cookie — a login-CSRF defense. Accounts
are **never auto-linked by email**: a collision surfaces an explicit error, and
linking a provider to an existing account is a separate, session-authenticated flow.
Unlinking refuses to remove the last remaining auth factor. Google ID tokens are
verified against Google's JWKS with pinned issuer/audience and rate-limited forced
refresh; One Tap nonces are single-use and bound to the anonymous id.

New OAuth users go through `POST /v0/auth/complete-signup` to pick a handle; the
pending payload is protected by a Redis lock and an idempotent completion state, so a
lost response can be retried safely.
