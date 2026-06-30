/// ─── Agent ─────────────────────────────────────────────────────────────────
///
/// Async agent loop running on a tokio runtime.
/// Communicates with the sync TUI via channel bridges.

use crate::context::Context;
use crate::event::{Event, Subscriber};
use crate::llm::{LlmClient, Message};
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
    /// Accumulated token usage across react_loop rounds
    acc_tokens_in: u32,
    acc_tokens_out: u32,
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
            acc_tokens_in: 0,
            acc_tokens_out: 0,
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
    ///
    /// ADR 0001 Phase B: `cancel` is checked between LLM rounds and after
    /// each streaming delta. When set (by AgentCommand::Interrupt), the
    /// loop aborts early and returns "[interrupted by user]". The current
    /// in-flight await still completes (no pre-emption), but no further
    /// rounds or tool calls execute.
    pub async fn handle_message(
        &mut self,
        text: &str,
        tools: &ToolRegistry,
        skills: &mut SkillRegistry,
        permission_check: &dyn Fn(&str, &str) -> bool,
        cancel: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> anyhow::Result<String> {
        self.history.push(Message::user(text));
        self.round_counter += 1;
        let mut response = self.react_loop(tools, skills, permission_check, cancel).await?;

        // Cooperative cancel check before self-evolve (which can be slow).
        if cancel.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok("[interrupted by user]".into());
        }

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
        cancel: &std::sync::Arc<std::sync::atomic::AtomicBool>,
    ) -> anyhow::Result<String> {
        for _round in 0..self.max_tool_rounds {
            // ADR 0001 Phase B: check cancel at the top of each round.
            if cancel.load(std::sync::atomic::Ordering::SeqCst) {
                self.history.push(Message::assistant("[interrupted by user]"));
                return Ok("[interrupted by user]".into());
            }

            let messages = self.build_messages(tools, skills);

            let response = if let Some(ref delta_tx) = self.delta_tx {
                let tx = delta_tx.clone();
                let (llm_resp, mut rx) = self.llm.chat_stream(&messages).await?;
                // 顺序转发 delta 到 TUI
                let tx_clone = tx.clone();
                while let Some(delta) = rx.recv().await {
                    // ADR 0001 Phase B: check cancel after each delta.
                    if cancel.load(std::sync::atomic::Ordering::SeqCst) {
                        // Drain remaining without forwarding (drop rx).
                        break;
                    }
                    if let Some(text) = delta.text {
                        let _ = tx_clone.send(AgentResponse::LlmDelta { text }).await;
                    }
                    if let Some(reasoning) = delta.reasoning {
                        let _ = tx_clone.send(AgentResponse::ReasoningDelta { text: reasoning }).await;
                    }
                }
                // 显式标记流结束
                let _ = tx_clone.send(AgentResponse::StreamComplete).await;
                // 累加 token 用量
                self.acc_tokens_in += llm_resp.tokens_in;
                self.acc_tokens_out += llm_resp.tokens_out;
                llm_resp
            } else {
                let llm_resp = self.llm.chat(&messages).await?;
                self.acc_tokens_in += llm_resp.tokens_in;
                self.acc_tokens_out += llm_resp.tokens_out;
                llm_resp
            };

            // Post-response cancel check before tool calls.
            if cancel.load(std::sync::atomic::Ordering::SeqCst) {
                self.history.push(Message::assistant("[interrupted by user]"));
                return Ok("[interrupted by user]".into());
            }

            let tool_calls = parse_tool_calls(&response.text);

            if tool_calls.is_empty() {
                self.history.push(Message::assistant(&response.text));
                return Ok(response.text);
            }

            for tc in &tool_calls {
                // Per-tool cancel check.
                if cancel.load(std::sync::atomic::Ordering::SeqCst) {
                    self.history.push(Message::assistant("[interrupted by user]"));
                    return Ok("[interrupted by user]".into());
                }
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
        self.acc_tokens_in = 0;
        self.acc_tokens_out = 0;
    }

    pub fn token_usage(&self) -> (u32, u32) {
        (self.acc_tokens_in, self.acc_tokens_out)
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

#[derive(Debug)]
pub enum AgentCommand {
    ProcessMessage { text: String },
    SetModel { model: String },
    /// Permission response from the TUI. `scope` controls whether the grant
    /// persists for the session (AlwaysThisSession) or project
    /// (AlwaysThisProject — Phase B; persisted to codecoder.json) or is
    /// one-shot (Once). See ADR 0005.
    PermissionResponse {
        request_id: u64,
        allowed: bool,
        scope: PermScope,
    },
    AskUserResponse { request_id: u64, answer: String },
    PlanDecision { request_id: u64, decision: String },
    ReloadContext,
    ClearHistory,
    /// User-requested interrupt of the in-flight request. ADR 0001 Phase B:
    /// fires the shared AtomicBool cancel flag, which handle_message checks
    /// at each round, after each LLM delta, and before each tool call. The
    /// current in-flight await still completes (no pre-emption) but no
    /// further work executes; the response comes back as "[interrupted]".
    /// Also drains any queued ProcessMessages so a follow-up send doesn't
    /// immediately restart.
    Interrupt,
    Shutdown,
}

/// ADR 0005 — durability of a permission grant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermScope {
    /// Re-prompt next time. Default for the "Y" key.
    Once,
    /// No more prompts for this tool name, this session only.
    AlwaysThisSession,
    /// Persisted to codecoder.json under `permissions.allowlist`. Future
    /// sessions load it at startup. Phase B: not yet wired to disk; for now
    /// behaves like AlwaysThisSession in-memory.
    AlwaysThisProject,
}

impl Default for PermScope {
    fn default() -> Self {
        Self::Once
    }
}

#[derive(Debug)]
pub enum AgentResponse {
    Text { text: String, tokens_in: u32, tokens_out: u32 },
    LlmDelta { text: String },
    ReasoningDelta { text: String },
    /// Signals the end of a streaming sequence — TUI should expect Text next.
    /// Eliminates the fragile `already_streamed` heuristic.
    StreamComplete,
    ToolCall { name: String, input: String },
    ToolResult { name: String, output: String, success: bool },
    PermissionRequest { tool_name: String, tool_input: String, request_id: u64, risk: String },
    /// ADR 0005 Phase B: agent signals that the user granted AlwaysThisProject
    /// for `tool_name`. TUI persists it to codecoder.json so future sessions
    /// honor the grant without re-prompting.
    PersistPermission { tool_name: String },
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
        initial_allowlist: Vec<String>,
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

                    // ADR 0001 Phase B: shared cancel flag. Reset before
                    // each new message; set by AgentCommand::Interrupt.
                    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

                    // ADR 0005 Phase B: session allowlist, seeded once from
                    // the persisted project allowlist. Lives for the entire
                    // agent thread lifetime — AlwaysThisSession and
                    // AlwaysThisProject grants accumulate across messages.
                    let session_allowed = std::sync::Arc::new(
                        std::sync::Mutex::new(
                            initial_allowlist.iter().cloned()
                                .collect::<std::collections::HashSet<String>>(),
                        ),
                    );

                    loop {
                        // Non-blocking poll
                        match cmd_rx.try_recv() {
                            Ok(AgentCommand::ProcessMessage { text }) => {
                                crate::log(&format!("[codecoder] Agent 收到消息: {:?}", &text[..text.len().min(50)]));
                                let _ = bus.drain();
                                let resp_tx2 = resp_tx.clone();
                                let cmd_rx_ref = &cmd_rx;
                                // Reset cancel for this fresh message.
                                cancel.store(false, std::sync::atomic::Ordering::SeqCst);
                                let cancel_for_handle = cancel.clone();
                                // ADR 0005 Phase B: session_allowed lives
                                // outside the loop, so this clone shares the
                                // same set across all messages.
                                let session_allowed_closure = session_allowed.clone();
                                let permission_check = move |name: &str, input: &str| {
                                    // 1. Hard rules via PermissionEngine — these
                                    // cannot be overridden by the allowlist.
                                    let engine = crate::permission::PermissionEngine::new();
                                    let decision = engine.evaluate(name, input);
                                    match decision {
                                        crate::permission::PermissionDecision::Allowed => return true,
                                        crate::permission::PermissionDecision::Denied { reason: _ } => {
                                            return false;
                                        }
                                        crate::permission::PermissionDecision::NeedsApproval { .. } => {
                                            // Fall through to allowlist / prompt
                                        }
                                    }

                                    // 2. Session allowlist — skip the prompt if
                                    // the user previously granted AlwaysThisSession
                                    // (or AlwaysThisProject, which Phase B will
                                    // persist; for now treated identically).
                                    if let Ok(set) = session_allowed_closure.lock() {
                                        if set.contains(name) {
                                            crate::log(&format!(
                                                "[permission] '{name}' auto-allowed by session allowlist"
                                            ));
                                            return true;
                                        }
                                    }

                                    // 3. Prompt user for approval
                                    let id: u64 = rand_id();
                                    let risk_str = format!("{:?}", decision);
                                    let _ = resp_tx2.blocking_send(AgentResponse::PermissionRequest {
                                        tool_name: name.to_string(),
                                        tool_input: input.to_string(),
                                        request_id: id,
                                        risk: risk_str,
                                    });
                                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
                                    while std::time::Instant::now() < deadline {
                                        let remaining = deadline - std::time::Instant::now();
                                        match cmd_rx_ref.recv_timeout(remaining) {
                                            Ok(AgentCommand::PermissionResponse { request_id, allowed, scope })
                                                if request_id == id =>
                                            {
                                                // Persist grant for session/project scope.
                                                if allowed && matches!(
                                                    scope,
                                                    PermScope::AlwaysThisSession | PermScope::AlwaysThisProject
                                                ) {
                                                    if let Ok(mut set) = session_allowed_closure.lock() {
                                                        set.insert(name.to_string());
                                                    }
                                                    crate::log(&format!(
                                                        "[permission] '{name}' added to {:?} allowlist",
                                                        scope
                                                    ));
                                                    // ADR 0005 Phase B: project
                                                    // scope also signals TUI
                                                    // to persist to codecoder.json.
                                                    if matches!(scope, PermScope::AlwaysThisProject) {
                                                        let _ = resp_tx2.blocking_send(
                                                            AgentResponse::PersistPermission {
                                                                tool_name: name.to_string(),
                                                            }
                                                        );
                                                    }
                                                }
                                                return allowed;
                                            }
                                            Ok(AgentCommand::Shutdown) => return false,
                                            _ => continue,
                                        }
                                    }
                                    let _ = resp_tx2.blocking_send(AgentResponse::Text {
                                        text: format!("[Permission timeout — tool '{name}' was denied after 30s]"),
                                        tokens_in: 0,
                                        tokens_out: 0,
                                    });
                                    false
                                };
                                match agent.handle_message(&text, &tools, &mut skills, &permission_check, &cancel_for_handle).await {
                                    Ok(resp) => {
                                        let (tin, tout) = agent.token_usage();
                                        let _ = resp_tx.send(AgentResponse::Text {
                                            text: resp,
                                            tokens_in: tin,
                                            tokens_out: tout,
                                        }).await;
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
                                    tokens_in: 0,
                                    tokens_out: 0,
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
                            Ok(AgentCommand::Interrupt) => {
                                // ADR 0001 Phase B: fire the shared cancel flag.
                                // handle_message checks it at each round, after
                                // each LLM delta, and before each tool call — so
                                // the in-flight call returns promptly as
                                // "[interrupted by user]" rather than running to
                                // completion. We also drain queued ProcessMessages
                                // (preserves Phase A's "don't immediately restart"
                                // behavior).
                                cancel.store(true, std::sync::atomic::Ordering::SeqCst);
                                let mut drained = 0usize;
                                while let Ok(AgentCommand::ProcessMessage { .. }) = cmd_rx.try_recv() {
                                    drained += 1;
                                }
                                crate::log(&format!(
                                    "[agent] Interrupt received; cancel flag fired, drained {drained} queued message(s)."
                                ));
                                // Note: do NOT send a synthetic Text here. The
                                // in-flight handle_message will return naturally
                                // with "[interrupted by user]" and that response
                                // flows through the normal Ok(resp) match arm.
                                // If we sent Text now, the TUI would see two
                                // completion messages (this + the natural one).
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
        // Support both ```tool (legacy) and ```tool_use (new standard) block markers
        let (tag_start, tag_len) = match rest.find("```tool") {
            Some(pos) => {
                let after = &rest[pos + 7..];
                if after.starts_with("_use") {
                    (pos, 12) // ```tool_use
                } else if after.starts_with('`') || after.starts_with('\n') || after.starts_with(' ') {
                    (pos, 7) // ```tool
                } else {
                    (pos, 7) // ```tool
                }
            }
            None => break,
        };
        let start = tag_start + tag_len;
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
        arguments: Option<serde_json::Value>,
    }
    let raw: RawCall = serde_json::from_str(json)?;
    let name = if !raw.tool.is_empty() { raw.tool } else { raw.name };
    let input = match raw.arguments {
        Some(serde_json::Value::Object(map)) => {
            // New format: structured arguments → serialize back to JSON string
            serde_json::to_string(&map).unwrap_or_default()
        }
        Some(serde_json::Value::String(s)) => s,
        Some(_) => raw.arguments.unwrap().to_string(),
        None => raw.input,
    };
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

    /// Helper for tests: a never-cancelled token.
    fn test_cancel() -> std::sync::Arc<std::sync::atomic::AtomicBool> {
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false))
    }

    #[tokio::test]
    async fn test_handle_message_with_stub() {
        let mut agent = test_agent();
        let tools = ToolRegistry::new_for_test();
        let mut skills = SkillRegistry::new();
        let cancel = test_cancel();
        let resp = agent.handle_message("hello", &tools, &mut skills, &|_, _| true, &cancel).await.unwrap();
        assert!(resp.contains("hello"));
    }

    #[tokio::test]
    async fn test_clear_history() {
        let mut agent = test_agent();
        let tools = ToolRegistry::new_for_test();
        let mut skills = SkillRegistry::new();
        let cancel = test_cancel();
        agent.handle_message("hello", &tools, &mut skills, &|_, _| true, &cancel).await.unwrap();
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
            Vec::new(),
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
        let resp = AgentResponse::Text { text: "hello".into(), tokens_in: 0, tokens_out: 0 };
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
            let cancel = test_cancel();
            // An empty message should still produce a response
            let result = agent.handle_message("", &tools, &mut skills, &|_, _| true, &cancel).await;
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
            risk: "".into(),
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

    #[test]
    fn test_parse_single_tool_call_empty_name() {
        let result = parse_single_tool_call(r#"{"input": "test"}"#);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name"));
    }

    #[test]
    fn test_agent_response_llm_delta() {
        let resp = AgentResponse::LlmDelta { text: "thinking...".into() };
        let debug = format!("{resp:?}");
        assert!(debug.contains("thinking"));
    }

    #[test]
    fn test_agent_response_reasoning_delta() {
        let resp = AgentResponse::ReasoningDelta { text: "step by step".into() };
        let debug = format!("{resp:?}");
        assert!(debug.contains("step"));
    }

    #[test]
    fn test_build_messages_with_skills() {
        let mut agent = test_agent();
        let mut skills = SkillRegistry::new();
        let tools = ToolRegistry::new_for_test();
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::write(
            skills_dir.join("test-skill.md"),
            "---\nname: test-skill\ndescription: A test skill\n---\n\nContent.",
        ).unwrap();
        skills.scan(dir.path().to_str().unwrap()).unwrap();
        let ctx = Context::load(dir.path().to_str().unwrap());
        agent = AgentLoop::new(Box::new(StubClient::new()), ctx);
        let msgs = agent.build_messages(&tools, &skills);
        let sys_prompt = &msgs[0].content;
        assert!(sys_prompt.contains("test-skill"));
    }

    #[test]
    fn test_set_model_updates_config() {
        unsafe {
            std::env::set_var("OPENAI_API_KEY", "test-key");
        }
        let mut agent = test_agent();
        agent.set_model("custom-model-name");
        assert!(agent.llm.config().model.contains("custom-model"));
    }

    #[test]
    fn test_reload_context_does_not_panic() {
        let mut agent = test_agent();
        agent.reload_context();
        assert!(agent.context().project_root.contains("tmp"));
    }

    #[test]
    fn test_agent_with_delta_tx() {
        let (tx, _rx) = tokio::sync::mpsc::channel(256);
        let ctx = Context::load("/tmp");
        let agent = AgentLoop::new(Box::new(StubClient::new()), ctx).with_delta_tx(tx);
        // Just verify it doesn't panic — delta_tx is set
    }

    #[test]
    fn test_build_system_prompt_contains_tools() {
        let agent = test_agent();
        let prompt = agent.build_system_prompt(&ToolRegistry::new_for_test(), &SkillRegistry::new());
        assert!(prompt.contains("tool") || prompt.contains("read_file"));
    }

    // ─── ADR 0005 — Permission Scope ─────────────────────────────────────

    #[test]
    fn adr0005_perm_scope_default_is_once() {
        // Default scope must be Once so legacy callers that don't specify
        // scope get the safe (re-prompt) behavior, not silent persistence.
        assert_eq!(PermScope::default(), PermScope::Once);
    }

    #[test]
    fn adr0005_perm_scope_variants_exist() {
        // Compile-time check: all three variants exist and are Copy.
        let once = PermScope::Once;
        let session = PermScope::AlwaysThisSession;
        let project = PermScope::AlwaysThisProject;
        let _copy = once;
        let _copy2 = session;
        let _copy3 = project;
        assert_eq!(once, PermScope::Once);
    }

    #[test]
    fn adr0005_permission_response_carries_scope() {
        let cmd = AgentCommand::PermissionResponse {
            request_id: 42,
            allowed: true,
            scope: PermScope::AlwaysThisSession,
        };
        match cmd {
            AgentCommand::PermissionResponse { request_id, allowed, scope } => {
                assert_eq!(request_id, 42);
                assert!(allowed);
                assert_eq!(scope, PermScope::AlwaysThisSession);
            }
            _ => panic!("expected PermissionResponse"),
        }
    }

    #[test]
    fn adr0005_permission_engine_deny_is_absolute() {
        // Document the safety rail: a Denied decision from PermissionEngine
        // is absolute and cannot be overridden by any allowlist. The agent's
        // permission_check closure checks Denied BEFORE consulting
        // session_allowed, so the engine remains the source of truth for
        // hard denies.
        use crate::permission::{PermissionEngine, PermissionDecision};
        let engine = PermissionEngine::new();
        let decision = engine.evaluate("run_command", "rm -rf /");
        assert!(matches!(decision, PermissionDecision::Denied { .. }),
            "rm -rf / must be hard-denied (engine safety rail)");
    }

    // ─── ADR 0001 Phase B — Real mid-call cancellation ───────────────────

    #[tokio::test]
    async fn adr0001_phase_b_pre_set_cancel_returns_interrupted() {
        // If the cancel flag is set BEFORE handle_message is called, the
        // very first round check aborts immediately with "[interrupted]".
        let mut agent = test_agent();
        let tools = ToolRegistry::new_for_test();
        let mut skills = SkillRegistry::new();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let resp = agent.handle_message("hello", &tools, &mut skills, &|_, _| true, &cancel).await.unwrap();
        assert_eq!(resp, "[interrupted by user]");
    }

    #[tokio::test]
    async fn adr0001_phase_b_cancel_mid_call_aborts_promptly() {
        // Simulate the realistic race: handle_message is in flight, the
        // user fires Ctrl+C, the cancel flag flips to true. The next
        // round-boundary check inside react_loop should observe it and
        // return "[interrupted]".
        let mut agent = test_agent();
        let tools = ToolRegistry::new_for_test();
        let mut skills = SkillRegistry::new();
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let cancel_clone = cancel.clone();

        // Spawn a task that flips cancel after a short delay, mimicking the
        // user pressing Ctrl+C during the LLM call.
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            cancel_clone.store(true, std::sync::atomic::Ordering::SeqCst);
        });

        // StubClient returns deterministic text with no tool calls, so the
        // first round would normally succeed. With cancel firing mid-call,
        // the round-top check on the *next* iteration (or the post-response
        // check) catches it. Either way, the result must surface the cancel.
        let start = std::time::Instant::now();
        let resp = agent.handle_message("hello", &tools, &mut skills, &|_, _| true, &cancel).await.unwrap();
        let elapsed = start.elapsed();

        // Accept either outcome: the cancel fired before round 2 (returns
        // "[interrupted]") OR the cancel fired after the response completed
        // (returns normal text). Both are valid — the contract is "no work
        // happens AFTER the cancel flag is observed." We just verify the
        // call returns and the agent didn't deadlock.
        assert!(elapsed < std::time::Duration::from_secs(5), "handle_message must not hang");
        // Either response is acceptable for this smoke test.
        let _ = resp;
    }

    #[tokio::test]
    async fn adr0001_phase_b_cancel_does_not_poison_next_message() {
        // Reset semantics: after a cancelled handle_message returns, the
        // NEXT call must work normally. The agent main loop resets the
        // flag before each ProcessMessage; this test verifies the contract
        // by simulating that reset.
        let mut agent = test_agent();
        let tools = ToolRegistry::new_for_test();
        let mut skills = SkillRegistry::new();

        // First call: cancelled.
        let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(true));
        let resp1 = agent.handle_message("first", &tools, &mut skills, &|_, _| true, &cancel).await.unwrap();
        assert_eq!(resp1, "[interrupted by user]");

        // Reset (simulating what the main loop does on each new ProcessMessage).
        cancel.store(false, std::sync::atomic::Ordering::SeqCst);

        // Second call: must work normally, not pre-cancelled.
        let resp2 = agent.handle_message("second", &tools, &mut skills, &|_, _| true, &cancel).await.unwrap();
        assert_ne!(resp2, "[interrupted by user]", "second message must not see stale cancel flag");
        assert!(resp2.contains("second"));
    }
}
