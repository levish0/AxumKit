use crate::kubernetes_environment::Environment;
use anyhow::{Context, Result, bail, ensure};
use std::{
    collections::BTreeMap,
    fs,
    net::Ipv4Addr,
    path::{Path, PathBuf},
    process::Command,
};

pub struct DataServicesDeployment {
    pub directory: PathBuf,
    pub environment: Environment,
    pub project: String,
    context: String,
    pub values: BTreeMap<String, String>,
}

impl DataServicesDeployment {
    pub fn load(root: &Path, environment: Environment) -> Result<Self> {
        let directory = root.join(format!(".envs/.{}/data-services", environment.name()));
        let values: BTreeMap<String, String> = dotenvy::from_path_iter(directory.join("data.env"))
            .map_err(|_| anyhow::anyhow!("copy and configure the selected data environment first"))?
            .collect::<std::result::Result<_, _>>()
            .map_err(|_| anyhow::anyhow!("invalid data environment file"))?;
        ensure!(
            !values.contains_key("DEPLOY_ENV"),
            "DEPLOY_ENV is selected by the command, not data.env"
        );
        let project = values
            .get("DATA_PROJECT_NAME")
            .context("DATA_PROJECT_NAME missing")?
            .clone();
        let context = values
            .get("DATA_DOCKER_CONTEXT")
            .context("DATA_DOCKER_CONTEXT missing")?
            .clone();
        ensure!(
            !context.is_empty() && !context.starts_with('-'),
            "select an explicit Docker context"
        );
        ensure!(valid_name(&project), "invalid data project name");
        ensure!(
            values
                .get("POSTGRES_IMAGE")
                .is_some_and(|image| valid_postgres_image(image)),
            "set POSTGRES_IMAGE to an explicit version tag or sha256 digest; latest is not allowed"
        );
        ensure!(
            values
                .get("POSTGRES_DATA_VOLUME")
                .is_some_and(|value| valid_name(value)),
            "invalid PostgreSQL volume name"
        );
        let address: Ipv4Addr = values
            .get("DATA_BIND_ADDRESS")
            .context("DATA_BIND_ADDRESS missing")?
            .parse()
            .context("select a private IPv4 address")?;
        ensure!(
            address.is_private() || address.is_loopback(),
            "data ports must bind to a private address"
        );
        for key in ["POSTGRES_PORT", "MEILISEARCH_PORT"] {
            ensure!(
                values
                    .get(key)
                    .and_then(|v| v.parse::<u16>().ok())
                    .is_some_and(|v| v > 0),
                "invalid {key}"
            );
        }
        ensure!(
            values["POSTGRES_PORT"] != values["MEILISEARCH_PORT"],
            "data service ports overlap"
        );
        ensure!(
            values
                .get("RESTORE_WAIT_SECONDS")
                .map_or(Some(3600), |v| v.parse::<u64>().ok())
                .is_some_and(|v| (1..=86400).contains(&v)),
            "RESTORE_WAIT_SECONDS must be between 1 and 86400"
        );
        let other_environment = match environment {
            Environment::Dev => "prod",
            Environment::Prod => "dev",
        };
        let other_path = root.join(format!(".envs/.{other_environment}/data-services/data.env"));
        if other_path.exists() {
            let other: BTreeMap<String, String> = dotenvy::from_path_iter(other_path)
                .map_err(|_| anyhow::anyhow!("invalid other data environment"))?
                .collect::<std::result::Result<_, _>>()
                .map_err(|_| anyhow::anyhow!("invalid other data environment"))?;
            if other.get("DATA_DOCKER_CONTEXT") == Some(&context) {
                for key in ["DATA_PROJECT_NAME", "POSTGRES_DATA_VOLUME"] {
                    ensure!(
                        other.get(key) != values.get(key),
                        "dev and prod data targets share {key}"
                    );
                }
                if other.get("DATA_BIND_ADDRESS") == values.get("DATA_BIND_ADDRESS") {
                    for key in ["POSTGRES_PORT", "MEILISEARCH_PORT"] {
                        ensure!(
                            ![other.get("POSTGRES_PORT"), other.get("MEILISEARCH_PORT")]
                                .contains(&values.get(key)),
                            "dev and prod data ports overlap"
                        );
                    }
                }
            }
        }
        Ok(Self {
            directory,
            environment,
            project,
            context,
            values,
        })
    }

