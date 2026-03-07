//! Metrics aggregation for the observability system
//!
//! Provides deterministic aggregation of events into metrics summaries.
//! All calculations use fixed formulas with no LLM involvement.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::event::{AgentLifecycleType, Event, ToolStatus};

/// Metrics summary for a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSummary {
    /// Time period start
    pub from: DateTime<Utc>,
    /// Time period end
    pub to: DateTime<Utc>,

    /// Total events
    pub total_events: u64,
    /// Events by type
    pub events_by_type: HashMap<String, u64>,

    /// LLM metrics
    pub llm: LlmMetrics,
    /// Tool metrics
    pub tools: ToolMetrics,
    /// Agent metrics
    pub agents: AgentMetrics,

    /// Per-model breakdown
    pub by_model: HashMap<String, ModelMetrics>,
    /// Per-agent breakdown
    pub by_agent: HashMap<String, AgentBreakdown>,
    /// Per-tool breakdown
    pub by_tool: HashMap<String, ToolBreakdown>,
}

impl Default for MetricsSummary {
    fn default() -> Self {
        Self {
            from: Utc::now(),
            to: Utc::now(),
            total_events: 0,
            events_by_type: HashMap::new(),
            llm: LlmMetrics::default(),
            tools: ToolMetrics::default(),
            agents: AgentMetrics::default(),
            by_model: HashMap::new(),
            by_agent: HashMap::new(),
            by_tool: HashMap::new(),
        }
    }
}

/// LLM-specific metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LlmMetrics {
    /// Total LLM calls
    pub total_calls: u64,
    /// Successful calls
    pub successful_calls: u64,
    /// Failed calls
    pub failed_calls: u64,

    /// Total input tokens
    pub total_input_tokens: u64,
    /// Total output tokens
    pub total_output_tokens: u64,
    /// Total cache read tokens
    pub total_cache_read_tokens: u64,
    /// Total cache write tokens
    pub total_cache_write_tokens: u64,

    /// Total latency in milliseconds
    pub total_latency_ms: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// P50 latency in milliseconds
    pub p50_latency_ms: u64,
    /// P95 latency in milliseconds
    pub p95_latency_ms: u64,
    /// P99 latency in milliseconds
    pub p99_latency_ms: u64,

    /// Total cost in USD
    pub total_cost_usd: f64,
    /// Average cost per call in USD
    pub avg_cost_per_call_usd: f64,

    /// Cache hit rate (cache_read / total_input)
    pub cache_hit_rate: f64,
    /// Success rate
    pub success_rate: f64,
}

/// Per-model metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelMetrics {
    /// Provider name
    pub provider: String,
    /// Model ID
    pub model: String,
    /// Call count
    pub calls: u64,
    /// Success count
    pub successes: u64,
    /// Total input tokens
    pub input_tokens: u64,
    /// Total output tokens
    pub output_tokens: u64,
    /// Total latency
    pub total_latency_ms: u64,
    /// Total cost
    pub total_cost_usd: f64,
}

/// Tool-specific metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolMetrics {
    /// Total tool executions
    pub total_executions: u64,
    /// Successful executions
    pub successful_executions: u64,
    /// Failed executions
    pub failed_executions: u64,
    /// Blocked executions
    pub blocked_executions: u64,
    /// Timed out executions
    pub timeout_executions: u64,
    /// Cancelled executions
    pub cancelled_executions: u64,

    /// Total duration in milliseconds
    pub total_duration_ms: u64,
    /// Average duration in milliseconds
    pub avg_duration_ms: f64,
    /// P50 duration in milliseconds
    pub p50_duration_ms: u64,
    /// P95 duration in milliseconds
    pub p95_duration_ms: u64,

    /// Total input bytes
    pub total_input_bytes: u64,
    /// Total output bytes
    pub total_output_bytes: u64,

    /// Success rate
    pub success_rate: f64,
}

/// Per-tool breakdown
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolBreakdown {
    /// Tool name
    pub tool_name: String,
    /// Execution count
    pub executions: u64,
    /// Success count
    pub successes: u64,
    /// Total duration
    pub total_duration_ms: u64,
    /// Average duration
    pub avg_duration_ms: f64,
}

