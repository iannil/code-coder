//! Docker-based sandbox implementation using bollard.
//!
//! Provides secure, isolated code execution in Docker containers with:
//! - Network isolation (disabled by default)
//! - Memory and CPU limits
//! - Automatic container cleanup
//! - Timeout enforcement

use crate::sandbox::types::{ExecutionResult, Language, SandboxConfig};
use anyhow::{Context, Result};
use bollard::container::{
    Config, CreateContainerOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    StartContainerOptions, WaitContainerOptions,
};
use bollard::exec::{CreateExecOptions, StartExecResults};
use bollard::image::CreateImageOptions;
use bollard::Docker;
use futures_util::StreamExt;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Docker-based sandbox for secure code execution.
pub struct DockerSandbox {
    client: Docker,
    config: SandboxConfig,
}

impl DockerSandbox {
    /// Create a new DockerSandbox with default configuration.
    pub async fn new() -> Result<Self> {
        Self::with_config(SandboxConfig::default()).await
    }

    /// Create a new DockerSandbox with custom configuration.
    pub async fn with_config(config: SandboxConfig) -> Result<Self> {
        let client =
            Docker::connect_with_local_defaults().context("Failed to connect to Docker daemon")?;

        Ok(Self { client, config })
    }

    /// Check if Docker is available and responsive.
    pub async fn health_check(&self) -> bool {
        self.client.ping().await.is_ok()
    }

    /// Execute code in a sandboxed container.
    pub async fn execute(&self, code: &str, language: Language) -> Result<ExecutionResult> {
        self.execute_with_timeout(code, language, self.config.timeout)
            .await
    }

    /// Execute code with a custom timeout.
    pub async fn execute_with_timeout(
        &self,
        code: &str,
        language: Language,
        exec_timeout: Duration,
    ) -> Result<ExecutionResult> {
        let start = Instant::now();
        let image = self
            .config
            .image
            .as_deref()
            .unwrap_or_else(|| language.default_image());

        // Pull image if needed
        self.pull_image_if_needed(image).await?;

        // Create unique container name
        let container_name = format!(
            "sandbox-{}-{}",
            language,
            uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
        );

        // Configure container
        let host_config = bollard::service::HostConfig {
            memory: Some(self.config.memory_limit as i64),
            nano_cpus: Some((self.config.cpu_quota * 1_000_000_000.0) as i64),
            network_mode: if self.config.network_enabled {
                Some("bridge".to_string())
            } else {
                Some("none".to_string())
            },
            auto_remove: Some(false), // We'll remove manually after collecting logs
            ..Default::default()
        };

        let filename = format!("/tmp/code.{}", language.file_extension());
        let cmd = language.run_command(&filename);

        let container_config = Config {
            image: Some(image.to_string()),
            cmd: Some(cmd.clone()),
            working_dir: Some(self.config.workdir.clone()),
            host_config: Some(host_config),
            tty: Some(false),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        // Create container
        let container = self
            .client
            .create_container(
                Some(CreateContainerOptions {
                    name: &container_name,
                    platform: None,
                }),
                container_config,
            )
            .await
            .context("Failed to create container")?;

        let container_id = container.id;

        // Copy code to container
        self.copy_code_to_container(&container_id, code, &filename)
            .await?;

        // Start container
        self.client
            .start_container(&container_id, None::<StartContainerOptions<String>>)
            .await
            .context("Failed to start container")?;

        // Wait for container with timeout
        let result = timeout(exec_timeout, self.wait_for_container(&container_id)).await;

        let (exit_code, timed_out) = match result {
            Ok(Ok(code)) => (code, false),
            Ok(Err(e)) => {
                // Clean up on error
                let _ = self.kill_container(&container_id).await;
                let _ = self.remove_container(&container_id).await;
                return Err(e);
            }
            Err(_) => {
                // Timeout - kill container
                let _ = self.kill_container(&container_id).await;
                (137, true) // SIGKILL exit code
            }
        };

        // Collect logs
        let (stdout, stderr) = self.collect_logs(&container_id).await?;

        // Remove container
        self.remove_container(&container_id).await?;

        let duration = start.elapsed();

        Ok(ExecutionResult {
            exit_code,
            stdout,
            stderr,
            duration,
            timed_out,
        })
    }

    /// Pull Docker image if not present locally.
    async fn pull_image_if_needed(&self, image: &str) -> Result<()> {
        // Check if image exists
        if self.client.inspect_image(image).await.is_ok() {
            return Ok(());
        }

        // Parse image name and tag
        let (name, tag) = image.split_once(':').unwrap_or((image, "latest"));

        // Pull image
        let options = CreateImageOptions {
            from_image: name,
            tag,
            ..Default::default()
        };

        let mut stream = self.client.create_image(Some(options), None, None);

        while let Some(result) = stream.next().await {
            result.context("Failed to pull image")?;
        }

        Ok(())
    }

    /// Copy code to container using exec.
    async fn copy_code_to_container(
        &self,
        container_id: &str,
        code: &str,
        filename: &str,
    ) -> Result<()> {
        // Use base64 encoding to safely pass code through shell
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, code);
        let command = format!("echo '{}' | base64 -d > {}", encoded, filename);

        let create_options = CreateExecOptions {
            cmd: Some(vec![
                "sh",
                "-c",
                &command,
            ]),
            attach_stdout: Some(true),
            attach_stderr: Some(true),
            ..Default::default()
        };

        let exec = self
            .client
            .create_exec(container_id, create_options)
            .await
            .context("Failed to create exec for code copy")?;

        let start_result = self
            .client
            .start_exec(&exec.id, None)
            .await
            .context("Failed to execute code copy")?;

        // Wait for exec to complete
        if let StartExecResults::Attached { mut output, .. } = start_result {
            while let Some(_) = output.next().await {}
        }

        Ok(())
    }

