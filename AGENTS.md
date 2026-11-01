# Agent Instructions

- Work autonomously within the user's requested scope. Ask only when requirements are genuinely ambiguous or a decision would materially change behavior, architecture, or scope.
- Read [project conventions](docs/conventions.md) and [confirmed requirements](docs/requirements.md). Inspect the relevant architecture and code before making changes.
- Organize code by responsibility. Use explicit names that describe purpose, even when longer. Give every function, type, module, and file a clear role.
- Prefer direct implementations over unnecessary abstractions, compatibility layers, configuration, or cleverness. Fix root causes.
- Do not modify, revert, reformat, or clean up unrelated user changes.
- Validate changes with relevant tests, checks, formatting, and builds. Never claim validation that was not performed.
- Commit completed changes in coherent logical units with concise English messages. Do not rewrite commits or use destructive Git operations unless explicitly requested.
- Use descriptive branch names without `codex/` or another tool or agent prefix.
- Open a draft pull request as soon as a work branch has its first reviewable commit. Continue implementation and validation on that PR; do not wait until the work is finished.

## Code Quality

- Follow existing conventions where appropriate, but do not assume the existing design is correct. Explain materially different designs and their tradeoffs before implementing changes outside the agreed direction.
- Treat clarity, maintainability, and file organization as completion requirements. Add concise English comments only for invariants or non-obvious decisions.
- For specifications, runbooks, developer documentation, docstrings, and comments: "Write this 80% of the way to ASD-STE100." Use short sentences, active voice, clear steps, and consistent terms. Preserve exact technical terms and meaning. See [writing rules](docs/conventions.md#technical-writing).

## Backend Essentials

- Read [architecture](docs/architecture.md) and the relevant [behavior contract](docs/behavior/README.md). Keep Axum handlers thin; services own business rules and authorization, repositories own queries, and HTTP adapters translate domain errors.
- Use Axum, SeaORM, and Tokio. Prefer SeaORM and SeaQuery for queries and migrations; raw SQL needs a specific unsupported operation and an explained reason. Application services must not depend on HTTP types or executable configuration. Reuse canonical service and policy functions across API, workers, and commands.
- Services own transactions and lock order. Recheck authority under the relevant locks; repository writes use the caller's transaction. Commit important follow-up work durably with the business change. See [transaction boundaries](docs/transaction-boundaries.md).
- Use concrete input/output types, the shared error format, and the documented [API and pagination conventions](docs/api.md). Do not expose persistence models or hide errors with broad fallbacks.
- Group HTTP by version, domain, and named operation under `src/http/v1/<feature>/`. Each operation owns its handler and exclusive DTOs; keep shared DTOs in `contracts.rs` and URL/method composition in `routes.rs`. Keep application services unversioned.
- Model finite states, purposes, roles, platforms, scopes, and ordering modes as enums. Convert strings at configuration/protocol/storage boundaries; preserve wire values explicitly and reject unknown values. Use SeaORM ActiveEnum for persisted closed sets.
- Use application-generated UUIDv7 primary keys and UUID foreign keys. Keep natural uniqueness in explicit constraints; integers are for counts and versions, not entity identities.
- Give each migration one clear schema responsibility. Keep its table, indexes, and constraints together; preserve applied migrations and add subsequent changes as new migrations.
- Keep entity column metadata accurate. Register constrained string-enum columns and run `just test-db` for schema or persisted-enum changes. See [schema consistency](docs/schema-verification.md).
- Pass settings explicitly. Keep API and worker processes replaceable; shared storage owns sessions, authentication flows, rate limits, and jobs. See [runtime](docs/runtime.md).
- Run periodic maintenance through the portable maintenance executable and its shared job queue, with separate database credentials. Do not require an operating-system scheduler.
- Keep environment examples under `.envs/.example/`, actual secrets under ignored `.envs/.<environment>/`, container build/local files under `compose/`, and deployment wiring under `deploy/`. Runtime processes must not receive migration-role credentials.
- Use structured `tracing` spans across requests, services, and jobs. Never record credentials, cookies, tokens, secret URLs, or mail bodies. Use the existing MJML mail templates and transport.
- Review authentication changes against [security](docs/security.md), including the OWASP references and regression tests. PostgreSQL is authoritative; cached data must never bypass revocation or final authorization.
- Password and OIDC login must share the same account and MFA gates. New-device email verification is excluded from the baseline; do not add it without a new product decision.
- External API keys represent organization-owned clients, not user sessions. Recheck scope and revocation under client/key locks and commit shared quota counters with admission history. Only operators allocate quotas; rotation must preserve usage. See [API access](docs/behavior/api-access.md).
- Store file originals unchanged in the private S3 store through presigned transfers. The API authorizes URL issuance and verifies completion; it does not proxy file bytes. PostgreSQL owns attachment access and lifecycle. Do not add content processing or scanning without a new requirement; see [files and search](docs/behavior/files-and-search.md).
- Use `just` and the locked Cargo workspace. Keep policy, database, router, and provider-contract checks focused. Run deployed E2E and k6 workflows through the real two-node Kubernetes fixture. Native process load is an optional diagnostic. See [development](docs/development.md) and `.github/workflows/ci.yml`.
- Use version tags or maintained channels for GitHub Actions. Manage dependency updates through `.github/dependabot.yml`; do not pin action references to commit hashes.
- Keep behavior, schema, OpenAPI, configuration, and tests aligned. Distinguish local validation from remote CI, publication, deployment, and provider acceptance.

## Memory Workflow

Record completed milestones that change architecture, APIs, schemas, configuration, tooling, or major behavior in `memory/YYYY-MM-DD-<scope>.md`. Update [the index](memory/README.md) and use [the template](memory/TEMPLATE.md).

- Record durable decisions, constraints, changes, and actual validation results, including skipped or failed checks.
- Never store secrets, credentials, or irrelevant conversation history.
- Add a correction or superseding entry when old facts change; do not rewrite unrelated entries.
