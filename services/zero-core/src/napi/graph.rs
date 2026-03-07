//! NAPI bindings for graph module
//!
//! Provides JavaScript/TypeScript bindings for:
//! - GraphEngine: Core graph operations
//! - CausalGraph: Decision → Action → Outcome tracking
//! - CallGraph: Function call relationships
//! - SemanticGraph: Code entity relationships
//! - Graph algorithms: BFS, DFS, cycle detection, topological sort

use std::sync::{Arc, Mutex};

use napi::bindgen_prelude::*;
use napi_derive::napi;

use crate::graph::{
    algorithms::GraphAlgorithms,
    call::{CallGraph as RustCallGraph, CallNode as RustCallNode, CallableKind, RecursionType},
    causal::{
        ActionNode as RustActionNode, AgentInsights as RustAgentInsights,
        CausalChain as RustCausalChain, CausalGraph as RustCausalGraph,
        CausalPattern as RustCausalPattern, CausalQuery as RustCausalQuery,
        CausalStats as RustCausalStats, DecisionNode as RustDecisionNode,
        OutcomeNode as RustOutcomeNode, OutcomeStatus as RustOutcomeStatus,
        SimilarDecision as RustSimilarDecision, TrendAnalysis as RustTrendAnalysis,
    },
    engine::{EdgeData, EdgeRelationship, GraphEngine as RustGraphEngine, NodeData},
    semantic::{
        SemanticGraph as RustSemanticGraph, SemanticNode as RustSemanticNode,
        SemanticNodeType,
    },
};

// ============================================================================
// Common Types
// ============================================================================

/// Node data for NAPI
#[napi(object)]
pub struct NapiNodeData {
    pub id: String,
    pub node_type: String,
    pub payload: String, // JSON string
    pub created_at: i64,
    pub updated_at: i64,
}

impl From<&NodeData> for NapiNodeData {
    fn from(node: &NodeData) -> Self {
        Self {
            id: node.id.clone(),
            node_type: node.node_type.clone(),
            payload: node.payload.to_string(),
            created_at: node.created_at,
            updated_at: node.updated_at,
        }
    }
}

/// Path finding result
#[napi(object)]
pub struct NapiPathResult {
    pub found: bool,
    pub path: Vec<String>,
    pub nodes_visited: u32,
}

/// Cycle detection result
#[napi(object)]
pub struct NapiCycleResult {
    pub has_cycles: bool,
    pub cycles: Vec<Vec<String>>,
}

// ============================================================================
// GraphEngine NAPI Handle
// ============================================================================

/// Thread-safe wrapper for GraphEngine
#[napi]
pub struct GraphEngineHandle {
    inner: Arc<Mutex<RustGraphEngine>>,
}

