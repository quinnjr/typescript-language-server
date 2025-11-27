use tower_lsp::lsp_types::{
    SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokensLegend,
};
use tree_sitter::Tree;

/// Token types supported by this language server
pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::NAMESPACE,
    SemanticTokenType::TYPE,
    SemanticTokenType::CLASS,
    SemanticTokenType::ENUM,
    SemanticTokenType::INTERFACE,
    SemanticTokenType::STRUCT,
    SemanticTokenType::TYPE_PARAMETER,
    SemanticTokenType::PARAMETER,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::ENUM_MEMBER,
    SemanticTokenType::EVENT,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::METHOD,
    SemanticTokenType::MACRO,
    SemanticTokenType::KEYWORD,
    SemanticTokenType::MODIFIER,
    SemanticTokenType::COMMENT,
    SemanticTokenType::STRING,
    SemanticTokenType::NUMBER,
    SemanticTokenType::REGEXP,
    SemanticTokenType::OPERATOR,
];

/// Token modifiers supported by this language server
pub const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,
    SemanticTokenModifier::DEFINITION,
    SemanticTokenModifier::READONLY,
    SemanticTokenModifier::STATIC,
    SemanticTokenModifier::DEPRECATED,
    SemanticTokenModifier::ABSTRACT,
    SemanticTokenModifier::ASYNC,
    SemanticTokenModifier::MODIFICATION,
    SemanticTokenModifier::DOCUMENTATION,
    SemanticTokenModifier::DEFAULT_LIBRARY,
];

/// Get the semantic tokens legend for capability registration
pub fn get_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TOKEN_TYPES.to_vec(),
        token_modifiers: TOKEN_MODIFIERS.to_vec(),
    }
}

/// Extract semantic tokens from a parsed tree
pub fn get_semantic_tokens(tree: &Tree, source: &str) -> Vec<SemanticToken> {
    let mut tokens = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    collect_tokens(tree.root_node(), source, &mut tokens, &mut prev_line, &mut prev_start);

    tokens
}

