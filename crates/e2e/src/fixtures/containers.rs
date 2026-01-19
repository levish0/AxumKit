//! Docker container management for E2E tests using docker-compose
//!
//! All containers are managed via docker-compose.e2e.yml

use anyhow::Result;
use std::process::Stdio;
use tokio::process::Command;

const COMPOSE_FILE: &str = "docker-compose.e2e.yml";

/// All E2E test containers managed via docker-compose
pub struct E2eContainers {
    project_root: std::path::PathBuf,
    cleaned_up: bool,
    /// Server base URL (with dynamically allocated port)
    pub base_url: String,
    /// Dynamically allocated PostgreSQL host port
    pub postgres_port: u16,
}

impl Drop for E2eContainers {
    fn drop(&mut self) {
        if self.cleaned_up {
            return;
        }
        tracing::info!("Cleaning up E2E containers...");
        let _ = std::process::Command::new("docker")
            .args(["compose", "-f", COMPOSE_FILE, "down", "-v"])
            .current_dir(&self.project_root)
            .output();
    }
}

impl E2eContainers {
    /// Start all containers using docker-compose (builds server/worker if needed)
    pub async fn start() -> Result<Self> {
        let project_root = find_project_root()?;

        tracing::info!("Cleaning up any existing E2E containers...");

        // Clean up any existing containers/volumes first
        let _ = Command::new("docker")
            .args([
                "compose",
                "-f",
                COMPOSE_FILE,
                "down",
                "-v",
                "--remove-orphans",
            ])
            .current_dir(&project_root)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        tracing::info!("Starting E2E containers with docker-compose...");

        // Start all containers, wait for health checks
        // Note: If images don't exist, docker compose will build them automatically
        // Subsequent runs will reuse the cached images (no rebuild)
        let status = Command::new("docker")
            .args(["compose", "-f", COMPOSE_FILE, "up", "-d", "--wait"])
            .current_dir(&project_root)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to start E2E containers"));
        }

        // Get dynamically allocated ports
        let postgres_port = get_container_port("e2e-postgres", 5432).await?;
        let server_port = get_container_port("e2e-server", 8000).await?;
        let base_url = format!("http://localhost:{}", server_port);

        tracing::info!("PostgreSQL available on host port {}", postgres_port);
        tracing::info!("Server available at {}", base_url);
        tracing::info!("All E2E containers are healthy");

        Ok(Self {
            project_root,
            cleaned_up: false,
            base_url,
            postgres_port,
        })
    }

    /// Run database migration
    pub async fn run_migration(&self) -> Result<()> {
        tracing::info!("Running database migration...");
        let output = Command::new("docker")
            .args(["exec", "e2e-server", "./migration"])
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .output()
            .await?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Migration failed"));
        }
        tracing::info!("Migration completed");
        Ok(())
    }

    /// Get server logs (last N lines)
    pub async fn get_server_logs(&self, lines: u32) -> Result<String> {
        let output = Command::new("docker")
            .args(["logs", "--tail", &lines.to_string(), "e2e-server"])
            .output()
            .await?;

        let logs = if output.stdout.is_empty() {
            String::from_utf8_lossy(&output.stderr).to_string()
        } else {
            String::from_utf8_lossy(&output.stdout).to_string()
        };

        Ok(logs)
    }

    /// Cleanup all containers
    pub async fn cleanup(&mut self) -> Result<()> {
        tracing::info!("Stopping E2E containers...");

        let status = Command::new("docker")
            .args(["compose", "-f", COMPOSE_FILE, "down", "-v"])
            .current_dir(&self.project_root)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            tracing::warn!("Failed to stop some containers");
        }

        self.cleaned_up = true;
        Ok(())
    }
}

/// Find project root (where docker-compose.e2e.yml is located)
fn find_project_root() -> Result<std::path::PathBuf> {
    let mut current = std::env::current_dir()?;

    loop {
        if current.join(COMPOSE_FILE).exists() {
            return Ok(current);
        }

        if !current.pop() {
            return Err(anyhow::anyhow!(
                "Could not find {} in any parent directory",
                COMPOSE_FILE
            ));
        }
    }
}

/// Get dynamically allocated host port for a container
async fn get_container_port(container_name: &str, container_port: u16) -> Result<u16> {
    for i in 0..10 {
        let output = Command::new("docker")
            .args(["port", container_name, &container_port.to_string()])
            .output()
            .await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(port_str) = stdout.trim().split(':').last() {
                if let Ok(port) = port_str.parse::<u16>() {
                    return Ok(port);
                }
            }
        }

        if i < 9 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    Err(anyhow::anyhow!(
        "Failed to get port mapping for {}:{}",
        container_name,
        container_port
    ))
}
