pub mod binder;
pub mod scope;
pub mod symbol;
pub mod symbol_table;

// Re-export public API
// Note: Some items are used internally or for future features
#[allow(unused_imports)]
pub use binder::Binder;
pub use scope::{Scope, ScopeKind};
pub use symbol::{Symbol, SymbolFlags, SymbolId};
pub use symbol_table::SymbolTable;
