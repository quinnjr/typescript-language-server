use tower_lsp::lsp_types::{DocumentSymbol, Position, Range, SymbolKind};
use tree_sitter::{Node, Tree};

/// Extract document symbols from a parsed tree
pub fn get_document_symbols(tree: &Tree, source: &str) -> Vec<DocumentSymbol> {
    let root = tree.root_node();
    extract_symbols(&root, source)
}

fn extract_symbols(node: &Node, source: &str) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(symbol) = node_to_symbol(&child, source) {
            symbols.push(symbol);
        } else {
            // Recursively search for symbols in non-symbol nodes
            symbols.extend(extract_symbols(&child, source));
        }
    }

    symbols
}

fn node_to_symbol(node: &Node, source: &str) -> Option<DocumentSymbol> {
    let kind = node.kind();

    let (name, symbol_kind, detail) = match kind {
        "function_declaration" => {
            let name = get_child_by_field(node, "name", source)?;
            (name, SymbolKind::FUNCTION, None)
        }
        "arrow_function" => {
            // Arrow functions often assigned to variables, try parent
            return None;
        }
        "class_declaration" => {
            let name = get_child_by_field(node, "name", source)?;
            (name, SymbolKind::CLASS, None)
        }
        "interface_declaration" => {
            let name = get_child_by_field(node, "name", source)?;
            (name, SymbolKind::INTERFACE, None)
        }
        "type_alias_declaration" => {
            let name = get_child_by_field(node, "name", source)?;
            (
                name,
                SymbolKind::TYPE_PARAMETER,
                Some("type alias".to_string()),
            )
        }
        "enum_declaration" => {
            let name = get_child_by_field(node, "name", source)?;
            (name, SymbolKind::ENUM, None)
        }
        "method_definition" => {
            let name = get_child_by_field(node, "name", source)?;
            (name, SymbolKind::METHOD, None)
        }
        "lexical_declaration" | "variable_declaration" => {
            // Handle const/let/var declarations
            return extract_variable_symbols(node, source);
        }
        "export_statement" => {
            // Look inside export statements for declarations
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if let Some(sym) = node_to_symbol(&child, source) {
                    return Some(sym);
                }
            }
            return None;
        }
        _ => return None,
    };

    let range = node_range(node);
    let selection_range = range;

    // Get children symbols
    let children = extract_symbols(node, source);
    let children = if children.is_empty() {
        None
    } else {
        Some(children)
    };

    #[allow(deprecated)]
    Some(DocumentSymbol {
        name,
        detail,
        kind: symbol_kind,
        tags: None,
        deprecated: None,
        range,
        selection_range,
        children,
    })
}

fn extract_variable_symbols(node: &Node, source: &str) -> Option<DocumentSymbol> {
    // Find the first variable declarator
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "variable_declarator" {
            let name = get_child_by_field(&child, "name", source)?;

            // Check if the value is an arrow function or function
            let symbol_kind = if let Some(value) = child.child_by_field_name("value") {
                match value.kind() {
                    "arrow_function" | "function" => SymbolKind::FUNCTION,
                    "class" => SymbolKind::CLASS,
                    _ => SymbolKind::VARIABLE,
                }
            } else {
                SymbolKind::VARIABLE
            };

            let range = node_range(node);

            #[allow(deprecated)]
            return Some(DocumentSymbol {
                name,
                detail: None,
                kind: symbol_kind,
                tags: None,
                deprecated: None,
                range,
                selection_range: range,
                children: None,
            });
        }
    }
    None
}

fn get_child_by_field(node: &Node, field: &str, source: &str) -> Option<String> {
    let child = node.child_by_field_name(field)?;
    Some(child.utf8_text(source.as_bytes()).ok()?.to_string())
}

