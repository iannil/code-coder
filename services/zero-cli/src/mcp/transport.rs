//! MCP Transport Layer
//!
//! Provides transport implementations for MCP communication:
//! - `StdioTransport`: For local subprocess communication
//! - `HttpTransport`: For remote HTTP-based MCP servers

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use std::collections::HashMap;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;

use super::types::{JsonRpcRequest, JsonRpcResponse};

// ══════════════════════════════════════════════════════════════════════════════
// Transport Trait
// ══════════════════════════════════════════════════════════════════════════════

/// Transport trait for MCP communication
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send a JSON-RPC request and wait for response
    async fn send(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse>;

    /// Send a notification (no response expected)
    async fn notify(&self, request: &JsonRpcRequest) -> Result<()>;

    /// Check if the transport is connected/healthy
    async fn is_alive(&self) -> bool;

    /// Close the transport gracefully
    async fn close(&self) -> Result<()>;
}

// ══════════════════════════════════════════════════════════════════════════════
// Stdio Transport
// ══════════════════════════════════════════════════════════════════════════════

/// Transport for local MCP servers via stdio
pub struct StdioTransport {
    process: Mutex<Child>,
    stdin: Mutex<tokio::process::ChildStdin>,
    stdout: Mutex<BufReader<tokio::process::ChildStdout>>,
}

impl StdioTransport {
    /// Spawn a new MCP server process
    #[allow(clippy::unused_async)] // Kept async for transport trait consistency
    pub async fn spawn(
        command: &[String],
        environment: Option<&HashMap<String, String>>,
    ) -> Result<Self> {
        if command.is_empty() {
            bail!("MCP server command cannot be empty");
        }

        let mut cmd = Command::new(&command[0]);
        cmd.args(&command[1..])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        // Add environment variables
        if let Some(env) = environment {
            for (key, value) in env {
                cmd.env(key, value);
            }
        }

        let mut process = cmd
            .spawn()
            .with_context(|| format!("Failed to spawn MCP server: {}", command[0]))?;

        let stdin = process
            .stdin
            .take()
            .context("Failed to capture MCP server stdin")?;
        let stdout = process
            .stdout
            .take()
            .context("Failed to capture MCP server stdout")?;

        tracing::info!("MCP server process started: {}", command[0]);

        Ok(Self {
            process: Mutex::new(process),
            stdin: Mutex::new(stdin),
            stdout: Mutex::new(BufReader::new(stdout)),
        })
    }

    /// Read a single JSON-RPC message from stdout
    async fn read_message(&self) -> Result<JsonRpcResponse> {
        let mut stdout = self.stdout.lock().await;
        let mut line = String::new();

        // Read lines until we get a non-empty JSON line
        loop {
            line.clear();
            let bytes = stdout
                .read_line(&mut line)
                .await
                .context("Failed to read from MCP server")?;

            if bytes == 0 {
                bail!("MCP server closed connection");
            }

            let trimmed = line.trim();
            if !trimmed.is_empty() && trimmed.starts_with('{') {
                break;
            }
        }

        serde_json::from_str(line.trim()).context("Failed to parse MCP server response")
    }

    /// Write a JSON-RPC message to stdin
    async fn write_message(&self, request: &JsonRpcRequest) -> Result<()> {
        let mut stdin = self.stdin.lock().await;
        let json = serde_json::to_string(request)?;

        stdin
            .write_all(json.as_bytes())
            .await
            .context("Failed to write to MCP server")?;
        stdin
            .write_all(b"\n")
            .await
            .context("Failed to write newline to MCP server")?;
        stdin
            .flush()
            .await
            .context("Failed to flush MCP server stdin")?;

        Ok(())
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
        self.write_message(request).await?;
        self.read_message().await
    }

    async fn notify(&self, request: &JsonRpcRequest) -> Result<()> {
        self.write_message(request).await
    }

    async fn is_alive(&self) -> bool {
        let mut process = self.process.lock().await;
        matches!(process.try_wait(), Ok(None))
    }

    async fn close(&self) -> Result<()> {
        let mut process = self.process.lock().await;

        // Try graceful shutdown first
        if let Err(e) = process.kill().await {
            tracing::warn!("Failed to kill MCP server process: {e}");
        }

        process.wait().await.context("Failed to wait for MCP server to exit")?;
        tracing::info!("MCP server process terminated");

        Ok(())
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// HTTP Transport
// ══════════════════════════════════════════════════════════════════════════════

/// Transport for remote MCP servers via HTTP
pub struct HttpTransport {
    client: reqwest::Client,
    url: String,
    headers: HashMap<String, String>,
}

impl HttpTransport {
    /// Create a new HTTP transport
    pub fn new(url: impl Into<String>, headers: Option<HashMap<String, String>>) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: url.into(),
            headers: headers.unwrap_or_default(),
        }
    }

    /// Build request with configured headers
    fn build_request(&self, request: &JsonRpcRequest) -> Result<reqwest::RequestBuilder> {
        let mut builder = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json");

        for (key, value) in &self.headers {
            builder = builder.header(key, value);
        }

        Ok(builder.json(request))
    }
}

#[async_trait]
impl Transport for HttpTransport {
    async fn send(&self, request: &JsonRpcRequest) -> Result<JsonRpcResponse> {
        let req = self.build_request(request)?;

        let response = req
            .send()
            .await
            .context("Failed to send HTTP request to MCP server")?;

        if !response.status().is_success() {
            bail!(
                "MCP server returned error status: {} {}",
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown")
            );
        }

        response
            .json()
            .await
            .context("Failed to parse MCP server response")
    }

    async fn notify(&self, request: &JsonRpcRequest) -> Result<()> {
        let req = self.build_request(request)?;

        req.send()
            .await
            .context("Failed to send HTTP notification to MCP server")?;

        Ok(())
    }

    async fn is_alive(&self) -> bool {
        // Send a ping to check if server is alive
        let ping = JsonRpcRequest::new(0i64, "ping");
        self.send(&ping).await.is_ok()
    }

    async fn close(&self) -> Result<()> {
        // HTTP transport doesn't need explicit close
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_transport_new() {
        let transport = HttpTransport::new("http://localhost:8080", None);
        assert_eq!(transport.url, "http://localhost:8080");
        assert!(transport.headers.is_empty());
    }

    #[test]
    fn http_transport_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("Authorization".into(), "Bearer token".into());

        let transport = HttpTransport::new("http://localhost:8080", Some(headers));
        assert_eq!(transport.headers.len(), 1);
        assert_eq!(transport.headers.get("Authorization").unwrap(), "Bearer token");
    }

    #[tokio::test]
    async fn stdio_transport_empty_command_fails() {
        let result = StdioTransport::spawn(&[], None).await;
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.to_string().contains("cannot be empty"));
    }

    #[tokio::test]
    async fn stdio_transport_invalid_command_fails() {
        let result = StdioTransport::spawn(&["nonexistent_command_12345".into()], None).await;
        assert!(result.is_err());
    }
}