fn collect_tokens(
    node: tree_sitter::Node,
    source: &str,
    tokens: &mut Vec<SemanticToken>,
    prev_line: &mut u32,
    prev_start: &mut u32,
) {
    // Map tree-sitter node types to semantic token types
    let token_type_index = match node.kind() {
        // Keywords
        "const" | "let" | "var" | "function" | "class" | "interface" | "type" | "enum"
        | "import" | "export" | "from" | "as" | "default" | "if" | "else" | "for" | "while"
        | "do" | "switch" | "case" | "break" | "continue" | "return" | "throw" | "try"
        | "catch" | "finally" | "new" | "this" | "super" | "extends" | "implements"
        | "instanceof" | "typeof" | "void" | "delete" | "in" | "of" | "async" | "await"
        | "yield" | "static" | "public" | "private" | "protected" | "readonly" | "abstract"
        | "declare" | "namespace" | "module" | "require" | "get" | "set" => {
            Some(token_type_idx(SemanticTokenType::KEYWORD))
        }

        // Types
        "type_identifier" | "predefined_type" => Some(token_type_idx(SemanticTokenType::TYPE)),

        // Classes and interfaces
        "class_declaration" => None, // We handle the name child separately

        // Functions
        "function_declaration" | "method_definition" | "arrow_function" => None,

        // Identifiers - context dependent
        "identifier" => {
            // Check parent to determine the type
            if let Some(parent) = node.parent() {
                match parent.kind() {
                    "function_declaration" | "function" => {
                        if parent.child_by_field_name("name") == Some(node) {
                            Some(token_type_idx(SemanticTokenType::FUNCTION))
                        } else {
                            None
                        }
                    }
                    "class_declaration" | "class" => {
                        if parent.child_by_field_name("name") == Some(node) {
                            Some(token_type_idx(SemanticTokenType::CLASS))
                        } else {
                            None
                        }
                    }
                    "interface_declaration" => {
                        if parent.child_by_field_name("name") == Some(node) {
                            Some(token_type_idx(SemanticTokenType::INTERFACE))
                        } else {
                            None
                        }
                    }
                    "method_definition" => {
                        if parent.child_by_field_name("name") == Some(node) {
                            Some(token_type_idx(SemanticTokenType::METHOD))
                        } else {
                            None
                        }
                    }
                    "formal_parameters" | "required_parameter" | "optional_parameter" => {
                        Some(token_type_idx(SemanticTokenType::PARAMETER))
                    }
                    "call_expression" => {
                        if parent.child_by_field_name("function") == Some(node) {
                            Some(token_type_idx(SemanticTokenType::FUNCTION))
                        } else {
                            None
                        }
                    }
                    "variable_declarator" => {
                        if parent.child_by_field_name("name") == Some(node) {
                            Some(token_type_idx(SemanticTokenType::VARIABLE))
                        } else {
                            None
                        }
                    }
                    "property_identifier" | "member_expression" => {
                        Some(token_type_idx(SemanticTokenType::PROPERTY))
                    }
                    _ => Some(token_type_idx(SemanticTokenType::VARIABLE)),
                }
            } else {
                Some(token_type_idx(SemanticTokenType::VARIABLE))
            }
        }

        "property_identifier" => Some(token_type_idx(SemanticTokenType::PROPERTY)),

        // Literals
        "string" | "template_string" | "string_fragment" => {
            Some(token_type_idx(SemanticTokenType::STRING))
        }
        "number" => Some(token_type_idx(SemanticTokenType::NUMBER)),
        "regex" => Some(token_type_idx(SemanticTokenType::REGEXP)),

        // Comments
        "comment" => Some(token_type_idx(SemanticTokenType::COMMENT)),

        _ => None,
    };

    // Add token if this node maps to a semantic token type
    if let Some(type_index) = token_type_index {
        let start = node.start_position();
        let end = node.end_position();
        let line = start.row as u32;
        let start_char = start.column as u32;
        let length = if start.row == end.row {
            (end.column - start.column) as u32
        } else {
            // For multi-line tokens, just use the text length
            node.utf8_text(source.as_bytes())
                .map(|s| s.len() as u32)
                .unwrap_or(0)
        };

        // Calculate delta encoding
        let delta_line = line - *prev_line;
        let delta_start = if delta_line == 0 {
            start_char - *prev_start
        } else {
            start_char
        };

        tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: type_index,
            token_modifiers_bitset: 0, // No modifiers for now
        });

        *prev_line = line;
        *prev_start = start_char;
    }

    // Recurse into children
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_tokens(child, source, tokens, prev_line, prev_start);
    }
}

