//! Autonomous Bridge for Hands integration with ccode Autonomous Orchestrator.
//!
//! This module provides an HTTP client for calling the CodeCoder autonomous API,
//! enabling Hands to use CLOSE decision framework and autonomous execution.
//!
//! # Architecture
//!
//! ```text
//! Hands (Rust) → AutonomousBridge → ccode API (TypeScript) → Orchestrator
//! ```
//!
//! # Example
//!
//! ```no_run
//! use zero_workflow::hands::autonomous_bridge::{AutonomousBridge, AutonomousRequest, AutonomyLevel, ResourceBudget};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let bridge = AutonomousBridge::new("http://127.0.0.1:4400".to_string());
//!
//! let request = AutonomousRequest {
//!     request: "Analyze market conditions".to_string(),
//!     config: AutonomousConfig {
//!         autonomy_level: AutonomyLevel::Crazy,
//!         unattended: true,
//!         resource_budget: ResourceBudget {
//!             max_tokens: 100000,
//!             max_cost_usd: 5.0,
//!             max_duration_sec: 600,
//!         },
//!         enable_evolution_loop: true,
//!         enable_web_search: false,
//!         max_iterations: Some(5),
//!     },
//!     context: None,
//!     session_id: None,
//! };
//!
//! let result = bridge.execute(request).await?;
//! println!("Result: {}", result.output);
//! # Ok(())
//! # }
//! ```

use super::manifest::{AutonomyLevel, HandManifest, RiskThreshold};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Autonomy level matching TypeScript API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BridgeAutonomyLevel {
    Lunatic,
    Insane,
    Crazy,
    Wild,
    Bold,
    Timid,
}

impl From<AutonomyLevel> for BridgeAutonomyLevel {
    fn from(level: AutonomyLevel) -> Self {
        match level {
            AutonomyLevel::Lunatic => BridgeAutonomyLevel::Lunatic,
            AutonomyLevel::Insane => BridgeAutonomyLevel::Insane,
            AutonomyLevel::Crazy => BridgeAutonomyLevel::Crazy,
            AutonomyLevel::Wild => BridgeAutonomyLevel::Wild,
            AutonomyLevel::Bold => BridgeAutonomyLevel::Bold,
            AutonomyLevel::Timid => BridgeAutonomyLevel::Timid,
        }
    }
}

/// Risk threshold for auto-approval (matches TypeScript RiskLevel).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BridgeRiskThreshold {
    Safe,
    Low,
    Medium,
    High,
}

impl From<RiskThreshold> for BridgeRiskThreshold {
    fn from(threshold: RiskThreshold) -> Self {
        match threshold {
            RiskThreshold::Safe => BridgeRiskThreshold::Safe,
            RiskThreshold::Low => BridgeRiskThreshold::Low,
            RiskThreshold::Medium => BridgeRiskThreshold::Medium,
            RiskThreshold::High => BridgeRiskThreshold::High,
        }
    }
}

/// Auto-approve configuration for autonomous tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoApproveConfig {
    /// Enable auto-approval
    pub enabled: bool,

    /// Tools allowed for auto-approval (whitelist)
    #[serde(rename = "allowedTools")]
    pub allowed_tools: Vec<String>,

    /// Maximum risk level for auto-approval
    #[serde(rename = "riskThreshold")]
    pub risk_threshold: BridgeRiskThreshold,

    /// Timeout in milliseconds before auto-approving
    #[serde(rename = "timeoutMs")]
    pub timeout_ms: u64,

    /// Whether running in unattended mode
    pub unattended: bool,
}

impl Default for AutoApproveConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_tools: Vec::new(),
            risk_threshold: BridgeRiskThreshold::Medium,
            timeout_ms: 30000,
            unattended: true,
        }
    }
}

/// Resource budget for autonomous execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceBudget {
    /// Maximum tokens to consume
    pub max_tokens: usize,

    /// Maximum cost in USD
    pub max_cost_usd: f64,

    /// Maximum duration in seconds
    pub max_duration_sec: usize,
}

impl Default for ResourceBudget {
    fn default() -> Self {
        Self {
            max_tokens: 100000,
            max_cost_usd: 5.0,
            max_duration_sec: 600,
        }
    }
}

