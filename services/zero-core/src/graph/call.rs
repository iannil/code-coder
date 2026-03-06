//! Call Graph - Function call relationship tracking
//!
//! Tracks function/method call relationships for code analysis:
//! - Function definitions and their locations
//! - Call relationships between functions
//! - Recursion detection (direct and indirect)
//! - Call chain analysis

use serde::{Deserialize, Serialize};

use super::algorithms::GraphAlgorithms;
use super::engine::{EdgeData, EdgeRelationship, GraphEngine, NodeData, NodeId};

/// Kind of callable entity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CallableKind {
    Function,
    Method,
    Constructor,
}

/// A node representing a function/method in the call graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallNode {
    pub id: NodeId,
    pub name: String,
    pub kind: CallableKind,
    pub file: String,
    pub line: u32,
    pub character: u32,
    pub detail: Option<String>,
}

/// An edge representing a call relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallEdge {
    pub id: String,
    pub caller: NodeId,
    pub callee: NodeId,
    pub locations: Vec<CallLocation>,
}

/// Location of a call site
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallLocation {
    pub line: u32,
    pub character: u32,
}

/// A chain of calls starting from a function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallChain {
    pub start_node: NodeId,
    pub max_depth: usize,
    pub nodes: Vec<CallNode>,
    pub edges: Vec<CallEdge>,
    pub depth: std::collections::HashMap<NodeId, usize>,
}

/// Information about detected recursion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecursionInfo {
    pub node: CallNode,
    pub recursion_type: RecursionType,
    pub cycle: Vec<NodeId>,
}

/// Type of recursion detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecursionType {
    Direct,
    Indirect,
}

/// Node type constant
const NODE_TYPE_CALL: &str = "call";

/// Call Graph implementation
#[derive(Debug)]
pub struct CallGraph {
    engine: GraphEngine,
    project_id: String,
    created_at: i64,
    updated_at: i64,
}

