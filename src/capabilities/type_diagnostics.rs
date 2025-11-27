use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, DiagnosticTag, NumberOrString, Position, Range};
use tree_sitter::{Node, Tree};

use crate::analysis::{SymbolFlags, SymbolTable};

/// Diagnostic codes for type errors
#[derive(Debug, Clone, Copy)]
pub enum TypeDiagnosticCode {
    UndefinedVariable = 2304,
    UndefinedType = 2552,
    TypeMismatch = 2322,
    MissingProperty = 2339,
    UnusedVariable = 6133,
    UnusedParameter = 6138,
    CannotReassignConst = 2588,
    ArgumentCountMismatch = 2554,
    NotCallable = 2349,
    NoImplicitAny = 7006,
}

impl TypeDiagnosticCode {
    pub fn as_number(&self) -> i32 {
        *self as i32
    }

    pub fn message(&self, context: &str) -> String {
        match self {
            TypeDiagnosticCode::UndefinedVariable => {
                format!("Cannot find name '{}'.", context)
            }
            TypeDiagnosticCode::UndefinedType => {
                format!("Cannot find name '{}'. Did you mean '{}'?", context, context)
            }
            TypeDiagnosticCode::TypeMismatch => {
                format!("Type '{}' is not assignable.", context)
            }
            TypeDiagnosticCode::MissingProperty => {
                format!("Property '{}' does not exist on type.", context)
            }
            TypeDiagnosticCode::UnusedVariable => {
                format!("'{}' is declared but its value is never read.", context)
            }
            TypeDiagnosticCode::UnusedParameter => {
                format!("'{}' is declared but its value is never read.", context)
            }
            TypeDiagnosticCode::CannotReassignConst => {
                format!("Cannot assign to '{}' because it is a constant.", context)
            }
            TypeDiagnosticCode::ArgumentCountMismatch => {
                format!("Expected {} arguments, but got more.", context)
            }
            TypeDiagnosticCode::NotCallable => {
                format!("This expression is not callable. Type '{}' has no call signatures.", context)
            }
            TypeDiagnosticCode::NoImplicitAny => {
                format!("Parameter '{}' implicitly has an 'any' type.", context)
            }
        }
    }
}

/// Get type-aware diagnostics for a document
pub fn get_type_diagnostics(
    tree: &Tree,
    source: &str,
    symbol_table: &SymbolTable,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Check for undefined variables
    check_undefined_references(tree, source, symbol_table, &mut diagnostics);

    // Check for unused variables
    check_unused_variables(symbol_table, &mut diagnostics);

    // Check for const reassignment
    check_const_reassignment(tree, source, symbol_table, &mut diagnostics);

    diagnostics
}

/// Check for references to undefined variables
fn check_undefined_references(
    tree: &Tree,
    source: &str,
    symbol_table: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let root = tree.root_node();
    check_node_references(root, source, symbol_table, diagnostics);
}

fn check_node_references(
    node: Node,
    source: &str,
    symbol_table: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Check identifiers that are references (not declarations)
    if node.kind() == "identifier" {
        if is_reference_identifier(&node) {
            let name = node.utf8_text(source.as_bytes()).unwrap_or("");

            // Skip built-in globals
            if is_builtin_global(name) {
                return;
            }

            let position = Position::new(
                node.start_position().row as u32,
                node.start_position().column as u32,
            );
            let scope_id = symbol_table.scope_at_position(position);

            // Check if the symbol exists
            if symbol_table.lookup(name, scope_id).is_none()
                && symbol_table.lookup_type(name, scope_id).is_none()
            {
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

                diagnostics.push(Diagnostic {
                    range,
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::Number(TypeDiagnosticCode::UndefinedVariable.as_number())),
                    code_description: None,
                    source: Some("ts-lsp-rust".to_string()),
                    message: TypeDiagnosticCode::UndefinedVariable.message(name),
                    related_information: None,
                    tags: None,
                    data: None,
                });
            }
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        check_node_references(child, source, symbol_table, diagnostics);
    }
}

/// Check if an identifier node is a reference (not a declaration)
fn is_reference_identifier(node: &Node) -> bool {
    if let Some(parent) = node.parent() {
        match parent.kind() {
            // Declaration contexts - not references
            "variable_declarator" => {
                // Check if this is the name (declaration) or the value (reference)
                parent.child_by_field_name("name") != Some(*node)
            }
            "function_declaration" | "class_declaration" | "interface_declaration"
            | "type_alias_declaration" | "enum_declaration" | "method_definition" => {
                parent.child_by_field_name("name") != Some(*node)
            }
            "import_specifier" | "shorthand_property_identifier_pattern"
            | "required_parameter" | "optional_parameter" | "rest_parameter" => false,
            "property_signature" | "public_field_definition" => {
                parent.child_by_field_name("name") != Some(*node)
            }
            // References
            _ => true,
        }
    } else {
        true
    }
}

