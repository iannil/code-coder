/// ─── LLM Client ────────────────────────────────────────────────────────────
///
/// OpenAI-compatible async API client (works with OpenAI, Anthropic, local
/// vLLM/llama.cpp servers, etc.).

use serde::{Deserialize, Serialize};

/// ─── Configuration ─────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LlmConfig {
    pub api_base: String,
    pub model: String,
    pub api_key: String,
    pub max_tokens: u32,
    pub temperature: f32,
}

impl LlmConfig {
    pub fn from_env() -> Self {
        Self {
            api_base: std::env::var("CODECODER_API_BASE")
                .or_else(|_| std::env::var("OPENAI_API_BASE"))
                .unwrap_or_else(|_| "https://api.openai.com/v1".into()),
            model: std::env::var("CODECODER_MODEL")
                .or_else(|_| std::env::var("OPENAI_MODEL"))
                .unwrap_or_else(|_| "gpt-4o".into()),
            api_key: std::env::var("CODECODER_API_KEY")
                .or_else(|_| std::env::var("OPENAI_API_KEY"))
                .expect("CODECODER_API_KEY or OPENAI_API_KEY must be set"),
            max_tokens: std::env::var("CODECODER_MAX_TOKENS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(4096),
            temperature: std::env::var("CODECODER_TEMPERATURE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.7),
        }
    }
}

/// ─── Message types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

impl Message {
    pub fn system(text: impl Into<String>) -> Self {
        Self { role: "system".into(), content: text.into() }
    }
    pub fn user(text: impl Into<String>) -> Self {
        Self { role: "user".into(), content: text.into() }
    }
    pub fn assistant(text: impl Into<String>) -> Self {
        Self { role: "assistant".into(), content: text.into() }
    }
}

/// ─── API response types ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LlmResponse {
    pub text: String,
    pub tokens_in: u32,
    pub tokens_out: u32,
}

#[derive(Debug, Clone)]
pub struct StreamDelta {
    pub text: Option<String>,
    pub reasoning: Option<String>,
}

pub type StreamReceiver = tokio::sync::mpsc::Receiver<StreamDelta>;

/// ─── OpenAI wire format ────────────────────────────────────────────────────

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    stream: bool,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: Delta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChoiceMessage,
}

#[derive(Deserialize)]
struct ChoiceMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct Usage {
    #[serde(default)]
    prompt_tokens: u32,
    #[serde(default)]
    completion_tokens: u32,
}

/// ─── LLM Client trait (async) ─────────────────────────────────────────────

#[async_trait::async_trait]
pub trait LlmClient: Send + 'static {
    fn config(&self) -> &LlmConfig;
    async fn chat(&self, messages: &[Message]) -> anyhow::Result<LlmResponse>;
    async fn chat_stream(&self, messages: &[Message]) -> anyhow::Result<(LlmResponse, StreamReceiver)>;
}

/// ─── OpenAiClient ──────────────────────────────────────────────────────────

pub struct OpenAiClient {
    config: LlmConfig,
    client: reqwest::Client,
}

impl OpenAiClient {
    pub fn new(config: LlmConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .connect_timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("failed to create HTTP client");
        Self { config, client }
    }

    pub fn from_env() -> Self {
        Self::new(LlmConfig::from_env())
    }

    fn build_request(&self, request: &ChatRequest) -> reqwest::RequestBuilder {
        let url = format!(
            "{}/chat/completions",
            self.config.api_base.trim_end_matches('/')
        );
        self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(request)
    }
}

#[async_trait::async_trait]
impl LlmClient for OpenAiClient {
    fn config(&self) -> &LlmConfig {
        &self.config
    }

    async fn chat(&self, messages: &[Message]) -> anyhow::Result<LlmResponse> {
        let chat_messages: Vec<ChatMessage> = messages.iter()
            .map(|m| ChatMessage { role: m.role.clone(), content: m.content.clone() })
            .collect();

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: chat_messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            stream: false,
        };

