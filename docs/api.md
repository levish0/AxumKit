# API conventions

The production router exposes `/api/v1/`, `/health/live`, `/health/ready`, and
`/openapi.json`. [openapi.json](openapi.json) is generated from that router's typed
contracts with `just openapi`. `just check` rejects drift. Do not hand-edit it.

HTTP handlers extract typed input, authentication context, and request correlation,
call an owning application service, and translate its result. They contain no
database queries or business policy. Return DTOs rather than persistence models.
Versioned adapters live in `src/http/v1/<feature>/`. Each operation file owns its
handler and exclusive DTOs. `routes.rs` declares URLs and methods, and
`contracts.rs` contains genuinely shared contracts. Common extraction, cookies,
errors, and middleware stay at the HTTP root.
Normal input objects reject unknown fields. The OIDC callback accepts protocol
extensions. Malformed JSON, UUIDs, and query values use the same error envelope.

## Typed input and validation

`ApiJson<T>` and `ApiQuery<T>` require `DeserializeOwned + validator::Validate`.
They deserialize concrete input types, then call `validate()` before the handler
runs. `ApiPath<T>` uses typed Serde extraction for path values such as UUIDs.
Serde enforces field types and required fields. Normal JSON DTOs use
`deny_unknown_fields`. `ToSchema` describes OpenAPI and does not perform runtime
validation. Keep schema descriptions aligned with the actual validation rules.

DTOs declare shape, length, and range checks. Custom validators call the owning
domain's pure validation functions through `src/http/validation.rs`: identity
owns email and new-password rules, organizations own names, and files own
filename and media-type metadata rules. Services call those same functions, so
workers and management commands cannot bypass value validation. Normalization
occurs in the service. Extraction checks do not mutate the submitted DTO.

Authorization, quotas, configuration-dependent limits, and rules that depend on
current state remain in services, with final checks under their locks. Never log
or serialize `ValidationErrors`: their parameters may contain submitted passwords
or tokens. Extraction and validation failures return the generic `400`
`invalid_input` error. An oversized JSON body retains its `413` error.

## Errors and headers

Errors have this shape:

```json
{"error":{"code":"unauthenticated","message":"Authentication is required."},"request_id":"019..."}
```

The server generates a UUIDv7 request ID and returns `X-Request-Id`. Caller-supplied
request IDs are not trusted. Expected failures use 400, 401, 403, 404, 409, or 429.
During planned deployment maintenance, APISIX returns `503` with code
`maintenance`, the common error envelope, and `Retry-After: 30`. Allowed browser
origins receive credentialed CORS and can read `Retry-After` and `X-Request-ID`.
Clients can show a maintenance message and retry later.
The gateway answers valid allowed-origin CORS preflights locally during maintenance.

Storage/provider unavailability uses 503. The API never returns internal details
or secrets. Middleware normalizes router and extractor errors too. All API responses
use `Cache-Control: no-store`, `nosniff`, a restrictive CSP, and no-referrer.
Authentication-required responses include a `WWW-Authenticate: Bearer realm="api"`
challenge.

Services use `ApplicationError`. The HTTP adapter centrally maps it to status and
public error codes. Worker handlers return error codes and retryability to the
job runtime, which owns attempt recording and retry decisions. Request/job spans
preserve correlation across these boundaries without returning internal details.

When Cloudflare Turnstile is configured, signup, verification resend, password-reset
requests, and password login read `human_verification_token` from the JSON body.
A missing or rejected token returns `403` `human_verification_failed`. A provider
failure returns `503` `human_verification_unavailable`. Without Turnstile the
field is optional and ignored. See [human verification](security.md#human-verification).

JSON bodies are bounded at 16 KiB. Requests have a configured deadline and a
bounded concurrency budget. Overload returns 503. Invalid attempts count toward
authentication limits. Authentication 429 responses include `Retry-After: 60`.
external API rate/quota errors use the actual remaining seconds to their window
boundary and distinct `api_rate_limited` / `api_quota_exceeded` codes.

## Authentication transports

Browser login defaults to an HttpOnly SameSite=Lax cookie and requires an allowed
Origin. Production cookies also use Secure, the `__Host-` prefix, and Path=/.
Unsafe cookie-authenticated requests require an exact allowed Origin. CORS uses
explicit origins. It does not authorize requests.

Native clients explicitly request `transport: "bearer"` at login and MFA completion
and receive the same opaque session credential in JSON. Send it as `Authorization:
Bearer <token>`. It is not a JWT. The API rejects mixed cookie and bearer credentials,
duplicate session cookies, and multiple Authorization headers. Session rotation
preserves the request's transport. Never put credentials in query strings.
The Bearer scheme name is case-insensitive. The credential remains case-sensitive.

Native clients sign in with an OIDC provider SDK through
`POST /api/v1/auth/oidc/{provider}/native` and
`POST /api/v1/auth/oidc/{provider}/native/complete`. The first route returns a
single-use `flow` and `nonce`. Pass the nonce to the SDK. Send the flow and the
SDK `id_token` to the second route. Completion always returns a bearer session or
an MFA challenge and never sets a cookie. These routes do not require an Origin.
See [native ID token login](behavior/identity.md#native-id-token-login).

External server applications use an organization-owned API key in the Bearer
header on explicitly designated external routes. Keys and user sessions cannot
be substituted for each other. See [API access](behavior/api-access.md) for key
format, owner management, operator allocation, scopes, charging, and headers.

`POST /api/v1/account/recovery-codes` accepts an MFA proof and requires recent
authentication. It returns ten replacement codes once, invalidates the previous
set and all sessions/challenges, and clears the session cookie. Save the returned
codes before signing in again. The endpoint follows the API's `no-store` policy.

## Pagination and updates

Lists use `limit` (default 25, range 1–100) and an optional opaque `cursor`. Results
are `{ "items": [...], "next_cursor": null }`. Sessions, organizations, clients,
keys, and files use ascending UUID order. Posts, comments, organization users,
and the inbox use descending `(created_at, id)` order, including an explicit UUID
tie-breaker. Their cursors carry typed positions and the list's organization,
actor, and filter scope. Creation timestamps are immutable. Edits do not move rows. Queries
fetch at most limit+1 rows and always apply the owner/membership scope in SQL.
Queries do not perform an implicit count. A cursor has a version and scope, but it
does not grant access. Every query still enforces access. Unknown versions,
invalid encoding, and mismatched scopes are invalid input.

Post and profile search use numbered `page` / `per_page` inputs. They hydrate at
most 1,000 candidate ID/revision pairs through the authorized PostgreSQL query
before slicing pages. Responses include `total_in_window`, `search_window_limit`,
and `has_next_page`. Counts cover only authorized results within that finite
candidate window. They do not report an unbounded search-engine total. Stale
revisions, private drafts, removed memberships, and deleted resources cannot leak
through result text or counts. Filename search retains its bounded first-page contract. See
[profiles and search](behavior/profiles-and-search.md).

Organization, post, comment-deletion, and profile changes require
`expected_version`. A stale value returns 409. New posts/profiles use version zero.
Current operations have explicit full input fields. If a future partial-update
contract accepts both omission and null, represent and test those states separately.
Unauthorized organization detail access returns 404, including for administrators
who lack membership. List results must not leak the existence of other scopes.