    /// Wait for container to finish and return exit code.
    async fn wait_for_container(&self, container_id: &str) -> Result<i32> {
        let options = WaitContainerOptions {
            condition: "not-running",
        };

        let mut stream = self.client.wait_container(container_id, Some(options));

        while let Some(result) = stream.next().await {
            let response = result.context("Error waiting for container")?;
            return Ok(response.status_code as i32);
        }

        // Container finished before we could read status
        let info = self
            .client
            .inspect_container(container_id, None)
            .await
            .context("Failed to inspect container")?;

        Ok(info
            .state
            .and_then(|s| s.exit_code)
            .map(|c| c as i32)
            .unwrap_or(-1))
    }

    /// Kill a running container.
    async fn kill_container(&self, container_id: &str) -> Result<()> {
        self.client
            .kill_container::<String>(container_id, None)
            .await
            .context("Failed to kill container")?;
        Ok(())
    }

    /// Remove a container.
    async fn remove_container(&self, container_id: &str) -> Result<()> {
        let options = RemoveContainerOptions {
            force: true,
            ..Default::default()
        };

        self.client
            .remove_container(container_id, Some(options))
            .await
            .context("Failed to remove container")?;

        Ok(())
    }

    /// Collect stdout and stderr from container.
    async fn collect_logs(&self, container_id: &str) -> Result<(String, String)> {
        let options = LogsOptions::<String> {
            stdout: true,
            stderr: true,
            follow: false,
            ..Default::default()
        };

        let mut stdout = String::new();
        let mut stderr = String::new();

        let mut stream = self.client.logs(container_id, Some(options));

        while let Some(result) = stream.next().await {
            match result {
                Ok(LogOutput::StdOut { message }) => {
                    stdout.push_str(&String::from_utf8_lossy(&message));
                }
                Ok(LogOutput::StdErr { message }) => {
                    stderr.push_str(&String::from_utf8_lossy(&message));
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::warn!("Error collecting logs: {}", e);
                }
            }
        }

        Ok((stdout, stderr))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn sandbox_health_check() {
        let sandbox = DockerSandbox::new().await.unwrap();
        assert!(sandbox.health_check().await);
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn execute_python_hello_world() {
        let sandbox = DockerSandbox::new().await.unwrap();
        let result = sandbox
            .execute("print('Hello, World!')", Language::Python)
            .await
            .unwrap();

        assert!(result.success());
        assert_eq!(result.stdout.trim(), "Hello, World!");
        assert!(result.stderr.is_empty());
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn execute_python_with_error() {
        let sandbox = DockerSandbox::new().await.unwrap();
        let result = sandbox
            .execute("raise ValueError('test error')", Language::Python)
            .await
            .unwrap();

        assert!(!result.success());
        assert!(result.stderr.contains("ValueError"));
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn execute_shell_script() {
        let sandbox = DockerSandbox::new().await.unwrap();
        let result = sandbox
            .execute("echo 'shell test' && echo 'line 2'", Language::Shell)
            .await
            .unwrap();

        assert!(result.success());
        assert!(result.stdout.contains("shell test"));
        assert!(result.stdout.contains("line 2"));
    }

    #[tokio::test]
    #[ignore = "requires Docker"]
    async fn execute_with_timeout() {
        let config = SandboxConfig {
            timeout: Duration::from_secs(2),
            ..Default::default()
        };
        let sandbox = DockerSandbox::with_config(config).await.unwrap();

        let result = sandbox
            .execute("import time; time.sleep(10)", Language::Python)
            .await
            .unwrap();

        assert!(!result.success());
        assert!(result.timed_out);
        assert!(result.duration < Duration::from_secs(5));
    }
}
