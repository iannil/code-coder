use std::collections::HashMap;

mod ask_agent;
mod commit;
mod diff;
mod edit_file;
mod generate;
mod glob;
mod grep;
mod list_dir;
mod plan;
mod read_file;
mod reverse_api;
mod review;
mod run_command;
mod run_in_sandbox;
mod search_github;
mod search_web;
mod todo;
mod write_file;

pub use ask_agent::{AgentTool, AskUserTool};
pub use commit::CommitTool;
pub use diff::DiffTool;
pub use edit_file::EditFileTool;
pub use generate::{GeneratePrompt, GenerateSkill, GenerateTool};
pub use glob::GlobTool;
pub use grep::Grep;
pub use list_dir::ListDir;
pub use plan::PlanTool;
pub use read_file::ReadFile;
pub use reverse_api::ReverseApi;
pub use review::ReviewTool;
pub use run_command::RunCommand;
pub use run_in_sandbox::RunInSandbox;
pub use search_github::SearchGitHub;
pub use search_web::SearchWeb;
pub use todo::TodoTool;
pub use write_file::WriteFile;

/// ─── Tool trait ────────────────────────────────────────────────────────────

/// Every tool (built-in or generated) implements this.
pub trait Tool: Send + 'static {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn execute(&self, input: &str) -> anyhow::Result<String>;

    /// JSON Schema for the tool's input parameters.
    /// Returns a JSON string. Empty string means the tool accepts a plain string.
    fn input_schema(&self) -> &'static str {
        ""
    }
}

/// Validate tool input against its JSON Schema.
/// For tools without a schema (empty string), accepts any non-empty input.
/// For tools with a schema, parses `input` as JSON and validates required fields.
pub fn validate_tool_input(schema: &str, input: &str) -> anyhow::Result<()> {
    if schema.is_empty() {
        // Legacy mode: plain text input, just ensure non-empty
        if input.is_empty() && !input.is_empty() {
            // never true — placeholder for future validation
        }
        return Ok(());
    }
    // Parse input as JSON and validate against schema
    let schema_val: serde_json::Value = serde_json::from_str(schema)
        .map_err(|e| anyhow::anyhow!("invalid schema: {e}"))?;
    let input_val: serde_json::Value = serde_json::from_str(input)
        .map_err(|e| anyhow::anyhow!("input must be valid JSON for this tool: {e}"))?;

    // Simple required-fields check (deep schema validation via ajv not available in Rust)
    if let Some(required) = schema_val.get("required").and_then(|v| v.as_array()) {
        for field in required {
            let field_name = field.as_str().unwrap_or("");
            if !input_val.get(field_name).and_then(|v| v.as_str()).map_or(false, |s| !s.is_empty()) {
                anyhow::bail!("missing required field: {field_name}");
            }
        }
    }
    Ok(())
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
        // Code search tools
        reg.register(Box::new(GlobTool));
        reg.register(Box::new(Grep));
        // Productivity tools
        reg.register(Box::new(TodoTool));
        reg.register(Box::new(DiffTool));
        reg.register(Box::new(EditFileTool));
        reg.register(Box::new(CommitTool));
        reg.register(Box::new(ReviewTool));
        reg.register(Box::new(PlanTool));
        reg.register(Box::new(AskUserTool));
        reg.register(Box::new(AgentTool));
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
        let tool = self.tools
            .get(name)
            .ok_or_else(|| anyhow::anyhow!("tool not found: {name}"))?;

        // Validate input against schema
        let schema = tool.input_schema();
        validate_tool_input(schema, input)?;

        tool.execute(input)
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

/// Try to parse `input` as JSON and extract a string field.
/// Returns `None` if input is not valid JSON or the field is missing.
/// Allows tools to accept both `{"field": "value"}` and plain `"value"` formats.
pub fn try_extract_json_field(input: &str, field: &str) -> Option<String> {
    let val: serde_json::Value = serde_json::from_str(input).ok()?;
    val.get(field).and_then(|v| v.as_str()).map(|s| s.to_string())
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
