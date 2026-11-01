"""Exercise PostgreSQL ownership, privileges, and offline backup restoration."""

import os
import subprocess
import sys
import time


def connection_environment(role):
    return {**os.environ, "PGPASSWORD": os.environ["PG_" + role.upper() + "_PASSWORD"]}


def redact_passwords(message):
    for name, value in os.environ.items():
        if name.startswith("PG_") and name.endswith("_PASSWORD") and value:
            message = message.replace(value, "[redacted]")
    return message


def sql(role, statement, database="kit", denied=False):
    result = subprocess.run(
        ["psql", "-X", "-h", "postgres", "-U", role, "-d", database,
         "-v", "ON_ERROR_STOP=1", "-At"],
        input=statement,
        text=True,
        capture_output=True,
        env=connection_environment(role),
    )
    if denied:
        assert result.returncode != 0 and any(
            reason in result.stderr for reason in ["permission denied", "must be owner"]
        ), (
            "PostgreSQL did not reject the prohibited runtime operation"
        )
    elif result.returncode:
        raise RuntimeError(f"PostgreSQL operation failed for {role}: {redact_passwords(result.stderr)}")
    return result.stdout.strip()


def create_database():
    sql("bootstrap", "CREATE DATABASE kit OWNER schema_owner;", "postgres")
    sql("bootstrap", "REVOKE CREATE ON SCHEMA public FROM PUBLIC;")


def create_roles():
    # PostgreSQL role/ownership/privilege administration is outside SeaORM migrations.
    for role in ["schema_owner", "runtime", "maintenance"]:
        password = connection_environment(role)["PGPASSWORD"]
        assert password.replace("-", "").replace("_", "").isalnum()
        sql("bootstrap", f"CREATE ROLE {role} LOGIN PASSWORD '{password}';", "postgres")
    create_database()


def grant_runtime_privileges():
    sql("schema_owner", """
        GRANT CONNECT ON DATABASE kit TO runtime, maintenance;
        GRANT USAGE ON SCHEMA public TO runtime, maintenance;
        GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO runtime;
        REVOKE UPDATE, DELETE ON audit_events FROM runtime;
        REVOKE UPDATE, DELETE ON api_usage FROM runtime;
        REVOKE INSERT, UPDATE, DELETE ON seaql_migrations FROM runtime;
        GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO maintenance;
        REVOKE UPDATE ON audit_events FROM maintenance;
        REVOKE UPDATE ON api_usage FROM maintenance;
        REVOKE INSERT, UPDATE, DELETE ON seaql_migrations FROM maintenance;
    """)


def verify_privileges():
    sql("runtime", "CREATE TABLE forbidden_runtime_ddl (value integer);", denied=True)
    sql("runtime", "ALTER TABLE accounts ADD COLUMN forbidden_runtime_column integer;", denied=True)
    sql("runtime", "UPDATE audit_events SET event=event WHERE false;", denied=True)
    sql("runtime", "DELETE FROM audit_events WHERE false;", denied=True)
    sql("runtime", "TRUNCATE audit_events;", denied=True)
    sql("runtime", "UPDATE api_usage SET operation=operation WHERE false;", denied=True)
    sql("runtime", "DELETE FROM api_usage WHERE false;", denied=True)
    sql("runtime", "TRUNCATE api_usage;", denied=True)
    sql("runtime", "DELETE FROM seaql_migrations WHERE false;", denied=True)
    sql("runtime", "SELECT version FROM seaql_migrations LIMIT 1;")
    sql("maintenance", "DELETE FROM audit_events WHERE false;")
    sql("maintenance", "DELETE FROM api_usage WHERE false;")
    sql("maintenance", "UPDATE api_usage SET operation=operation WHERE false;", denied=True)
    sql("maintenance", "UPDATE audit_events SET event=event WHERE false;", denied=True)
    sql("maintenance", "CREATE TABLE forbidden_maintenance_ddl (value integer);", denied=True)
    sql("maintenance", "ALTER TABLE accounts ADD COLUMN forbidden_maintenance_column integer;", denied=True)
    sql("maintenance", "TRUNCATE audit_events;", denied=True)
    sql("maintenance", "TRUNCATE api_usage;", denied=True)
    sql("maintenance", "DELETE FROM seaql_migrations WHERE false;", denied=True)
    assert sql("bootstrap", """
        SELECT count(*) FROM pg_roles WHERE rolname IN ('runtime', 'maintenance')
        AND (rolsuper OR rolcreatedb OR rolcreaterole OR rolbypassrls);
    """) == "0"
    assert sql("bootstrap", "SELECT count(*) FROM accounts WHERE totp_secret IS NOT NULL;") == "1"
    print("Runtime DDL and audit/usage mutation rejected; maintenance audit/usage deletion allowed.")


def backup():
    subprocess.run(
        ["pg_dump", "-h", "postgres", "-U", "schema_owner", "-d", "kit",
         "--format=custom", "--file=/state/database.dump"],
        check=True,
        env=connection_environment("schema_owner"),
    )
    print("PostgreSQL backup saved with encrypted MFA state.")


def verify_maintenance_worker():
    deadline = time.monotonic() + 30
    while time.monotonic() < deadline:
        if sql("runtime", "SELECT count(*) FROM jobs WHERE queue='maintenance' AND state='succeeded';") != "0":
            assert sql("runtime", """
                SELECT count(*) FROM job_attempts attempt JOIN jobs job ON attempt.job_id=job.id
                WHERE job.queue='maintenance' AND attempt.error_code IS NOT NULL;
            """) == "0"
            print("Portable maintenance worker completed its scheduled job with restricted credentials.")
            return
        time.sleep(0.2)
    raise RuntimeError("Scheduled maintenance did not complete")


def restore():
    sql("bootstrap", "DROP DATABASE kit;", "postgres")
    create_database()
    subprocess.run(
        ["pg_restore", "-h", "postgres", "-U", "schema_owner", "-d", "kit",
         "--exit-on-error", "/state/database.dump"],
        check=True,
        env=connection_environment("schema_owner"),
    )
    print("PostgreSQL restored into a newly created database.")


commands = {
    "roles": create_roles,
    "grants": grant_runtime_privileges,
    "privileges": verify_privileges,
    "backup": backup,
    "maintenance": verify_maintenance_worker,
    "restore": restore,
}
if __name__ == "__main__":
    commands[sys.argv[1]]()
