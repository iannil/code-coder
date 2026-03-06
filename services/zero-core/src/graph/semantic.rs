//! Semantic Graph - Code entity relationship tracking
//!
//! Tracks semantic relationships between code entities:
//! - Functions, classes, interfaces, types, enums, components
//! - Import/export relationships
//! - Inheritance and implementation
//! - References and containment

use serde::{Deserialize, Serialize};

use super::algorithms::GraphAlgorithms;
use super::engine::{EdgeData, EdgeRelationship, GraphEngine, NodeData, NodeId};

/// Type of semantic node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SemanticNodeType {
    Function,
    Class,
    Interface,
    Type,
    Enum,
    Component,
    File,
    Module,
}

impl SemanticNodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Class => "class",
            Self::Interface => "interface",
            Self::Type => "type",
            Self::Enum => "enum",
            Self::Component => "component",
            Self::File => "file",
            Self::Module => "module",
        }
    }
}

/// Type of semantic relationship
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SemanticEdgeType {
    Imports,
    Exports,
    Extends,
    Implements,
    Calls,
    Instantiates,
    References,
    Contains,
    Related,
}

impl From<SemanticEdgeType> for EdgeRelationship {
    fn from(edge_type: SemanticEdgeType) -> Self {
        match edge_type {
            SemanticEdgeType::Imports => EdgeRelationship::Imports,
            SemanticEdgeType::Exports => EdgeRelationship::Exports,
            SemanticEdgeType::Extends => EdgeRelationship::Extends,
            SemanticEdgeType::Implements => EdgeRelationship::Implements,
            SemanticEdgeType::Calls => EdgeRelationship::Calls,
            SemanticEdgeType::Instantiates => EdgeRelationship::Instantiates,
            SemanticEdgeType::References => EdgeRelationship::References,
            SemanticEdgeType::Contains => EdgeRelationship::Contains,
            SemanticEdgeType::Related => EdgeRelationship::Related,
        }
    }
}

/// A semantic node representing a code entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticNode {
    pub id: NodeId,
    pub node_type: SemanticNodeType,
    pub name: String,
    pub file: String,
    pub metadata: Option<serde_json::Value>,
}

/// A semantic edge representing a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticEdge {
    pub id: String,
    pub source: NodeId,
    pub target: NodeId,
    pub edge_type: SemanticEdgeType,
    pub weight: f64,
    pub metadata: Option<serde_json::Value>,
}

/// Node type constant
const NODE_TYPE_SEMANTIC: &str = "semantic";

/// Semantic Graph implementation
#[derive(Debug)]
pub struct SemanticGraph {
    engine: GraphEngine,
    project_id: String,
    created_at: i64,
    updated_at: i64,
}

impl SemanticGraph {
    /// Create a new semantic graph for a project
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
    fn node_id(node_type: SemanticNodeType, name: &str, file: &str) -> String {
        format!("{}:{}:{}", node_type.as_str(), name, file)
    }

    // ============================================================================
    // Node Operations
    // ============================================================================

    /// Add a semantic node to the graph
    pub fn add_node(&mut self, node: SemanticNode) -> NodeId {
        let id = node.id.clone();
        let node_data = NodeData::new(&id, NODE_TYPE_SEMANTIC, &node);
        self.engine.add_node(node_data);
        self.updated_at = chrono::Utc::now().timestamp_millis();
        id
    }

    /// Create and add a node
    pub fn add_entity(
        &mut self,
        node_type: SemanticNodeType,
        name: impl Into<String>,
        file: impl Into<String>,
        metadata: Option<serde_json::Value>,
    ) -> SemanticNode {
        let name = name.into();
        let file = file.into();
        let id = Self::node_id(node_type, &name, &file);

        let node = SemanticNode {
            id: id.clone(),
            node_type,
            name,
            file,
            metadata,
        };

        self.add_node(node.clone());
        node
    }

    /// Add a function node
    pub fn add_function(
        &mut self,
        name: impl Into<String>,
        file: impl Into<String>,
        signature: Option<&str>,
        exported: bool,
    ) -> SemanticNode {
        let metadata = serde_json::json!({
            "signature": signature,
            "exported": exported,
        });
        self.add_entity(SemanticNodeType::Function, name, file, Some(metadata))
    }

