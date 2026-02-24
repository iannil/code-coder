//! CodeCoder bridge for zero-channels.
//!
//! Handles the complete message flow:
//! 1. Receive ChannelMessage from webhook/polling
//! 2. Detect message intent (chat vs feasibility assessment)
//! 3. Forward to appropriate CodeCoder API
//! 4. Route response back to original channel

use crate::capture_bridge::CaptureBridge;
use crate::message::{ChannelMessage, MessageContent, OutgoingContent};
use crate::outbound::OutboundRouter;
use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use zero_common::config::CaptureConfig;

// ============================================================================
// CodeCoder API Types
// ============================================================================

/// Request to CodeCoder chat API.
#[derive(Debug, Clone, Serialize)]
pub struct ChatRequest {
    /// User message content
    pub message: String,
    /// Optional conversation ID for context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
    /// Optional agent to use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    /// User identifier
    pub user_id: String,
    /// Channel type for context
    pub channel: String,
}

/// Response from CodeCoder chat API (wrapped).
#[derive(Debug, Clone, Deserialize)]
pub struct ChatApiResponse {
    pub success: bool,
    pub data: Option<ChatResponseData>,
    pub error: Option<String>,
}

/// Chat response data (inner payload).
#[derive(Debug, Clone, Deserialize)]
pub struct ChatResponseData {
    /// Response message content
    pub message: String,
    /// Conversation ID for follow-ups
    pub conversation_id: Option<String>,
    /// Agent used
    pub agent: Option<String>,
    /// Token usage information
    pub usage: Option<TokenUsage>,
}

/// Token usage from LLM.
#[derive(Debug, Clone, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    #[serde(default)]
    pub total_tokens: i64,
}

/// Error response from CodeCoder.
#[derive(Debug, Clone, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(default)]
    pub code: Option<String>,
}

// ============================================================================
// Feasibility Assessment Types
// ============================================================================

/// Request to CodeCoder feasibility assessment API.
#[derive(Debug, Clone, Serialize)]
pub struct FeasibilityRequest {
    /// Natural language query describing the feature
    pub query: String,
    /// Analysis options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<FeasibilityOptions>,
}

/// Options for feasibility assessment.
#[derive(Debug, Clone, Serialize)]
pub struct FeasibilityOptions {
    /// Analysis depth
    pub depth: String,
    /// Include code references
    pub include_code_refs: bool,
    /// Response language
    pub language: String,
}

/// Response from CodeCoder feasibility assessment API.
#[derive(Debug, Clone, Deserialize)]
pub struct FeasibilityResponse {
    pub success: bool,
    pub data: Option<FeasibilityData>,
    pub error: Option<String>,
}

// ============================================================================
// A/B Test / Multi-Model Comparison Types
// ============================================================================

/// Request to CodeCoder compare API.
#[derive(Debug, Clone, Serialize)]
pub struct CompareRequest {
    /// Models to query (e.g., ["anthropic/claude-sonnet-4", "openai/gpt-4o"])
    pub models: Vec<String>,
    /// The prompt to send to all models
    pub prompt: String,
    /// Optional system prompt
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,
    /// Temperature (0.0 - 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
}

/// Response from CodeCoder compare API.
#[derive(Debug, Clone, Deserialize)]
pub struct CompareResponse {
    pub success: bool,
    pub data: Option<CompareData>,
    pub error: Option<String>,
}

/// Compare result data.
#[derive(Debug, Clone, Deserialize)]
pub struct CompareData {
    /// Results from each model
    pub results: Vec<ModelResult>,
    /// Total tokens used across all models
    pub total_tokens: i64,
    /// Total latency in milliseconds (max of all models)
    pub total_latency_ms: u64,
}

/// Result from a single model.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelResult {
    /// Full model identifier (provider/model)
    pub model: String,
    /// Provider name
    pub provider: String,
    /// Model ID within provider
    pub model_id: String,
    /// Response content
    pub content: String,
    /// Token usage
    pub tokens: ModelTokenInfo,
    /// Response latency in milliseconds
    pub latency_ms: u64,
    /// Error message if failed
    pub error: Option<String>,
}

/// Token info for a model result.
#[derive(Debug, Clone, Deserialize)]
pub struct ModelTokenInfo {
    pub input: i64,
    pub output: i64,
    pub total: i64,
}

// ============================================================================
// Knowledge Base Types
// ============================================================================

/// Request to CodeCoder knowledge search API.
#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeSearchRequest {
    /// Natural language search query
    pub query: String,
    /// Maximum number of results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i32>,
    /// Minimum relevance score (0-1)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_score: Option<f64>,
}

/// Response from CodeCoder knowledge search API.
#[derive(Debug, Clone, Deserialize)]
pub struct KnowledgeSearchResponse {
    pub success: bool,
    pub data: Option<KnowledgeSearchData>,
    pub error: Option<String>,
}

/// Knowledge search result data.
#[derive(Debug, Clone, Deserialize)]
pub struct KnowledgeSearchData {
    pub results: Vec<KnowledgeResult>,
    pub total: i32,
    pub query: String,
    pub search_mode: String,
}

/// A single knowledge search result.
#[derive(Debug, Clone, Deserialize)]
pub struct KnowledgeResult {
    pub content: String,
    pub score: f64,
    pub document_id: String,
    pub chunk_index: i32,
    pub filename: String,
    pub heading: Option<String>,
}

/// Feasibility assessment data.
#[derive(Debug, Clone, Deserialize)]
pub struct FeasibilityData {
    pub summary: String,
    pub complexity: String,
    pub analysis: FeasibilityAnalysis,
    pub confidence: f64,
    pub tokens_used: Option<i64>,
}

// ============================================================================
// Bug Report / Feature Request Types
// ============================================================================

/// Information about a detected bug report.
#[derive(Debug, Clone)]
pub struct BugReportInfo {
    /// Bug category (crash, error, functionality, display, data, broken)
    pub category: String,
    /// Original content
    pub content: String,
}

/// Information about a detected feature request.
#[derive(Debug, Clone)]
pub struct FeatureRequestInfo {
    /// Original content
    pub content: String,
}

/// Detailed feasibility analysis.
#[derive(Debug, Clone, Deserialize)]
pub struct FeasibilityAnalysis {
    pub complexity: String,
    pub summary: String,
    pub existing_capabilities: Vec<ExistingCapability>,
    pub required_changes: Vec<RequiredChange>,
    pub dependencies: Vec<Dependency>,
    pub risks: Vec<String>,
    pub confidence: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExistingCapability {
    pub name: String,
    pub path: String,
    pub relevance: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RequiredChange {
    pub file: String,
    pub action: String,
    pub description: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Dependency {
    pub name: String,
    #[serde(rename = "type")]
    pub dep_type: String,
    pub reason: String,
}

// ============================================================================
// Bridge
// ============================================================================

/// Bridge between IM channels and CodeCoder.
pub struct CodeCoderBridge {
    /// HTTP client for API calls
    client: reqwest::Client,
    /// CodeCoder API endpoint
    endpoint: String,
    /// Outbound router for sending responses
    router: Arc<OutboundRouter>,
    /// Request timeout
    timeout: Duration,
    /// Asset capture bridge (optional)
    capture_bridge: Option<Arc<CaptureBridge>>,
}

impl CodeCoderBridge {
    /// Create a new bridge.
    pub fn new(endpoint: impl Into<String>, router: Arc<OutboundRouter>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300)) // LLM calls can be slow
            .connect_timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            client,
            endpoint: endpoint.into(),
            router,
            timeout: Duration::from_secs(300),
            capture_bridge: None,
        }
    }

    /// Set the request timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the capture bridge for asset capture functionality.
    pub fn with_capture(mut self, config: CaptureConfig) -> Self {
        if config.enabled {
            let endpoint = self.endpoint.clone();
            self.capture_bridge = Some(Arc::new(CaptureBridge::new(config, endpoint)));
        }
        self
    }

    /// Get the capture bridge (if configured).
    pub fn capture_bridge(&self) -> Option<Arc<CaptureBridge>> {
        self.capture_bridge.clone()
    }

