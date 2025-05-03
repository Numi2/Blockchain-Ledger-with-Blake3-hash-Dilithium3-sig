// Core blockchain modules

// Block structure module (header, body, transactions)
pub mod block;

// Consensus related functionality
pub mod consensus;

// Cryptographic utilities
pub mod crypto;

// State management for the blockchain
pub mod state;

// Execution engine
pub mod execution;

// Memory pool for pending transactions
pub mod mempool;

// Blockchain synchronization
pub mod sync;

// Transaction validation
pub mod validation;

// STARK proofs for state transitions
pub mod stark;

pub use block::{Block, BlockHeader, BlockBody};
pub use mempool::MemPool;
pub use state::{UTXOState, State};
pub use validation::{TransactionValidator, ValidationError, ValidationResult};
pub use stark::{StateProof, ProofGenerator, ProofVerifier, StarkError, StarkResult};
