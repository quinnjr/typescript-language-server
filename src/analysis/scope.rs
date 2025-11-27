//! Scope definitions for lexical analysis
//! Some fields/methods are reserved for future features

#![allow(dead_code)]

use std::collections::HashMap;

use tower_lsp::lsp_types::Range;

use super::SymbolId;

/// The kind of scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    /// Global/module scope
    Global,
    /// Function scope (includes arrow functions)
    Function,
    /// Block scope (if, for, while, etc.)
    Block,
    /// Class body scope
    Class,
    /// For loop initializer scope
    ForLoop,
    /// Catch clause scope
    Catch,
    /// Switch statement scope
    Switch,
    /// With statement scope (deprecated but valid)
    With,
}

/// Represents a lexical scope in the program
#[derive(Debug, Clone)]
pub struct Scope {
    /// Unique identifier for this scope
    pub id: u32,
    /// The kind of scope
    pub kind: ScopeKind,
    /// Parent scope id (None for global scope)
    pub parent: Option<u32>,
    /// Child scope ids
    pub children: Vec<u32>,
    /// The range this scope covers
    pub range: Range,
    /// Symbols declared in this scope (name -> symbol id)
    pub symbols: HashMap<String, SymbolId>,
    /// Type symbols declared in this scope (for interfaces, type aliases)
    pub type_symbols: HashMap<String, SymbolId>,
}

impl Scope {
    pub fn new(id: u32, kind: ScopeKind, parent: Option<u32>, range: Range) -> Self {
        Self {
            id,
            kind,
            parent,
            children: Vec::new(),
            range,
            symbols: HashMap::new(),
            type_symbols: HashMap::new(),
        }
    }

    /// Add a symbol to this scope
    pub fn add_symbol(&mut self, name: String, symbol_id: SymbolId) {
        self.symbols.insert(name, symbol_id);
    }

    /// Add a type symbol to this scope
    pub fn add_type_symbol(&mut self, name: String, symbol_id: SymbolId) {
        self.type_symbols.insert(name, symbol_id);
    }

    /// Look up a symbol by name in this scope only
    pub fn lookup_local(&self, name: &str) -> Option<SymbolId> {
        self.symbols.get(name).copied()
    }

    /// Look up a type symbol by name in this scope only
    pub fn lookup_type_local(&self, name: &str) -> Option<SymbolId> {
        self.type_symbols.get(name).copied()
    }

    /// Check if this scope can have var declarations hoisted through it
    pub fn allows_var_hoisting(&self) -> bool {
        match self.kind {
            ScopeKind::Global | ScopeKind::Function => false,
            _ => true,
        }
    }

