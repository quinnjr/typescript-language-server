use tower_lsp::lsp_types::{
    CompletionItem, CompletionItemKind, CompletionItemLabelDetails, CompletionParams,
    Documentation, InsertTextFormat, MarkupContent, MarkupKind, Position,
};
use tree_sitter::{Node, Tree};

use crate::analysis::{SymbolFlags, SymbolTable};

/// Get completions for a position in the document
pub fn get_completions(
    tree: &Tree,
    source: &str,
    symbol_table: &SymbolTable,
    params: &CompletionParams,
) -> Vec<CompletionItem> {
    let position = params.text_document_position.position;
    let mut completions = Vec::new();

    // Get the context at the cursor position
    let context = get_completion_context(tree, source, position);

    match context {
        CompletionContext::MemberAccess(object_name) => {
            // Complete object members
            completions.extend(get_member_completions(&object_name));
        }
        CompletionContext::Import => {
            // Complete import paths
            completions.extend(get_import_completions());
        }
        CompletionContext::Type => {
            // Complete type names
            completions.extend(get_type_completions(symbol_table));
            completions.extend(get_builtin_type_completions());
        }
        CompletionContext::General => {
            // Complete with symbols in scope
            completions.extend(get_scope_completions(symbol_table, position));
            completions.extend(get_keyword_completions());
            completions.extend(get_snippet_completions());
        }
        CompletionContext::JsxTag => {
            // Complete JSX tag names
            completions.extend(get_jsx_completions());
        }
        CompletionContext::JsxAttribute => {
            // Complete JSX attributes
            completions.extend(get_jsx_attribute_completions());
        }
    }

    completions
}

/// Completion context types
#[derive(Debug)]
enum CompletionContext {
    /// After a dot, completing object members
    MemberAccess(String),
    /// Inside an import statement
    Import,
    /// Type position (after colon, extends, implements, etc.)
    Type,
    /// General code context
    General,
    /// Inside JSX opening/closing tags
    JsxTag,
    /// Inside JSX attribute position
    JsxAttribute,
}

/// Determine the completion context at a position
fn get_completion_context(tree: &Tree, source: &str, position: Position) -> CompletionContext {
    let root = tree.root_node();

    let point = tree_sitter::Point {
        row: position.line as usize,
        column: position.character as usize,
    };

    // Find the node at position
    if let Some(node) = root.descendant_for_point_range(point, point) {
        // Check parent contexts
        let mut current = node;
        while let Some(parent) = current.parent() {
            match parent.kind() {
                "member_expression" => {
                    // Check if we're after a dot
                    if let Some(obj) = parent.child_by_field_name("object") {
                        let obj_text = obj.utf8_text(source.as_bytes()).unwrap_or("").to_string();
                        return CompletionContext::MemberAccess(obj_text);
                    }
                }
                "import_statement" | "import_clause" | "named_imports" => {
                    return CompletionContext::Import;
                }
                "type_annotation" | "type_alias_declaration" | "extends_clause"
                | "implements_clause" | "type_arguments" => {
                    return CompletionContext::Type;
                }
                "jsx_element" | "jsx_self_closing_element" | "jsx_opening_element" => {
                    // Check if we're in tag name position or attribute position
                    if is_in_jsx_attribute_position(&node, source) {
                        return CompletionContext::JsxAttribute;
                    }
                    return CompletionContext::JsxTag;
                }
                _ => {}
            }
            current = parent;
        }

        // Check if right after a dot
        if position.character > 0 {
            let lines: Vec<&str> = source.lines().collect();
            if let Some(line) = lines.get(position.line as usize) {
                let chars: Vec<char> = line.chars().collect();
                if position.character as usize > 0 {
                    if let Some('.') = chars.get(position.character as usize - 1) {
                        // Find what's before the dot
                        let before_dot = &line[..position.character as usize - 1];
                        let object_name = before_dot.split_whitespace().last().unwrap_or("").to_string();
                        if !object_name.is_empty() {
                            return CompletionContext::MemberAccess(object_name);
                        }
                    }
                }
            }
        }
    }

    CompletionContext::General
}

