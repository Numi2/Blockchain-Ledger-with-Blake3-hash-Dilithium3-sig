use crate::consensus::types::{Consensus, ConsensusBlockHeader, BlockValidationResult, ConsensusStatus};
use crate::types::{Hash, Address, Slot};
use crate::consensus::ConsensusConfig;
use blake3;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

// Maximum target (minimum difficulty)
const MAX_TARGET: u64 = 0xFFFFFFFFFFFFFFFF;

/// Proof of Work consensus implementation
pub struct ProofOfWork {
    /// Consensus configuration
    config: ConsensusConfig,
    
    /// Current blockchain head
    head: Option<ConsensusBlockHeader>,
    
    /// Block headers by hash
    headers: HashMap<Hash, ConsensusBlockHeader>,
    
    /// Current difficulty
    current_difficulty: u64,
    
    /// Genesis timestamp
    genesis_time: u64,
    
    /// List of recent blocks for difficulty adjustment
    recent_blocks: Vec<ConsensusBlockHeader>,
}

impl ProofOfWork {
    /// Create a new PoW consensus instance
    pub fn new(config: ConsensusConfig, genesis_time: u64) -> Self {
        Self {
            config,
            head: None,
            headers: HashMap::new(),
            current_difficulty: config.pow_initial_difficulty,
            genesis_time,
            recent_blocks: Vec::new(),
        }
    }
    
    /// Check if the block hash meets the difficulty requirement
    pub fn check_pow(&self, block_hash: &Hash, difficulty: u64) -> bool {
        // Convert hash to a u64 for comparison (first 8 bytes)
        let hash_value = u64::from_be_bytes([
            block_hash[0], block_hash[1], block_hash[2], block_hash[3],
            block_hash[4], block_hash[5], block_hash[6], block_hash[7],
        ]);
        
        // The hash must be less than or equal to the target
        let target = MAX_TARGET / difficulty;
        hash_value <= target
    }
    
    /// Calculate the difficulty for a new block
    fn calculate_difficulty(&self) -> u64 {
        if self.recent_blocks.len() < self.config.pow_difficulty_adjustment_window as usize {
            return self.current_difficulty;
        }
        
        // Get the first and last block in the window
        let first = &self.recent_blocks[0];
        let last = &self.recent_blocks[self.recent_blocks.len() - 1];
        
        // Calculate the actual time it took to mine these blocks
        let time_diff = last.timestamp.saturating_sub(first.timestamp);
        if time_diff == 0 {
            return self.current_difficulty;
        }
        
        // Calculate the expected time
        let expected_time = self.config.target_block_time * (self.recent_blocks.len() as u64 - 1);
        
        // Adjust difficulty
        let mut new_difficulty = (self.current_difficulty as f64 * expected_time as f64 / time_diff as f64) as u64;
        
        // Clamp to avoid extreme changes
        let max_adjustment = self.current_difficulty * 4;
        let min_adjustment = self.current_difficulty / 4;
        
        if new_difficulty > max_adjustment {
            new_difficulty = max_adjustment;
        } else if new_difficulty < min_adjustment {
            new_difficulty = min_adjustment;
        }
        
        // Ensure minimum difficulty of 1
        if new_difficulty < 1 {
            new_difficulty = 1;
        }
        
        new_difficulty
    }
    
    /// Add a block to the chain
    fn add_block(&mut self, header: ConsensusBlockHeader) -> Result<(), String> {
        // Add to headers map
        self.headers.insert(header.hash, header.clone());
        
        // Update head if this is a longer chain
        if self.head.is_none() || header.height > self.head.as_ref().unwrap().height {
            self.head = Some(header.clone());
        }
        
        // Add to recent blocks for difficulty adjustment
        self.recent_blocks.push(header);
        
        // Keep only the last N blocks for difficulty adjustment
        if self.recent_blocks.len() > self.config.pow_difficulty_adjustment_window as usize {
            self.recent_blocks.remove(0);
        }
        
        // Recalculate difficulty if needed
        self.current_difficulty = self.calculate_difficulty();
        
        Ok(())
    }
    
    /// Calculate the hash for mining
    pub fn calculate_mining_hash(&self, header: &ConsensusBlockHeader, nonce: u64) -> Hash {
        let mut hasher = blake3::Hasher::new();
        
        // Add block data
        hasher.update(&header.prev_hash);
        hasher.update(&header.height.to_le_bytes());
        hasher.update(&header.timestamp.to_le_bytes());
        hasher.update(&header.state_root);
        hasher.update(&header.transactions_root);
        
        // Add nonce
        hasher.update(&nonce.to_le_bytes());
        
        // Finalize hash
        let hash_result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(hash_result.as_bytes());
        
        hash
    }
}

impl Consensus for ProofOfWork {
    fn initialize(&mut self) -> Result<(), String> {
        // For PoW, we don't need much initialization
        Ok(())
    }
    
    fn validate_block(&self, block_header: &ConsensusBlockHeader) -> BlockValidationResult {
        // Check if parent exists (except for genesis)
        if block_header.height > 0 {
            if !self.headers.contains_key(&block_header.prev_hash) {
                return BlockValidationResult::Pending(
                    format!("Parent block {} not found", hex::encode(block_header.prev_hash))
                );
            }
            
            // Get parent
            let parent = &self.headers[&block_header.prev_hash];
            
            // Check height
            if block_header.height != parent.height + 1 {
                return BlockValidationResult::Invalid(
                    format!("Invalid height: expected {}, got {}", 
                            parent.height + 1, block_header.height)
                );
            }
            
            // Check timestamp
            if block_header.timestamp <= parent.timestamp {
                return BlockValidationResult::Invalid(
                    "Block timestamp must be greater than parent timestamp".to_string()
                );
            }
        }
        
        // Verify PoW
        if !self.check_pow(&block_header.hash, self.current_difficulty) {
            return BlockValidationResult::Invalid(
                format!("Proof of work verification failed for block {}", 
                        hex::encode(block_header.hash))
            );
        }
        
        BlockValidationResult::Valid
    }
    
    fn process_block(&mut self, block_header: &ConsensusBlockHeader) -> Result<(), String> {
        // Validate the block first
        match self.validate_block(block_header) {
            BlockValidationResult::Valid => {},
            BlockValidationResult::Invalid(reason) => {
                return Err(format!("Invalid block: {}", reason));
            },
            BlockValidationResult::Pending(reason) => {
                return Err(format!("Cannot process block: {}", reason));
            }
        }
        
        // Add the block to our chain
        self.add_block(block_header.clone())
    }
    
    fn status(&self) -> ConsensusStatus {
        let head = self.head.as_ref();
        
        ConsensusStatus {
            current_height: head.map_or(0, |h| h.height),
            current_slot: head.map_or(0, |h| h.slot),
            current_difficulty: Some(self.current_difficulty),
            active_validators: None,
            total_staked: None,
            current_epoch: None,
            last_finalized_height: head.map_or(0, |h| h.height),
            head_hash: head.map_or([0u8; 32], |h| h.hash),
        }
    }
    
    fn can_produce_block(&self, _slot: Slot) -> bool {
        // In PoW, anyone can produce a block at any time
        true
    }
    
    fn get_block_producer(&self, _slot: Slot) -> Option<Address> {
        // In PoW, there's no predetermined block producer
        None
    }
} 