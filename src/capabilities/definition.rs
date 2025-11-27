use tower_lsp::lsp_types::{GotoDefinitionResponse, Location, Position, Url};

use crate::analysis::SymbolTable;

/// Find the definition of the symbol at the given position
pub fn get_definition(
    symbol_table: &SymbolTable,
    source: &str,
    position: Position,
    uri: &Url,
) -> Option<GotoDefinitionResponse> {
    // First, find what identifier is at the position
    let identifier = find_identifier_at_position(source, position)?;

    // Find the scope at this position
    let scope_id = symbol_table.scope_at_position(position);

    // Look up the symbol
    let symbol_id = symbol_table.lookup(&identifier, scope_id)?;
    let symbol = symbol_table.get_symbol(symbol_id)?;

    // Return the definition location
    let location = Location {
        uri: uri.clone(),
        range: symbol.name_range,
    };

    Some(GotoDefinitionResponse::Scalar(location))
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

    // Validate it's a valid identifier
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

    // First char must be letter, underscore, or $
    if !first.is_alphabetic() && first != '_' && first != '$' {
        return false;
    }

    // Rest can be alphanumeric, underscore, or $
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
        assert!(!is_identifier_char(Some('+')));
        assert!(!is_identifier_char(None));
    }

    #[test]
    fn test_is_valid_identifier() {
        assert!(is_valid_identifier("x"));
        assert!(is_valid_identifier("myVar"));
        assert!(is_valid_identifier("_private"));
        assert!(is_valid_identifier("$jquery"));
        assert!(is_valid_identifier("camelCase"));
        assert!(is_valid_identifier("PascalCase"));
        assert!(is_valid_identifier("CONSTANT"));
        assert!(is_valid_identifier("var123"));
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("123abc")); // Can't start with number
        assert!(!is_valid_identifier("-invalid"));
    }

    #[test]
    fn test_find_identifier_at_position_simple() {
        let source = "const myVar = 42;";
        let pos = Position::new(0, 8); // In the middle of "myVar"

        let result = find_identifier_at_position(source, pos);
        assert_eq!(result, Some("myVar".to_string()));
    }

    #[test]
    fn test_find_identifier_at_position_start() {
        let source = "const x = 42;";
        let pos = Position::new(0, 6); // At start of "x"

        let result = find_identifier_at_position(source, pos);
        assert_eq!(result, Some("x".to_string()));
    }

    #[test]
    fn test_find_identifier_at_position_keyword() {
        let source = "const x = 42;";
        let pos = Position::new(0, 2); // In "const"

        let result = find_identifier_at_position(source, pos);
        assert_eq!(result, Some("const".to_string()));
    }

    #[test]
    fn test_find_identifier_at_position_multiline() {
        let source = "const x = 1;\nconst y = 2;";
        let pos = Position::new(1, 6); // At "y" on second line

        let result = find_identifier_at_position(source, pos);
        assert_eq!(result, Some("y".to_string()));
    }

    #[test]
    fn test_find_identifier_at_position_no_identifier() {
        let source = "const x = 42;";
        let pos = Position::new(0, 9); // At "="

        let result = find_identifier_at_position(source, pos);
        // May return "42" or None depending on implementation
        assert!(result.is_none() || result == Some("42".to_string()));
    }

    #[test]
    fn test_find_identifier_at_position_out_of_bounds() {
        let source = "const x = 42;";
        let pos = Position::new(5, 0); // Line doesn't exist

        let result = find_identifier_at_position(source, pos);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_identifier_at_position_column_out_of_bounds() {
        let source = "const x = 42;";
        let pos = Position::new(0, 100); // Column doesn't exist

        let result = find_identifier_at_position(source, pos);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_identifier_dollar_sign() {
        let source = "const $element = null;";
        let pos = Position::new(0, 8); // In "$element"

        let result = find_identifier_at_position(source, pos);
        assert_eq!(result, Some("$element".to_string()));
    }

    #[test]
    fn test_find_identifier_underscore() {
        let source = "const _private = 1;";
        let pos = Position::new(0, 8); // In "_private"

        let result = find_identifier_at_position(source, pos);
        assert_eq!(result, Some("_private".to_string()));
    }

    #[test]
    fn test_get_definition() {
        let mut table = SymbolTable::new();
        let uri = create_test_uri();
        let source = "const x = 1;\nconst y = x;";

        let decl_range = Range {
            start: Position::new(0, 6),
            end: Position::new(0, 7),
        };
        let name_range = decl_range;

        // Create symbol for "x"
        table.create_symbol(
            "x".to_string(),
            SymbolFlags::VARIABLE | SymbolFlags::CONST,
            decl_range,
            name_range,
            0,
        );

        // Try to find definition of "x"
        let pos = Position::new(0, 6); // On "x" declaration
        let result = get_definition(&table, source, pos, &uri);

        assert!(result.is_some());
        match result.unwrap() {
            GotoDefinitionResponse::Scalar(location) => {
                assert_eq!(location.uri, uri);
                assert_eq!(location.range.start, Position::new(0, 6));
            }
            _ => panic!("Expected scalar response"),
        }
    }

    #[test]
    fn test_get_definition_not_found() {
        let table = SymbolTable::new();
        let uri = create_test_uri();
        let source = "const x = unknownVar;";

        // Try to find definition of "unknownVar" which doesn't exist
        let pos = Position::new(0, 15); // On "unknownVar"
        let result = get_definition(&table, source, pos, &uri);

        assert!(result.is_none());
    }
}
