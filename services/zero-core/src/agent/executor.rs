//! Agent executor with tool-calling loop.
//!
//! Handles the core agent execution cycle:
//! 1. Send message to LLM
//! 2. Parse tool calls from response
//! 3. Check permissions and request confirmation if needed
//! 4. Execute tools and feed results back
//! 5. Repeat until final text response
//!
//! ## Configuration-Driven Execution
//!
//! The executor can be configured using `AgentConfig`:
//! ```rust,ignore
//! use zero_core::agent::{ConfiguredExecutor, AgentConfig};
//!
//! let config = AgentConfig { ... };
//! let executor = ConfiguredExecutor::from_config(config, provider, tools).await?;
//! let response = executor.execute("Hello!").await?;
//! ```

use super::confirmation::{
    request_confirmation_and_wait, ConfirmationResponse,
};
use super::context::ToolContext;
use super::loader::{AgentConfig, PermissionAction, RiskThreshold};
use super::provider::Provider;
use crate::agent_tools::{Tool, ToolResult};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::Write;
use std::sync::Arc;

/// Maximum number of tool-calling iterations to prevent infinite loops.
const MAX_ITERATIONS: usize = 10;

/// Default maximum iterations (can be overridden by config)
const DEFAULT_MAX_ITERATIONS: usize = 10;

/// Parsed tool call from LLM response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub args: serde_json::Value,
}

/// Tool risk level for auto-approve decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ToolRisk {
    /// Completely safe (read-only, no side effects)
    Safe,
    /// Low risk (local reads, codebase exploration)
    Low,
    /// Medium risk (file writes, git operations)
    Medium,
    /// High risk (shell commands, network access)
    High,
    /// Critical risk (destructive operations)
    Critical,
}

impl From<&str> for ToolRisk {
    fn from(tool_name: &str) -> Self {
        match tool_name.to_lowercase().as_str() {
            // Safe: read-only operations
            "read" | "glob" | "grep" | "ls" | "list" | "webfetch" | "websearch" => ToolRisk::Safe,

            // Low risk: code exploration
            "codesearch" | "notebook_read" | "task_list" | "task_get" => ToolRisk::Low,

            // Medium risk: file modifications
            "edit" | "write" | "notebook_edit" | "task_create" | "task_update" => ToolRisk::Medium,

            // High risk: external interactions
            "bash" | "shell" | "mcp_call" | "browser" => ToolRisk::High,

            // Unknown tools default to high risk
            _ => ToolRisk::High,
        }
    }
}

impl ToolRisk {
    /// Convert to numeric level for comparison
    fn level(&self) -> u8 {
        match self {
            ToolRisk::Safe => 0,
            ToolRisk::Low => 1,
            ToolRisk::Medium => 2,
            ToolRisk::High => 3,
            ToolRisk::Critical => 4,
        }
    }

    /// Check if this risk level exceeds a threshold
    pub fn exceeds(&self, threshold: RiskThreshold) -> bool {
        let threshold_level = match threshold {
            RiskThreshold::Safe => 0,
            RiskThreshold::Low => 1,
            RiskThreshold::Medium => 2,
            RiskThreshold::High => 3,
        };
        self.level() > threshold_level
    }
}

/// Agent executor that handles the tool-calling loop.
pub struct AgentExecutor {
    provider: Arc<dyn Provider>,
    tools: Vec<Box<dyn Tool>>,
    system_prompt: String,
    model: String,
    temperature: f64,
}

impl AgentExecutor {
    /// Create a new agent executor.
    pub fn new(
        provider: Arc<dyn Provider>,
        tools: Vec<Box<dyn Tool>>,
        system_prompt: String,
        model: String,
        temperature: f64,
    ) -> Self {
        Self {
            provider,
            tools,
            system_prompt,
            model,
            temperature,
        }
    }

    /// Execute a message through the agentic loop.
    /// Returns the final text response after all tool calls are resolved.
    pub async fn execute(&self, user_message: &str) -> Result<String> {
        self.execute_with_context(user_message, None).await
    }

