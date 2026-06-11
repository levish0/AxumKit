use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use server::api::openapi::ApiDoc;
use std::fs;
use std::process::Command;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "AxumKit development helper commands")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Set up the development environment (docker + migrate)
    Dev,
    /// Start development Docker infrastructure services
    DockerUp,
    /// Stop and remove development Docker services (volumes are preserved)
    DockerDown,
    /// Show development Docker service status
    DockerStatus,
    /// Build and push GHCR Docker images
    DockerPublish {
        /// Version tag to publish, for example 0.8.0
        #[arg(long)]
        tag: String,
        /// Also publish the latest tag
        #[arg(long)]
        latest: bool,
        /// Image target to publish
        #[arg(long, value_enum, default_value_t = PublishTarget::All)]
        target: PublishTarget,
    },
    /// Run database migrations
    Migrate,
    /// Drop everything and re-run all migrations
    MigrateFresh,
    /// Export merged OpenAPI schema to swagger.json
    Openapi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum PublishTarget {
    All,
    Server,
    Worker,
}

const COMPOSE_FILE: &str = "docker-compose.dev.yml";
const INFRA_SERVICES: &[&str] = &[
    "postgres",
    "pgdog",
    "redis-session",
    "redis-cache",
    "redis-lock",
    "nats",
    "meilisearch",
];

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dev => dev()?,
        Commands::DockerUp => docker_up()?,
        Commands::DockerDown => docker_down()?,
        Commands::DockerStatus => docker_status()?,
        Commands::DockerPublish {
            tag,
            latest,
            target,
        } => docker_publish(&tag, latest, target)?,
        Commands::Migrate => migrate()?,
        Commands::MigrateFresh => migrate_fresh()?,
        Commands::Openapi => openapi()?,
    }

    Ok(())
}

fn dev() -> Result<()> {
    println!("Setting up development environment...\n");
    docker_up()?;

    println!("\n--- Running migrations ---\n");
    migrate()?;

    println!("\n=== Development environment ready! ===");
    println!("\nStart the server:");
    println!("  cargo run -p server");
    println!("\nStart the worker in another terminal:");
    println!("  cargo run -p worker");

    Ok(())
}

fn compose(args: &[&str]) -> Result<()> {
    let status = Command::new("docker")
        .args(["compose", "-f", COMPOSE_FILE])
        .args(args)
        .status()
        .context("Failed to run docker compose")?;

    if !status.success() {
        anyhow::bail!("docker compose {} failed", args.join(" "));
    }

    Ok(())
}

fn docker_up() -> Result<()> {
    println!("Starting Docker infrastructure services...\n");

    let mut args = vec!["up", "-d"];
    args.extend(INFRA_SERVICES);
    compose(&args)?;

    println!("\nAll Docker infrastructure services started.");
    Ok(())
}

fn docker_down() -> Result<()> {
    println!("Stopping Docker services...\n");
    compose(&["down"])?;
    println!("\nServices stopped. Volumes are preserved.");
    Ok(())
}

fn docker_status() -> Result<()> {
    compose(&["ps", "-a"])
}

fn docker_publish(tag: &str, latest: bool, target: PublishTarget) -> Result<()> {
    if tag.trim().is_empty() {
        anyhow::bail!("--tag must not be empty");
    }

    match target {
        PublishTarget::All => {
            publish_docker_image(
                "server",
                "server-runtime",
                "ghcr.io/levish0/axumkit",
                tag,
                latest,
            )?;
            publish_docker_image(
                "worker",
                "worker-runtime",
                "ghcr.io/levish0/axumkit-worker",
                tag,
                latest,
            )?;
        }
        PublishTarget::Server => publish_docker_image(
            "server",
            "server-runtime",
            "ghcr.io/levish0/axumkit",
            tag,
            latest,
        )?,
        PublishTarget::Worker => publish_docker_image(
            "worker",
            "worker-runtime",
            "ghcr.io/levish0/axumkit-worker",
            tag,
            latest,
        )?,
    }

    Ok(())
}

fn publish_docker_image(
    label: &str,
    docker_target: &str,
    image: &str,
    tag: &str,
    latest: bool,
) -> Result<()> {
    println!("Publishing {label} image...");

    let version_tag = format!("{image}:{tag}");
    let latest_tag = format!("{image}:latest");

    let mut args = vec![
        "buildx",
        "build",
        "--platform",
        "linux/amd64",
        "--file",
        "Dockerfile",
        "--target",
        docker_target,
        "--tag",
        &version_tag,
    ];

    if latest {
        args.push("--tag");
        args.push(&latest_tag);
    }

    args.push("--push");
    args.push(".");

    let status = Command::new("docker")
        .args(args)
        .status()
        .with_context(|| format!("Failed to build and push {label} image"))?;

    if !status.success() {
        anyhow::bail!("Failed to publish {label} image");
    }

    println!("Published {version_tag}");
    if latest {
        println!("Published {latest_tag}");
    }

    Ok(())
}

fn migrate() -> Result<()> {
    let status = Command::new("cargo")
        .args(["run", "-p", "migration"])
        .status()
        .context("Failed to run migration")?;

    if !status.success() {
        anyhow::bail!("Migration failed");
    }

    Ok(())
}

fn migrate_fresh() -> Result<()> {
    let status = Command::new("cargo")
        .args(["run", "-p", "migration", "fresh"])
        .status()
        .context("Failed to run fresh migration")?;

    if !status.success() {
        anyhow::bail!("Fresh migration failed");
    }

    Ok(())
}

fn openapi() -> Result<()> {
    println!("Exporting OpenAPI schema to swagger.json...\n");

    let json = serde_json::to_string_pretty(&ApiDoc::merged())
        .context("Failed to serialize OpenAPI schema")?;
    fs::write("swagger.json", format!("{json}\n")).context("Failed to write swagger.json")?;

    println!("OpenAPI schema exported to swagger.json");
    Ok(())
}