    /// Process an incoming message and send response.
    ///
    /// This is the main entry point for the bridge.
    /// Routes messages based on intent:
    /// - Feasibility questions -> /api/v1/assess/feasibility
    /// - General chat -> /api/v1/chat
    pub async fn process(&self, message: ChannelMessage) -> Result<()> {
        // Register the message for response routing
        self.router.register_pending(message.clone()).await;

        // Extract text content
        let text = match &message.content {
            MessageContent::Text { text } => text.clone(),
            MessageContent::Voice { .. } => {
                // Voice should have been transcribed before reaching here
                return Err(anyhow::anyhow!("Voice messages should be transcribed first"));
            }
            MessageContent::Image { caption, .. } => {
                caption.clone().unwrap_or_else(|| "[Image received]".to_string())
            }
            MessageContent::File { filename, .. } => {
                format!("[File received: {}]", filename)
            }
            MessageContent::Location { latitude, longitude, title } => {
                format!(
                    "[Location: {} at {}, {}]",
                    title.as_deref().unwrap_or("Unknown"),
                    latitude,
                    longitude
                )
            }
        };

        // Check if this is a capture request (highest priority for capture-related messages)
        if let Some(ref capture_bridge) = self.capture_bridge {
            if capture_bridge.is_capturable(&message) && capture_bridge.is_capture_request(&message)
            {
                tracing::info!(
                    message_id = %message.id,
                    "Detected capture request, processing asset capture"
                );

                match capture_bridge.capture(&message).await {
                    Ok(asset) => {
                        let formatted = capture_bridge.format_capture_response(&asset);
                        let content = OutgoingContent::Markdown { text: formatted };
                        let result = self.router.respond(&message.id, content).await;

                        if !result.success {
                            tracing::error!(
                                message_id = %message.id,
                                error = ?result.error,
                                "Failed to send capture response"
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "Capture processing failed");

                        let error_content = OutgoingContent::Text {
                            text: format!("内容捕获失败: {}", e),
                        };
                        let _ = self.router.respond(&message.id, error_content).await;
                    }
                }

                return Ok(());
            }
        }

        // Check if this is a feasibility question
        if Self::is_feasibility_question(&text) {
            tracing::info!(
                message_id = %message.id,
                "Detected feasibility question, routing to assessment API"
            );

            match self.call_feasibility(&text).await {
                Ok(resp) if resp.success && resp.data.is_some() => {
                    let data = resp.data.unwrap();
                    let formatted = Self::format_feasibility_response(&data);
                    let content = OutgoingContent::Markdown { text: formatted };
                    let result = self.router.respond(&message.id, content).await;

                    if !result.success {
                        tracing::error!(
                            message_id = %message.id,
                            error = ?result.error,
                            "Failed to send feasibility response"
                        );
                    }
                }
                Ok(resp) => {
                    // API returned but with error
                    let error_msg = resp.error.unwrap_or_else(|| "Unknown error".to_string());
                    tracing::error!(error = %error_msg, "Feasibility API returned error");

                    let error_content = OutgoingContent::Text {
                        text: format!("技术评估失败: {}", error_msg),
                    };
                    let _ = self.router.respond(&message.id, error_content).await;
                }
                Err(e) => {
                    tracing::error!(error = %e, "Feasibility API call failed");

                    // Fallback to regular chat
                    tracing::info!("Falling back to regular chat API");
                    self.process_chat_with_agent(&message, &text, None).await?;
                }
            }

            return Ok(());
        }

        // Check if this is an A/B test request
        if let Some((models, prompt)) = Self::is_ab_test_request(&text) {
            tracing::info!(
                message_id = %message.id,
                models = ?models,
                "Detected A/B test request, routing to compare API"
            );

            match self.call_compare(&models, &prompt).await {
                Ok(resp) if resp.success && resp.data.is_some() => {
                    let data = resp.data.unwrap();
                    let formatted = Self::format_compare_response(&data);
                    let content = OutgoingContent::Markdown { text: formatted };
                    let result = self.router.respond(&message.id, content).await;

                    if !result.success {
                        tracing::error!(
                            message_id = %message.id,
                            error = ?result.error,
                            "Failed to send compare response"
                        );
                    }
                }
                Ok(resp) => {
                    let error_msg = resp.error.unwrap_or_else(|| "Unknown error".to_string());
                    tracing::error!(error = %error_msg, "Compare API returned error");

                    let error_content = OutgoingContent::Text {
                        text: format!("模型对比失败: {}", error_msg),
                    };
                    let _ = self.router.respond(&message.id, error_content).await;
                }
                Err(e) => {
                    tracing::error!(error = %e, "Compare API call failed");

                    // Fallback to regular chat
                    tracing::info!("Falling back to regular chat API");
                    self.process_chat_with_agent(&message, &text, None).await?;
                }
            }

            return Ok(());
        }

        // Check if this is a knowledge base query
        if let Some(query) = Self::is_knowledge_question(&text) {
            tracing::info!(
                message_id = %message.id,
                query = %query,
                "Detected knowledge question, routing to knowledge search API"
            );

            match self.call_knowledge_search(&query).await {
                Ok(resp) if resp.success && resp.data.is_some() => {
                    let data = resp.data.unwrap();
                    let formatted = Self::format_knowledge_response(&data);
                    let content = OutgoingContent::Markdown { text: formatted };
                    let result = self.router.respond(&message.id, content).await;

                    if !result.success {
                        tracing::error!(
                            message_id = %message.id,
                            error = ?result.error,
                            "Failed to send knowledge search response"
                        );
                    }
                }
                Ok(resp) => {
                    let error_msg = resp.error.unwrap_or_else(|| "Unknown error".to_string());
                    tracing::error!(error = %error_msg, "Knowledge API returned error");

                    let error_content = OutgoingContent::Text {
                        text: format!("知识库搜索失败: {}", error_msg),
                    };
                    let _ = self.router.respond(&message.id, error_content).await;
                }
                Err(e) => {
                    tracing::error!(error = %e, "Knowledge API call failed");

                    // Fallback to regular chat
                    tracing::info!("Falling back to regular chat API");
                    self.process_chat_with_agent(&message, &text, None).await?;
                }
            }

            return Ok(());
        }

        // Check if this is a help request for available agents
        if Self::is_agent_help_request(&text) {
            tracing::info!(
                message_id = %message.id,
                "Detected agent help request, returning agent list"
            );

            let help_text = Self::format_agent_help();
            let content = OutgoingContent::Markdown { text: help_text };
            let result = self.router.respond(&message.id, content).await;

            if !result.success {
                tracing::error!(
                    message_id = %message.id,
                    error = ?result.error,
                    "Failed to send agent help response"
                );
            }

            return Ok(());
        }

        // Check if this is an agent command (@agent_name)
        if let Some((agent, prompt)) = Self::parse_agent_command(&text) {
            tracing::info!(
                message_id = %message.id,
                agent = %agent,
                "Detected agent command, routing to specific agent"
            );

            return self.process_chat_with_agent(&message, &prompt, Some(agent)).await;
        }

        // Regular chat processing
        self.process_chat_with_agent(&message, &text, None).await
    }

    /// Check if this is a request for agent help.
    fn is_agent_help_request(content: &str) -> bool {
        let content = content.trim().to_lowercase();
        matches!(
            content.as_str(),
            "@help" | "@?" | "@帮助" | "@agents" | "help agents" | "list agents"
        )
    }

    /// Format the agent help message for IM channels.
    fn format_agent_help() -> String {
        let lines = vec![
            "🤖 **可用的 Agent 列表**",
            "",
            "**祝融说系列 (ZRS)**",
            "• `@macro` - 宏观经济分析（PMI、GDP等数据解读）",
            "• `@decision` - CLOSE决策框架（五维评估分析）",
            "• `@trader` - 超短线交易指南（情绪周期、模式识别）",
            "• `@observer` - 观察者理论（可能性基底分析）",
            "• `@picker` - 选品专家（爆品方法论）",
            "• `@miniproduct` - 极小产品教练（MVP开发）",
            "• `@ai-engineer` - AI工程师导师",
            "",
            "**工程质量**",
            "• `@code-reviewer` - 代码审查",
            "• `@security-reviewer` - 安全审计",
            "• `@tdd-guide` - TDD开发指南",
            "• `@architect` - 系统架构设计",
            "",
            "**内容创作**",
            "• `@writer` - 长文写作（20k+字）",
            "• `@proofreader` - 文本校对",
            "",
            "**逆向工程**",
            "• `@code-reverse` - 网站逆向",
            "• `@jar-code-reverse` - JAR逆向",
            "",
            "**使用方式**: `@agent名称 你的问题`",
            "**示例**: `@macro 解读本月PMI数据`",
        ];

        lines.join("\n")
    }

    /// Parse agent command from message.
    ///
    /// Detects patterns like:
    /// - `@macro 解读PMI数据` → ("macro", "解读PMI数据")
    /// - `@decision 用CLOSE框架分析` → ("decision", "用CLOSE框架分析")
    /// - `@trader 分析今日情绪周期` → ("trader", "分析今日情绪周期")
    ///
    /// Returns Some((agent_name, prompt)) if detected, None otherwise.
    fn parse_agent_command(content: &str) -> Option<(String, String)> {
        // List of known agents that can be invoked via @mention
        // This matches the agents defined in CodeCoder's agent.ts
        const AGENTS: &[&str] = &[
            // Primary modes
            "build",
            "plan",
            "autonomous",
            "writer",
            // Engineering agents
            "code-reviewer",
            "security-reviewer",
            "tdd-guide",
            "architect",
            "explore",
            "general",
            // Content agents
            "proofreader",
            "expander",
            "expander-fiction",
            "expander-nonfiction",
            // Reverse engineering
            "code-reverse",
            "jar-code-reverse",
            // Zhurong series (祝融说)
            "observer",
            "decision",
            "macro",
            "trader",
            "picker",
            "miniproduct",
            "ai-engineer",
            // Tools
            "synton-assistant",
            "verifier",
        ];

        // Pattern: @agent_name <prompt>
        // Support both English and Chinese punctuation
        let content = content.trim();
        if !content.starts_with('@') {
            return None;
        }

        // Extract agent name (everything after @ until whitespace or punctuation)
        let rest = &content[1..];
        let agent_end = rest
            .find(|c: char| c.is_whitespace() || c == '：' || c == ':' || c == ',' || c == '，')
            .unwrap_or(rest.len());

        let agent_name = &rest[..agent_end];

        // Check if it's a known agent (case-insensitive)
        let agent_lower = agent_name.to_lowercase();
        let matched_agent = AGENTS.iter().find(|&&a| a == agent_lower)?;

        // Extract the prompt (everything after the agent name)
        let prompt_start = agent_end;
        let prompt = rest[prompt_start..]
            .trim_start_matches(|c: char| c.is_whitespace() || c == '：' || c == ':')
            .to_string();

        // Don't match if there's no actual prompt
        if prompt.is_empty() {
            return None;
        }

        Some((matched_agent.to_string(), prompt))
    }

    /// Process a chat message with optional explicit agent.
    async fn process_chat_with_agent(
        &self,
        message: &ChannelMessage,
        text: &str,
        agent: Option<String>,
    ) -> Result<()> {
        // Build the request with agent if specified
        let request = ChatRequest {
            message: text.to_string(),
            conversation_id: message.metadata.get("conversation_id").cloned(),
            agent: agent.or_else(|| message.metadata.get("agent").cloned()),
            user_id: message.user_id.clone(),
            channel: message.channel_type.as_str().to_string(),
        };

        // Send to CodeCoder
        let response = self.call_codecoder(&request).await;

        // Route the response
        match response {
            Ok(resp) => {
                let content = OutgoingContent::Markdown { text: resp.message };
                let result = self.router.respond(&message.id, content).await;

                if !result.success {
                    tracing::error!(
                        message_id = %message.id,
                        error = ?result.error,
                        "Failed to send response"
                    );
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "CodeCoder API call failed");

                // Send error message to user
                let error_content = OutgoingContent::Text {
                    text: format!("Sorry, I encountered an error: {}", e),
                };
                let _ = self.router.respond(&message.id, error_content).await;
            }
        }

        Ok(())
    }

    /// Call the CodeCoder API.
    async fn call_codecoder(&self, request: &ChatRequest) -> Result<ChatResponseData> {
        let url = format!("{}/api/v1/chat", self.endpoint);

        tracing::debug!(
            endpoint = %url,
            user_id = %request.user_id,
            "Calling CodeCoder API"
        );

        let response = self
            .client
            .post(&url)
            .json(request)
            .timeout(self.timeout)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();

            // Try to parse as error response
            if let Ok(error) = serde_json::from_str::<ErrorResponse>(&error_text) {
                return Err(anyhow::anyhow!("CodeCoder error: {}", error.error));
            }

            return Err(anyhow::anyhow!(
                "CodeCoder API returned {}: {}",
                status,
                error_text
            ));
        }

        let api_response: ChatApiResponse = response.json().await?;

        // Check if API returned success
        if !api_response.success {
            let error_msg = api_response.error.unwrap_or_else(|| "Unknown error".to_string());
            return Err(anyhow::anyhow!("CodeCoder API error: {}", error_msg));
        }

        // Extract the data payload
        let chat_response = api_response.data.ok_or_else(|| {
            anyhow::anyhow!("CodeCoder API returned success but no data")
        })?;

        tracing::debug!(
            conversation_id = ?chat_response.conversation_id,
            agent = ?chat_response.agent,
            usage = ?chat_response.usage,
            "CodeCoder response received"
        );

        Ok(chat_response)
    }