    /// Execute with optional tool context for interactive operations.
    pub async fn execute_with_context(
        &self,
        user_message: &str,
        context: Option<ToolContext>,
    ) -> Result<String> {
        let mut conversation = vec![format!("User: {user_message}")];
        let mut iterations = 0;

        loop {
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                tracing::warn!("Agent executor reached max iterations ({MAX_ITERATIONS})");
                return Ok("I apologize, but I've reached the maximum number of steps. Please try a simpler request.".to_string());
            }

            // Build the full message with conversation history
            let full_message = conversation.join("\n\n");

            // Call the LLM
            let response = self
                .provider
                .chat_with_system(
                    Some(&self.system_prompt),
                    &full_message,
                    &self.model,
                    self.temperature,
                )
                .await?;

            tracing::debug!(
                "LLM response (iteration {iterations}): {}",
                truncate(&response, 200)
            );

            // Try to parse tool calls from the response
            if let Some(tool_calls) = Self::parse_tool_calls(&response) {
                let mut tool_results = Vec::new();

                for call in tool_calls {
                    tracing::info!("Executing tool: {} with args: {}", call.tool, call.args);

                    // Inject context into tool args if available
                    let enriched_args = if let Some(ref ctx) = context {
                        let mut args = call.args.clone();
                        if let Some(obj) = args.as_object_mut() {
                            obj.insert(
                                "_context".to_string(),
                                serde_json::json!({
                                    "channel": ctx.channel_name,
                                    "sender_id": ctx.sender_id
                                }),
                            );
                        }
                        args
                    } else {
                        call.args.clone()
                    };

                    let enriched_call = ToolCall {
                        tool: call.tool.clone(),
                        args: enriched_args,
                    };

                    match self.execute_tool(&enriched_call).await {
                        Ok(result) => {
                            let result_text = if result.success {
                                tracing::info!(
                                    "Tool '{}' succeeded (output len: {})",
                                    call.tool,
                                    result.output.len()
                                );
                                format!("Tool '{}' succeeded:\n{}", call.tool, result.output)
                            } else {
                                tracing::warn!(
                                    "Tool '{}' returned failure: {:?}",
                                    call.tool,
                                    result.error
                                );
                                format!(
                                    "Tool '{}' failed: {}",
                                    call.tool,
                                    result.error.unwrap_or_else(|| "Unknown error".to_string())
                                )
                            };
                            tool_results.push(result_text);
                        }
                        Err(e) => {
                            let error_text = format!("Tool '{}' error: {e}", call.tool);
                            tracing::error!("{error_text}");
                            tool_results.push(error_text);
                        }
                    }
                }

                // Add assistant response and tool results to conversation
                conversation.push(format!("Assistant: {response}"));
                conversation.push(format!("Tool Results:\n{}", tool_results.join("\n\n")));
            } else {
                // No tool calls found - this is the final response
                // Extract the actual response text (remove any JSON artifacts)
                let final_response = Self::extract_final_response(&response);
                return Ok(final_response);
            }
        }
    }

    /// Parse tool calls from LLM response.
    /// Supports multiple formats:
    /// 1. JSON block: ```json\n{"tool": "name", "args": {...}}\n```
    /// 2. Inline JSON: {"tool": "name", "args": {...}}
    fn parse_tool_calls(response: &str) -> Option<Vec<ToolCall>> {
        let mut calls = Vec::new();

        // Try to find JSON blocks with tool calls
        // Pattern 1: ```json ... ```
        for block in extract_json_blocks(response) {
            if let Ok(call) = serde_json::from_str::<ToolCall>(&block) {
                calls.push(call);
            } else if let Ok(multi) = serde_json::from_str::<Vec<ToolCall>>(&block) {
                calls.extend(multi);
            }
        }

        // Pattern 2: Look for inline JSON objects with "tool" field
        if calls.is_empty() {
            if let Some(call) = Self::find_inline_tool_call(response) {
                calls.push(call);
            }
        }

        if calls.is_empty() {
            None
        } else {
            Some(calls)
        }
    }

    /// Find inline tool call JSON in response.
    fn find_inline_tool_call(response: &str) -> Option<ToolCall> {
        // Look for {"tool": pattern
        let patterns = [r#"{"tool":"#, r#"{ "tool":"#, r#"{"tool" :"#];

        for pattern in patterns {
            if let Some(start) = response.find(pattern) {
                // Find matching closing brace
                let rest = &response[start..];
                if let Some(end) = find_matching_brace(rest) {
                    let json_str = &rest[..=end];
                    if let Ok(call) = serde_json::from_str::<ToolCall>(json_str) {
                        return Some(call);
                    }
                }
            }
        }

        None
    }

    /// Execute a single tool call.
    async fn execute_tool(&self, call: &ToolCall) -> Result<ToolResult> {
        // Find the tool by name
        let tool = self
            .tools
            .iter()
            .find(|t| t.name() == call.tool)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", call.tool))?;

        tool.execute(call.args.clone()).await
    }

    /// Extract final response text, removing any JSON artifacts or tool call attempts.
    fn extract_final_response(response: &str) -> String {
        let mut result = response.to_string();

        // Remove JSON code blocks
        while let Some(start) = result.find("```json") {
            if let Some(end) = result[start..]
                .find("```\n")
                .or_else(|| result[start..].find("```"))
            {
                let end_pos = start + end + 3;
                if end_pos < result.len() && result.as_bytes().get(end_pos) == Some(&b'\n') {
                    result = format!("{}{}", &result[..start], &result[end_pos + 1..]);
                } else {
                    result = format!("{}{}", &result[..start], &result[end_pos..]);
                }
            } else {
                // Incomplete JSON block - truncate
                result = result[..start].to_string();
                break;
            }
        }

        // Clean up any trailing incomplete JSON
        if let Some(idx) = result.rfind('{') {
            let rest = &result[idx..];
            // If it looks like a tool call attempt but is incomplete
            if rest.contains("\"tool\"") && !rest.contains('}') {
                result = result[..idx].trim_end().to_string();
            }
        }

        result.trim().to_string()
    }

    /// Build tool descriptions for the system prompt.
    pub fn build_tool_prompt(&self) -> String {
        let mut prompt = String::from("## Available Tools\n\n");
        prompt.push_str("To use a tool, respond with a JSON block:\n");
        prompt.push_str(
            "```json\n{\"tool\": \"tool_name\", \"args\": {\"param\": \"value\"}}\n```\n\n",
        );
        prompt.push_str(
            "After tool execution, you'll receive the results. Continue using tools or provide a final text response.\n\n",
        );
        prompt.push_str("### Tools:\n\n");

        for tool in &self.tools {
            let _ = writeln!(prompt, "**{}**: {}", tool.name(), tool.description());
            let _ = writeln!(prompt, "Parameters: {}\n", tool.parameters_schema());
        }

        prompt
    }

    /// Get a reference to the tools.
    pub fn tools(&self) -> &[Box<dyn Tool>] {
        &self.tools
    }

    /// Get the model name.
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the temperature.
    pub fn temperature(&self) -> f64 {
        self.temperature
    }
}

