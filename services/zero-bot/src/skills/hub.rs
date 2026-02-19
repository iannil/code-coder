//! `SkillHub` - Registry client for discovering and managing skills
//!
//! Provides search, info, and update checking functionality
//! against a remote skill registry.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Default `SkillHub` registry URL
const SKILLHUB_REGISTRY: &str =
    "https://raw.githubusercontent.com/zerobot-skills/registry/main/index.json";

/// Cache time-to-live in seconds (5 minutes)
const CACHE_TTL_SECS: u64 = 300;

/// Skill metadata from the registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMeta {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub repo_url: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub updated_at: String,
}

/// Internal cache structure
#[derive(Debug, Default)]
struct SkillIndex {
    skills: HashMap<String, SkillMeta>,
    last_updated: Option<Instant>,
}

/// `SkillHub` client for searching and managing skills
pub struct SkillHub {
    http_client: reqwest::Client,
    cache: Arc<RwLock<SkillIndex>>,
    registry_url: String,
}

impl SkillHub {
    /// Create a new `SkillHub` client with default registry
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
            cache: Arc::new(RwLock::new(SkillIndex::default())),
            registry_url: SKILLHUB_REGISTRY.to_string(),
        }
    }

    /// Create a SkillHub client with custom registry URL (for testing)
    #[cfg(test)]
    pub fn with_registry(url: &str) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            cache: Arc::new(RwLock::new(SkillIndex::default())),
            registry_url: url.to_string(),
        }
    }

    /// Search for skills by query string
    ///
    /// Searches name, description, and tags. Results are sorted by downloads.
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SkillMeta>> {
        self.refresh_index_if_needed().await?;

        let index = self.cache.read().await;
        let query_lower = query.to_lowercase();

        let mut results: Vec<_> = index
            .skills
            .values()
            .filter(|s| {
                s.name.to_lowercase().contains(&query_lower)
                    || s.description.to_lowercase().contains(&query_lower)
                    || s.tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            })
            .cloned()
            .collect();

        // Sort by downloads (popularity)
        results.sort_by(|a, b| b.downloads.cmp(&a.downloads));
        results.truncate(limit);

        Ok(results)
    }

    /// Get skill info by exact name
    pub async fn get_info(&self, name: &str) -> Result<Option<SkillMeta>> {
        self.refresh_index_if_needed().await?;
        let index = self.cache.read().await;
        Ok(index.skills.get(name).cloned())
    }

    /// Check for updates to installed skills
    ///
    /// Returns a list of (name, `current_version`, `latest_version`) tuples
    pub async fn check_updates(
        &self,
        installed: &[super::Skill],
    ) -> Result<Vec<(String, String, String)>> {
        self.refresh_index_if_needed().await?;
        let index = self.cache.read().await;

        let mut updates = Vec::new();
        for skill in installed {
            if let Some(remote) = index.skills.get(&skill.name) {
                if remote.version != skill.version && skill.version != "open-skills" {
                    updates.push((
                        skill.name.clone(),
                        skill.version.clone(),
                        remote.version.clone(),
                    ));
                }
            }
        }

        Ok(updates)
    }

    /// List all available skills in the registry
    pub async fn list_all(&self, limit: usize) -> Result<Vec<SkillMeta>> {
        self.refresh_index_if_needed().await?;

        let index = self.cache.read().await;
        let mut results: Vec<_> = index.skills.values().cloned().collect();

        // Sort by downloads
        results.sort_by(|a, b| b.downloads.cmp(&a.downloads));
        results.truncate(limit);

        Ok(results)
    }

    /// Refresh the index cache if needed (expired or never loaded)
    async fn refresh_index_if_needed(&self) -> Result<()> {
        let needs_refresh = {
            let index = self.cache.read().await;
            index
                .last_updated
                .is_none_or(|t| t.elapsed().as_secs() > CACHE_TTL_SECS)
        };

        if needs_refresh {
            self.fetch_index().await?;
        }

        Ok(())
    }

    /// Fetch the skill index from the registry
    async fn fetch_index(&self) -> Result<()> {
        let response = self
            .http_client
            .get(&self.registry_url)
            .header("User-Agent", "ZeroBot/0.1.0")
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to fetch skill registry: HTTP {}",
                response.status()
            );
        }

        let skills: Vec<SkillMeta> = response.json().await?;

        let mut index = self.cache.write().await;
        index.skills = skills.into_iter().map(|s| (s.name.clone(), s)).collect();
        index.last_updated = Some(Instant::now());

        Ok(())
    }

    /// Force refresh the cache (for testing or manual refresh)
    #[allow(dead_code)]
    pub async fn force_refresh(&self) -> Result<()> {
        self.fetch_index().await
    }
}

