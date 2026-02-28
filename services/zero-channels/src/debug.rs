//! Debug information collection and formatting for IM replies.
//!
//! This module provides structures and functions for collecting and formatting
//! debug information when users append `@@debug` to their messages.
//!
//! # Debug Mode
//!
//! When a message contains `@@debug`, the system collects:
//! - Agents used (name, invocation count, duration)
//! - Models used (name, tokens, duration)
//! - Service call chain (which services were called, timing)
//! - Data flow metrics (request/response sizes)
//!
//! The debug information is appended to the final response in a platform-specific format.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

// ============================================================================
// Core Debug Structures
// ============================================================================

/// Debug context for tracking all debug information during task execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugContext {
    /// Unique trace ID for this request
    pub trace_id: String,
    /// Task start timestamp (Unix millis)
    pub start_time: u64,
    /// Agents used during execution
    pub agents_used: Vec<AgentUsage>,
    /// Models used during execution
    pub models_used: Vec<ModelUsage>,
    /// Service call chain
    pub call_chain: Vec<ServiceCall>,
    /// Data flow metrics
    pub data_flow: DataFlowMetrics,
    /// Additional metadata
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

/// Information about an agent usage during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentUsage {
    /// Agent name (e.g., "build", "code-reviewer", "macro")
    pub agent: String,
    /// Display name for the agent
    pub display_name: Option<String>,
    /// Number of times this agent was invoked
    pub invocations: u32,
    /// Total duration in milliseconds
    pub duration_ms: u64,
    /// Whether this agent was the primary agent
    pub is_primary: bool,
}

/// Information about a model usage during execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelUsage {
    /// Model identifier (e.g., "claude-opus-4.5")
    pub model: String,
    /// Provider (e.g., "anthropic", "openai")
    pub provider: Option<String>,
    /// Input tokens consumed
    pub input_tokens: u64,
    /// Output tokens generated
    pub output_tokens: u64,
    /// Total tokens
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u64>,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// A service call in the execution chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceCall {
    /// Service name (e.g., "zero-channels", "ccode", "anthropic")
    pub service: String,
    /// Operation performed
    pub operation: String,
    /// Duration in milliseconds
    pub duration_ms: u64,
}

/// Data flow metrics for the request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataFlowMetrics {
    /// Request size in bytes
    pub request_bytes: u64,
    /// Response size in bytes
    pub response_bytes: u64,
    /// Total duration in milliseconds
    pub total_duration_ms: u64,
}

impl DebugContext {
    /// Create a new debug context.
    pub fn new(trace_id: String) -> Self {
        Self {
            trace_id,
            start_time: current_timestamp_millis(),
            agents_used: Vec::new(),
            models_used: Vec::new(),
            call_chain: Vec::new(),
            data_flow: DataFlowMetrics {
                request_bytes: 0,
                response_bytes: 0,
                total_duration_ms: 0,
            },
            metadata: HashMap::new(),
        }
    }

    /// Add an agent usage record.
    pub fn add_agent_usage(&mut self, usage: AgentUsage) {
        // Check if agent already exists, update if so
        if let Some(existing) = self.agents_used.iter_mut().find(|a| a.agent == usage.agent) {
            existing.invocations += usage.invocations;
            existing.duration_ms += usage.duration_ms;
        } else {
            self.agents_used.push(usage);
        }
    }

    /// Add a model usage record.
    pub fn add_model_usage(&mut self, usage: ModelUsage) {
        self.models_used.push(usage);
    }

    /// Add a service call to the chain.
    pub fn add_service_call(&mut self, call: ServiceCall) {
        self.call_chain.push(call);
    }

    /// Set data flow metrics.
    pub fn set_data_flow(&mut self, metrics: DataFlowMetrics) {
        self.data_flow = metrics;
    }

    /// Add metadata key-value pair.
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Calculate total duration from start time.
    pub fn calculate_duration(&self) -> Duration {
        let elapsed = current_timestamp_millis().saturating_sub(self.start_time);
        Duration::from_millis(elapsed)
    }

