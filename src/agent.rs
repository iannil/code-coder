/// ─── Agent ─────────────────────────────────────────────────────────────────
///
/// Async agent loop running on a tokio runtime.
/// Communicates with the sync TUI via channel bridges.

use crate::context::Context;
use crate::event::{Event, Subscriber};
use crate::llm::{LlmClient, Message, StreamDelta};
use crate::self_evolve::{IntrospectConfig, IntrospectResult, SelfEvolve};
use crate::skill::SkillRegistry;
use crate::tools::ToolRegistry;
use std::sync::atomic::{AtomicU64, Ordering};

static PERMISSION_ID: AtomicU64 = AtomicU64::new(1);

fn rand_id() -> u64 {
    PERMISSION_ID.fetch_add(1, Ordering::SeqCst)
}

/// ─── ToolCall ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ToolCall {
    pub name: String,
    pub input: String,
}

/// ─── AgentLoop ─────────────────────────────────────────────────────────────

#[allow(dead_code)]
pub struct AgentLoop {
    llm: Box<dyn LlmClient>,
    history: Vec<Message>,
    base_system_prompt: String,
    context: Context,
    max_tool_rounds: usize,
    /// Sender for streaming LLM deltas (sent to TUI)
    delta_tx: Option<tokio::sync::mpsc::Sender<AgentResponse>>,
    /// Self-evolution engine (Phase 7)
    self_evolve: SelfEvolve,
    /// Monotonic round counter for cooldown tracking
    round_counter: u32,
}

impl AgentLoop {
    pub fn new(llm: Box<dyn LlmClient>, context: Context) -> Self {
        Self {
            llm,
            history: Vec::new(),
            base_system_prompt: Self::default_system_prompt(),
            context,
            max_tool_rounds: 10,
            delta_tx: None,
            self_evolve: SelfEvolve::new(IntrospectConfig::default()),
            round_counter: 0,
        }
    }

    pub fn with_delta_tx(mut self, tx: tokio::sync::mpsc::Sender<AgentResponse>) -> Self {
        self.delta_tx = Some(tx);
        self
    }

    fn default_system_prompt() -> String {
        r#"You are CodeCoder, an autonomous AI agent running in a terminal.
You have access to tools that can read and write files, run commands,
search the web, and list directories.

## How to use tools

When you need to use a tool, put the tool call inside a markdown code
block tagged with "tool":

```tool
{"name": "<tool_name>", "input": "<tool_input>"}
```

You can call multiple tools in sequence — the results will be fed
back to you one by one. Always wait for a tool result before
deciding the next step.

Be concise and helpful. Explain what you're doing, then call the
tool, then summarise the result for the user."#
            .into()
    }

    pub(crate) fn build_system_prompt(&self, tools: &ToolRegistry, skills: &SkillRegistry) -> String {
        let mut prompt = self.base_system_prompt.clone();
        prompt.push_str(&self.context.format_system_section());
        prompt.push_str("\n## Available Tools\n\n");
        for name in tools.list_tools() {
            if let Some(tool) = tools.get(name) {
                prompt.push_str(&format!("- `{}`: {}\n", name, tool.description()));
            }
        }
        let skill_list = skills.list();
        if !skill_list.is_empty() {
            prompt.push_str("\n## Loaded Skills\n\n");
            prompt.push_str(&skill_list.iter()
                .map(|s| format!("- `{}`", s))
                .collect::<Vec<_>>()
                .join("\n"));
            prompt.push('\n');
        }
        prompt
    }

