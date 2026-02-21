//! Browser automation tool using agent-browser CLI.
//!
//! Provides AI-optimized web browsing capabilities via the agent-browser CLI.

use crate::security::SecurityPolicy;
use crate::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;

/// Browser automation tool.
pub struct BrowserTool {
    security: Arc<SecurityPolicy>,
    allowed_domains: Vec<String>,
    session_name: Option<String>,
}

/// Response from agent-browser --json commands.
#[derive(Debug, Deserialize)]
struct AgentBrowserResponse {
    success: bool,
    data: Option<Value>,
    error: Option<String>,
}

/// Supported browser actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserAction {
    Open { url: String },
    Snapshot {
        #[serde(default)]
        interactive_only: bool,
        #[serde(default)]
        compact: bool,
    },
    Click { selector: String },
    Fill { selector: String, value: String },
    Type { selector: String, text: String },
    GetText { selector: String },
    GetTitle,
    GetUrl,
    Screenshot {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        full_page: bool,
    },
    Wait {
        #[serde(default)]
        selector: Option<String>,
        #[serde(default)]
        ms: Option<u64>,
    },
    Press { key: String },
    Hover { selector: String },
    Scroll {
        direction: String,
        #[serde(default)]
        pixels: Option<u32>,
    },
    Close,
}

impl BrowserTool {
    /// Create a new browser tool.
    pub fn new(
        security: Arc<SecurityPolicy>,
        allowed_domains: Vec<String>,
        session_name: Option<String>,
    ) -> Self {
        Self {
            security,
            allowed_domains: normalize_domains(allowed_domains),
            session_name,
        }
    }

    /// Check if agent-browser CLI is available.
    pub async fn is_available() -> bool {
        Command::new("agent-browser")
            .arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Validate URL against allowlist.
    fn validate_url(&self, url: &str) -> anyhow::Result<()> {
        let url = url.trim();

        if url.is_empty() {
            anyhow::bail!("URL cannot be empty");
        }

        // Allow file:// URLs for local testing
        if url.starts_with("file://") {
            return Ok(());
        }

        if !url.starts_with("https://") && !url.starts_with("http://") {
            anyhow::bail!("Only http:// and https:// URLs are allowed");
        }

        if self.allowed_domains.is_empty() {
            anyhow::bail!("Browser tool enabled but no allowed_domains configured");
        }

        let host = extract_host(url)?;

        if is_private_host(&host) {
            anyhow::bail!("Blocked local/private host: {}", host);
        }

        if !host_matches_allowlist(&host, &self.allowed_domains) {
            anyhow::bail!("Host '{}' not in allowed_domains", host);
        }

        Ok(())
    }

    /// Execute an agent-browser command.
    async fn run_command(&self, args: &[&str]) -> anyhow::Result<AgentBrowserResponse> {
        let mut cmd = Command::new("agent-browser");

        if let Some(ref session) = self.session_name {
            cmd.arg("--session").arg(session);
        }

        cmd.args(args).arg("--json");

        let output = cmd
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        if let Ok(resp) = serde_json::from_str::<AgentBrowserResponse>(&stdout) {
            return Ok(resp);
        }

        if output.status.success() {
            Ok(AgentBrowserResponse {
                success: true,
                data: Some(json!({ "output": stdout.trim() })),
                error: None,
            })
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Ok(AgentBrowserResponse {
                success: false,
                data: None,
                error: Some(stderr.trim().to_string()),
            })
        }
    }

    fn to_result(&self, resp: AgentBrowserResponse) -> anyhow::Result<ToolResult> {
        if resp.success {
            let output = resp
                .data
                .map(|d| serde_json::to_string_pretty(&d).unwrap_or_default())
                .unwrap_or_default();
            Ok(ToolResult::success(output))
        } else {
            Ok(ToolResult::failure(
                resp.error.unwrap_or_else(|| "Unknown error".to_string()),
            ))
        }
    }
}

#[async_trait]
impl Tool for BrowserTool {
    fn name(&self) -> &str {
        "browser"
    }

