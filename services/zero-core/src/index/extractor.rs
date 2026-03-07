//! Symbol Extractor Module
//!
//! Extracts code symbols from tree-sitter syntax trees for each language.

use tree_sitter::Node;

use super::{CodeSymbol, SymbolKind};

// ============================================================================
// Helper Functions
// ============================================================================

/// Get text from a node
fn node_text<'a>(node: &Node, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

/// Get optional child by field name
fn child_by_field<'a>(node: &'a Node, field: &str) -> Option<Node<'a>> {
    node.child_by_field_name(field)
}

/// Create a basic symbol from a node
fn make_symbol(name: String, kind: SymbolKind, node: &Node) -> CodeSymbol {
    CodeSymbol {
        name,
        kind,
        start_line: node.start_position().row as u32 + 1,
        end_line: node.end_position().row as u32 + 1,
        start_column: node.start_position().column as u32,
        end_column: node.end_position().column as u32,
        parent: None,
        doc_comment: None,
        parameters: Vec::new(),
        return_type: None,
        is_exported: false,
        is_async: false,
    }
}

// ============================================================================
// JavaScript/TypeScript Extractor
// ============================================================================

pub fn extract_js_ts_symbols(node: &Node, source: &str, symbols: &mut Vec<CodeSymbol>) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_declaration" | "function" => {
                if let Some(name_node) = child_by_field(&child, "name") {
                    let name = node_text(&name_node, source).to_string();
                    let mut symbol = make_symbol(name, SymbolKind::Function, &child);

                    // Check for async
                    for c in child.children(&mut child.walk()) {
                        if c.kind() == "async" {
                            symbol.is_async = true;
                            break;
                        }
                    }

                    // Extract parameters
                    if let Some(params) = child_by_field(&child, "parameters") {
                        symbol.parameters = extract_parameters(&params, source);
                    }

                    symbols.push(symbol);
                }
            }
            "arrow_function" => {
                // Arrow functions are usually assigned to variables
                // The parent would be a variable_declarator
            }
            "variable_declaration" | "lexical_declaration" => {
                for declarator in child.children(&mut child.walk()) {
                    if declarator.kind() == "variable_declarator" {
                        if let Some(name_node) = child_by_field(&declarator, "name") {
                            let name = node_text(&name_node, source).to_string();

                            // Check if value is an arrow function
                            if let Some(value) = child_by_field(&declarator, "value") {
                                if value.kind() == "arrow_function" {
                                    let mut symbol = make_symbol(name, SymbolKind::Function, &declarator);
                                    symbol.is_async = value.child(0).map(|c| c.kind() == "async").unwrap_or(false);

                                    if let Some(params) = child_by_field(&value, "parameters") {
                                        symbol.parameters = extract_parameters(&params, source);
                                    }

                                    symbols.push(symbol);
                                } else {
                                    // Regular variable/constant
                                    let kind = if child.kind() == "lexical_declaration"
                                        && child.child(0).map(|c| c.kind() == "const").unwrap_or(false)
                                    {
                                        SymbolKind::Constant
                                    } else {
                                        SymbolKind::Variable
                                    };
                                    symbols.push(make_symbol(name, kind, &declarator));
                                }
                            }
                        }
                    }
                }
            }
            "class_declaration" | "class" => {
                if let Some(name_node) = child_by_field(&child, "name") {
                    let class_name = node_text(&name_node, source).to_string();
                    symbols.push(make_symbol(class_name.clone(), SymbolKind::Class, &child));

                    // Extract methods
                    if let Some(body) = child_by_field(&child, "body") {
                        for member in body.children(&mut body.walk()) {
                            if member.kind() == "method_definition" || member.kind() == "public_field_definition" {
                                if let Some(mname) = child_by_field(&member, "name") {
                                    let name = node_text(&mname, source).to_string();
                                    let kind = if member.kind() == "method_definition" {
                                        SymbolKind::Method
                                    } else {
                                        SymbolKind::Property
                                    };
                                    let mut symbol = make_symbol(name, kind, &member);
                                    symbol.parent = Some(class_name.clone());
                                    symbols.push(symbol);
                                }
                            }
                        }
                    }
                }
            }
            "interface_declaration" => {
                if let Some(name_node) = child_by_field(&child, "name") {
                    let name = node_text(&name_node, source).to_string();
                    symbols.push(make_symbol(name, SymbolKind::Interface, &child));
                }
            }
            "type_alias_declaration" => {
                if let Some(name_node) = child_by_field(&child, "name") {
                    let name = node_text(&name_node, source).to_string();
                    symbols.push(make_symbol(name, SymbolKind::Type, &child));
                }
            }
            "enum_declaration" => {
                if let Some(name_node) = child_by_field(&child, "name") {
                    let name = node_text(&name_node, source).to_string();
                    symbols.push(make_symbol(name, SymbolKind::Enum, &child));
                }
            }
            "export_statement" => {
                // Export wraps other declarations
                for export_child in child.children(&mut child.walk()) {
                    let mut child_symbols = Vec::new();
                    extract_js_ts_symbols(&export_child, source, &mut child_symbols);
                    for mut s in child_symbols {
                        s.is_exported = true;
                        symbols.push(s);
                    }
                }
            }
            "import_statement" => {
                let mut symbol = make_symbol("import".to_string(), SymbolKind::Import, &child);
                symbol.name = node_text(&child, source).to_string();
                symbols.push(symbol);
            }
            _ => {
                // Recurse into nested structures
                extract_js_ts_symbols(&child, source, symbols);
            }
        }
    }
}

