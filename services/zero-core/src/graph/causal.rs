//! Causal Graph - Decision → Action → Outcome tracking
//!
//! Provides causal chain analysis for agent decisions:
//! - Record decisions with reasoning and confidence
//! - Track actions taken as a result of decisions
//! - Capture outcomes (success/partial/failure)
//! - Query causal chains for analysis

use serde::{Deserialize, Serialize};

use super::algorithms::GraphAlgorithms;
use super::engine::{EdgeData, EdgeRelationship, GraphEngine, NodeData, NodeId};

/// Outcome status for an action result
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeStatus {
    Success,
    Partial,
    Failure,
}

/// Decision node - represents an agent's decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionNode {
    pub id: NodeId,
    pub session_id: String,
    pub agent_id: String,
    pub prompt: String,
    pub reasoning: String,
    pub confidence: f64,
    pub timestamp: String,
    pub context: Option<serde_json::Value>,
}

/// Action node - represents an action taken from a decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionNode {
    pub id: NodeId,
    pub decision_id: NodeId,
    pub action_type: String,
    pub description: String,
    pub input: Option<serde_json::Value>,
    pub output: Option<serde_json::Value>,
    pub timestamp: String,
    pub duration: Option<u64>,
}

/// Outcome node - represents the result of an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutcomeNode {
    pub id: NodeId,
    pub action_id: NodeId,
    pub status: OutcomeStatus,
    pub description: String,
    pub metrics: Option<serde_json::Value>,
    pub feedback: Option<String>,
    pub timestamp: String,
}

/// Edge between causal nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalEdge {
    pub id: String,
    pub source: NodeId,
    pub target: NodeId,
    pub relationship: String,
    pub weight: f64,
    pub metadata: Option<serde_json::Value>,
}

/// A complete causal chain from decision to outcome(s)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalChain {
    pub decision: DecisionNode,
    pub actions: Vec<ActionNode>,
    pub outcomes: Vec<OutcomeNode>,
    pub edges: Vec<CausalEdge>,
}

/// Query parameters for causal graph search
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CausalQuery {
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub action_type: Option<String>,
    pub status: Option<OutcomeStatus>,
    pub min_confidence: Option<f64>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub limit: Option<usize>,
}

/// Statistics about the causal graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalStats {
    pub total_decisions: usize,
    pub total_actions: usize,
    pub total_outcomes: usize,
    pub total_edges: usize,
    pub success_rate: f64,
    pub avg_confidence: f64,
    pub top_agents: Vec<AgentStat>,
    pub action_type_distribution: std::collections::HashMap<String, usize>,
}

/// Per-agent statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStat {
    pub agent_id: String,
    pub decision_count: usize,
    pub success_rate: f64,
}

/// Node type constants
const NODE_TYPE_DECISION: &str = "decision";
const NODE_TYPE_ACTION: &str = "action";
const NODE_TYPE_OUTCOME: &str = "outcome";

/// Causal Graph implementation
#[derive(Debug)]
pub struct CausalGraph {
    engine: GraphEngine,
    project_id: String,
    created_at: i64,
    updated_at: i64,
}

