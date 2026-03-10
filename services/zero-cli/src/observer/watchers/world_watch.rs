//! World Watcher (WorldWatch)
//!
//! Observes external world data including:
//! - Market data (via macro/trader agents)
//! - News and headlines
//! - API changes and updates
//! - Dependency releases
//! - Security advisories
//!
//! Uses HTTP requests for external data fetching.

use crate::observer::types::{
    Observation, Sentiment, WorldData, WorldObservation, WorldObservationType, WatcherType,
};
use crate::observer::watchers::{
    BaseWatcherState, Watcher, WatcherMetrics, WatcherOptions, WatcherStatus,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;
use tracing::{debug, info, warn};

// ══════════════════════════════════════════════════════════════════════════════
// Configuration
// ══════════════════════════════════════════════════════════════════════════════

/// Configuration for WorldWatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldWatchConfig {
    /// Data sources to monitor
    #[serde(default)]
    pub sources: Vec<DataSource>,
    /// Keywords to filter news
    #[serde(default)]
    pub news_keywords: Vec<String>,
    /// Dependencies to track
    #[serde(default)]
    pub tracked_dependencies: Vec<String>,
    /// Enable Agent polling for macro data
    #[serde(default)]
    pub enable_agent_polling: bool,
    /// Agent polling interval (in observation cycles)
    #[serde(default = "default_agent_polling_cycles")]
    pub agent_polling_cycles: u32,
    /// Common watcher options
    #[serde(flatten)]
    pub options: WatcherOptions,
}

/// Data source configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSource {
    /// Source type
    #[serde(rename = "type")]
    pub source_type: WorldObservationType,
    /// URL to fetch from
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// API key for authentication
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Custom refresh interval in ms
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_interval: Option<u64>,
}

fn default_agent_polling_cycles() -> u32 {
    5
}

impl Default for WorldWatchConfig {
    fn default() -> Self {
        Self {
            sources: Vec::new(),
            news_keywords: Vec::new(),
            tracked_dependencies: Vec::new(),
            enable_agent_polling: false,
            agent_polling_cycles: 5,
            options: WatcherOptions {
                interval_ms: 300000, // Check every 5 minutes
                ..Default::default()
            },
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Public Data Types
// ══════════════════════════════════════════════════════════════════════════════

/// Market data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarketDataPoint {
    pub symbol: String,
    pub price: f64,
    pub change: f64,
    pub change_percent: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<u64>,
    pub timestamp: DateTime<Utc>,
}

/// News item.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsItem {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    pub source: String,
    pub url: String,
    pub published_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sentiment: Option<Sentiment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relevance_score: Option<f32>,
}

/// Security advisory.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityAdvisory {
    pub id: String,
    pub severity: AdvisorySeverity,
    pub package: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fixed_in: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cve: Option<String>,
}

/// Advisory severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AdvisorySeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Dependency release information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DependencyRelease {
    pub package: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub release_notes: Option<String>,
    #[serde(default)]
    pub is_breaking: bool,
}

/// API change information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiChange {
    pub api: String,
    pub version: String,
    #[serde(default)]
    pub breaking_changes: Vec<String>,
    #[serde(default)]
    pub deprecations: Vec<String>,
    #[serde(default)]
    pub new_features: Vec<String>,
}

/// Trend information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendInfo {
    pub name: String,
    pub description: String,
    pub direction: TrendDirection,
    pub strength: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_points: Option<Vec<serde_json::Value>>,
}

/// Trend direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Up,
    Down,
    Stable,
}

// ══════════════════════════════════════════════════════════════════════════════
// WorldWatch Implementation
// ══════════════════════════════════════════════════════════════════════════════

/// Watcher that observes external world data.
#[allow(dead_code)]
pub struct WorldWatch {
    /// Base watcher state
    state: BaseWatcherState,
    /// Configuration
    config: WorldWatchConfig,
    /// Last check timestamps per source
    last_checks: HashMap<String, DateTime<Utc>>,
    /// Observation cycle counter
    observation_cycle: u32,
    /// HTTP client
    client: reqwest::Client,
    /// Pending observations queue
    pending_observations: Vec<Observation>,
}

impl WorldWatch {
    /// Create a new WorldWatch instance.
    pub fn new(config: WorldWatchConfig) -> Self {
        let id = config
            .options
            .id
            .clone()
            .unwrap_or_else(|| BaseWatcherState::generate_id("world"));

        Self {
            state: BaseWatcherState::new(id),
            config,
            last_checks: HashMap::new(),
            observation_cycle: 0,
            client: reqwest::Client::new(),
            pending_observations: Vec::new(),
        }
    }

