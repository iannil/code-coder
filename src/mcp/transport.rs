/// ─── MCP Transport ─────────────────────────────────────────────────────────
///
/// Transport layer for MCP server communication.
/// Supports stdio-based transport (spawn a process, communicate via stdin/stdout).

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, Command, Stdio};

/// A transport for communicating with an MCP server process.
pub struct McpTransport {
    process: Child,
    stdin: ChildStdin,
    reader: BufReader<std::process::ChildStdout>,
    buffer: String,
}

impl McpTransport {
    /// Spawn an MCP server process (stdio transport).
    pub fn spawn(command: &str, args: &[String], env: &[(String, String)]) -> anyhow::Result<Self> {
        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit()); // let stderr show in terminal

        // Set environment variables
        for (k, v) in env {
            cmd.env(k, v);
        }

        let mut process = cmd.spawn()
            .map_err(|e| anyhow::anyhow!("Failed to spawn MCP server '{}': {e}", command))?;

        let stdin = process.stdin.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stdin for MCP server"))?;
        let stdout = process.stdout.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to open stdout for MCP server"))?;

        Ok(Self {
            process,
            stdin,
            reader: BufReader::new(stdout),
            buffer: String::new(),
        })
    }

    /// Send a raw JSON-RPC message and read the response.
    /// Reads line-by-line until it finds a matching response.
    pub fn send_request(&mut self, request: &str) -> anyhow::Result<String> {
        // Write request
        writeln!(self.stdin, "{}", request)
            .map_err(|e| anyhow::anyhow!("Failed to write to MCP server stdin: {e}"))?;
        self.stdin.flush()?;

        // Read response (MCP uses newline-delimited JSON)
        self.buffer.clear();
        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => anyhow::bail!("MCP server closed connection"),
                Ok(_) => {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        return Ok(trimmed.to_string());
                    }
                }
                Err(e) => anyhow::bail!("Failed to read from MCP server stdout: {e}"),
            }
        }
    }

    /// Check if the server process is still running.
    pub fn is_running(&mut self) -> bool {
        match self.process.try_wait() {
            Ok(Some(_)) => false, // exited
            _ => true,            // still running or error
        }
    }

    /// Shutdown the server process.
    pub fn shutdown(&mut self) -> anyhow::Result<()> {
        let _ = self.process.kill();
        self.process.wait()?;
        Ok(())
    }
}

impl Drop for McpTransport {
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}
