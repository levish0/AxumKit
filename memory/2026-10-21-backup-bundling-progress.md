# 2026-10-21 — Backup bundling and progress

## Changes

- Enable `repo1-bundle=y` in the shared dev/prod pgBackRest configuration.
- Add `--log-level-console=detail` to explicit backup commands. Operators can
  see native file, byte, and percentage progress.
- Extend actual data-service acceptance to require stored bundles and progress
  output before testing latest and point-in-time recovery.
- Require LF checkout for the two container shell scripts through `.gitattributes`.
  Windows CRLF checkout prevented PostgreSQL bootstrap in the first local run.

## Decisions and constraints

- Keep default bundle sizes, repository settings, encryption, and retention.
- Keep scheduled backup console logging at the global `info` level.
- Configuration is copied into the PostgreSQL image. Rebuild and recreate
  PostgreSQL and its backup scheduler to apply the bundling setting.
- Deployed environments and their backup repositories were not changed.

## Validation

- `cargo fmt --all -- --check` and `git diff --check` passed.
- `cargo clippy --locked --package xtask --all-targets -- -D warnings` passed.
- Python syntax validation passed.
- `DATA_TEST_APP_IMAGE=hirune-kit:presigned-k3s python -u
  compose/data-services/verify.py` passed after correcting checkout line endings.
  The fixture built the changed PostgreSQL image and used the existing release
  application image for migrations. Actual encrypted full/differential backups,
  stored bundles, progress output, non-root scheduler, isolated latest/PITR
  recovery, grants, and persistent data across service replacement passed.
- The disposable repository uses local storage. Real S3/R2 was not contacted.
- Remote CI runs on PR #24. Check its latest head for the current result.

## Remaining work

- Review PR #24 and apply image configuration changes on the selected data host.
