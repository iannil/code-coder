mod tui;
mod agent;
mod autonomous;
mod config;
mod context;
mod event;
mod llm;
mod mcp;
mod memory;
mod sandbox;
mod self_evolve;
mod session;
mod skill;
mod tools;

use std::io::Write;

/// Print a log message to codecoder.log AND stderr (stderr works in non-TUI mode).
/// In TUI raw mode, stderr shares the same terminal as the rendered UI, so a file
/// is the primary medium for persistent logs. Use `tail -f codecoder.log` alongside
/// the TUI session.
pub(crate) fn log(msg: &str) {
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("codecoder.log")
    {
        let _ = writeln!(f, "{}", msg);
        let _ = f.flush();
    }
    let _ = writeln!(std::io::stderr(), "{}", msg);
    let _ = std::io::stderr().flush();
}

use agent::BackgroundAgent;
use config::ConfigStore;
use context::Context;
use event::SharedEventBus;
use llm::OpenAiClient;
use memory::MemoryStore;
use session::SessionStore;
use skill::SkillRegistry;
use tools::ToolRegistry;
use std::sync::{Arc, Mutex};
use mcp::{McpRegistry, McpTool};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let is_daemon = args.iter().any(|a| a == "--daemon" || a == "-d");

    let project_root = std::env::var("CODECODER_ROOT")
        .unwrap_or_else(|_| std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".into()));

    // ── Initialise subsystems ──────────────────────────────────────────────

    let bus = SharedEventBus::new();
    let mut tools = ToolRegistry::new(&project_root);

    let context = Context::load(&project_root);
    let session_store = SessionStore::open(&project_root);

    let mut skills = SkillRegistry::new();
    let mem_store = MemoryStore::open(&project_root);
    skills.set_memory_store(mem_store);
    if let Err(e) = skills.scan(&project_root) {
        eprintln!("[codecoder] Warning: failed to scan skills: {e}");
    }

    // ── Config ──────────────────────────────────────────────────────────────

    let config = ConfigStore::load(&project_root);

    // ── 启动配置打印 ──────────────────────────────────────────────────────────

    let llm_cfg = config.to_llm_config();
    log("[codecoder] 启动配置:");
    log(&format!("[codecoder]   模型:     {}", llm_cfg.model));
    log(&format!("[codecoder]   API Base: {}", llm_cfg.api_base));
    log(&format!("[codecoder]   API Key:  {}", if llm_cfg.api_key.is_empty() { "(空 — 将使用 StubClient)" } else { "✓ 已设置" }));
    log(&format!("[codecoder]   环境变量: CODECODER_API_KEY={}", std::env::var("CODECODER_API_KEY").map(|_| "✓".to_string()).unwrap_or("✗".into())));
    log(&format!("[codecoder]   OPENAI_API_KEY={}", std::env::var("OPENAI_API_KEY").map(|_| "✓".to_string()).unwrap_or("✗".into())));
    log(&format!("[codecoder]   CODECODER_API_BASE={}", std::env::var("CODECODER_API_BASE").unwrap_or("(未设置)".into())));
    log(&format!("[codecoder]   CODECODER_MODEL={}", std::env::var("CODECODER_MODEL").unwrap_or("(未设置)".into())));

    // ── MCP ─────────────────────────────────────────────────────────────────

    let mcp_registry = Arc::new(Mutex::new(McpRegistry::new(config.get().mcp_servers.clone())));
    {
        let mut reg = mcp_registry.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let results = reg.start_all();
        for r in &results {
            eprintln!("[codecoder] MCP: {r}");
        }
        for tool in reg.all_tools() {
            let mcp_tool = McpTool::new(&tool.tool_name, &tool.description, mcp_registry.clone());
            tools.register(Box::new(mcp_tool));
        }
    }

    // ── LLM client (sync creation, async usage) ────────────────────────────

    let llm_config = config.to_llm_config();
    let llm: Box<dyn llm::LlmClient> = if llm_config.api_key.is_empty() {
        crate::log("[codecoder] No CODECODER_API_KEY or OPENAI_API_KEY set — using stub LLM");
        crate::log("[codecoder] Set the env var to connect to a real LLM provider.");
        Box::new(llm::StubClient::new())
    } else {
        Box::new(OpenAiClient::new(llm_config))
    };

    // ── Spawn background agent (async on tokio) ───────────────────────────

    let bg = BackgroundAgent::spawn(llm, context, tools, skills, bus.clone());

    // ── Run (TUI or Daemon) ────────────────────────────────────────────────

    if is_daemon {
        run_daemon(bg, bus, config, session_store)
    } else {
        tui::run_tui(bus.clone(), bg.cmd_tx, bg.resp_rx, session_store, config, mcp_registry)
    }
}

fn run_daemon(
    bg: BackgroundAgent,
    bus: SharedEventBus,
    config: ConfigStore,
    _session_store: SessionStore,
) -> anyhow::Result<()> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .enable_io()
        .build()
        .expect("failed to build daemon runtime");

    rt.block_on(async {
        let scheduled_tasks: Vec<autonomous::ScheduledTask> = config.get().scheduled_tasks.iter()
            .map(|t| autonomous::ScheduledTask {
                name: t.name.clone(),
                prompt: t.prompt.clone(),
                interval_secs: t.interval_secs,
            })
            .collect();

        let watch_paths: Vec<String> = config.get().watch_paths.clone();

        autonomous::AutonomousRunner::run(
            bg.cmd_tx,
            bg.resp_rx,
            bus,
            config,
            scheduled_tasks,
            watch_paths,
        ).await
    })
}