    /// Get total tokens used across all models.
    pub fn total_tokens(&self) -> u64 {
        self.models_used
            .iter()
            .map(|m| m.total_tokens.unwrap_or(m.input_tokens + m.output_tokens))
            .sum()
    }

    /// Check if any debug information was collected.
    pub fn is_empty(&self) -> bool {
        self.agents_used.is_empty()
            && self.models_used.is_empty()
            && self.call_chain.is_empty()
            && self.data_flow.request_bytes == 0
    }
}

// ============================================================================
// Formatting Functions
// ============================================================================

/// Format debug info for Telegram HTML output.
pub fn format_debug_html(debug: &DebugContext) -> String {
    let mut parts = vec![
        format!("<b>🐛 Debug Info</b>"),
        format!("<b>🔍 Trace ID:</b> <code>{}</code>", truncate_id(&debug.trace_id)),
    ];

    // Agents - show first
    if !debug.agents_used.is_empty() {
        parts.push("".to_string());
        parts.push("<b>🤵 Agents:</b>".to_string());
        for agent in &debug.agents_used {
            let display = agent.display_name.as_deref().unwrap_or(&agent.agent);
            let invocations = if agent.invocations > 1 {
                format!(" (x{})", agent.invocations)
            } else {
                String::new()
            };
            parts.push(format!(
                "• <b>{}</b>{}\n  └ Duration: {}",
                escape_html(display),
                invocations,
                format_duration_ms(agent.duration_ms)
            ));
        }
    }

    // Call chain
    if !debug.call_chain.is_empty() {
        parts.push("".to_string());
        parts.push("<b>📡 Call Chain:</b>".to_string());
        for (i, call) in debug.call_chain.iter().enumerate() {
            parts.push(format!(
                "{}. <b>{}</b> → <b>{}</b> ({}ms)",
                i + 1,
                escape_html(&call.service),
                escape_html(&call.operation),
                call.duration_ms
            ));
        }
    }

    // Models
    if !debug.models_used.is_empty() {
        parts.push("".to_string());
        parts.push("<b>🤖 Models:</b>".to_string());
        for model in &debug.models_used {
            let total = model.total_tokens.unwrap_or(model.input_tokens + model.output_tokens);
            let provider = model.provider.as_deref().unwrap_or("unknown");
            parts.push(format!(
                "• <b>{}</b>: {} in + {} out = {} tokens\n  └ Provider: {}",
                escape_html(&model.model),
                model.input_tokens,
                model.output_tokens,
                total,
                escape_html(provider)
            ));
        }

        // Summary
        let total = debug.total_tokens();
        parts.push(format!("  └ <b>Total:</b> {} tokens", total));
    }

    // Data flow
    let duration = debug.calculate_duration().as_secs_f64();
    let duration_str = if duration < 1.0 {
        format!("{:.0}ms", duration * 1000.0)
    } else if duration < 60.0 {
        format!("{:.1}s", duration)
    } else {
        let mins = (duration / 60.0).floor();
        let secs = duration % 60.0;
        format!("{}m {:.0}s", mins, secs)
    };

    parts.push("".to_string());
    parts.push("<b>📊 Data Flow:</b>".to_string());
    parts.push(format!(
        "Request: {} | Response: {}",
        format_bytes(debug.data_flow.request_bytes),
        format_bytes(debug.data_flow.response_bytes)
    ));
    parts.push(format!("Duration: {}", duration_str));

    parts.join("\n")
}

