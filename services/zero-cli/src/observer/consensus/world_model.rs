//! World Model Builder
//!
//! Creates and maintains the World Model - a convergent snapshot
//! of reality formed from multiple observation sources.
//!
//! Embodies "观察即收敛" (observation as convergence) - the act of
//! observing causes possibilities to collapse into a definite state.

use crate::observer::types::{
    BuildStatus, CodeObservation, CodeObservationType, CodeWorldState, ExternalWorldState,
    HealthStatus, MarketSentiment, MetaObservation, MetaObservationType, MetaState, Observation,
    ResourceUsage, SelfObservation, SelfObservationType, SelfState, SessionHealth, Severity,
    TechDebtLevel, WorldModel, WorldObservation, WorldObservationType,
};
use chrono::Utc;
use std::collections::VecDeque;
use tracing::debug;

/// World model builder configuration
#[derive(Debug, Clone)]
pub struct WorldModelConfig {
    /// Time window for snapshot (ms)
    pub window_ms: i64,
    /// Minimum observations for valid model
    pub min_observations: usize,
    /// Confidence threshold for inclusion
    pub min_confidence: f32,
    /// Maximum history size
    pub max_history: usize,
}

impl Default for WorldModelConfig {
    fn default() -> Self {
        Self {
            window_ms: 60_000, // 1 minute
            min_observations: 5,
            min_confidence: 0.3,
            max_history: 100,
        }
    }
}

/// World model builder
pub struct WorldModelBuilder {
    config: WorldModelConfig,
    current_model: Option<WorldModel>,
    model_history: VecDeque<WorldModel>,
}

impl WorldModelBuilder {
    /// Create a new world model builder
    pub fn new(config: WorldModelConfig) -> Self {
        Self {
            config,
            current_model: None,
            model_history: VecDeque::new(),
        }
    }

    /// Build a world model from observations
    pub fn build(&mut self, observations: &[Observation]) -> Option<WorldModel> {
        let now = Utc::now();
        let window_start = now - chrono::Duration::milliseconds(self.config.window_ms);

        // Filter to window and confidence
        let relevant: Vec<_> = observations
            .iter()
            .filter(|o| {
                o.timestamp() > window_start && o.confidence() >= self.config.min_confidence
            })
            .collect();

        if relevant.len() < self.config.min_observations {
            debug!(
                count = relevant.len(),
                required = self.config.min_observations,
                "Insufficient observations for world model"
            );
            return self.current_model.clone();
        }

        // Separate by watcher type
        let code_obs: Vec<_> = relevant
            .iter()
            .filter_map(|o| match o {
                Observation::Code(c) => Some(c),
                _ => None,
            })
            .collect();

        let world_obs: Vec<_> = relevant
            .iter()
            .filter_map(|o| match o {
                Observation::World(w) => Some(w),
                _ => None,
            })
            .collect();

        let self_obs: Vec<_> = relevant
            .iter()
            .filter_map(|o| match o {
                Observation::Self_(s) => Some(s),
                _ => None,
            })
            .collect();

        let meta_obs: Vec<_> = relevant
            .iter()
            .filter_map(|o| match o {
                Observation::Meta(m) => Some(m),
                _ => None,
            })
            .collect();

        // Build model sections
        let code = self.aggregate_code(&code_obs);
        let world = self.aggregate_world(&world_obs);
        let self_ = self.aggregate_self(&self_obs);
        let meta = self.aggregate_meta(&meta_obs);

        // Calculate overall confidence
        let confidence = self.calculate_confidence(
            &relevant,
            code_obs.len(),
            world_obs.len(),
            self_obs.len(),
            meta_obs.len(),
        );

        let model = WorldModel {
            id: crate::observer::types::generate_world_model_id(),
            timestamp: now,
            observation_ids: relevant.iter().map(|o| o.id().to_string()).collect(),
            code,
            world,
            self_,
            meta,
            confidence,
        };

        // Update history
        self.current_model = Some(model.clone());
        self.model_history.push_back(model.clone());
        if self.model_history.len() > self.config.max_history {
            self.model_history.pop_front();
        }

        debug!(
            id = %model.id,
            observations = relevant.len(),
            confidence = format!("{:.2}", model.confidence),
            "World model built"
        );

        Some(model)
    }

    /// Get current world model
    pub fn get_current(&self) -> Option<&WorldModel> {
        self.current_model.as_ref()
    }

    /// Get model history
    pub fn get_history(&self, limit: Option<usize>) -> Vec<&WorldModel> {
        let limit = limit.unwrap_or(20);
        self.model_history.iter().rev().take(limit).collect()
    }

    /// Clear all state
    pub fn clear(&mut self) {
        self.current_model = None;
        self.model_history.clear();
    }

    // ══════════════════════════════════════════════════════════════════════════════
    // Aggregation Methods
    // ══════════════════════════════════════════════════════════════════════════════

