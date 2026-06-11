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
    /// Export merged OpenAPI schema to swagger.json
    Openapi,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum PublishTarget {
    All,
    Server,
    Worker,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::DockerPublish {
            tag,
            latest,
            target,
        } => docker_publish(&tag, latest, target)?,
        Commands::Openapi => openapi()?,
    }

    Ok(())
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

fn openapi() -> Result<()> {
    println!("Exporting OpenAPI schema to swagger.json...\n");

    let json = serde_json::to_string_pretty(&ApiDoc::merged())
        .context("Failed to serialize OpenAPI schema")?;
    fs::write("swagger.json", format!("{json}\n")).context("Failed to write swagger.json")?;

    println!("OpenAPI schema exported to swagger.json");
    Ok(())
}
