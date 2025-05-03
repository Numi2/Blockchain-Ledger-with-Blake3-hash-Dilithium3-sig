// Main module for world-ledger

// Core blockchain logic
pub mod core;

// Network and P2P communication
pub mod p2p;

// Storage for blockchain data
pub mod storage;

// Simple Serialize (SSZ) format
pub mod ssz;

// STARK proofs
pub mod stark;

// Global types
pub mod types;

// Configuration
pub mod config;

// RPC interfaces
pub mod rpc;

// Node implementation
pub mod node;

// CLI and command handlers
pub mod cmd;

// Export core modules
pub mod wallet;

// Re-export key types for ease of use
pub use types::{Address, Hash, Signature, Transaction};
