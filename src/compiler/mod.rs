//! Compiler components: AST to Bytecode translation.

pub mod bytecode;
pub mod compile; // Changed from compiler.rs to avoid name clash
pub mod typecheck;

// Re-export main components
pub use bytecode::ByteCode;
pub use compile::{Code, Compiler};
