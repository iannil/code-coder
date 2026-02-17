use crate::agent::confirmation::ToolContext;
use crate::providers::Provider;
use crate::tools::{Tool, ToolResult};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use std::sync::Arc;

/// Maximum number of tool-calling iterations to prevent infinite loops
const MAX_ITERATIONS: usize = 10;

/// Parsed tool call from LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub tool: String,
    pub args: serde_json::Value,
}

/// Agent executor that handles the tool-calling loop
pub struct AgentExecutor {
    provider: Arc<dyn Provider>,
    tools: Vec<Box<dyn Tool>>,
    system_prompt: String,
    model: String,
    temperature: f64,
}

impl AgentExecutor {
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

    /// Execute a message through the agentic loop
    /// Returns the final text response after all tool calls are resolved
    pub async fn execute(&self, user_message: &str) -> Result<String> {
        self.execute_with_context(user_message, None).await
    }

    /// Execute with optional tool context for interactive operations
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

            tracing::debug!("LLM response (iteration {iterations}): {}", truncate(&response, 200));

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

                    match self.execute_tool(&enriched_call, context.as_ref()).await {
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

    /// Parse tool calls from LLM response
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

    /// Find inline tool call JSON in response
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

    /// Execute a single tool call
    async fn execute_tool(&self, call: &ToolCall, _context: Option<&ToolContext>) -> Result<ToolResult> {
        // Find the tool by name
        let tool = self
            .tools
            .iter()
            .find(|t| t.name() == call.tool)
            .ok_or_else(|| anyhow::anyhow!("Unknown tool: {}", call.tool))?;

        tool.execute(call.args.clone()).await
    }

    /// Extract final response text, removing any JSON artifacts or tool call attempts
    fn extract_final_response(response: &str) -> String {
        let mut result = response.to_string();

        // Remove JSON code blocks
        while let Some(start) = result.find("```json") {
            if let Some(end) = result[start..].find("```\n").or_else(|| result[start..].find("```")) {
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

    /// Build tool descriptions for the system prompt
    pub fn build_tool_prompt(&self) -> String {
        let mut prompt = String::from("## Available Tools\n\n");
        prompt.push_str("To use a tool, respond with a JSON block:\n");
        prompt.push_str("```json\n{\"tool\": \"tool_name\", \"args\": {\"param\": \"value\"}}\n```\n\n");
        prompt.push_str("After tool execution, you'll receive the results. Continue using tools or provide a final text response.\n\n");
        prompt.push_str("### Tools:\n\n");

        for tool in &self.tools {
            let _ = writeln!(prompt, "**{}**: {}", tool.name(), tool.description());
            let _ = writeln!(prompt, "Parameters: {}\n", tool.parameters_schema());
        }

        prompt
    }
}

/// Extract JSON blocks from markdown code fences
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

/// Find the index of the matching closing brace
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

/// Truncate string for logging
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
}
