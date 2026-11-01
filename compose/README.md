# Container configuration

The optional `files` and `search` profiles start SeaweedFS 4.48 and Meilisearch
1.55.0. `just infra-up-files` enables both. Ordinary `just infra-up` keeps them
disabled. Gateway ports bind to loopback. Compose does not publish the filer or master.
`seaweedfs/s3.local.json` contains public local development credentials. It is not
a production credential file. The S3 gateway requires authentication.

`compose.local.yml` supplies PostgreSQL and Mailpit for local development. Checks
use the same services with unique project names, dynamic ports, and temporary
volumes. `just infra-down` preserves development volumes.

`production/app/Dockerfile` builds API, worker, migration, and management binaries
from the repository-root build context. It excludes environment secrets and runs
as an unprivileged user. See [deployment](../deploy/README.md) for runtime wiring.

`data-services/compose.yaml` runs optional self-hosted PostgreSQL and Meilisearch outside
k3s. Deployments pull the version or digest in `POSTGRES_IMAGE` and do not build.
`production/postgres/Dockerfile` adds pgBackRest and the non-root backup scheduler.
The PostgreSQL release workflow verifies and publishes this image independently
of application releases. `data-services/compose.build.yaml` enables explicit
local/test builds. Use the explicit dev/prod `data-*`, `backup-*`, and new-volume `restore`
commands in `deploy/justfile`. See the [data runbook](../deploy/data-services/README.md).

`verification/` contains the disposable production-mode acceptance fixture:
private TLS certificates, an HTTPS proxy, STARTTLS Mailpit, restricted PostgreSQL
roles, and database restore checks against the release image. Run it through
`just test-production`; see [verification scope](../docs/development.md#production-fixture-scope).
Its scripts provision test infrastructure and are not deployment scripts.