    /// Check if a message is a feasibility question.
    ///
    /// Detects patterns like:
    /// - "技术复杂度高吗"
    /// - "能实现吗"
    /// - "需要改动多少文件"
    /// - "可行性如何"
    fn is_feasibility_question(content: &str) -> bool {
        // Chinese patterns
        let cn_patterns = [
            r"技术.*复杂.*吗",
            r"复杂度.*[高低].*吗",
            r"能.*实现.*吗",
            r"可以.*实现.*吗",
            r"需要.*改动.*多少",
            r"改动.*[大小多少].*吗",
            r"可行性.*[如何怎样]",
            r"风险.*[高低].*吗",
            r"难度.*[大小高低]",
            r"工作量.*[大小多少]",
            r"评估.*[一下]?.*[技术可行]",
        ];

        // English patterns
        let en_patterns = [
            r"(?i)how\s+complex",
            r"(?i)is\s+it\s+feasible",
            r"(?i)can\s+we\s+implement",
            r"(?i)how\s+difficult",
            r"(?i)technical\s+complexity",
            r"(?i)effort\s+estimate",
            r"(?i)how\s+much\s+work",
        ];

        for pattern in cn_patterns.iter().chain(en_patterns.iter()) {
            if let Ok(regex) = Regex::new(pattern) {
                if regex.is_match(content) {
                    return true;
                }
            }
        }

        false
    }

    /// Check if a message is an A/B test request.
    ///
    /// Detects patterns like:
    /// - "@A/B 生成一篇推文"
    /// - "@对比 写一个产品介绍"
    /// - "@compare create a product description"
    /// - "帮我对比 Claude 和 GPT 的回答"
    ///
    /// Returns Some((models, prompt)) if detected, None otherwise.
    fn is_ab_test_request(content: &str) -> Option<(Vec<String>, String)> {
        // Default models for A/B testing
        let default_models = vec![
            "anthropic/claude-sonnet-4".to_string(),
            "openai/gpt-4o".to_string(),
        ];

        // Pattern: @A/B <prompt>
        if let Some(captures) = Regex::new(r"(?i)@a/?b\s+(.+)")
            .ok()
            .and_then(|r| r.captures(content))
        {
            return Some((default_models, captures[1].to_string()));
        }

        // Pattern: @对比 <prompt> or @比较 <prompt>
        if let Some(captures) = Regex::new(r"@(?:对比|比较)\s+(.+)")
            .ok()
            .and_then(|r| r.captures(content))
        {
            return Some((default_models, captures[1].to_string()));
        }

        // Pattern: @compare <prompt>
        if let Some(captures) = Regex::new(r"(?i)@compare\s+(.+)")
            .ok()
            .and_then(|r| r.captures(content))
        {
            return Some((default_models, captures[1].to_string()));
        }

        // Pattern: 对比一下 Claude 和 GPT <prompt>
        if let Some(captures) = Regex::new(r"(?:对比|比较).*(?:Claude|GPT|模型).*[：:]\s*(.+)")
            .ok()
            .and_then(|r| r.captures(content))
        {
            return Some((default_models, captures[1].to_string()));
        }

        None
    }

