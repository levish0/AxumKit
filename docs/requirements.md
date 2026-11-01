# Confirmed requirements

This document records the owner's explicit constraints and accepted choices.
Use reference material to inform design. It does not override these requirements.
Before materially changing a requirement, agree on a new decision. Then update
this document and the relevant behavior contract. This document records
requirements. It does not establish that verification steps passed.

| Area | Required direction |
| --- | --- |
| Deliverable | A fresh, self-contained Rust source template, owned by each project that copies it. No backward compatibility with reference projects is required. |
| Ownership | Hirune Axum Kit; copyright 2026 Levi Laine; Apache-2.0. No template-origin branding in project prose. |
| Core stack | Axum, SeaORM, Tokio, structured tracing, MJML email templates, and justfile commands. |
| Operational layout | `.envs/` owns environment examples and ignored secrets; `compose/` owns container/local configuration; `deploy/` owns deployment wiring and commands. |
| Design | Independently reasoned crate and file boundaries. References demonstrate useful behavior, not a prescribed crate diagram. |
| Code quality | Clarity and maintainability first; explicit descriptive names, even when longer; concise English comments only where needed. |
| Technical writing | "Write this 80% of the way to ASD-STE100." Apply this clarity target to specifications, runbooks, developer documentation, docstrings, and comments. Use short sentences, active voice, clear steps, and consistent terms. Preserve technical terms and meaning. |
| Closed value sets | Existing and new internal states, purposes, roles, platforms, scopes, ordering, and environment modes use enums. String conversion belongs at protocol/configuration/storage boundaries; preserve explicit wire spellings and reject unknown values. |
| Persistence | Prefer SeaORM and SeaQuery, including migrations. Application-generated UUIDv7 entity primary keys and UUID references. |
| Migrations | One table with its own indexes and constraints per baseline migration. Subsequent migrations have a clear schema responsibility. |
| Scaling | Replaceable stateless API instances; shared state outside processes; independently scalable workers. Ready work fills available worker slots without a fixed inter-job delay; polling waits apply only after no eligible work is found. |
| Production ingress | Cloudflare Tunnel reaches private APISIX, then the API. Trust Cloudflare client-IP headers within that boundary. SSR client-IP overrides require a shared proxy secret; proxy Pod IPs do not establish trust. |
| Application publication | Publish the application image to the current repository's GHCR package on `v<workspace-version>` tags. Verify that the tag matches Cargo.toml and retain the existing quality, production, backup/recovery, and two-node Kubernetes gates. Publish the exact tested image without rebuilding and supply its digest. Branch, PR, and manual runs do not publish. Publication does not deploy services. |
| Single-host data services | Allow k3s and external Docker Compose PostgreSQL/Meilisearch on one Linux machine. Keep application and data-service lifecycle commands separate. Publish the project's PostgreSQL image to GHCR after actual backup/recovery acceptance. Use independent PostgreSQL release tags and an explicit POSTGRES_IMAGE version or digest; deployments pull without building. Retain explicit local/test builds. Use pgBackRest continuous WAL archiving, scheduled full/differential backups to an encrypted off-host S3-compatible repository, and recovery into new volumes. |
| Edge protection | APISIX replicas share a Valkey-backed IP counter in each deployment environment. This broad edge limit fails open on counter failure. PostgreSQL retains authentication limits and strict external API allocations. Do not use forward-auth for this accounting. |
| Kubernetes deployment | Use `deploy/justfile` with explicit dev/prod targets. Default to automatic selection: a read-only check with the new image selects APISIX maintenance and coordinated drain for any pending migration, active maintenance, or stopped applications; other releases roll. Verify every live gateway before stopping applications and before reopening. Failures retain maintenance; retry recovers even after migrations applied. Preserve HPA resources during maintenance, verify native deactivation with positive minimums, and resume saved replica counts automatically after successful migration. Preserve HPA-owned scale during rolling and recovery across repeated down/up. Keep explicit drain for incompatible jobs or configuration. Use k3s with Traefik disabled, private standalone APISIX, and at least two stateless API replicas. Verify two API Pods on two real k3s nodes with continuous traffic and shared authority across replacement. |
| Authentication | OWASP-informed controls, common password/OIDC account and MFA gate, server-side revocation, durable shared authentication state. |
| Session lifetime | Defaults are 14 days idle and 90 days absolute. Cookie and bearer sessions use the same values. Deployments can lower them through configuration. |
| External providers | Multiple OIDC providers may be configured simultaneously, each with an explicit name, issuer, client credentials, and callback URL. |
| Native OIDC login | Native apps exchange a provider SDK ID token for a bearer session. Each provider opts in with explicit native client IDs. The server issues a single-use nonce. Native login does not link identities; linking stays a browser flow. A browser redirect with an app handoff code is not part of the baseline. |
| Developer API | Provide organization-owned API clients and scoped, expiring, revocable API keys. All keys of a client share operator-allocated request budgets across API replicas; owners cannot self-allocate. OAuth2 client credentials is outside this baseline. |
| Privileged authentication | Global administrators must have confirmed TOTP enrollment. Role changes use an operator command, invalidate existing credentials, and are audited. Ordinary accounts may choose MFA. |
| Password screening | No external screening service. A local common/breached-password blocklist is deferred; do not add one without a new decision. |
| Retention | Configurable defaults: succeeded jobs 30 days, failed jobs 90 days, audit events 180 days. Cleanup is bounded and explicitly scheduled; runtime audit access stays append-only. |
| Periodic maintenance | A portable scheduler/worker process registers deduplicated PostgreSQL jobs. No systemd or host-cron dependency; maintenance keeps its restricted database role and dedicated queue. |
| Identity mail | Separate verification, password-reset, welcome, and password-change templates; account notifications commit durably with the successful account change. |
| Device policy | New-device email verification is excluded from the default template. Use MFA for the additional authentication factor. |
| Testing | Focused unit, PostgreSQL, API, and provider-contract coverage. Run deployed E2E and k6 workflows in Kubernetes through the release image and real ingress. Keep CI commands reproducible. Compare migrated schemas with entity metadata and registered string-enum CHECK values. Report schema failures in English. |
| Files | Direct presigned PUT/GET/HEAD transfers of original bytes in a separate private S3-compatible SeaweedFS store; authenticated API URL issuance and upload completion; PostgreSQL owns metadata, permissions, quota reservations, and lifecycle. No default content sanitization, antivirus, image reencoding, video processing, or media processor. |
| Search | Meilisearch is a derived index behind the authenticated API; current PostgreSQL state and permissions decide which results may be returned. |
| Content example | Organization posts and single-level comments connect authorization, original attachments, profile/user and post search, cursor lists, numbered search pages, and durable notifications. No wiki engine. |
| Notifications | Durable in-app comment notifications, preferences, unread/read state; optional FCM HTTP v1 using Google ADC. Disabled by default; no credentials required when disabled. |
| HTTP organization | All version-one domains live under `src/http/v1/<feature>/`. Named endpoint files own their handler and exclusive DTOs; `contracts.rs` contains shared DTOs and `routes.rs` contains URL/method composition. Application services remain unversioned. |
| Dependency maintenance | Use version tags or maintained channels for GitHub Actions, with Dependabot updates; do not pin action references to commit hashes. |
| Agent guidance | Concise mandatory instructions in AGENTS.md, detailed conventions and contracts in docs, milestone evidence in memory. |
| Git | Coherent local commits with concise English messages; no `codex/` branch prefix; preserve unrelated work. |

