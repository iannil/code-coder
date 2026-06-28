use super::Tool;

/// Generate a new skill and save it to `skills/<name>.md`.
///
/// Input format:
/// ```
/// <skill_name>
/// ---
/// <markdown content>
/// ```
pub struct GenerateSkill {
    pub project_root: String,
}

impl Tool for GenerateSkill {
    fn name(&self) -> &str {
        "generate_skill"
    }

    fn description(&self) -> &str {
        "Generate a new skill file in skills/. Input: '<name>\\n---\\n<markdown content>'"
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        let (name, content) = parse_name_content(input)?;
        let safe_name = sanitize_filename(&name);

        let skills_dir = std::path::Path::new(&self.project_root).join("skills");
        std::fs::create_dir_all(&skills_dir)
            .map_err(|e| anyhow::anyhow!("cannot create skills dir: {e}"))?;

        let path = skills_dir.join(format!("{}.md", safe_name));

        // Add frontmatter-style header if not present
        let final_content = if content.contains('#') {
            content.to_string()
        } else {
            format!("# {}\n\n{}", name, content)
        };

        std::fs::write(&path, &final_content)
            .map_err(|e| anyhow::anyhow!("cannot write skill: {e}"))?;

        Ok(format!(
            "Created skill '{name}' at {} ({} bytes)",
            path.display(),
            final_content.len()
        ))
    }
}

/// Generate a prompt template and save it to `prompts/<name>.md`.
///
/// Input format: same as generate_skill.
pub struct GeneratePrompt {
    pub project_root: String,
}

impl Tool for GeneratePrompt {
    fn name(&self) -> &str {
        "generate_prompt"
    }

    fn description(&self) -> &str {
        "Generate a prompt template in prompts/. Input: '<name>\\n---\\n<prompt content>'"
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        let (name, content) = parse_name_content(input)?;
        let safe_name = sanitize_filename(&name);

        let prompts_dir = std::path::Path::new(&self.project_root).join("prompts");
        std::fs::create_dir_all(&prompts_dir)
            .map_err(|e| anyhow::anyhow!("cannot create prompts dir: {e}"))?;

        let path = prompts_dir.join(format!("{}.md", safe_name));
        std::fs::write(&path, &content)
            .map_err(|e| anyhow::anyhow!("cannot write prompt: {e}"))?;

        Ok(format!(
            "Created prompt '{name}' at {} ({} bytes)",
            path.display(),
            content.len()
        ))
    }
}

/// Generate an executable tool script and save it to `tools/<name>`.
///
/// Input format:
/// ```
/// <tool_name>
/// ---
/// <script content>
/// ```
pub struct GenerateTool {
    pub project_root: String,
}

impl Tool for GenerateTool {
    fn name(&self) -> &str {
        "generate_tool"
    }

    fn description(&self) -> &str {
        "Generate an executable tool script in tools/. Input: '<name>\\n---\\n<script content>'"
    }

    fn execute(&self, input: &str) -> anyhow::Result<String> {
        let (name, content) = parse_name_content(input)?;
        let safe_name = sanitize_filename(&name);

        let tools_dir = std::path::Path::new(&self.project_root).join("tools");
        std::fs::create_dir_all(&tools_dir)
            .map_err(|e| anyhow::anyhow!("cannot create tools dir: {e}"))?;

        let path = tools_dir.join(&safe_name);

        // Add shebang if not present
        let final_content = if content.starts_with("#!") {
            content.to_string()
        } else {
            format!("#!/bin/sh\n\n{}", content)
        };

        std::fs::write(&path, &final_content)
            .map_err(|e| anyhow::anyhow!("cannot write tool: {e}"))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
                .map_err(|e| anyhow::anyhow!("cannot chmod tool: {e}"))?;
        }

        Ok(format!(
            "Created tool '{name}' at {} ({} bytes, executable)",
            path.display(),
            final_content.len()
        ))
    }
}

// ─── Helpers ────────────────────────────────────────────────────────────────

fn parse_name_content(input: &str) -> anyhow::Result<(String, String)> {
    let input = input.trim();
    let sep = input.find("\n---\n").or_else(|| input.find("\n---\r\n"));

    match sep {
        Some(pos) => {
            let name = input[..pos].trim().to_string();
            let content_start = pos + 5; // skip "\n---\n" or "\n---\r\n"
            let content = input[content_start..].trim().to_string();
            if name.is_empty() {
                anyhow::bail!("name cannot be empty");
            }
            if content.is_empty() {
                anyhow::bail!("content cannot be empty");
            }
            Ok((name, content))
        }
        None => anyhow::bail!(
            "input must be '<name>\\n---\\n<content>'"
        ),
    }
}