// ============================================================================
// Python Extractor
// ============================================================================

pub fn extract_python_symbols(node: &Node, source: &str, symbols: &mut Vec<CodeSymbol>) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_definition" => {
                if let Some(name_node) = child_by_field(&child, "name") {
                    let name = node_text(&name_node, source).to_string();
                    let mut symbol = make_symbol(name, SymbolKind::Function, &child);

                    // Check for async
                    if child.child(0).map(|c| c.kind() == "async").unwrap_or(false) {
                        symbol.is_async = true;
                    }

                    // Extract parameters
                    if let Some(params) = child_by_field(&child, "parameters") {
                        symbol.parameters = extract_python_parameters(&params, source);
                    }

                    symbols.push(symbol);
                }
            }
            "class_definition" => {
                if let Some(name_node) = child_by_field(&child, "name") {
                    let class_name = node_text(&name_node, source).to_string();
                    symbols.push(make_symbol(class_name.clone(), SymbolKind::Class, &child));

                    // Extract methods
                    if let Some(body) = child_by_field(&child, "body") {
                        for member in body.children(&mut body.walk()) {
                            if member.kind() == "function_definition" {
                                if let Some(mname) = child_by_field(&member, "name") {
                                    let name = node_text(&mname, source).to_string();
                                    let mut symbol = make_symbol(name, SymbolKind::Method, &member);
                                    symbol.parent = Some(class_name.clone());
                                    symbols.push(symbol);
                                }
                            }
                        }
                    }
                }
            }
            "import_statement" | "import_from_statement" => {
                let mut symbol = make_symbol("import".to_string(), SymbolKind::Import, &child);
                symbol.name = node_text(&child, source).to_string();
                symbols.push(symbol);
            }
            _ => {
                extract_python_symbols(&child, source, symbols);
            }
        }
    }
}

// ============================================================================
// Go Extractor
// ============================================================================