    fn aggregate_code(&self, observations: &[&CodeObservation]) -> CodeWorldState {
        // Find most recent git change
        let git_changes: Vec<_> = observations
            .iter()
            .filter(|o| o.observation_type == CodeObservationType::GitChange)
            .collect();

        let last_commit = git_changes.last().and_then(|o| {
            o.change
                .after
                .as_ref()
                .and_then(|v| v.as_str().map(String::from))
        });

        // Determine build status
        let build_obs: Vec<_> = observations
            .iter()
            .filter(|o| o.observation_type == CodeObservationType::BuildStatus)
            .collect();

        let build_status = build_obs.last().map_or(BuildStatus::Unknown, |o| {
            o.change
                .after
                .as_ref()
                .and_then(|v| v.get("status"))
                .and_then(|s| s.as_str())
                .map(|s| match s {
                    "passing" => BuildStatus::Passing,
                    "failing" => BuildStatus::Failing,
                    _ => BuildStatus::Unknown,
                })
                .unwrap_or(BuildStatus::Unknown)
        });

        // Get test coverage
        let test_obs: Vec<_> = observations
            .iter()
            .filter(|o| o.observation_type == CodeObservationType::TestCoverage)
            .collect();

        let test_coverage = test_obs.last().and_then(|o| {
            o.change
                .after
                .as_ref()
                .and_then(|v| v.get("coverage"))
                .and_then(|c| c.as_f64())
                .map(|c| c as f32)
        });

        // Assess tech debt
        let debt_obs: Vec<_> = observations
            .iter()
            .filter(|o| o.observation_type == CodeObservationType::TechDebt)
            .collect();

        let tech_debt_level = if !debt_obs.is_empty() {
            let avg_severity: f32 = debt_obs
                .iter()
                .map(|o| match o.impact.severity {
                    Severity::Critical => 4.0,
                    Severity::High => 3.0,
                    Severity::Medium => 2.0,
                    Severity::Low => 1.0,
                })
                .sum::<f32>()
                / debt_obs.len() as f32;

            Some(if avg_severity > 2.5 {
                TechDebtLevel::High
            } else if avg_severity > 1.5 {
                TechDebtLevel::Medium
            } else {
                TechDebtLevel::Low
            })
        } else {
            None
        };

        CodeWorldState {
            last_commit,
            build_status,
            test_coverage,
            tech_debt_level,
            recent_changes: observations.len(),
        }
    }

    fn aggregate_world(&self, observations: &[&WorldObservation]) -> ExternalWorldState {
        // Determine market sentiment
        let market_obs: Vec<_> = observations
            .iter()
            .filter(|o| o.observation_type == WorldObservationType::MarketData)
            .collect();

        let market_sentiment = if !market_obs.is_empty() {
            let sentiments: Vec<_> = market_obs.iter().filter_map(|o| o.sentiment).collect();

            let positive = sentiments
                .iter()
                .filter(|s| matches!(s, crate::observer::types::Sentiment::Positive))
                .count();
            let negative = sentiments
                .iter()
                .filter(|s| matches!(s, crate::observer::types::Sentiment::Negative))
                .count();

            Some(if positive as f32 > negative as f32 * 1.5 {
                MarketSentiment::Bullish
            } else if negative as f32 > positive as f32 * 1.5 {
                MarketSentiment::Bearish
            } else {
                MarketSentiment::Neutral
            })
        } else {
            None
        };

        // Collect relevant news
        let news_obs: Vec<_> = observations
            .iter()
            .filter(|o| o.observation_type == WorldObservationType::News && o.relevance > 0.5)
            .collect();

        let mut relevant_news: Vec<_> = news_obs
            .iter()
            .filter_map(|o| o.data.title.clone().or_else(|| o.data.summary.clone()))
            .take(5)
            .collect();
        relevant_news.truncate(5);

        // Identify external risks
        let mut external_risks = Vec::new();

        let security_obs: Vec<_> = observations
            .iter()
            .filter(|o| o.observation_type == WorldObservationType::SecurityAdvisory)
            .collect();
        if !security_obs.is_empty() {
            external_risks.push(format!("{} security advisory(ies)", security_obs.len()));
        }

        let api_changes: Vec<_> = observations
            .iter()
            .filter(|o| {
                o.observation_type == WorldObservationType::ApiChange
                    && o.sentiment == Some(crate::observer::types::Sentiment::Negative)
            })
            .collect();
        if !api_changes.is_empty() {
            external_risks.push(format!("{} breaking API change(s)", api_changes.len()));
        }

        // Identify opportunities
        let mut opportunities = Vec::new();

        let dep_releases: Vec<_> = observations
            .iter()
            .filter(|o| {
                o.observation_type == WorldObservationType::DependencyRelease
                    && o.sentiment == Some(crate::observer::types::Sentiment::Positive)
            })
            .collect();
        if !dep_releases.is_empty() {
            opportunities.push(format!(
                "{} dependency update(s) available",
                dep_releases.len()
            ));
        }

        let trends: Vec<_> = observations
            .iter()
            .filter(|o| {
                o.observation_type == WorldObservationType::Trend
                    && o.sentiment == Some(crate::observer::types::Sentiment::Positive)
            })
            .collect();
        for trend in trends {
            if let Some(title) = &trend.data.title {
                opportunities.push(title.clone());
            }
        }

        ExternalWorldState {
            market_sentiment,
            relevant_news,
            external_risks,
            opportunities,
        }
    }

