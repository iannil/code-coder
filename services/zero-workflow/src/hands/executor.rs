//! Hand execution engine.
//!
//! Executes a Hand by:
//! 1. Loading previous state from SQLite
//! 2. Building context with state history
//! 3. Checking auto-approval configuration for tool calls
//! 4. Calling the configured agent via ccode API
//!    - If autonomy is configured, uses Autonomous Bridge
//!    - Otherwise, uses simple chat API
//!    - For pipeline hands, executes multiple agents sequentially or in parallel
//! 5. Recording result to SQLite
//! 6. Writing Markdown memory file
//! 7. Updating state for next run

use super::auto_approve::{ApprovalDecision, ApprovalResult, AutoApprover};
use super::manifest::{HandManifest, PipelineMode};
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

    /// HITL Gateway endpoint (for approval requests)
    hitl_endpoint: Option<String>,
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

        // Default HITL endpoint
        let hitl_endpoint = Some("http://127.0.0.1:4430".to_string());

        Ok(Self {
            codecoder_endpoint,
            client,
            state_store,
            memory_dir,
            autonomous_bridge: Some(autonomous_bridge),
            hitl_endpoint,
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

    /// Set a custom HITL endpoint.
    pub fn with_hitl_endpoint(mut self, endpoint: String) -> Self {
        self.hitl_endpoint = Some(endpoint);
        self
    }

    /// Build an AutoApprover for a hand based on its configuration.
    fn build_auto_approver(&self, hand: &HandManifest) -> AutoApprover {
        let autonomy = hand.config.autonomy.as_ref();

        match autonomy {
            Some(autonomy_config) => {
                match &autonomy_config.auto_approve {
                    Some(auto_approve) => {
                        AutoApprover::new(
                            auto_approve.clone(),
                            autonomy_config.level,
                            autonomy_config.unattended,
                        )
                    }
                    None => {
                        // No auto_approve config, create a disabled approver
                        AutoApprover::disabled()
                    }
                }
            }
            None => {
                // No autonomy config, create a disabled approver
                AutoApprover::disabled()
            }
        }
    }

    /// Execute a hand.
    pub async fn execute(&self, hand: &HandManifest) -> Result<HandExecution> {
        let is_pipeline = hand.config.is_pipeline();
        let agents = hand.config.get_agents();

        tracing::info!(
            hand_id = %hand.config.id,
            agents = ?agents,
            pipeline = is_pipeline,
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

        // Call agent(s) based on pipeline configuration
        let (result, autonomous_metadata) = if is_pipeline {
            // Pipeline execution: multiple agents
            let pipeline_mode = hand.config.get_pipeline_mode();
            self.execute_pipeline(hand, &agents, pipeline_mode, &context, previous_results).await
        } else if use_autonomous {
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

    /// Execute a pipeline of multiple agents.
    async fn execute_pipeline(
        &self,
        hand: &HandManifest,
        agents: &[String],
        mode: PipelineMode,
        context: &AgentContext,
        previous_results: Vec<PreviousResult>,
    ) -> (Result<String>, Option<serde_json::Value>) {
        tracing::info!(
            hand_id = %hand.config.id,
            agents = ?agents,
            mode = ?mode,
            "Executing pipeline"
        );

        match mode {
            PipelineMode::Sequential => {
                self.execute_sequential(hand, agents, context, previous_results).await
            }
            PipelineMode::Parallel => {
                self.execute_parallel(hand, agents, context).await
            }
            PipelineMode::Conditional => {
                // Conditional execution requires CLOSE framework
                // Falls back to sequential for now, with decision points between agents
                self.execute_sequential(hand, agents, context, previous_results).await
            }
        }
    }

    /// Execute agents sequentially, passing output from one to the next.
    async fn execute_sequential(
        &self,
        hand: &HandManifest,
        agents: &[String],
        context: &AgentContext,
        _previous_results: Vec<PreviousResult>,
    ) -> (Result<String>, Option<serde_json::Value>) {
        let mut accumulated_output = String::new();
        let mut all_metadata = Vec::new();
        let mut current_context = context.clone();

        for (i, agent) in agents.iter().enumerate() {
            tracing::debug!(
                hand_id = %hand.config.id,
                agent = %agent,
                step = i + 1,
                total = agents.len(),
                "Executing pipeline step"
            );

            // Build prompt with previous agent's output
            let prompt = if accumulated_output.is_empty() {
                self.build_prompt(hand, &current_context)
            } else {
                format!(
                    "{}\n\n## Previous Agent Output\n\n{}\n\n**Please continue with your analysis:**\n",
                    self.build_prompt(hand, &current_context),
                    accumulated_output
                )
            };

            // Call the agent
            match self.call_agent(agent, &prompt, &current_context).await {
                Ok(output) => {
                    all_metadata.push(serde_json::json!({
                        "agent": agent,
                        "step": i + 1,
                        "success": true,
                    }));

                    // Update context with this output for next agent
                    current_context.last_output = Some(output.clone());
                    accumulated_output = output;
                }
                Err(e) => {
                    tracing::error!(
                        hand_id = %hand.config.id,
                        agent = %agent,
                        step = i + 1,
                        error = %e,
                        "Pipeline step failed"
                    );
                    all_metadata.push(serde_json::json!({
                        "agent": agent,
                        "step": i + 1,
                        "success": false,
                        "error": e.to_string(),
                    }));
                    return (Err(e), Some(serde_json::json!({
                        "pipeline": true,
                        "mode": "sequential",
                        "completed_steps": i,
                        "total_steps": agents.len(),
                        "steps": all_metadata,
                    })));
                }
            }
        }

        let metadata = serde_json::json!({
            "pipeline": true,
            "mode": "sequential",
            "completed_steps": agents.len(),
            "total_steps": agents.len(),
            "steps": all_metadata,
        });

        (Ok(accumulated_output), Some(metadata))
    }

    /// Execute agents in parallel and merge their outputs.
    async fn execute_parallel(
        &self,
        hand: &HandManifest,
        agents: &[String],
        context: &AgentContext,
    ) -> (Result<String>, Option<serde_json::Value>) {
        use futures::future::join_all;

        tracing::debug!(
            hand_id = %hand.config.id,
            agents = ?agents,
            "Executing pipeline steps in parallel"
        );

        let prompt = self.build_prompt(hand, context);

        // Create futures for all agents
        let futures: Vec<_> = agents.iter()
            .map(|agent| {
                let agent = agent.clone();
                let prompt = prompt.clone();
                let context = context.clone();
                async move {
                    let result = self.call_agent(&agent, &prompt, &context).await;
                    (agent, result)
                }
            })
            .collect();

        // Execute all in parallel
        let results = join_all(futures).await;

        // Collect results
        let mut outputs = Vec::new();
        let mut all_metadata = Vec::new();
        let mut had_error = false;
        let mut first_error: Option<anyhow::Error> = None;

        for (agent, result) in results {
            match result {
                Ok(output) => {
                    all_metadata.push(serde_json::json!({
                        "agent": agent,
                        "success": true,
                    }));
                    outputs.push(format!("## {} Output\n\n{}", agent, output));
                }
                Err(e) => {
                    had_error = true;
                    all_metadata.push(serde_json::json!({
                        "agent": agent,
                        "success": false,
                        "error": e.to_string(),
                    }));
                    if first_error.is_none() {
                        first_error = Some(e);
                    }
                }
            }
        }

        let metadata = serde_json::json!({
            "pipeline": true,
            "mode": "parallel",
            "completed_steps": outputs.len(),
            "total_steps": agents.len(),
            "steps": all_metadata,
        });

        // If all agents failed, return the first error
        if outputs.is_empty() && had_error {
            return (Err(first_error.unwrap_or_else(|| anyhow::anyhow!("All parallel agents failed"))), Some(metadata));
        }

        // Merge all outputs
        let merged_output = outputs.join("\n\n---\n\n");

        (Ok(merged_output), Some(metadata))
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

    /// Check if a tool call should be auto-approved or needs human approval.
    ///
    /// Returns the approval decision and optionally creates an approval request
    /// in the HITL system if the operation requires human review.
    pub async fn check_tool_approval(
        &self,
        hand: &HandManifest,
        execution_id: &str,
        tool: &str,
        args: &serde_json::Value,
    ) -> Result<ApprovalResult> {
        let auto_approver = self.build_auto_approver(hand);
        let result = auto_approver.should_approve(tool, args);

        tracing::debug!(
            hand_id = %hand.config.id,
            tool = %tool,
            decision = ?result.decision,
            risk_level = ?result.risk_evaluation.risk_level,
            "Tool approval check"
        );

        // If queued, create an approval request in HITL
        if result.decision == ApprovalDecision::Queue {
            if let Some(ref hitl_endpoint) = self.hitl_endpoint {
                if let Err(e) = self.create_hitl_approval_request(
                    hitl_endpoint,
                    hand,
                    execution_id,
                    tool,
                    args,
                    &result,
                ).await {
                    tracing::warn!(
                        hand_id = %hand.config.id,
                        tool = %tool,
                        error = %e,
                        "Failed to create HITL approval request, falling back to queue"
                    );
                }
            }
        }

        Ok(result)
    }

    /// Create an approval request in the HITL system.
    async fn create_hitl_approval_request(
        &self,
        hitl_endpoint: &str,
        hand: &HandManifest,
        execution_id: &str,
        tool: &str,
        args: &serde_json::Value,
        approval_result: &ApprovalResult,
    ) -> Result<String> {
        let risk_level = match approval_result.risk_evaluation.risk_level {
            super::risk::RiskLevel::Safe => "Low",
            super::risk::RiskLevel::Low => "Low",
            super::risk::RiskLevel::Medium => "Medium",
            super::risk::RiskLevel::High => "High",
            super::risk::RiskLevel::Critical => "Critical",
        };

        let request_body = serde_json::json!({
            "approval_type": {
                "type": "tool_execution",
                "tool": tool,
                "args": args,
                "risk_level": risk_level,
                "hand_id": hand.config.id,
                "execution_id": execution_id,
            },
            "requester": format!("hand:{}", hand.config.id),
            "approvers": ["admin"],
            "title": format!("Execute {} for Hand '{}'", tool, hand.config.name),
            "description": format!(
                "Tool: {}\nHand: {} ({})\nRisk: {}\nReasons: {}",
                tool,
                hand.config.name,
                hand.config.id,
                approval_result.risk_evaluation.risk_level,
                approval_result.reasons.join(", ")
            ),
            "channel": "tui",
            "metadata": {
                "hand_id": hand.config.id,
                "execution_id": execution_id,
                "tool": tool,
                "risk_level": risk_level,
            },
            "ttl_seconds": approval_result.timeout_ms.map(|ms| ms / 1000),
        });

        let url = format!("{}/api/v1/hitl/request", hitl_endpoint.trim_end_matches('/'));

        let response = self.client
            .post(&url)
            .json(&request_body)
            .timeout(Duration::from_secs(10))
            .send()
            .await
            .context("Failed to create HITL approval request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("HITL API returned {}: {}", status, body);
        }

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct HitlResponse {
            success: bool,
            approval: Option<HitlApproval>,
            error: Option<String>,
        }

        #[derive(Deserialize)]
        struct HitlApproval {
            id: String,
        }

        let hitl_response: HitlResponse = response
            .json()
            .await
            .context("Failed to parse HITL response")?;

        if let Some(error) = hitl_response.error {
            anyhow::bail!("HITL error: {}", error);
        }

        let approval_id = hitl_response.approval
            .map(|a| a.id)
            .unwrap_or_else(|| "unknown".to_string());

        tracing::info!(
            hand_id = %hand.config.id,
            tool = %tool,
            approval_id = %approval_id,
            "Created HITL approval request"
        );

        Ok(approval_id)
    }

    /// Poll for approval status from HITL.
    ///
    /// Returns true if approved, false if rejected, or an error if still pending.
    pub async fn poll_approval_status(&self, approval_id: &str) -> Result<Option<bool>> {
        let hitl_endpoint = self.hitl_endpoint.as_ref()
            .ok_or_else(|| anyhow::anyhow!("HITL endpoint not configured"))?;

        let url = format!("{}/api/v1/hitl/{}", hitl_endpoint.trim_end_matches('/'), approval_id);

        let response = self.client
            .get(&url)
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .context("Failed to poll HITL approval status")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("HITL API returned {}: {}", status, body);
        }

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct HitlStatusResponse {
            success: bool,
            approval: Option<HitlApprovalStatus>,
            error: Option<String>,
        }

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct HitlApprovalStatus {
            status: HitlStatus,
        }

        #[derive(Deserialize)]
        #[serde(tag = "status", rename_all = "snake_case")]
        #[allow(dead_code)]
        enum HitlStatus {
            Pending,
            Approved { by: String, at: String },
            Rejected { by: String, reason: Option<String>, at: String },
            Cancelled { reason: String },
        }

        let status_response: HitlStatusResponse = response
            .json()
            .await
            .context("Failed to parse HITL status response")?;

        if let Some(error) = status_response.error {
            anyhow::bail!("HITL error: {}", error);
        }

        match status_response.approval {
            Some(approval) => {
                match approval.status {
                    HitlStatus::Pending => Ok(None),
                    HitlStatus::Approved { .. } => Ok(Some(true)),
                    HitlStatus::Rejected { .. } => Ok(Some(false)),
                    HitlStatus::Cancelled { .. } => Ok(Some(false)),
                }
            }
            None => Ok(None),
        }
    }

    /// Wait for approval with timeout support.
    ///
    /// This method polls for approval status and handles timeout-based auto-approval
    /// for unattended mode. Critical risk operations never auto-approve.
    ///
    /// Returns:
    /// - Ok(true) if approved (by human or timeout)
    /// - Ok(false) if rejected
    /// - Err if polling failed or timeout for critical operations
    pub async fn wait_for_approval(
        &self,
        approval_id: &str,
        approval_result: &ApprovalResult,
    ) -> Result<bool> {
        let timeout_ms = approval_result.timeout_ms.unwrap_or(0);
        let timeout_applicable = approval_result.timeout_applicable;
        let is_critical = approval_result.risk_evaluation.risk_level == super::risk::RiskLevel::Critical;

        let start_time = std::time::Instant::now();
        let poll_interval = Duration::from_secs(2);

        tracing::info!(
            approval_id = %approval_id,
            timeout_ms = timeout_ms,
            timeout_applicable = timeout_applicable,
            is_critical = is_critical,
            "Waiting for approval"
        );

        loop {
            // Check elapsed time
            let elapsed = start_time.elapsed();

            // Check for timeout auto-approval
            if timeout_applicable && !is_critical && timeout_ms > 0 {
                if elapsed.as_millis() as u64 >= timeout_ms {
                    tracing::warn!(
                        approval_id = %approval_id,
                        elapsed_ms = elapsed.as_millis(),
                        "Timeout reached, auto-approving (non-critical operation)"
                    );
                    return Ok(true);
                }
            }

            // Poll for status
            match self.poll_approval_status(approval_id).await {
                Ok(Some(approved)) => {
                    tracing::info!(
                        approval_id = %approval_id,
                        approved = approved,
                        elapsed_ms = elapsed.as_millis(),
                        "Approval decision received"
                    );
                    return Ok(approved);
                }
                Ok(None) => {
                    // Still pending, continue polling
                    tracing::trace!(
                        approval_id = %approval_id,
                        elapsed_ms = elapsed.as_millis(),
                        "Approval still pending"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        approval_id = %approval_id,
                        error = %e,
                        "Error polling approval status"
                    );
                    // Continue polling on transient errors
                }
            }

            // Sleep before next poll
            tokio::time::sleep(poll_interval).await;

            // Safety limit: 1 hour max wait for any operation
            if elapsed > Duration::from_secs(3600) {
                tracing::error!(
                    approval_id = %approval_id,
                    "Maximum wait time exceeded (1 hour), rejecting"
                );
                return Ok(false);
            }
        }
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
