use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::process::Command;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "AxumKit development environment management tool")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Full development setup (docker + migrate)
    Dev,
    /// Start Docker services
    DockerUp,
    /// Stop and remove Docker services
    DockerDown,
    /// Check Docker service status
    DockerStatus,
    /// Run database migrations
    Migrate,
    /// Fresh migration (drop and recreate)
    MigrateFresh,
}

struct DockerService {
    name: &'static str,
    image: &'static str,
    ports: &'static [&'static str],
    env: &'static [&'static str],
    volumes: &'static [&'static str],
    extra_args: &'static [&'static str],
}

const SERVICES: &[DockerService] = &[
    DockerService {
        name: "redis-session",
        image: "redis:8-alpine",
        ports: &["6379:6379"],
        env: &[],
        volumes: &["redis-session-data:/data"],
        extra_args: &[
            "redis-server",
            "--appendonly",
            "yes",
            "--maxmemory",
            "512mb",
            "--maxmemory-policy",
            "volatile-ttl",
        ],
    },
    DockerService {
        name: "redis-cache",
        image: "redis:8-alpine",
        ports: &["6380:6379"],
        env: &[],
        volumes: &["redis-cache-data:/data"],
        extra_args: &[
            "redis-server",
            "--maxmemory",
            "1gb",
            "--maxmemory-policy",
            "allkeys-lru",
        ],
    },
    DockerService {
        name: "nats",
        image: "nats:2.12.3-alpine",
        ports: &["4222:4222", "8222:8222"],
        env: &[],
        volumes: &["nats-data:/data"],
        extra_args: &["--jetstream", "--store_dir=/data", "-m", "8222"],
    },
    DockerService {
        name: "meilisearch",
        image: "getmeili/meilisearch:v1.34.3",
        ports: &["7700:7700"],
        env: &[],
        volumes: &["meili_data:/meili_data"],
        extra_args: &[],
    },
    DockerService {
        name: "seaweedfs",
        image: "chrislusf/seaweedfs:4.04",
        ports: &["8333:8333", "9333:9333", "8888:8888"],
        env: &[],
        volumes: &["seaweedfs-data:/data"],
        extra_args: &["server", "-filer", "-s3"],
    },
];

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Dev => dev()?,
        Commands::DockerUp => docker_up()?,
        Commands::DockerDown => docker_down()?,
        Commands::DockerStatus => docker_status()?,
        Commands::Migrate => migrate()?,
        Commands::MigrateFresh => migrate_fresh()?,
    }

    Ok(())
}

fn dev() -> Result<()> {
    println!("Setting up development environment...\n");

    // 1. Start Docker services
    docker_up()?;

    // 2. Run migrations
    println!("\n--- Running migrations ---\n");
    migrate()?;

    println!("\n=== Development environment ready! ===");
    println!("\nStart the server:");
    println!("  cargo run -p axumkit-server");
    println!("\nStart the worker (in a separate terminal):");
    println!("  cargo run -p axumkit-worker");

    Ok(())
}

fn docker_up() -> Result<()> {
    println!("Starting Docker services...\n");

    for service in SERVICES {
        start_service(service)?;
    }

    println!("\nAll Docker services started!");

    Ok(())
}

fn start_service(service: &DockerService) -> Result<()> {
    // Check if already running
    let check = Command::new("docker")
        .args(["ps", "-q", "-f", &format!("name={}", service.name)])
        .output()
        .context("Failed to check docker status")?;

    if !check.stdout.is_empty() {
        println!("[{}] Already running", service.name);
        return Ok(());
    }

    // Check if container exists but stopped
    let check_stopped = Command::new("docker")
        .args(["ps", "-aq", "-f", &format!("name={}", service.name)])
        .output()
        .context("Failed to check docker status")?;

    if !check_stopped.stdout.is_empty() {
        println!("[{}] Starting existing container...", service.name);
        let status = Command::new("docker")
            .args(["start", service.name])
            .status()
            .context("Failed to start container")?;

        if status.success() {
            println!("[{}] Started", service.name);
        } else {
            println!("[{}] Failed to start", service.name);
        }
        return Ok(());
    }

    // Create new container
    println!("[{}] Creating container...", service.name);

    let mut args = vec!["run", "-d", "--name", service.name];

    for port in service.ports {
        args.push("-p");
        args.push(port);
    }

    for env in service.env {
        args.push("-e");
        args.push(env);
    }

    for volume in service.volumes {
        args.push("-v");
        args.push(volume);
    }

    args.push(service.image);
    args.extend(service.extra_args);

    let status = Command::new("docker")
        .args(&args)
        .status()
        .context(format!("Failed to create {}", service.name))?;

    if status.success() {
        println!("[{}] Created and started", service.name);
    } else {
        println!("[{}] Failed to create", service.name);
    }

    Ok(())
}

fn docker_down() -> Result<()> {
    println!("Stopping all Docker services...\n");

    let names: Vec<&str> = SERVICES.iter().map(|s| s.name).collect();

    // Stop containers
    let _ = Command::new("docker")
        .arg("stop")
        .args(&names)
        .status();

    // Remove containers
    let _ = Command::new("docker")
        .arg("rm")
        .args(&names)
        .status();

    println!("\nAll services stopped and removed.");
    println!("Note: Volumes are preserved. Use 'docker volume rm <name>' to delete data.");

    Ok(())
}

fn docker_status() -> Result<()> {
    println!("Docker Service Status:\n");

    for service in SERVICES {
        let output = Command::new("docker")
            .args(["ps", "-a", "--format", "{{.Status}}", "-f", &format!("name=^{}$", service.name)])
            .output()
            .context("Failed to check docker status")?;

        let status_str = String::from_utf8_lossy(&output.stdout);
        let status_str = status_str.trim();

        if status_str.is_empty() {
            println!("  {:15} Not created", service.name);
        } else {
            println!("  {:15} {}", service.name, status_str);
        }
    }

    Ok(())
}

fn migrate() -> Result<()> {
    println!("Running migrations...\n");

    let status = Command::new("cargo")
        .args(["run", "-p", "migration"])
        .status()
        .context("Failed to run migration")?;

    if status.success() {
        println!("\nMigration completed!");
    } else {
        println!("\nMigration failed!");
    }

    Ok(())
}

fn migrate_fresh() -> Result<()> {
    println!("Running fresh migration (drop and recreate)...\n");

    let status = Command::new("cargo")
        .args(["run", "-p", "migration", "fresh"])
        .status()
        .context("Failed to run migration")?;

    if status.success() {
        println!("\nFresh migration completed!");
    } else {
        println!("\nFresh migration failed!");
    }

    Ok(())
}
