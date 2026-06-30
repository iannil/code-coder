/// ─── AskUserTool ───────────────────────────────────────────────────────────
///
/// Ask the user a question during task execution.  The agent sends a
/// question, the user types an answer, and the tool returns the answer.
///
/// Uses a global pending-questions registry so the response can be matched
/// to the right request.
///
/// Input: {"question": "What version do you want?"}
/// Output: The user's answer (or timeout error).

use super::Tool;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;

static ASK_USER_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Registry of pending questions: request_id → sender for the answer.
static PENDING_QUESTIONS: std::sync::LazyLock<Mutex<HashMap<u64, mpsc::Sender<String>>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

/// The response channel to the TUI (set during tool execution).
static RESPONSE_TX: std::sync::LazyLock<Mutex<Option<mpsc::Sender<crate::agent::AgentResponse>>>> =
    std::sync::LazyLock::new(|| Mutex::new(None));

pub struct AskUserTool;

impl AskUserTool {
    /// Set the response channel used to send questions to the TUI.
    /// Called once when the agent spawns.
    pub fn set_response_tx(tx: mpsc::Sender<crate::agent::AgentResponse>) {
        *RESPONSE_TX.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(tx);
    }

    /// Deliver an answer to a pending question.
    pub fn deliver_answer(request_id: u64, answer: String) -> bool {
        let mut pending = PENDING_QUESTIONS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(tx) = pending.remove(&request_id) {
            let _ = tx.send(answer);
            true
        } else {
            false
        }
    }
}

impl Tool for AskUserTool {
    fn name(&self) -> &str {
        "ask_user"
    }

    fn description(&self) -> &str {
        "Ask the user a question. Input JSON: {\"question\":\"...\"}. Returns the user's answer."
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        #[derive(serde::Deserialize)]
        struct AskInput {
            question: String,
        }

        let parsed: AskInput = serde_json::from_str(input)
            .map_err(|e| anyhow::anyhow!("Invalid ask_user input: {e}"))?;

        let id = ASK_USER_COUNTER.fetch_add(1, Ordering::SeqCst);
        let (tx, rx) = mpsc::channel::<String>();

        // Register pending question
        {
            let mut pending = PENDING_QUESTIONS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            pending.insert(id, tx);
        }

        // Send to TUI
        {
            let resp_tx = RESPONSE_TX.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(ref tx) = *resp_tx {
                let _ = tx.send(crate::agent::AgentResponse::AskUser {
                    question: parsed.question,
                    request_id: id,
                });
            }
        }

        // Wait for answer (timeout: 300s)
        match rx.recv_timeout(std::time::Duration::from_secs(300)) {
            Ok(answer) => Ok(answer),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                PENDING_QUESTIONS.lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(&id);
                anyhow::bail!("[ask_user] Timed out waiting for user response")
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                anyhow::bail!("[ask_user] Response channel disconnected")
            }
        }
    }
}

/// ─── AgentTool ─────────────────────────────────────────────────────────────
///
/// Spawn a sub-agent to execute a task independently and return the result.

use crate::agent::AgentLoop;
use crate::context::Context;

pub struct AgentTool;

impl AgentTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for AgentTool {
    fn name(&self) -> &str {
        "agent"
    }

    fn description(&self) -> &str {
        "Spawn a sub-agent. Input JSON: {\"task\":\"Do X\", \"context\":\"optional context\"}. Returns the sub-agent's response."
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        #[derive(serde::Deserialize)]
        struct AgentInput {
            task: String,
            #[serde(default)]
            context: String,
        }

        let parsed: AgentInput = serde_json::from_str(input)
            .map_err(|e| anyhow::anyhow!("Invalid agent input: {e}"))?;

        let ctx = Context::load(".");
        let mut sub_agent = AgentLoop::new(
            // Clone the LLM by creating a new one from env
            Box::new(crate::llm::OpenAiClient::from_env()),
            ctx,
        );

        // Use a limited tool set (safe tools only)
        let mut sub_tools = crate::tools::ToolRegistry::new_for_test();
        // Also add read-only tools that are safe for sub-agents
        sub_tools.register(Box::new(crate::tools::read_file::ReadFile));
        sub_tools.register(Box::new(crate::tools::GlobTool));
        sub_tools.register(Box::new(crate::tools::Grep));
        sub_tools.register(Box::new(crate::tools::list_dir::ListDir));

        let task = if parsed.context.is_empty() {
            parsed.task
        } else {
            format!("Context:\n{}\n\nTask:\n{}", parsed.context, parsed.task)
        };

        // Run the sub-agent using the current tokio runtime
        let mut sub_skills = crate::skill::SkillRegistry::new();
        // ADR 0001 Phase B: sub-agent uses its own never-cancelled token.
        // The user's Ctrl+C applies to the parent request only — sub-agents
        // spawned via ask_agent complete their own work.
        let sub_cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let result = tokio::task::block_in_place(|| {
            let handle = tokio::runtime::Handle::current();
            handle.block_on(async {
                sub_agent.handle_message(&task, &sub_tools, &mut sub_skills, &|_, _| true, &sub_cancel).await
            })
        })?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── AskUserTool ─────────────────────────────────────────────────────

    #[test]
    fn test_ask_user_tool_name() {
        let tool = AskUserTool;
        assert_eq!(tool.name(), "ask_user");
    }

    #[test]
    fn test_ask_user_tool_description_not_empty() {
        let tool = AskUserTool;
        assert!(!tool.description().is_empty());
        assert!(tool.description().contains("user"));
    }

    #[test]
    fn test_ask_user_tool_execute_invalid_json() {
        let tool = AskUserTool;
        let result = tool.execute("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_ask_user_tool_execute_missing_question() {
        let tool = AskUserTool;
        let result = tool.execute(r#"{}"#);
        assert!(result.is_err());
    }

    #[test]
    fn test_ask_user_deliver_answer_nonexistent() {
        let result = AskUserTool::deliver_answer(99999, "answer".into());
        assert!(!result, "delivering to non-existent request should return false");
    }

    #[test]
    fn test_ask_user_tool_set_response_tx_and_answer() {
        let (tx, rx) = mpsc::channel();
        AskUserTool::set_response_tx(tx);

        let tool = AskUserTool;
        let input = r#"{"question":"What is your name?"}"#;

        let handle = std::thread::spawn(move || {
            let response = rx.recv().unwrap();
            match response {
                crate::agent::AgentResponse::AskUser { request_id, .. } => {
                    AskUserTool::deliver_answer(request_id, "Alice".into());
                }
                _ => panic!("Expected AskUser"),
            }
        });

        let result = tool.execute(input);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Alice");

        handle.join().unwrap();
    }

    // ─── AgentTool ────────────────────────────────────────────────────────

    #[test]
    fn test_agent_tool_new() {
        let tool = AgentTool::new();
        assert_eq!(tool.name(), "agent");
    }

    #[test]
    fn test_agent_tool_description_not_empty() {
        let tool = AgentTool::new();
        assert!(!tool.description().is_empty());
        assert!(tool.description().contains("sub-agent"));
    }

    #[test]
    fn test_agent_tool_execute_invalid_json() {
        let tool = AgentTool::new();
        let result = tool.execute("not json");
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_tool_execute_missing_task() {
        let tool = AgentTool::new();
        let result = tool.execute(r#"{}"#);
        assert!(result.is_err());
    }
}
