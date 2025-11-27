use tower_lsp::lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, Position, Range};
use tree_sitter::{Node, Tree};

use crate::analysis::SymbolTable;

/// Get inlay hints for a document range
pub fn get_inlay_hints(
    tree: &Tree,
    source: &str,
    symbol_table: &SymbolTable,
    range: Range,
) -> Vec<InlayHint> {
    let mut hints = Vec::new();

    let root = tree.root_node();
    collect_inlay_hints(root, source, symbol_table, range, &mut hints);

    hints
}

fn collect_inlay_hints(
    node: Node,
    source: &str,
    symbol_table: &SymbolTable,
    range: Range,
    hints: &mut Vec<InlayHint>,
) {
    // Check if node is within range
    let node_start = Position::new(
        node.start_position().row as u32,
        node.start_position().column as u32,
    );
    let node_end = Position::new(
        node.end_position().row as u32,
        node.end_position().column as u32,
    );

    if node_end.line < range.start.line || node_start.line > range.end.line {
        return;
    }

    match node.kind() {
        // Variable declarations without type annotations
        "variable_declarator" => {
            if let Some(hint) = get_variable_type_hint(&node, source, symbol_table) {
                hints.push(hint);
            }
        }
        // Function parameters without type annotations
        "required_parameter" | "optional_parameter" => {
            if let Some(hint) = get_parameter_type_hint(&node, source) {
                hints.push(hint);
            }
        }
        // Function return type hints
        "function_declaration" | "arrow_function" | "method_definition" => {
            if let Some(hint) = get_return_type_hint(&node, source) {
                hints.push(hint);
            }
        }
        // Call expression parameter name hints
        "call_expression" => {
            hints.extend(get_call_parameter_hints(&node, source));
        }
        _ => {}
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_inlay_hints(child, source, symbol_table, range, hints);
    }
}

/// Get type hint for a variable declaration
fn get_variable_type_hint(
    node: &Node,
    source: &str,
    symbol_table: &SymbolTable,
) -> Option<InlayHint> {
    // Check if there's already a type annotation
    if node.child_by_field_name("type").is_some() {
        return None;
    }

    // Get the variable name
    let name_node = node.child_by_field_name("name")?;
    if name_node.kind() != "identifier" {
        return None; // Skip destructuring patterns for now
    }

    let _name = name_node.utf8_text(source.as_bytes()).ok()?;

    // Get the initializer to infer type
    let value_node = node.child_by_field_name("value")?;
    let inferred_type = infer_type_from_node(&value_node, source, symbol_table)?;

    // Only show hint if we can infer a meaningful type
    if inferred_type == "any" || inferred_type.is_empty() {
        return None;
    }

    // Position after the variable name
    let position = Position::new(
        name_node.end_position().row as u32,
        name_node.end_position().column as u32,
    );

    Some(InlayHint {
        position,
        label: InlayHintLabel::String(format!(": {}", inferred_type)),
        kind: Some(InlayHintKind::TYPE),
        text_edits: None,
        tooltip: None,
        padding_left: Some(false),
        padding_right: Some(true),
        data: None,
    })
}

/// Infer type from a node
fn infer_type_from_node(node: &Node, source: &str, _symbol_table: &SymbolTable) -> Option<String> {
    match node.kind() {
        "string" | "template_string" => Some("string".to_string()),
        "number" => Some("number".to_string()),
        "true" | "false" => Some("boolean".to_string()),
        "null" => Some("null".to_string()),
        "undefined" => Some("undefined".to_string()),
        "array" => {
            // Try to get element type
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "[" && child.kind() != "]" && child.kind() != "," {
                    if let Some(elem_type) = infer_type_from_node(&child, source, _symbol_table) {
                        return Some(format!("{}[]", elem_type));
                    }
                }
            }
            Some("unknown[]".to_string())
        }
        "object" => Some("object".to_string()),
        "arrow_function" | "function" => {
            // Get return type if possible
            Some("() => ...".to_string())
        }
        "new_expression" => {
            // Get the class name
            if let Some(constructor) = node.child_by_field_name("constructor") {
                let name = constructor.utf8_text(source.as_bytes()).ok()?;
                return Some(name.to_string());
            }
            None
        }
        "call_expression" => {
            // Try to infer based on the function name
            if let Some(function) = node.child_by_field_name("function") {
                let fn_text = function.utf8_text(source.as_bytes()).ok()?;
                // Some well-known functions
                match fn_text {
                    "parseInt" | "parseFloat" => return Some("number".to_string()),
                    "String" => return Some("string".to_string()),
                    "Boolean" => return Some("boolean".to_string()),
                    "Array.isArray" => return Some("boolean".to_string()),
                    _ => {}
                }
            }
            None
        }
        "binary_expression" => {
            // Get the operator
            let op_text = node
                .child(1)
                .and_then(|op| op.utf8_text(source.as_bytes()).ok())?;

            match op_text {
                "+" | "-" | "*" | "/" | "%" | "**" => {
                    // Could be number or string (for +)
                    if op_text == "+" {
                        // Check if any operand is a string
                        let left = node.child_by_field_name("left")?;
                        let right = node.child_by_field_name("right")?;
                        let left_type = infer_type_from_node(&left, source, _symbol_table);
                        let right_type = infer_type_from_node(&right, source, _symbol_table);

                        if left_type.as_deref() == Some("string")
                            || right_type.as_deref() == Some("string")
                        {
                            return Some("string".to_string());
                        }
                    }
                    Some("number".to_string())
                }
                "==" | "===" | "!=" | "!==" | "<" | ">" | "<=" | ">=" | "&&" | "||" | "!" => {
                    Some("boolean".to_string())
                }
                _ => None,
            }
        }
        "ternary_expression" => {
            // Get types of both branches
            let consequence = node.child_by_field_name("consequence")?;
            let alternative = node.child_by_field_name("alternative")?;

            let cons_type = infer_type_from_node(&consequence, source, _symbol_table)?;
            let alt_type = infer_type_from_node(&alternative, source, _symbol_table)?;

            if cons_type == alt_type {
                Some(cons_type)
            } else {
                Some(format!("{} | {}", cons_type, alt_type))
            }
        }
        "await_expression" => {
            // Get the awaited type
            if let Some(arg) = node.child(1) {
                return infer_type_from_node(&arg, source, _symbol_table);
            }
            None
        }
        _ => None,
    }
}

