//! Runtime components: VM, Value, Type, Object systems.

pub mod object;
pub mod types;
pub mod value;
pub mod vm;

// Re-export key runtime types
// pub use value::Value;
// pub use types::Type;
// pub use object::Ref;
// pub use vm::ByteCodeMachine;