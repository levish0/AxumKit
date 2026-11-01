# 2026-10-17 — Technical writing

## Changes

- Added the owner's writing rule to AGENTS.md, confirmed requirements, and project
  conventions: "Write this 80% of the way to ASD-STE100."
- Revised existing README, deployment, runtime, security, API, architecture,
  development, and behavior documentation. Split long sentences and named the
  actor where an instruction was unclear.
- Clarified selected cryptography, worker, OIDC refresh, and file-transfer comments.
  Split the optional local integration instructions into clear steps.

## Decisions and constraints

- Apply the rule to this repository's specifications, runbooks, developer
  documentation, docstrings, and comments. This repository has no UI.
- Use short sentences, active voice, clear steps, and consistent terms. Treat the
  rule as a practical clarity target, not a formal STE compliance claim.
- Preserve technical names, commands, API fields, protocol terms, units,
  requirements, security conditions, and verification limits.
- Keep concise English comments only where needed. Leave clear prose and
  historical milestone records intact.
- Runtime behavior, schemas, OpenAPI, configuration values, and test contracts
  remain unchanged.

## Validation

- `just check` passed: formatting, dependency and HTTP boundaries, local document
  links, Clippy with warnings denied, 29 unit tests, doc tests, and OpenAPI drift.
- `git diff --check` passed.
- A one-off comparison against the starting commit confirmed that fenced
  examples, existing inline contracts, and numeric values were preserved. Rust
  and environment-file changes affect comments only. All 116 local Markdown
  links resolved, including the new milestone link.
- Database, process, integration, release-container, and k6 suites were not rerun
  for this prose-only change. Remote CI and deployment were not verified.

## Remaining work

- Apply the writing rule when adding or changing technical prose.