/// Format debug info for Slack mrkdwn output.
pub fn format_debug_mrkdwn(debug: &DebugContext) -> String {
    let mut parts = vec![
        "*🐛 Debug Info*".to_string(),
        format!("*🔍 Trace ID:* `{}`", truncate_id(&debug.trace_id)),
    ];

    // Agents - show first
    if !debug.agents_used.is_empty() {
        parts.push("".to_string());
        parts.push("*🤵 Agents:*".to_string());
        for agent in &debug.agents_used {
            let display = agent.display_name.as_deref().unwrap_or(&agent.agent);
            let invocations = if agent.invocations > 1 {
                format!(" (x{})", agent.invocations)
            } else {
                String::new()
            };
            parts.push(format!(
                "• *{}*{}\n  └ Duration: {}",
                display,
                invocations,
                format_duration_ms(agent.duration_ms)
            ));
        }
    }

    // Call chain
    if !debug.call_chain.is_empty() {
        parts.push("".to_string());
        parts.push("*📡 Call Chain:*".to_string());
        for (i, call) in debug.call_chain.iter().enumerate() {
            parts.push(format!(
                "{}. *{}* → *{}* ({}ms)",
                i + 1,
                call.service,
                call.operation,
                call.duration_ms
            ));
        }
    }

    // Models
    if !debug.models_used.is_empty() {
        parts.push("".to_string());
        parts.push("*🤖 Models:*".to_string());
        for model in &debug.models_used {
            let total = model.total_tokens.unwrap_or(model.input_tokens + model.output_tokens);
            let provider = model.provider.as_deref().unwrap_or("unknown");
            parts.push(format!(
                "• *{}*: {} in + {} out = {} tokens\n  └ Provider: {}",
                model.model, model.input_tokens, model.output_tokens, total, provider
            ));
        }

        let total = debug.total_tokens();
        parts.push(format!("  └ *Total:* {} tokens", total));
    }

    // Data flow
    let duration = debug.calculate_duration().as_secs_f64();
    let duration_str = if duration < 1.0 {
        format!("{:.0}ms", duration * 1000.0)
    } else if duration < 60.0 {
        format!("{:.1}s", duration)
    } else {
        let mins = (duration / 60.0).floor();
        let secs = duration % 60.0;
        format!("{}m {:.0}s", mins, secs)
    };

    parts.push("".to_string());
    parts.push("*📊 Data Flow:*".to_string());
    parts.push(format!(
        "Request: {} | Response: {}",
        format_bytes(debug.data_flow.request_bytes),
        format_bytes(debug.data_flow.response_bytes)
    ));
    parts.push(format!("Duration: {}", duration_str));

    parts.join("\n")
}

/// Format debug info for Discord Markdown output.
pub fn format_debug_markdown(debug: &DebugContext) -> String {
    let mut parts = vec![
        "**🐛 Debug Info**".to_string(),
        format!("**🔍 Trace ID:** `{}`", truncate_id(&debug.trace_id)),
    ];

    // Agents - show first
    if !debug.agents_used.is_empty() {
        parts.push("".to_string());
        parts.push("**🤵 Agents:**".to_string());
        for agent in &debug.agents_used {
            let display = agent.display_name.as_deref().unwrap_or(&agent.agent);
            let invocations = if agent.invocations > 1 {
                format!(" (x{})", agent.invocations)
            } else {
                String::new()
            };
            parts.push(format!(
                "• **{}**{}\n  └ Duration: {}",
                display,
                invocations,
                format_duration_ms(agent.duration_ms)
            ));
        }
    }

    // Call chain
    if !debug.call_chain.is_empty() {
        parts.push("".to_string());
        parts.push("**📡 Call Chain:**".to_string());
        for (i, call) in debug.call_chain.iter().enumerate() {
            parts.push(format!(
                "{}. **{}** → **{}** ({}ms)",
                i + 1,
                call.service,
                call.operation,
                call.duration_ms
            ));
        }
    }

    // Models
    if !debug.models_used.is_empty() {
        parts.push("".to_string());
        parts.push("**🤖 Models:**".to_string());
        for model in &debug.models_used {
            let total = model.total_tokens.unwrap_or(model.input_tokens + model.output_tokens);
            let provider = model.provider.as_deref().unwrap_or("unknown");
            parts.push(format!(
                "• **{}**: {} in + {} out = {} tokens\n  └ Provider: {}",
                model.model, model.input_tokens, model.output_tokens, total, provider
            ));
        }

        let total = debug.total_tokens();
        parts.push(format!("  └ **Total:** {} tokens", total));
    }

    // Data flow
    let duration = debug.calculate_duration().as_secs_f64();
    let duration_str = if duration < 1.0 {
        format!("{:.0}ms", duration * 1000.0)
    } else if duration < 60.0 {
        format!("{:.1}s", duration)
    } else {
        let mins = (duration / 60.0).floor();
        let secs = duration % 60.0;
        format!("{}m {:.0}s", mins, secs)
    };

    parts.push("".to_string());
    parts.push("**📊 Data Flow:**".to_string());
    parts.push(format!(
        "Request: {} | Response: {}",
        format_bytes(debug.data_flow.request_bytes),
        format_bytes(debug.data_flow.response_bytes)
    ));
    parts.push(format!("Duration: {}", duration_str));

    parts.join("\n")
}