        let resp = self.build_request(&request)
            .send().await
            .map_err(|e| {
                crate::log(&format!("[codecoder] LLM 请求失败: {e}"));
                anyhow::anyhow!("LLM API request failed: {e}")
            })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            crate::log(&format!("[codecoder] LLM HTTP {status}: {body}"));
            anyhow::bail!("LLM API returned {status}: {body}");
        }

        let chat_resp: ChatResponse = resp.json().await
            .map_err(|e| anyhow::anyhow!("LLM response parse failed: {e}"))?;

        let text = chat_resp.choices.first()
            .and_then(|c| c.message.content.as_deref()).unwrap_or("").to_string();

        let (tokens_in, tokens_out) = chat_resp.usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((0, 0));

        Ok(LlmResponse { text, tokens_in, tokens_out })
    }

    async fn chat_stream(&self, messages: &[Message]) -> anyhow::Result<(LlmResponse, StreamReceiver)> {
        let chat_messages: Vec<ChatMessage> = messages.iter()
            .map(|m| ChatMessage { role: m.role.clone(), content: m.content.clone() })
            .collect();

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: chat_messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            stream: true,
        };

        let url = format!(
            "{}/chat/completions",
            self.config.api_base.trim_end_matches('/')
        );
        crate::log(&format!("[codecoder] LLM 请求: POST {}", url));
        crate::log(&format!("[codecoder] 模型: {}", self.config.model));

        let response = self.build_request(&request)
            .send().await
            .map_err(|e| {
                crate::log(&format!("[codecoder] LLM 请求失败: {e}"));
                anyhow::anyhow!("LLM stream request failed: {e}")
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            crate::log(&format!("[codecoder] LLM 流请求 HTTP {status}: {body}"));
            anyhow::bail!("LLM stream returned {status}: {body}");
        }

        let (tx, rx) = tokio::sync::mpsc::channel::<StreamDelta>(256);

        tokio::spawn(async move {
            use futures_util::StreamExt;

            let mut stream = response.bytes_stream();
            let mut buf = Vec::new();

            while let Some(chunk) = stream.next().await {
                let chunk = match chunk {
                    Ok(c) => c,
                    Err(_) => break,
                };
                buf.extend_from_slice(&chunk);

                // Extract complete lines from buffer
                loop {
                    let nl_pos = match buf.iter().position(|&b| b == b'\n') {
                        Some(p) => p,
                        None => break,
                    };
                    let line_bytes: Vec<u8> = buf.drain(..=nl_pos).collect();
                    let line = String::from_utf8_lossy(
                        &line_bytes[..line_bytes.len().saturating_sub(1)]
                    ).trim().to_string();

                    if line.is_empty() { continue; }

                    let json_str = match line.strip_prefix("data: ") {
                        Some("") | Some("[DONE]") => return,
                        Some(s) => s,
                        None => continue,
                    };

                    if let Ok(chunk) = serde_json::from_str::<StreamChunk>(json_str) {
                        if let Some(c) = chunk.choices.first() {
                            if let Some(ref r) = c.delta.reasoning_content {
                                if !r.is_empty() {
                                    let _ = tx.send(StreamDelta { text: None, reasoning: Some(r.clone()) }).await;
                                }
                            }
                            if let Some(ref text) = c.delta.content {
                                if !text.is_empty() {
                                    let _ = tx.send(StreamDelta { text: Some(text.clone()), reasoning: None }).await;
                                }
                            }
                        }
                    }
                }
            }
        });

        Ok((LlmResponse { text: String::new(), tokens_in: 0, tokens_out: 0 }, rx))
    }
}

/// ─── StubClient (for testing / fallback) ───────────────────────────────────

#[allow(dead_code)]
pub struct StubClient {
    config: LlmConfig,
}

impl StubClient {
    pub fn new() -> Self {
        Self {
            config: LlmConfig {
                api_base: "http://localhost:9999".into(),
                model: "stub".into(),
                api_key: "stub".into(),
                max_tokens: 4096,
                temperature: 0.7,
            },
        }
    }
}

#[async_trait::async_trait]
impl LlmClient for StubClient {
    fn config(&self) -> &LlmConfig {
        &self.config
    }

    async fn chat(&self, messages: &[Message]) -> anyhow::Result<LlmResponse> {
        let last = messages.last().map(|m| m.content.as_str()).unwrap_or("");
        Ok(LlmResponse { text: format!("[StubClient] You said: {last}"), tokens_in: 0, tokens_out: 0 })
    }

