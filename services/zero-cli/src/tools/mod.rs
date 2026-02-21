//! Tool system for ZeroBot.
//!
//! This module provides the tool registry and unique tools.
//! Core tool implementations are imported from `zero-tools`.

pub mod auto_login;
pub mod browser_open;
pub mod registry;
pub mod skill_search;

pub use auto_login::AutoLoginTool;
pub use browser_open::BrowserOpenTool;
pub use registry::ToolRegistry;
pub use skill_search::SkillSearchTool;

// Re-export tool types from zero-tools
pub use zero_tools::{
    BrowserTool, CodeCoderTool, FileReadTool, FileWriteTool, MemoryForgetTool, MemoryRecallTool,
    MemoryStoreTool, ShellTool, Tool, ToolResult, ToolSpec,
};

// Re-export SecurityPolicy from zero-tools for direct use
pub use zero_tools::SecurityPolicy as ZeroToolsSecurityPolicy;

use crate::memory::Memory;
use crate::security::SecurityPolicy;
use std::sync::Arc;

/// Create the default tool registry
pub fn default_tools(security: Arc<SecurityPolicy>) -> Vec<Box<dyn Tool>> {
    // Convert local SecurityPolicy to zero_tools::SecurityPolicy
    let zt_security: Arc<ZeroToolsSecurityPolicy> = Arc::new(security.as_ref().into());
    vec![
        Box::new(ShellTool::new(zt_security.clone())),
        Box::new(FileReadTool::new(zt_security.clone())),
        Box::new(FileWriteTool::new(zt_security)),
    ]
}

/// Create full tool registry including memory tools and optional `CodeCoder`
pub fn all_tools(
    security: &Arc<SecurityPolicy>,
    memory: Arc<dyn Memory>,
    browser_config: &crate::config::BrowserConfig,
    codecoder_config: &crate::config::CodeCoderConfig,
    vault_config: &crate::config::VaultConfig,
    vault_path: &std::path::Path,
) -> Vec<Box<dyn Tool>> {
    // Convert local SecurityPolicy to zero_tools::SecurityPolicy
    let zt_security: Arc<ZeroToolsSecurityPolicy> = Arc::new(security.as_ref().into());

    let mut tools: Vec<Box<dyn Tool>> = vec![
        Box::new(ShellTool::new(zt_security.clone())),
        Box::new(FileReadTool::new(zt_security.clone())),
        Box::new(FileWriteTool::new(zt_security.clone())),
        Box::new(MemoryStoreTool::new(memory.clone())),
        Box::new(MemoryRecallTool::new(memory.clone())),
        Box::new(MemoryForgetTool::new(memory)),
        Box::new(SkillSearchTool::new()),
    ];

    if browser_config.enabled {
        // Add legacy browser_open tool for simple URL opening
        tools.push(Box::new(BrowserOpenTool::new(
            security.clone(),
            browser_config.allowed_domains.clone(),
        )));
        // Add full browser automation tool (agent-browser)
        tools.push(Box::new(BrowserTool::new(
            zt_security.clone(),
            browser_config.allowed_domains.clone(),
            browser_config.session_name.clone(),
        )));

        // Add auto-login tool (requires browser automation)
        if vault_config.enabled {
            tools.push(Box::new(AutoLoginTool::new(
                security.clone(),
                vault_path.to_path_buf(),
            )));
        }
    }

    // Add CodeCoder tool for invoking 23 AI agents
    if codecoder_config.enabled {
        tools.push(Box::new(CodeCoderTool::new(
            Some(&codecoder_config.endpoint),
            codecoder_config.api_key.as_deref(),
        )));
    }

    tools
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BrowserConfig, CodeCoderConfig, MemoryConfig};
    use tempfile::TempDir;

    #[test]
    fn default_tools_has_three() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security);
        assert_eq!(tools.len(), 3);
    }

    #[test]
    fn all_tools_excludes_browser_when_disabled() {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy::default());
        let mem_cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> =
            Arc::from(crate::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

        let browser = BrowserConfig {
            enabled: false,
            allowed_domains: vec!["example.com".into()],
            session_name: None,
        };
        let codecoder = CodeCoderConfig::default();
        let vault = crate::config::VaultConfig::default();

        let tools = all_tools(&security, mem, &browser, &codecoder, &vault, tmp.path());
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(!names.contains(&"browser_open"));
        assert!(names.contains(&"skill_search")); // skill_search is always included
    }

    #[test]
    fn all_tools_includes_browser_when_enabled() {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy::default());
        let mem_cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> =
            Arc::from(crate::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

        let browser = BrowserConfig {
            enabled: true,
            allowed_domains: vec!["example.com".into()],
            session_name: None,
        };
        let codecoder = CodeCoderConfig::default();
        let vault = crate::config::VaultConfig::default();

        let tools = all_tools(&security, mem, &browser, &codecoder, &vault, tmp.path());
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"browser_open"));
    }

    #[test]
    fn all_tools_includes_skill_search() {
        let tmp = TempDir::new().unwrap();
        let security = Arc::new(SecurityPolicy::default());
        let mem_cfg = MemoryConfig {
            backend: "markdown".into(),
            ..MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> =
            Arc::from(crate::memory::create_memory(&mem_cfg, tmp.path(), None).unwrap());

        let browser = BrowserConfig::default();
        let codecoder = CodeCoderConfig::default();
        let vault = crate::config::VaultConfig::default();

        let tools = all_tools(&security, mem, &browser, &codecoder, &vault, tmp.path());
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"skill_search"));
    }

    #[test]
    fn default_tools_names() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security);
        let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
        assert!(names.contains(&"shell"));
        assert!(names.contains(&"file_read"));
        assert!(names.contains(&"file_write"));
    }

    #[test]
    fn default_tools_all_have_descriptions() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security);
        for tool in &tools {
            assert!(
                !tool.description().is_empty(),
                "Tool {} has empty description",
                tool.name()
            );
        }
    }

    #[test]
    fn default_tools_all_have_schemas() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security);
        for tool in &tools {
            let schema = tool.parameters_schema();
            assert!(
                schema.is_object(),
                "Tool {} schema is not an object",
                tool.name()
            );
            assert!(
                schema["properties"].is_object(),
                "Tool {} schema has no properties",
                tool.name()
            );
        }
    }

    #[test]
    fn tool_spec_generation() {
        let security = Arc::new(SecurityPolicy::default());
        let tools = default_tools(security);
        for tool in &tools {
            let spec = tool.spec();
            assert_eq!(spec.name, tool.name());
            assert_eq!(spec.description, tool.description());
            assert!(spec.parameters.is_object());
        }
    }

    #[test]
    fn tool_result_serde() {
        let result = ToolResult {
            success: true,
            output: "hello".into(),
            error: None,
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: ToolResult = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
        assert_eq!(parsed.output, "hello");
        assert!(parsed.error.is_none());
    }

    #[test]
    fn tool_result_with_error_serde() {
        let result = ToolResult {
            success: false,
            output: String::new(),
            error: Some("boom".into()),
        };
        let json = serde_json::to_string(&result).unwrap();
        let parsed: ToolResult = serde_json::from_str(&json).unwrap();
        assert!(!parsed.success);
        assert_eq!(parsed.error.as_deref(), Some("boom"));
    }

    #[test]
    fn tool_spec_serde() {
        let spec = ToolSpec {
            name: "test".into(),
            description: "A test tool".into(),
            parameters: serde_json::json!({"type": "object"}),
        };
        let json = serde_json::to_string(&spec).unwrap();
        let parsed: ToolSpec = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.name, "test");
        assert_eq!(parsed.description, "A test tool");
    }
}
