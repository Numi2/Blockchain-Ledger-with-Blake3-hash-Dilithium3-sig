use crate::types::{Address, Hash, Signature, Slot, ValidatorIndex};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Status of a validator in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidatorStatus {
    /// Validator is active and can propose/vote
    Active,
    
    /// Validator is pending activation
    Pending,
    
    /// Validator is exiting the system
    Exiting,
    
    /// Validator is no longer active
    Exited,
    
    /// Validator has been slashed for equivocation
    Slashed,
}

/// Validator represents a consensus participant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Validator {
    /// The validator's index
    pub index: ValidatorIndex,
    
    /// Address associated with this validator
    pub address: Address,
    
    /// Public key used for signatures
    pub public_key: Vec<u8>,
    
    /// Effective balance for consensus weight
    pub effective_balance: u64,
    
    /// Current status of the validator
    pub status: ValidatorStatus,
    
    /// Slot when the validator was activated
    pub activation_slot: Slot,
    
    /// Slot when the validator exited (if applicable)
    pub exit_slot: Option<Slot>,
}

/// ValidatorRegistry manages the set of validators in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorRegistry {
    /// Map of validator indices to validators
    validators: HashMap<ValidatorIndex, Validator>,
    
    /// Total active balance (for calculating quorums)
    total_active_balance: u64,
}

impl ValidatorRegistry {
    /// Create a new empty validator registry
    pub fn new() -> Self {
        Self {
            validators: HashMap::new(),
            total_active_balance: 0,
        }
    }

    /// Add a new validator to the registry
    pub fn add_validator(&mut self, validator: Validator) {
        // If the validator is active, update the total balance
        if validator.status == ValidatorStatus::Active {
            self.total_active_balance += validator.effective_balance;
        }
        
        self.validators.insert(validator.index, validator);
    }

    /// Get a validator by index
    pub fn get_validator(&self, index: ValidatorIndex) -> Option<&Validator> {
        self.validators.get(&index)
    }

    /// Get all active validators
    pub fn active_validators(&self) -> Vec<&Validator> {
        self.validators
            .values()
            .filter(|v| v.status == ValidatorStatus::Active)
            .collect()
    }

    /// Get the total number of validators
    pub fn validator_count(&self) -> usize {
        self.validators.len()
    }

    /// Get the total active balance
    pub fn total_active_balance(&self) -> u64 {
        self.total_active_balance
    }

    /// Get the validator that should propose in the given slot
    pub fn get_proposer_for_slot(&self, slot: Slot) -> Option<ValidatorIndex> {
        let active_validators = self.active_validators();
        if active_validators.is_empty() {
            return None;
        }
        
        // Simple rotation scheme - slot % number of active validators
        let proposer_idx = (slot as usize) % active_validators.len();
        Some(active_validators[proposer_idx].index)
    }

    /// Verify a signature by a validator
    pub fn verify_signature(
        &self,
        validator_index: ValidatorIndex,
        message: &[u8],
        signature: &Signature,
    ) -> bool {
        // Get the validator
        let validator = match self.get_validator(validator_index) {
            Some(v) => v,
            None => return false,
        };
        
        // Verify using dilithium3 (not implemented here)
        // TODO: Implement signature verification with dilithium3
        true // Placeholder - always return true for now
    }
} 