pub fn extract_go_symbols(node: &Node, source: &str, symbols: &mut Vec<CodeSymbol>) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_declaration" => {
                if let Some(name_node) = child_by_field(&child, "name") {
                    let name = node_text(&name_node, source).to_string();
                    let mut symbol = make_symbol(name, SymbolKind::Function, &child);

                    // Check if exported (starts with uppercase)
                    symbol.is_exported = symbol.name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);

                    symbols.push(symbol);
                }
            }
            "method_declaration" => {
                if let Some(name_node) = child_by_field(&child, "name") {
                    let name = node_text(&name_node, source).to_string();
                    let mut symbol = make_symbol(name, SymbolKind::Method, &child);
                    symbol.is_exported = symbol.name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);

                    // Get receiver type as parent
                    if let Some(receiver) = child_by_field(&child, "receiver") {
                        symbol.parent = Some(node_text(&receiver, source).to_string());
                    }

                    symbols.push(symbol);
                }
            }
            "type_declaration" => {
                for spec in child.children(&mut child.walk()) {
                    if spec.kind() == "type_spec" {
                        if let Some(name_node) = child_by_field(&spec, "name") {
                            let name = node_text(&name_node, source).to_string();
                            let kind = if spec.child_by_field_name("type").map(|t| t.kind() == "struct_type").unwrap_or(false) {
                                SymbolKind::Struct
                            } else if spec.child_by_field_name("type").map(|t| t.kind() == "interface_type").unwrap_or(false) {
                                SymbolKind::Interface
                            } else {
                                SymbolKind::Type
                            };
                            let mut symbol = make_symbol(name, kind, &spec);
                            symbol.is_exported = symbol.name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
                            symbols.push(symbol);
                        }
                    }
                }
            }
            "import_declaration" => {
                let mut symbol = make_symbol("import".to_string(), SymbolKind::Import, &child);
                symbol.name = node_text(&child, source).to_string();
                symbols.push(symbol);
            }
            _ => {
                extract_go_symbols(&child, source, symbols);
            }
        }
    }
}

// ============================================================================
// Rust Extractor
// ============================================================================

pub fn extract_rust_symbols(node: &Node, source: &str, symbols: &mut Vec<CodeSymbol>) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        match child.kind() {
            "function_item" => {
                if let Some(name_node) = child_by_field(&child, "name") {
                    let name = node_text(&name_node, source).to_string();
                    let mut symbol = make_symbol(name, SymbolKind::Function, &child);

                    // Check for pub
                    for c in child.children(&mut child.walk()) {
                        if c.kind() == "visibility_modifier" {
                            symbol.is_exported = true;
                            break;
                        }
                        if c.kind() == "async" {
                            symbol.is_async = true;
                        }
                    }

                    symbols.push(symbol);
                }
            }
            "struct_item" => {
                if let Some(name_node) = child_by_field(&child, "name") {
                    let name = node_text(&name_node, source).to_string();
                    let mut symbol = make_symbol(name, SymbolKind::Struct, &child);

                    for c in child.children(&mut child.walk()) {
                        if c.kind() == "visibility_modifier" {
                            symbol.is_exported = true;
                            break;
                        }
                    }

                    symbols.push(symbol);
                }
            }
            "enum_item" => {
                if let Some(name_node) = child_by_field(&child, "name") {
                    let name = node_text(&name_node, source).to_string();
                    let mut symbol = make_symbol(name, SymbolKind::Enum, &child);

                    for c in child.children(&mut child.walk()) {
                        if c.kind() == "visibility_modifier" {
                            symbol.is_exported = true;
                            break;
                        }
                    }

                    symbols.push(symbol);
                }
            }
            "trait_item" => {
                if let Some(name_node) = child_by_field(&child, "name") {
                    let name = node_text(&name_node, source).to_string();
                    let mut symbol = make_symbol(name, SymbolKind::Interface, &child);

                    for c in child.children(&mut child.walk()) {
                        if c.kind() == "visibility_modifier" {
                            symbol.is_exported = true;
                            break;
                        }
                    }

                    symbols.push(symbol);
                }
            }
            "impl_item" => {
                // Extract methods from impl blocks
                let type_name = child_by_field(&child, "type")
                    .map(|t| node_text(&t, source).to_string())
                    .unwrap_or_default();

                if let Some(body) = child_by_field(&child, "body") {
                    for item in body.children(&mut body.walk()) {
                        if item.kind() == "function_item" {
                            if let Some(name_node) = child_by_field(&item, "name") {
                                let name = node_text(&name_node, source).to_string();
                                let mut symbol = make_symbol(name, SymbolKind::Method, &item);
                                symbol.parent = Some(type_name.clone());

                                for c in item.children(&mut item.walk()) {
                                    if c.kind() == "visibility_modifier" {
                                        symbol.is_exported = true;
                                        break;
                                    }
                                }

                                symbols.push(symbol);
                            }
                        }
                    }
                }
            }
            "mod_item" => {
                if let Some(name_node) = child_by_field(&child, "name") {
                    let name = node_text(&name_node, source).to_string();
                    symbols.push(make_symbol(name, SymbolKind::Module, &child));
                }
            }
            "use_declaration" => {
                let mut symbol = make_symbol("use".to_string(), SymbolKind::Import, &child);
                symbol.name = node_text(&child, source).to_string();
                symbols.push(symbol);
            }
            _ => {
                extract_rust_symbols(&child, source, symbols);
            }
        }
    }
}

