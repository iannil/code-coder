//! Lazy skill loading with LRU cache.
//!
//! This module provides on-demand skill loading to reduce startup time
//! and memory usage. Skills are loaded when first accessed and cached
//! using an LRU (Least Recently Used) policy.
//!
//! ## Design Principle
//!
//! Skill loading is a **deterministic** operation - it reads files from
//! disk and parses them. The TypeScript layer handles context-based
//! predictions for which skills to preload.

use chrono::{DateTime, Utc};
use lru::LruCache;
use std::collections::HashMap;
use std::num::NonZeroUsize;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

use super::{load_skill_json, load_skill_md, Skill};

/// A loaded skill with metadata.
#[derive(Debug, Clone)]
pub struct LoadedSkill {
    /// The skill data
    pub skill: Skill,
    /// When the skill was loaded
    pub loaded_at: DateTime<Utc>,
    /// When the skill was last accessed
    pub last_used: DateTime<Utc>,
    /// Number of times the skill has been accessed
    pub access_count: u64,
}

impl LoadedSkill {
    /// Create a new loaded skill entry.
    pub fn new(skill: Skill) -> Self {
        let now = Utc::now();
        Self {
            skill,
            loaded_at: now,
            last_used: now,
            access_count: 1,
        }
    }

    /// Mark the skill as accessed.
    pub fn touch(&mut self) {
        self.last_used = Utc::now();
        self.access_count += 1;
    }

    /// Get the time since last access.
    pub fn idle_time(&self) -> chrono::Duration {
        Utc::now() - self.last_used
    }
}

/// Skill index entry (lightweight metadata for discovery).
#[derive(Debug, Clone)]
pub struct SkillIndex {
    /// Skill identifier (usually directory name)
    pub id: String,
    /// Path to the skill directory
    pub path: PathBuf,
    /// Whether the skill uses SKILL.json or SKILL.md
    pub format: SkillFormat,
    /// Optional cached name (from quick parse)
    pub name: Option<String>,
    /// Optional cached description (from quick parse)
    pub description: Option<String>,
}

/// Skill manifest format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkillFormat {
    Json,
    Markdown,
}

/// Configuration for the skill loader.
#[derive(Debug, Clone)]
pub struct SkillLoaderConfig {
    /// Maximum number of skills to keep loaded
    pub max_loaded: usize,
    /// Duration after which idle skills are unloaded (in seconds)
    pub idle_timeout_secs: u64,
    /// Whether to build index at startup
    pub build_index: bool,
}

impl Default for SkillLoaderConfig {
    fn default() -> Self {
        Self {
            max_loaded: 50,
            idle_timeout_secs: 600, // 10 minutes
            build_index: true,
        }
    }
}

/// Lazy skill loader with LRU caching.
pub struct SkillLoader {
    /// Workspace directory containing skills
    workspace_dir: PathBuf,
    /// Loaded skills cache
    cache: Arc<RwLock<LruCache<String, LoadedSkill>>>,
    /// Skill index (all available skills)
    index: Arc<RwLock<HashMap<String, SkillIndex>>>,
    /// Configuration
    config: SkillLoaderConfig,
}

impl SkillLoader {
    /// Create a new skill loader.
    pub fn new(workspace_dir: PathBuf, config: SkillLoaderConfig) -> Self {
        let cache_size = NonZeroUsize::new(config.max_loaded).unwrap_or(NonZeroUsize::MIN);
        let loader = Self {
            workspace_dir: workspace_dir.clone(),
            cache: Arc::new(RwLock::new(LruCache::new(cache_size))),
            index: Arc::new(RwLock::new(HashMap::new())),
            config,
        };

        if loader.config.build_index {
            loader.rebuild_index();
        }

        loader
    }

    /// Create with default configuration.
    pub fn with_defaults(workspace_dir: PathBuf) -> Self {
        Self::new(workspace_dir, SkillLoaderConfig::default())
    }