// ─── Unit Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_main_no_panic_on_help() {
        // Just verify imports & basic structure compile
        fn check_types() {
            let _: fn(&str) = log;
        }
        check_types();
    }

    #[test]
    fn test_log_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();
        log("[test] log message");
        let content = std::fs::read_to_string(dir.path().join("codecoder.log")).unwrap_or_default();
        assert!(content.contains("[test] log message"), "log should write to file");
        std::env::set_current_dir(original).unwrap();
        let _ = dir.close();
    }
}

// ─── End-to-End Smoke Tests ─────────────────────────────────────────────────

#[cfg(test)]
mod e2e_tests {
    use crate::agent::*;
    use crate::context::Context;
    use crate::event::SharedEventBus;
    use crate::llm::StubClient;
    use crate::memory::MemoryStore;
    use crate::sandbox::DockerSandbox;
    use crate::sandbox::Sandbox;
    use crate::session::{Session, SessionStore};
    use crate::skill::SkillRegistry;
    use crate::tools::ToolRegistry;
    use crate::tui::MessageItem;

    fn test_background() -> std::sync::mpsc::Sender<AgentCommand> {
        let ctx = Context::load("/tmp");
        let tools = ToolRegistry::new_for_test();
        let skills = SkillRegistry::new();
        let bus = SharedEventBus::new();
        let bg = BackgroundAgent::spawn(Box::new(StubClient::new()), ctx, tools, skills, bus);
        bg.cmd_tx
    }

    #[test]
    fn e2e_background_message_roundtrip() {
        let cmd_tx = test_background();
        cmd_tx.send(AgentCommand::ProcessMessage { text: "hello".into() }).unwrap();
        cmd_tx.send(AgentCommand::Shutdown).unwrap();
    }

    #[test]
    fn e2e_agent_creates_skill() {
        let tools = ToolRegistry::new("/tmp");
        let result = tools.execute("generate_skill", "my-skill\n---\n# My Skill\n\nA test.");
        assert!(result.is_ok());
        let msg = result.unwrap();
        assert!(msg.contains("my-skill"));
    }

    #[test]
    fn e2e_tool_list_contains_all() {
        let tools = ToolRegistry::new("/tmp");
        let names = tools.list_tools();
        let required = [
            "read_file", "write_file", "run_command", "search_web",
            "list_directory", "generate_skill", "generate_prompt", "generate_tool",
            "search_github", "reverse_api", "run_in_sandbox",
            "glob", "grep", "todo", "diff", "ask_user", "agent", "edit_file", "commit", "review", "plan",
        ];
        for name in &required {
            assert!(names.contains(name), "Missing tool: {name}");
        }
    }

    #[test]
    fn e2e_context_loads_agents_md() {
        let ctx = Context::load(".");
        assert!(!ctx.agents_md.is_empty(), "AGENTS.md should exist");
        assert!(ctx.agents_md.contains("CodeCoder"));
    }

    #[test]
    fn e2e_skill_discovery_finds_greeter() {
        let mut skills = SkillRegistry::new();
        skills.scan(".").unwrap();
        let names = skills.list();
        assert!(names.contains(&"greeter"));
    }

    #[test]
    fn e2e_memory_write_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let mut store = MemoryStore::open(root);
        store.set("integration-test", "works").unwrap();
        assert_eq!(store.len(), 1);
        let store = MemoryStore::open(root);
        assert_eq!(store.get("integration-test").unwrap().value, "works");
    }

    #[test]
    fn e2e_sandbox_graceful_fallback() {
        let sb = DockerSandbox::new();
        match sb.run("print(1)", "python") {
            Ok(out) => println!("Docker available: {out}"),
            Err(e) => assert!(e.to_string().contains("Docker")),
        }
    }

    #[test]
    fn e2e_system_prompt_completeness() {
        let ctx = Context::load(".");
        let tools = ToolRegistry::new_for_test();
        let skills = SkillRegistry::new();
        let agent = AgentLoop::new(Box::new(StubClient::new()), ctx);
        let prompt = agent.build_system_prompt(&tools, &skills);
        assert!(prompt.contains("CodeCoder"));
        assert!(prompt.contains("Available Tools"));
    }

    #[tokio::test]
    async fn e2e_full_startup_simulation() {
        let ctx = Context::load(".");
        let tools = ToolRegistry::new("/tmp");
        let mut skills = SkillRegistry::new();
        let _ = skills.scan(".");
        let dir = tempfile::tempdir().unwrap();
        let _memory = MemoryStore::open(dir.path().to_str().unwrap());
        let mut agent = AgentLoop::new(Box::new(StubClient::new()), ctx);
        let resp = agent.handle_message("list all tools", &tools, &mut skills, &|_, _| true).await;
        assert!(resp.is_ok());
    }

    #[test]
    fn e2e_session_save_and_resume() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();
        let store = SessionStore::open(root);
        let mut session = Session::new("gpt-4o");
        session.messages.push(MessageItem::User { text: "hello".into() });
        session.messages.push(MessageItem::Assistant { text: "hi there".into() });
        session.touch();
        store.save(&session).unwrap();
        assert_eq!(store.list().len(), 1);
        let loaded = store.load(&session.id).unwrap();
        assert_eq!(loaded.messages.len(), 2);
    }
}
