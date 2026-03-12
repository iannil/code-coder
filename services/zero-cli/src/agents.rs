//! Agent management commands
//!
//! This module provides commands for searching, listing, and inspecting agents.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use zero_core::agent::{
    create_builtin_metadata, AgentCategory, AgentMetadata, AgentRole, MetadataIndex, SearchOptions,
};

/// Agent subcommands
#[derive(Debug, Clone)]
pub enum AgentsCommand {
    /// Search for agents by query
    Search {
        query: String,
        limit: usize,
    },
    /// List all agents
    List {
        category: Option<String>,
        mode: Option<String>,
    },
    /// Show detailed agent info
    Info {
        name: String,
    },
    /// Recommend an agent for a task
    Recommend {
        intent: String,
    },
}

/// Agent info output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub category: String,
    pub role: String,
    pub tags: Vec<String>,
    pub capabilities: Vec<String>,
    pub triggers: Vec<String>,
}

impl From<&AgentMetadata> for AgentInfo {
    fn from(m: &AgentMetadata) -> Self {
        Self {
            name: m.name.clone(),
            display_name: m.display_name.clone(),
            description: m.short_description.clone(),
            category: format!("{:?}", m.category),
            role: format!("{:?}", m.role),
            tags: m.tags.clone(),
            capabilities: m.capabilities.iter().map(|c| c.name.clone()).collect(),
            triggers: m.triggers.iter().map(|t| t.value.clone()).collect(),
        }
    }
}

/// Create and populate the metadata index
fn create_index() -> MetadataIndex {
    let mut index = MetadataIndex::new();
    for (_, meta) in create_builtin_metadata() {
        index.register(meta);
    }
    index
}

/// Execute agents search command
pub fn search(query: &str, limit: usize) -> Result<()> {
    let index = create_index();

    let options = SearchOptions {
        limit,
        threshold: 0.3,
    };

    let results = index.search(query, Some(options));

    if results.is_empty() {
        println!("No agents found matching '{}'", query);
        return Ok(());
    }

    println!("🔍 Search results for '{}' ({} found):\n", query, results.len());

    for result in results {
        let m = &result.agent;
        let desc = m.short_description.as_deref().unwrap_or("");
        let score = (result.score * 100.0) as u32;

        println!("  {} ({}% match)", m.name, score);
        if !desc.is_empty() {
            println!("    {}", desc);
        }
        println!("    Category: {:?} | Role: {:?}", m.category, m.role);
        if !m.tags.is_empty() {
            println!("    Tags: {}", m.tags.join(", "));
        }
        println!();
    }

    Ok(())
}

/// Execute agents list command
pub fn list(category: Option<&str>, mode: Option<&str>) -> Result<()> {
    let index = create_index();

    let agents: Vec<_> = if let Some(cat) = category {
        let cat_enum = match cat.to_lowercase().as_str() {
            "engineering" => AgentCategory::Engineering,
            "content" => AgentCategory::Content,
            "analysis" => AgentCategory::Analysis,
            "philosophy" => AgentCategory::Philosophy,
            "system" => AgentCategory::System,
            _ => AgentCategory::Custom,
        };
        index.list_by_category(cat_enum).into_iter().cloned().collect()
    } else if let Some(m) = mode {
        index.list_by_mode(m).into_iter().cloned().collect()
    } else {
        index.list_visible().into_iter().cloned().collect()
    };

    if agents.is_empty() {
        println!("No agents found");
        return Ok(());
    }

    println!("📋 Available agents ({}):\n", agents.len());

    // Group by category
    let mut by_category: std::collections::HashMap<String, Vec<&AgentMetadata>> =
        std::collections::HashMap::new();

    for agent in &agents {
        let cat = format!("{:?}", agent.category);
        by_category.entry(cat).or_default().push(agent);
    }

    for (category, agents) in by_category {
        println!("  {} [{}]", category_emoji(&category), category);
        for agent in agents {
            let desc = agent.short_description.as_deref().unwrap_or("");
            let role_marker = match agent.role {
                AgentRole::Primary => "★",
                AgentRole::Alternative => "◆",
                AgentRole::Capability => "●",
                AgentRole::System => "⚙",
                AgentRole::Hidden => "○",
            };
            println!("    {} {} - {}", role_marker, agent.name, desc);
        }
        println!();
    }

    println!("Legend: ★ Primary  ◆ Alternative  ● Capability  ⚙ System");

    Ok(())
}

/// Execute agents info command
pub fn info(name: &str) -> Result<()> {
    let index = create_index();

    let agent = match index.get(name) {
        Some(a) => a,
        None => {
            println!("Agent '{}' not found", name);
            return Ok(());
        }
    };

    println!("🤖 Agent: {}", agent.name);
    println!("═══════════════════════════════════════════════════════\n");

    if let Some(display) = &agent.display_name {
        println!("Display Name: {}", display);
    }

    if let Some(desc) = &agent.short_description {
        println!("Description:  {}", desc);
    }

    println!("Category:     {:?}", agent.category);
    println!("Role:         {:?}", agent.role);

    if let Some(mode) = &agent.mode {
        println!("Mode:         {}", mode);
    }

    if agent.recommended {
        println!("Recommended:  ✅ Yes");
    }

    if !agent.tags.is_empty() {
        println!("\nTags: {}", agent.tags.join(", "));
    }

    if !agent.capabilities.is_empty() {
        println!("\n📦 Capabilities:");
        for cap in &agent.capabilities {
            let primary = if cap.primary { " (primary)" } else { "" };
            println!("  • {}{}: {}", cap.name, primary, cap.description);
        }
    }

    if !agent.triggers.is_empty() {
        println!("\n⚡ Triggers:");
        for trigger in &agent.triggers {
            println!(
                "  • [{:?}] '{}' (priority: {})",
                trigger.trigger_type, trigger.value, trigger.priority
            );
        }
    }

    if !agent.examples.is_empty() {
        println!("\n📝 Examples:");
        for example in &agent.examples {
            println!("  {} ", example.title);
            println!("    Input:  {}", example.input);
            println!("    Output: {}", example.output);
        }
    }

    if let Some(long_desc) = &agent.long_description {
        println!("\n📄 Full Description:");
        println!("{}", long_desc);
    }

    Ok(())
}

/// Execute agents recommend command
pub fn recommend(intent: &str) -> Result<()> {
    let index = create_index();

    match index.recommend(intent) {
        Some(agent) => {
            println!("🎯 Recommended agent for '{}':\n", intent);
            println!("  {}", agent.name);
            if let Some(desc) = &agent.short_description {
                println!("  {}\n", desc);
            }
            println!("  Category: {:?} | Role: {:?}", agent.category, agent.role);
            if !agent.capabilities.is_empty() {
                let cap_names: Vec<_> = agent.capabilities.iter().map(|c| c.name.as_str()).collect();
                println!("  Capabilities: {}", cap_names.join(", "));
            }
        }
        None => {
            println!("No agent recommendation for '{}'", intent);
        }
    }

    Ok(())
}

/// Get emoji for category
fn category_emoji(category: &str) -> &'static str {
    match category.to_lowercase().as_str() {
        "engineering" => "🔧",
        "content" => "📝",
        "analysis" => "📊",
        "philosophy" => "🧠",
        "system" => "⚙️",
        _ => "📦",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_index() {
        let index = create_index();
        assert!(index.list().len() > 5);
    }

    #[test]
    fn test_search() {
        let index = create_index();
        let results = index.search("code review", None);
        assert!(!results.is_empty());
    }
}
