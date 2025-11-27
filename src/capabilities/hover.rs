use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Range};
use tree_sitter::Tree;

/// Get hover information for a position in the document
pub fn get_hover(tree: &Tree, source: &str, position: Position) -> Option<Hover> {
    let root = tree.root_node();

    // Find the node at the given position
    let point = tree_sitter::Point {
        row: position.line as usize,
        column: position.character as usize,
    };

    let node = root.descendant_for_point_range(point, point)?;

    // Get JSDoc comment if available
    let jsdoc = find_jsdoc_comment(&node, source);

    // Build hover content
    let mut content = String::new();

    // Show the node kind and text
    let node_text = node.utf8_text(source.as_bytes()).ok()?;
    let node_kind = node.kind();

    // Get parent context
    let parent_kind = node.parent().map(|p| p.kind()).unwrap_or("root");

    content.push_str(&format!("**{}**", get_display_kind(node_kind, parent_kind)));

    // Add the identifier name if it's short enough
    if node_text.len() <= 50 && !node_text.contains('\n') {
        content.push_str(&format!(": `{}`", node_text));
    }

    // Add JSDoc if found
    if let Some(doc) = jsdoc {
        content.push_str("\n\n---\n\n");
        content.push_str(&doc);
    }

    // Add syntax tree path for debugging
    content.push_str("\n\n---\n\n");
    content.push_str(&format!("*Node: {} → {}*", parent_kind, node_kind));

    let range = Range {
        start: Position::new(
            node.start_position().row as u32,
            node.start_position().column as u32,
        ),
        end: Position::new(
            node.end_position().row as u32,
            node.end_position().column as u32,
        ),
    };

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: content,
        }),
        range: Some(range),
    })
}

/// Find JSDoc comment associated with a node
fn find_jsdoc_comment(node: &tree_sitter::Node, source: &str) -> Option<String> {
    // Look for comment in previous siblings or parent's previous siblings
    let mut current = *node;

    // First, try to find the declaration this node belongs to
    while let Some(parent) = current.parent() {
        match parent.kind() {
            "function_declaration"
            | "class_declaration"
            | "method_definition"
            | "variable_declaration"
            | "lexical_declaration"
            | "interface_declaration"
            | "type_alias_declaration"
            | "enum_declaration" => {
                current = parent;
                break;
            }
            _ => current = parent,
        }
    }

    // Look for a comment before the declaration
    if let Some(prev) = current.prev_sibling() {
        if prev.kind() == "comment" {
            let text = prev.utf8_text(source.as_bytes()).ok()?;
            return parse_jsdoc(text);
        }
    }

    None
}

/// Parse JSDoc comment and extract documentation
fn parse_jsdoc(comment: &str) -> Option<String> {
    // Check if it's a JSDoc comment (starts with /**)
    if !comment.starts_with("/**") {
        return None;
    }

    let mut lines: Vec<&str> = comment.lines().collect();

    // Remove /** and */
    if let Some(first) = lines.first_mut() {
        *first = first.trim_start_matches("/**").trim();
    }
    if let Some(last) = lines.last_mut() {
        *last = last.trim_end_matches("*/").trim();
    }

    // Process each line
    let processed: Vec<String> = lines
        .iter()
        .map(|line| line.trim().trim_start_matches('*').trim_start().to_string())
        .filter(|line| !line.is_empty())
        .collect();

    if processed.is_empty() {
        return None;
    }

    // Format JSDoc tags
    let mut result = String::new();
    for line in processed {
        if line.starts_with('@') {
            // Format JSDoc tags
            let tag_line = format!("*{}*\n", line);
            result.push_str(&tag_line);
        } else {
            result.push_str(&line);
            result.push('\n');
        }
    }

    Some(result.trim().to_string())
}