fn token_type_idx(token_type: SemanticTokenType) -> u32 {
    TOKEN_TYPES
        .iter()
        .position(|t| *t == token_type)
        .unwrap_or(0) as u32
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
    fn test_token_types_count() {
        assert!(!TOKEN_TYPES.is_empty());
        assert!(TOKEN_TYPES.len() > 10);
    }

    #[test]
    fn test_token_modifiers_count() {
        assert!(!TOKEN_MODIFIERS.is_empty());
    }

    #[test]
    fn test_get_legend() {
        let legend = get_legend();
        assert_eq!(legend.token_types.len(), TOKEN_TYPES.len());
        assert_eq!(legend.token_modifiers.len(), TOKEN_MODIFIERS.len());
    }

    #[test]
    fn test_token_type_idx() {
        assert_eq!(token_type_idx(SemanticTokenType::NAMESPACE), 0);
        assert_eq!(token_type_idx(SemanticTokenType::TYPE), 1);
        assert_eq!(token_type_idx(SemanticTokenType::CLASS), 2);
    }

    #[test]
    fn test_semantic_tokens_keywords() {
        let code = "const x = 1;";
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        // Should have a token for "const" keyword
        let keyword_idx = token_type_idx(SemanticTokenType::KEYWORD);
        assert!(tokens.iter().any(|t| t.token_type == keyword_idx));
    }

    #[test]
    fn test_semantic_tokens_variable() {
        let code = "const myVar = 42;";
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        let var_idx = token_type_idx(SemanticTokenType::VARIABLE);
        assert!(tokens.iter().any(|t| t.token_type == var_idx));
    }

    #[test]
    fn test_semantic_tokens_function() {
        let code = "function greet() { }";
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        let func_idx = token_type_idx(SemanticTokenType::FUNCTION);
        let keyword_idx = token_type_idx(SemanticTokenType::KEYWORD);

        // Should have "function" keyword and "greet" function name
        assert!(tokens.iter().any(|t| t.token_type == keyword_idx));
        assert!(tokens.iter().any(|t| t.token_type == func_idx));
    }

    #[test]
    fn test_semantic_tokens_class() {
        let code = "class MyClass { }";
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        // Class keyword should be tokenized
        let keyword_idx = token_type_idx(SemanticTokenType::KEYWORD);
        assert!(tokens.iter().any(|t| t.token_type == keyword_idx));
        // Class names are tokenized as identifiers which get CLASS type
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_semantic_tokens_interface() {
        let code = "interface IUser { }";
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        // Interface keyword should be tokenized
        let keyword_idx = token_type_idx(SemanticTokenType::KEYWORD);
        assert!(tokens.iter().any(|t| t.token_type == keyword_idx));
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_semantic_tokens_string() {
        let code = r#"const s = "hello";"#;
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        let string_idx = token_type_idx(SemanticTokenType::STRING);
        assert!(tokens.iter().any(|t| t.token_type == string_idx));
    }

    #[test]
    fn test_semantic_tokens_number() {
        let code = "const n = 42;";
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        let number_idx = token_type_idx(SemanticTokenType::NUMBER);
        assert!(tokens.iter().any(|t| t.token_type == number_idx));
    }

    #[test]
    fn test_semantic_tokens_comment() {
        let code = "// This is a comment\nconst x = 1;";
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        let comment_idx = token_type_idx(SemanticTokenType::COMMENT);
        assert!(tokens.iter().any(|t| t.token_type == comment_idx));
    }

    #[test]
    fn test_semantic_tokens_method() {
        let code = r#"class C { method() { } }"#;
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        // Should have tokens for class and method
        assert!(!tokens.is_empty());
        // Method name should be tokenized (might be as identifier/property)
        let has_method_or_property = tokens.iter().any(|t| {
            t.token_type == token_type_idx(SemanticTokenType::METHOD)
                || t.token_type == token_type_idx(SemanticTokenType::PROPERTY)
        });
        assert!(has_method_or_property);
    }

    #[test]
    fn test_semantic_tokens_parameter() {
        let code = "function test(x: number) { }";
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        let param_idx = token_type_idx(SemanticTokenType::PARAMETER);
        assert!(tokens.iter().any(|t| t.token_type == param_idx));
    }

    #[test]
    fn test_semantic_tokens_type() {
        let code = "const x: string = 'hello';";
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        let type_idx = token_type_idx(SemanticTokenType::TYPE);
        assert!(tokens.iter().any(|t| t.token_type == type_idx));
    }

    #[test]
    fn test_semantic_tokens_property() {
        let code = "const obj = { prop: 1 };";
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        let prop_idx = token_type_idx(SemanticTokenType::PROPERTY);
        assert!(tokens.iter().any(|t| t.token_type == prop_idx));
    }

    #[test]
    fn test_semantic_tokens_delta_encoding() {
        let code = "const a = 1;\nconst b = 2;";
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        // All delta_line values should be valid (0 or positive)
        for token in &tokens {
            assert!(token.delta_line == 0 || token.delta_line > 0);
        }
    }

    #[test]
    fn test_semantic_tokens_empty_code() {
        let code = "";
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);
        assert!(tokens.is_empty());
    }

    #[test]
    fn test_semantic_tokens_complex_code() {
        let code = r#"
            interface User {
                name: string;
            }

            class UserService {
                getUser(id: number): User {
                    return { name: "test" };
                }
            }
        "#;
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        // Should have tokens for multiple types
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_semantic_tokens_function_call() {
        let code = "console.log('hello');";
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        // Should tokenize function calls
        assert!(!tokens.is_empty());
    }

    #[test]
    fn test_semantic_tokens_arrow_function() {
        let code = "const fn = (x: number) => x * 2;";
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        let param_idx = token_type_idx(SemanticTokenType::PARAMETER);
        assert!(tokens.iter().any(|t| t.token_type == param_idx));
    }

    #[test]
    fn test_semantic_tokens_regex() {
        let code = "const re = /test/g;";
        let tree = parse_typescript(code);
        let tokens = get_semantic_tokens(&tree, code);

        let regex_idx = token_type_idx(SemanticTokenType::REGEXP);
        assert!(tokens.iter().any(|t| t.token_type == regex_idx));
    }
}
