//! Process management for the zero-cli daemon.
//!
//! This module provides utilities for spawning and managing child processes
//! for the Zero microservices (gateway, channels, workflow).

use anyhow::{Context, Result};
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use tokio::time::{Duration, interval};

/// Configuration for a managed service.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Service name (for logging and health checks)
    pub name: String,
    /// Binary name to execute
    pub binary: String,
    /// Port the service listens on
    pub port: u16,
    /// Host to bind to
    pub host: String,
    /// Additional command-line arguments
    pub args: Vec<String>,
    /// Optional log file path for stdout/stderr redirection
    pub log_file: Option<PathBuf>,
}

/// A managed child process.
pub struct ManagedProcess {
    pub config: ServiceConfig,
    pub process: Option<Child>,
    pub restart_count: u32,
    pub last_error: Option<String>,
}

impl ManagedProcess {
    /// Create a new managed process configuration.
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            config,
            process: None,
            restart_count: 0,
            last_error: None,
        }
    }

    /// Spawn the process.
    pub fn spawn(&mut self, bin_dir: &PathBuf) -> Result<()> {
        let binary_path = bin_dir.join(&self.config.binary);

        if !binary_path.exists() {
            anyhow::bail!(
                "Binary not found: {}. Run 'cargo build --release' in services/",
                binary_path.display()
            );
        }

        let mut cmd = Command::new(&binary_path);

        // Add default arguments for port and host
        cmd.arg("--port")
            .arg(self.config.port.to_string())
            .arg("--host")
            .arg(&self.config.host);

        // Add any additional arguments
        for arg in &self.config.args {
            cmd.arg(arg);
        }

        // Set up stdio - redirect to log file if configured, otherwise inherit
        if let Some(log_path) = &self.config.log_file {
            // Ensure parent directory exists
            if let Some(parent) = log_path.parent() {
                std::fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create log directory: {}", parent.display())
                })?;
            }

            // Open log file in append mode (create if not exists)
            let log_file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
                .with_context(|| format!("Failed to open log file: {}", log_path.display()))?;

            let log_file_err = log_file
                .try_clone()
                .with_context(|| "Failed to clone log file handle")?;

            cmd.stdout(Stdio::from(log_file))
                .stderr(Stdio::from(log_file_err));

            tracing::debug!(
                "Redirecting {} output to {}",
                self.config.name,
                log_path.display()
            );
        } else {
            // No log file configured - inherit parent's stdio
            cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
        }

        let child = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn {}", self.config.name))?;

        tracing::info!(
            "Started {} (pid: {}, port: {})",
            self.config.name,
            child.id(),
            self.config.port
        );

        self.process = Some(child);
        self.last_error = None;
        Ok(())
    }

    /// Check if the process is still running.
    pub fn is_running(&mut self) -> bool {
        match &mut self.process {
            Some(child) => match child.try_wait() {
                Ok(None) => true, // Still running
                Ok(Some(status)) => {
                    self.last_error = Some(format!("Process exited with {}", status));
                    false
                }
                Err(e) => {
                    self.last_error = Some(format!("Failed to check process: {}", e));
                    false
                }
            },
            None => false,
        }
    }

    /// Stop the process gracefully.
    pub fn stop(&mut self) {
        if let Some(mut child) = self.process.take() {
            let pid = child.id();

            // Send SIGTERM first for graceful shutdown (Unix only)
            #[cfg(unix)]
            {
                // Use kill command to send SIGTERM
                let _ = std::process::Command::new("kill")
                    .args(["-TERM", &pid.to_string()])
                    .output();

                // Wait up to 3 seconds for graceful shutdown
                for _ in 0..30 {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            tracing::info!("Stopped {} (exited with {})", self.config.name, status);
                            return;
                        }
                        Ok(None) => {
                            std::thread::sleep(Duration::from_millis(100));
                        }
                        Err(e) => {
                            tracing::warn!("Error checking process status: {}", e);
                            break;
                        }
                    }
                }
            }

            // Check if still running and force kill if needed
            match child.try_wait() {
                Ok(Some(status)) => {
                    tracing::info!("Stopped {} (exited with {})", self.config.name, status);
                }
                Ok(None) => {
                    // Still running after grace period, force kill
                    tracing::warn!("{} did not exit gracefully, sending SIGKILL", self.config.name);
                    let _ = child.kill();
                    let _ = child.wait();
                    tracing::info!("Stopped {} (killed)", self.config.name);
                }
                Err(e) => {
                    tracing::warn!("Error checking process status: {}", e);
                    let _ = child.kill();
                    let _ = child.wait();
                }
            }
        }
    }

    /// Restart the process.
    pub fn restart(&mut self, bin_dir: &PathBuf) -> Result<()> {
        self.stop();
        self.restart_count += 1;
        tracing::info!(
            "Restarting {} (attempt {})",
            self.config.name,
            self.restart_count
        );
        self.spawn(bin_dir)
    }
}

