# 2026-10-19 — Automatic maintenance deployments

## Changes

- Default Kubernetes deployment mode is `auto` in both justfiles and xtask.
- A preflight Job runs the new image's `migration --check-pending` command in a
  read-only repeatable-read transaction. It reports a strict pending count through
  the successful container's termination message. The helper binds the result to
  its Job UID. Fresh history stays absent; unknown versions and gaps fail.
- Any pending migration, active maintenance, or stopped application selects
  maintenance and coordinated drain. Other releases roll. Explicit rolling rejects
  those interruption conditions; explicit drain remains available.
- APISIX reads stable projected operator state, returns the common maintenance
  503 with Retry-After and CORS, and has local gateway readiness. Backend readiness
  remains separate. Every live gateway must serve maintenance before apps stop.
- Successful recovery waits for runtime rollout and backend readiness through
  every gateway, disables maintenance, and verifies propagation. Off failures
  request and verify on again. Auto retries recover even after migrations applied.
- Added migration-history, environment-manifest, and real k3s acceptance coverage
  for these paths. Added explicit recovery maintenance commands and runbooks.

## Decisions and constraints

- This is a conservative pending-migration policy, not SQL compatibility analysis.
- Migration changes the real database. Application rollback does not undo it.
- Preserve maintenance state across ordinary apply, failure, and down/up. Missing
  or malformed state keeps gateway admission closed.
- Install maintenance-aware gateways before draining old applications. Keep
  gateway readiness independent of backend availability.
- Serialize releases. Remove HPAs from the overlay and namespace before a release
  that needs drain. Incompatible jobs or settings without migrations need explicit
  drain. External data-service lifecycle remains separate.
- Astra reviewed ordering and failure recovery. Its off-PATCH uncertainty finding
  was fixed so request and propagation errors both trigger recovery. Allowed
  browser preflights receive local 204 so clients can read the actual 503 body.

## Validation

- Local format, xtask/migration Clippy with denied warnings, the mode-selection
  unit test, and copied dev/prod Kustomize/Compose contracts passed.
- The actual migration binary passed PostgreSQL checks for fresh history with 31
  pending migrations, no history creation, current schema, one pending migration,
  invalid history, machine-readable reports, and skipped role-grant mutation. The
  preflight connections enforced PostgreSQL read-only mode.
- Local two-node k3s startup exhausted this managed host's disk while importing
  images with the DinD vfs/native snapshotters. The owned cluster was stopped and
  deleted before acceptance; this is not a passing k3s result. Final remote CI
  must verify the canonical image on two actual k3s nodes.
- No live environment or external Tunnel was changed.
- First remote k3s attempt stopped before maintenance checks: the fixture waited
  for a probe Pod before deployment created its application Secret. Move that
  readiness wait after deployment prerequisites. The acceptance assertions stay
  unchanged; the corrected head needs a new full CI run.
- Astra also found that the fixture's standalone tunnel-labelled client would
  join the full-down Pod deletion selector. Temporarily detach only its part-of
  label for down, then restore it before auto up. This keeps its session state,
  the production stop selector, and the production NetworkPolicies unchanged.

## Remaining work

- Record final two-node k3s and CI results, including any local host limitations.
- Real Tunnel/provider and operator-host deployment remain separate acceptance.