/// Get a user-friendly display name for a node kind
fn get_display_kind(kind: &str, parent_kind: &str) -> String {
    match kind {
        "identifier" => match parent_kind {
            "function_declaration" | "function" => "function".to_string(),
            "class_declaration" | "class" => "class".to_string(),
            "interface_declaration" => "interface".to_string(),
            "type_alias_declaration" => "type alias".to_string(),
            "enum_declaration" => "enum".to_string(),
            "variable_declarator" => "variable".to_string(),
            "formal_parameters" | "required_parameter" | "optional_parameter" => {
                "parameter".to_string()
            }
            "property_signature" | "public_field_definition" => "property".to_string(),
            "method_definition" => "method".to_string(),
            _ => "identifier".to_string(),
        },
        "type_identifier" => "type".to_string(),
        "property_identifier" => "property".to_string(),
        "string" | "template_string" => "string literal".to_string(),
        "number" => "number literal".to_string(),
        "true" | "false" => "boolean literal".to_string(),
        "null" => "null".to_string(),
        "undefined" => "undefined".to_string(),
        "this" => "this".to_string(),
        "super" => "super".to_string(),
        "arrow_function" => "arrow function".to_string(),
        "function_declaration" => "function declaration".to_string(),
        "class_declaration" => "class declaration".to_string(),
        "interface_declaration" => "interface declaration".to_string(),
        "type_alias_declaration" => "type alias".to_string(),
        "enum_declaration" => "enum declaration".to_string(),
        "import_statement" => "import statement".to_string(),
        "export_statement" => "export statement".to_string(),
        _ => kind.replace('_', " "),
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
    fn test_hover_on_variable() {
        let code = "const myVar = 42;";
        let tree = parse_typescript(code);

        // Hover on "myVar" (position 6)
        let hover = get_hover(&tree, code, Position::new(0, 8));
        assert!(hover.is_some());

        let hover = hover.unwrap();
        if let HoverContents::Markup(content) = hover.contents {
            assert!(content.value.contains("myVar"));
        }
    }

    #[test]
    fn test_hover_on_function() {
        let code = "function greet(name: string) { return name; }";
        let tree = parse_typescript(code);

        // Hover on "greet"
        let hover = get_hover(&tree, code, Position::new(0, 11));
        assert!(hover.is_some());
    }

    #[test]
    fn test_hover_on_number() {
        let code = "const x = 42;";
        let tree = parse_typescript(code);

        // Hover on "42"
        let hover = get_hover(&tree, code, Position::new(0, 10));
        assert!(hover.is_some());

        let hover = hover.unwrap();
        if let HoverContents::Markup(content) = hover.contents {
            assert!(content.value.contains("number"));
        }
    }

    #[test]
    fn test_hover_on_string() {
        let code = r#"const x = "hello";"#;
        let tree = parse_typescript(code);

        // Hover on "hello"
        let hover = get_hover(&tree, code, Position::new(0, 12));
        assert!(hover.is_some());

        let hover = hover.unwrap();
        if let HoverContents::Markup(content) = hover.contents {
            assert!(content.value.contains("string"));
        }
    }

    #[test]
    fn test_hover_returns_range() {
        let code = "const x = 42;";
        let tree = parse_typescript(code);

        let hover = get_hover(&tree, code, Position::new(0, 6)).unwrap();
        assert!(hover.range.is_some());

        let range = hover.range.unwrap();
        assert!(range.start.line <= range.end.line);
    }

    #[test]
    fn test_hover_with_jsdoc() {
        let code = r#"
/** This is a greeting function */
function greet() { }
"#;
        let tree = parse_typescript(code);

        // Hover on "greet"
        let hover = get_hover(&tree, code, Position::new(2, 11));
        assert!(hover.is_some());

        let hover = hover.unwrap();
        if let HoverContents::Markup(content) = hover.contents {
            assert!(content.value.contains("greeting"));
        }
    }

    #[test]
    fn test_get_display_kind_identifier() {
        assert_eq!(
            get_display_kind("identifier", "function_declaration"),
            "function"
        );
        assert_eq!(get_display_kind("identifier", "class_declaration"), "class");
        assert_eq!(
            get_display_kind("identifier", "interface_declaration"),
            "interface"
        );
        assert_eq!(
            get_display_kind("identifier", "variable_declarator"),
            "variable"
        );
        assert_eq!(
            get_display_kind("identifier", "method_definition"),
            "method"
        );
        assert_eq!(get_display_kind("identifier", "unknown"), "identifier");
    }

    #[test]
    fn test_get_display_kind_types() {
        assert_eq!(get_display_kind("type_identifier", "any"), "type");
        assert_eq!(get_display_kind("property_identifier", "any"), "property");
    }

    #[test]
    fn test_get_display_kind_literals() {
        assert_eq!(get_display_kind("string", "any"), "string literal");
        assert_eq!(get_display_kind("number", "any"), "number literal");
        assert_eq!(get_display_kind("true", "any"), "boolean literal");
        assert_eq!(get_display_kind("false", "any"), "boolean literal");
    }

    #[test]
    fn test_get_display_kind_keywords() {
        assert_eq!(get_display_kind("null", "any"), "null");
        assert_eq!(get_display_kind("undefined", "any"), "undefined");
        assert_eq!(get_display_kind("this", "any"), "this");
        assert_eq!(get_display_kind("super", "any"), "super");
    }

    #[test]
    fn test_get_display_kind_declarations() {
        assert_eq!(
            get_display_kind("function_declaration", "any"),
            "function declaration"
        );
        assert_eq!(
            get_display_kind("class_declaration", "any"),
            "class declaration"
        );
        assert_eq!(
            get_display_kind("interface_declaration", "any"),
            "interface declaration"
        );
    }

    #[test]
    fn test_get_display_kind_unknown() {
        assert_eq!(
            get_display_kind("some_unknown_node", "any"),
            "some unknown node"
        );
    }

    #[test]
    fn test_hover_empty_position() {
        let code = "const x = 42;";
        let tree = parse_typescript(code);

        // Even position 0,0 should return something
        let hover = get_hover(&tree, code, Position::new(0, 0));
        assert!(hover.is_some());
    }

    #[test]
    fn test_hover_on_class() {
        let code = "class MyClass { }";
        let tree = parse_typescript(code);

        // Hover on "MyClass"
        let hover = get_hover(&tree, code, Position::new(0, 8));
        assert!(hover.is_some());
    }

    #[test]
    fn test_hover_on_interface() {
        let code = "interface IUser { name: string; }";
        let tree = parse_typescript(code);

        // Hover on "IUser"
        let hover = get_hover(&tree, code, Position::new(0, 12));
        assert!(hover.is_some());
    }

    #[test]
    fn test_parse_jsdoc_simple() {
        let comment = "/** Simple comment */";
        let result = parse_jsdoc(comment);
        assert!(result.is_some());
        assert!(result.unwrap().contains("Simple comment"));
    }

    #[test]
    fn test_parse_jsdoc_multiline() {
        let comment = r#"/**
         * This is a description
         * @param name The name
         * @returns The greeting
         */"#;
        let result = parse_jsdoc(comment);
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.contains("description"));
        assert!(result.contains("@param"));
    }

    #[test]
    fn test_parse_jsdoc_not_jsdoc() {
        let comment = "// Regular comment";
        let result = parse_jsdoc(comment);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_jsdoc_empty() {
        let comment = "/** */";
        let result = parse_jsdoc(comment);
        assert!(result.is_none());
    }
}