    /// Add a class node
    pub fn add_class(
        &mut self,
        name: impl Into<String>,
        file: impl Into<String>,
        extends: Option<&str>,
        methods: &[&str],
    ) -> SemanticNode {
        let metadata = serde_json::json!({
            "extends": extends,
            "methods": methods,
        });
        self.add_entity(SemanticNodeType::Class, name, file, Some(metadata))
    }

    /// Add an interface node
    pub fn add_interface(
        &mut self,
        name: impl Into<String>,
        file: impl Into<String>,
        extends: Option<&str>,
    ) -> SemanticNode {
        let metadata = serde_json::json!({
            "extends": extends,
        });
        self.add_entity(SemanticNodeType::Interface, name, file, Some(metadata))
    }

    /// Add a file node
    pub fn add_file(&mut self, path: impl Into<String>) -> SemanticNode {
        let path = path.into();
        self.add_entity(SemanticNodeType::File, &path, &path, None)
    }

    /// Get a node by ID
    pub fn get_node(&self, id: &str) -> Option<SemanticNode> {
        self.engine
            .get_node(id)
            .filter(|n| n.node_type == NODE_TYPE_SEMANTIC)
            .and_then(|n| n.get_payload())
    }

    /// Get all nodes
    pub fn get_nodes(&self) -> Vec<SemanticNode> {
        self.engine
            .nodes_by_type(NODE_TYPE_SEMANTIC)
            .into_iter()
            .filter_map(|n| n.get_payload())
            .collect()
    }

    /// Get nodes by type
    pub fn get_nodes_by_type(&self, node_type: SemanticNodeType) -> Vec<SemanticNode> {
        self.get_nodes()
            .into_iter()
            .filter(|n| n.node_type == node_type)
            .collect()
    }

    /// Find nodes by name
    pub fn find_by_name(&self, name: &str) -> Vec<SemanticNode> {
        self.get_nodes()
            .into_iter()
            .filter(|n| n.name == name)
            .collect()
    }

    /// Find nodes in a file
    pub fn find_in_file(&self, file: &str) -> Vec<SemanticNode> {
        self.get_nodes()
            .into_iter()
            .filter(|n| n.file == file)
            .collect()
    }

    // ============================================================================
    // Edge Operations
    // ============================================================================

    /// Add an edge between two nodes
    pub fn add_edge(
        &mut self,
        source_id: &str,
        target_id: &str,
        edge_type: SemanticEdgeType,
        weight: f64,
        metadata: Option<serde_json::Value>,
    ) -> Option<String> {
        let edge_id = format!("{}->{}:{:?}", source_id, target_id, edge_type);

        let mut edge_data = EdgeData::new(source_id, target_id, edge_type.into())
            .with_weight(weight);

        if let Some(m) = metadata {
            edge_data = edge_data.with_metadata(&SemanticEdge {
                id: edge_id.clone(),
                source: source_id.to_string(),
                target: target_id.to_string(),
                edge_type,
                weight,
                metadata: Some(m),
            });
        }

        self.engine.add_edge(edge_data)?;
        self.updated_at = chrono::Utc::now().timestamp_millis();
        Some(edge_id)
    }

    /// Add an import relationship
    pub fn add_import(&mut self, importer: &str, imported: &str) -> Option<String> {
        self.add_edge(importer, imported, SemanticEdgeType::Imports, 1.0, None)
    }

    /// Add an export relationship
    pub fn add_export(&mut self, file: &str, exported: &str) -> Option<String> {
        self.add_edge(file, exported, SemanticEdgeType::Exports, 1.0, None)
    }

    /// Add an extends relationship
    pub fn add_extends(&mut self, child: &str, parent: &str) -> Option<String> {
        self.add_edge(child, parent, SemanticEdgeType::Extends, 1.0, None)
    }

    /// Add an implements relationship
    pub fn add_implements(&mut self, implementor: &str, interface: &str) -> Option<String> {
        self.add_edge(implementor, interface, SemanticEdgeType::Implements, 1.0, None)
    }

    /// Add a contains relationship (e.g., file contains function)
    pub fn add_contains(&mut self, container: &str, contained: &str) -> Option<String> {
        self.add_edge(container, contained, SemanticEdgeType::Contains, 1.0, None)
    }

    /// Add a references relationship
    pub fn add_references(&mut self, referencer: &str, referenced: &str, weight: f64) -> Option<String> {
        self.add_edge(referencer, referenced, SemanticEdgeType::References, weight, None)
    }