#[napi]
impl GraphEngineHandle {
    /// Create a new empty graph engine
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RustGraphEngine::new())),
        }
    }

    /// Add a node to the graph
    #[napi]
    pub fn add_node(&self, id: String, node_type: String, payload: String) -> Result<String> {
        let mut engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let payload_value: serde_json::Value =
            serde_json::from_str(&payload).unwrap_or(serde_json::Value::Null);
        let node = NodeData::new(&id, &node_type, &payload_value);
        Ok(engine.add_node(node))
    }

    /// Get a node by ID
    #[napi]
    pub fn get_node(&self, id: String) -> Result<Option<NapiNodeData>> {
        let engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(engine.get_node(&id).map(|n| n.into()))
    }

    /// Check if a node exists
    #[napi]
    pub fn contains_node(&self, id: String) -> Result<bool> {
        let engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(engine.contains_node(&id))
    }

    /// Add an edge between two nodes
    #[napi]
    pub fn add_edge(&self, source: String, target: String, relationship: String) -> Result<Option<String>> {
        let mut engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let rel = match relationship.as_str() {
            "causes" => EdgeRelationship::Causes,
            "results_in" => EdgeRelationship::ResultsIn,
            "calls" => EdgeRelationship::Calls,
            "imports" => EdgeRelationship::Imports,
            "exports" => EdgeRelationship::Exports,
            "extends" => EdgeRelationship::Extends,
            "implements" => EdgeRelationship::Implements,
            "references" => EdgeRelationship::References,
            "contains" => EdgeRelationship::Contains,
            _ => EdgeRelationship::Related,
        };
        let edge = EdgeData::new(&source, &target, rel);
        Ok(engine.add_edge(edge))
    }

    /// Get successors of a node
    #[napi]
    pub fn get_successors(&self, id: String) -> Result<Vec<String>> {
        let engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(engine.get_successors(&id))
    }

    /// Get predecessors of a node
    #[napi]
    pub fn get_predecessors(&self, id: String) -> Result<Vec<String>> {
        let engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(engine.get_predecessors(&id))
    }

    /// Get node count
    #[napi]
    pub fn node_count(&self) -> Result<u32> {
        let engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(engine.node_count() as u32)
    }

    /// Get edge count
    #[napi]
    pub fn edge_count(&self) -> Result<u32> {
        let engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(engine.edge_count() as u32)
    }

    /// BFS traversal from a starting node
    #[napi]
    pub fn bfs(&self, start: String) -> Result<Vec<String>> {
        let engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(GraphAlgorithms::bfs(&engine, &start))
    }

    /// DFS traversal from a starting node
    #[napi]
    pub fn dfs(&self, start: String) -> Result<Vec<String>> {
        let engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(GraphAlgorithms::dfs(&engine, &start))
    }

    /// Find path between two nodes
    #[napi]
    pub fn find_path(&self, from: String, to: String) -> Result<NapiPathResult> {
        let engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let result = GraphAlgorithms::find_path(&engine, &from, &to);
        Ok(NapiPathResult {
            found: result.found,
            path: result.path,
            nodes_visited: result.nodes_visited as u32,
        })
    }

    /// Check if graph has cycles
    #[napi]
    pub fn has_cycles(&self) -> Result<bool> {
        let engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(GraphAlgorithms::has_cycles(&engine))
    }

    /// Detect all cycles
    #[napi]
    pub fn detect_cycles(&self) -> Result<NapiCycleResult> {
        let engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let result = GraphAlgorithms::detect_cycles(&engine);
        Ok(NapiCycleResult {
            has_cycles: result.has_cycles,
            cycles: result.cycles,
        })
    }

    /// Topological sort (returns empty if graph has cycles)
    #[napi]
    pub fn topological_sort(&self) -> Result<Vec<String>> {
        let engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(GraphAlgorithms::topological_sort(&engine).unwrap_or_default())
    }

    /// Get reachable nodes within depth limit
    #[napi]
    pub fn get_reachable(&self, start: String, max_depth: u32) -> Result<Vec<String>> {
        let engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(GraphAlgorithms::get_reachable(&engine, &start, max_depth as usize))
    }

    /// Serialize to JSON
    #[napi]
    pub fn to_json(&self) -> Result<String> {
        let engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(engine.to_json().to_string())
    }

    /// Deserialize from JSON
    #[napi(factory)]
    pub fn from_json(json: String) -> Result<Self> {
        let value: serde_json::Value =
            serde_json::from_str(&json).map_err(|e| Error::from_reason(e.to_string()))?;
        let engine =
            RustGraphEngine::from_json(&value).map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(Mutex::new(engine)),
        })
    }

    /// Clear all nodes and edges
    #[napi]
    pub fn clear(&self) -> Result<()> {
        let mut engine = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        engine.clear();
        Ok(())
    }
}

// ============================================================================
// CausalGraph NAPI Types
// ============================================================================

/// Decision node for NAPI
#[napi(object)]
pub struct NapiDecisionNode {
    pub id: String,
    pub session_id: String,
    pub agent_id: String,
    pub prompt: String,
    pub reasoning: String,
    pub confidence: f64,
    pub timestamp: String,
    pub context: Option<String>, // JSON string
}

impl From<RustDecisionNode> for NapiDecisionNode {
    fn from(node: RustDecisionNode) -> Self {
        Self {
            id: node.id,
            session_id: node.session_id,
            agent_id: node.agent_id,
            prompt: node.prompt,
            reasoning: node.reasoning,
            confidence: node.confidence,
            timestamp: node.timestamp,
            context: node.context.map(|c| c.to_string()),
        }
    }
}

