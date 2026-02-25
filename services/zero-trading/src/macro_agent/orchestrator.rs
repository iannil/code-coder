//! Macro orchestrator for hybrid decision making.
//!
//! Coordinates between the fast rule engine and the LLM-powered macro agent
//! to provide intelligent macro-economic trading guidance.

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use super::bridge::{AgentBridge, AgentBridgeConfig};
use super::types::{
    AnalysisTrigger, DecisionSource, MacroContext, MacroDecision,
};
use crate::macro_filter::{MacroEnvironment, MacroFilter, TradingBias};

/// Configuration for the macro orchestrator.
#[derive(Debug, Clone)]
pub struct OrchestratorConfig {
    /// Enable agent analysis for anomalies
    pub agent_enabled: bool,
    /// Risk appetite thresholds for triggering agent
    pub extreme_risk_low: f64,
    pub extreme_risk_high: f64,
    /// Position multiplier threshold for agent trigger
    pub position_reduction_threshold: f64,
    /// PMI thresholds for agent trigger
    pub pmi_low_threshold: f64,
    pub pmi_high_threshold: f64,
    /// Minimum confidence to trust agent analysis
    pub min_agent_confidence: f64,
    /// Cache duration for agent analysis (seconds)
    pub agent_cache_secs: u64,
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            agent_enabled: true,
            extreme_risk_low: 30.0,
            extreme_risk_high: 70.0,
            position_reduction_threshold: 0.5,
            pmi_low_threshold: 48.0,
            pmi_high_threshold: 54.0,
            min_agent_confidence: 0.6,
            agent_cache_secs: 3600,
        }
    }
}

/// Cached agent analysis result.
struct CachedAnalysis {
    decision: MacroDecision,
    timestamp: std::time::Instant,
}

/// Orchestrator that coordinates rule engine and agent analysis.
pub struct MacroOrchestrator {
    /// Rule engine for fast decisions
    rule_engine: Arc<MacroFilter>,
    /// Agent bridge for deep analysis
    agent_bridge: AgentBridge,
    /// Configuration
    config: OrchestratorConfig,
    /// Cached agent analysis
    cache: RwLock<Option<CachedAnalysis>>,
}

impl MacroOrchestrator {
    /// Create a new orchestrator.
    pub fn new(
        rule_engine: Arc<MacroFilter>,
        agent_config: AgentBridgeConfig,
        orchestrator_config: OrchestratorConfig,
    ) -> Self {
        let agent_bridge = AgentBridge::new(agent_config);

        Self {
            rule_engine,
            agent_bridge,
            config: orchestrator_config,
            cache: RwLock::new(None),
        }
    }

    /// Get macro environment evaluation using hybrid mode.
    ///
    /// This first runs the rule engine for a fast decision, then checks if
    /// anomaly conditions warrant a deeper agent analysis.
    pub async fn evaluate(&self) -> Result<MacroDecision> {
        // Step 1: Get rule engine result
        let rule_result = self.rule_engine.get_environment().await?;

        // Step 2: Check if we need agent analysis
        let triggers = self.check_triggers(&rule_result);

        if triggers.is_empty() || !self.config.agent_enabled {
            debug!(
                "Using rule engine result (no triggers or agent disabled)"
            );
            return Ok(self.rule_to_decision(rule_result, DecisionSource::RuleEngine));
        }

        // Step 3: Check cache
        if let Some(cached) = self.get_cached_decision().await {
            debug!("Using cached agent decision");
            return Ok(cached);
        }

        // Step 4: Request agent analysis
        info!(
            triggers = ?triggers,
            "Triggering agent analysis for anomaly conditions"
        );

        let context = self.build_context(&rule_result);

        match self.agent_bridge.analyze(&context).await {
            Ok(agent_result) => {
                info!(
                    confidence = agent_result.confidence,
                    cycle_phase = ?agent_result.cycle_phase,
                    "Agent analysis completed"
                );

                // Only trust agent if confidence is high enough
                if agent_result.confidence >= self.config.min_agent_confidence {
                    let merged = self.merge_decisions(rule_result, agent_result, triggers);
                    self.cache_decision(&merged).await;
                    Ok(merged)
                } else {
                    warn!(
                        confidence = agent_result.confidence,
                        threshold = self.config.min_agent_confidence,
                        "Agent confidence too low, using rule engine result"
                    );
                    Ok(self.rule_to_decision(rule_result, DecisionSource::RuleEngine))
                }
            }
            Err(e) => {
                warn!(
                    error = %e,
                    "Agent analysis failed, falling back to rule engine"
                );
                Ok(self.rule_to_decision(rule_result, DecisionSource::Fallback))
            }
        }
    }