impl CallGraph {
    /// Create a new call graph for a project
    pub fn new(project_id: impl Into<String>) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        Self {
            engine: GraphEngine::new(),
            project_id: project_id.into(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Generate a node ID
    fn node_id(name: &str, file: &str, line: u32) -> String {
        format!("call:{}:{}:{}", name, file, line)
    }

    // ============================================================================
    // Node Operations
    // ============================================================================

    /// Add a function/method node to the graph
    pub fn add_node(&mut self, node: CallNode) -> NodeId {
        let id = node.id.clone();
        let node_data = NodeData::new(&id, NODE_TYPE_CALL, &node);
        self.engine.add_node(node_data);
        self.updated_at = chrono::Utc::now().timestamp_millis();
        id
    }

    /// Create and add a node
    pub fn add_function(
        &mut self,
        name: impl Into<String>,
        file: impl Into<String>,
        line: u32,
        character: u32,
        kind: CallableKind,
        detail: Option<String>,
    ) -> CallNode {
        let name = name.into();
        let file = file.into();
        let id = Self::node_id(&name, &file, line);

        let node = CallNode {
            id: id.clone(),
            name,
            kind,
            file,
            line,
            character,
            detail,
        };

        self.add_node(node.clone());
        node
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Option<CallNode> {
        self.engine
            .get_node(id)
            .filter(|n| n.node_type == NODE_TYPE_CALL)
            .and_then(|n| n.get_payload())
    }

    /// Get all nodes
    pub fn get_nodes(&self) -> Vec<CallNode> {
        self.engine
            .nodes_by_type(NODE_TYPE_CALL)
            .into_iter()
            .filter_map(|n| n.get_payload())
            .collect()
    }

    /// Find nodes by name
    pub fn find_by_name(&self, name: &str) -> Vec<CallNode> {
        self.get_nodes()
            .into_iter()
            .filter(|n| n.name == name)
            .collect()
    }

    /// Find nodes in a file
    pub fn find_in_file(&self, file: &str) -> Vec<CallNode> {
        self.get_nodes()
            .into_iter()
            .filter(|n| n.file == file)
            .collect()
    }

    // ============================================================================
    // Edge Operations
    // ============================================================================

    /// Add a call edge between two functions
    pub fn add_call(
        &mut self,
        caller_id: &str,
        callee_id: &str,
        locations: Vec<CallLocation>,
    ) -> Option<String> {
        // Create edge
        let edge_id = format!("{}->{}", caller_id, callee_id);
        let edge_data = EdgeData::new(caller_id, callee_id, EdgeRelationship::Calls)
            .with_metadata(&CallEdge {
                id: edge_id.clone(),
                caller: caller_id.to_string(),
                callee: callee_id.to_string(),
                locations,
            });

        self.engine.add_edge(edge_data)?;
        self.updated_at = chrono::Utc::now().timestamp_millis();
        Some(edge_id)
    }

    /// Get all callers of a function
    pub fn get_callers(&self, callee_id: &str) -> Vec<CallNode> {
        self.engine
            .get_predecessors(callee_id)
            .into_iter()
            .filter_map(|id| self.get_node(&id))
            .collect()
    }

    /// Get all callees of a function
    pub fn get_callees(&self, caller_id: &str) -> Vec<CallNode> {
        self.engine
            .get_successors(caller_id)
            .into_iter()
            .filter_map(|id| self.get_node(&id))
            .collect()
    }

    /// Get call edges from a node
    pub fn get_call_edges(&self, caller_id: &str) -> Vec<CallEdge> {
        self.engine
            .get_outgoing_edges(caller_id)
            .into_iter()
            .filter_map(|e| {
                e.metadata.as_ref().and_then(|m| {
                    serde_json::from_value::<CallEdge>(m.clone()).ok()
                })
            })
            .collect()
    }

    // ============================================================================
    // Chain Analysis
    // ============================================================================

    /// Get the call chain starting from a function
    pub fn get_call_chain(&self, start_id: &str, max_depth: usize) -> Option<CallChain> {
        let start_node = self.get_node(start_id)?;

        let reachable = GraphAlgorithms::get_reachable(&self.engine, start_id, max_depth);
        let distances = GraphAlgorithms::shortest_distances(&self.engine, start_id);

        let nodes: Vec<CallNode> = reachable
            .iter()
            .filter_map(|id| self.get_node(id))
            .collect();

        let mut edges = Vec::new();
        for node in &nodes {
            for edge in self.get_call_edges(&node.id) {
                edges.push(edge);
            }
        }

        Some(CallChain {
            start_node: start_node.id,
            max_depth,
            nodes,
            edges,
            depth: distances,
        })
    }

    /// Get the reverse call chain (all functions that call this one)
    pub fn get_reverse_call_chain(&self, target_id: &str, max_depth: usize) -> Vec<CallNode> {
        GraphAlgorithms::get_reverse_reachable(&self.engine, target_id, max_depth)
            .into_iter()
            .filter_map(|id| self.get_node(&id))
            .collect()
    }

    // ============================================================================
    // Recursion Detection
    // ============================================================================

    /// Detect all recursive functions
    pub fn detect_recursion(&self) -> Vec<RecursionInfo> {
        let cycle_result = GraphAlgorithms::detect_cycles(&self.engine);

        if !cycle_result.has_cycles {
            return Vec::new();
        }

        let mut recursion_info = Vec::new();

        for cycle in cycle_result.cycles {
            if cycle.is_empty() {
                continue;
            }

            // Determine type: direct (self-loop) or indirect
            let recursion_type = if cycle.len() == 1 {
                RecursionType::Direct
            } else {
                RecursionType::Indirect
            };

            // Get the first node in the cycle as the "recursive" function
            if let Some(node) = self.get_node(&cycle[0]) {
                recursion_info.push(RecursionInfo {
                    node,
                    recursion_type,
                    cycle,
                });
            }
        }

        recursion_info
    }

    /// Check if a specific function is recursive
    pub fn is_recursive(&self, node_id: &str) -> Option<RecursionInfo> {
        // Check for direct recursion (self-loop)
        let callees = self.engine.get_successors(node_id);
        if callees.contains(&node_id.to_string()) {
            if let Some(node) = self.get_node(node_id) {
                return Some(RecursionInfo {
                    node,
                    recursion_type: RecursionType::Direct,
                    cycle: vec![node_id.to_string()],
                });
            }
        }

        // Check for indirect recursion (can reach itself through other nodes)
        let path = GraphAlgorithms::find_path(&self.engine, node_id, node_id);
        if path.found && path.path.len() > 1 {
            if let Some(node) = self.get_node(node_id) {
                return Some(RecursionInfo {
                    node,
                    recursion_type: RecursionType::Indirect,
                    cycle: path.path,
                });
            }
        }

        None
    }

    // ============================================================================
    // Statistics
    // ============================================================================

    /// Get the most called functions
    pub fn get_hotspots(&self, limit: usize) -> Vec<(CallNode, usize)> {
        let mut call_counts: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for node_id in self.engine.node_ids() {
            let incoming = self.engine.get_incoming_edges(&node_id);
            call_counts.insert(node_id, incoming.len());
        }

        let mut hotspots: Vec<_> = call_counts
            .into_iter()
            .filter_map(|(id, count)| self.get_node(&id).map(|n| (n, count)))
            .collect();

        hotspots.sort_by(|a, b| b.1.cmp(&a.1));
        hotspots.truncate(limit);
        hotspots
    }

    /// Get functions with no callers (entry points)
    pub fn get_entry_points(&self) -> Vec<CallNode> {
        self.get_nodes()
            .into_iter()
            .filter(|n| self.engine.get_incoming_edges(&n.id).is_empty())
            .collect()
    }

    /// Get functions with no callees (leaf functions)
    pub fn get_leaf_functions(&self) -> Vec<CallNode> {
        self.get_nodes()
            .into_iter()
            .filter(|n| self.engine.get_outgoing_edges(&n.id).is_empty())
            .collect()
    }

    /// Get graph statistics
    pub fn stats(&self) -> CallGraphStats {
        let nodes = self.get_nodes();
        let recursion = self.detect_recursion();

        let mut max_depth = 0;
        for node in &nodes {
            let chain = GraphAlgorithms::get_reachable(&self.engine, &node.id, 100);
            max_depth = max_depth.max(chain.len());
        }

        CallGraphStats {
            total_functions: nodes.len(),
            total_calls: self.engine.edge_count(),
            entry_points: self.get_entry_points().len(),
            leaf_functions: self.get_leaf_functions().len(),
            recursive_functions: recursion.len(),
            max_call_depth: max_depth,
        }
    }

    // ============================================================================
    // Serialization
    // ============================================================================

    /// Serialize to JSON
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

    /// Access the underlying engine
    pub fn engine(&self) -> &GraphEngine {
        &self.engine
    }
}

/// Call graph statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallGraphStats {
    pub total_functions: usize,
    pub total_calls: usize,
    pub entry_points: usize,
    pub leaf_functions: usize,
    pub recursive_functions: usize,
    pub max_call_depth: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_function() {
        let mut graph = CallGraph::new("test-project");

        let node = graph.add_function("main", "src/main.rs", 10, 0, CallableKind::Function, None);

        assert!(node.id.contains("main"));
        assert_eq!(node.name, "main");
        assert_eq!(graph.get_nodes().len(), 1);
    }

    #[test]
    fn test_add_call() {
        let mut graph = CallGraph::new("test-project");

        let main = graph.add_function("main", "src/main.rs", 10, 0, CallableKind::Function, None);
        let helper =
            graph.add_function("helper", "src/main.rs", 20, 0, CallableKind::Function, None);

        let edge = graph.add_call(
            &main.id,
            &helper.id,
            vec![CallLocation {
                line: 12,
                character: 4,
            }],
        );

        assert!(edge.is_some());

        let callees = graph.get_callees(&main.id);
        assert_eq!(callees.len(), 1);
        assert_eq!(callees[0].name, "helper");
    }

    #[test]
    fn test_recursion_detection() {
        let mut graph = CallGraph::new("test-project");

        let recursive =
            graph.add_function("factorial", "src/math.rs", 10, 0, CallableKind::Function, None);

        // Direct recursion: factorial calls itself
        graph.add_call(
            &recursive.id,
            &recursive.id,
            vec![CallLocation {
                line: 15,
                character: 4,
            }],
        );

        let recursion = graph.detect_recursion();
        assert!(!recursion.is_empty());
        assert_eq!(recursion[0].recursion_type, RecursionType::Direct);
    }

    #[test]
    fn test_indirect_recursion() {
        let mut graph = CallGraph::new("test-project");

        let a = graph.add_function("a", "src/lib.rs", 10, 0, CallableKind::Function, None);
        let b = graph.add_function("b", "src/lib.rs", 20, 0, CallableKind::Function, None);
        let c = graph.add_function("c", "src/lib.rs", 30, 0, CallableKind::Function, None);

        // Indirect recursion: a -> b -> c -> a
        graph.add_call(&a.id, &b.id, vec![]);
        graph.add_call(&b.id, &c.id, vec![]);
        graph.add_call(&c.id, &a.id, vec![]);

        let recursion = graph.detect_recursion();
        assert!(!recursion.is_empty());
    }

    #[test]
    fn test_call_chain() {
        let mut graph = CallGraph::new("test-project");

        let main = graph.add_function("main", "src/main.rs", 1, 0, CallableKind::Function, None);
        let init = graph.add_function("init", "src/main.rs", 10, 0, CallableKind::Function, None);
        let setup =
            graph.add_function("setup", "src/main.rs", 20, 0, CallableKind::Function, None);
        let config =
            graph.add_function("config", "src/main.rs", 30, 0, CallableKind::Function, None);

        graph.add_call(&main.id, &init.id, vec![]);
        graph.add_call(&init.id, &setup.id, vec![]);
        graph.add_call(&setup.id, &config.id, vec![]);

        let chain = graph.get_call_chain(&main.id, 10).unwrap();

        assert_eq!(chain.nodes.len(), 4);
        assert_eq!(*chain.depth.get(&config.id).unwrap(), 3);
    }
}