// ============================================================================
// Configuration-Driven Executor
// ============================================================================

/// Configuration-driven executor that integrates with AgentConfig
pub struct ConfiguredExecutor {
    /// The underlying executor
    executor: AgentExecutor,
    /// Agent configuration
    config: Arc<AgentConfig>,
    /// Tools that have been auto-approved in this session
    auto_approved_tools: std::sync::Mutex<HashSet<String>>,
    /// Number of auto-approvals used in this session
    auto_approve_count: std::sync::atomic::AtomicU32,
    /// Session context for confirmations
    session_id: String,
    /// Channel for confirmations
    channel: Option<String>,
    /// User ID for confirmations
    user_id: Option<String>,
}

impl ConfiguredExecutor {
    /// Create a new configured executor from agent configuration
    pub fn new(
        config: AgentConfig,
        provider: Arc<dyn Provider>,
        tools: Vec<Box<dyn Tool>>,
        session_id: String,
    ) -> Self {
        let model = config
            .model
            .as_ref()
            .map(|m| m.model_id.clone())
            .unwrap_or_else(|| "claude-sonnet-4-5-20250514".to_string());

        let temperature = config
            .model
            .as_ref()
            .and_then(|m| m.temperature)
            .unwrap_or(0.7);

        let system_prompt = config.prompt_content.clone().unwrap_or_default();

        let executor = AgentExecutor::new(provider, tools, system_prompt, model, temperature);

        Self {
            executor,
            config: Arc::new(config),
            auto_approved_tools: std::sync::Mutex::new(HashSet::new()),
            auto_approve_count: std::sync::atomic::AtomicU32::new(0),
            session_id,
            channel: None,
            user_id: None,
        }
    }

