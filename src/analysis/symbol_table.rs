//! Symbol table for storing symbols and scopes
//! Some methods are reserved for future type checking features

#![allow(dead_code)]

use std::collections::HashMap;

use tower_lsp::lsp_types::{Position, Range};

use super::{Scope, ScopeKind, Symbol, SymbolFlags, SymbolId};

/// Stores all symbols and scopes for a document
#[derive(Debug)]
pub struct SymbolTable {
    /// All symbols indexed by their id
    symbols: HashMap<SymbolId, Symbol>,
    /// All scopes indexed by their id
    scopes: HashMap<u32, Scope>,
    /// The root scope id
    root_scope_id: u32,
    /// Counter for generating symbol ids
    next_symbol_id: u32,
    /// Counter for generating scope ids
    next_scope_id: u32,
}

impl SymbolTable {
    pub fn new() -> Self {
        let root_scope = Scope::new(
            0,
            ScopeKind::Global,
            None,
            Range {
                start: Position::new(0, 0),
                end: Position::new(u32::MAX, 0),
            },
        );

        let mut scopes = HashMap::new();
        scopes.insert(0, root_scope);

        Self {
            symbols: HashMap::new(),
            scopes,
            root_scope_id: 0,
            next_symbol_id: 0,
            next_scope_id: 1,
        }
    }

    /// Create a new symbol and add it to the table
    pub fn create_symbol(
        &mut self,
        name: String,
        flags: SymbolFlags,
        declaration_range: Range,
        name_range: Range,
        scope_id: u32,
    ) -> SymbolId {
        let id = SymbolId::new(self.next_symbol_id);
        self.next_symbol_id += 1;

        let symbol = Symbol::new(
            id,
            name.clone(),
            flags,
            declaration_range,
            name_range,
            scope_id,
        );

        // Add to scope
        if let Some(scope) = self.scopes.get_mut(&scope_id) {
            if flags.intersects(
                SymbolFlags::INTERFACE | SymbolFlags::TYPE_ALIAS | SymbolFlags::TYPE_PARAMETER,
            ) {
                scope.add_type_symbol(name, id);
            } else {
                scope.add_symbol(name, id);
            }
        }

        self.symbols.insert(id, symbol);
        id
    }

    /// Create a new scope
    pub fn create_scope(&mut self, kind: ScopeKind, parent_id: u32, range: Range) -> u32 {
        let id = self.next_scope_id;
        self.next_scope_id += 1;

        let scope = Scope::new(id, kind, Some(parent_id), range);
        self.scopes.insert(id, scope);

        // Add as child to parent
        if let Some(parent) = self.scopes.get_mut(&parent_id) {
            parent.children.push(id);
        }

        id
    }

    /// Get the root scope id
    pub fn root_scope_id(&self) -> u32 {
        self.root_scope_id
    }

    /// Get a symbol by id
    pub fn get_symbol(&self, id: SymbolId) -> Option<&Symbol> {
        self.symbols.get(&id)
    }

    /// Get a mutable symbol by id
    #[allow(dead_code)] // Reserved for future type checking features
    pub fn get_symbol_mut(&mut self, id: SymbolId) -> Option<&mut Symbol> {
        self.symbols.get_mut(&id)
    }

    /// Get a scope by id
    pub fn get_scope(&self, id: u32) -> Option<&Scope> {
        self.scopes.get(&id)
    }

    /// Get a mutable scope by id
    #[allow(dead_code)] // Reserved for future scope manipulation
    pub fn get_scope_mut(&mut self, id: u32) -> Option<&mut Scope> {
        self.scopes.get_mut(&id)
    }

    /// Look up a symbol by name, searching from the given scope upward
    pub fn lookup(&self, name: &str, scope_id: u32) -> Option<SymbolId> {
        let mut current_scope_id = Some(scope_id);

        while let Some(id) = current_scope_id {
            if let Some(scope) = self.scopes.get(&id) {
                if let Some(symbol_id) = scope.lookup_local(name) {
                    return Some(symbol_id);
                }
                current_scope_id = scope.parent;
            } else {
                break;
            }
        }

        None
    }

    /// Look up a type symbol by name, searching from the given scope upward
    pub fn lookup_type(&self, name: &str, scope_id: u32) -> Option<SymbolId> {
        let mut current_scope_id = Some(scope_id);

        while let Some(id) = current_scope_id {
            if let Some(scope) = self.scopes.get(&id) {
                if let Some(symbol_id) = scope.lookup_type_local(name) {
                    return Some(symbol_id);
                }
                current_scope_id = scope.parent;
            } else {
                break;
            }
        }

        None
    }

    /// Find the innermost scope containing a position
    pub fn scope_at_position(&self, pos: Position) -> u32 {
        self.find_scope_at_position(self.root_scope_id, pos)
    }

    fn find_scope_at_position(&self, scope_id: u32, pos: Position) -> u32 {
        if let Some(scope) = self.scopes.get(&scope_id) {
            // Check children first (they're more specific)
            for &child_id in &scope.children {
                if let Some(child) = self.scopes.get(&child_id) {
                    if child.contains_position(pos) {
                        return self.find_scope_at_position(child_id, pos);
                    }
                }
            }

            // No child contains the position, return this scope
            if scope.contains_position(pos) {
                return scope_id;
            }
        }

        self.root_scope_id
    }