    /// Get edges from a node
    pub fn get_outgoing_edges(&self, node_id: &str) -> Vec<SemanticEdge> {
        self.engine
            .get_outgoing_edges(node_id)
            .into_iter()
            .map(|e| SemanticEdge {
                id: e.id.clone(),
                source: e.source.clone(),
                target: e.target.clone(),
                edge_type: match e.relationship {
                    EdgeRelationship::Imports => SemanticEdgeType::Imports,
                    EdgeRelationship::Exports => SemanticEdgeType::Exports,
                    EdgeRelationship::Extends => SemanticEdgeType::Extends,
                    EdgeRelationship::Implements => SemanticEdgeType::Implements,
                    EdgeRelationship::Calls => SemanticEdgeType::Calls,
                    EdgeRelationship::Instantiates => SemanticEdgeType::Instantiates,
                    EdgeRelationship::References => SemanticEdgeType::References,
                    EdgeRelationship::Contains => SemanticEdgeType::Contains,
                    _ => SemanticEdgeType::Related,
                },
                weight: e.weight,
                metadata: e.metadata.clone(),
            })
            .collect()
    }

    /// Get edges to a node
    pub fn get_incoming_edges(&self, node_id: &str) -> Vec<SemanticEdge> {
        self.engine
            .get_incoming_edges(node_id)
            .into_iter()
            .map(|e| SemanticEdge {
                id: e.id.clone(),
                source: e.source.clone(),
                target: e.target.clone(),
                edge_type: match e.relationship {
                    EdgeRelationship::Imports => SemanticEdgeType::Imports,
                    EdgeRelationship::Exports => SemanticEdgeType::Exports,
                    EdgeRelationship::Extends => SemanticEdgeType::Extends,
                    EdgeRelationship::Implements => SemanticEdgeType::Implements,
                    EdgeRelationship::Calls => SemanticEdgeType::Calls,
                    EdgeRelationship::Instantiates => SemanticEdgeType::Instantiates,
                    EdgeRelationship::References => SemanticEdgeType::References,
                    EdgeRelationship::Contains => SemanticEdgeType::Contains,
                    _ => SemanticEdgeType::Related,
                },
                weight: e.weight,
                metadata: e.metadata.clone(),
            })
            .collect()
    }

    // ============================================================================
    // Analysis
    // ============================================================================

    /// Get all entities that import from a given entity
    pub fn get_importers(&self, entity_id: &str) -> Vec<SemanticNode> {
        self.engine
            .get_predecessors(entity_id)
            .into_iter()
            .filter_map(|id| self.get_node(&id))
            .collect()
    }

    /// Get all entities imported by a given entity
    pub fn get_imports(&self, entity_id: &str) -> Vec<SemanticNode> {
        self.get_outgoing_edges(entity_id)
            .into_iter()
            .filter(|e| e.edge_type == SemanticEdgeType::Imports)
            .filter_map(|e| self.get_node(&e.target))
            .collect()
    }

    /// Get the inheritance hierarchy for a class
    pub fn get_inheritance_chain(&self, class_id: &str) -> Vec<SemanticNode> {
        let mut chain = Vec::new();
        let mut current = Some(class_id.to_string());

        while let Some(id) = current.take() {
            if let Some(node) = self.get_node(&id) {
                chain.push(node);

                // Find parent
                let extends = self
                    .get_outgoing_edges(&id)
                    .into_iter()
                    .find(|e| e.edge_type == SemanticEdgeType::Extends)
                    .map(|e| e.target);

                current = extends;
            }
        }

        chain
    }

    /// Get all classes implementing an interface
    pub fn get_implementors(&self, interface_id: &str) -> Vec<SemanticNode> {
        self.engine
            .get_predecessors(interface_id)
            .into_iter()
            .filter_map(|id| {
                let edges = self.get_outgoing_edges(&id);
                if edges.iter().any(|e| {
                    e.edge_type == SemanticEdgeType::Implements && e.target == interface_id
                }) {
                    self.get_node(&id)
                } else {
                    None
                }
            })
            .collect()
    }

    /// Find related entities within a distance
    pub fn get_related(&self, entity_id: &str, max_depth: usize) -> Vec<SemanticNode> {
        GraphAlgorithms::get_reachable(&self.engine, entity_id, max_depth)
            .into_iter()
            .filter_map(|id| self.get_node(&id))
            .collect()
    }

    /// Check for circular dependencies
    pub fn has_circular_dependencies(&self) -> bool {
        GraphAlgorithms::has_cycles(&self.engine)
    }