    /// Create a world observation.
    fn create_observation(
        &self,
        obs_type: WorldObservationType,
        source: impl Into<String>,
        data: WorldData,
    ) -> WorldObservation {
        let mut obs = WorldObservation::new(&self.state.id, obs_type, source);
        obs.data = data;
        obs
    }

    /// Calculate news relevance based on keywords.
    fn calculate_news_relevance(&self, item: &NewsItem) -> f32 {
        if self.config.news_keywords.is_empty() {
            return 0.5;
        }

        let title_lower = item.title.to_lowercase();
        let summary_lower = item.summary.as_ref().map(|s| s.to_lowercase()).unwrap_or_default();
        let content = format!("{title_lower} {summary_lower}");

        let match_count = self
            .config
            .news_keywords
            .iter()
            .filter(|kw| content.contains(&kw.to_lowercase()))
            .count();

        // Base relevance + keyword matches
        let keywords_len = self.config.news_keywords.len() as f32;
        (0.3 + (match_count as f32 / keywords_len) * 0.7).min(1.0)
    }

    /// Summarize API change.
    fn summarize_api_change(&self, change: &ApiChange) -> String {
        let mut parts = Vec::new();

        if !change.breaking_changes.is_empty() {
            parts.push(format!("{} breaking changes", change.breaking_changes.len()));
        }
        if !change.deprecations.is_empty() {
            parts.push(format!("{} deprecations", change.deprecations.len()));
        }
        if !change.new_features.is_empty() {
            parts.push(format!("{} new features", change.new_features.len()));
        }

        if parts.is_empty() {
            "Minor update".to_string()
        } else {
            parts.join(", ")
        }
    }

