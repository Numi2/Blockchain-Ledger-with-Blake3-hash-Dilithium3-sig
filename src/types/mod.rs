// Basic blockchain types
mod transaction;
pub use transaction::Transaction;

/// Hash type (32 bytes)
pub type Hash = [u8; 32];

/// Address type (20 bytes)
pub type Address = [u8; 20];

/// Slot number type
pub type Slot = u64;

/// Validator index type
pub type ValidatorIndex = u32;

/// Signature type (variable length byte array)
pub type Signature = Vec<u8>;

/// Execution status
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ExecutionStatus {
    /// Execution succeeded
    Success,
    
    /// Execution reverted
    Reverted,
    
    /// Execution ran out of gas
    OutOfGas,
    
    /// Invalid operation
    InvalidOperation,
    
    /// Invalid jump destination
    InvalidJump,
    
    /// Stack overflow
    StackOverflow,
    
    /// Stack underflow
    StackUnderflow,
    
    /// Other error
    Other(String),
}
