# Architecture

Hirune Axum Kit is a source template for one service. Copy the repository, change
the application name and settings, and own the result. Internal crates are not
independently published products. Template updates do not migrate copied apps.

## Execution and ownership

This is a modular monolith with separately scalable API and worker processes.
PostgreSQL owns accounts, authentication state, authorization facts, deliveries,
and jobs. Neither process requires sticky routing or in-memory authority.
Migration and management commands are explicit one-off processes.
The optional [data profile](../deploy/data-services/README.md) keeps PostgreSQL,
Meilisearch, and pgBackRest in Docker Compose outside k3s, including on the same
Linux machine. Data volumes and backup recovery have a separate lifecycle from
application Pods.
The k3s ingress gateways share disposable Valkey counters for broad IP protection.
Authentication and strict API accounting remain in PostgreSQL. Application
processes do not connect to Valkey. See the [deployment contract](../deploy/k3s/README.md#shared-edge-limits).
The separate maintenance executable registers periodic jobs and consumes a
dedicated queue using restricted database credentials. It shares the same lease,
retry, and attempt-history implementation as ordinary workers.

| Location | Owns | May depend on |
| --- | --- | --- |
| `src/http/` | shared HTTP extraction, middleware, error mapping, version composition | application services |
| `src/http/v1/<feature>/` | feature router, handlers, and versioned input/output contracts | owning application service |
| `src/http/v1/openapi.rs` | version-one contract assembly | feature HTTP contracts |
| `src/integrations.rs` | explicit provisioning and bounded search-repair coordination | owning services |
| `src/job_handler.rs` | concrete application job dispatch | owning feature services |
| `src/bootstrap.rs` | connection and service assembly | application, jobs, mail |
| `src/bin/` | environment, process tracing, executable lifecycle | bootstrap and runtime |
| `src/maintenance.rs` | bounded cross-domain cleanup coordination and periodic maintenance registration | owning services and jobs |
| `crates/application/src/identity/` | accounts, credentials, sessions, authentication flows | auth-core, jobs, mail, SeaORM |
| `crates/application/src/organizations/` | minimal organizations and scoped memberships | authorization, SeaORM |
| `crates/application/src/api_access/` | organization-owned clients, hashed API keys, strict shared allocations, admission history | identity and organization services, SeaORM |
| `crates/application/src/files/` | original attachment metadata, quota, lifecycle, S3 transport, derived filename search | organization service, AWS S3 SDK, Meilisearch REST, SeaORM, jobs |
| `crates/application/src/posts/` | organization posts, single-level comments, attachment links, post search | identity, organizations, files, notifications, authorization, SeaORM, jobs |
| `crates/application/src/profiles/` | account profiles and organization-scoped user discovery | identity, organizations, search, SeaORM, jobs |
| `crates/application/src/notifications/` | durable comment inbox, preferences, session-bound devices, optional FCM delivery | identity, owning content facts, SeaORM, jobs, Google ADC |
| `crates/application/src/search/` | bounded Meilisearch HTTP/task transport | reqwest; domain modules own documents and eligibility |
| `crates/application/src/authorization/` | validated Cedar policies and decisions | Cedar |
| `crates/jobs/` | durable registration, leases, retries, attempt history | SeaORM, Tokio |
| `crates/mail/` | MJML/Minijinja rendering and SMTP | lettre, mrml, minijinja |
| `crates/auth-core/` | small cryptographic wrappers | established crypto libraries |
| `crates/migration/` | schema history | SeaORM migrations |
| `crates/test-support/` | isolated databases and fixtures | application, migration |
| `xtask/` | repeatable development and verification commands | tooling only |
| `.envs/` | tracked examples and ignored per-environment secrets | configuration only |
| `compose/` | local service composition and production image build | container tooling |
| `deploy/` | environment selectors and runtime/operator wiring | container tooling |

The application crate has no Axum dependency. Its feature modules contain their
own service, repository, model, and types. HTTP DTOs stay with their versioned
feature adapter. Each named operation has its own file with its exclusive DTOs;
shared DTOs stay in `contracts.rs`. `routes.rs` assembles URLs and methods. Module
indexes only declare or re-export modules. Nested comment and OIDC folders keep
related endpoints together. The top-level HTTP router owns health and OpenAPI
routes and nests the version router. A version router only composes feature
routers. Add a version directory when an HTTP contract needs a new version.
Reuse business services across API versions.
Notifications separate service assembly, inbox reads/writes, preferences, comment
recipient registration, devices, and push execution. Post API-client reads do not
live in search; profile module indexes contain no queries or job registration.

Do not create a crate for every module or a trait for every concrete service.
Introduce a boundary when it makes ownership, dependencies, or testing clearer.

Crate boundaries enforce dependency direction, not a list of technical layers.
Keeping each feature's service, repository, entities, and internal types together
makes a business change local. Jobs and mail have independent execution and
failure contracts; auth-core has no database or async runtime dependency.
Migration history and development/test tools do not belong in request handling.
API, worker, and management are binaries of the same assembled application.
Separate process deployment does not require separate service crates.

`bootstrap(settings)` prepares dependencies. `build_router(app, http_settings)`
only constructs the production router. `serve(listener, router, shutdown)` owns
listening and shutdown. Tests use these same functions. Router construction must
not start a listener, consumer, scheduler, or detached task.

## Extending the template

Add a feature with explicit service operations and feature-owned persistence.
Expose it through thin HTTP handlers and typed input/output contracts. Add the
operation's behavior, transaction boundary, authorization, pagination, and tests
together. Workers call those same operations when the business behavior is shared.

The initial worker backend is PostgreSQL. Before changing the backend, review the
adapter and transaction boundaries. A broker needs an outbox when a domain write
must commit with publication. A common function signature cannot create a
distributed transaction.

Organization posts and comments connect Cedar, profiles, scoped search,
pagination, attachments, and notifications through concrete service operations.
In-app notifications work without FCM credentials. Optional FCM uses the same
durable PostgreSQL queue. Search documents are expendable projections; PostgreSQL
decides visible fields, revision eligibility, and authorized result counts.

## Design references

- [Axum state and Tower integration](https://docs.rs/axum/latest/axum/)
- [SeaORM transactions](https://www.sea-ql.org/SeaORM/docs/advanced-query/transaction/)
- [Django database transactions](https://docs.djangoproject.com/en/6.0/topics/db/transactions/): explicit transaction boundaries and durable side-effect handling inform the conventions.
- [Django sessions](https://docs.djangoproject.com/en/6.0/topics/http/sessions/): shared server-side authority and credential rotation inform session behavior.

These are design references. The implementation remains ordinary Rust and Axum.
