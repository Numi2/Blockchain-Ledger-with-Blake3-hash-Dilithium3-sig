use crate::core::block::Block;
use crate::types::{Hash, Signature, Slot, ValidatorIndex};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Type of vote in consensus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoteType {
    /// Vote to propose a new block
    Propose,
    
    /// Vote to prepare a block (first phase of finality)
    Prepare,
    
    /// Vote to commit a block (second phase of finality)
    Commit,
}

/// A vote cast by a validator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// The validator who cast this vote
    pub validator_index: ValidatorIndex,
    
    /// The type of vote
    pub vote_type: VoteType,
    
    /// The target block hash
    pub target_hash: Hash,
    
    /// The slot in which this vote was cast
    pub slot: Slot,
    
    /// The signature on this vote
    pub signature: Signature,
}

/// Status of a block in the finality process
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockStatus {
    /// Block has just been proposed but not prepared
    Proposed,
    
    /// Block has passed the prepare phase
    Prepared,
    
    /// Block has been committed
    Committed,
    
    /// Block has been finalized
    Finalized,
}

/// FinalityState tracks the votes and status of blocks in the finality process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityState {
    /// Map of block hash to block status
    pub block_status: HashMap<Hash, BlockStatus>,
    
    /// Map of block hash to prepare votes (by validator index)
    pub prepare_votes: HashMap<Hash, HashSet<ValidatorIndex>>,
    
    /// Map of block hash to commit votes (by validator index)
    pub commit_votes: HashMap<Hash, HashSet<ValidatorIndex>>,
    
    /// Most recent finalized block hash
    pub latest_finalized_hash: Hash,
    
    /// Slot of the most recent finalized block
    pub latest_finalized_slot: Slot,
}

impl FinalityState {
    /// Create a new finality state
    pub fn new() -> Self {
        Self {
            block_status: HashMap::new(),
            prepare_votes: HashMap::new(),
            commit_votes: HashMap::new(),
            latest_finalized_hash: [0u8; 32], // Genesis hash
            latest_finalized_slot: 0,         // Genesis slot
        }
    }

    /// Register a new block in the finality state
    pub fn add_block(&mut self, block: &Block) {
        let block_hash = block.hash();
        
        // Only track blocks we haven't seen
        if !self.block_status.contains_key(&block_hash) {
            self.block_status.insert(block_hash, BlockStatus::Proposed);
            self.prepare_votes.insert(block_hash, HashSet::new());
            self.commit_votes.insert(block_hash, HashSet::new());
        }
    }

    /// Process a vote and update the finality state
    pub fn process_vote(&mut self, vote: &Vote, required_votes: usize) -> bool {
        match vote.vote_type {
            VoteType::Propose => {
                // Propose votes don't affect finality directly
                true
            }
            
            VoteType::Prepare => {
                // Add prepare vote
                if let Some(votes) = self.prepare_votes.get_mut(&vote.target_hash) {
                    votes.insert(vote.validator_index);
                    
                    // Check if we have enough prepare votes
                    if votes.len() >= required_votes {
                        // Update block status to prepared
                        self.block_status.insert(vote.target_hash, BlockStatus::Prepared);
                    }
                    true
                } else {
                    false // Unknown block
                }
            }
            
            VoteType::Commit => {
                // Check if the block is prepared before accepting commit votes
                if self.block_status.get(&vote.target_hash) != Some(&BlockStatus::Prepared) {
                    return false;
                }
                
                // Add commit vote
                if let Some(votes) = self.commit_votes.get_mut(&vote.target_hash) {
                    votes.insert(vote.validator_index);
                    
                    // Check if we have enough commit votes
                    if votes.len() >= required_votes {
                        // Update block status to committed
                        self.block_status.insert(vote.target_hash, BlockStatus::Committed);
                    }
                    true
                } else {
                    false // Unknown block
                }
            }
        }
    }

    /// Finalize a committed block
    pub fn finalize_block(&mut self, block_hash: Hash, slot: Slot) -> bool {
        // Check if the block is committed
        if self.block_status.get(&block_hash) != Some(&BlockStatus::Committed) {
            return false;
        }
        
        // Update status to finalized
        self.block_status.insert(block_hash, BlockStatus::Finalized);
        
        // Update latest finalized block
        self.latest_finalized_hash = block_hash;
        self.latest_finalized_slot = slot;
        
        true
    }

    /// Check if a block has been finalized
    pub fn is_finalized(&self, block_hash: &Hash) -> bool {
        self.block_status.get(block_hash) == Some(&BlockStatus::Finalized)
    }
}

/// The 3-slot finality mechanism
pub struct ThreeSlotFinality {
    /// The current finality state
    pub state: FinalityState,
    
    /// Number of validators in the system
    pub validator_count: usize,
}

impl ThreeSlotFinality {
    /// Create a new 3-slot finality mechanism
    pub fn new(validator_count: usize) -> Self {
        Self {
            state: FinalityState::new(),
            validator_count,
        }
    }

    /// Calculate the required votes for finality (2/3 of validators)
    pub fn required_votes(&self) -> usize {
        (self.validator_count * 2 / 3) + 1
    }

    /// Process votes for a slot
    pub fn process_slot_votes(&mut self, votes: &[Vote]) -> Vec<Hash> {
        let required_votes = self.required_votes();
        let mut newly_finalized = Vec::new();
        
        // Process all votes
        for vote in votes {
            self.state.process_vote(vote, required_votes);
        }
        
        // Check for blocks that can be finalized
        for (hash, status) in &self.state.block_status {
            if *status == BlockStatus::Committed {
                let commit_votes = self.state.commit_votes.get(hash).unwrap();
                if commit_votes.len() >= required_votes {
                    // This is a simplification - in reality, we need to also check the slot
                    let assumed_slot = self.state.latest_finalized_slot + 1;
                    if self.state.finalize_block(*hash, assumed_slot) {
                        newly_finalized.push(*hash);
                    }
                }
            }
        }
        
        newly_finalized
    }
} 