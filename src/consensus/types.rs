use crate::types::{Hash, Address, Slot};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

/// Block header for consensus-related information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusBlockHeader {
    /// Block hash
    pub hash: Hash,
    
    /// Previous block hash
    pub prev_hash: Hash,
    
    /// Block height
    pub height: u64,
    
    /// Slot number
    pub slot: Slot,
    
    /// Timestamp (seconds since Unix epoch)
    pub timestamp: u64,
    
    /// Merkle root of the state
    pub state_root: Hash,
    
    /// Merkle root of transactions
    pub transactions_root: Hash,
}

/// Block validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockValidationResult {
    /// Block is valid
    Valid,
    
    /// Block is invalid
    Invalid(String),
    
    /// Block validation is pending (e.g., waiting for parent)
    Pending(String),
}

/// Consensus status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusStatus {
    /// Current block height
    pub current_height: u64,
    
    /// Current slot
    pub current_slot: Slot,
    
    /// Current difficulty (for PoW)
    pub current_difficulty: Option<u64>,
    
    /// Current active validators count (for PoS)
    pub active_validators: Option<u32>,
    
    /// Total staked amount (for PoS)
    pub total_staked: Option<u64>,
    
    /// Current epoch (for PoS)
    pub current_epoch: Option<u64>,
    
    /// Last finalized block height
    pub last_finalized_height: u64,
    
    /// Chain head hash
    pub head_hash: Hash,
}

/// Get current Unix timestamp
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

/// Compute slot from timestamp
pub fn timestamp_to_slot(timestamp: u64, genesis_time: u64, slot_duration: u64) -> Slot {
    if timestamp < genesis_time {
        return 0;
    }
    
    ((timestamp - genesis_time) / slot_duration) as Slot
}

/// Consensus trait that all consensus mechanisms must implement
pub trait Consensus {
    /// Initialize the consensus mechanism
    fn initialize(&mut self) -> Result<(), String>;
    
    /// Validate a block
    fn validate_block(&self, block_header: &ConsensusBlockHeader) -> BlockValidationResult;
    
    /// Process a new block
    fn process_block(&mut self, block_header: &ConsensusBlockHeader) -> Result<(), String>;
    
    /// Get the current status
    fn status(&self) -> ConsensusStatus;
    
    /// Check if a block can be produced in the current slot
    fn can_produce_block(&self, slot: Slot) -> bool;
    
    /// Get the next block producer for a slot
    fn get_block_producer(&self, slot: Slot) -> Option<Address>;
} 