// ============================================================================
// Bash Extractor
// ============================================================================

pub fn extract_bash_symbols(node: &Node, source: &str, symbols: &mut Vec<CodeSymbol>) {
    let mut cursor = node.walk();

    for child in node.children(&mut cursor) {
        if child.kind() == "function_definition" {
            if let Some(name_node) = child_by_field(&child, "name") {
                let name = node_text(&name_node, source).to_string();
                symbols.push(make_symbol(name, SymbolKind::Function, &child));
            }
        } else {
            extract_bash_symbols(&child, source, symbols);
        }
    }
}

// ============================================================================
// Helper Extractors
// ============================================================================

fn extract_parameters(params_node: &Node, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = params_node.walk();

    for child in params_node.children(&mut cursor) {
        match child.kind() {
            "formal_parameters" | "required_parameter" | "optional_parameter" | "identifier" | "pattern" => {
                let param_text = node_text(&child, source);
                if !param_text.is_empty() && param_text != "(" && param_text != ")" && param_text != "," {
                    params.push(param_text.to_string());
                }
            }
            _ => {}
        }
    }

    params
}

fn extract_python_parameters(params_node: &Node, source: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut cursor = params_node.walk();

    for child in params_node.children(&mut cursor) {
        if child.kind() == "identifier" {
            let param_text = node_text(&child, source);
            if param_text != "self" && param_text != "cls" {
                params.push(param_text.to_string());
            }
        } else if child.kind() == "typed_parameter" || child.kind() == "default_parameter" {
            if let Some(name) = child_by_field(&child, "name") {
                let param_text = node_text(&name, source);
                if param_text != "self" && param_text != "cls" {
                    params.push(param_text.to_string());
                }
            }
        }
    }

    params
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_js_function() {
        use tree_sitter::Parser;

        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_javascript::LANGUAGE.into()).unwrap();

        let code = "function hello(name) { return 'Hi ' + name; }";
        let tree = parser.parse(code, None).unwrap();

        let mut symbols = Vec::new();
        extract_js_ts_symbols(&tree.root_node(), code, &mut symbols);

        assert!(!symbols.is_empty());
        assert!(symbols.iter().any(|s| s.name == "hello"));
    }

    #[test]
    fn test_extract_python_class() {
        use tree_sitter::Parser;

        let mut parser = Parser::new();
        parser.set_language(&tree_sitter_python::LANGUAGE.into()).unwrap();

        let code = r#"
class Greeter:
    def greet(self, name):
        return f"Hello, {name}!"
"#;
        let tree = parser.parse(code, None).unwrap();

        let mut symbols = Vec::new();
        extract_python_symbols(&tree.root_node(), code, &mut symbols);

        assert!(symbols.iter().any(|s| s.name == "Greeter" && matches!(s.kind, SymbolKind::Class)));
        assert!(symbols.iter().any(|s| s.name == "greet" && matches!(s.kind, SymbolKind::Method)));
    }
}