fn is_in_jsx_attribute_position(node: &Node, _source: &str) -> bool {
    let mut current = *node;
    while let Some(parent) = current.parent() {
        if parent.kind() == "jsx_attribute" {
            return true;
        }
        current = parent;
    }
    false
}

/// Get completions for symbols in the current scope
fn get_scope_completions(symbol_table: &SymbolTable, position: Position) -> Vec<CompletionItem> {
    let scope_id = symbol_table.scope_at_position(position);
    let mut completions = Vec::new();

    // Get symbols from current scope and parent scopes
    for symbol in symbol_table.all_symbols() {
        // Only include symbols visible from this scope
        if symbol_table.lookup(&symbol.name, scope_id).is_some() {
            let kind = symbol_flags_to_completion_kind(symbol.flags);

            completions.push(CompletionItem {
                label: symbol.name.clone(),
                kind: Some(kind),
                detail: Some(get_symbol_detail(symbol.flags)),
                label_details: Some(CompletionItemLabelDetails {
                    detail: None,
                    description: Some(get_symbol_description(symbol.flags)),
                }),
                documentation: symbol.documentation.clone().map(|doc| {
                    Documentation::MarkupContent(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: doc,
                    })
                }),
                ..Default::default()
            });
        }
    }

    completions
}

fn symbol_flags_to_completion_kind(flags: SymbolFlags) -> CompletionItemKind {
    if flags.contains(SymbolFlags::FUNCTION) {
        CompletionItemKind::FUNCTION
    } else if flags.contains(SymbolFlags::CLASS) {
        CompletionItemKind::CLASS
    } else if flags.contains(SymbolFlags::INTERFACE) {
        CompletionItemKind::INTERFACE
    } else if flags.contains(SymbolFlags::ENUM) {
        CompletionItemKind::ENUM
    } else if flags.contains(SymbolFlags::TYPE_ALIAS) {
        CompletionItemKind::TYPE_PARAMETER
    } else if flags.contains(SymbolFlags::METHOD) {
        CompletionItemKind::METHOD
    } else if flags.contains(SymbolFlags::PROPERTY) {
        CompletionItemKind::PROPERTY
    } else if flags.contains(SymbolFlags::PARAMETER) {
        CompletionItemKind::VARIABLE
    } else if flags.contains(SymbolFlags::CONST) {
        CompletionItemKind::CONSTANT
    } else if flags.contains(SymbolFlags::VARIABLE) {
        CompletionItemKind::VARIABLE
    } else {
        CompletionItemKind::TEXT
    }
}

fn get_symbol_detail(flags: SymbolFlags) -> String {
    if flags.contains(SymbolFlags::FUNCTION) {
        "function".to_string()
    } else if flags.contains(SymbolFlags::CLASS) {
        "class".to_string()
    } else if flags.contains(SymbolFlags::INTERFACE) {
        "interface".to_string()
    } else if flags.contains(SymbolFlags::ENUM) {
        "enum".to_string()
    } else if flags.contains(SymbolFlags::TYPE_ALIAS) {
        "type".to_string()
    } else if flags.contains(SymbolFlags::CONST) {
        "const".to_string()
    } else if flags.contains(SymbolFlags::LET) {
        "let".to_string()
    } else {
        "var".to_string()
    }
}

fn get_symbol_description(flags: SymbolFlags) -> String {
    let mut parts = Vec::new();

    if flags.contains(SymbolFlags::EXPORTED) {
        parts.push("export");
    }
    if flags.contains(SymbolFlags::ASYNC) {
        parts.push("async");
    }
    if flags.contains(SymbolFlags::STATIC) {
        parts.push("static");
    }

    parts.join(" ")
}

