# 2026-10-20 — Deployed Kubernetes E2E

## Changes

- Make `just test-k3s` and `just load-test` run the same two-node deployed E2E
  fixture. CI runs it once with the canonical release image. The workflow input
  selects the smoke or extended k6 profile.
- Keep unit, PostgreSQL, production-router, process/SMTP, and provider-contract
  checks focused. Move host-process k6 to the optional `just load-test-local`.
- Add real SeaweedFS and Meilisearch to the disposable Kubernetes fixture.
  A separate Caddy storage ingress preserves the signed Host and enforces a
  30-second total request-body timeout. Provider NetworkPolicies restrict access.
- Provision distinct schema-owner, runtime, and maintenance DB roles. Apply real
  migration grants and check allowed and denied audit/usage/DDL operations.
- Seed workflow users through real signup, worker mail verification, and login.
  Allocate API quotas through the operator command. Supply client credentials
  through a temporary Secret; give client Pods no S3 or Meilisearch credentials.
- Run the shared k6 organization, session, external API, original-file, attached
  content, profile, comment, notification, and search workflows as a Kubernetes
  Job through APISIX and the separate public storage endpoint.
- Verify conditional uploads, zero-byte originals, CORS, unchanged GET/HEAD/range
  downloads, authorization, parent attachments, and worker deletion. Keep a
  separate retained file/post fixture across API replacement and maintenance.
- Fix Linux native diagnostic networking so k6 can reach loopback-only provider
  ports through host networking. Retain the Docker Desktop path on other hosts.

## Decisions and constraints

- Deployed E2E uses the release image, real router, APISIX, worker, and providers.
  Gateway selection does not deterministically select an API Pod. Separate
  placement and rollout checks establish two API replicas on distinct nodes.
- Do not rewrite database timestamps in the deployed fixture. Late-upload,
  reservation-horizon, tombstone, and quota-release cases stay in focused provider
  tests. Object deletion checks do not imply immediate quota release.
- Run workloads before edge-limit fault injection. Give distinct clients distinct
  fixture Cloudflare IPs. Do not relax production account limits for the load run.
- Emit typed aggregate counters and failed check names, not raw k6 logs or bearer
  URLs. Do not connect a real Cloudflare Tunnel or use production services.

## Validation

- `just check` passed locally with serial unit execution.
- Python AST parsing passed for all four Kubernetes verification modules.
- Caddy 2.11.7 accepted the storage ingress configuration; `caddy fmt` formatted it.
- k6 2.2.0 script inspection passed for the shared workload.
- The first native diagnostic run failed with a storage-unavailable response and
  exceeded the latency threshold while a release build was running. This is not
  passing load evidence.
- After the image build completed, `just load-test-local` passed: 148 requests,
  two completed attached-content workflows, two session revocations, no failed
  requests/checks/workflows, and 403.81 ms p95. Its isolated providers were removed.
- The earlier branch CI failed native direct uploads on Linux. Inspection found
  that the bridge client could not reach loopback-only provider ports. The native
  diagnostic now uses host networking on Linux; deployed E2E uses cluster services.
- Final workspace Clippy passed with warnings denied.
- The canonical local release image build passed. An early deployed attempt
  reached two ready nodes before the image existed; image preflight now rejects
  that case before cluster creation.
- Follow-up review found that Caddy's `http.request.host` removes the client port.
  Use `http.request.hostport` to preserve the complete signed authority. Caddy
  validation and Astra's pinned-source review confirmed the correction. Stop and
  remove the older local fixture before rerunning the rebuilt test configuration.
- Full deployed execution and new remote CI remain pending. No production
  deployment occurred.
- The rebuilt local attempt stopped at PostgreSQL image import because a cached
  multi-platform index referenced a digest absent from its archive. Restrict
  Docker export to the daemon's native Linux platform with `--platform=linux`.
  Kubernetes acceptance requires Docker API 1.48 or newer for that option.
