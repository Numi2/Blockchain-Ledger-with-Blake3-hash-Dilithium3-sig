// Shared types and interfaces across the protocol
use serde::{Deserialize, Serialize};

/// Core cryptographic hash type used throughout the protocol (BLAKE3, 32 bytes)
pub type Hash = [u8; 32];

/// Address type (20 bytes, similar to Ethereum)
pub type Address = [u8; 20];

/// Slot identifier (incremental time units in consensus)
pub type Slot = u64;

/// Validator identifier
pub type ValidatorIndex = u32;

/// Signature type for Dilithium3 post-quantum signatures
pub type Signature = [u8; 2420]; // Dilithium3 signature size

/// Simple enum to represent the different chain networks
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Network {
    Mainnet,
    Testnet,
    Devnet,
}

/// Status codes for transaction execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Success,
    Failure,
    OutOfGas,
    InvalidInput,
}