/// Action node for NAPI
#[napi(object)]
pub struct NapiActionNode {
    pub id: String,
    pub decision_id: String,
    pub action_type: String,
    pub description: String,
    pub input: Option<String>, // JSON string
    pub output: Option<String>, // JSON string
    pub timestamp: String,
    pub duration: Option<u32>,
}

impl From<RustActionNode> for NapiActionNode {
    fn from(node: RustActionNode) -> Self {
        Self {
            id: node.id,
            decision_id: node.decision_id,
            action_type: node.action_type,
            description: node.description,
            input: node.input.map(|v| v.to_string()),
            output: node.output.map(|v| v.to_string()),
            timestamp: node.timestamp,
            duration: node.duration.map(|d| d as u32),
        }
    }
}

/// Outcome node for NAPI
#[napi(object)]
pub struct NapiOutcomeNode {
    pub id: String,
    pub action_id: String,
    pub status: String, // "success" | "partial" | "failure"
    pub description: String,
    pub metrics: Option<String>, // JSON string
    pub feedback: Option<String>,
    pub timestamp: String,
}

impl From<RustOutcomeNode> for NapiOutcomeNode {
    fn from(node: RustOutcomeNode) -> Self {
        Self {
            id: node.id,
            action_id: node.action_id,
            status: match node.status {
                RustOutcomeStatus::Success => "success".to_string(),
                RustOutcomeStatus::Partial => "partial".to_string(),
                RustOutcomeStatus::Failure => "failure".to_string(),
            },
            description: node.description,
            metrics: node.metrics.map(|v| v.to_string()),
            feedback: node.feedback,
            timestamp: node.timestamp,
        }
    }
}

/// Causal chain for NAPI
#[napi(object)]
pub struct NapiCausalChain {
    pub decision: NapiDecisionNode,
    pub actions: Vec<NapiActionNode>,
    pub outcomes: Vec<NapiOutcomeNode>,
}

impl From<RustCausalChain> for NapiCausalChain {
    fn from(chain: RustCausalChain) -> Self {
        Self {
            decision: chain.decision.into(),
            actions: chain.actions.into_iter().map(|a| a.into()).collect(),
            outcomes: chain.outcomes.into_iter().map(|o| o.into()).collect(),
        }
    }
}

/// Causal query for NAPI
#[napi(object)]
pub struct NapiCausalQuery {
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub action_type: Option<String>,
    pub status: Option<String>,
    pub min_confidence: Option<f64>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub limit: Option<u32>,
}

impl From<NapiCausalQuery> for RustCausalQuery {
    fn from(query: NapiCausalQuery) -> Self {
        Self {
            agent_id: query.agent_id,
            session_id: query.session_id,
            action_type: query.action_type,
            status: query.status.and_then(|s| match s.as_str() {
                "success" => Some(RustOutcomeStatus::Success),
                "partial" => Some(RustOutcomeStatus::Partial),
                "failure" => Some(RustOutcomeStatus::Failure),
                _ => None,
            }),
            min_confidence: query.min_confidence,
            date_from: query.date_from,
            date_to: query.date_to,
            limit: query.limit.map(|l| l as usize),
        }
    }
}

/// Causal stats for NAPI
#[napi(object)]
pub struct NapiCausalStats {
    pub total_decisions: u32,
    pub total_actions: u32,
    pub total_outcomes: u32,
    pub total_edges: u32,
    pub success_rate: f64,
    pub avg_confidence: f64,
}

impl From<RustCausalStats> for NapiCausalStats {
    fn from(stats: RustCausalStats) -> Self {
        Self {
            total_decisions: stats.total_decisions as u32,
            total_actions: stats.total_actions as u32,
            total_outcomes: stats.total_outcomes as u32,
            total_edges: stats.total_edges as u32,
            success_rate: stats.success_rate,
            avg_confidence: stats.avg_confidence,
        }
    }
}

/// Causal pattern for recurring decision-outcome combinations
#[napi(object)]
pub struct NapiCausalPattern {
    pub id: String,
    pub name: String,
    pub description: String,
    pub agent_id: String,
    pub action_type: String,
    pub occurrences: u32,
    pub success_rate: f64,
    pub avg_confidence: f64,
    pub examples: Vec<String>,
}

