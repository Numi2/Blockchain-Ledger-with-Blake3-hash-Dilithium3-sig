use crate::types::types::Hash;
use crate::core::state::utxo::{Utxo, TxInput, TxOutput, Transaction};
use crate::core::state::merkle::{MerkleTree, Blake3Merkle};
use std::collections::HashMap;
use crate::core::block::{Block, Transaction};
use crate::types::{Address, ExecutionStatus, Slot};
use blake3::Hasher;
use serde::{Deserialize, Serialize};

/// Represents the UTXO set and manages state transitions using Merkle 
tree state root.
pub struct UtxoSet {
    utxos: HashMap<(Hash, u32), Utxo>,
    merkle: Blake3Merkle,
}

impl UtxoSet {
    pub fn new() -> Self {
        Self {
            utxos: HashMap::new(),
            merkle: Blake3Merkle::new(),
        }
    }

    /// Apply a transaction to the current state. Returns true if 
successful.
    pub fn apply_transaction(&mut self, tx: &Transaction) -> bool {
        // Validate all inputs exist and sum >= sum of outputs
        let mut input_sum = 0u64;
        for inp in &tx.inputs {
            if let Some(utxo) = self.utxos.remove(&(inp.txid,
inp.index)) {
                input_sum += utxo.value;
                // Remove spent, update merkle
                self.merkle.insert(&[&utxo.txid[..],
&utxo.index.to_le_bytes()[..]].concat(), b"");
            } else {
                return false; // Double spend or invalid input
            }
        }
        let output_sum: u64 = tx.outputs.iter().map(|o| o.value).sum();
        if input_sum < output_sum {
            return false; // Insufficient input value
        }
        // Add outputs as new UTXOs
        for out in &tx.outputs {
            let key = (out.txid, out.index);
            self.utxos.insert(key, out.clone());
            self.merkle.insert(&[&out.txid[..],
&out.index.to_le_bytes()[..]].concat(), &out.value.to_le_bytes());
        }
        true
    }

    /// Apply a block: For now, just a batch of transactions.
    pub fn apply_block(&mut self, txs: &[Transaction]) -> bool {
        for tx in txs {
            if !self.apply_transaction(tx) {
                return false;
            }
        }
        true
    }

    /// Return state root (Merkle root over the UTXO set).
    pub fn state_root(&self) -> Hash {
        self.merkle.root()
    }
}

/// AccountState represents the state of an account in the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// Account balance
    pub balance: u64,
    
    /// Account nonce (for preventing replay attacks)
    pub nonce: u64,
    
    /// Account code (for smart contracts)
    pub code: Vec<u8>,
    
    /// Account storage (key-value store for contracts)
    pub storage: HashMap<Hash, Hash>,
}

impl Account {
    /// Create a new empty account
    pub fn new() -> Self {
        Self {
            balance: 0,
            nonce: 0,
            code: Vec::new(),
            storage: HashMap::new(),
        }
    }

    /// Create a new account with an initial balance
    pub fn with_balance(balance: u64) -> Self {
        let mut account = Self::new();
        account.balance = balance;
        account
    }

    /// Compute the hash of this account state
    pub fn hash(&self) -> Hash {
        let mut hasher = Hasher::new();
        
        // Hash account balance and nonce
        hasher.update(&self.balance.to_le_bytes());
        hasher.update(&self.nonce.to_le_bytes());
        
        // Hash the code
        hasher.update(&(self.code.len() as u64).to_le_bytes());
        hasher.update(&self.code);
        
        // Hash the storage - first sort keys for determinism
        let mut storage_keys: Vec<_> = self.storage.keys().collect();
        storage_keys.sort();
        
        for key in storage_keys {
            hasher.update(key);
            hasher.update(self.storage.get(key).unwrap());
        }
        
        // Get the hash result
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(result.as_bytes());
        hash
    }
}

/// WorldState represents the entire state of the blockchain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldState {
    /// Map of addresses to account states
    pub accounts: HashMap<Address, Account>,
    
    /// Current slot
    pub slot: Slot,
    
    /// Current block hash
    pub latest_block_hash: Hash,
}

impl WorldState {
    /// Create a new empty world state
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            slot: 0,
            latest_block_hash: [0u8; 32],
        }
    }

    /// Get an account state (creates new if doesn't exist)
    pub fn get_or_create_account(&mut self, address: &Address) -> &mut Account {
        if !self.accounts.contains_key(address) {
            self.accounts.insert(*address, Account::new());
        }
        self.accounts.get_mut(address).unwrap()
    }

    /// Apply a transaction to the world state
    pub fn apply_transaction(&mut self, tx: &Transaction) -> ExecutionStatus {
        // First, check the sender has enough balance and valid nonce
        let sender = tx.from;
        let sender_account = self.get_or_create_account(&sender);
        
        // Check nonce
        if sender_account.nonce != tx.nonce {
            return ExecutionStatus::InvalidInput;
        }
        
        // Check balance (including gas fees)
        let total_cost = tx.value + tx.gas_limit * tx.gas_price;
        if sender_account.balance < total_cost {
            return ExecutionStatus::OutOfGas;
        }
        
        // Increment sender's nonce
        sender_account.nonce += 1;
        
        // Deduct value from sender
        sender_account.balance -= tx.value;
        
        // If there's a recipient, add value to their account
        if let Some(to) = tx.to {
            let recipient_account = self.get_or_create_account(&to);
            recipient_account.balance += tx.value;
        }
        
        // TODO: Handle contract creation and execution
        // For now, we'll just return success
        ExecutionStatus::Success
    }

    /// Apply a block to the world state
    pub fn apply_block(&mut self, block: &Block) -> ExecutionStatus {
        // Update slot and latest block hash
        self.slot = block.header.slot;
        self.latest_block_hash = block.hash();
        
        // Apply all transactions
        for tx in &block.body.transactions {
            let status = self.apply_transaction(tx);
            if status != ExecutionStatus::Success {
                return status;
            }
        }
        
        ExecutionStatus::Success
    }

    /// Compute the state root hash
    pub fn state_root(&self) -> Hash {
        let mut hasher = Hasher::new();
        
        // Sort addresses for deterministic hashing
        let mut addresses: Vec<_> = self.accounts.keys().collect();
        addresses.sort();
        
        // Hash each account state
        for addr in addresses {
            hasher.update(addr);
            let account = &self.accounts[addr];
            hasher.update(&account.hash());
        }
        
        // Also include the current slot and latest block hash
        hasher.update(&self.slot.to_le_bytes());
        hasher.update(&self.latest_block_hash);
        
        // Get the hash result
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(result.as_bytes());
        hash
    }
}
