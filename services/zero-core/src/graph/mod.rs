//! Graph module - Unified graph engine for knowledge representation
//!
//! This module provides high-performance graph algorithms using petgraph:
//! - **engine**: Core GraphEngine with DiGraph backend
//! - **algorithms**: BFS, DFS, cycle detection, topological sort, path finding
//! - **causal**: Decision → Action → Outcome causal chains
//! - **call**: Function call graph for code analysis
//! - **semantic**: Semantic relationships between code entities

pub mod algorithms;
pub mod call;
pub mod causal;
pub mod engine;
pub mod semantic;

// Re-exports for convenience
pub use algorithms::{GraphAlgorithms, PathResult, CycleResult};
pub use call::{CallEdge, CallGraph, CallNode, RecursionInfo};
pub use causal::{
    ActionNode, CausalChain, CausalEdge, CausalGraph, CausalQuery, CausalStats, DecisionNode,
    OutcomeNode, OutcomeStatus,
};
pub use engine::{EdgeData, GraphEngine, NodeData, NodeId, EdgeId};
pub use semantic::{SemanticEdge, SemanticEdgeType, SemanticGraph, SemanticNode, SemanticNodeType};
