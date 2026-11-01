# Single-host data services

This optional profile runs PostgreSQL 18.6, Meilisearch 1.55.0, and a pgBackRest
backup scheduler with Docker Compose. The same Linux machine can also run k3s.
Application `up`, rolling, drain, and `down` commands still target Kubernetes.
They do not upgrade or stop these data services.

PostgreSQL has pgBackRest in its image for continuous WAL archiving. The separate
non-root scheduler reads the same data volume read-only and uses a shared Unix
socket and lock directory. It needs no Docker socket or host cron. A Kubernetes
CronJob would need host-path/node coupling or a secured remote pgBackRest service
to read this Docker-managed database. Keep backup execution beside the database.

One machine provides no host failover. Pod replicas permit compatible process
rollouts; off-host backups provide recovery after machine or disk loss.

## Configure the host

Install Docker Engine with Compose v2, Rust, and just. Copy the dev/prod examples
to ignored `.envs/.dev/` and `.envs/.prod/`. Each `data-services/` directory contains:

- `data.env`: explicit Docker context, project, published `POSTGRES_IMAGE`, private
  bind address, distinct ports, PostgreSQL volume name, and the UTC backup hour.
- `postgres.env`: bootstrap and separate schema-owner, runtime, and maintenance
  passwords. Use independent random URL-safe passwords. Keep `POSTGRES_USER`
  and `POSTGRES_DB` set to `postgres`.
- `pgbackrest.env`: S3-compatible repository endpoint, bucket, credentials,
  environment-specific path, and independent repository encryption secret.
- `meilisearch.env`: production mode and an independent master key.

Use different projects, volume names, ports, database passwords, master keys,
repository paths or buckets, and backup encryption secrets for dev and prod.
Changing env-file passwords does not change passwords in an existing database.
Rotate existing roles explicitly and update their Kubernetes credentials together.

Set `DATA_DOCKER_CONTEXT` to the intended host's Docker context. On the data host,
this is usually `default`. Every data command supplies that context explicitly.
Do not put `DEPLOY_ENV` in `data.env`; the command selects dev or prod.

Set `DATA_BIND_ADDRESS` to a private address on this host that k3s Pods can reach.
Loopback works only for host-local checks; a Pod's `127.0.0.1` is the Pod itself.
A private Docker bridge gateway address can be used on a host without a private
NIC after verifying the route. Do not use a public or wildcard address. Avoid
overlap between Docker networks, the k3s Pod CIDR, and the Service CIDR.

Restrict access to PostgreSQL and Meilisearch to the required node/Pod sources
and trusted operators. Check actual source addresses from k3s. Docker-published
ports traverse Docker's forwarding rules; UFW rules alone do not establish this
boundary. Use the host's Docker-aware firewall path and upstream network rules.
The profile does not change host firewall rules automatically.

The example limits PostgreSQL and Meilisearch to 2 GiB each and the backup
scheduler to 512 MiB. Reserve memory and disk I/O for k3s, both environments,
surge Pods, indexing, and backups. Tune from measurements. The PostgreSQL volume
must live on the intended persistent Docker data disk. This profile never removes
data volumes through its operator commands. Major PostgreSQL or Meilisearch
upgrades need their documented migration procedures; changing an image tag is
not a data upgrade plan.

## Publish and select the PostgreSQL image

Each project publishes its own PostgreSQL image. The workflow uses the current
repository name, not a shared template image. After the workflow is on the default
branch, create an independent PostgreSQL tag such as `postgres-v18.6-1`:

```sh
git tag postgres-v18.6-1
git push origin postgres-v18.6-1
```

The `PostgreSQL image` workflow builds one candidate and the application migration
image. It verifies encrypted full/differential backup, file bundles, progress,
restricted roles, and latest/PITR recovery. It publishes that same PostgreSQL
candidate without rebuilding as `ghcr.io/<owner>/<repository>-postgres:18.6-1`.
Repository names are lowercased. A manual workflow run verifies without publishing.
The workflow needs permission to write GitHub Packages. Keep image packages
available to the selected data host; authenticate Docker for private packages.

