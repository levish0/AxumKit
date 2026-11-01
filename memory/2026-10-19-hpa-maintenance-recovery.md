# 2026-10-19 — HPA maintenance recovery

## Changes

- Remove the manual HPA deletion requirement from automatic maintenance, explicit
  drain, and down/up. This supersedes that restriction in the earlier deployment
  milestones.
- Keep HPA identity and settings. Use Kubernetes native maintenance deactivation:
  desired replicas zero and a positive HPA minimum disable automatic adjustment.
- Save positive target counts in each Deployment's
  `hirune.axum-kit/resume-replicas` annotation before stopping. Repeated down and
  failed-migration retries retain that count. Verify HPA `ScalingActive=False`
  with reason `ScalingDisabled` before migration.
- Restore positive counts within current HPA bounds after migration success.
  New Deployments use the HPA minimum. Existing positive counts stay under HPA
  control during rolling updates.
- Transfer client-side apply ownership by removing replicas from the existing
  last-applied annotation before omitting the field from manifests. Send this
  configuration through stdin because environment values may be sensitive.
- Extend the two-node fixture with metrics-server and an actual HPA controller,
  replica ownership transfer, failed migration and recovery, and repeated down/up.

## Decisions and constraints

- Inspect live and rendered HPAs for owned Deployments. Leave unrelated HPAs alone.
- Native deactivation requires a positive minimum. Reject scale-to-zero HPAs for
  owned Deployments before changing maintenance or replicas.
- Preserve serialized release operations. Migration failure keeps admission
  closed and application targets at zero. Application startup failure retains
  maintenance; retry uses the retained recovery state.
- The HPA fixture uses minimum and maximum three to make capacity assertions
  deterministic and distinct from the base two replicas. The production example
  remains a CPU HPA with bounds two to four.
- Use the [official Kubernetes maintenance and ownership behavior](https://kubernetes.io/docs/tasks/run-application/horizontal-pod-autoscale/#implicit-maintenance-mode-deactivation).

## Validation

- Local formatting, xtask Clippy with warnings denied, three Kubernetes unit
  tests, and copied dev/prod Kustomize/Compose contracts passed.
- Astra reviewed the native-maintenance design and identified the client-side
  ownership transition required to prevent a reset to one replica.
- The extended full CI run must verify metrics-backed HPA activation, capacity
  preservation, zero replicas across controller reconciliations, failure/retry,
  unchanged HPA UID/spec, and resumed scaling after repeated down/up.
- The previous automatic-maintenance head `934f7a78ddd96473ab7d0397b7efc99150612c4a`
  passed both full CI runs and CodeQL, including two real k3s nodes. Those results
  precede this HPA change.
- No live deployment or external provider was changed. Full two-node validation
  uses CI because the earlier local image import exhausted the managed host disk.

## Remaining work

- Confirm the new full CI results and record them in the pull request.
