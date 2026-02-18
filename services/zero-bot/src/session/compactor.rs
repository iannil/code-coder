//! Context compaction using LLM summarization.

use super::types::{MessageRole, SessionMessage};
use crate::providers::Provider;
use anyhow::Result;
use std::fmt::Write;
use std::sync::Arc;

/// Compacts conversation history into a concise summary.
pub struct SessionCompactor {
    provider: Arc<dyn Provider>,
    model: String,
}

impl SessionCompactor {
    /// Create a new compactor with the given provider and model.
    pub fn new(provider: Arc<dyn Provider>, model: String) -> Self {
        Self { provider, model }
    }

    /// Compact messages into a summary.
    ///
    /// Returns a concise summary preserving key context, user intents,
    /// and important decisions.
    pub async fn compact(&self, messages: &[SessionMessage]) -> Result<String> {
        if messages.is_empty() {
            return Ok(String::new());
        }

        let history = Self::format_history(messages);

        let prompt = format!(
            "请将以下对话历史压缩为简洁的摘要，保留关键信息和上下文。\n\n\
             {history}\n\n\
             摘要要求：\n\
             1. 保留用户的核心意图和偏好\n\
             2. 保留关键决策和结论\n\
             3. 删除冗余的问候和过渡\n\
             4. 使用第三人称描述\n\
             5. 控制在 200 字以内\n\n\
             请直接输出摘要，不要添加任何前缀或解释。"
        );

        self.provider
            .chat_with_system(None, &prompt, &self.model, 0.3)
            .await
    }

    /// Format messages as conversation history for the LLM.
    fn format_history(messages: &[SessionMessage]) -> String {
        messages
            .iter()
            .map(|msg| {
                let role_label = match msg.role {
                    MessageRole::User => "用户",
                    MessageRole::Assistant => "助手",
                    MessageRole::System => "系统",
                };
                format!("{role_label}: {}", msg.content)
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// Format session messages for injection into the current prompt.
///
/// Returns a formatted conversation context string.
pub fn format_session_context(messages: &[SessionMessage]) -> String {
    if messages.is_empty() {
        return String::new();
    }

    let mut context = String::from("[对话历史]\n");

    for msg in messages {
        let role_label = match msg.role {
            MessageRole::User => "用户",
            MessageRole::Assistant => "助手",
            MessageRole::System => "背景摘要",
        };
        let _ = write!(context, "{role_label}: {}\n\n", msg.content);
    }

    context
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_messages() -> Vec<SessionMessage> {
        vec![
            SessionMessage::new(1, MessageRole::User, "你好".to_string(), 1000),
            SessionMessage::new(2, MessageRole::Assistant, "你好！有什么可以帮助你的吗？".to_string(), 1001),
            SessionMessage::new(3, MessageRole::User, "记住我喜欢 Rust".to_string(), 1002),
            SessionMessage::new(4, MessageRole::Assistant, "好的，我记住了你喜欢 Rust。".to_string(), 1003),
        ]
    }

    #[test]
    fn test_format_session_context_empty() {
        let context = format_session_context(&[]);
        assert!(context.is_empty());
    }

    #[test]
    fn test_format_session_context() {
        let messages = make_messages();
        let context = format_session_context(&messages);

        assert!(context.contains("[对话历史]"));
        assert!(context.contains("用户: 你好"));
        assert!(context.contains("助手: 你好！"));
        assert!(context.contains("用户: 记住我喜欢 Rust"));
    }

    #[test]
    fn test_format_session_context_with_system() {
        let messages = vec![
            SessionMessage::new(1, MessageRole::System, "之前讨论了 Rust".to_string(), 999),
            SessionMessage::new(2, MessageRole::User, "继续".to_string(), 1000),
        ];
        let context = format_session_context(&messages);

        assert!(context.contains("背景摘要: 之前讨论了 Rust"));
        assert!(context.contains("用户: 继续"));
    }
}
