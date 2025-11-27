use std::collections::HashMap;
use tower_lsp::lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Diagnostic, Position, Range, TextEdit, Url,
    WorkspaceEdit,
};

use crate::analysis::SymbolTable;

/// Get code actions for a range and its diagnostics
pub fn get_code_actions(
    uri: &Url,
    range: Range,
    diagnostics: &[Diagnostic],
    symbol_table: &SymbolTable,
    source: &str,
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();

    // Generate quick fixes for diagnostics
    for diagnostic in diagnostics {
        actions.extend(get_diagnostic_fixes(uri, diagnostic, source));
    }

    // Generate refactoring actions based on selection
    actions.extend(get_refactoring_actions(uri, range, symbol_table, source));

    // Generate source actions
    actions.extend(get_source_actions(uri, range, source));

    actions
}

/// Get quick fixes for a diagnostic
fn get_diagnostic_fixes(
    uri: &Url,
    diagnostic: &Diagnostic,
    source: &str,
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();

    // Check diagnostic code
    if let Some(ref code) = diagnostic.code {
        match code {
            tower_lsp::lsp_types::NumberOrString::Number(2304) => {
                // Undefined variable - offer to declare it
                let var_name = extract_name_from_message(&diagnostic.message);
                if let Some(name) = var_name {
                    actions.push(create_declare_variable_action(
                        uri,
                        &diagnostic.range,
                        &name,
                    ));
                }
            }
            tower_lsp::lsp_types::NumberOrString::Number(6133) => {
                // Unused variable - offer to prefix with underscore
                let var_name = extract_name_from_message(&diagnostic.message);
                if let Some(name) = var_name {
                    actions.push(create_prefix_underscore_action(
                        uri,
                        &diagnostic.range,
                        &name,
                    ));
                    actions.push(create_remove_unused_action(
                        uri,
                        &diagnostic.range,
                        source,
                        &name,
                    ));
                }
            }
            tower_lsp::lsp_types::NumberOrString::Number(2588) => {
                // Cannot reassign const - offer to change to let
                actions.push(create_change_to_let_action(uri, source, &diagnostic.range));
            }
            _ => {}
        }
    }

    // Add a generic "add @ts-ignore comment" action
    actions.push(create_ts_ignore_action(uri, &diagnostic.range));

    actions
}

/// Get refactoring actions for a selection
fn get_refactoring_actions(
    uri: &Url,
    range: Range,
    symbol_table: &SymbolTable,
    source: &str,
) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();

    // Get selected text
    let selected_text = get_text_in_range(source, range);

    if !selected_text.is_empty() {
        // Extract to variable
        actions.push(create_extract_variable_action(uri, range, &selected_text));

        // Extract to function
        actions.push(create_extract_function_action(uri, range, &selected_text));

        // If selection is a function, offer to convert to arrow function
        if selected_text.starts_with("function") {
            actions.push(create_convert_to_arrow_action(uri, range, &selected_text));
        }
    }

    // Check if we're on a symbol that can be renamed
    let position = range.start;
    if let Some(_symbol_id) = symbol_table.symbol_at_position(position) {
        // Rename symbol is handled by LSP rename request
    }

    actions
}

/// Get source-level actions
fn get_source_actions(uri: &Url, _range: Range, source: &str) -> Vec<CodeActionOrCommand> {
    let mut actions = Vec::new();

    // Organize imports
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: "Organize Imports".to_string(),
        kind: Some(CodeActionKind::SOURCE_ORGANIZE_IMPORTS),
        diagnostics: None,
        edit: Some(create_organize_imports_edit(uri, source)),
        command: None,
        is_preferred: Some(false),
        disabled: None,
        data: None,
    }));

    // Add missing imports (placeholder)
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: "Add Missing Imports".to_string(),
        kind: Some(CodeActionKind::new("source.addMissingImports")),
        diagnostics: None,
        edit: None, // Would need proper implementation
        command: None,
        is_preferred: Some(false),
        disabled: Some(tower_lsp::lsp_types::CodeActionDisabled {
            reason: "Not yet implemented".to_string(),
        }),
        data: None,
    }));

    // Sort imports alphabetically
    actions.push(CodeActionOrCommand::CodeAction(CodeAction {
        title: "Sort Imports".to_string(),
        kind: Some(CodeActionKind::new("source.sortImports")),
        diagnostics: None,
        edit: Some(create_sort_imports_edit(uri, source)),
        command: None,
        is_preferred: Some(false),
        disabled: None,
        data: None,
    }));

    actions
}