    pub fn compose(&self, arguments: &[&str]) -> Command {
        let mut command = self.docker(&[]);
        command
            .envs(&self.values)
            .env("DEPLOY_ENV", self.environment.name())
            .args(["compose", "--project-name", &self.project, "--env-file"])
            .arg(self.directory.join("data.env"))
            .args(["--file", "compose/data-services/compose.yaml"])
            .args(arguments);
        command
    }

    fn docker(&self, arguments: &[&str]) -> Command {
        let mut command = Command::new("docker");
        command.args(["--context", &self.context]).args(arguments);
        command
    }

    fn run(&self, arguments: &[&str]) -> Result<()> {
        crate::run(&mut self.compose(arguments))
    }

    pub fn pgbackrest(&self, arguments: &[&str]) -> Result<()> {
        let mut command = self.compose(&[
            "exec",
            "-T",
            "--user",
            "postgres",
            "postgres",
            "pgbackrest",
            "--stanza=db",
        ]);
        crate::run(command.args(arguments))
    }

    fn require_secrets(&self) -> Result<()> {
        for file in ["postgres.env", "pgbackrest.env", "meilisearch.env"] {
            let contents = fs::read_to_string(self.directory.join(file))?;
            ensure!(
                !contents.contains("REPLACE_"),
                "configure {file} before starting data services"
            );
        }
        Ok(())
    }

    fn recovery_volume(&self, label: &str) -> Result<String> {
        ensure!(
            valid_name(label) && label.len() <= 32,
            "use a short recovery label"
        );
        let volume = format!("{}-recovery-{label}", self.project);
        ensure!(
            volume != self.values["POSTGRES_DATA_VOLUME"],
            "recovery must use a different volume"
        );
        Ok(volume)
    }

    fn restore(&self, label: &str, target: Option<&str>) -> Result<()> {
        let volume = self.recovery_volume(label)?;
        if let Some(target) = target {
            ensure!(
                !target.trim().is_empty() && !target.starts_with('-'),
                "invalid recovery timestamp"
            );
        }
        let exists = self.docker(&["volume", "inspect", &volume]).output()?;
        ensure!(
            !exists.status.success(),
            "recovery volume already exists; select a new label"
        );
        crate::run(&mut self.docker(&["volume", "create", &volume]))?;
        let mut command = self.compose(&[
            "--profile",
            "recovery",
            "run",
            "--rm",
            "--no-deps",
            "restore",
        ]);
        command.env("DATA_RESTORE_VOLUME", &volume);
        if let Some(target) = target {
            command.args([
                "--type=time",
                &format!("--target={target}"),
                "--target-action=promote",
            ]);
        }
        crate::run(&mut command)?;
        self.validate_recovery(&volume)?;
        println!(
            "Restored and replayed WAL in isolated volume {volume}. The running database was not changed. Check application data before cutover."
        );
        Ok(())
    }

    fn validate_recovery(&self, volume: &str) -> Result<()> {
        let result = (|| -> Result<()> {
            crate::run(
                self.compose(&[
                    "--profile",
                    "recovery",
                    "up",
                    "--detach",
                    "--no-deps",
                    "recovery-check",
                ])
                .env("DATA_RESTORE_VOLUME", volume),
            )?;
            let seconds = self
                .values
                .get("RESTORE_WAIT_SECONDS")
                .map(|v| v.parse::<u64>())
                .transpose()?
                .unwrap_or(3600);
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
            loop {
                let output = self
                    .compose(&[
                        "exec",
                        "-T",
                        "--user",
                        "postgres",
                        "recovery-check",
                        "psql",
                        "-X",
                        "-h",
                        "/tmp",
                        "-U",
                        "postgres",
                        "-d",
                        "postgres",
                        "-Atc",
                        "SELECT NOT pg_is_in_recovery()",
                    ])
                    .env("DATA_RESTORE_VOLUME", volume)
                    .output()?;
                if output.status.success() && output.stdout == b"t\n" {
                    break;
                }
                ensure!(
                    std::time::Instant::now() < deadline,
                    "isolated recovery did not complete; inspect the retained volume and backup target"
                );
                std::thread::sleep(std::time::Duration::from_secs(1));
            }
            Ok(())
        })();
        let stopped = self
            .compose(&[
                "--profile",
                "recovery",
                "rm",
                "--stop",
                "--force",
                "recovery-check",
            ])
            .env("DATA_RESTORE_VOLUME", volume)
            .status()?;
        result?;
        ensure!(
            stopped.success(),
            "stop the isolated recovery container before cutover"
        );
        Ok(())
    }

