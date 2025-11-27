use tower_lsp::lsp_types::{
    ParameterInformation, ParameterLabel, Position, SignatureHelp, SignatureInformation,
};
use tree_sitter::{Node, Tree};

use crate::analysis::{SymbolFlags, SymbolTable};

/// Get signature help at a position
pub fn get_signature_help(
    tree: &Tree,
    source: &str,
    symbol_table: &SymbolTable,
    position: Position,
) -> Option<SignatureHelp> {
    let root = tree.root_node();

    let point = tree_sitter::Point {
        row: position.line as usize,
        column: position.character as usize,
    };

    // Find the call expression at or containing the position
    let node = root.descendant_for_point_range(point, point)?;

    // Look for a call expression in ancestors
    let call_node = find_call_expression(&node)?;

    // Get the function being called
    let function_node = call_node.child_by_field_name("function")?;
    let function_name = function_node.utf8_text(source.as_bytes()).ok()?;

    // Handle built-in functions
    if let Some(help) = get_builtin_signature_help(function_name) {
        // Determine active parameter
        let args_node = call_node.child_by_field_name("arguments")?;
        let active_param = count_args_before_position(&args_node, position);

        return Some(SignatureHelp {
            signatures: vec![help],
            active_signature: Some(0),
            active_parameter: Some(active_param as u32),
        });
    }

    // Try to find the function in the symbol table
    let function_base = function_name.split('.').last().unwrap_or(function_name);
    let scope_id = symbol_table.scope_at_position(position);

    if let Some(symbol_id) = symbol_table.lookup(function_base, scope_id) {
        if let Some(symbol) = symbol_table.get_symbol(symbol_id) {
            if symbol.flags.intersects(SymbolFlags::FUNCTION | SymbolFlags::METHOD) {
                let args_node = call_node.child_by_field_name("arguments")?;
                let active_param = count_args_before_position(&args_node, position);

                return Some(SignatureHelp {
                    signatures: vec![SignatureInformation {
                        label: format!("{}()", symbol.name),
                        documentation: symbol.documentation.clone().map(|d| {
                            tower_lsp::lsp_types::Documentation::String(d)
                        }),
                        parameters: None, // Would need function signature analysis
                        active_parameter: Some(active_param as u32),
                    }],
                    active_signature: Some(0),
                    active_parameter: Some(active_param as u32),
                });
            }
        }
    }

    None
}

/// Find a call expression in the node's ancestors
fn find_call_expression<'a>(node: &'a Node<'a>) -> Option<Node<'a>> {
    let mut current = *node;

    loop {
        if current.kind() == "call_expression" {
            return Some(current);
        }
        current = current.parent()?;
    }
}

/// Count arguments before the cursor position
fn count_args_before_position(args_node: &Node, position: Position) -> usize {
    let mut count = 0;
    let mut cursor = args_node.walk();

    for child in args_node.children(&mut cursor) {
        // Skip parentheses and commas
        if child.kind() == "(" || child.kind() == ")" {
            continue;
        }

        if child.kind() == "," {
            // Check if comma is before position
            let comma_pos = child.start_position();
            if comma_pos.row < position.line as usize
                || (comma_pos.row == position.line as usize
                    && comma_pos.column < position.character as usize)
            {
                count += 1;
            }
        }
    }

    count
}