// Helper functions

fn extract_name_from_message(message: &str) -> Option<String> {
    // Extract the name from messages like "Cannot find name 'foo'."
    let start = message.find('\'')?;
    let end = message.rfind('\'')?;
    if start < end {
        Some(message[start + 1..end].to_string())
    } else {
        None
    }
}

fn get_text_in_range(source: &str, range: Range) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut result = String::new();

    for (i, line) in lines.iter().enumerate() {
        let line_num = i as u32;

        if line_num < range.start.line || line_num > range.end.line {
            continue;
        }

        if line_num == range.start.line && line_num == range.end.line {
            // Single line selection
            let start = range.start.character as usize;
            let end = range.end.character as usize;
            if start < line.len() && end <= line.len() {
                result.push_str(&line[start..end]);
            }
        } else if line_num == range.start.line {
            // First line of multi-line selection
            let start = range.start.character as usize;
            if start < line.len() {
                result.push_str(&line[start..]);
                result.push('\n');
            }
        } else if line_num == range.end.line {
            // Last line of multi-line selection
            let end = range.end.character as usize;
            if end <= line.len() {
                result.push_str(&line[..end]);
            }
        } else {
            // Middle line
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

fn create_declare_variable_action(uri: &Url, range: &Range, name: &str) -> CodeActionOrCommand {
    let mut changes = HashMap::new();
    changes.insert(
        uri.clone(),
        vec![TextEdit {
            range: Range {
                start: Position::new(range.start.line, 0),
                end: Position::new(range.start.line, 0),
            },
            new_text: format!("const {} = undefined;\n", name),
        }],
    );

    CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Declare '{}' as const", name),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(false),
        disabled: None,
        data: None,
    })
}

fn create_prefix_underscore_action(uri: &Url, range: &Range, name: &str) -> CodeActionOrCommand {
    let mut changes = HashMap::new();
    changes.insert(
        uri.clone(),
        vec![TextEdit {
            range: *range,
            new_text: format!("_{}", name),
        }],
    );

    CodeActionOrCommand::CodeAction(CodeAction {
        title: format!("Prefix '{}' with underscore", name),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(true),
        disabled: None,
        data: None,
    })
}

fn create_remove_unused_action(
    uri: &Url,
    range: &Range,
    source: &str,
    _name: &str,
) -> CodeActionOrCommand {
    // Find the full declaration to remove
    let lines: Vec<&str> = source.lines().collect();
    let line_idx = range.start.line as usize;

    let delete_range = if line_idx < lines.len() {
        Range {
            start: Position::new(range.start.line, 0),
            end: Position::new(range.start.line + 1, 0),
        }
    } else {
        *range
    };

    let mut changes = HashMap::new();
    changes.insert(
        uri.clone(),
        vec![TextEdit {
            range: delete_range,
            new_text: String::new(),
        }],
    );

    CodeActionOrCommand::CodeAction(CodeAction {
        title: "Remove unused declaration".to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(false),
        disabled: None,
        data: None,
    })
}

fn create_change_to_let_action(uri: &Url, source: &str, range: &Range) -> CodeActionOrCommand {
    // Find the const keyword on the same line
    let lines: Vec<&str> = source.lines().collect();
    let line_idx = range.start.line as usize;

    let mut changes = HashMap::new();

    if line_idx < lines.len() {
        let line = lines[line_idx];
        if let Some(const_pos) = line.find("const ") {
            changes.insert(
                uri.clone(),
                vec![TextEdit {
                    range: Range {
                        start: Position::new(range.start.line, const_pos as u32),
                        end: Position::new(range.start.line, const_pos as u32 + 5),
                    },
                    new_text: "let".to_string(),
                }],
            );
        }
    }

    CodeActionOrCommand::CodeAction(CodeAction {
        title: "Change const to let".to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(true),
        disabled: None,
        data: None,
    })
}

