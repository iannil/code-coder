/// ─── Skill trait ───────────────────────────────────────────────────────────
///
/// Skills are the system's "capability units".  In the MVP they are
/// loaded from markdown files under `skills/`.  A skill describes how
/// to accomplish a task — the LLM reads the markdown and follows the
/// instructions.  Future versions will compile skills to WASM.

use std::collections::HashMap;
use crate::memory::MemoryStore;

/// Skill lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillStatus {
    Draft,
    Active,
    NeedsUpgrade,
}

impl SkillStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillStatus::Draft => "draft",
            SkillStatus::Active => "active",
            SkillStatus::NeedsUpgrade => "needs_upgrade",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "active" => SkillStatus::Active,
            "needs_upgrade" => SkillStatus::NeedsUpgrade,
            _ => SkillStatus::Draft,
        }
    }
}

/// Where the skill comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillSource {
    SelfGenerated,
    UserCreated,
    BuiltIn,
}

impl SkillSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            SkillSource::SelfGenerated => "self-generated",
            SkillSource::UserCreated => "user-created",
            SkillSource::BuiltIn => "built-in",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "self-generated" => SkillSource::SelfGenerated,
            "built-in" => SkillSource::BuiltIn,
            _ => SkillSource::UserCreated,
        }
    }
}

/// Metadata parsed from a skill's frontmatter.
#[derive(Debug, Clone)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub version: String,
    pub status: SkillStatus,
    pub source: SkillSource,
    pub trigger: String,
    pub usage_count: u32,
}

impl SkillMeta {
    /// Default metadata for a skill loaded without frontmatter.
    fn from_name(name: &str, path: &str) -> Self {
        Self {
            name: name.to_string(),
            description: format!("Skill loaded from {path}"),
            version: "0.1.0".into(),
            status: SkillStatus::Active,
            source: SkillSource::UserCreated,
            trigger: String::new(),
            usage_count: 0,
        }
    }
}

/// A loaded skill, backed by its markdown source.
#[derive(Debug, Clone)]
pub struct Skill {
    pub meta: SkillMeta,
    pub source: String,
    pub path: String,
}

/// Discovers and loads skills from the filesystem.
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
    /// Optional MemoryStore for dual-write sync of usage_count and status.
    memory_store: Option<MemoryStore>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
            memory_store: None,
        }
    }

    /// Attach a MemoryStore for dual-write sync.
    /// Skills loaded via scan() will sync their frontmatter status
    /// to memory, and record_usage()/promote() will keep them in sync.
    pub fn set_memory_store(&mut self, store: MemoryStore) {
        self.memory_store = Some(store);
    }

    /// Scan the `skills/` directory and load all `.md` files.
    /// Parses YAML frontmatter (delimited by `---`) when present.
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
                let path_str = path.to_string_lossy().to_string();
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unnamed")
                    .to_string();

                let meta = parse_frontmatter(&source)
                    .unwrap_or_else(|| SkillMeta::from_name(&name, &path_str));

                self.skills.insert(
                    name.clone(),
                    Skill {
                        meta: meta.clone(),
                        source,
                        path: path_str,
                    },
                );
                // Sync frontmatter to MemoryStore (dual-write)
                sync_meta_to_store(&mut self.memory_store, &name, &meta);
            }
        }
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Skill> {
        self.skills.get_mut(name)
    }

    pub fn list(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.skills.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// List skills filtered by status.
    pub fn list_by_status(&self, status: SkillStatus) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|s| s.meta.status == status)
            .collect()
    }

    /// Increment the usage count for a skill and update its file.
    /// Returns true if the skill was promoted as a result.
    pub fn record_usage(&mut self, name: &str, _project_root: &str) -> anyhow::Result<bool> {
        let skill = self.skills.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("skill '{name}' not found"))?;

        skill.meta.usage_count = skill.meta.usage_count.saturating_add(1);

        // Auto-promotion: first use or cumulative threshold
        let was_draft = skill.meta.status == SkillStatus::Draft;
        if was_draft && skill.meta.usage_count >= 3 {
            skill.meta.status = SkillStatus::Active;
        }

        // Write updated frontmatter back to file
        update_frontmatter_in_file(&skill.path, &skill.meta)?;

        // Dual-write: sync to MemoryStore
        sync_meta_to_store(&mut self.memory_store, name, &skill.meta);

        Ok(was_draft && skill.meta.status == SkillStatus::Active)
    }

    /// Manually promote a draft skill to active.
    pub fn promote(&mut self, name: &str) -> anyhow::Result<bool> {
        let skill = self.skills.get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("skill '{name}' not found"))?;

        if skill.meta.status == SkillStatus::Active {
            return Ok(false); // already active
        }

        skill.meta.status = SkillStatus::Active;
        update_frontmatter_in_file(&skill.path, &skill.meta)?;

        // Dual-write: sync to MemoryStore
        sync_meta_to_store(&mut self.memory_store, name, &skill.meta);

        Ok(true)
    }
}

