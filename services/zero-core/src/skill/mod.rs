//! Skill Parser Module
//!
//! Provides high-performance parsing of SKILL.md files with YAML frontmatter.
//! Skills are Markdown files with structured metadata for tool/agent definitions.
//!
//! # Example
//!
//! ```rust
//! use zero_core::skill::{parse_skill, SkillMetadata};
//!
//! let content = r#"---
//! name: my-skill
//! description: A helpful skill
//! ---
//!
//! # My Skill
//!
//! Content here...
//! "#;
//!
//! let skill = parse_skill(content).unwrap();
//! assert_eq!(skill.metadata.name, "my-skill");
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// ============================================================================
// Types
// ============================================================================

/// Skill metadata extracted from YAML frontmatter
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillMetadata {
    /// Skill name (required)
    pub name: String,
    /// Skill description (required)
    pub description: String,
    /// Skill category (optional)
    #[serde(default)]
    pub category: Option<String>,
    /// Trigger patterns for skill activation (optional)
    #[serde(default)]
    pub triggers: Vec<String>,
    /// Whether the skill is user-invocable (optional, default true)
    #[serde(default = "default_true")]
    pub user_invocable: bool,
    /// Required tools for the skill (optional)
    #[serde(default)]
    pub required_tools: Vec<String>,
    /// Skill version (optional)
    #[serde(default)]
    pub version: Option<String>,
    /// Author information (optional)
    #[serde(default)]
    pub author: Option<String>,
    /// Additional custom fields
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

fn default_true() -> bool {
    true
}

/// Parsed skill document
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ParsedSkill {
    /// Skill metadata from frontmatter
    pub metadata: SkillMetadata,
    /// Markdown content body (without frontmatter)
    pub content: String,
    /// Raw frontmatter YAML (for debugging/advanced use)
    pub raw_frontmatter: String,
}

/// Error during skill parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillParseError {
    /// Error message
    pub message: String,
    /// Line number where error occurred (if applicable)
    pub line: Option<usize>,
    /// Column number where error occurred (if applicable)
    pub column: Option<usize>,
}

impl std::fmt::Display for SkillParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let (Some(line), Some(col)) = (self.line, self.column) {
            write!(f, "{} at line {}, column {}", self.message, line, col)
        } else if let Some(line) = self.line {
            write!(f, "{} at line {}", self.message, line)
        } else {
            write!(f, "{}", self.message)
        }
    }
}

impl std::error::Error for SkillParseError {}

// ============================================================================
// Parser Implementation
// ============================================================================

/// Parse a skill document from Markdown content with YAML frontmatter.
///
/// The skill format is:
/// ```text
/// ---
/// name: skill-name
/// description: Skill description
/// ---
///
/// # Markdown Content
///
/// Instructions and documentation...
/// ```
pub fn parse_skill(content: &str) -> Result<ParsedSkill, SkillParseError> {
    // Extract frontmatter
    let raw_frontmatter = extract_frontmatter(content).ok_or_else(|| SkillParseError {
        message: "Missing YAML frontmatter (must start with ---)".to_string(),
        line: Some(1),
        column: Some(1),
    })?;

    // Preprocess frontmatter (handle colons in values)
    let processed_frontmatter = preprocess_frontmatter(&raw_frontmatter);

    // Parse YAML
    let metadata: SkillMetadata = serde_yaml::from_str(&processed_frontmatter).map_err(|e| {
        let (line, column) = match e.location() {
            Some(loc) => (Some(loc.line()), Some(loc.column())),
            None => (None, None),
        };
        SkillParseError {
            message: format!("Invalid YAML frontmatter: {}", e),
            line,
            column,
        }
    })?;

    // Validate required fields
    if metadata.name.is_empty() {
        return Err(SkillParseError {
            message: "Missing required field: name".to_string(),
            line: None,
            column: None,
        });
    }

    if metadata.description.is_empty() {
        return Err(SkillParseError {
            message: "Missing required field: description".to_string(),
            line: None,
            column: None,
        });
    }

    // Extract body content
    let content_body = strip_frontmatter(content);

    Ok(ParsedSkill {
        metadata,
        content: content_body.to_string(),
        raw_frontmatter,
    })
}

/// Parse a skill from a file path
pub fn parse_skill_file(path: &Path) -> Result<ParsedSkill, SkillParseError> {
    let content = std::fs::read_to_string(path).map_err(|e| SkillParseError {
        message: format!("Failed to read file: {}", e),
        line: None,
        column: None,
    })?;

    parse_skill(&content)
}

/// Extract frontmatter from markdown content
fn extract_frontmatter(text: &str) -> Option<String> {
    if !text.starts_with("---") {
        return None;
    }

    let rest = &text[3..];
    let end_match = rest.find("\n---");

    end_match.map(|end_idx| {
        let fm = &rest[..end_idx];
        // Skip leading newline if present
        if fm.starts_with('\n') {
            fm[1..].to_string()
        } else {
            fm.to_string()
        }
    })
}