/// Get keyword completions
fn get_keyword_completions() -> Vec<CompletionItem> {
    let keywords = [
        ("const", "Declare a constant variable"),
        ("let", "Declare a block-scoped variable"),
        ("var", "Declare a function-scoped variable"),
        ("function", "Declare a function"),
        ("class", "Declare a class"),
        ("interface", "Declare a TypeScript interface"),
        ("type", "Declare a type alias"),
        ("enum", "Declare an enumeration"),
        ("if", "If statement"),
        ("else", "Else clause"),
        ("for", "For loop"),
        ("while", "While loop"),
        ("do", "Do-while loop"),
        ("switch", "Switch statement"),
        ("case", "Case clause"),
        ("default", "Default clause"),
        ("break", "Break statement"),
        ("continue", "Continue statement"),
        ("return", "Return statement"),
        ("throw", "Throw an exception"),
        ("try", "Try block"),
        ("catch", "Catch block"),
        ("finally", "Finally block"),
        ("import", "Import statement"),
        ("export", "Export statement"),
        ("from", "Import from clause"),
        ("as", "Alias clause"),
        ("async", "Async function modifier"),
        ("await", "Await expression"),
        ("new", "Create instance"),
        ("this", "Current instance"),
        ("super", "Parent class reference"),
        ("extends", "Class inheritance"),
        ("implements", "Interface implementation"),
        ("typeof", "Type of operator"),
        ("instanceof", "Instance of operator"),
        ("in", "In operator"),
        ("of", "Of operator (for-of)"),
        ("true", "Boolean true"),
        ("false", "Boolean false"),
        ("null", "Null value"),
        ("undefined", "Undefined value"),
        ("void", "Void type/operator"),
        ("never", "Never type"),
        ("any", "Any type"),
        ("unknown", "Unknown type"),
        ("string", "String type"),
        ("number", "Number type"),
        ("boolean", "Boolean type"),
        ("object", "Object type"),
        ("symbol", "Symbol type"),
        ("bigint", "BigInt type"),
    ];

    keywords
        .iter()
        .map(|(keyword, detail)| CompletionItem {
            label: (*keyword).to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some((*detail).to_string()),
            ..Default::default()
        })
        .collect()
}

