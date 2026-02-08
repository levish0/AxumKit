# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.4] - 2026-02-08

### Refactored

- **OAuth provider trait extraction (`OAuthProviderConfig`)**
  - Unified duplicate Google/GitHub OAuth logic (URL generation, token exchange, service layer) into `OAuthProviderConfig` trait with generic functions
  - Added `generate_auth_url<P>()`, `exchange_code<P>()`, `service_generate_oauth_url<P>()` generic functions
  - Reorganized provider-specific code into `google/` and `github/` folders (config, client, generate_url, sign_in, link)
  - `provider/` now contains only shared infrastructure (trait + generic functions)
  - Added `OAUTH_STATE_TTL_SECONDS` constant to `axumkit-constants` (replaced hardcoded value)
  - Deleted 6 legacy flat service files and old provider subdirectories

---

## [0.4.3] - 2026-02-06

### Improved

- **Whitespace-only string validation (`validate_not_blank`)**
  - Added `string_validator::validate_not_blank` custom validator to reject whitespace-only strings
  - Applied to 13 fields across 9 request DTOs (handles, display names, titles, content, search queries, bio)
  - Excluded: passwords, tokens, TOTP codes, OAuth codes/states, email addresses
  - Addresses `validator` crate's `length(min=1)` not catching whitespace-only input (uses `chars().count()` without trim)

---

## [0.4.2] - 2026-02-06

### Improved

- **Environment variable validation**
  - `ServerConfig`: Replaced individual `.expect()` calls with `require!`/`require_parse!` macros that collect all missing/invalid variables and report them at once (30 required vars: 27 string + 3 parsed)
  - `WorkerConfig`: Same pattern applied with `require!` macro (20 required string vars)
  - Startup now shows all configuration errors in a single panic message instead of failing on the first missing variable

---

## [0.4.1] - 2026-02-02

### Added

- **Google OAuth verified_email check**
  - Added `verified_email` field validation for Google sign-in/link
  - Only verified emails are now allowed, consistent with GitHub behavior
  - Added `OauthEmailNotVerified` error type (`oauth:email_not_verified`)

---

## [0.4.0] - 2026-02-01

### Added

- **Helm Charts**: Kubernetes deployment support
  - `axumkit-server` chart with migration job (post-install hook + wait-for-postgres)
  - `axumkit-worker` chart for background workers
  - `axumkit` umbrella chart with all infrastructure dependencies
  - Dependencies: PostgreSQL, Redis (session/cache), NATS, MeiliSearch, SeaweedFS
  - Environment variables based on `server_config.rs` and `worker_config.rs`
  - HPA, PDB, Ingress, ServiceAccount templates included

---

## [0.3.2] - 2026-01-31

### Added

- **Stability Layer**: Tower middleware stack to protect server from overload
  - `ConcurrencyLimitLayer` - Max concurrent requests (default: 500)
  - `BufferLayer` - Request queue when at limit (default: 1024)
  - `TimeoutLayer` - Request timeout (default: 30s)
  - Configurable via `STABILITY_CONCURRENCY_LIMIT`, `STABILITY_BUFFER_SIZE`, `STABILITY_TIMEOUT_SECS`

- **xtask**: Development environment management tool
  - `cargo xtask dev` - Full setup (docker + migrate)
  - `cargo xtask docker-up/down/status` - Docker service management
  - `cargo xtask migrate/migrate-fresh` - Database migrations
  - Manages Redis Session, Redis Cache, NATS, MeiliSearch, SeaweedFS

- **load-tests**: k6 load testing suite
  - `health-check.js` - Health check endpoint load test
  - 4 scenarios: normal (100 VUs), at_limit (500), buffer (1000), spike (10000)

## [0.3.1] - 2026-01-27

### Changed

- **Read/Write DB Separation in Routes**: Read-only routes now use `read_db` (replica) instead of `write_db` (primary)
  - User: `get_user_profile`, `get_user_profile_by_id`, `get_my_profile`, `check_handle_available`
  - Posts: `list_posts`, `get_post`
  - Action Logs: `get_action_logs`
  - Auth: `list_oauth_connections`, `totp_status`

## [0.3.0] - 2026-01-26

### Changed

- **Database Connection Split**: Separate Write (Primary) and Read (Replica) database connections
  - `AppState.conn` → `AppState.write_db` and `AppState.read_db`
  - Environment variables changed from `POSTGRES_*` to `POSTGRES_WRITE_*` and `POSTGRES_READ_*`
  - Worker uses `POSTGRES_WRITE_*` only (background jobs don't need read replica)
  - Enables PgBouncer connection pooling and read replica support for better scalability

### Added

- Worker environment variables added to `.env.example` (SMTP, FRONTEND_HOST, etc.)

## [0.2.3] - 2026-01-26

### Changed

- Upgrade Rust version from 1.92.0 to 1.93.0
- Update `sea-orm` from 2.0.0-rc.28 to 2.0.0-rc.29
- Add v4 feature to `uuid` crate

### Improved

- Parallelize E2E tests
  - Each test run gets a unique project name for container isolation
  - File-based locking for coordinating image builds across test binaries
  - Use `docker compose` service names instead of container names for port lookup

## [0.2.2] - 2025-01-20

### Removed

- S3 checksum calculation/validation for SeaweedFS and R2 connections
  - Removed `RequestChecksumCalculation::WhenRequired` from SeaweedFS client
  - Removed `RequestChecksumCalculation::WhenRequired` and `ResponseChecksumValidation::WhenRequired` from R2 client
  - Applies to both `axumkit-server` and `axumkit-worker`