fn node_range(node: &Node) -> Range {
    let start = node.start_position();
    let end = node.end_position();
    Range {
        start: Position::new(start.row as u32, start.column as u32),
        end: Position::new(end.row as u32, end.column as u32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::Parser;

    fn parse_typescript(code: &str) -> Tree {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        parser.parse(code, None).unwrap()
    }

    #[test]
    fn test_function_symbol() {
        let code = "function greet(name: string): string { return name; }";
        let tree = parse_typescript(code);
        let symbols = get_document_symbols(&tree, code);

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "greet");
        assert_eq!(symbols[0].kind, SymbolKind::FUNCTION);
    }

    #[test]
    fn test_class_symbol() {
        let code = "class Person { }";
        let tree = parse_typescript(code);
        let symbols = get_document_symbols(&tree, code);

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Person");
        assert_eq!(symbols[0].kind, SymbolKind::CLASS);
    }

    #[test]
    fn test_interface_symbol() {
        let code = "interface User { name: string; }";
        let tree = parse_typescript(code);
        let symbols = get_document_symbols(&tree, code);

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "User");
        assert_eq!(symbols[0].kind, SymbolKind::INTERFACE);
    }

    #[test]
    fn test_type_alias_symbol() {
        let code = "type StringOrNumber = string | number;";
        let tree = parse_typescript(code);
        let symbols = get_document_symbols(&tree, code);

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "StringOrNumber");
        assert_eq!(symbols[0].kind, SymbolKind::TYPE_PARAMETER);
    }

    #[test]
    fn test_enum_symbol() {
        let code = "enum Color { Red, Green, Blue }";
        let tree = parse_typescript(code);
        let symbols = get_document_symbols(&tree, code);

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Color");
        assert_eq!(symbols[0].kind, SymbolKind::ENUM);
    }

    #[test]
    fn test_const_variable_symbol() {
        let code = "const x = 42;";
        let tree = parse_typescript(code);
        let symbols = get_document_symbols(&tree, code);

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "x");
        assert_eq!(symbols[0].kind, SymbolKind::VARIABLE);
    }

    #[test]
    fn test_let_variable_symbol() {
        let code = "let y = 'hello';";
        let tree = parse_typescript(code);
        let symbols = get_document_symbols(&tree, code);

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "y");
        assert_eq!(symbols[0].kind, SymbolKind::VARIABLE);
    }

    #[test]
    fn test_arrow_function_variable() {
        let code = "const greet = (name: string) => `Hello, ${name}!`;";
        let tree = parse_typescript(code);
        let symbols = get_document_symbols(&tree, code);

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "greet");
        assert_eq!(symbols[0].kind, SymbolKind::FUNCTION);
    }

    #[test]
    fn test_multiple_symbols() {
        let code = r#"
            interface User { id: number; }
            class UserService { }
            function getUser(id: number): User { }
            const MAX_USERS = 100;
        "#;
        let tree = parse_typescript(code);
        let symbols = get_document_symbols(&tree, code);

        assert_eq!(symbols.len(), 4);

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"User"));
        assert!(names.contains(&"UserService"));
        assert!(names.contains(&"getUser"));
        assert!(names.contains(&"MAX_USERS"));
    }

    #[test]
    fn test_class_with_methods() {
        let code = r#"
            class Calculator {
                add(a: number, b: number): number {
                    return a + b;
                }
                subtract(a: number, b: number): number {
                    return a - b;
                }
            }
        "#;
        let tree = parse_typescript(code);
        let symbols = get_document_symbols(&tree, code);

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "Calculator");
        assert_eq!(symbols[0].kind, SymbolKind::CLASS);

        // Check for child method symbols
        let children = symbols[0].children.as_ref().unwrap();
        assert_eq!(children.len(), 2);

        let method_names: Vec<&str> = children.iter().map(|s| s.name.as_str()).collect();
        assert!(method_names.contains(&"add"));
        assert!(method_names.contains(&"subtract"));
    }

    #[test]
    fn test_exported_function() {
        let code = "export function hello() { }";
        let tree = parse_typescript(code);
        let symbols = get_document_symbols(&tree, code);

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "hello");
        assert_eq!(symbols[0].kind, SymbolKind::FUNCTION);
    }

    #[test]
    fn test_exported_class() {
        let code = "export class MyClass { }";
        let tree = parse_typescript(code);
        let symbols = get_document_symbols(&tree, code);

        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "MyClass");
        assert_eq!(symbols[0].kind, SymbolKind::CLASS);
    }

    #[test]
    fn test_empty_code() {
        let code = "";
        let tree = parse_typescript(code);
        let symbols = get_document_symbols(&tree, code);
        assert!(symbols.is_empty());
    }

    #[test]
    fn test_only_imports() {
        let code = r#"
            import { foo } from 'bar';
            import * as baz from 'qux';
        "#;
        let tree = parse_typescript(code);
        let symbols = get_document_symbols(&tree, code);
        // Imports are not included in document symbols
        assert!(symbols.is_empty());
    }

    #[test]
    fn test_symbol_range() {
        let code = "function test() { }";
        let tree = parse_typescript(code);
        let symbols = get_document_symbols(&tree, code);

        assert_eq!(symbols.len(), 1);
        let range = symbols[0].range;
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
    }
}
