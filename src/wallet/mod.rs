// Wallet module for key management and transaction signing
mod keystore;
mod account;
mod transaction;
mod bip39;

pub use keystore::*;
pub use account::*;
pub use transaction::*;
pub use bip39::*;

use thiserror::Error;

/// Error type for wallet operations
#[derive(Debug, Error)]
pub enum WalletError {
    #[error("Storage error: {0}")]
    Storage(#[from] crate::storage::StorageError),

    #[error("Invalid mnemonic: {0}")]
    InvalidMnemonic(String),

    #[error("Invalid key: {0}")]
    InvalidKey(String),

    #[error("Invalid password")]
    InvalidPassword,

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Insufficient funds")]
    InsufficientFunds,

    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),

    #[error("Crypto error: {0}")]
    Crypto(String),

    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for wallet operations
pub type WalletResult<T> = std::result::Result<T, WalletError>; 