/// Service manager that orchestrates multiple child processes.
pub struct ServiceManager {
    services: Vec<ManagedProcess>,
    bin_dir: PathBuf,
}

impl ServiceManager {
    /// Create a new service manager.
    pub fn new(bin_dir: PathBuf) -> Self {
        Self {
            services: vec![],
            bin_dir,
        }
    }

    /// Add a service to manage.
    pub fn add_service(&mut self, config: ServiceConfig) {
        self.services.push(ManagedProcess::new(config));
    }

    /// Start all services.
    pub fn start_all(&mut self) -> Result<()> {
        for service in &mut self.services {
            service.spawn(&self.bin_dir)?;
        }
        Ok(())
    }

    /// Stop all services.
    pub fn stop_all(&mut self) {
        for service in &mut self.services {
            service.stop();
        }
    }

    /// Check health of all services and restart any that have failed.
    pub fn health_check_and_restart(&mut self) {
        for service in &mut self.services {
            if !service.is_running() {
                tracing::warn!(
                    "{} is not running (error: {:?})",
                    service.config.name,
                    service.last_error
                );

                if let Err(e) = service.restart(&self.bin_dir) {
                    tracing::error!("Failed to restart {}: {}", service.config.name, e);
                }
            }
        }
    }

    /// Get status of all services.
    pub fn status(&mut self) -> Vec<ServiceStatus> {
        self.services
            .iter_mut()
            .map(|s| ServiceStatus {
                name: s.config.name.clone(),
                port: s.config.port,
                running: s.is_running(),
                restart_count: s.restart_count,
                last_error: s.last_error.clone(),
            })
            .collect()
    }
}

/// Status of a single service.
#[derive(Debug, Clone)]
pub struct ServiceStatus {
    pub name: String,
    pub port: u16,
    pub running: bool,
    pub restart_count: u32,
    pub last_error: Option<String>,
}

/// HTTP health check client.
pub struct HealthChecker {
    client: reqwest::Client,
}

impl HealthChecker {
    /// Create a new health checker.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    /// Check if a service is healthy via HTTP.
    pub async fn check(&self, host: &str, port: u16) -> bool {
        let url = format!("http://{}:{}/health", host, port);
        match self.client.get(&url).send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

/// Run the health check loop.
pub async fn health_check_loop(
    mut manager: ServiceManager,
    health_checker: HealthChecker,
    check_interval_secs: u64,
) {
    let mut interval = interval(Duration::from_secs(check_interval_secs));

    loop {
        interval.tick().await;

        // First check via process status
        manager.health_check_and_restart();

        // Then verify via HTTP health endpoints
        for service in manager.status() {
            if service.running {
                let healthy = health_checker.check("127.0.0.1", service.port).await;
                if !healthy {
                    tracing::warn!(
                        "{} process running but HTTP health check failed",
                        service.name
                    );
                }
            }
        }
    }
}

/// Find the Rust binary directory.
pub fn find_rust_bin_dir() -> Result<PathBuf> {
    // Try release directory first
    let release_dir = PathBuf::from("services/target/release");
    if release_dir.exists() {
        return Ok(release_dir);
    }

    // Try debug directory
    let debug_dir = PathBuf::from("services/target/debug");
    if debug_dir.exists() {
        return Ok(debug_dir);
    }

    // Try from workspace root
    let workspace_release = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|p| p.join("target/release"));
    if let Some(p) = workspace_release {
        if p.exists() {
            return Ok(p);
        }
    }

    anyhow::bail!(
        "Could not find Rust binary directory. Run 'cargo build --release' in services/"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_config_new() {
        let config = ServiceConfig {
            name: "test".into(),
            binary: "test-bin".into(),
            port: 8080,
            host: "127.0.0.1".into(),
            args: vec![],
            log_file: None,
        };

        let service = ManagedProcess::new(config.clone());
        assert!(service.process.is_none());
        assert_eq!(service.restart_count, 0);
        assert_eq!(service.config.name, "test");
    }

    #[test]
    fn service_manager_add_service() {
        let mut manager = ServiceManager::new(PathBuf::from("/tmp"));

        manager.add_service(ServiceConfig {
            name: "gateway".into(),
            binary: "zero-gateway".into(),
            port: 4430,
            host: "127.0.0.1".into(),
            args: vec![],
            log_file: None,
        });

        manager.add_service(ServiceConfig {
            name: "channels".into(),
            binary: "zero-channels".into(),
            port: 4431,
            host: "127.0.0.1".into(),
            args: vec![],
            log_file: None,
        });

        assert_eq!(manager.services.len(), 2);
    }

    #[test]
    fn health_checker_new() {
        let checker = HealthChecker::new();
        // Just verify it constructs without panic
        let _ = checker;
    }
}
