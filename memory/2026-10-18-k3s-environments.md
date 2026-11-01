# 2026-10-18 — Kubernetes dev and prod deployment commands

## Changes

- Made `deploy/justfile` the canonical Kubernetes deployment entrypoint. `up`
  requires `dev` or `prod` and defaults to rolling. Root `deploy-k3s` delegates
  to this same entrypoint. Local `infra-up` remains Compose-based.
- Replaced the single-host deployment Compose files and production examples with
  namespace-neutral `deploy/k3s/base/` manifests and separate dev/prod overlay
  examples. Actual overlays and credentials stay in ignored environment paths.
- Bound all operator operations to an explicit context and validated namespace.
  Every action validates the rendered namespace before contacting Kubernetes.
- Rendered resources once per deployment. Generated Secret hashes update affected
  Pod templates. Schema-owner credentials remain isolated from runtime roles.
- Used namespace-local NetworkPolicy peers and the APISIX Downward API namespace
  for Service DNS. No fixed Pod IP or environment namespace remains in routing.
- Adapted real k3s acceptance to a prod overlay, an unusable default context,
  a same-name dev sentinel Deployment, and cross-namespace ingress denial.
- Documented Tunnel/connectors, independent replica scaling, migration ordering,
  generated-object retention, and external database persistence responsibilities.

## Decisions and constraints

- Hosted dev and prod use `APP_ENV=production` for HTTPS security. Deployment
  environment names do not change application security mode.
- Each environment has its own database, keys, origins, and Cloudflare Tunnel.
  Two cloudflared connectors share one environment's Tunnel token. API replicas
  scale independently. Optional HPA applies only to the API in the example.
- Production PostgreSQL remains external. These manifests do not allocate a
  database PVC or provide backup, replication, or storage expansion.
- One migration Job applies pending migrations before each runtime rollout.
  Rolling requires compatibility with running old processes. Drain stops API,
  worker, and maintenance before migration and has service interruption.
- Operators must serialize releases. Active HPAs must be removed before drain
  or down. Generated Secrets and ConfigMaps are retained for live/old references;
  deployment does not automatically prune them.

## Validation

- Local `cargo fmt --all -- --check` and xtask Clippy with warnings denied passed.
- `cargo xtask test-deployment` passed actual copied dev/prod Kustomize rendering,
  namespace matching, separate database roles, raw JSON Secret encoding,
  Secret-triggered application replacement, migration-only owner changes, and
  unchanged dev resources after prod edits. Local Compose validation also passed.
- Actual `cd deploy; just check dev` and `just check prod` passed with disposable
  ignored copies. `down`, `manage`, and `cleanup` rejected a dev target accidentally
  set to the prod namespace before any cluster contact. Temporary files were removed.
- Markdown links passed. `cargo test --locked --package xtask` passed but contains
  no unit tests; deployment contracts run through the explicit xtask command.
- The local real two-node k3s fixture passed signup/mail, CF/SSR forwarding,
  API replacement (636 successful authenticated requests, zero errors), gateway
  configuration replacement (1,286 successful requests, zero errors), retained
  sessions/quota, coordinated application drain, and unchanged dev Pods/replicas
  with an unusable default kubeconfig context.
- That full local command failed at private ingress denial because this managed
  kernel lacks `xt_physdev`. The embedded policy controller reported failed
  iptables synchronization. This is not a full local acceptance pass.
- Cross-namespace denial resolves DNS outside the caught TCP error path. It must
  pass on the supported remote CI runner. The run below completed that acceptance.
- Deployment just recipes use a POSIX shell on all platforms to preserve argument
  boundaries. Windows operators can use Git's `sh` or the Rust CLI directly.

- Remote [CI run 39096785480](https://github.com/HiruneWorks/hirune-axum-kit/actions/runs/39096785480)
  passed both `verify` and `container` for functional commit
  `64ffecc355a946ae86412209b49d84d1cdc98f00`. Later changes are documentation only.
  Verification included workspace checks/tests, PostgreSQL/API/process/files/content
  suites, deployment contracts, k6 smoke, and copied-template acceptance.
- That CI built the canonical release image and passed real production acceptance:
  HTTPS, STARTTLS, cookies/origins, restricted database roles, backup/restore,
  and encrypted MFA recovery.
- CI two-node k3s acceptance passed with two ready API Pods on distinct nodes,
  API replacement (553 successful requests, zero errors), gateway configuration
  replacement (1,452 successful requests, zero errors), retained sessions/quota,
  coordinated drain, unfinished migration guard, and both private ingress denials.
  Prod operations preserved the dev sentinel Pod and replicas with an unusable
  current context. A dev tunnel-labelled Pod could not reach the prod gateway/API.
- The optional HPA overlay with an API `spec.replicas` removal patch rendered
  successfully. This is a manifest check; HPA scaling was not exercised in k3s.

## Remaining work

Verify real Tunnel, provider behavior, database backup/recovery, and capacity on
actual target infrastructure. The acceptance fixture did not connect a live
Cloudflare Tunnel or deploy an external production database.