/// Strip frontmatter from markdown content
fn strip_frontmatter(text: &str) -> &str {
    if !text.starts_with("---") {
        return text;
    }

    let rest = &text[3..];
    if let Some(end_idx) = rest.find("\n---") {
        let after_end = end_idx + 4; // Skip past "\n---"
        if after_end < rest.len() {
            rest[after_end..].trim_start_matches('\n')
        } else {
            ""
        }
    } else {
        text
    }
}

/// Preprocess frontmatter to handle edge cases
///
/// - Converts values with colons to block scalars
/// - Preserves existing block scalars and quoted strings
fn preprocess_frontmatter(content: &str) -> String {
    let lines = content.lines();
    let mut result = Vec::new();

    for line in lines {
        // Skip comments and empty lines
        if line.trim().starts_with('#') || line.trim().is_empty() {
            result.push(line.to_string());
            continue;
        }

        // Skip lines that are continuations (indented)
        if line.starts_with(' ') || line.starts_with('\t') {
            result.push(line.to_string());
            continue;
        }

        // Match key: value pattern
        if let Some(colon_idx) = line.find(':') {
            let key = &line[..colon_idx];
            let value = line[colon_idx + 1..].trim();

            // Skip if value is empty, already quoted, or uses block scalar
            if value.is_empty()
                || value == ">"
                || value == "|"
                || value.starts_with('"')
                || value.starts_with('\'')
                || value.starts_with('[')
                || value.starts_with('{')
            {
                result.push(line.to_string());
                continue;
            }

            // If value contains a colon (and not already handled), convert to block scalar
            if value.contains(':') {
                result.push(format!("{}: |", key));
                result.push(format!("  {}", value));
                continue;
            }
        }

        result.push(line.to_string());
    }

    result.join("\n")
}

/// Parse just the metadata from a skill without the full content
pub fn parse_skill_metadata(content: &str) -> Result<SkillMetadata, SkillParseError> {
    let raw_frontmatter = extract_frontmatter(content).ok_or_else(|| SkillParseError {
        message: "Missing YAML frontmatter".to_string(),
        line: Some(1),
        column: Some(1),
    })?;

    let processed = preprocess_frontmatter(&raw_frontmatter);

    serde_yaml::from_str(&processed).map_err(|e| SkillParseError {
        message: format!("Invalid YAML: {}", e),
        line: e.location().map(|l| l.line()),
        column: e.location().map(|l| l.column()),
    })
}

/// Validate a skill document without fully parsing
pub fn validate_skill(content: &str) -> Result<(), SkillParseError> {
    parse_skill(content)?;
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_skill_basic() {
        let content = r#"---
name: test-skill
description: A test skill
---

# Test Skill

This is the content.
"#;

        let skill = parse_skill(content).unwrap();
        assert_eq!(skill.metadata.name, "test-skill");
        assert_eq!(skill.metadata.description, "A test skill");
        assert!(skill.content.contains("# Test Skill"));
    }

    #[test]
    fn test_parse_skill_with_triggers() {
        let content = r#"---
name: my-skill
description: Description
triggers:
  - pattern1
  - pattern2
---

Content
"#;

        let skill = parse_skill(content).unwrap();
        assert_eq!(skill.metadata.triggers.len(), 2);
        assert_eq!(skill.metadata.triggers[0], "pattern1");
    }

    #[test]
    fn test_parse_skill_with_colon_in_description() {
        let content = r#"---
name: test
description: This has a colon: inside
---

Content
"#;

        let skill = parse_skill(content).unwrap();
        // After preprocessing, the colon should be handled correctly
        assert!(skill.metadata.description.contains("colon"));
    }

    #[test]
    fn test_parse_skill_missing_frontmatter() {
        let content = "# No frontmatter here";
        let result = parse_skill(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_skill_missing_name() {
        let content = r#"---
description: Only description
---

Content
"#;

        let result = parse_skill(content);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("name"));
    }

    #[test]
    fn test_parse_skill_with_extra_fields() {
        let content = r#"---
name: test
description: Test
custom_field: custom_value
---

Content
"#;

        let skill = parse_skill(content).unwrap();
        assert!(skill.metadata.extra.contains_key("custom_field"));
    }

    #[test]
    fn test_preprocess_frontmatter() {
        let input = "name: test\ndescription: has: colon";
        let output = preprocess_frontmatter(input);
        assert!(output.contains("description: |"));
        assert!(output.contains("  has: colon"));
    }

    #[test]
    fn test_preprocess_frontmatter_quoted() {
        let input = "name: test\ndescription: \"already: quoted\"";
        let output = preprocess_frontmatter(input);
        assert_eq!(output, input);
    }

    #[test]
    fn test_validate_skill() {
        let valid = r#"---
name: valid
description: Valid skill
---

Content
"#;
        assert!(validate_skill(valid).is_ok());

        let invalid = "# No frontmatter";
        assert!(validate_skill(invalid).is_err());
    }

    #[test]
    fn test_parse_metadata_only() {
        let content = r#"---
name: meta-only
description: Testing metadata extraction
category: testing
---

Very long content that we don't need...
"#;

        let metadata = parse_skill_metadata(content).unwrap();
        assert_eq!(metadata.name, "meta-only");
        assert_eq!(metadata.category, Some("testing".to_string()));
    }
}
