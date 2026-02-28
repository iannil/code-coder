//! Hand execution engine.
//!
//! Executes a Hand by:
//! 1. Loading previous state from SQLite
//! 2. Building context with state history
//! 3. Calling the configured agent via ccode API
//!    - If autonomy is configured, uses Autonomous Bridge
//!    - Otherwise, uses simple chat API
//! 4. Recording result to SQLite
//! 5. Writing Markdown memory file
//! 6. Updating state for next run

use super::manifest::HandManifest;
use super::autonomous_bridge::{AutonomousBridge, PreviousResult};
use super::state::{ExecutionStatus, HandExecution, StateStore};
use anyhow::{Context, Result};
use chrono::Utc;
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Request to CodeCoder API for agent execution.
#[derive(Debug, Serialize)]
struct CodeCoderRequest {
    /// Message/prompt for the agent
    message: String,

    /// Agent to use
    #[serde(skip_serializing_if = "Option::is_none")]
    agent: Option<String>,

    /// User ID for tracking
    user_id: String,

    /// Channel identifier
    channel: String,

    /// Context from previous runs
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<AgentContext>,
}

/// Context provided to the agent from previous executions.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AgentContext {
    /// Hand ID
    hand_id: String,

    /// Hand name
    hand_name: String,

    /// Last execution output
    #[serde(skip_serializing_if = "Option::is_none")]
    last_output: Option<String>,

    /// Last execution time
    #[serde(skip_serializing_if = "Option::is_none")]
    last_execution_at: Option<String>,

    /// Run statistics
    #[serde(skip_serializing_if = "Option::is_none")]
    stats: Option<ExecutionStats>,

    /// Custom state from previous runs
    #[serde(skip_serializing_if = "Option::is_none")]
    custom_state: Option<serde_json::Value>,
}

/// Execution statistics for context.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecutionStats {
    total_runs: i64,
    success_count: i64,
    failure_count: i64,
}

/// Response from CodeCoder API.
#[derive(Debug, Deserialize)]
struct CodeCoderResponse {
    /// Response content
    #[serde(default)]
    content: String,

    /// Error if any
    #[serde(default)]
    error: Option<String>,
}

/// Hand execution engine.
pub struct HandExecutor {
    /// CodeCoder API endpoint
    codecoder_endpoint: String,

    /// HTTP client
    client: reqwest::Client,

    /// State store
    state_store: Arc<StateStore>,

    /// Memory base directory
    memory_dir: PathBuf,

    /// Autonomous bridge (optional, created when needed)
    autonomous_bridge: Option<AutonomousBridge>,
}

impl HandExecutor {
    /// Create a new hand executor.
    pub fn new(codecoder_endpoint: String) -> Result<Self> {
        let state_store = Arc::new(
            StateStore::new()
                .context("Failed to create state store")?
        );

        let memory_dir = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("memory")
            .join("hands");

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(300)) // 5 minute timeout
            .build()
            .context("Failed to create HTTP client")?;

        // Create autonomous bridge
        let autonomous_bridge = AutonomousBridge::new(codecoder_endpoint.clone());

