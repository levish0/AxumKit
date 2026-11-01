use anyhow::{Context, Result, ensure};
use base64::Engine;
use std::{fs, path::PathBuf, process::Command};
use tempfile::TempDir;
use uuid::Uuid;

pub fn verify() -> Result<()> {
    let fixture = ProductionFixture::prepare()?;
    let result = fixture.verify();
    if result.is_err() {
        let _ = fixture.compose(&[
            "logs",
            "--no-color",
            "--tail",
            "100",
            "api",
            "worker",
            "worker-untrusted",
            "worker-hostname",
            "maintenance-worker",
            "proxy",
            "mailpit",
        ]);
    }
    result?;
    println!(
        "Local release-image production acceptance passed: HTTPS, STARTTLS, cookies, origins, database privileges, backup/restore, and encrypted MFA recovery."
    );
    Ok(())
}

struct ProductionFixture {
    project: String,
    application_image: String,
    application_image_owned: bool,
    verification_image: String,
    bootstrap_password: String,
    schema_owner_password: String,
    runtime_password: String,
    maintenance_password: String,
    directory: TempDir,
    compose_file: PathBuf,
}

impl ProductionFixture {
    fn prepare() -> Result<Self> {
        let project = format!("kit-production-{}", Uuid::now_v7().simple());
        let supplied_image = std::env::var("PRODUCTION_TEST_IMAGE").ok();
        let fixture = Self {
            application_image: supplied_image
                .clone()
                .unwrap_or_else(|| format!("{project}:application")),
            application_image_owned: supplied_image.is_none(),
            verification_image: format!("{project}:verification"),
            bootstrap_password: auth_core::token::generate_secure_token(),
            schema_owner_password: auth_core::token::generate_secure_token(),
            runtime_password: auth_core::token::generate_secure_token(),
            maintenance_password: auth_core::token::generate_secure_token(),
            directory: tempfile::tempdir()?,
            compose_file: std::env::current_dir()?.join("compose/verification/compose.yml"),
            project,
        };
        fixture.write_environment()?;
        if fixture.application_image_owned {
            println!("Building the canonical release application image for production acceptance.");
            crate::run(Command::new("docker").args([
                "build",
                "--file",
                "compose/production/app/Dockerfile",
                "--tag",
                &fixture.application_image,
                ".",
            ]))?;
        } else {
            println!(
                "Using supplied production image: {}",
                fixture.application_image
            );
        }
        crate::run(Command::new("docker").args([
            "build",
            "--file",
            "compose/verification/Dockerfile",
            "--tag",
            &fixture.verification_image,
            ".",
        ]))?;
        Ok(fixture)
    }

    fn write_environment(&self) -> Result<()> {
        let raw_key = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(auth_core::token::generate_secure_token())?;
        let encoded_key = base64::engine::general_purpose::STANDARD.encode(raw_key);
        let secret_keys = serde_json::json!({"verification-v1": encoded_key});
        let contents = format!(
            "APP_ENV=production\n\
             APP_NAME=Production acceptance\n\
             PUBLIC_URL=https://api:8443/\n\
             ALLOWED_ORIGINS=https://api:8443\n\
             BIND_ADDRESS=0.0.0.0:3000\n\
             DATABASE_URL=postgresql://runtime:{}@postgres/kit\n\
             DATABASE_MAX_CONNECTIONS=5\n\
             ACTIVE_SECRET_KEY=verification-v1\n\
             SECRET_KEYS='{secret_keys}'\n\
             RATE_LIMIT_KEY={}\n\
             SMTP_HOST=mailpit\n\
             SMTP_PORT=1025\n\
             SMTP_STARTTLS=true\n\
             SMTP_CA_CERTIFICATE=/certificates/ca.crt\n\
             MAIL_FROM=Verification <noreply@example.test>\n\
             PASSWORD_CONCURRENCY=2\n\
             WORKER_CONCURRENCY=2\n\
             WORKER_DRAIN_SECONDS=5\n\
             RUST_LOG=info,sqlx=off,sea_orm=off,reqwest=off,hyper=off\n",
            self.runtime_password,
            auth_core::token::generate_secure_token(),
        );
        fs::write(self.directory.path().join("app.env"), contents)?;
        Ok(())
    }

    fn command(&self) -> Command {
        let mut command = Command::new("docker");
        command
            .env("PRODUCTION_IMAGE", &self.application_image)
            .env("VERIFICATION_IMAGE", &self.verification_image)
            .env("VERIFICATION_BOOTSTRAP_PASSWORD", &self.bootstrap_password)
            .env(
                "VERIFICATION_SCHEMA_OWNER_PASSWORD",
                &self.schema_owner_password,
            )
            .env("VERIFICATION_RUNTIME_PASSWORD", &self.runtime_password)
            .env(
                "VERIFICATION_MAINTENANCE_PASSWORD",
                &self.maintenance_password,
            )
            .env(
                "VERIFICATION_APP_ENV",
                self.directory.path().join("app.env"),
            )
            .args([
                "compose",
                "--profile",
                "*",
                "--project-name",
                &self.project,
                "--file",
            ])
            .arg(&self.compose_file);
        command
    }