    /// Poll agent for macro data.
    async fn poll_agent_data(&mut self) -> Option<Observation> {
        // In production, this would call the Agent API
        // For now, return None as this requires the full agent infrastructure
        debug!(
            watcher_id = %self.state.id,
            "Agent polling not yet implemented in Rust"
        );
        None
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Public Methods for Manual Observation
    // ──────────────────────────────────────────────────────────────────────────

    /// Observe market data.
    pub fn observe_market_data(&mut self, data: Vec<MarketDataPoint>) {
        let start = Instant::now();

        for point in data {
            let sentiment = if point.change_percent > 1.0 {
                Some(Sentiment::Positive)
            } else if point.change_percent < -1.0 {
                Some(Sentiment::Negative)
            } else {
                Some(Sentiment::Neutral)
            };

            let relevance = (point.change_percent.abs() / 10.0).min(1.0) as f32;

            let mut obs = self.create_observation(
                WorldObservationType::MarketData,
                &point.symbol,
                WorldData {
                    title: Some(format!(
                        "{}: {}{}%",
                        point.symbol,
                        if point.change_percent >= 0.0 { "+" } else { "" },
                        point.change_percent
                    )),
                    summary: Some(format!("Price: {}, Change: {}", point.price, point.change)),
                    content: serde_json::to_value(&point).unwrap_or_default(),
                    source_url: None,
                    published_at: Some(point.timestamp),
                },
            );

            obs.sentiment = sentiment;
            obs.relevance = relevance;

            self.pending_observations.push(Observation::World(obs));
        }

        self.state
            .record_observation(start.elapsed().as_millis() as u64);
    }

    /// Observe a news item.
    pub fn observe_news(&mut self, item: NewsItem) {
        let start = Instant::now();

        let relevance = self.calculate_news_relevance(&item);
        if relevance < 0.1 {
            return; // Skip irrelevant news
        }

        let mut obs = self.create_observation(
            WorldObservationType::News,
            &item.source,
            WorldData {
                title: Some(item.title.clone()),
                summary: item.summary.clone(),
                content: serde_json::to_value(&item).unwrap_or_default(),
                source_url: Some(item.url.clone()),
                published_at: Some(item.published_at),
            },
        );

        obs.relevance = relevance;
        obs.sentiment = item.sentiment;

        self.pending_observations.push(Observation::World(obs));
        self.state
            .record_observation(start.elapsed().as_millis() as u64);
    }

    /// Observe an API change.
    pub fn observe_api_change(&mut self, change: ApiChange) {
        let start = Instant::now();

        let summary = self.summarize_api_change(&change);
        let has_breaking = !change.breaking_changes.is_empty();

        let mut obs = self.create_observation(
            WorldObservationType::ApiChange,
            &change.api,
            WorldData {
                title: Some(format!("{} updated to {}", change.api, change.version)),
                summary: Some(summary),
                content: serde_json::to_value(&change).unwrap_or_default(),
                source_url: None,
                published_at: None,
            },
        );

        obs.relevance = if has_breaking { 1.0 } else { 0.5 };
        obs.sentiment = if has_breaking {
            Some(Sentiment::Negative)
        } else {
            Some(Sentiment::Positive)
        };

        self.pending_observations.push(Observation::World(obs));
        self.state
            .record_observation(start.elapsed().as_millis() as u64);
    }

    /// Observe a security advisory.
    pub fn observe_security_advisory(&mut self, advisory: SecurityAdvisory) {
        let start = Instant::now();

        let mut obs = self.create_observation(
            WorldObservationType::SecurityAdvisory,
            &advisory.package,
            WorldData {
                title: Some(advisory.title.clone()),
                summary: advisory.description.clone(),
                content: serde_json::to_value(&advisory).unwrap_or_default(),
                source_url: None,
                published_at: None,
            },
        );

        obs.relevance = match advisory.severity {
            AdvisorySeverity::Critical => 1.0,
            AdvisorySeverity::High => 0.9,
            AdvisorySeverity::Medium => 0.7,
            AdvisorySeverity::Low => 0.5,
        };
        obs.sentiment = Some(Sentiment::Negative);
        obs.base.confidence = 1.0;
        obs.base.tags = vec![
            "security".to_string(),
            format!("{:?}", advisory.severity).to_lowercase(),
            advisory.package.clone(),
        ];
        if let Some(ref cve) = advisory.cve {
            obs.base.tags.push(cve.clone());
        }

        self.pending_observations.push(Observation::World(obs));
        self.state
            .record_observation(start.elapsed().as_millis() as u64);
    }

    /// Observe a dependency release.
    pub fn observe_dependency_release(&mut self, release: DependencyRelease) {
        let start = Instant::now();

        let mut obs = self.create_observation(
            WorldObservationType::DependencyRelease,
            &release.package,
            WorldData {
                title: Some(format!("{}@{} released", release.package, release.version)),
                summary: release.release_notes.as_ref().map(|s| s.chars().take(200).collect()),
                content: serde_json::to_value(&release).unwrap_or_default(),
                source_url: None,
                published_at: None,
            },
        );

        obs.relevance = if release.is_breaking { 0.9 } else { 0.5 };
        obs.sentiment = if release.is_breaking {
            Some(Sentiment::Negative)
        } else {
            Some(Sentiment::Positive)
        };
        obs.base.tags = vec![release.package.clone(), release.version.clone()];

        self.pending_observations.push(Observation::World(obs));
        self.state
            .record_observation(start.elapsed().as_millis() as u64);
    }

    /// Observe a trend.
    pub fn observe_trend(&mut self, trend: TrendInfo) {
        let start = Instant::now();

        let mut obs = self.create_observation(
            WorldObservationType::Trend,
            &trend.name,
            WorldData {
                title: Some(trend.name.clone()),
                summary: Some(trend.description.clone()),
                content: serde_json::to_value(&trend).unwrap_or_default(),
                source_url: None,
                published_at: None,
            },
        );

        obs.relevance = trend.strength;
        obs.sentiment = Some(match trend.direction {
            TrendDirection::Up => Sentiment::Positive,
            TrendDirection::Down => Sentiment::Negative,
            TrendDirection::Stable => Sentiment::Neutral,
        });

        self.pending_observations.push(Observation::World(obs));
        self.state
            .record_observation(start.elapsed().as_millis() as u64);
    }
}

#[async_trait]
impl Watcher for WorldWatch {
    fn id(&self) -> &str {
        &self.state.id
    }

    fn watcher_type(&self) -> WatcherType {
        WatcherType::World
    }

    fn is_running(&self) -> bool {
        self.state.running
    }

    async fn start(&mut self) -> anyhow::Result<()> {
        if self.state.running {
            warn!(watcher_id = %self.state.id, "WorldWatch already running");
            return Ok(());
        }

        self.state.running = true;

        info!(
            watcher_id = %self.state.id,
            source_count = %self.config.sources.len(),
            news_keywords = ?self.config.news_keywords,
            tracked_dependencies = %self.config.tracked_dependencies.len(),
            enable_agent_polling = %self.config.enable_agent_polling,
            "WorldWatch started"
        );

        Ok(())
    }