    /// Set the channel for confirmation requests
    pub fn with_channel(mut self, channel: String) -> Self {
        self.channel = Some(channel);
        self
    }

    /// Set the user ID for confirmation requests
    pub fn with_user_id(mut self, user_id: String) -> Self {
        self.user_id = Some(user_id);
        self
    }

    /// Execute with permission checking and confirmation
    pub async fn execute(&self, user_message: &str) -> Result<String> {
        self.execute_with_context(user_message, None).await
    }

    /// Execute with context and permission checking
    pub async fn execute_with_context(
        &self,
        user_message: &str,
        context: Option<ToolContext>,
    ) -> Result<String> {
        let mut conversation = vec![format!("User: {user_message}")];
        let max_iterations = self.config.max_steps.unwrap_or(DEFAULT_MAX_ITERATIONS as u32) as usize;
        let mut iterations = 0;

        loop {
            iterations += 1;
            if iterations > max_iterations {
                tracing::warn!(
                    "Agent executor reached max iterations ({max_iterations})"
                );
                return Ok("I apologize, but I've reached the maximum number of steps. Please try a simpler request.".to_string());
            }

            // Build the full message with conversation history
            let full_message = conversation.join("\n\n");

            // Call the LLM
            let response = self
                .executor
                .provider
                .chat_with_system(
                    Some(&self.executor.system_prompt),
                    &full_message,
                    &self.executor.model,
                    self.executor.temperature,
                )
                .await?;

            tracing::debug!(
                "LLM response (iteration {iterations}): {}",
                truncate(&response, 200)
            );

            // Try to parse tool calls from the response
            if let Some(tool_calls) = AgentExecutor::parse_tool_calls(&response) {
                let mut tool_results = Vec::new();

                for call in tool_calls {
                    // Check permission for this tool
                    let (allowed, reason) = self.check_tool_permission(&call).await?;

                    if !allowed {
                        tracing::info!(
                            "Tool '{}' rejected: {}",
                            call.tool,
                            reason.as_deref().unwrap_or("Permission denied")
                        );
                        tool_results.push(format!(
                            "Tool '{}' not allowed: {}",
                            call.tool,
                            reason.unwrap_or_else(|| "Permission denied".to_string())
                        ));
                        continue;
                    }

                    tracing::info!("Executing tool: {} with args: {}", call.tool, call.args);

                    // Inject context into tool args if available
                    let enriched_args = if let Some(ref ctx) = context {
                        let mut args = call.args.clone();
                        if let Some(obj) = args.as_object_mut() {
                            obj.insert(
                                "_context".to_string(),
                                serde_json::json!({
                                    "channel": ctx.channel_name,
                                    "sender_id": ctx.sender_id
                                }),
                            );
                        }
                        args
                    } else {
                        call.args.clone()
                    };

                    let enriched_call = ToolCall {
                        tool: call.tool.clone(),
                        args: enriched_args,
                    };

                    match self.executor.execute_tool(&enriched_call).await {
                        Ok(result) => {
                            let result_text = if result.success {
                                tracing::info!(
                                    "Tool '{}' succeeded (output len: {})",
                                    call.tool,
                                    result.output.len()
                                );
                                format!("Tool '{}' succeeded:\n{}", call.tool, result.output)
                            } else {
                                tracing::warn!(
                                    "Tool '{}' returned failure: {:?}",
                                    call.tool,
                                    result.error
                                );
                                format!(
                                    "Tool '{}' failed: {}",
                                    call.tool,
                                    result.error.unwrap_or_else(|| "Unknown error".to_string())
                                )
                            };
                            tool_results.push(result_text);
                        }
                        Err(e) => {
                            let error_text = format!("Tool '{}' error: {e}", call.tool);
                            tracing::error!("{error_text}");
                            tool_results.push(error_text);
                        }
                    }
                }

                // Add assistant response and tool results to conversation
                conversation.push(format!("Assistant: {response}"));
                conversation.push(format!("Tool Results:\n{}", tool_results.join("\n\n")));
            } else {
                // No tool calls found - this is the final response
                let final_response = AgentExecutor::extract_final_response(&response);
                return Ok(final_response);
            }
        }
    }