    fn aggregate_self(&self, observations: &[&SelfObservation]) -> SelfState {
        // Find current agent
        let agent_obs: Vec<_> = observations
            .iter()
            .filter(|o| o.observation_type == SelfObservationType::AgentBehavior)
            .collect();

        let current_agent = agent_obs.last().map(|o| o.agent_id.clone());

        // Aggregate resource usage
        let resource_obs: Vec<_> = observations
            .iter()
            .filter(|o| o.observation_type == SelfObservationType::ResourceUsage)
            .collect();

        let mut resource_usage = ResourceUsage::default();
        for obs in &resource_obs {
            if let Some(input) = &obs.observation.input {
                if let Some(tokens) = input.get("tokens").and_then(|v| v.as_u64()) {
                    resource_usage.tokens += tokens;
                }
                if let Some(cost) = input.get("cost").and_then(|v| v.as_f64()) {
                    resource_usage.cost += cost;
                }
                if let Some(duration) = input.get("duration").and_then(|v| v.as_f64()) {
                    resource_usage.duration += duration;
                }
            }
        }

        // Count recent errors
        let recent_errors = observations
            .iter()
            .filter(|o| !o.observation.success)
            .count();

        // Calculate decision quality
        let decision_obs: Vec<_> = observations
            .iter()
            .filter(|o| o.observation_type == SelfObservationType::DecisionLog)
            .collect();

        let decision_quality = if !decision_obs.is_empty() {
            let scores: Vec<f32> = decision_obs
                .iter()
                .filter_map(|o| o.quality.close_score)
                .collect();

            if !scores.is_empty() {
                Some(scores.iter().sum::<f32>() / scores.len() as f32)
            } else {
                None
            }
        } else {
            None
        };

        // Determine session health
        let session_health = if recent_errors > 5 {
            SessionHealth::Critical
        } else if recent_errors > 2 {
            SessionHealth::Degraded
        } else {
            SessionHealth::Healthy
        };

        SelfState {
            current_agent,
            session_health,
            resource_usage,
            recent_errors,
            decision_quality,
        }
    }

    fn aggregate_meta(&self, observations: &[&MetaObservation]) -> MetaState {
        // Determine observer health
        let health_obs: Vec<_> = observations
            .iter()
            .filter(|o| o.observation_type == MetaObservationType::SystemHealth)
            .collect();

        let observer_health = health_obs
            .last()
            .map(|o| o.assessment.health)
            .unwrap_or(HealthStatus::Healthy);

        // Collect coverage gaps
        let mut coverage_gaps = Vec::new();
        let gap_obs: Vec<_> = observations
            .iter()
            .filter(|o| o.observation_type == MetaObservationType::CoverageGap)
            .collect();

        for obs in gap_obs {
            for issue in &obs.issues {
                if issue.issue_type == "coverage_gap" {
                    coverage_gaps.push(issue.description.clone());
                }
            }
        }

        // Calculate consensus strength
        let consensus_strength = if observations.is_empty() {
            0.5
        } else {
            observations.iter().map(|o| o.base.confidence).sum::<f32>() / observations.len() as f32
        };

        MetaState {
            observer_health,
            coverage_gaps,
            consensus_strength,
        }
    }

    fn calculate_confidence(
        &self,
        observations: &[&Observation],
        code_count: usize,
        world_count: usize,
        self_count: usize,
        meta_count: usize,
    ) -> f32 {
        if observations.is_empty() {
            return 0.0;
        }

        // Base confidence from observation count
        let count_factor = (observations.len() as f32 / 20.0).min(1.0);

        // Coverage factor (how many watcher types have observations)
        let coverage = [code_count, world_count, self_count, meta_count]
            .iter()
            .filter(|&&c| c > 0)
            .count() as f32
            / 4.0;

        // Average observation confidence
        let avg_confidence =
            observations.iter().map(|o| o.confidence()).sum::<f32>() / observations.len() as f32;

        // Combined confidence
        count_factor * 0.3 + coverage * 0.3 + avg_confidence * 0.4
    }
}

impl Default for WorldModelBuilder {
    fn default() -> Self {
        Self::new(WorldModelConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_world_model_builder_creation() {
        let builder = WorldModelBuilder::new(WorldModelConfig::default());
        assert!(builder.get_current().is_none());
    }

    #[test]
    fn test_empty_observations() {
        let mut builder = WorldModelBuilder::new(WorldModelConfig::default());
        let result = builder.build(&[]);
        assert!(result.is_none());
    }

    #[test]
    fn test_insufficient_observations() {
        let mut builder = WorldModelBuilder::new(WorldModelConfig {
            min_observations: 5,
            ..Default::default()
        });

        // Only 2 observations - less than minimum
        let obs = vec![
            Observation::Code(CodeObservation::new(
                "watcher",
                CodeObservationType::GitChange,
                "file.rs",
            )),
            Observation::Code(CodeObservation::new(
                "watcher",
                CodeObservationType::BuildStatus,
                "project",
            )),
        ];

        let result = builder.build(&obs);
        assert!(result.is_none());
    }
}