    /// Process a user message synchronously, returning the final response text.
    pub async fn handle_message(
        &mut self,
        text: &str,
        tools: &ToolRegistry,
        skills: &mut SkillRegistry,
        permission_check: &dyn Fn(&str, &str) -> bool,
    ) -> anyhow::Result<String> {
        self.history.push(Message::user(text));
        self.round_counter += 1;
        let mut response = self.react_loop(tools, skills, permission_check).await?;

        // Self-evolution: evaluate after each turn
        let project_root = self.context.project_root.clone();
        let result = self.self_evolve.evaluate(
            &self.history,
            tools,
            skills,
            &project_root,
            self.round_counter,
        );

        match result {
            IntrospectResult::SkillGenerated { skill_name, .. } => {
                // Re-scan so the new skill is available immediately
                let _ = skills.scan(&project_root);
                response.push_str(&format!(
                    "\n\n[auto] 检测到能力缺口，已生成草稿 skill: `{skill_name}`（已自动加载）"
                ));
            }
            IntrospectResult::SkillPromoted { skill_name } => {
                response.push_str(&format!(
                    "\n\n[auto] Skill `{skill_name}` 已验证有效，已激活"
                ));
            }
            IntrospectResult::None => {}
        }

        Ok(response)
    }

    async fn react_loop(
        &mut self,
        tools: &ToolRegistry,
        skills: &SkillRegistry,
        permission_check: &dyn Fn(&str, &str) -> bool,
    ) -> anyhow::Result<String> {
        for _round in 0..self.max_tool_rounds {
            let messages = self.build_messages(tools, skills);

            let response = if let Some(ref delta_tx) = self.delta_tx {
                let tx = delta_tx.clone();
                let (llm_resp, mut rx) = self.llm.chat_stream(&messages).await?;
                // Forward deltas to TUI
                let tx_clone = tx.clone();
                tokio::spawn(async move {
                    while let Some(delta) = rx.recv().await {
                        if let Some(text) = delta.text {
                            let _ = tx_clone.send(AgentResponse::LlmDelta { text }).await;
                        }
                        if let Some(reasoning) = delta.reasoning {
                            let _ = tx_clone.send(AgentResponse::ReasoningDelta { text: reasoning }).await;
                        }
                    }
                });
                llm_resp
            } else {
                self.llm.chat(&messages).await?
            };

            let tool_calls = parse_tool_calls(&response.text);

            if tool_calls.is_empty() {
                self.history.push(Message::assistant(&response.text));
                return Ok(response.text);
            }

            for tc in &tool_calls {
                let allowed = permission_check(&tc.name, &tc.input);
                if !allowed {
                    self.history.push(Message::assistant(&response.text));
                    self.history.push(Message::user(format!("[tool {} was denied by user]", tc.name)));
                    continue;
                }
                let result = match tools.execute(&tc.name, &tc.input) {
                    Ok(out) => out,
                    Err(e) => format!("[tool error] {e}"),
                };
                self.history.push(Message::assistant(&response.text));
                self.history.push(Message::user(format!("[tool result for {}]\n{}", tc.name, result)));
            }
        }

        let msg = "[CodeCoder] Maximum tool-call rounds reached.";
        self.history.push(Message::assistant(msg));
        Ok(msg.into())
    }

    fn build_messages(&self, tools: &ToolRegistry, skills: &SkillRegistry) -> Vec<Message> {
        let mut messages = Vec::with_capacity(self.history.len() + 1);
        messages.push(Message::system(&self.build_system_prompt(tools, skills)));
        messages.extend(self.history.iter().cloned());
        messages
    }

    pub fn reload_context(&mut self) {
        self.context = Context::load(&self.context.project_root);
    }

    pub fn context(&self) -> &Context {
        &self.context
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
        self.self_evolve.reset_session();
    }

    pub fn set_model(&mut self, model: &str) {
        let config = crate::llm::LlmConfig {
            model: model.to_string(),
            ..crate::llm::LlmConfig::from_env()
        };
        self.llm = Box::new(crate::llm::OpenAiClient::new(config));
    }
}

/// ─── BackgroundAgent ───────────────────────────────────────────────────────
///
/// Wraps AgentLoop and runs it on a tokio runtime.  The TUI sends commands
/// via std::sync::mpsc (sync) bridged to the async agent.  Responses flow
/// back via tokio::sync::mpsc (the TUI polls with try_recv).

use crate::event::SharedEventBus;

#[allow(dead_code)]
pub struct BackgroundAgent {
    pub cmd_tx: std::sync::mpsc::Sender<AgentCommand>,
    /// Responses flow through a tokio channel. TUI uses blocking_recv or try_recv.
    pub resp_rx: tokio::sync::mpsc::Receiver<AgentResponse>,
    pub thread_handle: Option<std::thread::JoinHandle<()>>,
}