/// Agent-specific metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentMetrics {
    /// Total agent starts
    pub total_starts: u64,
    /// Total completions
    pub total_completions: u64,
    /// Total errors
    pub total_errors: u64,
    /// Total forks
    pub total_forks: u64,

    /// Average turns per agent
    pub avg_turns: f64,
    /// Average duration per agent
    pub avg_duration_ms: f64,

    /// Completion rate
    pub completion_rate: f64,
}

/// Per-agent breakdown
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentBreakdown {
    /// Agent type
    pub agent_type: String,
    /// Start count
    pub starts: u64,
    /// Completion count
    pub completions: u64,
    /// Error count
    pub errors: u64,
    /// Total duration
    pub total_duration_ms: u64,
    /// Total turns
    pub total_turns: u64,
}

/// Metrics aggregator
///
/// Aggregates events into metrics summaries using deterministic calculations.
pub struct MetricsAggregator {
    latencies: Vec<u64>,
    durations: Vec<u64>,
}

impl MetricsAggregator {
    /// Create a new aggregator
    pub fn new() -> Self {
        Self {
            latencies: Vec::new(),
            durations: Vec::new(),
        }
    }

    /// Aggregate events into a metrics summary
    pub fn aggregate(&mut self, events: &[Event]) -> MetricsSummary {
        if events.is_empty() {
            return MetricsSummary::default();
        }

        self.latencies.clear();
        self.durations.clear();

        let mut summary = MetricsSummary {
            from: events.first().map(|e| e.timestamp()).unwrap_or_else(Utc::now),
            to: events.last().map(|e| e.timestamp()).unwrap_or_else(Utc::now),
            ..Default::default()
        };

        for event in events {
            summary.total_events += 1;
            *summary
                .events_by_type
                .entry(event.event_type().to_string())
                .or_insert(0) += 1;

            match event {
                Event::LlmCall(e) => self.aggregate_llm_call(&mut summary, e),
                Event::ToolExecution(e) => self.aggregate_tool_execution(&mut summary, e),
                Event::AgentLifecycle(e) => self.aggregate_agent_lifecycle(&mut summary, e),
                Event::Span(_) => {} // Spans are for tracing, not metrics
            }
        }

        // Calculate percentiles
        self.calculate_llm_percentiles(&mut summary);
        self.calculate_tool_percentiles(&mut summary);

        // Calculate rates
        self.calculate_rates(&mut summary);

        summary
    }

    fn aggregate_llm_call(
        &mut self,
        summary: &mut MetricsSummary,
        event: &super::event::LlmCallEvent,
    ) {
        let llm = &mut summary.llm;
        llm.total_calls += 1;

        if event.success {
            llm.successful_calls += 1;
        } else {
            llm.failed_calls += 1;
        }

        llm.total_input_tokens += event.input_tokens as u64;
        llm.total_output_tokens += event.output_tokens as u64;
        llm.total_cache_read_tokens += event.cache_read_tokens as u64;
        llm.total_cache_write_tokens += event.cache_write_tokens as u64;
        llm.total_latency_ms += event.latency_ms;
        llm.total_cost_usd += event.cost_usd;

        self.latencies.push(event.latency_ms);

        // Per-model breakdown
        let model_key = format!("{}:{}", event.provider, event.model);
        let model_metrics = summary.by_model.entry(model_key).or_insert_with(|| {
            ModelMetrics {
                provider: event.provider.clone(),
                model: event.model.clone(),
                ..Default::default()
            }
        });
        model_metrics.calls += 1;
        if event.success {
            model_metrics.successes += 1;
        }
        model_metrics.input_tokens += event.input_tokens as u64;
        model_metrics.output_tokens += event.output_tokens as u64;
        model_metrics.total_latency_ms += event.latency_ms;
        model_metrics.total_cost_usd += event.cost_usd;
    }

