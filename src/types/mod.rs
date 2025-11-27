pub mod checker;
pub mod printer;
pub mod types;

// Re-export public API for future use
#[allow(unused_imports)]
pub use checker::TypeChecker;
#[allow(unused_imports)]
pub use types::{Type, TypeFlags, TypeId};
