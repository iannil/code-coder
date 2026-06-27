use std::collections::HashMap;

mod generate;
mod list_dir;
mod read_file;
mod reverse_api;
mod run_command;
mod run_in_sandbox;
mod search_github;
mod search_web;
mod write_file;

pub use generate::{GeneratePrompt, GenerateSkill, GenerateTool};
pub use list_dir::ListDir;
pub use read_file::ReadFile;
pub use reverse_api::ReverseApi;
pub use run_command::RunCommand;
pub use run_in_sandbox::RunInSandbox;
pub use search_github::SearchGitHub;
pub use search_web::SearchWeb;
pub use write_file::WriteFile;

/// ─── Tool trait ────────────────────────────────────────────────────────────

/// Every tool (built-in or generated) implements this.
pub trait Tool: Send + 'static {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, input: &str) -> anyhow::Result<String>;
}

/// ─── ToolRegistry ──────────────────────────────────────────────────────────

pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a registry with all built-in tools.
    pub fn new(project_root: &str) -> Self {
        let mut reg = Self {
            tools: HashMap::new(),
        };
        reg.register(Box::new(ReadFile));
        reg.register(Box::new(WriteFile));
        reg.register(Box::new(RunCommand));
        reg.register(Box::new(SearchWeb));
        reg.register(Box::new(ListDir));
        // Self-evolution tools
        reg.register(Box::new(GenerateSkill {
            project_root: project_root.into(),
        }));
        reg.register(Box::new(GeneratePrompt {
            project_root: project_root.into(),
        }));
        reg.register(Box::new(GenerateTool {
            project_root: project_root.into(),
        }));
        // Search & discovery tools
        reg.register(Box::new(SearchGitHub));
        reg.register(Box::new(ReverseApi {
            project_root: project_root.into(),
        }));
        // Sandbox
        reg.register(Box::new(RunInSandbox));
        reg
    }

    /// Create a registry with only safe, no-side-effect tools (for tests).
    pub fn new_for_test() -> Self {
        let mut reg = Self {
            tools: HashMap::new(),
        };
        reg.register(Box::new(ReadFile));
        reg.register(Box::new(ListDir));
        reg
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }

    pub fn execute(&self, name: &str, input: &str) -> anyhow::Result<String> {
        self.tools
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("tool not found: {name}"))?
            .execute(input)
    }

    pub fn list_tools(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.tools.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|b| b.as_ref())
    }
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_default_tools() {
        let reg = ToolRegistry::new("/tmp");
        let names = reg.list_tools();
        assert!(names.contains(&"list_directory"));
        assert!(names.contains(&"read_file"));
        assert!(names.contains(&"write_file"));
        assert!(names.contains(&"run_command"));
        assert!(names.contains(&"search_web"));
        assert!(names.contains(&"generate_skill"));
        assert!(names.contains(&"generate_prompt"));
        assert!(names.contains(&"generate_tool"));
        assert!(names.contains(&"search_github"));
        assert!(names.contains(&"reverse_api"));
    }

    #[test]
    fn test_execute_unknown_tool() {
        let reg = ToolRegistry::new("/tmp");
        let result = reg.execute("nonexistent", "");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }
}
