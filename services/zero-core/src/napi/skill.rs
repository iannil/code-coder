//! NAPI bindings for Skill Parser
//!
//! Provides Node.js bindings for parsing SKILL.md files with YAML frontmatter.

use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::collections::HashMap;
use std::path::Path;

use crate::skill;

// ============================================================================
// NAPI Types
// ============================================================================

/// Skill metadata from YAML frontmatter
#[napi(object)]
pub struct NapiSkillMetadata {
    /// Skill name
    pub name: String,
    /// Skill description
    pub description: String,
    /// Skill category
    pub category: Option<String>,
    /// Trigger patterns
    pub triggers: Vec<String>,
    /// Whether user-invocable
    pub user_invocable: bool,
    /// Required tools
    pub required_tools: Vec<String>,
    /// Skill version
    pub version: Option<String>,
    /// Author
    pub author: Option<String>,
}

impl From<skill::SkillMetadata> for NapiSkillMetadata {
    fn from(m: skill::SkillMetadata) -> Self {
        Self {
            name: m.name,
            description: m.description,
            category: m.category,
            triggers: m.triggers,
            user_invocable: m.user_invocable,
            required_tools: m.required_tools,
            version: m.version,
            author: m.author,
        }
    }
}

/// Parsed skill result
#[napi(object)]
pub struct NapiParsedSkill {
    /// Skill metadata
    pub metadata: NapiSkillMetadata,
    /// Markdown content body
    pub content: String,
    /// Raw frontmatter YAML
    pub raw_frontmatter: String,
}

impl From<skill::ParsedSkill> for NapiParsedSkill {
    fn from(s: skill::ParsedSkill) -> Self {
        Self {
            metadata: s.metadata.into(),
            content: s.content,
            raw_frontmatter: s.raw_frontmatter,
        }
    }
}

/// Skill parse error info
#[napi(object)]
pub struct NapiSkillParseError {
    /// Error message
    pub message: String,
    /// Line number (if available)
    pub line: Option<u32>,
    /// Column number (if available)
    pub column: Option<u32>,
}

impl From<skill::SkillParseError> for NapiSkillParseError {
    fn from(e: skill::SkillParseError) -> Self {
        Self {
            message: e.message,
            line: e.line.map(|l| l as u32),
            column: e.column.map(|c| c as u32),
        }
    }
}

// ============================================================================
// NAPI Functions
// ============================================================================

/// Parse a skill from content string
#[napi]
pub fn parse_skill_content(content: String) -> Result<NapiParsedSkill> {
    skill::parse_skill(&content)
        .map(NapiParsedSkill::from)
        .map_err(|e| Error::from_reason(e.to_string()))
}

/// Parse a skill from a file path
#[napi]
pub fn parse_skill_from_file(path: String) -> Result<NapiParsedSkill> {
    skill::parse_skill_file(Path::new(&path))
        .map(NapiParsedSkill::from)
        .map_err(|e| Error::from_reason(e.to_string()))
}

/// Parse just the metadata from skill content (faster for listing)
#[napi]
pub fn parse_skill_metadata_only(content: String) -> Result<NapiSkillMetadata> {
    skill::parse_skill_metadata(&content)
        .map(NapiSkillMetadata::from)
        .map_err(|e| Error::from_reason(e.to_string()))
}

/// Validate a skill without returning the full parsed result
#[napi]
pub fn validate_skill_content(content: String) -> Result<bool> {
    match skill::validate_skill(&content) {
        Ok(()) => Ok(true),
        Err(e) => Err(Error::from_reason(e.to_string())),
    }
}

/// Parse multiple skills from an array of contents (batch operation)
#[napi]
pub fn parse_skills_batch(contents: Vec<String>) -> Vec<NapiParsedSkillResult> {
    contents
        .into_iter()
        .map(|content| match skill::parse_skill(&content) {
            Ok(s) => NapiParsedSkillResult {
                success: true,
                skill: Some(s.into()),
                error: None,
            },
            Err(e) => NapiParsedSkillResult {
                success: false,
                skill: None,
                error: Some(e.into()),
            },
        })
        .collect()
}

/// Result wrapper for batch parsing
#[napi(object)]
pub struct NapiParsedSkillResult {
    /// Whether parsing succeeded
    pub success: bool,
    /// Parsed skill (if success)
    pub skill: Option<NapiParsedSkill>,
    /// Error (if failure)
    pub error: Option<NapiSkillParseError>,
}

/// Extract just the frontmatter YAML from skill content
#[napi]
pub fn extract_skill_frontmatter(content: String) -> Option<String> {
    crate::markdown::extract_frontmatter(&content)
}

/// Strip frontmatter from skill content, returning just the body
#[napi]
pub fn strip_skill_frontmatter(content: String) -> String {
    crate::markdown::strip_frontmatter(&content).to_string()
}
