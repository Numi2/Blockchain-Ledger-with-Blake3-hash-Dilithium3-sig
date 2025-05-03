// SSZ (Simple Serialize) Serialization module
mod ssz;

pub use ssz::{serialize, deserialize, hash_tree_root};

/// Error type for SSZ operations
#[derive(Debug, thiserror::Error)]
pub enum SSZError {
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Deserialization error: {0}")]
    Deserialization(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Buffer error: {0}")]
    BufferError(String),
    
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for SSZ operations
pub type SSZResult<T> = std::result::Result<T, SSZError>;