/// Autonomous execution configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousConfig {
    /// Autonomy level (affects CLOSE thresholds)
    #[serde(rename = "autonomyLevel")]
    pub autonomy_level: BridgeAutonomyLevel,

    /// Enable unattended mode (no human interaction)
    pub unattended: bool,

    /// Resource budget
    #[serde(rename = "resourceBudget")]
    pub resource_budget: ResourceBudget,

    /// Enable evolution loop for problem solving
    #[serde(rename = "enableEvolutionLoop", default)]
    pub enable_evolution_loop: bool,

    /// Enable web search for solutions
    #[serde(rename = "enableWebSearch", default)]
    pub enable_web_search: bool,

    /// Maximum iterations
    #[serde(rename = "maxIterations")]
    pub max_iterations: Option<usize>,
}

/// Context from Hands system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HandsContext {
    /// Hand ID invoking autonomous
    #[serde(rename = "handId")]
    pub hand_id: String,

    /// Hand name
    #[serde(rename = "handName")]
    pub hand_name: Option<String>,

    /// Previous execution results for context
    #[serde(rename = "previousResults", default)]
    pub previous_results: Vec<PreviousResult>,

    /// Custom state from previous runs
    #[serde(default)]
    pub custom_state: serde_json::Value,
}

/// Previous execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreviousResult {
    /// Timestamp of execution
    pub timestamp: String,

    /// Output content
    pub output: String,

    /// Whether execution succeeded
    pub success: bool,
}

/// CLOSE decision score result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CLOSEScoreResult {
    /// Convergence: How focused the decision is (0-10)
    pub convergence: f32,

    /// Leverage: Impact vs effort ratio (0-10)
    pub leverage: f32,

    /// Optionality: Flexibility and reversibility (0-10)
    pub optionality: f32,

    /// Surplus: Resource availability (0-10)
    pub surplus: f32,

    /// Evolution: Learning value (0-10)
    pub evolution: f32,

    /// Total weighted score (0-10)
    pub total: f32,
}

/// Autonomous execution result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousExecutionResult {
    /// Whether execution completed successfully
    pub success: bool,

    /// Output content
    pub output: String,

    /// Quality score (0-100)
    #[serde(rename = "qualityScore")]
    pub quality_score: f64,

    /// Craziness score (0-100)
    #[serde(rename = "crazinessScore")]
    pub craziness_score: f64,

    /// Duration in milliseconds
    pub duration: u64,

    /// Tokens used
    #[serde(rename = "tokensUsed")]
    pub tokens_used: usize,

    /// Cost in USD
    #[serde(rename = "costUSD")]
    pub cost_usd: f64,

    /// Iterations completed
    #[serde(rename = "iterationsCompleted")]
    pub iterations_completed: usize,

    /// CLOSE decision scores
    #[serde(rename = "closeScores", default)]
    pub close_scores: Vec<CLOSEScoreResult>,

    /// Whether execution was paused (can be resumed)
    pub paused: bool,

    /// Pause reason if paused
    #[serde(rename = "pauseReason")]
    pub pause_reason: Option<String>,

    /// Error message if failed
    pub error: Option<String>,
}

/// Request to execute autonomous task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousRequest {
    /// Task request/description
    pub request: String,

    /// Agent to use for execution (e.g., "verifier", "macro", "general")
    pub agent: Option<String>,

    /// Autonomous configuration
    pub config: AutonomousConfig,

    /// Optional context from Hands
    pub context: Option<HandsContext>,

    /// Optional session ID (for resuming)
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,

    /// Optional auto-approve configuration
    #[serde(rename = "autoApproveConfig")]
    pub auto_approve_config: Option<AutoApproveConfig>,
}

/// Response from autonomous execute endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousResponse {
    /// Request success
    pub success: bool,

    /// Session ID for tracking/resuming
    #[serde(rename = "sessionId")]
    pub session_id: String,

    /// Execution result (if completed)
    pub result: Option<AutonomousExecutionResult>,

    /// Error message if failed
    pub error: Option<String>,
}

/// Error response from API.
#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: String,
}

/// Autonomous bridge client.
///
/// Handles HTTP communication with the CodeCoder autonomous API.
#[derive(Debug, Clone)]
pub struct AutonomousBridge {
    /// Base URL for the ccode API
    base_url: String,

    /// HTTP client
    client: reqwest::Client,

    /// Timeout for requests
    timeout: Duration,
}