    /// Rebuild the skill index by scanning the skills directory.
    pub fn rebuild_index(&self) {
        let skills_dir = self.workspace_dir.join("skills");
        if !skills_dir.exists() {
            debug!("Skills directory does not exist: {:?}", skills_dir);
            return;
        }

        let mut index = self.index.write().unwrap();
        index.clear();

        let entries = match std::fs::read_dir(&skills_dir) {
            Ok(e) => e,
            Err(e) => {
                warn!("Failed to read skills directory: {}", e);
                return;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let id = match path.file_name().and_then(|n| n.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };

            let manifest_path = path.join("SKILL.json");
            let md_path = path.join("SKILL.md");

            let format = if manifest_path.exists() {
                SkillFormat::Json
            } else if md_path.exists() {
                SkillFormat::Markdown
            } else {
                continue; // No skill manifest
            };

            index.insert(
                id.clone(),
                SkillIndex {
                    id,
                    path,
                    format,
                    name: None,
                    description: None,
                },
            );
        }

        info!("Indexed {} skills", index.len());
    }

    /// List all available skill IDs.
    pub fn list_available(&self) -> Vec<String> {
        let index = self.index.read().unwrap();
        index.keys().cloned().collect()
    }

    /// Check if a skill exists in the index.
    pub fn exists(&self, skill_id: &str) -> bool {
        let index = self.index.read().unwrap();
        index.contains_key(skill_id)
    }

    /// Check if a skill is currently loaded.
    pub fn is_loaded(&self, skill_id: &str) -> bool {
        let cache = self.cache.read().unwrap();
        cache.contains(skill_id)
    }

    /// Load a skill by ID. Returns cached version if available.
    pub fn load(&self, skill_id: &str) -> Option<Skill> {
        // Check cache first
        {
            let mut cache = self.cache.write().unwrap();
            if let Some(loaded) = cache.get_mut(skill_id) {
                loaded.touch();
                return Some(loaded.skill.clone());
            }
        }

        // Not cached, load from disk
        let index_entry = {
            let index = self.index.read().unwrap();
            index.get(skill_id).cloned()
        };

        let entry = index_entry?;

        let skill = match entry.format {
            SkillFormat::Json => {
                let manifest_path = entry.path.join("SKILL.json");
                load_skill_json(&manifest_path).ok()
            }
            SkillFormat::Markdown => {
                let md_path = entry.path.join("SKILL.md");
                load_skill_md(&md_path, &entry.path).ok()
            }
        };

        if let Some(skill) = skill {
            let mut cache = self.cache.write().unwrap();
            cache.put(skill_id.to_string(), LoadedSkill::new(skill.clone()));
            debug!("Loaded skill: {}", skill_id);
            Some(skill)
        } else {
            warn!("Failed to load skill: {}", skill_id);
            None
        }
    }

    /// Get a skill only if it's already loaded (no disk access).
    pub fn get_if_loaded(&self, skill_id: &str) -> Option<Skill> {
        let mut cache = self.cache.write().unwrap();
        cache.get_mut(skill_id).map(|loaded| {
            loaded.touch();
            loaded.skill.clone()
        })
    }

    /// Unload a specific skill from cache.
    pub fn unload(&self, skill_id: &str) -> bool {
        let mut cache = self.cache.write().unwrap();
        cache.pop(skill_id).is_some()
    }

    /// Unload skills that have been idle longer than the threshold.
    pub fn unload_idle(&self) -> Vec<String> {
        let threshold = chrono::Duration::seconds(self.config.idle_timeout_secs as i64);
        let mut unloaded = Vec::new();

        let mut cache = self.cache.write().unwrap();

        // Collect IDs of idle skills
        let idle_ids: Vec<String> = cache
            .iter()
            .filter(|(_, loaded)| loaded.idle_time() > threshold)
            .map(|(id, _)| id.clone())
            .collect();

        // Remove them
        for id in idle_ids {
            cache.pop(&id);
            unloaded.push(id);
        }

        if !unloaded.is_empty() {
            debug!("Unloaded {} idle skills", unloaded.len());
        }

        unloaded
    }

    /// Get statistics about the loader state.
    pub fn stats(&self) -> SkillLoaderStats {
        let cache = self.cache.read().unwrap();
        let index = self.index.read().unwrap();

        let total_accesses: u64 = cache.iter().map(|(_, l)| l.access_count).sum();
        let oldest_loaded = cache.iter().map(|(_, l)| l.loaded_at).min();

        SkillLoaderStats {
            available_count: index.len(),
            loaded_count: cache.len(),
            cache_capacity: self.config.max_loaded,
            total_accesses,
            oldest_loaded,
        }
    }

    /// Preload multiple skills at once.
    pub fn preload(&self, skill_ids: &[&str]) {
        for id in skill_ids {
            self.load(id);
        }
    }

    /// Clear all loaded skills from cache.
    pub fn clear_cache(&self) {
        let mut cache = self.cache.write().unwrap();
        cache.clear();
        info!("Cleared skill cache");
    }
}

/// Statistics about the skill loader.
#[derive(Debug, Clone)]
pub struct SkillLoaderStats {
    /// Total skills in the index
    pub available_count: usize,
    /// Currently loaded skills
    pub loaded_count: usize,
    /// Maximum cache capacity
    pub cache_capacity: usize,
    /// Total access count across all skills
    pub total_accesses: u64,
    /// When the oldest skill was loaded
    pub oldest_loaded: Option<DateTime<Utc>>,
}

impl SkillLoaderStats {
    /// Cache utilization as a percentage.
    pub fn cache_utilization(&self) -> f64 {
        if self.cache_capacity == 0 {
            0.0
        } else {
            (self.loaded_count as f64 / self.cache_capacity as f64) * 100.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn create_test_skill_dir() -> (PathBuf, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let skills_dir = dir.path().join("skills");
        fs::create_dir_all(&skills_dir).unwrap();
        (dir.path().to_path_buf(), dir)
    }

    fn create_json_skill(skills_dir: &Path, name: &str) {
        let skill_dir = skills_dir.join("skills").join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.json"),
            format!(
                r#"{{
                    "skill": {{
                        "name": "{}",
                        "description": "Test skill {}",
                        "version": "1.0.0"
                    }}
                }}"#,
                name, name
            ),
        )
        .unwrap();
    }

