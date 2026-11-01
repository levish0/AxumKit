# 2026-10-31 — Dependency compatibility

## Changes

- Updated workspace dependency minimums. MiniJinja now uses 3.0.5 without the
  removed `loader` feature. The lockfile removes its former `memo-map` dependency.
- Changed the existing mail rendering test to borrow reusable context strings.
  MiniJinja 3 consumes context values.
- Replaced the deprecated combined Sentry tracing layer with filtered `SpanLayer`
  and `ErrorLayer` instances. Updated the runtime contract to name both layers.
- Extended the telemetry regression to reject provider spans, unrelated targets
  with similar names, and application events below `ERROR`. It also checks that
  owned debug spans remain eligible for performance capture.

## Decisions and constraints

- Keep application `ERROR` events as Sentry error events. Keep performance spans
  for the existing owned target prefixes. Global log filters still apply.
- Keep request-local hubs, trace sampling, request/user payload removal, and the
  existing JSON logger. Do not add telemetry fields or enable Sentry logs.

## Validation

- Passed `just check`: formatting, architecture boundaries, documentation links,
  Clippy with warnings denied, workspace unit/doc tests, and OpenAPI drift.
- Workspace tests passed 47 cases. The 95 infrastructure cases stayed ignored,
  as intended by the default suite. The Sentry filtering/privacy regression and
  all four mail-purpose rendering checks passed.
- Passed `git diff --check`.
- Database, process, Kubernetes, and live-provider acceptance suites were not run
  for this dependency migration. No services were published or deployed.

## Remaining work

- Remote CI must verify the updated PR. Local success does not establish remote
  CI, provider acceptance, or deployment.