impl AutonomousBridge {
    /// Create a new autonomous bridge client.
    ///
    /// # Arguments
    ///
    /// * `base_url` - Base URL of the ccode API (e.g., "http://127.0.0.1:4400")
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .unwrap();

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client,
            timeout: Duration::from_secs(300),
        }
    }

    /// Set a custom timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self.client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .unwrap();
        self
    }

    /// Get the autonomous endpoint URL.
    fn endpoint_url(&self) -> String {
        format!("{}/api/v1/autonomous/execute", self.base_url)
    }

    /// Execute an autonomous task.
    ///
    /// # Arguments
    ///
    /// * `request` - Autonomous execution request
    ///
    /// # Returns
    ///
    /// The autonomous execution result
    pub async fn execute(&self, request: AutonomousRequest) -> Result<AutonomousExecutionResult> {
        let url = self.endpoint_url();

        tracing::debug!(
            hand_id = request.context.as_ref().map(|c| c.hand_id.as_str()),
            autonomy_level = ?request.config.autonomy_level,
            "Calling autonomous API"
        );

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to call autonomous API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            // Try to parse error response
            if let Ok(err) = serde_json::from_str::<ErrorResponse>(&body) {
                anyhow::bail!("Autonomous API returned {}: {}", status, err.error);
            }

            anyhow::bail!("Autonomous API returned {}: {}", status, body);
        }

        let api_response: AutonomousResponse = response
            .json()
            .await
            .context("Failed to parse autonomous response")?;

        if !api_response.success {
            if let Some(error) = api_response.error {
                anyhow::bail!("Autonomous execution failed: {}", error);
            }
            anyhow::bail!("Autonomous execution failed");
        }

        api_response.result.context("Missing result in response")
    }

    /// Execute a hand using autonomous mode.
    ///
    /// # Arguments
    ///
    /// * `hand` - Hand manifest
    /// * `previous_results` - Previous execution results for context
    ///
    /// # Returns
    ///
    /// The autonomous execution result
    pub async fn execute_hand(
        &self,
        hand: &HandManifest,
        previous_results: Vec<PreviousResult>,
    ) -> Result<AutonomousExecutionResult> {
        let autonomy = hand.config.autonomy.as_ref().context("Hand has no autonomy config")?;
        let resources = hand.config.resources.as_ref().cloned().unwrap_or_default();

        // Convert previous executions
        let context = HandsContext {
            hand_id: hand.config.id.clone(),
            hand_name: Some(hand.config.name.clone()),
            previous_results,
            custom_state: serde_json::json!({}),
        };

        // Build auto-approve config from hand manifest
        let auto_approve_config = autonomy.auto_approve.as_ref().map(|config| {
            AutoApproveConfig {
                enabled: config.enabled,
                allowed_tools: config.allowed_tools.clone(),
                risk_threshold: BridgeRiskThreshold::from(config.risk_threshold),
                timeout_ms: config.timeout_ms,
                unattended: autonomy.unattended,
            }
        });

        let request = AutonomousRequest {
            request: format!(
                "{}\n\n{}",
                hand.config.description,
                hand.content.chars().take(1000).collect::<String>()
            ),
            agent: Some(hand.config.agent.clone()),
            config: AutonomousConfig {
                autonomy_level: BridgeAutonomyLevel::from(autonomy.level),
                unattended: autonomy.unattended,
                resource_budget: ResourceBudget {
                    max_tokens: resources.max_tokens,
                    max_cost_usd: resources.max_cost_usd,
                    max_duration_sec: resources.max_duration_sec,
                },
                enable_evolution_loop: hand
                    .config
                    .decision
                    .as_ref()
                    .map_or(false, |d| d.evolution),
                enable_web_search: hand
                    .config
                    .decision
                    .as_ref()
                    .map_or(false, |d| d.web_search),
                max_iterations: Some(autonomy.max_iterations),
            },
            context: Some(context),
            session_id: None,
            auto_approve_config,
        };

        self.execute(request).await
    }

    /// Health check for the autonomous API.
    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/api/v1/autonomous/health", self.base_url);

        let response = self
            .client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .context("Failed to call health check")?;

        Ok(response.status().is_success())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autonomy_level_conversion() {
        assert_eq!(
            BridgeAutonomyLevel::from(AutonomyLevel::Crazy),
            BridgeAutonomyLevel::Crazy
        );
        assert_eq!(
            BridgeAutonomyLevel::from(AutonomyLevel::Lunatic),
            BridgeAutonomyLevel::Lunatic
        );
    }

    #[test]
    fn test_resource_budget_default() {
        let budget = ResourceBudget::default();
        assert_eq!(budget.max_tokens, 100000);
        assert_eq!(budget.max_cost_usd, 5.0);
        assert_eq!(budget.max_duration_sec, 600);
    }

    #[test]
    fn test_bridge_creation() {
        let bridge = AutonomousBridge::new("http://localhost:4400".to_string());
        assert_eq!(bridge.base_url, "http://localhost:4400");
        assert_eq!(
            bridge.endpoint_url(),
            "http://localhost:4400/api/v1/autonomous/execute"
        );
    }
}