/// Sync a skill's usage_count and status to the attached MemoryStore.
fn sync_meta_to_store(store: &mut Option<MemoryStore>, name: &str, meta: &SkillMeta) {
    if let Some(store) = store {
        let usage_key = format!("skill:{name}:usage_count");
        let status_key = format!("skill:{name}:status");
        let _ = store.set(&usage_key, &meta.usage_count.to_string());
        let _ = store.set(&status_key, meta.status.as_str());
    }
}

// ─── Frontmatter parsing ─────────────────────────────────────────────────────

/// Try to parse YAML frontmatter from markdown content.
/// Format: first line `---`, then key: value lines, then closing `---`.
/// Returns None if no valid frontmatter is found.
fn parse_frontmatter(source: &str) -> Option<SkillMeta> {
    let source = source.trim_start();
    if !source.starts_with("---\n") && !source.starts_with("---\r\n") {
        return None;
    }

    // Find the closing ---
    let after_open = source[3..].trim_start(); // skip past opening ---
    let end = after_open.find("\n---")?;
    let block = &after_open[..end];

    let mut name = String::new();
    let mut description = String::new();
    let mut version = String::from("0.1.0");
    let mut status = SkillStatus::Draft;
    let mut source_kind = SkillSource::SelfGenerated;
    let mut trigger = String::new();
    let mut usage_count = 0u32;

    for line in block.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let colon = match line.find(':') {
            Some(p) => p,
            None => continue,
        };
        let key = line[..colon].trim().to_lowercase();
        let val = line[colon + 1..].trim().trim_matches('"').to_string();

        match key.as_str() {
            "name" => name = val,
            "description" => description = val,
            "version" => version = val,
            "status" => status = SkillStatus::from_str(&val),
            "source" => source_kind = SkillSource::from_str(&val),
            "trigger" => trigger = val,
            "usage_count" => {
                usage_count = val.parse::<u32>().unwrap_or(0);
            }
            _ => {}
        }
    }

    if name.is_empty() {
        return None;
    }

    Some(SkillMeta {
        name,
        description,
        version,
        status,
        source: source_kind,
        trigger,
        usage_count,
    })
}

/// Build a YAML frontmatter string from metadata.
fn build_frontmatter(meta: &SkillMeta) -> String {
    format!(
        "---\nname: {}\ndescription: {}\nversion: {}\nstatus: {}\nsource: {}\ntrigger: {}\nusage_count: {}\n---",
        meta.name,
        meta.description,
        meta.version,
        meta.status.as_str(),
        meta.source.as_str(),
        meta.trigger,
        meta.usage_count,
    )
}

