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

// Consensus implementations
pub mod consensus;

// Smart contract implementation
pub mod contracts;

// Monitoring and telemetry
pub mod monitoring;

// Export core modules
pub mod wallet;

// Re-export key types for ease of use
pub use types::{Address, Hash, Signature, Transaction, Slot, ValidatorIndex};

// Re-export consensus and core types
pub use consensus::ConsensusType;
pub use core::block::Block;
pub use core::mempool::MemPool;
pub use core::state::WorldState;
pub use core::sync::ChainSync;

// Export RPC types
pub use rpc::server::RpcServer;