    fn create_md_skill(skills_dir: &Path, name: &str) {
        let skill_dir = skills_dir.join("skills").join(name);
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(
            skill_dir.join("SKILL.md"),
            format!("# {}\nTest markdown skill {}", name, name),
        )
        .unwrap();
    }

    #[test]
    fn test_skill_loader_empty() {
        let (workspace, _dir) = create_test_skill_dir();
        let loader = SkillLoader::with_defaults(workspace);

        assert!(loader.list_available().is_empty());
        assert_eq!(loader.stats().available_count, 0);
    }

    #[test]
    fn test_skill_loader_index() {
        let (workspace, _dir) = create_test_skill_dir();
        create_json_skill(&workspace, "skill-1");
        create_md_skill(&workspace, "skill-2");

        let loader = SkillLoader::with_defaults(workspace);
        let available = loader.list_available();

        assert_eq!(available.len(), 2);
        assert!(loader.exists("skill-1"));
        assert!(loader.exists("skill-2"));
        assert!(!loader.exists("nonexistent"));
    }

    #[test]
    fn test_skill_loader_load() {
        let (workspace, _dir) = create_test_skill_dir();
        create_json_skill(&workspace, "test-skill");

        let loader = SkillLoader::with_defaults(workspace);

        assert!(!loader.is_loaded("test-skill"));

        let skill = loader.load("test-skill");
        assert!(skill.is_some());
        assert_eq!(skill.unwrap().name, "test-skill");

        assert!(loader.is_loaded("test-skill"));
    }

    #[test]
    fn test_skill_loader_cache() {
        let (workspace, _dir) = create_test_skill_dir();
        create_json_skill(&workspace, "cached-skill");

        let loader = SkillLoader::with_defaults(workspace);

        // First load - from disk
        let skill1 = loader.load("cached-skill");
        assert!(skill1.is_some());

        // Second load - from cache
        let skill2 = loader.load("cached-skill");
        assert!(skill2.is_some());

        // Access count should be 2
        let cache = loader.cache.read().unwrap();
        assert_eq!(cache.peek("cached-skill").unwrap().access_count, 2);
    }

    #[test]
    fn test_skill_loader_get_if_loaded() {
        let (workspace, _dir) = create_test_skill_dir();
        create_json_skill(&workspace, "test-skill");

        let loader = SkillLoader::with_defaults(workspace);

        // Not loaded yet
        assert!(loader.get_if_loaded("test-skill").is_none());

        // Load it
        loader.load("test-skill");

        // Now it's available
        assert!(loader.get_if_loaded("test-skill").is_some());
    }

