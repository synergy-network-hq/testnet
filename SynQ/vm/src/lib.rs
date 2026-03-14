pub mod opcode;
pub mod vm;
pub mod assembler;

// Re-export for convenience
pub use opcode::{OpCode, VMError};
pub use vm::{QuantumVM, Value};
pub use assembler::Assembler;
