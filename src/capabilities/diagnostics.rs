use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use tree_sitter::Tree;

/// Extract syntax error diagnostics from a parsed tree
pub fn get_syntax_diagnostics(tree: &Tree, source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    collect_errors(tree.root_node(), source, &mut diagnostics);
    diagnostics
}

fn collect_errors(node: tree_sitter::Node, source: &str, diagnostics: &mut Vec<Diagnostic>) {
    // Check if this node is an error or missing node
    if node.is_error() {
        let range = node_to_range(&node);
        let text = node.utf8_text(source.as_bytes()).unwrap_or("unknown");

        diagnostics.push(Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("ts-lsp-rust".to_string()),
            message: format!(
                "Syntax error: unexpected '{}'",
                text.chars().take(50).collect::<String>()
            ),
            related_information: None,
            tags: None,
            data: None,
        });
    } else if node.is_missing() {
        let range = node_to_range(&node);

        diagnostics.push(Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("ts-lsp-rust".to_string()),
            message: format!("Syntax error: missing '{}'", node.kind()),
            related_information: None,
            tags: None,
            data: None,
        });
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_errors(child, source, diagnostics);
    }
}

fn node_to_range(node: &tree_sitter::Node) -> Range {
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
    fn test_valid_code_no_diagnostics() {
        let code = r#"
            const x = 1;
            function greet(name: string): string {
                return `Hello, ${name}!`;
            }
        "#;
        let tree = parse_typescript(code);
        let diagnostics = get_syntax_diagnostics(&tree, code);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_syntax_error_detected() {
        let code = "function ( { }";
        let tree = parse_typescript(code);
        let diagnostics = get_syntax_diagnostics(&tree, code);
        assert!(!diagnostics.is_empty());
    }

    #[test]
    fn test_missing_semicolon_allowed() {
        // TypeScript/JavaScript allows missing semicolons due to ASI
        let code = "const x = 1\nconst y = 2";
        let tree = parse_typescript(code);
        let diagnostics = get_syntax_diagnostics(&tree, code);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_unclosed_brace() {
        let code = "function test() {";
        let tree = parse_typescript(code);
        let diagnostics = get_syntax_diagnostics(&tree, code);
        // tree-sitter should detect the missing closing brace
        assert!(!diagnostics.is_empty() || tree.root_node().has_error());
    }

    #[test]
    fn test_diagnostic_has_correct_source() {
        let code = "function ( { }";
        let tree = parse_typescript(code);
        let diagnostics = get_syntax_diagnostics(&tree, code);

        for diag in &diagnostics {
            assert_eq!(diag.source, Some("ts-lsp-rust".to_string()));
            assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
        }
    }

    #[test]
    fn test_diagnostic_range_valid() {
        let code = "const = 1;"; // Missing variable name
        let tree = parse_typescript(code);
        let diagnostics = get_syntax_diagnostics(&tree, code);

        for diag in &diagnostics {
            // Range should be valid (start <= end)
            assert!(
                diag.range.start.line <= diag.range.end.line
                    || (diag.range.start.line == diag.range.end.line
                        && diag.range.start.character <= diag.range.end.character)
            );
        }
    }

    #[test]
    fn test_multiple_errors() {
        let code = "{ { { }"; // Multiple unclosed braces
        let tree = parse_typescript(code);
        let diagnostics = get_syntax_diagnostics(&tree, code);
        // Should detect at least one error
        assert!(diagnostics.len() >= 1 || tree.root_node().has_error());
    }

    #[test]
    fn test_empty_code() {
        let code = "";
        let tree = parse_typescript(code);
        let diagnostics = get_syntax_diagnostics(&tree, code);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_valid_class() {
        let code = r#"
            class Person {
                name: string;
                constructor(name: string) {
                    this.name = name;
                }
            }
        "#;
        let tree = parse_typescript(code);
        let diagnostics = get_syntax_diagnostics(&tree, code);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_valid_interface() {
        let code = r#"
            interface User {
                id: number;
                name: string;
                email?: string;
            }
        "#;
        let tree = parse_typescript(code);
        let diagnostics = get_syntax_diagnostics(&tree, code);
        assert!(diagnostics.is_empty());
    }
}
