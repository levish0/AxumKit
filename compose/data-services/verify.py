"""Verify disposable data services, physical recovery, and backup scheduling."""

import datetime as dt
import importlib.util
import json
import os
import pathlib
import secrets
import shutil
import socket
import subprocess
import tempfile
import time
import urllib.error
import urllib.request

SOURCE = pathlib.Path(__file__).resolve().parents[2]
PROJECT = "hirune-data-test-" + secrets.token_hex(6)


def redact_output(value, environment):
    for name in ["MIGRATION_DATABASE_URL", "POSTGRES_PASSWORD", "SCHEMA_OWNER_PASSWORD", "RUNTIME_PASSWORD", "MAINTENANCE_PASSWORD", "PGBACKREST_REPO1_CIPHER_PASS", "MEILI_MASTER_KEY"]:
        secret = (environment or {}).get(name)
        if secret:
            value = value.replace(secret, "[redacted]")
    return value


def execute(arguments, *, env=None, text=None, success=True):
    result = subprocess.run(arguments, input=text, text=True, capture_output=True, env=env)
    result.stdout = redact_output(result.stdout, env)
    result.stderr = redact_output(result.stderr, env)
    if success and result.returncode:
        raise RuntimeError("Command failed: " + " ".join(arguments[:6]) + "\n" + result.stdout[-2500:] + result.stderr[-3500:])
    return result


def free_port():
    with socket.socket() as listener:
        listener.bind(("127.0.0.1", 0))
        return listener.getsockname()[1]


def scheduling():
    spec = importlib.util.spec_from_file_location("backup_scheduler", SOURCE / "compose/data-services/backup_scheduler.py")
    scheduler = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(scheduler)
    sunday = dt.datetime(2026, 10, 18, 3, tzinfo=dt.timezone.utc)

    def backup(kind, moment):
        return {"type": kind, "timestamp": {"stop": moment.timestamp()}}

    previous = backup("full", sunday - dt.timedelta(days=7))
    assert scheduler.due_backup(sunday, [], 3) == "full"
    assert scheduler.due_backup(sunday - dt.timedelta(seconds=1), [previous, backup("diff", sunday - dt.timedelta(days=1))], 3) is None
    assert scheduler.due_backup(sunday, [previous], 3) == "full"
    assert scheduler.due_backup(sunday + dt.timedelta(hours=1), [previous, backup("diff", sunday)], 3) == "full"
    current = backup("full", sunday)
    assert scheduler.due_backup(sunday, [current], 3) is None
    assert scheduler.due_backup(sunday + dt.timedelta(days=1), [current], 3) == "diff"
    assert scheduler.due_backup(sunday + dt.timedelta(days=3), [previous], 3) == "full"
    print("UTC boundaries, initial full, missed slots, and weekly-full priority passed.")