    /// Check if the rule engine result triggers agent analysis.
    fn check_triggers(&self, result: &MacroEnvironment) -> Vec<AnalysisTrigger> {
        let mut triggers = Vec::new();

        // Extreme risk appetite
        if result.risk_appetite < self.config.extreme_risk_low
            || result.risk_appetite > self.config.extreme_risk_high
        {
            triggers.push(AnalysisTrigger::ExtremeRiskAppetite);
        }

        // Avoid trading signal
        if result.trading_bias == TradingBias::AvoidTrading {
            triggers.push(AnalysisTrigger::AvoidTradingSignal);
        }

        // Significant position reduction
        if result.position_multiplier < self.config.position_reduction_threshold {
            triggers.push(AnalysisTrigger::SignificantPositionReduction);
        }

        // Extreme PMI
        if let Some(pmi) = result.pmi {
            if pmi < self.config.pmi_low_threshold || pmi > self.config.pmi_high_threshold {
                triggers.push(AnalysisTrigger::ExtremePmi);
            }
        }

        // Indicator divergence (simple heuristic)
        if self.check_indicator_divergence(result) {
            triggers.push(AnalysisTrigger::IndicatorDivergence);
        }

        triggers
    }

    /// Check for indicator divergence (multiple indicators pointing different directions).
    fn check_indicator_divergence(&self, result: &MacroEnvironment) -> bool {
        let mut bullish_signals = 0;
        let mut bearish_signals = 0;

        // PMI signal
        if let Some(pmi) = result.pmi {
            if pmi > 51.0 {
                bullish_signals += 1;
            } else if pmi < 49.0 {
                bearish_signals += 1;
            }
        }

        // M2 growth signal
        if let Some(m2) = result.m2_growth {
            if m2 > 10.0 {
                bullish_signals += 1;
            } else if m2 < 8.0 {
                bearish_signals += 1;
            }
        }

        // Divergence detected if we have both bullish and bearish signals
        bullish_signals > 0 && bearish_signals > 0
    }

    /// Build macro context from rule engine result.
    fn build_context(&self, result: &MacroEnvironment) -> MacroContext {
        MacroContext {
            pmi: result.pmi,
            m2_growth: result.m2_growth,
            social_financing: result.social_financing,
            risk_appetite: result.risk_appetite,
            position_multiplier: result.position_multiplier,
            trading_bias: result.trading_bias,
            notes: result.notes.clone(),
        }
    }

    /// Convert rule engine result to macro decision.
    fn rule_to_decision(
        &self,
        result: MacroEnvironment,
        source: DecisionSource,
    ) -> MacroDecision {
        MacroDecision {
            source,
            cycle_phase: result.cycle_phase,
            position_multiplier: result.position_multiplier,
            trading_bias: result.trading_bias,
            risk_appetite: result.risk_appetite,
            risk_warnings: Vec::new(),
            summary: result.notes,
            confidence: match source {
                DecisionSource::RuleEngine => 0.7,
                DecisionSource::Fallback => 0.5,
                _ => 0.7,
            },
        }
    }

    /// Merge rule engine and agent analysis results.
    fn merge_decisions(
        &self,
        rule: MacroEnvironment,
        agent: super::types::AgentAnalysis,
        triggers: Vec<AnalysisTrigger>,
    ) -> MacroDecision {
        // Use agent's cycle phase assessment
        let cycle_phase = agent.cycle_phase;

        // Weighted average of position advice (agent gets more weight)
        let position_multiplier = (rule.position_multiplier * 0.3 + agent.position_advice * 0.7)
            .clamp(0.3, 1.5);

        // Use more conservative trading bias
        let trading_bias = self.more_conservative_bias(rule.trading_bias, agent.trading_bias);

        // Combine risk warnings
        let mut risk_warnings = agent.risk_warnings;

        // Add trigger-based warnings
        for trigger in &triggers {
            risk_warnings.push(format!("触发条件: {}", trigger));
        }

        // Build summary
        let summary = format!(
            "{}判断: {:?} | 仓位建议: {:.0}% | 方向: {:?}\n分析: {}",
            if triggers.is_empty() { "规则引擎" } else { "智能体分析" },
            cycle_phase,
            position_multiplier * 100.0,
            trading_bias,
            agent.reasoning
        );

        MacroDecision {
            source: DecisionSource::Merged,
            cycle_phase,
            position_multiplier,
            trading_bias,
            risk_appetite: rule.risk_appetite,
            risk_warnings,
            summary,
            confidence: agent.confidence,
        }
    }

