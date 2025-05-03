// Storage module for persistent blockchain data
mod database;
mod columns;
mod blocks;
mod state;
mod transactions;
mod crypto;

pub use database::*;
pub use columns::*;
pub use blocks::*;
pub use state::*;
pub use transactions::*;
pub use crypto::*;

/// Error type for storage operations
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(#[from] rocksdb::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Item not found: {0}")]
    NotFound(String),

    #[error("Invalid data: {0}")]
    InvalidData(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for storage operations
pub type StorageResult<T> = std::result::Result<T, StorageError>;