## Accepted architecture

The initial architecture is a modular monolith. Separate API and worker
executables use the same application services and PostgreSQL database. It is
not a fleet of microservices. Shared storage makes instances replaceable without
sticky routing. Organize source code by feature ownership and dependency
boundaries. Do not build a framework or a generic abstraction over every backend.

PostgreSQL owns sessions, challenges, OIDC flows, authentication limits, strict
API allocations, and jobs. Application processes do not require Valkey. The k3s
gateway uses a separate disposable Valkey instance for shared edge IP limits.
Its failure relaxes only edge enforcement; it cannot authorize requests or reset
PostgreSQL budgets. Provider public-key caching is separate from session authority.

Password signup requires email verification. MFA is optional for ordinary accounts
and mandatory for global administrators. Once enabled it applies to every supported
primary login method. Recovery-code regeneration requires recent authentication
and a fresh factor proof, replaces all codes, and invalidates existing credentials.
New-device email verification is intentionally excluded. OIDC linking requires an
authenticated account. Matching email addresses alone do not merge accounts.

## Scope boundaries

The baseline demonstrates identity, organizations with owner-managed existing
members, profiles, organization posts and comments, Cedar authorization,
pagination, durable background work, and transactional mail.
The file example demonstrates organization attachments and filename search.
Post/user search and notification behavior are defined in their owning contracts.
The external API example demonstrates scoped server-to-server access with shared
minute and calendar-month allocations. See [API access](behavior/api-access.md).
Conversation-specific attachment permissions belong to the owning conversation
service in a consuming project. The template does not parse or execute arbitrary
file contents. `ready` means storage is complete. It does not establish that a
file is free of malware or safe to use.
Storage, search, FCM, and Cloudflare Turnstile human verification are separately
optional integrations. User profiles are discoverable only within current shared
organization membership. Wiki engines, campaign email, public social graphs, and
application-specific permission models remain consuming-project extensions.
The [regression inventory](regressions.md) records retained security behavior and
intentional exclusions from the reference test coverage.

Use [architecture](architecture.md) for ownership, [conventions](conventions.md)
for code rules, and [behavior contracts](behavior/README.md) for public behavior.
Use [milestones](../memory/README.md) for actual validation status and remaining
work. Do not treat a requirement, a planned CI job, or a documentation statement
as evidence that a runtime check passed.