    /// Return the more conservative of two trading biases.
    fn more_conservative_bias(&self, a: TradingBias, b: TradingBias) -> TradingBias {
        // Order from most to least conservative:
        // AvoidTrading > Bearish > Neutral > Bullish
        let score = |bias: TradingBias| match bias {
            TradingBias::AvoidTrading => 0,
            TradingBias::Bearish => 1,
            TradingBias::Neutral => 2,
            TradingBias::Bullish => 3,
        };

        if score(a) <= score(b) {
            a
        } else {
            b
        }
    }

    /// Get cached decision if still valid.
    async fn get_cached_decision(&self) -> Option<MacroDecision> {
        let cache = self.cache.read().await;
        if let Some(ref cached) = *cache {
            if cached.timestamp.elapsed().as_secs() < self.config.agent_cache_secs {
                return Some(cached.decision.clone());
            }
        }
        None
    }

    /// Cache a decision.
    async fn cache_decision(&self, decision: &MacroDecision) {
        let mut cache = self.cache.write().await;
        *cache = Some(CachedAnalysis {
            decision: decision.clone(),
            timestamp: std::time::Instant::now(),
        });
    }

    /// Force a fresh agent analysis, bypassing cache and triggers.
    pub async fn force_analyze(&self) -> Result<MacroDecision> {
        let rule_result = self.rule_engine.get_environment().await?;
        let context = self.build_context(&rule_result);

        let agent_result = self.agent_bridge.analyze(&context).await?;
        let merged = self.merge_decisions(
            rule_result,
            agent_result,
            vec![AnalysisTrigger::ManualTrigger],
        );

        self.cache_decision(&merged).await;
        Ok(merged)
    }

    /// Check if trading is recommended based on current macro conditions.
    pub async fn is_trading_recommended(&self) -> bool {
        match self.evaluate().await {
            Ok(decision) => decision.trading_bias != TradingBias::AvoidTrading,
            Err(_) => {
                // Fallback to rule engine if orchestrator fails
                self.rule_engine.is_trading_recommended().await
            }
        }
    }

    /// Get adjusted position size based on current macro conditions.
    pub async fn adjust_position_size(&self, base_size: f64) -> f64 {
        match self.evaluate().await {
            Ok(decision) => base_size * decision.position_multiplier,
            Err(_) => {
                // Fallback to rule engine
                self.rule_engine.adjust_position_size(base_size).await
            }
        }
    }

