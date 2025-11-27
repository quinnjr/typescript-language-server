use tower_lsp::lsp_types::{Location, Position, Url};

use crate::analysis::SymbolTable;

/// Find all references to the symbol at the given position
pub fn get_references(
    symbol_table: &SymbolTable,
    source: &str,
    position: Position,
    uri: &Url,
    include_declaration: bool,
) -> Vec<Location> {
    // First, find what identifier is at the position
    let identifier = match find_identifier_at_position(source, position) {
        Some(id) => id,
        None => return Vec::new(),
    };

    // Find the scope at this position
    let scope_id = symbol_table.scope_at_position(position);

    // Look up the symbol
    let symbol_id = match symbol_table.lookup(&identifier, scope_id) {
        Some(id) => id,
        None => return Vec::new(),
    };

    let symbol = match symbol_table.get_symbol(symbol_id) {
        Some(s) => s,
        None => return Vec::new(),
    };

    let mut locations = Vec::new();

    // Include declaration if requested
    if include_declaration {
        locations.push(Location {
            uri: uri.clone(),
            range: symbol.name_range,
        });
    }

    // Add all references
    for range in &symbol.references {
        locations.push(Location {
            uri: uri.clone(),
            range: *range,
        });
    }

    locations
}

/// Find the identifier at a given position in the source
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

    // Find the start and end of the identifier
    let chars: Vec<char> = line.chars().collect();

    // Find start of identifier
    let mut start = col;
    while start > 0 && is_identifier_char(chars.get(start - 1).copied()) {
        start -= 1;
    }

    // Find end of identifier
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
    fn test_get_references_with_declaration() {
        let mut table = SymbolTable::new();
        let uri = create_test_uri();
        let source = "const x = 1;";

        let decl_range = Range {
            start: Position::new(0, 6),
            end: Position::new(0, 7),
        };

        // Create symbol
        let id = table.create_symbol(
            "x".to_string(),
            SymbolFlags::VARIABLE | SymbolFlags::CONST,
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

        // Get references including declaration
        let refs = get_references(&table, source, Position::new(0, 6), &uri, true);

        // Should have declaration + 1 reference
        assert_eq!(refs.len(), 2);
    }

    #[test]
    fn test_get_references_without_declaration() {
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

        let ref_range = Range {
            start: Position::new(1, 0),
            end: Position::new(1, 1),
        };
        table.add_reference(id, ref_range);

        // Get references without declaration
        let refs = get_references(&table, source, Position::new(0, 6), &uri, false);

        // Should only have the reference, not the declaration
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].range, ref_range);
    }

    #[test]
    fn test_get_references_unknown_symbol() {
        let table = SymbolTable::new();
        let uri = create_test_uri();
        let source = "const x = unknownVar;";

        let refs = get_references(&table, source, Position::new(0, 15), &uri, true);

        assert!(refs.is_empty());
    }

    #[test]
    fn test_get_references_no_identifier() {
        let table = SymbolTable::new();
        let uri = create_test_uri();
        let source = "const x = 42;";

        // Position on operator/number
        let refs = get_references(&table, source, Position::new(0, 8), &uri, true);

        // "=" or "42" - may or may not be found
        // Either empty or some result, but should not panic
        let _ = refs;
    }

    #[test]
    fn test_get_references_multiple_references() {
        let mut table = SymbolTable::new();
        let uri = create_test_uri();
        let source = "const x = 1;\nconsole.log(x);\nreturn x;";

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

        // Add multiple references
        table.add_reference(
            id,
            Range {
                start: Position::new(1, 12),
                end: Position::new(1, 13),
            },
        );
        table.add_reference(
            id,
            Range {
                start: Position::new(2, 7),
                end: Position::new(2, 8),
            },
        );

        let refs = get_references(&table, source, Position::new(0, 6), &uri, true);

        // Declaration + 2 references
        assert_eq!(refs.len(), 3);
    }

    #[test]
    fn test_find_identifier_at_position_simple() {
        let source = "const myVar = 42;";
        let result = find_identifier_at_position(source, Position::new(0, 8));
        assert_eq!(result, Some("myVar".to_string()));
    }

    #[test]
    fn test_find_identifier_at_position_out_of_bounds() {
        let source = "const x = 42;";
        let result = find_identifier_at_position(source, Position::new(10, 0));
        assert!(result.is_none());
    }

    #[test]
    fn test_references_all_have_same_uri() {
        let mut table = SymbolTable::new();
        let uri = create_test_uri();
        let source = "const x = 1;";

        let range = Range {
            start: Position::new(0, 6),
            end: Position::new(0, 7),
        };

        let id = table.create_symbol("x".to_string(), SymbolFlags::VARIABLE, range, range, 0);
        table.add_reference(id, range);

        let refs = get_references(&table, source, Position::new(0, 6), &uri, true);

        for r in refs {
            assert_eq!(r.uri, uri);
        }
    }
}