    /// Check if a message is a knowledge base question.
    ///
    /// Detects patterns like:
    /// - "帮我查一下..."
    /// - "查询知识库..."
    /// - "@知识库 ..."
    /// - "@knowledge ..."
    /// - "公司文档里有关于...的内容吗"
    ///
    /// Returns Some(query) if detected, None otherwise.
    fn is_knowledge_question(content: &str) -> Option<String> {
        // Pattern: @知识库 <query> or @知识 <query>
        if let Some(captures) = Regex::new(r"@(?:知识库|知识)\s+(.+)")
            .ok()
            .and_then(|r| r.captures(content))
        {
            return Some(captures[1].to_string());
        }

        // Pattern: @knowledge <query> or @kb <query>
        if let Some(captures) = Regex::new(r"(?i)@(?:knowledge|kb)\s+(.+)")
            .ok()
            .and_then(|r| r.captures(content))
        {
            return Some(captures[1].to_string());
        }

        // Pattern: 帮我查一下/查询一下 ...
        if let Some(captures) = Regex::new(r"(?:帮我)?查(?:[询问])?一?下\s*(.+)")
            .ok()
            .and_then(|r| r.captures(content))
        {
            return Some(captures[1].to_string());
        }

        // Pattern: 搜索一下/搜索关于 ...
        if let Some(captures) = Regex::new(r"搜索[一下]*\s*(?:关于)?\s*(.+)")
            .ok()
            .and_then(|r| r.captures(content))
        {
            return Some(captures[1].to_string());
        }

        // Pattern: 文档/知识库里有关于...的内容吗
        if let Some(captures) = Regex::new(r"(?:文档|知识库)里?有关于\s*(.+?)\s*的(?:内容|信息|资料)")
            .ok()
            .and_then(|r| r.captures(content))
        {
            return Some(captures[1].to_string());
        }

        // Pattern: search for <query>
        if let Some(captures) = Regex::new(r"(?i)search\s+(?:for\s+)?(.+)")
            .ok()
            .and_then(|r| r.captures(content))
        {
            // Only match if starts with "search" to avoid false positives
            if content.to_lowercase().starts_with("search") {
                return Some(captures[1].to_string());
            }
        }

        None
    }

    /// Check if a message is a bug report or error feedback.
    ///
    /// Detects patterns like:
    /// - "应用崩溃了"
    /// - "出现白屏"
    /// - "报错了"
    /// - "无法登录"
    /// - "功能不能用"
    ///
    /// Returns Some(BugReportInfo) if detected, None otherwise.
    pub fn is_bug_report(content: &str) -> Option<BugReportInfo> {
        // Chinese bug report patterns
        let cn_patterns = [
            (r"(?i)(bug|崩溃|crash|白屏|黑屏)", "crash"),
            (r"(?i)(报错|error|异常|exception)", "error"),
            (r"(?i)(闪退|卡死|卡住|卡顿)", "crash"),
            (r"(?i)(无法|不能|失败).{0,10}(登录|打开|使用|加载|访问|连接)", "functionality"),
            (r"(?i)(出问题|有问题|坏了|挂了|不工作)", "broken"),
            (r"(?i)(显示.{0,5}(错误|不对|异常))", "display"),
            (r"(?i)(数据.{0,5}(丢失|错误|不对))", "data"),
        ];

        // English bug report patterns
        let en_patterns = [
            (r"(?i)\b(crash|crashes|crashed)\b", "crash"),
            (r"(?i)\b(error|errors|exception)\b", "error"),
            (r"(?i)\b(bug|bugs)\b", "bug"),
            (r"(?i)(not working|doesn't work|broken)", "broken"),
            (r"(?i)(can't|cannot|unable to).{0,15}(login|open|use|load|access)", "functionality"),
            (r"(?i)(white|blank|black)\s*screen", "display"),
        ];

        for (pattern, category) in cn_patterns.iter().chain(en_patterns.iter()) {
            if let Ok(regex) = Regex::new(pattern) {
                if regex.is_match(content) {
                    return Some(BugReportInfo {
                        category: category.to_string(),
                        content: content.to_string(),
                    });
                }
            }
        }

        None
    }

    /// Check if a message is a feature request.
    ///
    /// Detects patterns like:
    /// - "希望能添加..."
    /// - "建议增加..."
    /// - "能不能支持..."
    /// - "feature request"
    ///
    /// Returns Some(FeatureRequestInfo) if detected, None otherwise.
    pub fn is_feature_request(content: &str) -> Option<FeatureRequestInfo> {
        // Chinese feature request patterns
        let cn_patterns = [
            r"(?i)(希望|期望|期待).{0,5}(能够?|可以|添加|增加|支持)",
            r"(?i)(建议|请求|需要).{0,5}(添加|增加|做|实现|支持)",
            r"(?i)(能不能|可不可以|是否可以).{0,10}(添加|增加|做|实现|支持)",
            r"(?i)(功能|特性).{0,5}(请求|需求|建议)",
            r"(?i)(如果能|要是能).{0,10}(就好了|更好)",
        ];

        // English feature request patterns
        let en_patterns = [
            r"(?i)(feature\s*request)",
            r"(?i)(would\s+be\s+nice|would\s+like|wish).{0,15}(to\s+have|if)",
            r"(?i)(can\s+you|could\s+you).{0,10}(add|implement|support)",
            r"(?i)(please\s+add|please\s+implement|please\s+support)",
            r"(?i)(suggestion|request).{0,5}(for|to)",
        ];

        for pattern in cn_patterns.iter().chain(en_patterns.iter()) {
            if let Ok(regex) = Regex::new(pattern) {
                if regex.is_match(content) {
                    return Some(FeatureRequestInfo {
                        content: content.to_string(),
                    });
                }
            }
        }

        None
    }

