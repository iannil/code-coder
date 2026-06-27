use std::io::Read;

use crate::agent::{AgentCommand, AgentResponse, BackgroundAgent};
use crate::event::{Event, EventBus, SharedEventBus, SystemCommand};
use crate::memory::MemoryStore;

/// ─── REPL ──────────────────────────────────────────────────────────────────
///
/// Foreground interactive terminal.  Sends messages to the background
/// agent and displays responses + heartbeats.

pub struct Repl {
    prompt: String,
}

impl Repl {
    pub fn new() -> Self {
        Self {
            prompt: "cc> ".into(),
        }
    }

    pub fn run(
        &mut self,
        bus: SharedEventBus,
        bg: BackgroundAgent,
        memory: &MemoryStore,
    ) -> anyhow::Result<()> {
        let is_pipe = !atty::is(atty::Stream::Stdin);
        if is_pipe {
            return self.run_pipe(bg);
        }
        self.run_interactive(bus, bg, memory)
    }

    fn run_interactive(
        &mut self,
        bus: SharedEventBus,
        bg: BackgroundAgent,
        _memory: &MemoryStore,
    ) -> anyhow::Result<()> {
        println!("CodeCoder v{}", env!("CARGO_PKG_VERSION"));
        println!("Type /exit to quit.\n");

        // Spawn a heartbeat listener thread
        let resp_rx = bg.resp_rx;
        let cmd_tx = bg.cmd_tx;

        // Main input loop
        loop {
            let mut input = String::new();
            print!("{}", self.prompt);
            use std::io::Write;
            std::io::stdout().flush()?;

            if std::io::stdin().read_line(&mut input).is_err() || input.is_empty() {
                break;
            }

            let input = input.trim().to_string();
            if input.is_empty() {
                continue;
            }

            // Handle built-in commands (no agent needed)
            if input.starts_with('/') {
                let should_exit = self.handle_command(&input, &cmd_tx, &mut bus.clone())?;
                if should_exit {
                    cmd_tx.send(AgentCommand::Shutdown).ok();
                    break;
                }
                continue;
            }

            // Send to background agent
            cmd_tx.send(AgentCommand::ProcessMessage { text: input })?;

            // Wait for response (with heartbeat display)
            loop {
                match resp_rx.recv() {
                    Ok(AgentResponse::Text { text: resp }) => {
                        println!("\n{resp}\n");
                        break;
                    }
                    Ok(AgentResponse::Heartbeat { pending, .. }) => {
                        if pending > 0 {
                            // Quiet heartbeat — just shows system is alive
                        }
                    }
                    Ok(AgentResponse::Shutdown) => {
                        return Ok(());
                    }
                    _ => {}
                }
            }
        }

        // Clean shutdown
        cmd_tx.send(AgentCommand::Shutdown).ok();
        Ok(())
    }

    fn run_pipe(&mut self, bg: BackgroundAgent) -> anyhow::Result<()> {
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input)?;
        let input = input.trim().to_string();

        if !input.is_empty() {
            bg.cmd_tx
                .send(AgentCommand::ProcessMessage { text: input })?;
            match bg.resp_rx.recv() {
                Ok(AgentResponse::Text { text: resp }) => println!("{}", resp),
                Ok(AgentResponse::Shutdown) => {}
                _ => {}
            }
        }

        bg.cmd_tx.send(AgentCommand::Shutdown).ok();
        Ok(())
    }

    fn handle_command(
        &self,
        cmd: &str,
        cmd_tx: &std::sync::mpsc::Sender<AgentCommand>,
        bus: &mut SharedEventBus,
    ) -> anyhow::Result<bool> {
        let cmd = cmd.trim().to_lowercase();

        match cmd.as_str() {
            "/exit" | "/quit" => return Ok(true),

            "/help" => {
                println!("Commands:");
                println!("  /exit, /quit    Exit the REPL");
                println!("  /help           Show this help");
                println!("  /reload         Reload context and skills");
                println!("  /tools          List available tools");
                println!("  /skills         List loaded skills");
                println!("  /memory         List persistent memory entries");
                println!("  /clear          Clear conversation history");
                println!("  /history        Show conversation history count");
            }

            "/reload" => {
                bus.publish(Event::System(SystemCommand::ReloadSkills))?;
                cmd_tx.send(AgentCommand::ReloadContext)?;
                println!("→ Context and skills reloaded from disk");
            }

            "/tools" => {
                println!("→ Use the agent to list tools (they are in system prompt)");
            }

            "/skills" => {
                println!("→ Use the agent to list skills (they are in system prompt)");
            }

            "/memory" => {
                println!("→ Memory entries are stored in memory/ directory");
            }

            "/clear" => {
                cmd_tx.send(AgentCommand::ClearHistory)?;
                println!("→ Conversation history cleared");
            }

            "/history" => {
                cmd_tx.send(AgentCommand::ProcessMessage {
                    text: "How many messages are in our conversation history? Just tell me the count."
                        .into(),
                })?;
            }

            _ => {
                println!("Unknown command: {cmd}. Type /help for available commands.");
            }
        }

        Ok(false)
    }
}
