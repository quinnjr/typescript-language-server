use tower_lsp::lsp_types::Url;
use tree_sitter::{Parser, Tree};

/// Supported source languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLanguage {
    TypeScript,
    TypeScriptReact,
    JavaScript,
    JavaScriptReact,
}

impl SourceLanguage {
    /// Detect language from file URI
    pub fn from_uri(uri: &Url) -> Self {
        let path = uri.path();
        Self::from_extension(path)
    }

    /// Detect language from file extension
    pub fn from_extension(path: &str) -> Self {
        if path.ends_with(".tsx") {
            Self::TypeScriptReact
        } else if path.ends_with(".ts") || path.ends_with(".mts") || path.ends_with(".cts") {
            Self::TypeScript
        } else if path.ends_with(".jsx") {
            Self::JavaScriptReact
        } else {
            // .js, .mjs, .cjs, or unknown defaults to JavaScript
            Self::JavaScript
        }
    }

    /// Check if this is a React variant (TSX or JSX)
    pub fn is_react(&self) -> bool {
        matches!(self, Self::TypeScriptReact | Self::JavaScriptReact)
    }

    /// Check if this is a TypeScript variant
    pub fn is_typescript(&self) -> bool {
        matches!(self, Self::TypeScript | Self::TypeScriptReact)
    }
}

/// Multi-language parser wrapper using tree-sitter
pub struct SourceParser {
    parser: Parser,
    language: SourceLanguage,
}

impl SourceParser {
    /// Create a new parser for the specified language
    pub fn new(language: SourceLanguage) -> Self {
        let mut parser = Parser::new();

        let tree_sitter_lang = match language {
            SourceLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            SourceLanguage::TypeScriptReact => tree_sitter_typescript::LANGUAGE_TSX.into(),
            SourceLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            SourceLanguage::JavaScriptReact => tree_sitter_javascript::LANGUAGE.into(),
        };

        parser
            .set_language(&tree_sitter_lang)
            .expect("Failed to load grammar");

        Self { parser, language }
    }

    /// Create a parser based on file URI
    pub fn from_uri(uri: &Url) -> Self {
        Self::new(SourceLanguage::from_uri(uri))
    }

    /// Get the current language
    pub fn language(&self) -> SourceLanguage {
        self.language
    }

    /// Parse source code into a syntax tree
    pub fn parse(&mut self, source: &str, old_tree: Option<&Tree>) -> Option<Tree> {
        self.parser.parse(source, old_tree)
    }

    /// Change the parser's language
    pub fn set_language(&mut self, language: SourceLanguage) {
        if self.language != language {
            let tree_sitter_lang = match language {
                SourceLanguage::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                SourceLanguage::TypeScriptReact => tree_sitter_typescript::LANGUAGE_TSX.into(),
                SourceLanguage::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
                SourceLanguage::JavaScriptReact => tree_sitter_javascript::LANGUAGE.into(),
            };

            self.parser
                .set_language(&tree_sitter_lang)
                .expect("Failed to load grammar");
            self.language = language;
        }
    }
}

impl Default for SourceParser {
    fn default() -> Self {
        Self::new(SourceLanguage::TypeScript)
    }
}