    /// Find symbol at a specific position
    pub fn symbol_at_position(&self, pos: Position) -> Option<SymbolId> {
        for symbol in self.symbols.values() {
            if symbol.name_range.start <= pos && pos <= symbol.name_range.end {
                return Some(symbol.id);
            }
        }
        None
    }

    /// Get all symbols in the table
    pub fn all_symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.symbols.values()
    }

    /// Get all scopes in the table
    pub fn all_scopes(&self) -> impl Iterator<Item = &Scope> {
        self.scopes.values()
    }

    /// Add a reference to a symbol
    pub fn add_reference(&mut self, symbol_id: SymbolId, range: Range) {
        if let Some(symbol) = self.symbols.get_mut(&symbol_id) {
            symbol.add_reference(range);
        }
    }

    /// Find the definition of a symbol at a given position
    #[allow(dead_code)] // Reserved for future go-to-definition enhancements
    pub fn find_definition(&self, pos: Position) -> Option<&Symbol> {
        // First check if we're on a symbol's name
        if let Some(symbol_id) = self.symbol_at_position(pos) {
            return self.get_symbol(symbol_id);
        }

        // Otherwise, we might be on a reference - need to look up the name
        // This requires knowing what identifier we're on, which needs the source text
        None
    }

    /// Find all references to a symbol
    pub fn find_references(&self, symbol_id: SymbolId) -> Vec<Range> {
        if let Some(symbol) = self.get_symbol(symbol_id) {
            let mut refs = vec![symbol.name_range];
            refs.extend(symbol.references.iter().cloned());
            refs
        } else {
            Vec::new()
        }
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_table_new() {
        let table = SymbolTable::new();
        assert_eq!(table.root_scope_id(), 0);

        let root = table.get_scope(0).unwrap();
        assert_eq!(root.kind, ScopeKind::Global);
        assert!(root.parent.is_none());
    }

    #[test]
    fn test_symbol_table_default() {
        let table = SymbolTable::default();
        assert_eq!(table.root_scope_id(), 0);
    }

    #[test]
    fn test_create_symbol() {
        let mut table = SymbolTable::new();

        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(0, 10),
        };

        let id = table.create_symbol("test".to_string(), SymbolFlags::VARIABLE, range, range, 0);

        let symbol = table.get_symbol(id).unwrap();
        assert_eq!(symbol.name, "test");
        assert_eq!(symbol.flags, SymbolFlags::VARIABLE);
        assert_eq!(symbol.scope_id, 0);
    }

    #[test]
    fn test_create_type_symbol() {
        let mut table = SymbolTable::new();

        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(0, 10),
        };

        let id = table.create_symbol("User".to_string(), SymbolFlags::INTERFACE, range, range, 0);

        // Type symbols should be in type_symbols
        let scope = table.get_scope(0).unwrap();
        assert!(scope.type_symbols.contains_key("User"));
        assert_eq!(scope.type_symbols.get("User").copied(), Some(id));
    }

    #[test]
    fn test_create_scope() {
        let mut table = SymbolTable::new();

        let range = Range {
            start: Position::new(5, 0),
            end: Position::new(10, 0),
        };

        let scope_id = table.create_scope(ScopeKind::Function, 0, range);

        let scope = table.get_scope(scope_id).unwrap();
        assert_eq!(scope.kind, ScopeKind::Function);
        assert_eq!(scope.parent, Some(0));

        // Should be added as child to parent
        let parent = table.get_scope(0).unwrap();
        assert!(parent.children.contains(&scope_id));
    }

    #[test]
    fn test_lookup_in_scope() {
        let mut table = SymbolTable::new();

        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(0, 10),
        };

        let id = table.create_symbol("x".to_string(), SymbolFlags::VARIABLE, range, range, 0);

        assert_eq!(table.lookup("x", 0), Some(id));
        assert_eq!(table.lookup("y", 0), None);
    }

    #[test]
    fn test_lookup_in_parent_scope() {
        let mut table = SymbolTable::new();

        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(10, 0),
        };

        // Create symbol in root scope
        let id = table.create_symbol("x".to_string(), SymbolFlags::VARIABLE, range, range, 0);

        // Create child scope
        let child_scope_id = table.create_scope(ScopeKind::Function, 0, range);

        // Should find symbol from child scope
        assert_eq!(table.lookup("x", child_scope_id), Some(id));
    }

    #[test]
    fn test_lookup_shadowing() {
        let mut table = SymbolTable::new();

        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(10, 0),
        };

        // Create symbol in root scope
        let outer_id = table.create_symbol("x".to_string(), SymbolFlags::VARIABLE, range, range, 0);

        // Create child scope
        let child_scope_id = table.create_scope(ScopeKind::Block, 0, range);

        // Create shadowing symbol in child scope
        let inner_id = table.create_symbol(
            "x".to_string(),
            SymbolFlags::VARIABLE,
            range,
            range,
            child_scope_id,
        );

        // Should find inner symbol from child scope
        assert_eq!(table.lookup("x", child_scope_id), Some(inner_id));
        // Should find outer symbol from root scope
        assert_eq!(table.lookup("x", 0), Some(outer_id));
    }

    #[test]
    fn test_lookup_type() {
        let mut table = SymbolTable::new();

        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(0, 10),
        };

        let id = table.create_symbol("User".to_string(), SymbolFlags::INTERFACE, range, range, 0);

        assert_eq!(table.lookup_type("User", 0), Some(id));
        assert_eq!(table.lookup_type("Unknown", 0), None);
    }

    #[test]
    fn test_scope_at_position() {
        let mut table = SymbolTable::new();

        let child_range = Range {
            start: Position::new(5, 0),
            end: Position::new(10, 0),
        };

        let child_scope_id = table.create_scope(ScopeKind::Function, 0, child_range);

        // Position inside child scope
        let scope = table.scope_at_position(Position::new(7, 5));
        assert_eq!(scope, child_scope_id);

        // Position outside child scope
        let scope = table.scope_at_position(Position::new(2, 0));
        assert_eq!(scope, 0);
    }

    #[test]
    fn test_symbol_at_position() {
        let mut table = SymbolTable::new();

        let decl_range = Range {
            start: Position::new(0, 0),
            end: Position::new(0, 15),
        };
        let name_range = Range {
            start: Position::new(0, 6),
            end: Position::new(0, 7),
        };

        let id = table.create_symbol(
            "x".to_string(),
            SymbolFlags::VARIABLE,
            decl_range,
            name_range,
            0,
        );

        // Position on name
        assert_eq!(table.symbol_at_position(Position::new(0, 6)), Some(id));

        // Position outside name
        assert_eq!(table.symbol_at_position(Position::new(0, 10)), None);
    }

    #[test]
    fn test_add_reference() {
        let mut table = SymbolTable::new();

        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(0, 10),
        };

        let id = table.create_symbol("x".to_string(), SymbolFlags::VARIABLE, range, range, 0);

        let ref_range = Range {
            start: Position::new(5, 0),
            end: Position::new(5, 1),
        };

        table.add_reference(id, ref_range);

        let symbol = table.get_symbol(id).unwrap();
        assert_eq!(symbol.references.len(), 1);
        assert_eq!(symbol.references[0], ref_range);
    }

    #[test]
    fn test_find_references() {
        let mut table = SymbolTable::new();

        let decl_range = Range {
            start: Position::new(0, 0),
            end: Position::new(0, 10),
        };
        let name_range = Range {
            start: Position::new(0, 6),
            end: Position::new(0, 7),
        };

        let id = table.create_symbol(
            "x".to_string(),
            SymbolFlags::VARIABLE,
            decl_range,
            name_range,
            0,
        );

        let ref1 = Range {
            start: Position::new(1, 0),
            end: Position::new(1, 1),
        };
        let ref2 = Range {
            start: Position::new(2, 5),
            end: Position::new(2, 6),
        };

        table.add_reference(id, ref1);
        table.add_reference(id, ref2);

        let refs = table.find_references(id);
        assert_eq!(refs.len(), 3); // name_range + 2 references
        assert!(refs.contains(&name_range));
        assert!(refs.contains(&ref1));
        assert!(refs.contains(&ref2));
    }

    #[test]
    fn test_all_symbols() {
        let mut table = SymbolTable::new();

        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(0, 10),
        };

        table.create_symbol("a".to_string(), SymbolFlags::VARIABLE, range, range, 0);
        table.create_symbol("b".to_string(), SymbolFlags::FUNCTION, range, range, 0);
        table.create_symbol("c".to_string(), SymbolFlags::CLASS, range, range, 0);

        let symbols: Vec<_> = table.all_symbols().collect();
        assert_eq!(symbols.len(), 3);

        let names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"a"));
        assert!(names.contains(&"b"));
        assert!(names.contains(&"c"));
    }

    #[test]
    fn test_all_scopes() {
        let mut table = SymbolTable::new();

        let range = Range {
            start: Position::new(0, 0),
            end: Position::new(10, 0),
        };

        table.create_scope(ScopeKind::Function, 0, range);
        table.create_scope(ScopeKind::Block, 0, range);

        let scopes: Vec<_> = table.all_scopes().collect();
        assert_eq!(scopes.len(), 3); // root + 2 created
    }

    #[test]
    fn test_nested_scopes() {
        let mut table = SymbolTable::new();

        let outer_range = Range {
            start: Position::new(0, 0),
            end: Position::new(20, 0),
        };
        let inner_range = Range {
            start: Position::new(5, 0),
            end: Position::new(15, 0),
        };

        let outer_id = table.create_scope(ScopeKind::Function, 0, outer_range);
        let inner_id = table.create_scope(ScopeKind::Block, outer_id, inner_range);

        // Position in inner scope
        let scope = table.scope_at_position(Position::new(10, 0));
        assert_eq!(scope, inner_id);

        // Position in outer scope but not inner
        let scope = table.scope_at_position(Position::new(17, 0));
        assert_eq!(scope, outer_id);
    }
}