/// Get snippet completions
fn get_snippet_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "log".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("console.log statement".to_string()),
            insert_text: Some("console.log($1);$0".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "fn".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Function declaration".to_string()),
            insert_text: Some("function ${1:name}(${2:params}): ${3:void} {\n\t$0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "afn".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Arrow function".to_string()),
            insert_text: Some("const ${1:name} = (${2:params}) => {\n\t$0\n};".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "iife".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Immediately invoked function expression".to_string()),
            insert_text: Some("(function() {\n\t$0\n})();".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "class".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Class declaration".to_string()),
            insert_text: Some("class ${1:ClassName} {\n\tconstructor(${2:params}) {\n\t\t$0\n\t}\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "interface".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Interface declaration".to_string()),
            insert_text: Some("interface ${1:InterfaceName} {\n\t$0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "type".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Type alias declaration".to_string()),
            insert_text: Some("type ${1:TypeName} = $0;".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "for".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("For loop".to_string()),
            insert_text: Some("for (let ${1:i} = 0; ${1:i} < ${2:length}; ${1:i}++) {\n\t$0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "forof".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("For-of loop".to_string()),
            insert_text: Some("for (const ${1:item} of ${2:items}) {\n\t$0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "forin".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("For-in loop".to_string()),
            insert_text: Some("for (const ${1:key} in ${2:obj}) {\n\t$0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "if".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("If statement".to_string()),
            insert_text: Some("if (${1:condition}) {\n\t$0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "ife".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("If-else statement".to_string()),
            insert_text: Some("if (${1:condition}) {\n\t$2\n} else {\n\t$0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "try".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Try-catch block".to_string()),
            insert_text: Some("try {\n\t$1\n} catch (${2:error}) {\n\t$0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "switch".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Switch statement".to_string()),
            insert_text: Some("switch (${1:expr}) {\n\tcase ${2:value}:\n\t\t$0\n\t\tbreak;\n\tdefault:\n\t\tbreak;\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "import".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Import statement".to_string()),
            insert_text: Some("import { $2 } from '$1';$0".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "export".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Export statement".to_string()),
            insert_text: Some("export { $0 };".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
        CompletionItem {
            label: "async".to_string(),
            kind: Some(CompletionItemKind::SNIPPET),
            detail: Some("Async function".to_string()),
            insert_text: Some("async function ${1:name}(${2:params}): Promise<${3:void}> {\n\t$0\n}".to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        },
    ]
}

/// Get member completions for common objects
fn get_member_completions(object_name: &str) -> Vec<CompletionItem> {
    match object_name {
        "console" => get_console_completions(),
        "Math" => get_math_completions(),
        "JSON" => get_json_completions(),
        "Object" => get_object_completions(),
        "Array" => get_array_static_completions(),
        "String" => get_string_static_completions(),
        "Promise" => get_promise_completions(),
        _ => Vec::new(),
    }
}

fn get_console_completions() -> Vec<CompletionItem> {
    vec![
        create_method_completion("log", "(...data: any[]): void", "Log output to console"),
        create_method_completion("error", "(...data: any[]): void", "Log error to console"),
        create_method_completion("warn", "(...data: any[]): void", "Log warning to console"),
        create_method_completion("info", "(...data: any[]): void", "Log info to console"),
        create_method_completion("debug", "(...data: any[]): void", "Log debug to console"),
        create_method_completion("table", "(data: any): void", "Display data as table"),
        create_method_completion("time", "(label?: string): void", "Start timer"),
        create_method_completion("timeEnd", "(label?: string): void", "End timer"),
        create_method_completion("trace", "(...data: any[]): void", "Log stack trace"),
        create_method_completion("clear", "(): void", "Clear console"),
        create_method_completion("count", "(label?: string): void", "Count occurrences"),
        create_method_completion("group", "(...data: any[]): void", "Start group"),
        create_method_completion("groupEnd", "(): void", "End group"),
        create_method_completion("assert", "(condition?: boolean, ...data: any[]): void", "Assert condition"),
    ]
}

fn get_math_completions() -> Vec<CompletionItem> {
    vec![
        create_method_completion("abs", "(x: number): number", "Absolute value"),
        create_method_completion("ceil", "(x: number): number", "Round up"),
        create_method_completion("floor", "(x: number): number", "Round down"),
        create_method_completion("round", "(x: number): number", "Round to nearest"),
        create_method_completion("max", "(...values: number[]): number", "Maximum value"),
        create_method_completion("min", "(...values: number[]): number", "Minimum value"),
        create_method_completion("random", "(): number", "Random number 0-1"),
        create_method_completion("pow", "(base: number, exponent: number): number", "Power"),
        create_method_completion("sqrt", "(x: number): number", "Square root"),
        create_method_completion("sin", "(x: number): number", "Sine"),
        create_method_completion("cos", "(x: number): number", "Cosine"),
        create_method_completion("tan", "(x: number): number", "Tangent"),
        create_method_completion("log", "(x: number): number", "Natural logarithm"),
        create_method_completion("exp", "(x: number): number", "e^x"),
        create_property_completion("PI", "number", "Pi constant"),
        create_property_completion("E", "number", "Euler's number"),
    ]
}

fn get_json_completions() -> Vec<CompletionItem> {
    vec![
        create_method_completion("parse", "(text: string): any", "Parse JSON string"),
        create_method_completion("stringify", "(value: any): string", "Convert to JSON string"),
    ]
}

fn get_object_completions() -> Vec<CompletionItem> {
    vec![
        create_method_completion("keys", "(obj: object): string[]", "Get object keys"),
        create_method_completion("values", "(obj: object): any[]", "Get object values"),
        create_method_completion("entries", "(obj: object): [string, any][]", "Get key-value pairs"),
        create_method_completion("assign", "(target: object, ...sources: object[]): object", "Copy properties"),
        create_method_completion("freeze", "<T>(obj: T): Readonly<T>", "Freeze object"),
        create_method_completion("seal", "<T>(obj: T): T", "Seal object"),
        create_method_completion("create", "(proto: object | null): object", "Create with prototype"),
        create_method_completion("defineProperty", "(obj: object, prop: string, descriptor: object): object", "Define property"),
        create_method_completion("hasOwn", "(obj: object, prop: string): boolean", "Check own property"),
        create_method_completion("fromEntries", "(entries: Iterable<[string, any]>): object", "Create from entries"),
    ]
}

fn get_array_static_completions() -> Vec<CompletionItem> {
    vec![
        create_method_completion("isArray", "(value: any): boolean", "Check if array"),
        create_method_completion("from", "(arrayLike: ArrayLike<any>): any[]", "Create from array-like"),
        create_method_completion("of", "(...items: any[]): any[]", "Create from items"),
    ]
}

fn get_string_static_completions() -> Vec<CompletionItem> {
    vec![
        create_method_completion("fromCharCode", "(...codes: number[]): string", "Create from char codes"),
        create_method_completion("fromCodePoint", "(...codePoints: number[]): string", "Create from code points"),
        create_method_completion("raw", "(template: TemplateStringsArray, ...substitutions: any[]): string", "Raw template string"),
    ]
}

fn get_promise_completions() -> Vec<CompletionItem> {
    vec![
        create_method_completion("all", "<T>(values: Promise<T>[]): Promise<T[]>", "Wait for all promises"),
        create_method_completion("race", "<T>(values: Promise<T>[]): Promise<T>", "First settled promise"),
        create_method_completion("resolve", "<T>(value: T): Promise<T>", "Create resolved promise"),
        create_method_completion("reject", "(reason: any): Promise<never>", "Create rejected promise"),
        create_method_completion("allSettled", "<T>(values: Promise<T>[]): Promise<PromiseSettledResult<T>[]>", "All settled results"),
        create_method_completion("any", "<T>(values: Promise<T>[]): Promise<T>", "First fulfilled promise"),
    ]
}

fn create_method_completion(name: &str, signature: &str, description: &str) -> CompletionItem {
    CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::METHOD),
        detail: Some(signature.to_string()),
        documentation: Some(Documentation::String(description.to_string())),
        insert_text: Some(format!("{}($0)", name)),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
    }
}

fn create_property_completion(name: &str, ty: &str, description: &str) -> CompletionItem {
    CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::PROPERTY),
        detail: Some(ty.to_string()),
        documentation: Some(Documentation::String(description.to_string())),
        ..Default::default()
    }
}

/// Get type completions from the symbol table
fn get_type_completions(symbol_table: &SymbolTable) -> Vec<CompletionItem> {
    symbol_table
        .all_symbols()
        .filter(|s| {
            s.flags.intersects(
                SymbolFlags::CLASS
                    | SymbolFlags::INTERFACE
                    | SymbolFlags::TYPE_ALIAS
                    | SymbolFlags::ENUM
                    | SymbolFlags::TYPE_PARAMETER,
            )
        })
        .map(|s| CompletionItem {
            label: s.name.clone(),
            kind: Some(symbol_flags_to_completion_kind(s.flags)),
            detail: Some(get_symbol_detail(s.flags)),
            ..Default::default()
        })
        .collect()
}

/// Get built-in type completions
fn get_builtin_type_completions() -> Vec<CompletionItem> {
    let types = [
        ("string", "Primitive string type"),
        ("number", "Primitive number type"),
        ("boolean", "Primitive boolean type"),
        ("any", "Any type (disables type checking)"),
        ("unknown", "Unknown type (type-safe any)"),
        ("void", "Void type (no return value)"),
        ("never", "Never type (never returns)"),
        ("null", "Null type"),
        ("undefined", "Undefined type"),
        ("object", "Object type"),
        ("symbol", "Symbol type"),
        ("bigint", "BigInt type"),
        ("Array", "Array type"),
        ("Promise", "Promise type"),
        ("Record", "Record type"),
        ("Partial", "Partial type"),
        ("Required", "Required type"),
        ("Readonly", "Readonly type"),
        ("Pick", "Pick type"),
        ("Omit", "Omit type"),
        ("Exclude", "Exclude type"),
        ("Extract", "Extract type"),
        ("NonNullable", "NonNullable type"),
        ("ReturnType", "ReturnType type"),
        ("Parameters", "Parameters type"),
        ("InstanceType", "InstanceType type"),
    ];

    types
        .iter()
        .map(|(name, detail)| CompletionItem {
            label: (*name).to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some((*detail).to_string()),
            ..Default::default()
        })
        .collect()
}

/// Get import completions (placeholder - would need file system access)
fn get_import_completions() -> Vec<CompletionItem> {
    vec![
        CompletionItem {
            label: "react".to_string(),
            kind: Some(CompletionItemKind::MODULE),
            detail: Some("React library".to_string()),
            ..Default::default()
        },
        CompletionItem {
            label: "react-dom".to_string(),
            kind: Some(CompletionItemKind::MODULE),
            detail: Some("React DOM library".to_string()),
            ..Default::default()
        },
    ]
}

/// Get JSX tag completions
fn get_jsx_completions() -> Vec<CompletionItem> {
    let html_tags = [
        "div", "span", "p", "a", "button", "input", "form", "label",
        "h1", "h2", "h3", "h4", "h5", "h6", "header", "footer", "main",
        "nav", "section", "article", "aside", "ul", "ol", "li",
        "table", "tr", "td", "th", "thead", "tbody", "img", "video",
        "audio", "canvas", "svg", "iframe", "pre", "code", "br", "hr",
    ];

    html_tags
        .iter()
        .map(|tag| CompletionItem {
            label: (*tag).to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some("HTML element".to_string()),
            insert_text: Some(format!("{}>$0</{}", tag, tag)),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        })
        .collect()
}

/// Get JSX attribute completions
fn get_jsx_attribute_completions() -> Vec<CompletionItem> {
    let common_attrs = [
        ("className", "CSS class name"),
        ("id", "Element ID"),
        ("style", "Inline styles"),
        ("onClick", "Click event handler"),
        ("onChange", "Change event handler"),
        ("onSubmit", "Submit event handler"),
        ("value", "Input value"),
        ("placeholder", "Placeholder text"),
        ("disabled", "Disable element"),
        ("type", "Input type"),
        ("href", "Link URL"),
        ("src", "Source URL"),
        ("alt", "Alternative text"),
        ("key", "React key"),
        ("ref", "React ref"),
        ("children", "Child elements"),
    ];

    common_attrs
        .iter()
        .map(|(attr, detail)| CompletionItem {
            label: (*attr).to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some((*detail).to_string()),
            insert_text: Some(format!("{}=$0", attr)),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_completions() {
        let completions = get_keyword_completions();
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.label == "const"));
        assert!(completions.iter().any(|c| c.label == "function"));
    }

    #[test]
    fn test_snippet_completions() {
        let completions = get_snippet_completions();
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.label == "log"));
        assert!(completions.iter().any(|c| c.insert_text_format == Some(InsertTextFormat::SNIPPET)));
    }

    #[test]
    fn test_console_completions() {
        let completions = get_member_completions("console");
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.label == "log"));
    }

    #[test]
    fn test_math_completions() {
        let completions = get_member_completions("Math");
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.label == "abs"));
        assert!(completions.iter().any(|c| c.label == "PI"));
    }

    #[test]
    fn test_builtin_type_completions() {
        let completions = get_builtin_type_completions();
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.label == "string"));
        assert!(completions.iter().any(|c| c.label == "number"));
    }

    #[test]
    fn test_jsx_completions() {
        let completions = get_jsx_completions();
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.label == "div"));
    }

    #[test]
    fn test_jsx_attribute_completions() {
        let completions = get_jsx_attribute_completions();
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.label == "className"));
    }

    #[test]
    fn test_symbol_flags_to_completion_kind() {
        assert_eq!(symbol_flags_to_completion_kind(SymbolFlags::FUNCTION), CompletionItemKind::FUNCTION);
        assert_eq!(symbol_flags_to_completion_kind(SymbolFlags::CLASS), CompletionItemKind::CLASS);
        assert_eq!(symbol_flags_to_completion_kind(SymbolFlags::VARIABLE), CompletionItemKind::VARIABLE);
        assert_eq!(symbol_flags_to_completion_kind(SymbolFlags::CONST | SymbolFlags::VARIABLE), CompletionItemKind::CONSTANT);
    }
}