impl From<RustCausalPattern> for NapiCausalPattern {
    fn from(pattern: RustCausalPattern) -> Self {
        Self {
            id: pattern.id,
            name: pattern.name,
            description: pattern.description,
            agent_id: pattern.agent_id,
            action_type: pattern.action_type,
            occurrences: pattern.occurrences as u32,
            success_rate: pattern.success_rate,
            avg_confidence: pattern.avg_confidence,
            examples: pattern.examples,
        }
    }
}

/// Result of finding similar decisions
#[napi(object)]
pub struct NapiSimilarDecision {
    pub decision_id: String,
    pub prompt: String,
    pub similarity: f64,
}

impl From<RustSimilarDecision> for NapiSimilarDecision {
    fn from(similar: RustSimilarDecision) -> Self {
        Self {
            decision_id: similar.decision_id,
            prompt: similar.prompt,
            similarity: similar.similarity,
        }
    }
}

/// Trend analysis result
#[napi(object)]
pub struct NapiTrendAnalysis {
    pub total_decisions: u32,
    pub success_rate_before: f64,
    pub success_rate_after: f64,
    pub confidence_before: f64,
    pub confidence_after: f64,
    pub action_type_shifts: String, // JSON string of HashMap<String, (usize, usize)>
}

impl From<RustTrendAnalysis> for NapiTrendAnalysis {
    fn from(trend: RustTrendAnalysis) -> Self {
        Self {
            total_decisions: trend.total_decisions as u32,
            success_rate_before: trend.success_rate_trend[0],
            success_rate_after: trend.success_rate_trend[1],
            confidence_before: trend.confidence_trend[0],
            confidence_after: trend.confidence_trend[1],
            action_type_shifts: serde_json::to_string(&trend.action_type_shifts).unwrap_or_default(),
        }
    }
}

/// Agent insights result
#[napi(object)]
pub struct NapiAgentInsights {
    pub total_decisions: u32,
    pub success_rate: f64,
    pub avg_confidence: f64,
    pub strongest_action_type: Option<String>,
    pub weakest_action_type: Option<String>,
    pub recent_trend: String,
    pub suggestions: Vec<String>,
}

impl From<RustAgentInsights> for NapiAgentInsights {
    fn from(insights: RustAgentInsights) -> Self {
        Self {
            total_decisions: insights.total_decisions as u32,
            success_rate: insights.success_rate,
            avg_confidence: insights.avg_confidence,
            strongest_action_type: insights.strongest_action_type,
            weakest_action_type: insights.weakest_action_type,
            recent_trend: insights.recent_trend,
            suggestions: insights.suggestions,
        }
    }
}

// ============================================================================
// CausalGraph NAPI Handle
// ============================================================================

/// Thread-safe wrapper for CausalGraph
#[napi]
pub struct CausalGraphHandle {
    inner: Arc<Mutex<RustCausalGraph>>,
}