    fn initial_backup(&self) -> Result<()> {
        let output = self
            .compose(&[
                "exec",
                "-T",
                "--user",
                "postgres",
                "postgres",
                "pgbackrest",
                "--stanza=db",
                "--output=json",
                "info",
            ])
            .output()?;
        ensure!(
            output.status.success(),
            "cannot inspect initial backup history"
        );
        let stanzas: serde_json::Value = serde_json::from_slice(&output.stdout)?;
        if !stanzas[0]["backup"]
            .as_array()
            .is_some_and(|backups| !backups.is_empty())
        {
            self.pgbackrest(&["--type=full", "backup"])?;
        }
        Ok(())
    }
}

fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 100
        && !value.starts_with('-')
        && value
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_')
}

fn valid_postgres_image(image: &str) -> bool {
    if image.is_empty() || image.starts_with('-') || image.chars().any(char::is_whitespace) {
        return false;
    }
    if let Some((name, digest)) = image.split_once('@') {
        return !name.is_empty()
            && digest.strip_prefix("sha256:").is_some_and(|hash| {
                hash.len() == 64 && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
            });
    }
    image.rsplit('/').next().is_some_and(|name| {
        name.rsplit_once(':').is_some_and(|(name, tag)| {
            !name.is_empty()
                && !tag.eq_ignore_ascii_case("latest")
                && (1..=128).contains(&tag.len())
                && tag
                    .bytes()
                    .next()
                    .is_some_and(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
                && tag
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
        })
    })
}

pub fn from_arguments() -> Result<()> {
    let arguments: Vec<_> = std::env::args().skip(2).collect();
    let action = arguments.first().context("select a data action")?;
    let environment = Environment::parse(arguments.get(1).context("select dev or prod")?)?;
    let deployment = DataServicesDeployment::load(Path::new("."), environment)?;
    let extra: Vec<_> = arguments.iter().skip(2).map(String::as_str).collect();
    println!(
        "Data target: context={}, project={}, environment={}",
        deployment.context,
        deployment.project,
        environment.name()
    );
    if !matches!(
        action.as_str(),
        "backup" | "restore" | "restore-resume" | "logs"
    ) {
        ensure!(extra.is_empty(), "unexpected data arguments");
    }
    match action.as_str() {
        "check" => deployment.run(&["config", "--quiet"]),
        "up" => {
            deployment.require_secrets()?;
            deployment.run(&["pull", "postgres", "meilisearch"])?;
            deployment.run(&["stop", "backup-scheduler"])?;
            deployment.run(&[
                "up",
                "--detach",
                "--no-build",
                "--pull",
                "never",
                "--wait",
                "postgres",
                "meilisearch",
            ])?;
            deployment.pgbackrest(&["stanza-create"])?;
            deployment.pgbackrest(&["check"])?;
            deployment.initial_backup()?;
            deployment.run(&[
                "up",
                "--detach",
                "--no-build",
                "--pull",
                "never",
                "backup-scheduler",
            ])
        }
        "down" => deployment.run(&["stop", "backup-scheduler", "meilisearch", "postgres"]),
        "status" => deployment.run(&["ps", "--all"]),
        "logs" => {
            let service = extra.first().copied().unwrap_or("postgres");
            ensure!(
                extra.len() <= 1
                    && ["postgres", "meilisearch", "backup-scheduler"].contains(&service),
                "select a data service"
            );
            deployment.run(&["logs", "--follow", service])
        }
        "backup" => {
            let kind = extra.first().copied().unwrap_or("incr");
            ensure!(
                extra.len() <= 1 && ["full", "diff", "incr"].contains(&kind),
                "select full, diff, or incr"
            );
            deployment.pgbackrest(&[
                &format!("--type={kind}"),
                "--log-level-console=detail",
                "backup",
            ])
        }
        "backup-check" => deployment.pgbackrest(&["check"]),
        "backup-info" => deployment.pgbackrest(&["info"]),
        "restore" => {
            ensure!(
                (1..=2).contains(&extra.len()),
                "restore needs a new volume label and optional UTC target timestamp"
            );
            deployment.restore(extra[0], extra.get(1).copied())
        }
        "restore-resume" => {
            ensure!(
                extra.len() == 1,
                "restore-resume needs a retained recovery label"
            );
            let volume = deployment.recovery_volume(extra[0])?;
            ensure!(
                deployment
                    .docker(&["volume", "inspect", &volume])
                    .output()?
                    .status
                    .success(),
                "recovery volume does not exist"
            );
            deployment.validate_recovery(&volume)
        }
        _ => bail!("unknown data action"),
    }
}
