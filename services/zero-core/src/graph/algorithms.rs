//! Graph Algorithms - High-performance traversal and analysis
//!
//! Provides graph algorithms implemented with petgraph:
//! - BFS/DFS traversal
//! - Cycle detection
//! - Topological sorting
//! - Path finding
//! - Reachability analysis

use std::collections::{HashSet, VecDeque};

use petgraph::algo::{has_path_connecting, is_cyclic_directed, toposort};
use petgraph::visit::{Bfs, Dfs, Reversed};
use petgraph::Direction;

use super::engine::{GraphEngine, NodeId};

/// Result of a path finding operation
#[derive(Debug, Clone)]
pub struct PathResult {
    /// Whether a path exists
    pub found: bool,
    /// The path from source to target (empty if not found)
    pub path: Vec<NodeId>,
    /// Number of nodes visited during search
    pub nodes_visited: usize,
}

/// Result of cycle detection
#[derive(Debug, Clone)]
pub struct CycleResult {
    /// Whether the graph contains cycles
    pub has_cycles: bool,
    /// Detected cycles (each cycle is a list of node IDs)
    pub cycles: Vec<Vec<NodeId>>,
}

/// Graph algorithm operations
pub struct GraphAlgorithms;

impl GraphAlgorithms {
    /// Breadth-first search from a starting node
    ///
    /// Returns all reachable nodes in BFS order
    pub fn bfs(engine: &GraphEngine, start: &str) -> Vec<NodeId> {
        let Some(start_idx) = engine.get_node_index(start) else {
            return Vec::new();
        };

        let graph = engine.inner();
        let mut bfs = Bfs::new(graph, start_idx);
        let mut result = Vec::new();

        while let Some(node_idx) = bfs.next(graph) {
            if let Some(node) = graph.node_weight(node_idx) {
                result.push(node.id.clone());
            }
        }

        result
    }

    /// Depth-first search from a starting node
    ///
    /// Returns all reachable nodes in DFS order
    pub fn dfs(engine: &GraphEngine, start: &str) -> Vec<NodeId> {
        let Some(start_idx) = engine.get_node_index(start) else {
            return Vec::new();
        };

        let graph = engine.inner();
        let mut dfs = Dfs::new(graph, start_idx);
        let mut result = Vec::new();

        while let Some(node_idx) = dfs.next(graph) {
            if let Some(node) = graph.node_weight(node_idx) {
                result.push(node.id.clone());
            }
        }

        result
    }

    /// Find a path from source to target using BFS
    ///
    /// Returns the shortest path (by edge count)
    pub fn find_path(engine: &GraphEngine, from: &str, to: &str) -> PathResult {
        let Some(from_idx) = engine.get_node_index(from) else {
            return PathResult {
                found: false,
                path: Vec::new(),
                nodes_visited: 0,
            };
        };

        let Some(to_idx) = engine.get_node_index(to) else {
            return PathResult {
                found: false,
                path: Vec::new(),
                nodes_visited: 0,
            };
        };

        let graph = engine.inner();

        // Quick check if path exists
        if !has_path_connecting(graph, from_idx, to_idx, None) {
            return PathResult {
                found: false,
                path: Vec::new(),
                nodes_visited: 0,
            };
        }

        // BFS to find actual path
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        let mut parent = std::collections::HashMap::new();
        let mut nodes_visited = 0;

        queue.push_back(from_idx);
        visited.insert(from_idx);

        while let Some(current) = queue.pop_front() {
            nodes_visited += 1;

            if current == to_idx {
                // Reconstruct path
                let mut path = Vec::new();
                let mut curr = to_idx;
                while let Some(&prev) = parent.get(&curr) {
                    if let Some(node) = graph.node_weight(curr) {
                        path.push(node.id.clone());
                    }
                    curr = prev;
                }
                if let Some(node) = graph.node_weight(from_idx) {
                    path.push(node.id.clone());
                }
                path.reverse();

                return PathResult {
                    found: true,
                    path,
                    nodes_visited,
                };
            }

            for neighbor in graph.neighbors_directed(current, Direction::Outgoing) {
                if visited.insert(neighbor) {
                    parent.insert(neighbor, current);
                    queue.push_back(neighbor);
                }
            }
        }

        PathResult {
            found: false,
            path: Vec::new(),
            nodes_visited,
        }
    }