    async fn chat_stream(&self, messages: &[Message]) -> anyhow::Result<(LlmResponse, StreamReceiver)> {
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        let last = messages.last().map(|m| m.content.as_str()).unwrap_or("");
        let text = format!("[StubClient] You said: {last}");
        let text_clone = text.clone();

        tokio::spawn(async move {
            let _ = tx.send(StreamDelta { text: Some(text_clone), reasoning: None }).await;
        });

        Ok((LlmResponse { text, tokens_in: 0, tokens_out: 0 }, rx))
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_stub_client() {
        let client = StubClient::new();
        let resp = client.chat(&[Message::user("hello")]).await.unwrap();
        assert!(resp.text.contains("hello"));
    }

    #[tokio::test]
    async fn test_stub_stream() {
        let client = StubClient::new();
        let (_resp, mut rx) = client.chat_stream(&[Message::user("hi")]).await.unwrap();
        while let Some(delta) = rx.recv().await {
            if let Some(text) = delta.text {
                assert!(text.contains("hi"));
            }
        }
    }

    #[test]
    fn test_message_constructors() {
        assert_eq!(Message::system("x").role, "system");
        assert_eq!(Message::user("x").role, "user");
        assert_eq!(Message::assistant("x").role, "assistant");
    }

    #[test]
    fn test_llm_config_direct_construction() {
        let config = LlmConfig {
            api_base: "https://test.example.com/v1".into(),
            model: "test-model".into(),
            api_key: "test-key".into(),
            max_tokens: 2048,
            temperature: 0.5,
        };
        assert_eq!(config.api_base, "https://test.example.com/v1");
        assert_eq!(config.model, "test-model");
        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.max_tokens, 2048);
        assert_eq!(config.temperature, 0.5);
    }

    #[test]
    fn test_llm_response_construction() {
        let resp = LlmResponse {
            text: "Hello".into(),
            tokens_in: 10,
            tokens_out: 5,
        };
        assert_eq!(resp.text, "Hello");
        assert_eq!(resp.tokens_in, 10);
        assert_eq!(resp.tokens_out, 5);
    }

    #[test]
    fn test_stream_delta_text() {
        let delta = StreamDelta {
            text: Some("hello".into()),
            reasoning: None,
        };
        assert_eq!(delta.text.as_deref(), Some("hello"));
        assert!(delta.reasoning.is_none());
    }

    #[test]
    fn test_stream_delta_reasoning() {
        let delta = StreamDelta {
            text: None,
            reasoning: Some("thinking...".into()),
        };
        assert!(delta.text.is_none());
        assert_eq!(delta.reasoning.as_deref(), Some("thinking..."));
    }

    #[test]
    fn test_stream_delta_both() {
        let delta = StreamDelta {
            text: Some("answer".into()),
            reasoning: Some("step by step".into()),
        };
        assert_eq!(delta.text.as_deref(), Some("answer"));
        assert_eq!(delta.reasoning.as_deref(), Some("step by step"));
    }

    #[test]
    fn test_message_content() {
        let msg = Message::user("hello world");
        assert_eq!(msg.content, "hello world");
    }

    #[test]
    fn test_openai_client_new() {
        let config = LlmConfig {
            api_base: "http://localhost:8080".into(),
            model: "test-model".into(),
            api_key: "key".into(),
            max_tokens: 100,
            temperature: 0.5,
        };
        let client = OpenAiClient::new(config);
        // config getter
        assert_eq!(client.config.api_base, "http://localhost:8080");
        assert_eq!(client.config.model, "test-model");
    }

    #[test]
    fn test_openai_client_from_env_vars() {
        unsafe {
            // Test CODECODER_ prefixed vars first
            std::env::set_var("CODECODER_API_KEY", "test-key-from-env");
            std::env::set_var("CODECODER_API_BASE", "https://custom.example.com/v1");
            std::env::set_var("CODECODER_MODEL", "custom-model");
            std::env::set_var("CODECODER_MAX_TOKENS", "8192");
            std::env::set_var("CODECODER_TEMPERATURE", "0.2");
        }
        let config = LlmConfig::from_env();
        assert_eq!(config.api_key, "test-key-from-env");
        assert_eq!(config.api_base, "https://custom.example.com/v1");
        assert_eq!(config.model, "custom-model");
        assert_eq!(config.max_tokens, 8192);

        let client = OpenAiClient::from_env();
        assert_eq!(client.config.model, "custom-model");

        // Now test fallback to OPENAI_ prefixed vars
        unsafe {
            std::env::remove_var("CODECODER_API_KEY");
            std::env::remove_var("CODECODER_API_BASE");
            std::env::remove_var("CODECODER_MODEL");
            std::env::set_var("OPENAI_API_KEY", "openai-fallback-key");
            std::env::set_var("OPENAI_API_BASE", "https://openai.example.com/v1");
            std::env::set_var("OPENAI_MODEL", "gpt-4o");
        }
        let config2 = LlmConfig::from_env();
        assert_eq!(config2.api_key, "openai-fallback-key");
        assert_eq!(config2.api_base, "https://openai.example.com/v1");
        assert_eq!(config2.model, "gpt-4o");
    }

    #[test]
    fn test_stub_client_config() {
        let client = StubClient::new();
        let config = client.config();
        assert_eq!(config.model, "stub");
        assert_eq!(config.api_key, "stub");
    }

