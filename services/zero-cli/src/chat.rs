//! Interactive Chat Command
//!
//! Provides a terminal-based interactive chat with full agent capabilities:
//! - Streaming responses
//! - Tool execution (grep, edit, bash, etc.)
//! - Session persistence
//! - Memory integration
//!
//! This is the Rust-native chat implementation that doesn't require Node.js.

use anyhow::Result;
use futures_util::StreamExt;
use std::io::{stdout, Write};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use zero_core::agent::{
    AnthropicProvider, ContentPart, GoogleProvider, Message, OpenAIProvider, Role, StreamEvent,
    StreamRequest, StreamingProvider, ToolDef,
};

use crate::config::Config;
use crate::memory::Memory;
use crate::security::SecurityPolicy;
use crate::session::store::SessionStore;
use crate::tools::ToolRegistry;

/// Chat session configuration
#[derive(Debug, Clone)]
pub struct ChatConfig {
    /// Model to use (e.g., "claude-sonnet-4-5-20250514")
    pub model: String,
    /// Temperature for generation (0.0 - 1.0)
    pub temperature: f64,
    /// Maximum tokens per response
    pub max_tokens: Option<usize>,
    /// Session ID for persistence
    pub session_id: Option<String>,
    /// Whether to show tool execution details
    pub verbose: bool,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            model: "claude-sonnet-4-5-20250514".to_string(),
            temperature: 0.7,
            max_tokens: Some(8192),
            session_id: None,
            verbose: false,
        }
    }
}

/// Run interactive chat session
pub async fn run_chat(config: Config, chat_config: ChatConfig) -> Result<()> {
    // ══════════════════════════════════════════════════════════════════════════════
    // Initialize components
    // ══════════════════════════════════════════════════════════════════════════════

    // Security policy
    let security = Arc::new(SecurityPolicy::from_config(
        &config.autonomy,
        &config.workspace_dir,
    ));

    // Memory
    let mem: Arc<dyn Memory> = Arc::from(crate::memory::create_memory(
        &config.memory,
        &config.workspace_dir,
        config.api_key.as_deref(),
    )?);

    // Tool registry
    let vault_path = config
        .config_path
        .parent()
        .map_or_else(|| config.workspace_dir.clone(), std::path::Path::to_path_buf);

    let registry = ToolRegistry::with_native_tools(
        &security,
        mem.clone(),
        &config.browser,
        &config.codecoder,
        &config.vault,
        &vault_path,
    );

    // Connect MCP servers if configured
    if !config.mcp.servers.is_empty() {
        if let Err(e) = registry.connect_mcp_servers(&config.mcp).await {
            eprintln!("Warning: Failed to connect some MCP servers: {e}");
        }
    }

    let registry = Arc::new(RwLock::new(registry));

    // Session store
    let db_path = config.workspace_dir.join("sessions.db");
    let sessions = SessionStore::new(&db_path).unwrap_or_else(|_| {
        let temp_db = std::env::temp_dir().join("codecoder_chat_sessions.db");
        SessionStore::new(&temp_db).expect("Temp store should work")
    });

    // Create or resume session
    let session_id = chat_config.session_id.clone().unwrap_or_else(|| {
        format!("chat-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S"))
    });

    // Session is created implicitly when first message is added

    // LLM Provider
    let api_key = config
        .api_key
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("API key required. Set via ANTHROPIC_API_KEY env or config"))?;

    let provider: Arc<dyn StreamingProvider> = create_provider(&config.default_provider, api_key);

    // System prompt (load from prompts dir if available)
    let system_prompt = build_system_prompt(&config.workspace_dir);

    // ══════════════════════════════════════════════════════════════════════════════
    // Print welcome banner
    // ══════════════════════════════════════════════════════════════════════════════

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║  🧠 CodeCoder Interactive Chat                               ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Model: {:<52} ║", &chat_config.model[..chat_config.model.len().min(52)]);
    println!("║  Session: {:<50} ║", &session_id[..session_id.len().min(50)]);
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Commands:                                                   ║");
    println!("║    /quit, /exit  - Exit chat                                 ║");
    println!("║    /clear        - Clear conversation                        ║");
    println!("║    /model <name> - Switch model                              ║");
    println!("║    /verbose      - Toggle verbose mode                       ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // ══════════════════════════════════════════════════════════════════════════════
    // Main chat loop
    // ══════════════════════════════════════════════════════════════════════════════

    let mut messages: Vec<Message> = Vec::new();
    let mut verbose = chat_config.verbose;
    let mut model = chat_config.model.clone();

    loop {
        // Print prompt
        print!("\n\x1b[36m❯\x1b[0m ");
        stdout().flush()?;

        // Read user input
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }

        // Handle commands
        if input.starts_with('/') {
            match input.split_whitespace().collect::<Vec<_>>().as_slice() {
                ["/quit"] | ["/exit"] => {
                    println!("\nGoodbye! 👋\n");
                    break;
                }
                ["/clear"] => {
                    messages.clear();
                    println!("✓ Conversation cleared");
                    continue;
                }
                ["/model", name] => {
                    model = name.to_string();
                    println!("✓ Switched to model: {}", model);
                    continue;
                }
                ["/verbose"] => {
                    verbose = !verbose;
                    println!("✓ Verbose mode: {}", if verbose { "ON" } else { "OFF" });
                    continue;
                }
                _ => {
                    println!("Unknown command. Try /quit, /clear, /model <name>, /verbose");
                    continue;
                }
            }
        }

        // Add user message
        messages.push(Message::user(input));

        // Build tools list
        let tools: Vec<ToolDef> = {
            let reg = registry.read().await;
            reg.native_tools()
                .iter()
                .map(|t| ToolDef {
                    name: t.name().to_string(),
                    description: t.description().to_string(),
                    input_schema: t.parameters_schema(),
                })
                .collect()
        };

        // Build request
        let request = StreamRequest {
            system: vec![system_prompt.clone()],
            messages: messages.clone(),
            tools,
            model: model.clone(),
            temperature: Some(chat_config.temperature),
            max_tokens: chat_config.max_tokens,
        };

        // Execute agent loop
        let result = execute_agent_loop(
            provider.clone(),
            registry.clone(),
            request,
            &mut messages,
            verbose,
        )
        .await;

        match result {
            Ok(response_text) => {
                // Store in session
                let _ = sessions.add_message(
                    &session_id,
                    crate::session::types::MessageRole::User,
                    input,
                );
                let _ = sessions.add_message(
                    &session_id,
                    crate::session::types::MessageRole::Assistant,
                    &response_text,
                );
            }
            Err(e) => {
                println!("\n\x1b[31m❌ Error: {}\x1b[0m\n", e);
                // Remove last user message on error
                messages.pop();
            }
        }
    }

    Ok(())
}