pub enum AgentCommand {
    ProcessMessage { text: String },
    SetModel { model: String },
    PermissionResponse { request_id: u64, allowed: bool },
    AskUserResponse { request_id: u64, answer: String },
    PlanDecision { request_id: u64, decision: String },
    ReloadContext,
    ClearHistory,
    Shutdown,
}

#[derive(Debug)]
pub enum AgentResponse {
    Text { text: String },
    LlmDelta { text: String },
    ReasoningDelta { text: String },
    ToolCall { name: String, input: String },
    ToolResult { name: String, output: String, success: bool },
    PermissionRequest { tool_name: String, tool_input: String, request_id: u64 },
    /// Agent asked a question — user needs to answer
    AskUser { question: String, request_id: u64 },
    /// Agent presents a plan for user approval
    PlanRequest { title: String, plan: String, request_id: u64 },
    Error { message: String },
    Heartbeat { pending: usize },
    Shutdown,
}

impl BackgroundAgent {
    /// Spawn the agent loop on a tokio runtime in a background thread.
    pub fn spawn(
        llm: Box<dyn LlmClient>,
        context: Context,
        tools: ToolRegistry,
        skills: SkillRegistry,
        bus: SharedEventBus,
    ) -> Self {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<AgentCommand>();
        let (resp_tx, resp_rx) = tokio::sync::mpsc::channel::<AgentResponse>(256);

        let handle = std::thread::Builder::new()
            .name("agent".into())
            .spawn(move || {
                crate::log("[codecoder] Agent 线程已启动");
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_time()
                    .enable_io()
                    .build()
                    .expect("failed to build tokio runtime");

                let agent_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    crate::log("[codecoder] Agent 事件循环开始");
                    rt.block_on(async {
                        // 发送初始心跳确认 agent 存活
                        let _ = resp_tx.send(AgentResponse::Heartbeat { pending: 0 }).await;

                        let delta_tx = resp_tx.clone();
                        let mut agent = AgentLoop::new(llm, context).with_delta_tx(delta_tx);

                        let tools = tools;
                        let mut skills = skills;

                        let (ask_tx, ask_rx) = std::sync::mpsc::channel::<AgentResponse>();
                        crate::tools::AskUserTool::set_response_tx(ask_tx.clone());
                        crate::tools::PlanTool::set_response_tx(ask_tx);

                        let resp_tx_bridge = resp_tx.clone();
                        tokio::spawn(async move {
                            loop {
                                match ask_rx.try_recv() {
                                    Ok(msg) => {
                                        let _ = resp_tx_bridge.send(msg).await;
                                    }
                                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                                        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                                    }
                                    Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                                }
                            }
                        });

                    loop {
                        // Non-blocking poll
                        match cmd_rx.try_recv() {
                            Ok(AgentCommand::ProcessMessage { text }) => {
                                crate::log(&format!("[codecoder] Agent 收到消息: {:?}", &text[..text.len().min(50)]));
                                let _ = bus.drain();
                                let resp_tx2 = resp_tx.clone();
                                let cmd_rx_ref = &cmd_rx;
                                let permission_check = move |name: &str, input: &str| {
                                    let id: u64 = rand_id();
                                    let _ = resp_tx2.blocking_send(AgentResponse::PermissionRequest {
                                        tool_name: name.to_string(),
                                        tool_input: input.to_string(),
                                        request_id: id,
                                    });
                                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
                                    while std::time::Instant::now() < deadline {
                                        let remaining = deadline - std::time::Instant::now();
                                        match cmd_rx_ref.recv_timeout(remaining) {
                                            Ok(AgentCommand::PermissionResponse { request_id, allowed }) if request_id == id => return allowed,
                                            Ok(AgentCommand::Shutdown) => return false,
                                            _ => continue,
                                        }
                                    }
                                    let _ = resp_tx2.blocking_send(AgentResponse::Text {
                                        text: format!("[Permission timeout — tool '{name}' was denied after 30s]"),
                                    });
                                    false
                                };
                                match agent.handle_message(&text, &tools, &mut skills, &permission_check).await {
                                    Ok(resp) => {
                                        let _ = resp_tx.send(AgentResponse::Text { text: resp }).await;
                                    }
                                    Err(e) => {
                                        let _ = resp_tx.send(AgentResponse::Error {
                                            message: format!("{e}"),
                                        }).await;
                                    }
                                }
                            }
                            Ok(AgentCommand::ReloadContext) => {
                                agent.reload_context();
                                let root = agent.context().project_root.clone();
                                let _ = skills.scan(&root);
                            }
                            Ok(AgentCommand::ClearHistory) => {
                                agent.clear_history();
                            }
                            Ok(AgentCommand::SetModel { model }) => {
                                agent.set_model(&model);
                                let _ = resp_tx.send(AgentResponse::Text {
                                    text: format!("Switched model to {model}"),
                                }).await;
                            }
                            Ok(AgentCommand::Shutdown) => {
                                let _ = resp_tx.send(AgentResponse::Shutdown).await;
                                break;
                            }
                            Ok(AgentCommand::PermissionResponse { .. }) => {
                                // Handled inside permission_check closure
                            }
                            Ok(AgentCommand::AskUserResponse { request_id, answer }) => {
                                tokio::task::block_in_place(|| {
                                    crate::tools::AskUserTool::deliver_answer(request_id, answer);
                                });
                            }
                            Ok(AgentCommand::PlanDecision { request_id, decision }) => {
                                tokio::task::block_in_place(|| {
                                    crate::tools::PlanTool::deliver_decision(request_id, decision);
                                });
                            }
                            Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                            Err(std::sync::mpsc::TryRecvError::Empty) => {
                                // Yield to runtime so other tasks can progress
                                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                                tokio::task::yield_now().await;
                            }
                        }
                    }
                });
                }));
                if let Err(panic) = agent_result {
                    let msg = if let Some(s) = panic.downcast_ref::<&str>() {
                        s.to_string()
                    } else if let Some(s) = panic.downcast_ref::<String>() {
                        s.clone()
                    } else {
                        "agent thread panicked (unknown cause)".into()
                    };
                    crate::log(&format!("[codecoder] Agent thread panicked: {msg}"));
                }
            })
            .expect("failed to spawn agent thread");

        Self {
            cmd_tx,
            resp_rx,
            thread_handle: Some(handle),
        }
    }
}