    /// Check if the graph contains any cycles
    pub fn has_cycles(engine: &GraphEngine) -> bool {
        is_cyclic_directed(engine.inner())
    }

    /// Detect all cycles in the graph
    ///
    /// Uses Tarjan's algorithm internally
    pub fn detect_cycles(engine: &GraphEngine) -> CycleResult {
        let graph = engine.inner();

        // Quick check
        if !is_cyclic_directed(graph) {
            return CycleResult {
                has_cycles: false,
                cycles: Vec::new(),
            };
        }

        // Find strongly connected components with >1 node
        let sccs = petgraph::algo::tarjan_scc(graph);
        let mut cycles = Vec::new();

        for scc in sccs {
            if scc.len() > 1 {
                // This is a cycle
                let cycle: Vec<NodeId> = scc
                    .iter()
                    .filter_map(|&idx| graph.node_weight(idx).map(|n| n.id.clone()))
                    .collect();
                cycles.push(cycle);
            } else if scc.len() == 1 {
                // Check for self-loop
                let idx = scc[0];
                if graph.neighbors_directed(idx, Direction::Outgoing).any(|n| n == idx) {
                    if let Some(node) = graph.node_weight(idx) {
                        cycles.push(vec![node.id.clone()]);
                    }
                }
            }
        }

        CycleResult {
            has_cycles: !cycles.is_empty(),
            cycles,
        }
    }

    /// Topological sort of the graph
    ///
    /// Returns None if the graph contains cycles
    pub fn topological_sort(engine: &GraphEngine) -> Option<Vec<NodeId>> {
        let graph = engine.inner();

        toposort(graph, None).ok().map(|indices| {
            indices
                .into_iter()
                .filter_map(|idx| graph.node_weight(idx).map(|n| n.id.clone()))
                .collect()
        })
    }

    /// Get all nodes reachable from a starting node within a depth limit
    pub fn get_reachable(engine: &GraphEngine, start: &str, max_depth: usize) -> Vec<NodeId> {
        let Some(start_idx) = engine.get_node_index(start) else {
            return Vec::new();
        };

        let graph = engine.inner();
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut queue: VecDeque<(_, usize)> = VecDeque::new();

        queue.push_back((start_idx, 0));
        visited.insert(start_idx);

        while let Some((node_idx, depth)) = queue.pop_front() {
            if let Some(node) = graph.node_weight(node_idx) {
                result.push(node.id.clone());
            }

            if depth < max_depth {
                for neighbor in graph.neighbors_directed(node_idx, Direction::Outgoing) {
                    if visited.insert(neighbor) {
                        queue.push_back((neighbor, depth + 1));
                    }
                }
            }
        }

        result
    }

    /// Get all nodes that can reach the target (reverse reachability)
    pub fn get_reverse_reachable(engine: &GraphEngine, target: &str, max_depth: usize) -> Vec<NodeId> {
        let Some(target_idx) = engine.get_node_index(target) else {
            return Vec::new();
        };

        let graph = engine.inner();
        let reversed = Reversed(graph);
        let mut visited = HashSet::new();
        let mut result = Vec::new();
        let mut bfs = Bfs::new(&reversed, target_idx);
        let mut depth_map = std::collections::HashMap::new();

        depth_map.insert(target_idx, 0usize);

        while let Some(node_idx) = bfs.next(&reversed) {
            let current_depth = depth_map.get(&node_idx).copied().unwrap_or(0);

            if current_depth > max_depth {
                continue;
            }

            if visited.insert(node_idx) {
                if let Some(node) = graph.node_weight(node_idx) {
                    result.push(node.id.clone());
                }

                // Set depth for neighbors
                for neighbor in graph.neighbors_directed(node_idx, Direction::Incoming) {
                    depth_map.entry(neighbor).or_insert(current_depth + 1);
                }
            }
        }

        result
    }