Copy the published `POSTGRES_IMAGE=...@sha256:...` reference from the workflow
summary into the selected `data-services/data.env`. The example image is a
placeholder. An explicit version tag is also accepted; untagged images and
`latest` are rejected. Prefer a digest to select the exact tested bytes.
Use a new PostgreSQL release tag for changes to PostgreSQL, pgBackRest, backup
configuration, or container tools. Keep security updates in this release cycle.
Application releases do not change `POSTGRES_IMAGE` or restart these data services.

The base image includes the PostgreSQL and backup tools, shared configuration,
and non-secret scripts. Runtime credentials stay in the ignored environment files.

## Start and connect Kubernetes

Provision the backup bucket outside this machine. Use the endpoint hostname
without a URL scheme or bucket path. TLS verification stays enabled. The default
uses path-style S3 requests and AES-256-CBC repository encryption. Retain the
repository encryption secret outside the machine, together with the application
encryption keys and deployment credentials needed for recovery.

From `deploy/`, run:

```sh
just data-services-check prod
just data-services-up prod
just backup-info prod
```

`data-services-up` pulls the selected images before changing containers. It stops
the backup scheduler and starts PostgreSQL and Meilisearch without building.
It creates and checks the pgBackRest stanza, takes an initial full backup if none exists,
then starts the scheduler. Repository failure stops this setup command. A running
PostgreSQL can remain healthy during later archive failures; monitor backups
separately. Do not delete a stanza to resolve an error without a recovery plan.

Configure the matching Kubernetes overlay:

1. Set runtime and maintenance `DATABASE_URL` values to this host's private bind
   address and selected PostgreSQL port. Use the corresponding role/password and
   `hirune_dev` or `hirune_prod` database.
2. Set `MIGRATION_DATABASE_URL` to the schema-owner connection. In the same
   `k3s/migration.env`, set `MIGRATION_RUNTIME_ROLE=runtime` and
   `MIGRATION_MAINTENANCE_ROLE=maintenance`.
3. Set `MEILISEARCH_URL=http://<private-address>:<selected-port>` and
   `MEILISEARCH_ALLOW_HTTP=true` in `k3s/app.env`. Put the matching
   `MEILISEARCH_API_KEY` in `k3s/secrets.env`.
4. Run `just check prod`, then `just up prod`. Initialize configured search with
   `just manage prod initialize-integrations`.

These direct same-host connections use the private host network. The stock data
profile does not terminate PostgreSQL or Meilisearch TLS. Use `sslmode=disable`
explicitly for this PostgreSQL path. If data traffic crosses an untrusted network,
configure TLS and CA trust before using that path. Do not disable TLS verification
for the remote backup repository.

The bootstrap script creates three roles only for an empty database volume.
The optional migration grant step runs after schema migration and before the
migration Job completes. It grants ordinary table DML, preserves read-only
migration history, keeps runtime audit/API usage append-only, and permits only
maintenance deletion on those records. New tables start with read-only default
grants until that step runs. Existing DDL authority or inherited/PUBLIC privileges
that violate this contract cause deployment to fail. Application Pods never
receive bootstrap or schema-owner credentials. Leave both grant-role settings
empty when another operator manages permissions externally.

## Backup operation

```sh
just backup prod full
just backup prod diff
just backup prod incr
just backup-check prod
just backup-info prod
just data-services-logs prod backup-scheduler
```

Manual backup commands show native pgBackRest file, byte, and percentage progress
through `--log-level-console=detail`. Scheduled backups keep the global `info`
console level.

The shared dev/prod configuration enables `repo1-bundle=y`. pgBackRest combines
small files to reduce repository object counts and requests. Bundle sizes use the
pgBackRest defaults. Publish image configuration changes, set `POSTGRES_IMAGE` to
the verified image, then run `just data-services-up <environment>` on the data host.

