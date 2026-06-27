use crate::context::Context;
use crate::event::{Event, Subscriber};
use crate::llm::{LlmClient, Message};
use crate::skill::SkillRegistry;
use crate::tools::ToolRegistry;

/// ─── ToolCall ──────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ToolCall {
    pub name: String,
    pub input: String,
}

/// ─── AgentLoop ─────────────────────────────────────────────────────────────

#[allow(dead_code)]
pub struct AgentLoop {
    running: bool,
    llm: Box<dyn LlmClient>,
    history: Vec<Message>,
    base_system_prompt: String,
    context: Context,
    max_tool_rounds: usize,
}

impl AgentLoop {
    pub fn new(llm: Box<dyn LlmClient>, context: Context) -> Self {
        Self {
            running: false,
            llm,
            history: Vec::new(),
            base_system_prompt: Self::default_system_prompt(),
            context,
            max_tool_rounds: 10,
        }
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
            prompt.push_str(&format!(
                "You have the following skills available:\n\n{}",
                skill_list
                    .iter()
                    .map(|s| format!("- `{}`", s))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
            prompt.push('\n');
        }
        prompt
    }

    pub fn handle_message(
        &mut self,
        text: &str,
        tools: &ToolRegistry,
        skills: &SkillRegistry,
    ) -> anyhow::Result<String> {
        self.history.push(Message::user(text));
        self.react_loop(tools, skills)
    }

    fn react_loop(&mut self, tools: &ToolRegistry, skills: &SkillRegistry) -> anyhow::Result<String> {
        for _round in 0..self.max_tool_rounds {
            let messages = self.build_messages(tools, skills);
            let response = self.llm.chat(&messages)?;
            let tool_calls = parse_tool_calls(&response.text);

            if tool_calls.is_empty() {
                self.history.push(Message::assistant(&response.text));
                return Ok(response.text);
            }

            for tc in &tool_calls {
                let result = match tools.execute(&tc.name, &tc.input) {
                    Ok(out) => out,
                    Err(e) => format!("[tool error] {e}"),
                };
                self.history.push(Message::assistant(&response.text));
                self.history.push(Message::user(format!(
                    "[tool result for {}]\n{}",
                    tc.name, result
                )));
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

    pub fn history(&self) -> &[Message] {
        &self.history
    }

    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    pub fn shutdown(&mut self) {
        self.running = false;
    }
}

/// ─── BackgroundAgent ───────────────────────────────────────────────────────
///
/// Wraps AgentLoop and runs it in a background thread.  The REPL sends
/// messages via a channel, and the agent processes them independently.
/// Progress is reported back via the event bus (heartbeat events) and
/// final results via a response channel.

use crate::event::SharedEventBus;

#[allow(dead_code)]
pub struct BackgroundAgent {
    pub cmd_tx: std::sync::mpsc::Sender<AgentCommand>,
    pub resp_rx: std::sync::mpsc::Receiver<AgentResponse>,
    pub thread_handle: Option<std::thread::JoinHandle<()>>,
}

pub enum AgentCommand {
    ProcessMessage(String),
    ReloadContext,
    ClearHistory,
    Shutdown,
}

#[derive(Debug)]
pub enum AgentResponse {
    MessageResult(String),
    Heartbeat { pending: usize },
    Shutdown,
}

impl BackgroundAgent {
    /// Spawn the agent loop in a background thread.
    pub fn spawn(
        llm: Box<dyn LlmClient>,
        context: Context,
        tools: ToolRegistry,
        skills: SkillRegistry,
        bus: SharedEventBus,
    ) -> Self {
        let (cmd_tx, cmd_rx) = std::sync::mpsc::channel::<AgentCommand>();
        let (resp_tx, resp_rx) = std::sync::mpsc::channel::<AgentResponse>();

        let handle = std::thread::Builder::new()
            .name("agent".into())
            .spawn(move || {
                let mut agent = AgentLoop::new(llm, context);
                let tools = tools;
                let mut skills = skills;
                let bus = bus; // owned clone

                // Heartbeat counter
                let mut heartbeat_count = 0;

                loop {
                    // Check for commands (block with timeout so we can heartbeat)
                    match cmd_rx.recv_timeout(std::time::Duration::from_millis(500)) {
                        Ok(AgentCommand::ProcessMessage(text)) => {
                            let _ = bus.drain();
                            match agent.handle_message(&text, &tools, &skills) {
                                Ok(resp) => {
                                    let _ = resp_tx.send(AgentResponse::MessageResult(resp));
                                }
                                Err(e) => {
                                    let _ = resp_tx.send(AgentResponse::MessageResult(
                                        format!("[error] {e}"),
                                    ));
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
                        Ok(AgentCommand::Shutdown) => {
                            let _ = resp_tx.send(AgentResponse::Shutdown);
                            break;
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            // Heartbeat — report status every ~2 seconds
                            heartbeat_count += 1;
                            if heartbeat_count % 4 == 0 {
                                let pending = bus.pending();
                                let _ = resp_tx.send(AgentResponse::Heartbeat {
                                    pending,
                                });
                            }
                        }
                        Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                    }
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

    #[test]
    fn test_handle_message_with_stub() {
        let mut agent = test_agent();
        let tools = ToolRegistry::new_for_test();
        let skills = SkillRegistry::new();
        let resp = agent.handle_message("hello", &tools, &skills).unwrap();
        assert!(resp.contains("hello"));
    }

    #[test]
    fn test_clear_history() {
        let mut agent = test_agent();
        let tools = ToolRegistry::new_for_test();
        let skills = SkillRegistry::new();
        agent.handle_message("hello", &tools, &skills).unwrap();
        assert!(agent.history().len() >= 2);
        agent.clear_history();
        assert_eq!(agent.history().len(), 0);
    }

    #[test]
    fn test_background_agent_spawn_and_shutdown() {
        let ctx = Context::load("/tmp");
        let tools = ToolRegistry::new_for_test();
        let skills = SkillRegistry::new();
        let mut bus = SharedEventBus::new();

        let bg = BackgroundAgent::spawn(
            Box::new(StubClient::new()),
            ctx,
            tools,
            skills,
            bus,
        );

        bg.cmd_tx.send(AgentCommand::Shutdown).unwrap();
        match bg.resp_rx.recv_timeout(std::time::Duration::from_secs(2)) {
            Ok(AgentResponse::Shutdown) => {} // expected
            other => panic!("expected Shutdown, got {other:?}"),
        }

        // Ensure thread finishes
        if let Some(handle) = bg.thread_handle {
            handle.join().unwrap();
        }
    }
}
