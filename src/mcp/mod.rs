/// ─── MCP Module ────────────────────────────────────────────────────────────
///
/// Model Context Protocol (MCP) support for CodeCoder.
/// Manages MCP server processes, discovers tools, and provides McpTool
/// implementations that delegate to MCP servers.

pub mod protocol;
pub mod transport;

use protocol::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use transport::McpTransport;

/// ─── McpServerConfig ──────────────────────────────────────────────────────

/// Configuration for a single MCP server.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    /// Optional env vars: array of "KEY=VALUE" strings
    #[serde(default)]
    pub env: Vec<String>,
}

/// ─── McpServerHandle ──────────────────────────────────────────────────────

/// A running MCP server instance.
struct McpServerInstance {
    config: McpServerConfig,
    transport: McpTransport,
    server_info: ServerInfo,
    tools: Vec<ToolDescription>,
}

/// ─── McpRegistry ──────────────────────────────────────────────────────────

/// Manages MCP server lifecycle and tool discovery.
pub struct McpRegistry {
    configs: Vec<McpServerConfig>,
    instances: HashMap<String, McpServerInstance>,
    next_id: u64,
}

impl McpRegistry {
    pub fn new(configs: Vec<McpServerConfig>) -> Self {
        Self {
            configs,
            instances: HashMap::new(),
            next_id: 1,
        }
    }

    /// Start all configured MCP servers and discover their tools.
    pub fn start_all(&mut self) -> Vec<String> {
        let mut results = Vec::new();
        let configs = std::mem::take(&mut self.configs);
        for config in configs {
            let name = config.name.clone();
            match self.start_server(config) {
                Ok(_) => results.push(format!("✓ {name}")),
                Err(e) => results.push(format!("✗ {name}: {e}")),
            }
        }
        results
    }