    async fn stop(&mut self) -> anyhow::Result<()> {
        self.state.running = false;
        info!(watcher_id = %self.state.id, "WorldWatch stopped");
        Ok(())
    }

    async fn observe(&mut self) -> Option<Observation> {
        if !self.state.running {
            return None;
        }

        // Increment observation cycle
        self.observation_cycle += 1;

        // Return any pending observations first
        if !self.pending_observations.is_empty() {
            return self.pending_observations.pop();
        }

        // Agent polling for macro data
        if self.config.enable_agent_polling
            && self.observation_cycle % self.config.agent_polling_cycles == 0
        {
            if let Some(obs) = self.poll_agent_data().await {
                return Some(obs);
            }
        }

        // TODO: Implement dependency update checking
        // For now, this is handled via manual observation methods

        None
    }

    fn get_status(&self) -> WatcherStatus {
        WatcherStatus {
            id: self.state.id.clone(),
            watcher_type: WatcherType::World,
            running: self.state.running,
            health: self.state.calculate_health(),
            last_observation: self.state.last_observation,
            observation_count: self.state.observation_count,
            error_count: self.state.error_count,
            avg_latency_ms: self.state.avg_latency(),
        }
    }

    fn get_metrics(&self) -> WatcherMetrics {
        self.state.get_metrics()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Tests
// ══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_watch_creation() {
        let config = WorldWatchConfig::default();
        let watch = WorldWatch::new(config);

        assert!(!watch.is_running());
        assert!(watch.state.id.starts_with("world_"));
    }

    #[test]
    fn test_world_watch_config_defaults() {
        let config = WorldWatchConfig::default();

        assert!(!config.enable_agent_polling);
        assert_eq!(config.agent_polling_cycles, 5);
        assert_eq!(config.options.interval_ms, 300000);
        assert!(config.sources.is_empty());
    }

    #[test]
    fn test_calculate_news_relevance() {
        let mut config = WorldWatchConfig::default();
        config.news_keywords = vec!["rust".to_string(), "security".to_string()];

        let watch = WorldWatch::new(config);

        let item = NewsItem {
            title: "New Rust Security Update".to_string(),
            summary: Some("Important security patch".to_string()),
            source: "tech-news".to_string(),
            url: "https://example.com".to_string(),
            published_at: Utc::now(),
            sentiment: None,
            relevance_score: None,
        };

        let relevance = watch.calculate_news_relevance(&item);
        assert!(relevance > 0.5); // Should match both keywords
    }

    #[test]
    fn test_summarize_api_change() {
        let config = WorldWatchConfig::default();
        let watch = WorldWatch::new(config);

        let change = ApiChange {
            api: "some-api".to_string(),
            version: "2.0.0".to_string(),
            breaking_changes: vec!["removed endpoint".to_string()],
            deprecations: vec![],
            new_features: vec!["new feature".to_string()],
        };

        let summary = watch.summarize_api_change(&change);
        assert!(summary.contains("1 breaking"));
        assert!(summary.contains("1 new features"));
    }

    #[test]
    fn test_market_data_observation() {
        let config = WorldWatchConfig::default();
        let mut watch = WorldWatch::new(config);
        watch.state.running = true;

        let data = vec![MarketDataPoint {
            symbol: "BTC".to_string(),
            price: 50000.0,
            change: 500.0,
            change_percent: 1.0,
            volume: Some(1000000),
            timestamp: Utc::now(),
        }];

        watch.observe_market_data(data);

        assert_eq!(watch.pending_observations.len(), 1);
        assert_eq!(watch.state.observation_count, 1);
    }

    #[test]
    fn test_security_advisory_observation() {
        let config = WorldWatchConfig::default();
        let mut watch = WorldWatch::new(config);
        watch.state.running = true;

        let advisory = SecurityAdvisory {
            id: "ADV-001".to_string(),
            severity: AdvisorySeverity::Critical,
            package: "some-package".to_string(),
            title: "Critical vulnerability".to_string(),
            description: Some("Remote code execution".to_string()),
            fixed_in: Some("1.2.3".to_string()),
            cve: Some("CVE-2024-0001".to_string()),
        };

        watch.observe_security_advisory(advisory);

        assert_eq!(watch.pending_observations.len(), 1);
    }
}
