//! `SkillSearch` Tool - Enables agents to search and recommend skills
//!
//! This tool allows the AI agent to search the `SkillHub` registry
//! when a user asks for capabilities the agent doesn't have.

use crate::skills::hub::SkillHub;
use crate::tools::traits::{Tool, ToolResult};
use async_trait::async_trait;
use serde_json::Value;

/// Tool for searching `SkillHub` registry from within agent conversations
pub struct SkillSearchTool {
    hub: SkillHub,
}

impl SkillSearchTool {
    pub fn new() -> Self {
        Self {
            hub: SkillHub::new(),
        }
    }
}

impl Default for SkillSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for SkillSearchTool {
    fn name(&self) -> &str {
        "skill_search"
    }

    fn description(&self) -> &str {
        "Search SkillHub for available skills. Use when the user asks for capabilities \
         you don't have or when they want to extend the bot's functionality. \
         Returns a list of matching skills with install instructions."
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query for skills (e.g., 'code review', 'web scraping', 'data analysis')"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results to return (default: 5)",
                    "default": 5,
                    "minimum": 1,
                    "maximum": 20
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let query = args["query"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing required 'query' parameter"))?;

        let limit = args["limit"].as_u64().unwrap_or(5) as usize;

        match self.hub.search(query, limit).await {
            Ok(skills) => {
                if skills.is_empty() {
                    Ok(ToolResult {
                        success: true,
                        output: format!(
                            "No skills found matching '{query}'. The user can create a custom skill \
                             or browse the registry at https://github.com/zerobot-skills/registry"
                        ),
                        error: None,
                    })
                } else {
                    let mut output = format!("Found {} skill(s) matching '{}':\n\n", skills.len(), query);
                    for skill in &skills {
                        output.push_str(&format!(
                            "• **{}** (v{}) — {}\n  Install: `zero-bot skills install {}`\n",
                            skill.name, skill.version, skill.description, skill.repo_url
                        ));
                        if !skill.tags.is_empty() {
                            output.push_str(&format!("  Tags: {}\n", skill.tags.join(", ")));
                        }
                        output.push_str(&format!("  Downloads: {}\n\n", skill.downloads));
                    }
                    output.push_str(
                        "Tell the user which skill(s) match their needs and how to install them.",
                    );

                    Ok(ToolResult {
                        success: true,
                        output,
                        error: None,
                    })
                }
            }
            Err(e) => Ok(ToolResult {
                success: false,
                output: String::new(),
                error: Some(format!(
                    "Failed to search SkillHub registry: {e}. \
                     The registry may be temporarily unavailable."
                )),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_name() {
        let tool = SkillSearchTool::new();
        assert_eq!(tool.name(), "skill_search");
    }

    #[test]
    fn tool_description_not_empty() {
        let tool = SkillSearchTool::new();
        assert!(!tool.description().is_empty());
    }

    #[test]
    fn tool_schema_has_query() {
        let tool = SkillSearchTool::new();
        let schema = tool.parameters_schema();

        assert!(schema["properties"]["query"].is_object());
        assert_eq!(schema["properties"]["query"]["type"], "string");
    }

    #[test]
    fn tool_schema_has_limit() {
        let tool = SkillSearchTool::new();
        let schema = tool.parameters_schema();

        assert!(schema["properties"]["limit"].is_object());
        assert_eq!(schema["properties"]["limit"]["type"], "integer");
    }

    #[test]
    fn tool_schema_requires_query() {
        let tool = SkillSearchTool::new();
        let schema = tool.parameters_schema();

        let required = schema["required"].as_array().unwrap();
        assert!(required.contains(&serde_json::json!("query")));
    }

    #[test]
    fn tool_default() {
        let tool = SkillSearchTool::default();
        assert_eq!(tool.name(), "skill_search");
    }

    #[tokio::test]
    async fn execute_missing_query_fails() {
        let tool = SkillSearchTool::new();
        let result = tool.execute(serde_json::json!({})).await;
        assert!(result.is_err());
    }

    #[test]
    fn spec_generation() {
        let tool = SkillSearchTool::new();
        let spec = tool.spec();

        assert_eq!(spec.name, "skill_search");
        assert!(!spec.description.is_empty());
        assert!(spec.parameters["properties"]["query"].is_object());
    }
}