/// Execute the agent loop with tool calling
async fn execute_agent_loop(
    provider: Arc<dyn StreamingProvider>,
    registry: Arc<RwLock<ToolRegistry>>,
    mut request: StreamRequest,
    messages: &mut Vec<Message>,
    verbose: bool,
) -> Result<String> {
    let max_iterations = 10;
    let tool_timeout = Duration::from_secs(30);
    let mut iterations = 0;
    let mut full_response = String::new();

    loop {
        iterations += 1;
        if iterations > max_iterations {
            return Err(anyhow::anyhow!("Max iterations exceeded"));
        }

        // Start stream
        let mut event_stream = provider.stream(request.clone()).await?;

        let mut iteration_text = String::new();
        let mut tool_calls: Vec<(String, String, serde_json::Value)> = vec![];
        let mut current_tool_id = String::new();
        let mut current_tool_name = String::new();
        let mut current_tool_args = String::new();
        let mut first_text = true;

        // Process stream
        while let Some(result) = event_stream.next().await {
            match result {
                Ok(event) => match event {
                    StreamEvent::Start => {}

                    StreamEvent::TextDelta { content } => {
                        if first_text {
                            print!("\n\x1b[32m◆\x1b[0m ");
                            first_text = false;
                        }
                        print!("{}", content);
                        stdout().flush()?;
                        iteration_text.push_str(&content);
                    }

                    StreamEvent::ReasoningDelta { content } => {
                        if verbose {
                            print!("\x1b[90m{}\x1b[0m", content);
                            stdout().flush()?;
                        }
                    }

                    StreamEvent::ToolCallStart { id, name } => {
                        current_tool_id = id;
                        current_tool_name = name;
                        current_tool_args.clear();
                    }

                    StreamEvent::ToolCallDelta { arguments_delta, .. } => {
                        current_tool_args.push_str(&arguments_delta);
                    }

                    StreamEvent::ToolCall { id, name, arguments } => {
                        tool_calls.push((id, name, arguments));
                    }

                    StreamEvent::Finish { .. } => {
                        // Finalize accumulated tool call
                        if !current_tool_id.is_empty() && !current_tool_name.is_empty() {
                            let arguments: serde_json::Value =
                                serde_json::from_str(&current_tool_args).unwrap_or_default();
                            tool_calls.push((
                                current_tool_id.clone(),
                                current_tool_name.clone(),
                                arguments,
                            ));
                        }
                        break;
                    }

                    StreamEvent::Error { message, .. } => {
                        return Err(anyhow::anyhow!("Stream error: {}", message));
                    }
                },
                Err(e) => {
                    tracing::warn!("Stream error: {}", e);
                }
            }
        }

        // Accumulate text
        if !iteration_text.is_empty() {
            println!();
        }
        full_response.push_str(&iteration_text);

        // If no tool calls, we're done
        if tool_calls.is_empty() {
            // Add assistant message
            messages.push(Message::assistant(&full_response));
            break;
        }

        // Add assistant message with tool uses
        let mut assistant_content = Vec::new();
        if !iteration_text.is_empty() {
            assistant_content.push(ContentPart::Text { text: iteration_text });
        }
        for (id, name, input) in &tool_calls {
            assistant_content.push(ContentPart::ToolUse {
                id: id.clone(),
                name: name.clone(),
                input: input.clone(),
            });
        }
        request.messages.push(Message {
            role: Role::Assistant,
            content: assistant_content,
        });

        // Execute tools
        for (tool_call_id, tool_name, arguments) in tool_calls {
            // Print tool execution
            print!("\n\x1b[33m⚙ {}...\x1b[0m", tool_name);
            stdout().flush()?;

            let result = {
                let reg = registry.read().await;
                if let Some(tool) = reg.get_tool(&tool_name).await {
                    match tokio::time::timeout(tool_timeout, tool.execute(arguments)).await {
                        Ok(Ok(r)) => {
                            if r.success {
                                (Some(r.output), None)
                            } else {
                                (None, Some(r.error.unwrap_or_default()))
                            }
                        }
                        Ok(Err(e)) => (None, Some(e.to_string())),
                        Err(_) => (None, Some("Tool timeout".to_string())),
                    }
                } else {
                    (None, Some(format!("Unknown tool: {}", tool_name)))
                }
            };

            // Print result status
            if result.0.is_some() {
                print!(" \x1b[32m✓\x1b[0m");
            } else {
                print!(" \x1b[31m✗ {}\x1b[0m", result.1.as_deref().unwrap_or("failed"));
            }
            println!();

            // Show output in verbose mode
            if verbose {
                if let Some(ref output) = result.0 {
                    let preview = if output.len() > 200 {
                        format!("{}...", &output[..200])
                    } else {
                        output.clone()
                    };
                    println!("\x1b[90m   {}\x1b[0m", preview);
                }
            }

            // Add tool result to messages
            let result_str = result
                .0
                .unwrap_or_else(|| format!("Error: {}", result.1.unwrap_or_default()));
            request.messages.push(Message::tool_result(&tool_call_id, &result_str));
        }
    }

    Ok(full_response)
}