    fn aggregate_tool_execution(
        &mut self,
        summary: &mut MetricsSummary,
        event: &super::event::ToolExecutionEvent,
    ) {
        let tools = &mut summary.tools;
        tools.total_executions += 1;

        match event.status {
            ToolStatus::Success => tools.successful_executions += 1,
            ToolStatus::Error => tools.failed_executions += 1,
            ToolStatus::Blocked => tools.blocked_executions += 1,
            ToolStatus::Timeout => tools.timeout_executions += 1,
            ToolStatus::Cancelled => tools.cancelled_executions += 1,
        }

        tools.total_duration_ms += event.duration_ms;
        tools.total_input_bytes += event.input_size_bytes as u64;
        tools.total_output_bytes += event.output_size_bytes as u64;

        self.durations.push(event.duration_ms);

        // Per-tool breakdown
        let tool_metrics = summary
            .by_tool
            .entry(event.tool_name.clone())
            .or_insert_with(|| ToolBreakdown {
                tool_name: event.tool_name.clone(),
                ..Default::default()
            });
        tool_metrics.executions += 1;
        if event.status == ToolStatus::Success {
            tool_metrics.successes += 1;
        }
        tool_metrics.total_duration_ms += event.duration_ms;
    }

    fn aggregate_agent_lifecycle(
        &mut self,
        summary: &mut MetricsSummary,
        event: &super::event::AgentLifecycleEvent,
    ) {
        let agents = &mut summary.agents;

        match event.lifecycle_type {
            AgentLifecycleType::Start => agents.total_starts += 1,
            AgentLifecycleType::Complete => agents.total_completions += 1,
            AgentLifecycleType::Error => agents.total_errors += 1,
            AgentLifecycleType::Fork => agents.total_forks += 1,
            _ => {}
        }

        // Per-agent breakdown
        let agent_metrics = summary
            .by_agent
            .entry(event.agent_type.clone())
            .or_insert_with(|| AgentBreakdown {
                agent_type: event.agent_type.clone(),
                ..Default::default()
            });

        match event.lifecycle_type {
            AgentLifecycleType::Start => agent_metrics.starts += 1,
            AgentLifecycleType::Complete => {
                agent_metrics.completions += 1;
                if let Some(duration) = event.duration_ms {
                    agent_metrics.total_duration_ms += duration;
                }
                if let Some(turns) = event.turn_count {
                    agent_metrics.total_turns += turns as u64;
                }
            }
            AgentLifecycleType::Error => agent_metrics.errors += 1,
            _ => {}
        }
    }

    fn calculate_llm_percentiles(&mut self, summary: &mut MetricsSummary) {
        if self.latencies.is_empty() {
            return;
        }

        self.latencies.sort_unstable();

        let llm = &mut summary.llm;
        llm.avg_latency_ms = if llm.total_calls > 0 {
            llm.total_latency_ms as f64 / llm.total_calls as f64
        } else {
            0.0
        };

        llm.p50_latency_ms = percentile(&self.latencies, 50);
        llm.p95_latency_ms = percentile(&self.latencies, 95);
        llm.p99_latency_ms = percentile(&self.latencies, 99);
    }

    fn calculate_tool_percentiles(&mut self, summary: &mut MetricsSummary) {
        if self.durations.is_empty() {
            return;
        }

        self.durations.sort_unstable();

        let tools = &mut summary.tools;
        tools.avg_duration_ms = if tools.total_executions > 0 {
            tools.total_duration_ms as f64 / tools.total_executions as f64
        } else {
            0.0
        };

        tools.p50_duration_ms = percentile(&self.durations, 50);
        tools.p95_duration_ms = percentile(&self.durations, 95);

        // Calculate per-tool averages
        for breakdown in summary.by_tool.values_mut() {
            breakdown.avg_duration_ms = if breakdown.executions > 0 {
                breakdown.total_duration_ms as f64 / breakdown.executions as f64
            } else {
                0.0
            };
        }
    }

