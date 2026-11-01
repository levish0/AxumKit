# 2026-10-18 — Private k3s deployment and rolling acceptance

## Changes

- Added a Kustomize profile for Cloudflare Tunnel → private standalone APISIX →
  API Service. API, gateway, and Tunnel each default to two replicas. Traefik
  and ServiceLB are disabled in the cluster setup instructions.
- Added readiness, TCP liveness, startup checks, graceful termination, topology
  spread, disruption budgets, resource limits, and optional CPU autoscaling.
- Kept external PostgreSQL/SMTP and separate runtime, maintenance, migration,
  and Tunnel Secrets. API and worker never receive the migration URL.
- Added `just deploy-k3s rolling` and `just deploy-k3s drain`. Both wait for one
  migration Job before runtime changes. Unfinished Jobs block another deploy;
  finished Jobs are replaced. Offline upgrades drain API, worker, and maintenance.
- Added `K3S_TEST_IMAGE=... just test-k3s` and CI wiring. The fixture uses one
  real k3s server and one agent, the canonical release image, real APISIX, local
  database/mail, and probe Pods. Real cloudflared replicas are zero in its copy.

## Decisions and constraints

- Use stable standalone APISIX YAML with a catch-all Service route. No controller,
  gateway etcd, Admin API, forward-auth, Redis, or business quota plugin is needed.
  The backend keeps PostgreSQL authority for sessions, limits, and allocation.
- NetworkPolicies allow only Tunnel-labelled Pods into the gateway and only
  gateway-labelled Pods into the API, within the intended namespace. Restrict
  namespace write access. Public Services and host-network bypass are excluded.
- APISIX preserves CF and secret-authenticated SSR identity headers. Upstream
  retries are zero to avoid duplicate business admissions. Hashed ConfigMaps
  trigger replacement instead of depending on subPath hot reload.
- Compatible schema/settings/job changes can roll. Incompatible changes need a
  coordinated drain or an expand/migrate/contract release. Serialize deployments;
  the helper is not a distributed release lock.
- Pin k3s v1.37.1+k3s1, k3d v5.9.0, kubectl v1.37.1, APISIX 3.19.0-debian,
  and cloudflared 2026.10.1. The nested fixture uses native snapshots and Flannel
  host-gw. Its controller still requires host kernel policy modules.

## Validation

- The real k3s API server accepted all production manifests and the optional HPA
  with server dry-run. YAML parsing, Python syntax, local links, formatting,
  `cargo build --locked --package xtask`, and Clippy with warnings denied passed.
- A real two-node run put two Ready API Pods on distinct nodes. Signup, worker
  verification mail, bearer sessions, CF/SSR forwarding, and unfinished migration
  rejection passed. API replacement served 809 authenticated requests with zero
  errors and retained the session and shared monthly allocation. A config-only
  gateway replacement served 1351 authenticated requests with zero errors.
- The final local repeat served 488 API-rollout requests and 1480 gateway-rollout
  requests with zero errors. The coordinated drain stopped all three application
  Deployments before migration, restored them, and retained the existing session
  and exhausted quota. It then failed the ingress test for the confirmed kernel
  reason below; this repeat is also not a full acceptance pass.
- **The local full acceptance command did not pass.** Direct-ingress denial
  failed. A separate minimal real-k3s diagnosis confirmed that this managed
  host's kernel 6.18.55 lacks `xt_physdev`: kube-router aborts `iptables-restore`
  with `Extension physdev revision 0 not supported`. Base controller chains and
  successful manifest apply do not establish enforcement. The test keeps this
  failure visible. CI explicitly loads the required modules on its Ubuntu host.
- Reading `/proc/config.gz` later confirmed both `# CONFIG_MODULES is not set`
  and `# CONFIG_NETFILTER_XT_MATCH_PHYSDEV is not set`. This host cannot load a
  separately installed module. Enabling it requires a different host kernel,
  rather than an additional package inside the k3d containers.
- Earlier nested-host attempts also exposed missing `/dev/kmsg`, unavailable
  VXLAN/overlay extraction, and disk pressure. The fixture now matches native
  imports, uses host-gw, waits for both nodes to be Ready without disk pressure,
  and documents disk needs. Only generated build caches were removed locally.
- Cloudflared 2026.10.1 ran its version command as the configured non-root user
  with a read-only root filesystem. This does not establish Tunnel connectivity.
- Astra reviewed the design and final diff. Its three concrete findings were
  fixed: the Tunnel Deployment name, post-update two-node placement, and DNS
  resolution before expected NetworkPolicy denial. No further review findings.
- The application source is unchanged from the preceding Tunnel/Caddy milestone.
  Its release-image HTTPS/STARTTLS, restricted-role, and backup/restore acceptance
  already passed there. This milestone does not replace those checks.
- **Remote CI passed in full** for implementation commit
  `89ab22d62a4951245c6bf43e38882020fa0a66a6`:
  [CI run 39093136076](https://github.com/HiruneWorks/hirune-axum-kit/actions/runs/39093136076).
  Both `verify` and `container` succeeded. This includes all existing regression
  suites, copied-template verification, and release-image production acceptance.
- On the CI host, all required kernel modules loaded and the complete real-k3s
  acceptance passed: two Ready API Pods on distinct nodes; 634 authenticated
  requests during API replacement and 1266 during a config-only APISIX replacement,
  all with zero errors; retained sessions/quota; and coordinated drained upgrade.
  The Tunnel-labelled probe reached APISIX but could not bypass it to the API.
  The untrusted probe could reach neither gateway nor API. The real Tunnel was
  still disconnected, as intended by the fixture.

## Remaining work

- Full ingress acceptance in this managed workspace requires a different kernel
  with `physdev` support. The complete CI result above supplies the successful
  host-level acceptance; the local partial run remains a failed full check.
- Provision actual production providers, credentials, database grants/backups,
  and cluster capacity. Verify real Tunnel header provenance, public hostname,
  node drain, and any optional integrations on the target cluster. No production
  cluster or real Cloudflare Tunnel was deployed by these local checks.