/// Create streaming provider based on config
fn create_provider(provider_name: &Option<String>, api_key: &str) -> Arc<dyn StreamingProvider> {
    let provider_id = provider_name
        .as_ref()
        .map(|s| s.to_lowercase())
        .unwrap_or_else(|| detect_provider(api_key));

    match provider_id.as_str() {
        "openai" | "openai-compatible" => Arc::new(OpenAIProvider::new(api_key)),
        "google" | "gemini" => Arc::new(GoogleProvider::new(api_key)),
        _ => Arc::new(AnthropicProvider::new(api_key)),
    }
}

/// Detect provider from API key format
fn detect_provider(api_key: &str) -> String {
    if api_key.starts_with("sk-ant-") || api_key.starts_with("sk-proj-") {
        "anthropic".to_string()
    } else if api_key.starts_with("sk-") {
        "openai".to_string()
    } else if api_key.starts_with("AIza") {
        "google".to_string()
    } else {
        "anthropic".to_string()
    }
}

/// Build system prompt from workspace
fn build_system_prompt(workspace: &std::path::Path) -> String {
    // Try to load from CLAUDE.md
    let claude_md = workspace.join("CLAUDE.md");
    if claude_md.exists() {
        if let Ok(content) = std::fs::read_to_string(&claude_md) {
            return format!(
                "You are CodeCoder, an AI coding assistant. Follow these guidelines:\n\n{}",
                content
            );
        }
    }

    // Default system prompt
    r#"You are CodeCoder, an advanced AI coding assistant with access to powerful tools.

## Capabilities
- Read, write, and edit files
- Search code with grep and glob patterns
- Execute shell commands
- Browse the web for documentation
- Access memory for context

## Guidelines
- Be concise and precise
- Explain your reasoning before acting
- Use tools proactively to gather context
- Ask for clarification when requirements are ambiguous
- Follow project conventions discovered in CLAUDE.md or README.md

## Tool Usage
- Use `grep` to search for patterns in code
- Use `glob` to find files by name
- Use `read` to examine file contents
- Use `edit` for surgical code changes
- Use `bash` for shell commands (be careful with destructive operations)
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_provider() {
        assert_eq!(detect_provider("sk-ant-api03-xxx"), "anthropic");
        assert_eq!(detect_provider("sk-proj-xxx"), "anthropic");
        assert_eq!(detect_provider("sk-xxx"), "openai");
        assert_eq!(detect_provider("AIzaSy-xxx"), "google");
        assert_eq!(detect_provider("unknown-key"), "anthropic");
    }

    #[test]
    fn test_chat_config_default() {
        let config = ChatConfig::default();
        assert!(config.model.contains("claude"));
        assert!(config.temperature > 0.0);
        assert!(config.max_tokens.is_some());
    }
}
