//! GraphEngine - Core graph data structure using petgraph
//!
//! Provides a unified, high-performance graph engine that backs all knowledge
//! graph types (causal, call, semantic).

use std::collections::HashMap;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::Direction;
use serde::{Deserialize, Serialize};

/// Unique identifier for a node
pub type NodeId = String;

/// Unique identifier for an edge
pub type EdgeId = String;

/// Generic node data that can hold any graph node type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeData {
    /// Unique node identifier
    pub id: NodeId,
    /// Node type discriminator
    pub node_type: String,
    /// Serialized payload (JSON)
    pub payload: serde_json::Value,
    /// Creation timestamp (ms since epoch)
    pub created_at: i64,
    /// Last update timestamp (ms since epoch)
    pub updated_at: i64,
}

impl NodeData {
    /// Create a new node with the given type and payload
    pub fn new<T: Serialize>(id: impl Into<String>, node_type: impl Into<String>, data: &T) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            id: id.into(),
            node_type: node_type.into(),
            payload: serde_json::to_value(data).unwrap_or(serde_json::Value::Null),
            created_at: now,
            updated_at: now,
        }
    }

    /// Extract the typed payload
    pub fn get_payload<T: for<'de> Deserialize<'de>>(&self) -> Option<T> {
        serde_json::from_value(self.payload.clone()).ok()
    }
}

/// Edge relationship types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeRelationship {
    /// Causal relationship (decision causes action)
    Causes,
    /// Result relationship (action results in outcome)
    ResultsIn,
    /// Function call relationship
    Calls,
    /// Import relationship
    Imports,
    /// Export relationship
    Exports,
    /// Inheritance relationship
    Extends,
    /// Interface implementation
    Implements,
    /// Reference relationship
    References,
    /// Containment (parent-child)
    Contains,
    /// Generic related
    Related,
    /// Instantiation (new)
    Instantiates,
}

/// Generic edge data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeData {
    /// Unique edge identifier
    pub id: EdgeId,
    /// Source node ID
    pub source: NodeId,
    /// Target node ID
    pub target: NodeId,
    /// Relationship type
    pub relationship: EdgeRelationship,
    /// Edge weight (0.0 to 1.0)
    pub weight: f64,
    /// Optional metadata
    pub metadata: Option<serde_json::Value>,
}

impl EdgeData {
    /// Create a new edge
    pub fn new(
        source: impl Into<String>,
        target: impl Into<String>,
        relationship: EdgeRelationship,
    ) -> Self {
        let src = source.into();
        let tgt = target.into();
        Self {
            id: format!("{}->{}", src, tgt),
            source: src,
            target: tgt,
            relationship,
            weight: 1.0,
            metadata: None,
        }
    }

    /// Set edge weight
    pub fn with_weight(mut self, weight: f64) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Set edge metadata
    pub fn with_metadata<T: Serialize>(mut self, metadata: &T) -> Self {
        self.metadata = serde_json::to_value(metadata).ok();
        self
    }
}

/// High-performance graph engine backed by petgraph
#[derive(Debug)]
pub struct GraphEngine {
    /// The underlying directed graph
    graph: DiGraph<NodeData, EdgeData>,
    /// Fast lookup from node ID to graph index
    node_index: HashMap<NodeId, NodeIndex>,
}