    /// Check if a tool call is permitted
    async fn check_tool_permission(&self, call: &ToolCall) -> Result<(bool, Option<String>)> {
        let tool_name = call.tool.to_lowercase();

        // Get the permission action for this tool
        let pattern = self.extract_pattern_from_args(&call.args);
        let action = self.config.permission.check(&tool_name, &pattern);

        match action {
            PermissionAction::Allow => Ok((true, None)),
            PermissionAction::Deny => Ok((false, Some("Tool is denied by configuration".to_string()))),
            PermissionAction::Ask => {
                // Check if we can auto-approve
                if self.can_auto_approve(&call.tool) {
                    self.record_auto_approval(&call.tool);
                    return Ok((true, Some("Auto-approved".to_string())));
                }

                // Request user confirmation
                self.request_confirmation(call).await
            }
        }
    }

    /// Extract a pattern from tool arguments (for permission matching)
    fn extract_pattern_from_args(&self, args: &serde_json::Value) -> String {
        // Try common field names for file paths
        if let Some(obj) = args.as_object() {
            for key in ["path", "file_path", "filepath", "file", "pattern", "command"] {
                if let Some(value) = obj.get(key) {
                    if let Some(s) = value.as_str() {
                        return s.to_string();
                    }
                }
            }
        }
        "*".to_string()
    }

    /// Check if a tool can be auto-approved
    fn can_auto_approve(&self, tool_name: &str) -> bool {
        let auto_approve = &self.config.auto_approve;

        if !auto_approve.enabled {
            return false;
        }

        // Check if tool is in allowed list
        let tool_allowed = auto_approve.allowed_tools.is_empty()
            || auto_approve.allowed_tools.iter().any(|t| {
                t.eq_ignore_ascii_case(tool_name)
            });

        if !tool_allowed {
            return false;
        }

        // Check risk threshold
        let tool_risk = ToolRisk::from(tool_name);
        if tool_risk.exceeds(auto_approve.risk_threshold) {
            return false;
        }

        // Check max approvals
        if let Some(max) = auto_approve.max_approvals {
            let current = self.auto_approve_count.load(std::sync::atomic::Ordering::SeqCst);
            if current >= max {
                return false;
            }
        }

        true
    }

    /// Record an auto-approval
    fn record_auto_approval(&self, tool_name: &str) {
        self.auto_approve_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        if let Ok(mut approved) = self.auto_approved_tools.lock() {
            approved.insert(tool_name.to_string());
        }
    }

    /// Request user confirmation for a tool call
    async fn request_confirmation(&self, call: &ToolCall) -> Result<(bool, Option<String>)> {
        let channel = self.channel.as_deref().unwrap_or("cli");
        let user_id = self.user_id.as_deref().unwrap_or("unknown");
        let request_id = format!("{}-{}", self.session_id, uuid::Uuid::new_v4());

        let message = format!(
            "Tool '{}' requires confirmation.\nArguments: {}",
            call.tool,
            serde_json::to_string_pretty(&call.args).unwrap_or_else(|_| call.args.to_string())
        );

        match request_confirmation_and_wait(
            channel,
            user_id,
            &request_id,
            &call.tool,
            &message,
            Some(120), // 2 minute timeout
        )
        .await
        {
            Ok(response) => match response {
                ConfirmationResponse::Once => Ok((true, Some("Approved once".to_string()))),
                ConfirmationResponse::Always => {
                    // Record for future auto-approval
                    self.record_auto_approval(&call.tool);
                    Ok((true, Some("Always approved".to_string())))
                }
                ConfirmationResponse::Reject => Ok((false, Some("User rejected".to_string()))),
            },
            Err(e) => {
                tracing::warn!("Confirmation request failed: {}", e);
                Ok((false, Some(format!("Confirmation failed: {}", e))))
            }
        }
    }

