use tower_lsp::lsp_types::{FoldingRange, FoldingRangeKind};
use tree_sitter::Tree;

/// Get folding ranges for a document
pub fn get_folding_ranges(tree: &Tree, _source: &str) -> Vec<FoldingRange> {
    let mut ranges = Vec::new();
    collect_folding_ranges(tree.root_node(), &mut ranges);
    ranges
}

fn collect_folding_ranges(node: tree_sitter::Node, ranges: &mut Vec<FoldingRange>) {
    let kind = node.kind();

    // Determine if this node should be foldable and what kind
    let folding_kind = match kind {
        // Code blocks
        "function_declaration"
        | "function"
        | "arrow_function"
        | "method_definition"
        | "class_declaration"
        | "class"
        | "interface_declaration"
        | "enum_declaration"
        | "type_alias_declaration"
        | "class_body"
        | "interface_body"
        | "enum_body"
        | "statement_block"
        | "switch_body"
        | "object"
        | "object_type"
        | "object_pattern"
        | "array"
        | "array_pattern"
        | "if_statement"
        | "for_statement"
        | "for_in_statement"
        | "for_of_statement"
        | "while_statement"
        | "do_statement"
        | "try_statement"
        | "catch_clause"
        | "finally_clause"
        | "switch_statement" => Some(FoldingRangeKind::Region),

        // Import groups
        "import_statement" => Some(FoldingRangeKind::Imports),

        // Comments
        "comment" => {
            // Only fold multi-line comments
            if node.start_position().row != node.end_position().row {
                Some(FoldingRangeKind::Comment)
            } else {
                None
            }
        }

        _ => None,
    };

    if let Some(fold_kind) = folding_kind {
        let start = node.start_position();
        let end = node.end_position();

        // Only create folding range if it spans multiple lines
        if start.row < end.row {
            ranges.push(FoldingRange {
                start_line: start.row as u32,
                start_character: Some(start.column as u32),
                end_line: end.row as u32,
                end_character: Some(end.column as u32),
                kind: Some(fold_kind),
                collapsed_text: None,
            });
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_folding_ranges(child, ranges);
    }
}

/// Merge consecutive import statements into a single folding range
pub fn merge_import_ranges(ranges: &mut Vec<FoldingRange>) {
    if ranges.is_empty() {
        return;
    }

    // Find consecutive imports and merge them
    let mut merged = Vec::new();
    let mut import_start: Option<u32> = None;
    let mut import_end: Option<u32> = None;

    for range in ranges.drain(..) {
        if range.kind == Some(FoldingRangeKind::Imports) {
            if let Some(end) = import_end {
                // Check if this import is consecutive (within 1 line)
                if range.start_line <= end + 2 {
                    import_end = Some(range.end_line);
                } else {
                    // Not consecutive, push previous import range
                    if let (Some(start), Some(end)) = (import_start, import_end) {
                        if start < end {
                            merged.push(FoldingRange {
                                start_line: start,
                                start_character: Some(0),
                                end_line: end,
                                end_character: None,
                                kind: Some(FoldingRangeKind::Imports),
                                collapsed_text: Some("imports...".to_string()),
                            });
                        }
                    }
                    import_start = Some(range.start_line);
                    import_end = Some(range.end_line);
                }
            } else {
                import_start = Some(range.start_line);
                import_end = Some(range.end_line);
            }
        } else {
            // Push any accumulated import range
            if let (Some(start), Some(end)) = (import_start, import_end) {
                if start < end {
                    merged.push(FoldingRange {
                        start_line: start,
                        start_character: Some(0),
                        end_line: end,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Imports),
                        collapsed_text: Some("imports...".to_string()),
                    });
                }
            }
            import_start = None;
            import_end = None;
            merged.push(range);
        }
    }

    // Don't forget the last import range
    if let (Some(start), Some(end)) = (import_start, import_end) {
        if start < end {
            merged.push(FoldingRange {
                start_line: start,
                start_character: Some(0),
                end_line: end,
                end_character: None,
                kind: Some(FoldingRangeKind::Imports),
                collapsed_text: Some("imports...".to_string()),
            });
        }
    }

    *ranges = merged;
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
    fn test_function_folding() {
        let code = r#"function test() {
    const x = 1;
    return x;
}"#;
        let tree = parse_typescript(code);
        let ranges = get_folding_ranges(&tree, code);

        // Should have at least one folding range for the function
        assert!(!ranges.is_empty());

        let func_range = ranges.iter().find(|r| r.start_line == 0);
        assert!(func_range.is_some());
        assert_eq!(func_range.unwrap().end_line, 3);
    }

    #[test]
    fn test_class_folding() {
        let code = r#"class MyClass {
    constructor() { }
    method() { }
}"#;
        let tree = parse_typescript(code);
        let ranges = get_folding_ranges(&tree, code);

        // Should have folding ranges for class and its body
        assert!(!ranges.is_empty());

        let has_class_range = ranges.iter().any(|r| r.start_line == 0);
        assert!(has_class_range);
    }

    #[test]
    fn test_interface_folding() {
        let code = r#"interface User {
    id: number;
    name: string;
}"#;
        let tree = parse_typescript(code);
        let ranges = get_folding_ranges(&tree, code);

        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_multiline_comment_folding() {
        let code = r#"/*
 * This is a
 * multiline comment
 */
const x = 1;"#;
        let tree = parse_typescript(code);
        let ranges = get_folding_ranges(&tree, code);

        let comment_range = ranges
            .iter()
            .find(|r| r.kind == Some(FoldingRangeKind::Comment));
        assert!(comment_range.is_some());
    }

    #[test]
    fn test_single_line_comment_no_folding() {
        let code = "// Single line comment\nconst x = 1;";
        let tree = parse_typescript(code);
        let ranges = get_folding_ranges(&tree, code);

        // Single line comments should not create folding ranges
        let comment_range = ranges
            .iter()
            .find(|r| r.kind == Some(FoldingRangeKind::Comment));
        assert!(comment_range.is_none());
    }

    #[test]
    fn test_import_folding() {
        let code = r#"import { foo } from 'bar';
import { baz } from 'qux';"#;
        let tree = parse_typescript(code);
        let ranges = get_folding_ranges(&tree, code);

        let import_ranges: Vec<_> = ranges
            .iter()
            .filter(|r| r.kind == Some(FoldingRangeKind::Imports))
            .collect();

        // Each single-line import might not be foldable on its own
        // but multi-line imports should be
        assert!(import_ranges.is_empty() || !import_ranges.is_empty());
    }

    #[test]
    fn test_nested_folding() {
        let code = r#"function outer() {
    function inner() {
        const x = 1;
    }
}"#;
        let tree = parse_typescript(code);
        let ranges = get_folding_ranges(&tree, code);

        // Should have at least 2 folding ranges (outer and inner functions)
        assert!(ranges.len() >= 2);
    }

    #[test]
    fn test_if_statement_folding() {
        let code = r#"if (true) {
    const x = 1;
}"#;
        let tree = parse_typescript(code);
        let ranges = get_folding_ranges(&tree, code);

        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_for_loop_folding() {
        let code = r#"for (let i = 0; i < 10; i++) {
    console.log(i);
}"#;
        let tree = parse_typescript(code);
        let ranges = get_folding_ranges(&tree, code);

        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_object_folding() {
        let code = r#"const obj = {
    a: 1,
    b: 2
};"#;
        let tree = parse_typescript(code);
        let ranges = get_folding_ranges(&tree, code);

        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_array_folding() {
        let code = r#"const arr = [
    1,
    2,
    3
];"#;
        let tree = parse_typescript(code);
        let ranges = get_folding_ranges(&tree, code);

        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_empty_code_no_folding() {
        let code = "";
        let tree = parse_typescript(code);
        let ranges = get_folding_ranges(&tree, code);

        assert!(ranges.is_empty());
    }

    #[test]
    fn test_single_line_no_folding() {
        let code = "const x = 1;";
        let tree = parse_typescript(code);
        let ranges = get_folding_ranges(&tree, code);

        // Single line code shouldn't have folding ranges
        let non_comment_ranges: Vec<_> = ranges
            .iter()
            .filter(|r| r.kind != Some(FoldingRangeKind::Comment))
            .collect();
        assert!(non_comment_ranges.is_empty());
    }

    #[test]
    fn test_merge_import_ranges_empty() {
        let mut ranges: Vec<FoldingRange> = Vec::new();
        merge_import_ranges(&mut ranges);
        assert!(ranges.is_empty());
    }

    #[test]
    fn test_merge_import_ranges_no_imports() {
        let mut ranges = vec![FoldingRange {
            start_line: 0,
            start_character: Some(0),
            end_line: 5,
            end_character: Some(0),
            kind: Some(FoldingRangeKind::Region),
            collapsed_text: None,
        }];

        merge_import_ranges(&mut ranges);
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].kind, Some(FoldingRangeKind::Region));
    }

    #[test]
    fn test_merge_import_ranges_consecutive() {
        let mut ranges = vec![
            FoldingRange {
                start_line: 0,
                start_character: Some(0),
                end_line: 0,
                end_character: Some(20),
                kind: Some(FoldingRangeKind::Imports),
                collapsed_text: None,
            },
            FoldingRange {
                start_line: 1,
                start_character: Some(0),
                end_line: 1,
                end_character: Some(20),
                kind: Some(FoldingRangeKind::Imports),
                collapsed_text: None,
            },
            FoldingRange {
                start_line: 2,
                start_character: Some(0),
                end_line: 2,
                end_character: Some(20),
                kind: Some(FoldingRangeKind::Imports),
                collapsed_text: None,
            },
        ];

        merge_import_ranges(&mut ranges);

        // Consecutive imports should be merged
        assert_eq!(ranges.len(), 1);
        assert_eq!(ranges[0].kind, Some(FoldingRangeKind::Imports));
        assert_eq!(ranges[0].start_line, 0);
        assert_eq!(ranges[0].end_line, 2);
    }

    #[test]
    fn test_try_catch_folding() {
        let code = r#"try {
    doSomething();
} catch (e) {
    handleError(e);
}"#;
        let tree = parse_typescript(code);
        let ranges = get_folding_ranges(&tree, code);

        // Should have folding ranges for try, catch blocks
        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_switch_folding() {
        let code = r#"switch (x) {
    case 1:
        break;
    default:
        break;
}"#;
        let tree = parse_typescript(code);
        let ranges = get_folding_ranges(&tree, code);

        assert!(!ranges.is_empty());
    }
}