#[napi]
impl CausalGraphHandle {
    /// Create a new causal graph for a project
    #[napi(constructor)]
    pub fn new(project_id: String) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RustCausalGraph::new(project_id))),
        }
    }

    /// Record a decision
    #[napi]
    pub fn record_decision(
        &self,
        session_id: String,
        agent_id: String,
        prompt: String,
        reasoning: String,
        confidence: f64,
        context: Option<String>,
    ) -> Result<NapiDecisionNode> {
        let mut graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let context_value: Option<serde_json::Value> =
            context.and_then(|c| serde_json::from_str(&c).ok());
        let node = graph.record_decision(session_id, agent_id, prompt, reasoning, confidence, context_value);
        Ok(node.into())
    }

    /// Record an action
    #[napi]
    pub fn record_action(
        &self,
        decision_id: String,
        action_type: String,
        description: String,
        input: Option<String>,
        output: Option<String>,
        duration: Option<u32>,
    ) -> Result<NapiActionNode> {
        let mut graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let input_value: Option<serde_json::Value> = input.and_then(|i| serde_json::from_str(&i).ok());
        let output_value: Option<serde_json::Value> = output.and_then(|o| serde_json::from_str(&o).ok());
        let node = graph
            .record_action(&decision_id, action_type, description, input_value, output_value, duration.map(|d| d as u64))
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(node.into())
    }

    /// Record an outcome
    #[napi]
    pub fn record_outcome(
        &self,
        action_id: String,
        status: String,
        description: String,
        metrics: Option<String>,
        feedback: Option<String>,
    ) -> Result<NapiOutcomeNode> {
        let mut graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let status_enum = match status.as_str() {
            "success" => RustOutcomeStatus::Success,
            "partial" => RustOutcomeStatus::Partial,
            _ => RustOutcomeStatus::Failure,
        };
        let metrics_value: Option<serde_json::Value> = metrics.and_then(|m| serde_json::from_str(&m).ok());
        let node = graph
            .record_outcome(&action_id, status_enum, description, metrics_value, feedback)
            .map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(node.into())
    }

    /// Get a decision by ID
    #[napi]
    pub fn get_decision(&self, id: String) -> Result<Option<NapiDecisionNode>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_decision(&id).map(|n| n.into()))
    }

    /// Get all decisions
    #[napi]
    pub fn get_decisions(&self) -> Result<Vec<NapiDecisionNode>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_decisions().into_iter().map(|n| n.into()).collect())
    }

    /// Get a causal chain starting from a decision
    #[napi]
    pub fn get_causal_chain(&self, decision_id: String) -> Result<Option<NapiCausalChain>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_causal_chain(&decision_id).map(|c| c.into()))
    }

    /// Query the causal graph
    #[napi]
    pub fn query(&self, query: NapiCausalQuery) -> Result<Vec<NapiCausalChain>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let rust_query: RustCausalQuery = query.into();
        Ok(graph.query(&rust_query).into_iter().map(|c| c.into()).collect())
    }

    /// Get success rate
    #[napi]
    pub fn get_success_rate(&self, agent_id: Option<String>) -> Result<f64> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_success_rate(agent_id.as_deref()))
    }

    /// Get statistics
    #[napi]
    pub fn get_stats(&self) -> Result<NapiCausalStats> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_stats().into())
    }

    /// Check for cycles (should be none in a valid causal graph)
    #[napi]
    pub fn has_cycles(&self) -> Result<bool> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.has_cycles())
    }

    /// Find recurring decision-outcome patterns
    #[napi]
    pub fn find_patterns(
        &self,
        agent_id: Option<String>,
        min_occurrences: u32,
        limit: u32,
    ) -> Result<Vec<NapiCausalPattern>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph
            .find_patterns(agent_id.as_deref(), min_occurrences as usize, limit as usize)
            .into_iter()
            .map(|p| p.into())
            .collect())
    }

    /// Find decisions similar to the given prompt
    #[napi]
    pub fn find_similar_decisions(
        &self,
        prompt: String,
        agent_id: String,
        limit: u32,
    ) -> Result<Vec<NapiSimilarDecision>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph
            .find_similar_decisions(&prompt, &agent_id, limit as usize)
            .into_iter()
            .map(|s| s.into())
            .collect())
    }

    /// Analyze decision trends over time
    #[napi]
    pub fn analyze_trends(&self, agent_id: Option<String>, period_days: u32) -> Result<NapiTrendAnalysis> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.analyze_trends(agent_id.as_deref(), period_days as u64).into())
    }

    /// Get aggregated insights for an agent
    #[napi]
    pub fn get_agent_insights(&self, agent_id: String) -> Result<NapiAgentInsights> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_agent_insights(&agent_id).into())
    }

    /// Serialize to JSON
    #[napi]
    pub fn to_json(&self) -> Result<String> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.to_json().to_string())
    }

    /// Deserialize from JSON
    #[napi(factory)]
    pub fn from_json(json: String) -> Result<Self> {
        let value: serde_json::Value =
            serde_json::from_str(&json).map_err(|e| Error::from_reason(e.to_string()))?;
        let graph =
            RustCausalGraph::from_json(&value).map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(Mutex::new(graph)),
        })
    }
}

// ============================================================================
// CallGraph NAPI Types
// ============================================================================

