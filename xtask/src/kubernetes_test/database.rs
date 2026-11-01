use super::*;

impl Fixture {
    pub(super) fn verify_database_roles(&self) -> Result<()> {
        for (role, statement, allowed) in [
            (
                "runtime",
                "CREATE TABLE forbidden_runtime_ddl (value integer)",
                false,
            ),
            ("runtime", "DELETE FROM audit_events WHERE false", false),
            ("runtime", "DELETE FROM api_usage WHERE false", false),
            ("maintenance", "DELETE FROM audit_events WHERE false", true),
            ("maintenance", "DELETE FROM api_usage WHERE false", true),
            (
                "maintenance",
                "UPDATE audit_events SET event=event WHERE false",
                false,
            ),
        ] {
            let mut child = self
                .kubectl(&[
                    "exec",
                    "-i",
                    "deployment/postgres",
                    "--",
                    "psql",
                    "-U",
                    "bootstrap",
                    "-d",
                    "kit",
                    "-v",
                    "ON_ERROR_STOP=1",
                ])
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()?;
            child
                .stdin
                .take()
                .context("privilege check stdin missing")?
                .write_all(
                    format!("BEGIN; SET LOCAL ROLE {role}; {statement}; ROLLBACK;\n").as_bytes(),
                )?;
            ensure!(
                child.wait()?.success() == allowed,
                "Kubernetes database role contract failed for {role}"
            );
        }
        println!(
            "Kubernetes runtime and maintenance roles enforce DDL and audit/usage boundaries."
        );
        Ok(())
    }

    pub(super) fn create_database_roles(
        &self,
        schema_password: &str,
        runtime_password: &str,
        maintenance_password: &str,
    ) -> Result<()> {
        // Role and ownership DDL is fixture administration, outside SeaORM migrations.
        let sql = format!(
            "CREATE ROLE schema_owner LOGIN PASSWORD '{schema_password}';\nCREATE ROLE runtime LOGIN PASSWORD '{runtime_password}';\nCREATE ROLE maintenance LOGIN PASSWORD '{maintenance_password}';\nCREATE DATABASE kit OWNER schema_owner;\n\\connect kit\nREVOKE CREATE ON SCHEMA public FROM PUBLIC;\n"
        );
        let mut child = self
            .kubectl(&[
                "exec",
                "-i",
                "deployment/postgres",
                "--",
                "psql",
                "-U",
                "bootstrap",
                "-d",
                "postgres",
                "-v",
                "ON_ERROR_STOP=1",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        child
            .stdin
            .take()
            .context("role setup stdin missing")?
            .write_all(sql.as_bytes())?;
        ensure!(
            child.wait()?.success(),
            "Kubernetes database role setup failed"
        );
        Ok(())
    }
}
