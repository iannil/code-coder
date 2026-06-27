/// ─── Skill trait ───────────────────────────────────────────────────────────
///
/// Skills are the system's "capability units".  In the MVP they are
/// loaded from markdown files under `skills/`.  A skill describes how
/// to accomplish a task — the LLM reads the markdown and follows the
/// instructions.  Future versions will compile skills to WASM.

use std::collections::HashMap;

/// Metadata parsed from a skill's frontmatter.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub version: String,
}

/// A loaded skill, backed by its markdown source.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Skill {
    pub meta: SkillMeta,
    pub source: String,
    pub path: String,
}

/// Discovers and loads skills from the filesystem.
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Scan the `skills/` directory and load all `.md` files.
    pub fn scan(&mut self, base_path: &str) -> anyhow::Result<()> {
        let skills_dir = std::path::Path::new(base_path).join("skills");
        if !skills_dir.exists() {
            return Ok(()); // no skills dir yet — that's fine
        }

        let entries = std::fs::read_dir(&skills_dir)
            .map_err(|e| anyhow::anyhow!("cannot read {skills_dir:?}: {e}"))?;

        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "md") {
                let source = std::fs::read_to_string(&path)?;
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unnamed")
                    .to_string();
                let meta = SkillMeta {
                    description: format!("Skill loaded from {}", path.display()),
                    name: name.clone(),
                    version: "0.1.0".into(),
                };
                self.skills.insert(
                    name,
                    Skill {
                        meta,
                        source,
                        path: path.to_string_lossy().to_string(),
                    },
                );
            }
        }
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    pub fn list(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.skills.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }
}
