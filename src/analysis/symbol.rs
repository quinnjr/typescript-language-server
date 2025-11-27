//! Symbol definitions for the analysis module
//! Some fields/methods are reserved for future type checking features

#![allow(dead_code)]

use tower_lsp::lsp_types::{Position, Range};

/// Unique identifier for a symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);

impl SymbolId {
    pub fn new(id: u32) -> Self {
        Self(id)
    }
}

bitflags::bitflags! {
    /// Flags describing the kind and properties of a symbol
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct SymbolFlags: u32 {
        const NONE = 0;

        // Declaration kinds
        const VARIABLE = 1 << 0;
        const FUNCTION = 1 << 1;
        const CLASS = 1 << 2;
        const INTERFACE = 1 << 3;
        const ENUM = 1 << 4;
        const TYPE_ALIAS = 1 << 5;
        const PARAMETER = 1 << 6;
        const PROPERTY = 1 << 7;
        const METHOD = 1 << 8;
        const NAMESPACE = 1 << 9;
        const ENUM_MEMBER = 1 << 10;
        const TYPE_PARAMETER = 1 << 11;

        // Modifiers
        const CONST = 1 << 16;
        const LET = 1 << 17;
        const EXPORTED = 1 << 18;
        const DEFAULT = 1 << 19;
        const ASYNC = 1 << 20;
        const STATIC = 1 << 21;
        const READONLY = 1 << 22;
        const PRIVATE = 1 << 23;
        const PROTECTED = 1 << 24;
        const PUBLIC = 1 << 25;
        const ABSTRACT = 1 << 26;

        // Special
        const HOISTED = 1 << 28;  // var and function declarations
        const IMPORT = 1 << 29;
        const EXPORT = 1 << 30;
    }
}

/// Represents a symbol (named declaration) in the program
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Unique identifier
    pub id: SymbolId,
    /// The name of the symbol
    pub name: String,
    /// Flags describing the symbol
    pub flags: SymbolFlags,
    /// The range where this symbol is declared
    pub declaration_range: Range,
    /// The range of just the name identifier
    pub name_range: Range,
    /// All references to this symbol (positions where it's used)
    pub references: Vec<Range>,
    /// The scope this symbol belongs to
    pub scope_id: u32,
    /// JSDoc documentation if available
    pub documentation: Option<String>,
}

impl Symbol {
    pub fn new(
        id: SymbolId,
        name: String,
        flags: SymbolFlags,
        declaration_range: Range,
        name_range: Range,
        scope_id: u32,
    ) -> Self {
        Self {
            id,
            name,
            flags,
            declaration_range,
            name_range,
            references: Vec::new(),
            scope_id,
            documentation: None,
        }
    }

    /// Check if the symbol is a value (can be used in expressions)
    pub fn is_value(&self) -> bool {
        self.flags.intersects(
            SymbolFlags::VARIABLE
                | SymbolFlags::FUNCTION
                | SymbolFlags::CLASS
                | SymbolFlags::ENUM
                | SymbolFlags::PARAMETER
                | SymbolFlags::PROPERTY
                | SymbolFlags::METHOD
                | SymbolFlags::NAMESPACE
                | SymbolFlags::ENUM_MEMBER,
        )
    }

    /// Check if the symbol is a type (can be used in type positions)
    pub fn is_type(&self) -> bool {
        self.flags.intersects(
            SymbolFlags::CLASS
                | SymbolFlags::INTERFACE
                | SymbolFlags::TYPE_ALIAS
                | SymbolFlags::ENUM
                | SymbolFlags::TYPE_PARAMETER,
        )
    }

    /// Check if the symbol is hoisted (var, function)
    pub fn is_hoisted(&self) -> bool {
        self.flags.contains(SymbolFlags::HOISTED)
    }

    /// Add a reference to this symbol
    pub fn add_reference(&mut self, range: Range) {
        self.references.push(range);
    }

