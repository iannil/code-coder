/// ─── LLM Client ────────────────────────────────────────────────────────────
///
/// OpenAI-compatible API client (works with OpenAI, Anthropic, local
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
    /// Load config from environment variables with sensible defaults.
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
        Self {
            role: "system".into(),
            content: text.into(),
        }
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: text.into(),
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: text.into(),
        }
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

/// ─── OpenAI request/response wire format ───────────────────────────────────

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
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

/// ─── LLM Client trait ──────────────────────────────────────────────────────

pub trait LlmClient: Send + 'static {
    fn config(&self) -> &LlmConfig;
    fn chat(&self, messages: &[Message]) -> anyhow::Result<LlmResponse>;
}

/// ─── OpenAiClient ──────────────────────────────────────────────────────────

pub struct OpenAiClient {
    config: LlmConfig,
    client: reqwest::blocking::Client,
}

impl OpenAiClient {
    pub fn new(config: LlmConfig) -> Self {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("failed to create HTTP client");
        Self { config, client }
    }

    pub fn from_env() -> Self {
        Self::new(LlmConfig::from_env())
    }
}

impl LlmClient for OpenAiClient {
    fn config(&self) -> &LlmConfig {
        &self.config
    }

    fn chat(&self, messages: &[Message]) -> anyhow::Result<LlmResponse> {
        let chat_messages: Vec<ChatMessage> = messages
            .iter()
            .map(|m| ChatMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let request = ChatRequest {
            model: self.config.model.clone(),
            messages: chat_messages,
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
        };

        let url = format!(
            "{}/chat/completions",
            self.config.api_base.trim_end_matches('/')
        );

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&request)
            .send()
            .map_err(|e| anyhow::anyhow!("LLM API request failed: {e}"))?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().unwrap_or_default();
            anyhow::bail!("LLM API returned {status}: {body}");
        }

        let chat_resp: ChatResponse = resp
            .json()
            .map_err(|e| anyhow::anyhow!("LLM API response parse failed: {e}"))?;

        let text = chat_resp
            .choices
            .first()
            .and_then(|c| c.message.content.as_deref())
            .unwrap_or("")
            .to_string();

        let (tokens_in, tokens_out) = chat_resp
            .usage
            .map(|u| (u.prompt_tokens, u.completion_tokens))
            .unwrap_or((0, 0));

        Ok(LlmResponse {
            text,
            tokens_in,
            tokens_out,
        })
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

impl LlmClient for StubClient {
    fn config(&self) -> &LlmConfig {
        &self.config
    }

    fn chat(&self, messages: &[Message]) -> anyhow::Result<LlmResponse> {
        let last = messages.last().map(|m| m.content.as_str()).unwrap_or("");
        Ok(LlmResponse {
            text: format!("[StubClient] You said: {last}"),
            tokens_in: 0,
            tokens_out: 0,
        })
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_client() {
        let client = StubClient::new();
        let resp = client
            .chat(&[Message::user("hello")])
            .unwrap();
        assert!(resp.text.contains("hello"));
    }

    #[test]
    fn test_message_constructors() {
        let sys = Message::system("be helpful");
        assert_eq!(sys.role, "system");
        let usr = Message::user("hi");
        assert_eq!(usr.role, "user");
        let asst = Message::assistant("hello");
        assert_eq!(asst.role, "assistant");
    }
}
