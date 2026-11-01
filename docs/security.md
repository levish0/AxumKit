# Security controls

This template uses OWASP guidance as a review baseline. It does not claim ASVS
certification. Review the controls against your application's threat model.
Check the actual source and tests when changing authentication. UUIDv7 is an entity identity,
not a secret or an access-control mechanism.

## Required review checks

| Area | Contract and evidence |
| --- | --- |
| Passwords | At least 15 Unicode characters, at most 1024 UTF-8 bytes, no truncation or composition rule. Argon2id uses 19 MiB, 2 iterations, and 1 lane with random salt. Blocking work retains its concurrency permit until it actually finishes. |
| Enumeration | Signup, verification resend, and reset requests return the same accepted response for eligible and ineligible accounts. Invalid logins share an authentication error; missing accounts use dummy password verification. |
| Abuse limits | PostgreSQL stores keyed hashes of client and identity rate keys. Client requests are limited before hashing/provider requests; identity limits survive failed authentication. Defaults are 60 requests per client and 10 attempts per operation/identity per minute, with explicit settings. There is no attacker-exhaustible global login quota. |
| Human verification | Optional Cloudflare Turnstile, disabled unless `TURNSTILE_SECRET_KEY` is set. Then signup, verification resend, password-reset requests, and password login need a token that passes server-side siteverify (success, configured hostnames, per-operation action) before the identity service runs. Provider failures fail closed. See [human verification](#human-verification). |
| Sessions | Random 256-bit opaque credentials; only hashes stored. Shared database authority, idle and absolute expiry (defaults 14 and 90 days), rotation, per-session and all-session revocation. Credential changes increment account security version and revoke existing sessions/challenges. |
| External API keys | Organization-owned principals, explicit scopes, expiring 256-bit random secrets with domain-separated hashes, one-time issuance, revocation, and atomic shared request accounting. New clients have zero allocation; only trusted operators grant quotas. See [API access](behavior/api-access.md). |
| MFA | TOTP is optional for ordinary accounts and required for global administrators. Secrets are encrypted; accepted TOTP steps and hashed recovery codes are single use. Password and OIDC login share the gate, which is also checked at session issuance, session authentication, and final service authorization. New-device email challenges are excluded. |
| Sensitive actions | Require a valid session and recent full authentication (within 5 minutes), with final checks under account lock. Password changes require the current password. An OIDC-only account can add a first password with recent authentication; an existing password returns 409. MFA removal and recovery-code regeneration additionally require an unused TOTP step or recovery code; administrators cannot remove MFA. |
| OIDC | Maintained protocol library; authorization code flow, PKCE S256, state, browser binding, nonce, issuer, audience, signature, expiry, and access-token hash validation. Native SDK ID tokens require opt-in native client IDs, a server-issued single-use nonce, a flow secret held by the client, and configured values for every `aud` and any `azp`. Explicit linking only; matching email does not merge accounts. |
| Mail secrets | Challenge hashes in authority tables; encrypted short-lived pending mail payloads with key IDs and row-specific context. Clear ciphertext after acceptance or expiry. Links use fragments; never log them. |
| Authorization | Cedar schema and policies validated at startup. Evaluation diagnostics fail closed. Membership-scoped list queries and explicit actor/resource checks. |
| Browser transport | Secure host-only production cookies, exact Origin validation, explicit CORS, bounded body size, no-store, and no secret response logging. |
| Failures | Storage and identity-provider outages are unavailable errors, never a successful anonymous fallback. Failed OIDC code exchanges consume state; start a new flow. |
| Input validation | JSON/query DTOs validate after typed deserialization and reuse domain value rules. Validation error parameters may contain secrets; return generic input errors without logging or serializing those parameters. Services remain responsible for mutable authority and repeat value checks for non-HTTP callers. |
| Retention | Cleanup defaults to 30 days for succeeded jobs, 90 days for failed jobs, and 180 days for audit events. Pending/running jobs are preserved. Audit deletion uses a separate maintenance role; runtime audit access remains append-only. |

See [identity behavior](behavior/identity.md), [transactions](transaction-boundaries.md),
and the [regression inventory](regressions.md) for executable coverage. A test name
describes intended coverage. Milestone notes record whether the test actually ran.

## Proxy and cache trust

Production ingress passes through Cloudflare Tunnel and APISIX. The API trusts
`CF-Connecting-IP` because the tunnel is the only public ingress. Keep APISIX and
the API off public listeners. In Kubernetes, use private Services and restrict
backend access to the gateway. A public origin that bypasses Cloudflare violates
this contract. No proxy IP allowlist is required.

An SSR server can preserve the original client IP with `X-Real-Client-IP` and
`X-Internal-Secret`. Configure the same random `INTERNAL_PROXY_SECRET` in the SSR
server and the API. It must contain at least 32 visible ASCII characters. The API
compares it in constant time. A matching secret permits the `X-Real-Client-IP`
override. Missing, incorrect, or repeated secret headers do not permit an override.
Never expose this secret to browsers or mobile clients.

Otherwise, the API uses `CF-Connecting-IP`, then the connection peer when that
header is absent, such as in local development. The selected IP header must have
exactly one valid address. Invalid addresses, lists, and repeated selected headers
return the common 400 error. IPv4-mapped IPv6 addresses share the IPv4 budget.
`X-Forwarded-For` does not affect client identity. `TRUSTED_PROXY_IPS` is no longer
supported; remove it from existing settings.

APISIX must preserve the Cloudflare and SSR headers and apply the same identity
precedence for any edge IP limit. The SSR server must replace client-supplied
override headers with the client IP obtained from its own trusted ingress.
NAT users share a client budget. Tune the application policy for its audience.
Network/WAF limits protect PostgreSQL from volumetric attacks before application
authentication or accounting runs.

The k3s profile enforces a shared IP limit through APISIX and Valkey. The gateway
canonicalizes the selected address into a private request-context variable;
caller headers cannot replace the counter key. Exact GET/HEAD liveness and
readiness paths bypass this edge limit. Valkey timeouts relax only this broad
limit. Authentication limits and strict API allocations stay in PostgreSQL.
Gateway errors use the common JSON error envelope and a generated request ID.
Edge 429 responses include `Retry-After` and exact-origin credentialed CORS.
Backend 429 bodies, IDs, and quota headers pass through; the gateway exposes
`Retry-After` to allowed browser origins. See [edge operations](../deploy/k3s/README.md#shared-edge-limits).

PostgreSQL is authoritative for authentication state. The
OIDC public-key cache has a bounded lifetime, serialized refresh, unknown-key
refresh, and failed-refresh cooldown. It never caches session authority. Do not
add cached allow decisions without an explicit revocation contract.

`AUTH_IDENTITY_LIMIT_PER_MINUTE` defaults to 10 for each operation/identity;
`AUTH_CLIENT_LIMIT_PER_MINUTE` defaults to 60 shared across authentication routes
for each client. Both accept 1 through 100000. Counters are shared across processes
and use a common saturation ceiling above every supported budget, so a lower-limit
instance cannot reset or truncate a higher-limit instance's counter. Keep instance
settings consistent. Changing a budget does not reset the active counter window.

## Human verification

Cloudflare Turnstile is an optional abuse control for public identity
operations. It is disabled by default. When `TURNSTILE_SECRET_KEY` is empty or
unset, the API ignores `human_verification_token` and behaves as before.
Startup never contacts Cloudflare.

When the secret is set, these operations require a token in the JSON body:

| Operation | Turnstile widget `action` |
| --- | --- |
| `POST /api/v1/auth/signup` | `signup` |
| `POST /api/v1/auth/verification` | `email_verification` |
| `POST /api/v1/auth/password/reset` | `password_reset` |
| `POST /api/v1/auth/login` | `login` |

The API checks the shared client budget and the JSON body first. The handler then
sends the token to siteverify before it calls the identity service. A rejected
request therefore does not reach password hashing or the account lock, and it
does not consume the identity budget. The server sends the secret, the token,
the client IP from the trusted-proxy contract when known, and a random
`idempotency_key`. After a transport error, a timeout, HTTP 429 or 5xx, or the
`internal-error` code, the server retries once with the same key, so Cloudflare
does not report the retry as a reused token. Each attempt has a five-second
deadline. Redirects are not followed, and the response body is limited to 16 KiB.

The token passes only when `success` is true, the `hostname` matches one of
`TURNSTILE_EXPECTED_HOSTNAMES` (when set), and the `action` equals the operation's
action (unless `TURNSTILE_VERIFY_ACTION=false`). A missing, rejected, expired, or
reused token returns `403 human_verification_failed`. A token longer than 2048
bytes returns `400 invalid_input`. A provider outage, an invalid response, or a
rejected secret returns `503 human_verification_unavailable`; the API fails
closed. Logs contain only a fixed failure category, never the token, the secret,
or the provider response.

A token is single use. If the identity operation fails after verification, for
example because the password is wrong, the client must reset the widget and get
a new token. Native clients that log in with `transport: "bearer"` must also
supply a token when Turnstile is enabled.

OIDC login does not use Turnstile, for browser redirects or native ID tokens.
The provider performs the interactive sign-in. The API accepts only a signed ID
token for a configured client that carries the flow's single-use nonce. Provider
SDK sign-in has no Turnstile widget.

Turnstile does not replace the PostgreSQL client and identity limits, and it is
not a volumetric defense. Keep network or WAF limits in front of the API.
Production configuration rejects Cloudflare's published test secret keys.
See [runtime configuration](runtime.md#optional-human-verification).

## Administrator and recovery policy

Only the operator command `manage set-administrator <uuid> <true|false>` changes
the global administrator flag. Promotion requires an active, verified account
with confirmed TOTP enrollment. A changed flag advances the security version,
revokes sessions/challenges, and appends an audit event in one transaction. Access
to this command must be controlled by the deployment. There is no public role
administration endpoint. Organization ownership does not imply global administrator
status.

An existing administrator without TOTP cannot authenticate or use an old session.
The operator must demote that account before it can sign in, enroll TOTP, and be
promoted again. Password reset preserves MFA. Recovery-code regeneration requires
recent authentication and a fresh proof, replaces all ten codes atomically, and
revokes every session/challenge. Plaintext codes are returned once and never stored.
Support-assisted factor reset remains a separate product policy.

## Key rotation and production responsibilities

Add a new 32-byte encryption key under a new key ID and set it active. Retain old
keys while rows encrypted with them remain, including enrolled TOTP secrets.
Removing an old key early makes those rows unreadable. The application does not
re-encrypt them automatically. Rotate database, SMTP, and provider credentials
using their own deployment procedures. Use separate migration, runtime, and maintenance database
roles. Runtime must not update or delete audit events or API-usage records;
maintenance adds deletion for retention, without UPDATE, TRUNCATE, or schema-owner
privileges on those records. Development Compose
credentials are public local fixtures.

External password-screening services are excluded by owner decision. A local
common/breached-password blocklist is deferred and is not implemented. Current
password validation enforces the length contract above. Do not add a network
dependency or describe this as breach screening.

Before a public deployment, review recovery/support policy, edge abuse controls,
the configured retention periods, backups, TLS termination, and provider-specific
acceptance tests. The baseline does not include passkeys, account email changes,
administrator UI, or an email delivery guarantee. Do not describe these as
implemented merely because they appear in OWASP guidance.

Successful password changes, initial passwords for OIDC-only accounts, and resets
register an encrypted, tokenless security notification in the same transaction
as the credential change. It contains no password. Existing sessions remain
revoked and a normal login is required. The notification remains eligible after
account deactivation so deactivation cannot suppress a pending password-change
alert. Signup welcome messages require a verified, active account; action mail
still requires its live challenge/version.

## Data services and recovery

The optional [self-hosted data profile](../deploy/data-services/README.md) uses private
host bindings, separate database roles, encrypted off-host pgBackRest repositories,
and new-volume recovery. Its non-root backup scheduler has read-only data access
and no Docker socket. Protect namespace writes, Docker administration, backup
credentials, retained application keys, and the host's forwarding firewall.
Local recovery acceptance does not establish real-provider access or host failover.

## Sources

- [OWASP Authentication Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html)
- [OWASP Session Management Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Session_Management_Cheat_Sheet.html)
- [OWASP Password Storage Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html)
- [OWASP Forgot Password Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Forgot_Password_Cheat_Sheet.html)
- [OWASP OAuth2 Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/OAuth2_Cheat_Sheet.html)
- [OWASP REST Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/REST_Security_Cheat_Sheet.html)
- [OWASP API resource consumption](https://owasp.org/API-Security/editions/2023/en/0xa4-unrestricted-resource-consumption/)
- [Cloudflare Turnstile server-side validation](https://developers.cloudflare.com/turnstile/get-started/server-side-validation/)
- [OpenID Connect library verification guidance](https://docs.rs/openidconnect/latest/openidconnect/)