    /// Check if a position is within this symbol's declaration
    pub fn contains_position(&self, pos: Position) -> bool {
        pos >= self.declaration_range.start && pos <= self.declaration_range.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_range() -> Range {
        Range {
            start: Position::new(0, 0),
            end: Position::new(0, 10),
        }
    }

    #[test]
    fn test_symbol_id_new() {
        let id = SymbolId::new(42);
        assert_eq!(id.0, 42);
    }

    #[test]
    fn test_symbol_id_equality() {
        let id1 = SymbolId::new(1);
        let id2 = SymbolId::new(1);
        let id3 = SymbolId::new(2);

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_symbol_flags_none() {
        let flags = SymbolFlags::NONE;
        assert!(flags.is_empty());
    }

    #[test]
    fn test_symbol_flags_combine() {
        let flags = SymbolFlags::VARIABLE | SymbolFlags::CONST;
        assert!(flags.contains(SymbolFlags::VARIABLE));
        assert!(flags.contains(SymbolFlags::CONST));
        assert!(!flags.contains(SymbolFlags::FUNCTION));
    }

    #[test]
    fn test_symbol_new() {
        let range = create_test_range();
        let symbol = Symbol::new(
            SymbolId::new(1),
            "test".to_string(),
            SymbolFlags::VARIABLE,
            range,
            range,
            0,
        );

        assert_eq!(symbol.id, SymbolId::new(1));
        assert_eq!(symbol.name, "test");
        assert_eq!(symbol.flags, SymbolFlags::VARIABLE);
        assert_eq!(symbol.scope_id, 0);
        assert!(symbol.references.is_empty());
        assert!(symbol.documentation.is_none());
    }

    #[test]
    fn test_symbol_is_value() {
        let range = create_test_range();

        let var_symbol = Symbol::new(
            SymbolId::new(1),
            "x".to_string(),
            SymbolFlags::VARIABLE,
            range,
            range,
            0,
        );
        assert!(var_symbol.is_value());

        let func_symbol = Symbol::new(
            SymbolId::new(2),
            "f".to_string(),
            SymbolFlags::FUNCTION,
            range,
            range,
            0,
        );
        assert!(func_symbol.is_value());

        let class_symbol = Symbol::new(
            SymbolId::new(3),
            "C".to_string(),
            SymbolFlags::CLASS,
            range,
            range,
            0,
        );
        assert!(class_symbol.is_value());

        let interface_symbol = Symbol::new(
            SymbolId::new(4),
            "I".to_string(),
            SymbolFlags::INTERFACE,
            range,
            range,
            0,
        );
        assert!(!interface_symbol.is_value());
    }

    #[test]
    fn test_symbol_is_type() {
        let range = create_test_range();

        let interface_symbol = Symbol::new(
            SymbolId::new(1),
            "IUser".to_string(),
            SymbolFlags::INTERFACE,
            range,
            range,
            0,
        );
        assert!(interface_symbol.is_type());

        let type_alias = Symbol::new(
            SymbolId::new(2),
            "StringOrNumber".to_string(),
            SymbolFlags::TYPE_ALIAS,
            range,
            range,
            0,
        );
        assert!(type_alias.is_type());

        let class_symbol = Symbol::new(
            SymbolId::new(3),
            "MyClass".to_string(),
            SymbolFlags::CLASS,
            range,
            range,
            0,
        );
        assert!(class_symbol.is_type()); // Classes are both values and types

        let var_symbol = Symbol::new(
            SymbolId::new(4),
            "x".to_string(),
            SymbolFlags::VARIABLE,
            range,
            range,
            0,
        );
        assert!(!var_symbol.is_type());
    }

    #[test]
    fn test_symbol_is_hoisted() {
        let range = create_test_range();

        let var_symbol = Symbol::new(
            SymbolId::new(1),
            "x".to_string(),
            SymbolFlags::VARIABLE | SymbolFlags::HOISTED,
            range,
            range,
            0,
        );
        assert!(var_symbol.is_hoisted());

        let let_symbol = Symbol::new(
            SymbolId::new(2),
            "y".to_string(),
            SymbolFlags::VARIABLE | SymbolFlags::LET,
            range,
            range,
            0,
        );
        assert!(!let_symbol.is_hoisted());
    }

    #[test]
    fn test_symbol_add_reference() {
        let range = create_test_range();
        let mut symbol = Symbol::new(
            SymbolId::new(1),
            "test".to_string(),
            SymbolFlags::VARIABLE,
            range,
            range,
            0,
        );

        assert!(symbol.references.is_empty());

        let ref_range = Range {
            start: Position::new(5, 0),
            end: Position::new(5, 4),
        };
        symbol.add_reference(ref_range);

        assert_eq!(symbol.references.len(), 1);
        assert_eq!(symbol.references[0], ref_range);
    }

    #[test]
    fn test_symbol_contains_position() {
        let range = Range {
            start: Position::new(1, 0),
            end: Position::new(1, 10),
        };
        let symbol = Symbol::new(
            SymbolId::new(1),
            "test".to_string(),
            SymbolFlags::VARIABLE,
            range,
            range,
            0,
        );

        // Position within range
        assert!(symbol.contains_position(Position::new(1, 5)));

        // Position at start
        assert!(symbol.contains_position(Position::new(1, 0)));

        // Position at end
        assert!(symbol.contains_position(Position::new(1, 10)));

        // Position before
        assert!(!symbol.contains_position(Position::new(0, 5)));

        // Position after
        assert!(!symbol.contains_position(Position::new(2, 0)));
    }

    #[test]
    fn test_symbol_flags_exported() {
        let range = create_test_range();
        let symbol = Symbol::new(
            SymbolId::new(1),
            "exportedFunc".to_string(),
            SymbolFlags::FUNCTION | SymbolFlags::EXPORTED,
            range,
            range,
            0,
        );

        assert!(symbol.flags.contains(SymbolFlags::EXPORTED));
        assert!(symbol.is_value());
    }

    #[test]
    fn test_symbol_flags_async() {
        let range = create_test_range();
        let symbol = Symbol::new(
            SymbolId::new(1),
            "asyncFunc".to_string(),
            SymbolFlags::FUNCTION | SymbolFlags::ASYNC,
            range,
            range,
            0,
        );

        assert!(symbol.flags.contains(SymbolFlags::ASYNC));
    }

    #[test]
    fn test_symbol_flags_class_modifiers() {
        let range = create_test_range();
        let symbol = Symbol::new(
            SymbolId::new(1),
            "privateMethod".to_string(),
            SymbolFlags::METHOD | SymbolFlags::PRIVATE | SymbolFlags::STATIC,
            range,
            range,
            0,
        );

        assert!(symbol.flags.contains(SymbolFlags::PRIVATE));
        assert!(symbol.flags.contains(SymbolFlags::STATIC));
        assert!(!symbol.flags.contains(SymbolFlags::PUBLIC));
    }
}
