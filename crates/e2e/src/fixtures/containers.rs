//! Docker container management for E2E tests using docker-compose
//!
//! Each test run gets a unique project name for isolation.

use anyhow::Result;
use std::process::Stdio;
use std::{fs, thread, time::Duration};
use tokio::process::Command;

const COMPOSE_FILE: &str = "docker-compose.e2e.yml";
const BUILD_LOCK_FILE: &str = ".e2e-build.lock";
const BUILD_DONE_FILE: &str = ".e2e-build.done";

/// All E2E test containers managed via docker-compose
pub struct E2eContainers {
    project_root: std::path::PathBuf,
    /// Unique project name for this test run (e.g., "e2e-abc123")
    project_name: String,
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
        tracing::info!(
            "Cleaning up E2E containers (project: {})...",
            self.project_name
        );
        let output = std::process::Command::new("docker")
            .args([
                "compose",
                "-p",
                &self.project_name,
                "-f",
                COMPOSE_FILE,
                "down",
                "-v",
            ])
            .current_dir(&self.project_root)
            .output();

        if let Ok(out) = output {
            if !out.status.success() {
                tracing::warn!(
                    "Failed to tear down containers: {}",
                    String::from_utf8_lossy(&out.stderr)
                );
            }
        }
    }
}

impl E2eContainers {
    /// Start all containers using docker-compose with a unique project name
    pub async fn start() -> Result<Self> {
        let project_root = find_project_root()?;

        // Build images once across all test binaries using file-based lock
        ensure_images_built(&project_root)?;

        // Generate unique project name for this test run
        let project_name = format!("e2e-{}", uuid::Uuid::new_v4());
        tracing::info!(
            "Starting E2E test environment (project: {})...",
            project_name
        );

        // Start all containers, wait for health checks
        let status = Command::new("docker")
            .args([
                "compose",
                "-p",
                &project_name,
                "-f",
                COMPOSE_FILE,
                "up",
                "-d",
                "--no-build",
                "--wait",
            ])
            .current_dir(&project_root)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .await?;

        if !status.success() {
            return Err(anyhow::anyhow!("Failed to start E2E containers"));
        }

        // Get dynamically allocated ports using service names
        let postgres_port =
            get_service_port(&project_root, &project_name, "postgres", 5432).await?;
        let server_port = get_service_port(&project_root, &project_name, "server", 8000).await?;
        let base_url = format!("http://localhost:{}", server_port);

        tracing::info!("PostgreSQL available on host port {}", postgres_port);
        tracing::info!("Server available at {}", base_url);
        tracing::info!("All E2E containers are healthy");

        Ok(Self {
            project_root,
            project_name,
            cleaned_up: false,
            base_url,
            postgres_port,
        })
    }

    /// Run database migration
    pub async fn run_migration(&self) -> Result<()> {
        tracing::info!("Running database migration...");
        let output = Command::new("docker")
            .args([
                "compose",
                "-p",
                &self.project_name,
                "-f",
                COMPOSE_FILE,
                "exec",
                "server",
                "./migration",
            ])
            .current_dir(&self.project_root)
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
            .args([
                "compose",
                "-p",
                &self.project_name,
                "-f",
                COMPOSE_FILE,
                "logs",
                "--tail",
                &lines.to_string(),
                "server",
            ])
            .current_dir(&self.project_root)
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
        tracing::info!(
            "Stopping E2E containers (project: {})...",
            self.project_name
        );

        let status = Command::new("docker")
            .args([
                "compose",
                "-p",
                &self.project_name,
                "-f",
                COMPOSE_FILE,
                "down",
                "-v",
            ])
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

/// Check if required Docker images exist
fn docker_images_exist() -> bool {
    let check_image = |name: &str| {
        std::process::Command::new("docker")
            .args(["image", "inspect", name])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    };

    check_image("e2e-server:latest") && check_image("e2e-worker:latest")
}

/// Ensures Docker images are built exactly once across all test binaries
/// Uses file-based locking to coordinate between processes
fn ensure_images_built(project_root: &std::path::Path) -> Result<()> {
    let lock_file = project_root.join(BUILD_LOCK_FILE);
    let done_file = project_root.join(BUILD_DONE_FILE);

    // Fast path: if images already exist, skip build
    if docker_images_exist() {
        if !done_file.exists() {
            // Images exist but .done doesn't - create it for faster future checks
            fs::write(&done_file, "done").ok();
        }
        tracing::info!("E2E images already exist, skipping build");
        return Ok(());
    }

    // Remove stale done file since images don't exist
    if done_file.exists() {
        tracing::info!("Removing stale {} (images not found)", BUILD_DONE_FILE);
        fs::remove_file(&done_file).ok();
    }

    // Remove stale lock file (older than 30 minutes = likely crashed process)
    if lock_file.exists() {
        if let Ok(metadata) = fs::metadata(&lock_file) {
            if let Ok(modified) = metadata.modified() {
                if modified.elapsed().unwrap_or_default() > Duration::from_secs(30 * 60) {
                    tracing::warn!("Removing stale lock file (older than 30 minutes)");
                    fs::remove_file(&lock_file).ok();
                }
            }
        }
    }

    // Try to acquire lock by creating the lock file exclusively
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_file)
    {
        Ok(_) => {
            // We acquired the lock, do the build
            tracing::info!("Building E2E images (this only runs once across all tests)...");
            let status = std::process::Command::new("docker")
                .args(["compose", "-f", COMPOSE_FILE, "build"])
                .current_dir(project_root)
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .status();

            match status {
                Ok(s) if s.success() => {
                    tracing::info!("E2E images built successfully");
                    // Mark build as done
                    fs::write(&done_file, "done").ok();
                    // Remove lock file
                    fs::remove_file(&lock_file).ok();
                    Ok(())
                }
                Ok(s) => {
                    fs::remove_file(&lock_file).ok();
                    Err(anyhow::anyhow!(
                        "Build failed with exit code: {:?}",
                        s.code()
                    ))
                }
                Err(e) => {
                    fs::remove_file(&lock_file).ok();
                    Err(anyhow::anyhow!("Failed to run docker: {}", e))
                }
            }
        }
        Err(_) => {
            // Another process is building, wait for it (max 30 minutes)
            tracing::info!("Waiting for another process to finish building E2E images...");
            let start = std::time::Instant::now();
            let timeout = Duration::from_secs(30 * 60);

            loop {
                // Check if images are now available
                if docker_images_exist() {
                    tracing::info!("E2E images are now ready");
                    return Ok(());
                }
                if !lock_file.exists() {
                    // Lock released but images don't exist - build failed, retry
                    return ensure_images_built(project_root);
                }
                if start.elapsed() > timeout {
                    // Timeout - remove stale lock and retry
                    tracing::warn!("Timeout waiting for build, retrying...");
                    fs::remove_file(&lock_file).ok();
                    return ensure_images_built(project_root);
                }
                thread::sleep(Duration::from_secs(2));
            }
        }
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

/// Get dynamically allocated host port for a service using docker compose port
async fn get_service_port(
    project_root: &std::path::Path,
    project_name: &str,
    service_name: &str,
    container_port: u16,
) -> Result<u16> {
    for i in 0..10 {
        let output = Command::new("docker")
            .args([
                "compose",
                "-p",
                project_name,
                "-f",
                COMPOSE_FILE,
                "port",
                service_name,
                &container_port.to_string(),
            ])
            .current_dir(project_root)
            .output()
            .await?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            // Output format: "0.0.0.0:12345" or "[::]:12345"
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
        "Failed to get port mapping for service {}:{}",
        service_name,
        container_port
    ))
}