fn create_ts_ignore_action(uri: &Url, range: &Range) -> CodeActionOrCommand {
    let mut changes = HashMap::new();
    changes.insert(
        uri.clone(),
        vec![TextEdit {
            range: Range {
                start: Position::new(range.start.line, 0),
                end: Position::new(range.start.line, 0),
            },
            new_text: "// @ts-ignore\n".to_string(),
        }],
    );

    CodeActionOrCommand::CodeAction(CodeAction {
        title: "Add @ts-ignore comment".to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(false),
        disabled: None,
        data: None,
    })
}

fn create_extract_variable_action(uri: &Url, range: Range, text: &str) -> CodeActionOrCommand {
    let var_name = "extracted";

    let mut changes = HashMap::new();
    changes.insert(
        uri.clone(),
        vec![
            // Insert variable declaration before the current line
            TextEdit {
                range: Range {
                    start: Position::new(range.start.line, 0),
                    end: Position::new(range.start.line, 0),
                },
                new_text: format!("const {} = {};\n", var_name, text.trim()),
            },
            // Replace the selection with the variable name
            TextEdit {
                range,
                new_text: var_name.to_string(),
            },
        ],
    );

    CodeActionOrCommand::CodeAction(CodeAction {
        title: "Extract to variable".to_string(),
        kind: Some(CodeActionKind::REFACTOR_EXTRACT),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(false),
        disabled: None,
        data: None,
    })
}

fn create_extract_function_action(uri: &Url, range: Range, text: &str) -> CodeActionOrCommand {
    let fn_name = "extractedFunction";

    let mut changes = HashMap::new();
    changes.insert(
        uri.clone(),
        vec![
            // Insert function declaration before the current line
            TextEdit {
                range: Range {
                    start: Position::new(range.start.line, 0),
                    end: Position::new(range.start.line, 0),
                },
                new_text: format!(
                    "function {}() {{\n  return {};\n}}\n\n",
                    fn_name,
                    text.trim()
                ),
            },
            // Replace the selection with the function call
            TextEdit {
                range,
                new_text: format!("{}()", fn_name),
            },
        ],
    );

    CodeActionOrCommand::CodeAction(CodeAction {
        title: "Extract to function".to_string(),
        kind: Some(CodeActionKind::REFACTOR_EXTRACT),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(false),
        disabled: None,
        data: None,
    })
}

fn create_convert_to_arrow_action(uri: &Url, range: Range, text: &str) -> CodeActionOrCommand {
    // Simple conversion - a real implementation would parse the function
    let arrow_fn = text.replace("function ", "const f = ").replace(")", ") =>");

    let mut changes = HashMap::new();
    changes.insert(
        uri.clone(),
        vec![TextEdit {
            range,
            new_text: arrow_fn,
        }],
    );

    CodeActionOrCommand::CodeAction(CodeAction {
        title: "Convert to arrow function".to_string(),
        kind: Some(CodeActionKind::REFACTOR_REWRITE),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(false),
        disabled: None,
        data: None,
    })
}

fn create_organize_imports_edit(uri: &Url, source: &str) -> WorkspaceEdit {
    // Find all import statements and group them
    let lines: Vec<&str> = source.lines().collect();
    let mut import_lines: Vec<(usize, &str)> = Vec::new();
    let mut first_import_line: Option<usize> = None;
    let mut last_import_line: Option<usize> = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("import ") {
            if first_import_line.is_none() {
                first_import_line = Some(i);
            }
            last_import_line = Some(i);
            import_lines.push((i, *line));
        }
    }

    let mut changes = HashMap::new();

    if let (Some(first), Some(last)) = (first_import_line, last_import_line) {
        // Sort imports
        let mut imports: Vec<&str> = import_lines.iter().map(|(_, l)| *l).collect();
        imports.sort_by(|a, b| {
            // Sort by module path - extract the path after "from"
            let path_a = a.find(" from ").map(|p| &a[p + 6..]).unwrap_or(a);
            let path_b = b.find(" from ").map(|p| &b[p + 6..]).unwrap_or(b);
            path_a.cmp(path_b)
        });

        let new_text = imports.join("\n") + "\n";

        changes.insert(
            uri.clone(),
            vec![TextEdit {
                range: Range {
                    start: Position::new(first as u32, 0),
                    end: Position::new(last as u32 + 1, 0),
                },
                new_text,
            }],
        );
    }

    WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    }
}