    fn description(&self) -> &str {
        "Web browser automation using agent-browser. Supports navigation, clicking, \
        filling forms, taking screenshots, and getting accessibility snapshots. \
        Use 'snapshot' to get interactive elements with refs, then use refs for \
        precise element interaction. Allowed domains only."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["open", "snapshot", "click", "fill", "type", "get_text",
                             "get_title", "get_url", "screenshot", "wait", "press",
                             "hover", "scroll", "close"],
                    "description": "Browser action to perform"
                },
                "url": {
                    "type": "string",
                    "description": "URL to navigate to (for 'open' action)"
                },
                "selector": {
                    "type": "string",
                    "description": "Element selector: @ref (e.g. @e1), CSS, or text=..."
                },
                "value": {
                    "type": "string",
                    "description": "Value to fill or type"
                },
                "text": {
                    "type": "string",
                    "description": "Text to type"
                },
                "key": {
                    "type": "string",
                    "description": "Key to press (Enter, Tab, Escape, etc.)"
                },
                "direction": {
                    "type": "string",
                    "enum": ["up", "down", "left", "right"],
                    "description": "Scroll direction"
                },
                "pixels": {
                    "type": "integer",
                    "description": "Pixels to scroll"
                },
                "interactive_only": {
                    "type": "boolean",
                    "description": "For snapshot: only show interactive elements"
                },
                "compact": {
                    "type": "boolean",
                    "description": "For snapshot: remove empty structural elements"
                },
                "full_page": {
                    "type": "boolean",
                    "description": "For screenshot: capture full page"
                },
                "path": {
                    "type": "string",
                    "description": "File path for screenshot"
                },
                "ms": {
                    "type": "integer",
                    "description": "Milliseconds to wait"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        if !self.security.can_act() {
            return Ok(ToolResult::failure("Action blocked: autonomy is read-only"));
        }

        if !self.security.record_action() {
            return Ok(ToolResult::failure("Action blocked: rate limit exceeded"));
        }

        if !Self::is_available().await {
            return Ok(ToolResult::failure(
                "agent-browser CLI not found. Install with: npm install -g agent-browser",
            ));
        }

        let action_str = args
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing 'action' parameter"))?;

        match action_str {
            "open" => {
                let url = args
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'url' for open action"))?;
                if let Err(e) = self.validate_url(url) {
                    return Ok(ToolResult::failure(e.to_string()));
                }
                let resp = self.run_command(&["open", url]).await?;
                self.to_result(resp)
            }
            "snapshot" => {
                let mut args_vec = vec!["snapshot"];
                if args.get("interactive_only").and_then(|v| v.as_bool()).unwrap_or(true) {
                    args_vec.push("-i");
                }
                if args.get("compact").and_then(|v| v.as_bool()).unwrap_or(true) {
                    args_vec.push("-c");
                }
                let resp = self.run_command(&args_vec).await?;
                self.to_result(resp)
            }
            "click" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'selector' for click"))?;
                let resp = self.run_command(&["click", selector]).await?;
                self.to_result(resp)
            }
            "fill" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'selector' for fill"))?;
                let value = args
                    .get("value")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'value' for fill"))?;
                let resp = self.run_command(&["fill", selector, value]).await?;
                self.to_result(resp)
            }
            "type" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'selector' for type"))?;
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'text' for type"))?;
                let resp = self.run_command(&["type", selector, text]).await?;
                self.to_result(resp)
            }
            "get_text" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'selector' for get_text"))?;
                let resp = self.run_command(&["get", "text", selector]).await?;
                self.to_result(resp)
            }
            "get_title" => {
                let resp = self.run_command(&["get", "title"]).await?;
                self.to_result(resp)
            }
            "get_url" => {
                let resp = self.run_command(&["get", "url"]).await?;
                self.to_result(resp)
            }
            "screenshot" => {
                let mut args_vec = vec!["screenshot"];
                if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
                    args_vec.push(path);
                }
                if args.get("full_page").and_then(|v| v.as_bool()).unwrap_or(false) {
                    args_vec.push("--full");
                }
                let resp = self.run_command(&args_vec).await?;
                self.to_result(resp)
            }
            "wait" => {
                let mut args_vec = vec!["wait"];
                if let Some(selector) = args.get("selector").and_then(|v| v.as_str()) {
                    args_vec.push(selector);
                } else if let Some(ms) = args.get("ms").and_then(|v| v.as_u64()) {
                    let ms_str = ms.to_string();
                    let resp = self.run_command(&["wait", &ms_str]).await?;
                    return self.to_result(resp);
                }
                let resp = self.run_command(&args_vec).await?;
                self.to_result(resp)
            }
            "press" => {
                let key = args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'key' for press"))?;
                let resp = self.run_command(&["press", key]).await?;
                self.to_result(resp)
            }
            "hover" => {
                let selector = args
                    .get("selector")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'selector' for hover"))?;
                let resp = self.run_command(&["hover", selector]).await?;
                self.to_result(resp)
            }
            "scroll" => {
                let direction = args
                    .get("direction")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("Missing 'direction' for scroll"))?;
                let args_vec = vec!["scroll", direction];
                if let Some(pixels) = args.get("pixels").and_then(|v| v.as_u64()) {
                    let px_str = pixels.to_string();
                    let resp = self.run_command(&["scroll", direction, &px_str]).await?;
                    return self.to_result(resp);
                }
                let resp = self.run_command(&args_vec).await?;
                self.to_result(resp)
            }
            "close" => {
                let resp = self.run_command(&["close"]).await?;
                self.to_result(resp)
            }
            _ => Ok(ToolResult::failure(format!("Unknown action: {}", action_str))),
        }
    }
}

