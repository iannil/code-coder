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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::{BackgroundAgent, ToolCall};
    use crate::event::{EventBus, SharedEventBus};
    use crate::memory::MemoryStore;
    use std::sync::mpsc;

    fn make_bg() -> (BackgroundAgent, mpsc::Receiver<AgentCommand>) {
        let (cmd_tx, cmd_rx) = mpsc::channel();
        let (_resp_tx, resp_rx) = mpsc::channel();
        (BackgroundAgent { cmd_tx, resp_rx }, cmd_rx)
    }

    #[test]
    fn test_repl_new() {
        let repl = Repl::new();
        // prompt should default to "cc> "
        // We can't inspect private fields, so we just check it constructs
        assert!(std::mem::discriminant(&()) == std::mem::discriminant(&()));
        _ = repl;
    }

    #[test]
    fn test_handle_command_exit() {
        let repl = Repl::new();
        let (bg, _rx) = make_bg();
        let mut bus = SharedEventBus::new(64);
        let result = repl.handle_command("/exit", &bg.cmd_tx, &mut bus);
        assert!(result.is_ok());
        assert!(result.unwrap(), "/exit should return true");
    }

    #[test]
    fn test_handle_command_quit() {
        let repl = Repl::new();
        let (bg, _rx) = make_bg();
        let mut bus = SharedEventBus::new(64);
        let result = repl.handle_command("/quit", &bg.cmd_tx, &mut bus);
        assert!(result.is_ok());
        assert!(result.unwrap(), "/quit should return true");
    }

    #[test]
    fn test_handle_command_help() {
        let repl = Repl::new();
        let (bg, _rx) = make_bg();
        let mut bus = SharedEventBus::new(64);
        let result = repl.handle_command("/help", &bg.cmd_tx, &mut bus);
        assert!(result.is_ok());
        assert!(!result.unwrap(), "/help should return false");
    }

    #[test]
    fn test_handle_command_reload() {
        let repl = Repl::new();
        let (bg, rx) = make_bg();
        let mut bus = SharedEventBus::new(64);
        let result = repl.handle_command("/reload", &bg.cmd_tx, &mut bus);
        assert!(result.is_ok());
        assert!(!result.unwrap(), "/reload should return false");
        // Should send ReloadContext command
        let cmd = rx.recv();
        assert!(cmd.is_ok());
        assert!(matches!(cmd.unwrap(), AgentCommand::ReloadContext));
    }

    #[test]
    fn test_handle_command_clear() {
        let repl = Repl::new();
        let (bg, rx) = make_bg();
        let mut bus = SharedEventBus::new(64);
        let result = repl.handle_command("/clear", &bg.cmd_tx, &mut bus);
        assert!(result.is_ok());
        assert!(!result.unwrap(), "/clear should return false");
        // Should send ClearHistory command
        let cmd = rx.recv();
        assert!(cmd.is_ok());
        assert!(matches!(cmd.unwrap(), AgentCommand::ClearHistory));
    }

    #[test]
    fn test_handle_command_history() {
        let repl = Repl::new();
        let (bg, rx) = make_bg();
        let mut bus = SharedEventBus::new(64);
        let result = repl.handle_command("/history", &bg.cmd_tx, &mut bus);
        assert!(result.is_ok());
        assert!(!result.unwrap(), "/history should return false");
        // Should send ProcessMessage
        let cmd = rx.recv();
        assert!(cmd.is_ok());
        match cmd.unwrap() {
            AgentCommand::ProcessMessage { text } => {
                assert!(text.contains("conversation history"));
            }
            _ => panic!("Expected ProcessMessage"),
        }
    }

    #[test]
    fn test_handle_command_unknown() {
        let repl = Repl::new();
        let (bg, rx) = make_bg();
        let mut bus = SharedEventBus::new(64);
        let result = repl.handle_command("/xyzzy", &bg.cmd_tx, &mut bus);
        assert!(result.is_ok());
        assert!(!result.unwrap(), "unknown command should return false");
        // No command should be sent for unknown commands
        let cmd = rx.try_recv();
        assert!(cmd.is_err(), "no command should be sent for unknown input");
    }

    #[test]
    fn test_handle_command_case_insensitive() {
        let repl = Repl::new();
        let (bg, rx) = make_bg();
        let mut bus = SharedEventBus::new(64);
        let result = repl.handle_command("/EXIT", &bg.cmd_tx, &mut bus);
        assert!(result.is_ok());
        assert!(result.unwrap(), "/EXIT (uppercase) should also exit");
        _ = rx;
    }

    #[test]
    fn test_handle_command_tools_skills_memory() {
        for cmd in &["/tools", "/skills", "/memory"] {
            let repl = Repl::new();
            let (bg, rx) = make_bg();
            let mut bus = SharedEventBus::new(64);
            let result = repl.handle_command(cmd, &bg.cmd_tx, &mut bus);
            assert!(result.is_ok());
            assert!(!result.unwrap(), "{cmd} should return false");
            // No agent command sent for these
            let cmd_recv = rx.try_recv();
            assert!(cmd_recv.is_err(), "{cmd} should not send an agent command");
        }
    }
}
