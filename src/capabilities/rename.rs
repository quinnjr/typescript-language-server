use std::collections::HashMap;

use tower_lsp::lsp_types::{Position, TextEdit, Url, WorkspaceEdit};

use crate::analysis::SymbolTable;

/// Prepare rename - check if renaming is valid at this position
pub fn prepare_rename(
    symbol_table: &SymbolTable,
    source: &str,
    position: Position,
) -> Option<tower_lsp::lsp_types::Range> {
    let identifier = find_identifier_at_position(source, position)?;
    let scope_id = symbol_table.scope_at_position(position);
    let symbol_id = symbol_table.lookup(&identifier, scope_id)?;
    let symbol = symbol_table.get_symbol(symbol_id)?;

    // Return the range of the identifier being renamed
    Some(symbol.name_range)
}

/// Rename the symbol at the given position
pub fn rename_symbol(
    symbol_table: &SymbolTable,
    source: &str,
    position: Position,
    new_name: &str,
    uri: &Url,
) -> Option<WorkspaceEdit> {
    let identifier = find_identifier_at_position(source, position)?;
    let scope_id = symbol_table.scope_at_position(position);
    let symbol_id = symbol_table.lookup(&identifier, scope_id)?;
    let symbol = symbol_table.get_symbol(symbol_id)?;

    let mut edits = Vec::new();

    // Edit the declaration
    edits.push(TextEdit {
        range: symbol.name_range,
        new_text: new_name.to_string(),
    });

    // Edit all references
    for range in &symbol.references {
        edits.push(TextEdit {
            range: *range,
            new_text: new_name.to_string(),
        });
    }

    let mut changes = HashMap::new();
    changes.insert(uri.clone(), edits);

    Some(WorkspaceEdit {
        changes: Some(changes),
        document_changes: None,
        change_annotations: None,
    })
}

fn find_identifier_at_position(source: &str, position: Position) -> Option<String> {
    let lines: Vec<&str> = source.lines().collect();
    let line_idx = position.line as usize;

    if line_idx >= lines.len() {
        return None;
    }

    let line = lines[line_idx];
    let col = position.character as usize;

    if col > line.len() {
        return None;
    }

    let chars: Vec<char> = line.chars().collect();

    let mut start = col;
    while start > 0 && is_identifier_char(chars.get(start - 1).copied()) {
        start -= 1;
    }

    let mut end = col;
    while end < chars.len() && is_identifier_char(chars.get(end).copied()) {
        end += 1;
    }

    if start == end {
        return None;
    }

    let identifier: String = chars[start..end].iter().collect();

    if is_valid_identifier(&identifier) {
        Some(identifier)
    } else {
        None
    }
}

fn is_identifier_char(c: Option<char>) -> bool {
    match c {
        Some(c) => c.is_alphanumeric() || c == '_' || c == '$',
        None => false,
    }
}

fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();
    let first = chars.next().unwrap();

    if !first.is_alphabetic() && first != '_' && first != '$' {
        return false;
    }

    chars.all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{SymbolFlags, SymbolTable};
    use tower_lsp::lsp_types::Range;

    fn create_test_uri() -> Url {
        Url::parse("file:///test/test.ts").unwrap()
    }

    #[test]
    fn test_is_identifier_char() {
        assert!(is_identifier_char(Some('a')));
        assert!(is_identifier_char(Some('Z')));
        assert!(is_identifier_char(Some('0')));
        assert!(is_identifier_char(Some('_')));
        assert!(is_identifier_char(Some('$')));
        assert!(!is_identifier_char(Some(' ')));
        assert!(!is_identifier_char(Some('.')));
        assert!(!is_identifier_char(None));
    }

    #[test]
    fn test_is_valid_identifier() {
        assert!(is_valid_identifier("x"));
        assert!(is_valid_identifier("myVar"));
        assert!(is_valid_identifier("_private"));
        assert!(is_valid_identifier("$jquery"));
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("123abc"));
    }

    #[test]
    fn test_find_identifier_at_position() {
        let source = "const myVar = 42;";
        let result = find_identifier_at_position(source, Position::new(0, 8));
        assert_eq!(result, Some("myVar".to_string()));
    }

    #[test]
    fn test_find_identifier_out_of_bounds() {
        let source = "const x = 42;";
        let result = find_identifier_at_position(source, Position::new(10, 0));
        assert!(result.is_none());
    }

    #[test]
    fn test_find_identifier_column_out_of_bounds() {
        let source = "const x = 42;";
        let result = find_identifier_at_position(source, Position::new(0, 100));
        assert!(result.is_none());
    }

    #[test]
    fn test_prepare_rename() {
        let mut table = SymbolTable::new();
        let source = "const x = 1;";

        let range = Range {
            start: Position::new(0, 6),
            end: Position::new(0, 7),
        };

        table.create_symbol("x".to_string(), SymbolFlags::VARIABLE, range, range, 0);

        let result = prepare_rename(&table, source, Position::new(0, 6));
        assert!(result.is_some());
        assert_eq!(result.unwrap(), range);
    }

    #[test]
    fn test_prepare_rename_not_found() {
        let table = SymbolTable::new();
        let source = "const x = unknownVar;";

        let result = prepare_rename(&table, source, Position::new(0, 15));
        assert!(result.is_none());
    }

    #[test]
    fn test_rename_symbol() {
        let mut table = SymbolTable::new();
        let uri = create_test_uri();
        let source = "const x = 1;";

        let range = Range {
            start: Position::new(0, 6),
            end: Position::new(0, 7),
        };

        table.create_symbol("x".to_string(), SymbolFlags::VARIABLE, range, range, 0);

        let result = rename_symbol(&table, source, Position::new(0, 6), "y", &uri);
        assert!(result.is_some());

        let edit = result.unwrap();
        assert!(edit.changes.is_some());

        let changes = edit.changes.unwrap();
        let file_edits = changes.get(&uri).unwrap();
        assert_eq!(file_edits.len(), 1);
        assert_eq!(file_edits[0].new_text, "y");
    }

    #[test]
    fn test_rename_symbol_with_references() {
        let mut table = SymbolTable::new();
        let uri = create_test_uri();
        let source = "const x = 1;";

        let decl_range = Range {
            start: Position::new(0, 6),
            end: Position::new(0, 7),
        };

        let id = table.create_symbol(
            "x".to_string(),
            SymbolFlags::VARIABLE,
            decl_range,
            decl_range,
            0,
        );

        // Add a reference
        let ref_range = Range {
            start: Position::new(1, 0),
            end: Position::new(1, 1),
        };
        table.add_reference(id, ref_range);

        let result = rename_symbol(&table, source, Position::new(0, 6), "newName", &uri);
        assert!(result.is_some());

        let edit = result.unwrap();
        let changes = edit.changes.unwrap();
        let file_edits = changes.get(&uri).unwrap();

        // Should have declaration + 1 reference
        assert_eq!(file_edits.len(), 2);
    }

    #[test]
    fn test_rename_symbol_not_found() {
        let table = SymbolTable::new();
        let uri = create_test_uri();
        let source = "const x = unknownVar;";

        let result = rename_symbol(&table, source, Position::new(0, 15), "newName", &uri);
        assert!(result.is_none());
    }
}