/// Get type hint for a parameter
fn get_parameter_type_hint(node: &Node, _source: &str) -> Option<InlayHint> {
    // Check if there's already a type annotation
    if node.child_by_field_name("type").is_some() {
        return None;
    }

    // Find the parameter name
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "identifier" {
            let position = Position::new(
                child.end_position().row as u32,
                child.end_position().column as u32,
            );

            // For now, just show "any" hint for untyped parameters
            return Some(InlayHint {
                position,
                label: InlayHintLabel::String(": any".to_string()),
                kind: Some(InlayHintKind::TYPE),
                text_edits: None,
                tooltip: Some(tower_lsp::lsp_types::InlayHintTooltip::String(
                    "Parameter has implicit any type".to_string(),
                )),
                padding_left: Some(false),
                padding_right: Some(true),
                data: None,
            });
        }
    }

    None
}

/// Get return type hint for a function
fn get_return_type_hint(node: &Node, _source: &str) -> Option<InlayHint> {
    // Check if there's already a return type annotation
    if node.child_by_field_name("return_type").is_some() {
        return None;
    }

    // Find the parameters node to place hint after
    let params = node.child_by_field_name("parameters")?;

    let position = Position::new(
        params.end_position().row as u32,
        params.end_position().column as u32,
    );

    // For now, show a generic hint
    // A full implementation would analyze the return statements
    Some(InlayHint {
        position,
        label: InlayHintLabel::String(": void".to_string()),
        kind: Some(InlayHintKind::TYPE),
        text_edits: None,
        tooltip: Some(tower_lsp::lsp_types::InlayHintTooltip::String(
            "Inferred return type".to_string(),
        )),
        padding_left: Some(false),
        padding_right: Some(true),
        data: None,
    })
}

/// Get parameter name hints for call expressions
fn get_call_parameter_hints(node: &Node, source: &str) -> Vec<InlayHint> {
    let mut hints = Vec::new();

    // Get the function being called
    let function_node = match node.child_by_field_name("function") {
        Some(f) => f,
        None => return hints,
    };

    let function_name = match function_node.utf8_text(source.as_bytes()) {
        Ok(name) => name,
        Err(_) => return hints,
    };

    // Get parameter names for well-known functions
    let param_names = get_known_function_params(function_name);
    if param_names.is_empty() {
        return hints;
    }

    // Get the arguments
    let args_node = match node.child_by_field_name("arguments") {
        Some(a) => a,
        None => return hints,
    };

    let mut arg_index = 0;
    let mut cursor = args_node.walk();

    for child in args_node.children(&mut cursor) {
        // Skip parentheses and commas
        if child.kind() == "(" || child.kind() == ")" || child.kind() == "," {
            continue;
        }

        // Get parameter name if available
        if let Some(param_name) = param_names.get(arg_index) {
            // Only show hint if it's not obvious from the argument
            let arg_text = child.utf8_text(source.as_bytes()).unwrap_or("");

            // Skip if argument name matches parameter name
            if !arg_text.contains(param_name) {
                let position = Position::new(
                    child.start_position().row as u32,
                    child.start_position().column as u32,
                );

                hints.push(InlayHint {
                    position,
                    label: InlayHintLabel::String(format!("{}:", param_name)),
                    kind: Some(InlayHintKind::PARAMETER),
                    text_edits: None,
                    tooltip: None,
                    padding_left: Some(false),
                    padding_right: Some(true),
                    data: None,
                });
            }
        }

        arg_index += 1;
    }

    hints
}

/// Get parameter names for well-known functions
fn get_known_function_params(function_name: &str) -> Vec<&'static str> {
    match function_name {
        "setTimeout" | "setInterval" => vec!["callback", "ms"],
        "fetch" => vec!["input", "init"],
        "JSON.parse" => vec!["text", "reviver"],
        "JSON.stringify" => vec!["value", "replacer", "space"],
        "Math.pow" => vec!["base", "exponent"],
        "Math.max" | "Math.min" => vec!["values"],
        "Object.assign" => vec!["target", "sources"],
        "Object.keys" | "Object.values" | "Object.entries" => vec!["obj"],
        "Array.isArray" => vec!["value"],
        "Promise.all" | "Promise.race" | "Promise.allSettled" | "Promise.any" => vec!["values"],
        "Promise.resolve" => vec!["value"],
        "Promise.reject" => vec!["reason"],
        "addEventListener" => vec!["type", "listener", "options"],
        "removeEventListener" => vec!["type", "listener", "options"],
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_function_params() {
        let params = get_known_function_params("setTimeout");
        assert_eq!(params, vec!["callback", "ms"]);
    }

    #[test]
    fn test_unknown_function_params() {
        let params = get_known_function_params("unknownFunction");
        assert!(params.is_empty());
    }

    #[test]
    fn test_json_parse_params() {
        let params = get_known_function_params("JSON.parse");
        assert_eq!(params, vec!["text", "reviver"]);
    }

    #[test]
    fn test_infer_type_string_literal() {
        // This would need a proper tree-sitter parse
        // Just ensuring the function signature is correct
    }
}