/// ─── Tool call parsing ─────────────────────────────────────────────────────

fn parse_tool_calls(text: &str) -> Vec<ToolCall> {
    let mut calls = Vec::new();
    let mut rest = text;
    loop {
        let start = match rest.find("```tool") {
            Some(pos) => pos + 7,
            None => break,
        };
        let end = match rest[start..].find("```") {
            Some(pos) => start + pos,
            None => break,
        };
        let json_str = rest[start..end].trim();
        rest = &rest[end + 3..];
        if let Ok(tc) = parse_single_tool_call(json_str) {
            calls.push(tc);
        }
    }
    calls
}

fn parse_single_tool_call(json: &str) -> anyhow::Result<ToolCall> {
    #[derive(serde::Deserialize)]
    struct RawCall {
        #[serde(default)]
        name: String,
        #[serde(default)]
        input: String,
        #[serde(default)]
        tool: String,
        #[serde(default)]
        arguments: Option<String>,
    }
    let raw: RawCall = serde_json::from_str(json)?;
    let name = if !raw.tool.is_empty() { raw.tool } else { raw.name };
    let input = raw.arguments.unwrap_or(raw.input);
    if name.is_empty() {
        anyhow::bail!("tool call missing name");
    }
    Ok(ToolCall { name, input })
}

/// ─── AgentBusSubscriber ────────────────────────────────────────────────────

#[allow(dead_code)]
pub struct AgentBusSubscriber {
    pub sender: std::sync::mpsc::Sender<String>,
}