impl Default for SkillHub {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_registry() -> Vec<SkillMeta> {
        vec![
            SkillMeta {
                name: "code-review".to_string(),
                description: "AI-powered code review".to_string(),
                version: "1.0.0".to_string(),
                author: "zerobot-skills".to_string(),
                repo_url: "https://github.com/zerobot-skills/code-review".to_string(),
                tags: vec!["code".to_string(), "review".to_string()],
                downloads: 1000,
                updated_at: "2026-02-15T10:00:00Z".to_string(),
            },
            SkillMeta {
                name: "web-scraper".to_string(),
                description: "Web scraping toolkit".to_string(),
                version: "0.5.0".to_string(),
                author: "community".to_string(),
                repo_url: "https://github.com/example/web-scraper".to_string(),
                tags: vec!["web".to_string(), "scraping".to_string()],
                downloads: 500,
                updated_at: "2026-02-10T08:00:00Z".to_string(),
            },
        ]
    }

    #[tokio::test]
    async fn skill_meta_deserialize() {
        let json = r#"{
            "name": "test-skill",
            "description": "A test",
            "version": "1.0.0",
            "author": "tester",
            "repo_url": "https://github.com/test/skill",
            "tags": ["test"],
            "downloads": 100,
            "updated_at": "2026-01-01T00:00:00Z"
        }"#;

