# Identity

Accounts use normalized email as a unique login identifier. Entity IDs are UUIDv7.
Password signup creates an unverified account that cannot sign in. It queues a
verification email in the same transaction. Verification consumes one expiring
challenge and registers a welcome email in the same transaction.
Resending replaces the previous verification challenge and only sends for pending
accounts. Duplicate signup and ineligible resend/reset requests return accepted
without disclosing whether an account exists. When Turnstile is configured,
signup, verification resend, password-reset requests, and password login first
require a valid `human_verification_token`. See
[human verification](../security.md#human-verification).

Password login verifies credentials before locking, then rechecks the account and
credential snapshot under lock. An account must be active and verified. If TOTP
is enabled, primary login returns a short-lived MFA challenge rather than a
session. Global administrators must have confirmed TOTP enrollment. Password and
OIDC login use this same final gate. Session issuance, session authentication, and
final service checks also reject administrators without MFA. The baseline does
not send a separate confirmation email for every new device.

New passwords require at least 15 Unicode characters and at most 1024 UTF-8 bytes.
They are not trimmed, truncated, or subject to composition rules. Argon2id hashing
uses a random salt. There is no external password-screening service or local
common/breached-password blocklist.

Sessions are opaque credentials backed by PostgreSQL, with idle and absolute
expiry. The defaults are 14 days idle and 90 days absolute for both cookie and
bearer sessions, so native clients stay signed in between uses. Recent
authentication is a separate five-minute window from the last primary login.
Activity extends idle expiry at most once per minute and never beyond the
absolute deadline. Rotation replaces the credential. The old token stops working
across instances. Lists and revocation are owner-scoped. Account deactivation,
password reset/change, an initial password, and MFA configuration changes
invalidate existing sessions and outstanding challenges. Deactivation is not
physical data erasure.
Authentication maintenance deletes expired rows in bounded batches. Session
deletion rechecks both deadlines after a concurrent activity update releases its
row lock, so an accepted idle extension cannot be removed from an old snapshot.

Password reset is available for verified active accounts with a password. It is
single use, changes the password, and invalidates sessions in one transaction.
It does not remove TOTP. OIDC-only accounts do not gain a password through reset.
Changing a password requires the current password and recent authentication.
Successful password changes, initial passwords, and resets register a tokenless
security notification in that transaction. See [mail delivery behavior](jobs-and-mail.md).

## Initial password for OIDC-only accounts

An account created through OIDC has no password. Its owner can add one with
`POST /api/v1/account/password/initial`. `GET /api/v1/account` returns
`has_password`, so a client can choose this operation or a password change.

This operation follows the password-change rules, except that no current
password exists to verify. It requires a valid session with recent authentication
(within five minutes) and applies the new-password rules. It has its own identity
rate-limit key. The service hashes the password before it locks the account.
Under the account lock, it rechecks the session, security version, and recent
authentication. If the account already has a password, the service returns
`409 conflict` and changes nothing.

One transaction stores the hash, advances the security version, revokes all
sessions and challenges, registers the password-change security notification,
and appends an `initial_password_set` audit event. The current session is also
revoked, and the response clears the session cookie. The owner then signs in
again with the password or the OIDC provider. After this change, password reset
is available, and the owner can unlink the last external identity.

## MFA and recovery

TOTP enrollment requires recent authentication, an expiring setup challenge, and
a valid authenticator code. The service enables MFA only after confirmation.
The caller receives ten recovery codes once. The service stores only their hashes. TOTP secrets are
encrypted with the active application key and account-specific context.

Accepted TOTP steps are tracked on the account, so reusing a step through another
challenge fails. Recovery codes and MFA challenges are consumed atomically under
the account lock. Removing MFA requires recent authentication plus an unused
TOTP step or recovery code, and revokes sessions. Global administrators cannot
remove MFA. Keep the returned recovery codes outside the authenticated browser
session.

`POST /api/v1/account/recovery-codes` requires recent full authentication within
five minutes and an unused TOTP step or recovery code. Under the account lock it
consumes the proof, replaces all ten recovery-code hashes, advances the security
version, revokes sessions/challenges, and appends an audit event in one transaction.
It returns plaintext replacement codes once with `no-store` and clears the cookie.
Concurrent regeneration cannot expose two usable generations. A stale session or
previous recovery code cannot authorize another regeneration. Sign in again after
saving the new codes. Support-assisted factor reset is not a baseline operation.

## Administration, limits, and retention

`manage set-administrator <uuid> <true|false>` is an operator-only command with no
public HTTP counterpart. Promotion requires an active, verified account with TOTP
already enrolled. A changed flag advances the security version, revokes existing
sessions/challenges, and records `administrator_granted` or `administrator_removed`
atomically. Repeating the current flag makes no change. Organization ownership
does not grant this global role. An administrator missing TOTP must be demoted by
an operator before signing in, enrolling TOTP, and being promoted again.

Authentication budgets are shared PostgreSQL counters. The identity limit defaults
to 10 attempts per operation/identity per minute. The client limit defaults to 60
requests shared across authentication routes per minute. Both are configurable
from 1 to 100000. Failed attempts persist independently of an authentication
transaction rollback. Counter saturation is shared across configurations and
exceeds the maximum allowed budget. Lower instance limits cannot erase attempts.

Audit events are append-only to runtime processes. Explicit maintenance deletes
up to 1000 events older than `AUDIT_RETENTION_DAYS` per invocation, ordered by
creation time and UUID. The default is 180 days. The supported range is 1 through
36500 days. The deployment maintenance role has the required deletion grant and
no schema-owner privileges. Authentication cleanup retains its per-table bounds.

## OIDC

`OIDC_PROVIDERS` configures multiple named providers together. Login and linking
select a provider through the existing `{provider}` route parameter. Names must
be unique. Each entry has its own issuer, client credentials, and callback URL.
No provider is enabled by default. See [runtime configuration](../runtime.md).

Each new verified OIDC account registers a welcome email in the account-creation
transaction. A returning identity or a link operation does not register another
welcome.

Configured OIDC providers use authorization code flow and PKCE S256. State,
nonce, verifier, and browser binding persist in a five-minute flow. The service
encrypts sensitive flow material. Another API instance can handle the callback.
Consume callback state before external code exchange, including cancellation.
Retry an interrupted login by starting a new flow, not replaying its code.

The protocol library checks signed ID token issuer, audience, nonce, signature,
expiry, and access-token hash when present. Provider metadata/JWKS refresh after
five minutes and on an unknown signing key, with serialized refresh and a five-second
retry cooldown measured from completion or cancellation. Transport/provider
failures return unavailable. Invalid grants and claims return unauthenticated.
The callback handler checks optional `iss` parameters.

Match a returning identity by exact issuer and subject. A new identity
requires a verified provider email and cannot silently claim an existing local
email address. Linking starts from a recent authenticated session, and callback
rechecks that initiating session and security version. Unlinking cannot remove an
account's last login method. Provider access tokens are not stored.
The same subject from different issuers identifies different external identities.
A callback sent to another configured provider is rejected without consuming the
valid initiating flow. Login can start on one API replica and finish on another.

### Native ID token login

Native applications sign in with the provider SDK, not a browser redirect. A
provider supports this only when its `native_client_ids` list is not empty.
Otherwise both native routes return `404 not_found`.

1. The client calls `POST /api/v1/auth/oidc/{provider}/native`. The API stores
   hashes of a new `flow` secret and `nonce` in a five-minute native flow and
   returns both values.
2. The client passes `nonce` unchanged to the provider SDK sign-in request. It
   keeps `flow` and does not send it to the provider.
3. The client sends `flow` and the returned `id_token` to
   `POST /api/v1/auth/oidc/{provider}/native/complete`.

Completion consumes the flow before it parses or verifies the token, so a
rejected token requires a new flow. A flow sent to another provider is rejected
without being consumed. The protocol library verifies the signature, issuer,
expiry, and nonce. The service then requires every `aud` value and any `azp`
value to be the provider `client_id` or a configured native client ID. A token
with multiple audiences must contain `azp`. An unknown signing key refreshes
provider keys once, with the same cooldown as browser login.

A verified token uses the browser login rules for returning identities, new
accounts, the welcome email, and the MFA gate. Completion always uses bearer
transport. An `mfa_required` result continues at `POST /api/v1/auth/mfa` with
`transport: "bearer"`. Browser and native flows are stored separately, so neither
completion route accepts the other flow. Native routes do not link identities.
Link from an authenticated browser session.

These controls are exercised in `tests/identity.rs`, `tests/api.rs`,
`tests/oidc.rs`, and `tests/process.rs`. Live provider registration, consent,
redirect configuration, and email delivery outside Mailpit remain deployment
acceptance work.