    #[test]
    fn test_chat_request_serialization() {
        let req = ChatRequest {
            model: "gpt-4".into(),
            messages: vec![
                ChatMessage { role: "user".into(), content: "hello".into() },
            ],
            max_tokens: 100,
            temperature: 0.5,
            stream: false,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "gpt-4");
        assert_eq!(json["messages"][0]["role"], "user");
        assert_eq!(json["messages"][0]["content"], "hello");
        assert_eq!(json["max_tokens"], 100);
        assert_eq!(json["temperature"], 0.5);
        // stream: false is skipped due to skip_serializing_if
        assert!(json.get("stream").is_none() || json["stream"] == false);
    }

    #[test]
    fn test_chat_request_stream_serialization() {
        let req = ChatRequest {
            model: "gpt-4".into(),
            messages: vec![],
            max_tokens: 100,
            temperature: 0.5,
            stream: true,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["stream"], true);
    }

    #[test]
    fn test_chat_request_skip_serialization_when_not_stream() {
        // stream: false should skip the "stream" field
        let req = ChatRequest {
            model: "gpt-4".into(),
            messages: vec![],
            max_tokens: 100,
            temperature: 0.5,
            stream: false,
        };
        let json = serde_json::to_value(&req).unwrap();
        // When stream=false, the field should be omitted due to skip_serializing_if
        assert!(json.get("stream").is_none() || json["stream"] == false);
    }

    #[test]
    fn test_chat_response_deserialize() {
        let json = r#"{
            "choices": [{"message": {"content": "Hello world"}}],
            "usage": {"prompt_tokens": 10, "completion_tokens": 20}
        }"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.choices.len(), 1);
        assert_eq!(resp.choices[0].message.content.as_deref(), Some("Hello world"));
        assert!(resp.usage.is_some());
        assert_eq!(resp.usage.as_ref().unwrap().prompt_tokens, 10);
    }

    #[test]
    fn test_chat_response_no_usage() {
        let json = r#"{"choices": [{"message": {"content": "test"}}]}"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert!(resp.usage.is_none());
    }

    #[test]
    fn test_stream_chunk_deserialize() {
        let json = r#"{"choices": [{"delta": {"content": "Hello"}, "finish_reason": null}]}"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        assert_eq!(chunk.choices.len(), 1);
        assert_eq!(chunk.choices[0].delta.content.as_deref(), Some("Hello"));
    }

    #[test]
    fn test_stream_chunk_with_reasoning() {
        let json = r#"{"choices": [{"delta": {"content": "answer", "reasoning_content": "thinking step"}}]}"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        let delta = &chunk.choices[0].delta;
        assert_eq!(delta.content.as_deref(), Some("answer"));
        assert_eq!(delta.reasoning_content.as_deref(), Some("thinking step"));
    }

    #[test]
    fn test_stream_chunk_empty_delta() {
        let json = r#"{"choices": [{"delta": {}}]}"#;
        let chunk: StreamChunk = serde_json::from_str(json).unwrap();
        assert!(chunk.choices[0].delta.content.is_none());
        assert!(chunk.choices[0].delta.reasoning_content.is_none());
    }

    #[test]
    fn test_choice_message_with_content() {
        let json = r#"{"message": {"content": "test response"}}"#;
        let choice: Choice = serde_json::from_str(json).unwrap();
        assert_eq!(choice.message.content.as_deref(), Some("test response"));
    }

    #[test]
    fn test_choice_message_null_content() {
        let json = r#"{"message": {}}"#;
        let choice: Choice = serde_json::from_str(json).unwrap();
        assert!(choice.message.content.is_none());
    }

    #[test]
    fn test_usage_deserialize() {
        let json = r#"{"prompt_tokens": 50, "completion_tokens": 100}"#;
        let usage: Usage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.prompt_tokens, 50);
        assert_eq!(usage.completion_tokens, 100);
    }

    #[test]
    fn test_usage_defaults() {
        let json = r#"{}"#;
        let usage: Usage = serde_json::from_str(json).unwrap();
        assert_eq!(usage.prompt_tokens, 0);
        assert_eq!(usage.completion_tokens, 0);
    }

    #[test]
    fn test_chat_message_serialize() {
        let msg = ChatMessage { role: "user".into(), content: "hello".into() };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "user");
        assert_eq!(json["content"], "hello");
    }

    #[tokio::test]
    async fn test_stub_client_chat_returns_response() {
        let client = StubClient::new();
        let resp = client.chat(&[Message::user("test message")]).await.unwrap();
        assert!(resp.text.contains("test message"));
    }

    #[tokio::test]
    async fn test_stub_client_chat_empty_messages() {
        let client = StubClient::new();
        let resp = client.chat(&[]).await.unwrap();
        assert!(!resp.text.is_empty()); // should still produce a response
    }
}