def verify(root):
    for directory in ["compose/data-services", "compose/production/postgres", ".envs/.example/prod/data-services"]:
        shutil.copytree(SOURCE / directory, root / directory)
    profile = root / ".envs/.prod/data-services"
    shutil.copytree(root / ".envs/.example/prod/data-services", profile)
    pg_port, meili_port = free_port(), free_port()
    passwords = {name: secrets.token_urlsafe(32) for name in ["POSTGRES_PASSWORD", "SCHEMA_OWNER_PASSWORD", "RUNTIME_PASSWORD", "MAINTENANCE_PASSWORD"]}
    database = "hirune_fixture"
    data_volume = PROJECT + "-postgres"
    postgres_image = os.environ.get("DATA_TEST_POSTGRES_IMAGE")
    build_postgres = not postgres_image
    postgres_image = postgres_image or PROJECT + "-postgres:local"
    (profile / "data.env").write_text(f"DATA_PROJECT_NAME={PROJECT}\nDATA_BIND_ADDRESS=127.0.0.1\nPOSTGRES_PORT={pg_port}\nMEILISEARCH_PORT={meili_port}\nPOSTGRES_DATA_VOLUME={data_volume}\nBACKUP_HOUR_UTC=3\n")
    master_key = secrets.token_urlsafe(32)
    database_environment = {"POSTGRES_USER": "postgres", "POSTGRES_DB": "postgres", "APPLICATION_DATABASE": database, **passwords}
    repository_environment = {"PGBACKREST_REPO1_TYPE": "posix", "PGBACKREST_REPO1_PATH": "/backup", "PGBACKREST_REPO1_CIPHER_TYPE": "aes-256-cbc", "PGBACKREST_REPO1_CIPHER_PASS": secrets.token_urlsafe(32)}
    search_environment = {"MEILI_ENV": "production", "MEILI_NO_ANALYTICS": "true", "MEILI_MASTER_KEY": master_key}
    environment = {**os.environ, "DEPLOY_ENV": "prod", "POSTGRES_IMAGE": postgres_image, **database_environment, **repository_environment, **search_environment}
    # Compose inherits fixture credentials from the process; files contain names only.
    service_variables = {
        "postgres": [*database_environment, *repository_environment],
        "backup-scheduler": list(repository_environment),
        "restore": list(repository_environment),
        "recovery-check": list(repository_environment),
        "meilisearch": list(search_environment),
    }
    override = root / "fixture.yaml"
    override.write_text("services:\n" + "".join(
        f"  {name}:\n    env_file: !reset []\n    environment:\n" + "".join(f"      {variable}:\n" for variable in variables)
        + ("    volumes: [fixture_backup:/backup]\n    pull_policy: never\n" if name != "meilisearch" else "")
        for name, variables in service_variables.items()
    ) + "volumes:\n  fixture_backup: {}\n")
    fixture_secrets = [*passwords.values(), master_key, repository_environment["PGBACKREST_REPO1_CIPHER_PASS"]]
    for path in [*profile.glob("*.env"), override]:
        assert all(secret not in path.read_text() for secret in fixture_secrets), "Fixture configuration contains a generated secret"
    if environment.get("CODEX_PROXY_CERT"):
        public_ca = root / "build-proxy-ca.pem"
        shutil.copyfile(environment["CODEX_PROXY_CERT"], public_ca)
        environment["DATA_BUILD_CA_FILE"] = str(public_ca)
    base = ["docker", "compose", "--project-name", PROJECT, "--env-file", str(profile / "data.env"), "-f", str(root / "compose/data-services/compose.yaml")]
    base += ["-f", str(override)]
    recovery_volumes = []

    def compose(*arguments, success=True, extra_env=None):
        return execute(base + list(arguments), env={**environment, **(extra_env or {})}, success=success)

    def sql(statement, role="postgres", service="postgres", name=database, success=True):
        return execute(base + ["exec", "-T", "--user", "postgres", service, "psql", "-X", "-h", "/tmp" if service == "recovery-check" else "/var/run/postgresql", "-U", role, "-d", name, "-v", "ON_ERROR_STOP=1", "-At"], env=environment, text=statement, success=success)

    def pgbackrest(*arguments, service="postgres"):
        return compose("exec", "-T", "--user", "postgres", service, "pgbackrest", "--stanza=db", *arguments)

    def search(method, path, payload=None, authenticated=True):
        headers = {"Authorization": "Bearer " + master_key} if authenticated else {}
        if payload is not None:
            headers["Content-Type"] = "application/json"
        request = urllib.request.Request(f"http://127.0.0.1:{meili_port}" + path, data=json.dumps(payload).encode() if payload is not None else None, headers=headers, method=method)
        opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
        try:
            response = opener.open(request, timeout=5)
        except urllib.error.HTTPError as error:
            response = error
        with response:
            body = response.read()
            return response.status, json.loads(body) if body else None

    try:
        if build_postgres:
            compose("--file", str(root / "compose/data-services/compose.build.yaml"), "build", "postgres")
        image_id = execute(["docker", "image", "inspect", "--format", "{{.Id}}", postgres_image]).stdout.strip()
        environment["POSTGRES_IMAGE"] = image_id
        print("Data-service acceptance uses PostgreSQL image " + postgres_image + " (" + image_id + ")", flush=True)
        compose("run", "--rm", "--no-deps", "--user", "root", "--entrypoint", "sh", "postgres", "-c", "mkdir -p /backup && chown postgres:postgres /backup")
        compose("up", "-d", "--wait", "postgres", "meilisearch")
        pgbackrest("stanza-create")
        pgbackrest("check")
        migration_environment = {**environment, "MIGRATION_RUNTIME_ROLE": "runtime", "MIGRATION_MAINTENANCE_ROLE": "maintenance"}
        image = os.environ.get("DATA_TEST_APP_IMAGE")
        def migrate(success=True):
            if image:
                migration_environment["MIGRATION_DATABASE_URL"] = f"postgres://schema_owner:{passwords['SCHEMA_OWNER_PASSWORD']}@postgres:5432/{database}"
                return execute(["docker", "run", "--rm", "--network", PROJECT + "_data", "-e", "MIGRATION_DATABASE_URL", "-e", "MIGRATION_RUNTIME_ROLE", "-e", "MIGRATION_MAINTENANCE_ROLE", "--entrypoint", "migration", image], env=migration_environment, success=success)
            else:
                execute(["cargo", "build", "--locked", "--package", "migration"])
                migration_environment["MIGRATION_DATABASE_URL"] = f"postgres://schema_owner:{passwords['SCHEMA_OWNER_PASSWORD']}@127.0.0.1:{pg_port}/{database}"
                binary = "migration.exe" if os.name == "nt" else "migration"
                return execute([str(SOURCE / "target/debug" / binary)], env=migration_environment, success=success)
        migrate()
        sql("GRANT CREATE ON SCHEMA public TO runtime")
        assert migrate(success=False).returncode, "Existing DDL authority was accepted"
        sql("REVOKE CREATE ON SCHEMA public FROM runtime")
        sql("GRANT UPDATE ON audit_events TO PUBLIC")
        assert migrate(success=False).returncode, "PUBLIC-derived audit mutation was accepted"
        sql("REVOKE UPDATE ON audit_events FROM PUBLIC")
        sql("GRANT UPDATE (event) ON audit_events TO PUBLIC")
        assert migrate(success=False).returncode, "PUBLIC column audit mutation was accepted"
        sql("REVOKE UPDATE (event) ON audit_events FROM PUBLIC")
        migrate()
        for role, denied in [("runtime", ["CREATE TABLE forbidden(value integer)", "DELETE FROM audit_events WHERE false", "UPDATE api_usage SET operation=operation WHERE false", "TRUNCATE api_usage", "DELETE FROM seaql_migrations WHERE false"]), ("maintenance", ["UPDATE audit_events SET event=event WHERE false", "TRUNCATE api_usage", "CREATE TABLE forbidden(value integer)"])]:
            for statement in denied:
                result = sql(statement, role=role, success=False)
                assert result.returncode and "permission denied" in result.stderr, result.stderr
        sql("DELETE FROM audit_events WHERE false; DELETE FROM api_usage WHERE false", role="maintenance")
        sql("CREATE TABLE backup_probe(value text PRIMARY KEY); INSERT INTO backup_probe VALUES ('before')", role="schema_owner")
        full_backup = pgbackrest("--type=full", "--log-level-console=detail", "backup")
        assert "DETAIL:" in full_backup.stdout and "%" in full_backup.stdout, "Backup progress was not reported"
        history = json.loads(pgbackrest("--output=json", "info").stdout)
        label = history[0]["backup"][-1]["label"]
        bundles = json.loads(pgbackrest("--output=json", "repo-ls", f"backup/db/{label}/bundle").stdout)
        assert any(entry["type"] == "file" and entry["size"] > 0 for entry in bundles.values()), "Backup has no file bundles"
        print("Encrypted full backup stored file bundles and reported native percentage progress.")
        # Repository backup stop timestamps use whole seconds.
        time.sleep(1.1)
        target = sql("SELECT clock_timestamp()::text").stdout.strip()
        time.sleep(1.1)
        sql("INSERT INTO backup_probe VALUES ('after')", role="schema_owner")
        pgbackrest("check")
        # The real scheduler identity can back up with PGDATA mounted read-only.
        compose("run", "--rm", "--no-deps", "--entrypoint", "pgbackrest", "backup-scheduler", "--stanza=db", "--type=diff", "backup")
        compose("up", "-d", "backup-scheduler")
        assert compose("exec", "-T", "backup-scheduler", "id", "-u").stdout.strip() != "0"
        print("Schema-owner migration grants, append-only runtime roles, full/differential backup, and non-root read-only scheduler passed.")
        rejected = compose("--profile", "recovery", "run", "--rm", "--no-deps", "restore", success=False, extra_env={"DATA_RESTORE_VOLUME": data_volume})
        assert rejected.returncode and "empty volume" in rejected.stderr
        assert sql("SELECT count(*) FROM backup_probe").stdout.strip() == "2"
        for label, timestamp, expected in [("latest", None, "2"), ("pitr", target, "1")]:
            volume = PROJECT + "-recovery-" + label
            recovery_volumes.append(volume)
            execute(["docker", "volume", "create", volume])
            environment["DATA_RESTORE_VOLUME"] = volume
            arguments = ["--profile", "recovery", "run", "--rm", "--no-deps", "restore"]
            if timestamp:
                arguments += ["--type=time", "--target=" + timestamp, "--target-action=promote"]
            compose(*arguments)
            compose("--profile", "recovery", "up", "-d", "--no-deps", "recovery-check")
            deadline = time.monotonic() + 60
            while True:
                ready = sql("SELECT pg_is_in_recovery()", service="recovery-check", success=False)
                if ready.returncode == 0 and ready.stdout.strip() == "f":
                    break
                assert time.monotonic() < deadline, "Restored database did not finish WAL replay"
                time.sleep(0.5)
            assert sql("SELECT count(*) FROM backup_probe", service="recovery-check").stdout.strip() == expected
            assert sql("SELECT has_table_privilege('runtime','api_usage','DELETE')", service="recovery-check").stdout.strip() == "f"
            compose("--profile", "recovery", "rm", "--stop", "--force", "recovery-check")
            print(label + " recovery into an isolated new volume preserved data and grants.")
        compose("restart", "postgres")
        compose("up", "-d", "--wait", "postgres")
        assert sql("SELECT count(*) FROM backup_probe").stdout.strip() == "2"
        assert search("GET", "/indexes", authenticated=False)[0] in [401, 403]
        status, task = search("POST", "/indexes", {"uid": "persistence", "primaryKey": "id"})
        assert status == 202
        for task in [task, search("POST", "/indexes/persistence/documents", [{"id": "survives", "text": "fixture"}])[1]]:
            deadline = time.monotonic() + 30
            while search("GET", "/tasks/" + str(task["taskUid"]))[1]["status"] != "succeeded":
                assert time.monotonic() < deadline, "Search indexing did not complete"
                time.sleep(0.2)
        compose("up", "-d", "--force-recreate", "--wait", "meilisearch")
        assert search("GET", "/indexes/persistence/documents/survives")[0] == 200
        print("PostgreSQL restart and authenticated Meilisearch replacement retained persistent state.")
    except BaseException:
        logs = compose("--profile", "recovery", "logs", "--tail", "35", success=False).stdout
        print(logs)
        raise
    finally:
        compose("--profile", "recovery", "down", "--volumes", "--remove-orphans", success=False)
        for volume in recovery_volumes:
            execute(["docker", "volume", "rm", volume], success=False)
        if build_postgres:
            execute(["docker", "image", "rm", postgres_image], success=False)


if __name__ == "__main__":
    scheduling()
    with tempfile.TemporaryDirectory(prefix="hirune-data-") as directory:
        verify(pathlib.Path(directory))
    print("Data-service acceptance passed with an encrypted local fixture repository. Real S3/R2 was not contacted.")
