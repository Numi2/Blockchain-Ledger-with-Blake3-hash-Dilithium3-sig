use crate::types::{Hash, Slot, Signature, ValidatorIndex};
use blake3::Hasher;
use serde::{Deserialize, Serialize};

/// BlockHeader represents the metadata of a block 
/// It contains all the information needed for consensus and validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Slot number (time unit in the chain)
    pub slot: Slot,
    
    /// Reference to parent block
    pub parent_hash: Hash,
    
    /// Root hash of the state after executing this block
    pub state_root: Hash,
    
    /// Root hash of the block body (transactions and other content)
    pub body_root: Hash,
    
    /// Index of the validator who proposed this block
    pub proposer_index: ValidatorIndex,
    
    /// Signature of the proposer
    pub proposer_signature: Signature,
}

impl BlockHeader {
    /// Creates a new block header
    pub fn new(
        slot: Slot,
        parent_hash: Hash,
        state_root: Hash,
        body_root: Hash,
        proposer_index: ValidatorIndex,
        proposer_signature: Signature,
    ) -> Self {
        Self {
            slot,
            parent_hash,
            state_root,
            body_root,
            proposer_index,
            proposer_signature,
        }
    }

    /// Compute the hash of this block header using BLAKE3
    pub fn hash(&self) -> Hash {
        let mut hasher = Hasher::new();
        
        // Add all header fields to the hasher
        hasher.update(&self.slot.to_le_bytes());
        hasher.update(&self.parent_hash);
        hasher.update(&self.state_root);
        hasher.update(&self.body_root);
        hasher.update(&self.proposer_index.to_le_bytes());
        
        // Note: We don't hash the signature as it's added after the block is created
        
        // Get the hash result and convert to our Hash type
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(result.as_bytes());
        hash
    }
} 