The scheduler uses successful repository history. It takes a weekly full on
Sunday and a differential on other days at `BACKUP_HOUR_UTC` (default 03:00 UTC,
12:00 Asia/Seoul). After downtime it takes the currently required backup, without
replaying every missed day. An overdue full takes priority over a daily backup.
Failures retry after 60 seconds. Manual and scheduled backups share pgBackRest
locks. Two retained full backups and their required WAL form the default history;
adjust retention in `compose/data-services/pgbackrest.conf` for the recovery requirement.

PostgreSQL continuously archives completed WAL segments. `archive_timeout=60`
requests periodic WAL switches; this is not a guaranteed recovery-point bound.
Archive or network failures can delay delivery. Monitor `pg_stat_archiver`,
repository checks, latest successful backup times, `pg_wal` growth, and disk space.
`max_wal_size` does not cap WAL retained after archive failures. Never suppress an
archive error to make the database appear backed up.

## Restore and cut over

List backup history and choose the recovery target first. Use a unique label:

```sh
just backup-info prod
just restore prod rehearsal
just restore prod point-in-time 2026-10-18T03:00:00+00
```

The command refuses an existing recovery volume. It restores into
`<project>-recovery-<label>`, completes WAL replay in an isolated server with no
TCP listener and archiving disabled, then stops that server. It does not stop,
overwrite, or redirect the live database. A failed restore retains its recovery
volume for inspection. `RESTORE_WAIT_SECONDS` defaults to 3600 and accepts 1 through
86400. If healthy WAL replay needs more time, increase it in `data.env` and run
`just restore-resume prod <label>` to continue the retained isolated recovery.
Select another label to restore a new copy. The command does
not prove that the selected application data is the intended recovery point.
Use a rehearsal to verify the actual data and recovery time before an incident.

For a real cutover, stop application writers and the backup scheduler, then stop
the old PostgreSQL process. Confirm that no other process uses either volume.
Change `POSTGRES_DATA_VOLUME` in the selected `data.env` to the verified recovery
volume and run `just data-services-up <environment>`. Keep the old volume until validation
and a new full backup complete. Never run two writable servers that archive the
same recovered cluster to the same repository. Serialize all data operations.

Physical recovery includes roles and their password hashes. Check Kubernetes
credentials and retained application encryption keys against the recovery point.
Do not rerun fresh-cluster initialization against restored data.

Meilisearch is derived state. Keep search unavailable until its indexes match the
recovered PostgreSQL data. Rebuild its configured indexes from PostgreSQL and
initialize/repair them through management commands; wait for the repair jobs and
check coverage before restoring search traffic. pgBackRest does not back up
Meilisearch, original object-store files, or application environment secrets.

## Validation scope

`compose/data-services/compose.build.yaml` adds build settings only for explicit
local/test use. `just test-data-services` builds a disposable PostgreSQL image
through this override by default. Set `DATA_TEST_POSTGRES_IMAGE` to verify an
existing local candidate without rebuilding or pulling it. The fixture pins all
database, scheduler, and recovery containers to that candidate's image ID.
CI uses this mode for the image it will publish. `DATA_TEST_APP_IMAGE` selects the
application migration image; without it, the fixture builds the native migration
executable. For a manual local image build from the repository root, use:

```sh
POSTGRES_IMAGE=service-postgres:local DEPLOY_ENV=dev docker compose \
  --env-file .envs/.dev/data-services/data.env \
  -f compose/data-services/compose.yaml \
  -f compose/data-services/compose.build.yaml build postgres
```

Run `just test-data-services` with Docker and Python 3. The disposable acceptance fixture
uses the actual production Compose/image and an encrypted local repository. It
checks role restrictions, full/differential backup, read-only non-root scheduling,
latest recovery, point-in-time recovery, live-volume rejection, PostgreSQL restart,
and authenticated Meilisearch persistence after container replacement. CI uses
the canonical release migration executable. This test does not contact real
S3/R2, change production data, verify the target host firewall, or establish host
failover. Rehearse provider access and recovery on the chosen infrastructure.