    /// Detect circular dependency cycles
    pub fn detect_circular_dependencies(&self) -> Vec<Vec<SemanticNode>> {
        let cycle_result = GraphAlgorithms::detect_cycles(&self.engine);

        cycle_result
            .cycles
            .into_iter()
            .map(|cycle| {
                cycle
                    .into_iter()
                    .filter_map(|id| self.get_node(&id))
                    .collect()
            })
            .collect()
    }

    // ============================================================================
    // Statistics
    // ============================================================================

    /// Get graph statistics
    pub fn stats(&self) -> SemanticGraphStats {
        let nodes = self.get_nodes();
        let type_counts = GraphAlgorithms::count_by_type(&self.engine);

        let mut files = 0;
        let mut functions = 0;
        let mut classes = 0;
        let mut interfaces = 0;

        for node in &nodes {
            match node.node_type {
                SemanticNodeType::File => files += 1,
                SemanticNodeType::Function => functions += 1,
                SemanticNodeType::Class => classes += 1,
                SemanticNodeType::Interface => interfaces += 1,
                _ => {}
            }
        }

        SemanticGraphStats {
            total_nodes: nodes.len(),
            total_edges: self.engine.edge_count(),
            files,
            functions,
            classes,
            interfaces,
            type_distribution: type_counts,
            has_cycles: self.has_circular_dependencies(),
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

/// Semantic graph statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticGraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub files: usize,
    pub functions: usize,
    pub classes: usize,
    pub interfaces: usize,
    pub type_distribution: std::collections::HashMap<String, usize>,
    pub has_cycles: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_entities() {
        let mut graph = SemanticGraph::new("test-project");

        let func = graph.add_function("main", "src/main.ts", Some("() => void"), true);
        let class = graph.add_class("App", "src/app.ts", None, &["render", "init"]);
        let interface = graph.add_interface("IPlugin", "src/plugin.ts", None);

        assert_eq!(graph.get_nodes().len(), 3);
        assert_eq!(func.node_type, SemanticNodeType::Function);
        assert_eq!(class.node_type, SemanticNodeType::Class);
        assert_eq!(interface.node_type, SemanticNodeType::Interface);
    }

    #[test]
    fn test_inheritance() {
        let mut graph = SemanticGraph::new("test-project");

        let base = graph.add_class("Base", "src/base.ts", None, &[]);
        let derived = graph.add_class("Derived", "src/derived.ts", Some("Base"), &[]);

        graph.add_extends(&derived.id, &base.id);

        let chain = graph.get_inheritance_chain(&derived.id);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].name, "Derived");
        assert_eq!(chain[1].name, "Base");
    }

    #[test]
    fn test_imports() {
        let mut graph = SemanticGraph::new("test-project");

        let file_a = graph.add_file("src/a.ts");
        let file_b = graph.add_file("src/b.ts");

        graph.add_import(&file_a.id, &file_b.id);

        let imports = graph.get_imports(&file_a.id);
        assert_eq!(imports.len(), 1);
        assert_eq!(imports[0].name, "src/b.ts");

        let importers = graph.get_importers(&file_b.id);
        assert_eq!(importers.len(), 1);
        assert_eq!(importers[0].name, "src/a.ts");
    }

    #[test]
    fn test_circular_dependency_detection() {
        let mut graph = SemanticGraph::new("test-project");

        let a = graph.add_file("a.ts");
        let b = graph.add_file("b.ts");
        let c = graph.add_file("c.ts");

        // Create circular dependency: a -> b -> c -> a
        graph.add_import(&a.id, &b.id);
        graph.add_import(&b.id, &c.id);
        graph.add_import(&c.id, &a.id);

        assert!(graph.has_circular_dependencies());

        let cycles = graph.detect_circular_dependencies();
        assert!(!cycles.is_empty());
    }

    #[test]
    fn test_stats() {
        let mut graph = SemanticGraph::new("test-project");

        graph.add_file("src/index.ts");
        graph.add_function("main", "src/index.ts", None, true);
        graph.add_class("App", "src/app.ts", None, &[]);
        graph.add_interface("IService", "src/service.ts", None);

        let stats = graph.stats();

        assert_eq!(stats.total_nodes, 4);
        assert_eq!(stats.files, 1);
        assert_eq!(stats.functions, 1);
        assert_eq!(stats.classes, 1);
        assert_eq!(stats.interfaces, 1);
    }
}
