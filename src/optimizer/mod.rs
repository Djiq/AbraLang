//! Optimization pipelines for both AST and Bytecode.

// Make sure paths are correct relative to the new structure

// --- Public API ---
// (optimize_ast, optimize_bytecode functions as before)
// ...

// --- AST Optimizer Module ---
mod ast_optimizer {
    // Use items from optimizer::mod.rs
    // Import specific AST nodes relative to crate root

    // ... content of ast_optimizer module ...
    // Ensure internal `use` statements reference correctly (e.g. StaticValue from runtime::value)
}

// --- Bytecode Optimizer Module ---
mod bytecode_optimizer {
    // Use items from optimizer::mod.rs
    // Import specific bytecodes relative to crate root
    // If using helpers like try_evaluate_binary_op
    // If using helpers like try_evaluate_binary_op

    // ... content of bytecode_optimizer module ...
    // Ensure internal `use` statements reference correctly
}