/// Format debug info as plain text (fallback for other platforms).
pub fn format_debug_plain(debug: &DebugContext) -> String {
    let mut parts = vec![
        "🐛 Debug Info".to_string(),
        format!("🔍 Trace ID: {}", truncate_id(&debug.trace_id)),
    ];

    // Agents - show first
    if !debug.agents_used.is_empty() {
        parts.push("".to_string());
        parts.push("🤵 Agents:".to_string());
        for agent in &debug.agents_used {
            let display = agent.display_name.as_deref().unwrap_or(&agent.agent);
            let invocations = if agent.invocations > 1 {
                format!(" (x{})", agent.invocations)
            } else {
                String::new()
            };
            parts.push(format!(
                "• {}{}\n  └ Duration: {}",
                display,
                invocations,
                format_duration_ms(agent.duration_ms)
            ));
        }
    }

    // Call chain
    if !debug.call_chain.is_empty() {
        parts.push("".to_string());
        parts.push("📡 Call Chain:".to_string());
        for (i, call) in debug.call_chain.iter().enumerate() {
            parts.push(format!(
                "{}. {} → {} ({}ms)",
                i + 1, call.service, call.operation, call.duration_ms
            ));
        }
    }

    // Models
    if !debug.models_used.is_empty() {
        parts.push("".to_string());
        parts.push("🤖 Models:".to_string());
        for model in &debug.models_used {
            let total = model.total_tokens.unwrap_or(model.input_tokens + model.output_tokens);
            let provider = model.provider.as_deref().unwrap_or("unknown");
            parts.push(format!(
                "• {}: {} in + {} out = {} tokens\n  └ Provider: {}",
                model.model, model.input_tokens, model.output_tokens, total, provider
            ));
        }

        let total = debug.total_tokens();
        parts.push(format!("  └ Total: {} tokens", total));
    }

    // Data flow
    let duration = debug.calculate_duration().as_secs_f64();
    let duration_str = if duration < 1.0 {
        format!("{:.0}ms", duration * 1000.0)
    } else if duration < 60.0 {
        format!("{:.1}s", duration)
    } else {
        let mins = (duration / 60.0).floor();
        let secs = duration % 60.0;
        format!("{}m {:.0}s", mins, secs)
    };

    parts.push("".to_string());
    parts.push("📊 Data Flow:".to_string());
    parts.push(format!(
        "Request: {} | Response: {}",
        format_bytes(debug.data_flow.request_bytes),
        format_bytes(debug.data_flow.response_bytes)
    ));
    parts.push(format!("Duration: {}", duration_str));

    parts.join("\n")
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get current timestamp in milliseconds.
fn current_timestamp_millis() -> u64 {
    use std::time::SystemTime;
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Truncate ID to first 8 characters for display.
fn truncate_id(id: &str) -> String {
    if id.len() > 8 {
        format!("{}...", &id[..8])
    } else {
        id.to_string()
    }
}

/// Format bytes in human-readable format.
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else if bytes > 0 {
        format!("{} B", bytes)
    } else {
        "0 B".to_string()
    }
}

/// Format duration in milliseconds as human-readable string.
fn format_duration_ms(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        let mins = ms / 60_000;
        let secs = (ms % 60_000) / 1000;
        format!("{}m {:.0}s", mins, secs)
    }
}

