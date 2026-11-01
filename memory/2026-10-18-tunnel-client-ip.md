# 2026-10-18 — Tunnel client identity and Caddy verification

## Changes

- Replace the fixed proxy IP allowlist with the Cloudflare Tunnel ingress
  contract. Both authentication traffic limits and human verification use the
  same HTTP client-address resolver.
- Add optional, redacted `INTERNAL_PROXY_SECRET` configuration. A matching
  constant-time comparison permits an SSR server's `X-Real-Client-IP` override.
  Otherwise, use `CF-Connecting-IP`, then the socket peer for local requests.
- Require one valid address in the selected IP header. Reject malformed values,
  lists, and duplicates. Normalize IPv4-mapped IPv6 addresses. Ignore
  `X-Forwarded-For` and reject the removed `TRUSTED_PROXY_IPS` setting.
- Update configuration examples, runtime/security requirements, and deployment
  guidance. PostgreSQL authentication, authority checks, and strict API-client
  accounting retain their existing transactions.
- Replace the local release-image fixture's nginx proxy with Caddy 2.11.7. Keep
  the ephemeral test certificate and private CA. Disable automatic HTTPS and the
  Caddy admin API. The fixture overwrites Cloudflare client IP headers and retains
  an explicit malformed-header path.

## Decisions and constraints

- Production ingress is Cloudflare Tunnel to private APISIX, then the API.
  Direct public access to the gateway or API violates the header trust boundary.
  Proxy Pod addresses do not establish trust.
- SSR servers and API replicas share a random secret of at least 32 visible ASCII
  characters. Browsers and mobile applications must never receive it. Missing,
  incorrect, or duplicate secret headers cannot authorize an SSR override.
- Caddy is a local HTTPS verification dependency. Production uses APISIX.
- A file-driven APISIX route to the internal API Service is sufficient for the
  current single backend. Adding application endpoints does not require new
  gateway routes. Kubernetes routing resources can be adopted when independently
  deployed backends need dynamic routes.

## Validation

- `cargo test --locked --lib --quiet`: 13 tests passed.
- `cargo xtask check`: passed formatting, architecture/document links, Clippy with
  denied warnings, 43 workspace tests, doc tests, and OpenAPI drift. The default
  workspace run left 89 infrastructure tests ignored.
- `cargo xtask test-api`: 31 PostgreSQL-backed API, API-access, human-verification,
  and OIDC tests passed. The new regression uses two applications sharing one
  database and verifies shared IP limits, rejected spoofed overrides, accepted SSR
  identity, and the IP sent to the local human-verification provider.
- `cargo xtask test-deployment`: local and deployment Compose checks passed.
- Caddy configuration formatting and validation passed. An isolated Docker HTTPS
  smoke check verified trusted local TLS, replacement of forged Cloudflare
  headers, preservation of SSR headers, and malformed-header route rewriting.
- Python fixture syntax and `git diff --check` passed. Astra reviewed the diff
  without actionable findings; that review did not execute tests.
- `PRODUCTION_TEST_IMAGE=hirune-tunnel-production-check:local cargo xtask
  test-production`: passed with the release image and Caddy. Verified HTTPS,
  STARTTLS, secure cookies, origin rejection, header replacement, restricted
  database grants, maintenance, PostgreSQL backup/restore, encrypted MFA state,
  and restored API allocation/history/revocation.
- The managed executor required Rust 1.98.1 installation. Temporary application
  and verifier Dockerfiles mounted the supplied proxy CA for build-time HTTPS.
  The application used the canonical release build flags and runtime stage.
  Neither the CA nor the temporary build wiring is part of the repository or
  application image. No live provider or production deployment was exercised.

## Remaining work

This milestone does not add k3s manifests or deploy a live Cloudflare Tunnel.
The real ingress boundary, provider acceptance, and cluster rollout need their
own deployment validation. No image publication or remote CI result is implied.