    fn compose(&self, arguments: &[&str]) -> Result<()> {
        crate::run(self.command().args(arguments))
    }

    fn check(&self, script: &str, operation: Option<&str>) -> Result<()> {
        let mut command = self.command();
        command.args(["run", "--rm", "--no-deps", "verifier", script]);
        if let Some(operation) = operation {
            command.arg(operation);
        }
        crate::run(&mut command)
    }

    fn verify(&self) -> Result<()> {
        self.compose(&["up", "--detach", "--wait", "postgres"])?;
        self.check("certificates.py", None)?;
        self.check("database.py", Some("roles"))?;
        self.compose(&["run", "--rm", "--no-deps", "migration"])?;
        self.check("database.py", Some("grants"))?;
        self.compose(&["up", "--detach", "mailpit", "api", "proxy"])?;
        for worker in ["worker-untrusted", "worker-hostname"] {
            self.compose(&["up", "--detach", worker])?;
            self.check("http_checks.py", Some("rejected-certificate"))?;
            self.compose(&["stop", "--timeout", "10", worker])?;
        }
        self.compose(&["up", "--detach", "worker"])?;
        self.check("http_checks.py", Some("prepare"))?;
        self.allocate_external_api_quota()?;
        self.check("http_checks.py", Some("external-api"))?;
        self.check("database.py", Some("privileges"))?;
        self.compose(&["run", "--rm", "--no-deps", "maintenance"])?;
        self.compose(&["up", "--detach", "maintenance-worker"])?;
        self.check("database.py", Some("maintenance"))?;
        self.compose(&[
            "stop",
            "--timeout",
            "40",
            "proxy",
            "api",
            "worker",
            "maintenance-worker",
        ])?;
        self.check("database.py", Some("backup"))?;
        self.check("database.py", Some("restore"))?;
        self.compose(&["start", "api", "worker", "proxy", "maintenance-worker"])?;
        self.check("http_checks.py", Some("restored"))?;
        self.check("database.py", Some("privileges"))?;
        self.assert_runtime_environment()?;
        Ok(())
    }

    fn allocate_external_api_quota(&self) -> Result<()> {
        let output = self
            .command()
            .args([
                "run",
                "--rm",
                "--no-deps",
                "verifier",
                "http_checks.py",
                "api-client-id",
            ])
            .output()?;
        ensure!(
            output.status.success(),
            "production API client lookup failed"
        );
        let client_id = Uuid::parse_str(String::from_utf8(output.stdout)?.trim())?.to_string();
        self.compose(&[
            "run",
            "--rm",
            "--no-deps",
            "--entrypoint",
            "manage",
            "api",
            "set-api-client-quota",
            &client_id,
            "100",
            "2",
        ])
    }

    fn assert_runtime_environment(&self) -> Result<()> {
        let output = self
            .command()
            .args(["config", "--format", "json"])
            .output()?;
        ensure!(
            output.status.success(),
            "production Compose configuration failed"
        );
        let configuration: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        for name in ["api", "worker"] {
            let environment = &configuration["services"][name]["environment"];
            ensure!(
                environment["APP_ENV"] == "production"
                    && environment.get("MIGRATION_DATABASE_URL").is_none()
                    && environment["DATABASE_URL"]
                        .as_str()
                        .context("runtime database configuration missing")?
                        .starts_with("postgresql://runtime:"),
                "production runtime credential boundary failed"
            );
            ensure!(
                configuration["services"][name]["read_only"] == true,
                "production runtime filesystem is not read-only"
            );
            let rendered_environment = environment.to_string();
            ensure!(
                [
                    &self.bootstrap_password,
                    &self.schema_owner_password,
                    &self.maintenance_password
                ]
                .iter()
                .all(|password| !rendered_environment.contains(password.as_str())),
                "production runtime received another database role's credentials"
            );
            let user = self
                .command()
                .args(["exec", "-T", name, "id", "-u"])
                .output()?;
            ensure!(
                user.status.success() && String::from_utf8_lossy(&user.stdout).trim() == "10001",
                "production runtime is not using the application user"
            );
        }
        Ok(())
    }
}

impl Drop for ProductionFixture {
    fn drop(&mut self) {
        match self
            .command()
            .args(["down", "--volumes", "--remove-orphans"])
            .output()
        {
            Ok(output) if output.status.success() => {}
            _ => eprintln!("Could not remove owned production fixture {}", self.project),
        }
        let mut images = vec![self.verification_image.as_str()];
        if self.application_image_owned {
            images.push(&self.application_image);
        }
        let _ = Command::new("docker")
            .args(["image", "rm"])
            .args(images)
            .output();
    }
}