        Ok(Self {
            codecoder_endpoint,
            client,
            state_store,
            memory_dir,
            autonomous_bridge: Some(autonomous_bridge),
        })
    }

    /// Set a custom memory directory.
    pub fn with_memory_dir(mut self, dir: PathBuf) -> Self {
        self.memory_dir = dir;
        self
    }

    /// Set a custom state store.
    pub fn with_state_store(mut self, store: Arc<StateStore>) -> Self {
        self.state_store = store;
        self
    }

    /// Execute a hand.
    pub async fn execute(&self, hand: &HandManifest) -> Result<HandExecution> {
        tracing::info!(
            hand_id = %hand.config.id,
            agent = %hand.config.agent,
            autonomous = hand.is_autonomous(),
            "Executing hand"
        );

        // Load previous state
        let state = self.state_store.get_state(&hand.config.id)?;

        // Check if hand uses autonomous mode
        let use_autonomous = hand.is_autonomous();

        // Create execution record
        let mut execution = self.state_store.create_execution(
            &hand.config.id,
            state.last_execution_id.clone(),
        )?;

        // Update to running status
        execution.status = ExecutionStatus::Running;
        self.state_store.update_execution(&execution)?;

        // Build context for agent
        let context = self.build_context(hand, &state, &execution).await?;

        // Build previous results for autonomous mode
        let previous_results = self.build_previous_results(&state).await?;

        // Call agent (either autonomous or simple)
        let (result, autonomous_metadata) = if use_autonomous {
            let (output, metadata) = self.call_autonomous(hand, &context, previous_results).await?;
            (Ok(output), Some(metadata))
        } else {
            let prompt = self.build_prompt(hand, &context);
            (self.call_agent(&hand.config.agent, &prompt, &context).await, None)
        };

        // Handle result
        match result {
            Ok(output) => {
                execution.status = ExecutionStatus::Success;
                execution.output = Some(output.clone());
                execution.ended_at = Some(Utc::now());

                // Write memory file
                if let Some(memory_path) = self.write_memory(hand, &execution, &output).await? {
                    execution.memory_path = Some(memory_path.to_string_lossy().to_string());
                }

                tracing::info!(
                    hand_id = %hand.config.id,
                    execution_id = %execution.id,
                    "Hand executed successfully"
                );
            }
            Err(e) => {
                execution.status = ExecutionStatus::Failed;
                execution.error = Some(e.to_string());
                execution.ended_at = Some(Utc::now());

                tracing::error!(
                    hand_id = %hand.config.id,
                    execution_id = %execution.id,
                    error = %e,
                    "Hand execution failed"
                );
            }
        }

        // Update metadata
        let duration_ms = execution.ended_at.unwrap_or(Utc::now())
            .timestamp_millis() - execution.started_at.timestamp_millis();

        execution.metadata = if let Some(meta) = autonomous_metadata {
            serde_json::json!({
                "duration_ms": duration_ms,
                "autonomous": true,
                "close_scores": meta.get("close_scores"),
                "quality_score": meta.get("quality_score"),
                "craziness_score": meta.get("craziness_score"),
                "tokens_used": meta.get("tokens_used"),
                "cost_usd": meta.get("cost_usd"),
            })
        } else {
            serde_json::json!({
                "duration_ms": duration_ms,
                "autonomous": false,
            })
        };

        // Store execution result
        self.state_store.update_execution(&execution)?;
        self.state_store.update_state(&hand.config.id, &execution)?;

        Ok(execution)
    }

    /// Build agent context from state.
    async fn build_context(
        &self,
        hand: &HandManifest,
        state: &crate::hands::state::HandState,
        _execution: &HandExecution,
    ) -> Result<AgentContext> {
        let last_output = if let Some(ref last_id) = state.last_execution_id {
            if let Ok(Some(last_exec)) = self.state_store.get_execution(last_id) {
                last_exec.output
            } else {
                None
            }
        } else {
            None
        };

        let last_execution_at = state.last_success_at
            .as_ref()
            .map(|dt| dt.to_rfc3339());

        Ok(AgentContext {
            hand_id: hand.config.id.clone(),
            hand_name: hand.config.name.clone(),
            last_output,
            last_execution_at,
            stats: Some(ExecutionStats {
                total_runs: state.total_runs,
                success_count: state.success_count,
                failure_count: state.failure_count,
            }),
            custom_state: if state.custom_state.is_object() {
                Some(state.custom_state.clone())
            } else {
                None
            },
        })
    }

    /// Build the prompt for the agent.
    fn build_prompt(&self, hand: &HandManifest, context: &AgentContext) -> String {
        let mut prompt = format!(
            "# Hand: {}\n\n",
            hand.config.name
        );

        // Add description if available
        if !hand.config.description.is_empty() {
            prompt.push_str(&format!("**Description:** {}\n\n", hand.config.description));
        }

        // Add markdown content
        if !hand.content.is_empty() {
            prompt.push_str(&format!("**Instructions:**\n\n{}\n\n", hand.content));
        }

        // Add context
        prompt.push_str("**Context:**\n\n");

        if let Some(stats) = &context.stats {
            prompt.push_str(&format!(
                "- Run #{} ({} successes, {} failures)\n",
                stats.total_runs + 1,
                stats.success_count,
                stats.failure_count
            ));
        }

        if let Some(last_output) = &context.last_output {
            prompt.push_str(&format!("- **Previous output:** {}\n\n",
                last_output.chars().take(200).collect::<String>()));
        }

        // Add custom params
        if hand.config.params.is_object() && !hand.config.params.as_object().unwrap().is_empty() {
            prompt.push_str(&format!("**Parameters:**\n```\n{}\n```\n\n",
                serde_json::to_string_pretty(&hand.config.params).unwrap_or_default()));
        }

        prompt.push_str("**Please provide your analysis:**\n");

        prompt
    }

    /// Call the CodeCoder agent.
    async fn call_agent(
        &self,
        agent: &str,
        prompt: &str,
        context: &AgentContext,
    ) -> Result<String> {
        let url = format!("{}/api/v1/chat", self.codecoder_endpoint.trim_end_matches('/'));

        let request = CodeCoderRequest {
            message: prompt.to_string(),
            agent: Some(agent.to_string()),
            user_id: format!("hand-{}", context.hand_id),
            channel: "hands".to_string(),
            context: Some(context.clone()),
        };

        tracing::debug!(
            agent = %agent,
            hand_id = %context.hand_id,
            "Calling CodeCoder API"
        );

        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to call CodeCoder API")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("CodeCoder API returned {}: {}", status, body);
        }

        let codecoder_response: CodeCoderResponse = response
            .json()
            .await
            .context("Failed to parse CodeCoder response")?;

        if let Some(error) = codecoder_response.error {
            anyhow::bail!("Agent error: {}", error);
        }

        Ok(codecoder_response.content)
    }

    /// Build previous results for autonomous mode context.
    async fn build_previous_results(&self, state: &crate::hands::state::HandState) -> Result<Vec<PreviousResult>> {
        let mut results = Vec::new();

        // Get the last few executions
        if let Some(ref last_id) = state.last_execution_id {
            if let Ok(Some(exec)) = self.state_store.get_execution(last_id) {
                let prev_id = exec.previous_execution_id.clone();

                results.push(PreviousResult {
                    timestamp: exec.started_at.to_rfc3339(),
                    output: exec.output.unwrap_or_default(),
                    success: exec.status == ExecutionStatus::Success,
                });

                // Try to get the execution before that
                if let Some(ref prev_id) = prev_id {
                    if let Ok(Some(prev_exec)) = self.state_store.get_execution(prev_id) {
                        results.push(PreviousResult {
                            timestamp: prev_exec.started_at.to_rfc3339(),
                            output: prev_exec.output.unwrap_or_default(),
                            success: prev_exec.status == ExecutionStatus::Success,
                        });
                    }
                }
            }
        }

        // Reverse so most recent is last (matching the API expectation)
        results.reverse();
        Ok(results)
    }

    /// Call the autonomous API for enhanced execution with CLOSE decision framework.
    async fn call_autonomous(
        &self,
        hand: &HandManifest,
        context: &AgentContext,
        previous_results: Vec<PreviousResult>,
    ) -> Result<(String, serde_json::Value)> {
        let bridge = self.autonomous_bridge.as_ref()
            .context("Autonomous bridge not initialized")?;

        tracing::debug!(
            hand_id = %context.hand_id,
            autonomy_level = ?hand.config.autonomy.as_ref().map(|a| a.level),
            "Calling autonomous API"
        );

        let result = bridge.execute_hand(hand, previous_results).await
            .context("Failed to execute via autonomous bridge")?;

        // Check if execution was paused
        if result.paused {
            tracing::warn!(
                hand_id = %context.hand_id,
                pause_reason = ?result.pause_reason,
                "Autonomous execution paused"
            );
        }

        // Check for explicit failure
        if !result.success {
            if let Some(error) = &result.error {
                anyhow::bail!("Autonomous execution failed: {}", error);
            }
        }

        // Log CLOSE scores if available
        if !result.close_scores.is_empty() {
            if let Some(first_score) = result.close_scores.first() {
                tracing::info!(
                    hand_id = %context.hand_id,
                    close_score = first_score.total,
                    quality_score = result.quality_score,
                    craziness_score = result.craziness_score,
                    "Autonomous execution completed with CLOSE evaluation"
                );
            }
        }

        // Build metadata
        let metadata = serde_json::json!({
            "close_scores": result.close_scores,
            "quality_score": result.quality_score,
            "craziness_score": result.craziness_score,
            "tokens_used": result.tokens_used,
            "cost_usd": result.cost_usd,
            "iterations": result.iterations_completed,
            "paused": result.paused,
            "pause_reason": result.pause_reason,
        });

        Ok((result.output, metadata))
    }

    /// Write execution result to memory file.
    async fn write_memory(
        &self,
        hand: &HandManifest,
        execution: &HandExecution,
        output: &str,
    ) -> Result<Option<PathBuf>> {
        let memory_path_template = hand.config.memory_path.as_ref()
            .map(|s| s.as_str())
            .unwrap_or("hands/{id}/{date}.md");

        // Replace placeholders
        let date = execution.started_at.format("%Y-%m-%d").to_string();
        let memory_path = memory_path_template
            .replace("{id}", &hand.config.id)
            .replace("{name}", &hand.config.name)
            .replace("{date}", &date)
            .replace("{version}", &hand.config.version);

        let full_path = self.memory_dir.join(&memory_path);

        // Ensure directory exists
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create memory directory: {}", parent.display()))?;
        }

        // Build markdown content
        let content = self.format_memory_output(hand, execution, output);

        // Write file
        std::fs::write(&full_path, content)
            .with_context(|| format!("Failed to write memory file: {}", full_path.display()))?;

        tracing::debug!(
            hand_id = %hand.config.id,
            path = %full_path.display(),
            "Wrote memory file"
        );

        Ok(Some(full_path))
    }

    /// Format the memory output as markdown.
    fn format_memory_output(&self, hand: &HandManifest, execution: &HandExecution, output: &str) -> String {
        let now = Utc::now();
        let datetime = now.format("%Y-%m-%d %H:%M:%S UTC").to_string();

        let mut report = format!(
            "# {} - Execution Report\n\n",
            hand.config.name
        );

        report.push_str(&format!("**Hand ID:** {}\n", hand.config.id));
        report.push_str(&format!("**Execution ID:** {}\n", execution.id));
        report.push_str(&format!("**Status:** {:?}\n", execution.status));
        report.push_str(&format!("**Started:** {}\n", execution.started_at.format("%Y-%m-%d %H:%M:%S UTC")));
        report.push_str(&format!("**Ended:** {}\n",
            execution.ended_at.map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string()).unwrap_or_else(|| "N/A".to_string())));
        report.push_str(&format!("**Agent:** {}\n", hand.config.agent));

        // Add autonomy info if configured
        if let Some(ref autonomy) = hand.config.autonomy {
            report.push_str(&format!("**Autonomy Level:** {:?}\n", autonomy.level));
        }

        report.push_str("\n---\n\n");

        // Add CLOSE scores from metadata if available
        if let Some(metadata) = execution.metadata.as_object() {
            if let Some(close_scores) = metadata.get("close_scores").and_then(|v| v.as_array()) {
                if let Some(first_score) = close_scores.first() {
                    report.push_str("## CLOSE Evaluation\n\n");
                    if let Some(total) = first_score.get("total").and_then(|v| v.as_f64()) {
                        report.push_str(&format!("**CLOSE Score:** {:.2}/10\n", total));
                    }
                    if let Some(convergence) = first_score.get("convergence").and_then(|v| v.as_f64()) {
                        report.push_str(&format!("- Convergence: {:.2}\n", convergence));
                    }
                    if let Some(leverage) = first_score.get("leverage").and_then(|v| v.as_f64()) {
                        report.push_str(&format!("- Leverage: {:.2}\n", leverage));
                    }
                    if let Some(optionality) = first_score.get("optionality").and_then(|v| v.as_f64()) {
                        report.push_str(&format!("- Optionality: {:.2}\n", optionality));
                    }
                    if let Some(surplus) = first_score.get("surplus").and_then(|v| v.as_f64()) {
                        report.push_str(&format!("- Surplus: {:.2}\n", surplus));
                    }
                    if let Some(evolution) = first_score.get("evolution").and_then(|v| v.as_f64()) {
                        report.push_str(&format!("- Evolution: {:.2}\n", evolution));
                    }
                    report.push_str("\n");
                }
            }
        }

        report.push_str("## Output\n\n");
        report.push_str(output);
        report.push_str("\n\n---\n\n");
        report.push_str(&format!("*Generated by Hands system at {}*\n", datetime));

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_build_prompt() {
        let tmp = TempDir::new().unwrap();
        let store = Arc::new(StateStore::with_data_dir(tmp.path().to_path_buf()).unwrap());
        let executor = HandExecutor::new("http://localhost:4400".to_string())
            .unwrap()
            .with_state_store(store);

        let hand_md = r#"---
id: "test"
name: "Test Hand"
schedule: "0 * * * * *"
agent: "echo"
enabled: true
---

Test description
"#;
        let hand = HandManifest::from_content(
            hand_md.to_string(),
            PathBuf::from("/test/HAND.md")
        ).unwrap();

        let context = AgentContext {
            hand_id: "test".to_string(),
            hand_name: "Test Hand".to_string(),
            last_output: None,
            last_execution_at: None,
            stats: Some(ExecutionStats {
                total_runs: 5,
                success_count: 4,
                failure_count: 1,
            }),
            custom_state: None,
        };

        let prompt = executor.build_prompt(&hand, &context);

        assert!(prompt.contains("Test Hand"));
        assert!(prompt.contains("Test description"));
        assert!(prompt.contains("Run #6"));
        assert!(prompt.contains("4 successes"));
    }

    #[test]
    fn test_format_memory_output() {
        let tmp = TempDir::new().unwrap();
        let store = Arc::new(StateStore::with_data_dir(tmp.path().to_path_buf()).unwrap());
        let executor = HandExecutor::new("http://localhost:4400".to_string())
            .unwrap()
            .with_state_store(store);

        let hand = HandManifest::from_content(
            r#"---
id: "test"
name: "Test Hand"
schedule: "0 * * * * *"
agent: "echo"
enabled: true
---
"#.to_string(),
            PathBuf::from("/test/HAND.md")
        ).unwrap();

        let exec = HandExecution {
            id: "exec-1".to_string(),
            hand_id: "test".to_string(),
            status: ExecutionStatus::Success,
            started_at: Utc::now(),
            ended_at: Some(Utc::now()),
            output: Some("Test output".to_string()),
            error: None,
            memory_path: None,
            previous_execution_id: None,
            metadata: serde_json::json!({}),
        };

        let output = executor.format_memory_output(&hand, &exec, "Agent response");

        assert!(output.contains("# Test Hand - Execution Report"));
        assert!(output.contains("## Output"));
        assert!(output.contains("Agent response"));
        assert!(output.contains("**Status:** Success"));
    }
}