impl Default for GraphEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphEngine {
    /// Create a new empty graph engine
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_index: HashMap::new(),
        }
    }

    /// Create with capacity hints
    pub fn with_capacity(nodes: usize, edges: usize) -> Self {
        Self {
            graph: DiGraph::with_capacity(nodes, edges),
            node_index: HashMap::with_capacity(nodes),
        }
    }

    /// Add a node to the graph, returns the node ID
    pub fn add_node(&mut self, data: NodeData) -> NodeId {
        let id = data.id.clone();
        if let Some(&existing_idx) = self.node_index.get(&id) {
            // Update existing node
            if let Some(node) = self.graph.node_weight_mut(existing_idx) {
                *node = data;
            }
        } else {
            // Insert new node
            let idx = self.graph.add_node(data);
            self.node_index.insert(id.clone(), idx);
        }
        id
    }

    /// Remove a node from the graph
    pub fn remove_node(&mut self, id: &str) -> Option<NodeData> {
        if let Some(&idx) = self.node_index.get(id) {
            self.node_index.remove(id);
            self.graph.remove_node(idx)
        } else {
            None
        }
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Option<&NodeData> {
        self.node_index
            .get(id)
            .and_then(|&idx| self.graph.node_weight(idx))
    }

    /// Get a mutable reference to a node
    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut NodeData> {
        self.node_index
            .get(id)
            .copied()
            .and_then(|idx| self.graph.node_weight_mut(idx))
    }

    /// Check if a node exists
    pub fn contains_node(&self, id: &str) -> bool {
        self.node_index.contains_key(id)
    }

    /// Add an edge between two nodes
    pub fn add_edge(&mut self, data: EdgeData) -> Option<EdgeId> {
        let source_idx = self.node_index.get(&data.source)?;
        let target_idx = self.node_index.get(&data.target)?;
        let id = data.id.clone();
        self.graph.add_edge(*source_idx, *target_idx, data);
        Some(id)
    }

    /// Get all edges from a source node
    pub fn get_outgoing_edges(&self, source: &str) -> Vec<&EdgeData> {
        self.node_index
            .get(source)
            .map(|&idx| {
                self.graph
                    .edges_directed(idx, Direction::Outgoing)
                    .map(|e| e.weight())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all edges to a target node
    pub fn get_incoming_edges(&self, target: &str) -> Vec<&EdgeData> {
        self.node_index
            .get(target)
            .map(|&idx| {
                self.graph
                    .edges_directed(idx, Direction::Incoming)
                    .map(|e| e.weight())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get direct successors of a node
    pub fn get_successors(&self, id: &str) -> Vec<NodeId> {
        self.node_index
            .get(id)
            .map(|&idx| {
                self.graph
                    .neighbors_directed(idx, Direction::Outgoing)
                    .filter_map(|neighbor_idx| {
                        self.graph
                            .node_weight(neighbor_idx)
                            .map(|n| n.id.clone())
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get direct predecessors of a node
    pub fn get_predecessors(&self, id: &str) -> Vec<NodeId> {
        self.node_index
            .get(id)
            .map(|&idx| {
                self.graph
                    .neighbors_directed(idx, Direction::Incoming)
                    .filter_map(|neighbor_idx| {
                        self.graph
                            .node_weight(neighbor_idx)
                            .map(|n| n.id.clone())
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get total node count
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get total edge count
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Get all node IDs
    pub fn node_ids(&self) -> Vec<NodeId> {
        self.node_index.keys().cloned().collect()
    }

    /// Get all nodes of a specific type
    pub fn nodes_by_type(&self, node_type: &str) -> Vec<&NodeData> {
        self.graph
            .node_weights()
            .filter(|n| n.node_type == node_type)
            .collect()
    }

    /// Get the underlying petgraph for advanced operations
    pub fn inner(&self) -> &DiGraph<NodeData, EdgeData> {
        &self.graph
    }

    /// Get mutable access to the underlying petgraph
    pub fn inner_mut(&mut self) -> &mut DiGraph<NodeData, EdgeData> {
        &mut self.graph
    }

    /// Get node index for a given ID (for algorithm use)
    pub fn get_node_index(&self, id: &str) -> Option<NodeIndex> {
        self.node_index.get(id).copied()
    }

    /// Clear all nodes and edges
    pub fn clear(&mut self) {
        self.graph.clear();
        self.node_index.clear();
    }

    /// Serialize the graph to JSON
    pub fn to_json(&self) -> serde_json::Value {
        let nodes: Vec<&NodeData> = self.graph.node_weights().collect();
        let edges: Vec<&EdgeData> = self.graph.edge_weights().collect();
        serde_json::json!({
            "nodes": nodes,
            "edges": edges,
            "node_count": self.node_count(),
            "edge_count": self.edge_count()
        })
    }

    /// Deserialize a graph from JSON
    pub fn from_json(value: &serde_json::Value) -> anyhow::Result<Self> {
        let mut engine = Self::new();

        if let Some(nodes) = value.get("nodes").and_then(|v| v.as_array()) {
            for node_value in nodes {
                let node: NodeData = serde_json::from_value(node_value.clone())?;
                engine.add_node(node);
            }
        }

        if let Some(edges) = value.get("edges").and_then(|v| v.as_array()) {
            for edge_value in edges {
                let edge: EdgeData = serde_json::from_value(edge_value.clone())?;
                engine.add_edge(edge);
            }
        }

        Ok(engine)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_and_get_node() {
        let mut engine = GraphEngine::new();

        let node = NodeData::new("node1", "test", &serde_json::json!({"value": 42}));
        engine.add_node(node);

        assert!(engine.contains_node("node1"));
        assert!(!engine.contains_node("node2"));

        let retrieved = engine.get_node("node1").unwrap();
        assert_eq!(retrieved.id, "node1");
        assert_eq!(retrieved.node_type, "test");
    }

    #[test]
    fn test_add_edge() {
        let mut engine = GraphEngine::new();

        engine.add_node(NodeData::new("a", "test", &()));
        engine.add_node(NodeData::new("b", "test", &()));

        let edge = EdgeData::new("a", "b", EdgeRelationship::Calls);
        assert!(engine.add_edge(edge).is_some());

        let outgoing = engine.get_outgoing_edges("a");
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].target, "b");
    }

    #[test]
    fn test_successors_predecessors() {
        let mut engine = GraphEngine::new();

        engine.add_node(NodeData::new("a", "test", &()));
        engine.add_node(NodeData::new("b", "test", &()));
        engine.add_node(NodeData::new("c", "test", &()));

        engine.add_edge(EdgeData::new("a", "b", EdgeRelationship::Calls));
        engine.add_edge(EdgeData::new("a", "c", EdgeRelationship::Calls));

        let successors = engine.get_successors("a");
        assert_eq!(successors.len(), 2);

        let predecessors = engine.get_predecessors("b");
        assert_eq!(predecessors.len(), 1);
        assert_eq!(predecessors[0], "a");
    }

    #[test]
    fn test_serialization() {
        let mut engine = GraphEngine::new();
        engine.add_node(NodeData::new("x", "test", &42));
        engine.add_node(NodeData::new("y", "test", &"hello"));
        engine.add_edge(EdgeData::new("x", "y", EdgeRelationship::Related));

        let json = engine.to_json();
        let restored = GraphEngine::from_json(&json).unwrap();

        assert_eq!(restored.node_count(), 2);
        assert_eq!(restored.edge_count(), 1);
    }
}
