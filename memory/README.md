# Milestones

Record durable implementation decisions and validation evidence using [TEMPLATE.md](TEMPLATE.md).
This index is updated when a milestone is complete.

- [2026-10-31 — Dependency compatibility](2026-10-31-dependency-compatibility.md):
  MiniJinja 3 feature and context migration, separate Sentry span/error layers,
  and telemetry filtering regressions.

- [2026-10-30 — PostgreSQL job load coverage](2026-10-30-postgres-job-load.md):
  transactional visibility, saturated workers, multi-batch retention, and queue
  throughput/wait measurements alongside foreground database transactions.

- [2026-10-22 — Application image releases](2026-10-22-application-image-releases.md):
  verified tag-only GHCR application publication, image identity preservation,
  independent database releases, and explicit publication scope.

- [2026-10-22 — Published PostgreSQL images](2026-10-22-published-postgres-images.md):
  independent verified GHCR releases, explicit data image selection, deployment
  pulls without builds, retained local builds, and final-server readiness.

- [2026-10-21 — Backup bundling and progress](2026-10-21-backup-bundling-progress.md):
  bundled small files, explicit backup progress, LF container scripts, and actual
  encrypted backup plus latest/PITR recovery acceptance.

- [2026-10-20 — Deployed Kubernetes E2E](2026-10-20-kubernetes-e2e.md):
  canonical two-node acceptance with real providers, direct presigned transfers,
  attached content, retained state, restricted roles, and a Kubernetes k6 Job.

- [2026-10-20 — Presigned original-file transfers](2026-10-20-presigned-file-transfers.md):
  direct private S3 PUT/GET/HEAD, authorized URL issuance and completion, immutable
  originals, quota-safe late-write cleanup, the updated client contract, and
  confined local transfer clients with CodeQL dataflow review.

- [2026-10-19 — HPA maintenance recovery](2026-10-19-hpa-maintenance-recovery.md):
  native HPA deactivation, durable replica recovery, and automatic replica
  ownership transfer; supersedes manual HPA removal for maintenance.

- [2026-10-19 — Automatic maintenance deployments](2026-10-19-automatic-maintenance-deployments.md):
  read-only migration preflight, automatic maintenance and drain, verified gateway
  admission, and failure/retry recovery.

- [2026-10-18 — Single-host data services and pgBackRest](2026-10-18-single-host-data-backups.md):
  Docker Compose PostgreSQL/Meilisearch beside k3s, off-host backup configuration,
  restricted grants, and isolated physical/PITR recovery.

- [2026-10-18 — Shared Valkey edge limits](2026-10-18-shared-valkey-edge-limits.md):
  per-environment shared APISIX IP counters, preserved PostgreSQL authority,
  disposable counter lifecycle, and failure/isolation acceptance coverage.

- [2026-10-18 — Kubernetes dev and prod deployment commands](2026-10-18-k3s-environments.md):
  canonical Kubernetes operator commands, explicit environment targets, generated
  Secret rollouts, and cross-environment acceptance coverage.

- [2026-10-18 — Private k3s deployment and rolling acceptance](2026-10-18-k3s-deployment.md):
  standalone APISIX/Tunnel profile, two-node API rollouts, coordinated migrations,
  reproducible CI, and the local kernel limitation on policy enforcement.

- [2026-10-18 — Tunnel client identity and Caddy verification](2026-10-18-tunnel-client-ip.md):
  Tunnel-only ingress, shared-secret SSR IP overrides, shared client-IP validation,
  two-instance traffic-limit verification, and the Caddy HTTPS fixture.

- [2026-10-13 — Template foundation](2026-10-13-template-foundation.md): architecture,
  identity, durable work, operational layout, conventions, and verification scope.
- [2026-10-13 — Original files and derived search](2026-10-13-files-and-search.md):
  private storage, scoped access, bounded streaming, quotas, cleanup, search
  consistency, optional integrations, and local verification.
- [2026-10-13 — Concurrent session cleanup](2026-10-13-session-cleanup.md):
  expiry rechecks, a synchronized PostgreSQL regression, and preserved dependency
  updates from main.
- [2026-10-13 — Integrated template verification](2026-10-13-integration-verification.md):
  combined local suites, extended k6, successful implementation CI, copied-template
  validation, and Linux release-image build evidence.
- [2026-10-15 — Authentication, retention, and runtime verification](2026-10-15-security-and-verification.md):
  typed validation, administrator MFA, recovery-code replacement, bounded retention,
  mixed workloads, foreign-key lock ordering, and release-runtime acceptance.
- [2026-10-15 — Multiple providers, portable maintenance, and identity mail](2026-10-15-oidc-maintenance-and-mail.md):
  multiple OIDC providers, queue-scoped periodic maintenance, dedicated mail
  templates, account notifications, and shared error/worker verification.
- [2026-10-15 — External API keys and shared allocation](2026-10-15-external-api-access.md):
  organization-owned clients, expiring scoped keys, operator allocation, strict
  shared request accounting, usage retention, and multi-instance/runtime evidence.
- [2026-10-15 — Connected content and typed HTTP structure](2026-10-15-content-and-notifications.md):
  organization posts/comments, scoped profiles/search, transactional inbox and
  optional FCM, endpoint organization, existing/new closed-set enums, and full
  local verification including multi-instance k6 and release-runtime checks.
- [2026-10-16 — Worker dispatch](2026-10-16-worker-dispatch.md): immediate execution
  slot refill, idle-only polling, recovery progress, independent 1/2/4-pool
  regressions, and local database/process/k6 verification boundaries.
- [2026-10-17 — Technical writing](2026-10-17-technical-writing.md): practical
  ASD-STE100 clarity guidance, revised existing documentation and comments, and
  preserved technical contracts.
- [2026-10-17 — Initial password for OIDC-only accounts](2026-10-17-oidc-initial-password.md):
  first-password operation, credential revocation and notification parity with
  password change, `has_password`, and local API/database verification.
- [2026-10-17 — Optional Turnstile human verification](2026-10-17-turnstile-human-verification.md):
  optional siteverify checks for public identity operations, explicit
  configuration, fail-closed provider errors, and fake-provider verification.
- [2026-10-18 — Schema consistency](2026-10-18-schema-consistency.md): automatic
  entity discovery, PostgreSQL schema comparison, string-enum CHECK validation,
  and regression tests for deliberate drift.
- [2026-10-18 — Native app authentication](2026-10-18-native-app-authentication.md):
  14/90-day default session lifetimes, native OIDC ID token login with opt-in
  client IDs, separate single-use native flows, and shared login rules.