    #[test]
    fn test_skill_loader_unload() {
        let (workspace, _dir) = create_test_skill_dir();
        create_json_skill(&workspace, "to-unload");

        let loader = SkillLoader::with_defaults(workspace);
        loader.load("to-unload");

        assert!(loader.is_loaded("to-unload"));
        assert!(loader.unload("to-unload"));
        assert!(!loader.is_loaded("to-unload"));

        // Unloading non-existent returns false
        assert!(!loader.unload("to-unload"));
    }

    #[test]
    fn test_skill_loader_lru_eviction() {
        let (workspace, _dir) = create_test_skill_dir();
        create_json_skill(&workspace, "skill-1");
        create_json_skill(&workspace, "skill-2");
        create_json_skill(&workspace, "skill-3");

        // Create loader with capacity of 2
        let config = SkillLoaderConfig {
            max_loaded: 2,
            ..Default::default()
        };
        let loader = SkillLoader::new(workspace, config);

        // Load skill-1 and skill-2
        loader.load("skill-1");
        loader.load("skill-2");

        assert!(loader.is_loaded("skill-1"));
        assert!(loader.is_loaded("skill-2"));

        // Load skill-3 - should evict skill-1 (LRU)
        loader.load("skill-3");

        assert!(!loader.is_loaded("skill-1")); // Evicted
        assert!(loader.is_loaded("skill-2"));
        assert!(loader.is_loaded("skill-3"));
    }

    #[test]
    fn test_skill_loader_preload() {
        let (workspace, _dir) = create_test_skill_dir();
        create_json_skill(&workspace, "skill-a");
        create_json_skill(&workspace, "skill-b");

        let loader = SkillLoader::with_defaults(workspace);

        loader.preload(&["skill-a", "skill-b", "nonexistent"]);

        assert!(loader.is_loaded("skill-a"));
        assert!(loader.is_loaded("skill-b"));
        assert!(!loader.is_loaded("nonexistent"));
    }

    #[test]
    fn test_skill_loader_stats() {
        let (workspace, _dir) = create_test_skill_dir();
        create_json_skill(&workspace, "stat-skill");

        let loader = SkillLoader::with_defaults(workspace);

        let stats = loader.stats();
        assert_eq!(stats.available_count, 1);
        assert_eq!(stats.loaded_count, 0);

        loader.load("stat-skill");

        let stats = loader.stats();
        assert_eq!(stats.loaded_count, 1);
        assert_eq!(stats.total_accesses, 1);
    }

    #[test]
    fn test_skill_loader_clear_cache() {
        let (workspace, _dir) = create_test_skill_dir();
        create_json_skill(&workspace, "clear-skill");

        let loader = SkillLoader::with_defaults(workspace);
        loader.load("clear-skill");

        assert!(loader.is_loaded("clear-skill"));

        loader.clear_cache();

        assert!(!loader.is_loaded("clear-skill"));
    }

    #[test]
    fn test_loaded_skill_touch() {
        let skill = Skill {
            name: "test".to_string(),
            description: "Test".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            tags: vec![],
            tools: vec![],
            prompts: vec![],
            location: None,
        };

        let mut loaded = LoadedSkill::new(skill);
        assert_eq!(loaded.access_count, 1);

        std::thread::sleep(std::time::Duration::from_millis(10));
        loaded.touch();

        assert_eq!(loaded.access_count, 2);
        assert!(loaded.idle_time() < chrono::Duration::seconds(1));
    }

    #[test]
    fn test_skill_format() {
        assert_eq!(SkillFormat::Json, SkillFormat::Json);
        assert_ne!(SkillFormat::Json, SkillFormat::Markdown);
    }

    #[test]
    fn test_stats_cache_utilization() {
        let stats = SkillLoaderStats {
            available_count: 10,
            loaded_count: 5,
            cache_capacity: 50,
            total_accesses: 100,
            oldest_loaded: None,
        };

        assert_eq!(stats.cache_utilization(), 10.0);
    }
}