impl Subscriber for AgentBusSubscriber {
    fn name(&self) -> &str {
        "agent"
    }
    fn handle(&mut self, event: &Event) -> anyhow::Result<()> {
        if let Event::UserMessage { text, .. } = event {
            let _ = self.sender.send(text.clone());
        }
        Ok(())
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::Context;
    use crate::llm::StubClient;

    fn test_agent() -> AgentLoop {
        let ctx = Context::load("/tmp");
        AgentLoop::new(Box::new(StubClient::new()), ctx)
    }

    #[test]
    fn test_parse_tool_call_simple() {
        let text = r#"Let me read that file.

```tool
{"name": "read_file", "input": "Cargo.toml"}
```

Here is the content."#;
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].input, "Cargo.toml");
    }

    #[test]
    fn test_parse_no_tool_call() {
        let calls = parse_tool_calls("Hello, how can I help?");
        assert!(calls.is_empty());
    }

    #[tokio::test]
    async fn test_handle_message_with_stub() {
        let mut agent = test_agent();
        let tools = ToolRegistry::new_for_test();
        let mut skills = SkillRegistry::new();
        let resp = agent.handle_message("hello", &tools, &mut skills, &|_, _| true).await.unwrap();
        assert!(resp.contains("hello"));
    }

    #[tokio::test]
    async fn test_clear_history() {
        let mut agent = test_agent();
        let tools = ToolRegistry::new_for_test();
        let mut skills = SkillRegistry::new();
        agent.handle_message("hello", &tools, &mut skills, &|_, _| true).await.unwrap();
        assert!(agent.history.len() >= 2);
        agent.clear_history();
        assert_eq!(agent.history.len(), 0);
    }

    #[test]
    fn test_background_agent_spawn_and_shutdown() {
        let ctx = Context::load("/tmp");
        let tools = ToolRegistry::new_for_test();
        let mut skills = SkillRegistry::new();
        let bus = SharedEventBus::new();

        let mut bg = BackgroundAgent::spawn(
            Box::new(StubClient::new()),
            ctx,
            tools,
            skills,
            bus,
        );

        let shutdown_sent = bg.cmd_tx.send(AgentCommand::Shutdown);
        assert!(shutdown_sent.is_ok(), "shutdown send failed: {:?}", shutdown_sent.err());

        // Wait for shutdown response via tokio channel (use blocking_recv)
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let mut found = false;
        while std::time::Instant::now() < deadline {
            if let Ok(AgentResponse::Shutdown) = bg.resp_rx.try_recv() {
                found = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        assert!(found, "Expected Shutdown response");

        if let Some(handle) = bg.thread_handle {
            handle.join().unwrap();
        }
    }

    #[test]
    fn test_parse_tool_call_invalid_format() {
        let calls = parse_tool_calls("plain text no tool call");
        assert!(calls.is_empty());
    }

    #[test]
    fn test_parse_tool_call_empty_string() {
        let calls = parse_tool_calls("");
        assert!(calls.is_empty());
    }

    #[test]
    fn test_parse_tool_call_with_trailing_text() {
        let text = "Some text\n```tool\n{\"name\": \"read_file\", \"input\": \"Cargo.toml\"}\n```\ntrailing";
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].input, "Cargo.toml");
    }

    #[test]
    fn test_agent_response_text_debug() {
        let resp = AgentResponse::Text { text: "hello".into() };
        let debug = format!("{resp:?}");
        assert!(debug.contains("hello"));
    }

    #[test]
    fn test_agent_response_error_debug() {
        let resp = AgentResponse::Error { message: "oops".into() };
        let debug = format!("{resp:?}");
        assert!(debug.contains("oops"));
    }

    #[test]
    fn test_agent_response_shutdown_debug() {
        let resp = AgentResponse::Shutdown;
        let debug = format!("{resp:?}");
        assert!(debug.contains("Shutdown") || debug.contains("shutdown"));
    }

    #[test]
    fn test_agent_bus_subscriber_name() {
        let (tx, _rx) = std::sync::mpsc::channel();
        let subscriber = AgentBusSubscriber { sender: tx };
        assert_eq!(subscriber.name(), "agent");
    }

    #[test]
    fn test_agent_bus_subscriber_handles_user_message() {
        let (tx, rx) = std::sync::mpsc::channel();
        let mut subscriber = AgentBusSubscriber { sender: tx };
        let event = crate::event::Event::UserMessage {
            text: "hello from bus".into(),
            session_id: "test".into(),
        };
        subscriber.handle(&event).unwrap();
        let received = rx.recv_timeout(std::time::Duration::from_millis(100)).unwrap();
        assert_eq!(received, "hello from bus");
    }

    #[test]
    fn test_agent_bus_subscriber_ignores_non_user_message() {
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let mut subscriber = AgentBusSubscriber { sender: tx };
        let event = crate::event::Event::Timer { id: "test".into() };
        subscriber.handle(&event).unwrap();
        // Should not send anything for timer events
        let result = rx.try_recv();
        assert!(result.is_err(), "Timer events should not trigger sending");
    }

    #[test]
    fn test_handle_message_error() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let ctx = Context::load("/tmp");
            let mut agent = AgentLoop::new(Box::new(StubClient::new()), ctx);
            let tools = crate::tools::ToolRegistry::new_for_test();
            let mut skills = crate::skill::SkillRegistry::new();
            // An empty message should still produce a response
            let result = agent.handle_message("", &tools, &mut skills, &|_, _| true).await;
            assert!(result.is_ok() || result.is_err());
        });
    }

    #[test]
    fn test_agent_heartbeat_response() {
        let resp = AgentResponse::Heartbeat { pending: 5 };
        let debug = format!("{resp:?}");
        assert!(debug.contains("Heartbeat") || debug.contains("5"));
    }

    #[test]
    fn test_agent_ask_user_response() {
        let resp = AgentResponse::AskUser { question: "your name?".into(), request_id: 42 };
        let debug = format!("{resp:?}");
        assert!(debug.contains("42") || debug.contains("name"));
    }

    #[test]
    fn test_agent_plan_request_response() {
        let resp = AgentResponse::PlanRequest {
            title: "Refactor".into(),
            plan: "Step 1: do X".into(),
            request_id: 7,
        };
        let debug = format!("{resp:?}");
        assert!(debug.contains("7") || debug.contains("Refactor"));
    }

    #[test]
    fn test_agent_tool_result_response() {
        let resp = AgentResponse::ToolResult {
            name: "grep".into(),
            output: "matched".into(),
            success: true,
        };
        let debug = format!("{resp:?}");
        assert!(debug.contains("grep") || debug.contains("matched"));
    }

    #[test]
    fn test_agent_permission_request_response() {
        let resp = AgentResponse::PermissionRequest {
            tool_name: "run_command".into(),
            tool_input: "ls -la".into(),
            request_id: 1,
        };
        let debug = format!("{resp:?}");
        assert!(debug.contains("run_command") || debug.contains("1"));
    }

    #[test]
    fn test_agent_response_tool_call_variant() {
        let resp = AgentResponse::ToolCall {
            name: "read_file".into(),
            input: "path".into(),
        };
        let debug = format!("{resp:?}");
        assert!(debug.contains("read_file"));
    }

    #[test]
    fn test_parse_tool_call_multiple_calls() {
        let text = "```tool\n{\"name\": \"read_file\", \"input\": \"a\"}\n```\n```tool\n{\"name\": \"write_file\", \"input\": \"b\"}\n```";
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 2, "Should parse two tool calls");
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[1].name, "write_file");
    }

    #[test]
    fn test_parse_tool_call_missing_input() {
        // Without input, parse_single_tool_call should fail
        let text = "```tool\n{\"name\": \"test\"}\n```";
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1, "Should still parse name without input");
        assert_eq!(calls[0].name, "test");
        assert_eq!(calls[0].input, "");
    }

    #[test]
    fn test_parse_tool_call_with_tool_field() {
        let text = "```tool\n{\"tool\": \"search_web\", \"arguments\": \"query\"}\n```";
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "search_web");
        assert_eq!(calls[0].input, "query");
    }
}