    /// Get the agent configuration
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }

    /// Get the session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get the number of auto-approvals used
    pub fn auto_approve_count(&self) -> u32 {
        self.auto_approve_count.load(std::sync::atomic::Ordering::SeqCst)
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Extract JSON blocks from markdown code fences.
fn extract_json_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut remaining = text;

    while let Some(start) = remaining.find("```json") {
        let after_marker = &remaining[start + 7..];
        // Skip optional newline after ```json
        let content_start = usize::from(after_marker.starts_with('\n'));

        if let Some(end) = after_marker[content_start..].find("```") {
            let json_content = &after_marker[content_start..content_start + end];
            blocks.push(json_content.trim().to_string());
            remaining = &after_marker[content_start + end + 3..];
        } else {
            // Incomplete block - try to salvage
            let json_content = after_marker[content_start..].trim();
            if !json_content.is_empty() {
                blocks.push(json_content.to_string());
            }
            break;
        }
    }

    blocks
}

/// Find the index of the matching closing brace.
fn find_matching_brace(s: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;

    for (i, c) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }

        match c {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }

    None
}

/// Truncate string for logging.
fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_blocks() {
        let text = r#"I'll help you with that.

```json
{"tool": "codecoder", "args": {"agent": "general", "prompt": "query weather"}}
```

Let me check."#;

        let blocks = extract_json_blocks(text);
        assert_eq!(blocks.len(), 1);
        assert!(blocks[0].contains("codecoder"));
    }

    #[test]
    fn test_extract_multiple_json_blocks() {
        let text = r#"
```json
{"tool": "memory_recall", "args": {"query": "user preferences"}}
```

Now let me also check:

```json
{"tool": "codecoder", "args": {"agent": "general", "prompt": "hello"}}
```
"#;

        let blocks = extract_json_blocks(text);
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn test_find_matching_brace() {
        let s = r#"{"tool": "test", "args": {"nested": "value"}}"#;
        assert_eq!(find_matching_brace(s), Some(s.len() - 1));
    }

    #[test]
    fn test_find_matching_brace_with_string() {
        let s = r#"{"text": "hello } world"}"#;
        assert_eq!(find_matching_brace(s), Some(s.len() - 1));
    }

    #[test]
    fn test_parse_tool_call() {
        let json = r#"{"tool": "codecoder", "args": {"agent": "general", "prompt": "test"}}"#;
        let call: ToolCall = serde_json::from_str(json).unwrap();
        assert_eq!(call.tool, "codecoder");
    }

    #[test]
    fn test_parse_tool_calls_from_json_block() {
        let response = r#"I'll help with that.

```json
{"tool": "shell", "args": {"command": "ls"}}
```

Let me run this."#;

        let calls = AgentExecutor::parse_tool_calls(response);
        assert!(calls.is_some());
        let calls = calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool, "shell");
    }

    #[test]
    fn test_parse_inline_tool_call() {
        let response = r#"Let me check: {"tool": "file_read", "args": {"path": "README.md"}}"#;

        let calls = AgentExecutor::parse_tool_calls(response);
        assert!(calls.is_some());
        let calls = calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].tool, "file_read");
    }

    #[test]
    fn test_extract_final_response_clean() {
        let response = "Here is the result. Everything looks good!";
        let final_resp = AgentExecutor::extract_final_response(response);
        assert_eq!(final_resp, response);
    }

    #[test]
    fn test_extract_final_response_with_json() {
        let response = r#"I found the info:

```json
{"tool": "search", "args": {}}
```

The answer is 42."#;

        let final_resp = AgentExecutor::extract_final_response(response);
        assert!(final_resp.contains("I found the info"));
        assert!(final_resp.contains("The answer is 42"));
        assert!(!final_resp.contains("```json"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 5), "hello...");
    }

    #[test]
    fn test_tool_risk_levels() {
        assert_eq!(ToolRisk::from("read"), ToolRisk::Safe);
        assert_eq!(ToolRisk::from("glob"), ToolRisk::Safe);
        assert_eq!(ToolRisk::from("edit"), ToolRisk::Medium);
        assert_eq!(ToolRisk::from("bash"), ToolRisk::High);
        assert_eq!(ToolRisk::from("unknown_tool"), ToolRisk::High);
    }

    #[test]
    fn test_tool_risk_comparison() {
        assert!(!ToolRisk::Safe.exceeds(RiskThreshold::Low));
        assert!(!ToolRisk::Low.exceeds(RiskThreshold::Low));
        assert!(ToolRisk::Medium.exceeds(RiskThreshold::Low));
        assert!(ToolRisk::High.exceeds(RiskThreshold::Medium));
    }
}

