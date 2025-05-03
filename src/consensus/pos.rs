use crate::consensus::types::{Consensus, ConsensusBlockHeader, BlockValidationResult, ConsensusStatus};
use crate::types::{Hash, Address, Slot};
use crate::consensus::ConsensusConfig;
use blake3;
use std::collections::{HashMap, BTreeMap};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha20Rng;

/// Validator information
#[derive(Debug, Clone)]
pub struct Validator {
    /// Validator address
    pub address: Address,
    
    /// Staked amount
    pub stake: u64,
    
    /// Public key (for verifying blocks)
    pub public_key: Vec<u8>,
    
    /// When this validator joined
    pub joined_epoch: u64,
    
    /// Last block produced
    pub last_block_height: u64,
}

/// Proof of Stake consensus implementation
pub struct ProofOfStake {
    /// Consensus configuration
    config: ConsensusConfig,
    
    /// Current blockchain head
    head: Option<ConsensusBlockHeader>,
    
    /// Block headers by hash
    headers: HashMap<Hash, ConsensusBlockHeader>,
    
    /// Validators by address
    validators: HashMap<Address, Validator>,
    
    /// Genesis timestamp
    genesis_time: u64,
    
    /// Slot duration in seconds
    slot_duration: u64,
    
    /// Epochs (blocks per epoch)
    blocks_per_epoch: u64,
    
    /// Randomness source for validator selection
    randomness_seed: Hash,
    
    /// Current epoch
    current_epoch: u64,
    
    /// Finalized block height (2/3 of validators have confirmed)
    finalized_height: u64,
}

impl ProofOfStake {
    /// Create a new PoS consensus instance
    pub fn new(
        config: ConsensusConfig, 
        genesis_time: u64,
        slot_duration: u64,
        blocks_per_epoch: u64
    ) -> Self {
        Self {
            config,
            head: None,
            headers: HashMap::new(),
            validators: HashMap::new(),
            genesis_time,
            slot_duration,
            blocks_per_epoch,
            randomness_seed: [0u8; 32],
            current_epoch: 0,
            finalized_height: 0,
        }
    }
    
    /// Register a new validator
    pub fn register_validator(
        &mut self,
        address: Address,
        stake: u64,
        public_key: Vec<u8>,
    ) -> Result<(), String> {
        // Check minimum stake
        if stake < self.config.pos_minimum_stake {
            return Err(format!(
                "Stake amount {} is below minimum {}",
                stake, self.config.pos_minimum_stake
            ));
        }
        
        // Check if we're at max validators
        if self.validators.len() >= self.config.pos_max_validators as usize {
            return Err("Maximum validator count reached".to_string());
        }
        
        // Create validator
        let validator = Validator {
            address,
            stake,
            public_key,
            joined_epoch: self.current_epoch,
            last_block_height: 0,
        };
        
        // Add to validators
        self.validators.insert(address, validator);
        
        Ok(())
    }
    
    /// Get total stake amount
    pub fn total_stake(&self) -> u64 {
        self.validators.values().map(|v| v.stake).sum()
    }
    
    /// Update the randomness seed (e.g., at epoch boundaries)
    fn update_randomness(&mut self, new_seed: Hash) {
        self.randomness_seed = new_seed;
    }
    
    /// Select a validator for the given slot
    fn select_validator_for_slot(&self, slot: Slot) -> Option<Address> {
        if self.validators.is_empty() {
            return None;
        }
        
        // Deterministic randomness based on slot and seed
        let mut hasher = blake3::Hasher::new();
        hasher.update(&self.randomness_seed);
        hasher.update(&slot.to_le_bytes());
        let rand_hash = hasher.finalize();
        
        // Convert to a seed for the RNG
        let mut seed = [0u8; 32];
        seed.copy_from_slice(rand_hash.as_bytes());
        
        // Create a deterministic RNG
        let mut rng = ChaCha20Rng::from_seed(seed);
        
        // Weight validators by stake
        let total_stake = self.total_stake();
        let target = rng.gen_range(0..total_stake);
        
        // Find the validator
        let mut cumulative = 0;
        for (addr, validator) in &self.validators {
            cumulative += validator.stake;
            if cumulative > target {
                return Some(*addr);
            }
        }
        
        // Fallback to the first validator (should never happen)
        self.validators.keys().next().copied()
    }
    
    /// Add a block to the chain
    fn add_block(&mut self, header: ConsensusBlockHeader) -> Result<(), String> {
        // Add to headers map
        self.headers.insert(header.hash, header.clone());
        
        // Update head if this is a longer chain
        if self.head.is_none() || header.height > self.head.as_ref().unwrap().height {
            self.head = Some(header.clone());
            
            // Update the validator's last block
            if let Some(producer) = self.get_block_producer(header.slot) {
                if let Some(validator) = self.validators.get_mut(&producer) {
                    validator.last_block_height = header.height;
                }
            }
            
            // Update epoch if needed
            let new_epoch = header.height / self.blocks_per_epoch;
            if new_epoch > self.current_epoch {
                self.current_epoch = new_epoch;
                
                // Update randomness at epoch boundaries
                self.update_randomness(header.hash);
            }
        }
        
        Ok(())
    }
}

impl Consensus for ProofOfStake {
    fn initialize(&mut self) -> Result<(), String> {
        // For PoS, we might need to load validator state, etc.
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
            
            // Check timestamp and slot
            if block_header.timestamp <= parent.timestamp {
                return BlockValidationResult::Invalid(
                    "Block timestamp must be greater than parent timestamp".to_string()
                );
            }
        }
        
        // Verify the block producer is authorized for this slot
        let expected_producer = self.get_block_producer(block_header.slot);
        
        if expected_producer.is_none() {
            return BlockValidationResult::Invalid(
                format!("No validator authorized for slot {}", block_header.slot)
            );
        }
        
        // In a real implementation, we would verify the block signature here
        
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
            current_difficulty: None,
            active_validators: Some(self.validators.len() as u32),
            total_staked: Some(self.total_stake()),
            current_epoch: Some(self.current_epoch),
            last_finalized_height: self.finalized_height,
            head_hash: head.map_or([0u8; 32], |h| h.hash),
        }
    }
    
    fn can_produce_block(&self, slot: Slot) -> bool {
        self.get_block_producer(slot).is_some()
    }
    
    fn get_block_producer(&self, slot: Slot) -> Option<Address> {
        self.select_validator_for_slot(slot)
    }
} 