/// Replace the frontmatter block in a markdown file with updated values.
fn update_frontmatter_in_file(path: &str, meta: &SkillMeta) -> anyhow::Result<()> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("cannot read {path}: {e}"))?;

    let new_fm = build_frontmatter(meta);

    // If the file starts with frontmatter, replace it; otherwise prepend it
    let trimmed = content.trim_start();
    let updated = if trimmed.starts_with("---\n") || trimmed.starts_with("---\r\n") {
        // Find end of frontmatter
        let after_open = trimmed[3..].trim_start();
        if let Some(end) = after_open.find("\n---") {
            let body = &after_open[end + 4..]; // skip past closing ---
            format!("{}\n\n{}", new_fm, body.trim())
        } else {
            // malformed — just prepend
            format!("{}\n\n{}", new_fm, trimmed)
        }
    } else {
        format!("{}\n\n{}", new_fm, trimmed)
    };

    std::fs::write(path, updated.as_bytes())
        .map_err(|e| anyhow::anyhow!("cannot write {path}: {e}"))?;

    Ok(())
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter_full() {
        let src = "---\nname: git-helper\ndescription: Git operations helper\nversion: 0.2.0\nstatus: draft\nsource: self-generated\ntrigger: \"git\"\nusage_count: 0\n---\n\n# Git Helper\n\nSome content.";
        let meta = parse_frontmatter(src).unwrap();
        assert_eq!(meta.name, "git-helper");
        assert_eq!(meta.description, "Git operations helper");
        assert_eq!(meta.version, "0.2.0");
        assert_eq!(meta.status, SkillStatus::Draft);
        assert_eq!(meta.source, SkillSource::SelfGenerated);
        assert_eq!(meta.trigger, "git");
        assert_eq!(meta.usage_count, 0);
    }

    #[test]
    fn test_parse_frontmatter_minimal() {
        let src = "---\nname: test\n---\n\nContent.";
        let meta = parse_frontmatter(src).unwrap();
        assert_eq!(meta.name, "test");
        assert_eq!(meta.status, SkillStatus::Draft);
        assert_eq!(meta.source, SkillSource::SelfGenerated);
        assert_eq!(meta.version, "0.1.0");
        assert!(meta.description.is_empty());
    }

    #[test]
    fn test_parse_frontmatter_none() {
        let src = "# Just a markdown file\n\nNo frontmatter.";
        assert!(parse_frontmatter(src).is_none());
    }

    #[test]
    fn test_parse_frontmatter_missing_name() {
        let src = "---\ndescription: no name here\n---\n\nContent.";
        assert!(parse_frontmatter(src).is_none());
    }

    #[test]
    fn test_build_frontmatter_roundtrip() {
        let meta = SkillMeta {
            name: "my-skill".into(),
            description: "Does stuff".into(),
            version: "1.0.0".into(),
            status: SkillStatus::Active,
            source: SkillSource::UserCreated,
            trigger: "search".into(),
            usage_count: 5,
        };
        let fm = build_frontmatter(&meta);
        assert!(fm.starts_with("---"));
        assert!(fm.contains("name: my-skill"));
        assert!(fm.contains("status: active"));
        assert!(fm.contains("usage_count: 5"));

        // Parse it back
        let full_doc = format!("{}\n\n# Content", fm);
        let parsed = parse_frontmatter(&full_doc).unwrap();
        assert_eq!(parsed.name, "my-skill");
        assert_eq!(parsed.usage_count, 5);
        assert_eq!(parsed.status, SkillStatus::Active);
    }

    #[test]
    fn test_update_frontmatter_in_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.md");
        std::fs::write(&path, "---\nname: test\nusage_count: 0\n---\n\nBody").unwrap();

        let meta = SkillMeta {
            name: "test".into(),
            description: "Updated".into(),
            version: "0.1.0".into(),
            status: SkillStatus::Active,
            source: SkillSource::SelfGenerated,
            trigger: String::new(),
            usage_count: 3,
        };

        update_frontmatter_in_file(path.to_str().unwrap(), &meta).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("usage_count: 3"));
        assert!(content.contains("status: active"));
        assert!(content.contains("Body"));

        // Parse back
        let parsed = parse_frontmatter(&content).unwrap();
        assert_eq!(parsed.usage_count, 3);
        assert_eq!(parsed.status, SkillStatus::Active);
    }

    #[test]
    fn test_scan_loads_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();

        let skill_path = skills_dir.join("git-helper.md");
        std::fs::write(&skill_path,
            "---\nname: git-helper\ndescription: Git helper\nstatus: draft\nsource: self-generated\ntrigger: git\nusage_count: 0\n---\n\n# Git Helper\n\nContent."
        ).unwrap();

        let mut registry = SkillRegistry::new();
        registry.scan(dir.path().to_str().unwrap()).unwrap();

        let skill = registry.get("git-helper").unwrap();
        assert_eq!(skill.meta.status, SkillStatus::Draft);
        assert_eq!(skill.meta.source, SkillSource::SelfGenerated);
        assert_eq!(skill.meta.trigger, "git");
    }

    #[test]
    fn test_scan_fallback_no_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();
        std::fs::write(skills_dir.join("legacy.md"), "# Legacy Skill\n\nOld format.").unwrap();

        let mut registry = SkillRegistry::new();
        registry.scan(dir.path().to_str().unwrap()).unwrap();

        let skill = registry.get("legacy").unwrap();
        assert_eq!(skill.meta.status, SkillStatus::Active); // default for old skills
        assert_eq!(skill.meta.source, SkillSource::UserCreated);
    }

    #[test]
    fn test_record_usage_promotes_after_3() {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();
        let skill_path = skills_dir.join("test.md");
        std::fs::write(&skill_path,
            "---\nname: test\nstatus: draft\nusage_count: 0\n---\n\nBody."
        ).unwrap();

        let mut registry = SkillRegistry::new();
        registry.scan(dir.path().to_str().unwrap()).unwrap();

        // First two uses: still draft
        for i in 1..=2 {
            let promoted = registry.record_usage("test", dir.path().to_str().unwrap()).unwrap();
            assert!(!promoted, "iteration {i}: should not promote yet");
        }

        // Third use: promote
        let promoted = registry.record_usage("test", dir.path().to_str().unwrap()).unwrap();
        assert!(promoted);

        // Verify file was updated
        let content = std::fs::read_to_string(&skill_path).unwrap();
        assert!(content.contains("status: active"));
        assert!(content.contains("usage_count: 3"));
    }

    #[test]
    fn test_dual_write_to_memory_store() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_str().unwrap();

        // Create a skill file
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();
        let skill_path = skills_dir.join("test.md");
        std::fs::write(&skill_path,
            "---\nname: test\nstatus: draft\nusage_count: 0\n---\n\nBody."
        ).unwrap();

        // Create registry + memory store and wire them together
        let mut registry = SkillRegistry::new();
        let store = MemoryStore::open(root);
        registry.set_memory_store(store);
        registry.scan(root).unwrap();

        // Verify memory store has the synced values from scan
        // Memory keys are stored as memory/<key>.md files
        let usage_key = format!("skill:test:usage_count");
        let status_key = format!("skill:test:status");

        // Re-open MemoryStore to verify persistence
        let verify_store = MemoryStore::open(root);
        let usage_entry = verify_store.get(&usage_key).expect("usage_count should exist in memory");
        assert_eq!(usage_entry.value, "0", "scan should sync usage_count=0");
        let status_entry = verify_store.get(&status_key).expect("status should exist in memory");
        assert_eq!(status_entry.value, "draft", "scan should sync status=draft");

        // Now record usage — should update both file and memory
        let promoted = registry.record_usage("test", root).unwrap();
        assert!(!promoted, "first use should not promote");

        // Verify memory store was updated
        let verify_store2 = MemoryStore::open(root);
        let usage_entry2 = verify_store2.get(&usage_key).expect("usage_count should exist after record_usage");
        assert_eq!(usage_entry2.value, "1", "record_usage should update memory to 1");

        // Third use triggers promotion
        registry.record_usage("test", root).unwrap();
        registry.record_usage("test", root).unwrap();

        let verify_store3 = MemoryStore::open(root);
        let usage_entry3 = verify_store3.get(&usage_key).unwrap();
        assert_eq!(usage_entry3.value, "3");
        let status_entry3 = verify_store3.get(&status_key).unwrap();
        assert_eq!(status_entry3.value, "active", "promotion should update memory status");
    }
}
