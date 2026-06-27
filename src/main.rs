mod agent;
mod context;
mod event;
mod llm;
mod memory;
mod repl;
mod sandbox;
mod skill;
mod tools;

use agent::BackgroundAgent;
use context::Context;
use event::SharedEventBus;
use llm::OpenAiClient;
use memory::MemoryStore;
use skill::SkillRegistry;
use tools::ToolRegistry;

fn main() -> anyhow::Result<()> {
    let project_root = std::env::var("CODECODER_ROOT")
        .unwrap_or_else(|_| std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".into()));

    // ── Initialise subsystems ──────────────────────────────────────────────

    let bus = SharedEventBus::new();
    let tools = ToolRegistry::new(&project_root);
    let context = Context::load(&project_root);
    let memory = MemoryStore::open(&project_root);

    if !memory.is_empty() {
        println!("[codecoder] Loaded {} memory entries", memory.len());
    }

    let mut skills = SkillRegistry::new();
    if let Err(e) = skills.scan(&project_root) {
        eprintln!("[codecoder] Warning: failed to scan skills: {e}");
    }

    let skill_count = skills.list().len();
    if skill_count > 0 {
        println!("[codecoder] Loaded {skill_count} skill(s) from skills/");
    }

    // ── LLM client ─────────────────────────────────────────────────────────

    let llm: Box<dyn llm::LlmClient> = match std::env::var("CODECODER_API_KEY")
        .or_else(|_| std::env::var("OPENAI_API_KEY"))
    {
        Ok(_) => Box::new(OpenAiClient::from_env()),
        Err(_) => {
            eprintln!("[codecoder] No CODECODER_API_KEY or OPENAI_API_KEY set — using stub LLM");
            eprintln!("[codecoder] Set the env var to connect to a real LLM provider.");
            Box::new(llm::StubClient::new())
        },
    };

    // ── Spawn background agent ────────────────────────────────────────────

    let bg = BackgroundAgent::spawn(llm, context, tools, skills, bus.clone());

    // ── REPL (foreground) ─────────────────────────────────────────────────

    let mut repl = repl::Repl::new();
    repl.run(bus, bg, &memory)
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
    use crate::skill::SkillRegistry;
    use crate::tools::ToolRegistry;

    /// Build a full background agent for testing.
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
        cmd_tx.send(AgentCommand::ProcessMessage("hello".into())).unwrap();
        cmd_tx.send(AgentCommand::Shutdown).unwrap();
        // If no panic, channels work
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
        ];
        for name in &required {
            assert!(names.contains(name), "Missing tool: {name}");
        }
    }

    #[test]
    fn e2e_context_loads_agents_md() {
        let ctx = Context::load(".");
        assert!(!ctx.agents_md.is_empty(), "AGENTS.md should exist in project");
        assert!(ctx.agents_md.contains("CodeCoder"), "AGENTS.md should mention CodeCoder");
    }

    #[test]
    fn e2e_skill_discovery_finds_greeter() {
        let mut skills = SkillRegistry::new();
        skills.scan(".").unwrap();
        let names = skills.list();
        assert!(names.contains(&"greeter"), "greeter.md not found: {names:?}");
    }

    #[test]
    fn e2e_memory_write_and_read() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        let mut store = MemoryStore::open(root);
        store.set("integration-test", "works").unwrap();
        assert_eq!(store.len(), 1);

        let store = MemoryStore::open(root);
        let entry = store.get("integration-test").unwrap();
        assert_eq!(entry.value, "works");
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
        assert!(prompt.contains("read_file"));
        // AGENTS.md content should be in the prompt
        assert!(prompt.contains("Rust") || prompt.contains("agent"));
    }

    #[test]
    fn e2e_full_startup_simulation() {
        // Simulate what main() does
        let ctx = Context::load(".");
        let tools = ToolRegistry::new("/tmp");
        let mut skills = SkillRegistry::new();
        let _ = skills.scan(".");
        let dir = tempfile::tempdir().unwrap();
        let _memory = MemoryStore::open(dir.path().to_str().unwrap());

        let mut agent = AgentLoop::new(Box::new(StubClient::new()), ctx);
        let resp = agent.handle_message("list all tools", &tools, &skills);
        assert!(resp.is_ok(), "handle_message failed: {:?}", resp.err());
    }
}