// Keep backwards compatibility alias
pub type TsParser = SourceParser;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_language_from_extension() {
        assert_eq!(
            SourceLanguage::from_extension("file.ts"),
            SourceLanguage::TypeScript
        );
        assert_eq!(
            SourceLanguage::from_extension("file.tsx"),
            SourceLanguage::TypeScriptReact
        );
        assert_eq!(
            SourceLanguage::from_extension("file.js"),
            SourceLanguage::JavaScript
        );
        assert_eq!(
            SourceLanguage::from_extension("file.jsx"),
            SourceLanguage::JavaScriptReact
        );
        assert_eq!(
            SourceLanguage::from_extension("file.mts"),
            SourceLanguage::TypeScript
        );
        assert_eq!(
            SourceLanguage::from_extension("file.cts"),
            SourceLanguage::TypeScript
        );
        assert_eq!(
            SourceLanguage::from_extension("file.mjs"),
            SourceLanguage::JavaScript
        );
        assert_eq!(
            SourceLanguage::from_extension("file.cjs"),
            SourceLanguage::JavaScript
        );
        // Unknown defaults to JavaScript
        assert_eq!(
            SourceLanguage::from_extension("file.unknown"),
            SourceLanguage::JavaScript
        );
    }

    #[test]
    fn test_source_language_is_react() {
        assert!(!SourceLanguage::TypeScript.is_react());
        assert!(SourceLanguage::TypeScriptReact.is_react());
        assert!(!SourceLanguage::JavaScript.is_react());
        assert!(SourceLanguage::JavaScriptReact.is_react());
    }

    #[test]
    fn test_source_language_is_typescript() {
        assert!(SourceLanguage::TypeScript.is_typescript());
        assert!(SourceLanguage::TypeScriptReact.is_typescript());
        assert!(!SourceLanguage::JavaScript.is_typescript());
        assert!(!SourceLanguage::JavaScriptReact.is_typescript());
    }

    #[test]
    fn test_source_language_from_uri() {
        let uri = Url::parse("file:///path/to/file.ts").unwrap();
        assert_eq!(SourceLanguage::from_uri(&uri), SourceLanguage::TypeScript);

        let uri = Url::parse("file:///path/to/file.tsx").unwrap();
        assert_eq!(SourceLanguage::from_uri(&uri), SourceLanguage::TypeScriptReact);

        let uri = Url::parse("file:///path/to/file.js").unwrap();
        assert_eq!(SourceLanguage::from_uri(&uri), SourceLanguage::JavaScript);
    }

    #[test]
    fn test_parser_new() {
        let parser = SourceParser::new(SourceLanguage::TypeScript);
        assert_eq!(parser.language(), SourceLanguage::TypeScript);
    }

    #[test]
    fn test_parser_default() {
        let parser = SourceParser::default();
        assert_eq!(parser.language(), SourceLanguage::TypeScript);
    }

    #[test]
    fn test_parser_from_uri() {
        let uri = Url::parse("file:///path/to/file.tsx").unwrap();
        let parser = SourceParser::from_uri(&uri);
        assert_eq!(parser.language(), SourceLanguage::TypeScriptReact);
    }

    #[test]
    fn test_parser_set_language() {
        let mut parser = SourceParser::new(SourceLanguage::TypeScript);
        assert_eq!(parser.language(), SourceLanguage::TypeScript);

        parser.set_language(SourceLanguage::JavaScript);
        assert_eq!(parser.language(), SourceLanguage::JavaScript);

        // Setting same language should be no-op
        parser.set_language(SourceLanguage::JavaScript);
        assert_eq!(parser.language(), SourceLanguage::JavaScript);
    }

    #[test]
    fn test_parse_typescript() {
        let mut parser = SourceParser::new(SourceLanguage::TypeScript);
        let code = r#"
            interface User {
                name: string;
                age: number;
            }

            function greet(user: User): string {
                return `Hello, ${user.name}!`;
            }
        "#;

        let tree = parser.parse(code, None);
        assert!(tree.is_some());

        let tree = tree.unwrap();
        let root = tree.root_node();
        assert_eq!(root.kind(), "program");
        assert!(root.child_count() > 0);
    }

    #[test]
    fn test_parse_javascript() {
        let mut parser = SourceParser::new(SourceLanguage::JavaScript);
        let code = r#"
            function add(a, b) {
                return a + b;
            }

            const result = add(1, 2);
        "#;

        let tree = parser.parse(code, None);
        assert!(tree.is_some());

        let tree = tree.unwrap();
        let root = tree.root_node();
        assert_eq!(root.kind(), "program");
    }

    #[test]
    fn test_parse_tsx() {
        let mut parser = SourceParser::new(SourceLanguage::TypeScriptReact);
        let code = r#"
            interface Props {
                name: string;
            }

            const Component: React.FC<Props> = ({ name }) => {
                return <div>Hello, {name}!</div>;
            };
        "#;

        let tree = parser.parse(code, None);
        assert!(tree.is_some());
    }

    #[test]
    fn test_parse_jsx() {
        let mut parser = SourceParser::new(SourceLanguage::JavaScriptReact);
        let code = r#"
            function Component({ name }) {
                return <div>Hello, {name}!</div>;
            }
        "#;

        let tree = parser.parse(code, None);
        assert!(tree.is_some());
    }

    #[test]
    fn test_incremental_parsing() {
        let mut parser = SourceParser::new(SourceLanguage::TypeScript);

        let original = "const x = 1;";
        let tree1 = parser.parse(original, None).unwrap();

        let modified = "const x = 42;";
        let tree2 = parser.parse(modified, Some(&tree1));
        assert!(tree2.is_some());
    }

    #[test]
    fn test_parse_syntax_error() {
        let mut parser = SourceParser::new(SourceLanguage::TypeScript);
        let code = "function ( { }"; // Invalid syntax

        let tree = parser.parse(code, None);
        assert!(tree.is_some()); // tree-sitter still produces a tree

        let tree = tree.unwrap();
        let root = tree.root_node();
        // The tree should contain error nodes
        assert!(root.has_error());
    }

    #[test]
    fn test_parse_empty_string() {
        let mut parser = SourceParser::new(SourceLanguage::TypeScript);
        let tree = parser.parse("", None);
        assert!(tree.is_some());
    }

    #[test]
    fn test_parse_class() {
        let mut parser = SourceParser::new(SourceLanguage::TypeScript);
        let code = r#"
            class Person {
                private name: string;

                constructor(name: string) {
                    this.name = name;
                }

                greet(): string {
                    return `Hello, I'm ${this.name}`;
                }
            }
        "#;

        let tree = parser.parse(code, None);
        assert!(tree.is_some());

        let tree = tree.unwrap();
        assert!(!tree.root_node().has_error());
    }

    #[test]
    fn test_parse_generics() {
        let mut parser = SourceParser::new(SourceLanguage::TypeScript);
        let code = r#"
            function identity<T>(value: T): T {
                return value;
            }

            interface Container<T> {
                value: T;
            }

            type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };
        "#;

        let tree = parser.parse(code, None);
        assert!(tree.is_some());
        assert!(!tree.unwrap().root_node().has_error());
    }
}