/// Check for unused variables
fn check_unused_variables(symbol_table: &SymbolTable, diagnostics: &mut Vec<Diagnostic>) {
    for symbol in symbol_table.all_symbols() {
        // Skip if not a variable or parameter
        if !symbol.flags.intersects(SymbolFlags::VARIABLE | SymbolFlags::PARAMETER) {
            continue;
        }

        // Skip imported symbols
        if symbol.flags.contains(SymbolFlags::IMPORT) {
            continue;
        }

        // Skip if name starts with underscore (intentionally unused)
        if symbol.name.starts_with('_') {
            continue;
        }

        // Check if the symbol has any references
        if symbol.references.is_empty() {
            let code = if symbol.flags.contains(SymbolFlags::PARAMETER) {
                TypeDiagnosticCode::UnusedParameter
            } else {
                TypeDiagnosticCode::UnusedVariable
            };

            diagnostics.push(Diagnostic {
                range: symbol.name_range,
                severity: Some(DiagnosticSeverity::HINT),
                code: Some(NumberOrString::Number(code.as_number())),
                code_description: None,
                source: Some("ts-lsp-rust".to_string()),
                message: code.message(&symbol.name),
                related_information: None,
                tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                data: None,
            });
        }
    }
}

/// Check for reassignment of const variables
fn check_const_reassignment(
    tree: &Tree,
    source: &str,
    symbol_table: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let root = tree.root_node();
    check_assignments(root, source, symbol_table, diagnostics);
}

