// Core wallet functionality
mod bip39;
mod keystore;
mod account;
mod transaction;

// Testing
#[cfg(test)]
mod test_dilithium;

// Re-exports
pub use bip39::*;
pub use keystore::*;
pub use account::*;
pub use transaction::*;

// Type aliases
pub type WalletResult<T> = Result<T, WalletError>;

// Wallet errors
#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("Wallet not found: {0}")]
    WalletNotFound(String),
    
    #[error("Account not found: {0}")]
    AccountNotFound(String),
    
    #[error("Invalid password")]
    InvalidPassword,
    
    #[error("Invalid key: {0}")]
    InvalidKey(String),
    
    #[error("Invalid mnemonic: {0}")]
    InvalidMnemonic(String),
    
    #[error("Crypto error: {0}")]
    Crypto(String),
    
    #[error("Storage error: {0}")]
    Storage(String),
    
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    
    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),
    
    #[error("Other error: {0}")]
    Other(String),
    
    #[error("Insufficient funds")]
    InsufficientFunds,
}

// If in a test build, export the test module
#[cfg(test)]
pub use test_dilithium; 