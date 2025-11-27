use dashmap::DashMap;
use tower_lsp::lsp_types::Url;
use tree_sitter::Tree;

use crate::analysis::{binder, SymbolTable};
use crate::parser::{SourceLanguage, SourceParser};

/// Represents an open document with its content and parsed tree
pub struct Document {
    pub content: String,
    pub tree: Option<Tree>,
    pub version: i32,
    pub language: SourceLanguage,
    pub symbol_table: Option<SymbolTable>,
}

impl Document {
    pub fn new(uri: &Url, content: String, version: i32, parser: &mut SourceParser) -> Self {
        let language = SourceLanguage::from_uri(uri);
        parser.set_language(language);
        let tree = parser.parse(&content, None);

        // Bind the document to create the symbol table
        let symbol_table = tree.as_ref().map(|t| binder::bind_document(t, &content));

        Self {
            content,
            tree,
            version,
            language,
            symbol_table,
        }
    }

    /// Apply incremental changes to the document
    pub fn apply_changes(
        &mut self,
        changes: Vec<tower_lsp::lsp_types::TextDocumentContentChangeEvent>,
        new_version: i32,
        parser: &mut SourceParser,
    ) {
        for change in changes {
            if let Some(range) = change.range {
                // Incremental update
                let start_offset = self.offset_at_position(range.start);
                let end_offset = self.offset_at_position(range.end);

                self.content
                    .replace_range(start_offset..end_offset, &change.text);
            } else {
                // Full document replacement
                self.content = change.text;
            }
        }

        self.version = new_version;
        // Ensure parser is set to correct language before re-parsing
        parser.set_language(self.language);
        // Re-parse with old tree for incremental parsing
        self.tree = parser.parse(&self.content, self.tree.as_ref());

        // Re-bind the document
        self.symbol_table = self
            .tree
            .as_ref()
            .map(|t| binder::bind_document(t, &self.content));
    }

    /// Convert LSP position to byte offset
    fn offset_at_position(&self, position: tower_lsp::lsp_types::Position) -> usize {
        let mut offset = 0;
        for (i, line) in self.content.lines().enumerate() {
            if i == position.line as usize {
                return offset + position.character as usize;
            }
            offset += line.len() + 1; // +1 for newline
        }
        offset
    }

    /// Convert byte offset to LSP position
    pub fn position_at_offset(&self, offset: usize) -> tower_lsp::lsp_types::Position {
        let mut line = 0;
        let mut col = 0;
        let mut current_offset = 0;

        for ch in self.content.chars() {
            if current_offset >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
            current_offset += ch.len_utf8();
        }

        tower_lsp::lsp_types::Position::new(line, col)
    }
}

/// Manages all open documents
pub struct DocumentManager {
    documents: DashMap<Url, Document>,
}

impl DocumentManager {
    pub fn new() -> Self {
        Self {
            documents: DashMap::new(),
        }
    }

    /// Open a new document
    pub fn open(&self, uri: Url, content: String, version: i32, parser: &mut SourceParser) {
        let doc = Document::new(&uri, content, version, parser);
        self.documents.insert(uri, doc);
    }

    /// Update an existing document
    pub fn change(
        &self,
        uri: &Url,
        changes: Vec<tower_lsp::lsp_types::TextDocumentContentChangeEvent>,
        version: i32,
        parser: &mut SourceParser,
    ) {
        if let Some(mut doc) = self.documents.get_mut(uri) {
            doc.apply_changes(changes, version, parser);
        }
    }

    /// Close a document
    pub fn close(&self, uri: &Url) {
        self.documents.remove(uri);
    }

    /// Get a reference to a document
    pub fn get(&self, uri: &Url) -> Option<dashmap::mapref::one::Ref<'_, Url, Document>> {
        self.documents.get(uri)
    }
}