/// Get signature help for built-in functions
fn get_builtin_signature_help(name: &str) -> Option<SignatureInformation> {
    match name {
        "console.log" => Some(SignatureInformation {
            label: "log(...data: any[]): void".to_string(),
            documentation: Some(tower_lsp::lsp_types::Documentation::String(
                "Prints to stdout with newline.".to_string(),
            )),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple("...data: any[]".to_string()),
                documentation: Some(tower_lsp::lsp_types::Documentation::String(
                    "Data to print".to_string(),
                )),
            }]),
            active_parameter: None,
        }),
        "console.error" => Some(SignatureInformation {
            label: "error(...data: any[]): void".to_string(),
            documentation: Some(tower_lsp::lsp_types::Documentation::String(
                "Prints to stderr with newline.".to_string(),
            )),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple("...data: any[]".to_string()),
                documentation: Some(tower_lsp::lsp_types::Documentation::String(
                    "Data to print".to_string(),
                )),
            }]),
            active_parameter: None,
        }),
        "JSON.parse" => Some(SignatureInformation {
            label: "parse(text: string, reviver?: (key: string, value: any) => any): any".to_string(),
            documentation: Some(tower_lsp::lsp_types::Documentation::String(
                "Converts a JavaScript Object Notation (JSON) string into an object.".to_string(),
            )),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("text: string".to_string()),
                    documentation: Some(tower_lsp::lsp_types::Documentation::String(
                        "A valid JSON string.".to_string(),
                    )),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("reviver?: (key: string, value: any) => any".to_string()),
                    documentation: Some(tower_lsp::lsp_types::Documentation::String(
                        "A function that transforms the results.".to_string(),
                    )),
                },
            ]),
            active_parameter: None,
        }),
        "JSON.stringify" => Some(SignatureInformation {
            label: "stringify(value: any, replacer?: (key: string, value: any) => any, space?: string | number): string".to_string(),
            documentation: Some(tower_lsp::lsp_types::Documentation::String(
                "Converts a JavaScript value to a JavaScript Object Notation (JSON) string.".to_string(),
            )),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("value: any".to_string()),
                    documentation: Some(tower_lsp::lsp_types::Documentation::String(
                        "A JavaScript value, usually an object or array, to be converted.".to_string(),
                    )),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("replacer?: (key: string, value: any) => any".to_string()),
                    documentation: Some(tower_lsp::lsp_types::Documentation::String(
                        "A function that transforms the results.".to_string(),
                    )),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("space?: string | number".to_string()),
                    documentation: Some(tower_lsp::lsp_types::Documentation::String(
                        "Adds indentation, white space, and line break characters to the return-value JSON text.".to_string(),
                    )),
                },
            ]),
            active_parameter: None,
        }),
        "Math.max" | "Math.min" => Some(SignatureInformation {
            label: format!("{}(...values: number[]): number", name.split('.').last().unwrap_or(name)),
            documentation: Some(tower_lsp::lsp_types::Documentation::String(
                format!("Returns the {} of a set of supplied numeric expressions.", if name.contains("max") { "larger" } else { "smaller" }),
            )),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple("...values: number[]".to_string()),
                documentation: Some(tower_lsp::lsp_types::Documentation::String(
                    "Numeric expressions to be evaluated.".to_string(),
                )),
            }]),
            active_parameter: None,
        }),
        "Math.pow" => Some(SignatureInformation {
            label: "pow(x: number, y: number): number".to_string(),
            documentation: Some(tower_lsp::lsp_types::Documentation::String(
                "Returns the value of a base expression taken to a specified power.".to_string(),
            )),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("x: number".to_string()),
                    documentation: Some(tower_lsp::lsp_types::Documentation::String(
                        "The base value of the expression.".to_string(),
                    )),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("y: number".to_string()),
                    documentation: Some(tower_lsp::lsp_types::Documentation::String(
                        "The exponent value of the expression.".to_string(),
                    )),
                },
            ]),
            active_parameter: None,
        }),
        "setTimeout" => Some(SignatureInformation {
            label: "setTimeout(callback: () => void, ms?: number, ...args: any[]): number".to_string(),
            documentation: Some(tower_lsp::lsp_types::Documentation::String(
                "Schedules execution of a one-time callback after delay milliseconds.".to_string(),
            )),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("callback: () => void".to_string()),
                    documentation: Some(tower_lsp::lsp_types::Documentation::String(
                        "The function to call when the timer elapses.".to_string(),
                    )),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("ms?: number".to_string()),
                    documentation: Some(tower_lsp::lsp_types::Documentation::String(
                        "The number of milliseconds to wait before calling the callback.".to_string(),
                    )),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("...args: any[]".to_string()),
                    documentation: Some(tower_lsp::lsp_types::Documentation::String(
                        "Optional arguments to pass when the callback is called.".to_string(),
                    )),
                },
            ]),
            active_parameter: None,
        }),
        "setInterval" => Some(SignatureInformation {
            label: "setInterval(callback: () => void, ms?: number, ...args: any[]): number".to_string(),
            documentation: Some(tower_lsp::lsp_types::Documentation::String(
                "Schedules repeated execution of callback every delay milliseconds.".to_string(),
            )),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("callback: () => void".to_string()),
                    documentation: Some(tower_lsp::lsp_types::Documentation::String(
                        "The function to call when the timer elapses.".to_string(),
                    )),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("ms?: number".to_string()),
                    documentation: Some(tower_lsp::lsp_types::Documentation::String(
                        "The number of milliseconds between calls.".to_string(),
                    )),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("...args: any[]".to_string()),
                    documentation: Some(tower_lsp::lsp_types::Documentation::String(
                        "Optional arguments to pass when the callback is called.".to_string(),
                    )),
                },
            ]),
            active_parameter: None,
        }),
        "Array.isArray" => Some(SignatureInformation {
            label: "isArray(arg: any): arg is any[]".to_string(),
            documentation: Some(tower_lsp::lsp_types::Documentation::String(
                "Determines whether the passed value is an Array.".to_string(),
            )),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple("arg: any".to_string()),
                documentation: Some(tower_lsp::lsp_types::Documentation::String(
                    "The value to be checked.".to_string(),
                )),
            }]),
            active_parameter: None,
        }),
        "Object.keys" => Some(SignatureInformation {
            label: "keys(o: object): string[]".to_string(),
            documentation: Some(tower_lsp::lsp_types::Documentation::String(
                "Returns the names of the enumerable string properties and methods of an object.".to_string(),
            )),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple("o: object".to_string()),
                documentation: Some(tower_lsp::lsp_types::Documentation::String(
                    "Object that contains the properties and methods.".to_string(),
                )),
            }]),
            active_parameter: None,
        }),
        "Object.values" => Some(SignatureInformation {
            label: "values<T>(o: { [s: string]: T }): T[]".to_string(),
            documentation: Some(tower_lsp::lsp_types::Documentation::String(
                "Returns an array of values of the enumerable properties of an object.".to_string(),
            )),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple("o: object".to_string()),
                documentation: Some(tower_lsp::lsp_types::Documentation::String(
                    "Object that contains the properties.".to_string(),
                )),
            }]),
            active_parameter: None,
        }),
        "Object.entries" => Some(SignatureInformation {
            label: "entries<T>(o: { [s: string]: T }): [string, T][]".to_string(),
            documentation: Some(tower_lsp::lsp_types::Documentation::String(
                "Returns an array of key/values of the enumerable properties of an object.".to_string(),
            )),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple("o: object".to_string()),
                documentation: Some(tower_lsp::lsp_types::Documentation::String(
                    "Object that contains the properties.".to_string(),
                )),
            }]),
            active_parameter: None,
        }),
        "Promise.all" => Some(SignatureInformation {
            label: "all<T>(values: Iterable<T | PromiseLike<T>>): Promise<Awaited<T>[]>".to_string(),
            documentation: Some(tower_lsp::lsp_types::Documentation::String(
                "Creates a Promise that is resolved with an array of results when all of the provided Promises resolve.".to_string(),
            )),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple("values: Iterable<T | PromiseLike<T>>".to_string()),
                documentation: Some(tower_lsp::lsp_types::Documentation::String(
                    "An iterable of Promises.".to_string(),
                )),
            }]),
            active_parameter: None,
        }),
        "Promise.race" => Some(SignatureInformation {
            label: "race<T>(values: Iterable<T | PromiseLike<T>>): Promise<Awaited<T>>".to_string(),
            documentation: Some(tower_lsp::lsp_types::Documentation::String(
                "Creates a Promise that is resolved or rejected when any of the provided Promises are resolved or rejected.".to_string(),
            )),
            parameters: Some(vec![ParameterInformation {
                label: ParameterLabel::Simple("values: Iterable<T | PromiseLike<T>>".to_string()),
                documentation: Some(tower_lsp::lsp_types::Documentation::String(
                    "An iterable of Promises.".to_string(),
                )),
            }]),
            active_parameter: None,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_signature_help() {
        let help = get_builtin_signature_help("console.log");
        assert!(help.is_some());

        let sig = help.unwrap();
        assert!(sig.label.contains("log"));
    }

    #[test]
    fn test_json_parse_signature() {
        let help = get_builtin_signature_help("JSON.parse");
        assert!(help.is_some());

        let sig = help.unwrap();
        assert!(sig.parameters.is_some());
        assert_eq!(sig.parameters.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_unknown_function() {
        let help = get_builtin_signature_help("unknown.function");
        assert!(help.is_none());
    }

    #[test]
    fn test_count_args_before_position() {
        // This would need a proper tree-sitter parse to test properly
        // Just ensure the function exists and has the right signature
    }
}

