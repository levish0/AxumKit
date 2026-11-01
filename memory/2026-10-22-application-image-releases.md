# 2026-10-22 — Application image releases

## Changes

- Add application image publication to CI for `v<workspace-version>` tags.
  Resolve the image path from the repository and reject a mismatched tag before
  building or testing. Keep branch, PR, and manual runs read-only.
- Build one application candidate with source, revision, and version labels.
  Retain the existing verify, production, backup/recovery, and two-node Kubernetes
  gates. Transfer the tested image to a separate package-write job and compare
  its image ID before publishing. Do not rebuild during publication.
- Supply `APPLICATION_IMAGE=...@sha256:...` in the workflow summary. Document
  digest selection and separate package access from runtime deployment.
- Restrict CI tag triggers to application `v*` tags. PostgreSQL keeps the
  independent `postgres-v*` workflow and does not need another full app CI run.

## Decisions and constraints

- The requested scope is GHCR Packages publication. Do not configure or deploy
  an operator's server as part of this release.
- Keep application and PostgreSQL versions independent. Prepare application
  `v0.1.0` and PostgreSQL `postgres-v18.6-1` as the first releases.
- Use short-lived workflow credentials. Only the tag-only publication job gets
  package-write permission. New package access must be configured separately.

## Validation

- Actions actionlint, Python syntax, and whitespace checks passed locally.
- Release metadata checks passed for an untagged branch and matching `v0.1.0`.
  A mismatched app tag, a PostgreSQL tag, and an invalid repository were rejected.
- Opened PR #26 before full remote validation. Check its latest head for CI.
- Pushed `postgres-v18.6-1` at PR #25 merge commit
  `7566363d042c965410f81e789f7608862eca7909`. PostgreSQL release workflow
  run `39784031389` is in progress; publication is not yet established here.

## Remaining work

- Complete PR CI, merge, and push `v0.1.0` at the reviewed merge commit.
- Verify both GHCR package versions and record their immutable digests. A tag or
  successful build alone does not establish completed package publication.