fn create_sort_imports_edit(uri: &Url, source: &str) -> WorkspaceEdit {
    // Same as organize imports for now
    create_organize_imports_edit(uri, source)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_uri() -> Url {
        Url::parse("file:///test/file.ts").unwrap()
    }

    #[test]
    fn test_extract_name_from_message() {
        let message = "Cannot find name 'foo'.";
        let name = extract_name_from_message(message);
        assert_eq!(name, Some("foo".to_string()));
    }

    #[test]
    fn test_extract_name_no_quotes() {
        let message = "Some message without quotes";
        let name = extract_name_from_message(message);
        assert!(name.is_none());
    }

    #[test]
    fn test_extract_name_single_quote() {
        let message = "Cannot find 'x'";
        let name = extract_name_from_message(message);
        assert_eq!(name, Some("x".to_string()));
    }

    #[test]
    fn test_get_text_in_range_single_line() {
        let source = "const x = 1;\nconst y = 2;";
        let range = Range {
            start: Position::new(0, 6),
            end: Position::new(0, 7),
        };
        let text = get_text_in_range(source, range);
        assert_eq!(text, "x");
    }

    #[test]
    fn test_get_text_in_range_multi_line() {
        let source = "line1\nline2\nline3";
        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(1, 5),
        };
        let text = get_text_in_range(source, range);
        assert!(text.contains("line1"));
        assert!(text.contains("line2"));
    }

    #[test]
    fn test_get_text_in_range_out_of_bounds() {
        let source = "short";
        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(10, 0),
        };
        let text = get_text_in_range(source, range);
        // Includes trailing newline from multiline extraction
        assert!(text.starts_with("short"));
    }

    #[test]
    fn test_get_text_in_range_empty() {
        let source = "const x = 1;";
        let range = Range {
            start: Position::new(0, 5),
            end: Position::new(0, 5),
        };
        let text = get_text_in_range(source, range);
        assert_eq!(text, "");
    }

    #[test]
    fn test_create_ts_ignore_action() {
        let uri = test_uri();
        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(0, 10),
        };

        let action = create_ts_ignore_action(&uri, &range);

        if let CodeActionOrCommand::CodeAction(ca) = action {
            assert_eq!(ca.title, "Add @ts-ignore comment");
            assert_eq!(ca.kind, Some(CodeActionKind::QUICKFIX));
            assert!(ca.edit.is_some());
        } else {
            panic!("Expected CodeAction");
        }
    }

    #[test]
    fn test_create_declare_variable_action() {
        let uri = test_uri();
        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(0, 3),
        };

        let action = create_declare_variable_action(&uri, &range, "foo");

        if let CodeActionOrCommand::CodeAction(ca) = action {
            assert!(ca.title.contains("foo"));
            assert_eq!(ca.kind, Some(CodeActionKind::QUICKFIX));
        } else {
            panic!("Expected CodeAction");
        }
    }

    #[test]
    fn test_create_prefix_underscore_action() {
        let uri = test_uri();
        let range = Range {
            start: Position::new(0, 6),
            end: Position::new(0, 9),
        };

        let action = create_prefix_underscore_action(&uri, &range, "foo");

        if let CodeActionOrCommand::CodeAction(ca) = action {
            assert!(ca.title.contains("foo"));
            assert!(ca.title.contains("underscore"));
            assert_eq!(ca.is_preferred, Some(true));
        } else {
            panic!("Expected CodeAction");
        }
    }

    #[test]
    fn test_create_remove_unused_action() {
        let uri = test_uri();
        let source = "const x = 1;\nconst y = 2;";
        let range = Range {
            start: Position::new(0, 6),
            end: Position::new(0, 7),
        };

        let action = create_remove_unused_action(&uri, &range, source, "x");

        if let CodeActionOrCommand::CodeAction(ca) = action {
            assert!(ca.title.contains("Remove"));
            assert_eq!(ca.kind, Some(CodeActionKind::QUICKFIX));
        } else {
            panic!("Expected CodeAction");
        }
    }

    #[test]
    fn test_create_change_to_let_action() {
        let uri = test_uri();
        let source = "const x = 1;";
        let range = Range {
            start: Position::new(0, 6),
            end: Position::new(0, 7),
        };

        let action = create_change_to_let_action(&uri, source, &range);

        if let CodeActionOrCommand::CodeAction(ca) = action {
            assert!(ca.title.contains("let"));
            assert_eq!(ca.is_preferred, Some(true));
        } else {
            panic!("Expected CodeAction");
        }
    }

    #[test]
    fn test_create_extract_variable_action() {
        let uri = test_uri();
        let range = Range {
            start: Position::new(0, 10),
            end: Position::new(0, 15),
        };

        let action = create_extract_variable_action(&uri, range, "1 + 2");

        if let CodeActionOrCommand::CodeAction(ca) = action {
            assert!(ca.title.contains("Extract"));
            assert_eq!(ca.kind, Some(CodeActionKind::REFACTOR_EXTRACT));
        } else {
            panic!("Expected CodeAction");
        }
    }

    #[test]
    fn test_create_extract_function_action() {
        let uri = test_uri();
        let range = Range {
            start: Position::new(0, 10),
            end: Position::new(0, 20),
        };

        let action = create_extract_function_action(&uri, range, "doSomething()");

        if let CodeActionOrCommand::CodeAction(ca) = action {
            assert!(ca.title.contains("function"));
            assert_eq!(ca.kind, Some(CodeActionKind::REFACTOR_EXTRACT));
        } else {
            panic!("Expected CodeAction");
        }
    }

    #[test]
    fn test_create_convert_to_arrow_action() {
        let uri = test_uri();
        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(2, 1),
        };

        let action = create_convert_to_arrow_action(&uri, range, "function foo() { return 1; }");

        if let CodeActionOrCommand::CodeAction(ca) = action {
            assert!(ca.title.contains("arrow"));
            assert_eq!(ca.kind, Some(CodeActionKind::REFACTOR_REWRITE));
        } else {
            panic!("Expected CodeAction");
        }
    }

    #[test]
    fn test_create_organize_imports_edit() {
        let uri = test_uri();
        let source = r#"import { z } from 'z';
import { a } from 'a';
import { m } from 'm';

const x = 1;"#;

        let edit = create_organize_imports_edit(&uri, source);

        assert!(edit.changes.is_some());
        let changes = edit.changes.unwrap();
        assert!(!changes.is_empty());
    }

    #[test]
    fn test_create_organize_imports_no_imports() {
        let uri = test_uri();
        let source = "const x = 1;";

        let edit = create_organize_imports_edit(&uri, source);

        // Should still return a valid edit, just with no changes
        assert!(edit.changes.is_some());
    }

    #[test]
    fn test_create_sort_imports_edit() {
        let uri = test_uri();
        let source = r#"import { z } from 'z';
import { a } from 'a';"#;

        let edit = create_sort_imports_edit(&uri, source);

        assert!(edit.changes.is_some());
    }

    #[test]
    fn test_get_source_actions() {
        let uri = test_uri();
        let source = "const x = 1;";
        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(0, 10),
        };

        let actions = get_source_actions(&uri, range, source);

        // Should include organize imports, add missing imports, sort imports
        assert!(actions.len() >= 2);

        // Check for organize imports action
        let has_organize = actions.iter().any(|a| {
            if let CodeActionOrCommand::CodeAction(ca) = a {
                ca.kind == Some(CodeActionKind::SOURCE_ORGANIZE_IMPORTS)
            } else {
                false
            }
        });
        assert!(has_organize);
    }

    #[test]
    fn test_get_refactoring_actions_empty_selection() {
        let uri = test_uri();
        let source = "const x = 1;";
        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(0, 0),
        };
        let symbol_table = SymbolTable::new();

        let actions = get_refactoring_actions(&uri, range, &symbol_table, source);

        // Empty selection should not produce extract actions
        assert!(actions.is_empty());
    }

    #[test]
    fn test_get_refactoring_actions_with_selection() {
        let uri = test_uri();
        let source = "const x = 1 + 2 + 3;";
        let range = Range {
            start: Position::new(0, 10),
            end: Position::new(0, 19),
        };
        let symbol_table = SymbolTable::new();

        let actions = get_refactoring_actions(&uri, range, &symbol_table, source);

        // Should have extract variable and extract function
        assert!(actions.len() >= 2);
    }

    #[test]
    fn test_get_refactoring_actions_function_selection() {
        let uri = test_uri();
        let source = "function foo() { return 1; }";
        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(0, 28),
        };
        let symbol_table = SymbolTable::new();

        let actions = get_refactoring_actions(&uri, range, &symbol_table, source);

        // Should include convert to arrow function action
        let has_arrow = actions.iter().any(|a| {
            if let CodeActionOrCommand::CodeAction(ca) = a {
                ca.title.contains("arrow")
            } else {
                false
            }
        });
        assert!(has_arrow);
    }

    #[test]
    fn test_get_diagnostic_fixes_undefined() {
        let uri = test_uri();
        let diagnostic = Diagnostic {
            range: Range {
                start: Position::new(0, 0),
                end: Position::new(0, 3),
            },
            severity: None,
            code: Some(tower_lsp::lsp_types::NumberOrString::Number(2304)),
            code_description: None,
            source: None,
            message: "Cannot find name 'foo'.".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };

        let actions = get_diagnostic_fixes(&uri, &diagnostic, "foo");

        // Should include declare variable and ts-ignore
        assert!(actions.len() >= 2);

        let has_declare = actions.iter().any(|a| {
            if let CodeActionOrCommand::CodeAction(ca) = a {
                ca.title.contains("Declare")
            } else {
                false
            }
        });
        assert!(has_declare);
    }

    #[test]
    fn test_get_diagnostic_fixes_unused() {
        let uri = test_uri();
        let diagnostic = Diagnostic {
            range: Range {
                start: Position::new(0, 6),
                end: Position::new(0, 7),
            },
            severity: None,
            code: Some(tower_lsp::lsp_types::NumberOrString::Number(6133)),
            code_description: None,
            source: None,
            message: "'x' is declared but never used.".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };

        let actions = get_diagnostic_fixes(&uri, &diagnostic, "const x = 1;");

        // Should include prefix underscore, remove, and ts-ignore
        assert!(actions.len() >= 3);
    }

    #[test]
    fn test_get_diagnostic_fixes_const_reassign() {
        let uri = test_uri();
        let diagnostic = Diagnostic {
            range: Range {
                start: Position::new(1, 0),
                end: Position::new(1, 5),
            },
            severity: None,
            code: Some(tower_lsp::lsp_types::NumberOrString::Number(2588)),
            code_description: None,
            source: None,
            message: "Cannot assign to 'x' because it is a constant.".to_string(),
            related_information: None,
            tags: None,
            data: None,
        };

        let source = "const x = 1;\nx = 2;";
        let actions = get_diagnostic_fixes(&uri, &diagnostic, source);

        // Should include change to let and ts-ignore
        assert!(actions.len() >= 2);

        let has_let = actions.iter().any(|a| {
            if let CodeActionOrCommand::CodeAction(ca) = a {
                ca.title.contains("let")
            } else {
                false
            }
        });
        assert!(has_let);
    }

    #[test]
    fn test_get_code_actions() {
        let uri = test_uri();
        let source = "const x = 1 + 2;";
        let range = Range {
            start: Position::new(0, 10),
            end: Position::new(0, 15),
        };
        let symbol_table = SymbolTable::new();

        let actions = get_code_actions(&uri, range, &[], &symbol_table, source);

        // Should have source actions and refactoring actions
        assert!(!actions.is_empty());
    }

    #[test]
    fn test_get_code_actions_with_diagnostics() {
        let uri = test_uri();
        let source = "foo";
        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(0, 3),
        };
        let symbol_table = SymbolTable::new();
        let diagnostics = vec![Diagnostic {
            range,
            severity: None,
            code: Some(tower_lsp::lsp_types::NumberOrString::Number(2304)),
            code_description: None,
            source: None,
            message: "Cannot find name 'foo'.".to_string(),
            related_information: None,
            tags: None,
            data: None,
        }];

        let actions = get_code_actions(&uri, range, &diagnostics, &symbol_table, source);

        // Should have diagnostic fixes + source actions
        assert!(actions.len() >= 3);
    }

    #[test]
    fn test_change_to_let_no_const() {
        let uri = test_uri();
        let source = "let x = 1;";
        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(0, 10),
        };

        let action = create_change_to_let_action(&uri, source, &range);

        // Should still create an action, just with empty changes
        if let CodeActionOrCommand::CodeAction(ca) = action {
            assert!(ca.edit.is_some());
        } else {
            panic!("Expected CodeAction");
        }
    }
}