    /// Compute the shortest distances from a source to all reachable nodes
    pub fn shortest_distances(engine: &GraphEngine, start: &str) -> std::collections::HashMap<NodeId, usize> {
        let Some(start_idx) = engine.get_node_index(start) else {
            return std::collections::HashMap::new();
        };

        let graph = engine.inner();
        let mut distances = std::collections::HashMap::new();
        let mut queue = VecDeque::new();

        queue.push_back((start_idx, 0usize));

        while let Some((node_idx, dist)) = queue.pop_front() {
            if let Some(node) = graph.node_weight(node_idx) {
                if distances.contains_key(&node.id) {
                    continue;
                }
                distances.insert(node.id.clone(), dist);

                for neighbor in graph.neighbors_directed(node_idx, Direction::Outgoing) {
                    queue.push_back((neighbor, dist + 1));
                }
            }
        }

        distances
    }

    /// Count nodes by type
    pub fn count_by_type(engine: &GraphEngine) -> std::collections::HashMap<String, usize> {
        let mut counts = std::collections::HashMap::new();

        for node in engine.inner().node_weights() {
            *counts.entry(node.node_type.clone()).or_insert(0) += 1;
        }

        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::engine::{EdgeData, EdgeRelationship, NodeData};

    fn create_test_graph() -> GraphEngine {
        let mut engine = GraphEngine::new();

        // Create a diamond graph: a -> b, a -> c, b -> d, c -> d
        engine.add_node(NodeData::new("a", "test", &()));
        engine.add_node(NodeData::new("b", "test", &()));
        engine.add_node(NodeData::new("c", "test", &()));
        engine.add_node(NodeData::new("d", "test", &()));

        engine.add_edge(EdgeData::new("a", "b", EdgeRelationship::Calls));
        engine.add_edge(EdgeData::new("a", "c", EdgeRelationship::Calls));
        engine.add_edge(EdgeData::new("b", "d", EdgeRelationship::Calls));
        engine.add_edge(EdgeData::new("c", "d", EdgeRelationship::Calls));

        engine
    }

    #[test]
    fn test_bfs() {
        let engine = create_test_graph();
        let result = GraphAlgorithms::bfs(&engine, "a");

        assert_eq!(result.len(), 4);
        assert_eq!(result[0], "a"); // Start node first
    }

    #[test]
    fn test_dfs() {
        let engine = create_test_graph();
        let result = GraphAlgorithms::dfs(&engine, "a");

        assert_eq!(result.len(), 4);
        assert_eq!(result[0], "a");
    }

    #[test]
    fn test_find_path() {
        let engine = create_test_graph();

        let result = GraphAlgorithms::find_path(&engine, "a", "d");
        assert!(result.found);
        assert!(result.path.len() >= 3); // a -> b/c -> d

        let no_path = GraphAlgorithms::find_path(&engine, "d", "a");
        assert!(!no_path.found);
    }

    #[test]
    fn test_cycle_detection() {
        let mut engine = create_test_graph();

        // No cycles initially
        assert!(!GraphAlgorithms::has_cycles(&engine));

        // Add a cycle: d -> a
        engine.add_edge(EdgeData::new("d", "a", EdgeRelationship::Calls));

        assert!(GraphAlgorithms::has_cycles(&engine));

        let cycles = GraphAlgorithms::detect_cycles(&engine);
        assert!(cycles.has_cycles);
        assert!(!cycles.cycles.is_empty());
    }

    #[test]
    fn test_topological_sort() {
        let engine = create_test_graph();

        let sorted = GraphAlgorithms::topological_sort(&engine);
        assert!(sorted.is_some());

        let sorted = sorted.unwrap();
        assert_eq!(sorted.len(), 4);

        // Verify a comes before b and c, and d comes last
        let pos_a = sorted.iter().position(|x| x == "a").unwrap();
        let pos_d = sorted.iter().position(|x| x == "d").unwrap();
        assert!(pos_a < pos_d);
    }

    #[test]
    fn test_reachable() {
        let engine = create_test_graph();

        let reachable = GraphAlgorithms::get_reachable(&engine, "a", 1);
        assert_eq!(reachable.len(), 3); // a, b, c (depth 1)

        let reachable_all = GraphAlgorithms::get_reachable(&engine, "a", 10);
        assert_eq!(reachable_all.len(), 4);
    }
}