fn check_assignments(
    node: Node,
    source: &str,
    symbol_table: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Check assignment expressions
    if node.kind() == "assignment_expression" {
        if let Some(left) = node.child_by_field_name("left") {
            if left.kind() == "identifier" {
                let name = left.utf8_text(source.as_bytes()).unwrap_or("");
                let position = Position::new(
                    left.start_position().row as u32,
                    left.start_position().column as u32,
                );
                let scope_id = symbol_table.scope_at_position(position);

                if let Some(symbol_id) = symbol_table.lookup(name, scope_id) {
                    if let Some(symbol) = symbol_table.get_symbol(symbol_id) {
                        if symbol.flags.contains(SymbolFlags::CONST) {
                            let range = Range {
                                start: Position::new(
                                    left.start_position().row as u32,
                                    left.start_position().column as u32,
                                ),
                                end: Position::new(
                                    left.end_position().row as u32,
                                    left.end_position().column as u32,
                                ),
                            };

                            diagnostics.push(Diagnostic {
                                range,
                                severity: Some(DiagnosticSeverity::ERROR),
                                code: Some(NumberOrString::Number(
                                    TypeDiagnosticCode::CannotReassignConst.as_number(),
                                )),
                                code_description: None,
                                source: Some("ts-lsp-rust".to_string()),
                                message: TypeDiagnosticCode::CannotReassignConst.message(name),
                                related_information: None,
                                tags: None,
                                data: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // Check update expressions (++, --)
    if node.kind() == "update_expression" {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "identifier" {
                let name = child.utf8_text(source.as_bytes()).unwrap_or("");
                let position = Position::new(
                    child.start_position().row as u32,
                    child.start_position().column as u32,
                );
                let scope_id = symbol_table.scope_at_position(position);

                if let Some(symbol_id) = symbol_table.lookup(name, scope_id) {
                    if let Some(symbol) = symbol_table.get_symbol(symbol_id) {
                        if symbol.flags.contains(SymbolFlags::CONST) {
                            let range = Range {
                                start: Position::new(
                                    child.start_position().row as u32,
                                    child.start_position().column as u32,
                                ),
                                end: Position::new(
                                    child.end_position().row as u32,
                                    child.end_position().column as u32,
                                ),
                            };

                            diagnostics.push(Diagnostic {
                                range,
                                severity: Some(DiagnosticSeverity::ERROR),
                                code: Some(NumberOrString::Number(
                                    TypeDiagnosticCode::CannotReassignConst.as_number(),
                                )),
                                code_description: None,
                                source: Some("ts-lsp-rust".to_string()),
                                message: TypeDiagnosticCode::CannotReassignConst.message(name),
                                related_information: None,
                                tags: None,
                                data: None,
                            });
                        }
                    }
                }
            }
        }
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        check_assignments(child, source, symbol_table, diagnostics);
    }
}

/// Check if a name is a built-in global
fn is_builtin_global(name: &str) -> bool {
    matches!(
        name,
        "console"
            | "window"
            | "document"
            | "global"
            | "globalThis"
            | "process"
            | "require"
            | "module"
            | "exports"
            | "__dirname"
            | "__filename"
            | "Buffer"
            | "setTimeout"
            | "setInterval"
            | "clearTimeout"
            | "clearInterval"
            | "setImmediate"
            | "clearImmediate"
            | "Promise"
            | "Array"
            | "Object"
            | "String"
            | "Number"
            | "Boolean"
            | "Symbol"
            | "BigInt"
            | "Function"
            | "Date"
            | "RegExp"
            | "Error"
            | "TypeError"
            | "ReferenceError"
            | "SyntaxError"
            | "RangeError"
            | "EvalError"
            | "URIError"
            | "Map"
            | "Set"
            | "WeakMap"
            | "WeakSet"
            | "Proxy"
            | "Reflect"
            | "JSON"
            | "Math"
            | "Intl"
            | "Atomics"
            | "SharedArrayBuffer"
            | "ArrayBuffer"
            | "DataView"
            | "Int8Array"
            | "Uint8Array"
            | "Uint8ClampedArray"
            | "Int16Array"
            | "Uint16Array"
            | "Int32Array"
            | "Uint32Array"
            | "Float32Array"
            | "Float64Array"
            | "BigInt64Array"
            | "BigUint64Array"
            | "NaN"
            | "Infinity"
            | "undefined"
            | "eval"
            | "isFinite"
            | "isNaN"
            | "parseFloat"
            | "parseInt"
            | "decodeURI"
            | "decodeURIComponent"
            | "encodeURI"
            | "encodeURIComponent"
            | "escape"
            | "unescape"
            | "React"
            | "JSX"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::binder::bind_document;
    use tree_sitter::Parser;

    fn parse_and_bind(code: &str) -> (Tree, SymbolTable) {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        let tree = parser.parse(code, None).unwrap();
        let symbol_table = bind_document(&tree, code);
        (tree, symbol_table)
    }

    #[test]
    fn test_undefined_variable() {
        let code = "const x = unknownVar;";
        let (tree, symbol_table) = parse_and_bind(code);
        let diagnostics = get_type_diagnostics(&tree, code, &symbol_table);

        assert!(diagnostics.iter().any(|d| d.message.contains("unknownVar")));
    }

    #[test]
    fn test_defined_variable_no_error() {
        let code = "const x = 1;\nconst y = x;";
        let (tree, symbol_table) = parse_and_bind(code);
        let diagnostics = get_type_diagnostics(&tree, code, &symbol_table);

        // Should not report x as undefined
        assert!(!diagnostics.iter().any(|d| {
            d.code == Some(NumberOrString::Number(TypeDiagnosticCode::UndefinedVariable.as_number()))
                && d.message.contains("'x'")
        }));
    }

    #[test]
    fn test_unused_variable() {
        let code = "const unusedVar = 1;";
        let (tree, symbol_table) = parse_and_bind(code);
        let diagnostics = get_type_diagnostics(&tree, code, &symbol_table);

        assert!(diagnostics.iter().any(|d| d.message.contains("unusedVar")));
    }

    #[test]
    fn test_used_variable_no_unused_warning() {
        let code = "const x = 1;\nconsole.log(x);";
        let (tree, symbol_table) = parse_and_bind(code);
        let diagnostics = get_type_diagnostics(&tree, code, &symbol_table);

        // Should not report x as unused
        assert!(!diagnostics.iter().any(|d| {
            d.code == Some(NumberOrString::Number(TypeDiagnosticCode::UnusedVariable.as_number()))
                && d.message.contains("'x'")
        }));
    }

    #[test]
    fn test_underscore_prefix_not_reported() {
        let code = "const _unused = 1;";
        let (tree, symbol_table) = parse_and_bind(code);
        let diagnostics = get_type_diagnostics(&tree, code, &symbol_table);

        // Variables starting with _ should not be reported
        assert!(!diagnostics.iter().any(|d| d.message.contains("_unused")));
    }

    #[test]
    fn test_const_reassignment() {
        let code = "const x = 1;\nx = 2;";
        let (tree, symbol_table) = parse_and_bind(code);
        let diagnostics = get_type_diagnostics(&tree, code, &symbol_table);

        assert!(diagnostics.iter().any(|d| {
            d.code == Some(NumberOrString::Number(TypeDiagnosticCode::CannotReassignConst.as_number()))
        }));
    }

    #[test]
    fn test_let_reassignment_allowed() {
        let code = "let x = 1;\nx = 2;";
        let (tree, symbol_table) = parse_and_bind(code);
        let diagnostics = get_type_diagnostics(&tree, code, &symbol_table);

        // Should not report reassignment error for let
        assert!(!diagnostics.iter().any(|d| {
            d.code == Some(NumberOrString::Number(TypeDiagnosticCode::CannotReassignConst.as_number()))
        }));
    }

    #[test]
    fn test_builtin_global_not_undefined() {
        let code = "console.log('hello');";
        let (tree, symbol_table) = parse_and_bind(code);
        let diagnostics = get_type_diagnostics(&tree, code, &symbol_table);

        // console should not be reported as undefined
        assert!(!diagnostics.iter().any(|d| d.message.contains("console")));
    }

    #[test]
    fn test_type_diagnostic_code() {
        assert_eq!(TypeDiagnosticCode::UndefinedVariable.as_number(), 2304);
        assert_eq!(TypeDiagnosticCode::UnusedVariable.as_number(), 6133);
        assert_eq!(TypeDiagnosticCode::CannotReassignConst.as_number(), 2588);
    }
}