/// Sanitize a string for use as a filename: keep only alphanumeric,
/// hyphens, underscores, and dots (dots not allowed as first char).
/// Replaces all other characters with underscores.
fn sanitize_filename(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    for (i, c) in name.chars().enumerate() {
        if c.is_alphanumeric() || c == '-' || c == '_' {
            result.push(c);
        } else if i > 0 && c == '.' {
            result.push(c);
        } else {
            result.push('_');
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_name_content() {
        let input = "my-skill\n---\n# My Skill\n\nThis is a test skill.";
        let (name, content) = parse_name_content(input).unwrap();
        assert_eq!(name, "my-skill");
        assert_eq!(content, "# My Skill\n\nThis is a test skill.");
    }

    #[test]
    fn test_parse_name_content_with_crlf() {
        let input = "my-skill\r\n---\r\ncontent here";
        let (name, content) = parse_name_content(input).unwrap();
        assert_eq!(name, "my-skill");
        assert_eq!(content, "content here");
    }

    #[test]
    fn test_parse_missing_separator() {
        assert!(parse_name_content("no-separator-here").is_err());
    }

    #[test]
    fn test_parse_empty_name() {
        let input = "\n---\ncontent";
        assert!(parse_name_content(input).is_err());
    }

    #[test]
    fn test_parse_empty_content() {
        let input = "name\n---\n";
        assert!(parse_name_content(input).is_err());
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("hello world"), "hello_world");
        assert_eq!(sanitize_filename("my-skill_v2"), "my-skill_v2");
        assert_eq!(sanitize_filename("../evil"), "_._evil");
        assert_eq!(sanitize_filename(".hidden"), "_hidden");
    }

    #[test]
    fn test_generate_skill() {
        let dir = tempfile::tempdir().unwrap();
        let tool = GenerateSkill {
            project_root: dir.path().to_string_lossy().to_string(),
        };
        let input = "test-skill\n---\n# Test Skill\n\nA test.";
        let result = tool.execute(input).unwrap();
        assert!(result.contains("test-skill"));
        assert!(result.contains("bytes"));

        let path = dir.path().join("skills/test-skill.md");
        assert!(path.exists());
    }

    #[test]
    fn test_generate_skill_adds_header() {
        let dir = tempfile::tempdir().unwrap();
        let tool = GenerateSkill {
            project_root: dir.path().to_string_lossy().to_string(),
        };
        // Content without # should get a header added
        let input = "my-task\n---\njust some plain text";
        let result = tool.execute(input).unwrap();
        assert!(result.contains("my-task"));
        let path = dir.path().join("skills/my-task.md");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("# my-task"));
    }

    #[test]
    fn test_generate_tool_executable() {
        let dir = tempfile::tempdir().unwrap();
        let tool = GenerateTool {
            project_root: dir.path().to_string_lossy().to_string(),
        };
        let input = "hello\n---\necho hello world";
        let result = tool.execute(input).unwrap();
        assert!(result.contains("executable"));

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = std::fs::metadata(dir.path().join("tools/hello")).unwrap();
            assert!(meta.permissions().mode() & 0o111 != 0);
        }
    }

    #[test]
    fn test_generate_tool_with_shebang() {
        let dir = tempfile::tempdir().unwrap();
        let tool = GenerateTool {
            project_root: dir.path().to_string_lossy().to_string(),
        };
        let input = "script\n---\n#!/usr/bin/env bash\necho hi";
        let result = tool.execute(input).unwrap();
        assert!(result.contains("executable"));
        let path = dir.path().join("tools/script");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("#!/usr/bin/env bash"));
    }

    #[test]
    fn test_generate_prompt() {
        let dir = tempfile::tempdir().unwrap();
        let tool = GeneratePrompt {
            project_root: dir.path().to_string_lossy().to_string(),
        };
        let input = "review-prompt\n---\nReview this code for bugs.";
        let result = tool.execute(input).unwrap();
        assert!(result.contains("review-prompt"));

        let path = dir.path().join("prompts/review-prompt.md");
        assert!(path.exists());
    }

    #[test]
    fn test_generate_skill_empty_name() {
        let dir = tempfile::tempdir().unwrap();
        let tool = GenerateSkill {
            project_root: dir.path().to_string_lossy().to_string(),
        };
        let result = tool.execute("\n---\ncontent");
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_tool_empty_content() {
        let tool = GenerateTool {
            project_root: "/tmp".into(),
        };
        let result = tool.execute("foo\n---\n");
        assert!(result.is_err());
    }
}
