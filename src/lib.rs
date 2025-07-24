//! # Abra Programming Language Crate Root
//!
//! This crate contains the core components for the Abra language implementation,
//! including the parser, compiler, optimizer, and runtime virtual machine.

// Declare top-level modules
pub mod cli;
pub mod compiler;
pub mod frontend;
pub mod optimizer;
pub mod runtime;
#[cfg(test)]
pub mod test;

// Re-export key components for easier use if this were a library
// pub use compiler::compile_from_source;
// pub use runtime::execute_bytecode;
// ... etc.