/// Call node for NAPI
#[napi(object)]
pub struct NapiCallNode {
    pub id: String,
    pub name: String,
    pub kind: String, // "function" | "method" | "constructor"
    pub file: String,
    pub line: u32,
    pub character: u32,
    pub detail: Option<String>,
}

impl From<RustCallNode> for NapiCallNode {
    fn from(node: RustCallNode) -> Self {
        Self {
            id: node.id,
            name: node.name,
            kind: match node.kind {
                CallableKind::Function => "function".to_string(),
                CallableKind::Method => "method".to_string(),
                CallableKind::Constructor => "constructor".to_string(),
            },
            file: node.file,
            line: node.line,
            character: node.character,
            detail: node.detail,
        }
    }
}

/// Recursion info for NAPI
#[napi(object)]
pub struct NapiRecursionInfo {
    pub node: NapiCallNode,
    pub recursion_type: String, // "direct" | "indirect"
    pub cycle: Vec<String>,
}

// ============================================================================
// CallGraph NAPI Handle
// ============================================================================

/// Thread-safe wrapper for CallGraph
#[napi]
pub struct CallGraphHandle {
    inner: Arc<Mutex<RustCallGraph>>,
}

#[napi]
impl CallGraphHandle {
    /// Create a new call graph
    #[napi(constructor)]
    pub fn new(project_id: String) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RustCallGraph::new(project_id))),
        }
    }

    /// Add a function
    #[napi]
    pub fn add_function(
        &self,
        name: String,
        file: String,
        line: u32,
        character: u32,
        kind: String,
        detail: Option<String>,
    ) -> Result<NapiCallNode> {
        let mut graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let kind_enum = match kind.as_str() {
            "method" => CallableKind::Method,
            "constructor" => CallableKind::Constructor,
            _ => CallableKind::Function,
        };
        let node = graph.add_function(name, file, line, character, kind_enum, detail);
        Ok(node.into())
    }

    /// Add a call relationship
    #[napi]
    pub fn add_call(
        &self,
        caller_id: String,
        callee_id: String,
        line: u32,
        character: u32,
    ) -> Result<Option<String>> {
        let mut graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let locations = vec![crate::graph::call::CallLocation { line, character }];
        Ok(graph.add_call(&caller_id, &callee_id, locations))
    }

    /// Get a node by ID
    #[napi]
    pub fn get_node(&self, id: String) -> Result<Option<NapiCallNode>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_node(&id).map(|n| n.into()))
    }

    /// Get all nodes
    #[napi]
    pub fn get_nodes(&self) -> Result<Vec<NapiCallNode>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_nodes().into_iter().map(|n| n.into()).collect())
    }

    /// Get callers of a function
    #[napi]
    pub fn get_callers(&self, callee_id: String) -> Result<Vec<NapiCallNode>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_callers(&callee_id).into_iter().map(|n| n.into()).collect())
    }

    /// Get callees of a function
    #[napi]
    pub fn get_callees(&self, caller_id: String) -> Result<Vec<NapiCallNode>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_callees(&caller_id).into_iter().map(|n| n.into()).collect())
    }

    /// Detect recursive functions
    #[napi]
    pub fn detect_recursion(&self) -> Result<Vec<NapiRecursionInfo>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph
            .detect_recursion()
            .into_iter()
            .map(|r| NapiRecursionInfo {
                node: r.node.into(),
                recursion_type: match r.recursion_type {
                    RecursionType::Direct => "direct".to_string(),
                    RecursionType::Indirect => "indirect".to_string(),
                },
                cycle: r.cycle,
            })
            .collect())
    }

    /// Get entry points (functions with no callers)
    #[napi]
    pub fn get_entry_points(&self) -> Result<Vec<NapiCallNode>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_entry_points().into_iter().map(|n| n.into()).collect())
    }

    /// Get leaf functions (functions with no callees)
    #[napi]
    pub fn get_leaf_functions(&self) -> Result<Vec<NapiCallNode>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_leaf_functions().into_iter().map(|n| n.into()).collect())
    }

    /// Serialize to JSON
    #[napi]
    pub fn to_json(&self) -> Result<String> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.to_json().to_string())
    }

    /// Deserialize from JSON
    #[napi(factory)]
    pub fn from_json(json: String) -> Result<Self> {
        let value: serde_json::Value =
            serde_json::from_str(&json).map_err(|e| Error::from_reason(e.to_string()))?;
        let graph =
            RustCallGraph::from_json(&value).map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(Mutex::new(graph)),
        })
    }
}

