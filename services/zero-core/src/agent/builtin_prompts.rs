//! Built-in agent prompts embedded in the binary.
//!
//! These prompts are available without external files.

/// Get embedded prompt content by agent name
pub fn get_builtin_prompt(name: &str) -> Option<&'static str> {
    match name {
        "build" => Some(include_str!("prompts/build.txt")),
        "plan" => Some(include_str!("prompts/build.txt")), // Plan uses build prompt as base
        "explore" => Some(include_str!("prompts/explore.txt")),
        "general" => Some(include_str!("prompts/general.txt")),
        "compaction" => Some(include_str!("prompts/compaction.txt")),
        "title" => Some(include_str!("prompts/title.txt")),
        "summary" => Some(include_str!("prompts/summary.txt")),
        "code-reviewer" => Some(include_str!("prompts/code-reviewer.txt")),
        "security-reviewer" => Some(include_str!("prompts/security-reviewer.txt")),
        "tdd-guide" => Some(include_str!("prompts/tdd-guide.txt")),
        "architect" => Some(include_str!("prompts/architect.txt")),
        "writer" => Some(include_str!("prompts/writer.txt")),
        "expander" => Some(include_str!("prompts/expander.txt")),
        "proofreader" => Some(include_str!("prompts/proofreader.txt")),
        "code-reverse" => Some(include_str!("prompts/code-reverse.txt")),
        "jar-code-reverse" => Some(include_str!("prompts/jar-code-reverse.txt")),
        "observer" => Some(include_str!("prompts/observer.txt")),
        "decision" => Some(include_str!("prompts/decision.txt")),
        "macro" => Some(include_str!("prompts/macro.txt")),
        "trader" => Some(include_str!("prompts/trader.txt")),
        "picker" => Some(include_str!("prompts/picker.txt")),
        "miniproduct" => Some(include_str!("prompts/miniproduct.txt")),
        "synton-assistant" => Some(include_str!("prompts/synton-assistant.txt")),
        "ai-engineer" => Some(include_str!("prompts/ai-engineer.txt")),
        "value-analyst" => Some(include_str!("prompts/value-analyst.txt")),
        "verifier" => Some(include_str!("prompts/verifier.txt")),
        "autonomous" => Some(include_str!("prompts/autonomous.txt")),
        "prd-generator" => Some(include_str!("prompts/prd-generator.txt")),
        "feasibility-assess" => Some(include_str!("prompts/feasibility-assess.txt")),
        _ => None,
    }
}

/// List all built-in agent names
pub fn list_builtin_agents() -> &'static [&'static str] {
    &[
        "build",
        "plan",
        "explore",
        "general",
        "compaction",
        "title",
        "summary",
        "code-reviewer",
        "security-reviewer",
        "tdd-guide",
        "architect",
        "writer",
        "expander",
        "proofreader",
        "code-reverse",
        "jar-code-reverse",
        "observer",
        "decision",
        "macro",
        "trader",
        "picker",
        "miniproduct",
        "synton-assistant",
        "ai-engineer",
        "value-analyst",
        "verifier",
        "autonomous",
        "prd-generator",
        "feasibility-assess",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_builtin_prompt() {
        let build = get_builtin_prompt("build");
        assert!(build.is_some());
        assert!(build.unwrap().contains("Build Agent"));
    }

    #[test]
    fn test_all_builtins_exist() {
        for name in list_builtin_agents() {
            let prompt = get_builtin_prompt(name);
            assert!(prompt.is_some(), "Missing prompt for: {}", name);
            assert!(!prompt.unwrap().is_empty(), "Empty prompt for: {}", name);
        }
    }
}
