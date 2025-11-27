use tower_lsp::lsp_types::{Position, Range, SelectionRange};
use tree_sitter::Tree;

/// Get selection ranges for given positions (smart expand/shrink selection)
pub fn get_selection_ranges(tree: &Tree, positions: &[Position]) -> Vec<SelectionRange> {
    positions
        .iter()
        .filter_map(|pos| get_selection_range_at(tree, *pos))
        .collect()
}

fn get_selection_range_at(tree: &Tree, position: Position) -> Option<SelectionRange> {
    let root = tree.root_node();

    let point = tree_sitter::Point {
        row: position.line as usize,
        column: position.character as usize,
    };

    // Find the smallest node containing the position
    let node = root.descendant_for_point_range(point, point)?;

    // Build selection range hierarchy from innermost to outermost
    build_selection_range(node)
}

fn build_selection_range(node: tree_sitter::Node) -> Option<SelectionRange> {
    let range = node_to_range(&node);

    // Get parent's selection range
    let parent = if let Some(parent_node) = node.parent() {
        // Skip nodes that have the same range as their child
        if parent_node.start_position() == node.start_position()
            && parent_node.end_position() == node.end_position()
        {
            // Use grandparent if parent has same range
            if let Some(grandparent) = parent_node.parent() {
                build_selection_range(grandparent).map(Box::new)
            } else {
                None
            }
        } else {
            build_selection_range(parent_node).map(Box::new)
        }
    } else {
        None
    };

    Some(SelectionRange { range, parent })
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
    fn test_selection_range_basic() {
        let code = "const x = 42;";
        let tree = parse_typescript(code);

        let ranges = get_selection_ranges(&tree, &[Position::new(0, 6)]);
        assert!(!ranges.is_empty());

        let range = &ranges[0];
        // Should have a range
        assert!(range.range.start.character <= 6);
        assert!(range.range.end.character >= 6);
    }

    #[test]
    fn test_selection_range_has_parent() {
        let code = "const x = 42;";
        let tree = parse_typescript(code);

        let ranges = get_selection_ranges(&tree, &[Position::new(0, 6)]);
        assert!(!ranges.is_empty());

        // The selection range should have a parent
        // (x -> variable_declarator -> lexical_declaration -> program)
        let range = &ranges[0];
        assert!(range.parent.is_some());
    }

    #[test]
    fn test_selection_range_multiple_positions() {
        let code = "const x = 1;\nconst y = 2;";
        let tree = parse_typescript(code);

        let positions = vec![Position::new(0, 6), Position::new(1, 6)];
        let ranges = get_selection_ranges(&tree, &positions);

        assert_eq!(ranges.len(), 2);
    }

    #[test]
    fn test_selection_range_nested() {
        let code = r#"function test() {
    if (true) {
        const x = 1;
    }
}"#;
        let tree = parse_typescript(code);

        // Position inside the if block
        let ranges = get_selection_ranges(&tree, &[Position::new(2, 15)]);
        assert!(!ranges.is_empty());

        // Should have multiple parent levels
        let range = &ranges[0];
        let mut depth = 0;
        let mut current = range;
        while let Some(ref parent) = current.parent {
            depth += 1;
            current = parent;
        }

        // Should have multiple nesting levels
        assert!(depth >= 2);
    }

    #[test]
    fn test_selection_range_empty_positions() {
        let code = "const x = 42;";
        let tree = parse_typescript(code);

        let ranges = get_selection_ranges(&tree, &[]);
        assert!(ranges.is_empty());
    }

    #[test]
    fn test_selection_range_function_body() {
        let code = r#"function test() {
    return 1;
}"#;
        let tree = parse_typescript(code);

        // Position on "return"
        let ranges = get_selection_ranges(&tree, &[Position::new(1, 6)]);
        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_selection_range_class_method() {
        let code = r#"class MyClass {
    method() {
        return this;
    }
}"#;
        let tree = parse_typescript(code);

        // Position on "this"
        let ranges = get_selection_ranges(&tree, &[Position::new(2, 15)]);
        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_selection_range_string() {
        let code = r#"const s = "hello world";"#;
        let tree = parse_typescript(code);

        // Position inside string
        let ranges = get_selection_ranges(&tree, &[Position::new(0, 14)]);
        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_selection_range_array() {
        let code = "const arr = [1, 2, 3];";
        let tree = parse_typescript(code);

        // Position on middle element
        let ranges = get_selection_ranges(&tree, &[Position::new(0, 16)]);
        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_selection_range_object() {
        let code = "const obj = { a: 1, b: 2 };";
        let tree = parse_typescript(code);

        // Position on property
        let ranges = get_selection_ranges(&tree, &[Position::new(0, 16)]);
        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_selection_range_hierarchy() {
        // Test that selection ranges form a proper hierarchy (each parent contains child)
        let code = "const x = 1 + 2;";
        let tree = parse_typescript(code);

        let ranges = get_selection_ranges(&tree, &[Position::new(0, 10)]);
        assert!(!ranges.is_empty());

        let range = &ranges[0];
        let mut current = range;

        while let Some(ref parent) = current.parent {
            // Parent range should contain child range
            assert!(parent.range.start <= current.range.start);
            assert!(parent.range.end >= current.range.end);
            current = parent;
        }
    }
}