    fn calculate_rates(&self, summary: &mut MetricsSummary) {
        // LLM rates
        let llm = &mut summary.llm;
        llm.success_rate = if llm.total_calls > 0 {
            llm.successful_calls as f64 / llm.total_calls as f64
        } else {
            0.0
        };

        llm.avg_cost_per_call_usd = if llm.total_calls > 0 {
            llm.total_cost_usd / llm.total_calls as f64
        } else {
            0.0
        };

        let total_input = llm.total_input_tokens + llm.total_cache_write_tokens;
        llm.cache_hit_rate = if total_input > 0 {
            llm.total_cache_read_tokens as f64 / total_input as f64
        } else {
            0.0
        };

        // Tool rates
        let tools = &mut summary.tools;
        tools.success_rate = if tools.total_executions > 0 {
            tools.successful_executions as f64 / tools.total_executions as f64
        } else {
            0.0
        };

        // Agent rates
        let agents = &mut summary.agents;
        agents.completion_rate = if agents.total_starts > 0 {
            agents.total_completions as f64 / agents.total_starts as f64
        } else {
            0.0
        };

        // Calculate agent averages
        let mut total_duration = 0u64;
        let mut total_turns = 0u64;
        let mut count = 0u64;
        for breakdown in summary.by_agent.values() {
            total_duration += breakdown.total_duration_ms;
            total_turns += breakdown.total_turns;
            count += breakdown.completions;
        }

        agents.avg_duration_ms = if count > 0 {
            total_duration as f64 / count as f64
        } else {
            0.0
        };

        agents.avg_turns = if count > 0 {
            total_turns as f64 / count as f64
        } else {
            0.0
        };
    }
}

impl Default for MetricsAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate percentile from sorted values
fn percentile(sorted: &[u64], p: u32) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = (sorted.len() as f64 * p as f64 / 100.0).ceil() as usize;
    sorted.get(idx.saturating_sub(1)).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observability::event::{LlmCallEvent, ToolExecutionEvent};

    #[test]
    fn test_empty_aggregation() {
        let mut aggregator = MetricsAggregator::new();
        let summary = aggregator.aggregate(&[]);
        assert_eq!(summary.total_events, 0);
    }

    #[test]
    fn test_llm_aggregation() {
        let mut aggregator = MetricsAggregator::new();

        let events = vec![
            Event::LlmCall(LlmCallEvent {
                provider: "anthropic".into(),
                model: "claude-opus-4-5".into(),
                input_tokens: 1000,
                output_tokens: 500,
                latency_ms: 2000,
                cost_usd: 0.05,
                success: true,
                ..Default::default()
            }),
            Event::LlmCall(LlmCallEvent {
                provider: "anthropic".into(),
                model: "claude-opus-4-5".into(),
                input_tokens: 500,
                output_tokens: 200,
                latency_ms: 1000,
                cost_usd: 0.02,
                success: true,
                ..Default::default()
            }),
        ];

        let summary = aggregator.aggregate(&events);

        assert_eq!(summary.total_events, 2);
        assert_eq!(summary.llm.total_calls, 2);
        assert_eq!(summary.llm.total_input_tokens, 1500);
        assert_eq!(summary.llm.total_output_tokens, 700);
        assert_eq!(summary.llm.total_cost_usd, 0.07);
        assert_eq!(summary.llm.success_rate, 1.0);
    }

    #[test]
    fn test_tool_aggregation() {
        let mut aggregator = MetricsAggregator::new();

        let events = vec![
            Event::ToolExecution(ToolExecutionEvent {
                tool_name: "Read".into(),
                duration_ms: 50,
                status: ToolStatus::Success,
                ..Default::default()
            }),
            Event::ToolExecution(ToolExecutionEvent {
                tool_name: "Write".into(),
                duration_ms: 100,
                status: ToolStatus::Success,
                ..Default::default()
            }),
            Event::ToolExecution(ToolExecutionEvent {
                tool_name: "Bash".into(),
                duration_ms: 500,
                status: ToolStatus::Error,
                ..Default::default()
            }),
        ];

        let summary = aggregator.aggregate(&events);

        assert_eq!(summary.tools.total_executions, 3);
        assert_eq!(summary.tools.successful_executions, 2);
        assert_eq!(summary.tools.failed_executions, 1);
        assert!(summary.tools.success_rate > 0.6 && summary.tools.success_rate < 0.7);
    }

    #[test]
    fn test_percentile_calculation() {
        let sorted = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        assert_eq!(percentile(&sorted, 50), 5);
        assert_eq!(percentile(&sorted, 90), 9);
        assert_eq!(percentile(&sorted, 100), 10);
    }
}