// ============================================================================
// SemanticGraph NAPI Types
// ============================================================================

/// Semantic node for NAPI
#[napi(object)]
pub struct NapiSemanticNode {
    pub id: String,
    pub node_type: String, // "function" | "class" | "interface" | etc.
    pub name: String,
    pub file: String,
    pub metadata: Option<String>, // JSON string
}

impl From<RustSemanticNode> for NapiSemanticNode {
    fn from(node: RustSemanticNode) -> Self {
        Self {
            id: node.id,
            node_type: node.node_type.as_str().to_string(),
            name: node.name,
            file: node.file,
            metadata: node.metadata.map(|m| m.to_string()),
        }
    }
}

/// Semantic graph stats for NAPI
#[napi(object)]
pub struct NapiSemanticStats {
    pub total_nodes: u32,
    pub total_edges: u32,
    pub files: u32,
    pub functions: u32,
    pub classes: u32,
    pub interfaces: u32,
    pub has_cycles: bool,
}

// ============================================================================
// SemanticGraph NAPI Handle
// ============================================================================

/// Thread-safe wrapper for SemanticGraph
#[napi]
pub struct SemanticGraphHandle {
    inner: Arc<Mutex<RustSemanticGraph>>,
}

#[napi]
impl SemanticGraphHandle {
    /// Create a new semantic graph
    #[napi(constructor)]
    pub fn new(project_id: String) -> Self {
        Self {
            inner: Arc::new(Mutex::new(RustSemanticGraph::new(project_id))),
        }
    }

    /// Add a function
    #[napi]
    pub fn add_function(
        &self,
        name: String,
        file: String,
        signature: Option<String>,
        exported: bool,
    ) -> Result<NapiSemanticNode> {
        let mut graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let node = graph.add_function(name, file, signature.as_deref(), exported);
        Ok(node.into())
    }

    /// Add a class
    #[napi]
    pub fn add_class(
        &self,
        name: String,
        file: String,
        extends: Option<String>,
        methods: Vec<String>,
    ) -> Result<NapiSemanticNode> {
        let mut graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let method_refs: Vec<&str> = methods.iter().map(|s| s.as_str()).collect();
        let node = graph.add_class(name, file, extends.as_deref(), &method_refs);
        Ok(node.into())
    }

    /// Add an interface
    #[napi]
    pub fn add_interface(&self, name: String, file: String, extends: Option<String>) -> Result<NapiSemanticNode> {
        let mut graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let node = graph.add_interface(name, file, extends.as_deref());
        Ok(node.into())
    }

    /// Add a file
    #[napi]
    pub fn add_file(&self, path: String) -> Result<NapiSemanticNode> {
        let mut graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let node = graph.add_file(path);
        Ok(node.into())
    }