    /// Check if a position is within this scope
    pub fn contains_position(&self, pos: tower_lsp::lsp_types::Position) -> bool {
        pos >= self.range.start && pos <= self.range.end
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::Position;

    fn create_test_range() -> Range {
        Range {
            start: Position::new(0, 0),
            end: Position::new(10, 0),
        }
    }

    #[test]
    fn test_scope_kind() {
        assert_eq!(ScopeKind::Global, ScopeKind::Global);
        assert_ne!(ScopeKind::Global, ScopeKind::Function);
    }

    #[test]
    fn test_scope_new() {
        let range = create_test_range();
        let scope = Scope::new(1, ScopeKind::Function, Some(0), range);

        assert_eq!(scope.id, 1);
        assert_eq!(scope.kind, ScopeKind::Function);
        assert_eq!(scope.parent, Some(0));
        assert!(scope.children.is_empty());
        assert!(scope.symbols.is_empty());
        assert!(scope.type_symbols.is_empty());
    }

    #[test]
    fn test_scope_new_global() {
        let range = create_test_range();
        let scope = Scope::new(0, ScopeKind::Global, None, range);

        assert_eq!(scope.id, 0);
        assert_eq!(scope.kind, ScopeKind::Global);
        assert!(scope.parent.is_none());
    }

    #[test]
    fn test_scope_add_symbol() {
        let range = create_test_range();
        let mut scope = Scope::new(0, ScopeKind::Global, None, range);

        let symbol_id = SymbolId::new(1);
        scope.add_symbol("test".to_string(), symbol_id);

        assert!(scope.symbols.contains_key("test"));
        assert_eq!(scope.symbols.get("test").copied(), Some(symbol_id));
    }

    #[test]
    fn test_scope_add_type_symbol() {
        let range = create_test_range();
        let mut scope = Scope::new(0, ScopeKind::Global, None, range);

        let symbol_id = SymbolId::new(1);
        scope.add_type_symbol("MyType".to_string(), symbol_id);

        assert!(scope.type_symbols.contains_key("MyType"));
        assert_eq!(scope.type_symbols.get("MyType").copied(), Some(symbol_id));
    }

    #[test]
    fn test_scope_lookup_local() {
        let range = create_test_range();
        let mut scope = Scope::new(0, ScopeKind::Global, None, range);

        let symbol_id = SymbolId::new(1);
        scope.add_symbol("x".to_string(), symbol_id);

        assert_eq!(scope.lookup_local("x"), Some(symbol_id));
        assert_eq!(scope.lookup_local("y"), None);
    }

    #[test]
    fn test_scope_lookup_type_local() {
        let range = create_test_range();
        let mut scope = Scope::new(0, ScopeKind::Global, None, range);

        let symbol_id = SymbolId::new(1);
        scope.add_type_symbol("User".to_string(), symbol_id);

        assert_eq!(scope.lookup_type_local("User"), Some(symbol_id));
        assert_eq!(scope.lookup_type_local("Person"), None);
    }

    #[test]
    fn test_scope_allows_var_hoisting() {
        let range = create_test_range();

        let global = Scope::new(0, ScopeKind::Global, None, range);
        assert!(!global.allows_var_hoisting()); // Stops hoisting

        let function = Scope::new(1, ScopeKind::Function, Some(0), range);
        assert!(!function.allows_var_hoisting()); // Stops hoisting

        let block = Scope::new(2, ScopeKind::Block, Some(1), range);
        assert!(block.allows_var_hoisting()); // Allows hoisting

        let for_loop = Scope::new(3, ScopeKind::ForLoop, Some(1), range);
        assert!(for_loop.allows_var_hoisting()); // Allows hoisting
    }

    #[test]
    fn test_scope_contains_position() {
        let range = Range {
            start: Position::new(5, 0),
            end: Position::new(10, 0),
        };
        let scope = Scope::new(0, ScopeKind::Function, None, range);

        // Position within range
        assert!(scope.contains_position(Position::new(7, 5)));

        // Position at start
        assert!(scope.contains_position(Position::new(5, 0)));

        // Position at end
        assert!(scope.contains_position(Position::new(10, 0)));

        // Position before
        assert!(!scope.contains_position(Position::new(4, 10)));

        // Position after
        assert!(!scope.contains_position(Position::new(11, 0)));
    }

    #[test]
    fn test_scope_multiple_symbols() {
        let range = create_test_range();
        let mut scope = Scope::new(0, ScopeKind::Global, None, range);

        scope.add_symbol("a".to_string(), SymbolId::new(1));
        scope.add_symbol("b".to_string(), SymbolId::new(2));
        scope.add_symbol("c".to_string(), SymbolId::new(3));

        assert_eq!(scope.symbols.len(), 3);
        assert_eq!(scope.lookup_local("a"), Some(SymbolId::new(1)));
        assert_eq!(scope.lookup_local("b"), Some(SymbolId::new(2)));
        assert_eq!(scope.lookup_local("c"), Some(SymbolId::new(3)));
    }

    #[test]
    fn test_scope_symbol_override() {
        let range = create_test_range();
        let mut scope = Scope::new(0, ScopeKind::Global, None, range);

        scope.add_symbol("x".to_string(), SymbolId::new(1));
        scope.add_symbol("x".to_string(), SymbolId::new(2));

        // Second insertion should override
        assert_eq!(scope.lookup_local("x"), Some(SymbolId::new(2)));
    }
}