impl Default for DocumentManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::{Position, Range, TextDocumentContentChangeEvent};

    fn create_test_uri(filename: &str) -> Url {
        Url::parse(&format!("file:///test/{}", filename)).unwrap()
    }

    #[test]
    fn test_document_new() {
        let mut parser = SourceParser::default();
        let uri = create_test_uri("test.ts");
        let content = "const x = 1;".to_string();

        let doc = Document::new(&uri, content.clone(), 1, &mut parser);

        assert_eq!(doc.content, content);
        assert_eq!(doc.version, 1);
        assert_eq!(doc.language, SourceLanguage::TypeScript);
        assert!(doc.tree.is_some());
        assert!(doc.symbol_table.is_some());
    }

    #[test]
    fn test_document_language_detection() {
        let mut parser = SourceParser::default();

        let uri = create_test_uri("test.tsx");
        let doc = Document::new(&uri, "".to_string(), 1, &mut parser);
        assert_eq!(doc.language, SourceLanguage::TypeScriptReact);

        let uri = create_test_uri("test.js");
        let doc = Document::new(&uri, "".to_string(), 1, &mut parser);
        assert_eq!(doc.language, SourceLanguage::JavaScript);

        let uri = create_test_uri("test.jsx");
        let doc = Document::new(&uri, "".to_string(), 1, &mut parser);
        assert_eq!(doc.language, SourceLanguage::JavaScriptReact);
    }

    #[test]
    fn test_document_apply_changes_full() {
        let mut parser = SourceParser::default();
        let uri = create_test_uri("test.ts");
        let mut doc = Document::new(&uri, "const x = 1;".to_string(), 1, &mut parser);

        let changes = vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "const y = 2;".to_string(),
        }];

        doc.apply_changes(changes, 2, &mut parser);

        assert_eq!(doc.content, "const y = 2;");
        assert_eq!(doc.version, 2);
        assert!(doc.tree.is_some());
    }

    #[test]
    fn test_document_apply_changes_incremental() {
        let mut parser = SourceParser::default();
        let uri = create_test_uri("test.ts");
        let mut doc = Document::new(&uri, "const x = 1;".to_string(), 1, &mut parser);

        // Change "1" to "42"
        let changes = vec![TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 0, character: 10 },
                end: Position { line: 0, character: 11 },
            }),
            range_length: Some(1),
            text: "42".to_string(),
        }];

        doc.apply_changes(changes, 2, &mut parser);

        assert_eq!(doc.content, "const x = 42;");
        assert_eq!(doc.version, 2);
    }

    #[test]
    fn test_document_offset_at_position() {
        let mut parser = SourceParser::default();
        let uri = create_test_uri("test.ts");
        let content = "line1\nline2\nline3".to_string();
        let doc = Document::new(&uri, content, 1, &mut parser);

        // Test line 0, character 0
        let offset = doc.offset_at_position(Position { line: 0, character: 0 });
        assert_eq!(offset, 0);

        // Test line 0, character 3
        let offset = doc.offset_at_position(Position { line: 0, character: 3 });
        assert_eq!(offset, 3);

        // Test line 1, character 0 (after "line1\n")
        let offset = doc.offset_at_position(Position { line: 1, character: 0 });
        assert_eq!(offset, 6);

        // Test line 2, character 2
        let offset = doc.offset_at_position(Position { line: 2, character: 2 });
        assert_eq!(offset, 14);
    }

    #[test]
    fn test_document_position_at_offset() {
        let mut parser = SourceParser::default();
        let uri = create_test_uri("test.ts");
        let content = "line1\nline2\nline3".to_string();
        let doc = Document::new(&uri, content, 1, &mut parser);

        let pos = doc.position_at_offset(0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);

        let pos = doc.position_at_offset(3);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 3);

        let pos = doc.position_at_offset(6);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);

        let pos = doc.position_at_offset(14);
        assert_eq!(pos.line, 2);
        assert_eq!(pos.character, 2);
    }

    #[test]
    fn test_document_manager_new() {
        let manager = DocumentManager::new();
        let uri = create_test_uri("test.ts");
        assert!(manager.get(&uri).is_none());
    }

    #[test]
    fn test_document_manager_default() {
        let manager = DocumentManager::default();
        let uri = create_test_uri("test.ts");
        assert!(manager.get(&uri).is_none());
    }

    #[test]
    fn test_document_manager_open_and_get() {
        let manager = DocumentManager::new();
        let mut parser = SourceParser::default();
        let uri = create_test_uri("test.ts");

        manager.open(uri.clone(), "const x = 1;".to_string(), 1, &mut parser);

        let doc = manager.get(&uri);
        assert!(doc.is_some());
        let doc = doc.unwrap();
        assert_eq!(doc.content, "const x = 1;");
        assert_eq!(doc.version, 1);
    }

    #[test]
    fn test_document_manager_change() {
        let manager = DocumentManager::new();
        let mut parser = SourceParser::default();
        let uri = create_test_uri("test.ts");

        manager.open(uri.clone(), "const x = 1;".to_string(), 1, &mut parser);

        let changes = vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "const y = 2;".to_string(),
        }];

        manager.change(&uri, changes, 2, &mut parser);

        let doc = manager.get(&uri).unwrap();
        assert_eq!(doc.content, "const y = 2;");
        assert_eq!(doc.version, 2);
    }

    #[test]
    fn test_document_manager_close() {
        let manager = DocumentManager::new();
        let mut parser = SourceParser::default();
        let uri = create_test_uri("test.ts");

        manager.open(uri.clone(), "const x = 1;".to_string(), 1, &mut parser);
        assert!(manager.get(&uri).is_some());

        manager.close(&uri);
        assert!(manager.get(&uri).is_none());
    }

    #[test]
    fn test_document_manager_change_nonexistent() {
        let manager = DocumentManager::new();
        let mut parser = SourceParser::default();
        let uri = create_test_uri("test.ts");

        // Should not panic
        let changes = vec![TextDocumentContentChangeEvent {
            range: None,
            range_length: None,
            text: "const y = 2;".to_string(),
        }];

        manager.change(&uri, changes, 2, &mut parser);
    }

    #[test]
    fn test_document_with_complex_code() {
        let mut parser = SourceParser::default();
        let uri = create_test_uri("test.ts");
        let content = r#"
            interface User {
                id: number;
                name: string;
            }

            class UserService {
                private users: User[] = [];

                addUser(user: User): void {
                    this.users.push(user);
                }

                getUser(id: number): User | undefined {
                    return this.users.find(u => u.id === id);
                }
            }

            export { User, UserService };
        "#.to_string();

        let doc = Document::new(&uri, content, 1, &mut parser);

        assert!(doc.tree.is_some());
        assert!(doc.symbol_table.is_some());

        let tree = doc.tree.as_ref().unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn test_document_multiline_changes() {
        let mut parser = SourceParser::default();
        let uri = create_test_uri("test.ts");
        let content = "line1\nline2\nline3".to_string();
        let mut doc = Document::new(&uri, content, 1, &mut parser);

        // Replace "line2" with "replaced"
        let changes = vec![TextDocumentContentChangeEvent {
            range: Some(Range {
                start: Position { line: 1, character: 0 },
                end: Position { line: 1, character: 5 },
            }),
            range_length: Some(5),
            text: "replaced".to_string(),
        }];

        doc.apply_changes(changes, 2, &mut parser);

        assert_eq!(doc.content, "line1\nreplaced\nline3");
    }
}