    /// Add an import relationship
    #[napi]
    pub fn add_import(&self, importer: String, imported: String) -> Result<Option<String>> {
        let mut graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.add_import(&importer, &imported))
    }

    /// Add an extends relationship
    #[napi]
    pub fn add_extends(&self, child: String, parent: String) -> Result<Option<String>> {
        let mut graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.add_extends(&child, &parent))
    }

    /// Add an implements relationship
    #[napi]
    pub fn add_implements(&self, implementor: String, interface: String) -> Result<Option<String>> {
        let mut graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.add_implements(&implementor, &interface))
    }

    /// Add a contains relationship
    #[napi]
    pub fn add_contains(&self, container: String, contained: String) -> Result<Option<String>> {
        let mut graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.add_contains(&container, &contained))
    }

    /// Get a node by ID
    #[napi]
    pub fn get_node(&self, id: String) -> Result<Option<NapiSemanticNode>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_node(&id).map(|n| n.into()))
    }

    /// Get all nodes
    #[napi]
    pub fn get_nodes(&self) -> Result<Vec<NapiSemanticNode>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_nodes().into_iter().map(|n| n.into()).collect())
    }

    /// Get nodes by type
    #[napi]
    pub fn get_nodes_by_type(&self, node_type: String) -> Result<Vec<NapiSemanticNode>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let type_enum = match node_type.as_str() {
            "function" => SemanticNodeType::Function,
            "class" => SemanticNodeType::Class,
            "interface" => SemanticNodeType::Interface,
            "type" => SemanticNodeType::Type,
            "enum" => SemanticNodeType::Enum,
            "component" => SemanticNodeType::Component,
            "file" => SemanticNodeType::File,
            "module" => SemanticNodeType::Module,
            _ => return Ok(vec![]),
        };
        Ok(graph.get_nodes_by_type(type_enum).into_iter().map(|n| n.into()).collect())
    }

    /// Get imports of an entity
    #[napi]
    pub fn get_imports(&self, entity_id: String) -> Result<Vec<NapiSemanticNode>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_imports(&entity_id).into_iter().map(|n| n.into()).collect())
    }

    /// Get importers of an entity
    #[napi]
    pub fn get_importers(&self, entity_id: String) -> Result<Vec<NapiSemanticNode>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_importers(&entity_id).into_iter().map(|n| n.into()).collect())
    }

    /// Get inheritance chain
    #[napi]
    pub fn get_inheritance_chain(&self, class_id: String) -> Result<Vec<NapiSemanticNode>> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.get_inheritance_chain(&class_id).into_iter().map(|n| n.into()).collect())
    }

    /// Check for circular dependencies
    #[napi]
    pub fn has_circular_dependencies(&self) -> Result<bool> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.has_circular_dependencies())
    }

    /// Get statistics
    #[napi]
    pub fn stats(&self) -> Result<NapiSemanticStats> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        let s = graph.stats();
        Ok(NapiSemanticStats {
            total_nodes: s.total_nodes as u32,
            total_edges: s.total_edges as u32,
            files: s.files as u32,
            functions: s.functions as u32,
            classes: s.classes as u32,
            interfaces: s.interfaces as u32,
            has_cycles: s.has_cycles,
        })
    }

    /// Serialize to JSON
    #[napi]
    pub fn to_json(&self) -> Result<String> {
        let graph = self.inner.lock().map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(graph.to_json().to_string())
    }

    /// Deserialize from JSON
    #[napi(factory)]
    pub fn from_json(json: String) -> Result<Self> {
        let value: serde_json::Value =
            serde_json::from_str(&json).map_err(|e| Error::from_reason(e.to_string()))?;
        let graph =
            RustSemanticGraph::from_json(&value).map_err(|e| Error::from_reason(e.to_string()))?;
        Ok(Self {
            inner: Arc::new(Mutex::new(graph)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_engine_handle() {
        let engine = GraphEngineHandle::new();
        engine.add_node("a".to_string(), "test".to_string(), "{}".to_string()).unwrap();
        engine.add_node("b".to_string(), "test".to_string(), "{}".to_string()).unwrap();
        engine.add_edge("a".to_string(), "b".to_string(), "calls".to_string()).unwrap();

        assert_eq!(engine.node_count().unwrap(), 2);
        assert_eq!(engine.edge_count().unwrap(), 1);
    }

    #[test]
    fn test_causal_graph_handle() {
        let graph = CausalGraphHandle::new("test-project".to_string());
        let decision = graph.record_decision(
            "session1".to_string(),
            "agent1".to_string(),
            "Fix bug".to_string(),
            "Need to add null check".to_string(),
            0.9,
            None,
        ).unwrap();

        assert!(decision.id.starts_with("dec_"));
    }

    #[test]
    fn test_call_graph_handle() {
        let graph = CallGraphHandle::new("test-project".to_string());
        let func = graph.add_function(
            "main".to_string(),
            "src/main.rs".to_string(),
            10,
            0,
            "function".to_string(),
            None,
        ).unwrap();

        assert!(func.id.contains("main"));
    }

    #[test]
    fn test_semantic_graph_handle() {
        let graph = SemanticGraphHandle::new("test-project".to_string());
        let func = graph.add_function(
            "main".to_string(),
            "src/main.ts".to_string(),
            Some("() => void".to_string()),
            true,
        ).unwrap();

        assert_eq!(func.node_type, "function");
    }
}