// Helper functions

fn normalize_domains(domains: Vec<String>) -> Vec<String> {
    domains
        .into_iter()
        .map(|d| d.trim().to_lowercase())
        .filter(|d| !d.is_empty())
        .collect()
}

fn extract_host(url_str: &str) -> anyhow::Result<String> {
    let url = url_str.trim();
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .or_else(|| url.strip_prefix("file://"))
        .unwrap_or(url);

    let host = without_scheme
        .split('/')
        .next()
        .unwrap_or(without_scheme)
        .split(':')
        .next()
        .unwrap_or(without_scheme);

    if host.is_empty() {
        anyhow::bail!("Invalid URL: no host");
    }

    Ok(host.to_lowercase())
}

fn is_private_host(host: &str) -> bool {
    let private_patterns = [
        "localhost",
        "127.",
        "10.",
        "192.168.",
        "172.16.",
        "172.17.",
        "172.18.",
        "172.19.",
        "172.20.",
        "172.21.",
        "172.22.",
        "172.23.",
        "172.24.",
        "172.25.",
        "172.26.",
        "172.27.",
        "172.28.",
        "172.29.",
        "172.30.",
        "172.31.",
        "0.0.0.0",
        "::1",
        "[::1]",
    ];

    private_patterns.iter().any(|p| host.starts_with(p) || host == *p)
}

fn host_matches_allowlist(host: &str, allowed: &[String]) -> bool {
    allowed.iter().any(|pattern| {
        if pattern == "*" {
            return true;
        }
        if pattern.starts_with("*.") {
            let suffix = &pattern[1..];
            host.ends_with(suffix) || host == &pattern[2..]
        } else {
            host == pattern || host.ends_with(&format!(".{}", pattern))
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_domains_works() {
        let domains = vec![
            "  Example.COM  ".into(),
            "docs.example.com".into(),
            String::new(),
        ];
        let normalized = normalize_domains(domains);
        assert_eq!(normalized, vec!["example.com", "docs.example.com"]);
    }

    #[test]
    fn extract_host_works() {
        assert_eq!(extract_host("https://example.com/path").unwrap(), "example.com");
        assert_eq!(
            extract_host("https://Sub.Example.COM:8080/").unwrap(),
            "sub.example.com"
        );
    }

    #[test]
    fn is_private_host_detects_local() {
        assert!(is_private_host("localhost"));
        assert!(is_private_host("127.0.0.1"));
        assert!(is_private_host("192.168.1.1"));
        assert!(!is_private_host("example.com"));
    }

    #[test]
    fn host_matches_allowlist_exact() {
        let allowed = vec!["example.com".into()];
        assert!(host_matches_allowlist("example.com", &allowed));
        assert!(host_matches_allowlist("sub.example.com", &allowed));
        assert!(!host_matches_allowlist("notexample.com", &allowed));
    }

    #[test]
    fn host_matches_allowlist_wildcard() {
        let allowed = vec!["*.example.com".into()];
        assert!(host_matches_allowlist("sub.example.com", &allowed));
        assert!(host_matches_allowlist("example.com", &allowed));
        assert!(!host_matches_allowlist("other.com", &allowed));
    }

    #[test]
    fn browser_tool_name() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = BrowserTool::new(security, vec!["example.com".into()], None);
        assert_eq!(tool.name(), "browser");
    }

    #[test]
    fn browser_tool_validates_url() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = BrowserTool::new(security, vec!["example.com".into()], None);

        assert!(tool.validate_url("https://example.com").is_ok());
        assert!(tool.validate_url("https://sub.example.com/path").is_ok());
        assert!(tool.validate_url("https://other.com").is_err());
        assert!(tool.validate_url("https://localhost").is_err());
        assert!(tool.validate_url("file:///tmp/test.html").is_ok());
    }

    #[test]
    fn browser_tool_empty_allowlist_blocks() {
        let security = Arc::new(SecurityPolicy::default());
        let tool = BrowserTool::new(security, vec![], None);
        assert!(tool.validate_url("https://example.com").is_err());
    }
}