    /// Call the compare API.
    async fn call_compare(&self, models: &[String], prompt: &str) -> Result<CompareResponse> {
        let url = format!("{}/api/v1/compare", self.endpoint);

        tracing::debug!(
            endpoint = %url,
            models = ?models,
            "Calling CodeCoder compare API"
        );

        let request = CompareRequest {
            models: models.to_vec(),
            prompt: prompt.to_string(),
            system: None,
            max_tokens: Some(4096),
            temperature: Some(0.7),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Compare API returned {}: {}",
                status,
                error_text
            ));
        }

        let compare_response: CompareResponse = response.json().await?;

        tracing::debug!(
            success = compare_response.success,
            result_count = compare_response.data.as_ref().map(|d| d.results.len()),
            "Compare response received"
        );

        Ok(compare_response)
    }

    /// Format compare results for IM channels.
    fn format_compare_response(data: &CompareData) -> String {
        let mut lines = vec![
            "🔄 **多模型对比结果**".to_string(),
            String::new(),
        ];

        for (i, result) in data.results.iter().enumerate() {
            let model_name = result.model_id.split('/').next_back().unwrap_or(&result.model_id);
            let provider_emoji = match result.provider.as_str() {
                "anthropic" => "🟣",
                "openai" => "🟢",
                "google" => "🔵",
                "mistral" => "🟠",
                _ => "⚪",
            };

            lines.push(format!(
                "### {} {} ({}ms)",
                provider_emoji,
                model_name,
                result.latency_ms
            ));

            if let Some(error) = &result.error {
                lines.push(format!("❌ 错误: {}", error));
            } else {
                // Truncate content for IM display
                let content = if result.content.len() > 1000 {
                    format!("{}...\n\n*[内容已截断]*", &result.content[..1000])
                } else {
                    result.content.clone()
                };
                lines.push(content);
            }

            lines.push(format!(
                "_Tokens: {} in / {} out_",
                result.tokens.input, result.tokens.output
            ));

            if i < data.results.len() - 1 {
                lines.push(String::new());
                lines.push("---".to_string());
                lines.push(String::new());
            }
        }

        lines.push(String::new());
        lines.push(format!(
            "📊 **总计**: {} tokens, {}ms",
            data.total_tokens, data.total_latency_ms
        ));

        lines.join("\n")
    }

    /// Call the knowledge search API.
    async fn call_knowledge_search(&self, query: &str) -> Result<KnowledgeSearchResponse> {
        let url = format!("{}/api/v1/knowledge/search", self.endpoint);

        tracing::debug!(
            endpoint = %url,
            query = %query,
            "Calling CodeCoder knowledge search API"
        );

        let request = KnowledgeSearchRequest {
            query: query.to_string(),
            limit: Some(5),
            min_score: Some(0.3),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Knowledge API returned {}: {}",
                status,
                error_text
            ));
        }

        let search_response: KnowledgeSearchResponse = response.json().await?;

        tracing::debug!(
            success = search_response.success,
            result_count = search_response.data.as_ref().map(|d| d.results.len()),
            "Knowledge search response received"
        );

        Ok(search_response)
    }

    /// Format knowledge search results for IM channels.
    fn format_knowledge_response(data: &KnowledgeSearchData) -> String {
        if data.results.is_empty() {
            return "📚 **知识库搜索**\n\n未找到相关内容。请尝试使用其他关键词搜索。".to_string();
        }

        let mut lines = vec![
            "📚 **知识库搜索结果**".to_string(),
            String::new(),
            format!("🔍 查询: {}", data.query),
            format!("📊 找到 {} 条相关内容", data.total),
            String::new(),
        ];

        for (i, result) in data.results.iter().take(5).enumerate() {
            // Format score as percentage
            let score_pct = (result.score * 100.0).round() as i32;
            let score_emoji = if score_pct >= 80 {
                "🟢"
            } else if score_pct >= 60 {
                "🟡"
            } else {
                "🟠"
            };

            // Build result header
            let header = if let Some(ref heading) = result.heading {
                format!("### {}. {} ({}%)", i + 1, heading.trim_start_matches('#').trim(), score_pct)
            } else {
                format!("### {}. 片段 {} ({}%)", i + 1, result.chunk_index + 1, score_pct)
            };

            lines.push(format!("{} {}", score_emoji, header));

            // Truncate content for IM display
            let content = if result.content.len() > 500 {
                format!("{}...", &result.content[..500])
            } else {
                result.content.clone()
            };
            lines.push(content);

            lines.push(format!("_来源: {}_", result.filename));

            if i < data.results.len() - 1 {
                lines.push(String::new());
                lines.push("---".to_string());
                lines.push(String::new());
            }
        }

        lines.push(String::new());
        lines.push(format!("🔄 搜索模式: {}", data.search_mode));

        lines.join("\n")
    }

    /// Call the feasibility assessment API.
    async fn call_feasibility(&self, query: &str) -> Result<FeasibilityResponse> {
        let url = format!("{}/api/v1/assess/feasibility", self.endpoint);

        tracing::debug!(
            endpoint = %url,
            query = %query,
            "Calling CodeCoder feasibility API"
        );

        let request = FeasibilityRequest {
            query: query.to_string(),
            options: Some(FeasibilityOptions {
                depth: "standard".to_string(),
                include_code_refs: true,
                language: "zh-CN".to_string(),
            }),
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .timeout(self.timeout)
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Feasibility API returned {}: {}",
                status,
                error_text
            ));
        }

        let feasibility_response: FeasibilityResponse = response.json().await?;

        tracing::debug!(
            success = feasibility_response.success,
            complexity = ?feasibility_response.data.as_ref().map(|d| &d.complexity),
            "Feasibility response received"
        );

        Ok(feasibility_response)
    }

    /// Format feasibility analysis for IM channels.
    fn format_feasibility_response(data: &FeasibilityData) -> String {
        let complexity_emoji = match data.complexity.as_str() {
            "low" => "🟢",
            "medium" => "🟡",
            "high" => "🟠",
            "critical" => "🔴",
            _ => "⚪",
        };

        let complexity_label = match data.complexity.as_str() {
            "low" => "低",
            "medium" => "中等",
            "high" => "较高",
            "critical" => "关键",
            _ => "未知",
        };

        let mut lines = vec![
            "📊 **技术可行性评估**".to_string(),
            String::new(),
            format!("**需求**: {}", data.summary),
            format!("**复杂度**: {} {}", complexity_emoji, complexity_label),
            String::new(),
        ];

        // Existing capabilities
        if !data.analysis.existing_capabilities.is_empty() {
            lines.push("✅ **现有能力**".to_string());
            for cap in data.analysis.existing_capabilities.iter().take(5) {
                lines.push(format!("• {} ({})", cap.name, cap.path));
            }
            lines.push(String::new());
        }

        // Required changes
        if !data.analysis.required_changes.is_empty() {
            lines.push("📝 **需要修改**".to_string());
            for change in data.analysis.required_changes.iter().take(8) {
                let action_label = match change.action.as_str() {
                    "create" => "[新建]",
                    "modify" => "[修改]",
                    "delete" => "[删除]",
                    _ => "[变更]",
                };
                lines.push(format!("{} {}", action_label, change.file));
            }
            lines.push(String::new());
        }

        // Dependencies
        if !data.analysis.dependencies.is_empty() {
            lines.push("📦 **新增依赖**".to_string());
            for dep in data.analysis.dependencies.iter().take(5) {
                lines.push(format!("• {} ({})", dep.name, dep.dep_type));
            }
            lines.push(String::new());
        }

        // Risks
        if !data.analysis.risks.is_empty() {
            lines.push("⚠️ **风险提示**".to_string());
            for risk in data.analysis.risks.iter().take(3) {
                lines.push(format!("• {}", risk));
            }
            lines.push(String::new());
        }

        lines.push(format!("置信度: {}%", (data.confidence * 100.0).round() as i32));

        lines.join("\n")
    }

    /// Start a background processor that handles messages from a channel.
    pub fn spawn_processor(
        bridge: Arc<Self>,
        mut rx: mpsc::Receiver<ChannelMessage>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            tracing::info!("CodeCoder bridge processor started");

            while let Some(message) = rx.recv().await {
                let bridge = bridge.clone();

                // Process each message in its own task
                tokio::spawn(async move {
                    if let Err(e) = bridge.process(message).await {
                        tracing::error!(error = %e, "Failed to process message");
                    }
                });
            }

            tracing::info!("CodeCoder bridge processor stopped");
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest {
            message: "Hello".into(),
            conversation_id: Some("conv-1".into()),
            agent: None,
            user_id: "user1".into(),
            channel: "telegram".into(),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"message\":\"Hello\""));
        assert!(json.contains("\"conversation_id\":\"conv-1\""));
        assert!(!json.contains("\"agent\"")); // Should be skipped when None
    }

    #[test]
    fn test_chat_response_deserialization() {
        // Test the wrapped API response format
        let json = r#"{
            "success": true,
            "data": {
                "message": "Hello back!",
                "conversation_id": "conv-1",
                "agent": "general",
                "usage": {
                    "input_tokens": 10,
                    "output_tokens": 20,
                    "total_tokens": 30
                }
            }
        }"#;

        let api_response: ChatApiResponse = serde_json::from_str(json).unwrap();
        assert!(api_response.success);
        assert!(api_response.data.is_some());

        let response = api_response.data.unwrap();
        assert_eq!(response.message, "Hello back!");
        assert_eq!(response.conversation_id, Some("conv-1".into()));
        assert!(response.usage.is_some());

        let usage = response.usage.unwrap();
        assert_eq!(usage.input_tokens, 10);
        assert_eq!(usage.output_tokens, 20);
    }

    #[test]
    fn test_chat_response_error_deserialization() {
        // Test error response format
        let json = r#"{
            "success": false,
            "error": "message is required"
        }"#;

        let api_response: ChatApiResponse = serde_json::from_str(json).unwrap();
        assert!(!api_response.success);
        assert!(api_response.data.is_none());
        assert_eq!(api_response.error, Some("message is required".into()));
    }

    #[test]
    fn test_bridge_creation() {
        let router = Arc::new(OutboundRouter::new());
        let bridge = CodeCoderBridge::new("http://localhost:4400", router);
        assert_eq!(bridge.endpoint, "http://localhost:4400");
    }

    #[test]
    fn test_feasibility_question_detection_chinese() {
        // Should match
        assert!(CodeCoderBridge::is_feasibility_question("增加微信登录，技术复杂度高吗？"));
        assert!(CodeCoderBridge::is_feasibility_question("这个功能能实现吗"));
        assert!(CodeCoderBridge::is_feasibility_question("需要改动多少文件"));
        assert!(CodeCoderBridge::is_feasibility_question("可行性如何"));
        assert!(CodeCoderBridge::is_feasibility_question("风险高吗"));
        assert!(CodeCoderBridge::is_feasibility_question("难度大吗"));
        assert!(CodeCoderBridge::is_feasibility_question("工作量大吗"));
        assert!(CodeCoderBridge::is_feasibility_question("帮我评估一下技术可行性"));

        // Should not match
        assert!(!CodeCoderBridge::is_feasibility_question("今天天气怎么样"));
        assert!(!CodeCoderBridge::is_feasibility_question("帮我写个函数"));
        assert!(!CodeCoderBridge::is_feasibility_question("这段代码有bug"));
    }

    #[test]
    fn test_feasibility_question_detection_english() {
        // Should match
        assert!(CodeCoderBridge::is_feasibility_question("How complex is adding OAuth?"));
        assert!(CodeCoderBridge::is_feasibility_question("Is it feasible to add real-time sync?"));
        assert!(CodeCoderBridge::is_feasibility_question("Can we implement WebSocket support?"));
        assert!(CodeCoderBridge::is_feasibility_question("How difficult is the migration?"));
        assert!(CodeCoderBridge::is_feasibility_question("What's the technical complexity?"));
        assert!(CodeCoderBridge::is_feasibility_question("Effort estimate for this feature?"));
        assert!(CodeCoderBridge::is_feasibility_question("How much work is this?"));

        // Should not match
        assert!(!CodeCoderBridge::is_feasibility_question("Hello world"));
        assert!(!CodeCoderBridge::is_feasibility_question("Write a function for me"));
    }

    #[test]
    fn test_feasibility_request_serialization() {
        let request = FeasibilityRequest {
            query: "增加微信支付功能".into(),
            options: Some(FeasibilityOptions {
                depth: "standard".into(),
                include_code_refs: true,
                language: "zh-CN".into(),
            }),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"query\":\"增加微信支付功能\""));
        assert!(json.contains("\"depth\":\"standard\""));
    }

    #[test]
    fn test_feasibility_response_deserialization() {
        let json = r#"{
            "success": true,
            "data": {
                "summary": "低风险，预计改动 3 个文件",
                "complexity": "low",
                "analysis": {
                    "complexity": "low",
                    "summary": "低风险，预计改动 3 个文件",
                    "existing_capabilities": [
                        {"name": "Auth模块", "path": "src/auth/", "relevance": "OAuth基础设施"}
                    ],
                    "required_changes": [
                        {"file": "src/auth/wechat.ts", "action": "create", "description": "新建微信OAuth"}
                    ],
                    "dependencies": [
                        {"name": "wechat-oauth", "type": "npm", "reason": "微信SDK"}
                    ],
                    "risks": ["需要申请微信开放平台"],
                    "confidence": 0.85
                },
                "confidence": 0.85
            }
        }"#;

        let response: FeasibilityResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert!(response.data.is_some());

        let data = response.data.unwrap();
        assert_eq!(data.complexity, "low");
        assert_eq!(data.confidence, 0.85);
        assert_eq!(data.analysis.existing_capabilities.len(), 1);
        assert_eq!(data.analysis.required_changes.len(), 1);
    }

    #[test]
    fn test_format_feasibility_response() {
        let data = FeasibilityData {
            summary: "增加微信登录功能".into(),
            complexity: "low".into(),
            analysis: FeasibilityAnalysis {
                complexity: "low".into(),
                summary: "增加微信登录功能".into(),
                existing_capabilities: vec![
                    ExistingCapability {
                        name: "Auth模块".into(),
                        path: "src/auth/".into(),
                        relevance: "OAuth基础设施".into(),
                    }
                ],
                required_changes: vec![
                    RequiredChange {
                        file: "src/auth/wechat.ts".into(),
                        action: "create".into(),
                        description: "新建微信OAuth".into(),
                    }
                ],
                dependencies: vec![
                    Dependency {
                        name: "wechat-oauth".into(),
                        dep_type: "npm".into(),
                        reason: "微信SDK".into(),
                    }
                ],
                risks: vec!["需要申请微信开放平台".into()],
                confidence: 0.85,
            },
            confidence: 0.85,
            tokens_used: Some(1500),
        };

        let formatted = CodeCoderBridge::format_feasibility_response(&data);

        assert!(formatted.contains("📊 **技术可行性评估**"));
        assert!(formatted.contains("🟢 低"));
        assert!(formatted.contains("Auth模块"));
        assert!(formatted.contains("[新建]"));
        assert!(formatted.contains("wechat-oauth"));
        assert!(formatted.contains("需要申请微信开放平台"));
        assert!(formatted.contains("85%"));
    }

    #[test]
    fn test_ab_test_request_detection() {
        // Should match @A/B pattern
        let result = CodeCoderBridge::is_ab_test_request("@A/B 生成一篇推文介绍我们的产品");
        assert!(result.is_some());
        let (models, prompt) = result.unwrap();
        assert_eq!(models.len(), 2);
        assert!(prompt.contains("生成一篇推文"));

        // Should match @对比 pattern
        let result = CodeCoderBridge::is_ab_test_request("@对比 写一个产品介绍");
        assert!(result.is_some());
        let (_, prompt) = result.unwrap();
        assert!(prompt.contains("写一个产品介绍"));

        // Should match @compare pattern
        let result = CodeCoderBridge::is_ab_test_request("@compare create a product description");
        assert!(result.is_some());
        let (_, prompt) = result.unwrap();
        assert!(prompt.contains("create a product description"));

        // Should NOT match regular chat
        assert!(CodeCoderBridge::is_ab_test_request("帮我写一篇文章").is_none());
        assert!(CodeCoderBridge::is_ab_test_request("Hello world").is_none());
    }

    #[test]
    fn test_compare_request_serialization() {
        let request = CompareRequest {
            models: vec!["anthropic/claude-sonnet-4".into(), "openai/gpt-4o".into()],
            prompt: "写一篇产品介绍".into(),
            system: None,
            max_tokens: Some(4096),
            temperature: Some(0.7),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"models\":["));
        assert!(json.contains("\"anthropic/claude-sonnet-4\""));
        assert!(json.contains("\"prompt\":\"写一篇产品介绍\""));
        assert!(!json.contains("\"system\"")); // Should be skipped when None
    }

    #[test]
    fn test_compare_response_deserialization() {
        let json = r#"{
            "success": true,
            "data": {
                "results": [
                    {
                        "model": "anthropic/claude-sonnet-4",
                        "provider": "anthropic",
                        "model_id": "claude-sonnet-4",
                        "content": "这是 Claude 的回复",
                        "tokens": {"input": 100, "output": 200, "total": 300},
                        "latency_ms": 1500
                    },
                    {
                        "model": "openai/gpt-4o",
                        "provider": "openai",
                        "model_id": "gpt-4o",
                        "content": "这是 GPT 的回复",
                        "tokens": {"input": 110, "output": 210, "total": 320},
                        "latency_ms": 1200
                    }
                ],
                "total_tokens": 620,
                "total_latency_ms": 1500
            }
        }"#;

        let response: CompareResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert!(response.data.is_some());

        let data = response.data.unwrap();
        assert_eq!(data.results.len(), 2);
        assert_eq!(data.total_tokens, 620);
        assert_eq!(data.results[0].provider, "anthropic");
        assert_eq!(data.results[1].provider, "openai");
    }

    #[test]
    fn test_format_compare_response() {
        let data = CompareData {
            results: vec![
                ModelResult {
                    model: "anthropic/claude-sonnet-4".into(),
                    provider: "anthropic".into(),
                    model_id: "claude-sonnet-4".into(),
                    content: "这是 Claude 的精彩回复".into(),
                    tokens: ModelTokenInfo { input: 100, output: 200, total: 300 },
                    latency_ms: 1500,
                    error: None,
                },
                ModelResult {
                    model: "openai/gpt-4o".into(),
                    provider: "openai".into(),
                    model_id: "gpt-4o".into(),
                    content: "这是 GPT 的精彩回复".into(),
                    tokens: ModelTokenInfo { input: 110, output: 210, total: 320 },
                    latency_ms: 1200,
                    error: None,
                },
            ],
            total_tokens: 620,
            total_latency_ms: 1500,
        };

        let formatted = CodeCoderBridge::format_compare_response(&data);

        assert!(formatted.contains("🔄 **多模型对比结果**"));
        assert!(formatted.contains("🟣")); // Anthropic emoji
        assert!(formatted.contains("🟢")); // OpenAI emoji
        assert!(formatted.contains("Claude 的精彩回复"));
        assert!(formatted.contains("GPT 的精彩回复"));
        assert!(formatted.contains("620 tokens"));
    }

    #[test]
    fn test_knowledge_question_detection_chinese() {
        // Should match @知识库 pattern
        let result = CodeCoderBridge::is_knowledge_question("@知识库 公司的产品愿景");
        assert!(result.is_some());
        assert!(result.unwrap().contains("产品愿景"));

        // Should match 帮我查一下 pattern
        let result = CodeCoderBridge::is_knowledge_question("帮我查一下公司的福利政策");
        assert!(result.is_some());
        assert!(result.unwrap().contains("公司的福利政策"));

        // Should match 搜索一下 pattern
        let result = CodeCoderBridge::is_knowledge_question("搜索一下项目管理流程");
        assert!(result.is_some());

        // Should match 文档里有关于 pattern
        let result = CodeCoderBridge::is_knowledge_question("文档里有关于入职流程的内容吗");
        assert!(result.is_some());
        assert!(result.unwrap().contains("入职流程"));

        // Should NOT match regular chat
        assert!(CodeCoderBridge::is_knowledge_question("今天天气怎么样").is_none());
        assert!(CodeCoderBridge::is_knowledge_question("帮我写个函数").is_none());
    }

    #[test]
    fn test_knowledge_question_detection_english() {
        // Should match @knowledge pattern
        let result = CodeCoderBridge::is_knowledge_question("@knowledge company policies");
        assert!(result.is_some());
        assert!(result.unwrap().contains("company policies"));

        // Should match @kb pattern
        let result = CodeCoderBridge::is_knowledge_question("@kb onboarding process");
        assert!(result.is_some());

        // Should match search pattern
        let result = CodeCoderBridge::is_knowledge_question("search for vacation policy");
        assert!(result.is_some());
        assert!(result.unwrap().contains("vacation policy"));

        // Should NOT match regular chat
        assert!(CodeCoderBridge::is_knowledge_question("Hello world").is_none());
    }

    #[test]
    fn test_knowledge_request_serialization() {
        let request = KnowledgeSearchRequest {
            query: "公司福利政策".into(),
            limit: Some(5),
            min_score: Some(0.3),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"query\":\"公司福利政策\""));
        assert!(json.contains("\"limit\":5"));
        assert!(json.contains("\"min_score\":0.3"));
    }

    #[test]
    fn test_knowledge_response_deserialization() {
        let json = "{
            \"success\": true,
            \"data\": {
                \"results\": [
                    {
                        \"content\": \"公司提供以下福利：五险一金、年假15天...\",
                        \"score\": 0.85,
                        \"document_id\": \"doc-123\",
                        \"chunk_index\": 2,
                        \"filename\": \"员工手册.md\",
                        \"heading\": \"福利待遇\"
                    }
                ],
                \"total\": 1,
                \"query\": \"公司福利政策\",
                \"search_mode\": \"hybrid\"
            }
        }";

        let response: KnowledgeSearchResponse = serde_json::from_str(json).unwrap();
        assert!(response.success);
        assert!(response.data.is_some());

        let data = response.data.unwrap();
        assert_eq!(data.results.len(), 1);
        assert_eq!(data.total, 1);
        assert_eq!(data.search_mode, "hybrid");
        assert_eq!(data.results[0].filename, "员工手册.md");
        assert_eq!(data.results[0].score, 0.85);
    }

    #[test]
    fn test_format_knowledge_response() {
        let data = KnowledgeSearchData {
            results: vec![
                KnowledgeResult {
                    content: "公司提供以下福利：五险一金、年假15天、免费午餐...".into(),
                    score: 0.85,
                    document_id: "doc-123".into(),
                    chunk_index: 2,
                    filename: "员工手册.md".into(),
                    heading: Some("## 福利待遇".into()),
                },
                KnowledgeResult {
                    content: "年假根据工龄计算，第一年15天，每增加一年增加1天...".into(),
                    score: 0.72,
                    document_id: "doc-123".into(),
                    chunk_index: 3,
                    filename: "员工手册.md".into(),
                    heading: Some("### 年假政策".into()),
                },
            ],
            total: 2,
            query: "公司福利政策".into(),
            search_mode: "hybrid".into(),
        };

        let formatted = CodeCoderBridge::format_knowledge_response(&data);

        assert!(formatted.contains("📚 **知识库搜索结果**"));
        assert!(formatted.contains("公司福利政策"));
        assert!(formatted.contains("找到 2 条相关内容"));
        assert!(formatted.contains("福利待遇"));
        assert!(formatted.contains("85%"));
        assert!(formatted.contains("员工手册.md"));
        assert!(formatted.contains("hybrid"));
    }

    #[test]
    fn test_format_knowledge_response_empty() {
        let data = KnowledgeSearchData {
            results: vec![],
            total: 0,
            query: "不存在的内容".into(),
            search_mode: "hybrid".into(),
        };

        let formatted = CodeCoderBridge::format_knowledge_response(&data);
        assert!(formatted.contains("未找到相关内容"));
    }

    #[test]
    fn test_bug_report_detection_chinese() {
        // Should match crash patterns
        let result = CodeCoderBridge::is_bug_report("应用崩溃了，一打开就闪退");
        assert!(result.is_some());
        assert_eq!(result.unwrap().category, "crash");

        // Should match error patterns
        let result = CodeCoderBridge::is_bug_report("页面报错了，显示500错误");
        assert!(result.is_some());
        assert_eq!(result.unwrap().category, "error");

        // Should match functionality patterns
        let result = CodeCoderBridge::is_bug_report("无法登录，一直转圈");
        assert!(result.is_some());
        assert_eq!(result.unwrap().category, "functionality");

        // Should match broken patterns
        let result = CodeCoderBridge::is_bug_report("这个功能坏了");
        assert!(result.is_some());
        assert_eq!(result.unwrap().category, "broken");

        // Should NOT match regular chat
        assert!(CodeCoderBridge::is_bug_report("今天天气真好").is_none());
        assert!(CodeCoderBridge::is_bug_report("帮我写个函数").is_none());
    }

    #[test]
    fn test_bug_report_detection_english() {
        // Should match crash patterns
        let result = CodeCoderBridge::is_bug_report("The app crashes when I click the button");
        assert!(result.is_some());
        assert_eq!(result.unwrap().category, "crash");

        // Should match error patterns
        let result = CodeCoderBridge::is_bug_report("Getting an error when submitting the form");
        assert!(result.is_some());
        assert_eq!(result.unwrap().category, "error");

        // Should match functionality patterns
        let result = CodeCoderBridge::is_bug_report("Can't login to my account");
        assert!(result.is_some());
        assert_eq!(result.unwrap().category, "functionality");

        // Should match display patterns
        let result = CodeCoderBridge::is_bug_report("Showing a white screen after update");
        assert!(result.is_some());
        assert_eq!(result.unwrap().category, "display");

        // Should NOT match regular chat
        assert!(CodeCoderBridge::is_bug_report("Hello, how are you?").is_none());
    }

    #[test]
    fn test_feature_request_detection_chinese() {
        // Should match request patterns
        let result = CodeCoderBridge::is_feature_request("希望能添加深色模式");
        assert!(result.is_some());

        let result = CodeCoderBridge::is_feature_request("建议增加导出功能");
        assert!(result.is_some());

        let result = CodeCoderBridge::is_feature_request("能不能支持微信登录？");
        assert!(result.is_some());

        let result = CodeCoderBridge::is_feature_request("如果能自动保存就好了");
        assert!(result.is_some());

        // Should NOT match regular chat
        assert!(CodeCoderBridge::is_feature_request("今天天气真好").is_none());
        assert!(CodeCoderBridge::is_feature_request("帮我查一下资料").is_none());
    }

    #[test]
    fn test_feature_request_detection_english() {
        // Should match request patterns
        let result = CodeCoderBridge::is_feature_request("Feature request: add dark mode");
        assert!(result.is_some());

        let result = CodeCoderBridge::is_feature_request("Would be nice to have auto-save");
        assert!(result.is_some());

        let result = CodeCoderBridge::is_feature_request("Can you add export to PDF?");
        assert!(result.is_some());

        let result = CodeCoderBridge::is_feature_request("Please add multi-language support");
        assert!(result.is_some());

        // Should NOT match regular chat
        assert!(CodeCoderBridge::is_feature_request("Hello world").is_none());
    }

    #[test]
    fn test_agent_command_parsing_zhurong_agents() {
        // Should match @macro pattern
        let result = CodeCoderBridge::parse_agent_command("@macro 解读本月PMI数据");
        assert!(result.is_some());
        let (agent, prompt) = result.unwrap();
        assert_eq!(agent, "macro");
        assert_eq!(prompt, "解读本月PMI数据");

        // Should match @decision pattern
        let result = CodeCoderBridge::parse_agent_command("@decision 用CLOSE框架分析这个职业选择");
        assert!(result.is_some());
        let (agent, prompt) = result.unwrap();
        assert_eq!(agent, "decision");
        assert!(prompt.contains("CLOSE框架"));

        // Should match @trader pattern
        let result = CodeCoderBridge::parse_agent_command("@trader 分析今日情绪周期");
        assert!(result.is_some());
        let (agent, prompt) = result.unwrap();
        assert_eq!(agent, "trader");
        assert!(prompt.contains("情绪周期"));

        // Should match @observer pattern
        let result = CodeCoderBridge::parse_agent_command("@observer 用可能性基底解释这个现象");
        assert!(result.is_some());
        let (agent, prompt) = result.unwrap();
        assert_eq!(agent, "observer");

        // Should match @picker pattern
        let result = CodeCoderBridge::parse_agent_command("@picker 分析这个选品机会");
        assert!(result.is_some());
        let (agent, prompt) = result.unwrap();
        assert_eq!(agent, "picker");

        // Should match @miniproduct pattern
        let result = CodeCoderBridge::parse_agent_command("@miniproduct 帮我验证这个产品想法");
        assert!(result.is_some());
        let (agent, prompt) = result.unwrap();
        assert_eq!(agent, "miniproduct");
    }

    #[test]
    fn test_agent_command_parsing_engineering_agents() {
        // Should match @code-reviewer pattern
        let result = CodeCoderBridge::parse_agent_command("@code-reviewer review the auth module");
        assert!(result.is_some());
        let (agent, prompt) = result.unwrap();
        assert_eq!(agent, "code-reviewer");

        // Should match @security-reviewer pattern
        let result = CodeCoderBridge::parse_agent_command("@security-reviewer check for vulnerabilities");
        assert!(result.is_some());
        let (agent, prompt) = result.unwrap();
        assert_eq!(agent, "security-reviewer");

        // Should match @architect pattern
        let result = CodeCoderBridge::parse_agent_command("@architect design the payment system");
        assert!(result.is_some());
        let (agent, prompt) = result.unwrap();
        assert_eq!(agent, "architect");

        // Should match @tdd-guide pattern
        let result = CodeCoderBridge::parse_agent_command("@tdd-guide write tests for the user service");
        assert!(result.is_some());
        let (agent, prompt) = result.unwrap();
        assert_eq!(agent, "tdd-guide");
    }

    #[test]
    fn test_agent_command_parsing_with_chinese_colon() {
        // Should handle Chinese colon separator
        let result = CodeCoderBridge::parse_agent_command("@macro：解读本月PMI数据");
        assert!(result.is_some());
        let (agent, prompt) = result.unwrap();
        assert_eq!(agent, "macro");
        assert_eq!(prompt, "解读本月PMI数据");

        // Should handle English colon separator
        let result = CodeCoderBridge::parse_agent_command("@decision: analyze this choice");
        assert!(result.is_some());
        let (agent, prompt) = result.unwrap();
        assert_eq!(agent, "decision");
        assert_eq!(prompt, "analyze this choice");
    }

    #[test]
    fn test_agent_command_parsing_case_insensitive() {
        // Should be case insensitive
        let result = CodeCoderBridge::parse_agent_command("@MACRO 解读数据");
        assert!(result.is_some());
        let (agent, _) = result.unwrap();
        assert_eq!(agent, "macro");

        let result = CodeCoderBridge::parse_agent_command("@Trader analyze");
        assert!(result.is_some());
        let (agent, _) = result.unwrap();
        assert_eq!(agent, "trader");
    }

    #[test]
    fn test_agent_command_parsing_negative_cases() {
        // Should NOT match unknown agents
        assert!(CodeCoderBridge::parse_agent_command("@unknown_agent do something").is_none());

        // Should NOT match without prompt
        assert!(CodeCoderBridge::parse_agent_command("@macro").is_none());
        assert!(CodeCoderBridge::parse_agent_command("@macro ").is_none());

        // Should NOT match regular messages
        assert!(CodeCoderBridge::parse_agent_command("Hello world").is_none());
        assert!(CodeCoderBridge::parse_agent_command("帮我分析数据").is_none());

        // Should NOT match email addresses
        assert!(CodeCoderBridge::parse_agent_command("email@example.com").is_none());

        // Should NOT match Twitter handles (unknown agents)
        assert!(CodeCoderBridge::parse_agent_command("@username hello").is_none());
    }

    #[test]
    fn test_agent_help_request_detection() {
        // Should match various help patterns
        assert!(CodeCoderBridge::is_agent_help_request("@help"));
        assert!(CodeCoderBridge::is_agent_help_request("@?"));
        assert!(CodeCoderBridge::is_agent_help_request("@帮助"));
        assert!(CodeCoderBridge::is_agent_help_request("@agents"));
        assert!(CodeCoderBridge::is_agent_help_request("help agents"));
        assert!(CodeCoderBridge::is_agent_help_request("list agents"));

        // Should be case insensitive
        assert!(CodeCoderBridge::is_agent_help_request("@HELP"));
        assert!(CodeCoderBridge::is_agent_help_request("@Agents"));

        // Should NOT match regular messages
        assert!(!CodeCoderBridge::is_agent_help_request("hello"));
        assert!(!CodeCoderBridge::is_agent_help_request("@macro 解读数据"));
        assert!(!CodeCoderBridge::is_agent_help_request("help me"));
    }

    #[test]
    fn test_agent_help_format() {
        let help = CodeCoderBridge::format_agent_help();

        // Should contain key sections
        assert!(help.contains("🤖 **可用的 Agent 列表**"));
        assert!(help.contains("祝融说系列"));
        assert!(help.contains("@macro"));
        assert!(help.contains("@decision"));
        assert!(help.contains("@trader"));
        assert!(help.contains("工程质量"));
        assert!(help.contains("@code-reviewer"));
        assert!(help.contains("使用方式"));
    }
}