impl CausalGraph {
    /// Create a new causal graph for a project
    pub fn new(project_id: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            engine: GraphEngine::new(),
            project_id: project_id.into(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Get the project ID
    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    // ============================================================================
    // ID Generators
    // ============================================================================

    fn generate_id(prefix: &str) -> String {
        let timestamp = chrono::Utc::now().timestamp_millis();
        let random: u32 = rand::random();
        format!("{}_{:x}_{:x}", prefix, timestamp, random)
    }

    // ============================================================================
    // Record Operations
    // ============================================================================

    /// Record a decision made by an agent
    pub fn record_decision(
        &mut self,
        session_id: impl Into<String>,
        agent_id: impl Into<String>,
        prompt: impl Into<String>,
        reasoning: impl Into<String>,
        confidence: f64,
        context: Option<serde_json::Value>,
    ) -> DecisionNode {
        let id = Self::generate_id("dec");
        let timestamp = chrono::Utc::now().to_rfc3339();

        let node = DecisionNode {
            id: id.clone(),
            session_id: session_id.into(),
            agent_id: agent_id.into(),
            prompt: prompt.into(),
            reasoning: reasoning.into(),
            confidence: confidence.clamp(0.0, 1.0),
            timestamp,
            context,
        };

        let node_data = NodeData::new(&id, NODE_TYPE_DECISION, &node);
        self.engine.add_node(node_data);
        self.updated_at = chrono::Utc::now().timestamp_millis();

        node
    }

    /// Record an action taken as a result of a decision
    pub fn record_action(
        &mut self,
        decision_id: &str,
        action_type: impl Into<String>,
        description: impl Into<String>,
        input: Option<serde_json::Value>,
        output: Option<serde_json::Value>,
        duration: Option<u64>,
    ) -> anyhow::Result<ActionNode> {
        // Verify decision exists
        let decision = self.get_decision(decision_id)
            .ok_or_else(|| anyhow::anyhow!("Decision '{}' not found", decision_id))?;

        let id = Self::generate_id("act");
        let timestamp = chrono::Utc::now().to_rfc3339();

        let node = ActionNode {
            id: id.clone(),
            decision_id: decision_id.to_string(),
            action_type: action_type.into(),
            description: description.into(),
            input,
            output,
            timestamp,
            duration,
        };

        let node_data = NodeData::new(&id, NODE_TYPE_ACTION, &node);
        self.engine.add_node(node_data);

        // Create edge from decision to action
        let edge = EdgeData::new(decision_id, &id, EdgeRelationship::Causes)
            .with_weight(decision.confidence);
        self.engine.add_edge(edge);
        self.updated_at = chrono::Utc::now().timestamp_millis();

        Ok(node)
    }

    /// Record the outcome of an action
    pub fn record_outcome(
        &mut self,
        action_id: &str,
        status: OutcomeStatus,
        description: impl Into<String>,
        metrics: Option<serde_json::Value>,
        feedback: Option<String>,
    ) -> anyhow::Result<OutcomeNode> {
        // Verify action exists
        if self.get_action(action_id).is_none() {
            anyhow::bail!("Action '{}' not found", action_id);
        }

        let id = Self::generate_id("out");
        let timestamp = chrono::Utc::now().to_rfc3339();

        let node = OutcomeNode {
            id: id.clone(),
            action_id: action_id.to_string(),
            status,
            description: description.into(),
            metrics: metrics.clone(),
            feedback,
            timestamp,
        };

        let node_data = NodeData::new(&id, NODE_TYPE_OUTCOME, &node);
        self.engine.add_node(node_data);

        // Determine edge weight based on outcome status
        let weight = match status {
            OutcomeStatus::Success => 1.0,
            OutcomeStatus::Partial => 0.5,
            OutcomeStatus::Failure => 0.0,
        };

        // Create edge from action to outcome
        let mut edge = EdgeData::new(action_id, &id, EdgeRelationship::ResultsIn)
            .with_weight(weight);
        if let Some(m) = metrics {
            edge = edge.with_metadata(&m);
        }
        self.engine.add_edge(edge);
        self.updated_at = chrono::Utc::now().timestamp_millis();

        Ok(node)
    }

    // ============================================================================
    // Query Operations
    // ============================================================================

    /// Get a decision by ID
    pub fn get_decision(&self, id: &str) -> Option<DecisionNode> {
        self.engine
            .get_node(id)
            .filter(|n| n.node_type == NODE_TYPE_DECISION)
            .and_then(|n| n.get_payload())
    }

    /// Get an action by ID
    pub fn get_action(&self, id: &str) -> Option<ActionNode> {
        self.engine
            .get_node(id)
            .filter(|n| n.node_type == NODE_TYPE_ACTION)
            .and_then(|n| n.get_payload())
    }

    /// Get an outcome by ID
    pub fn get_outcome(&self, id: &str) -> Option<OutcomeNode> {
        self.engine
            .get_node(id)
            .filter(|n| n.node_type == NODE_TYPE_OUTCOME)
            .and_then(|n| n.get_payload())
    }

    /// Get all decisions
    pub fn get_decisions(&self) -> Vec<DecisionNode> {
        self.engine
            .nodes_by_type(NODE_TYPE_DECISION)
            .into_iter()
            .filter_map(|n| n.get_payload())
            .collect()
    }

    /// Get all actions
    pub fn get_actions(&self) -> Vec<ActionNode> {
        self.engine
            .nodes_by_type(NODE_TYPE_ACTION)
            .into_iter()
            .filter_map(|n| n.get_payload())
            .collect()
    }

    /// Get all outcomes
    pub fn get_outcomes(&self) -> Vec<OutcomeNode> {
        self.engine
            .nodes_by_type(NODE_TYPE_OUTCOME)
            .into_iter()
            .filter_map(|n| n.get_payload())
            .collect()
    }

    /// Get decisions by agent
    pub fn get_decisions_by_agent(&self, agent_id: &str) -> Vec<DecisionNode> {
        self.get_decisions()
            .into_iter()
            .filter(|d| d.agent_id == agent_id)
            .collect()
    }

    /// Get a complete causal chain starting from a decision
    pub fn get_causal_chain(&self, decision_id: &str) -> Option<CausalChain> {
        let decision = self.get_decision(decision_id)?;

        let mut actions = Vec::new();
        let mut outcomes = Vec::new();
        let mut edges = Vec::new();

        // Find all actions linked to this decision
        let action_ids = self.engine.get_successors(decision_id);
        for action_id in action_ids {
            if let Some(action) = self.get_action(&action_id) {
                // Record edge
                let edge_data = self.engine.get_outgoing_edges(decision_id)
                    .into_iter()
                    .find(|e| e.target == action_id);
                if let Some(e) = edge_data {
                    edges.push(CausalEdge {
                        id: e.id.clone(),
                        source: e.source.clone(),
                        target: e.target.clone(),
                        relationship: format!("{:?}", e.relationship),
                        weight: e.weight,
                        metadata: e.metadata.clone(),
                    });
                }

                // Find outcomes linked to this action
                let outcome_ids = self.engine.get_successors(&action_id);
                for outcome_id in outcome_ids {
                    if let Some(outcome) = self.get_outcome(&outcome_id) {
                        // Record edge
                        let edge_data = self.engine.get_outgoing_edges(&action_id)
                            .into_iter()
                            .find(|e| e.target == outcome_id);
                        if let Some(e) = edge_data {
                            edges.push(CausalEdge {
                                id: e.id.clone(),
                                source: e.source.clone(),
                                target: e.target.clone(),
                                relationship: format!("{:?}", e.relationship),
                                weight: e.weight,
                                metadata: e.metadata.clone(),
                            });
                        }
                        outcomes.push(outcome);
                    }
                }

                actions.push(action);
            }
        }

        Some(CausalChain {
            decision,
            actions,
            outcomes,
            edges,
        })
    }

    /// Get all causal chains for a session
    pub fn get_causal_chains_for_session(&self, session_id: &str) -> Vec<CausalChain> {
        self.get_decisions()
            .into_iter()
            .filter(|d| d.session_id == session_id)
            .filter_map(|d| self.get_causal_chain(&d.id))
            .collect()
    }

    /// Query the causal graph with filters
    pub fn query(&self, query: &CausalQuery) -> Vec<CausalChain> {
        let limit = query.limit.unwrap_or(100);
        let mut decisions = self.get_decisions();

        // Apply filters
        if let Some(ref agent_id) = query.agent_id {
            decisions.retain(|d| d.agent_id == *agent_id);
        }
        if let Some(ref session_id) = query.session_id {
            decisions.retain(|d| d.session_id == *session_id);
        }
        if let Some(min_confidence) = query.min_confidence {
            decisions.retain(|d| d.confidence >= min_confidence);
        }
        if let Some(ref date_from) = query.date_from {
            decisions.retain(|d| d.timestamp >= *date_from);
        }
        if let Some(ref date_to) = query.date_to {
            decisions.retain(|d| d.timestamp <= *date_to);
        }

        // Get chains and apply action/outcome filters
        let mut chains: Vec<CausalChain> = decisions
            .into_iter()
            .take(limit)
            .filter_map(|d| self.get_causal_chain(&d.id))
            .collect();

        // Apply action type filter
        if let Some(ref action_type) = query.action_type {
            for chain in &mut chains {
                chain.actions.retain(|a| a.action_type == *action_type);
            }
        }

        // Apply status filter
        if let Some(status) = query.status {
            for chain in &mut chains {
                chain.outcomes.retain(|o| o.status == status);
            }
        }

        // Remove chains with no matching actions/outcomes
        chains.retain(|c| !c.actions.is_empty() || !c.outcomes.is_empty());

        chains
    }

    /// Calculate success rate for an agent or overall
    pub fn get_success_rate(&self, agent_id: Option<&str>) -> f64 {
        let decisions = match agent_id {
            Some(id) => self.get_decisions_by_agent(id),
            None => self.get_decisions(),
        };

        if decisions.is_empty() {
            return 0.0;
        }

        let outcomes: Vec<OutcomeNode> = decisions
            .iter()
            .flat_map(|d| {
                self.engine
                    .get_successors(&d.id)
                    .iter()
                    .flat_map(|action_id| self.engine.get_successors(action_id))
                    .filter_map(|outcome_id| self.get_outcome(&outcome_id))
                    .collect::<Vec<_>>()
            })
            .collect();

        if outcomes.is_empty() {
            return 0.0;
        }

        let success_count = outcomes
            .iter()
            .filter(|o| o.status == OutcomeStatus::Success)
            .count();

        success_count as f64 / outcomes.len() as f64
    }

    // ============================================================================
    // Statistics
    // ============================================================================

    /// Get comprehensive statistics about the causal graph
    pub fn get_stats(&self) -> CausalStats {
        let decisions = self.get_decisions();
        let actions = self.get_actions();
        let outcomes = self.get_outcomes();

        // Calculate success rate
        let success_count = outcomes
            .iter()
            .filter(|o| o.status == OutcomeStatus::Success)
            .count();
        let success_rate = if outcomes.is_empty() {
            0.0
        } else {
            success_count as f64 / outcomes.len() as f64
        };

        // Calculate average confidence
        let avg_confidence = if decisions.is_empty() {
            0.0
        } else {
            decisions.iter().map(|d| d.confidence).sum::<f64>() / decisions.len() as f64
        };

        // Group by agent
        let mut agent_stats: std::collections::HashMap<String, (usize, usize)> =
            std::collections::HashMap::new();
        for decision in &decisions {
            let entry = agent_stats.entry(decision.agent_id.clone()).or_insert((0, 0));
            entry.0 += 1;

            // Count successes for this decision's chain
            let chain_outcomes: Vec<_> = self
                .engine
                .get_successors(&decision.id)
                .iter()
                .flat_map(|action_id| self.engine.get_successors(action_id))
                .filter_map(|outcome_id| self.get_outcome(&outcome_id))
                .collect();
            entry.1 += chain_outcomes
                .iter()
                .filter(|o| o.status == OutcomeStatus::Success)
                .count();
        }

        let mut top_agents: Vec<AgentStat> = agent_stats
            .into_iter()
            .map(|(agent_id, (count, successes))| AgentStat {
                agent_id,
                decision_count: count,
                success_rate: if count > 0 {
                    successes as f64 / count as f64
                } else {
                    0.0
                },
            })
            .collect();
        top_agents.sort_by(|a, b| b.decision_count.cmp(&a.decision_count));
        top_agents.truncate(10);

        // Action type distribution
        let mut action_type_distribution: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();
        for action in &actions {
            *action_type_distribution
                .entry(action.action_type.clone())
                .or_insert(0) += 1;
        }

        CausalStats {
            total_decisions: decisions.len(),
            total_actions: actions.len(),
            total_outcomes: outcomes.len(),
            total_edges: self.engine.edge_count(),
            success_rate,
            avg_confidence,
            top_agents,
            action_type_distribution,
        }
    }

    // ============================================================================
    // Graph Operations
    // ============================================================================

    /// Get all nodes reachable from a decision
    pub fn get_reachable_from_decision(&self, decision_id: &str, max_depth: usize) -> Vec<NodeId> {
        GraphAlgorithms::get_reachable(&self.engine, decision_id, max_depth)
    }

    /// Check if there are any cycles in the causal graph (should be none)
    pub fn has_cycles(&self) -> bool {
        GraphAlgorithms::has_cycles(&self.engine)
    }

    // ============================================================================
    // Serialization
    // ============================================================================

    /// Serialize the graph to JSON
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "project_id": self.project_id,
            "created_at": self.created_at,
            "updated_at": self.updated_at,
            "graph": self.engine.to_json()
        })
    }

    /// Deserialize from JSON
    pub fn from_json(value: &serde_json::Value) -> anyhow::Result<Self> {
        let project_id = value
            .get("project_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("Missing project_id"))?
            .to_string();

        let created_at = value
            .get("created_at")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

        let updated_at = value
            .get("updated_at")
            .and_then(|v| v.as_i64())
            .unwrap_or_else(|| chrono::Utc::now().timestamp_millis());

        let graph_value = value
            .get("graph")
            .ok_or_else(|| anyhow::anyhow!("Missing graph"))?;

        let engine = GraphEngine::from_json(graph_value)?;

        Ok(Self {
            engine,
            project_id,
            created_at,
            updated_at,
        })
    }

    /// Access the underlying engine for advanced operations
    pub fn engine(&self) -> &GraphEngine {
        &self.engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_decision() {
        let mut graph = CausalGraph::new("test-project");

        let decision = graph.record_decision(
            "session-1",
            "build-agent",
            "Fix the bug in auth",
            "The bug is caused by missing null check",
            0.85,
            None,
        );

        assert!(decision.id.starts_with("dec_"));
        assert_eq!(decision.agent_id, "build-agent");
        assert_eq!(decision.confidence, 0.85);
    }

    #[test]
    fn test_record_action() {
        let mut graph = CausalGraph::new("test-project");

        let decision = graph.record_decision(
            "session-1",
            "build-agent",
            "Fix the bug",
            "Need to add null check",
            0.9,
            None,
        );

        let action = graph
            .record_action(
                &decision.id,
                "edit",
                "Add null check to auth.ts",
                None,
                None,
                Some(150),
            )
            .unwrap();

        assert!(action.id.starts_with("act_"));
        assert_eq!(action.decision_id, decision.id);
        assert_eq!(action.action_type, "edit");
    }

    #[test]
    fn test_record_outcome() {
        let mut graph = CausalGraph::new("test-project");

        let decision = graph.record_decision("s1", "agent", "Task", "Reason", 0.9, None);
        let action = graph
            .record_action(&decision.id, "edit", "Made change", None, None, None)
            .unwrap();
        let outcome = graph
            .record_outcome(
                &action.id,
                OutcomeStatus::Success,
                "Change applied successfully",
                None,
                None,
            )
            .unwrap();

        assert!(outcome.id.starts_with("out_"));
        assert_eq!(outcome.status, OutcomeStatus::Success);
    }

    #[test]
    fn test_get_causal_chain() {
        let mut graph = CausalGraph::new("test-project");

        let decision = graph.record_decision("s1", "agent", "Task", "Reason", 0.9, None);
        let action = graph
            .record_action(&decision.id, "edit", "Made change", None, None, None)
            .unwrap();
        let _ = graph.record_outcome(
            &action.id,
            OutcomeStatus::Success,
            "Done",
            None,
            None,
        );

        let chain = graph.get_causal_chain(&decision.id).unwrap();

        assert_eq!(chain.decision.id, decision.id);
        assert_eq!(chain.actions.len(), 1);
        assert_eq!(chain.outcomes.len(), 1);
        assert_eq!(chain.edges.len(), 2);
    }

    #[test]
    fn test_stats() {
        let mut graph = CausalGraph::new("test-project");

        for i in 0..5 {
            let decision = graph.record_decision(
                "s1",
                if i % 2 == 0 { "agent-a" } else { "agent-b" },
                format!("Task {}", i),
                "Reason",
                0.8,
                None,
            );
            let action = graph
                .record_action(&decision.id, "edit", "Change", None, None, None)
                .unwrap();
            let status = if i < 3 {
                OutcomeStatus::Success
            } else {
                OutcomeStatus::Failure
            };
            let _ = graph.record_outcome(&action.id, status, "Result", None, None);
        }

        let stats = graph.get_stats();

        assert_eq!(stats.total_decisions, 5);
        assert_eq!(stats.total_actions, 5);
        assert_eq!(stats.total_outcomes, 5);
        assert!((stats.success_rate - 0.6).abs() < 0.01); // 3/5 = 0.6
    }
}