    /// Start a single MCP server by config.
    pub fn start_server(&mut self, config: McpServerConfig) -> anyhow::Result<String> {
        let name = config.name.clone();

        // Parse env vars
        let env_pairs: Vec<(String, String)> = config.env.iter()
            .filter_map(|e| {
                let parts: Vec<&str> = e.splitn(2, '=').collect();
                if parts.len() == 2 {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    None
                }
            })
            .collect();

        // Spawn transport
        let mut transport = McpTransport::spawn(&config.command, &config.args, &env_pairs)?;

        // Initialize
        let init_params = InitializeParams {
            protocol_version: "2024-11-05".into(),
            capabilities: ClientCapabilities {
                tools: Some(serde_json::json!({})),
            },
            client_info: ClientInfo {
                name: "codecoder".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        };

        let req = JsonRpcRequest::new(self.next_id(), "initialize", Some(serde_json::to_value(init_params)?));
        let resp_text = transport.send_request(&req.serialize())?;
        let resp: JsonRpcResponse = serde_json::from_str(&resp_text)
            .map_err(|e| anyhow::anyhow!("MCP init response parse error: {e} — response: {resp_text}"))?;

        let init_result = match resp {
            JsonRpcResponse::Success { result, .. } => {
                serde_json::from_value::<InitializeResult>(result)
                    .map_err(|e| anyhow::anyhow!("Invalid init result: {e}"))?
            }
            JsonRpcResponse::Error { error, .. } => {
                anyhow::bail!("MCP init error: {} (code {})", error.message, error.code);
            }
        };

        // Send initialized notification
        let notif = JsonRpcRequest::new(0, "notifications/initialized", None);
        let _ = transport.send_request(&notif.serialize());

        // List tools
        let tools = self.list_tools_from_server(&mut transport)?;

        self.instances.insert(name.clone(), McpServerInstance {
            config,
            transport,
            server_info: init_result.server_info,
            tools,
        });

        Ok(name)
    }

    /// List tools from a specific MCP server.
    fn list_tools_from_server(&mut self, transport: &mut McpTransport) -> anyhow::Result<Vec<ToolDescription>> {
        let req = JsonRpcRequest::new(self.next_id(), "tools/list", None);
        let resp_text = transport.send_request(&req.serialize())?;
        let resp: JsonRpcResponse = serde_json::from_str(&resp_text)
            .map_err(|e| anyhow::anyhow!("tools/list response parse error: {e}"))?;

        match resp {
            JsonRpcResponse::Success { result, .. } => {
                let list: ListToolsResult = serde_json::from_value(result)
                    .map_err(|e| anyhow::anyhow!("Invalid tools/list result: {e}"))?;
                Ok(list.tools)
            }
            JsonRpcResponse::Error { error, .. } => {
                anyhow::bail!("MCP tools/list error: {} (code {})", error.message, error.code);
            }
        }
    }

    /// Call a tool on the MCP server that owns it.
    pub fn call_tool(&mut self, tool_name: &str, input: &str) -> anyhow::Result<String> {
        // Extract key info first to avoid borrow conflicts
        let server_name = match self.find_tool(tool_name) {
            Some((srv, _tdesc)) => srv.to_string(),
            None => anyhow::bail!("MCP tool '{tool_name}' not found in any server"),
        };
        let id = self.next_id();

        // Build the params first (no instance borrow yet)
        let arguments: serde_json::Value = match serde_json::from_str(input) {
            Ok(v) => v,
            Err(_) => serde_json::json!({"input": input}),
        };

        // Now borrow instance
        let instance = self.instances.get_mut(&server_name)
            .ok_or_else(|| anyhow::anyhow!("Server '{server_name}' is not running"))?;

        // Get the tool description name from the actual tool list
        let tool_desc_name = instance.tools.iter()
            .find(|t| t.name == tool_name)
            .map(|t| t.name.clone())
            .unwrap_or_else(|| tool_name.to_string());

        let params = CallToolParams {
            name: tool_desc_name,
            arguments,
        };

        let req = JsonRpcRequest::new(id, "tools/call", Some(serde_json::to_value(params)?));
        let resp_text = instance.transport.send_request(&req.serialize())?;
        let resp: JsonRpcResponse = serde_json::from_str(&resp_text)
            .map_err(|e| anyhow::anyhow!("tools/call response parse error: {e}"))?;

        match resp {
            JsonRpcResponse::Success { result, .. } => {
                let call_result: CallToolResult = serde_json::from_value(result)
                    .map_err(|e| anyhow::anyhow!("Invalid tools/call result: {e}"))?;
                if call_result.is_error {
                    Ok(format!("Error: {}", call_result.text()))
                } else {
                    Ok(call_result.text())
                }
            }
            JsonRpcResponse::Error { error, .. } => {
                anyhow::bail!("MCP tool call error: {} (code {})", error.message, error.code);
            }
        }
    }

    /// Find which server provides a tool.
    fn find_tool(&self, tool_name: &str) -> Option<(&str, &ToolDescription)> {
        for (name, instance) in &self.instances {
            for tool in &instance.tools {
                if tool.name == tool_name {
                    return Some((name.as_str(), tool));
                }
            }
        }
        None
    }

    /// Get all available tools from all servers.
    pub fn all_tools(&self) -> Vec<ToolInfo> {
        let mut tools = Vec::new();
        for (server_name, instance) in &self.instances {
            for tool in &instance.tools {
                tools.push(ToolInfo {
                    server_name: server_name.clone(),
                    tool_name: tool.name.clone(),
                    description: tool.description.clone(),
                });
            }
        }
        tools.sort_by(|a, b| a.tool_name.cmp(&b.tool_name));
        tools
    }

    /// Get list of running servers.
    pub fn list_servers(&self) -> Vec<ServerStatus> {
        self.instances.iter().map(|(name, instance)| {
            ServerStatus {
                name: name.clone(),
                server_info: instance.server_info.clone(),
                tool_count: instance.tools.len(),
            }
        }).collect()
    }

    /// Check if a specific tool is available.
    pub fn has_tool(&self, tool_name: &str) -> bool {
        self.find_tool(tool_name).is_some()
    }

    /// Stop a specific server.
    pub fn stop_server(&mut self, name: &str) -> anyhow::Result<()> {
        if let Some(mut instance) = self.instances.remove(name) {
            instance.transport.shutdown()?;
        }
        Ok(())
    }

    /// Stop all servers.
    pub fn stop_all(&mut self) {
        for (_, mut instance) in self.instances.drain() {
            let _ = instance.transport.shutdown();
        }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

impl Drop for McpRegistry {
    fn drop(&mut self) {
        self.stop_all();
    }
}

/// ─── Data types ───────────────────────────────────────────────────────────

/// Information about a discovered MCP tool.
#[derive(Debug, Clone)]
pub struct ToolInfo {
    pub server_name: String,
    pub tool_name: String,
    pub description: String,
}

/// Status of a running MCP server.
#[derive(Debug, Clone)]
pub struct ServerStatus {
    pub name: String,
    pub server_info: ServerInfo,
    pub tool_count: usize,
}

/// ─── McpTool (wraps MCP tool as a CodeCoder Tool) ─────────────────────────

use crate::tools::Tool;

/// Implements CodeCoder's Tool trait by delegating to an MCP server.
pub struct McpTool {
    name: String,
    description: String,
    registry: Arc<Mutex<McpRegistry>>,
}

impl McpTool {
    pub fn new(name: &str, description: &str, registry: Arc<Mutex<McpRegistry>>) -> Self {
        Self {
            name: name.to_string(),
            description: format!("[MCP] {}", description),
            registry,
        }
    }
}

impl Tool for McpTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        let mut reg = self.registry.lock()
            .map_err(|e| anyhow::anyhow!("MCP registry lock error: {e}"))?;
        reg.call_tool(&self.name, input)
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_config_deserialize() {
        let json = r#"{
            "name": "test-server",
            "command": "echo",
            "args": ["hello"],
            "env": ["KEY=VALUE"]
        }"#;
        let config: McpServerConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "test-server");
        assert_eq!(config.command, "echo");
        assert_eq!(config.args, vec!["hello"]);
        assert_eq!(config.env, vec!["KEY=VALUE"]);
    }

    #[test]
    fn test_tool_info_ordering() {
        let mut tools = vec![
            ToolInfo { server_name: "s1".into(), tool_name: "z_last".into(), description: "".into() },
            ToolInfo { server_name: "s1".into(), tool_name: "a_first".into(), description: "".into() },
        ];
        tools.sort_by(|a, b| a.tool_name.cmp(&b.tool_name));
        assert_eq!(tools[0].tool_name, "a_first");
        assert_eq!(tools[1].tool_name, "z_last");
    }

    #[test]
    fn test_mcp_registry_empty() {
        let reg = McpRegistry::new(vec![]);
        assert!(reg.list_servers().is_empty());
        assert!(reg.all_tools().is_empty());
        assert!(!reg.has_tool("nonexistent"));
    }
}
