pub mod utxo;
pub mod merkle;
pub mod state;

pub use utxo::*;
pub use merkle::*;
pub use state::*;

use crate::types::{Address, Hash};
use std::collections::HashMap;

/// Account stores an individual account state
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Account {
    /// The account nonce (transaction count)
    pub nonce: u64,
    
    /// The account balance
    pub balance: u64,
    
    /// The account storage (key-value map)
    pub storage: HashMap<Hash, Hash>,
    
    /// The account code (for contracts)
    pub code: Vec<u8>,
}

impl Account {
    /// Create a new empty account
    pub fn new() -> Self {
        Self {
            nonce: 0,
            balance: 0,
            storage: HashMap::new(),
            code: Vec::new(),
        }
    }
    
    /// Create a new account with the given balance
    pub fn with_balance(balance: u64) -> Self {
        Self {
            nonce: 0,
            balance,
            storage: HashMap::new(),
            code: Vec::new(),
        }
    }
    
    /// Increment the account nonce
    pub fn increment_nonce(&mut self) {
        self.nonce += 1;
    }
    
    /// Update the account balance
    pub fn update_balance(&mut self, amount: i64) -> bool {
        if amount < 0 && self.balance < amount.unsigned_abs() {
            return false; // Insufficient funds
        }
        
        if amount < 0 {
            self.balance -= amount.unsigned_abs();
        } else {
            self.balance += amount as u64;
        }
        
        true
    }
    
    /// Set a storage key
    pub fn set_storage(&mut self, key: Hash, value: Hash) {
        self.storage.insert(key, value);
    }
    
    /// Get a storage value
    pub fn get_storage(&self, key: &Hash) -> Option<&Hash> {
        self.storage.get(key)
    }
    
    /// Set account code
    pub fn set_code(&mut self, code: Vec<u8>) {
        self.code = code;
    }
    
    /// Check if this is a contract account
    pub fn is_contract(&self) -> bool {
        !self.code.is_empty()
    }
}

/// WorldState represents the entire blockchain state
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorldState {
    /// Accounts by address
    pub accounts: HashMap<Address, Account>,
    
    /// State version/block height
    pub version: u64,
}

impl WorldState {
    /// Create a new empty world state
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            version: 0,
        }
    }
    
    /// Get an account by address
    pub fn get_account(&self, address: &Address) -> Option<&Account> {
        self.accounts.get(address)
    }
    
    /// Get a mutable account, creating it if it doesn't exist
    pub fn get_or_create_account(&mut self, address: &Address) -> &mut Account {
        self.accounts.entry(*address).or_insert_with(Account::new)
    }
    
    /// Delete an account
    pub fn delete_account(&mut self, address: &Address) -> Option<Account> {
        self.accounts.remove(address)
    }
    
    /// Calculate the root hash of the world state
    pub fn state_root(&self) -> Hash {
        // For now we'll use a simple hash of the serialized state
        // In production, this would be a Merkle Patricia Trie root
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        
        // Use BLAKE3 for hashing
        let hash = blake3::hash(&bytes);
        
        // Convert to a fixed-size array
        let mut state_root = [0u8; 32];
        state_root.copy_from_slice(hash.as_bytes());
        
        state_root
    }
}