    /// Check if the agent bridge is available.
    pub async fn is_agent_available(&self) -> bool {
        self.agent_bridge.health_check().await
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::macro_filter::EconomicCyclePhase;

    #[test]
    fn test_orchestrator_config_default() {
        let config = OrchestratorConfig::default();
        assert!(config.agent_enabled);
        assert!((config.extreme_risk_low - 30.0).abs() < 0.001);
        assert!((config.extreme_risk_high - 70.0).abs() < 0.001);
    }

    #[test]
    fn test_more_conservative_bias() {
        let rule_engine = Arc::new(MacroFilter::new(&zero_common::config::Config::default()));
        let orchestrator = MacroOrchestrator::new(
            rule_engine,
            AgentBridgeConfig::default(),
            OrchestratorConfig::default(),
        );

        // AvoidTrading is most conservative
        assert_eq!(
            orchestrator.more_conservative_bias(TradingBias::Bullish, TradingBias::AvoidTrading),
            TradingBias::AvoidTrading
        );

        // Bearish is more conservative than Bullish
        assert_eq!(
            orchestrator.more_conservative_bias(TradingBias::Bullish, TradingBias::Bearish),
            TradingBias::Bearish
        );

        // Neutral is more conservative than Bullish
        assert_eq!(
            orchestrator.more_conservative_bias(TradingBias::Bullish, TradingBias::Neutral),
            TradingBias::Neutral
        );
    }

    #[test]
    fn test_check_triggers_extreme_risk() {
        let rule_engine = Arc::new(MacroFilter::new(&zero_common::config::Config::default()));
        let orchestrator = MacroOrchestrator::new(
            rule_engine,
            AgentBridgeConfig::default(),
            OrchestratorConfig::default(),
        );

        // Low risk appetite should trigger
        let result = MacroEnvironment {
            cycle_phase: EconomicCyclePhase::Contraction,
            m2_growth: Some(8.0),
            social_financing: None,
            risk_appetite: 25.0, // Below threshold
            pmi: Some(48.5),
            position_multiplier: 0.6,
            trading_bias: TradingBias::Bearish,
            notes: String::new(),
        };

        let triggers = orchestrator.check_triggers(&result);
        assert!(triggers.contains(&AnalysisTrigger::ExtremeRiskAppetite));
    }

    #[test]
    fn test_check_triggers_avoid_trading() {
        let rule_engine = Arc::new(MacroFilter::new(&zero_common::config::Config::default()));
        let orchestrator = MacroOrchestrator::new(
            rule_engine,
            AgentBridgeConfig::default(),
            OrchestratorConfig::default(),
        );

        let result = MacroEnvironment {
            cycle_phase: EconomicCyclePhase::Contraction,
            m2_growth: Some(8.0),
            social_financing: None,
            risk_appetite: 50.0,
            pmi: Some(50.0),
            position_multiplier: 0.6,
            trading_bias: TradingBias::AvoidTrading,
            notes: String::new(),
        };

        let triggers = orchestrator.check_triggers(&result);
        assert!(triggers.contains(&AnalysisTrigger::AvoidTradingSignal));
    }

    #[test]
    fn test_check_triggers_position_reduction() {
        let rule_engine = Arc::new(MacroFilter::new(&zero_common::config::Config::default()));
        let orchestrator = MacroOrchestrator::new(
            rule_engine,
            AgentBridgeConfig::default(),
            OrchestratorConfig::default(),
        );

        let result = MacroEnvironment {
            cycle_phase: EconomicCyclePhase::Slowdown,
            m2_growth: Some(8.0),
            social_financing: None,
            risk_appetite: 50.0,
            pmi: Some(50.0),
            position_multiplier: 0.4, // Below threshold
            trading_bias: TradingBias::Neutral,
            notes: String::new(),
        };

        let triggers = orchestrator.check_triggers(&result);
        assert!(triggers.contains(&AnalysisTrigger::SignificantPositionReduction));
    }

    #[test]
    fn test_check_triggers_extreme_pmi() {
        let rule_engine = Arc::new(MacroFilter::new(&zero_common::config::Config::default()));
        let orchestrator = MacroOrchestrator::new(
            rule_engine,
            AgentBridgeConfig::default(),
            OrchestratorConfig::default(),
        );

        // Low PMI
        let result = MacroEnvironment {
            cycle_phase: EconomicCyclePhase::Contraction,
            m2_growth: Some(8.0),
            social_financing: None,
            risk_appetite: 50.0,
            pmi: Some(46.0), // Below threshold
            position_multiplier: 0.7,
            trading_bias: TradingBias::Neutral,
            notes: String::new(),
        };

        let triggers = orchestrator.check_triggers(&result);
        assert!(triggers.contains(&AnalysisTrigger::ExtremePmi));

        // High PMI
        let result_high = MacroEnvironment {
            pmi: Some(55.0), // Above threshold
            ..result
        };

        let triggers_high = orchestrator.check_triggers(&result_high);
        assert!(triggers_high.contains(&AnalysisTrigger::ExtremePmi));
    }

    #[test]
    fn test_indicator_divergence() {
        let rule_engine = Arc::new(MacroFilter::new(&zero_common::config::Config::default()));
        let orchestrator = MacroOrchestrator::new(
            rule_engine,
            AgentBridgeConfig::default(),
            OrchestratorConfig::default(),
        );

        // PMI bullish, M2 bearish = divergence
        let result = MacroEnvironment {
            cycle_phase: EconomicCyclePhase::EarlyRecovery,
            m2_growth: Some(7.5), // Bearish (tight liquidity)
            social_financing: None,
            risk_appetite: 50.0,
            pmi: Some(52.0), // Bullish
            position_multiplier: 1.0,
            trading_bias: TradingBias::Neutral,
            notes: String::new(),
        };

        assert!(orchestrator.check_indicator_divergence(&result));

        // Both bullish = no divergence
        let result_aligned = MacroEnvironment {
            m2_growth: Some(11.0), // Bullish
            pmi: Some(52.0), // Bullish
            ..result
        };

        assert!(!orchestrator.check_indicator_divergence(&result_aligned));
    }

    #[test]
    fn test_rule_to_decision() {
        let rule_engine = Arc::new(MacroFilter::new(&zero_common::config::Config::default()));
        let orchestrator = MacroOrchestrator::new(
            rule_engine,
            AgentBridgeConfig::default(),
            OrchestratorConfig::default(),
        );

        let env = MacroEnvironment {
            cycle_phase: EconomicCyclePhase::Expansion,
            m2_growth: Some(10.0),
            social_financing: Some(3.5),
            risk_appetite: 60.0,
            pmi: Some(52.0),
            position_multiplier: 1.2,
            trading_bias: TradingBias::Bullish,
            notes: "测试".to_string(),
        };

        let decision = orchestrator.rule_to_decision(env, DecisionSource::RuleEngine);
        assert_eq!(decision.source, DecisionSource::RuleEngine);
        assert_eq!(decision.cycle_phase, EconomicCyclePhase::Expansion);
        assert!((decision.position_multiplier - 1.2).abs() < 0.001);
    }
}
