/// ─── Context ───────────────────────────────────────────────────────────────
///
/// Loads project context files (AGENTS.md, skills/, memory/) from the
/// filesystem and assembles them into the system prompt.

use std::path::Path;

/// All context loaded at startup / reload time.
#[derive(Debug, Clone)]
pub struct Context {
    /// Content of AGENTS.md (empty if file doesn't exist).
    pub agents_md: String,
    /// Base path for the project.
    pub project_root: String,
}

impl Context {
    /// Load context from the given project root.
    pub fn load(project_root: &str) -> Self {
        let agents_path = Path::new(project_root).join("AGENTS.md");
        let agents_md = if agents_path.exists() {
            std::fs::read_to_string(&agents_path).unwrap_or_default()
        } else {
            String::new()
        };

        Self {
            agents_md,
            project_root: project_root.to_string(),
        }
    }

    /// Format the context as a section to inject into the system prompt.
    pub fn format_system_section(&self) -> String {
        let mut section = String::new();
        section.push_str("\n\n## Project Context\n\n");

        if !self.agents_md.is_empty() {
            section.push_str(&self.agents_md);
            section.push('\n');
        } else {
            section.push_str("(No AGENTS.md found — this is a fresh project.)\n");
        }

        section
    }
}

/// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_nonexistent() {
        let ctx = Context::load("/tmp/nonexistent_codecoder_project_xyz");
        assert!(ctx.agents_md.is_empty());
        let section = ctx.format_system_section();
        assert!(section.contains("No AGENTS.md found"));
    }

    #[test]
    fn test_load_existing() {
        let dir = tempfile::tempdir().unwrap();
        let agents_path = dir.path().join("AGENTS.md");
        std::fs::write(&agents_path, "# Test Project\nThis is a test.").unwrap();

        let ctx = Context::load(dir.path().to_str().unwrap());
        assert!(ctx.agents_md.contains("Test Project"));
        let section = ctx.format_system_section();
        assert!(section.contains("Test Project"));
    }
}