        let meta: SkillMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.name, "test-skill");
        assert_eq!(meta.version, "1.0.0");
        assert_eq!(meta.downloads, 100);
    }

    #[tokio::test]
    async fn skill_meta_deserialize_minimal() {
        let json = r#"{
            "name": "minimal",
            "description": "Minimal skill",
            "version": "0.1.0",
            "author": "test",
            "repo_url": "https://example.com"
        }"#;

        let meta: SkillMeta = serde_json::from_str(json).unwrap();
        assert_eq!(meta.name, "minimal");
        assert!(meta.tags.is_empty());
        assert_eq!(meta.downloads, 0);
    }

    #[test]
    fn skillhub_default() {
        let hub = SkillHub::default();
        assert_eq!(hub.registry_url, SKILLHUB_REGISTRY);
    }

    #[tokio::test]
    async fn search_filters_by_name() {
        let hub = SkillHub::new();

        // Pre-populate cache
        {
            let mut index = hub.cache.write().await;
            for skill in sample_registry() {
                index.skills.insert(skill.name.clone(), skill);
            }
            index.last_updated = Some(Instant::now());
        }

        let results = hub.search("code", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "code-review");
    }

    #[tokio::test]
    async fn search_filters_by_description() {
        let hub = SkillHub::new();

        {
            let mut index = hub.cache.write().await;
            for skill in sample_registry() {
                index.skills.insert(skill.name.clone(), skill);
            }
            index.last_updated = Some(Instant::now());
        }

        let results = hub.search("scraping", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "web-scraper");
    }

    #[tokio::test]
    async fn search_filters_by_tags() {
        let hub = SkillHub::new();

        {
            let mut index = hub.cache.write().await;
            for skill in sample_registry() {
                index.skills.insert(skill.name.clone(), skill);
            }
            index.last_updated = Some(Instant::now());
        }

        let results = hub.search("web", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "web-scraper");
    }

    #[tokio::test]
    async fn search_case_insensitive() {
        let hub = SkillHub::new();

        {
            let mut index = hub.cache.write().await;
            for skill in sample_registry() {
                index.skills.insert(skill.name.clone(), skill);
            }
            index.last_updated = Some(Instant::now());
        }

        let results = hub.search("CODE", 10).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn search_respects_limit() {
        let hub = SkillHub::new();

        {
            let mut index = hub.cache.write().await;
            for skill in sample_registry() {
                index.skills.insert(skill.name.clone(), skill);
            }
            index.last_updated = Some(Instant::now());
        }

        // Search matches both skills but limit to 1
        let results = hub.search("", 1).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn search_sorts_by_downloads() {
        let hub = SkillHub::new();

        {
            let mut index = hub.cache.write().await;
            for skill in sample_registry() {
                index.skills.insert(skill.name.clone(), skill);
            }
            index.last_updated = Some(Instant::now());
        }

        let results = hub.search("", 10).await.unwrap();
        assert_eq!(results.len(), 2);
        // Higher downloads first
        assert_eq!(results[0].name, "code-review");
        assert_eq!(results[1].name, "web-scraper");
    }

    #[tokio::test]
    async fn get_info_existing() {
        let hub = SkillHub::new();

        {
            let mut index = hub.cache.write().await;
            for skill in sample_registry() {
                index.skills.insert(skill.name.clone(), skill);
            }
            index.last_updated = Some(Instant::now());
        }

        let info = hub.get_info("code-review").await.unwrap();
        assert!(info.is_some());
        assert_eq!(info.unwrap().version, "1.0.0");
    }

    #[tokio::test]
    async fn get_info_nonexistent() {
        let hub = SkillHub::new();

        {
            let mut index = hub.cache.write().await;
            index.last_updated = Some(Instant::now());
        }

        let info = hub.get_info("nonexistent").await.unwrap();
        assert!(info.is_none());
    }

    #[tokio::test]
    async fn check_updates_finds_outdated() {
        let hub = SkillHub::new();

        {
            let mut index = hub.cache.write().await;
            for skill in sample_registry() {
                index.skills.insert(skill.name.clone(), skill);
            }
            index.last_updated = Some(Instant::now());
        }

        let installed = vec![crate::skills::Skill {
            name: "code-review".to_string(),
            description: "Old version".to_string(),
            version: "0.9.0".to_string(),
            author: None,
            tags: vec![],
            tools: vec![],
            prompts: vec![],
            location: None,
        }];

        let updates = hub.check_updates(&installed).await.unwrap();
        assert_eq!(updates.len(), 1);
        assert_eq!(updates[0].0, "code-review");
        assert_eq!(updates[0].1, "0.9.0");
        assert_eq!(updates[0].2, "1.0.0");
    }

    #[tokio::test]
    async fn check_updates_ignores_current() {
        let hub = SkillHub::new();

        {
            let mut index = hub.cache.write().await;
            for skill in sample_registry() {
                index.skills.insert(skill.name.clone(), skill);
            }
            index.last_updated = Some(Instant::now());
        }

        let installed = vec![crate::skills::Skill {
            name: "code-review".to_string(),
            description: "Current".to_string(),
            version: "1.0.0".to_string(),
            author: None,
            tags: vec![],
            tools: vec![],
            prompts: vec![],
            location: None,
        }];

        let updates = hub.check_updates(&installed).await.unwrap();
        assert!(updates.is_empty());
    }

    #[tokio::test]
    async fn check_updates_ignores_open_skills() {
        let hub = SkillHub::new();

        {
            let mut index = hub.cache.write().await;
            for skill in sample_registry() {
                index.skills.insert(skill.name.clone(), skill);
            }
            index.last_updated = Some(Instant::now());
        }

        let installed = vec![crate::skills::Skill {
            name: "code-review".to_string(),
            description: "From open-skills".to_string(),
            version: "open-skills".to_string(),
            author: Some("besoeasy/open-skills".to_string()),
            tags: vec![],
            tools: vec![],
            prompts: vec![],
            location: None,
        }];

        let updates = hub.check_updates(&installed).await.unwrap();
        assert!(updates.is_empty());
    }

    #[tokio::test]
    async fn list_all_respects_limit() {
        let hub = SkillHub::new();

        {
            let mut index = hub.cache.write().await;
            for skill in sample_registry() {
                index.skills.insert(skill.name.clone(), skill);
            }
            index.last_updated = Some(Instant::now());
        }

        let results = hub.list_all(1).await.unwrap();
        assert_eq!(results.len(), 1);
    }
}
