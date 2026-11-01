# 2026-10-22 — Published PostgreSQL images

## Changes

- Require an explicit `POSTGRES_IMAGE` version or SHA-256 digest in each data
  environment. Reject missing versions, `latest`, and malformed digests.
- Remove builds and build secrets from production data Compose. Startup pulls
  images before stopping the backup scheduler, then starts services without
  building. Database and application deployments keep separate lifecycles.
- Add `compose.build.yaml` for explicit local/test image builds. Acceptance can
  use `DATA_TEST_POSTGRES_IMAGE` without rebuilding or pulling the candidate.
  All database, scheduler, and recovery containers use its fixed local image ID.
- Add an independent `postgres-v*` release workflow. It verifies one PostgreSQL
  candidate with actual encrypted backup/recovery before publishing that same
  candidate to the current repository's GHCR namespace. Manual runs verify only.
  The workflow summary supplies the published digest for `POSTGRES_IMAGE`.
- Probe PostgreSQL over TCP for readiness. The official entrypoint's temporary
  initialization server accepts Unix sockets and can otherwise appear ready
  before bootstrap has finished.
- Native acceptance selects `migration.exe` on Windows. Disposable local builds
  use unique image tags and remove only their own tag after fixture cleanup.

## Decisions and constraints

- Keep PostgreSQL release versions independent of the application version.
- Keep credentials in ignored environment files. Image builds include only tools,
  shared configuration, and non-secret scripts.
- Existing deployments must set `POSTGRES_IMAGE` before using the new commands.
  Select the digest from a reviewed PostgreSQL release and apply it explicitly on
  the data host. Application releases do not change this selection.
- This supersedes host image builds described in the 2026-10-18 data-service note.

## Validation

- Formatting, whitespace, Python syntax, and Actions `actionlint` checks passed.
- xtask Clippy with warnings denied and `cargo xtask test-deployment` passed.
  Checks include all four data/recovery services, production build exclusion,
  explicit local build support, version/digest admission, and dev/prod isolation.
- Built the PostgreSQL candidate locally. Acceptance against that fixed image
  passed encrypted full/differential backups, stored file bundles, progress,
  restricted roles, non-root scheduling, latest/PITR recovery, and persistence.
- The default local-build path initially exposed the temporary-server readiness
  race. After the TCP readiness correction, the default build path passed the
  complete encrypted backup/recovery and persistence acceptance too.
- Remote CI runs on PR #25. Check its latest head for the current result.
- Merge-gate CI exposed a timing race in the existing file integration test:
  its successful refresh check reused the one-second expiry-probe lifetime, and
  storage rejected the refreshed URL as expired before CI sent the request.
  Keep the one-second expiry rejection check and use the normal lifetime for
  successful renewal. Production signing and TTL settings remain unchanged.
- Local acceptance uses an encrypted disposable POSIX repository. It does not
  establish S3/R2 access, GHCR publication, or production deployment.

## Remaining work

- Review PR #25, publish the first independent PostgreSQL release tag, and select
  its digest in the intended data environment before an operator applies it.