- A direct PostgreSQL export/archive inspection passed with all 11 referenced
  blobs present and only `linux/amd64`, the current daemon platform.
- CI run `39362555931` passed all focused checks, release-container/TLS/restore
  acceptance, and data-service acceptance. Its Kubernetes run passed restricted
  roles, presigned PUT/GET/HEAD/range, zero-byte originals, replay, CORS, tenant
  authority, attached posts, filename search, and retained reads after API rollout.
  It then failed the HPA maintenance test because the completed integration-init
  Job retained worker labels and blocked the drain's Pod-deletion wait. Delete
  that Job with foreground propagation immediately after successful initialization.
- Disable k6 raw log output in the optional native diagnostic as well. A prior
  failed native run confirmed signed URLs in automatic request-warning messages.
  Typed summaries and process error-code counters remain available.
- Opened draft PR #21 for the complete presigned-transfer and Kubernetes E2E
  change. Project instructions now require a draft PR after the first reviewable
  branch commit, with further implementation and validation on that PR.
- Astra's follow-up review found the same terminal-Pod drain issue for the k6
  Job's tunnel label. Delete that Job with foreground propagation after its
  summary and thresholds are checked, before deleting its client Secret.
- The older local diagnostic runner received the initialization Job cleanup
  manually after the runner consumed its result. It passed file/attachment/search
  checks and API-replacement continuity, then failed gateway-replacement continuity
  with 7,312 successful requests and one unclassified error. The cluster was
  removed by normal teardown. Keep the zero-error threshold; add safe HTTP-status
  and network-exception-type counts for future continuity failures.

- CI run `39366403135` passed the focused verification job. Its deployed workload
  completed four workflows but failed 12 readiness checks. Push run `39366399637`
  failed six readiness checks at the same commit. Keep the strict thresholds and
  report bounded HTTP-status counters without raw request logs to diagnose them.
- Diagnostic run `39386753870` passed focused checks and completed four Kubernetes
  workflows, but six readiness calls had k6 status 0. This also covers response
  body failures; it does not prove no response arrived. Add numeric client-error
  codes, iteration bounds, and an EOF count without logging raw errors or URLs.
  Synthetic zero-failure and one-transport-failure counter checks both passed.
- A reduced local protocol diagnostic used the existing release API image and
  pinned APISIX for 360 requests, including readiness at five-second iteration
  intervals. All passed. Its initial fixture setup needed a TCP health check in
  place of missing curl and literal LF maintenance-state bytes on Windows.
  This diagnostic does not establish Kubernetes policy readiness.
- Both CI runs at `f634ca8` completed four workflows and failed six readiness
  calls with client code 1212 (connection refused), iteration bounds 0 through 5,
  and no EOF errors. All focused checks and preceding deployment checks passed.
  The failure pattern is consistent with startup connectivity convergence.
- Add a credential-free init container in the same workload Pod. Require both
  selected gateway Pods to be Ready, then poll exact HTTP 204 from both and TCP
  access to the storage proxy before k6 starts. Kubernetes documents asynchronous
  allow-rule application at Pod startup, including differences between nodes.
  Keep every measured readiness check and the strict zero-error thresholds.
- The initialization failure path selects containers explicitly, prints only a
  typed diagnostic summary, and preserves the original Job error. Successful
  startup also reports any retry counts. Astra reviewed the gate and failure
  handling. Local retry, status, redirect-refusal, timeout, and redaction checks
  passed; xtask Clippy, Python syntax, and whitespace checks passed.

## Remaining work

- Final deployed acceptance is reported in [PR #21 checks](https://github.com/HiruneWorks/hirune-axum-kit/pull/21/checks).
  The startup gate still requires the next canonical two-node execution.
- Real Tunnel, browser Fetch, external providers, target-cluster drain, and
  production capacity require their own acceptance when those boundaries change.
