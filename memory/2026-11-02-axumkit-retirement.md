# 2026-11-02 — AxumKit retirement

## Changes

- Replaced the old source tree with Hirune Axum Kit main at
  `31ccac9c35c2f8a89e910721435a1e7ec66fdebb`.
- Added a README notice that directs visitors to
  <https://github.com/HiruneWorks/hirune-axum-kit> for source, documentation,
  issues, and releases.
- Preserved previous main at `4fe58497f25f09aa44943c091623a08f0d402388` on
  `legacy-axumkit`. Existing history and tags remain intact.
- Disabled GitHub Actions in `levish0/AxumKit` before pushing the transition.
- Opened [PR #61](https://github.com/levish0/AxumKit/pull/61) as a draft after
  the first reviewable commit.

## Decisions and constraints

- `HiruneWorks/hirune-axum-kit` is the sole active repository. This repository
  holds a one-time source snapshot and a forwarding notice. Do not mirror updates.
- Keep the canonical workflow files in the snapshot. Repository settings stop
  duplicate CI and publication. Archive this repository after the transition.
- The Hirune template is a new implementation. Legacy applications need separate
  API, configuration, and database migration work.
- Keep the imported Apache-2.0 license and NOTICE. The legacy branch retains the
  earlier MIT implementation.
- Preserve ignored local artifacts and secrets. Local Git excludes keep those
  files ignored after the old documentation tree is removed.
- The canonical Hirune checkout receives no changes from this transition.

## Validation

- Compared Git blobs and file modes for all 534 imported source paths. All 533
  non-README files matched the canonical commit exactly. The README retains the
  full canonical text after the forwarding notice.
- `git diff --cached --check` passed for the source import.
- The legacy and transition branches were pushed successfully. The Actions
  permissions API returned `enabled: false` before the push.
- Local ignored files were not copied or staged. No unexpected untracked files
  remained after the import.
- Application tests and remote CI were not rerun for this unchanged source
  snapshot. This record does not establish that canonical CI passed. No images
  were published and no applications were deployed.
- Local GPG signing waited for an unavailable PIN entry. The attempt was stopped.
  Transition commits use per-command unsigned commits; Git signing settings stay
  unchanged.

## Remaining work

After the final checks, merge PR #61, update the repository description and
homepage to the new home, and archive this repository. Read GitHub repository
metadata for the final archive status.