/// Escape HTML special characters.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

// ============================================================================
// Utility Functions for Bridge Integration
// ============================================================================

/// Detect if a message contains the @@debug flag.
pub fn is_debug_request(content: &str) -> bool {
    content.trim().contains("@@debug")
}

/// Extract the debug flag and return cleaned message content.
///
/// Returns (has_debug, cleaned_content)
pub fn extract_debug_flag(content: &str) -> (bool, String) {
    let has_debug = is_debug_request(content);
    let cleaned = content.replace("@@debug", "").trim().to_string();
    (has_debug, cleaned)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debug_context_new() {
        let ctx = DebugContext::new("test-trace-123".to_string());
        assert_eq!(ctx.trace_id, "test-trace-123");
        assert!(ctx.models_used.is_empty());
        assert!(ctx.call_chain.is_empty());
        assert!(ctx.is_empty());
    }

    #[test]
    fn test_debug_context_add_model() {
        let mut ctx = DebugContext::new("test".to_string());
        ctx.add_model_usage(ModelUsage {
            model: "claude-opus-4.5".to_string(),
            provider: Some("anthropic".to_string()),
            input_tokens: 1000,
            output_tokens: 500,
            total_tokens: Some(1500),
            duration_ms: 1200,
        });
        assert!(!ctx.is_empty());
        assert_eq!(ctx.models_used.len(), 1);
        assert_eq!(ctx.total_tokens(), 1500);
    }

    #[test]
    fn test_debug_context_add_service_call() {
        let mut ctx = DebugContext::new("test".to_string());
        ctx.add_service_call(ServiceCall {
            service: "ccode".to_string(),
            operation: "process".to_string(),
            duration_ms: 50,
        });
        assert!(!ctx.is_empty());
        assert_eq!(ctx.call_chain.len(), 1);
    }

    #[test]
    fn test_is_debug_request() {
        assert!(is_debug_request("@@debug hello"));
        assert!(is_debug_request("hello @@debug"));
        assert!(is_debug_request("hello @@debug world"));
        assert!(!is_debug_request("hello world"));
        assert!(!is_debug_request("@debug hello"));
    }

    #[test]
    fn test_extract_debug_flag() {
        let (has_debug, cleaned) = extract_debug_flag("@@debug hello world");
        assert!(has_debug);
        assert_eq!(cleaned, "hello world");

        let (has_debug, cleaned) = extract_debug_flag("hello @@debug world");
        assert!(has_debug);
        assert_eq!(cleaned, "hello world");

        let (has_debug, cleaned) = extract_debug_flag("hello world");
        assert!(!has_debug);
        assert_eq!(cleaned, "hello world");
    }

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.0 GB");
    }

    #[test]
    fn test_truncate_id() {
        assert_eq!(truncate_id("abc12345"), "abc12345...");
        assert_eq!(truncate_id("abc123"), "abc123");
        assert_eq!(truncate_id(""), "");
    }

    #[test]
    fn test_format_debug_html() {
        let mut ctx = DebugContext::new("trace-123".to_string());
        ctx.add_model_usage(ModelUsage {
            model: "claude-opus".to_string(),
            provider: Some("anthropic".to_string()),
            input_tokens: 1000,
            output_tokens: 500,
            total_tokens: Some(1500),
            duration_ms: 1000,
        });
        ctx.add_service_call(ServiceCall {
            service: "ccode".to_string(),
            operation: "process".to_string(),
            duration_ms: 50,
        });

        let html = format_debug_html(&ctx);
        assert!(html.contains("🐛 Debug Info"));
        assert!(html.contains("claude-opus"));
        assert!(html.contains("1500 tokens"));
        assert!(html.contains("ccode"));
        assert!(html.contains("trace-12"));
    }

    #[test]
    fn test_escape_html() {
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(escape_html("a & b"), "a &amp; b");
        assert_eq!(escape_html("\"quote\""), "&quot;quote&quot;");
    }
}
